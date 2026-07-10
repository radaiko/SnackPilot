//! Pure Gourmet HTML parsing (01-gourmet-scraping.md §3-§11). No network, no async.
use crate::datetime::{parse_menu_date, parse_orders_date};
use crate::domain::{GourmetUserInfo, MenuCategory, MenuItem, OrderedMenu};
use crate::error::{CoreError, CoreResult};
use regex::Regex;
use scraper::{ElementRef, Html, Selector};

fn parse_err(msg: impl Into<String>) -> CoreError {
    CoreError::Parse { detail: msg.into() }
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
        .ok_or_else(|| {
            parse_err(format!(
                "Could not find __ncforminfo in form: {form_selector}"
            ))
        })?;
    Ok((ufprt.to_string(), ncform.to_string()))
}

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
        doc.select(&sel)
            .next()?
            .value()
            .attr("value")
            .map(|s| s.to_string())
    };
    let shop_model_id = attr_value("shopModel");
    let eater_id = attr_value("eater");
    let staff_group_id = attr_value("staffGroup");
    let (shop_model_id, eater_id, staff_group_id) = match (shop_model_id, eater_id, staff_group_id)
    {
        (Some(s), Some(e), Some(g)) => (s, e, g),
        _ => return Err(parse_err("Could not extract user info from page")),
    };
    let loginname_sel = Selector::parse("span.loginname").unwrap();
    let username = doc
        .select(&loginname_sel)
        .next()
        .map(text_trim)
        .unwrap_or_default();
    Ok(GourmetUserInfo {
        username,
        shop_model_id,
        eater_id,
        staff_group_id,
    })
}

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
            .map(direct_text)
            .unwrap_or_default();
        let subtitle = meal
            .select(&subtitle_sel)
            .next()
            .map(text_trim)
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
            .map(text_trim)
            .unwrap_or_default();
        let category = detect_category(&title);
        items.push(MenuItem {
            id,
            day,
            title,
            subtitle,
            allergens,
            available,
            ordered,
            category,
            price,
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

#[derive(Debug, Clone, PartialEq)]
pub struct CancelFormData {
    pub position_id: String,
    pub eating_cycle_id: String,
    pub date: String,
    pub ufprt: String,
    pub ncforminfo: String,
}

/// Parse ordered menus (01 §9.1). Approved iff a `.fa-check` or `.checkmark` descendant exists.
/// `now_epoch_ms` is the fallback date for an order-item missing its `cp_Date_` input (matches v1's
/// `new Date()` — G-2; without it a missing date became epoch 0 and misfiled the order as past).
pub fn parse_ordered_menus(html: &str, now_epoch_ms: i64) -> Vec<OrderedMenu> {
    let doc = Html::parse_document(html);
    // Match v1's `div.order-item, div[class*="order-item"]` (G-1) so hyphenated variant classes
    // (e.g. `order-item-cancelled`) aren't missed.
    let item_sel = Selector::parse(r#"div.order-item, div[class*="order-item"]"#).unwrap();
    let pos_sel = Selector::parse(r#"input[name="cp_PositionId"]"#).unwrap();
    let ec_sel = Selector::parse(r#"input[name^="cp_EatingCycleId_"]"#).unwrap();
    let date_sel = Selector::parse(r#"input[name^="cp_Date_"]"#).unwrap();
    let title_sel = Selector::parse(".title").unwrap();
    let subtitle_sel = Selector::parse(".subtitle").unwrap();
    let approved_sel = Selector::parse(".fa-check, .checkmark").unwrap();

    let mut out = Vec::new();
    for item in doc.select(&item_sel) {
        let position_id = match item
            .select(&pos_sel)
            .next()
            .and_then(|e| e.value().attr("value"))
        {
            Some(v) if !v.is_empty() => v.to_string(),
            _ => continue,
        };
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
        let date_epoch_ms = parse_orders_date(date_str).unwrap_or(now_epoch_ms);
        let title = item
            .select(&title_sel)
            .next()
            .map(text_trim)
            .unwrap_or_default();
        let subtitle = item
            .select(&subtitle_sel)
            .next()
            .map(text_trim)
            .unwrap_or_default();
        let approved = item.select(&approved_sel).next().is_some();
        out.push(OrderedMenu {
            position_id,
            eating_cycle_id,
            date_epoch_ms,
            title,
            subtitle,
            approved,
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
    form.select(&input_sel)
        .next()?
        .value()
        .attr("value")
        .map(|s| s.to_string())
}

/// Extract the cancel form for a position (01 §9.4). Tokens required; ec/date default to "".
pub fn extract_cancel_form_data(html: &str, position_id: &str) -> CoreResult<CancelFormData> {
    let doc = Html::parse_document(html);
    let not_found = || {
        parse_err(format!(
            "Could not extract cancel form data for position: {position_id}"
        ))
    };
    // Prefer form#form_{id}_cp, else the form containing the matching cp_PositionId input.
    let form = Selector::parse(&format!("form#form_{position_id}_cp"))
        .ok()
        .and_then(|s| doc.select(&s).next())
        .or_else(|| find_cancel_form_by_position(&doc, position_id))
        .ok_or_else(not_found)?;

    let ec_sel = Selector::parse(r#"input[name^="cp_EatingCycleId_"]"#).unwrap();
    let date_sel = Selector::parse(r#"input[name^="cp_Date_"]"#).unwrap();
    let ufprt_sel = Selector::parse(r#"input[name="ufprt"]"#).unwrap();
    let ncform_sel = Selector::parse(r#"input[name="__ncforminfo"]"#).unwrap();

    let eating_cycle_id = form
        .select(&ec_sel)
        .next()
        .and_then(|e| e.value().attr("value"))
        .unwrap_or("")
        .to_string();
    let date = form
        .select(&date_sel)
        .next()
        .and_then(|e| e.value().attr("value"))
        .unwrap_or("")
        .to_string();
    let ufprt = form
        .select(&ufprt_sel)
        .next()
        .and_then(|e| e.value().attr("value"));
    let ncform = form
        .select(&ncform_sel)
        .next()
        .and_then(|e| e.value().attr("value"));
    let (ufprt, ncform) = match (ufprt, ncform) {
        (Some(u), Some(n)) => (u.to_string(), n.to_string()),
        _ => return Err(not_found()),
    };
    Ok(CancelFormData {
        position_id: position_id.to_string(),
        eating_cycle_id,
        date,
        ufprt,
        ncforminfo: ncform,
    })
}

/// Logout-form tokens (01 §11). The form holding the header logout button.
pub fn extract_logout_form_tokens(html: &str) -> CoreResult<(String, String)> {
    let doc = Html::parse_document(html);
    let form = Selector::parse(r#"form:has(button#btnHeaderLogout)"#)
        .ok()
        .and_then(|s| doc.select(&s).next())
        .or_else(|| find_logout_form_by_text(&doc))
        .ok_or_else(|| parse_err("Could not find logout form"))?;

    let ufprt_sel = Selector::parse(r#"input[name="ufprt"]"#).unwrap();
    let ncform_sel = Selector::parse(r#"input[name="__ncforminfo"]"#).unwrap();
    let ufprt = form
        .select(&ufprt_sel)
        .next()
        .and_then(|e| e.value().attr("value"));
    let ncform = form
        .select(&ncform_sel)
        .next()
        .and_then(|e| e.value().attr("value"));
    match (ufprt, ncform) {
        (Some(u), Some(n)) => Ok((u.to_string(), n.to_string())),
        _ => Err(parse_err("Could not extract logout form tokens")),
    }
}

fn find_cancel_form_by_position<'a>(doc: &'a Html, position_id: &str) -> Option<ElementRef<'a>> {
    let form_sel = Selector::parse("form").unwrap();
    let pos_sel = Selector::parse(r#"input[name="cp_PositionId"]"#).unwrap();
    doc.select(&form_sel).find(|form| {
        form.select(&pos_sel)
            .any(|i| i.value().attr("value") == Some(position_id))
    })
}

fn find_logout_form_by_text<'a>(doc: &'a Html) -> Option<ElementRef<'a>> {
    let form_sel = Selector::parse("form").unwrap();
    let button_sel = Selector::parse("button").unwrap();
    doc.select(&form_sel).find(|form| {
        form.select(&button_sel)
            .any(|b| b.text().collect::<String>().contains("Logout"))
    })
}

/// Concatenate only the DIRECT child text nodes of an element (excludes nested elements),
/// trimmed. Mirrors v1's "direct text nodes only" title extraction (01 §8.2).
fn direct_text(el: ElementRef) -> String {
    el.children()
        .filter_map(|c| c.value().as_text().map(|t| t.to_string()))
        .collect::<String>()
        .trim()
        .to_string()
}

fn text_trim(el: ElementRef) -> String {
    el.text().collect::<String>().trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOGIN_PAGE: &str = include_str!("../../tests/fixtures/gourmet/login-page.html");
    const LOGIN_SUCCESS: &str = include_str!("../../tests/fixtures/gourmet/login-success.html");
    const LOGIN_FAILED: &str = include_str!("../../tests/fixtures/gourmet/login-failed.html");
    const MENUS_PAGE_0: &str = include_str!("../../tests/fixtures/gourmet/menus-page-0.html");
    const MENUS_PAGE_1: &str = include_str!("../../tests/fixtures/gourmet/menus-page-1.html");
    const ORDERS_PAGE: &str = include_str!("../../tests/fixtures/gourmet/orders-page.html");
    const ORDERS_EDIT: &str =
        include_str!("../../tests/fixtures/gourmet/orders-page-edit-mode.html");

    #[test]
    fn extracts_login_form_tokens() {
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

    #[test]
    fn is_logged_in_true_on_authenticated_page() {
        assert!(is_logged_in(LOGIN_SUCCESS));
    }

    #[test]
    fn is_logged_in_false_on_login_and_failed_pages() {
        assert!(!is_logged_in(LOGIN_PAGE));
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

    #[test]
    fn detects_categories() {
        assert_eq!(detect_category("MENÜ I"), MenuCategory::Menu1);
        assert_eq!(detect_category("MENÜ II"), MenuCategory::Menu2);
        assert_eq!(detect_category("menü iii"), MenuCategory::Menu3);
        assert_eq!(detect_category("MENU I"), MenuCategory::Menu1);
        assert_eq!(
            detect_category("SUPPE & SALAT heute"),
            MenuCategory::SoupAndSalad
        );
        assert_eq!(detect_category("Tagesgericht"), MenuCategory::Unknown);
    }

    #[test]
    fn parses_desktop_meals_only_no_duplicates() {
        let items = parse_menu_items(MENUS_PAGE_0);
        assert_eq!(items.len(), 7);
        assert!(items
            .iter()
            .any(|i| i.id == "menu-001" && i.day == "2026-02-10"));
        assert!(items.iter().any(|i| i.category == MenuCategory::Menu1));
    }

    #[test]
    fn next_page_detection() {
        assert!(has_next_menu_page(MENUS_PAGE_0));
        assert!(!has_next_menu_page(MENUS_PAGE_1));
    }

    #[test]
    fn parses_ordered_menus() {
        let orders = parse_ordered_menus(ORDERS_PAGE, 0);
        assert!(!orders.is_empty());
        assert!(orders.iter().all(|o| !o.position_id.is_empty()));
    }

    #[test]
    fn edit_mode_value_present_on_orders_pages() {
        assert!(extract_edit_mode(ORDERS_PAGE).is_some());
    }

    #[test]
    fn cancel_form_extraction_or_clear_error() {
        let orders = parse_ordered_menus(ORDERS_EDIT, 0);
        if let Some(first) = orders.first() {
            let data = extract_cancel_form_data(ORDERS_EDIT, &first.position_id).unwrap();
            assert_eq!(data.position_id, first.position_id);
            assert!(!data.ufprt.is_empty());
            assert!(!data.ncforminfo.is_empty());
        }
        let err = extract_cancel_form_data(ORDERS_EDIT, "NOPE-999").unwrap_err();
        assert_eq!(
            err.to_string(),
            "Could not extract cancel form data for position: NOPE-999"
        );
    }

    #[test]
    fn logout_tokens_from_authenticated_page() {
        let (ufprt, ncform) = extract_logout_form_tokens(LOGIN_SUCCESS).unwrap();
        assert!(!ufprt.is_empty());
        assert!(!ncform.is_empty());
    }
}
