fn actor_email(state: &tauri::State<AppState>) -> String {
    state
        .session
        .lock()
        .expect("session")
        .as_ref()
        .map(|s| s.email.clone())
        .unwrap_or_default()
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
    let actor = actor_email(&state);
    let mut books = state.books.lock().expect("local books");
    let dirty = crate::hive::list_dirty_keys(&books).unwrap_or_default();
    let dirty_keys: Vec<String> = dirty.into_iter().map(|k| format!("{}:{}", k.kind, k.key)).collect();
    let _ = crate::core::pipeline::gate_refresh(
        books.conn(),
        "voucher",
        is_online(),
        dirty_keys,
        &actor,
    );
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
    let actor = actor_email(&state);
    let mut books = state.books.lock().expect("local books");
    let _ = crate::core::pipeline::record(
        books.conn(),
        crate::core::pipeline::Intent::DiscardLocal,
        "voucher",
        "*",
        &crate::core::pipeline::Facts::refresh(is_online(), vec![]),
        &actor,
    );
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
    crate::voucher_edit::get_voucher_full(&books, voucher_number).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_voucher(
    state: tauri::State<AppState>,
    payload: VoucherSave,
) -> std::result::Result<crate::vouchers::VoucherView, String> {
    let session = require_mutate(&state)?;
    let mut books = state.books.lock().expect("local books");
    let key = payload.voucher_number.to_string();
    let _ = crate::core::pipeline::park_save(books.conn(), "voucher", &key, &session.email);
    crate::save_voucher(&mut books, payload).map_err(|e| e.to_string())
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
    let actor = actor_email(&state);
    let mut books = state.books.lock().expect("local books");
    let decision = crate::core::pipeline::gate_submit(
        books.conn(),
        "voucher",
        &voucher_number.to_string(),
        is_online(),
        true,
        &actor,
    )
    .map_err(|e| e.to_string())?;
    if let crate::core::pipeline::Decision::Refuse { message } = decision {
        return Err(message);
    }
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
    let session = require_mutate(&state)?;
    let mut books = state.books.lock().expect("local books");
    let key = if payload.po_number.trim().is_empty() {
        format!("PUR-id-{}", payload.id.unwrap_or(0))
    } else {
        payload.po_number.trim().to_string()
    };
    let _ = crate::core::pipeline::park_save(books.conn(), "purchase", &key, &session.email);
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
