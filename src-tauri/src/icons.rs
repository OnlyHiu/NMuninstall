//! Resolves a `DisplayIcon` registry value and renders it into something a
//! WebView `<img>` can actually display.
//!
//! `DisplayIcon` has three shapes in the wild:
//!   `C:\path\app.exe,0`      — explicit resource index
//!   `C:\path\app.exe`        — bare path
//!   `%SystemRoot%\system32\a.dll,-101`
//!
//! [`resolve_icon_path`] normalises all three to an existing file, but that is
//! only half the job. The result is a **PE or ICO container**, and a WebView
//! decodes neither — handing the raw bytes to `<img src>` produced a broken
//! image for essentially every program, which the UI then masked behind a
//! letter avatar. [`icon_data_url`] therefore asks the Windows shell for the
//! icon and re-encodes it as a PNG data URL.

use std::path::PathBuf;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use image::imageops::{self, FilterType};
use image::{ImageFormat, RgbaImage};
use tracing::trace;

/// Longest expansion we accept, to stay inside the legacy `MAX_PATH` budget.
const MAX_EXPANDED_LEN: usize = 260;

/// Prefix of the URL produced by [`icon_data_url`].
const PNG_DATA_URL_PREFIX: &str = "data:image/png;base64,";

/// Rendered sizes outside this range are clamped. The lower bound keeps a
/// tiny row from upscaling a 32 px shell icon into visible mush; the upper
/// bound stops a single detail-panel request from ballooning into a
/// multi-megabyte PNG.
const MIN_ICON_PX: u32 = 16;
const MAX_ICON_PX: u32 = 128;

/// Splits a trailing `,<integer>` index suffix, but only when the suffix really
/// is an integer (`C:\a,b.exe` must survive intact).
pub fn strip_icon_index(raw: &str) -> &str {
    match raw.rsplit_once(',') {
        Some((head, tail)) if tail.trim().parse::<i32>().is_ok() => head,
        _ => raw,
    }
}

/// Expands `%VAR%` references present in the string.
///
/// Fails closed (returns `None`) when a variable cannot be resolved or when the
/// expansion would exceed the legacy `MAX_PATH` budget.
pub fn expand_env_vars(input: &str) -> Option<String> {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(end) = after.find('%') else {
            // Unterminated `%` — keep the remainder verbatim.
            out.push('%');
            out.push_str(after);
            return finish(out);
        };
        let name = &after[..end];
        if name.is_empty() {
            out.push('%');
            rest = &after[end + 1..];
            continue;
        }
        // `%SystemRoot%` and friends are present in the process environment on
        // Windows; an unresolved name aborts expansion (fail closed).
        out.push_str(&std::env::var(name).ok()?);
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    finish(out)
}

fn finish(out: String) -> Option<String> {
    if out.len() > MAX_EXPANDED_LEN {
        None
    } else {
        Some(out)
    }
}

/// Full resolution: strip index, expand env vars, verify existence.
pub fn resolve_icon_path(display_icon: Option<&str>) -> Option<PathBuf> {
    let raw = display_icon?.trim();
    if raw.is_empty() {
        return None;
    }
    let without_index = strip_icon_index(raw).trim().trim_matches('"').trim();
    if without_index.is_empty() {
        return None;
    }
    let expanded = if without_index.contains('%') {
        expand_env_vars(without_index)?
    } else {
        without_index.to_string()
    };
    let path = PathBuf::from(&expanded);
    if !path.is_file() {
        trace!(icon = %expanded, "DisplayIcon target missing, hiding icon");
        return None;
    }
    Some(path)
}

/// Renders a program icon into a `data:image/png;base64,…` URL.
///
/// Returns `None` when the program has no icon, the file has since vanished,
/// or the shell refused to decode it. Callers are expected to fall back to a
/// letter avatar — a missing icon must never be able to break the list.
pub fn icon_data_url(icon_path: Option<&str>, size: u32) -> Option<String> {
    let path = resolve_icon_path(icon_path)?;
    let png = render_png(&path, size.clamp(MIN_ICON_PX, MAX_ICON_PX))?;
    Some(format!("{PNG_DATA_URL_PREFIX}{}", BASE64.encode(png)))
}

