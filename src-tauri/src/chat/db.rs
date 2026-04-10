use super::vector::{bytes_to_vec_f32, vec_f32_to_bytes};
use chrono::Local;
use rusqlite::{params, Connection};
use std::path::Path;

pub struct DbState {
    pub conn: Connection,
}

impl DbState {
    pub fn new(db_path: &str) -> Result<Self, String> {
        let path = Path::new(db_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create DB directory: {}", e))?;
        }

        let conn = Connection::open(db_path).map_err(|e| format!("DB open failed: {}", e))?;

        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             CREATE TABLE IF NOT EXISTS chat_logs (id INTEGER PRIMARY KEY AUTOINCREMENT, role TEXT NOT NULL, content TEXT NOT NULL, timestamp TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS rag_chunks (id INTEGER PRIMARY KEY AUTOINCREMENT, content TEXT NOT NULL, embedding BLOB NOT NULL, created_at TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS global_memory (id INTEGER PRIMARY KEY CHECK (id = 1), content TEXT NOT NULL DEFAULT '');
             CREATE TABLE IF NOT EXISTS context_memory (id INTEGER PRIMARY KEY CHECK (id = 1), content TEXT NOT NULL DEFAULT '');"
        ).map_err(|e| format!("Schema init failed: {}", e))?;
        let _ = conn.execute(
            "ALTER TABLE chat_logs ADD COLUMN grammar_corrections TEXT DEFAULT NULL",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE chat_logs ADD COLUMN parsed_content TEXT DEFAULT NULL",
            [],
        );
        conn.execute("INSERT OR IGNORE INTO global_memory (id) VALUES (1)", [])
            .map_err(|e| e.to_string())?;
        conn.execute("INSERT OR IGNORE INTO context_memory (id) VALUES (1)", [])
            .map_err(|e| e.to_string())?;
        Ok(Self { conn })
    }

    pub fn append_log(&self, role: &str, content: &str) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO chat_logs (role, content, timestamp) VALUES (?1, ?2, ?3)",
                params![role, content, Local::now().to_rfc3339()],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn append_log_with_time(
        &self,
        role: &str,
        content: &str,
        timestamp: &str,
    ) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO chat_logs (role, content, timestamp) VALUES (?1, ?2, ?3)",
                params![role, content, timestamp],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn append_log_return_id(&self, role: &str, content: &str) -> Result<i64, String> {
        self.conn
            .execute(
                "INSERT INTO chat_logs (role, content, timestamp) VALUES (?1, ?2, ?3)",
                params![role, content, Local::now().to_rfc3339()],
            )
            .map_err(|e| e.to_string())?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn update_grammar(&self, log_id: i64, corrections_json: String) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE chat_logs SET grammar_corrections = ?1 WHERE id = ?2",
                params![corrections_json, log_id],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_context(&self) -> Result<String, String> {
        self.conn
            .query_row("SELECT content FROM context_memory WHERE id = 1", [], |r| {
                r.get(0)
            })
            .map_err(|e| e.to_string())
    }

    pub fn set_context(&self, content: &str) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE context_memory SET content = ?1 WHERE id = 1",
                params![content],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_global_memory(&self) -> Result<String, String> {
        self.conn
            .query_row("SELECT content FROM global_memory WHERE id = 1", [], |r| {
                r.get(0)
            })
            .map_err(|e| e.to_string())
    }

    pub fn set_global_memory(&self, content: &str) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE global_memory SET content = ?1 WHERE id = 1",
                params![content],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_all_rag_chunks(&self) -> Result<Vec<(String, Vec<f32>, String)>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT content, embedding, created_at FROM rag_chunks ORDER BY id ASC")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    bytes_to_vec_f32(&row.get::<_, Vec<u8>>(1)?),
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn append_rag_chunks(&self, chunks: &[(String, Vec<f32>, String)]) -> Result<(), String> {
        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;
        let mut stmt = tx
            .prepare("INSERT INTO rag_chunks (content, embedding, created_at) VALUES (?1, ?2, ?3)")
            .map_err(|e| e.to_string())?;
        for (text, emb, ts) in chunks {
            stmt.execute(params![text, vec_f32_to_bytes(emb), ts])
                .map_err(|e| e.to_string())?;
        }
        drop(stmt);
        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_latest_logs(
        &self,
        limit: u32,
    ) -> Result<Vec<(i64, String, String, String, Option<String>, Option<String>)>, String> {
        let mut stmt = self.conn
        .prepare("SELECT id, role, content, timestamp, grammar_corrections, parsed_content FROM chat_logs ORDER BY id DESC LIMIT ?")
        .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![limit], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn get_logs_before(
        &self,
        before_id: i64,
        limit: u32,
    ) -> Result<Vec<(i64, String, String, String, Option<String>, Option<String>)>, String> {
        let mut stmt = self.conn
        .prepare("SELECT id, role, content, timestamp, grammar_corrections, parsed_content FROM chat_logs WHERE id < ? ORDER BY id DESC LIMIT ?")
        .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![before_id, limit], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn update_parsed_content(&self, log_id: i64, parsed_json: String) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE chat_logs SET parsed_content = ?1 WHERE id = ?2",
                params![parsed_json, log_id],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}
