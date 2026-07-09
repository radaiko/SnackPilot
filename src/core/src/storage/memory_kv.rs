use crate::error::CoreResult;
use crate::storage::Kv;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Default)]
pub struct MemoryKv {
    map: Mutex<HashMap<String, String>>,
}

impl MemoryKv {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Kv for MemoryKv {
    fn get(&self, key: &str) -> CoreResult<Option<String>> {
        Ok(self.map.lock().unwrap().get(key).cloned())
    }
    fn set(&self, key: &str, value: &str) -> CoreResult<()> {
        self.map
            .lock()
            .unwrap()
            .insert(key.to_string(), value.to_string());
        Ok(())
    }
    fn remove(&self, key: &str) -> CoreResult<()> {
        self.map.lock().unwrap().remove(key);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Kv;

    #[test]
    fn absent_key_returns_none_not_error() {
        let kv = MemoryKv::new();
        assert_eq!(kv.get("missing").unwrap(), None);
    }

    #[test]
    fn set_get_remove_roundtrip() {
        let kv = MemoryKv::new();
        kv.set("k", "{\"a\":1}").unwrap();
        assert_eq!(kv.get("k").unwrap().as_deref(), Some("{\"a\":1}"));
        kv.remove("k").unwrap();
        assert_eq!(kv.get("k").unwrap(), None);
    }

    #[test]
    fn remove_absent_key_is_ok() {
        let kv = MemoryKv::new();
        assert!(kv.remove("never").is_ok());
    }
}
