//! Backup, restore, version, and update checks. No cloud reporting.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::params;
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::db::{data_dir, db_path, LocalBooks};
use crate::log::{self, log_dir};
use crate::online::is_online;
use crate::sheets::credentials_path;
use crate::{BooksError, Result, APP_VERSION, CREDENTIALS_FILE_NAME, DB_FILE_NAME};

pub const UPDATE_API: &str =
    "https://api.github.com/repos/nitinthota/t-books/releases/latest";
pub const AUTO_BACKUP_KEEP: usize = 5;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpsInfo {
    pub version: String,
    pub data_dir: String,
    pub db_path: String,
    pub auto_backup: bool,
    pub debug: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current: String,
    pub latest: String,
    pub newer: bool,
    pub notes: String,
    pub download_url: Option<String>,
}

fn stamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

pub fn backups_dir() -> PathBuf {
    let dir = data_dir().join("backups");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn parse_semver(raw: &str) -> Option<(u64, u64, u64)> {
    let s = raw.trim().trim_start_matches('v');
    let mut parts = s.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().ok()?;
    let patch = parts
        .next()
        .unwrap_or("0")
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .ok()?;
    Some((major, minor, patch))
}

pub fn is_newer(latest: &str, current: &str) -> bool {
    match (parse_semver(latest), parse_semver(current)) {
        (Some(l), Some(c)) => l > c,
        _ => false,
    }
}

pub fn store_app_version(books: &LocalBooks) -> Result<()> {
    books.conn().execute(
        "INSERT OR REPLACE INTO app_meta (key, value) VALUES ('version', ?1)",
        params![APP_VERSION],
    )?;
    books.conn().execute(
        "INSERT OR IGNORE INTO app_meta (key, value) VALUES ('auto_backup', 'yes')",
        [],
    )?;
    Ok(())
}

pub fn meta(books: &LocalBooks, key: &str) -> Option<String> {
    books
        .conn()
        .query_row(
            "SELECT value FROM app_meta WHERE key = ?1 LIMIT 1",
            params![key],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .filter(|s| !s.is_empty())
}

pub fn set_auto_backup(books: &LocalBooks, enabled: bool) -> Result<()> {
    books.conn().execute(
        "INSERT OR REPLACE INTO app_meta (key, value) VALUES ('auto_backup', ?1)",
        params![if enabled { "yes" } else { "no" }],
    )?;
    Ok(())
}

pub fn auto_backup_enabled(books: &LocalBooks) -> bool {
    match meta(books, "auto_backup") {
        Some(v) => !matches!(v.to_ascii_lowercase().as_str(), "no" | "0" | "false"),
        None => true,
    }
}

pub fn ops_info(books: &LocalBooks) -> OpsInfo {
    OpsInfo {
        version: meta(books, "version").unwrap_or_else(|| APP_VERSION.to_string()),
        data_dir: data_dir().display().to_string(),
        db_path: db_path().display().to_string(),
        auto_backup: auto_backup_enabled(books),
        debug: log::is_debug(),
    }
}

fn checkpoint(books: &LocalBooks) -> Result<()> {
    books
        .conn()
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
    Ok(())
}

fn add_file_bytes(zip: &mut ZipWriter<File>, name: &str, bytes: &[u8]) -> Result<()> {
    let options = FileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);
    zip.start_file(name, options)
        .map_err(|e| BooksError::from(format!("Could not write backup entry: {e}")))?;
    zip.write_all(bytes)
        .map_err(|_| BooksError::from("Could not write the backup file on this PC."))?;
    Ok(())
}

fn read_if_exists(path: &Path) -> Result<Option<Vec<u8>>> {
    if !path.is_file() {
        return Ok(None);
    }
    Ok(Some(fs::read(path).map_err(|_| {
        BooksError::from("Could not read a file for backup on this PC.")
    })?))
}

pub fn write_backup_zip(books: &LocalBooks, dest: &Path, include_logs: bool) -> Result<PathBuf> {
    checkpoint(books)?;
    if let Some(parent) = dest.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let file = File::create(dest)
        .map_err(|_| BooksError::from("Could not create the backup file on this PC."))?;
    let mut zip = ZipWriter::new(file);
    let db_file = books.path();
    if db_file.as_os_str() == ":memory:" || !db_file.is_file() {
        return Err(BooksError::from(
            "There is no local books file on this PC to back up.",
        ));
    }
    let db_bytes = fs::read(db_file)
        .map_err(|_| BooksError::from("Could not read the local books for backup."))?;
    add_file_bytes(&mut zip, DB_FILE_NAME, &db_bytes)?;
    if let Some(wal) = read_if_exists(&PathBuf::from(format!("{}-wal", db_file.display())))? {
        add_file_bytes(&mut zip, &format!("{DB_FILE_NAME}-wal"), &wal)?;
    }
    if let Some(shm) = read_if_exists(&PathBuf::from(format!("{}-shm", db_file.display())))? {
        add_file_bytes(&mut zip, &format!("{DB_FILE_NAME}-shm"), &shm)?;
    }
    if let Some(cred) = read_if_exists(&credentials_path())? {
        add_file_bytes(&mut zip, CREDENTIALS_FILE_NAME, &cred)?;
    }
    if include_logs {
        for name in ["error.log", "performance.log", "debug.log"] {
            if let Some(bytes) = read_if_exists(&log_dir().join(name))? {
                add_file_bytes(&mut zip, &format!("logs/{name}"), &bytes)?;
            }
        }
    }
    let manifest = format!(
        "{{\n  \"app\": \"T Books\",\n  \"version\": \"{}\",\n  \"created\": \"{}\"\n}}\n",
        APP_VERSION,
        stamp()
    );
    add_file_bytes(&mut zip, "manifest.json", manifest.as_bytes())?;
    zip.finish()
        .map_err(|_| BooksError::from("Could not finish the backup zip on this PC."))?;
    Ok(dest.to_path_buf())
}

#[allow(dead_code)]
pub fn default_backup_name() -> String {
    format!("T-Books-backup-{}.zip", stamp())
}

pub fn backup_to(books: &LocalBooks, dest: &Path) -> Result<PathBuf> {
    crate::log::event(
        crate::log::Level::Info,
        "ops",
        "backup",
        None,
        "start",
    );
    let path = write_backup_zip(books, dest, true)?;
    crate::log::event(crate::log::Level::Info, "ops", "backup", None, "success");
    Ok(path)
}

pub fn prune_auto_backups() -> Result<()> {
    let dir = backups_dir();
    let mut zips: Vec<_> = fs::read_dir(&dir)
        .map_err(|_| BooksError::from("Could not read the backups folder on this PC."))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().and_then(|s| s.to_str()) == Some("zip")
                && p.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .starts_with("T-Books-auto-")
        })
        .collect();
    zips.sort();
    while zips.len() > AUTO_BACKUP_KEEP {
        let old = zips.remove(0);
        let _ = fs::remove_file(old);
    }
    Ok(())
}

