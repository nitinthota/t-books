//! ZEF event log + AMOT decision desk.
//!
//! Screens never call Google. AMOT is the only writer.

use rusqlite::Connection;

use crate::Result;

pub const CONFLICT_SUFFIX: &str = "was just updated by another user. Reload and submit again.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Save,
    Submit,
    Refresh,
    DiscardLocal,
    KeepLocal,
}

impl Intent {
    pub fn as_str(self) -> &'static str {
        match self {
            Intent::Save => "save",
            Intent::Submit => "submit",
            Intent::Refresh => "refresh",
            Intent::DiscardLocal => "discard_local",
            Intent::KeepLocal => "keep_local",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub intent: Intent,
    pub book: String,
    pub key: String,
    pub actor: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Facts {
    pub online: bool,
    pub dirty: bool,
    pub posted_forever: bool,
    pub can_submit: bool,
    pub known_fp: String,
    pub hive_fp: String,
    pub hive_exists: bool,
    pub dirty_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Park,
    Post,
    Refuse { message: String },
    StopRefresh { dirty_keys: Vec<String> },
    ApplyRefresh { skip_keys: Vec<String> },
    KeepLocal,
    DiscardLocal,
}

impl Decision {
    pub fn as_str(&self) -> &'static str {
        match self {
            Decision::Park => "park",
            Decision::Post => "post",
            Decision::Refuse { .. } => "refuse",
            Decision::StopRefresh { .. } => "stop_refresh",
            Decision::ApplyRefresh { .. } => "apply_refresh",
            Decision::KeepLocal => "keep_local",
            Decision::DiscardLocal => "discard_local",
        }
    }
}

pub fn conflict_message(key: &str) -> String {
    format!("{key} {CONFLICT_SUFFIX}")
}

/// Posted child rows such as PUR-0001-01 are never rewritten.
pub fn posted_forever_key(key: &str) -> bool {
    let k = key.trim().to_ascii_uppercase();
    k.starts_with("PUR-") && k.contains("-01")
}

pub fn decide(event: &Event, facts: &Facts) -> Decision {
    match event.intent {
        Intent::Save => Decision::Park,
        Intent::Submit => decide_submit(event, facts),
        Intent::Refresh => decide_refresh(facts),
        Intent::KeepLocal => Decision::KeepLocal,
        Intent::DiscardLocal => Decision::DiscardLocal,
    }
}

fn decide_submit(event: &Event, facts: &Facts) -> Decision {
    if facts.posted_forever || posted_forever_key(&event.key) {
        return Decision::Refuse {
            message: format!("{} is posted and is never rewritten.", event.key),
        };
    }
    if !facts.can_submit {
        return Decision::Refuse {
            message: "You cannot Submit.".into(),
        };
    }
    if !facts.online {
        return Decision::Refuse {
            message: "Offline. Save stays on this PC. Submit when online.".into(),
        };
    }
    if facts.hive_exists && facts.known_fp != facts.hive_fp {
        return Decision::Refuse {
            message: conflict_message(&event.key),
        };
    }
    Decision::Post
}

fn decide_refresh(facts: &Facts) -> Decision {
    if !facts.online {
        return Decision::Refuse {
            message: "Offline. Refresh paused. Local books stay as they are.".into(),
        };
    }
    if !facts.dirty_keys.is_empty() {
        return Decision::StopRefresh {
            dirty_keys: facts.dirty_keys.clone(),
        };
    }
    Decision::ApplyRefresh { skip_keys: vec![] }
}

pub fn ensure_zef_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS zef_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            ts TEXT NOT NULL,
            actor TEXT NOT NULL DEFAULT '',
            intent TEXT NOT NULL,
            book TEXT NOT NULL,
            key TEXT NOT NULL,
            payload TEXT NOT NULL DEFAULT '{}',
            decision TEXT NOT NULL DEFAULT '',
            detail TEXT NOT NULL DEFAULT ''
        );
        CREATE INDEX IF NOT EXISTS idx_zef_events_book_key ON zef_events(book, key);",
    )?;
    Ok(())
}

