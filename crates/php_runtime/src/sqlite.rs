//! Request-local SQLite3 extension state.

use crate::{ArrayKey, PhpArray, PhpString, Value};
use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags};
use std::collections::HashMap;
use std::fmt;
use std::path::Path;

/// `SQLite3Result::fetchArray()` associative columns.
pub const SQLITE3_ASSOC: i64 = 1;
/// `SQLite3Result::fetchArray()` numeric columns.
pub const SQLITE3_NUM: i64 = 2;
/// `SQLite3Result::fetchArray()` associative and numeric columns.
pub const SQLITE3_BOTH: i64 = SQLITE3_ASSOC | SQLITE3_NUM;

/// SQLite integer storage class.
pub const SQLITE3_INTEGER: i64 = 1;
/// SQLite float storage class.
pub const SQLITE3_FLOAT: i64 = 2;
/// SQLite text storage class.
pub const SQLITE3_TEXT: i64 = 3;
/// SQLite blob storage class.
pub const SQLITE3_BLOB: i64 = 4;
/// SQLite null storage class.
pub const SQLITE3_NULL: i64 = 5;

/// Open database read-only.
pub const SQLITE3_OPEN_READONLY: i64 = 0x0000_0001;
/// Open database read/write.
pub const SQLITE3_OPEN_READWRITE: i64 = 0x0000_0002;
/// Create database when opening.
pub const SQLITE3_OPEN_CREATE: i64 = 0x0000_0004;
/// Deterministic function flag.
pub const SQLITE3_DETERMINISTIC: i64 = 0x0000_0800;

#[derive(Clone, Debug, Eq, PartialEq)]
struct SqliteRow {
    columns: Vec<String>,
    values: Vec<Value>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SqliteResult {
    columns: Vec<String>,
    rows: Vec<SqliteRow>,
    offset: usize,
}

struct SqliteConnection {
    connection: Connection,
    last_error_code: i64,
    last_error_msg: String,
}

impl fmt::Debug for SqliteConnection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SqliteConnection")
            .field("last_error_code", &self.last_error_code)
            .field("last_error_msg", &self.last_error_msg)
            .finish_non_exhaustive()
    }
}

/// Request-local SQLite connections and materialized result sets.
#[derive(Default)]
pub struct SqliteState {
    next_connection_id: i64,
    connections: HashMap<i64, SqliteConnection>,
    next_result_id: i64,
    results: HashMap<i64, SqliteResult>,
}