pub fn auto_backup(books: &LocalBooks) -> Result<Option<PathBuf>> {
    if !auto_backup_enabled(books) {
        return Ok(None);
    }
    let dest = backups_dir().join(format!("T-Books-auto-{}.zip", stamp()));
    let path = write_backup_zip(books, &dest, false)?;
    prune_auto_backups()?;
    Ok(Some(path))
}

fn zip_has_db(path: &Path) -> Result<bool> {
    let file = File::open(path)
        .map_err(|_| BooksError::from("Could not open that backup file."))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|_| BooksError::from("That file is not a T Books backup zip."))?;
    for i in 0..archive.len() {
        let entry = archive
            .by_index(i)
            .map_err(|_| BooksError::from("That backup zip could not be read."))?;
        let name = entry.name().replace('\\', "/");
        if name == DB_FILE_NAME || name.ends_with(&format!("/{DB_FILE_NAME}")) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn extract_named(archive: &mut ZipArchive<File>, want: &str) -> Result<Option<Vec<u8>>> {
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|_| BooksError::from("That backup zip could not be read."))?;
        let name = entry.name().replace('\\', "/");
        let file_name = name.rsplit('/').next().unwrap_or(&name);
        if file_name == want {
            let mut buf = Vec::new();
            entry
                .read_to_end(&mut buf)
                .map_err(|_| BooksError::from("Could not read a file from the backup zip."))?;
            return Ok(Some(buf));
        }
    }
    Ok(None)
}

