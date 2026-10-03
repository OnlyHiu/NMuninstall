//! Walks the three `Uninstall` roots and produces the merged program list.
//!
//! 64-bit and 32-bit registry views are opened separately (F-101); a source
//! that cannot be opened is logged and skipped rather than aborting (F-102).

use std::collections::HashSet;
use std::time::Instant;

use tracing::{debug, trace, warn};
use winreg::enums::KEY_READ;

use crate::models::{Hive, ProgramInfo, RegistryView, ScanStats};
use crate::registry::reader::{read_program, SourceRef};

pub const SOURCES: &[SourceRef] = &[
    SourceRef {
        hive: Hive::HkLocalMachine,
        sub_path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        view: RegistryView::Bit64,
    },
    SourceRef {
        hive: Hive::HkLocalMachine,
        sub_path: r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
        view: RegistryView::Bit32,
    },
    SourceRef {
        hive: Hive::HkCurrentUser,
        sub_path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        view: RegistryView::Bit64,
    },
];

/// Scans every source and returns the merged, filtered, de-duplicated list
/// sorted case-insensitively by display name.
pub fn scan_all(include_system: bool, include_32bit: bool) -> (Vec<ProgramInfo>, ScanStats) {
    let started = Instant::now();
    let mut stats = ScanStats::default();
    let mut out: Vec<ProgramInfo> = Vec::with_capacity(1024);
    let mut seen_ids: HashSet<String> = HashSet::with_capacity(1024);

    for src in SOURCES {
        if src.view == RegistryView::Bit32 && !include_32bit {
            continue;
        }
        match scan_source(src, include_system, &mut seen_ids, &mut stats) {
            Ok(items) => {
                debug!(path = %src.root_path(), count = items.len(), "scanned source");
                out.extend(items);
            }
            Err(e) => {
                warn!(path = %src.root_path(), error = %e, "cannot open uninstall root");
            }
        }
    }

    // F-108: collapse patch/update entries that duplicate the main program.
    let before = out.len();
    dedupe(&mut out);
    stats.deduplicated = before - out.len();

    out.sort_by(|a, b| {
        a.display_name
            .to_lowercase()
            .cmp(&b.display_name.to_lowercase())
            .then_with(|| a.id.cmp(&b.id))
    });

    stats.total = out.len();
    stats.scanned_keys = SOURCES.len();
    trace!(
        count = out.len(),
        skipped_no_name = stats.skipped_no_name,
        skipped_inaccessible = stats.skipped_inaccessible,
        deduplicated = stats.deduplicated,
        elapsed_ms = started.elapsed().as_millis() as u64,
        "scan finished"
    );
    (out, stats)
}

fn scan_source(
    src: &SourceRef,
    include_system: bool,
    seen_ids: &mut HashSet<String>,
    stats: &mut ScanStats,
) -> std::io::Result<Vec<ProgramInfo>> {
    let view_flag = match src.view {
        RegistryView::Bit64 => winreg::enums::KEY_WOW64_64KEY,
        RegistryView::Bit32 => winreg::enums::KEY_WOW64_32KEY,
    };
    let flags = KEY_READ | view_flag;

    let root = src
        .hive
        .predef()
        .open_subkey_with_flags(src.sub_path, flags)?;
    let mut items = Vec::new();
    for name in root.enum_keys().flatten() {
        let sub = match root.open_subkey_with_flags(&name, flags) {
            Ok(k) => k,
            Err(_) => {
                stats.skipped_inaccessible += 1;
                continue;
            }
        };
        match read_program(&sub, src, &name) {
            Some(info) => {
                if info.is_system && !include_system {
                    continue; // F-106
                }
                if !seen_ids.insert(info.id.clone()) {
                    debug!(id = %info.id, "duplicate id, keeping first");
                    continue; // F-109
                }
                items.push(info);
            }
            None => {
                stats.skipped_no_name += 1; // F-104
            }
        }
    }
    Ok(items)
}

/// Identity used for duplicate detection: name + version, case-insensitive,
/// with surrounding whitespace removed.
pub fn dedupe_key(p: &ProgramInfo) -> String {
    format!(
        "{}|{}",
        p.display_name.trim().to_lowercase(),
        p.display_version
            .as_deref()
            .unwrap_or_default()
            .trim()
            .to_lowercase()
    )
}

/// Keeps the entry without `ParentKeyName`, preferring the larger reported size.
///
/// Two entries collapse only when one is a known patch/update of the other:
/// either it carries `ParentKeyName`, or its `install_location` is the directory
/// holding the other's icon (技术文档 §3.1.3 F-108).
pub fn dedupe(programs: &mut Vec<ProgramInfo>) {
    let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
    for (i, p) in programs.iter().enumerate() {
        let key = dedupe_key(p);
        match groups.iter_mut().find(|(k, _)| *k == key) {
            Some((_, idxs)) => idxs.push(i),
            None => groups.push((key, vec![i])),
        }
    }

    let mut drop: HashSet<usize> = HashSet::new();
    for (_, idxs) in &groups {
        if idxs.len() < 2 {
            continue;
        }
        let mut keep = idxs[0];
        for &i in &idxs[1..] {
            if is_patch_of(&programs[i], &programs[keep]) {
                // `i` is an update of the current keeper — drop `i`.
                drop.insert(i);
            } else if is_patch_of(&programs[keep], &programs[i]) {
                // The keeper is the update — promote `i`.
                drop.insert(keep);
                keep = i;
            } else if better_primary(&programs[i], &programs[keep]) {
                drop.insert(keep);
                keep = i;
            } else {
                drop.insert(i);
            }
        }
    }

    if drop.is_empty() {
        return;
    }
    let mut i = 0;
    programs.retain(|_| {
        let keep = !drop.contains(&i);
        i += 1;
        keep
    });
}

