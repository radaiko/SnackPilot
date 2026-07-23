//! Gourmet (Kantine) scraping — client, parser, API (docs/requirements/01-gourmet-scraping.md).
pub mod api;
pub mod client;
pub mod parser;
pub mod provider;

/// 01-gourmet-scraping §1.
pub const GOURMET_BASE_URL: &str = "https://alaclickneu.gourmet.at";
pub const GOURMET_ORIGIN: &str = "https://alaclickneu.gourmet.at";
/// GET entry point. Still 302-redirects to `/de/startseite/` (which carries the login form
/// and, when authenticated, the logout markers). Kept for stale-session detection + Referer.
pub const GOURMET_LOGIN_URL: &str = "https://alaclickneu.gourmet.at/start/";
/// The rebuilt AngularJS site posts credentials here as `application/x-www-form-urlencoded`
/// (`Email`/`Password`/`RememberMe`), with NO `ufprt`/`__ncforminfo` tokens — verified live
/// 2026-07-23. Do not revert to the old multipart `/start/` POST (see design doc).
pub const GOURMET_LOGIN_SUBMIT_URL: &str =
    "https://alaclickneu.gourmet.at/Controller/AlaLogin/Submit";
/// Logout is now a tokenless empty POST here (was a multipart `/start/` POST with tokens).
pub const GOURMET_LOGOUT_URL: &str =
    "https://alaclickneu.gourmet.at/Controller/AlaLogin/SubmitLogout";
/// Menus. MUST be the rebuilt `/de/menues/` path, NOT the old `/menus/`: the old path
/// 301-redirects to `/de/menues/` and **drops the `?page=N` query string**, so every paginated
/// request returned page 0 (and the "next" link is always present) → the pager looped all 10
/// pages and every menu appeared ~10×. Verified live 2026-07-23.
pub const GOURMET_MENUS_URL: &str = "https://alaclickneu.gourmet.at/de/menues/";
pub const GOURMET_ORDERS_URL: &str = "https://alaclickneu.gourmet.at/de/bestellungen/";
/// Cancel a placed order position. Rebuilt site: urlencoded, tokenless POST with
/// `cp_PositionId`/`cp_EatingCycleId_{id}`/`cp_Date_{id}` (was a multipart POST to `/bestellungen/`
/// with `ufprt`/`__ncforminfo`). The cancel form renders whenever an order exists — no edit-mode
/// toggle required. Verified live 2026-07-23.
pub const GOURMET_CANCEL_POSITION_URL: &str =
    "https://alaclickneu.gourmet.at/Controller/AlaMyOrders/CancelPosition";
/// Toggle order edit-mode / confirm. Rebuilt site: urlencoded, tokenless POST with the current
/// `editMode` value (was a multipart POST to `/bestellungen/` with tokens).
pub const GOURMET_TOGGLE_EDIT_MODE_URL: &str =
    "https://alaclickneu.gourmet.at/Controller/AlaMyOrders/ToggleEditMode";
pub const GOURMET_ADD_TO_CART_URL: &str =
    "https://alaclickneu.gourmet.at/umbraco/api/AlaCartApi/AddToMenuesCart";
pub const GOURMET_BILLING_URL: &str =
    "https://alaclickneu.gourmet.at/umbraco/api/AlaMyBillingApi/GetMyBillings";
