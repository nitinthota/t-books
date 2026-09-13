use rusqlite::params;

use crate::access::{access_denial_message, session_role};
use crate::db::LocalBooks;
use crate::passwords::{hash_password, validate_new_password, verify_password};
use crate::{is_valid_email_shape, normalize_email, AuthInspect, BooksError, Result, Session};

struct UserRow {
    email: String,
    password_hash: String,
}

fn get_user(books: &LocalBooks, email: &str) -> Result<Option<UserRow>> {
    let mut stmt = books.conn().prepare(
        "SELECT email, password_hash FROM users_local WHERE email = ?1 COLLATE NOCASE LIMIT 1",
    )?;
    let mut rows = stmt.query(params![email])?;
    match rows.next()? {
        None => Ok(None),
        Some(row) => Ok(Some(UserRow {
            email: row.get(0)?,
            password_hash: row.get(1)?,
        })),
    }
}

pub fn inspect_email(books: &LocalBooks, raw_email: &str) -> AuthInspect {
    let email = normalize_email(raw_email);
    if !is_valid_email_shape(&email) {
        return AuthInspect::Denied {
            message: "Enter a valid email.".into(),
        };
    }
    match access_denial_message(books, &email) {
        Err(err) => {
            return AuthInspect::Denied {
                message: err.to_string(),
            }
        }
        Ok(Some(message)) => return AuthInspect::Denied { message },
        Ok(None) => {}
    }
    match get_user(books, &email).ok().flatten() {
        Some(_) => AuthInspect::Password { email },
        None => AuthInspect::SetPassword { email },
    }
}

fn session_for(books: &LocalBooks, email: &str) -> Session {
    Session {
        email: email.to_string(),
        role: session_role(books, email),
    }
}

pub fn sign_in(books: &LocalBooks, raw_email: &str, password: &str) -> Result<Session> {
    match inspect_email(books, raw_email) {
        AuthInspect::Denied { message } => Err(BooksError::from(message)),
        AuthInspect::SetPassword { .. } => {
            Err(BooksError::from("Set a password for this PC first."))
        }
        AuthInspect::Password { email } => {
            let user = get_user(books, &email)?
                .ok_or_else(|| BooksError::from("Could not open that account on this PC."))?;
            if !verify_password(password, Some(user.password_hash.as_str())) {
                return Err(BooksError::from("Invalid credentials."));
            }
            Ok(session_for(books, &user.email))
        }
    }
}

pub fn set_first_password(
    books: &LocalBooks,
    raw_email: &str,
    password: &str,
    confirm: &str,
) -> Result<Session> {
    match inspect_email(books, raw_email) {
        AuthInspect::Denied { message } => return Err(BooksError::from(message)),
        AuthInspect::Password { .. } => {
            return Err(BooksError::from(
                "This PC already has a password for that email.",
            ));
        }
        AuthInspect::SetPassword { email } => {
            if password != confirm {
                return Err(BooksError::from("Those passwords do not match."));
            }
            let next = validate_new_password(password, Some(&email))?;
            let hash = hash_password(&next)?;
            books.conn().execute(
                r#"
                INSERT INTO users_local (email, password_hash, created_at)
                VALUES (?1, ?2, datetime('now'))
                "#,
                params![email, hash],
            )?;
            Ok(session_for(books, &email))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::access::{apply_access_rows, AccessRow};
    use crate::{open_memory, OWNER_EMAIL};

    fn cache_row(name: &str, email: &str, role: &str, active: &str) -> AccessRow {
        AccessRow {
            name: name.into(),
            email: email.into(),
            role: role.into(),
            active: active.into(),
            last_synced: String::new(),
            account_type: role.to_string(),
        }
    }

    #[test]
    fn owner_missing_row_is_first_setup() {
        let books = open_memory().unwrap();
        assert!(get_user(&books, OWNER_EMAIL).unwrap().is_none());
        match inspect_email(&books, "thotanitin123@gmail.com") {
            AuthInspect::SetPassword { email } => assert_eq!(email, "thotanitin123@gmail.com"),
            other => panic!("expected set-password, got {other:?}"),
        }
    }

    #[test]
    fn owner_sets_password_then_signs_in() {
        let books = open_memory().unwrap();
        let session = set_first_password(
            &books,
            "thotanitin123@gmail.com",
            "ledger-key-92",
            "ledger-key-92",
        )
        .unwrap();
        assert_eq!(session.email, "thotanitin123@gmail.com");
        assert_eq!(session.role, "owner");
        let again = sign_in(&books, "thotanitin123@gmail.com", "ledger-key-92").unwrap();
        assert_eq!(again.email, session.email);
        assert!(sign_in(&books, "thotanitin123@gmail.com", "wrong-pass").is_err());
    }

    #[test]
    fn unknown_email_is_denied_when_cache_empty() {
        let books = open_memory().unwrap();
        match inspect_email(&books, "operator@example.com") {
            AuthInspect::Denied { message } => assert!(message.contains("Access list")),
            other => panic!("expected denied, got {other:?}"),
        }
    }

    #[test]
    fn cached_operator_can_set_password_and_sign_in() {
        let mut books = open_memory().unwrap();
        apply_access_rows(
            &mut books,
            &[cache_row("Ada", "ada@example.com", "operator", "Yes")],
        )
        .unwrap();
        match inspect_email(&books, "ada@example.com") {
            AuthInspect::SetPassword { email } => assert_eq!(email, "ada@example.com"),
            other => panic!("expected set-password, got {other:?}"),
        }
        let session =
            set_first_password(&books, "ada@example.com", "ledger-key-92", "ledger-key-92")
                .unwrap();
        assert_eq!(session.role, "operator");
        let again = sign_in(&books, "ada@example.com", "ledger-key-92").unwrap();
        assert_eq!(again.role, "operator");
    }

    #[test]
    fn cached_admin_gets_admin_role() {
        let mut books = open_memory().unwrap();
        apply_access_rows(
            &mut books,
            &[cache_row("Ada", "ada@example.com", "admin", "Yes")],
        )
        .unwrap();
        let session =
            set_first_password(&books, "ada@example.com", "ledger-key-92", "ledger-key-92")
                .unwrap();
        assert_eq!(session.role, "admin");
    }

    #[test]
    fn inactive_cached_user_is_denied() {
        let mut books = open_memory().unwrap();
        apply_access_rows(
            &mut books,
            &[cache_row("Ada", "ada@example.com", "operator", "No")],
        )
        .unwrap();
        match inspect_email(&books, "ada@example.com") {
            AuthInspect::Denied { message } => assert!(message.contains("not active")),
            other => panic!("expected denied, got {other:?}"),
        }
    }

    #[test]
    fn local_password_without_access_is_denied() {
        let books = open_memory().unwrap();
        books
            .conn()
            .execute(
                "INSERT INTO users_local (email, password_hash, created_at) VALUES (?1, ?2, datetime('now'))",
                params!["ghost@example.com", "not-a-real-hash"],
            )
            .unwrap();
        match inspect_email(&books, "ghost@example.com") {
            AuthInspect::Denied { message } => assert!(message.contains("Access list")),
            other => panic!("expected denied, got {other:?}"),
        }
    }
}
