# SnackPilot v2 Rust Core — Foundation (Phase 1a) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the foundational layer of `snackpilot-core` — the Rust crate skeleton, all portable domain records/enums, the `CoreError` type, the `datetime` module (Vienna-time math + every wire date format + a `Clock` trait), the `http::Transport` trait with its reqwest implementation, capturing test fake, and the Ventopay cookie-jar helper, and the `storage::kv` persistent key-value store — so that every later Rust core sub-plan (Gourmet scraping, Ventopay scraping, feature services, notify/demo, UniFFI facade) can build on stable, tested interfaces.

**Architecture:** A single Rust library crate `snackpilot-core` under `src/core/`, split into focused modules (`domain`, `datetime`, `http`, `storage`). Everything I/O-shaped sits behind an injected trait (`Transport`, `Kv`, `Clock`) so tests run with in-memory fakes and a fixed clock — a direct port of v1's record & replay strategy (docs/requirements/06-testing.md). No UniFFI exports yet; this is pure Rust with `cargo test` as the only gate. The 13 sanitized fixtures are mirrored from `docs/fixtures/` into `src/core/tests/fixtures/` and kept byte-identical.

**Tech Stack:** Rust 2021 edition, `reqwest` (blocking-free async via `tokio`), `scraper` (added later, not this plan), `chrono` + `chrono-tz` (Europe/Vienna), `serde` + `serde_json`, `thiserror`. Test-only: `tokio` test runtime.

## Global Constraints

- **Baseline:** all behavior traces to SnackPilot v1.4.5 (`main` @ 6997c44); requirements live in `docs/requirements/`, architecture in `docs/architecture/v2-architecture.md`. The requirements doc always wins on behavioral detail.
- **Crate location:** `src/core/` on the `v2` branch (worktree `/Users/radaiko/dev/private/SnackPilot-v2`). All paths below are relative to that worktree root.
- **Min toolchain:** Rust stable ≥ 1.78. Edition 2021.
- **No account-ban-sensitive request behavior in this plan** — the HTTP layer here is generic transport; the exact Gourmet/Ventopay request sequences are built on top of it in later plans. But the transport MUST support: a per-instance cookie store, an explicit `Accept: application/json, text/plain, */*` header on every request, **no** `User-Agent` header, redirect limit 5, and status 200–399 = success (docs/architecture §3.1; 01-gourmet-scraping §2, 02-ventopay-scraping §2.1).
- **Dates across module boundaries** are local-date keys `"YYYY-MM-DD"`; timestamps are epoch milliseconds `i64` (docs/architecture §4). All timezone math uses `Europe/Vienna` where the requirement says so (cutoffs) and device-local otherwise (upcoming/past split) — the two are deliberately different (orders.md §7 open question).
- **`localDateKey` is NOT `toISOString()`** — it is the local Y/M/D zero-padded, no UTC conversion (menus.md §1; v1: dateUtils.ts:45-50).
- **Fixtures are hand-authored synthetic replicas with sentinel values** asserted verbatim — never regenerate them (06-testing §2).
- Commit after every green step. Conventional-commit messages, scoped `core`.

---

## File structure (this plan)

```
src/core/
├── Cargo.toml                     # crate manifest, deps
├── build.rs                       # (Task 1) fixture-mirror freshness guard
├── src/
│   ├── lib.rs                     # module declarations only
│   ├── domain/
│   │   ├── mod.rs                 # re-exports
│   │   ├── menu.rs                # MenuItem, MenuCategory, MenuSnapshot, OrderProgress
│   │   ├── order.rs               # OrderedMenu, OrdersSplit
│   │   ├── billing.rs             # Bill, BillingItem, *MonthlyBilling, MonthOption, VentopayTransaction
│   │   └── user.rs                # GourmetUserInfo, Credentials
│   ├── error.rs                   # CoreError
│   ├── datetime/
│   │   ├── mod.rs                 # Clock trait, FixedClock, SystemClock, re-exports
│   │   └── formats.rs             # all wire date formats + localDateKey + cutoff
│   ├── http/
│   │   ├── mod.rs                 # Transport trait, Request, HttpResponse, Method
│   │   ├── reqwest_transport.rs   # production Transport on reqwest
│   │   ├── fake.rs                # CapturingTransport test double
│   │   └── cookie_jar.rs          # Ventopay insertion-order cookie jar
│   └── storage/
│       ├── mod.rs                 # Kv trait, re-exports
│       ├── file_kv.rs             # FileKv (JSON file per key, atomic write, in-proc lock)
│       └── memory_kv.rs           # MemoryKv test double
└── tests/
    └── fixtures/                  # byte-identical mirror of docs/fixtures/ (13 files)
```

---

### Task 1: Crate skeleton + fixture mirror

**Files:**
- Create: `src/core/Cargo.toml`, `src/core/src/lib.rs`, `src/core/build.rs`
- Create: `src/core/tests/fixtures/` (copied from `docs/fixtures/`)
- Create: `src/core/rust-toolchain.toml`

**Interfaces:**
- Produces: a compiling empty crate `snackpilot_core`; a build-time guard that fails if `src/core/tests/fixtures/` drifts from `docs/fixtures/`.

- [ ] **Step 1: Create the crate manifest**

`src/core/Cargo.toml`:

```toml
[package]
name = "snackpilot-core"
version = "2.0.0"
edition = "2021"
rust-version = "1.78"

[lib]
name = "snackpilot_core"
crate-type = ["lib"]

[dependencies]
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "cookies", "multipart"] }
tokio = { version = "1", features = ["sync", "rt", "macros"] }
chrono = { version = "0.4", default-features = false, features = ["clock", "std"] }
chrono-tz = "0.9"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"

[dev-dependencies]
tokio = { version = "1", features = ["rt", "macros", "rt-multi-thread"] }
tempfile = "3"
```

- [ ] **Step 2: Create the toolchain pin and lib root**

`src/core/rust-toolchain.toml`:

```toml
[toolchain]
channel = "stable"
```

`src/core/src/lib.rs`:

```rust
//! snackpilot-core: portable logic for SnackPilot v2 (scraping, domain, caching,
//! notification decisions). Behavior traces to v1.4.5 (main @ 6997c44); see
//! docs/requirements/ and docs/architecture/v2-architecture.md.

pub mod datetime;
pub mod domain;
pub mod error;
pub mod http;
pub mod storage;
```

Create empty module stubs so it compiles (these are replaced by later tasks):
`src/core/src/error.rs`, `src/core/src/domain/mod.rs`, `src/core/src/datetime/mod.rs`, `src/core/src/http/mod.rs`, `src/core/src/storage/mod.rs` — each containing only a doc comment line `//! placeholder` for now.