pub fn restore_from(books: &mut LocalBooks, zip_path: &Path) -> Result<()> {
    if !zip_has_db(zip_path)? {
        return Err(BooksError::from(
            "That zip does not contain tbooks.db. Local books were not changed.",
        ));
    }
    let dest = books.path().to_path_buf();
    if dest.as_os_str() == ":memory:" || dest.as_os_str().is_empty() {
        return Err(BooksError::from(
            "Restore needs a books file on this PC, not a memory copy.",
        ));
    }
    let safety = dest
        .parent()
        .unwrap_or(Path::new("."))
        .join(format!("T-Books-pre-restore-{}.zip", stamp()));
    write_backup_zip(books, &safety, false)?;
    checkpoint(books)?;

    let file = File::open(zip_path)
        .map_err(|_| BooksError::from("Could not open that backup file."))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|_| BooksError::from("That file is not a T Books backup zip."))?;
    let db_bytes = extract_named(&mut archive, DB_FILE_NAME)?
        .ok_or_else(|| BooksError::from("That zip does not contain tbooks.db."))?;
    let cred = extract_named(&mut archive, CREDENTIALS_FILE_NAME)?;

    let placeholder = crate::db::open_memory()?;
    let old = std::mem::replace(books, placeholder);
    drop(old);

    fs::write(&dest, &db_bytes)
        .map_err(|_| BooksError::from("Could not replace the local books file."))?;
    let _ = fs::remove_file(format!("{}-wal", dest.display()));
    let _ = fs::remove_file(format!("{}-shm", dest.display()));
    if dest == db_path() {
        if let Some(bytes) = cred {
            let _ = fs::write(credentials_path(), bytes);
        }
    }

    *books = crate::db::open_at(&dest)?;
    store_app_version(books)?;
    crate::log::event(crate::log::Level::Info, "ops", "restore", None, "success");
    Ok(())
}

pub fn export_logs_zip(dest: &Path) -> Result<PathBuf> {
    if let Some(parent) = dest.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let file = File::create(dest)
        .map_err(|_| BooksError::from("Could not create the logs zip on this PC."))?;
    let mut zip = ZipWriter::new(file);
    let mut any = false;
    for name in ["error.log", "performance.log", "debug.log"] {
        if let Some(bytes) = read_if_exists(&log_dir().join(name))? {
            add_file_bytes(&mut zip, name, &bytes)?;
            any = true;
        }
    }
    if !any {
        add_file_bytes(&mut zip, "empty.txt", b"No T Books logs on this PC yet.\n")?;
    }
    zip.finish()
        .map_err(|_| BooksError::from("Could not finish the logs zip on this PC."))?;
    crate::log::event(crate::log::Level::Info, "ops", "export_logs", None, "success");
    Ok(dest.to_path_buf())
}

pub fn parse_latest_release(body: &str, current: &str) -> Result<UpdateInfo> {
    let value: serde_json::Value = serde_json::from_str(body)
        .map_err(|_| BooksError::from("The update listing was not valid JSON."))?;
    if let Some(msg) = value.get("message").and_then(|v| v.as_str()) {
        if msg.to_ascii_lowercase().contains("not found") {
            return Err(BooksError::from(
                "No published T Books release was found. Install from T-Books-Setup.exe.",
            ));
        }
    }
    let latest = value
        .get("tag_name")
        .or_else(|| value.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if latest.is_empty() {
        return Err(BooksError::from("The update listing has no version tag."));
    }
    let notes = value
        .get("body")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .chars()
        .take(400)
        .collect::<String>();
    let download_url = value
        .get("assets")
        .and_then(|v| v.as_array())
        .and_then(|assets| {
            assets.iter().find_map(|a| {
                let name = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
                if name.to_ascii_lowercase().contains("setup")
                    && name.to_ascii_lowercase().ends_with(".exe")
                {
                    a.get("browser_download_url")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                } else {
                    None
                }
            })
        })
        .or_else(|| {
            value
                .get("html_url")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        });
    Ok(UpdateInfo {
        current: current.to_string(),
        latest: latest.clone(),
        newer: is_newer(&latest, current),
        notes,
        download_url,
    })
}

pub fn check_updates(current: &str) -> Result<UpdateInfo> {
    if !is_online() {
        return Err(BooksError::from(
            "This PC is offline. Cannot check for updates.",
        ));
    }
    let response = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(12))
        .user_agent("T-Books/1.0")
        .build()
        .get(UPDATE_API)
        .set("Accept", "application/vnd.github+json")
        .call();
    let body = match response {
        Ok(resp) => resp
            .into_string()
            .map_err(|_| BooksError::from("The update listing was unreadable."))?,
        Err(ureq::Error::Status(code, resp)) => {
            let text = resp.into_string().unwrap_or_default();
            if code == 404 {
                return Err(BooksError::from(
                    "No published T Books release was found. Install from T-Books-Setup.exe.",
                ));
            }
            return Err(BooksError::from(if text.is_empty() {
                format!("Update check failed (HTTP {code}).")
            } else {
                "Update check failed.".into()
            }));
        }
        Err(_) => {
            return Err(BooksError::from(
                "Could not reach the update listing from this PC.",
            ));
        }
    };
    parse_latest_release(&body, current)
}

