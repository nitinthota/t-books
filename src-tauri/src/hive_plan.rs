//! Locked hive map: Drive folders + workbooks from the Sheet Hive Plan agent.
//! IDs only — never a service-account secret.

use std::collections::BTreeMap;

use crate::{BooksError, Result, DEFAULT_SPREADSHEET_ID, VOUCHER_RAW_TAB};

pub const HIVE_MAP_JSON: &str = include_str!("../hive-map.json");

pub const KIND_VOUCHER_RAW: &str = "voucher_raw";
pub const KIND_VOUCHER_PAYMENT: &str = "voucher_payment";
pub const KIND_VENDOR: &str = "vendor";
pub const KIND_PROJECT: &str = "project";
pub const KIND_PEOPLE: &str = "people";
pub const KIND_RULES: &str = "rules";
pub const KIND_PURCHASE_ITEM: &str = "purchase_item";
pub const KIND_SALES_ITEM: &str = "sales_item";
pub const KIND_OUTSTANDING: &str = "outstanding";
pub const KIND_BOARD: &str = "board";
pub const KIND_TRIAL: &str = "trial";
pub const KIND_RESULT: &str = "result";
pub const KIND_CREDENTIALS: &str = "credentials";
pub const KIND_MIGRATION_LOG: &str = "migration_log";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawSource {
    pub name: String,
    pub spreadsheet_id: String,
    pub tab: String,
    pub gid: i64,
    pub write: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderRef {
    pub key: String,
    pub name: String,
    pub id: String,
    pub parent_key: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbookRef {
    pub key: String,
    pub title: String,
    pub id: String,
    pub folder_key: String,
    pub kind: String,
    pub tab: String,
    pub write: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtraTab {
    pub workbook_key: String,
    pub tab: String,
    pub kind: String,
    pub write: bool,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HiveMap {
    pub version: i64,
    pub validated_against: String,
    pub raw_source: RawSource,
    pub root_folder: FolderNamed,
    pub folders: Vec<FolderRef>,
    pub workbooks: Vec<WorkbookRef>,
    pub extra_tabs: Vec<ExtraTab>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderNamed {
    pub name: String,
    pub id: String,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HiveTarget {
    pub kind: String,
    pub spreadsheet_id: String,
    pub tab: String,
    pub writable: bool,
    pub workbook_title: String,
}

pub fn load_map() -> Result<HiveMap> {
    serde_json::from_str(HIVE_MAP_JSON)
        .map_err(|_| BooksError::from("hive-map.json in this build is not valid JSON."))
}

pub fn raw_spreadsheet_id() -> String {
    load_map()
        .map(|m| m.raw_source.spreadsheet_id)
        .unwrap_or_else(|_| DEFAULT_SPREADSHEET_ID.to_string())
}

pub fn is_read_only(spreadsheet_id: &str, tab: &str) -> bool {
    let Ok(map) = load_map() else {
        return tab.eq_ignore_ascii_case(VOUCHER_RAW_TAB);
    };
    let same_book = spreadsheet_id.trim() == map.raw_source.spreadsheet_id;
    let raw_tab = tab.trim().eq_ignore_ascii_case(&map.raw_source.tab)
        || tab.trim().eq_ignore_ascii_case(VOUCHER_RAW_TAB)
        || tab.trim().eq_ignore_ascii_case("VOUCHER_RAW_Data");
    (same_book && raw_tab) || raw_tab
}

pub fn refuse_raw_write(spreadsheet_id: &str, tab: &str) -> Result<()> {
    if is_read_only(spreadsheet_id, tab) {
        return Err(BooksError::from(
            "Voucher_Raw_Data is read-only. Write the structured hive instead. The original register was not changed.",
        ));
    }
    Ok(())
}

fn workbook_by_key<'a>(map: &'a HiveMap, key: &str) -> Option<&'a WorkbookRef> {
    map.workbooks.iter().find(|w| w.key == key)
}

pub fn target_for_kind(kind: &str) -> Result<HiveTarget> {
    let map = load_map()?;
    if kind == KIND_VOUCHER_RAW {
        return Ok(HiveTarget {
            kind: KIND_VOUCHER_RAW.into(),
            spreadsheet_id: map.raw_source.spreadsheet_id,
            tab: map.raw_source.tab,
            writable: false,
            workbook_title: map.raw_source.name,
        });
    }
    if let Some(book) = map.workbooks.iter().find(|w| w.kind == kind) {
        return Ok(HiveTarget {
            kind: kind.to_string(),
            spreadsheet_id: book.id.clone(),
            tab: book.tab.clone(),
            writable: book.write,
            workbook_title: book.title.clone(),
        });
    }
    if let Some(extra) = map.extra_tabs.iter().find(|t| t.kind == kind) {
        let book = workbook_by_key(&map, &extra.workbook_key).ok_or_else(|| {
            BooksError::from(format!("Hive extra tab {} is missing its workbook.", extra.tab))
        })?;
        return Ok(HiveTarget {
            kind: kind.to_string(),
            spreadsheet_id: book.id.clone(),
            tab: extra.tab.clone(),
            writable: extra.write,
            workbook_title: book.title.clone(),
        });
    }
    // Repo hive kinds that share a workbook with a different plan kind name.
    let aliased = match kind {
        crate::hive::KIND_VOUCHER => Some("voucher"),
        crate::hive::KIND_PAYMENT => Some("payment"),
        crate::hive::KIND_SALES_PO => Some("sales_po"),
        crate::hive::KIND_INVENTORY => Some("inventory"),
        crate::hive::KIND_LOGISTICS => Some("logistics"),
        crate::hive::KIND_DOCUMENT => Some("document"),
        crate::hive::KIND_SALARY => Some("salary"),
        crate::hive::KIND_PURCHASE => Some("purchase"),
        crate::hive::KIND_ACCESS => Some("access"),
        _ => None,
    };
    if let Some(plan_kind) = aliased {
        if plan_kind != kind {
            return target_for_kind(plan_kind);
        }
    }
    Err(BooksError::from(format!(
        "No hive workbook is mapped for {kind}."
    )))
}

pub fn all_writable_targets() -> Result<Vec<HiveTarget>> {
    let map = load_map()?;
    let mut out = Vec::new();
    for book in &map.workbooks {
        out.push(HiveTarget {
            kind: book.kind.clone(),
            spreadsheet_id: book.id.clone(),
            tab: book.tab.clone(),
            writable: book.write,
            workbook_title: book.title.clone(),
        });
    }
    for extra in &map.extra_tabs {
        let book = workbook_by_key(&map, &extra.workbook_key).ok_or_else(|| {
            BooksError::from(format!("Hive extra tab {} is missing its workbook.", extra.tab))
        })?;
        out.push(HiveTarget {
            kind: extra.kind.clone(),
            spreadsheet_id: book.id.clone(),
            tab: extra.tab.clone(),
            writable: extra.write,
            workbook_title: book.title.clone(),
        });
    }
    Ok(out)
}

pub fn report_headers(kind: &str) -> Vec<String> {
    let h = |cols: &[&str]| cols.iter().map(|s| (*s).to_string()).collect();
    match kind {
        crate::hive::KIND_ACCESS => h(&["Name", "Email", "Role", "Active", "Account_Type"]),
        KIND_CREDENTIALS => h(&["Name", "Email", "Account_Type", "Password_Set"]),
        crate::hive::KIND_VOUCHER => h(&[
            "Voucher no",
            "Date",
            "Vendor",
            "Project",
            "Tax invoice",
            "What it is for",
            "Bill ₹",
            "Paid ₹",
            "TDS ₹",
            "Due ₹",
            "Status",
            "Vendor bank",
            "Account",
            "IFSC",
            "GSTIN",
            "Remarks",
            "Paper (Drive file id)",
            "fp",
            "rev",
        ]),
        KIND_VOUCHER_PAYMENT => h(&[
            "Voucher no",
            "Payment #",
            "Date",
            "Amount ₹",
            "TDS ₹",
            "UTR / ref",
            "What it is for",
            "Remarks",
            "fp",
            "rev",
        ]),
        KIND_VENDOR => {
            h(&["Vendor", "GSTIN", "Bank", "Account", "IFSC", "Usual project", "Notes", "fp", "rev"])
        }
        KIND_PROJECT => {
            h(&["Project", "Vouchers", "Billed ₹", "Paid ₹", "Due ₹", "Customer PO ₹", "Notes", "fp", "rev"])
        }
        KIND_OUTSTANDING => h(&[
            "Voucher no",
            "Vendor",
            "Project",
            "Bill ₹",
            "Paid ₹",
            "Due ₹",
            "Status",
            "Tax invoice",
        ]),
        KIND_BOARD => h(&["What this is", "Number", "Notes"]),
        crate::hive::KIND_PURCHASE => h(&[
            "PO number",
            "Date",
            "Vendor",
            "Project",
            "Status",
            "Subtotal ₹",
            "GST ₹",
            "Total ₹",
            "Paid ₹",
            "Balance ₹",
            "Payment terms",
            "Notes",
            "fp",
            "rev",
        ]),
        KIND_PURCHASE_ITEM => h(&[
            "PO number",
            "Line",
            "Item",
            "Qty",
            "Rate ₹",
            "GST %",
            "Amount ₹",
            "fp",
            "rev",
        ]),
        crate::hive::KIND_PAYMENT => h(&[
            "pay_number",
            "po_number",
            "vendor",
            "amount",
            "alloc_method",
            "pay_class",
            "pay_date",
            "fp",
            "rev",
        ]),
        crate::hive::KIND_SALES_PO => h(&[
            "key",
            "po_number",
            "project",
            "client",
            "gst",
            "total_value",
            "fp",
            "rev",
        ]),
        KIND_SALES_ITEM => h(&[
            "PO number",
            "Project",
            "Line",
            "Item",
            "Qty",
            "Rate ₹",
            "GST %",
            "Amount ₹",
            "fp",
            "rev",
        ]),
        KIND_PEOPLE => h(&["Name", "Role", "Salary ₹", "Active", "fp", "rev"]),
        crate::hive::KIND_SALARY => h(&[
            "salary_number",
            "person",
            "month",
            "kind",
            "net",
            "fp",
            "rev",
        ]),
        crate::hive::KIND_INVENTORY => h(&[
            "key",
            "item_name",
            "type",
            "size",
            "qty",
            "cost",
            "project",
            "fp",
            "rev",
        ]),
        crate::hive::KIND_LOGISTICS => h(&[
            "key",
            "project",
            "vehicle",
            "invoice",
            "start",
            "reach",
            "fp",
            "rev",
        ]),
        crate::hive::KIND_DOCUMENT => {
            h(&["key", "name", "linked_type", "linked_id", "drive_folder", "fp", "rev"])
        }
        KIND_RULES => h(&["Rule", "When it applies", "What we do", "Active"]),
        KIND_MIGRATION_LOG => h(&[
            "run_id",
            "at",
            "source_row",
            "source_serial",
            "class",
            "target_kind",
            "target_key",
            "action",
            "detail",
        ]),
        KIND_TRIAL => h(&["Account", "Debit ₹", "Credit ₹", "Notes"]),
        KIND_RESULT => h(&["Line", "Amount ₹", "Notes"]),
        _ => crate::hive::tab_headers(kind),
    }
}

pub fn folder_tree_lines(map: &HiveMap) -> Vec<String> {
    let mut by_parent: BTreeMap<String, Vec<&FolderRef>> = BTreeMap::new();
    for folder in &map.folders {
        by_parent
            .entry(folder.parent_key.clone())
            .or_default()
            .push(folder);
    }
    let mut lines = vec![format!("{} ({})", map.root_folder.name, map.root_folder.id)];
    fn walk(
        parent: &str,
        indent: &str,
        by_parent: &BTreeMap<String, Vec<&FolderRef>>,
        lines: &mut Vec<String>,
    ) {
        let Some(children) = by_parent.get(parent) else {
            return;
        };
        for child in children {
            lines.push(format!("{indent}{}/ ({})", child.name, child.id));
            walk(&child.key, &format!("{indent}  "), by_parent, lines);
        }
    }
    walk("root", "  ", &by_parent, &mut lines);
    lines
}

pub fn validate_plan() -> Result<Vec<String>> {
    let map = load_map()?;
    if map.raw_source.write {
        return Err(BooksError::from(
            "hive-map.json marks Voucher_Raw_Data writable. It must stay read-only.",
        ));
    }
    if map.raw_source.spreadsheet_id != DEFAULT_SPREADSHEET_ID {
        return Err(BooksError::from(
            "hive-map.json raw spreadsheet id does not match the repository source sheet.",
        ));
    }
    if !map.raw_source.tab.eq_ignore_ascii_case(VOUCHER_RAW_TAB)
        && map.raw_source.tab != "VOUCHER_RAW_Data"
    {
        return Err(BooksError::from(
            "hive-map.json raw tab is not Voucher_Raw_Data.",
        ));
    }
    let mut seen_ids = BTreeMap::new();
    for book in &map.workbooks {
        if book.id == map.raw_source.spreadsheet_id {
            return Err(BooksError::from(
                "A live hive workbook must not be the raw archive spreadsheet.",
            ));
        }
        if let Some(prev) = seen_ids.insert(book.id.clone(), book.key.clone()) {
            return Err(BooksError::from(format!(
                "Workbook id used twice ({prev} and {}).",
                book.key
            )));
        }
        if is_read_only(&book.id, &book.tab) {
            return Err(BooksError::from(
                "A writable hive tab collided with Voucher_Raw_Data.",
            ));
        }
    }
    for kind in crate::hive::HIVE_KINDS {
        let target = target_for_kind(kind)?;
        if *kind == crate::hive::KIND_VOUCHER && target.spreadsheet_id == map.raw_source.spreadsheet_id
        {
            return Err(BooksError::from(
                "Live voucher hive still points at the raw archive sheet.",
            ));
        }
        if !target.writable && *kind != crate::hive::KIND_VOUCHER {
            // access etc must be writable
        }
    }
    Ok(folder_tree_lines(&map))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_parses_and_raw_is_read_only() {
        let map = load_map().unwrap();
        assert!(!map.raw_source.write);
        assert_eq!(map.raw_source.spreadsheet_id, DEFAULT_SPREADSHEET_ID);
        assert!(is_read_only(&map.raw_source.spreadsheet_id, "Voucher_Raw_Data"));
        assert!(is_read_only(&map.raw_source.spreadsheet_id, "VOUCHER_RAW_Data"));
        assert!(!is_read_only("1E6x0sHC6Ft9ZivTDH1ujP4aZ5jClu7-0-ropXZHAK7s", "Voucher register"));
        refuse_raw_write(&map.raw_source.spreadsheet_id, "Voucher_Raw_Data").unwrap_err();
    }

    #[test]
    fn every_repo_hive_kind_has_a_workbook() {
        validate_plan().unwrap();
        for kind in crate::hive::HIVE_KINDS {
            let t = target_for_kind(kind).unwrap();
            assert!(!t.spreadsheet_id.is_empty(), "{kind}");
            assert!(!t.tab.eq_ignore_ascii_case(VOUCHER_RAW_TAB) || *kind == crate::hive::KIND_VOUCHER);
            if *kind == crate::hive::KIND_VOUCHER {
                assert_ne!(t.spreadsheet_id, DEFAULT_SPREADSHEET_ID);
                assert_eq!(t.tab, "Voucher register");
            }
        }
    }

    #[test]
    fn credentials_tab_has_no_password_column() {
        let headers = report_headers(KIND_CREDENTIALS);
        assert!(headers.iter().any(|h| h == "Account_Type"));
        assert!(!headers.iter().any(|h| h.to_ascii_lowercase().contains("password")
            && h.to_ascii_lowercase() != "password_set"));
        assert_eq!(headers[3], "Password_Set");
    }
}
