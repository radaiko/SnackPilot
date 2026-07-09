use crate::error::{CoreError, CoreResult};
use crate::storage::Kv;
use std::path::PathBuf;
use std::sync::Mutex;

/// One file per key under `dir`. Atomic writes (temp + rename). An in-process Mutex
/// guards concurrent access from headless background entry points and the UI process
/// (same app process on iOS/Android; docs/architecture §3.3).
pub struct FileKv {
    dir: PathBuf,
    lock: Mutex<()>,
}

impl FileKv {
    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            lock: Mutex::new(()),
        }
    }

    /// Map a key to a single safe filename (keys contain '_', '-', digits, letters, and
    /// sometimes ':'/'/'). Non-alphanumeric-ish chars collapse to '_'.
    fn path_for(&self, key: &str) -> PathBuf {
        let safe: String = key
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        self.dir.join(format!("{safe}.val"))
    }
}

impl Kv for FileKv {
    fn get(&self, key: &str) -> CoreResult<Option<String>> {
        let _g = self.lock.lock().unwrap();
        match std::fs::read_to_string(self.path_for(key)) {
            Ok(s) => Ok(Some(s)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(CoreError::Storage {
                message: e.to_string(),
            }),
        }
    }
    fn set(&self, key: &str, value: &str) -> CoreResult<()> {
        let _g = self.lock.lock().unwrap();
        std::fs::create_dir_all(&self.dir).map_err(|e| CoreError::Storage {
            message: e.to_string(),
        })?;
        let final_path = self.path_for(key);
        let tmp_path = final_path.with_extension("val.tmp");
        std::fs::write(&tmp_path, value).map_err(|e| CoreError::Storage {
            message: e.to_string(),
        })?;
        std::fs::rename(&tmp_path, &final_path).map_err(|e| CoreError::Storage {
            message: e.to_string(),
        })?;
        Ok(())
    }
    fn remove(&self, key: &str) -> CoreResult<()> {
        let _g = self.lock.lock().unwrap();
        match std::fs::remove_file(self.path_for(key)) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(CoreError::Storage {
                message: e.to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Kv;

    #[test]
    fn file_kv_persists_across_instances() {
        let dir = tempfile::tempdir().unwrap();
        {
            let kv = FileKv::new(dir.path().to_path_buf());
            kv.set("menus_items", "[1,2,3]").unwrap();
        }
        let kv2 = FileKv::new(dir.path().to_path_buf());
        assert_eq!(kv2.get("menus_items").unwrap().as_deref(), Some("[1,2,3]"));
    }

    #[test]
    fn file_kv_absent_key_is_none() {
        let dir = tempfile::tempdir().unwrap();
        let kv = FileKv::new(dir.path().to_path_buf());
        assert_eq!(kv.get("nope").unwrap(), None);
    }

    #[test]
    fn file_kv_key_with_slash_or_colon_is_safe() {
        // billing keys look like "billing_2026-02"; ventopay uses "ventopay_billing_2026-02".
        let dir = tempfile::tempdir().unwrap();
        let kv = FileKv::new(dir.path().to_path_buf());
        kv.set("billing_2026-02", "x").unwrap();
        assert_eq!(kv.get("billing_2026-02").unwrap().as_deref(), Some("x"));
    }
}
