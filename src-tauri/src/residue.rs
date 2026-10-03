//! Post-uninstall residue detection and cleanup.
//!
//! Deletion is the only destructive thing this app does, so every path is put
//! through [`is_removable_path`] and re-resolved from the scan cache — no path
//! coming from the frontend is ever trusted verbatim.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use tracing::{info, warn};

use crate::error::AppError;
use crate::models::{CleanFailure, CleanResult, ProgramInfo, ResidueItem, ResidueReport};
use crate::registry::writer::key_exists;

/// Directory names that are never acceptable deletion targets.
pub const DENY_NAMES: &[&str] = &[
    "windows",
    "program files",
    "program files (x86)",
    "programdata",
    "users",
    "windows.old",
    "recycler",
    "system volume information",
    "$recycle.bin",
];

/// Roots under which deletion is permitted.
pub const ALLOWED_ROOTS: &[&str] = &[
    r"C:\Program Files",
    r"C:\Program Files (x86)",
    r"C:\ProgramData",
];

const MAX_DEPTH: usize = 6;
const MAX_FILES: u64 = 20_000;

/// The `Uninstall` registry key is treated as removable.
pub fn registry_item_removable(path: &str) -> bool {
    crate::registry::writer::is_uninstall_subkey(path)
}

/// Normalises a path enough for a prefix comparison without requiring the
/// directory to exist.
pub fn canonicalish(path: &str) -> Option<PathBuf> {
    let expanded = crate::icons::expand_env_vars(path.trim().trim_matches('"'))?;
    let p = PathBuf::from(expanded);
    if !p.is_absolute() {
        return None;
    }
    Some(normalise_components(&p))
}

/// Removes `.` / `..` segments and redundant separators. Case is preserved;
/// comparison is done case-insensitively.
fn normalise_components(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in p.components() {
        use std::path::Component::*;
        match comp {
            Prefix(_) | RootDir => out.push(comp.as_os_str()),
            CurDir => {}
            ParentDir => {
                out.pop();
            }
            Normal(seg) => out.push(seg),
        }
    }
    out
}

/// True when `path` is inside one of the allowed roots and none of the
/// components *below* the root is a denied directory name.
pub fn is_removable_path(path: &str) -> bool {
    let Some(resolved) = canonicalish(path) else {
        return false;
    };
    if is_symlink(&resolved) {
        return false; // A junction could point anywhere.
    }
    let lower = resolved.to_string_lossy().to_lowercase();

    // The path must be strictly *inside* an allowed root; a root is never a
    // valid target itself.
    let rest = ALLOWED_ROOTS.iter().find_map(|root| {
        let prefix = format!("{}\\", root.to_lowercase());
        lower
            .strip_prefix(&prefix)
            .filter(|r| !r.is_empty())
            .map(str::to_string)
    });
    let Some(rest) = rest else {
        return false;
    };

    // The deny list is applied below the root only, otherwise
    // `C:\Program Files\App` would be rejected for containing "Program Files".
    !rest.split('\\').any(|seg| DENY_NAMES.contains(&seg))
}

fn is_symlink(p: &Path) -> bool {
    match fs::symlink_metadata(p) {
        Ok(meta) => {
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
                meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
            }
            #[cfg(not(windows))]
            {
                meta.file_type().is_symlink()
            }
        }
        Err(_) => false,
    }
}

/// Recursively counts files and total bytes, bounded in depth and file count.
pub fn dir_stats(path: &Path) -> Option<(u64, u64)> {
    fn walk(dir: &Path, depth: usize, bytes: &mut u64, files: &mut u64) -> bool {
        if depth > MAX_DEPTH || *files >= MAX_FILES {
            return false;
        }
        let Ok(entries) = fs::read_dir(dir) else {
            return true; // Unreadable subdir: stop, but keep what we counted.
        };
        for entry in entries.flatten() {
            let Ok(meta) = entry.metadata() else { continue };
            if *files >= MAX_FILES {
                return false;
            }
            if meta.is_dir() {
                if !walk(&entry.path(), depth + 1, bytes, files) {
                    return false;
                }
            } else {
                *bytes += meta.len();
                *files += 1;
            }
        }
        true
    }
    if !path.is_dir() {
        return None;
    }
    let mut bytes = 0;
    let mut files = 0;
    walk(path, 0, &mut bytes, &mut files);
    Some((bytes, files))
}

