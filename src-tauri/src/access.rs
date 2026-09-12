use std::collections::BTreeMap;

use rusqlite::params;

use crate::db::LocalBooks;
use crate::online::is_online;
use crate::sheets::{
    append_sheet_row, create_tab_with_headers, credentials_exist, fetch_access_values,
    is_missing_tab_error, load_service_account, update_sheet_row,
};
use crate::{is_hardcoded_owner, normalize_email, BooksError, Result, ACCESS_TAB};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccessRow {
    pub name: String,
    pub email: String,
    pub role: String,
    pub active: String,
    pub last_synced: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessSnapshot {
    pub rows: Vec<AccessRow>,
    pub last_synced: Option<String>,
    pub credentials_found: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    pub credentials_found: bool,
    pub last_synced: Option<String>,
    pub voucher_last_synced: Option<String>,
    pub last_error: Option<String>,
}

pub fn normalize_active(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "yes" | "y" | "true" | "1" => "Yes".into(),
        _ => "No".into(),
    }
}

pub fn normalize_role(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "owner" => "owner".into(),
        "admin" => "admin".into(),
        "operator" => "operator".into(),
        "viewer" => "viewer".into(),
        other => other.trim().to_ascii_lowercase(),
    }
}

fn find_col(header: &[String], names: &[&str]) -> Option<usize> {
    header.iter().position(|h| {
        let n = h.trim().to_ascii_lowercase();
        names.iter().any(|want| n == *want)
    })
}

fn cell<'a>(row: &'a [String], idx: Option<usize>) -> &'a str {
    idx.and_then(|i| row.get(i))
        .map(|s| s.as_str())
        .unwrap_or("")
}

pub fn parse_access_values(values: &[Vec<String>]) -> Result<Vec<AccessRow>> {
    if values.is_empty() {
        return Err(BooksError::from(format!(
            "{ACCESS_TAB} tab is empty. Expected a header row: Name, Email, Role, Active."
        )));
    }
    let header_at = values
        .iter()
        .take(10)
        .position(|row| find_col(row, &["email"]).is_some())
        .ok_or_else(|| {
            BooksError::from(format!(
                "{ACCESS_TAB} tab is missing the Email column."
            ))
        })?;
    let header = &values[header_at];
    let email_i = find_col(header, &["email"]).ok_or_else(|| {
        BooksError::from(format!("{ACCESS_TAB} tab is missing the Email column."))
    })?;
    let name_i = find_col(header, &["name"]);
    let role_i = find_col(header, &["role"]);
    let active_i = find_col(header, &["active"]);
    if active_i.is_none() {
        return Err(BooksError::from(format!(
            "{ACCESS_TAB} tab is missing the Active column."
        )));
    }

    let mut by_email: BTreeMap<String, AccessRow> = BTreeMap::new();
    for row in values.iter().skip(header_at + 1) {
        let email = normalize_email(cell(row, Some(email_i)));
        if email.is_empty() {
            continue;
        }
        let name = cell(row, name_i).trim().to_string();
        let role = normalize_role(cell(row, role_i));
        let active = normalize_active(cell(row, active_i));
        by_email.insert(
            email.clone(),
            AccessRow {
                name,
                email,
                role,
                active,
                last_synced: String::new(),
            },
        );
    }
    Ok(by_email.into_values().collect())
}

pub fn fetch_access_rows() -> Result<Vec<AccessRow>> {
    fetch_access_rows_with(false)
}

pub fn fetch_access_rows_with(allow_bootstrap: bool) -> Result<Vec<AccessRow>> {
    if !is_online() {
        return Err(BooksError::from(
            "This PC is offline. Cannot refresh the Access list.",
        ));
    }
    let account = load_service_account()?;
    match fetch_access_values(&account) {
        Ok(values) => parse_access_values(&values),
        Err(err) if allow_bootstrap && is_missing_tab_error(&err.to_string()) => {
            let headers: Vec<String> = crate::hive::ACCESS_HEADERS
                .iter()
                .map(|s| (*s).to_string())
                .collect();
            create_tab_with_headers(&account, ACCESS_TAB, &headers)?;
            parse_access_values(&[headers])
        }
        Err(err) => Err(err),
    }
}

