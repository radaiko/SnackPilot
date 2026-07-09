#[derive(Debug, Clone, PartialEq)]
pub struct BillingItem {
    pub id: String,
    pub article_id: String,
    pub count: i64,
    pub description: String,
    pub total: f64,
    pub subsidy: f64,
    pub discount_value: f64,
    pub is_custom_menu: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Bill {
    pub bill_nr: i64,
    pub bill_date_epoch_ms: i64,
    pub location: String,
    pub items: Vec<BillingItem>,
    pub billing: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GourmetMonthlyBilling {
    pub month_key: String,
    pub label: String,
    pub bills: Vec<Bill>,
    pub total_gross: f64,
    pub total_subsidy: f64,
    pub total_discount: f64,
    pub total_billing: f64,
    pub fetched_at: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VentopayTransaction {
    pub id: String,
    pub date_epoch_ms: i64,
    pub amount: f64,
    pub restaurant: String,
    pub location: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VentopayMonthlyBilling {
    pub month_key: String,
    pub label: String,
    pub transactions: Vec<VentopayTransaction>,
    pub total: f64,
    pub fetched_at: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MonthOption {
    pub key: String,
    pub label: String,
    pub offset: u8,
}
