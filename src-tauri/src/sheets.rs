use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};

use crate::db::data_dir;
use crate::hive_plan::refuse_raw_write;
use crate::{
    BooksError, CREDENTIALS_FILE_NAME, DEFAULT_SPREADSHEET_ID, Result,
};

const TOKEN_URI_DEFAULT: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_SCOPE: &str = "https://www.googleapis.com/auth/spreadsheets https://www.googleapis.com/auth/drive";
const HTTP_TIMEOUT_SECS: u64 = 15;

pub struct ServiceAccount {
    client_email: String,
    private_key: String,
    private_key_id: Option<String>,
    token_uri: String,
    pub spreadsheet_id: String,
}

struct CachedToken {
    iss: String,
    scope: String,
    token: String,
    exp: u64,
}

fn token_cache() -> &'static Mutex<Option<CachedToken>> {
    static CACHE: OnceLock<Mutex<Option<CachedToken>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

pub fn credentials_path() -> PathBuf {
    data_dir().join(CREDENTIALS_FILE_NAME)
}

fn workspace_secret_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../secrets/google-service-account.json")
}

pub fn credential_search_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(from_env) = std::env::var("TBOOKS_GOOGLE_SA_PATH") {
        let trimmed = from_env.trim();
        if !trimmed.is_empty() {
            paths.push(PathBuf::from(trimmed));
        }
    }
    paths.push(credentials_path());
    paths.push(workspace_secret_path());
    paths
}

pub fn credentials_exist() -> bool {
    if let Ok(raw) = std::env::var("TBOOKS_GOOGLE_SA_JSON") {
        if !raw.trim().is_empty() {
            return true;
        }
    }
    credential_search_paths().iter().any(|p| p.is_file())
}

pub fn parse_sheet_id(input: &str) -> String {
    let s = input.trim();
    if s.is_empty() {
        return DEFAULT_SPREADSHEET_ID.to_string();
    }
    if let Some(rest) = s.split("/spreadsheets/d/").nth(1) {
        let id = rest.split('/').next().unwrap_or(rest);
        let id = id.split('?').next().unwrap_or(id).trim();
        if looks_like_sheet_id(id) {
            return id.to_string();
        }
    }
    if looks_like_sheet_id(s) {
        return s.to_string();
    }
    DEFAULT_SPREADSHEET_ID.to_string()
}

fn looks_like_sheet_id(id: &str) -> bool {
    let len = id.len();
    (20..80).contains(&len) && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

pub fn parse_service_account(raw: &str) -> Result<ServiceAccount> {
    let text = raw.trim().trim_start_matches('\u{feff}');
    let value: serde_json::Value = serde_json::from_str(text)
        .map_err(|_| BooksError::from("credentials.json is not valid JSON."))?;
    let client_email = value
        .get("client_email")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    let private_key = value
        .get("private_key")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if client_email.is_empty() || private_key.is_empty() {
        return Err(BooksError::from(
            "credentials.json is missing client_email or private_key.",
        ));
    }
    let private_key = private_key.replace("\\n", "\n");
    let private_key_id = value
        .get("private_key_id")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let token_uri = value
        .get("token_uri")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| TOKEN_URI_DEFAULT.to_string());
    let spreadsheet_id = value
        .get("spreadsheet_id")
        .or_else(|| value.get("spreadsheetId"))
        .or_else(|| value.get("sheet_id"))
        .and_then(|v| v.as_str())
        .map(parse_sheet_id)
        .filter(|s| looks_like_sheet_id(s))
        .unwrap_or_else(|| DEFAULT_SPREADSHEET_ID.to_string());
    Ok(ServiceAccount {
        client_email: client_email.to_string(),
        private_key,
        private_key_id,
        token_uri,
        spreadsheet_id,
    })
}

fn read_service_account_json() -> Result<String> {
    if let Ok(raw) = std::env::var("TBOOKS_GOOGLE_SA_JSON") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }
    for path in credential_search_paths() {
        if path.is_file() {
            return std::fs::read_to_string(&path)
                .map_err(|_| BooksError::from("Could not read the Google service account file on this PC."));
        }
    }
    Err(BooksError::from(
        "Google credentials not found at %LOCALAPPDATA%/T-Books/credentials.json.",
    ))
}

pub fn load_service_account() -> Result<ServiceAccount> {
    let raw = read_service_account_json()?;
    parse_service_account(&raw)
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(HTTP_TIMEOUT_SECS))
        .user_agent("T-Books/0.1")
        .build()
}

