//! Registry scanning integration test.
//!
//! Runs against the *live* machine, so every assertion is written to be
//! portable: it checks internal consistency rather than a fixed program list.

use std::collections::HashSet;

use nmuninstall_lib::registry::scanner::{dedupe, scan_all, SOURCES};
use nmuninstall_lib::registry::writer::is_uninstall_subkey;

#[test]
fn sources_cover_all_three_roots() {
    assert_eq!(SOURCES.len(), 3);
    assert!(SOURCES
        .iter()
        .any(|s| s.view.is_64bit() && s.hive.as_str() == "HKLM"));
    assert!(SOURCES.iter().any(|s| !s.view.is_64bit()));
    assert!(SOURCES.iter().any(|s| s.hive.as_str() == "HKCU"));
}

#[test]
fn scan_produces_a_consistent_list() {
    let (programs, stats) = scan_all(true, true);

    // Nothing about a specific machine, only self-consistency.
    let mut ids = HashSet::new();
    for p in &programs {
        assert!(
            !p.display_name.trim().is_empty(),
            "F-104: empty name slipped through"
        );
        assert!(ids.insert(p.id.clone()), "F-109: duplicate id {}", p.id);
        assert!(p.id.starts_with(&format!("{}:", p.hive.as_str())));
        assert!(p.is_64bit == p.view.is_64bit());
        assert_eq!(
            p.can_uninstall,
            p.uninstall_string.is_some() || p.quiet_uninstall_string.is_some()
        );
        // A resolved icon must point at a file that exists right now.
        if let Some(icon) = &p.icon_path {
            assert!(
                std::path::Path::new(icon).is_file(),
                "icon path does not exist: {icon}"
            );
        }
    }

    assert_eq!(stats.total, programs.len());
    assert_eq!(stats.scanned_keys, 3);
    assert!(stats.skipped_no_name + programs.len() > 0);
}

#[test]
fn scan_is_fast_enough() {
    let started = std::time::Instant::now();
    let (programs, _) = scan_all(false, true);
    let elapsed = started.elapsed();
    // Generous bound: CI machines and antivirus make this noisy. F-601 is 2s
    // for the whole list on a normal desktop.
    assert!(
        elapsed.as_secs() < 5,
        "scan took {:?} for {} programs",
        elapsed,
        programs.len()
    );
}

#[test]
fn hiding_system_components_never_grows_the_list() {
    let (with_system, _) = scan_all(true, true);
    let (without_system, _) = scan_all(false, true);
    assert!(
        without_system.len() <= with_system.len(),
        "filtering must not add entries"
    );
}

#[test]
fn hiding_32bit_never_grows_the_list() {
    let (with32, _) = scan_all(true, true);
    let (without32, _) = scan_all(true, false);
    assert!(without32.len() <= with32.len());
    assert!(
        without32.iter().all(|p| p.is_64bit),
        "the 32-bit view must be gone"
    );
}

#[test]
fn dedupe_is_idempotent() {
    let (programs, _) = scan_all(true, true);
    let before = programs.len();
    let mut copy = programs.clone();
    dedupe(&mut copy);
    assert_eq!(copy.len(), before, "a deduped list must not shrink again");
}

#[test]
fn every_uninstall_key_path_passes_its_own_validator() {
    let (programs, _) = scan_all(true, true);
    for p in &programs {
        assert!(
            is_uninstall_subkey(&p.reg_path),
            "generated path rejected by the writer guard: {}",
            p.reg_path
        );
    }
}
