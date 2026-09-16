//! Read-only transform of Voucher_Raw_Data into structured hive rows.
//! Never writes the source tab. Dummy tokens stay out of the books.

use std::collections::{BTreeMap, BTreeSet};

use crate::core::business_rules::{
    is_dummy_serial, is_dotted_child_serial, should_skip_sheet_row, summarize_voucher,
    PaymentBlock, VoucherInput, MAX_PAYMENT_BLOCKS,
};
use crate::hive_plan::{
    report_headers, KIND_BOARD, KIND_MIGRATION_LOG, KIND_OUTSTANDING, KIND_PROJECT, KIND_VENDOR,
    KIND_VOUCHER_PAYMENT,
};
use crate::vouchers::{parse_voucher_values, ParsedVoucher};
use crate::hive::{KIND_VOUCHER, Hive};

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MigrationLogRow {
    pub run_id: String,
    pub at: String,
    pub source_row: i32,
    pub source_serial: String,
    pub class: String,
    pub target_kind: String,
    pub target_key: String,
    pub action: String,
    pub detail: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuredVoucher {
    pub key: String,
    pub cells: Vec<String>,
    pub class: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuredPayment {
    pub key: String,
    pub cells: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationPlan {
    pub run_id: String,
    pub vouchers: Vec<StructuredVoucher>,
    pub payments: Vec<StructuredPayment>,
    pub vendors: Vec<Vec<String>>,
    pub jobs: Vec<Vec<String>>,
    pub outstanding: Vec<Vec<String>>,
    pub board: Vec<Vec<String>>,
    pub log: Vec<MigrationLogRow>,
    pub imported: u32,
    pub skipped: u32,
    pub payments_out: u32,
}

fn money(n: f64) -> String {
    format!("{n:.2}")
}

pub fn slot_occupied(slot: &PaymentBlock) -> bool {
    slot.pi_value_rupees.unwrap_or(0.0) != 0.0
        || slot.paid_rupees.unwrap_or(0.0) != 0.0
        || slot.tds_rupees.unwrap_or(0.0) != 0.0
        || !slot.payment_date.trim().is_empty()
        || !slot.payment_details.trim().is_empty()
        || !slot.pi_no.trim().is_empty()
}

fn classify_voucher(v: &ParsedVoucher) -> &'static str {
    if !v.project.trim().is_empty() {
        "project_expense"
    } else {
        "purchase_bill"
    }
}

/// Voucher register cells (no fp/rev — hive CAS appends those).
pub fn register_cells_from_parsed(
    v: &ParsedVoucher,
    totals: &crate::core::business_rules::VoucherComputed,
    purpose: &str,
    remarks: &str,
) -> Vec<String> {
    let tds: f64 = v.payments.iter().map(|p| p.tds_rupees.unwrap_or(0.0)).sum();
    vec![
        v.voucher_number.to_string(),
        v.voucher_date.clone(),
        v.vendor.clone(),
        v.project.clone(),
        v.tax_invoice.clone(),
        purpose.to_string(),
        money(totals.total_value),
        money(totals.total_paid),
        money(tds),
        money(totals.remaining),
        totals.status.clone(),
        v.bank.clone(),
        v.account_number.clone(),
        v.ifsc.clone(),
        v.gst.clone(),
        remarks.to_string(),
        String::new(),
    ]
}

pub fn register_cells_for_voucher(v: &ParsedVoucher) -> Vec<String> {
    let input = VoucherInput {
        voucher_number: v.voucher_number,
        tax_invoice: v.tax_invoice.clone(),
        vendor: v.vendor.clone(),
        bank: v.bank.clone(),
        account_number: v.account_number.clone(),
        ifsc: v.ifsc.clone(),
        gst: v.gst.clone(),
        project: v.project.clone(),
        comments: String::new(),
        payments: v.payments.clone(),
    };
    let totals = summarize_voucher(&input);
    let purpose: String = v
        .payments
        .iter()
        .map(|p| p.description.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" · ");
    let remarks: String = v
        .payments
        .iter()
        .map(|p| p.remarks.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" · ");
    register_cells_from_parsed(v, &totals, &purpose, &remarks)
}

/// Payment lines on the same voucher. Key `{voucher}#{n}`. Not a second bill.
pub fn payment_lines_for_voucher(v: &ParsedVoucher) -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    let mut pay_no = 0i32;
    for slot in &v.payments {
        if !slot_occupied(slot) {
            continue;
        }
        pay_no += 1;
        let key = format!("{}#{}", v.voucher_number, pay_no);
        out.push((
            key,
            vec![
                v.voucher_number.to_string(),
                pay_no.to_string(),
                slot.payment_date.clone(),
                money(slot.paid_rupees.unwrap_or(0.0)),
                money(slot.tds_rupees.unwrap_or(0.0)),
                slot.payment_details.clone(),
                slot.description.clone(),
                slot.remarks.clone(),
            ],
        ));
    }
    out
}

fn log_row(
    run_id: &str,
    at: &str,
    source_row: i32,
    source_serial: &str,
    class: &str,
    target_kind: &str,
    target_key: &str,
    action: &str,
    detail: &str,
) -> MigrationLogRow {
    MigrationLogRow {
        run_id: run_id.into(),
        at: at.into(),
        source_row,
        source_serial: source_serial.into(),
        class: class.into(),
        target_kind: target_kind.into(),
        target_key: target_key.into(),
        action: action.into(),
        detail: detail.into(),
    }
}

/// Pure transform. Source grid is never mutated.
pub fn plan_from_raw_values(values: &[Vec<String>], run_id: &str) -> MigrationPlan {
    let at = "now";
    let parsed = parse_voucher_values(values);
    let mut log = Vec::new();
    let mut seen_serials: BTreeSet<i64> = BTreeSet::new();

    for (i, cols) in values.iter().enumerate() {
        let row_no = (i + 1) as i32;
        if cols.iter().all(|c| c.trim().is_empty()) {
            continue;
        }
        let serial = cols.first().map(|s| s.as_str()).unwrap_or("");
        let vendor = cols.get(3).map(|s| s.as_str()).unwrap_or("");
        if should_skip_sheet_row(serial, vendor) {
            let class = if is_dummy_serial(serial) {
                "skipped_dummy"
            } else if is_dotted_child_serial(serial) {
                "skipped_dotted"
            } else if vendor.trim().is_empty() {
                "skipped_empty_vendor"
            } else {
                "skipped"
            };
            log.push(log_row(
                run_id,
                at,
                row_no,
                serial,
                class,
                "",
                "",
                "skip",
                "Repository skip rules. Source row was not copied.",
            ));
        }
    }

    let mut vouchers = Vec::new();
    let mut payments = Vec::new();
    let mut vendor_map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut job_map: BTreeMap<String, (u32, f64, f64, f64)> = BTreeMap::new();
    let mut outstanding = Vec::new();
    let mut full = 0u32;
    let mut partial = 0u32;
    let mut pending = 0u32;
    let mut advance = 0u32;
    let mut missing = 0u32;

    for v in &parsed.vouchers {
        if !seen_serials.insert(v.voucher_number) {
            log.push(log_row(
                run_id,
                at,
                0,
                &v.voucher_number.to_string(),
                "duplicate_serial",
                KIND_VOUCHER,
                &v.voucher_number.to_string(),
                "keep_last",
                "Later source row wins. One integer voucher number.",
            ));
        }
        let input = VoucherInput {
            voucher_number: v.voucher_number,
            tax_invoice: v.tax_invoice.clone(),
            vendor: v.vendor.clone(),
            bank: v.bank.clone(),
            account_number: v.account_number.clone(),
            ifsc: v.ifsc.clone(),
            gst: v.gst.clone(),
            project: v.project.clone(),
            comments: String::new(),
            payments: v.payments.clone(),
        };
        let totals = summarize_voucher(&input);
        let class = classify_voucher(v);
        let purpose: String = v
            .payments
            .iter()
            .map(|p| p.description.trim())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" · ");
        let remarks: String = v
            .payments
            .iter()
            .map(|p| p.remarks.trim())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" · ");
        let key = v.voucher_number.to_string();
        let cells = register_cells_from_parsed(v, &totals, &purpose, &remarks);
        vouchers.push(StructuredVoucher {
            key: key.clone(),
            cells,
            class: class.into(),
        });
        log.push(log_row(
            run_id,
            at,
            0,
            &key,
            class,
            KIND_VOUCHER,
            &key,
            "map",
            "Integer voucher → Voucher register. Payments stay linked, not new bills.",
        ));

        let vendor_key = v.vendor.trim().to_ascii_lowercase();
        vendor_map.entry(vendor_key).or_insert_with(|| {
            vec![
                v.vendor.clone(),
                v.gst.clone(),
                v.bank.clone(),
                v.account_number.clone(),
                v.ifsc.clone(),
                v.project.clone(),
                String::new(),
            ]
        });

        if !v.project.trim().is_empty() {
            let job = job_map.entry(v.project.trim().to_string()).or_insert((0, 0.0, 0.0, 0.0));
            job.0 += 1;
            job.1 += totals.total_value;
            job.2 += totals.total_paid;
            job.3 += totals.remaining;
        }

        let sl = totals.status.to_ascii_lowercase();
        if sl.contains("missing") {
            missing += 1;
        } else if sl.contains("pending") {
            pending += 1;
        } else if sl.contains("partial") {
            partial += 1;
        } else if sl.contains("advance") {
            advance += 1;
        } else if sl.contains("full") {
            full += 1;
        }
        if totals.remaining.abs() > 0.004 {
            outstanding.push(vec![
                key.clone(),
                v.vendor.clone(),
                v.project.clone(),
                money(totals.total_value),
                money(totals.total_paid),
                money(totals.remaining),
                totals.status.clone(),
                v.tax_invoice.clone(),
            ]);
        }

        for (pay_key, cells) in payment_lines_for_voucher(v) {
            payments.push(StructuredPayment {
                key: pay_key.clone(),
                cells,
            });
            log.push(log_row(
                run_id,
                at,
                0,
                &v.voucher_number.to_string(),
                "payment_line",
                KIND_VOUCHER_PAYMENT,
                &pay_key,
                "map",
                "Payment on the same voucher. Not a second bill.",
            ));
        }
        let _ = MAX_PAYMENT_BLOCKS;
    }

    for err in &parsed.errors {
        log.push(log_row(
            run_id,
            at,
            err.row,
            &err.voucher_number.map(|n| n.to_string()).unwrap_or_default(),
            &err.error_type,
            "",
            "",
            "error",
            "Parse skip recorded. Source not changed.",
        ));
    }

    let vendors: Vec<Vec<String>> = vendor_map.into_values().collect();
    let jobs: Vec<Vec<String>> = job_map
        .into_iter()
        .map(|(project, (count, billed, paid, due))| {
            vec![
                project,
                count.to_string(),
                money(billed),
                money(paid),
                money(due),
                money(0.0),
                String::new(),
            ]
        })
        .collect();

    let board = vec![
        vec!["Vouchers".into(), vouchers.len().to_string(), "Historical register from raw".into()],
        vec!["Paid in full".into(), full.to_string(), String::new()],
        vec!["Part paid".into(), partial.to_string(), String::new()],
        vec!["Pending".into(), pending.to_string(), String::new()],
        vec!["Advance".into(), advance.to_string(), String::new()],
        vec!["Missing tax invoice".into(), missing.to_string(), String::new()],
        vec!["Payment lines".into(), payments.len().to_string(), String::new()],
        vec!["Vendors".into(), vendors.len().to_string(), String::new()],
        vec!["Jobs".into(), jobs.len().to_string(), String::new()],
        vec![
            "Sales / HR / stock / trips from raw".into(),
            "0".into(),
            "No source in Voucher_Raw_Data. Headers only.".into(),
        ],
    ];

    log.push(log_row(
        run_id,
        at,
        0,
        "",
        "unmapped_module",
        "sales_po",
        "",
        "skip",
        "Raw sheet has no sales PO rows. Sales workbooks stay header-only.",
    ));
    log.push(log_row(
        run_id,
        at,
        0,
        "",
        "unmapped_module",
        "salary",
        "",
        "skip",
        "Raw sheet has no payroll rows. Payslips stay header-only.",
    ));
    log.push(log_row(
        run_id,
        at,
        0,
        "",
        "unmapped_module",
        "inventory",
        "",
        "skip",
        "Raw sheet has no stock rows. Stock stays header-only.",
    ));
    log.push(log_row(
        run_id,
        at,
        0,
        "",
        "unmapped_module",
        "logistics",
        "",
        "skip",
        "Raw sheet has no trip rows. Trips stay header-only.",
    ));

    let imported = vouchers.len() as u32;
    let payments_out = payments.len() as u32;
    let skipped = parsed.skipped;
    MigrationPlan {
        run_id: run_id.into(),
        vouchers,
        payments,
        vendors,
        jobs,
        outstanding,
        board,
        log,
        imported,
        skipped,
        payments_out,
    }
}

