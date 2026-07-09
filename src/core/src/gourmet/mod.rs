//! Gourmet (Kantine) scraping — client, parser, API (docs/requirements/01-gourmet-scraping.md).
pub mod client;
pub mod parser;

/// 01-gourmet-scraping §1.
pub const GOURMET_BASE_URL: &str = "https://alaclickneu.gourmet.at";
pub const GOURMET_ORIGIN: &str = "https://alaclickneu.gourmet.at";