pub fn download_installer(url: &str, dest: &Path) -> Result<PathBuf> {
    if !is_online() {
        return Err(BooksError::from(
            "This PC is offline. Cannot download the installer.",
        ));
    }
    if let Some(parent) = dest.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let response = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(60))
        .user_agent("T-Books/1.0")
        .build()
        .get(url)
        .call()
        .map_err(|_| BooksError::from("Could not download T-Books-Setup.exe."))?;
    let mut reader = response.into_reader();
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|_| BooksError::from("The installer download was incomplete."))?;
    if bytes.len() < 64 {
        return Err(BooksError::from("The installer download was empty."));
    }
    fs::write(dest, bytes)
        .map_err(|_| BooksError::from("Could not save T-Books-Setup.exe on this PC."))?;
    crate::log::event(
        crate::log::Level::Info,
        "ops",
        "download_update",
        None,
        "success",
    );
    Ok(dest.to_path_buf())
}

#[cfg(feature = "desktop")]
pub fn pick_save(default_name: &str) -> Result<PathBuf> {
    rfd::FileDialog::new()
        .set_file_name(default_name)
        .add_filter("Zip", &["zip"])
        .save_file()
        .ok_or_else(|| BooksError::from("Backup cancelled."))
}

#[cfg(feature = "desktop")]
pub fn pick_open_zip() -> Result<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Zip", &["zip"])
        .pick_file()
        .ok_or_else(|| BooksError::from("Restore cancelled."))
}

#[cfg(feature = "desktop")]
pub fn pick_save_exe(default_name: &str) -> Result<PathBuf> {
    rfd::FileDialog::new()
        .set_file_name(default_name)
        .add_filter("Installer", &["exe"])
        .save_file()
        .ok_or_else(|| BooksError::from("Download cancelled."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testdata::{parse_clean_one_block, temp_db};
    use crate::{apply_voucher_rows, open_at};

    #[test]
    fn stores_semver() {
        let path = temp_db("ops-ver");
        let books = open_at(&path).unwrap();
        assert_eq!(meta(&books, "version").as_deref(), Some("1.0.0"));
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn semver_detects_newer() {
        assert!(is_newer("1.1.0", "1.0.0"));
        assert!(is_newer("v1.0.1", "1.0.0"));
        assert!(!is_newer("1.0.0", "1.0.0"));
        assert!(!is_newer("1.0.0", "1.1.0"));
    }

    #[test]
    fn parse_release_json() {
        let body = r#"{
          "tag_name": "v1.1.0",
          "body": "fixes",
          "html_url": "https://github.com/nitinthota/t-books/releases/tag/v1.1.0",
          "assets": [{"name": "T-Books-Setup.exe", "browser_download_url": "https://example.invalid/T-Books-Setup.exe"}]
        }"#;
        let info = parse_latest_release(body, "1.0.0").unwrap();
        assert!(info.newer);
        assert_eq!(info.latest, "v1.1.0");
        assert!(info.download_url.unwrap().contains("T-Books-Setup.exe"));
    }

    #[test]
    fn backup_restore_roundtrip() {
        let path = temp_db("ops-bak");
        let mut books = open_at(&path).unwrap();
        store_app_version(&books).unwrap();
        apply_voucher_rows(&mut books, &parse_clean_one_block(3)).unwrap();
        let zip = path.parent().unwrap().join(default_backup_name());
        backup_to(&books, &zip).unwrap();
        books
            .conn()
            .execute_batch("DELETE FROM voucher_payments; DELETE FROM vouchers;")
            .unwrap();
        assert_eq!(
            books
                .conn()
                .query_row("SELECT COUNT(*) FROM vouchers", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            0
        );
        restore_from(&mut books, &zip).unwrap();
        let n: i64 = books
            .conn()
            .query_row("SELECT COUNT(*) FROM vouchers", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 3);
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn bad_zip_does_not_wipe() {
        let path = temp_db("ops-bad");
        let mut books = open_at(&path).unwrap();
        apply_voucher_rows(&mut books, &parse_clean_one_block(2)).unwrap();
        let zip = path.parent().unwrap().join("nope.zip");
        fs::write(&zip, b"not a zip").unwrap();
        assert!(restore_from(&mut books, &zip).is_err());
        let n: i64 = books
            .conn()
            .query_row("SELECT COUNT(*) FROM vouchers", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 2);
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn prune_keeps_five() {
        let dir = backups_dir();
        for i in 0..7 {
            let p = dir.join(format!("T-Books-auto-{i:03}.zip"));
            fs::write(&p, b"pk").unwrap();
        }
        prune_auto_backups().unwrap();
        let left: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .starts_with("T-Books-auto-")
            })
            .collect();
        assert!(left.len() <= AUTO_BACKUP_KEEP);
    }
}