pub fn append_event(
    conn: &Connection,
    event: &Event,
    decision: &Decision,
    detail: &str,
) -> Result<i64> {
    ensure_zef_table(conn)?;
    conn.execute(
        "INSERT INTO zef_events (ts, actor, intent, book, key, payload, decision, detail)
         VALUES (datetime('now'), ?1, ?2, ?3, ?4, '{}', ?5, ?6)",
        rusqlite::params![
            event.actor,
            event.intent.as_str(),
            event.book,
            event.key,
            decision.as_str(),
            detail
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn list_events_for_key(conn: &Connection, book: &str, key: &str) -> Result<Vec<(String, String)>> {
    ensure_zef_table(conn)?;
    let mut stmt = conn.prepare(
        "SELECT intent, decision FROM zef_events WHERE book = ?1 AND key = ?2 ORDER BY id",
    )?;
    let rows = stmt.query_map(rusqlite::params![book, key], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub fn record(
    conn: &Connection,
    intent: Intent,
    book: &str,
    key: &str,
    facts: &Facts,
    actor: &str,
) -> Result<Decision> {
    let event = Event {
        intent,
        book: book.to_string(),
        key: key.to_string(),
        actor: actor.to_string(),
    };
    let decision = decide(&event, facts);
    append_event(conn, &event, &decision, decision.as_str())?;
    Ok(decision)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_memory;

    fn ev(book: &str, intent: Intent, key: &str) -> Event {
        Event {
            intent,
            book: book.into(),
            key: key.into(),
            actor: "thotanitin123@gmail.com".into(),
        }
    }

    fn base() -> Facts {
        Facts {
            online: true,
            dirty: true,
            posted_forever: false,
            can_submit: true,
            known_fp: "aaa".into(),
            hive_fp: "aaa".into(),
            hive_exists: true,
            dirty_keys: vec![],
        }
    }

    #[test]
    fn save_always_parks() {
        let d = decide(&ev("voucher", Intent::Save, "20"), &Facts { online: false, ..base() });
        assert_eq!(d, Decision::Park);
    }

    #[test]
    fn purchase_save_parks() {
        assert_eq!(decide(&ev("purchase", Intent::Save, "PUR-0001"), &base()), Decision::Park);
    }

    #[test]
    fn purchase_child_never_rewritten() {
        let d = decide(&ev("purchase", Intent::Submit, "PUR-0001-01"), &base());
        match d {
            Decision::Refuse { message } => assert!(message.contains("never rewritten")),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn pur_and_pay_do_not_block() {
        let bill = decide(&ev("purchase", Intent::Submit, "PUR-0004"), &base());
        let pay = decide(&ev("payment", Intent::Submit, "PAY-00010"), &base());
        assert_eq!(bill, Decision::Post);
        assert_eq!(pay, Decision::Post);
    }

    #[test]
    fn submit_cas_mismatch_uses_exact_message() {
        let d = decide(
            &ev("purchase", Intent::Submit, "PUR-0004"),
            &Facts {
                hive_fp: "bbb".into(),
                ..base()
            },
        );
        assert_eq!(
            d,
            Decision::Refuse {
                message: "PUR-0004 was just updated by another user. Reload and submit again.".into()
            }
        );
    }

    #[test]
    fn refresh_stops_on_dirty_purchase_only() {
        let d = decide(
            &ev("purchase", Intent::Refresh, "*"),
            &Facts {
                dirty_keys: vec!["PUR-0004".into()],
                ..base()
            },
        );
        assert_eq!(
            d,
            Decision::StopRefresh {
                dirty_keys: vec!["PUR-0004".into()]
            }
        );
    }

    #[test]
    fn zef_appends_never_updates() {
        let books = open_memory().unwrap();
        let e = ev("purchase", Intent::Save, "PUR-0001");
        let d = decide(&e, &base());
        append_event(books.conn(), &e, &d, "first").unwrap();
        append_event(books.conn(), &e, &d, "second").unwrap();
        let rows = list_events_for_key(books.conn(), "purchase", "PUR-0001").unwrap();
        assert_eq!(rows.len(), 2);
    }
}
