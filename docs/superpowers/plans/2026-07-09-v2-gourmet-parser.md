# SnackPilot v2 — Gourmet Parser (Phase 1b-i) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `gourmet::parser` — the pure HTML-parsing functions for the Gourmet (Kantine) site: form-token extraction, login-state detection, user-info extraction, menu-item parsing (with category detection), pagination detection, ordered-menu parsing, edit-mode form, cancel-form extraction, and logout-form extraction — every selector/regex byte-for-byte per `docs/requirements/01-gourmet-scraping.md`, verified against the recorded fixtures.

**Architecture:** A single module `src/core/src/gourmet/parser.rs` of pure, synchronous functions over `&str` HTML (no network, no async). Uses the `scraper` crate (CSS selectors, `Html`/`Selector`) plus a couple of small helpers for v1's non-CSS constructs (`:contains(...)` logout button, "direct text nodes only" title, prefix-matched input names, substring `isLoggedIn`). This is the ban-critical selector logic; correctness is proven by asserting every fixture-exact value in `docs/requirements/06-testing.md` §6.2.

**Tech Stack:** Rust 2021, `scraper` 0.20 (new dep), `regex` 1 (new dep, for the category regex), the domain types + `CoreError` from the foundation.

## Global Constraints

- **Baseline:** all behavior traces to v1.4.5 (`main` @ 6997c44); the authoritative spec is `docs/requirements/01-gourmet-scraping.md`. Where CLAUDE.md and code disagree, the code (as documented in the spec's discrepancy tables) wins.
- **Crate location:** `src/core/` on the `v2` branch worktree `/Users/radaiko/dev/private/SnackPilot-v2`. Paths below are relative to that root. Run tests with `cd src/core && cargo test`.
- **Fixtures:** the 9 Gourmet fixtures live at `src/core/tests/fixtures/gourmet/` (mirror of `docs/fixtures/gourmet/`). Sentinel values are asserted verbatim (06-testing §2). Known sentinels used below: login-page `ufprt="CSRF-TOKEN-LOGIN-ABC123"`, `__ncforminfo="NCFORM-TOKEN-LOGIN-XYZ789"`; login-success `loginname=TestUser`, `#shopModel=SM-TEST-123`, `#eater=EATER-TEST-456`, `#staffGroup=SG-TEST-789`; menus-page-0 has meals `menu-001..menu-00N` with `data-date="02-10-2026"`.
- **Category regex — copy exactly:** `MEN(?:Ü|U)\s+([I]{1,3})` case-insensitive; the literal `SUPPE & SALAT` is matched (case-sensitive `contains`) BEFORE the regex (01 §8.3).
- **`isLoggedIn` is substring checks, NOT selectors** — any of `/einstellungen/`, `btnHeaderLogout`, `class="loginname"`, `id="eater"` (01 §4).
- **Menu item selector is desktop-only:** `div.row.hide-sm-down .meal` (NOT global `div.meal`, which would double-parse) (01 §8.2).
- **Approved marker:** `.fa-check` OR `.checkmark` (NOT `.confirmed`) (01 §9.1).
- **Missing tokens abort with the exact error strings** (§3, §9.4, §11). All parser error messages carried verbatim into `CoreError::Parse { message }`.
- Commit after each green step. Conventional-commit, scope `core`.

---

## File structure (this plan)

```
src/core/
├── Cargo.toml                 # + scraper, regex deps
└── src/
    ├── lib.rs                 # + pub mod gourmet;
    └── gourmet/
        ├── mod.rs             # module re-exports
        └── parser.rs          # all pure parsing functions + tests
```

---

### Task 1: Module scaffold + dependencies + token extraction

**Files:**
- Modify: `src/core/Cargo.toml` (add deps), `src/core/src/lib.rs` (add module)
- Create: `src/core/src/gourmet/mod.rs`, `src/core/src/gourmet/parser.rs`
- Test: inline `#[cfg(test)]` in `parser.rs`

**Interfaces:**
- Consumes: `crate::error::{CoreError, CoreResult}`.
- Produces: `pub fn extract_form_tokens(html: &str, form_selector: &str) -> CoreResult<(String, String)>` — within the first element matching `form_selector`, read `input[name="ufprt"]` and `input[name="__ncforminfo"]` `value` attrs; missing either → `CoreError::Parse` with `Could not find ufprt in form: {selector}` / `Could not find __ncforminfo in form: {selector}`.

- [ ] **Step 1: Add dependencies**

In `src/core/Cargo.toml`, under `[dependencies]`, add:

```toml
scraper = "0.20"
regex = "1"
```

- [ ] **Step 2: Wire the module**

In `src/core/src/lib.rs`, add after `pub mod domain;`:

```rust
pub mod gourmet;
```

Create `src/core/src/gourmet/mod.rs`:

```rust
//! Gourmet (Kantine) scraping — client, parser, API (docs/requirements/01-gourmet-scraping.md).
pub mod parser;
```

- [ ] **Step 3: Write the failing token-extraction test**

Create `src/core/src/gourmet/parser.rs` with a test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const LOGIN_PAGE: &str = include_str!("../../tests/fixtures/gourmet/login-page.html");

    #[test]
    fn extracts_login_form_tokens() {
        // login page's first form is the login form (01 §6.2).
        let (ufprt, ncform) = extract_form_tokens(LOGIN_PAGE, "form:first-of-type").unwrap();
        assert_eq!(ufprt, "CSRF-TOKEN-LOGIN-ABC123");
        assert_eq!(ncform, "NCFORM-TOKEN-LOGIN-XYZ789");
    }

    #[test]
    fn missing_ufprt_errors_with_exact_message() {
        let html = r#"<form><input name="__ncforminfo" value="x"></form>"#;
        let err = extract_form_tokens(html, "form").unwrap_err();
        assert_eq!(err.to_string(), "Could not find ufprt in form: form");
    }
}
```

- [ ] **Step 4: Run, expect failure**

Run: `cd src/core && cargo test --lib gourmet::parser::tests::extracts_login_form_tokens`
Expected: FAIL — `extract_form_tokens` not defined (may need `scraper` to download first).

- [ ] **Step 5: Implement `extract_form_tokens`**

At the top of `src/core/src/gourmet/parser.rs`:

```rust
//! Pure Gourmet HTML parsing (01-gourmet-scraping.md §3-§11). No network, no async.
use crate::error::{CoreError, CoreResult};
use scraper::{Html, Selector};

