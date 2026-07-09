//! Durable, unencrypted key-value store (docs/architecture §3.3; caching.md §1-2).
//! Values are opaque strings; callers own JSON (de)serialization, matching v1 AsyncStorage.
//! Absent key -> Ok(None), never an error (loadCached* no-op contract, caching.md §3.4).
use crate::error::CoreResult;

pub mod file_kv;
pub mod memory_kv;
pub use file_kv::FileKv;
pub use memory_kv::MemoryKv;

pub trait Kv: Send + Sync {
    fn get(&self, key: &str) -> CoreResult<Option<String>>;
    fn set(&self, key: &str, value: &str) -> CoreResult<()>;
    fn remove(&self, key: &str) -> CoreResult<()>;
}
