//! Book dates may be 2025-04-12 or 12.04.2025.

pub fn book_date_to_iso(raw: &str) -> Option<String> {
    let s = raw.trim();
    let bytes = s.as_bytes();
    if s.len() >= 10 && bytes.get(4) == Some(&b'-') && bytes.get(7) == Some(&b'-') {
        return Some(s[..10].to_string());
    }
    let parts: Vec<&str> = s.split(|c| c == '.' || c == '/' || c == '-').collect();
    if parts.len() >= 3 {
        let d = parts[0];
        let m = parts[1];
        let y = parts[2];
        if y.len() == 4 && !d.is_empty() && !m.is_empty() {
            return Some(format!("{:0>4}-{:0>2}-{:0>2}", y, m, d));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dmy_dot() {
        assert_eq!(book_date_to_iso("12.04.2025").as_deref(), Some("2025-04-12"));
    }
}
