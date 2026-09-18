//! Indian FY trial balance. Reads SQLite only. Must balance.

use rusqlite::params;

use crate::book_date::book_date_to_iso;
use crate::core::business_rules::{
    clamp_as_of, current_indian_fy, fy_bounds, fy_options, rupees_to_paise, signed_dr_cr,
    trial_balanced,
};
use crate::db::LocalBooks;
use crate::Result;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrialLine {
    pub account: String,
    pub debit_rupees: f64,
    pub credit_rupees: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrialBalance {
    pub fy: String,
    pub as_of: String,
    pub start: String,
    pub end: String,
    pub lines: Vec<TrialLine>,
    pub total_debit_rupees: f64,
    pub total_credit_rupees: f64,
    pub balanced: bool,
}

fn in_range(date: &str, start: &str, end: &str) -> bool {
    match book_date_to_iso(date) {
        Some(d) => d.as_str() >= start && d.as_str() <= end,
        None => false,
    }
}

fn month_in_range(month: &str, start: &str, end: &str) -> bool {
    if month.len() < 7 {
        return true;
    }
    let day = format!("{month}-01");
    in_range(&day, start, end)
}

fn add(map: &mut std::collections::BTreeMap<String, (i64, i64)>, account: &str, paise: i64) {
    let name = account.trim();
    if name.is_empty() || paise == 0 {
        return;
    }
    let (dr, cr) = signed_dr_cr(paise);
    let slot = map.entry(name.to_string()).or_insert((0, 0));
    slot.0 += dr;
    slot.1 += cr;
}

pub fn build_trial(books: &LocalBooks, fy: Option<&str>, as_of: Option<&str>) -> Result<TrialBalance> {
    let fy_label = fy
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| current_indian_fy(None));
    let (start, end) = fy_bounds(&fy_label);
    let as_of_day = clamp_as_of(as_of, &start, &end, &end);
    let mut amounts: std::collections::BTreeMap<String, (i64, i64)> = std::collections::BTreeMap::new();

    {
        let mut stmt = books.conn().prepare(
            "SELECT COALESCE(project,''), COALESCE(vendor,''), COALESCE(total_value,0),
                    COALESCE(total_paid,0), COALESCE(voucher_date,'')
             FROM vouchers",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
                row.get::<_, f64>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;
        for row in rows {
            let (project, vendor, value, paid, date) = row?;
            if !in_range(&date, &start, &as_of_day) {
                continue;
            }
            let value_p = rupees_to_paise(value);
            let paid_p = rupees_to_paise(paid);
            add(&mut amounts, &format!("Project {project}"), value_p);
            add(&mut amounts, &format!("Vendor {vendor}"), -value_p);
            add(&mut amounts, &format!("Vendor {vendor}"), paid_p);
            add(&mut amounts, "Bank", -paid_p);
        }
    }
    {
        let mut stmt = books.conn().prepare(
            "SELECT COALESCE(project,''), COALESCE(vendor,''), COALESCE(total_value,0),
                    COALESCE(created_at,''), po_number
             FROM purchase_po",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;
        for row in rows {
            let (project, vendor, value, date, po) = row?;
            if !in_range(&date, &start, &as_of_day) {
                continue;
            }
            let value_p = rupees_to_paise(value);
            add(&mut amounts, &format!("Project {project}"), value_p);
            add(&mut amounts, &format!("Vendor {vendor}"), -value_p);
            let paid: f64 = books.conn().query_row(
                "SELECT COALESCE(SUM(amount_rupees),0) FROM purchase_payments WHERE po_number = ?1 COLLATE NOCASE",
                params![po],
                |r| r.get(0),
            )?;
            let paid_p = rupees_to_paise(paid);
            add(&mut amounts, &format!("Vendor {vendor}"), paid_p);
            add(&mut amounts, "Bank", -paid_p);
        }
    }
    {
        let mut stmt = books.conn().prepare(
            "SELECT p.month, p.net_rupees, p.pf_employee, p.pf_company, p.tds, p.recovery_rupees,
                    p.salary_rupees, COALESCE(h.name,'')
             FROM hr_payroll p JOIN hr_people h ON h.id = p.person_id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<f64>>(1)?.unwrap_or(0.0),
                row.get::<_, Option<f64>>(2)?.unwrap_or(0.0),
                row.get::<_, Option<f64>>(3)?.unwrap_or(0.0),
                row.get::<_, Option<f64>>(4)?.unwrap_or(0.0),
                row.get::<_, Option<f64>>(5)?.unwrap_or(0.0),
                row.get::<_, Option<f64>>(6)?.unwrap_or(0.0),
                row.get::<_, String>(7)?,
            ))
        })?;
        for row in rows {
            let (month, net, pf_e, pf_c, tds, recovery, salary, _name) = row?;
            if !month_in_range(&month, &start, &as_of_day) {
                continue;
            }
            let expense = rupees_to_paise(salary) + rupees_to_paise(pf_c);
            add(&mut amounts, "Salary", expense);
            add(&mut amounts, "PF payable", -(rupees_to_paise(pf_e) + rupees_to_paise(pf_c)));
            add(&mut amounts, "TDS", -rupees_to_paise(tds));
            add(&mut amounts, "Staff advance", -rupees_to_paise(recovery));
            add(&mut amounts, "Bank", -rupees_to_paise(net));
        }
    }

    let pairs: Vec<(i64, i64)> = amounts.values().copied().collect();
    let balanced = pairs.is_empty() || trial_balanced(&pairs);
    let mut lines: Vec<TrialLine> = amounts
        .into_iter()
        .map(|(account, (dr, cr))| TrialLine {
            account,
            debit_rupees: dr as f64 / 100.0,
            credit_rupees: cr as f64 / 100.0,
        })
        .collect();
    lines.sort_by(|a, b| a.account.cmp(&b.account));
    let total_debit_rupees: f64 = lines.iter().map(|l| l.debit_rupees).sum();
    let total_credit_rupees: f64 = lines.iter().map(|l| l.credit_rupees).sum();
    Ok(TrialBalance {
        fy: fy_label,
        as_of: as_of_day,
        start,
        end,
        lines,
        total_debit_rupees,
        total_credit_rupees,
        balanced,
    })
}

