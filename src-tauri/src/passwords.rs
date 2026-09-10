use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Algorithm, Argon2, Params, Version,
};

use crate::{BooksError, MIN_PASSWORD_LENGTH, Result};

pub fn validate_new_password(password: &str, email: Option<&str>) -> Result<String> {
    let value = password.trim();
    if value.len() < MIN_PASSWORD_LENGTH {
        return Err(BooksError::from(format!(
            "Password must be at least {MIN_PASSWORD_LENGTH} characters."
        )));
    }
    if let Some(email) = email {
        if value.eq_ignore_ascii_case(email.trim()) {
            return Err(BooksError::from("Password cannot be the same as the email."));
        }
    }
    Ok(value.to_string())
}

fn hasher() -> Result<Argon2<'static>> {
    let params = Params::new(19_456, 2, 1, Some(32))
        .map_err(|_| BooksError::from("Could not configure password hashing on this PC."))?;
    Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
}

fn random_salt() -> Result<SaltString> {
    let mut bytes = [0u8; 16];
    getrandom::getrandom(&mut bytes)
        .map_err(|_| BooksError::from("Could not generate a salt on this PC."))?;
    SaltString::encode_b64(&bytes)
        .map_err(|_| BooksError::from("Could not encode a salt on this PC."))
}

/// Argon2id PHC string. Never log this value.
pub fn hash_password(plain: &str) -> Result<String> {
    let salt = random_salt()?;
    let hash = hasher()?
        .hash_password(plain.as_bytes(), &salt)
        .map_err(|_| BooksError::from("Could not hash the password on this PC."))?;
    Ok(hash.to_string())
}

/// Constant-time verify via the argon2 crate (password-hash).
pub fn verify_password(plain: &str, stored: Option<&str>) -> bool {
    let Some(stored) = stored else {
        return false;
    };
    let Ok(parsed) = PasswordHash::new(stored) else {
        return false;
    };
    let Ok(engine) = hasher() else {
        return false;
    };
    engine.verify_password(plain.as_bytes(), &parsed).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let hash = hash_password("ledger-key-92").unwrap();
        assert!(hash.starts_with("$argon2id$"));
        assert!(verify_password("ledger-key-92", Some(&hash)));
        assert!(!verify_password("wrong-pass", Some(&hash)));
        assert!(!verify_password("ledger-key-92", None));
    }

    #[test]
    fn rejects_short() {
        assert!(validate_new_password("short", None).is_err());
    }
}