- [ ] **Step 3: Mirror the fixtures**

Run:
```bash
cd /Users/radaiko/dev/private/SnackPilot-v2
mkdir -p src/core/tests/fixtures
cp -R docs/fixtures/gourmet src/core/tests/fixtures/gourmet
cp -R docs/fixtures/ventopay src/core/tests/fixtures/ventopay
find src/core/tests/fixtures -type f | wc -l
```
Expected: `13`

- [ ] **Step 4: Add the fixture-drift build guard**

`src/core/build.rs`:

```rust
//! Fails the build if the test fixtures drift from the canonical docs/fixtures copies.
use std::path::Path;

fn main() {
    // docs/fixtures lives two levels up from src/core.
    let docs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/fixtures");
    let tests = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    println!("cargo:rerun-if-changed={}", docs.display());
    println!("cargo:rerun-if-changed={}", tests.display());
    if let Err(e) = compare_dirs(&docs, &tests) {
        panic!("fixture mirror out of sync: {e}\nRe-copy docs/fixtures into src/core/tests/fixtures");
    }
}

fn compare_dirs(a: &Path, b: &Path) -> Result<(), String> {
    for sub in ["gourmet", "ventopay"] {
        let (da, db) = (a.join(sub), b.join(sub));
        let mut names: Vec<_> = std::fs::read_dir(&da)
            .map_err(|e| format!("read {}: {e}", da.display()))?
            .filter_map(|e| e.ok().map(|e| e.file_name()))
            .collect();
        names.sort();
        for name in names {
            let fa = std::fs::read(da.join(&name)).map_err(|e| e.to_string())?;
            let fb = std::fs::read(db.join(&name))
                .map_err(|_| format!("missing mirror {}/{:?}", sub, name))?;
            if fa != fb {
                return Err(format!("byte mismatch in {}/{:?}", sub, name));
            }
        }
    }
    Ok(())
}
```

- [ ] **Step 5: Verify it compiles and the guard passes**

Run: `cd src/core && cargo build`
Expected: `Finished` with no errors; build script runs without panic.

- [ ] **Step 6: Commit**

```bash
git add src/core
git commit -m "feat(core): crate skeleton, deps, and fixture-mirror build guard"
```

---

### Task 2: Domain records & enums

**Files:**
- Create: `src/core/src/domain/menu.rs`, `order.rs`, `billing.rs`, `user.rs`
- Modify: `src/core/src/domain/mod.rs`
- Test: inline `#[cfg(test)]` in `menu.rs`

**Interfaces:**
- Produces:
  - `MenuCategory` enum `{ Menu1, Menu2, Menu3, SoupAndSalad, Unknown }` with `fn display(&self) -> &'static str`
  - `MenuItem { id, day, title, subtitle, allergens: Vec<String>, available, ordered, category: MenuCategory, price }` (all `String` except the two `bool`s and `category`; `day` is a `"YYYY-MM-DD"` key)
  - `OrderProgress { Adding, Confirming, Cancelling, Refreshing }`
  - `MenuSnapshot { items, available_dates, pending_orders, pending_cancellations, loading, refreshing, error: Option<String> }`
  - `OrderedMenu { position_id, eating_cycle_id, date_epoch_ms: i64, title, subtitle, approved: bool }`
  - `OrdersSplit { upcoming: Vec<OrderedMenu>, past: Vec<OrderedMenu> }`
  - `BillingItem`, `Bill`, `GourmetMonthlyBilling`, `VentopayTransaction`, `VentopayMonthlyBilling`, `MonthOption` (fields per docs/architecture §4.2)
  - `GourmetUserInfo { username, shop_model_id, eater_id, staff_group_id }`, `Credentials { username, password }`
  - All derive `Debug, Clone, PartialEq`; `serde::{Serialize, Deserialize}` where they are cached (see notes).

- [ ] **Step 1: Write the failing test for category display strings**

Append to `src/core/src/domain/menu.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_display_strings_match_v1() {
        // 03-features/menus.md §1 — exact display strings owned by the core.
        assert_eq!(MenuCategory::Menu1.display(), "MENÜ I");
        assert_eq!(MenuCategory::Menu2.display(), "MENÜ II");
        assert_eq!(MenuCategory::Menu3.display(), "MENÜ III");
        assert_eq!(MenuCategory::SoupAndSalad.display(), "SUPPE & SALAT");
        assert_eq!(MenuCategory::Unknown.display(), "UNKNOWN");
    }
}
```

- [ ] **Step 2: Run it, expect failure**

Run: `cd src/core && cargo test category_display_strings_match_v1`
Expected: FAIL — `MenuCategory` not found / method missing.

- [ ] **Step 3: Implement the domain types**

`src/core/src/domain/menu.rs` (above the test module):

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MenuCategory { Menu1, Menu2, Menu3, SoupAndSalad, Unknown }