/// Builds the residue report for one program (F-301 ~ F-303).
pub fn detect(p: &ProgramInfo) -> ResidueReport {
    let mut registry_keys = Vec::new();
    let mut install_dirs = Vec::new();

    if key_exists(&p.reg_path) {
        registry_keys.push(ResidueItem {
            id: "registry:0".into(),
            kind: "registry".into(),
            path: p.reg_path.clone(),
            size_bytes: None,
            file_count: None,
            removable: registry_item_removable(&p.reg_path),
            reason_if_locked: None,
        });
    }

    if let Some(loc) = p.install_location.as_deref() {
        let path = PathBuf::from(loc);
        if let Some((bytes, files)) = dir_stats(&path) {
            if files > 0 {
                let removable = is_removable_path(loc);
                install_dirs.push(ResidueItem {
                    id: "directory:0".into(),
                    kind: "directory".into(),
                    path: loc.to_string(),
                    size_bytes: Some(bytes),
                    file_count: Some(files),
                    removable,
                    reason_if_locked: (!removable)
                        .then(|| "路径不在允许删除的白名单内，仅可打开查看".to_string()),
                });
            }
        }
    }

    // The registry key is missing when the uninstaller did its job cleanly;
    // that case yields an empty report and the UI says "无残留".
    let cleanable_count = registry_keys
        .iter()
        .chain(install_dirs.iter())
        .filter(|i| i.removable)
        .count();

    ResidueReport {
        registry_keys,
        install_dirs,
        cleanable_count,
    }
}

/// Removes the files and empty directories inside `dir`, but never the
/// directory itself — that is the user's to keep or not.
pub fn remove_dir_contents(dir: &Path) -> Result<u64, AppError> {
    if !dir.is_dir() {
        return Err(AppError::PathNotFound(dir.display().to_string()));
    }
    if !is_removable_path(&dir.to_string_lossy()) {
        return Err(AppError::PathNotAllowed(dir.display().to_string()));
    }
    if is_symlink(dir) {
        return Err(AppError::PathNotAllowed(dir.display().to_string()));
    }

    let mut removed = 0u64;
    let entries =
        fs::read_dir(dir).map_err(|e| AppError::CleanFailed(format!("{}: {e}", dir.display())))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if is_symlink(&path) {
            warn!(path = %path.display(), "skipping reparse point");
            continue;
        }
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(e) => {
                warn!(path = %path.display(), error = %e, "cannot stat residue entry");
                continue;
            }
        };
        if meta.is_dir() {
            // Recurse, then drop the directory if it ended up empty.
            match remove_dir_contents(&path) {
                Ok(n) => removed += n,
                Err(e) => warn!(path = %path.display(), error = %e, "recurse failed"),
            }
            match fs::remove_dir(&path) {
                Ok(()) => removed += 1,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => {
                    warn!(path = %path.display(), error = %e, "directory not empty, kept");
                }
            }
        } else {
            match fs::remove_file(&path) {
                Ok(()) => removed += 1,
                Err(e) => {
                    warn!(path = %path.display(), error = %e, "file delete failed");
                }
            }
        }
    }
    Ok(removed)
}

pub struct CleanPlan {
    pub registry_paths: Vec<String>,
    pub directories: Vec<String>,
}

/// Turns the ids the frontend selected into concrete paths, re-validating
/// every one of them. Unknown ids are ignored rather than fatal.
pub fn plan(p: &ProgramInfo, registry_ids: &[String], dir_ids: &[String]) -> CleanPlan {
    let report = detect(p);
    let pick = |items: &[ResidueItem], wanted: &[String]| -> Vec<String> {
        items
            .iter()
            .filter(|i| wanted.iter().any(|w| w == &i.id))
            .filter(|i| i.removable)
            .map(|i| i.path.clone())
            .collect()
    };
    CleanPlan {
        registry_paths: pick(&report.registry_keys, registry_ids),
        directories: pick(&report.install_dirs, dir_ids),
    }
}

