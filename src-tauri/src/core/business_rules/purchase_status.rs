//! PUR-n / PAY-n purchase bills. Port of loopbook `purchase-status.ts`.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PurchasePayStatus {
    Unpaid,
    PartiallyPaid,
    FullyPaid,
}

impl PurchasePayStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            PurchasePayStatus::Unpaid => "Unpaid",
            PurchasePayStatus::PartiallyPaid => "Partially Paid",
            PurchasePayStatus::FullyPaid => "Fully Paid",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PurchaseType {
    Po,
    NonPo,
}

impl PurchaseType {
    pub fn as_str(self) -> &'static str {
        match self {
            PurchaseType::Po => "po",
            PurchaseType::NonPo => "non_po",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeMode {
    Existing,
    New,
    Default,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaymentClass {
    Advance,
    Partial,
    FullyPaid,
}

impl PaymentClass {
    pub fn as_str(self) -> &'static str {
        match self {
            PaymentClass::Advance => "advance",
            PaymentClass::Partial => "partial",
            PaymentClass::FullyPaid => "fully_paid",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaymentFilterKey {
    Advance,
    Partial,
    FullyPaid,
    MissingTax,
}

impl PaymentFilterKey {
    pub fn as_str(self) -> &'static str {
        match self {
            PaymentFilterKey::Advance => "advance",
            PaymentFilterKey::Partial => "partial",
            PaymentFilterKey::FullyPaid => "fully_paid",
            PaymentFilterKey::MissingTax => "missing_tax",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocMethod {
    Advance,
    Against,
    OnAccount,
}

impl AllocMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            AllocMethod::Advance => "advance",
            AllocMethod::Against => "against",
            AllocMethod::OnAccount => "on_account",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkedVoucherRef {
    pub id: String,
    pub voucher_no: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeVoucherHint {
    pub id: String,
    pub vendor_key: String,
    pub po_id: Option<String>,
    pub source: Option<String>,
    pub reversed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefaultMergeTarget {
    Existing { po_id: String },
    New,
}

pub fn normalize_purchase_type(value: Option<&str>) -> PurchaseType {
    if value.map(|s| s.trim()) == Some("non_po") {
        PurchaseType::NonPo
    } else {
        PurchaseType::Po
    }
}

pub fn purchase_type_label(value: Option<&str>) -> &'static str {
    match normalize_purchase_type(value) {
        PurchaseType::Po => "Techsol Purchase",
        PurchaseType::NonPo => "Project Expenses",
    }
}

pub fn purchase_pay_status(grand_paise: i64, paid_paise: i64) -> PurchasePayStatus {
    if paid_paise <= 0 {
        PurchasePayStatus::Unpaid
    } else if paid_paise < grand_paise {
        PurchasePayStatus::PartiallyPaid
    } else {
        PurchasePayStatus::FullyPaid
    }
}

pub fn purchase_status_tone(status: PurchasePayStatus) -> super::status::StatusTone {
    match status {
        PurchasePayStatus::FullyPaid => super::status::StatusTone::Credit,
        PurchasePayStatus::Unpaid => super::status::StatusTone::Debit,
        PurchasePayStatus::PartiallyPaid => super::status::StatusTone::Muted,
    }
}

pub fn normalize_payment_class(value: Option<&str>) -> PaymentClass {
    match value.map(|s| s.trim()) {
        Some("partial") => PaymentClass::Partial,
        Some("fully_paid") => PaymentClass::FullyPaid,
        _ => PaymentClass::Advance,
    }
}

pub fn payment_class_label(value: Option<&str>) -> &'static str {
    match normalize_payment_class(value) {
        PaymentClass::Advance => "Advance",
        PaymentClass::Partial => "Partial",
        PaymentClass::FullyPaid => "Fully Paid",
    }
}

pub fn classify_purchase_payment(
    goods_received: bool,
    tax_invoice_missing: bool,
    grand_paise: i64,
    paid_before_paise: i64,
    this_paise: i64,
) -> (PaymentClass, bool) {
    let grand = grand_paise.max(0);
    let before = paid_before_paise.max(0);
    let amt = this_paise.max(0);
    let after = before + amt;
    let pay_class = if !goods_received {
        PaymentClass::Advance
    } else if grand > 0 && after >= grand {
        PaymentClass::FullyPaid
    } else {
        PaymentClass::Partial
    };
    (pay_class, tax_invoice_missing)
}

pub fn payment_over_total(grand_paise: i64, paid_before_paise: i64, this_paise: i64) -> bool {
    let grand = grand_paise.max(0);
    let before = paid_before_paise.max(0);
    let amt = this_paise.max(0);
    if amt <= 0 {
        return false;
    }
    before + amt > grand
}

pub fn payment_matches_filters(
    pay_class: Option<&str>,
    missing_tax_invoice: bool,
    selected: &[PaymentFilterKey],
) -> bool {
    if selected.is_empty() {
        return true;
    }
    let cls = normalize_payment_class(pay_class);
    selected.iter().any(|key| match key {
        PaymentFilterKey::MissingTax => missing_tax_invoice,
        PaymentFilterKey::Advance => cls == PaymentClass::Advance,
        PaymentFilterKey::Partial => cls == PaymentClass::Partial,
        PaymentFilterKey::FullyPaid => cls == PaymentClass::FullyPaid,
    })
}

pub fn tax_invoice_missing(no: Option<&str>, date: Option<&str>) -> bool {
    no.unwrap_or("").trim().is_empty() && date.unwrap_or("").trim().is_empty()
}

pub fn tax_invoice_label(no: Option<&str>, date: Option<&str>) -> String {
    let n = no.unwrap_or("").trim();
    let d = date.unwrap_or("").trim();
    let d = if d.len() >= 10 { &d[..10] } else { d };
    if !n.is_empty() && !d.is_empty() {
        format!("{n} / {d}")
    } else if !n.is_empty() {
        n.to_string()
    } else {
        d.to_string()
    }
}

pub fn is_http_url(raw: &str) -> bool {
    let s = raw.trim();
    if s.is_empty() {
        return false;
    }
    let lower = s.to_ascii_lowercase();
    (lower.starts_with("http://") || lower.starts_with("https://")) && s.contains('.')
}

pub fn document_link_audit(name: Option<&str>, user_id: &str, created_at: &str) -> String {
    let who = name.unwrap_or("").trim();
    let who = if who.is_empty() {
        if user_id.is_empty() {
            "someone"
        } else {
            "team member"
        }
    } else {
        who
    };
    let when = if created_at.len() >= 10 {
        &created_at[..10]
    } else {
        created_at.trim()
    };
    if when.is_empty() {
        format!("Added by {who}")
    } else {
        format!("Added by {who} · {when}")
    }
}

pub fn normalize_alloc_method(value: Option<&str>) -> AllocMethod {
    match value.map(|s| s.trim()) {
        Some("against") => AllocMethod::Against,
        Some("on_account") => AllocMethod::OnAccount,
        _ => AllocMethod::Advance,
    }
}

pub fn alloc_method_for_purchase(goods_received: bool) -> AllocMethod {
    if goods_received {
        AllocMethod::Against
    } else {
        AllocMethod::Advance
    }
}

pub fn alloc_method_label(value: Option<&str>) -> &'static str {
    match normalize_alloc_method(value) {
        AllocMethod::Advance => "Advance",
        AllocMethod::Against => "Against Ref",
        AllocMethod::OnAccount => "On Account",
    }
}

pub fn payment_allocation_label(method: Option<&str>, po_number: Option<&str>) -> String {
    let po = po_number.unwrap_or("").trim();
    let word = match normalize_alloc_method(method) {
        AllocMethod::Against => "Agst Ref",
        AllocMethod::OnAccount => "On Account",
        AllocMethod::Advance => "Advance",
    };
    if po.is_empty() {
        word.to_string()
    } else {
        format!("{word} {po}")
    }
}

pub fn parse_pay_voucher_seq(raw: Option<&str>) -> i64 {
    let s = raw.unwrap_or("").trim();
    let upper = s.to_ascii_uppercase();
    let rest = if let Some(r) = upper.strip_prefix("PAY-") {
        r
    } else if let Some(r) = upper.strip_prefix("PAY:") {
        r
    } else if let Some(r) = upper.strip_prefix("PAY/") {
        r
    } else if let Some(r) = upper.strip_prefix("PAY ") {
        r
    } else {
        return 0;
    };
    if rest.is_empty() || !rest.chars().all(|c| c.is_ascii_digit()) {
        0
    } else {
        rest.parse().unwrap_or(0)
    }
}

pub fn is_pay_voucher_number(raw: Option<&str>) -> bool {
    let s = raw.unwrap_or("").trim();
    let upper = s.to_ascii_uppercase();
    if let Some(rest) = upper.strip_prefix("PAY-") {
        !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit())
    } else {
        false
    }
}

fn eq_ci(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

fn strip_prefix_ci<'a>(hay: &'a str, prefix: &str) -> Option<&'a str> {
    if hay.len() < prefix.len() {
        return None;
    }
    if hay.get(..prefix.len()).is_some_and(|h| h.eq_ignore_ascii_case(prefix)) {
        Some(&hay[prefix.len()..])
    } else {
        None
    }
}

pub fn is_purchase_number(raw: &str) -> bool {
    let s = raw.trim();
    let upper = s.to_ascii_uppercase();
    if let Some(rest) = upper.strip_prefix("PUR-") {
        !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit())
    } else {
        false
    }
}

pub fn is_child_pay_number(raw: Option<&str>, po_number: Option<&str>) -> bool {
    let pay = raw.unwrap_or("").trim();
    if pay.is_empty() || is_pay_voucher_number(Some(pay)) {
        return false;
    }
    let po = po_number.unwrap_or("").trim();
    if !po.is_empty() {
        let prefix = format!("{po}-");
        if let Some(rest) = strip_prefix_ci(pay, &prefix) {
            return !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit());
        }
        return false;
    }
    if is_purchase_number(pay) {
        return false;
    }
    match pay.rsplit_once('-') {
        Some((head, tail)) => {
            !head.is_empty()
                && (1..=2).contains(&tail.len())
                && tail.chars().all(|c| c.is_ascii_digit())
        }
        None => false,
    }
}

pub fn parse_payment_seq(raw: Option<&str>, po_number: Option<&str>) -> i64 {
    let pay = parse_pay_voucher_seq(raw);
    if pay > 0 {
        return pay;
    }
    let child = raw.unwrap_or("").trim();
    let po = po_number.unwrap_or("").trim();
    if !po.is_empty() {
        let prefix = format!("{po}-");
        if let Some(rest) = strip_prefix_ci(child, &prefix) {
            if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()) {
                return rest.parse().unwrap_or(0);
            }
        }
    }
    if is_purchase_number(child) || is_pay_voucher_number(Some(child)) {
        return 0;
    }
    match child.rsplit_once('-') {
        Some((head, tail))
            if !head.is_empty()
                && (1..=2).contains(&tail.len())
                && tail.chars().all(|c| c.is_ascii_digit()) =>
        {
            tail.parse().unwrap_or(0)
        }
        _ => 0,
    }
}

pub fn format_payment_number(seq: i64) -> String {
    let n = seq.max(1);
    format!("PAY-{n:04}")
}

pub fn format_purchase_pay_number(po_number: &str, seq: i64) -> String {
    let po = po_number.trim();
    let po = if po.is_empty() { "PUR" } else { po };
    let n = seq.max(1);
    format!("{po}-{n:02}")
}

/// Placeholders we may replace. Posted PAY-n / PUR-n-01 are real — do not rewrite.
pub fn is_legacy_pay_number(raw: Option<&str>) -> bool {
    let s = raw.unwrap_or("").trim();
    if s.is_empty() {
        return true;
    }
    let upper = s.to_ascii_uppercase();
    if let Some(rest) = upper.strip_prefix("PAY:") {
        return !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit());
    }
    if let Some(rest) = upper.strip_prefix("PAY ") {
        return !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit());
    }
    false
}