pub fn log_cells(row: &MigrationLogRow) -> Vec<String> {
    vec![
        row.run_id.clone(),
        row.at.clone(),
        row.source_row.to_string(),
        row.source_serial.clone(),
        row.class.clone(),
        row.target_kind.clone(),
        row.target_key.clone(),
        row.action.clone(),
        row.detail.clone(),
    ]
}

/// Apply a plan onto an in-memory hive. Used by tests and dry-run. Never touches raw.
pub fn apply_plan_to_hive(hive: &mut dyn Hive, plan: &MigrationPlan) -> crate::Result<()> {
    let kinds = [
        (KIND_VOUCHER, report_headers(KIND_VOUCHER)),
        (KIND_VOUCHER_PAYMENT, report_headers(KIND_VOUCHER_PAYMENT)),
        (KIND_VENDOR, report_headers(KIND_VENDOR)),
        (KIND_PROJECT, report_headers(KIND_PROJECT)),
        (KIND_OUTSTANDING, report_headers(KIND_OUTSTANDING)),
        (KIND_BOARD, report_headers(KIND_BOARD)),
        (KIND_MIGRATION_LOG, report_headers(KIND_MIGRATION_LOG)),
    ];
    for (kind, headers) in kinds {
        let _ = hive.ensure_tab(kind, &headers)?;
    }
    for row in &plan.vouchers {
        hive.cas_row(KIND_VOUCHER, &row.key, "", 0, &row.cells, "mig")?;
    }
    for row in &plan.payments {
        hive.cas_row(KIND_VOUCHER_PAYMENT, &row.key, "", 0, &row.cells, "mig")?;
    }
    for cells in &plan.vendors {
        let key = cells.first().cloned().unwrap_or_default();
        hive.cas_row(KIND_VENDOR, &key, "", 0, cells, "mig")?;
    }
    for cells in &plan.jobs {
        let key = cells.first().cloned().unwrap_or_default();
        hive.cas_row(KIND_PROJECT, &key, "", 0, cells, "mig")?;
    }
    for cells in &plan.outstanding {
        let key = cells.first().cloned().unwrap_or_default();
        hive.cas_row(KIND_OUTSTANDING, &key, "", 0, cells, "mig")?;
    }
    for (i, cells) in plan.board.iter().enumerate() {
        hive.cas_row(KIND_BOARD, &format!("B{}", i + 1), "", 0, cells, "mig")?;
    }
    for (i, row) in plan.log.iter().enumerate() {
        hive.cas_row(
            KIND_MIGRATION_LOG,
            &format!("{}-{}", plan.run_id, i + 1),
            "",
            0,
            &log_cells(row),
            "mig",
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hive::MemoryHive;
    use crate::testdata::{mixed_sheet, voucher_row};

    #[test]
    fn dummy_and_empty_are_logged_and_not_imported() {
        let mut values = vec![vec![
            "voucher_number".into(),
            "date".into(),
            "tax inv".into(),
            "vendor".into(),
        ]];
        values.push(voucher_row(20, 1.0));
        values.push(vec![
            "VOUCHER_1001".into(),
            "".into(),
            "".into(),
            "VEND_02".into(),
        ]);
        values.push(vec!["21".into(), "".into(), "".into(), "".into()]);
        values.push(vec!["20.1".into(), "".into(), "".into(), "VEND_02".into()]);
        let plan = plan_from_raw_values(&values, "run-test");
        assert_eq!(plan.imported, 1);
        assert!(plan.log.iter().any(|r| r.class == "skipped_dummy"));
        assert!(plan.log.iter().any(|r| r.class == "skipped_empty_vendor"));
        assert!(plan.log.iter().any(|r| r.class == "skipped_dotted"));
        assert_eq!(plan.vouchers[0].key, "20");
        assert_eq!(plan.vouchers[0].class, "project_expense");
        assert!(plan.log.iter().any(|r| r.action == "map" && r.target_kind == KIND_VOUCHER));
        assert!(plan.log.iter().any(|r| r.class == "unmapped_module" && r.target_kind == "logistics"));
        assert!(!plan.payments.is_empty());
        assert!(plan.vendors.iter().any(|v| v[0].starts_with("VEND_")));
    }

    #[test]
    fn apply_does_not_need_raw_tab() {
        let values = mixed_sheet(3);
        let plan = plan_from_raw_values(&values, "run-mem");
        let mut hive = MemoryHive::default();
        apply_plan_to_hive(&mut hive, &plan).unwrap();
        assert!(hive.list_keys(KIND_VOUCHER).unwrap().len() >= 1);
        assert!(hive.get_row("voucher_raw", "20").unwrap().is_none());
    }

    #[test]
    fn payments_are_lines_not_new_vouchers() {
        let mut values = vec![vec!["voucher_number".into(), "d".into(), "t".into(), "vendor".into()]];
        values.push(voucher_row(44, 0.4));
        let plan = plan_from_raw_values(&values, "run-pay");
        assert_eq!(plan.vouchers.len(), 1);
        assert_eq!(plan.payments.len(), 1);
        assert!(plan.payments[0].key.starts_with("44#"));
        assert!(plan.log.iter().any(|r| r.detail.contains("Not a second bill")));
    }
}
