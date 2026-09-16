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
