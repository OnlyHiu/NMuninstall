# Generates the full Tauri icon set from a single drawing routine.
# Usage: powershell -ExecutionPolicy Bypass -File scripts\generate-icons.ps1
param(
    [string]$OutDir = (Join-Path $PSScriptRoot "..\src-tauri\icons")
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$script:Cache = @{}

function Get-Badge([int]$Size) {
    if ($script:Cache.ContainsKey($Size)) { return $script:Cache[$Size] }

    $bmp = [System.Drawing.Bitmap]::new($Size, $Size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.Clear([System.Drawing.Color]::Transparent)

    $s = [double]$Size
    $r = 0.22   # corner radius as a fraction of the icon size

    # Rounded-square background in the Windows accent colour.
    $bg = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(255, 0, 103, 192))
    $d = $s * $r * 2
    $path = [System.Drawing.Drawing2D.GraphicsPath]::new()
    $path.AddArc([single]0, [single]0, [single]$d, [single]$d, 180, 90)
    $path.AddArc([single]($s - $d), [single]0, [single]$d, [single]$d, 270, 90)
    $path.AddArc([single]($s - $d), [single]($s - $d), [single]$d, [single]$d, 0, 90)
    $path.AddArc([single]0, [single]($s - $d), [single]$d, [single]$d, 90, 90)
    $path.CloseFigure()
    $g.FillPath($bg, $path)
    $bg.Dispose()
    $path.Dispose()

    # Downward arrow = "uninstall".
    $white = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::White)
    $stem = $s * 0.11
    $g.FillRectangle($white, [single](($s - $stem) / 2), [single]($s * 0.19), [single]$stem, [single]($s * 0.33))

    $pts = [System.Drawing.PointF[]]@(
        [System.Drawing.PointF]::new([single]($s * 0.29), [single]($s * 0.50)),
        [System.Drawing.PointF]::new([single]($s * 0.71), [single]($s * 0.50)),
        [System.Drawing.PointF]::new([single]($s * 0.50), [single]($s * 0.73))
    )
    $g.FillPolygon($white, $pts)

    # Tray line = "installed software".
    $g.FillRectangle($white, [single]($s * 0.26), [single]($s * 0.79), [single]($s * 0.48), [single]($s * 0.10))

    $white.Dispose()
    $g.Dispose()
    $script:Cache[$Size] = $bmp
    return $bmp
}

# --- PNG set -------------------------------------------------------------
$sizes = [ordered]@{
    '32x32.png' = 32
    '128x128.png' = 128
    '128x128@2x.png' = 256
    'icon.png' = 512
    'Square30x30Logo.png' = 30
    'Square44x44Logo.png' = 44
    'Square71x71Logo.png' = 71
    'Square89x89Logo.png' = 89
    'Square107x107Logo.png' = 107
    'Square142x142Logo.png' = 142
    'Square150x150Logo.png' = 150
    'Square284x284Logo.png' = 284
    'Square310x310Logo.png' = 310
    'StoreLogo.png' = 50
}

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
foreach ($name in $sizes.Keys) {
    $bmp = Get-Badge $sizes[$name]
    $bmp.Save((Join-Path $OutDir $name), [System.Drawing.Imaging.ImageFormat]::Png)
    Write-Host ("png  {0} ({1})" -f $name, $sizes[$name])
}