pub fn available_fy(books: &LocalBooks) -> Result<Vec<String>> {
    let mut min: Option<String> = None;
    let mut max: Option<String> = None;
    let mut stmt = books
        .conn()
        .prepare("SELECT COALESCE(voucher_date,'') FROM vouchers")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    for row in rows {
        let d = row?;
        let Some(s) = book_date_to_iso(&d) else {
            continue;
        };
        min = Some(match min.take() {
            Some(cur) if cur <= s => cur,
            _ => s.clone(),
        });
        max = Some(match max.take() {
            Some(cur) if cur >= s => cur,
            _ => s,
        });
    }
    Ok(fy_options(min.as_deref(), max.as_deref(), None))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::office::{save_purchase_po, PurchasePoSave};
    use crate::open_memory;

    #[test]
    fn trial_from_local_purchase_balances() {
        let mut books = open_memory().unwrap();
        save_purchase_po(
            &mut books,
            PurchasePoSave {
                project: "CUST_01".into(),
                vendor: "VEND_02".into(),
                po_number: "PUR-0001".into(),
                po_type: "simple".into(),
                total_value: 100.0,
                ..Default::default()
            },
        )
        .unwrap();
        let trial = build_trial(&books, Some("2026-27"), None).unwrap();
        assert!(trial.balanced, "{trial:?}");
        assert!(!trial.lines.is_empty());
    }

    #[test]
    fn book_date_12_apr_is_2025_26() {
        assert_eq!(
            book_date_to_iso("12.04.2025").as_deref(),
            Some("2025-04-12")
        );
        assert!(in_range("12.04.2025", "2025-04-01", "2026-03-31"));
        assert!(!in_range("12.04.2025", "2024-04-01", "2025-03-31"));
    }
}