pub fn payment_ordinal(seq: i64) -> String {
    let n = seq.max(0);
    if n <= 0 {
        return "payment".into();
    }
    let mod100 = n % 100;
    let mod10 = n % 10;
    let suffix = if !(11..=13).contains(&mod100) {
        match mod10 {
            1 => "st",
            2 => "nd",
            3 => "rd",
            _ => "th",
        }
    } else {
        "th"
    };
    format!("{n}{suffix} payment")
}

pub fn payment_ref(po_number: Option<&str>, pay_number: Option<&str>, method: Option<&str>) -> String {
    let po = po_number.unwrap_or("").trim();
    let pay = pay_number.unwrap_or("").trim();
    if !pay.is_empty() && !po.is_empty() {
        if method.is_some() {
            return format!("{pay} · {}", payment_allocation_label(method, Some(po)));
        }
        if is_child_pay_number(Some(pay), Some(po)) || eq_ci(pay, po) {
            return pay.to_string();
        }
        return format!("{pay} · {po}");
    }
    if !pay.is_empty() {
        pay.to_string()
    } else {
        po.to_string()
    }
}

pub fn is_external_payment(paid_by_name: Option<&str>) -> bool {
    !paid_by_name.unwrap_or("").trim().is_empty()
}

pub fn vendor_merge_key(vendor_name: Option<&str>, party_id: Option<&str>) -> String {
    let name = vendor_name.unwrap_or("").trim().to_ascii_lowercase();
    if !name.is_empty() {
        return format!("name:{name}");
    }
    let party = party_id.unwrap_or("").trim();
    if !party.is_empty() {
        return format!("party:{party}");
    }
    "name:".into()
}

