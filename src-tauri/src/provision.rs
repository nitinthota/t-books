//! Create missing hive tabs (headers only) and migrate from the read-only raw sheet.

use crate::hive::{EnsureTab, Hive, HIVE_KINDS};
use crate::hive_plan::{
    all_writable_targets, load_map, report_headers, target_for_kind, validate_plan, KIND_VOUCHER_RAW,
};
use crate::migrate::{apply_plan_to_hive, plan_from_raw_values, MigrationPlan};
use crate::office_sync::GoogleOfficeHive;
use crate::online::is_online;
use crate::sheets::{credentials_exist, fetch_values_on, load_service_account};
use crate::{BooksError, Result};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvisionReport {
    pub folders: Vec<String>,
    pub tabs: Vec<String>,
    pub error: Option<String>,
}

pub fn provision_hive(allow_create: bool) -> Result<ProvisionReport> {
    let folders = validate_plan()?;
    if !is_online() {
        return Ok(ProvisionReport {
            folders,
            tabs: Vec::new(),
            error: Some("This PC is offline. Cannot create Google tabs.".into()),
        });
    }
    if !credentials_exist() {
        return Ok(ProvisionReport {
            folders,
            tabs: Vec::new(),
            error: Some(
                "Google credentials not found at %LOCALAPPDATA%/T-Books/credentials.json.".into(),
            ),
        });
    }
    if !allow_create {
        return Ok(ProvisionReport {
            folders,
            tabs: all_writable_targets()?
                .into_iter()
                .map(|t| format!("{} / {}", t.workbook_title, t.tab))
                .collect(),
            error: None,
        });
    }
    let mut hive = GoogleOfficeHive::connect_bootstrap()?;
    let mut tabs = Vec::new();
    let mut kinds: Vec<String> = HIVE_KINDS.iter().map(|s| (*s).to_string()).collect();
    for extra in [
        "vendor",
        "project",
        "rules",
        "people",
        "purchase_item",
        "sales_item",
        "outstanding",
        "board",
        "credentials",
        "migration_log",
        "voucher_payment",
        "document",
    ] {
        kinds.push(extra.into());
    }
    for kind in kinds {
        if kind == KIND_VOUCHER_RAW {
            continue;
        }
        let headers = report_headers(&kind);
        match hive.ensure_tab(&kind, &headers) {
            Ok(EnsureTab::Exists) => {
                let target = target_for_kind(&kind)?;
                tabs.push(format!("{} already on {}", target.tab, target.workbook_title));
            }
            Ok(EnsureTab::BootstrappedHeaders) => {
                let target = target_for_kind(&kind)?;
                tabs.push(format!("{} created with headers on {}", target.tab, target.workbook_title));
            }
            Err(err) => tabs.push(format!("{kind}: {err}")),
        }
    }
    Ok(ProvisionReport {
        folders,
        tabs,
        error: None,
    })
}

pub fn migrate_from_raw(dry_run: bool) -> Result<MigrationPlan> {
    validate_plan()?;
    if !is_online() {
        return Err(BooksError::from(
            "This PC is offline. Cannot read Voucher_Raw_Data.",
        ));
    }
    if !credentials_exist() {
        return Err(BooksError::from(
            "Google credentials not found at %LOCALAPPDATA%/T-Books/credentials.json.",
        ));
    }
    let account = load_service_account()?;
    let map = load_map()?;
    let values = fetch_values_on(
        &account,
        &map.raw_source.spreadsheet_id,
        &map.raw_source.tab,
    )?;
    let run_id = format!("mig-{}", crate::hive_plan::raw_spreadsheet_id());
    let run_id: String = run_id.chars().rev().take(12).collect::<String>().chars().rev().collect();
    let plan = plan_from_raw_values(&values, &format!("mig-{run_id}"));
    if dry_run {
        return Ok(plan);
    }
    let mut hive = GoogleOfficeHive::connect_bootstrap()?;
    apply_plan_to_hive(&mut hive, &plan)?;
    Ok(plan)
}
