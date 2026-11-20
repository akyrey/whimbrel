//! Whimbrel's local data (FR-8): the history of sent requests, in SQLite in the app's data
//! folder, never in a collection. Callers mask secret values before recording (D-H12).

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use wbl_core::Request;
use wbl_http::Response;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("The history database failed: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("A history entry could not be read: {0}")]
    Data(#[from] serde_json::Error),
    #[error("The history is unusable; restart the app")]
    Poisoned,
}

/// How a send ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Outcome {
    Completed { response: Box<Response> },
    Failed { code: String, message: String },
}

/// What is recorded, already masked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewEntry {
    pub sent_at_ms: i64,
    /// The collection's folder, if the request belongs to one.
    pub collection: Option<String>,
    pub name: String,
    /// As sent (variables resolved, secrets masked).
    pub request: Request,
    pub outcome: Outcome,
}

/// A line of the history list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct HistorySummary {
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub id: i64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub sent_at_ms: i64,
    pub collection: Option<String>,
    pub name: String,
    pub method: String,
    pub url: String,
    pub status: Option<u16>,
    pub error_code: Option<String>,
    #[cfg_attr(feature = "ts", ts(type = "number | null"))]
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub id: i64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub sent_at_ms: i64,
    pub collection: Option<String>,
    pub name: String,
    pub request: Request,
    pub outcome: Outcome,
}

/// Entries kept; older ones are removed as new ones come.
const DEFAULT_LIMIT: i64 = 1000;

/// Forward-only migrations, by `PRAGMA user_version` (as in Dunlin).
const MIGRATIONS: [&str; 1] = ["CREATE TABLE history (
        id INTEGER PRIMARY KEY,
        sent_at_ms INTEGER NOT NULL,
        collection TEXT,
        name TEXT NOT NULL,
        method TEXT NOT NULL,
        url TEXT NOT NULL,
        status INTEGER,
        error_code TEXT,
        duration_ms INTEGER,
        request TEXT NOT NULL,
        outcome TEXT NOT NULL
    );
    CREATE INDEX history_sent ON history (sent_at_ms DESC, id DESC);"];

pub struct History {
    connection: Mutex<Connection>,
    limit: i64,
}

impl History {
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        let connection = Connection::open(path)?;
        migrate(&connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
            limit: DEFAULT_LIMIT,
        })
    }

    /// Keeps at most `limit` entries.
    pub fn with_limit(self, limit: i64) -> Self {
        Self { limit, ..self }
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, StoreError> {
        self.connection.lock().map_err(|_| StoreError::Poisoned)
    }

    pub fn record(&self, entry: &NewEntry) -> Result<i64, StoreError> {
        let (status, error_code, duration) = match &entry.outcome {
            Outcome::Completed { response } => {
                (Some(response.status), None, Some(response.timing.total_ms))
            }
            Outcome::Failed { code, .. } => (None, Some(code.clone()), None),
        };
        let connection = self.lock()?;
        connection.execute(
            "INSERT INTO history (sent_at_ms, collection, name, method, url, status, error_code,
                duration_ms, request, outcome) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                entry.sent_at_ms,
                entry.collection,
                entry.name,
                entry.request.method,
                entry.request.url,
                status,
                error_code,
                duration.and_then(|d| i64::try_from(d).ok()),
                serde_json::to_string(&entry.request)?,
                serde_json::to_string(&entry.outcome)?,
            ],
        )?;
        let id = connection.last_insert_rowid();
        connection.execute(
            "DELETE FROM history WHERE id NOT IN
                (SELECT id FROM history ORDER BY sent_at_ms DESC, id DESC LIMIT ?1)",
            params![self.limit],
        )?;
        Ok(id)
    }

    /// Newest first; `query` matches name, URL, method or status, ignoring case.
    pub fn list(
        &self,
        query: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<HistorySummary>, StoreError> {
        let pattern = format!("%{}%", escape_like(query));
        let connection = self.lock()?;
        let mut statement = connection.prepare(
            "SELECT id, sent_at_ms, collection, name, method, url, status, error_code, duration_ms
             FROM history
             WHERE name LIKE ?1 ESCAPE '\\' OR url LIKE ?1 ESCAPE '\\' OR method LIKE ?1 ESCAPE '\\'
                OR CAST(status AS TEXT) LIKE ?1 ESCAPE '\\'
             ORDER BY sent_at_ms DESC, id DESC LIMIT ?2 OFFSET ?3",
        )?;
        let rows = statement.query_map(params![pattern, limit, offset], |row| {
            Ok(HistorySummary {
                id: row.get(0)?,
                sent_at_ms: row.get(1)?,
                collection: row.get(2)?,
                name: row.get(3)?,
                method: row.get(4)?,
                url: row.get(5)?,
                status: row.get(6)?,
                error_code: row.get(7)?,
                duration_ms: row
                    .get::<_, Option<i64>>(8)?
                    .and_then(|d| u64::try_from(d).ok()),
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn get(&self, id: i64) -> Result<Option<HistoryEntry>, StoreError> {
        let connection = self.lock()?;
        let row = connection
            .query_row(
                "SELECT sent_at_ms, collection, name, request, outcome FROM history WHERE id = ?1",
                params![id],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                },
            )
            .optional()?;
        let Some((sent_at_ms, collection, name, request, outcome)) = row else {
            return Ok(None);
        };
        Ok(Some(HistoryEntry {
            id,
            sent_at_ms,
            collection,
            name,
            request: serde_json::from_str(&request)?,
            outcome: serde_json::from_str(&outcome)?,
        }))
    }

    pub fn clear(&self) -> Result<(), StoreError> {
        self.lock()?.execute("DELETE FROM history", [])?;
        Ok(())
    }
}

fn escape_like(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn migrate(connection: &Connection) -> Result<(), StoreError> {
    let version: i64 = connection.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    let done = usize::try_from(version).unwrap_or(0);
    for (index, sql) in MIGRATIONS.iter().enumerate().skip(done) {
        connection.execute_batch(sql)?;
        connection.execute_batch(&format!("PRAGMA user_version = {}", index + 1))?;
    }
    Ok(())
}
