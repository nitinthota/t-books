mod access;
mod auth;
pub mod core;
mod db;
mod hive;
mod log;
mod office;
mod office_sync;
mod online;
mod ops;
mod passwords;
mod sheets;
mod submit;
pub mod testdata;
mod trial;
mod vouchers;

pub use access::{
    apply_access_rows, fetch_access_rows, get_access_snapshot, get_last_synced, list_access,
    parse_access_values, refresh_access, write_access_row, AccessRow, AccessSnapshot, AccessWrite,
    AppStatus,
};
pub use auth::{inspect_email, set_first_password, sign_in};
pub use core::business_rules;
pub use db::{data_dir, db_path, open_at, open_db, open_memory, LocalBooks};
pub use hive::{
    hive_conflict_message, list_dirty_keys, list_pending, submit_hive_row, tab_headers, tab_name,
    CasOutcome, DirtyKey, Hive, MemoryHive, PendingSubmit, KIND_ACCESS, KIND_DOCUMENT,
    KIND_INVENTORY, KIND_LOGISTICS, KIND_PAYMENT, KIND_PURCHASE, KIND_SALARY, KIND_VOUCHER,
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
    delete_purchase_payment, delete_purchase_po, delete_sales_po, list_purchase_payments,
    list_purchase_po, list_sales_po, save_purchase_payment, save_purchase_po, save_sales_po,
    DocumentRow, HrPerson, InventoryRow, LogisticsRow, PayrollRow, PoItemIn, PoPreview,
    PurchasePayment, PurchasePaymentSave, PurchasePo, PurchasePoSave, SalesPo, SalesPoSave,
    SearchHit, VendorRef,
};
pub use office_sync::{submit_office, submit_office_with};
pub use trial::{available_fy, build_trial, TrialBalance, TrialLine};
pub use passwords::{hash_password, validate_new_password, verify_password};
pub use submit::{
    conflict_message, reload_voucher, reload_voucher_with, submit_voucher, submit_voucher_with,
    MemorySheet, SubmitOutcome,
};
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
pub const SCHEMA_VERSION: &str = "8";
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
mod desktop {
    use super::*;
    use crate::online::is_online;
    use crate::sheets::credentials_exist;
    use std::sync::Mutex;

    struct AppState {
        books: Mutex<LocalBooks>,
        session: Mutex<Option<Session>>,
        last_error: Mutex<Option<String>>,
    }

    fn set_last_error(slot: &Mutex<Option<String>>, message: Option<String>) {
        *slot.lock().expect("last error") = message;
    }

    fn login_prefetch_access() -> Option<std::result::Result<Vec<AccessRow>, String>> {
        if !is_online() || !credentials_exist() {
            return None;
        }
        Some(fetch_access_rows().map_err(|e| e.to_string()))
    }

    fn apply_prefetch(
        last_error: &Mutex<Option<String>>,
        books: &mut LocalBooks,
        prefetch: Option<std::result::Result<Vec<AccessRow>, String>>,
    ) {
        match prefetch {
            Some(Ok(rows)) => match apply_access_rows(books, &rows) {
                Ok(()) => set_last_error(last_error, None),
                Err(err) => set_last_error(last_error, Some(err.to_string())),
            },
            Some(Err(err)) => set_last_error(last_error, Some(err)),
            None => {}
        }
    }

    #[tauri::command]
    fn inspect(state: tauri::State<AppState>, email: String) -> AuthInspect {
        let books = state.books.lock().expect("local books");
        inspect_email(&books, &email)
    }

    #[tauri::command]
    fn login(
        state: tauri::State<AppState>,
        email: String,
        password: String,
    ) -> std::result::Result<Session, String> {
        let prefetch = login_prefetch_access();
        let mut books = state.books.lock().expect("local books");
        apply_prefetch(&state.last_error, &mut books, prefetch);
        let session = sign_in(&books, &email, &password).map_err(|e| e.to_string())?;
        *state.session.lock().expect("session") = Some(session.clone());
        Ok(session)
    }

