use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
pub enum MenuCategory {
    Menu1,
    Menu2,
    Menu3,
    SoupAndSalad,
    Unknown,
}

impl MenuCategory {
    /// Exact user-visible category label (03-features/menus.md §1).
    pub fn display(&self) -> &'static str {
        match self {
            MenuCategory::Menu1 => "MENÜ I",
            MenuCategory::Menu2 => "MENÜ II",
            MenuCategory::Menu3 => "MENÜ III",
            MenuCategory::SoupAndSalad => "SUPPE & SALAT",
            MenuCategory::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Record)]
pub struct MenuItem {
    pub id: String,
    /// Local-date key "YYYY-MM-DD" (menus.md §1).
    pub day: String,
    pub title: String,
    pub subtitle: String,
    pub allergens: Vec<String>,
    pub available: bool,
    pub ordered: bool,
    pub category: MenuCategory,
    pub price: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum OrderProgress {
    Adding,
    Confirming,
    Cancelling,
    Refreshing,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct MenuSnapshot {
    pub items: Vec<MenuItem>,
    pub available_dates: Vec<String>,
    pub pending_orders: Vec<String>,
    pub pending_cancellations: Vec<String>,
    pub loading: bool,
    pub refreshing: bool,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_display_strings_match_v1() {
        // 03-features/menus.md §1 — exact display strings owned by the core.
        assert_eq!(MenuCategory::Menu1.display(), "MENÜ I");
        assert_eq!(MenuCategory::Menu2.display(), "MENÜ II");
        assert_eq!(MenuCategory::Menu3.display(), "MENÜ III");
        assert_eq!(MenuCategory::SoupAndSalad.display(), "SUPPE & SALAT");
        assert_eq!(MenuCategory::Unknown.display(), "UNKNOWN");
    }
}
