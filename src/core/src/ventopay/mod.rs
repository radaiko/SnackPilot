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
