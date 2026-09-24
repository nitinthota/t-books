//! Parent Post walks child rows after a good CAS.
//! Vendor / extra banks / PUR lines / SAL lines use child_cells.

use crate::core::business_rules::sha256_hex;
use crate::db::LocalBooks;
use crate::hive::{
    pending_payload_json, set_row_rev, submit_hive_row, tab_name, CasOutcome, EnsureTab, Hive,
};
use crate::hive_plan::report_headers;
use crate::office_children::{
    child_cells, is_child_kind, is_office_child_or_vendor, related_keys,
};
use crate::office_sync::{missing_tab_message, GoogleOfficeHive};
use crate::{BooksError, Result};

fn fingerprint_cells(cells: &[String]) -> String {
    sha256_hex(&cells.join("\n"))
}

fn headers_for(kind: &str) -> Vec<String> {
    if kind == "vendor_bank" {
        return ["Vendor", "Label", "Bank", "Account", "IFSC", "Primary", "fp", "rev"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
    }
    report_headers(kind)
}

fn submit_cells(
    books: &LocalBooks,
    hive: &mut dyn Hive,
    kind: &str,
    key: &str,
    allow_bootstrap: bool,
    cells: Vec<String>,
    base_fp: String,
    base_rev: i64,
) -> Result<CasOutcome> {
    let new_fp = fingerprint_cells(&cells);
    match hive.ensure_tab(kind, &headers_for(kind)) {
        Ok(EnsureTab::Exists) => {}
        Ok(EnsureTab::BootstrappedHeaders) => {
            if !allow_bootstrap {
                return Err(BooksError::from(missing_tab_message(kind)));
            }
            return Err(BooksError::from(format!(
                "{} tab was created with headers only. Submit again.",
                tab_name(kind)
            )));
        }
        Err(err) => return Err(err),
    }
    let payload = pending_payload_json(kind, key);
    let out = submit_hive_row(
        books, hive, kind, key, &base_fp, base_rev, &cells, &new_fp, &payload,
    )?;
    if let CasOutcome::Ok { ref fp, rev, .. } = out {
        if is_office_child_or_vendor(kind) {
            set_row_rev(books, kind, key, fp, rev)?;
        }
    }
    Ok(out)
}

fn submit_related(
    books: &LocalBooks,
    hive: &mut dyn Hive,
    kind: &str,
    key: &str,
    allow_bootstrap: bool,
) -> Result<()> {
    let mut first_err: Option<String> = None;
    for (child_kind, child_key) in related_keys(books, kind, key)? {
        match submit_one(books, hive, &child_kind, &child_key, allow_bootstrap) {
            Ok(CasOutcome::Ok { .. }) => {}
            Ok(CasOutcome::Conflict { message, .. }) => {
                if first_err.is_none() {
                    first_err = Some(message);
                }
            }
            Err(err) => {
                if first_err.is_none() {
                    first_err = Some(err.to_string());
                }
            }
        }
    }
    if let Some(message) = first_err {
        return Err(BooksError::from(message));
    }
    Ok(())
}

fn submit_one(
    books: &LocalBooks,
    hive: &mut dyn Hive,
    kind: &str,
    key: &str,
    allow_bootstrap: bool,
) -> Result<CasOutcome> {
    if is_office_child_or_vendor(kind) {
        let (cells, base_fp, base_rev) = child_cells(books, kind, key)?;
        let out = submit_cells(
            books,
            hive,
            kind,
            key,
            allow_bootstrap,
            cells,
            base_fp,
            base_rev,
        )?;
        if matches!(out, CasOutcome::Ok { .. }) && !is_child_kind(kind) {
            submit_related(books, hive, kind, key, allow_bootstrap)?;
        }
        return Ok(out);
    }
    let out = crate::office_sync::submit_office_with(books, hive, kind, key, allow_bootstrap)?;
    if matches!(out, CasOutcome::Ok { .. }) {
        submit_related(books, hive, kind, key, allow_bootstrap)?;
    }
    Ok(out)
}

pub fn submit_office(
    books: &LocalBooks,
    kind: &str,
    key: &str,
    allow_bootstrap: bool,
) -> Result<CasOutcome> {
    let mut hive = GoogleOfficeHive::connect()?;
    submit_one(books, &mut hive, kind, key, allow_bootstrap)
}
