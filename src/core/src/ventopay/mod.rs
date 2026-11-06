//! Ventopay (Automaten) client, parser, API (docs/requirements/02-ventopay-scraping.md).
//! The Ventopay "xperience" platform replaced the old mocca.website ASP.NET portal
//! (2026-11): JSON auth service + OpenAPI gateway, no HTML scraping.
pub mod api;
pub mod client;
pub mod parser;
pub mod provider;

/// 02-ventopay-scraping §1. SPA shell + auth service host.
pub const VENTOPAY_WEB_URL: &str = "https://xperience-trumpf.ventopay.cloud";
/// 02-ventopay-scraping §1. Business API gateway (from mocca.web.xml <APIEndpoint>).
pub const VENTOPAY_API_URL: &str = "https://gw-trumpf.ventopay.cloud";
pub const VENTOPAY_LOGIN_URL: &str = "https://xperience-trumpf.ventopay.cloud/auth/login";
pub const VENTOPAY_ACCESS_TOKEN_URL: &str =
    "https://xperience-trumpf.ventopay.cloud/auth/accessToken";
pub const VENTOPAY_LOGOUT_URL: &str = "https://xperience-trumpf.ventopay.cloud/auth/logout";
pub const VENTOPAY_TRANSACTIONS_URL: &str =
    "https://gw-trumpf.ventopay.cloud/Customers/v1/person/bons";
/// Origin/Referer host for the auth POSTs (02-ventopay-scraping §2.4).
pub const VENTOPAY_ORIGIN: &str = "https://xperience-trumpf.ventopay.cloud";
/// Restaurant selector sent on every login (02-ventopay-scraping §3). Same UUID the old
/// portal's company dropdown used; served live by GET /Restaurant/v1/restaurants.
pub const VENTOPAY_RESTAURANT_ID: &str = "0da8d3ec-0178-47d5-9ccd-a996f04acb61";