    #[tauri::command]
    fn set_password(
        state: tauri::State<AppState>,
        email: String,
        password: String,
        confirm: String,
    ) -> std::result::Result<Session, String> {
        let prefetch = login_prefetch_access();
        let mut books = state.books.lock().expect("local books");
        apply_prefetch(&state.last_error, &mut books, prefetch);
        let session = set_first_password(&books, &email, &password, &confirm)
            .map_err(|e| e.to_string())?;
        *state.session.lock().expect("session") = Some(session.clone());
        Ok(session)
    }

    #[tauri::command]
    fn current_session(state: tauri::State<AppState>) -> Option<Session> {
        state.session.lock().expect("session").clone()
    }

    #[tauri::command]
    fn logout(state: tauri::State<AppState>) {
        *state.session.lock().expect("session") = None;
    }

    #[tauri::command]
    fn local_data_dir() -> String {
        data_dir().display().to_string()
    }

    #[tauri::command]
    fn get_ops_info(state: tauri::State<AppState>) -> OpsInfo {
        let books = state.books.lock().expect("local books");
        crate::ops_info(&books)
    }

    #[tauri::command]
    fn backup_data(
        state: tauri::State<AppState>,
        dest: Option<String>,
    ) -> std::result::Result<String, String> {
        let books = state.books.lock().expect("local books");
        let path = match dest.filter(|s| !s.trim().is_empty()) {
            Some(p) => std::path::PathBuf::from(p),
            None => crate::ops::pick_save(&crate::ops::default_backup_name()).map_err(|e| e.to_string())?,
        };
        crate::backup_to(&books, &path)
            .map(|p| p.display().to_string())
            .map_err(|e| e.to_string())
    }

    #[tauri::command]
    fn restore_data(
        state: tauri::State<AppState>,
        src: String,
    ) -> std::result::Result<String, String> {
        if src.trim().is_empty() {
            return Err("Choose a backup zip on this PC.".into());
        }
        let mut books = state.books.lock().expect("local books");
        crate::restore_from(&mut books, std::path::Path::new(&src)).map_err(|e| e.to_string())?;
        Ok("Restored. Sign in again if asked.".into())
    }

    #[tauri::command]
    fn pick_restore_zip() -> std::result::Result<String, String> {
        crate::ops::pick_open_zip()
            .map(|p| p.display().to_string())
            .map_err(|e| e.to_string())
    }

    #[tauri::command]
    fn export_logs(dest: Option<String>) -> std::result::Result<String, String> {
        let path = match dest.filter(|s| !s.trim().is_empty()) {
            Some(p) => std::path::PathBuf::from(p),
            None => crate::ops::pick_save("T-Books-logs.zip").map_err(|e| e.to_string())?,
        };
        crate::export_logs_zip(&path)
            .map(|p| p.display().to_string())
            .map_err(|e| e.to_string())
    }

    #[tauri::command]
    fn clear_logs_cmd() -> std::result::Result<(), String> {
        crate::clear_logs().map_err(|e| e.to_string())
    }

    #[tauri::command]
    fn set_auto_backup_cmd(
        state: tauri::State<AppState>,
        enabled: bool,
    ) -> std::result::Result<(), String> {
        let books = state.books.lock().expect("local books");
        crate::set_auto_backup(&books, enabled).map_err(|e| e.to_string())
    }

    #[tauri::command]
    fn check_for_updates(
        state: tauri::State<AppState>,
    ) -> std::result::Result<UpdateInfo, String> {
        let books = state.books.lock().expect("local books");
        let current = crate::ops::meta(&books, "version").unwrap_or_else(|| APP_VERSION.to_string());
        crate::check_updates(&current).map_err(|e| e.to_string())
    }

    #[tauri::command]
    fn download_update(
        url: String,
        dest: Option<String>,
    ) -> std::result::Result<String, String> {
        let path = match dest.filter(|s| !s.trim().is_empty()) {
            Some(p) => std::path::PathBuf::from(p),
            None => crate::ops::pick_save_exe("T-Books-Setup.exe").map_err(|e| e.to_string())?,
        };
        crate::download_installer(&url, &path)
            .map(|p| p.display().to_string())
            .map_err(|e| e.to_string())
    }