fn is_patch_of(child: &ProgramInfo, parent: &ProgramInfo) -> bool {
    if child.parent_key_name.is_none() {
        return false;
    }
    if let (Some(cl), Some(pi)) = (&child.install_location, &parent.icon_path) {
        if let (Some(cl_dir), Some(pi_dir)) = (parent_of(cl), parent_of(pi)) {
            return cl_dir.eq_ignore_ascii_case(&pi_dir);
        }
    }
    // A ParentKeyName alone is a strong enough signal for a patch entry.
    true
}

fn better_primary(candidate: &ProgramInfo, current: &ProgramInfo) -> bool {
    match (
        candidate.parent_key_name.is_none(),
        current.parent_key_name.is_none(),
    ) {
        (true, false) => true,
        (false, true) => false,
        _ => candidate.estimated_size.unwrap_or(0) > current.estimated_size.unwrap_or(0),
    }
}

fn parent_of(path: &str) -> Option<String> {
    let trimmed = path.trim_end_matches(['\\', '/']);
    trimmed
        .rfind(['\\', '/'])
        .map(|idx| trimmed[..idx].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::RegistryView;

    fn program(name: &str, version: Option<&str>, size: Option<u64>) -> ProgramInfo {
        ProgramInfo {
            id: format!("HKLM:64:{name}"),
            key_name: name.to_string(),
            reg_path: format!(r"HKLM\SOFTWARE\...\Uninstall\{name}"),
            display_name: name.to_string(),
            display_version: version.map(str::to_string),
            publisher: None,
            install_date: None,
            raw_install_date: None,
            install_location: None,
            estimated_size: size,
            uninstall_string: Some(r#"C:\a\un.exe"#.into()),
            quiet_uninstall_string: None,
            display_icon: None,
            icon_path: None,
            system_component: false,
            windows_installer: false,
            parent_key_name: None,
            no_modify: false,
            no_repair: false,
            is_system: false,
            hive: Hive::HkLocalMachine,
            view: RegistryView::Bit64,
            is_64bit: true,
            can_uninstall: true,
        }
    }

    #[test]
    fn dedupe_key_is_case_and_space_insensitive() {
        let a = program("App", Some("1.0"), None);
        let mut b = program("App", Some("1.0"), None);
        b.display_name = "  app ".into();
        assert_eq!(dedupe_key(&a), dedupe_key(&b));
    }

    #[test]
    fn different_versions_are_not_deduplicated() {
        let mut v = vec![
            program("App", Some("1.0"), Some(10)),
            program("App", Some("2.0"), Some(10)),
        ];
        dedupe(&mut v);
        assert_eq!(
            v.len(),
            2,
            "a patch with a different version is its own entry"
        );
    }

    #[test]
    fn a_lone_entry_always_survives() {
        let mut v = vec![program("App", Some("1.0"), None)];
        dedupe(&mut v);
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn patch_entry_with_parent_key_name_is_removed() {
        let mut main = program("Real App", Some("1.0"), Some(5000));
        main.parent_key_name = None;
        let mut patch = program("Real App", Some("1.0"), Some(120));
        patch.parent_key_name = Some("Real App".into());
        let mut v = vec![patch, main.clone()];
        dedupe(&mut v);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].estimated_size, Some(5000));
    }

    #[test]
    fn identical_twins_collapse_to_the_larger_one() {
        let mut a = program("Twin", Some("2.0"), Some(100));
        let mut b = program("Twin", Some("2.0"), Some(900));
        a.key_name = "a".into();
        b.key_name = "b".into();
        let mut v = vec![a, b];
        dedupe(&mut v);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].estimated_size, Some(900));
    }

    #[test]
    fn same_name_different_versions_both_survive() {
        let mut v = vec![
            program("App", Some("1.0"), Some(10)),
            program("App", Some("2.0"), Some(10)),
        ];
        dedupe(&mut v);
        assert_eq!(v.len(), 2);
    }

    #[test]
    fn parent_of_handles_trailing_separator() {
        assert_eq!(parent_of(r"C:\a\b\"), Some(r"C:\a".into()));
        assert_eq!(parent_of(r"C:\a\b"), Some(r"C:\a".into()));
        assert_eq!(parent_of(r"C:\"), None);
    }

    #[test]
    fn sources_cover_both_views_and_hkcu() {
        assert_eq!(SOURCES.len(), 3);
        assert!(SOURCES.iter().any(|s| s.view == RegistryView::Bit32));
        assert!(SOURCES
            .iter()
            .any(|s| s.hive == Hive::HkCurrentUser && s.view == RegistryView::Bit64));
    }
}
