# SnackPilot v2 — Ventopay Parser (Phase 1c-i) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `ventopay::parser` — the pure HTML/text parsing for the Ventopay (Automaten) site: ASP.NET hidden-state extraction, login-state detection, and transaction-list parsing (title→amount+restaurant, German number parsing with parseFloat-prefix semantics, timestamp parsing with the German month table incl. `Jän`/`Mrz`, and the Gourmet filter) — byte-for-byte per `docs/requirements/02-ventopay-scraping.md` §3/§4/§6, verified against the recorded fixtures. Also adds the `datetime::local_epoch_from_parts` helper the timestamp parser needs.

**Architecture:** `src/core/src/ventopay/parser.rs` of pure, synchronous functions over `&str`. Uses `scraper` (already a dep) for the transaction elements and ASP.NET inputs, and `regex` (already a dep) for the login check, title, and timestamp patterns. German number parsing replicates JavaScript `parseFloat`'s longest-numeric-prefix behavior (a small hand-written scanner) — a strict Rust parse would diverge on malformed inputs (02 §6.3).

**Tech Stack:** Rust 2021, `scraper` 0.20, `regex` 1, `chrono` (via a new `datetime` helper), the foundation `domain::VentopayTransaction` + `error::CoreError`.

## Global Constraints

- **Baseline:** v1.4.5 (`main` @ 6997c44); authoritative spec `docs/requirements/02-ventopay-scraping.md`. Where CLAUDE.md and code disagree, the code (per the spec's discrepancy notes) wins.
- **Crate:** `src/core/` on the `v2` worktree `/Users/radaiko/dev/private/SnackPilot-v2`. Test with `cd src/core && cargo test`.
- **Fixtures:** 4 Ventopay fixtures at `src/core/tests/fixtures/ventopay/`. Sentinels: login-page `__VIEWSTATE="VIEWSTATE-TOKEN-LONG-BASE64-STRING-ABC123"`, `__VIEWSTATEGENERATOR="ABCD1234"`, `__EVENTVALIDATION="EVENTVALIDATION-TOKEN-XYZ789"`; login-success contains `Ausloggen.aspx`; transactions-page has 6 `.transact` (one restaurant `Gourmet Betriebsrestaurant` → filtered, leaving 5; months include `Jän`/`Mrz`); transactions-empty says `Keine Transaktionen in diesem Zeitraum.`
- **ASP.NET state (§3 Step 2):** six inputs by id; `__VIEWSTATE`/`__VIEWSTATEGENERATOR`/`__EVENTVALIDATION` required-non-empty (else `Could not extract ASP.NET state from page`); the three `__LASTFOCUS`/`__EVENTTARGET`/`__EVENTARGUMENT` optional, default `""`.
- **Login check (§3 Step 4):** regex `/href="Ausloggen\.aspx"/i` (case-insensitive; escaped dot).
- **Transaction selector (§6.1):** `div.transact` (no ancestor constraint). Skip if `id` attr missing/empty; skip if title text empty; empty timestamp → use the injected `now` (not skip).
- **Title regex (§6.2):** `€\s*([\d,]+)\s*\((.+)\)` — group1 amount, group2 (greedy) restaurant; no-match fallback: amount = German-parse of whole title, restaurant = whole title.
- **German number (§6.3):** strip all chars except `[0-9,\-]`; replace FIRST `,` with `.`; then **parseFloat-prefix** parse (longest leading numeric prefix, ignore trailing; no leading number → `0`).
- **Timestamp (§6.4):** regex `(\d{1,2})\.\s*(\p{L}{3})\p{L}*\s+(\d{4})\s*-\s*(\d{1,2}):(\d{2})` (unicode); month table (0-based) `jan/jän→0, feb→1, mär/mar/mrz→2, apr→3, mai→4, jun→5, jul→6, aug→7, sep→8, okt→9, nov→10, dez→11`, unknown→0; seconds 0; LOCAL time. No regex match → ISO-8601 fallback (`parse_bill_date`); otherwise → injected `now`.
- **Gourmet filter (§6.5):** drop any transaction whose restaurant contains `gourmet` (case-insensitive). No Kaffeeautomat exception.
- **Output (§6.6):** `location` == `restaurant` (duplicated); document order preserved.
- Commit after each green task. Conventional-commit, scope `core`.

---

## File structure (this plan)

```
src/core/src/
├── datetime/formats.rs   # + pub fn local_epoch_from_parts(...)
└── ventopay/
    ├── mod.rs            # module + constants
    └── parser.rs         # aspnet state, login check, transactions parse + tests
src/core/src/lib.rs       # + pub mod ventopay;
```

---

### Task 1: `datetime::local_epoch_from_parts` helper

**Files:**
- Modify: `src/core/src/datetime/formats.rs`

**Interfaces:**
- Produces: `pub fn local_epoch_from_parts(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> Option<i64>` — local-tz epoch ms for the given Y/M/D h:m (seconds 0); `None` on an invalid date.

- [ ] **Step 1: Write the failing test**

Add to the `tests` module in `src/core/src/datetime/formats.rs`:

```rust
#[test]
fn local_epoch_from_parts_matches_local_midnight_helper() {
    let a = local_epoch_from_parts(2026, 2, 10, 0, 0).unwrap();
    assert_eq!(a, vienna_like_local("2026-02-10", 0, 0));
    let b = local_epoch_from_parts(2026, 2, 10, 11, 49).unwrap();
    assert_eq!(b, vienna_like_local("2026-02-10", 11, 49));
    assert_eq!(local_epoch_from_parts(2026, 13, 40, 0, 0), None); // invalid
}
```

- [ ] **Step 2: Run, expect failure**

Run: `cd src/core && cargo test --lib datetime::formats::tests::local_epoch_from_parts`
Expected: FAIL — function not defined.

- [ ] **Step 3: Implement**

Add to `src/core/src/datetime/formats.rs` (near `local_epoch_ms`):

```rust
/// Local-tz epoch ms for Y/M/D h:m (seconds 0). None on an invalid date (Ventopay §6.4).
pub fn local_epoch_from_parts(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
) -> Option<i64> {
    let d = NaiveDate::from_ymd_opt(year, month, day)?;
    let t = NaiveTime::from_hms_opt(hour, minute, 0)?;
    local_epoch_ms(d.and_time(t))
}
```

- [ ] **Step 4: Run, expect pass**

Run: `cd src/core && cargo test --lib datetime`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/core/src/datetime/formats.rs
git commit -m "feat(core): datetime::local_epoch_from_parts for Ventopay timestamps"
```

---

### Task 2: `ventopay` module + constants + ASP.NET state + login check

**Files:**
- Modify: `src/core/src/lib.rs`
- Create: `src/core/src/ventopay/mod.rs`, `src/core/src/ventopay/parser.rs`

**Interfaces:**
- Produces:
  - constants in `ventopay` module: `VENTOPAY_BASE_URL`, `VENTOPAY_LOGIN_URL`, `VENTOPAY_TRANSACTIONS_URL`, `VENTOPAY_LOGOUT_URL`, `VENTOPAY_ORIGIN`, `VENTOPAY_COMPANY_ID`.
  - `pub struct AspNetState { pub last_focus, pub event_target, pub event_argument, pub viewstate, pub viewstate_generator, pub event_validation: String }`
  - `pub fn extract_aspnet_state(html: &str) -> CoreResult<AspNetState>` (§3 Step 2).
  - `pub fn is_logged_in(html: &str) -> bool` — `/href="Ausloggen\.aspx"/i` (§3 Step 4).

- [ ] **Step 1: Write failing tests**

In `src/core/src/lib.rs` add after `pub mod storage;`:

```rust
pub mod ventopay;
```

Create `src/core/src/ventopay/mod.rs`:

```rust
//! Ventopay (Automaten) scraping — client, parser, API (docs/requirements/02-ventopay-scraping.md).
pub mod parser;

/// 02-ventopay-scraping §1.
pub const VENTOPAY_BASE_URL: &str = "https://my.ventopay.com/mocca.website";
pub const VENTOPAY_LOGIN_URL: &str = "https://my.ventopay.com/mocca.website/Login.aspx";
pub const VENTOPAY_TRANSACTIONS_URL: &str =
    "https://my.ventopay.com/mocca.website/Transaktionen.aspx";
pub const VENTOPAY_LOGOUT_URL: &str = "https://my.ventopay.com/mocca.website/Ausloggen.aspx";
pub const VENTOPAY_ORIGIN: &str = "https://my.ventopay.com";
pub const VENTOPAY_COMPANY_ID: &str = "0da8d3ec-0178-47d5-9ccd-a996f04acb61";
```

Create `src/core/src/ventopay/parser.rs` with a test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const LOGIN_PAGE: &str = include_str!("../../tests/fixtures/ventopay/login-page.html");
    const LOGIN_SUCCESS: &str = include_str!("../../tests/fixtures/ventopay/login-success.html");

    #[test]
    fn extracts_aspnet_state() {
        let s = extract_aspnet_state(LOGIN_PAGE).unwrap();
        assert_eq!(s.viewstate, "VIEWSTATE-TOKEN-LONG-BASE64-STRING-ABC123");
        assert_eq!(s.viewstate_generator, "ABCD1234");
        assert_eq!(s.event_validation, "EVENTVALIDATION-TOKEN-XYZ789");
    }

    #[test]
    fn missing_required_state_errors() {
        let html = r#"<input id="__VIEWSTATE" value=""><input id="__VIEWSTATEGENERATOR" value="g">"#;
        let err = extract_aspnet_state(html).unwrap_err();
        assert_eq!(err.to_string(), "Could not extract ASP.NET state from page");
    }

    #[test]
    fn login_check_matches_logout_link() {
        assert!(is_logged_in(LOGIN_SUCCESS));
        assert!(is_logged_in(r#"<a href="Ausloggen.aspx">x</a>"#));
        assert!(!is_logged_in(LOGIN_PAGE));
    }
}
```

- [ ] **Step 2: Run, expect failure**

Run: `cd src/core && cargo test --lib ventopay`
Expected: FAIL.

- [ ] **Step 3: Implement**

Top of `src/core/src/ventopay/parser.rs`:

```rust
//! Pure Ventopay HTML/text parsing (02-ventopay-scraping §3-§6). No network, no async.
use crate::error::{CoreError, CoreResult};
use regex::Regex;
use scraper::{Html, Selector};

fn parse_err(msg: impl Into<String>) -> CoreError {
    CoreError::Parse { message: msg.into() }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AspNetState {
    pub last_focus: String,
    pub event_target: String,
    pub event_argument: String,
    pub viewstate: String,
    pub viewstate_generator: String,
    pub event_validation: String,
}

/// §3 Step 2 — six hidden inputs by id; three required-non-empty.
pub fn extract_aspnet_state(html: &str) -> CoreResult<AspNetState> {
    let doc = Html::parse_document(html);
    let by_id = |id: &str| -> String {
        Selector::parse(&format!("#{id}"))
            .ok()
            .and_then(|s| doc.select(&s).next())
            .and_then(|e| e.value().attr("value"))
            .unwrap_or("")
            .to_string()
    };
    let viewstate = by_id("__VIEWSTATE");
    let viewstate_generator = by_id("__VIEWSTATEGENERATOR");
    let event_validation = by_id("__EVENTVALIDATION");
    if viewstate.is_empty() || viewstate_generator.is_empty() || event_validation.is_empty() {
        return Err(parse_err("Could not extract ASP.NET state from page"));
    }
    Ok(AspNetState {
        last_focus: by_id("__LASTFOCUS"),
        event_target: by_id("__EVENTTARGET"),
        event_argument: by_id("__EVENTARGUMENT"),
        viewstate,
        viewstate_generator,
        event_validation,
    })
}

/// §3 Step 4 — logout link presence, case-insensitive.
pub fn is_logged_in(html: &str) -> bool {
    Regex::new(r#"(?i)href="Ausloggen\.aspx""#)
        .unwrap()
        .is_match(html)
}
```

- [ ] **Step 4: Run, expect pass**

Run: `cd src/core && cargo test --lib ventopay`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/core/src/lib.rs src/core/src/ventopay
git commit -m "feat(core): ventopay module — constants, ASP.NET state extraction, login check"
```

---

### Task 3: `parse_transactions` (title, German number, timestamp, Gourmet filter)

**Files:**
- Modify: `src/core/src/ventopay/parser.rs`

**Interfaces:**
- Consumes: `domain::VentopayTransaction`, `datetime::{local_epoch_from_parts, parse_bill_date}`.
- Produces:
  - `pub fn parse_transactions(html: &str, now_epoch_ms: i64) -> Vec<VentopayTransaction>` (§6.1–§6.6).
  - private helpers `parse_german_amount(&str) -> f64`, `parse_float_prefix(&str) -> f64`, `parse_ventopay_timestamp(&str, i64) -> i64`, `german_month(&str) -> u32`.

- [ ] **Step 1: Write failing tests**

Add to `tests`:

```rust
const TX_PAGE: &str = include_str!("../../tests/fixtures/ventopay/transactions-page.html");
const TX_EMPTY: &str = include_str!("../../tests/fixtures/ventopay/transactions-empty.html");

#[test]
fn parses_transactions_and_applies_gourmet_filter() {
    let txs = parse_transactions(TX_PAGE, 0);
    // 6 in the fixture, the Gourmet one is filtered → 5.
    assert_eq!(txs.len(), 5);
    assert!(txs.iter().all(|t| !t.restaurant.to_lowercase().contains("gourmet")));
    let first = &txs[0];
    assert_eq!(first.id, "dHhuLTAwMQ==");
    assert!((first.amount - 1.8).abs() < 1e-9);
    assert_eq!(first.restaurant, "Café + Co. Automaten");
    assert_eq!(first.location, first.restaurant); // duplicated
}

#[test]
fn empty_page_yields_no_transactions() {
    assert_eq!(parse_transactions(TX_EMPTY, 0).len(), 0);
}

#[test]
fn german_month_variants_parse() {
    // Jän → January, Mrz → March (via the month table).
    assert_eq!(german_month("jän"), 0);
    assert_eq!(german_month("mrz"), 2);
    assert_eq!(german_month("mär"), 2);
    assert_eq!(german_month("zzz"), 0); // unknown → January
}

#[test]
fn german_amount_prefix_parse() {
    assert!((parse_german_amount("€ 1,80") - 1.8).abs() < 1e-9);
    assert!((parse_german_amount("0,50") - 0.5).abs() < 1e-9);
    // "1,80 / 2,00" → cleaned "1,802,00" → "1.802,00" → prefix 1.802
    assert!((parse_german_amount("1,80 / 2,00") - 1.802).abs() < 1e-9);
    assert_eq!(parse_german_amount("garbage"), 0.0);
}

#[test]
fn title_fallback_when_no_paren_format() {
    // a transact whose title is just "1,80" → amount 1.8, restaurant "1,80"
    let html = r#"<div class="transact" id="x"><div class="transact_title">1,80</div>
        <div class="transact_timestamp">09. Feb 2026 - 11:49 Uhr</div></div>"#;
    let txs = parse_transactions(html, 0);
    assert_eq!(txs.len(), 1);
    assert!((txs[0].amount - 1.8).abs() < 1e-9);
    assert_eq!(txs[0].restaurant, "1,80");
}

#[test]
fn skip_rules_and_empty_timestamp_uses_now() {
    let html = r#"
        <div class="transact"><div class="transact_title">€ 1,00 (A)</div></div>
        <div class="transact" id="y"><div class="transact_title"></div></div>
        <div class="transact" id="z"><div class="transact_title">€ 2,00 (B)</div>
            <div class="transact_timestamp"></div></div>"#;
    let now = 1_700_000_000_000;
    let txs = parse_transactions(html, now);
    // first skipped (no id), second skipped (empty title), third kept with now.
    assert_eq!(txs.len(), 1);
    assert_eq!(txs[0].id, "z");
    assert_eq!(txs[0].date_epoch_ms, now);
}
```

- [ ] **Step 2: Run, expect failure**

Run: `cd src/core && cargo test --lib ventopay`
Expected: FAIL.

- [ ] **Step 3: Implement `parse_transactions` and helpers**

Add imports at top of `parser.rs`: `use crate::datetime::{local_epoch_from_parts, parse_bill_date}; use crate::domain::VentopayTransaction;`. Add:

```rust
/// §6 — parse the transactions list. `now_epoch_ms` is used when a timestamp is empty.
pub fn parse_transactions(html: &str, now_epoch_ms: i64) -> Vec<VentopayTransaction> {
    let doc = Html::parse_document(html);
    let tx_sel = Selector::parse("div.transact").unwrap();
    let title_sel = Selector::parse(".transact_title").unwrap();
    let ts_sel = Selector::parse(".transact_timestamp").unwrap();
    let title_re = Regex::new(r"€\s*([\d,]+)\s*\((.+)\)").unwrap();

    let mut out = Vec::new();
    for el in doc.select(&tx_sel) {
        let id = match el.value().attr("id") {
            Some(v) if !v.is_empty() => v.to_string(),
            _ => continue, // skip: no id (§6.1)
        };
        let title = el
            .select(&title_sel)
            .next()
            .map(|t| t.text().collect::<String>().trim().to_string())
            .unwrap_or_default();
        if title.is_empty() {
            continue; // skip: empty title (§6.1)
        }
        let ts_text = el
            .select(&ts_sel)
            .next()
            .map(|t| t.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        let (amount, restaurant) = match title_re.captures(&title) {
            Some(c) => (
                parse_german_amount(&c[1]),
                c[2].trim().to_string(),
            ),
            None => (parse_german_amount(&title), title.clone()),
        };
        // §6.5 Gourmet filter.
        if restaurant.to_lowercase().contains("gourmet") {
            continue;
        }
        let date_epoch_ms = parse_ventopay_timestamp(&ts_text, now_epoch_ms);
        out.push(VentopayTransaction {
            id,
            date_epoch_ms,
            amount,
            restaurant: restaurant.clone(),
            location: restaurant, // §6.6 location == restaurant
        });
    }
    out
}

/// §6.3 — strip to [0-9,-], first ',' → '.', then parseFloat-prefix parse.
fn parse_german_amount(text: &str) -> f64 {
    let cleaned: String = text
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == ',' || *c == '-')
        .collect();
    let replaced = match cleaned.find(',') {
        Some(i) => {
            let mut s = cleaned.clone();
            s.replace_range(i..i + 1, ".");
            s
        }
        None => cleaned,
    };
    parse_float_prefix(&replaced)
}

/// JavaScript parseFloat semantics: longest leading numeric prefix; no leading number → 0.
fn parse_float_prefix(s: &str) -> f64 {
    let bytes = s.as_bytes();
    let mut i = 0;
    if i < bytes.len() && (bytes[i] == b'-' || bytes[i] == b'+') {
        i += 1;
    }
    let mut seen_dot = false;
    let mut last_digit_end = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'0'..=b'9' => {
                i += 1;
                last_digit_end = i;
            }
            b'.' if !seen_dot => {
                seen_dot = true;
                i += 1;
            }
            _ => break,
        }
    }
    if last_digit_end == 0 {
        return 0.0;
    }
    s[..last_digit_end].parse::<f64>().unwrap_or(0.0)
}

