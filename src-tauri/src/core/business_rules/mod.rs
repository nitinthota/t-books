//! Loopbook logic, ported by hand. Never fetched at runtime.
//!
//! Same inputs → same outputs as loopbook `src/lib/erp`.
//! If loopbook changes calcPo / status / payments, edit only this module.

pub mod calc_po;
pub mod fingerprint;
pub mod money;
pub mod payment;
pub mod status;

pub use calc_po::{
    calc_payroll, calc_po, calc_po_item, clamp_pct, payment_term_days, PayrollCalc, PoItemCalc,
    PoItemInput, PoSummary, TermUnit,
};
pub use money::{format_inr, format_rupees, paise_to_rupees, rupees_to_paise};
pub use fingerprint::{canon_num, sha256_hex, voucher_canonical, voucher_fingerprint};
pub use payment::{
    add_payment, amount_or_zero, can_add_payment, empty_payment, normalize_voucher_no,
    occupied_payment_count, parse_amount, parse_voucher_number, summarize_voucher, voucher_totals,
    PaymentBlock, VoucherComputed, VoucherInput, MAX_PAYMENT_BLOCKS,
};
pub use status::{is_na_tax_inv, payment_status, status_tone, tax_flag_for, StatusTone, TaxFlag};

/// Stored in `app_meta.loopbook_logic_version`. Bump only with a controlled port.
pub const LOOPBOOK_LOGIC_VERSION: &str = "v1";