impl MenuCategory {
    /// Exact user-visible category label (03-features/menus.md §1).
    pub fn display(&self) -> &'static str {
        match self {
            MenuCategory::Menu1 => "MENÜ I",
            MenuCategory::Menu2 => "MENÜ II",
            MenuCategory::Menu3 => "MENÜ III",
            MenuCategory::SoupAndSalad => "SUPPE & SALAT",
            MenuCategory::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MenuItem {
    pub id: String,
    /// Local-date key "YYYY-MM-DD" (menus.md §1).
    pub day: String,
    pub title: String,
    pub subtitle: String,
    pub allergens: Vec<String>,
    pub available: bool,
    pub ordered: bool,
    pub category: MenuCategory,
    pub price: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderProgress { Adding, Confirming, Cancelling, Refreshing }

#[derive(Debug, Clone, PartialEq)]
pub struct MenuSnapshot {
    pub items: Vec<MenuItem>,
    pub available_dates: Vec<String>,
    pub pending_orders: Vec<String>,
    pub pending_cancellations: Vec<String>,
    pub loading: bool,
    pub refreshing: bool,
    pub error: Option<String>,
}
```

`src/core/src/domain/order.rs`:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct OrderedMenu {
    pub position_id: String,
    pub eating_cycle_id: String,
    pub date_epoch_ms: i64,
    pub title: String,
    pub subtitle: String,
    pub approved: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrdersSplit {
    pub upcoming: Vec<OrderedMenu>,
    pub past: Vec<OrderedMenu>,
}
```

`src/core/src/domain/billing.rs`:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct BillingItem {
    pub id: String, pub article_id: String, pub count: i64, pub description: String,
    pub total: f64, pub subsidy: f64, pub discount_value: f64, pub is_custom_menu: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Bill {
    pub bill_nr: i64, pub bill_date_epoch_ms: i64, pub location: String,
    pub items: Vec<BillingItem>, pub billing: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GourmetMonthlyBilling {
    pub month_key: String, pub label: String, pub bills: Vec<Bill>,
    pub total_gross: f64, pub total_subsidy: f64, pub total_discount: f64,
    pub total_billing: f64, pub fetched_at: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VentopayTransaction {
    pub id: String, pub date_epoch_ms: i64, pub amount: f64,
    pub restaurant: String, pub location: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VentopayMonthlyBilling {
    pub month_key: String, pub label: String,
    pub transactions: Vec<VentopayTransaction>, pub total: f64, pub fetched_at: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MonthOption { pub key: String, pub label: String, pub offset: u8 }
```

`src/core/src/domain/user.rs`:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct GourmetUserInfo {
    pub username: String, pub shop_model_id: String,
    pub eater_id: String, pub staff_group_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Credentials { pub username: String, pub password: String }
```

`src/core/src/domain/mod.rs`:

```rust
//! Portable domain records and enums (docs/architecture §4.2).
pub mod billing;
pub mod menu;
pub mod order;
pub mod user;

pub use billing::*;
pub use menu::*;
pub use order::*;
pub use user::*;
```

- [ ] **Step 4: Run the test, expect pass**

Run: `cd src/core && cargo test category_display_strings_match_v1`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/core/src/domain
git commit -m "feat(core): domain records and enums with exact category display strings"
```

---

### Task 3: `CoreError`

**Files:**
- Create/replace: `src/core/src/error.rs`
- Test: inline `#[cfg(test)]`

**Interfaces:**
- Produces: `pub enum CoreError` with variants `LoginFailed { message }`, `SessionExpired`, `NotLoggedIn`, `AddToCartFailed { message }`, `EditModeFailed`, `Parse { message }`, `Http { message }`, `Storage { message }`; `impl std::fmt::Display` yielding v1's exact user-facing strings; `pub type CoreResult<T> = Result<T, CoreError>`.

- [ ] **Step 1: Write the failing test for the verbatim messages**

`src/core/src/error.rs` test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_strings_are_verbatim_v1() {
        // 01-gourmet-scraping §14, §7; orders.md §8
        assert_eq!(CoreError::SessionExpired.to_string(), "Session expired");
        assert_eq!(CoreError::NotLoggedIn.to_string(), "Not logged in");
        assert_eq!(CoreError::EditModeFailed.to_string(), "Failed to enter edit mode");
        assert_eq!(
            CoreError::LoginFailed { message: "Login failed: invalid credentials or account blocked".into() }.to_string(),
            "Login failed: invalid credentials or account blocked"
        );
        assert_eq!(
            CoreError::AddToCartFailed { message: "boom".into() }.to_string(),
            "Add to cart failed: boom"
        );
    }
}
```

- [ ] **Step 2: Run it, expect failure**

Run: `cd src/core && cargo test display_strings_are_verbatim_v1`
Expected: FAIL — `CoreError` missing.

- [ ] **Step 3: Implement `CoreError`**

`src/core/src/error.rs` (above the test):

```rust
//! Core error type. Variants preserve v1's exact user-facing messages where the shell
//! displays them (01-gourmet-scraping §14, 02-ventopay-scraping §3-4, orders.md §8).
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CoreError {
    /// LoginFailed carries the full message verbatim (Gourmet or Ventopay variant text).
    #[error("{message}")]
    LoginFailed { message: String },
    #[error("Session expired")]
    SessionExpired,
    #[error("Not logged in")]
    NotLoggedIn,
    #[error("Add to cart failed: {message}")]
    AddToCartFailed { message: String },
    #[error("Failed to enter edit mode")]
    EditModeFailed,
    /// Parser errors, incl. missing-token messages, carried verbatim.
    #[error("{message}")]
    Parse { message: String },
    /// Transport-level failure or HTTP status >= 400.
    #[error("{message}")]
    Http { message: String },
    #[error("{message}")]
    Storage { message: String },
}

pub type CoreResult<T> = Result<T, CoreError>;
```

- [ ] **Step 4: Run the test, expect pass**

Run: `cd src/core && cargo test display_strings_are_verbatim_v1`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/core/src/error.rs
git commit -m "feat(core): CoreError with verbatim v1 user-facing messages"
```

---

### Task 4: `datetime` — Clock, formats, cutoff

**Files:**
- Create/replace: `src/core/src/datetime/mod.rs`, `src/core/src/datetime/formats.rs`
- Test: inline `#[cfg(test)]` in `formats.rs`

**Interfaces:**
- Consumes: nothing (leaf module).
- Produces:
  - `pub trait Clock: Send + Sync { fn now_epoch_ms(&self) -> i64; }`
  - `pub struct SystemClock;` (impl via `chrono::Utc::now`) and `pub struct FixedClock { pub epoch_ms: i64 }` for tests.
  - `pub fn local_date_key(epoch_ms: i64) -> String` — device-local Y/M/D `"YYYY-MM-DD"`.
  - `pub fn vienna_date_key(epoch_ms: i64) -> String` — Europe/Vienna Y/M/D.
  - `pub fn format_menu_date(date_key: &str) -> String` — `"YYYY-MM-DD"` → `"MM-dd-yyyy"` (menu `data-date` / AddToMenuesCart).
  - `pub fn parse_menu_date(mmddyyyy: &str) -> Option<String>` — `"MM-dd-yyyy"` → `"YYYY-MM-DD"` key.
  - `pub fn parse_orders_date(s: &str) -> Option<i64>` — `"dd.MM.yyyy HH:mm:ss"` (missing time → `00:00:00`) → local-midnight-based epoch ms.
  - `pub fn format_ventopay_date(date_key: &str) -> String` — `"YYYY-MM-DD"` → `"dd.MM.yyyy"`.
  - `pub fn parse_bill_date(s: &str) -> Option<i64>` — ISO-like `"2026-02-10T12:00:00"` (no tz) as local → epoch ms.
  - `pub fn is_ordering_cutoff(clock: &dyn Clock, date_key: &str) -> bool` — Vienna rule: date before Vienna-today → true; date == Vienna-today → `true` iff current Vienna time ≥ 09:00; future → false (orders.md §4.2 / menus.md §6.2).

- [ ] **Step 1: Write failing tests for the formats and cutoff**

`src/core/src/datetime/formats.rs` test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::datetime::{Clock, FixedClock};

    // Epoch for 2026-02-10 08:00:00 Europe/Vienna == 07:00 UTC == 1770707... compute via chrono.
    fn vienna(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> i64 {
        use chrono::TimeZone;
        chrono_tz::Europe::Vienna
            .with_ymd_and_hms(y, mo, d, h, mi, 0)
            .single().unwrap().timestamp_millis()
    }

    #[test]
    fn menu_date_roundtrip() {
        // MM-dd-yyyy is the Gourmet wire format (01 §12).
        assert_eq!(format_menu_date("2026-02-10"), "02-10-2026");
        assert_eq!(parse_menu_date("02-10-2026").as_deref(), Some("2026-02-10"));
        assert_eq!(parse_menu_date("garbage"), None);
    }

    #[test]
    fn ventopay_date_format() {
        assert_eq!(format_ventopay_date("2026-02-28"), "28.02.2026");
    }

    #[test]
    fn orders_date_defaults_missing_time_to_midnight() {
        // dd.MM.yyyy HH:mm:ss; missing time → 00:00:00 (01 §12).
        let with_time = parse_orders_date("10.02.2026 09:30:00").unwrap();
        let no_time = parse_orders_date("10.02.2026").unwrap();
        assert_eq!(no_time, vienna_like_local("2026-02-10", 0, 0));
        assert!(with_time > no_time);
    }

    #[test]
    fn cutoff_before_today_is_blocked() {
        let clock = FixedClock { epoch_ms: vienna(2026, 2, 10, 8, 0) };
        assert!(is_ordering_cutoff(&clock, "2026-02-09")); // yesterday
    }

    #[test]
    fn cutoff_today_depends_on_0900_vienna() {
        let before = FixedClock { epoch_ms: vienna(2026, 2, 10, 8, 59) };
        let after = FixedClock { epoch_ms: vienna(2026, 2, 10, 9, 0) };
        assert!(!is_ordering_cutoff(&before, "2026-02-10")); // before 09:00 → open
        assert!(is_ordering_cutoff(&after, "2026-02-10"));   // at/after 09:00 → blocked
    }

    #[test]
    fn cutoff_future_never_blocked() {
        let clock = FixedClock { epoch_ms: vienna(2026, 2, 10, 23, 0) };
        assert!(!is_ordering_cutoff(&clock, "2026-02-11"));
    }

    // local-midnight epoch for a YYYY-MM-DD in device-local tz + h:m offset.
    fn vienna_like_local(key: &str, h: u32, mi: u32) -> i64 {
        use chrono::{NaiveDate, NaiveTime, Local, TimeZone};
        let d = NaiveDate::parse_from_str(key, "%Y-%m-%d").unwrap();
        let t = NaiveTime::from_hms_opt(h, mi, 0).unwrap();
        Local.from_local_datetime(&d.and_time(t)).single().unwrap().timestamp_millis()
    }
}
```

> Note for the implementer: `parse_orders_date` produces a **local**-timezone epoch (v1 parses to a local `Date`, dateUtils.ts). The test's `vienna_like_local` uses `chrono::Local`, matching that. Cutoff math, by contrast, is explicitly `Europe/Vienna` regardless of device tz.

- [ ] **Step 2: Run tests, expect failure**

Run: `cd src/core && cargo test --lib datetime`
Expected: FAIL — functions not defined.

- [ ] **Step 3: Implement the Clock trait and clocks**

`src/core/src/datetime/mod.rs`:

```rust
//! Time and date-format helpers. Vienna-time cutoffs, all Gourmet/Ventopay wire formats,
//! and a Clock trait for deterministic tests (docs/architecture §3, 01 §12, orders.md §4.2).
pub mod formats;
pub use formats::*;

/// Injected clock so cutoff/notification logic is deterministic in tests (06-testing §9.3).
pub trait Clock: Send + Sync {
    fn now_epoch_ms(&self) -> i64;
}

pub struct SystemClock;
impl Clock for SystemClock {
    fn now_epoch_ms(&self) -> i64 {
        chrono::Utc::now().timestamp_millis()
    }
}

pub struct FixedClock { pub epoch_ms: i64 }
impl Clock for FixedClock {
    fn now_epoch_ms(&self) -> i64 { self.epoch_ms }
}
```

- [ ] **Step 4: Implement the formats and cutoff**

`src/core/src/datetime/formats.rs` (above the test module):

```rust
use crate::datetime::Clock;
use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Timelike};
use chrono_tz::Europe::Vienna;

/// Device-local Y/M/D for an epoch, "YYYY-MM-DD" (menus.md §1 — NOT toISOString).
pub fn local_date_key(epoch_ms: i64) -> String {
    let dt = chrono::Local.timestamp_millis_opt(epoch_ms).single().expect("valid epoch");
    format!("{:04}-{:02}-{:02}", dt.year(), dt.month(), dt.day())
}

/// Europe/Vienna Y/M/D for an epoch, "YYYY-MM-DD".
pub fn vienna_date_key(epoch_ms: i64) -> String {
    let dt = Vienna.timestamp_millis_opt(epoch_ms).single().expect("valid epoch");
    format!("{:04}-{:02}-{:02}", dt.year(), dt.month(), dt.day())
}

/// "YYYY-MM-DD" -> "MM-dd-yyyy" (Gourmet wire, 01 §12).
pub fn format_menu_date(date_key: &str) -> String {
    match NaiveDate::parse_from_str(date_key, "%Y-%m-%d") {
        Ok(d) => format!("{:02}-{:02}-{:04}", d.month(), d.day(), d.year()),
        Err(_) => String::new(),
    }
}

/// "MM-dd-yyyy" -> "YYYY-MM-DD" (None if malformed).
pub fn parse_menu_date(mmddyyyy: &str) -> Option<String> {
    let d = NaiveDate::parse_from_str(mmddyyyy, "%m-%d-%Y").ok()?;
    Some(format!("{:04}-{:02}-{:02}", d.year(), d.month(), d.day()))
}

/// "dd.MM.yyyy HH:mm:ss" (missing time -> 00:00:00) -> local epoch ms (01 §12).
pub fn parse_orders_date(s: &str) -> Option<i64> {
    let s = s.trim();
    let (date_part, time_part) = match s.split_once(' ') {
        Some((d, t)) => (d, t),
        None => (s, "00:00:00"),
    };
    let d = NaiveDate::parse_from_str(date_part, "%d.%m.%Y").ok()?;
    let t = NaiveTime::parse_from_str(time_part, "%H:%M:%S").ok()?;
    local_epoch_ms(d.and_time(t))
}

/// "YYYY-MM-DD" -> "dd.MM.yyyy" (Ventopay wire, 02 §5).
pub fn format_ventopay_date(date_key: &str) -> String {
    match NaiveDate::parse_from_str(date_key, "%Y-%m-%d") {
        Ok(d) => format!("{:02}.{:02}.{:04}", d.day(), d.month(), d.year()),
        Err(_) => String::new(),
    }
}

/// ISO-like "2026-02-10T12:00:00" (no tz) parsed as local -> epoch ms (01 §12 BillDate).
pub fn parse_bill_date(s: &str) -> Option<i64> {
    let ndt = NaiveDateTime::parse_from_str(s.trim(), "%Y-%m-%dT%H:%M:%S").ok()?;
    local_epoch_ms(ndt)
}

/// Ordering/cancellation cutoff (Europe/Vienna): past day blocked; today blocked iff
/// Vienna time >= 09:00; future never (orders.md §4.2 / menus.md §6.2).
pub fn is_ordering_cutoff(clock: &dyn Clock, date_key: &str) -> bool {
    let now = clock.now_epoch_ms();
    let vienna_now = Vienna.timestamp_millis_opt(now).single().expect("valid epoch");
    let today = vienna_now.date_naive();
    let target = match NaiveDate::parse_from_str(date_key, "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => return false,
    };
    if target < today { return true; }
    if target > today { return false; }
    // target == today: blocked once the clock hits 09:00 Vienna.
    vienna_now.hour() >= 9
}

fn local_epoch_ms(ndt: NaiveDateTime) -> Option<i64> {
    chrono::Local.from_local_datetime(&ndt).single().map(|dt| dt.timestamp_millis())
}
```

- [ ] **Step 5: Run tests, expect pass**

Run: `cd src/core && cargo test --lib datetime`
Expected: all datetime tests PASS.

- [ ] **Step 6: Commit**

```bash
git add src/core/src/datetime
git commit -m "feat(core): datetime — Clock, wire date formats, Vienna 09:00 cutoff"
```

---

### Task 5: `http` — Transport trait, request/response types, capturing fake

**Files:**
- Create/replace: `src/core/src/http/mod.rs`
- Create: `src/core/src/http/fake.rs`
- Test: inline `#[cfg(test)]` in `fake.rs`

**Interfaces:**
- Consumes: `CoreError` (Task 3).
- Produces:
  - `pub enum Method { Get, Post }`
  - `pub struct Request { pub method: Method, pub url: String, pub headers: Vec<(String, String)>, pub body: Option<RequestBody> }`
  - `pub enum RequestBody { Multipart(Vec<(String, String)>), Form(Vec<(String, String)>), Json(String) }` (field order preserved — a `Vec`, never a map)
  - `pub struct HttpResponse { pub status: u16, pub headers: Vec<(String, String)>, pub body: String }`
  - `#[async_trait-free]` trait: `pub trait Transport: Send + Sync { fn send(&self, req: Request) -> impl std::future::Future<Output = CoreResult<HttpResponse>> + Send; }` — implemented with `-> Pin<Box<dyn Future...>>` if RPITIT proves awkward; see note.
  - `pub struct CapturingTransport` — records every `Request` in order, returns queued `HttpResponse`s; `fn queue_response(&self, resp: HttpResponse)`, `fn requests(&self) -> Vec<Request>`.

> Implementer note on the trait: to keep the fake and reqwest impl object-safe behind `dyn Transport`, define the method as returning a boxed future:
> `fn send<'a>(&'a self, req: Request) -> Pin<Box<dyn Future<Output = CoreResult<HttpResponse>> + Send + 'a>>;`
> This is the pattern used throughout the core; do NOT use the `async-trait` crate (keep deps minimal).

- [ ] **Step 1: Write the failing test for the capturing fake**

`src/core/src/http/fake.rs` test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::{Method, Request, RequestBody, HttpResponse, Transport};

    #[tokio::test]
    async fn capturing_transport_records_requests_and_returns_queued_bodies() {
        let t = CapturingTransport::new();
        t.queue_response(HttpResponse { status: 200, headers: vec![], body: "OK".into() });

        let req = Request {
            method: Method::Post,
            url: "https://example.test/x".into(),
            headers: vec![("Accept".into(), "application/json, text/plain, */*".into())],
            body: Some(RequestBody::Multipart(vec![
                ("Username".into(), "u".into()),
                ("Password".into(), "p".into()),
            ])),
        };
        let resp = t.send(req).await.unwrap();
        assert_eq!(resp.body, "OK");

        let captured = t.requests();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].url, "https://example.test/x");
        // field order preserved
        match &captured[0].body {
            Some(RequestBody::Multipart(fields)) => {
                assert_eq!(fields[0].0, "Username");
                assert_eq!(fields[1].0, "Password");
            }
            _ => panic!("expected multipart"),
        }
    }

    #[tokio::test]
    async fn capturing_transport_errors_when_queue_empty() {
        let t = CapturingTransport::new();
        let req = Request { method: Method::Get, url: "u".into(), headers: vec![], body: None };
        assert!(t.send(req).await.is_err());
    }
}
```

- [ ] **Step 2: Run it, expect failure**

Run: `cd src/core && cargo test --lib http`
Expected: FAIL — types not defined.

- [ ] **Step 3: Implement the http types and trait**

`src/core/src/http/mod.rs`:

```rust
//! HTTP transport abstraction. All request-shape guarantees live in the gourmet/ventopay
//! clients ABOVE this trait, so tests pin exact bytes with a fake (docs/architecture §3.1, §9.2).
use crate::error::CoreResult;
use std::future::Future;
use std::pin::Pin;

pub mod fake;
pub mod cookie_jar;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method { Get, Post }

#[derive(Debug, Clone, PartialEq)]
pub enum RequestBody {
    /// multipart/form-data — field order preserved (Gourmet, 01 §2.2).
    Multipart(Vec<(String, String)>),
    /// application/x-www-form-urlencoded — insertion order (Ventopay, 02 §2.4).
    Form(Vec<(String, String)>),
    /// application/json — pre-serialized body.
    Json(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub method: Method,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<RequestBody>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

/// Object-safe async transport. Implementations: reqwest (production), CapturingTransport
/// (tests). Boxed future keeps `dyn Transport` usable without the async-trait crate.
pub trait Transport: Send + Sync {
    fn send<'a>(
        &'a self,
        req: Request,
    ) -> Pin<Box<dyn Future<Output = CoreResult<HttpResponse>> + Send + 'a>>;
}
```

`src/core/src/http/fake.rs` (above the test module):

```rust
use crate::error::{CoreError, CoreResult};
use crate::http::{HttpResponse, Request, Transport};
use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;

/// Test double: records outgoing requests in order, replies with queued responses (§9.2).
pub struct CapturingTransport {
    requests: Mutex<Vec<Request>>,
    responses: Mutex<std::collections::VecDeque<HttpResponse>>,
}

impl CapturingTransport {
    pub fn new() -> Self {
        Self { requests: Mutex::new(vec![]), responses: Mutex::new(Default::default()) }
    }
    pub fn queue_response(&self, resp: HttpResponse) {
        self.responses.lock().unwrap().push_back(resp);
    }
    pub fn requests(&self) -> Vec<Request> {
        self.requests.lock().unwrap().clone()
    }
}

impl Default for CapturingTransport {
    fn default() -> Self { Self::new() }
}

impl Transport for CapturingTransport {
    fn send<'a>(
        &'a self,
        req: Request,
    ) -> Pin<Box<dyn Future<Output = CoreResult<HttpResponse>> + Send + 'a>> {
        Box::pin(async move {
            self.requests.lock().unwrap().push(req);
            self.responses
                .lock()
                .unwrap()
                .pop_front()
                .ok_or_else(|| CoreError::Http { message: "no queued response".into() })
        })
    }
}
```

- [ ] **Step 4: Run tests, expect pass**

Run: `cd src/core && cargo test --lib http`
Expected: PASS (cookie_jar module is empty for now; add a stub `//! placeholder` in `src/core/src/http/cookie_jar.rs` so `mod cookie_jar;` compiles).

- [ ] **Step 5: Commit**

```bash
git add src/core/src/http
git commit -m "feat(core): http Transport trait, request/response types, capturing fake"
```

---

### Task 6: Ventopay cookie jar (insertion-order semantics)

**Files:**
- Create/replace: `src/core/src/http/cookie_jar.rs`
- Test: inline `#[cfg(test)]`

**Interfaces:**
- Produces: `pub struct CookieJar` with `fn new()`, `fn capture(&mut self, set_cookie_values: &[String])`, `fn header(&self) -> Option<String>`, `fn clear(&mut self)`. Semantics exactly per 02-ventopay-scraping §2.2.

- [ ] **Step 1: Write the failing tests**

`src/core/src/http/cookie_jar.rs` test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captures_name_value_ignoring_attributes() {
        // take substring before first ';', split on first '=' (02 §2.2).
        let mut jar = CookieJar::new();
        jar.capture(&["ASP.NET_SessionId=abc123; path=/; HttpOnly".to_string()]);
        assert_eq!(jar.header().as_deref(), Some("ASP.NET_SessionId=abc123"));
    }

    #[test]
    fn overwrite_preserves_insertion_position() {
        let mut jar = CookieJar::new();
        jar.capture(&["a=1".into()]);
        jar.capture(&["b=2".into()]);
        jar.capture(&["a=3".into()]); // overwrite a, keep position
        assert_eq!(jar.header().as_deref(), Some("a=3; b=2"));
    }

    #[test]
    fn ignores_malformed_and_empty() {
        let mut jar = CookieJar::new();
        jar.capture(&["=novalue".into(), "noequals".into()]);
        assert_eq!(jar.header(), None); // '=' at index 0 or absent → ignored
    }

    #[test]
    fn no_header_when_empty() {
        assert_eq!(CookieJar::new().header(), None);
    }
}
```

- [ ] **Step 2: Run, expect failure**

Run: `cd src/core && cargo test --lib cookie_jar`
Expected: FAIL.

- [ ] **Step 3: Implement `CookieJar`**

`src/core/src/http/cookie_jar.rs` (above test):

```rust
//! Ventopay's app-owned cookie jar. Exact v1 semantics: substring before first ';',
//! split on first '=', ignore attributes, never expire, overwrite preserving insertion
//! position, emit "n1=v1; n2=v2" in insertion order, no header when empty
//! (02-ventopay-scraping §2.2; v1: ventopayClient.ts:31-58).

/// Insertion-ordered name→value store.
#[derive(Debug, Default, Clone)]
pub struct CookieJar {
    entries: Vec<(String, String)>, // insertion order preserved
}

impl CookieJar {
    pub fn new() -> Self { Self::default() }

