//! Billing state and actions: ports `selectMonth`/`reloadBilling` from
//! `AppModel.swift` and the derived entry list and totals of
//! `BillingView.swift` (billing §6.2/§6.3) into the model.

use crate::runtime::handle;
use crate::session::AppModel;
use crate::state::{BillingSource, ErrorText};

/// Where a merged billing entry came from (BillingView `Entry.source`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntrySource {
    Gourmet,
    Ventopay,
}

/// An expandable line inside a Gourmet bill card (BillingView `ItemLine`).
#[derive(Debug, Clone, PartialEq)]
pub struct BillingLine {
    pub count: i64,
    pub description: String,
    pub total: f64,
}

/// One row of the unified Abrechnung list (BillingView `Entry`).
#[derive(Debug, Clone, PartialEq)]
pub struct BillingEntry {
    pub id: String,
    pub epoch_ms: i64,
    pub source: EntrySource,
    pub description: String,
    pub amount: f64,
    pub items: Vec<BillingLine>,
}

/// The Abrechnung summary (04-ui-ux.md §3 "Gesamt / Belege / Zuschuss").
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BillingTotals {
    pub gesamt: f64,
    pub belege: usize,
    pub zuschuss: f64,
}

impl AppModel {
    /// Swift `selectMonth` (:546): select a month offset and load its billing.
    pub fn select_month(&self, offset: u8) {
        self.hub().update("selected_offset", move |s| {
            s.selected_offset = offset;
            // Context change: another month's rows, so the receipts preview
            // goes; the cursor stays (desktop-ui-redesign D2).
            s.set_preview(crate::navigation::ListKind::Receipts, None);
        });
        handle().block_on(self.load_billing(false));
    }

    /// Swift `reloadBilling` (:553): pull-to-refresh forces the fetch,
    /// bypassing the past-month cache skip.
    pub fn reload_billing(&self) {
        handle().block_on(self.load_billing(true));
    }

    /// BillingView's source picker (UI-only state, mirrors `sourceFilter`).
    pub fn set_billing_source(&self, source: BillingSource) {
        self.hub().update("billing_source", move |s| {
            s.billing_source = source;
            // Context change: the filtered list is a different set of rows,
            // so the receipts preview goes; the cursor stays (D2).
            s.set_preview(crate::navigation::ListKind::Receipts, None);
        });
    }

    /// Swift `billingError` (:602): the core's last billing fetch/parse error;
    /// pull-to-refresh is the retry (v1 parity).
    pub fn billing_error(&self) -> Option<ErrorText> {
        self.core().billing_error().map(|message| ErrorText {
            message,
            retry: true,
        })
    }

    /// BillingView `entries` (:213): merged, date-descending list for the
    /// selected month, gated on auth and the source filter.
    pub fn merged_entries(&self) -> Vec<BillingEntry> {
        let state = self.state();
        let mut list = Vec::new();
        let show_gourmet =
            state.billing_source != BillingSource::Ventopay && state.gourmet_authenticated;
        let show_ventopay =
            state.billing_source != BillingSource::Gourmet && state.ventopay_authenticated;
        if show_gourmet {
            if let Some(g) = state.gourmet_month.as_ref() {
                for bill in &g.bills {
                    list.push(BillingEntry {
                        id: format!("g-{}", bill.bill_nr),
                        epoch_ms: bill.bill_date_epoch_ms,
                        source: EntrySource::Gourmet,
                        description: String::new(),
                        amount: bill.billing,
                        items: bill
                            .items
                            .iter()
                            .map(|i| BillingLine {
                                count: i.count,
                                description: i.description.clone(),
                                total: i.total,
                            })
                            .collect(),
                    });
                }
            }
        }
        if show_ventopay {
            if let Some(v) = state.ventopay_month.as_ref() {
                for tx in &v.transactions {
                    list.push(BillingEntry {
                        id: format!("v-{}", tx.id),
                        epoch_ms: tx.date_epoch_ms,
                        source: EntrySource::Ventopay,
                        description: tx.restaurant.clone(),
                        amount: tx.amount,
                        items: Vec::new(),
                    });
                }
            }
        }
        list.sort_by_key(|e| std::cmp::Reverse(e.epoch_ms));
        list
    }

    /// BillingView totals (:239-249): each side is gated on its own auth and
    /// the source filter; Belege counts the merged entries.
    pub fn billing_totals(&self) -> BillingTotals {
        let state = self.state();
        let show_gourmet =
            state.billing_source != BillingSource::Ventopay && state.gourmet_authenticated;
        let show_ventopay =
            state.billing_source != BillingSource::Gourmet && state.ventopay_authenticated;
        let gourmet_total = if show_gourmet {
            state
                .gourmet_month
                .as_ref()
                .map(|g| g.total_billing)
                .unwrap_or(0.0)
        } else {
            0.0
        };
        let ventopay_total = if show_ventopay {
            state
                .ventopay_month
                .as_ref()
                .map(|v| v.total)
                .unwrap_or(0.0)
        } else {
            0.0
        };
        let zuschuss = if show_gourmet {
            state
                .gourmet_month
                .as_ref()
                .map(|g| g.total_subsidy)
                .unwrap_or(0.0)
        } else {
            0.0
        };
        BillingTotals {
            gesamt: gourmet_total + ventopay_total,
            belege: self.merged_entries().len(),
            zuschuss,
        }
    }
}
