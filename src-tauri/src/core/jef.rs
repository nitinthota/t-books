//! JEF — local picture of the books.
//! Built from zef_events on this PC. Never calls Google.

use rusqlite::Connection;

use crate::core::pipeline::ensure_zef_table;
use crate::Result;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JefSlice {
    pub book: String,
    pub key: String,
    pub last_intent: String,
    pub last_decision: String,
    pub last_at: String,
    pub actor: String,
}

/// Last event per book + key. Empty book = every book.
pub fn project_book(conn: &Connection, book: Option<&str>) -> Result<Vec<JefSlice>> {
    ensure_zef_table(conn)?;
    let filter = book.unwrap_or("").trim().to_string();
    let mut stmt = conn.prepare(
        "SELECT book, key, intent, decision, ts, actor
         FROM zef_events
         WHERE id IN (SELECT MAX(id) FROM zef_events GROUP BY book, key)
           AND (?1 = '' OR book = ?1)
         ORDER BY book, key",
    )?;
    let rows = stmt.query_map(rusqlite::params![filter], |row| {
        Ok(JefSlice {
            book: row.get(0)?,
            key: row.get(1)?,
            last_intent: row.get(2)?,
            last_decision: row.get(3)?,
            last_at: row.get(4)?,
            actor: row.get(5)?,
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// Timeline for one key. Oldest first. Never rewritten.
pub fn timeline(conn: &Connection, book: &str, key: &str) -> Result<Vec<JefSlice>> {
    ensure_zef_table(conn)?;
    let mut stmt = conn.prepare(
        "SELECT book, key, intent, decision, ts, actor
         FROM zef_events
         WHERE book = ?1 AND key = ?2
         ORDER BY id",
    )?;
    let rows = stmt.query_map(rusqlite::params![book, key], |row| {
        Ok(JefSlice {
            book: row.get(0)?,
            key: row.get(1)?,
            last_intent: row.get(2)?,
            last_decision: row.get(3)?,
            last_at: row.get(4)?,
            actor: row.get(5)?,
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::pipeline::park_save;
    use crate::db::open_memory;

    #[test]
    fn last_save_is_the_picture() {
        let books = open_memory().unwrap();
        park_save(books.conn(), "voucher", "20", "a").unwrap();
        park_save(books.conn(), "voucher", "20", "b").unwrap();
        park_save(books.conn(), "purchase", "PUR-0004", "a").unwrap();
        let all = project_book(books.conn(), None).unwrap();
        assert_eq!(all.len(), 2);
        let v = project_book(books.conn(), Some("voucher")).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].key, "20");
        assert_eq!(v[0].last_intent, "save");
        assert_eq!(v[0].last_decision, "park");
        assert_eq!(v[0].actor, "b");
        let line = timeline(books.conn(), "voucher", "20").unwrap();
        assert_eq!(line.len(), 2);
    }
}
