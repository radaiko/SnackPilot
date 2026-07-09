//! Pure Ventopay HTML/text parsing (02-ventopay-scraping §3-§6). No network, no async.
use crate::datetime::{local_epoch_from_parts, parse_bill_date};
use crate::domain::VentopayTransaction;
use crate::error::{CoreError, CoreResult};
use regex::Regex;
use scraper::{Html, Selector};

fn parse_err(msg: impl Into<String>) -> CoreError {
    CoreError::Parse {
        message: msg.into(),
    }
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
            Some(c) => (parse_german_amount(&c[1]), c[2].trim().to_string()),
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
    let re =
        Regex::new(r"(\d{1,2})\.\s*(\p{L}{3})\p{L}*\s+(\d{4})\s*-\s*(\d{1,2}):(\d{2})").unwrap();
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

#[cfg(test)]
mod tests {
    use super::*;

    const LOGIN_PAGE: &str = include_str!("../../tests/fixtures/ventopay/login-page.html");
    const LOGIN_SUCCESS: &str = include_str!("../../tests/fixtures/ventopay/login-success.html");
    const TX_PAGE: &str = include_str!("../../tests/fixtures/ventopay/transactions-page.html");
    const TX_EMPTY: &str = include_str!("../../tests/fixtures/ventopay/transactions-empty.html");

    #[test]
    fn extracts_aspnet_state() {
        let s = extract_aspnet_state(LOGIN_PAGE).unwrap();
        assert_eq!(s.viewstate, "VIEWSTATE-TOKEN-LONG-BASE64-STRING-ABC123");
        assert_eq!(s.viewstate_generator, "ABCD1234");
        assert_eq!(s.event_validation, "EVENTVALIDATION-TOKEN-XYZ789");
    }

    #[test]
    fn missing_required_state_errors() {
        let html =
            r#"<input id="__VIEWSTATE" value=""><input id="__VIEWSTATEGENERATOR" value="g">"#;
        let err = extract_aspnet_state(html).unwrap_err();
        assert_eq!(err.to_string(), "Could not extract ASP.NET state from page");
    }

    #[test]
    fn login_check_matches_logout_link() {
        assert!(is_logged_in(LOGIN_SUCCESS));
        assert!(is_logged_in(r#"<a href="Ausloggen.aspx">x</a>"#));
        assert!(!is_logged_in(LOGIN_PAGE));
    }

    #[test]
    fn parses_transactions_and_applies_gourmet_filter() {
        let txs = parse_transactions(TX_PAGE, 0);
        assert_eq!(txs.len(), 5);
        assert!(txs
            .iter()
            .all(|t| !t.restaurant.to_lowercase().contains("gourmet")));
        let first = &txs[0];
        assert_eq!(first.id, "dHhuLTAwMQ==");
        assert!((first.amount - 1.8).abs() < 1e-9);
        assert_eq!(first.restaurant, "Café + Co. Automaten");
        assert_eq!(first.location, first.restaurant);
    }

    #[test]
    fn empty_page_yields_no_transactions() {
        assert_eq!(parse_transactions(TX_EMPTY, 0).len(), 0);
    }

    #[test]
    fn german_month_variants_parse() {
        assert_eq!(german_month("jän"), 0);
        assert_eq!(german_month("mrz"), 2);
        assert_eq!(german_month("mär"), 2);
        assert_eq!(german_month("zzz"), 0);
    }

    #[test]
    fn german_amount_prefix_parse() {
        assert!((parse_german_amount("€ 1,80") - 1.8).abs() < 1e-9);
        assert!((parse_german_amount("0,50") - 0.5).abs() < 1e-9);
        assert!((parse_german_amount("1,80 / 2,00") - 1.802).abs() < 1e-9);
        assert_eq!(parse_german_amount("garbage"), 0.0);
    }

    #[test]
    fn title_fallback_when_no_paren_format() {
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
        assert_eq!(txs.len(), 1);
        assert_eq!(txs[0].id, "z");
        assert_eq!(txs[0].date_epoch_ms, now);
    }
}
