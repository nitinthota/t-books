//! Loopbook logic, ported by hand. Never fetched at runtime.
//!
//! Same inputs → same outputs as loopbook `src/lib/erp` @ 6e4ac1eb.
//! If loopbook changes calcPo / status / payments, edit only this module.

pub mod alloc;
pub mod calc_po;
pub mod fingerprint;
pub mod fy;
pub mod hr_payroll;
pub mod money;
pub mod payment;
pub mod purchase_status;
pub mod rbac;
pub mod status;

pub use alloc::{
    allocate_or_reject, allocate_voucher_serial, keep_posted_number, merge_key_pool,
    next_converted_po_number, next_payment_from_hive_and_local, next_payment_number,
    next_project_expense_number, next_purchase_from_hive_and_local, next_purchase_number,
    next_salary_from_hive_and_local, next_salary_number,
};
pub use calc_po::{
    calc_payroll, calc_payroll_money, calc_po, calc_po_item, clamp_pct, payment_term_days,
    PayrollCalc, PoItemCalc, PoItemInput, PoSummary, TermUnit,
};
pub use fy::{
    clamp_as_of, current_indian_fy, fy_bounds, fy_options, signed_dr_cr, trial_balanced,
    trial_closing,
};
pub use hr_payroll::{
    bank_account_code, calc_salary_slip, keep_posted_pay_number, lines_balance, month_label,
    normalize_pay_kind, outstanding_advance_paise, payroll_money_changed, payroll_net_ok,
    pick_payroll_as_of, salary_period_taken, salary_voucher_lines, LedgerLine, PayKind,
    PayrollAsOfRow, PayrollMoney, SalaryPeriodRow, SalarySlip, MONTHS,
};
pub use money::{format_inr, format_qty, format_rupees, paise_to_rupees, rupees_to_paise};
pub use fingerprint::{canon_num, sha256_hex, voucher_canonical, voucher_fingerprint};
pub use payment::{
    add_payment, amount_or_zero, can_add_payment, empty_payment, is_dotted_child_serial,
    is_dummy_serial, normalize_voucher_no, occupied_payment_count, parse_amount,
    parse_voucher_number, should_skip_sheet_row, summarize_voucher, voucher_totals, PaymentBlock,
    VoucherComputed, VoucherInput, MAX_PAYMENT_BLOCKS,
};
pub use purchase_status::{
    alloc_method_for_purchase, alloc_method_label, classify_purchase_payment, format_payment_number,
    is_child_pay_number, is_legacy_pay_number, is_pay_voucher_number, normalize_alloc_method,
    parse_pay_voucher_seq, payment_allocation_label, payment_class_label, payment_matches_filters,
    payment_ordinal, purchase_pay_status, purchase_type_label, tax_invoice_missing, AllocMethod,
    PaymentClass, PaymentFilterKey, PurchasePayStatus, PurchaseType,
};
pub use rbac::{can_mutate, can_open_access, can_refresh, can_write, require_admin, require_write};
pub use status::{is_na_tax_inv, payment_status, status_tone, tax_flag_for, StatusTone, TaxFlag};

/// Stored in `app_meta.loopbook_logic_version`. Bump only with a controlled port.
pub const LOOPBOOK_LOGIC_VERSION: &str = "v1";
