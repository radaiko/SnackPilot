use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderedMenu {
    pub position_id: String,
    pub eating_cycle_id: String,
    pub date_epoch_ms: i64,
    pub title: String,
    pub subtitle: String,
    pub approved: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrdersSplit {
    pub upcoming: Vec<OrderedMenu>,
    pub past: Vec<OrderedMenu>,
}
