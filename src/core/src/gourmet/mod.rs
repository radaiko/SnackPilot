//! Gourmet (Kantine) scraping — client, parser, API (docs/requirements/01-gourmet-scraping.md).
pub mod api;
pub mod client;
pub mod parser;

/// 01-gourmet-scraping §1.
pub const GOURMET_BASE_URL: &str = "https://alaclickneu.gourmet.at";
pub const GOURMET_ORIGIN: &str = "https://alaclickneu.gourmet.at";
pub const GOURMET_LOGIN_URL: &str = "https://alaclickneu.gourmet.at/start/";
pub const GOURMET_MENUS_URL: &str = "https://alaclickneu.gourmet.at/menus/";
pub const GOURMET_ORDERS_URL: &str = "https://alaclickneu.gourmet.at/bestellungen/";
pub const GOURMET_ADD_TO_CART_URL: &str =
    "https://alaclickneu.gourmet.at/umbraco/api/AlaCartApi/AddToMenuesCart";
pub const GOURMET_BILLING_URL: &str =
    "https://alaclickneu.gourmet.at/umbraco/api/AlaMyBillingApi/GetMyBillings";