    pub fn capture(&mut self, set_cookie_values: &[String]) {
        for raw in set_cookie_values {
            let name_value = raw.split(';').next().unwrap_or("");
            let eq = match name_value.find('=') {
                Some(i) if i > 0 => i,          // '=' absent or at index 0 → ignore
                _ => continue,
            };
            let name = name_value[..eq].trim().to_string();
            let value = name_value[eq + 1..].trim().to_string();
            match self.entries.iter_mut().find(|(n, _)| *n == name) {
                Some(slot) => slot.1 = value,   // overwrite in place
                None => self.entries.push((name, value)),
            }
        }
    }

    pub fn header(&self) -> Option<String> {
        if self.entries.is_empty() { return None; }
        Some(
            self.entries.iter()
                .map(|(n, v)| format!("{n}={v}"))
                .collect::<Vec<_>>()
                .join("; "),
        )
    }

    pub fn clear(&mut self) { self.entries.clear(); }
}
```

- [ ] **Step 4: Run, expect pass**

Run: `cd src/core && cargo test --lib cookie_jar`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/core/src/http/cookie_jar.rs
git commit -m "feat(core): Ventopay insertion-order cookie jar with exact v1 semantics"
```

---

### Task 7: reqwest production transport

**Files:**
- Create: `src/core/src/http/reqwest_transport.rs`
- Modify: `src/core/src/http/mod.rs` (add `pub mod reqwest_transport;`)
- Test: inline `#[cfg(test)]` using a loopback `tokio` TCP stub (no live network).

**Interfaces:**
- Consumes: `Transport`, `Request`, `HttpResponse`, `Method`, `RequestBody` (Task 5).
- Produces: `pub struct ReqwestTransport` with `fn new() -> CoreResult<Self>` that builds a `reqwest::Client` configured: **cookie store enabled** (per-instance), redirect policy limited to 5, no default `User-Agent`. `send` maps `Request` → reqwest call, always adding `Accept: application/json, text/plain, */*`, encoding `Multipart`/`Form`/`Json` bodies with field order preserved, treating status 200–399 as success and ≥ 400 as `CoreError::Http`.

> The exact per-service header/cookie rules (Gourmet uses this cookie store; Ventopay disables it and uses `CookieJar` instead) are applied by the client layer in later plans. This task provides the generic reqwest-backed transport with the shared config (Accept, no-UA, redirect limit, status validation) and one constructor flag for the cookie store.

- [ ] **Step 1: Write the failing test against a loopback server**

`src/core/src/http/reqwest_transport.rs` test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::{Method, Request, RequestBody, Transport};
    use std::io::{Read, Write};
    use std::net::TcpListener;