#[derive(Serialize)]
struct SaClaims {
    iss: String,
    scope: String,
    aud: String,
    iat: u64,
    exp: u64,
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn sign_jwt(account: &ServiceAccount) -> Result<String> {
    let iat = now_unix();
    let claims = SaClaims {
        iss: account.client_email.clone(),
        scope: GOOGLE_SCOPE.to_string(),
        aud: TOKEN_URI_DEFAULT.to_string(),
        iat,
        exp: iat + 3600,
    };
    let mut header = Header::new(Algorithm::RS256);
    header.typ = Some("JWT".into());
    if let Some(kid) = &account.private_key_id {
        header.kid = Some(kid.clone());
    }
    let key = EncodingKey::from_rsa_pem(account.private_key.as_bytes()).map_err(|_| {
        BooksError::from("credentials.json private_key is not a valid RSA PEM.")
    })?;
    encode(&header, &claims, &key)
        .map_err(|_| BooksError::from("Could not sign the Google service account JWT on this PC."))
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    expires_in: Option<u64>,
    error: Option<String>,
    error_description: Option<String>,
}

fn access_token(account: &ServiceAccount) -> Result<String> {
    {
        let cache = token_cache()
            .lock()
            .map_err(|_| BooksError::from("Could not lock the Google token cache on this PC."))?;
        if let Some(cached) = cache.as_ref() {
            if cached.iss == account.client_email
                && cached.scope == GOOGLE_SCOPE
                && cached.exp > now_unix() + 60
            {
                return Ok(cached.token.clone());
            }
        }
    }

    let jwt = sign_jwt(account)?;
    let response = agent()
        .post(&account.token_uri)
        .set("Accept", "application/json")
        .send_form(&[
            ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
            ("assertion", &jwt),
        ]);

    let body = match response {
        Ok(resp) => resp
            .into_string()
            .map_err(|_| BooksError::from("Google token endpoint returned an unreadable body."))?,
        Err(ureq::Error::Status(_, resp)) => resp.into_string().unwrap_or_default(),
        Err(_) => {
            return Err(BooksError::from(
                "Could not reach Google token endpoint from this PC.",
            ));
        }
    };

    let parsed: TokenResponse = serde_json::from_str(&body).unwrap_or(TokenResponse {
        access_token: None,
        expires_in: None,
        error: None,
        error_description: None,
    });
    let Some(token) = parsed.access_token.filter(|t| !t.is_empty()) else {
        let message = parsed
            .error_description
            .or(parsed.error)
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| {
                "Google rejected the service account JWT. Enable the Google Sheets API on that Cloud project.".into()
            });
        return Err(BooksError::from(sanitize_google_error(&message)));
    };

    let exp = now_unix() + parsed.expires_in.unwrap_or(3600);
    if let Ok(mut cache) = token_cache().lock() {
        *cache = Some(CachedToken {
            iss: account.client_email.clone(),
            scope: GOOGLE_SCOPE.to_string(),
            token: token.clone(),
            exp,
        });
    }
    Ok(token)
}

fn encode_range(tab: &str) -> String {
    encode_a1(tab, None)
}

fn encode_a1(tab: &str, a1: Option<&str>) -> String {
    let quoted = format!("'{}'", tab.replace('\'', "''"));
    let full = match a1 {
        Some(range) => format!("{quoted}!{range}"),
        None => quoted,
    };
    let mut out = String::new();
    for b in full.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[derive(Deserialize)]
struct SheetsErrorEnvelope {
    error: Option<SheetsErrorBody>,
}

#[derive(Deserialize)]
struct SheetsErrorBody {
    message: Option<String>,
    status: Option<String>,
}

#[derive(Deserialize)]
struct SheetsValues {
    values: Option<Vec<Vec<serde_json::Value>>>,
}

fn cell_to_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => if *b { "Yes" } else { "No" }.to_string(),
        other => other.to_string(),
    }
}

fn google_http_message(code: u16, body: &str, tab: &str) -> String {
    let parsed = serde_json::from_str::<SheetsErrorEnvelope>(body).ok();
    let api = parsed
        .as_ref()
        .and_then(|e| e.error.as_ref())
        .and_then(|e| e.message.as_deref())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(sanitize_google_error);
    match (code, api) {
        (403, Some(msg)) => format!(
            "{msg} Share the Restricted spreadsheet with the service account from credentials.json as Editor."
        ),
        (401, Some(msg)) => msg,
        (404, Some(msg)) => format!(
            "{msg} Check the Sheet ID and that a tab named {tab} exists."
        ),
        (_, Some(msg)) => msg,
        (0, None) => "Could not reach Google Sheets from this PC.".into(),
        (code, None) => format!("Google Sheets HTTP {code}."),
    }
}

