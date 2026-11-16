//! Durable, unencrypted key-value store (docs/architecture §3.3; caching.md §1-2).
//! Values are opaque strings; callers own JSON (de)serialization, matching v1 AsyncStorage.
//! Absent key -> Ok(None), never an error (loadCached* no-op contract, caching.md §3.4).
use crate::error::CoreResult;

pub mod cache;
pub mod file_kv;
pub mod memory_kv;
pub use file_kv::FileKv;
pub use memory_kv::MemoryKv;

pub trait Kv: Send + Sync {
    fn get(&self, key: &str) -> CoreResult<Option<String>>;
    fn set(&self, key: &str, value: &str) -> CoreResult<()>;
    fn remove(&self, key: &str) -> CoreResult<()>;

    /// Every key this store can see. Billing history is addressed by month key
    /// (`billing_2026-06`), so a month the app stopped offering is only
    /// discoverable by looking at the keys themselves. Stores that cannot
    /// enumerate return nothing: their callers then simply see the window.
    fn keys(&self) -> CoreResult<Vec<String>> {
        Ok(Vec::new())
    }
}
