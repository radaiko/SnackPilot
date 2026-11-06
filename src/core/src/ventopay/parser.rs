//! Pure Ventopay JSON parsing (02-ventopay-scraping §3, §5). No network, no async.
use crate::datetime::parse_bill_date;
use crate::domain::VentopayTransaction;
use crate::error::{CoreError, CoreResult};
use serde::Deserialize;

fn parse_err(msg: impl Into<String>) -> CoreError {
    CoreError::Parse {
        detail: format!("Ventopay: {}", msg.into()),
    }
}

/// §3 — login response. `otP_ID` non-null means 2FA, which the app does not support.
#[derive(Debug, Deserialize)]
struct LoginResponse {
    #[serde(rename = "otP_ID")]
    otp_id: Option<serde_json::Value>,
}

/// §3 — validate the POST /auth/login response. Errors: 2FA unsupported.
pub fn check_login_response(json: &str) -> CoreResult<()> {
    let resp: LoginResponse =
        serde_json::from_str(json).map_err(|e| parse_err(format!("login response: {e}")))?;
    if matches!(&resp.otp_id, Some(v) if !v.is_null()) {
        return Err(CoreError::LoginFailed {
            detail: "Ventopay login failed: two-factor authentication is not supported".into(),
        });
    }
    Ok(())
}

/// §3 — extract `accessToken` from GET /auth/accessToken.
pub fn parse_access_token(json: &str) -> CoreResult<String> {
    let v: serde_json::Value =
        serde_json::from_str(json).map_err(|e| parse_err(format!("access token: {e}")))?;
    v.get("accessToken")
        .and_then(|t| t.as_str())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .ok_or_else(|| parse_err("accessToken missing from response"))
}

#[derive(Debug, Deserialize)]
struct BonsResponse {
    #[serde(default)]
    error: String,
    #[serde(default)]
    bons: Vec<Bon>,
}

#[derive(Debug, Deserialize)]
struct Bon {
    #[serde(default)]
    id: String,
    #[serde(rename = "bonTotal", default)]
    bon_total: f64,
    #[serde(rename = "restaurantName", default)]
    restaurant_name: String,
    #[serde(rename = "originalBonCreationDate", default)]
    original_bon_creation_date: String,
}

/// §5 — parse the bons list. `now_epoch_ms` is used when the date is missing or unparseable.
/// Skips bons without an id and Gourmet bons (the app covers Gourmet separately).
pub fn parse_transactions(json: &str, now_epoch_ms: i64) -> CoreResult<Vec<VentopayTransaction>> {
    let resp: BonsResponse =
        serde_json::from_str(json).map_err(|e| parse_err(format!("bons response: {e}")))?;
    if resp.error != "ERR_NONE" {
        return Err(parse_err(format!("bons error: {}", resp.error)));
    }
    let mut out = Vec::new();
    for b in resp.bons {
        if b.id.is_empty() {
            continue; // no id (§5.1)
        }
        if b.restaurant_name.to_lowercase().contains("gourmet") {
            continue; // Gourmet filter (§5.2)
        }
        let date_epoch_ms = parse_bill_date(&b.original_bon_creation_date).unwrap_or(now_epoch_ms);
        out.push(VentopayTransaction {
            id: b.id,
            date_epoch_ms,
            amount: b.bon_total,
            restaurant: b.restaurant_name.clone(),
            location: b.restaurant_name, // §5.3 location == restaurant
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BONS: &str = include_str!("../../tests/fixtures/ventopay/bons.json");
    const BONS_EMPTY: &str = include_str!("../../tests/fixtures/ventopay/bons-empty.json");

    #[test]
    fn login_response_ok_without_otp() {
        let json = r#"{"person":{"id":"p"},"otP_ID":null,"entityType":1}"#;
        assert!(check_login_response(json).is_ok());
    }

    #[test]
    fn login_response_rejects_otp() {
        let json = r#"{"otP_ID":"abc-123"}"#;
        let err = check_login_response(json).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Ventopay login failed: two-factor authentication is not supported"
        );
    }

    #[test]
    fn access_token_extracted() {
        assert_eq!(
            parse_access_token(r#"{"accessToken":"TOK","expires":1}"#).unwrap(),
            "TOK"
        );
        assert!(parse_access_token(r#"{"expires":1}"#).is_err());
        assert!(parse_access_token("not json").is_err());
    }

    #[test]
    fn parses_transactions_and_applies_gourmet_filter() {
        let txs = parse_transactions(BONS, 0).unwrap();
        assert_eq!(txs.len(), 5);
        assert!(txs
            .iter()
            .all(|t| !t.restaurant.to_lowercase().contains("gourmet")));
        let first = &txs[0];
        assert_eq!(first.id, "687efd5d-0fe5-4149-9e61-7183b9a616a3");
        assert!((first.amount - 1.8).abs() < 1e-9);
        assert_eq!(first.restaurant, "Café + Co. Automaten");
        assert_eq!(first.location, first.restaurant);
        // 09. Feb 2026 11:49 local
        assert_eq!(
            first.date_epoch_ms,
            crate::datetime::local_epoch_from_parts(2026, 2, 9, 11, 49).unwrap()
        );
    }

    #[test]
    fn error_code_fails_parse() {
        let err = parse_transactions(r#"{"error":"ERR_UNKNOWN","bons":[]}"#, 0).unwrap_err();
        assert_eq!(err.to_string(), "Ventopay: bons error: ERR_UNKNOWN");
    }

    #[test]
    fn empty_list_yields_no_transactions() {
        assert_eq!(parse_transactions(BONS_EMPTY, 0).unwrap().len(), 0);
    }

    #[test]
    fn skips_idless_and_bad_date_uses_now() {
        let json = r#"{"error":"ERR_NONE","bons":[
            {"id":"","bonTotal":1.0,"restaurantName":"A","originalBonCreationDate":"2026-02-09T11:49:00"},
            {"id":"z","bonTotal":2.0,"restaurantName":"B","originalBonCreationDate":"garbage"}]}"#;
        let now = 1_700_000_000_000;
        let txs = parse_transactions(json, now).unwrap();
        assert_eq!(txs.len(), 1);
        assert_eq!(txs[0].id, "z");
        assert_eq!(txs[0].date_epoch_ms, now);
    }

    #[test]
    fn scientific_notation_amount_parses() {
        // .NET serialises zero as 0E-10 — valid JSON number.
        let json = r#"{"error":"ERR_NONE","bons":[
            {"id":"a","bonTotal":0E-10,"restaurantName":"B","originalBonCreationDate":"2026-02-09T11:49:00"}]}"#;
        let txs = parse_transactions(json, 0).unwrap();
        assert_eq!(txs[0].amount, 0.0);
    }
}
