//! Feature services — the in-memory state + orchestration v1 kept in Zustand stores
//! (03-features/{menus,orders,billing}.md; docs/architecture §3.4). The shells render
//! snapshots and call operations; no business logic lives in the shells.
pub mod billing;