    #[tauri::command]
    fn refresh_access(
        state: tauri::State<AppState>,
    ) -> std::result::Result<AccessSnapshot, String> {
        let allow_bootstrap = {
            let session = state.session.lock().expect("session");
            session
                .as_ref()
                .map(|s| {
                    s.role.eq_ignore_ascii_case("owner")
                        || s.email.eq_ignore_ascii_case(OWNER_EMAIL)
                })
                .unwrap_or(false)
        };
        let rows = match crate::access::fetch_access_rows_with(allow_bootstrap) {
            Ok(rows) => rows,
            Err(err) => {
                let message = err.to_string();
                set_last_error(&state.last_error, Some(message.clone()));
                return Err(message);
            }
        };
        let mut books = state.books.lock().expect("local books");
        if let Err(err) = apply_access_rows(&mut books, &rows) {
            let message = err.to_string();
            set_last_error(&state.last_error, Some(message.clone()));
            return Err(message);
        }
        set_last_error(&state.last_error, None);
        Ok(get_access_snapshot(&books))
    }

    #[tauri::command]
    fn get_access_list(state: tauri::State<AppState>) -> AccessSnapshot {
        let books = state.books.lock().expect("local books");
        get_access_snapshot(&books)
    }

    #[tauri::command]
    fn get_app_status(state: tauri::State<AppState>) -> AppStatus {
        let books = state.books.lock().expect("local books");
        AppStatus {
            credentials_found: credentials_exist(),
            last_synced: get_last_synced(&books),
            voucher_last_synced: crate::vouchers::get_voucher_last_synced(&books),
            last_error: state.last_error.lock().expect("last error").clone(),
        }
    }

    #[tauri::command]
    fn get_dirty_vouchers(state: tauri::State<AppState>) -> std::result::Result<Vec<i64>, String> {
        let books = state.books.lock().expect("local books");
        crate::vouchers::list_dirty_vouchers(&books).map_err(|e| e.to_string())
    }

    #[tauri::command]
    fn refresh_vouchers(
        state: tauri::State<AppState>,
    ) -> std::result::Result<RefreshOutcome, String> {
        let allow_bootstrap = {
            let session = state.session.lock().expect("session");
            session
                .as_ref()
                .map(|s| {
                    s.role.eq_ignore_ascii_case("owner")
                        || s.email.eq_ignore_ascii_case(OWNER_EMAIL)
                })
                .unwrap_or(false)
        };
        let mut books = state.books.lock().expect("local books");
        match crate::vouchers::refresh_vouchers_with(&mut books, allow_bootstrap) {
            Ok(out) => {
                if matches!(&out, RefreshOutcome::Ok { .. }) {
                    set_last_error(&state.last_error, None);
                }
                Ok(out)
            }
            Err(err) => {
                let message = err.to_string();
                set_last_error(&state.last_error, Some(message.clone()));
                Err(message)
            }
        }
    }

    #[tauri::command]
    fn force_refresh_vouchers(
        state: tauri::State<AppState>,
    ) -> std::result::Result<RefreshOutcome, String> {
        let mut books = state.books.lock().expect("local books");
        match crate::vouchers::force_refresh_vouchers(&mut books) {
            Ok(out) => {
                set_last_error(&state.last_error, None);
                Ok(out)
            }
            Err(err) => {
                let message = err.to_string();
                set_last_error(&state.last_error, Some(message.clone()));
                Err(message)
            }
        }
    }

    #[tauri::command]
    fn get_voucher_summary(
        state: tauri::State<AppState>,
    ) -> std::result::Result<VoucherSummary, String> {
        let books = state.books.lock().expect("local books");
        crate::vouchers::get_voucher_summary(&books).map_err(|e| e.to_string())
    }

    #[tauri::command]
    fn list_vouchers(
        state: tauri::State<AppState>,
    ) -> std::result::Result<Vec<crate::vouchers::VoucherListRow>, String> {
        let books = state.books.lock().expect("local books");
        crate::vouchers::list_vouchers(&books).map_err(|e| e.to_string())
    }

