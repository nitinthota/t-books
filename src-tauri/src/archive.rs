//! Park a deleted row for 30 days. Live lists hide it. Restore puts it back.
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::db::{data_dir, LocalBooks};
use crate::{BooksError, Result};

pub const HOLD_DAYS: i64 = 30;

fn wipe_hive(books: &LocalBooks, kind: &str, key: &str) {
    let (hive_kind, hive_key) = crate::hive_erase::kind_for_parked(kind, key);
    crate::hive_erase::erase_hive_row_quiet(books, hive_kind, &hive_key);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchivedRow {
    pub id: i64,
    pub kind: String,
    pub key: String,
    pub label: String,
    pub deleted_at: String,
    pub purge_after: String,
    pub days_left: i64,
}
