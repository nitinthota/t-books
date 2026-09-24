//! Vendor master on this PC. One party, many bank accounts.
//! Primary bank still lives on vendors.bank / account_number / ifsc for the hive.

use rusqlite::params;

use crate::db::LocalBooks;
use crate::masters::VendorRow;
use crate::Result;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VendorAccount {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub bank: String,
    #[serde(default)]
    pub account_number: String,
    #[serde(default)]
    pub ifsc: String,
    #[serde(default)]
    pub is_primary: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VendorSave {
    pub vendor: String,
    #[serde(default)]
    pub gst: String,
    #[serde(default)]
    pub accounts: Vec<VendorAccount>,
}

fn ensure(books: &LocalBooks) -> Result<()> {
    books.conn().execute_batch(
        "CREATE TABLE IF NOT EXISTS vendor_accounts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            vendor TEXT NOT NULL COLLATE NOCASE,
            label TEXT,
            bank TEXT,
            account_number TEXT,
            ifsc TEXT,
            is_primary INTEGER NOT NULL DEFAULT 0
         );",
    )?;
    Ok(())
}

fn seed_from_vendor_row(books: &LocalBooks, row: &VendorRow) -> Result<()> {
    ensure(books)?;
    let count: i64 = books.conn().query_row(
        "SELECT COUNT(*) FROM vendor_accounts WHERE vendor = ?1",
        params![row.vendor],
        |r| r.get(0),
    )?;
    if count > 0 {
        return Ok(());
    }
    if row.bank.trim().is_empty() && row.account_number.trim().is_empty() && row.ifsc.trim().is_empty() {
        return Ok(());
    }
    books.conn().execute(
        "INSERT INTO vendor_accounts (vendor, label, bank, account_number, ifsc, is_primary)
         VALUES (?1, 'Primary', ?2, ?3, ?4, 1)",
        params![row.vendor, row.bank, row.account_number, row.ifsc],
    )?;
    Ok(())
}

fn load_accounts(books: &LocalBooks, vendor: &str) -> Result<Vec<VendorAccount>> {
    ensure(books)?;
    let mut stmt = books.conn().prepare(
        "SELECT id, COALESCE(label,''), COALESCE(bank,''), COALESCE(account_number,''), COALESCE(ifsc,''), is_primary
         FROM vendor_accounts WHERE vendor = ?1 ORDER BY is_primary DESC, id",
    )?;
    let rows = stmt.query_map(params![vendor], |row| {
        Ok(VendorAccount {
            id: row.get(0)?,
            label: row.get(1)?,
            bank: row.get(2)?,
            account_number: row.get(3)?,
            ifsc: row.get(4)?,
            is_primary: row.get::<_, i64>(5)? != 0,
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub fn list_accounts(books: &LocalBooks, vendor: &str) -> Result<Vec<VendorAccount>> {
    let vendor = vendor.trim();
    if vendor.is_empty() {
        return Ok(Vec::new());
    }
    let row = crate::masters::list_vendors(books)?
        .into_iter()
        .find(|v| v.vendor.eq_ignore_ascii_case(vendor));
    if let Some(row) = row {
        seed_from_vendor_row(books, &row)?;
    }
    load_accounts(books, vendor)
}

pub fn save_vendor(books: &LocalBooks, payload: VendorSave) -> Result<VendorRow> {
    ensure(books)?;
    let name = payload.vendor.trim();
    if name.is_empty() {
        return Err("Vendor name is required.".into());
    }
    let gst = payload.gst.trim().to_string();
    let mut accounts: Vec<VendorAccount> = payload
        .accounts
        .into_iter()
        .filter(|a| {
            !a.bank.trim().is_empty()
                || !a.account_number.trim().is_empty()
                || !a.ifsc.trim().is_empty()
                || !a.label.trim().is_empty()
        })
        .collect();
    if accounts.is_empty() {
        accounts.push(VendorAccount {
            label: "Primary".into(),
            ..VendorAccount::default()
        });
    }
    if !accounts.iter().any(|a| a.is_primary) {
        accounts[0].is_primary = true;
    }
    let primary = accounts
        .iter()
        .find(|a| a.is_primary)
        .cloned()
        .unwrap_or_else(|| accounts[0].clone());

    books.conn().execute(
        "INSERT INTO vendors (vendor, gst, bank, account_number, ifsc, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))
         ON CONFLICT(vendor) DO UPDATE SET
           gst = excluded.gst,
           bank = excluded.bank,
           account_number = excluded.account_number,
           ifsc = excluded.ifsc,
           updated_at = excluded.updated_at",
        params![name, gst, primary.bank, primary.account_number, primary.ifsc],
    )?;
    books.conn().execute("DELETE FROM vendor_accounts WHERE vendor = ?1", params![name])?;
    for (i, acc) in accounts.iter().enumerate() {
        let primary_flag = if acc.is_primary { 1 } else { 0 };
        let label = if acc.label.trim().is_empty() {
            if primary_flag == 1 {
                "Primary".to_string()
            } else {
                format!("Account {}", i + 1)
            }
        } else {
            acc.label.trim().to_string()
        };
        books.conn().execute(
            "INSERT INTO vendor_accounts (vendor, label, bank, account_number, ifsc, is_primary)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![name, label, acc.bank.trim(), acc.account_number.trim(), acc.ifsc.trim(), primary_flag],
        )?;
    }
    Ok(VendorRow {
        vendor: name.to_string(),
        gst,
        bank: primary.bank,
        account_number: primary.account_number,
        ifsc: primary.ifsc,
    })
}
