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
