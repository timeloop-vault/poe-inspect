use poe_rqe::predicate::Condition;
use poe_rqe::store::{QueryId, QueryStore};
use rusqlite::Connection;

const DEFAULT_PATH: &str = "rqe.db";

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn open(path: Option<&str>) -> Self {
        let conn = Connection::open(path.unwrap_or(DEFAULT_PATH)).expect("failed to open database");
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS queries (
                id    INTEGER PRIMARY KEY,
                conditions TEXT NOT NULL,
                labels     TEXT NOT NULL
            )",
        )
        .expect("failed to create table");
        Self { conn }
    }

    /// Load all stored queries into a `QueryStore`, returning the store
    /// and the next ID to use (max existing ID + 1).
    pub fn load_all(&self) -> QueryStore {
        let mut stmt = self
            .conn
            .prepare("SELECT id, conditions, labels FROM queries ORDER BY id")
            .expect("failed to prepare select");

        let mut store = QueryStore::new();
        let mut max_id: Option<QueryId> = None;

        let rows = stmt
            .query_map([], |row| {
                let id: QueryId = row.get(0)?;
                let conditions_json: String = row.get(1)?;
                let labels_json: String = row.get(2)?;
                Ok((id, conditions_json, labels_json))
            })
            .expect("failed to query");

        for row in rows {
            let (id, conditions_json, labels_json) = row.expect("failed to read row");
            let conditions: Vec<Condition> =
                serde_json::from_str(&conditions_json).expect("corrupt conditions JSON in db");
            let labels: Vec<String> =
                serde_json::from_str(&labels_json).expect("corrupt labels JSON in db");
            store.add_with_id(id, conditions, labels);
            max_id = Some(match max_id {
                Some(prev) => prev.max(id),
                None => id,
            });
        }

        if let Some(max) = max_id {
            store.set_next_id(max + 1);
        }

        store
    }

    pub fn insert(&self, id: QueryId, conditions: &[Condition], labels: &[String]) {
        let conditions_json = serde_json::to_string(conditions).expect("failed to serialize");
        let labels_json = serde_json::to_string(labels).expect("failed to serialize");
        self.conn
            .execute(
                "INSERT INTO queries (id, conditions, labels) VALUES (?1, ?2, ?3)",
                rusqlite::params![id, conditions_json, labels_json],
            )
            .expect("failed to insert query");
    }

    pub fn delete(&self, id: QueryId) -> bool {
        let affected = self
            .conn
            .execute("DELETE FROM queries WHERE id = ?1", rusqlite::params![id])
            .expect("failed to delete query");
        affected > 0
    }
}