    #[tauri::command]
    fn get_voucher(
        state: tauri::State<AppState>,
        voucher_number: i64,
    ) -> std::result::Result<crate::vouchers::VoucherView, String> {
        let books = state.books.lock().expect("local books");
        crate::vouchers::get_voucher(&books, voucher_number).map_err(|e| e.to_string())
    }

    fn require_submit_role(state: &tauri::State<AppState>) -> std::result::Result<(), String> {
        match state.session.lock().expect("session").as_ref() {
            Some(s) if s.role == "owner" || s.role == "admin" => Ok(()),
            Some(_) => Err("Your role cannot submit to Google.".into()),
            None => Err("Sign in first.".into()),
        }
    }

    fn require_mutate(state: &tauri::State<AppState>) -> std::result::Result<Session, String> {
        match state.session.lock().expect("session").as_ref() {
            Some(s) => {
                crate::core::business_rules::require_write(&s.role)?;
                Ok(s.clone())
            }
            None => Err("Sign in first.".into()),
        }
    }

    fn require_owner(state: &tauri::State<AppState>) -> std::result::Result<Session, String> {
        match state.session.lock().expect("session").as_ref() {
            Some(s)
                if s.role == "owner" || crate::is_hardcoded_owner(&s.email) =>
            {
                Ok(s.clone())
            }
            Some(_) => Err("Only the owner can change Access.".into()),
            None => Err("Sign in first.".into()),
        }
    }

    #[tauri::command]
    fn submit_voucher(
        state: tauri::State<AppState>,
        voucher_number: i64,
    ) -> std::result::Result<SubmitOutcome, String> {
        require_submit_role(&state)?;
        let mut books = state.books.lock().expect("local books");
        match crate::submit_voucher(&mut books, voucher_number) {
            Ok(out) => {
                if matches!(&out, SubmitOutcome::Ok { .. }) {
                    set_last_error(&state.last_error, None);
                }
                Ok(out)
            }
            Err(err) => {
                let message = err.to_string();
                set_last_error(&state.last_error, Some(message.clone()));
                Err(message)
            }
        }
    }

    #[tauri::command]
    fn reload_voucher(
        state: tauri::State<AppState>,
        voucher_number: i64,
    ) -> std::result::Result<crate::vouchers::VoucherView, String> {
        require_submit_role(&state)?;
        let mut books = state.books.lock().expect("local books");
        match crate::reload_voucher(&mut books, voucher_number) {
            Ok(view) => {
                set_last_error(&state.last_error, None);
                Ok(view)
            }
            Err(err) => {
                let message = err.to_string();
                set_last_error(&state.last_error, Some(message.clone()));
                Err(message)
            }
        }
    }

    fn map_err(err: BooksError) -> String {
        err.to_string()
    }

    #[tauri::command]
    fn list_sales_po(state: tauri::State<AppState>) -> std::result::Result<Vec<SalesPo>, String> {
        let books = state.books.lock().expect("local books");
        crate::office::list_sales_po(&books).map_err(map_err)
    }

    #[tauri::command]
    fn get_sales_po(
        state: tauri::State<AppState>,
        id: i64,
    ) -> std::result::Result<SalesPo, String> {
        let books = state.books.lock().expect("local books");
        crate::office::get_sales_po(&books, id).map_err(map_err)
    }

    #[tauri::command]
    fn save_sales_po(
        state: tauri::State<AppState>,
        payload: SalesPoSave,
    ) -> std::result::Result<SalesPo, String> {
        require_mutate(&state)?;
        let mut books = state.books.lock().expect("local books");
        crate::office::save_sales_po(&mut books, payload).map_err(map_err)
    }

    #[tauri::command]
    fn delete_sales_po(state: tauri::State<AppState>, id: i64) -> std::result::Result<(), String> {
        require_mutate(&state)?;
        let books = state.books.lock().expect("local books");
        crate::office::delete_sales_po(&books, id).map_err(map_err)
    }

    #[tauri::command]
    fn preview_po(items: Vec<PoItemIn>) -> PoPreview {
        crate::office::preview_po(&items)
    }

    #[tauri::command]
    fn list_purchase_po(
        state: tauri::State<AppState>,
    ) -> std::result::Result<Vec<PurchasePo>, String> {
        let books = state.books.lock().expect("local books");
        crate::office::list_purchase_po(&books).map_err(map_err)
    }

