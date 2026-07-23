//! snackpilot-core: portable logic for SnackPilot v2 (scraping, domain, caching,
//! notification decisions). Behavior traces to v1.4.5 (main @ 6997c44); see
//! docs/requirements/ and docs/architecture/v2-architecture.md.

pub mod broadcast;
pub mod datetime;
pub mod demo;
pub mod domain;
pub mod error;
pub mod features;
pub mod ffi;
pub mod gourmet;
pub mod http;
pub mod notify;
pub mod storage;

uniffi::setup_scaffolding!();
pub mod ventopay;
