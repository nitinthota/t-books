use std::collections::BTreeMap;

use rusqlite::params;

use crate::db::LocalBooks;
use crate::online::is_online;
use crate::sheets::{credentials_exist, fetch_access_values, load_service_account};
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
        _ => "operator".into(),
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
    if !is_online() {
        return Err(BooksError::from(
            "This PC is offline. Cannot refresh the Access list.",
        ));
    }
    let account = load_service_account()?;
    let values = fetch_access_values(&account)?;
    parse_access_values(&values)
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
pub fn access_denial_message(books: &LocalBooks, email: &str) -> Result<Option<String>> {
    if is_hardcoded_owner(email) {
        return Ok(None);
    }
    match find_access_row(books, email)? {
        None => Ok(Some(
            "This email is not on the Access list for this PC.".into(),
        )),
        Some(row) if is_active_yes(&row.active) => Ok(None),
        Some(_) => Ok(Some("This email is not active on the Access list.".into())),
    }
}

pub fn session_role(books: &LocalBooks, email: &str) -> String {
    if is_hardcoded_owner(email) {
        return "owner".into();
    }
    match find_access_row(books, email).ok().flatten() {
        Some(row) if row.role.eq_ignore_ascii_case("admin") => "admin".into(),
        Some(row) if row.role.eq_ignore_ascii_case("owner") => "admin".into(),
        _ => "operator".into(),
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
