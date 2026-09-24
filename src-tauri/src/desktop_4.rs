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

#[tauri::command]
fn list_duplicates(
    state: tauri::State<AppState>,
) -> std::result::Result<DuplicatesReport, String> {
    let books = state.books.lock().expect("local books");
    crate::control::list_duplicates(&books).map_err(map_err)
}

#[tauri::command]
fn merge_master(
    state: tauri::State<AppState>,
    kind: String,
    keep_id: String,
    absorb_ids: Vec<String>,
) -> std::result::Result<i64, String> {
    require_mutate(&state)?;
    let books = state.books.lock().expect("local books");
    crate::control::merge_master(&books, &kind, &keep_id, &absorb_ids).map_err(map_err)
}

#[tauri::command]
fn explore_table(
    state: tauri::State<AppState>,
    table: String,
) -> std::result::Result<Vec<std::collections::BTreeMap<String, String>>, String> {
    let books = state.books.lock().expect("local books");
    crate::control::explore_table(&books, &table).map_err(map_err)
}

#[tauri::command]
fn list_rules(state: tauri::State<AppState>) -> std::result::Result<Vec<RuleRow>, String> {
    let books = state.books.lock().expect("local books");
    crate::control::list_rules(&books).map_err(map_err)
}

#[tauri::command]
fn toggle_rule(
    state: tauri::State<AppState>,
    id: String,
    enabled: bool,
) -> std::result::Result<RuleRow, String> {
    require_mutate(&state)?;
    let books = state.books.lock().expect("local books");
    crate::control::toggle_rule(&books, &id, enabled).map_err(map_err)
}

#[tauri::command]
fn hive_status() -> HiveStatus {
    crate::office_sync::hive_status()
}

#[tauri::command]
fn bootstrap_hive_tab(
    state: tauri::State<AppState>,
    kind: String,
) -> std::result::Result<String, String> {
    require_owner(&state)?;
    crate::office_sync::bootstrap_hive_tab(&kind).map_err(map_err)
}

#[tauri::command]
fn provision_hive(
    state: tauri::State<AppState>,
) -> std::result::Result<crate::provision::ProvisionReport, String> {
    require_owner(&state)?;
    crate::provision_hive(true).map_err(map_err)
}

#[tauri::command]
fn migrate_from_raw(
    state: tauri::State<AppState>,
    dry_run: bool,
) -> std::result::Result<crate::migrate::MigrationPlan, String> {
    require_owner(&state)?;
    crate::migrate_from_raw(dry_run).map_err(map_err)
}

#[tauri::command]
fn retry_pending_submit(
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
    let mut books = state.books.lock().expect("local books");
    if kind == crate::KIND_VOUCHER {
        let n = crate::business_rules::parse_voucher_number(&key)
            .ok_or_else(|| "Voucher number must be a whole integer.".to_string())?;
        match crate::submit_voucher(&mut books, n).map_err(map_err)? {
            crate::SubmitOutcome::Ok { voucher_number } => Ok(CasOutcome::Ok {
                key: voucher_number.to_string(),
                fp: String::new(),
                rev: 0,
            }),
            crate::SubmitOutcome::Conflict { voucher_number, message } => Ok(CasOutcome::Conflict {
                key: voucher_number.to_string(),
                message,
            }),
        }
    } else {
        crate::office_sync::submit_office(&books, &kind, &key, allow).map_err(map_err)
    }
}

#[tauri::command]
fn list_jef(
    state: tauri::State<AppState>,
    book: Option<String>,
) -> std::result::Result<Vec<crate::core::jef::JefSlice>, String> {
    let books = state.books.lock().expect("local books");
    crate::core::jef::project_book(books.conn(), book.as_deref()).map_err(map_err)
}

#[tauri::command]
fn jef_timeline(
    state: tauri::State<AppState>,
    book: String,
    key: String,
) -> std::result::Result<Vec<crate::core::jef::JefSlice>, String> {
    let books = state.books.lock().expect("local books");
    crate::core::jef::timeline(books.conn(), &book, &key).map_err(map_err)
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
                if let Some(state) = window.try_state::<AppState>() {
                    let locked = state.books.lock().ok();
                    if let Some(books) = locked.as_ref() {
                        let _ = crate::auto_backup(books);
                    }
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
            save_voucher,
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
            list_duplicates,
            merge_master,
            explore_table,
            list_rules,
            toggle_rule,
            hive_status,
            bootstrap_hive_tab,
            provision_hive,
            migrate_from_raw,
            retry_pending_submit,
            list_jef,
            jef_timeline,
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
            list_vendors,
            list_vendor_accounts,
            save_vendor,
            merge_onto_purchase,
            unmerge_voucher
        ])
        .run(tauri::generate_context!())
        .expect("error while running T Books");
}
