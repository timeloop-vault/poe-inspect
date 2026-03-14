use poe_rqe::index::IndexedStore;
use poe_rqe::predicate::Condition;
use poe_rqe::store::QueryId;
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
                labels     TEXT NOT NULL,
                owner      TEXT
            )",
        )
        .expect("failed to create table");

        // Add owner column to existing databases that don't have it yet.
        let has_owner = conn.prepare("SELECT owner FROM queries LIMIT 0").is_ok();
        if !has_owner {
            let _ = conn.execute_batch("ALTER TABLE queries ADD COLUMN owner TEXT");
        }
        Self { conn }
    }

    /// Load all stored queries into an `IndexedStore` (decision DAG),
    /// returning the store with the next ID set to max existing + 1.
    pub fn load_all(&self) -> IndexedStore {
        let mut stmt = self
            .conn
            .prepare("SELECT id, conditions, labels, owner FROM queries ORDER BY id")
            .expect("failed to prepare select");

        let mut store = IndexedStore::new();
        let mut max_id: Option<QueryId> = None;

        let rows = stmt
            .query_map([], |row| {
                let id: QueryId = row.get(0)?;
                let conditions_json: String = row.get(1)?;
                let labels_json: String = row.get(2)?;
                let owner: Option<String> = row.get(3)?;
                Ok((id, conditions_json, labels_json, owner))
            })
            .expect("failed to query");

        for row in rows {
            let (id, conditions_json, labels_json, owner) = row.expect("failed to read row");
            let conditions: Vec<Condition> =
                serde_json::from_str(&conditions_json).expect("corrupt conditions JSON in db");
            let labels: Vec<String> =
                serde_json::from_str(&labels_json).expect("corrupt labels JSON in db");
            store.add_with_id(id, conditions, labels, owner);
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

    pub fn insert(
        &self,
        id: QueryId,
        conditions: &[Condition],
        labels: &[String],
        owner: Option<&str>,
    ) {
        let conditions_json = serde_json::to_string(conditions).expect("failed to serialize");
        let labels_json = serde_json::to_string(labels).expect("failed to serialize");
        self.conn
            .execute(
                "INSERT INTO queries (id, conditions, labels, owner) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![id, conditions_json, labels_json, owner],
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