fn sanitize_google_error(message: &str) -> String {
    let mut out = String::with_capacity(message.len());
    let mut rest = message;
    while let Some(idx) = rest.find("Bearer ") {
        out.push_str(&rest[..idx]);
        out.push_str("Bearer [redacted]");
        let after = &rest[idx + "Bearer ".len()..];
        rest = after.split_whitespace().nth(1).unwrap_or("");
        if rest.is_empty() {
            break;
        }
        out.push(' ');
    }
    out.push_str(rest);
    out
}

fn parse_values_body(body: &str) -> Result<Vec<Vec<String>>> {
    if let Ok(env) = serde_json::from_str::<SheetsErrorEnvelope>(body) {
        if let Some(err) = env.error {
            if let Some(msg) = err.message.or(err.status) {
                return Err(BooksError::from(sanitize_google_error(&msg)));
            }
        }
    }
    let parsed: SheetsValues = serde_json::from_str(body)
        .map_err(|_| BooksError::from("Google Sheets returned a non-JSON body."))?;
    let values = parsed.values.unwrap_or_default();
    Ok(values
        .into_iter()
        .map(|row| row.iter().map(cell_to_string).collect())
        .collect())
}

fn get_values(account: &ServiceAccount, spreadsheet_id: &str, encoded_range: &str, tab: &str) -> Result<Vec<Vec<String>>> {
    let token = access_token(account)?;
    let url = format!(
        "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}?majorDimension=ROWS&valueRenderOption=FORMATTED_VALUE&dateTimeRenderOption=FORMATTED_STRING",
        spreadsheet_id,
        encoded_range
    );
    let auth = format!("Bearer {token}");
    let response = agent()
        .get(&url)
        .set("Authorization", &auth)
        .set("Accept", "application/json")
        .call();
    match response {
        Ok(resp) => {
            let body = resp
                .into_string()
                .map_err(|_| BooksError::from("Google Sheets returned an unreadable body."))?;
            parse_values_body(&body)
        }
        Err(ureq::Error::Status(code, resp)) => {
            let body = resp.into_string().unwrap_or_default();
            Err(BooksError::from(google_http_message(code, &body, tab)))
        }
        Err(_) => Err(BooksError::from(
            "Could not reach Google Sheets from this PC.",
        )),
    }
}

pub fn fetch_values_on(
    account: &ServiceAccount,
    spreadsheet_id: &str,
    tab: &str,
) -> Result<Vec<Vec<String>>> {
    get_values(account, spreadsheet_id, &encode_range(tab), tab)
}

pub fn fetch_sheet_values(account: &ServiceAccount, tab: &str) -> Result<Vec<Vec<String>>> {
    fetch_values_on(account, &account.spreadsheet_id, tab)
}

pub fn fetch_access_values(account: &ServiceAccount) -> Result<Vec<Vec<String>>> {
    let target = crate::hive_plan::target_for_kind(crate::hive::KIND_ACCESS)?;
    fetch_values_on(account, &target.spreadsheet_id, &target.tab)
}

pub fn fetch_a1_on(
    account: &ServiceAccount,
    spreadsheet_id: &str,
    tab: &str,
    a1: &str,
) -> Result<Vec<Vec<String>>> {
    get_values(account, spreadsheet_id, &encode_a1(tab, Some(a1)), tab)
}

#[allow(dead_code)]
pub fn fetch_sheet_a1(account: &ServiceAccount, tab: &str, a1: &str) -> Result<Vec<Vec<String>>> {
    fetch_a1_on(account, &account.spreadsheet_id, tab, a1)
}

fn values_url_on(spreadsheet_id: &str, tab: &str, a1: &str) -> String {
    format!(
        "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}",
        spreadsheet_id,
        encode_a1(tab, Some(a1))
    )
}