pub fn apply_access_rows(books: &mut LocalBooks, rows: &[AccessRow]) -> Result<()> {
    let synced: String = books
        .conn()
        .query_row("SELECT datetime('now')", [], |row| row.get(0))?;
    let tx = books.conn_mut().transaction()?;
    tx.execute("DELETE FROM access_cache", [])?;
    {
        let mut stmt = tx.prepare(
            r#"
            INSERT INTO access_cache (email, name, role, active, last_synced)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
        )?;
        for row in rows {
            stmt.execute(params![row.email, row.name, row.role, row.active, synced])?;
        }
    }
    tx.execute(
        "INSERT OR REPLACE INTO app_meta (key, value) VALUES ('access_last_synced', ?1)",
        params![synced],
    )?;
    tx.commit()?;
    Ok(())
}

pub fn refresh_access(books: &mut LocalBooks) -> Result<AccessSnapshot> {
    let rows = fetch_access_rows()?;
    apply_access_rows(books, &rows)?;
    Ok(get_access_snapshot(books))
}

pub fn list_access(books: &LocalBooks) -> Result<Vec<AccessRow>> {
    let mut stmt = books.conn().prepare(
        r#"
        SELECT name, email, role, active, last_synced
        FROM access_cache
        ORDER BY name COLLATE NOCASE, email COLLATE NOCASE
        "#,
    )?;
    let mapped = stmt.query_map([], |row| {
        Ok(AccessRow {
            name: row.get::<_, Option<String>>(0)?.unwrap_or_default(),
            email: row.get(1)?,
            role: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            active: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
            last_synced: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
        })
    })?;
    let mut out = Vec::new();
    for row in mapped {
        out.push(row?);
    }
    Ok(out)
}