/// §6.4 — German timestamp; empty → now; no regex match → ISO fallback → now.
fn parse_ventopay_timestamp(text: &str, now_epoch_ms: i64) -> i64 {
    let t = text.trim();
    if t.is_empty() {
        return now_epoch_ms;
    }
    let re = Regex::new(r"(\d{1,2})\.\s*(\p{L}{3})\p{L}*\s+(\d{4})\s*-\s*(\d{1,2}):(\d{2})").unwrap();
    if let Some(c) = re.captures(t) {
        let day: u32 = c[1].parse().unwrap_or(1);
        let month = german_month(&c[2].to_lowercase()) + 1; // 1-based for chrono
        let year: i32 = c[3].parse().unwrap_or(1970);
        let hour: u32 = c[4].parse().unwrap_or(0);
        let minute: u32 = c[5].parse().unwrap_or(0);
        if let Some(ms) = local_epoch_from_parts(year, month, day, hour, minute) {
            return ms;
        }
    }
    // ISO-8601 fallback, else now (undefined in v1).
    parse_bill_date(t).unwrap_or(now_epoch_ms)
}

/// §6.4 — 0-based German month index; unknown → 0 (January).
fn german_month(m: &str) -> u32 {
    match m {
        "jan" | "jän" => 0,
        "feb" => 1,
        "mär" | "mar" | "mrz" => 2,
        "apr" => 3,
        "mai" => 4,
        "jun" => 5,
        "jul" => 6,
        "aug" => 7,
        "sep" => 8,
        "okt" => 9,
        "nov" => 10,
        "dez" => 11,
        _ => 0,
    }
}
```

- [ ] **Step 4: Run, expect pass**

Run: `cd src/core && cargo test --lib ventopay`
Expected: all transaction tests PASS.

- [ ] **Step 5: Full suite + fmt/clippy + commit**

Run: `cd src/core && cargo test && cargo fmt && cargo clippy --all-targets`
Expected: full suite green; clippy clean.

```bash
git add src/core
git commit -m "feat(core): ventopay parser — transactions (title/amount/timestamp), German locale, Gourmet filter"
```

---

## Post-plan: next Ventopay sub-plans

- **Phase 1c-ii — `ventopay::client`**: `get`/`post_form` over the foundation `Transport` + the `CookieJar` (capture `Set-Cookie` from responses, inject `Cookie` on requests), url-encoded bodies, `Origin`/`Referer`, `lastPageUrl`, `reset` (02 §2). Request-shape + cookie tests via `CapturingTransport`.
- **Phase 1c-iii — `ventopay::api`**: login (GET state → 11-field POST → verify), `ensure_session` (single re-login), `get_transactions` (date params, expiry-retry-once), logout (02 §3–§5).

## Self-review notes

- **Spec coverage:** §3 Step 2 (ASP.NET state), §3 Step 4 (login check), §6.1 (selector + skip rules + empty-timestamp→now), §6.2 (title regex + fallback), §6.3 (German number + parseFloat-prefix), §6.4 (timestamp regex + month table + ISO fallback), §6.5 (Gourmet filter), §6.6 (location==restaurant, document order). The client's cookie/url-encode/header behavior and the api's login/transactions sequences are deferred to 1c-ii/1c-iii.
- **Type consistency:** `extract_aspnet_state`/`AspNetState`, `is_logged_in`, `parse_transactions` are the public surface `ventopay::api` (1c-iii) consumes; helpers are private; `VentopayTransaction` and `datetime::*` come from prior phases.
- **parseFloat-prefix rationale:** replicated per 02 §6.3's explicit v1-mechanism note; a strict Rust parse would return 0 where v1 returns a number on malformed titles — the `german_amount_prefix_parse` test's `1,80 / 2,00 → 1.802` case pins this.