    #[tauri::command]
    fn get_purchase_po(
        state: tauri::State<AppState>,
        id: i64,
    ) -> std::result::Result<PurchasePo, String> {
        let books = state.books.lock().expect("local books");
        crate::office::get_purchase_po(&books, id).map_err(map_err)
    }

    #[tauri::command]
    fn save_purchase_po(
        state: tauri::State<AppState>,
        payload: PurchasePoSave,
    ) -> std::result::Result<PurchasePo, String> {
        require_mutate(&state)?;
        let mut books = state.books.lock().expect("local books");
        crate::office::save_purchase_po(&mut books, payload).map_err(map_err)
    }

    #[tauri::command]
    fn delete_purchase_po(
        state: tauri::State<AppState>,
        id: i64,
    ) -> std::result::Result<(), String> {
        require_mutate(&state)?;
        let books = state.books.lock().expect("local books");
        crate::office::delete_purchase_po(&books, id).map_err(map_err)
    }

    #[tauri::command]
    fn list_hr_people(state: tauri::State<AppState>) -> std::result::Result<Vec<HrPerson>, String> {
        let books = state.books.lock().expect("local books");
        crate::office::list_hr_people(&books).map_err(map_err)
    }

    #[tauri::command]
    fn save_hr_person(
        state: tauri::State<AppState>,
        payload: HrPerson,
    ) -> std::result::Result<HrPerson, String> {
        require_mutate(&state)?;
        let books = state.books.lock().expect("local books");
        crate::office::save_hr_person(&books, payload).map_err(map_err)
    }

    #[tauri::command]
    fn delete_hr_person(state: tauri::State<AppState>, id: i64) -> std::result::Result<(), String> {
        require_mutate(&state)?;
        let books = state.books.lock().expect("local books");
        crate::office::delete_hr_person(&books, id).map_err(map_err)
    }

    #[tauri::command]
    fn list_hr_payroll(
        state: tauri::State<AppState>,
    ) -> std::result::Result<Vec<PayrollRow>, String> {
        let books = state.books.lock().expect("local books");
        crate::office::list_hr_payroll(&books).map_err(map_err)
    }

    #[tauri::command]
    fn save_hr_payroll(
        state: tauri::State<AppState>,
        payload: PayrollRow,
    ) -> std::result::Result<PayrollRow, String> {
        require_mutate(&state)?;
        let books = state.books.lock().expect("local books");
        crate::office::save_hr_payroll(&books, payload).map_err(map_err)
    }

    #[tauri::command]
    fn delete_hr_payroll(state: tauri::State<AppState>, id: i64) -> std::result::Result<(), String> {
        require_mutate(&state)?;
        let books = state.books.lock().expect("local books");
        crate::office::delete_hr_payroll(&books, id).map_err(map_err)
    }

    #[tauri::command]
    fn list_inventory(
        state: tauri::State<AppState>,
        query: Option<String>,
    ) -> std::result::Result<Vec<InventoryRow>, String> {
        let books = state.books.lock().expect("local books");
        crate::office::list_inventory(&books, query.as_deref()).map_err(map_err)
    }

    #[tauri::command]
    fn save_inventory(
        state: tauri::State<AppState>,
        payload: InventoryRow,
    ) -> std::result::Result<InventoryRow, String> {
        require_mutate(&state)?;
        let books = state.books.lock().expect("local books");
        crate::office::save_inventory(&books, payload).map_err(map_err)
    }

    #[tauri::command]
    fn delete_inventory(state: tauri::State<AppState>, id: i64) -> std::result::Result<(), String> {
        require_mutate(&state)?;
        let books = state.books.lock().expect("local books");
        crate::office::delete_inventory(&books, id).map_err(map_err)
    }

    #[tauri::command]
    fn move_inventory(
        state: tauri::State<AppState>,
        id: i64,
        project: String,
    ) -> std::result::Result<InventoryRow, String> {
        require_mutate(&state)?;
        let books = state.books.lock().expect("local books");
        crate::office::move_inventory(&books, id, &project).map_err(map_err)
    }