pub fn merge_blocked_reason(
    vouchers: &[MergeVoucherHint],
    target_po_id: Option<&str>,
    target_vendor_key: Option<&str>,
) -> Option<String> {
    let live: Vec<&MergeVoucherHint> = vouchers.iter().filter(|v| !v.reversed).collect();
    if live.is_empty() {
        return Some("Select at least one voucher.".into());
    }
    let mut keys = std::collections::BTreeSet::new();
    for v in &live {
        keys.insert(v.vendor_key.as_str());
    }
    if keys.len() > 1 {
        return Some("Same vendor required.".into());
    }
    if let (Some(target), Some(first)) = (target_vendor_key, live.first()) {
        if first.vendor_key != target && first.vendor_key != "name:" {
            return Some("Same vendor required.".into());
        }
    }
    let mut linked: Vec<&str> = live
        .iter()
        .filter_map(|v| v.po_id.as_deref().map(str::trim).filter(|s| !s.is_empty()))
        .collect();
    linked.sort();
    linked.dedup();
    if linked.len() > 1 {
        return Some("A voucher is already on another purchase.".into());
    }
    if let (Some(target), Some(only)) = (target_po_id.filter(|s| !s.trim().is_empty()), linked.first()) {
        if *only != target {
            return Some("A voucher is already on another purchase.".into());
        }
    }
    None
}

