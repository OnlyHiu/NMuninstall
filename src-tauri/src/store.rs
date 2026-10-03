//! In-memory cache of the last scan.
//!
//! Commands accept an opaque `id` instead of a path, which removes any
//! "frontend supplies an arbitrary path" attack surface (技术文档 §5.3). The
//! cache is the single source of truth for where a program lives.

use std::collections::HashMap;
use std::sync::RwLock;

use crate::error::{AppError, AppResult};
use crate::models::ProgramInfo;

#[derive(Default)]
pub struct ProgramStore {
    inner: RwLock<HashMap<String, ProgramInfo>>,
}

impl ProgramStore {
    pub fn replace_all(&self, programs: Vec<ProgramInfo>) {
        let mut map = HashMap::with_capacity(programs.len());
        for p in programs {
            map.insert(p.id.clone(), p);
        }
        match self.inner.write() {
            Ok(mut guard) => *guard = map,
            Err(poisoned) => *poisoned.into_inner() = map,
        }
    }

    pub fn get(&self, id: &str) -> AppResult<ProgramInfo> {
        match self.inner.read() {
            Ok(guard) => guard
                .get(id)
                .cloned()
                .ok_or_else(|| AppError::NotFound { id: id.to_string() }),
            Err(poisoned) => poisoned
                .into_inner()
                .get(id)
                .cloned()
                .ok_or_else(|| AppError::NotFound { id: id.to_string() }),
        }
    }

    pub fn remove(&self, id: &str) {
        if let Ok(mut guard) = self.inner.write() {
            guard.remove(id);
        }
    }

    pub fn len(&self) -> usize {
        self.inner.read().map(|g| g.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Hive, RegistryView};

    fn sample(id: &str) -> ProgramInfo {
        ProgramInfo {
            id: id.into(),
            key_name: id.into(),
            reg_path: format!(r"HKLM\SOFTWARE\...\Uninstall\{id}"),
            display_name: id.into(),
            display_version: None,
            publisher: None,
            install_date: None,
            raw_install_date: None,
            install_location: None,
            estimated_size: None,
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
    fn get_returns_not_found_for_unknown_id() {
        let store = ProgramStore::default();
        let err = store.get("HKLM:64:nope").unwrap_err();
        assert_eq!(err.code(), "NOT_FOUND");
    }

    #[test]
    fn replace_all_swaps_contents() {
        let store = ProgramStore::default();
        store.replace_all(vec![sample("a"), sample("b")]);
        assert_eq!(store.len(), 2);
        store.replace_all(vec![sample("c")]);
        assert_eq!(store.len(), 1);
        assert!(store.get("a").is_err());
        assert!(store.get("c").is_ok());
    }

    #[test]
    fn remove_drops_one_entry() {
        let store = ProgramStore::default();
        store.replace_all(vec![sample("a"), sample("b")]);
        store.remove("a");
        assert_eq!(store.len(), 1);
        assert!(store.get("b").is_ok());
    }
}
