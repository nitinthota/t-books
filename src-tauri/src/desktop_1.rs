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
