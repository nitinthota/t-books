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
    let session = require_mutate(&state)?;
    let books = state.books.lock().expect("local books");
    let key = match payload.id {
        Some(id) if id > 0 => format!("HR-{id}"),
        _ => format!("HR-new-{}", payload.name.trim()),
    };
    let _ = crate::core::pipeline::park_save(books.conn(), "people", &key, &session.email);
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
    let session = require_mutate(&state)?;
    let books = state.books.lock().expect("local books");
    let key = if payload.salary_number.trim().is_empty() {
        format!("SALARY-id-{}", payload.id.unwrap_or(0))
    } else {
        payload.salary_number.trim().to_string()
    };
    let _ = crate::core::pipeline::park_save(books.conn(), "salary", &key, &session.email);
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
    let session = require_mutate(&state)?;
    let books = state.books.lock().expect("local books");
    let key = match payload.id {
        Some(id) if id > 0 => format!("INV-{id}"),
        _ => format!("INV-new-{}", payload.item_name.trim()),
    };
    let _ = crate::core::pipeline::park_save(books.conn(), "inventory", &key, &session.email);
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
    let session = require_mutate(&state)?;
    let books = state.books.lock().expect("local books");
    let key = format!("INV-{id}");
    let _ = crate::core::pipeline::park_save(books.conn(), "inventory", &key, &session.email);
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
    let session = require_mutate(&state)?;
    let books = state.books.lock().expect("local books");
    let key = match payload.id {
        Some(id) if id > 0 => format!("TRIP-{id}"),
        _ => format!("TRIP-new-{}", payload.vehicle_number.trim()),
    };
    let _ = crate::core::pipeline::park_save(books.conn(), "logistics", &key, &session.email);
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
    let session = require_mutate(&state)?;
    let books = state.books.lock().expect("local books");
    let key = format!("TRIP-{id}");
    let _ = crate::core::pipeline::park_save(books.conn(), "logistics", &key, &session.email);
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
    let session = require_mutate(&state)?;
    let books = state.books.lock().expect("local books");
    let key = match payload.id {
        Some(id) if id > 0 => format!("DOC-{id}"),
        _ => format!("DOC-new-{}", payload.name.trim()),
    };
    let _ = crate::core::pipeline::park_save(books.conn(), "document", &key, &session.email);
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
fn list_vendors(state: tauri::State<AppState>) -> std::result::Result<Vec<crate::VendorRow>, String> {
    let books = state.books.lock().expect("local books");
    crate::masters::list_vendors(&books).map_err(map_err)
}

#[tauri::command]
fn list_vendor_accounts(
    state: tauri::State<AppState>,
    vendor: String,
) -> std::result::Result<Vec<crate::vendor_master::VendorAccount>, String> {
    let books = state.books.lock().expect("local books");
    crate::vendor_master::list_accounts(&books, &vendor).map_err(map_err)
}

#[tauri::command]
fn save_vendor(
    state: tauri::State<AppState>,
    payload: crate::vendor_master::VendorSave,
) -> std::result::Result<crate::VendorRow, String> {
    require_mutate(&state)?;
    let books = state.books.lock().expect("local books");
    crate::vendor_master::save_vendor(&books, payload).map_err(map_err)
}

#[tauri::command]
fn merge_onto_purchase(
    state: tauri::State<AppState>,
    po_number: String,
    voucher_numbers: Vec<i64>,
) -> std::result::Result<crate::MergeReport, String> {
    require_mutate(&state)?;
    let mut books = state.books.lock().expect("local books");
    crate::merge_onto_purchase(&mut books, &po_number, &voucher_numbers).map_err(map_err)
}

#[tauri::command]
fn unmerge_voucher(
    state: tauri::State<AppState>,
    voucher_number: i64,
) -> std::result::Result<(), String> {
    require_mutate(&state)?;
    let mut books = state.books.lock().expect("local books");
    crate::unmerge_voucher(&mut books, voucher_number).map_err(map_err)
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
    let session = require_mutate(&state)?;
    let mut books = state.books.lock().expect("local books");
    let key = if payload.pay_number.trim().is_empty() {
        format!("PAY-id-{}", payload.id.unwrap_or(0))
    } else {
        payload.pay_number.trim().to_string()
    };
    let _ = crate::core::pipeline::park_save(books.conn(), "payment", &key, &session.email);
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
    let actor = actor_email(&state);
    let allow = state
        .session
        .lock()
        .expect("session")
        .as_ref()
        .map(|s| s.role == "owner" || crate::is_hardcoded_owner(&s.email))
        .unwrap_or(false);
    let books = state.books.lock().expect("local books");
    let decision = crate::core::pipeline::gate_submit(
        books.conn(),
        &kind,
        &key,
        is_online(),
        true,
        &actor,
    )
    .map_err(map_err)?;
    if let crate::core::pipeline::Decision::Refuse { message } = decision {
        return Err(message);
    }
    crate::office_submit::submit_office(&books, &kind, &key, allow).map_err(map_err)
}

#[tauri::command]
fn get_dirty_keys(state: tauri::State<AppState>) -> std::result::Result<Vec<DirtyKey>, String> {
    let books = state.books.lock().expect("local books");
    crate::hive::list_dirty_keys(&books).map_err(map_err)
}