fn parse_err(msg: impl Into<String>) -> CoreError {
    CoreError::Parse { message: msg.into() }
}

/// Read `ufprt` + `__ncforminfo` value attrs from the FIRST element matching `form_selector`.
/// Missing either aborts with the verbatim v1 error (01 §3).
pub fn extract_form_tokens(html: &str, form_selector: &str) -> CoreResult<(String, String)> {
    let doc = Html::parse_document(html);
    let form_sel = Selector::parse(form_selector)
        .map_err(|_| parse_err(format!("Invalid selector: {form_selector}")))?;
    let form = doc
        .select(&form_sel)
        .next()
        .ok_or_else(|| parse_err(format!("Form not found: {form_selector}")))?;
    let ufprt_sel = Selector::parse(r#"input[name="ufprt"]"#).unwrap();
    let ncform_sel = Selector::parse(r#"input[name="__ncforminfo"]"#).unwrap();
    let ufprt = form
        .select(&ufprt_sel)
        .next()
        .and_then(|e| e.value().attr("value"))
        .ok_or_else(|| parse_err(format!("Could not find ufprt in form: {form_selector}")))?;
    let ncform = form
        .select(&ncform_sel)
        .next()
        .and_then(|e| e.value().attr("value"))
        .ok_or_else(|| parse_err(format!("Could not find __ncforminfo in form: {form_selector}")))?;
    Ok((ufprt.to_string(), ncform.to_string()))
}
```

- [ ] **Step 6: Run, expect pass**

Run: `cd src/core && cargo test --lib gourmet`
Expected: both token tests PASS.

- [ ] **Step 7: Commit**

```bash
git add src/core/Cargo.toml src/core/Cargo.lock src/core/src/lib.rs src/core/src/gourmet
git commit -m "feat(core): gourmet::parser scaffold + form-token extraction"
```

---

### Task 2: `is_logged_in` + `extract_user_info`

**Files:**
- Modify: `src/core/src/gourmet/parser.rs`

**Interfaces:**
- Consumes: `GourmetUserInfo` (foundation domain).
- Produces:
  - `pub fn is_logged_in(html: &str) -> bool` — true iff HTML contains any of the four substrings (01 §4).
  - `pub fn extract_user_info(html: &str) -> CoreResult<GourmetUserInfo>` — `#shopModel`/`#eater`/`#staffGroup` value attrs required (missing any → `Could not extract user info from page`); `span.loginname` text trimmed, empty tolerated (01 §5).

- [ ] **Step 1: Write failing tests**

Add to the `tests` module in `parser.rs`:

```rust
const LOGIN_SUCCESS: &str = include_str!("../../tests/fixtures/gourmet/login-success.html");
const LOGIN_FAILED: &str = include_str!("../../tests/fixtures/gourmet/login-failed.html");

#[test]
fn is_logged_in_true_on_authenticated_page() {
    assert!(is_logged_in(LOGIN_SUCCESS));
}

#[test]
fn is_logged_in_false_on_login_and_failed_pages() {
    assert!(!is_logged_in(include_str!("../../tests/fixtures/gourmet/login-page.html")));
    assert!(!is_logged_in(LOGIN_FAILED));
}

#[test]
fn extracts_user_info_from_success_page() {
    let info = extract_user_info(LOGIN_SUCCESS).unwrap();
    assert_eq!(info.username, "TestUser");
    assert_eq!(info.shop_model_id, "SM-TEST-123");
    assert_eq!(info.eater_id, "EATER-TEST-456");
    assert_eq!(info.staff_group_id, "SG-TEST-789");
}

#[test]
fn user_info_missing_ids_errors() {
    let html = r#"<span class="loginname">x</span>"#;
    let err = extract_user_info(html).unwrap_err();
    assert_eq!(err.to_string(), "Could not extract user info from page");
}
```

- [ ] **Step 2: Run, expect failure**

Run: `cd src/core && cargo test --lib gourmet`
Expected: FAIL — functions not defined.

- [ ] **Step 3: Implement both functions**

Add to `parser.rs` (import the domain type at the top: `use crate::domain::GourmetUserInfo;`):

```rust
/// Substring-based login detection — NOT selectors (01 §4).
pub fn is_logged_in(html: &str) -> bool {
    html.contains("/einstellungen/")
        || html.contains("btnHeaderLogout")
        || html.contains(r#"class="loginname""#)
        || html.contains(r#"id="eater""#)
}

/// Extract user info; the three IDs are required, username is tolerated empty (01 §5).
pub fn extract_user_info(html: &str) -> CoreResult<GourmetUserInfo> {
    let doc = Html::parse_document(html);
    let attr_value = |id: &str| -> Option<String> {
        let sel = Selector::parse(&format!("#{id}")).ok()?;
        doc.select(&sel).next()?.value().attr("value").map(|s| s.to_string())
    };
    let shop_model_id = attr_value("shopModel");
    let eater_id = attr_value("eater");
    let staff_group_id = attr_value("staffGroup");
    let (shop_model_id, eater_id, staff_group_id) = match (shop_model_id, eater_id, staff_group_id) {
        (Some(s), Some(e), Some(g)) => (s, e, g),
        _ => return Err(parse_err("Could not extract user info from page")),
    };
    let loginname_sel = Selector::parse("span.loginname").unwrap();
    let username = doc
        .select(&loginname_sel)
        .next()
        .map(|e| e.text().collect::<String>().trim().to_string())
        .unwrap_or_default();
    Ok(GourmetUserInfo { username, shop_model_id, eater_id, staff_group_id })
}
```

- [ ] **Step 4: Run, expect pass**

Run: `cd src/core && cargo test --lib gourmet`
Expected: all login/user-info tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/core/src/gourmet/parser.rs
git commit -m "feat(core): gourmet parser — is_logged_in (substrings) + user-info extraction"
```

---

### Task 3: `detect_category` + `parse_menu_items` + `has_next_menu_page`

**Files:**
- Modify: `src/core/src/gourmet/parser.rs`

**Interfaces:**
- Consumes: `MenuItem`, `MenuCategory` (foundation domain), `crate::datetime::parse_menu_date`.
- Produces:
  - `pub fn detect_category(title: &str) -> MenuCategory` — `SUPPE & SALAT` literal first, then `MEN(?:Ü|U)\s+([I]{1,3})` case-insensitive → Menu1/2/3 by roman-numeral length; else `Unknown` (01 §8.3).
  - `pub fn parse_menu_items(html: &str) -> Vec<MenuItem>` — select `div.row.hide-sm-down .meal`; per meal extract id/day/title/subtitle/allergens/available/ordered/price/category; skip items missing id or date (01 §8.2). `day` is the `"YYYY-MM-DD"` key from `parse_menu_date` of the `MM-dd-yyyy` `data-date`.
  - `pub fn has_next_menu_page(html: &str) -> bool` — any `<a>` whose class contains `menues-next`, i.e. `a[class*="menues-next"]` (01 §8.1).

- [ ] **Step 1: Write failing tests**

Add to `tests`:

```rust
const MENUS_PAGE_0: &str = include_str!("../../tests/fixtures/gourmet/menus-page-0.html");
const MENUS_PAGE_1: &str = include_str!("../../tests/fixtures/gourmet/menus-page-1.html");

#[test]
fn detects_categories() {
    assert_eq!(detect_category("MENÜ I"), MenuCategory::Menu1);
    assert_eq!(detect_category("MENÜ II"), MenuCategory::Menu2);
    assert_eq!(detect_category("menü iii"), MenuCategory::Menu3); // case-insensitive
    assert_eq!(detect_category("MENU I"), MenuCategory::Menu1);   // U-without-umlaut
    assert_eq!(detect_category("SUPPE & SALAT heute"), MenuCategory::SoupAndSalad);
    assert_eq!(detect_category("Tagesgericht"), MenuCategory::Unknown);
}

#[test]
fn parses_desktop_meals_only_no_duplicates() {
    let items = parse_menu_items(MENUS_PAGE_0);
    // 7 desktop-layout meals in the fixture (06-testing §6.2).
    assert_eq!(items.len(), 7);
    // ids and the date key are as recorded.
    assert!(items.iter().any(|i| i.id == "menu-001" && i.day == "2026-02-10"));
    // category derived from title
    assert!(items.iter().any(|i| i.category == MenuCategory::Menu1));
}

#[test]
fn next_page_detection() {
    assert!(has_next_menu_page(MENUS_PAGE_0)); // page 0 links to page 1
    assert!(!has_next_menu_page(MENUS_PAGE_1)); // last page, no next link
}
```

> If the fixture's actual desktop-meal count differs from 7, adjust the assertion to the count the fixture contains (open `src/core/tests/fixtures/gourmet/menus-page-0.html` and count `.meal` under `div.row.hide-sm-down`). Do not change the fixture.

- [ ] **Step 2: Run, expect failure**

Run: `cd src/core && cargo test --lib gourmet`
Expected: FAIL.

- [ ] **Step 3: Implement the three functions**

Add to `parser.rs` (imports: `use crate::domain::{MenuCategory, MenuItem}; use crate::datetime::parse_menu_date; use regex::Regex;`). Compile the category regex once with a module-level lazy pattern:

```rust
fn category_regex() -> Regex {
    // MEN + Ü or U + spaces + 1..3 'I's, case-insensitive (01 §8.3).
    Regex::new(r"(?i)MEN(?:Ü|U)\s+([I]{1,3})").unwrap()
}

/// Category from title: literal SUPPE & SALAT first, then the roman-numeral regex (01 §8.3).
pub fn detect_category(title: &str) -> MenuCategory {
    if title.contains("SUPPE & SALAT") {
        return MenuCategory::SoupAndSalad;
    }
    if let Some(caps) = category_regex().captures(title) {
        return match caps.get(1).map(|m| m.as_str().len()).unwrap_or(0) {
            1 => MenuCategory::Menu1,
            2 => MenuCategory::Menu2,
            3 => MenuCategory::Menu3,
            _ => MenuCategory::Unknown,
        };
    }
    MenuCategory::Unknown
}

/// Parse desktop-layout meals only (01 §8.2). Skip meals missing id or date.
pub fn parse_menu_items(html: &str) -> Vec<MenuItem> {
    let doc = Html::parse_document(html);
    let meal_sel = Selector::parse("div.row.hide-sm-down .meal").unwrap();
    let detail_sel = Selector::parse(".open_info.menu-article-detail").unwrap();
    let title_sel = Selector::parse(".title").unwrap();
    let subtitle_sel = Selector::parse(".subtitle").unwrap();
    let allergen_sel = Selector::parse("li.allergen").unwrap();
    let checkbox_sel = Selector::parse(r#"input[type="checkbox"].menu-clicked"#).unwrap();
    let price_sel = Selector::parse(".price span").unwrap();

    let mut items = Vec::new();
    for meal in doc.select(&meal_sel) {
        let detail = match meal.select(&detail_sel).next() {
            Some(d) => d,
            None => continue,
        };
        let id = match detail.value().attr("data-id") {
            Some(v) if !v.is_empty() => v.to_string(),
            _ => continue,
        };
        let day = match detail.value().attr("data-date").and_then(parse_menu_date) {
            Some(k) => k,
            None => continue,
        };
        // title = direct text nodes only of `.title` (exclude nested `.subtitle` div).
        let title = meal
            .select(&title_sel)
            .next()
            .map(|t| direct_text(t))
            .unwrap_or_default();
        let subtitle = meal
            .select(&subtitle_sel)
            .next()
            .map(|s| s.text().collect::<String>().trim().to_string())
            .unwrap_or_default();
        let allergens = meal
            .select(&allergen_sel)
            .next()
            .map(|a| {
                a.text()
                    .collect::<String>()
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let checkbox = meal.select(&checkbox_sel).next();
        let available = checkbox.is_some();
        let ordered = checkbox
            .map(|c| c.value().attr("checked").is_some())
            .unwrap_or(false);
        let price = meal
            .select(&price_sel)
            .next()
            .map(|p| p.text().collect::<String>().trim().to_string())
            .unwrap_or_default();
        let category = detect_category(&title);
        items.push(MenuItem {
            id, day, title, subtitle, allergens, available, ordered, category, price,
        });
    }
    items
}

/// Next-page link: any `<a>` whose class contains "menues-next" (01 §8.1).
pub fn has_next_menu_page(html: &str) -> bool {
    let doc = Html::parse_document(html);
    let sel = Selector::parse(r#"a[class*="menues-next"]"#).unwrap();
    doc.select(&sel).next().is_some()
}

/// Concatenate only the DIRECT child text nodes of an element (excludes nested elements),
/// trimmed. Mirrors v1's "direct text nodes only" title extraction (01 §8.2).
fn direct_text(el: scraper::ElementRef) -> String {
    el.children()
        .filter_map(|c| c.value().as_text().map(|t| t.to_string()))
        .collect::<String>()
        .trim()
        .to_string()
}
```

- [ ] **Step 4: Run, expect pass**

Run: `cd src/core && cargo test --lib gourmet`
Expected: category/menu/pagination tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/core/src/gourmet/parser.rs
git commit -m "feat(core): gourmet parser — category detection, desktop-only meal parse, pagination"
```

---

### Task 4: `parse_ordered_menus` + edit-mode + cancel-form + logout-form

**Files:**
- Modify: `src/core/src/gourmet/parser.rs`

**Interfaces:**
- Consumes: `OrderedMenu` (foundation), `crate::datetime::parse_orders_date`.
- Produces:
  - `pub fn parse_ordered_menus(html: &str) -> Vec<OrderedMenu>` — `div.order-item`; `position_id` from `input[name="cp_PositionId"]`; `eating_cycle_id`/date from prefix inputs (fallback `""`/now); title/subtitle from `.title`/`.subtitle`; `approved` iff a `.fa-check` or `.checkmark` descendant exists (01 §9.1).
  - `pub fn extract_edit_mode(html: &str) -> Option<String>` — the `editMode` hidden input value inside `form.form-toggleEditMode` (01 §9.2).
  - `pub struct CancelFormData { pub position_id: String, pub eating_cycle_id: String, pub date: String, pub ufprt: String, pub ncforminfo: String }` and `pub fn extract_cancel_form_data(html: &str, position_id: &str) -> CoreResult<CancelFormData>` — locate `form#form_{id}_cp` else the form containing `input[name="cp_PositionId"][value="{id}"]`; eating-cycle/date by name prefix with `""` fallback; tokens required else `Could not extract cancel form data for position: {id}` (01 §9.4).
  - `pub fn extract_logout_form_tokens(html: &str) -> CoreResult<(String, String)>` — the form containing `button#btnHeaderLogout` (or a Logout-text button); missing form → `Could not find logout form`, missing tokens → `Could not extract logout form tokens` (01 §11).

- [ ] **Step 1: Write failing tests**

Add to `tests`:

```rust
const ORDERS_PAGE: &str = include_str!("../../tests/fixtures/gourmet/orders-page.html");
const ORDERS_EDIT: &str = include_str!("../../tests/fixtures/gourmet/orders-page-edit-mode.html");

#[test]
fn parses_ordered_menus() {
    let orders = parse_ordered_menus(ORDERS_PAGE);
    assert!(!orders.is_empty());
    // at least one order has a non-empty position id and a title.
    assert!(orders.iter().all(|o| !o.position_id.is_empty()));
}

#[test]
fn edit_mode_value_present_on_orders_pages() {
    // the toggle form echoes editMode ("False" when confirmed view, "True" in edit view).
    assert!(extract_edit_mode(ORDERS_PAGE).is_some());
}

#[test]
fn cancel_form_extraction_or_clear_error() {
    let orders = parse_ordered_menus(ORDERS_EDIT);
    if let Some(first) = orders.first() {
        let data = extract_cancel_form_data(ORDERS_EDIT, &first.position_id).unwrap();
        assert_eq!(data.position_id, first.position_id);
        assert!(!data.ufprt.is_empty());
        assert!(!data.ncforminfo.is_empty());
    }
    // a bogus id yields the exact error.
    let err = extract_cancel_form_data(ORDERS_EDIT, "NOPE-999").unwrap_err();
    assert_eq!(err.to_string(), "Could not extract cancel form data for position: NOPE-999");
}

#[test]
fn logout_tokens_from_authenticated_page() {
    let (ufprt, ncform) = extract_logout_form_tokens(LOGIN_SUCCESS).unwrap();
    assert!(!ufprt.is_empty());
    assert!(!ncform.is_empty());
}
```

> Adjust the `approved`/count assertions to what `orders-page.html` actually contains if the loose assertions above need tightening; inspect the fixture, never edit it.

- [ ] **Step 2: Run, expect failure**

Run: `cd src/core && cargo test --lib gourmet`
Expected: FAIL.

- [ ] **Step 3: Implement the functions**

Add to `parser.rs` (imports: `use crate::domain::OrderedMenu; use crate::datetime::parse_orders_date;`). Provide the `CancelFormData` struct and functions:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct CancelFormData {
    pub position_id: String,
    pub eating_cycle_id: String,
    pub date: String,
    pub ufprt: String,
    pub ncforminfo: String,
}

/// Parse ordered menus (01 §9.1). Approved iff a `.fa-check` or `.checkmark` descendant exists.
pub fn parse_ordered_menus(html: &str) -> Vec<OrderedMenu> {
    let doc = Html::parse_document(html);
    let item_sel = Selector::parse("div.order-item").unwrap();
    let pos_sel = Selector::parse(r#"input[name="cp_PositionId"]"#).unwrap();
    let title_sel = Selector::parse(".title").unwrap();
    let subtitle_sel = Selector::parse(".subtitle").unwrap();
    let approved_sel = Selector::parse(".fa-check, .checkmark").unwrap();

    let mut out = Vec::new();
    for item in doc.select(&item_sel) {
        let position_id = match item.select(&pos_sel).next().and_then(|e| e.value().attr("value")) {
            Some(v) if !v.is_empty() => v.to_string(),
            _ => continue,
        };
        // eating-cycle + date inputs by name prefix within this item.
        let ec_sel = Selector::parse(r#"input[name^="cp_EatingCycleId_"]"#).unwrap();
        let date_sel = Selector::parse(r#"input[name^="cp_Date_"]"#).unwrap();
        let eating_cycle_id = item
            .select(&ec_sel)
            .next()
            .and_then(|e| e.value().attr("value"))
            .unwrap_or("")
            .to_string();
        let date_str = item
            .select(&date_sel)
            .next()
            .and_then(|e| e.value().attr("value"))
            .unwrap_or("");
        let date_epoch_ms = parse_orders_date(date_str).unwrap_or(0);
        let title = item.select(&title_sel).next().map(text_trim).unwrap_or_default();
        let subtitle = item.select(&subtitle_sel).next().map(text_trim).unwrap_or_default();
        let approved = item.select(&approved_sel).next().is_some();
        out.push(OrderedMenu {
            position_id, eating_cycle_id, date_epoch_ms, title, subtitle, approved,
        });
    }
    out
}

/// The `editMode` hidden input value inside the edit-mode toggle form (01 §9.2).
pub fn extract_edit_mode(html: &str) -> Option<String> {
    let doc = Html::parse_document(html);
    let form_sel = Selector::parse("form.form-toggleEditMode").unwrap();
    let input_sel = Selector::parse(r#"input[name="editMode"]"#).unwrap();
    let form = doc.select(&form_sel).next()?;
    form.select(&input_sel).next()?.value().attr("value").map(|s| s.to_string())
}

/// Extract the cancel form for a position (01 §9.4). Tokens required; ec/date default to "".
pub fn extract_cancel_form_data(html: &str, position_id: &str) -> CoreResult<CancelFormData> {
    let doc = Html::parse_document(html);
    // Prefer form#form_{id}_cp, else the form containing the matching cp_PositionId input.
    let by_id = Selector::parse(&format!("form#form_{position_id}_cp")).ok();
    let form = by_id
        .and_then(|s| doc.select(&s).next())
        .or_else(|| {
            let contains = Selector::parse(&format!(
                r#"form:has(input[name="cp_PositionId"][value="{position_id}"])"#
            )).ok()?;
            doc.select(&contains).next()
        })
        .ok_or_else(|| parse_err(format!(
            "Could not extract cancel form data for position: {position_id}"
        )))?;

    let ec_sel = Selector::parse(r#"input[name^="cp_EatingCycleId_"]"#).unwrap();
    let date_sel = Selector::parse(r#"input[name^="cp_Date_"]"#).unwrap();
    let ufprt_sel = Selector::parse(r#"input[name="ufprt"]"#).unwrap();
    let ncform_sel = Selector::parse(r#"input[name="__ncforminfo"]"#).unwrap();

    let eating_cycle_id = form.select(&ec_sel).next().and_then(|e| e.value().attr("value")).unwrap_or("").to_string();
    let date = form.select(&date_sel).next().and_then(|e| e.value().attr("value")).unwrap_or("").to_string();
    let ufprt = form.select(&ufprt_sel).next().and_then(|e| e.value().attr("value"));
    let ncform = form.select(&ncform_sel).next().and_then(|e| e.value().attr("value"));
    let (ufprt, ncform) = match (ufprt, ncform) {
        (Some(u), Some(n)) => (u.to_string(), n.to_string()),
        _ => return Err(parse_err(format!(
            "Could not extract cancel form data for position: {position_id}"
        ))),
    };
    Ok(CancelFormData { position_id: position_id.to_string(), eating_cycle_id, date, ufprt, ncforminfo: ncform })
}

/// Logout-form tokens (01 §11). The form holding the header logout button.
pub fn extract_logout_form_tokens(html: &str) -> CoreResult<(String, String)> {
    let doc = Html::parse_document(html);
    // form containing button#btnHeaderLogout.
    let form = {
        let sel = Selector::parse(r#"form:has(button#btnHeaderLogout)"#).unwrap();
        doc.select(&sel).next()
            // fallback: any form whose button text contains "Logout" (scan buttons).
            .or_else(|| find_logout_form_by_text(&doc))
    }.ok_or_else(|| parse_err("Could not find logout form"))?;

    let ufprt_sel = Selector::parse(r#"input[name="ufprt"]"#).unwrap();
    let ncform_sel = Selector::parse(r#"input[name="__ncforminfo"]"#).unwrap();
    let ufprt = form.select(&ufprt_sel).next().and_then(|e| e.value().attr("value"));
    let ncform = form.select(&ncform_sel).next().and_then(|e| e.value().attr("value"));
    match (ufprt, ncform) {
        (Some(u), Some(n)) => Ok((u.to_string(), n.to_string())),
        _ => Err(parse_err("Could not extract logout form tokens")),
    }
}

fn find_logout_form_by_text<'a>(doc: &'a Html) -> Option<scraper::ElementRef<'a>> {
    let form_sel = Selector::parse("form").unwrap();
    let button_sel = Selector::parse("button").unwrap();
    doc.select(&form_sel).find(|form| {
        form.select(&button_sel)
            .any(|b| b.text().collect::<String>().contains("Logout"))
    })
}

fn text_trim(el: scraper::ElementRef) -> String {
    el.text().collect::<String>().trim().to_string()
}
```

- [ ] **Step 4: Run, expect pass**

Run: `cd src/core && cargo test --lib gourmet`
Expected: all parser tests PASS.

- [ ] **Step 5: Full crate test + fmt/clippy**

Run: `cd src/core && cargo test && cargo fmt --check && cargo clippy --all-targets`
Expected: all tests pass; fmt clean; clippy no warnings. (If `cargo fmt --check` fails, run `cargo fmt` and re-run tests.)

- [ ] **Step 6: Commit**

```bash
git add src/core
git commit -m "feat(core): gourmet parser — ordered menus, edit-mode, cancel-form, logout-form"
```

---

## Post-plan: next Gourmet sub-plans

- **Phase 1b-ii — `gourmet::client`**: the `get`/`postForm`/`postJson` primitives over the foundation `Transport`, with `Origin`/`Referer`/`lastPageUrl` rules and the reqwest cookie store (01 §2). Request-shape tests via `CapturingTransport`.
- **Phase 1b-iii — `gourmet::api`**: orchestration — login (incl. stale-session pre-logout), `ensureSession`, `getMenus` pagination, `getOrders`, `addToCart`, `confirmOrders`, `cancelOrders` (edit-mode loop), `getBillings`, logout. Full sequence tests per 06-testing §6.1.

## Self-review notes

- **Spec coverage:** this plan covers 01-gourmet-scraping §3 (tokens), §4 (isLoggedIn), §5 (user info), §8.2/§8.3 (menu parse + category), §8.1 (pagination), §9.1 (ordered menus + approval markers), §9.2 (edit-mode), §9.4 (cancel form), §11 (logout tokens) — the entire parser surface. The client/api request sequences are explicitly deferred to 1b-ii/1b-iii.
- **`scraper` `:has()` support:** `scraper` 0.20 supports `:has()`; if a target version rejects it, the cancel-form and logout-form lookups fall back to the explicit scan helpers already shown (`find_logout_form_by_text` pattern) — implement the scan variant if `Selector::parse` errors on `:has`.
- **Type consistency:** `extract_form_tokens`, `is_logged_in`, `extract_user_info`, `detect_category`, `parse_menu_items`, `has_next_menu_page`, `parse_ordered_menus`, `extract_edit_mode`, `extract_cancel_form_data`/`CancelFormData`, `extract_logout_form_tokens` are the public surface 1b-iii (`gourmet::api`) will consume; all use foundation types (`GourmetUserInfo`, `MenuItem`, `MenuCategory`, `OrderedMenu`, `CoreError`).
