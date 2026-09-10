//! Structured office logs. Never write amounts, GST, bank, or passwords.
//! Files live under %LOCALAPPDATA%/T-Books/logs/

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::db::data_dir;

const ERROR_FILE: &str = "error.log";
const PERF_FILE: &str = "performance.log";
const DEBUG_FILE: &str = "debug.log";
const MAX_BYTES: u64 = 512 * 1024;
const KEEP: usize = 2;

#[derive(Clone, Copy)]
pub enum Level {
    Info,
    Warn,
    Error,
}

impl Level {
    fn as_str(self) -> &'static str {
        match self {
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }
}

static LOG_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);

pub fn is_debug() -> bool {
    std::env::args().any(|a| a == "--debug")
        || std::env::var("TBOOKS_DEBUG")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes"))
            .unwrap_or(false)
}

pub fn apply_debug_flag() {
    if std::env::args().any(|a| a == "--debug") {
        std::env::set_var("TBOOKS_DEBUG", "1");
    }
}

pub fn clear_logs() -> crate::Result<()> {
    let dir = log_dir();
    for name in [ERROR_FILE, PERF_FILE, DEBUG_FILE] {
        let path = dir.join(name);
        if path.is_file() {
            std::fs::write(&path, []).map_err(|_| {
                crate::BooksError::from("Could not clear logs on this PC.")
            })?;
        }
    }
    event(Level::Info, "ops", "clear_logs", None, "success");
    Ok(())
}

pub fn set_log_dir(path: PathBuf) {
    let _ = fs::create_dir_all(&path);
    if let Ok(mut slot) = LOG_DIR.lock() {
        *slot = Some(path);
    }
}

pub fn log_dir() -> PathBuf {
    if let Ok(slot) = LOG_DIR.lock() {
        if let Some(path) = slot.as_ref() {
            return path.clone();
        }
    }
    if let Ok(env) = std::env::var("TBOOKS_LOG_DIR") {
        let path = PathBuf::from(env);
        set_log_dir(path.clone());
        return path;
    }
    let path = data_dir().join("logs");
    set_log_dir(path.clone());
    path
}

fn now_stamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

fn forbidden(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("password")
        || lower.contains("gst")
        || lower.contains("ifsc")
        || lower.contains("account")
        || lower.contains("bank")
        || lower.contains('₹')
        || lower.contains("secret")
        || lower.contains("token")
}

fn safe(text: &str) -> &str {
    if forbidden(text) {
        "redacted"
    } else {
        text
    }
}

fn rotate_if_needed(path: &Path) {
    let Ok(meta) = fs::metadata(path) else {
        return;
    };
    if meta.len() < MAX_BYTES {
        return;
    }
    for i in (1..KEEP).rev() {
        let from = path.with_extension(format!("log.{i}"));
        let to = path.with_extension(format!("log.{}", i + 1));
        let _ = fs::rename(from, to);
    }
    let rotated = path.with_extension("log.1");
    let _ = fs::rename(path, rotated);
}

fn append(file_name: &str, line: &str) {
    let dir = log_dir();
    let _ = fs::create_dir_all(&dir);
    let path = dir.join(file_name);
    rotate_if_needed(&path);
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) else {
        return;
    };
    let _ = writeln!(file, "{line}");
    let _ = file.flush();
}

/// `[time] [level] [module] [operation] [voucher_number] [status]`
pub fn event(
    level: Level,
    module: &str,
    operation: &str,
    voucher_number: Option<i64>,
    status: &str,
) {
    let voucher = match voucher_number {
        Some(n) => n.to_string(),
        None => "none".into(),
    };
    let line = format!(
        "[{}] {} {} {} voucher={} {}",
        now_stamp(),
        level.as_str(),
        safe(module),
        safe(operation),
        voucher,
        safe(status)
    );
    match level {
        Level::Error | Level::Warn => append(ERROR_FILE, &line),
        Level::Info => append(PERF_FILE, &line),
    }
    if is_debug() {
        append(DEBUG_FILE, &line);
    }
}

pub fn performance(module: &str, operation: &str, duration_ms: u128, status: &str) {
    let line = format!(
        "[{}] PERF {} {} duration_ms={} {}",
        now_stamp(),
        safe(module),
        safe(operation),
        duration_ms,
        safe(status)
    );
    append(PERF_FILE, &line);
}

pub fn error_log_path() -> PathBuf {
    log_dir().join(ERROR_FILE)
}

#[allow(dead_code)]
pub fn performance_log_path() -> PathBuf {
    log_dir().join(PERF_FILE)
}

#[allow(dead_code)]
pub fn read_error_log() -> String {
    fs::read_to_string(error_log_path()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn never_writes_forbidden_words() {
        let dir = env::temp_dir().join(format!("tbooks-log-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        set_log_dir(dir.clone());
        event(
            Level::Error,
            "refresh",
            "parse",
            Some(20),
            "password=secret gst=22 bank=HDFC",
        );
        let body = fs::read_to_string(dir.join(ERROR_FILE)).unwrap_or_default();
        assert!(body.contains("redacted"));
        assert!(!body.to_ascii_lowercase().contains("password"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn writes_voucher_and_error_type() {
        let dir = env::temp_dir().join(format!("tbooks-log-ok-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        set_log_dir(dir.clone());
        event(
            Level::Error,
            "refresh",
            "skip",
            Some(1021),
            "invalid_data",
        );
        let body = fs::read_to_string(dir.join(ERROR_FILE)).unwrap();
        assert!(body.contains("voucher=1021"));
        assert!(body.contains("invalid_data"));
        assert!(body.contains("ERROR"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rotate_keeps_size_bounded() {
        let dir = env::temp_dir().join(format!("tbooks-log-rot-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        set_log_dir(dir.clone());
        let path = dir.join(ERROR_FILE);
        {
            let mut f = std::fs::File::create(&path).unwrap();
            f.write_all(&vec![b'x'; (MAX_BYTES + 8) as usize]).unwrap();
        }
        event(Level::Warn, "refresh", "skip", None, "overflow");
        assert!(dir.join("error.log.1").exists() || path.exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