/// Asks the platform for the icon and encodes it as PNG.
///
/// The `DisplayIcon` resource index is dropped during scanning (`ProgramInfo`
/// keeps only the resolved path), so extraction always requests icon group 0.
/// A multi-image `.ico` that relies on an explicit index therefore still lands
/// on image 0 — unchanged from the previous asset-protocol behaviour.
#[cfg(windows)]
fn render_png(path: &std::path::Path, size: u32) -> Option<Vec<u8>> {
    let (bgra, width, height) = shell::extract_first_icon(path)?;
    let image = to_top_down_rgba(&bgra, width, height)?;
    let scaled = upscale_if_needed(image, size);

    let mut out = Vec::new();
    scaled
        .write_to(&mut std::io::Cursor::new(&mut out), ImageFormat::Png)
        .ok()?;
    Some(out)
}

#[cfg(not(windows))]
fn render_png(_path: &std::path::Path, _size: u32) -> Option<Vec<u8>> {
    None
}

/// Converts a bottom-up 32bpp BGRA DIB into a top-down RGBA image.
///
/// Both conversions live out here rather than inside the FFI layer so they can
/// be unit tested on any platform:
///   * scanlines are flipped — GDI hands back bottom-up rows, PNG stores
///     top-down;
///   * blue and red are swapped — GDI is BGRA, PNG is RGBA.
fn to_top_down_rgba(bgra: &[u8], width: u32, height: u32) -> Option<RgbaImage> {
    let stride = width as usize * 4;
    if bgra.len() != stride * height as usize {
        return None;
    }
    let mut out = bgra.to_vec();

    // Swap row 0 with row n-1, row 1 with row n-2, and so on. `zip` stops at
    // the shorter half, which is what we want for an odd `height`.
    let half = stride * (height as usize / 2);
    let (top, bottom) = out.split_at_mut(half);
    for (t, b) in top
        .chunks_exact_mut(stride)
        .zip(bottom.chunks_exact_mut(stride).rev())
    {
        t.swap_with_slice(b);
    }

    let mut any_opaque = false;
    for px in out.chunks_exact_mut(4) {
        px.swap(0, 2);
        any_opaque |= px[3] != 0;
    }
    if !any_opaque {
        // Legacy 32bpp icons leave the alpha byte at 0 even though every pixel
        // is meant to be visible. Encoding that as-is yields a fully
        // transparent square, so read "no alpha anywhere" as "fully opaque".
        for px in out.chunks_exact_mut(4) {
            px[3] = 0xFF;
        }
    }

    RgbaImage::from_raw(width, height, out)
}

/// Upscales when — and only when — the shell returned less than we asked for.
///
/// `ExtractIconExW` sizes icons from `SM_CXICON`, so a HiDPI display already
/// hands back 48 or 64 px for free. Below that the browser downscales on its
/// own, which looks better than any filter we could run here.
fn upscale_if_needed(image: RgbaImage, target: u32) -> RgbaImage {
    if target <= image.width().max(image.height()) {
        return image;
    }
    // CatmullRom keeps hard icon edges crisp; Lanczos3 would ring around them.
    imageops::resize(&image, target, target, FilterType::CatmullRom)
}

/// The only `unsafe` corner of the crate: a thin wrapper over the Win32 icon
/// APIs. Everything above this module is portable and unit tested.
#[cfg(windows)]
mod shell {
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use std::ptr;