/// Executes a validated plan; partial failures are reported, not fatal.
pub fn clean(plan: &CleanPlan) -> CleanResult {
    let started = Instant::now();
    let mut result = CleanResult::default();

    for path in &plan.registry_paths {
        match crate::registry::writer::delete_subkey_with_audit(path, "residue::clean") {
            Ok(()) => result.removed.push(path.clone()),
            Err(e) => result.failed.push(CleanFailure {
                path: path.clone(),
                error: e.to_string(),
            }),
        }
    }

    for path in &plan.directories {
        match remove_dir_contents(Path::new(path)) {
            Ok(_) => result.removed.push(path.clone()),
            Err(e) => result.failed.push(CleanFailure {
                path: path.clone(),
                error: e.to_string(),
            }),
        }
    }

    info!(
        removed = result.removed.len(),
        failed = result.failed.len(),
        elapsed_ms = started.elapsed().as_millis() as u64,
        "residue cleanup finished"
    );
    result
}

/// Total size and file count across a directory, for the UI summary.
pub fn summarise(report: &ResidueReport) -> (u64, u64) {
    report.install_dirs.iter().fold((0, 0), |(b, f), item| {
        (
            b + item.size_bytes.unwrap_or(0),
            f + item.file_count.unwrap_or(0),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn program_files_subdirectories_are_removable() {
        assert!(is_removable_path(r"C:\Program Files\App"));
        assert!(is_removable_path(r"C:\Program Files (x86)\App\sub"));
        assert!(is_removable_path(r"C:\ProgramData\Vendor"));
    }

    #[test]
    fn system_and_root_paths_are_refused() {
        assert!(!is_removable_path(r"C:\Windows"));
        assert!(!is_removable_path(r"C:\Windows\System32\drivers"));
        assert!(!is_removable_path(r"C:\"));
        assert!(!is_removable_path(r"C:\Program Files"));
        assert!(!is_removable_path(r"C:\Program Files (x86)"));
        assert!(!is_removable_path(r"C:\ProgramData"));
        assert!(!is_removable_path(r"C:\Users\Someone\AppData\Local\App"));
        assert!(!is_removable_path(r"C:\Windows.old\Things"));
    }

    #[test]
    fn traversal_is_normalised_away() {
        // `..` must not let a path climb out of the allowed root.
        assert!(!is_removable_path(
            r"C:\Program Files\App\..\..\Windows\System32"
        ));
        assert!(!is_removable_path(r"C:\Program Files\..\Windows"));
    }

    #[test]
    fn relative_and_empty_paths_are_refused() {
        assert!(!is_removable_path(""));
        assert!(!is_removable_path("Program Files\\App"));
        assert!(!is_removable_path(r"\\server\share\app"));
    }

    #[test]
    fn case_insensitive_root_matching() {
        assert!(is_removable_path(r"c:\program files\app"));
    }

    #[test]
    fn registry_key_is_removable_only_under_uninstall() {
        assert!(registry_item_removable(
            r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\{GUID}"
        ));
        assert!(!registry_item_removable(
            r"HKLM\SYSTEM\CurrentControlSet\Services\Tcpip"
        ));
    }

    #[test]
    fn dir_stats_on_a_real_temp_dir() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), b"hello").unwrap();
        fs::create_dir(dir.path().join("nested")).unwrap();
        fs::write(dir.path().join("nested").join("b.txt"), vec![0u8; 10]).unwrap();
        let (bytes, files) = dir_stats(dir.path()).unwrap();
        assert_eq!(files, 2);
        assert_eq!(bytes, 15);
    }

    #[test]
    fn dir_stats_is_none_for_missing_dirs() {
        assert!(dir_stats(Path::new(r"C:\definitely\not\here")).is_none());
    }

    #[test]
    fn summarise_adds_up_directories() {
        let report = ResidueReport {
            registry_keys: vec![],
            install_dirs: vec![
                ResidueItem {
                    id: "directory:0".into(),
                    kind: "directory".into(),
                    path: r"C:\Program Files\App".into(),
                    size_bytes: Some(100),
                    file_count: Some(3),
                    removable: true,
                    reason_if_locked: None,
                },
                ResidueItem {
                    id: "directory:0".into(),
                    kind: "directory".into(),
                    path: r"C:\ProgramData\App".into(),
                    size_bytes: Some(50),
                    file_count: Some(1),
                    removable: true,
                    reason_if_locked: None,
                },
            ],
            cleanable_count: 2,
        };
        assert_eq!(summarise(&report), (150, 4));
    }
}