# --- ICO set --------------------------------------------------------------
# The Windows resource compiler (rc.exe, used by tauri-winres) rejects
# PNG-compressed ICO entries, so every entry is written as a 32bpp
# BITMAPINFOHEADER DIB: header, bottom-up BGRA pixels, then a 1bpp AND mask.
function ConvertTo-DibEntry([System.Drawing.Bitmap]$Bmp) {
    $w = $Bmp.Width
    $h = $Bmp.Height

    $rect = New-Object System.Drawing.Rectangle 0, 0, $w, $h
    $data = $Bmp.LockBits($rect, [System.Drawing.Imaging.ImageLockMode]::ReadOnly, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $stride = $w * 4
    $pixels = New-Object byte[] ($stride * $h)
    [System.Runtime.InteropServices.Marshal]::Copy($data.Scan0, $pixels, 0, $pixels.Length)
    $Bmp.UnlockBits($data)

    # DIB rows run bottom-up; the bitmap buffer is top-down.
    $xor = New-Object byte[] ($stride * $h)
    for ($y = 0; $y -lt $h; $y++) {
        [Array]::Copy($pixels, $y * $stride, $xor, ($h - 1 - $y) * $stride, $stride)
    }

    # 1bpp AND mask, each row padded to a 4-byte boundary. Alpha carries the
    # real transparency, so the mask stays all-zero (fully opaque).
    $maskStride = [int]([Math]::Ceiling($w / 32.0) * 4)
    $and = New-Object byte[] ($maskStride * $h)

    $header = New-Object byte[] 40
    [BitConverter]::GetBytes([UInt32]40).CopyTo($header, 0)                    # biSize
    [BitConverter]::GetBytes([Int32]$w).CopyTo($header, 4)                      # biWidth
    [BitConverter]::GetBytes([Int32]($h * 2)).CopyTo($header, 8)                # biHeight = image + mask
    [BitConverter]::GetBytes([UInt16]1).CopyTo($header, 12)                     # biPlanes
    [BitConverter]::GetBytes([UInt16]32).CopyTo($header, 14)                    # biBitCount
    [BitConverter]::GetBytes([UInt32]0).CopyTo($header, 16)                     # BI_RGB
    [BitConverter]::GetBytes([UInt32]($xor.Length + $and.Length)).CopyTo($header, 20)  # biSizeImage

    $out = New-Object byte[] (40 + $xor.Length + $and.Length)
    [Array]::Copy($header, 0, $out, 0, 40)
    [Array]::Copy($xor, 0, $out, 40, $xor.Length)
    [Array]::Copy($and, 0, $out, 40 + $xor.Length, $and.Length)
    return $out
}

function New-IconFile([int[]]$Sizes, [string]$Path) {
    $count = $Sizes.Count
    $dirSize = 6
    $entrySize = 16
    $payloads = New-Object System.Collections.Generic.List[byte[]]

    $header = [byte[]]::new($dirSize + $entrySize * $count)
    # ICONDIR
    [BitConverter]::GetBytes([UInt16]0).CopyTo($header, 0)      # reserved
    [BitConverter]::GetBytes([UInt16]1).CopyTo($header, 2)      # type: icon
    [BitConverter]::GetBytes([UInt16]$count).CopyTo($header, 4) # image count

    $offset = $dirSize + $entrySize * $count
    for ($i = 0; $i -lt $count; $i++) {
        $size = $Sizes[$i]
        $bytes = ConvertTo-DibEntry (Get-Badge $size)
        $payloads.Add($bytes)

        # ICONDIRENTRY for image $i starts after the directory and previous entries.
        $e = $dirSize + $i * $entrySize
        $dim = if ($size -ge 256) { [byte]0 } else { [byte]$size }
        $header[$e + 0] = $dim                                   # width  (0 = 256)
        $header[$e + 1] = $dim                                   # height (0 = 256)
        $header[$e + 2] = 0                                      # palette size
        $header[$e + 3] = 0                                      # reserved
        [BitConverter]::GetBytes([UInt16]1).CopyTo($header, $e + 4)    # colour planes
        [BitConverter]::GetBytes([UInt16]32).CopyTo($header, $e + 6)   # bits per pixel
        [BitConverter]::GetBytes([UInt32]$bytes.Length).CopyTo($header, $e + 8)
        [BitConverter]::GetBytes([UInt32]$offset).CopyTo($header, $e + 12)
        $offset += $bytes.Length
    }

    $fs = [System.IO.File]::Create($Path)
    try {
        $fs.Write($header, 0, $header.Length)
        foreach ($p in $payloads) { $fs.Write($p, 0, $p.Length) }
    }
    finally {
        $fs.Dispose()
    }
    Write-Host ("ico  {0} ({1})" -f (Split-Path -Leaf $Path), ($Sizes -join ', '))
}

New-IconFile @(16, 24, 32, 48, 64, 128, 256) (Join-Path $OutDir 'icon.ico')
New-IconFile @(16, 32, 48) (Join-Path $OutDir 'icon_16.ico')
New-IconFile @(32, 64, 128) (Join-Path $OutDir 'icon_32.ico')
New-IconFile @(48, 64, 128) (Join-Path $OutDir 'icon_64.ico')
New-IconFile @(256) (Join-Path $OutDir 'icon_256.ico')

Write-Host "icons written to $OutDir"