    // Minimal one-shot HTTP/1.1 server: captures the raw request, replies 200.
    fn spawn_capturing_server() -> (String, std::sync::mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 4096];
                let n = stream.read(&mut buf).unwrap_or(0);
                let _ = tx.send(String::from_utf8_lossy(&buf[..n]).to_string());
                let _ = stream.write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nhi",
                );
            }
        });
        (format!("http://{addr}/path"), rx)
    }

    #[tokio::test]
    async fn sends_accept_header_and_no_user_agent() {
        let (url, rx) = spawn_capturing_server();
        let t = ReqwestTransport::new(true).unwrap();
        let resp = t.send(Request {
            method: Method::Get, url, headers: vec![], body: None,
        }).await.unwrap();
        assert_eq!(resp.status, 200);
        assert_eq!(resp.body, "hi");

        let raw = rx.recv().unwrap();
        assert!(raw.contains("accept: application/json, text/plain, */*")
                || raw.contains("Accept: application/json, text/plain, */*"),
                "missing Accept header in:\n{raw}");
        assert!(!raw.to_lowercase().contains("user-agent:"),
                "reqwest sent a User-Agent, must be none:\n{raw}");
    }

    #[tokio::test]
    async fn status_400_maps_to_http_error() {
        // server that replies 404
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            if let Ok((mut s, _)) = listener.accept() {
                let mut b = [0u8; 1024]; let _ = s.read(&mut b);
                let _ = s.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n");
            }
        });
        let t = ReqwestTransport::new(true).unwrap();
        let out = t.send(Request {
            method: Method::Get, url: format!("http://{addr}/x"),
            headers: vec![], body: None,
        }).await;
        assert!(matches!(out, Err(crate::error::CoreError::Http { .. })));
    }
}
```

- [ ] **Step 2: Run, expect failure**

Run: `cd src/core && cargo test --lib reqwest_transport`
Expected: FAIL — `ReqwestTransport` not defined.

- [ ] **Step 3: Implement `ReqwestTransport`**

Add `pub mod reqwest_transport;` to `src/core/src/http/mod.rs`. Create `src/core/src/http/reqwest_transport.rs` (above test):

```rust
//! Production Transport on reqwest. Shared config for both services: explicit Accept
//! header on every request, NO User-Agent, redirect limit 5, status 200-399 = success
//! (docs/architecture §3.1; 01 §2, 02 §2.1). Per-service cookie behavior is layered above.
use crate::error::{CoreError, CoreResult};
use crate::http::{HttpResponse, Method, Request, RequestBody, Transport};
use std::future::Future;
use std::pin::Pin;