pub fn merge_mode_blocked(mode: MergeMode, po_id: Option<&str>) -> Option<String> {
    if mode == MergeMode::Existing && po_id.unwrap_or("").trim().is_empty() {
        Some("Choose a purchase to merge onto.".into())
    } else {
        None
    }
}

pub fn pick_default_merge_target(
    voucher_po_ids: &[Option<&str>],
    vendor_purchase_ids: &[&str],
) -> DefaultMergeTarget {
    let mut linked: Vec<String> = voucher_po_ids
        .iter()
        .filter_map(|id| id.map(str::trim).filter(|s| !s.is_empty()).map(|s| s.to_string()))
        .collect();
    linked.sort();
    linked.dedup();
    if linked.len() == 1 {
        return DefaultMergeTarget::Existing {
            po_id: linked[0].clone(),
        };
    }
    if linked.is_empty() && vendor_purchase_ids.len() == 1 {
        return DefaultMergeTarget::Existing {
            po_id: vendor_purchase_ids[0].to_string(),
        };
    }
    DefaultMergeTarget::New
}

pub fn is_converted_origin(origin: Option<&str>) -> bool {
    matches!(origin, Some("converted") | Some("merged"))
}

pub fn is_linked_import_payment(source: Option<&str>) -> bool {
    let s = source.unwrap_or("").trim();
    !s.is_empty() && s != "purchase"
}

pub fn can_unmerge_voucher(source: Option<&str>) -> bool {
    is_linked_import_payment(source)
}

pub fn delete_reason_ok(reason: Option<&str>) -> bool {
    reason.unwrap_or("").trim().len() >= 3
}

pub fn is_blank_po_item(item_name: Option<&str>, description: Option<&str>, unit_rate_rupees: f64) -> bool {
    let named = !item_name.unwrap_or("").trim().is_empty() || !description.unwrap_or("").trim().is_empty();
    let valued = unit_rate_rupees > 0.0;
    !named && !valued
}

pub fn display_purchase_grand_paise(preview_grand_paise: i64, stored_grand_paise: i64) -> i64 {
    if preview_grand_paise > 0 {
        preview_grand_paise
    } else if stored_grand_paise > 0 {
        stored_grand_paise
    } else {
        0
    }
}

pub fn po_list_money(
    kind: &str,
    grand_total_paise: i64,
    amount_received_paise: i64,
    paid_paise: i64,
    balance_paise: i64,
) -> (i64, i64, i64) {
    if kind == "purchase" {
        (paid_paise, paid_paise, balance_paise)
    } else {
        (amount_received_paise, paid_paise, grand_total_paise - amount_received_paise)
    }
}

pub fn sum_rupees(values: &[Option<f64>]) -> f64 {
    values.iter().map(|v| v.unwrap_or(0.0)).sum()
}

pub fn combine_voucher_descriptions(existing: Option<&str>, narrations: &[&str]) -> String {
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    let mut push = |raw: &str| {
        let t = raw.trim();
        if t.is_empty() {
            return;
        }
        let key = t.to_ascii_lowercase();
        if seen.insert(key) {
            out.push(t.to_string());
        }
    };
    if let Some(ex) = existing {
        push(ex);
    }
    for n in narrations {
        push(n);
    }
    out.join(" · ")
}

