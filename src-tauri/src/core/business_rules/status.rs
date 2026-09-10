//! Status from amounts. Matches loopbook `paymentStatus` / `taxFlagFor`.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaxFlag {
    Missing,
    NaNeedsComment,
    NaOk,
    Ok,
}

impl TaxFlag {
    pub fn as_str(self) -> &'static str {
        match self {
            TaxFlag::Missing => "missing",
            TaxFlag::NaNeedsComment => "na_needs_comment",
            TaxFlag::NaOk => "na_ok",
            TaxFlag::Ok => "ok",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusTone {
    Credit,
    Debit,
    Muted,
}

pub fn is_na_tax_inv(raw: &str) -> bool {
    matches!(raw.trim().to_ascii_lowercase().as_str(), "na" | "n/a")
}

/// Invoice / paid / tax-invoice → register status. Do not restyle these strings.
pub fn payment_status(invoice: f64, payment: f64, tax_inv: &str) -> String {
    let out = super::money::js_round(invoice - payment);
    if out < 0 {
        return "Advance Payment".into();
    }
    if tax_inv.trim().is_empty() {
        return "Missing Tax Invoice".into();
    }
    if payment == 0.0 {
        return "Pending Payment".into();
    }
    if out == 0 {
        return "Full Payment".into();
    }
    "Partial Payment".into()
}

pub fn tax_flag_for(tax_inv: &str, comments: &str) -> TaxFlag {
    let t = tax_inv.trim();
    if t.is_empty() {
        return TaxFlag::Missing;
    }
    if is_na_tax_inv(t) {
        if comments.trim().is_empty() {
            TaxFlag::NaNeedsComment
        } else {
            TaxFlag::NaOk
        }
    } else {
        TaxFlag::Ok
    }
}

pub fn status_tone(status: &str) -> StatusTone {
    let s = status.to_ascii_lowercase();
    if s.contains("full") || s.contains("advance") {
        StatusTone::Credit
    } else if s.contains("pending") || s.contains("missing") || s.contains("partial") {
        StatusTone::Debit
    } else {
        StatusTone::Muted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payment_status_matches_loopbook() {
        assert_eq!(payment_status(100.0, 0.0, "INV"), "Pending Payment");
        assert_eq!(payment_status(100.0, 40.0, "INV"), "Partial Payment");
        assert_eq!(payment_status(100.0, 100.0, "INV"), "Full Payment");
        assert_eq!(payment_status(100.0, 120.0, "INV"), "Advance Payment");
        assert_eq!(payment_status(100.0, 0.0, ""), "Missing Tax Invoice");
    }

    #[test]
    fn tax_flags_match_loopbook() {
        assert_eq!(tax_flag_for("", ""), TaxFlag::Missing);
        assert_eq!(tax_flag_for("NA", ""), TaxFlag::NaNeedsComment);
        assert_eq!(tax_flag_for("NA", "cash purchase"), TaxFlag::NaOk);
        assert_eq!(tax_flag_for("INV-1", ""), TaxFlag::Ok);
    }
}