    #[tauri::command]
    fn list_logistics(
        state: tauri::State<AppState>,
    ) -> std::result::Result<Vec<LogisticsRow>, String> {
        let books = state.books.lock().expect("local books");
        crate::office::list_logistics(&books).map_err(map_err)
    }

    #[tauri::command]
    fn save_logistics(
        state: tauri::State<AppState>,
        payload: LogisticsRow,
    ) -> std::result::Result<LogisticsRow, String> {
        require_mutate(&state)?;
        let books = state.books.lock().expect("local books");
        crate::office::save_logistics(&books, payload).map_err(map_err)
    }

    #[tauri::command]
    fn delete_logistics(state: tauri::State<AppState>, id: i64) -> std::result::Result<(), String> {
        require_mutate(&state)?;
        let books = state.books.lock().expect("local books");
        crate::office::delete_logistics(&books, id).map_err(map_err)
    }

    #[tauri::command]
    fn move_logistics(
        state: tauri::State<AppState>,
        id: i64,
        project: String,
    ) -> std::result::Result<LogisticsRow, String> {
        require_mutate(&state)?;
        let books = state.books.lock().expect("local books");
        crate::office::move_logistics(&books, id, &project).map_err(map_err)
    }

    #[tauri::command]
    fn list_documents(
        state: tauri::State<AppState>,
    ) -> std::result::Result<Vec<DocumentRow>, String> {
        let books = state.books.lock().expect("local books");
        crate::office::list_documents(&books).map_err(map_err)
    }

    #[tauri::command]
    fn save_document(
        state: tauri::State<AppState>,
        payload: DocumentRow,
    ) -> std::result::Result<DocumentRow, String> {
        require_mutate(&state)?;
        let books = state.books.lock().expect("local books");
        crate::office::save_document(&books, payload).map_err(map_err)
    }

    #[tauri::command]
    fn delete_document(state: tauri::State<AppState>, id: i64) -> std::result::Result<(), String> {
        require_mutate(&state)?;
        let books = state.books.lock().expect("local books");
        crate::office::delete_document(&books, id).map_err(map_err)
    }

    #[tauri::command]
    fn open_document(
        state: tauri::State<AppState>,
        id: i64,
    ) -> std::result::Result<String, String> {
        let books = state.books.lock().expect("local books");
        crate::office::open_document(&books, id).map_err(map_err)
    }

    #[tauri::command]
    fn search_office(
        state: tauri::State<AppState>,
        query: String,
    ) -> std::result::Result<Vec<SearchHit>, String> {
        let books = state.books.lock().expect("local books");
        crate::office::search_office(&books, &query).map_err(map_err)
    }

    #[tauri::command]
    fn list_projects(state: tauri::State<AppState>) -> std::result::Result<Vec<String>, String> {
        let books = state.books.lock().expect("local books");
        crate::office::list_projects(&books).map_err(map_err)
    }

    #[tauri::command]
    fn list_vendors(state: tauri::State<AppState>) -> std::result::Result<Vec<VendorRef>, String> {
        let books = state.books.lock().expect("local books");
        crate::office::list_vendors(&books).map_err(map_err)
    }

    #[tauri::command]
    fn list_purchase_payments(
        state: tauri::State<AppState>,
        po_number: Option<String>,
    ) -> std::result::Result<Vec<PurchasePayment>, String> {
        let books = state.books.lock().expect("local books");
        crate::office::list_purchase_payments(&books, po_number.as_deref()).map_err(map_err)
    }

    #[tauri::command]
    fn save_purchase_payment(
        state: tauri::State<AppState>,
        payload: PurchasePaymentSave,
    ) -> std::result::Result<PurchasePayment, String> {
        require_mutate(&state)?;
        let mut books = state.books.lock().expect("local books");
        crate::office::save_purchase_payment(&mut books, payload).map_err(map_err)
    }

    #[tauri::command]
    fn delete_purchase_payment(
        state: tauri::State<AppState>,
        id: i64,
    ) -> std::result::Result<(), String> {
        require_mutate(&state)?;
        let books = state.books.lock().expect("local books");
        crate::office::delete_purchase_payment(&books, id).map_err(map_err)
    }