pub fn should_post_purchase_bill(goods_received: bool, grand_paise: i64, converted: bool) -> bool {
    if converted {
        return false;
    }
    if grand_paise <= 0 {
        return false;
    }
    goods_received
}

pub fn last_voucher_unmerge(
    remaining_count: i64,
    auto_delete: bool,
    item_count: i64,
    has_posted_payments: bool,
) -> &'static str {
    if remaining_count > 0 {
        "keep"
    } else if has_posted_payments || item_count > 0 {
        "empty"
    } else if auto_delete {
        "delete"
    } else {
        "empty"
    }
}

pub fn format_bank_label(
    bank_name: Option<&str>,
    account_no: Option<&str>,
    ifsc: Option<&str>,
    holder_name: Option<&str>,
) -> String {
    [bank_name, account_no, ifsc, holder_name]
        .into_iter()
        .filter_map(|s| s.map(str::trim).filter(|t| !t.is_empty()))
        .collect::<Vec<_>>()
        .join(" · ")
}

pub fn has_bank_details(bank_name: Option<&str>, account_no: Option<&str>, ifsc: Option<&str>) -> bool {
    [bank_name, account_no, ifsc]
        .into_iter()
        .any(|s| s.map(str::trim).is_some_and(|t| !t.is_empty()))
}

pub fn parse_voucher_links(raw: Option<&str>) -> Vec<LinkedVoucherRef> {
    let text = raw.unwrap_or("").trim();
    if text.is_empty() {
        return Vec::new();
    }
    if text.starts_with('[') || text.starts_with('{') {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(text) {
            let arr = if parsed.is_array() {
                parsed.as_array().cloned().unwrap_or_default()
            } else {
                Vec::new()
            };
            return arr
                .into_iter()
                .filter_map(|row| {
                    let id = row.get("id").and_then(|v| v.as_str()).unwrap_or("").trim();
                    let voucher_no = row
                        .get("voucher_no")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .trim();
                    if voucher_no.is_empty() {
                        None
                    } else {
                        Some(LinkedVoucherRef {
                            id: id.to_string(),
                            voucher_no: voucher_no.to_string(),
                        })
                    }
                })
                .collect();
        }
    }
    text.split('\n')
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            if let Some((id, no)) = line.split_once('\t') {
                let voucher_no = no.trim();
                if voucher_no.is_empty() {
                    None
                } else {
                    Some(LinkedVoucherRef {
                        id: id.trim().to_string(),
                        voucher_no: voucher_no.to_string(),
                    })
                }
            } else {
                Some(LinkedVoucherRef {
                    id: String::new(),
                    voucher_no: line.to_string(),
                })
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_advance_until_goods_received() {
        let (cls, missing) = classify_purchase_payment(false, true, 100_000, 0, 10_000);
        assert_eq!(cls, PaymentClass::Advance);
        assert!(missing);
        let (cls, _) = classify_purchase_payment(true, false, 100_000, 0, 100_000);
        assert_eq!(cls, PaymentClass::FullyPaid);
        let (cls, _) = classify_purchase_payment(true, false, 100_000, 10_000, 10_000);
        assert_eq!(cls, PaymentClass::Partial);
    }

    #[test]
    fn posted_pay_numbers_are_not_legacy() {
        assert!(!is_legacy_pay_number(Some("PAY-0001")));
        assert!(is_legacy_pay_number(Some("PAY:1")));
        assert!(is_legacy_pay_number(Some("")));
        assert!(is_pay_voucher_number(Some("PAY-0001")));
        assert!(!is_pay_voucher_number(Some("PUR-0001-01")));
    }

    #[test]
    fn child_serial_is_not_pay_seq() {
        assert!(is_child_pay_number(Some("PUR-0001-01"), Some("PUR-0001")));
        assert_eq!(parse_payment_seq(Some("PUR-0001-01"), Some("PUR-0001")), 1);
        assert_eq!(parse_pay_voucher_seq(Some("PUR-0001-01")), 0);
    }

    #[test]
    fn merge_requires_same_vendor() {
        let rows = [
            MergeVoucherHint {
                id: "a".into(),
                vendor_key: "name:vend_02".into(),
                po_id: None,
                source: None,
                reversed: false,
            },
            MergeVoucherHint {
                id: "b".into(),
                vendor_key: "name:other".into(),
                po_id: None,
                source: None,
                reversed: false,
            },
        ];
        assert_eq!(
            merge_blocked_reason(&rows, None, None).as_deref(),
            Some("Same vendor required.")
        );
    }
}
