//! T Books roles stay owner / admin / operator.
//! Loopbook `viewer` (and anything else) is fail-closed on mutate.

pub fn normalize_role(role: &str) -> String {
    role.trim().to_ascii_lowercase()
}

/// Books mutate: owner, admin, operator. Viewer / unknown / empty → no.
pub fn can_mutate(role: &str) -> bool {
    matches!(normalize_role(role).as_str(), "owner" | "admin" | "operator")
}

pub fn can_write(role: &str) -> bool {
    can_mutate(role)
}

pub fn can_refresh(role: &str) -> bool {
    matches!(normalize_role(role).as_str(), "owner" | "admin")
}

pub fn can_open_access(role: &str) -> bool {
    normalize_role(role) == "owner"
}

pub fn require_write(role: &str) -> Result<(), String> {
    if can_write(role) {
        Ok(())
    } else {
        Err("You cannot change the books with this role.".into())
    }
}

pub fn require_admin(role: &str) -> Result<(), String> {
    if matches!(normalize_role(role).as_str(), "owner" | "admin") {
        Ok(())
    } else {
        Err("You cannot refresh or administer with this role.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewer_is_fail_closed() {
        assert!(!can_mutate("viewer"));
        assert!(!can_mutate(""));
        assert!(!can_mutate("guest"));
        assert!(can_mutate("owner"));
        assert!(can_mutate("admin"));
        assert!(can_mutate("operator"));
        assert!(require_write("viewer").is_err());
        assert!(require_write("operator").is_ok());
        assert!(!can_open_access("admin"));
        assert!(can_open_access("owner"));
    }
}
