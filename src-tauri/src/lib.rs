mod access;
mod auth;
mod control;
pub mod core;
mod db;
mod hive;
mod hive_plan;
mod log;
mod migrate;
mod office;
mod office_sync;
mod online;
mod ops;
mod passwords;
mod provision;
mod sheets;
mod submit;
pub mod testdata;
mod trial;
mod voucher_edit;
mod vouchers;

pub use access::{
    apply_access_rows, fetch_access_rows, get_access_snapshot, get_last_synced, list_access,
    parse_access_values, refresh_access, write_access_row, AccessRow, AccessSnapshot, AccessWrite,
    AppStatus,
};
pub use auth::{inspect_email, set_first_password, sign_in};
pub use control::{
    explore_table, list_duplicates, list_rules, merge_master, toggle_rule, DuplicateGroup,
    DuplicateSerial, DuplicatesReport, RuleRow, EXPLORER_TABLES,
};
pub use core::business_rules;
pub use db::{data_dir, db_path, open_at, open_db, open_memory, LocalBooks};
pub use hive::{
    hive_conflict_message, is_live_dummy_key, list_dirty_keys, list_pending, submit_hive_row,
    tab_headers, tab_name, CasOutcome, DirtyKey, Hive, MemoryHive, PendingSubmit, KIND_ACCESS,
    KIND_DOCUMENT, KIND_INVENTORY, KIND_LOGISTICS, KIND_PAYMENT, KIND_PURCHASE, KIND_SALARY,
    KIND_SALES_PO, KIND_VOUCHER, HIVE_KINDS,
};
pub use log::{
    apply_debug_flag, clear_logs, error_log_path, event as log_event, is_debug,
    performance as log_performance, set_log_dir, Level as LogLevel,
};
pub use ops::{
    auto_backup, backup_to, check_updates, download_installer, export_logs_zip, is_newer, ops_info,
    parse_latest_release, prune_auto_backups, restore_from, set_auto_backup, store_app_version,
    OpsInfo, UpdateInfo,
};
pub use office::{
    delete_purchase_payment, delete_purchase_po, delete_sales_po, list_documents, list_hr_people,
    list_hr_payroll, list_inventory, list_logistics, list_projects, list_purchase_payments,
    list_purchase_po, list_sales_po, list_vendors, save_document, save_hr_payroll, save_hr_person,
    save_inventory, save_logistics, save_purchase_payment, save_purchase_po, save_sales_po,
    search_office, DocumentRow, HrPerson, InventoryRow, LogisticsRow, PayrollRow, PoItemIn,
    PoPreview, PurchasePayment, PurchasePaymentSave, PurchasePo, PurchasePoSave, SalesPo,
    SalesPoSave, SearchHit, VendorRef,
};
pub use office_sync::{
    bootstrap_hive_tab, hive_status, submit_office, submit_office_with, GoogleOfficeHive, HiveStatus,
    HiveTabStatus,
};
pub use sheets::credentials_exist;
pub use provision::{migrate_from_raw, provision_hive, ProvisionReport};
pub use hive_plan::{load_map as load_hive_map, refuse_raw_write, validate_plan, HiveTarget};
pub use trial::{available_fy, build_trial, TrialBalance, TrialLine};
pub use passwords::{hash_password, validate_new_password, verify_password};
pub use submit::{
    conflict_message, reload_voucher, reload_voucher_with, submit_voucher, submit_voucher_with,
    MemorySheet, SubmitOutcome,
};
pub use voucher_edit::{next_voucher_number, save_voucher, PaymentSave, VoucherSave};
pub use vouchers::{
    apply_voucher_rows, apply_voucher_rows_discarding_dirty, dirty_guard, discard_dirty_vouchers,
    force_refresh_vouchers, get_voucher, get_voucher_summary, list_dirty_vouchers, list_vouchers,
    mark_dirty, parse_voucher_values, refresh_vouchers, ParseReport, RefreshOutcome, VoucherListRow,
    VoucherSummary, VoucherView,
};

pub const APP_NAME: &str = "T Books";
pub const APP_VERSION: &str = "1.0.0";
pub const DATA_FOLDER_NAME: &str = "T-Books";
pub const DB_FILE_NAME: &str = "tbooks.db";
pub const CREDENTIALS_FILE_NAME: &str = "credentials.json";
pub const OWNER_EMAIL: &str = "thotanitin123@gmail.com";
pub const ACCESS_TAB: &str = "Access";
pub const VOUCHER_RAW_TAB: &str = "Voucher_Raw_Data";
pub const DEFAULT_SPREADSHEET_ID: &str = "1J9ZuNL1uZ7DmqOGIZojuOCMeYp9VnC-SEow6YG86cgE";
pub const OFFLINE_BANNER: &str =
    "Offline — working on this PC. Access list and Refresh paused.";
pub const CREDENTIALS_BANNER: &str = "Google credentials not found. Running offline.";
pub const MIN_PASSWORD_LENGTH: usize = 8;
pub const SCHEMA_VERSION: &str = "10";
pub const LOOPBOOK_LOGIC_VERSION: &str = business_rules::LOOPBOOK_LOGIC_VERSION;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct Session {
    pub email: String,
    pub role: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AuthInspect {
    #[serde(rename = "set-password")]
    SetPassword { email: String },
    Password { email: String },
    Denied { message: String },
}

#[derive(Debug, thiserror::Error)]
pub enum BooksError {
    #[error("{0}")]
    Message(String),
    #[error("{0}")]
    Sqlite(#[from] rusqlite::Error),
}

pub type Result<T> = std::result::Result<T, BooksError>;

impl From<String> for BooksError {
    fn from(value: String) -> Self {
        BooksError::Message(value)
    }
}

impl From<&str> for BooksError {
    fn from(value: &str) -> Self {
        BooksError::Message(value.to_string())
    }
}

pub fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

pub fn is_hardcoded_owner(email: &str) -> bool {
    normalize_email(email) == OWNER_EMAIL
}

pub fn is_valid_email_shape(email: &str) -> bool {
    let value = normalize_email(email);
    let Some((user, domain)) = value.split_once('@') else {
        return false;
    };
    !user.is_empty() && domain.contains('.') && domain.len() >= 3 && !value.contains(' ')
}

#[cfg(feature = "desktop")]
pub fn run_desktop() {
    desktop::run();
}

#[cfg(feature = "desktop")]
#[path = "desktop.rs"]
mod desktop;