    #[tauri::command]
    fn submit_office(
        state: tauri::State<AppState>,
        kind: String,
        key: String,
    ) -> std::result::Result<CasOutcome, String> {
        require_submit_role(&state)?;
        let allow = state
            .session
            .lock()
            .expect("session")
            .as_ref()
            .map(|s| s.role == "owner" || crate::is_hardcoded_owner(&s.email))
            .unwrap_or(false);
        let books = state.books.lock().expect("local books");
        crate::office_sync::submit_office(&books, &kind, &key, allow).map_err(map_err)
    }

    #[tauri::command]
    fn get_dirty_keys(state: tauri::State<AppState>) -> std::result::Result<Vec<DirtyKey>, String> {
        let books = state.books.lock().expect("local books");
        crate::hive::list_dirty_keys(&books).map_err(map_err)
    }

    #[tauri::command]
    fn list_pending_submit(
        state: tauri::State<AppState>,
    ) -> std::result::Result<Vec<PendingSubmit>, String> {
        let books = state.books.lock().expect("local books");
        crate::hive::list_pending(&books).map_err(map_err)
    }

    #[tauri::command]
    fn get_trial(
        state: tauri::State<AppState>,
        fy: Option<String>,
        as_of: Option<String>,
    ) -> std::result::Result<TrialBalance, String> {
        let books = state.books.lock().expect("local books");
        crate::trial::build_trial(&books, fy.as_deref(), as_of.as_deref()).map_err(map_err)
    }

    #[tauri::command]
    fn list_fy(state: tauri::State<AppState>) -> std::result::Result<Vec<String>, String> {
        let books = state.books.lock().expect("local books");
        crate::trial::available_fy(&books).map_err(map_err)
    }

    #[tauri::command]
    fn write_access_row(
        state: tauri::State<AppState>,
        payload: AccessWrite,
    ) -> std::result::Result<AccessRow, String> {
        require_owner(&state)?;
        let mut books = state.books.lock().expect("local books");
        crate::access::write_access_row(&mut books, payload).map_err(map_err)
    }

    pub fn run() {
        let books = open_db().expect("open T Books SQLite");
        tauri::Builder::default()
            .manage(AppState {
                books: Mutex::new(books),
                session: Mutex::new(None),
                last_error: Mutex::new(None),
            })
            .on_window_event(|window, event| {
                if let tauri::WindowEvent::CloseRequested { .. } = event {
                    let state: tauri::State<AppState> = window.state();
                    if let Ok(books) = state.books.lock() {
                        let _ = crate::auto_backup(&books);
                    }
                }
            })
            .invoke_handler(tauri::generate_handler![
                inspect,
                login,
                set_password,
                current_session,
                logout,
                local_data_dir,
                get_ops_info,
                backup_data,
                restore_data,
                pick_restore_zip,
                export_logs,
                clear_logs_cmd,
                set_auto_backup_cmd,
                check_for_updates,
                download_update,
                refresh_access,
                get_access_list,
                get_app_status,
                get_dirty_vouchers,
                refresh_vouchers,
                force_refresh_vouchers,
                get_voucher_summary,
                list_vouchers,
                get_voucher,
                submit_voucher,
                reload_voucher,
                list_sales_po,
                get_sales_po,
                save_sales_po,
                delete_sales_po,
                preview_po,
                list_purchase_po,
                get_purchase_po,
                save_purchase_po,
                delete_purchase_po,
                list_purchase_payments,
                save_purchase_payment,
                delete_purchase_payment,
                submit_office,
                get_dirty_keys,
                list_pending_submit,
                get_trial,
                list_fy,
                write_access_row,
                list_hr_people,
                save_hr_person,
                delete_hr_person,
                list_hr_payroll,
                save_hr_payroll,
                delete_hr_payroll,
                list_inventory,
                save_inventory,
                delete_inventory,
                move_inventory,
                list_logistics,
                save_logistics,
                delete_logistics,
                move_logistics,
                list_documents,
                save_document,
                delete_document,
                open_document,
                search_office,
                list_projects,
                list_vendors
            ])
            .run(tauri::generate_context!())
            .expect("error while running T Books");
    }
}