    use windows_sys::Win32::Graphics::Gdi::{
        GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS,
    };
    use windows_sys::Win32::UI::Shell::ExtractIconExW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO};

    /// Refuses a corrupt DIB header that would otherwise send us allocating a
    /// gigabyte-sized pixel buffer.
    const MAX_EDGE_PX: u32 = 1024;

    /// Loads the first icon group of `path` and returns its pixels as
    /// **bottom-up BGRA**, together with the dimensions.
    pub fn extract_first_icon(path: &Path) -> Option<(Vec<u8>, u32, u32)> {
        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let mut large: HICON = ptr::null_mut();
        // SAFETY: `wide` is a NUL-terminated UTF-16 string that outlives the
        // call and `large` is a valid out-pointer. A null `small` simply means
        // the 16×16 variant is not requested.
        let found = unsafe { ExtractIconExW(wide.as_ptr(), 0, &mut large, ptr::null_mut(), 1) };
        if found == 0 || large.is_null() {
            return None;
        }

        // SAFETY: `icon` holds a live handle just returned by `ExtractIconExW`.
        unsafe { icon_dib(&OwnedIcon(large).0) }
    }

    /// `ExtractIconExW` hands back a copy the caller owns. Leaking one per
    /// program row would pin GDI objects until the process exits.
    struct OwnedIcon(HICON);

    impl Drop for OwnedIcon {
        fn drop(&mut self) {
            // SAFETY: the handle came from `ExtractIconExW` and is destroyed
            // exactly once, by this single owner.
            unsafe { DestroyIcon(self.0) };
        }
    }

    /// Walks `icon → ICONINFO → DIB` and copies the colour bitmap out.
    ///
    /// # Safety
    /// `icon` must be a live `HICON` from `ExtractIconExW` that has not been
    /// destroyed yet.
    unsafe fn icon_dib(icon: &HICON) -> Option<(Vec<u8>, u32, u32)> {
        let mut info = ICONINFO::default();
        if GetIconInfo(*icon, &mut info) == 0 || info.hbmColor.is_null() {
            // `hbmColor` is null for legacy monochrome icons, whose shape lives
            // in the 1-bit mask. No supported installer registers one.
            return None;
        }

        // `GetObjectW` writes a `BITMAP` for a device-dependent bitmap and a
        // `BITMAPINFO` for a DIB. Both are 40 bytes and both put the width at
        // offset 4 and the height at offset 8, so a single read covers either.
        // What differs is everything after — and getting that wrong is fatal:
        // reading a DDB as a `BITMAPINFOHEADER` leaves `biSize = 0` and
        // `biPlanes = bmWidthBytes`, and `GetDIBits` rejects that request
        // outright, which is why this used to return no icon at all. The
        // output header is therefore rebuilt from scratch below instead of
        // being patched in place.
        let mut probed = BITMAPINFO::default();
        if GetObjectW(
            info.hbmColor.cast(),
            std::mem::size_of::<BITMAPINFO>() as i32,
            ptr::from_mut(&mut probed).cast(),
        ) == 0
        {
            return None;
        }

        let width = probed.bmiHeader.biWidth.unsigned_abs();
        let height = probed.bmiHeader.biHeight.unsigned_abs();
        if width == 0 || height == 0 || width > MAX_EDGE_PX || height > MAX_EDGE_PX {
            return None;
        }

        // Ask for 32bpp bottom-up no matter how the source bitmap is stored,
        // so callers only ever have to understand one buffer layout.
        let mut bmi = BITMAPINFO::default();
        bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = width as i32;
        bmi.bmiHeader.biHeight = height as i32;
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = 0; // BI_RGB
        bmi.bmiHeader.biSizeImage = width * height * 4;

        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        let dc = GetDC(ptr::null_mut());
        if dc.is_null() {
            return None;
        }
        let scanned = GetDIBits(
            dc,
            info.hbmColor,
            0,
            height,
            pixels.as_mut_ptr().cast(),
            &mut bmi,
            DIB_RGB_COLORS,
        );
        ReleaseDC(ptr::null_mut(), dc);

        if scanned == 0 {
            return None;
        }
        Some((pixels, width, height))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_only_integer_suffixes() {
        assert_eq!(strip_icon_index(r"C:\a\app.exe,0"), r"C:\a\app.exe");
        assert_eq!(strip_icon_index(r"C:\a\app.exe,-101"), r"C:\a\app.exe");
        assert_eq!(strip_icon_index(r"C:\a\app.exe"), r"C:\a\app.exe");
        // Not an index, so it belongs to the file name.
        assert_eq!(strip_icon_index(r"C:\a\b.exe"), r"C:\a\b.exe");
    }

    #[test]
    fn env_expansion_resolves_system_root() {
        let out = expand_env_vars(r"%SystemRoot%\system32\kernel32.dll").expect("expansion");
        assert!(out.to_lowercase().ends_with(r"system32\kernel32.dll"));
        assert!(!out.contains('%'));
    }

    #[test]
    fn env_expansion_leaves_plain_strings_alone() {
        assert_eq!(
            expand_env_vars(r"C:\Program Files\App\app.exe").unwrap(),
            r"C:\Program Files\App\app.exe"
        );
    }

    #[test]
    fn env_expansion_rejects_unknown_variables() {
        assert!(expand_env_vars(r"%NM_DEFINITELY_NOT_SET_VAR_12345%\x").is_none());
    }

    #[test]
    fn resolves_existing_file_only() {
        // A file that certainly exists on Windows.
        let got = resolve_icon_path(Some(r"C:\Windows\System32\kernel32.dll,0"));
        assert!(got.is_some(), "kernel32.dll should resolve");
        assert!(resolve_icon_path(Some(r"C:\definitely\not\here.exe")).is_none());
        assert!(resolve_icon_path(None).is_none());
        assert!(resolve_icon_path(Some("   ")).is_none());
    }

    // ── rendering ──────────────────────────────────────────────────────────

    /// One BGRA pixel: `(b, g, r, a)`.
    fn bgra_pixel(b: u8, g: u8, r: u8, a: u8) -> [u8; 4] {
        [b, g, r, a]
    }

    /// Flattens pixels into the byte buffer `to_top_down_rgba` expects.
    fn pixels(px: &[[u8; 4]]) -> Vec<u8> {
        px.iter().flatten().copied().collect()
    }

    /// Resolves a file below `%WINDIR%`.
    ///
    /// The icon tests need a binary that genuinely carries an `RT_ICON`
    /// resource. `kernel32.dll`, `gdi32.dll` and `winspool.drv` do not —
    /// `ExtractIconExW` correctly returns 0 for them — and .NET's
    /// `Icon.ExtractAssociatedIcon` hides this by silently handing back a
    /// generic shell icon, so it is useless as a probe.
    #[cfg(windows)]
    fn windir_file(relative: &str) -> String {
        let root = std::env::var_os("WINDIR").expect("WINDIR is always set on Windows");
        std::path::Path::new(&root)
            .join(relative)
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn flips_rows_and_swaps_blue_for_red() {
        // 1×2, bottom-up: the *first* input pixel is the bottom scanline, so
        // the flipped result must start with the second one — and its red
        // channel must come from the input's blue byte.
        let bottom = bgra_pixel(0x11, 0x22, 0x33, 0xFF);
        let top = bgra_pixel(0x44, 0x55, 0x66, 0xFF);
        let got = to_top_down_rgba(&pixels(&[bottom, top]), 1, 2).expect("1x2 image");
        assert_eq!(got.dimensions(), (1, 2));
        assert_eq!(&got.as_raw()[..4], &[0x66, 0x55, 0x44, 0xFF], "top row");
        assert_eq!(&got.as_raw()[4..], &[0x33, 0x22, 0x11, 0xFF], "bottom row");
    }

    #[test]
    fn leaves_single_row_images_untouched() {
        let left = bgra_pixel(0x01, 0x02, 0x03, 0xFF);
        let right = bgra_pixel(0x0A, 0x0B, 0x0C, 0xFF);
        let got = to_top_down_rgba(&pixels(&[left, right]), 2, 1).expect("2x1 image");
        assert_eq!(got.dimensions(), (2, 1));
        assert_eq!(&got.as_raw()[..4], &[0x03, 0x02, 0x01, 0xFF]);
        assert_eq!(&got.as_raw()[4..], &[0x0C, 0x0B, 0x0A, 0xFF]);
    }

    #[test]
    fn flips_odd_row_counts_without_panicking() {
        let px = bgra_pixel(0x01, 0x02, 0x03, 0xFF);
        let got = to_top_down_rgba(&pixels(&[px; 3]), 1, 3).expect("1x3 image");
        assert_eq!(got.dimensions(), (1, 3));
        // Every row is identical here, so only the dimensions are asserted —
        // the point is that the uneven split does not panic.
    }

    #[test]
    fn repairs_fully_transparent_alpha_but_keeps_real_alpha() {
        // Every alpha byte is 0 → treated as a legacy opaque icon.
        let opaque =
            to_top_down_rgba(&pixels(&[bgra_pixel(0x10, 0x20, 0x30, 0x00)]), 1, 1).expect("1x1");
        assert_eq!(opaque.as_raw()[3], 0xFF, "zero alpha means opaque");

        // A single opaque pixel keeps the whole image transparent, so the real
        // alpha channel is preserved untouched.
        let mixed = to_top_down_rgba(
            &pixels(&[
                bgra_pixel(0x10, 0x20, 0x30, 0x00),
                bgra_pixel(0x40, 0x50, 0x60, 0x80),
            ]),
            2,
            1,
        )
        .expect("1x2 image");
        assert_eq!(mixed.as_raw()[3], 0x00, "transparent stays transparent");
        assert_eq!(mixed.as_raw()[7], 0x80, "semi-transparent is preserved");
    }

    #[test]
    fn rejects_a_buffer_that_does_not_match_the_dimensions() {
        assert!(to_top_down_rgba(&[0; 4], 2, 1).is_none(), "short buffer");
        assert!(to_top_down_rgba(&[0; 4], 1, 1).is_some(), "exact buffer");
    }

    #[test]
    fn never_upscales_when_the_shell_already_returned_enough() {
        let base = to_top_down_rgba(&pixels(&[bgra_pixel(1, 2, 3, 0xFF); 64]), 8, 8).expect("8x8");
        let same = upscale_if_needed(base.clone(), 8);
        assert_eq!(same.dimensions(), (8, 8));
        let smaller = upscale_if_needed(base, 4);
        assert_eq!(smaller.dimensions(), (8, 8), "no downscale here");
    }

    #[test]
    fn upscales_to_the_requested_size_when_asked_for_more() {
        let base = to_top_down_rgba(&pixels(&[bgra_pixel(1, 2, 3, 0xFF); 16]), 4, 4).expect("4x4");
        let big = upscale_if_needed(base, 24);
        assert_eq!(big.dimensions(), (24, 24));
        // Alpha must survive the resample, otherwise the icon vanishes.
        assert!(
            big.pixels().all(|p| p.0[3] == 0xFF),
            "resample dropped alpha"
        );
    }

    #[test]
    fn icon_data_url_never_invents_an_icon() {
        assert!(icon_data_url(None, 32).is_none());
        assert!(icon_data_url(Some("   "), 32).is_none());
        assert!(icon_data_url(Some(r"C:\definitely\not\here.exe"), 32).is_none());
    }

    #[cfg(windows)]
    #[test]
    fn icon_data_url_is_a_png_data_url() {
        let path = windir_file("explorer.exe");
        let url = icon_data_url(Some(&path), 32).expect("explorer.exe ships an icon");
        let payload = url
            .strip_prefix(PNG_DATA_URL_PREFIX)
            .expect("data url prefix");
        let bytes = BASE64.decode(payload).expect("valid base64");
        assert_eq!(
            &bytes[..8],
            &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A],
            "PNG magic"
        );
        // A 32×32 RGBA icon at PNG's typical compression lands well under
        // 64 KiB; anything larger means we are shipping raw pixels.
        assert!(
            bytes.len() < 64 * 1024,
            "png too large: {} bytes",
            bytes.len()
        );
    }

    #[cfg(windows)]
    #[test]
    fn files_without_an_icon_resource_yield_none() {
        // kernel32.dll and gdi32.dll genuinely ship no `RT_ICON`, so the shell
        // returns 0. That must surface as `None`, not as a blank image.
        for name in ["kernel32.dll", "gdi32.dll"] {
            let path = windir_file(&format!(r"System32\{name}"));
            assert!(
                icon_data_url(Some(&path), 32).is_none(),
                "{name} has no icon and must not fabricate one"
            );
        }
    }

    #[cfg(windows)]
    #[test]
    fn clamps_absurd_requested_sizes() {
        let path = windir_file("explorer.exe");
        // 0 and 8 clamp up to 16; 4096 clamps down to 128. All must render.
        for size in [0, 8, 16, 128, 4096] {
            let url = icon_data_url(Some(&path), size)
                .unwrap_or_else(|| panic!("size {size} should render"));
            assert!(url.starts_with(PNG_DATA_URL_PREFIX));
        }
    }
}