pub fn update_row_on(
    account: &ServiceAccount,
    spreadsheet_id: &str,
    tab: &str,
    a1: &str,
    cells: &[String],
) -> Result<()> {
    refuse_raw_write(spreadsheet_id, tab)?;
    let token = access_token(account)?;
    let url = format!("{}?valueInputOption=RAW", values_url_on(spreadsheet_id, tab, a1));
    let body = serde_json::json!({
        "range": format!("'{tab}'!{a1}"),
        "majorDimension": "ROWS",
        "values": [cells],
    });
    let auth = format!("Bearer {token}");
    let response = agent()
        .put(&url)
        .set("Authorization", &auth)
        .set("Accept", "application/json")
        .set("Content-Type", "application/json")
        .send_json(body);
    match response {
        Ok(_) => Ok(()),
        Err(ureq::Error::Status(code, resp)) => {
            let text = resp.into_string().unwrap_or_default();
            Err(BooksError::from(google_http_message(code, &text, tab)))
        }
        Err(_) => Err(BooksError::from(
            "Could not reach Google Sheets from this PC.",
        )),
    }
}

#[allow(dead_code)]
pub fn update_sheet_row(
    account: &ServiceAccount,
    tab: &str,
    a1: &str,
    cells: &[String],
) -> Result<()> {
    update_row_on(account, &account.spreadsheet_id, tab, a1, cells)
}

pub fn append_row_on(
    account: &ServiceAccount,
    spreadsheet_id: &str,
    tab: &str,
    cells: &[String],
) -> Result<()> {
    refuse_raw_write(spreadsheet_id, tab)?;
    let token = access_token(account)?;
    let url = format!(
        "{}:append?valueInputOption=RAW&insertDataOption=INSERT_ROWS",
        values_url_on(spreadsheet_id, tab, "A:BL")
    );
    let body = serde_json::json!({
        "majorDimension": "ROWS",
        "values": [cells],
    });
    let auth = format!("Bearer {token}");
    let response = agent()
        .post(&url)
        .set("Authorization", &auth)
        .set("Accept", "application/json")
        .set("Content-Type", "application/json")
        .send_json(body);
    match response {
        Ok(_) => Ok(()),
        Err(ureq::Error::Status(code, resp)) => {
            let text = resp.into_string().unwrap_or_default();
            Err(BooksError::from(google_http_message(code, &text, tab)))
        }
        Err(_) => Err(BooksError::from(
            "Could not reach Google Sheets from this PC.",
        )),
    }
}

#[allow(dead_code)]
pub fn append_sheet_row(account: &ServiceAccount, tab: &str, cells: &[String]) -> Result<()> {
    append_row_on(account, &account.spreadsheet_id, tab, cells)
}

pub fn is_missing_tab_error(message: &str) -> bool {
    let m = message.to_ascii_lowercase();
    m.contains("unable to parse range")
        || m.contains("unable to parse the range")
        || m.contains("not found")
        || (m.contains("tab named") && m.contains("exist"))
}

/// Owner bootstrap: create the tab with headers only. Never copies data rows.
pub fn create_tab_with_headers_on(
    account: &ServiceAccount,
    spreadsheet_id: &str,
    tab: &str,
    headers: &[String],
) -> Result<()> {
    refuse_raw_write(spreadsheet_id, tab)?;
    let token = access_token(account)?;
    let url = format!(
        "https://sheets.googleapis.com/v4/spreadsheets/{spreadsheet_id}:batchUpdate"
    );
    let body = serde_json::json!({
        "requests": [{
            "addSheet": {
                "properties": { "title": tab }
            }
        }]
    });
    let auth = format!("Bearer {token}");
    let response = agent()
        .post(&url)
        .set("Authorization", &auth)
        .set("Accept", "application/json")
        .set("Content-Type", "application/json")
        .send_json(body);
    match response {
        Ok(_) => {}
        Err(ureq::Error::Status(code, resp)) => {
            let text = resp.into_string().unwrap_or_default();
            let lower = text.to_ascii_lowercase();
            if !(lower.contains("already exists") || lower.contains("duplicate")) {
                return Err(BooksError::from(google_http_message(code, &text, tab)));
            }
        }
        Err(_) => {
            return Err(BooksError::from(
                "Could not reach Google Sheets from this PC.",
            ));
        }
    }
    if headers.is_empty() {
        return Ok(());
    }
    let last = column_letter(headers.len().saturating_sub(1));
    let a1 = format!("A1:{last}1");
    update_row_on(account, spreadsheet_id, tab, &a1, headers)
}

fn column_letter(index: usize) -> String {
    let mut n = index as i32;
    let mut out = String::new();
    loop {
        let rem = n % 26;
        out.insert(0, (b'A' + rem as u8) as char);
        n = n / 26 - 1;
        if n < 0 {
            break;
        }
    }
    out
}