const ACCEPT: &str = "application/json, text/plain, */*";

pub struct ReqwestTransport {
    client: reqwest::Client,
}

impl ReqwestTransport {
    /// `cookie_store` = true for the Gourmet client (reqwest manages cookies),
    /// false for Ventopay (which manages its own jar above this layer).
    pub fn new(cookie_store: bool) -> CoreResult<Self> {
        let client = reqwest::Client::builder()
            .cookie_store(cookie_store)
            .redirect(reqwest::redirect::Policy::limited(5))
            // reqwest sets no default UA when we don't call .user_agent(); leave it absent.
            .build()
            .map_err(|e| CoreError::Http { message: e.to_string() })?;
        Ok(Self { client })
    }
}

impl Transport for ReqwestTransport {
    fn send<'a>(
        &'a self,
        req: Request,
    ) -> Pin<Box<dyn Future<Output = CoreResult<HttpResponse>> + Send + 'a>> {
        Box::pin(async move {
            let method = match req.method {
                Method::Get => reqwest::Method::GET,
                Method::Post => reqwest::Method::POST,
            };
            let mut rb = self.client.request(method, &req.url).header("Accept", ACCEPT);
            for (k, v) in &req.headers {
                rb = rb.header(k.as_str(), v.as_str());
            }
            rb = match req.body {
                None => rb,
                Some(RequestBody::Json(s)) => rb.header("Content-Type", "application/json").body(s),
                Some(RequestBody::Form(fields)) => {
                    // preserve field order; reqwest .form() takes a slice of pairs.
                    rb.form(&fields)
                }
                Some(RequestBody::Multipart(fields)) => {
                    let mut form = reqwest::multipart::Form::new();
                    for (k, v) in fields {
                        form = form.text(k, v);
                    }
                    rb.multipart(form)
                }
            };
            let resp = rb.send().await.map_err(|e| CoreError::Http { message: e.to_string() })?;
            let status = resp.status().as_u16();
            if status >= 400 {
                return Err(CoreError::Http { message: format!("HTTP {status}") });
            }
            let headers = resp.headers().iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .collect();
            let body = resp.text().await.map_err(|e| CoreError::Http { message: e.to_string() })?;
            Ok(HttpResponse { status, headers, body })
        })
    }
}
```

> Note: reqwest's default redirect policy already follows redirects; `Policy::limited(5)` matches v1's `maxRedirects: 5`. reqwest does not send a `User-Agent` unless `.user_agent()` is called — the test asserts this. For multipart, reqwest generates the boundary automatically (matches v1's axios.postForm).

- [ ] **Step 4: Run, expect pass**

Run: `cd src/core && cargo test --lib reqwest_transport`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/core/src/http
git commit -m "feat(core): reqwest transport — Accept header, no UA, redirect 5, status validation"
```

