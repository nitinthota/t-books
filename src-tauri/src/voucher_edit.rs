//! Local voucher create / edit. SQLite only. Views never call Google.

use rusqlite::params;

use crate::core::business_rules::{
    add_payment, empty_payment, is_dotted_child_serial, is_dummy_serial, parse_voucher_number,
    summarize_voucher, PaymentBlock, VoucherInput, MAX_PAYMENT_BLOCKS,
};
use crate::db::LocalBooks;
use crate::vouchers::VoucherView;
use crate::{BooksError, Result};

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentSave {
    pub slot: i32,
    #[serde(default)]
    pub pi_no: String,
    #[serde(default)]
    pub pi_date: String,
    #[serde(default)]
    pub pi_value: f64,
    #[serde(default)]
    pub paid: f64,
    #[serde(default)]
    pub remaining: f64,
    #[serde(default)]
    pub payment_date: String,
    #[serde(default)]
    pub remarks: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tds: f64,
    #[serde(default)]
    pub payment_details: String,
    #[serde(default)]
    pub payment_percent: f64,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoucherSave {
    pub voucher_number: i64,
    #[serde(default)]
    pub voucher_date: String,
    #[serde(default)]
    pub tax_invoice: String,
    pub vendor: String,
    #[serde(default)]
    pub bank: String,
    #[serde(default)]
    pub account_number: String,
    #[serde(default)]
    pub ifsc: String,
    #[serde(default)]
    pub gst: String,
    #[serde(default)]
    pub project: String,
    #[serde(default)]
    pub comments: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub voucher_type: String,
    #[serde(default)]
    pub payments: Vec<PaymentSave>,
}

fn occupied(block: &PaymentBlock) -> bool {
    block.pi_value_rupees.unwrap_or(0.0) != 0.0
        || block.paid_rupees.unwrap_or(0.0) != 0.0
        || block.tds_rupees.unwrap_or(0.0) != 0.0
        || !block.payment_date.trim().is_empty()
        || !block.payment_details.trim().is_empty()
        || !block.pi_no.trim().is_empty()
}

fn block_from(p: &PaymentSave, fallback_slot: i32) -> PaymentBlock {
    let mut block = empty_payment(if p.slot >= 1 && p.slot <= MAX_PAYMENT_BLOCKS as i32 {
        p.slot
    } else {
        fallback_slot
    });
    block.pi_no = p.pi_no.trim().to_string();
    block.pi_date = p.pi_date.trim().to_string();
    block.pi_value_rupees = Some(p.pi_value);
    block.payment_percent = Some(p.payment_percent);
    block.description = p.description.trim().to_string();
    block.tds_rupees = Some(p.tds);
    block.paid_rupees = Some(p.paid);
    block.payment_details = p.payment_details.trim().to_string();
    block.payment_date = p.payment_date.trim().to_string();
    block.remaining_rupees = Some(p.remaining);
    block.remarks = p.remarks.trim().to_string();
    block
}

pub fn next_voucher_number(books: &LocalBooks) -> Result<i64> {
    let n: i64 = books
        .conn()
        .query_row("SELECT COALESCE(MAX(voucher_number), 0) FROM vouchers", [], |row| {
            row.get(0)
        })?;
    Ok(n + 1)
}

pub fn get_voucher_full(books: &LocalBooks, voucher_number: i64) -> Result<VoucherView> {
    crate::vouchers::get_voucher(books, voucher_number)
}

pub fn save_voucher(books: &mut LocalBooks, payload: VoucherSave) -> Result<VoucherView> {
    let raw = payload.voucher_number.to_string();
    if is_dotted_child_serial(&raw) {
        return Err(BooksError::Message(
            "Voucher 20.1 is a child serial. Use integer 20.".into(),
        ));
    }
    if is_dummy_serial(&raw) {
        return Err(BooksError::Message("Dummy / sample rows are not books.".into()));
    }
    let number = parse_voucher_number(&raw).unwrap_or(payload.voucher_number);
    if number <= 0 {
        return Err(BooksError::Message(
            "Column A must be an integer voucher number. Never 20.1.".into(),
        ));
    }
    let vendor = payload.vendor.trim();
    if vendor.is_empty() {
        return Err(BooksError::Message("Vendor is required.".into()));
    }

    let mut live: Vec<PaymentBlock> = payload
        .payments
        .iter()
        .enumerate()
        .map(|(i, p)| block_from(p, (i as i32) + 1))
        .filter(|b| occupied(b))
        .collect();
    if live.len() > MAX_PAYMENT_BLOCKS {
        let extra = live[MAX_PAYMENT_BLOCKS].clone();
        live.truncate(MAX_PAYMENT_BLOCKS);
        match add_payment(&live, extra) {
            Ok(_) => {
                return Err(BooksError::Message(
                    "This voucher already has 5 payments.".into(),
                ))
            }
            Err(msg) => return Err(BooksError::Message(msg)),
        }
    }

    let input = VoucherInput {
        voucher_number: number,
        tax_invoice: payload.tax_invoice.trim().to_string(),
        vendor: vendor.to_string(),
        bank: payload.bank.trim().to_string(),
        account_number: payload.account_number.trim().to_string(),
        ifsc: payload.ifsc.trim().to_string(),
        gst: payload.gst.trim().to_string(),
        project: payload.project.trim().to_string(),
        comments: payload.comments.trim().to_string(),
        payments: live.clone(),
    };
    let computed = summarize_voucher(&input);
    let comments = input.comments.clone();
    let description = payload.description.trim().to_string();

    let tx = books.conn_mut().transaction()?;
    tx.execute(
        r#"
        INSERT INTO vouchers (
          voucher_number, voucher_date, tax_invoice, vendor, bank, account_number, ifsc, gst, project,
          comments, description, dirty, is_dirty, source, total_value, total_paid, remaining,
          status, tax_flag, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 1, 1, 'local', ?12, ?13, ?14, ?15, ?16, datetime('now'), datetime('now'))
        ON CONFLICT(voucher_number) DO UPDATE SET
          voucher_date = excluded.voucher_date,
          tax_invoice = excluded.tax_invoice,
          vendor = excluded.vendor,
          bank = excluded.bank,
          account_number = excluded.account_number,
          ifsc = excluded.ifsc,
          gst = excluded.gst,
          project = excluded.project,
          comments = excluded.comments,
          description = excluded.description,
          dirty = 1,
          is_dirty = 1,
          source = 'local',
          total_value = excluded.total_value,
          total_paid = excluded.total_paid,
          remaining = excluded.remaining,
          status = excluded.status,
          tax_flag = excluded.tax_flag,
          updated_at = excluded.updated_at
        "#,
        params![
            number,
            payload.voucher_date.trim(),
            input.tax_invoice,
            vendor,
            input.bank,
            input.account_number,
            input.ifsc,
            input.gst,
            input.project,
            comments,
            description,
            computed.total_value,
            computed.total_paid,
            computed.remaining,
            computed.status,
            computed.tax_flag.as_str(),
        ],
    )?;
    tx.execute(
        "DELETE FROM voucher_payments WHERE voucher_number = ?1",
        params![number],
    )?;
    for p in &live {
        let remaining = p.remaining_rupees.unwrap_or(
            (p.pi_value_rupees.unwrap_or(0.0) - p.paid_rupees.unwrap_or(0.0) - p.tds_rupees.unwrap_or(0.0))
                .round(),
        );
        tx.execute(
            r#"
            INSERT INTO voucher_payments (
              voucher_number, slot, pi_no, pi_date, pi_value_rupees, payment_percent, description,
              tds_rupees, paid_rupees, payment_details, payment_date, remaining_rupees, remarks
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            "#,
            params![
                number,
                p.slot,
                p.pi_no,
                p.pi_date,
                p.pi_value_rupees.unwrap_or(0.0),
                p.payment_percent.unwrap_or(0.0),
                p.description,
                p.tds_rupees.unwrap_or(0.0),
                p.paid_rupees.unwrap_or(0.0),
                p.payment_details,
                p.payment_date,
                remaining,
                p.remarks,
            ],
        )?;
    }
    if !vendor.is_empty() {
        tx.execute(
            r#"
            INSERT INTO vendors (vendor, bank, account_number, ifsc, gst, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))
            ON CONFLICT(vendor) DO UPDATE SET
              bank = excluded.bank,
              account_number = excluded.account_number,
              ifsc = excluded.ifsc,
              gst = excluded.gst,
              updated_at = excluded.updated_at
            "#,
            params![
                vendor,
                input.bank,
                input.account_number,
                input.ifsc,
                input.gst
            ],
        )?;
    }
    if !input.project.is_empty() {
        tx.execute(
            r#"
            INSERT INTO projects (project, updated_at)
            VALUES (?1, datetime('now'))
            ON CONFLICT(project) DO UPDATE SET updated_at = excluded.updated_at
            "#,
            params![input.project],
        )?;
    }
    tx.commit()?;
    get_voucher_full(books, number)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_memory;

    #[test]
    fn save_marks_dirty_and_rejects_sixth() {
        let mut books = open_memory().unwrap();
        let mut pays = Vec::new();
        for i in 1..=5 {
            pays.push(PaymentSave {
                slot: i,
                pi_no: format!("PI-{i}"),
                pi_date: String::new(),
                pi_value: 100.0,
                paid: 10.0,
                remaining: 90.0,
                payment_date: String::new(),
                remarks: String::new(),
                description: String::new(),
                tds: 0.0,
                payment_details: String::new(),
                payment_percent: 0.0,
            });
        }
        let view = save_voucher(
            &mut books,
            VoucherSave {
                voucher_number: 20,
                voucher_date: "2026-09-01".into(),
                tax_invoice: "INV-20".into(),
                vendor: "VEND_02".into(),
                bank: "BANK_01".into(),
                account_number: "000".into(),
                ifsc: "IFSC000".into(),
                gst: "GST00".into(),
                project: "CUST_01".into(),
                comments: String::new(),
                description: "first".into(),
                voucher_type: "purchase".into(),
                payments: pays.clone(),
            },
        )
        .unwrap();
        assert_eq!(view.voucher_number, 20);
        assert!(view.is_dirty);
        assert_eq!(view.payments.len(), 5);

        pays.push(PaymentSave {
            slot: 6,
            pi_no: "PI-6".into(),
            pi_date: String::new(),
            pi_value: 1.0,
            paid: 0.0,
            remaining: 1.0,
            payment_date: String::new(),
            remarks: String::new(),
            description: String::new(),
            tds: 0.0,
            payment_details: String::new(),
            payment_percent: 0.0,
        });
        let err = save_voucher(
            &mut books,
            VoucherSave {
                voucher_number: 20,
                voucher_date: String::new(),
                tax_invoice: "INV-20".into(),
                vendor: "VEND_02".into(),
                bank: String::new(),
                account_number: String::new(),
                ifsc: String::new(),
                gst: String::new(),
                project: "CUST_01".into(),
                comments: String::new(),
                description: String::new(),
                voucher_type: String::new(),
                payments: pays,
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("5 payments"));
    }
}
