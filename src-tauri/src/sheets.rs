use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};

use crate::db::data_dir;
use crate::{
    ACCESS_TAB, BooksError, CREDENTIALS_FILE_NAME, DEFAULT_SPREADSHEET_ID, Result,
};

const TOKEN_URI_DEFAULT: &str = "https://oauth2.googleapis.com/token";
const SHEETS_SCOPE: &str = "https://www.googleapis.com/auth/spreadsheets";
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

pub fn credentials_exist() -> bool {
    credentials_path().is_file()
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

pub fn load_service_account() -> Result<ServiceAccount> {
    if !credentials_exist() {
        return Err(BooksError::from(
            "Google credentials not found at %LOCALAPPDATA%/T-Books/credentials.json.",
        ));
    }
    let raw = std::fs::read_to_string(credentials_path())
        .map_err(|_| BooksError::from("Could not read credentials.json on this PC."))?;
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
        scope: SHEETS_SCOPE.to_string(),
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
                && cached.scope == SHEETS_SCOPE
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
            scope: SHEETS_SCOPE.to_string(),
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

pub fn fetch_sheet_values(account: &ServiceAccount, tab: &str) -> Result<Vec<Vec<String>>> {
    let token = access_token(account)?;
    let url = format!(
        "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}?majorDimension=ROWS&valueRenderOption=FORMATTED_VALUE&dateTimeRenderOption=FORMATTED_STRING",
        account.spreadsheet_id,
        encode_range(tab)
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

pub fn fetch_access_values(account: &ServiceAccount) -> Result<Vec<Vec<String>>> {
    fetch_sheet_values(account, ACCESS_TAB)
}

pub fn fetch_sheet_a1(account: &ServiceAccount, tab: &str, a1: &str) -> Result<Vec<Vec<String>>> {
    let token = access_token(account)?;
    let url = format!(
        "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}?majorDimension=ROWS&valueRenderOption=FORMATTED_VALUE&dateTimeRenderOption=FORMATTED_STRING",
        account.spreadsheet_id,
        encode_a1(tab, Some(a1))
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

fn values_url(account: &ServiceAccount, tab: &str, a1: &str) -> String {
    format!(
        "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}",
        account.spreadsheet_id,
        encode_a1(tab, Some(a1))
    )
}

pub fn update_sheet_row(
    account: &ServiceAccount,
    tab: &str,
    a1: &str,
    cells: &[String],
) -> Result<()> {
    let token = access_token(account)?;
    let url = format!("{}?valueInputOption=RAW", values_url(account, tab, a1));
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

pub fn append_sheet_row(account: &ServiceAccount, tab: &str, cells: &[String]) -> Result<()> {
    let token = access_token(account)?;
    let url = format!(
        "{}:append?valueInputOption=RAW&insertDataOption=INSERT_ROWS",
        values_url(account, tab, "A:BL")
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
}