---

### Task 8: `storage::kv` — Kv trait, file-backed store, in-memory fake

**Files:**
- Create/replace: `src/core/src/storage/mod.rs`
- Create: `src/core/src/storage/file_kv.rs`, `src/core/src/storage/memory_kv.rs`
- Test: inline `#[cfg(test)]` in both impls

**Interfaces:**
- Consumes: `CoreError` (Task 3).
- Produces:
  - `pub trait Kv: Send + Sync { fn get(&self, key: &str) -> CoreResult<Option<String>>; fn set(&self, key: &str, value: &str) -> CoreResult<()>; fn remove(&self, key: &str) -> CoreResult<()>; }` — string values (callers serialize JSON themselves, matching v1 AsyncStorage semantics).
  - `pub struct FileKv` — one file per key under an injected dir; `fn new(dir: PathBuf) -> Self`; atomic writes (temp file + rename); in-process `Mutex` guarding concurrent headless/UI access (docs/architecture §3.3).
  - `pub struct MemoryKv` — `Mutex<HashMap<String,String>>` test double.
  - Absent key → `Ok(None)` (never an error); this is the contract that makes `loadCachedMenus`/`loadCachedOrders` no-op on a missing entry (caching.md §3.4).

- [ ] **Step 1: Write the failing tests (shared behavior, run against both impls)**

