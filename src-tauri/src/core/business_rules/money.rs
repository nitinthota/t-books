//! Paise in store, rupees on screen. Matches loopbook `money.ts`.

/// JavaScript `Math.round`: half toward +∞.
pub fn js_round(n: f64) -> i64 {
    if !n.is_finite() {
        return 0;
    }
    (n + 0.5).floor() as i64
}

pub fn rupees_to_paise(rupees: f64) -> i64 {
    if !rupees.is_finite() {
        return 0;
    }
    js_round(rupees * 100.0)
}

pub fn paise_to_rupees(paise: i64) -> f64 {
    paise as f64 / 100.0
}

pub fn format_inr(paise: i64) -> String {
    let sign = if paise < 0 { "-" } else { "" };
    let abs = paise.unsigned_abs();
    let rs = abs / 100;
    let ps = abs % 100;
    format!("{sign}₹{}.{ps:02}", group_en_in(rs))
}

pub fn format_rupees(rupees: Option<f64>) -> String {
    match rupees {
        Some(n) if n.is_finite() => {
            let sign = if n < 0.0 { "-" } else { "" };
            let abs = n.abs();
            let grouped = group_en_in_float(abs);
            format!("{sign}₹{grouped}")
        }
        _ => "—".into(),
    }
}

pub fn format_qty(qty: f64) -> String {
    if !qty.is_finite() {
        return "—".into();
    }
    let sign = if qty < 0.0 { "-" } else { "" };
    let abs = qty.abs();
    let trunc = abs.trunc() as u64;
    let frac = abs - abs.trunc();
    if frac.abs() < 1e-12 {
        return format!("{sign}{}", group_en_in(trunc));
    }
    let mut decimals = format!("{frac:.3}");
    if let Some(stripped) = decimals.strip_prefix("0.") {
        decimals = stripped.to_string();
    } else {
        decimals = decimals.trim_start_matches("0.").to_string();
    }
    let decimals = decimals.trim_end_matches('0');
    if decimals.is_empty() {
        format!("{sign}{}", group_en_in(trunc))
    } else {
        format!("{sign}{}.{}", group_en_in(trunc), decimals)
    }
}

fn group_en_in(n: u64) -> String {
    let s = n.to_string();
    if s.len() <= 3 {
        return s;
    }
    let (head, tail) = s.split_at(s.len() - 3);
    let mut grouped = String::new();
    let chars: Vec<char> = head.chars().collect();
    let rem = chars.len() % 2;
    if rem > 0 {
        grouped.extend(&chars[..rem]);
        if chars.len() > rem {
            grouped.push(',');
        }
    }
    for (i, chunk) in chars[rem..].chunks(2).enumerate() {
        if i > 0 {
            grouped.push(',');
        }
        grouped.extend(chunk);
    }
    grouped.push(',');
    grouped.push_str(tail);
    grouped
}

fn group_en_in_float(n: f64) -> String {
    let trunc = n.trunc() as u64;
    let frac = n - n.trunc();
    if frac.abs() < 1e-9 {
        group_en_in(trunc)
    } else {
        format!("{}.{}", group_en_in(trunc), format!("{:.2}", frac).trim_start_matches("0."))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rupees_to_paise_rounds() {
        assert_eq!(rupees_to_paise(1.0), 100);
        assert_eq!(rupees_to_paise(21.494), 2149);
        assert_eq!(rupees_to_paise(f64::NAN), 0);
        assert_eq!(rupees_to_paise(f64::INFINITY), 0);
    }

    #[test]
    fn paise_format() {
        assert_eq!(paise_to_rupees(21494), 214.94);
        assert_eq!(format_inr(0), "₹0.00");
        let s = format_inr(10_000_000);
        assert!(s.contains("₹1,00,000"), "{s}");
        assert_eq!(format_qty(f64::NAN), "—");
        assert_eq!(format_qty(12.5), "12.5");
        assert_eq!(format_qty(1000.0), "1,000");
    }
}