impl fmt::Debug for SqliteState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SqliteState")
            .field("next_connection_id", &self.next_connection_id)
            .field("connections", &self.connections.keys().collect::<Vec<_>>())
            .field("next_result_id", &self.next_result_id)
            .field("results", &self.results.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl SqliteState {
    /// Opens an in-memory or local filesystem SQLite database.
    pub fn open(&mut self, filename: &str, flags: i64) -> Result<i64, String> {
        let connection = if filename == ":memory:" {
            Connection::open_in_memory_with_flags(open_flags(flags))
        } else {
            Connection::open_with_flags(Path::new(filename), open_flags(flags))
        }
        .map_err(sqlite_error_message)?;

        self.next_connection_id = self.next_connection_id.saturating_add(1).max(1);
        let id = self.next_connection_id;
        self.connections.insert(
            id,
            SqliteConnection {
                connection,
                last_error_code: 0,
                last_error_msg: "not an error".to_owned(),
            },
        );
        Ok(id)
    }

    /// Closes an open connection.
    pub fn close(&mut self, id: i64) -> bool {
        self.connections.remove(&id).is_some()
    }

    /// Executes a statement that does not return rows.
    pub fn exec(&mut self, id: i64, sql: &str) -> bool {
        self.exec_changes(id, sql).is_some()
    }

    /// Executes a statement and returns SQLite's affected row count.
    pub fn exec_changes(&mut self, id: i64, sql: &str) -> Option<i64> {
        let connection = self.connections.get_mut(&id)?;
        match connection.connection.execute_batch(sql) {
            Ok(()) => {
                connection.last_error_code = 0;
                connection.last_error_msg = "not an error".to_owned();
                Some(
                    connection
                        .connection
                        .changes()
                        .try_into()
                        .unwrap_or(i64::MAX),
                )
            }
            Err(error) => {
                set_connection_error(connection, error);
                None
            }
        }
    }

    /// Executes a query and stores all rows in a request-local result set.
    pub fn query(&mut self, id: i64, sql: &str) -> Option<i64> {
        let connection = self.connections.get_mut(&id)?;
        match materialize_query(&connection.connection, sql) {
            Ok(result) => {
                connection.last_error_code = 0;
                connection.last_error_msg = "not an error".to_owned();
                self.next_result_id = self.next_result_id.saturating_add(1).max(1);
                let result_id = self.next_result_id;
                self.results.insert(result_id, result);
                Some(result_id)
            }
            Err(error) => {
                set_connection_error(connection, error);
                None
            }
        }
    }

    /// Executes a query and returns the first column or whole first row.
    pub fn query_single(&mut self, id: i64, sql: &str, entire_row: bool) -> Value {
        let Some(result_id) = self.query(id, sql) else {
            return Value::Bool(false);
        };
        let value = self
            .results
            .get(&result_id)
            .and_then(|result| result.rows.first())
            .map_or(Value::Null, |row| {
                if entire_row {
                    row_to_array(row, SQLITE3_ASSOC)
                } else {
                    row.values.first().cloned().unwrap_or(Value::Null)
                }
            });
        self.results.remove(&result_id);
        value
    }

    /// Returns the last connection error code.
    #[must_use]
    pub fn last_error_code(&self, id: i64) -> i64 {
        self.connections
            .get(&id)
            .map_or(1, |connection| connection.last_error_code)
    }

    /// Returns the last connection error message.
    #[must_use]
    pub fn last_error_msg(&self, id: i64) -> String {
        self.connections.get(&id).map_or_else(
            || "not an open SQLite3 database".to_owned(),
            |connection| connection.last_error_msg.clone(),
        )
    }

    /// Fetches one row from a materialized result set.
    pub fn fetch_array(&mut self, id: i64, mode: i64) -> Value {
        let Some(result) = self.results.get_mut(&id) else {
            return Value::Bool(false);
        };
        let Some(row) = result.rows.get(result.offset).cloned() else {
            return Value::Bool(false);
        };
        result.offset = result.offset.saturating_add(1);
        row_to_array(&row, mode)
    }

    /// Returns all rows from a materialized result set.
    pub fn fetch_all(&mut self, id: i64, mode: i64) -> Value {
        let Some(result) = self.results.get_mut(&id) else {
            return Value::Bool(false);
        };
        let mut rows = PhpArray::new();
        for row in result.rows.iter().skip(result.offset) {
            rows.append(row_to_array(row, mode));
        }
        result.offset = result.rows.len();
        Value::Array(rows)
    }

    /// Resets a materialized result set cursor.
    pub fn reset_result(&mut self, id: i64) -> bool {
        let Some(result) = self.results.get_mut(&id) else {
            return false;
        };
        result.offset = 0;
        true
    }

    /// Finalizes a materialized result set.
    pub fn finalize_result(&mut self, id: i64) -> bool {
        self.results.remove(&id).is_some()
    }

    /// Returns the number of columns in a materialized result set.
    #[must_use]
    pub fn num_columns(&self, id: i64) -> i64 {
        self.results
            .get(&id)
            .map_or(0, |result| result.columns.len() as i64)
    }
}

fn open_flags(flags: i64) -> OpenFlags {
    let mut out = if flags & SQLITE3_OPEN_READONLY != 0 {
        OpenFlags::SQLITE_OPEN_READ_ONLY
    } else {
        OpenFlags::SQLITE_OPEN_READ_WRITE
    };
    if flags & SQLITE3_OPEN_CREATE != 0 {
        out |= OpenFlags::SQLITE_OPEN_CREATE;
    }
    out
}

fn materialize_query(connection: &Connection, sql: &str) -> Result<SqliteResult, rusqlite::Error> {
    let mut statement = connection.prepare(sql)?;
    let columns = statement
        .column_names()
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let column_count = statement.column_count();
    let mut query = statement.query([])?;
    let mut out = Vec::new();
    while let Some(row) = query.next()? {
        let mut values = Vec::with_capacity(column_count);
        for index in 0..column_count {
            values.push(sqlite_value(row.get_ref(index)?));
        }
        out.push(SqliteRow {
            columns: columns.clone(),
            values,
        });
    }
    Ok(SqliteResult {
        columns,
        rows: out,
        offset: 0,
    })
}

fn sqlite_value(value: ValueRef<'_>) -> Value {
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(value) => Value::Int(value),
        ValueRef::Real(value) => Value::float(value),
        ValueRef::Text(value) | ValueRef::Blob(value) => Value::string(value.to_vec()),
    }
}

fn row_to_array(row: &SqliteRow, mode: i64) -> Value {
    let mut array = PhpArray::new();
    if mode & SQLITE3_NUM != 0 {
        for (index, value) in row.values.iter().enumerate() {
            array.insert(ArrayKey::Int(index as i64), value.clone());
        }
    }
    if mode & SQLITE3_ASSOC != 0 {
        for (name, value) in row.columns.iter().zip(row.values.iter()) {
            array.insert(
                ArrayKey::String(PhpString::from_bytes(name.as_bytes().to_vec())),
                value.clone(),
            );
        }
    }
    Value::Array(array)
}

fn set_connection_error(connection: &mut SqliteConnection, error: rusqlite::Error) {
    connection.last_error_code = 1;
    connection.last_error_msg = sqlite_error_message(error);
}

fn sqlite_error_message(error: rusqlite::Error) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::{SQLITE3_ASSOC, SQLITE3_OPEN_CREATE, SQLITE3_OPEN_READWRITE, SqliteState};
    use crate::Value;

    #[test]
    fn sqlite_state_executes_memory_queries() {
        let mut state = SqliteState::default();
        let db = state
            .open(":memory:", SQLITE3_OPEN_READWRITE | SQLITE3_OPEN_CREATE)
            .expect("open");

        assert!(state.exec(db, "CREATE TABLE demo (id INTEGER, name TEXT)"));
        assert!(state.exec(db, "INSERT INTO demo VALUES (1, 'alpha')"));
        assert_eq!(
            state.query_single(db, "SELECT name FROM demo WHERE id = 1", false),
            Value::string("alpha")
        );

        let result = state.query(db, "SELECT id, name FROM demo").expect("query");
        let row = state.fetch_array(result, SQLITE3_ASSOC);
        assert!(matches!(row, Value::Array(_)));
        assert!(state.finalize_result(result));
        assert!(state.close(db));
    }
}