#[allow(dead_code)]
pub fn create_tab_with_headers(
    account: &ServiceAccount,
    tab: &str,
    headers: &[String],
) -> Result<()> {
    create_tab_with_headers_on(account, &account.spreadsheet_id, tab, headers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_requires_email_and_key() {
        match parse_service_account(r#"{"type":"service_account"}"#) {
            Err(err) => {
                let message = err.to_string();
                assert!(message.contains("client_email") || message.contains("private_key"));
            }
            Ok(_) => panic!("expected credentials parse error"),
        }
    }

    #[test]
    fn parse_rejects_non_json() {
        match parse_service_account("not-json") {
            Err(err) => assert!(err.to_string().contains("valid JSON")),
            Ok(_) => panic!("expected credentials parse error"),
        }
    }

    #[test]
    fn parse_reads_optional_spreadsheet_id() {
        let account = parse_service_account(
            r#"{
              "client_email": "sa@example.com",
              "private_key": "-----BEGIN PRIVATE KEY-----\nMIIB\n-----END PRIVATE KEY-----\n",
              "spreadsheet_id": "1J9ZuNL1uZ7DmqOGIZojuOCMeYp9VnC-SEow6YG86cgE"
            }"#,
        )
        .unwrap();
        assert_eq!(
            account.spreadsheet_id,
            "1J9ZuNL1uZ7DmqOGIZojuOCMeYp9VnC-SEow6YG86cgE"
        );
    }

    #[test]
    fn sheet_id_from_url() {
        let id = parse_sheet_id(
            "https://docs.google.com/spreadsheets/d/1J9ZuNL1uZ7DmqOGIZojuOCMeYp9VnC-SEow6YG86cgE/edit#gid=0",
        );
        assert_eq!(id, "1J9ZuNL1uZ7DmqOGIZojuOCMeYp9VnC-SEow6YG86cgE");
    }

    #[test]
    fn missing_tab_error_is_detected() {
        assert!(is_missing_tab_error(
            "Unable to parse range: 'Voucher_Raw_Data'"
        ));
        assert!(is_missing_tab_error(
            "Check the Sheet ID and that a tab named Access exists."
        ));
        assert!(!is_missing_tab_error("Could not reach Google Sheets from this PC."));
    }

    #[test]
    fn update_refuses_raw_archive_without_calling_google() {
        let account = parse_service_account(
            r#"{
              "client_email": "sa@example.com",
              "private_key": "-----BEGIN PRIVATE KEY-----\nMIIB\n-----END PRIVATE KEY-----\n"
            }"#,
        )
        .unwrap();
        let err = update_row_on(
            &account,
            "1J9ZuNL1uZ7DmqOGIZojuOCMeYp9VnC-SEow6YG86cgE",
            "Voucher_Raw_Data",
            "A1",
            &["20".into()],
        )
        .unwrap_err();
        assert!(err.to_string().contains("read-only"));
        let err = append_row_on(
            &account,
            "1J9ZuNL1uZ7DmqOGIZojuOCMeYp9VnC-SEow6YG86cgE",
            "Voucher_Raw_Data",
            &["VOUCHER_1001".into()],
        )
        .unwrap_err();
        assert!(err.to_string().contains("read-only"));
    }

    /// Optional live READ of Voucher_Raw_Data. Never writes. Skips without a service account.
    #[test]
    fn optional_live_read_of_raw_archive_never_writes() {
        crate::hive_plan::refuse_raw_write(crate::DEFAULT_SPREADSHEET_ID, crate::VOUCHER_RAW_TAB)
            .unwrap_err();
        if !credentials_exist() {
            eprintln!("skip live Google: no TBOOKS_GOOGLE_SA_JSON / credentials.json");
            return;
        }
        let account = match load_service_account() {
            Ok(a) => a,
            Err(err) => {
                if std::env::var("TBOOKS_LIVE_GOOGLE").ok().as_deref() == Some("1") {
                    panic!("{}", err);
                }
                eprintln!("skip live Google: could not parse service account");
                return;
            }
        };
        let map = crate::hive_plan::load_map().unwrap();
        match fetch_values_on(&account, &map.raw_source.spreadsheet_id, &map.raw_source.tab) {
            Ok(values) => {
                let n = values.len();
                assert!(n < 500_000, "unexpectedly huge grid");
                eprintln!("live Google READ Voucher_Raw_Data: {n} rows (cells not logged)");
            }
            Err(err) => {
                let message = err.to_string();
                if std::env::var("TBOOKS_LIVE_GOOGLE").ok().as_deref() == Some("1") {
                    panic!("{message}");
                }
                eprintln!("skip live Google READ: {message}");
            }
        }
    }
}