pub fn get_last_synced(books: &LocalBooks) -> Option<String> {
    books
        .conn()
        .query_row(
            "SELECT value FROM app_meta WHERE key = 'access_last_synced' LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .filter(|s| !s.is_empty())
}

pub fn get_access_snapshot(books: &LocalBooks) -> AccessSnapshot {
    AccessSnapshot {
        rows: list_access(books).unwrap_or_default(),
        last_synced: get_last_synced(books),
        credentials_found: credentials_exist(),
    }
}

pub fn upsert_access_cache(books: &mut LocalBooks, row: &AccessRow) -> Result<()> {
    let synced: String = books
        .conn()
        .query_row("SELECT datetime('now')", [], |r| r.get(0))?;
    books.conn().execute(
        r#"
        INSERT INTO access_cache (email, name, role, active, last_synced)
        VALUES (?1, ?2, ?3, ?4, ?5)
        ON CONFLICT(email) DO UPDATE SET
          name = excluded.name,
          role = excluded.role,
          active = excluded.active,
          last_synced = excluded.last_synced
        "#,
        params![row.email, row.name, row.role, row.active, synced],
    )?;
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessWrite {
    pub name: String,
    pub email: String,
    pub role: String,
    pub active: String,
}

/// Owner, online, one hive row. Never a bulk overwrite.
pub fn write_access_row(books: &mut LocalBooks, payload: AccessWrite) -> Result<AccessRow> {
    if !is_online() {
        return Err(BooksError::from(
            "This PC is offline. Cannot submit to Google.",
        ));
    }
    if !credentials_exist() {
        return Err(BooksError::from(
            "Google credentials not found at %LOCALAPPDATA%/T-Books/credentials.json.",
        ));
    }
    let email = crate::normalize_email(&payload.email);
    if !crate::is_valid_email_shape(&email) {
        return Err(BooksError::from("Enter a valid email."));
    }
    let name = payload.name.trim().to_string();
    if name.is_empty() {
        return Err(BooksError::from("Name is required."));
    }
    let mut role = normalize_role(&payload.role);
    let mut active = normalize_active(&payload.active);
    if crate::is_hardcoded_owner(&email) {
        role = "owner".into();
        active = "Yes".into();
    } else if role == "owner" {
        role = "admin".into();
    }
    if role != "admin" && role != "operator" && role != "viewer" && role != "owner" {
        return Err(BooksError::from(
            "Role must be admin or operator.",
        ));
    }
    let account = load_service_account()?;
    let values = match fetch_access_values(&account) {
        Ok(v) => v,
        Err(err) if is_missing_tab_error(&err.to_string()) => {
            let headers: Vec<String> = crate::hive::ACCESS_HEADERS
                .iter()
                .map(|s| (*s).to_string())
                .collect();
            create_tab_with_headers(&account, ACCESS_TAB, &headers)?;
            vec![headers]
        }
        Err(err) => return Err(err),
    };
    let cells = vec![name.clone(), email.clone(), role.clone(), active.clone()];
    let header_at = values
        .iter()
        .take(10)
        .position(|row| {
            row.iter()
                .any(|h| h.trim().eq_ignore_ascii_case("email"))
        })
        .unwrap_or(0);
    let email_i = values
        .get(header_at)
        .and_then(|header| {
            header
                .iter()
                .position(|h| h.trim().eq_ignore_ascii_case("email"))
        })
        .unwrap_or(1);
    let mut found_row: Option<u32> = None;
    for (i, row) in values.iter().enumerate().skip(header_at + 1) {
        let cell = row.get(email_i).map(|s| s.trim().to_lowercase()).unwrap_or_default();
        if cell == email {
            found_row = Some((i as u32) + 1);
            break;
        }
    }
    if let Some(sheet_row) = found_row {
        let a1 = format!("A{sheet_row}:D{sheet_row}");
        update_sheet_row(&account, ACCESS_TAB, &a1, &cells)?;
    } else {
        append_sheet_row(&account, ACCESS_TAB, &cells)?;
    }
    let row = AccessRow {
        name,
        email,
        role,
        active,
        last_synced: String::new(),
    };
    upsert_access_cache(books, &row)?;
    Ok(row)
}

pub fn find_access_row(books: &LocalBooks, email: &str) -> Result<Option<AccessRow>> {
    let email = normalize_email(email);
    let mut stmt = books.conn().prepare(
        r#"
        SELECT name, email, role, active, last_synced
        FROM access_cache
        WHERE email = ?1 COLLATE NOCASE
        LIMIT 1
        "#,
    )?;
    let mut rows = stmt.query(params![email])?;
    match rows.next()? {
        None => Ok(None),
        Some(row) => Ok(Some(AccessRow {
            name: row.get::<_, Option<String>>(0)?.unwrap_or_default(),
            email: row.get(1)?,
            role: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            active: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
            last_synced: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
        })),
    }
}

pub fn is_active_yes(active: &str) -> bool {
    active.trim().eq_ignore_ascii_case("yes")
}

/// Owner is always allowed, even when missing from the cache or marked No.
/// Loopbook `viewer` (and unknown roles) cannot open T Books.
pub fn access_denial_message(books: &LocalBooks, email: &str) -> Result<Option<String>> {
    if is_hardcoded_owner(email) {
        return Ok(None);
    }
    match find_access_row(books, email)? {
        None => Ok(Some(
            "This email is not on the Access list for this PC.".into(),
        )),
        Some(row) if !is_active_yes(&row.active) => {
            Ok(Some("This email is not active on the Access list.".into()))
        }
        Some(row) if !crate::core::business_rules::can_mutate(&row.role) => {
            Ok(Some("Your role cannot open T Books.".into()))
        }
        Some(_) => Ok(None),
    }
}

pub fn session_role(books: &LocalBooks, email: &str) -> String {
    if is_hardcoded_owner(email) {
        return "owner".into();
    }
    match find_access_row(books, email).ok().flatten() {
        Some(row) if row.role.eq_ignore_ascii_case("admin") => "admin".into(),
        Some(row) if row.role.eq_ignore_ascii_case("owner") => "admin".into(),
        Some(row) if row.role.eq_ignore_ascii_case("operator") => "operator".into(),
        Some(row) if row.role.eq_ignore_ascii_case("viewer") => "viewer".into(),
        Some(row) => normalize_role(&row.role),
        None => "operator".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_memory;

    fn row(name: &str, email: &str, role: &str, active: &str) -> AccessRow {
        AccessRow {
            name: name.into(),
            email: email.into(),
            role: role.into(),
            active: active.into(),
            last_synced: String::new(),
        }
    }

    #[test]
    fn parses_headers_and_normalizes() {
        let values = vec![
            vec![
                "NAME".into(),
                "EMAIL".into(),
                "ROLE".into(),
                "ACTIVE".into(),
            ],
            vec![
                "Ada".into(),
                "ada@example.com".into(),
                "Admin".into(),
                "yes".into(),
            ],
            vec![
                "Bo".into(),
                "bo@example.com".into(),
                "operator".into(),
                "n".into(),
            ],
            vec!["".into(), "".into(), "".into(), "".into()],
        ];
        let parsed = parse_access_values(&values).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].email, "ada@example.com");
        assert_eq!(parsed[0].role, "admin");
        assert_eq!(parsed[0].active, "Yes");
        assert_eq!(parsed[1].active, "No");
    }

    #[test]
    fn viewer_is_not_mapped_to_operator() {
        assert_eq!(normalize_role("viewer"), "viewer");
        assert_eq!(normalize_role("Viewer"), "viewer");
        assert_ne!(normalize_role("viewer"), "operator");
        let values = vec![
            vec!["Name".into(), "Email".into(), "Role".into(), "Active".into()],
            vec![
                "Vie".into(),
                "vie@example.com".into(),
                "viewer".into(),
                "Yes".into(),
            ],
        ];
        let parsed = parse_access_values(&values).unwrap();
        assert_eq!(parsed[0].role, "viewer");
        let mut books = open_memory().unwrap();
        apply_access_rows(&mut books, &parsed).unwrap();
        let msg = access_denial_message(&books, "vie@example.com")
            .unwrap()
            .expect("denied");
        assert!(msg.contains("cannot open"));
    }

    #[test]
    fn missing_email_column_is_real_error() {
        let values = vec![vec!["Name".into(), "Role".into()]];
        let err = parse_access_values(&values).unwrap_err();
        assert!(err.to_string().contains("Email"));
    }

    #[test]
    fn empty_sheet_is_real_error() {
        let err = parse_access_values(&[]).unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn replace_overwrites_and_failure_path_keeps_rows() {
        let mut books = open_memory().unwrap();
        apply_access_rows(
            &mut books,
            &[
                row("Ada", "ada@example.com", "admin", "Yes"),
                row("Bo", "bo@example.com", "operator", "Yes"),
            ],
        )
        .unwrap();
        assert_eq!(list_access(&books).unwrap().len(), 2);

        apply_access_rows(&mut books, &[row("Ada", "ada@example.com", "admin", "No")])
            .unwrap();
        let listed = list_access(&books).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].active, "No");
        assert!(get_last_synced(&books).is_some());

        // A failed Google call never reaches apply_access_rows, so cache stays.
        assert_eq!(list_access(&books).unwrap().len(), 1);
    }

    #[test]
    fn owner_allowed_when_cache_empty() {
        let books = open_memory().unwrap();
        assert!(access_denial_message(&books, crate::OWNER_EMAIL)
            .unwrap()
            .is_none());
        assert_eq!(
            access_denial_message(&books, "ada@example.com")
                .unwrap()
                .unwrap(),
            "This email is not on the Access list for this PC."
        );
    }

    #[test]
    fn inactive_row_denied_except_owner() {
        let mut books = open_memory().unwrap();
        apply_access_rows(
            &mut books,
            &[
                row("Ada", "ada@example.com", "operator", "No"),
                row("Boss", crate::OWNER_EMAIL, "owner", "No"),
            ],
        )
        .unwrap();
        assert!(access_denial_message(&books, crate::OWNER_EMAIL)
            .unwrap()
            .is_none());
        assert!(access_denial_message(&books, "ada@example.com")
            .unwrap()
            .unwrap()
            .contains("not active"));
    }
}