`src/core/src/storage/memory_kv.rs` test module:

```rust
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
```

`src/core/src/storage/file_kv.rs` test module:

```rust
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
        // ensure the on-disk filename is safe and roundtrips.
        let dir = tempfile::tempdir().unwrap();
        let kv = FileKv::new(dir.path().to_path_buf());
        kv.set("billing_2026-02", "x").unwrap();
        assert_eq!(kv.get("billing_2026-02").unwrap().as_deref(), Some("x"));
    }
}
```

- [ ] **Step 2: Run, expect failure**

Run: `cd src/core && cargo test --lib storage`
Expected: FAIL — types not defined.

- [ ] **Step 3: Implement the trait and both stores**

`src/core/src/storage/mod.rs`:

```rust
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
```

`src/core/src/storage/memory_kv.rs` (above test):

```rust
use crate::error::CoreResult;
use crate::storage::Kv;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Default)]
pub struct MemoryKv { map: Mutex<HashMap<String, String>> }

impl MemoryKv {
    pub fn new() -> Self { Self::default() }
}

impl Kv for MemoryKv {
    fn get(&self, key: &str) -> CoreResult<Option<String>> {
        Ok(self.map.lock().unwrap().get(key).cloned())
    }
    fn set(&self, key: &str, value: &str) -> CoreResult<()> {
        self.map.lock().unwrap().insert(key.to_string(), value.to_string());
        Ok(())
    }
    fn remove(&self, key: &str) -> CoreResult<()> {
        self.map.lock().unwrap().remove(key);
        Ok(())
    }
}
```

`src/core/src/storage/file_kv.rs` (above test):

```rust
use crate::error::{CoreError, CoreResult};
use crate::storage::Kv;
use std::path::PathBuf;
use std::sync::Mutex;

/// One file per key under `dir`. Atomic writes (temp + rename). An in-process Mutex
/// guards concurrent access from headless background entry points and the UI process
/// (same app process on iOS/Android; docs/architecture §3.3).
pub struct FileKv { dir: PathBuf, lock: Mutex<()> }

impl FileKv {
    pub fn new(dir: PathBuf) -> Self { Self { dir, lock: Mutex::new(()) } }

    /// Percent-ish encode so keys with '/', ':' etc. map to a single safe filename.
    fn path_for(&self, key: &str) -> PathBuf {
        let safe: String = key.chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' })
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
            Err(e) => Err(CoreError::Storage { message: e.to_string() }),
        }
    }
    fn set(&self, key: &str, value: &str) -> CoreResult<()> {
        let _g = self.lock.lock().unwrap();
        std::fs::create_dir_all(&self.dir)
            .map_err(|e| CoreError::Storage { message: e.to_string() })?;
        let final_path = self.path_for(key);
        let tmp_path = final_path.with_extension("val.tmp");
        std::fs::write(&tmp_path, value)
            .map_err(|e| CoreError::Storage { message: e.to_string() })?;
        std::fs::rename(&tmp_path, &final_path)
            .map_err(|e| CoreError::Storage { message: e.to_string() })?;
        Ok(())
    }
    fn remove(&self, key: &str) -> CoreResult<()> {
        let _g = self.lock.lock().unwrap();
        match std::fs::remove_file(self.path_for(key)) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(CoreError::Storage { message: e.to_string() }),
        }
    }
}
```

- [ ] **Step 4: Run, expect pass**

Run: `cd src/core && cargo test --lib storage`
Expected: all storage tests PASS.

- [ ] **Step 5: Full crate test + commit**

Run: `cd src/core && cargo test`
Expected: all tests across domain/error/datetime/http/storage PASS.

```bash
git add src/core/src/storage
git commit -m "feat(core): storage::kv — Kv trait, atomic FileKv, MemoryKv fake"
```

---

## Post-plan: subsequent Rust core sub-plans

This foundation unblocks (each gets its own plan, written against the verified requirements docs):

1. **Phase 1b — Gourmet scraping** (`gourmet::{client,parser,api}`): request sequences, `ufprt`+`__ncforminfo`, multipart, selectors, session expiry, JSON APIs, logout. Fixture + capturing-transport tests per 06-testing §6.1–6.2. **Ban-critical.**
2. **Phase 1c — Ventopay scraping** (`ventopay::{client,parser,api}`): ASP.NET state flow, the `CookieJar` from Task 6, transactions parsing, Gourmet filter.
3. **Phase 1d — Feature services** (`features::{menus,orders,billing}`, `storage::cache`): TTL/in-flight guards, pending sets, optimistic update+revert, dual-source billing asymmetry, cache write-through — over a fake API trait + `MemoryKv` + `FixedClock`.
4. **Phase 1e — Notify + demo** (`notify::*`, `demo::*`): fingerprint, decision functions returning `NotificationCommand`, the LCG-in-f64 demo PRNG.
5. **Phase 1f — UniFFI facade + bindings + CI** (`lib.rs`, `ffi::*`): `SnackPilotCore`, the async operations, `AnalyticsSink`/`ProgressListener` callbacks, XCFramework + AAR packaging, `.github/workflows/core-test.yml`.

## Self-review notes

- **Spec coverage:** this plan covers docs/architecture §3.1 (transport config), §3.2 (regex/selector infra is deferred to parser plans — correct), §3.3 (`storage::kv`), §4.2 (all records/enums + `CoreError`), and the datetime/cutoff behavior underpinning menus.md §6.2 / orders.md §4.2 and all wire formats (01 §12, 02 §5). Fixture mirroring (§9.1) is Task 1.
- **Deferred (not gaps):** `scraper` dependency, the `Clock`-in-features wiring, `storage::cache` typed helpers, and UniFFI derives are intentionally in later plans; this plan keeps the crate dependency-minimal and export-free.
- **Type consistency:** `MenuCategory::display`, `CoreError` variants, `Transport::send` boxed-future signature, `Kv` absent-key→`Ok(None)`, and `RequestBody` order-preserving `Vec` are used consistently and are the contracts later plans consume.
