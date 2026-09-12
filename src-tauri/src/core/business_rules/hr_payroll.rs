//! SAL-n salary slips. Port of loopbook `hr-payroll.ts`.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonthName {
    pub n: u8,
    pub label: &'static str,
    pub short: &'static str,
}

pub const MONTHS: [MonthName; 12] = [
    MonthName { n: 1, label: "January", short: "Jan" },
    MonthName { n: 2, label: "February", short: "Feb" },
    MonthName { n: 3, label: "March", short: "Mar" },
    MonthName { n: 4, label: "April", short: "Apr" },
    MonthName { n: 5, label: "May", short: "May" },
    MonthName { n: 6, label: "June", short: "Jun" },
    MonthName { n: 7, label: "July", short: "Jul" },
    MonthName { n: 8, label: "August", short: "Aug" },
    MonthName { n: 9, label: "September", short: "Sep" },
    MonthName { n: 10, label: "October", short: "Oct" },
    MonthName { n: 11, label: "November", short: "Nov" },
    MonthName { n: 12, label: "December", short: "Dec" },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayKind {
    Salary,
    Advance,
}

impl PayKind {
    pub fn as_str(self) -> &'static str {
        match self {
            PayKind::Salary => "salary",
            PayKind::Advance => "advance",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayrollStatus {
    Current,
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayrollMoney {
    pub salary_paise: i64,
    pub pf_company_paise: i64,
    pub pf_employee_paise: i64,
    pub pf_total_paise: i64,
    pub tds_paise: i64,
    pub net_paise: i64,
    pub ctc_paise: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SalarySlip {
    pub kind: PayKind,
    pub gross_paise: i64,
    pub pf_employee_paise: i64,
    pub pf_company_paise: i64,
    pub tds_paise: i64,
    pub recovery_paise: i64,
    pub net_paise: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerLine {
    pub account_code: String,
    pub debit_paise: i64,
    pub credit_paise: i64,
    pub remarks: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayrollAsOfRow {
    pub id: String,
    pub employee_id: Option<String>,
    pub employee_name: String,
    pub effective_from: Option<String>,
    pub revision_no: Option<i64>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SalaryPeriodRow {
    pub id: Option<String>,
    pub employee_id: Option<String>,
    pub employee_name: String,
    pub pay_kind: Option<String>,
    pub period_month: i32,
    pub period_year: i32,
}

pub fn month_label(month: i32, short: bool) -> &'static str {
    MONTHS
        .iter()
        .find(|m| m.n as i32 == month)
        .map(|m| if short { m.short } else { m.label })
        .unwrap_or("—")
}

pub fn period_label(month: i32, year: i32) -> String {
    if !(1990..).contains(&year) {
        return month_label(month, false).to_string();
    }
    format!("{} {year}", month_label(month, true))
}

pub fn current_period(today_iso: &str) -> (i32, i32, String) {
    let d = if today_iso.len() >= 10 {
        &today_iso[..10]
    } else {
        today_iso
    };
    let year: i32 = d.get(..4).and_then(|s| s.parse().ok()).unwrap_or(2026);
    let month: i32 = d.get(5..7).and_then(|s| s.parse().ok()).unwrap_or(1);
    (month, year, d.to_string())
}

pub fn year_options(from: i32, today_year: i32) -> Vec<i32> {
    let max = today_year + 1;
    (from..=max).rev().collect()
}

pub fn normalize_pay_kind(raw: Option<&str>) -> PayKind {
    if raw.unwrap_or("").trim().eq_ignore_ascii_case("advance") {
        PayKind::Advance
    } else {
        PayKind::Salary
    }
}

/// Posted salary voucher numbers are kept; never steal a live number.
pub fn keep_posted_pay_number(posted_no: &str, requested_no: &str) -> String {
    let posted = posted_no.trim();
    if !posted.is_empty() {
        posted.to_string()
    } else {
        requested_no.trim().to_string()
    }
}

fn paise(v: i64) -> i64 {
    v
}

pub fn payroll_money_changed(a: &PayrollMoney, b: &PayrollMoney) -> bool {
    a.salary_paise != b.salary_paise
        || a.pf_company_paise != b.pf_company_paise
        || a.pf_employee_paise != b.pf_employee_paise
        || a.tds_paise != b.tds_paise
}

pub fn payroll_net_ok(money: &PayrollMoney) -> bool {
    money.salary_paise > 0 && money.net_paise >= 0
}

pub fn outstanding_advance_paise(rows: &[(PayKind, i64, i64)]) -> i64 {
    let mut adv = 0i64;
    let mut rec = 0i64;
    for (kind, amount, recovery) in rows {
        if *kind == PayKind::Advance {
            adv += paise(*amount);
        }
        rec += paise(*recovery);
    }
    (adv - rec).max(0)
}

pub fn calc_salary_slip(
    kind: PayKind,
    salary_paise: i64,
    pf_employee_paise: i64,
    pf_company_paise: i64,
    tds_paise: i64,
    recovery_paise: i64,
    amount_paise: i64,
) -> SalarySlip {
    if kind == PayKind::Advance {
        let amount = amount_paise.max(0);
        return SalarySlip {
            kind,
            gross_paise: amount,
            pf_employee_paise: 0,
            pf_company_paise: 0,
            tds_paise: 0,
            recovery_paise: 0,
            net_paise: amount,
        };
    }
    let gross = salary_paise.max(0);
    let emp_pf = pf_employee_paise.max(0);
    let co_pf = pf_company_paise.max(0);
    let tds = tds_paise.max(0);
    let max_recovery = (gross - emp_pf - tds).max(0);
    let recovery = recovery_paise.max(0).min(max_recovery);
    SalarySlip {
        kind,
        gross_paise: gross,
        pf_employee_paise: emp_pf,
        pf_company_paise: co_pf,
        tds_paise: tds,
        recovery_paise: recovery,
        net_paise: gross - emp_pf - tds - recovery,
    }
}

pub fn bank_account_code(pay_mode: Option<&str>) -> &'static str {
    if pay_mode.unwrap_or("").trim().eq_ignore_ascii_case("cash") {
        "1000"
    } else {
        "1010"
    }
}

pub fn salary_voucher_lines(slip: &SalarySlip, pay_mode: Option<&str>) -> Vec<LedgerLine> {
    let bank = bank_account_code(pay_mode);
    let mut lines = Vec::new();
    let mut push = |account_code: &str, debit: i64, credit: i64, remarks: &str| {
        if debit <= 0 && credit <= 0 {
            return;
        }
        lines.push(LedgerLine {
            account_code: account_code.to_string(),
            debit_paise: debit,
            credit_paise: credit,
            remarks: remarks.to_string(),
        });
    };
    if slip.kind == PayKind::Advance {
        push("1400", slip.net_paise, 0, "Staff advance");
        push(
            bank,
            0,
            slip.net_paise,
            if bank == "1000" { "Cash" } else { "Bank" },
        );
        return lines;
    }
    let expense = slip.gross_paise + slip.pf_company_paise;
    let pf_payable = slip.pf_employee_paise + slip.pf_company_paise;
    push("5300", expense, 0, "Salary");
    push("5400", 0, pf_payable, "PF payable");
    push("2100", 0, slip.tds_paise, "TDS");
    push("1400", 0, slip.recovery_paise, "Advance recovered");
    push(
        bank,
        0,
        slip.net_paise,
        if bank == "1000" { "Cash" } else { "Bank" },
    );
    lines
}

pub fn lines_balance(lines: &[LedgerLine]) -> bool {
    let dr: i64 = lines.iter().map(|l| l.debit_paise).sum();
    let cr: i64 = lines.iter().map(|l| l.credit_paise).sum();
    dr == cr && dr > 0
}

pub fn pick_payroll_as_of<'a>(
    rows: &'a [PayrollAsOfRow],
    employee_id: Option<&str>,
    employee_name: Option<&str>,
    on_date: &str,
) -> Option<&'a PayrollAsOfRow> {
    let id = employee_id.unwrap_or("").trim();
    let name = employee_name.unwrap_or("").trim().to_ascii_lowercase();
    let date = if on_date.len() >= 10 {
        &on_date[..10]
    } else {
        on_date
    };
    let mut matches: Vec<&PayrollAsOfRow> = rows
        .iter()
        .filter(|row| {
            if !id.is_empty() {
                row.employee_id.as_deref() == Some(id)
            } else if !name.is_empty() {
                row.employee_name.trim().eq_ignore_ascii_case(&name)
            } else {
                false
            }
        })
        .filter(|row| {
            if date.is_empty() {
                return true;
            }
            match row.effective_from.as_deref() {
                None | Some("") => true,
                Some(from) => {
                    let from = if from.len() >= 10 { &from[..10] } else { from };
                    from <= date
                }
            }
        })
        .collect();
    matches.sort_by(|a, b| {
        let da = a
            .effective_from
            .as_deref()
            .map(|s| if s.len() >= 10 { &s[..10] } else { s })
            .unwrap_or("");
        let db = b
            .effective_from
            .as_deref()
            .map(|s| if s.len() >= 10 { &s[..10] } else { s })
            .unwrap_or("");
        db.cmp(da)
            .then((b.revision_no.unwrap_or(0)).cmp(&a.revision_no.unwrap_or(0)))
    });
    matches.into_iter().next()
}

pub fn salary_period_taken(existing: &[SalaryPeriodRow], input: &SalaryPeriodRow) -> bool {
    let id = input.employee_id.as_deref().unwrap_or("").trim();
    let name = input.employee_name.trim().to_ascii_lowercase();
    existing.iter().any(|row| {
        if normalize_pay_kind(row.pay_kind.as_deref()) != PayKind::Salary {
            return false;
        }
        if row.period_month != input.period_month || row.period_year != input.period_year {
            return false;
        }
        if let (Some(a), Some(b)) = (input.id.as_deref(), row.id.as_deref()) {
            if a == b {
                return false;
            }
        }
        if !id.is_empty() {
            row.employee_id.as_deref() == Some(id)
        } else {
            row.employee_name.trim().eq_ignore_ascii_case(&name)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn salary_slip_recovers_advance_and_balances() {
        let slip = calc_salary_slip(
            PayKind::Salary,
            50_000_00,
            1_800_00,
            1_800_00,
            500_00,
            2_000_00,
            0,
        );
        assert_eq!(slip.net_paise, 50_000_00 - 1_800_00 - 500_00 - 2_000_00);
        let lines = salary_voucher_lines(&slip, Some("bank"));
        assert!(lines_balance(&lines));
    }

    #[test]
    fn posted_salary_number_is_sacred() {
        assert_eq!(keep_posted_pay_number("SAL-0001", "SAL-0009"), "SAL-0001");
        assert_eq!(keep_posted_pay_number("", "SAL-0009"), "SAL-0009");
    }

    #[test]
    fn one_salary_per_person_per_month() {
        let existing = [SalaryPeriodRow {
            id: Some("a".into()),
            employee_id: Some("p1".into()),
            employee_name: "CUST_01".into(),
            pay_kind: Some("salary".into()),
            period_month: 9,
            period_year: 2026,
        }];
        let clash = SalaryPeriodRow {
            id: Some("b".into()),
            employee_id: Some("p1".into()),
            employee_name: "CUST_01".into(),
            pay_kind: Some("salary".into()),
            period_month: 9,
            period_year: 2026,
        };
        assert!(salary_period_taken(&existing, &clash));
        let same = SalaryPeriodRow {
            id: Some("a".into()),
            ..clash.clone()
        };
        assert!(!salary_period_taken(&existing, &same));
    }

    #[test]
    fn outstanding_advance_never_negative() {
        assert_eq!(
            outstanding_advance_paise(&[(PayKind::Advance, 1000, 0), (PayKind::Salary, 0, 400)]),
            600
        );
        assert_eq!(
            outstanding_advance_paise(&[(PayKind::Advance, 100, 0), (PayKind::Salary, 0, 400)]),
            0
        );
    }
}
