use chrono::Utc;
use rusqlite::{params, Connection};
use serde::Serialize;
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct Session {
    pub id: String,
    pub title: String,
    pub agent: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MessageRow {
    pub id: i64,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkResult {
    pub id: i64,
    pub provider: String,
    pub model: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_ms: u64,
    pub tokens_per_s: f64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompatRow {
    pub id: i64,
    pub engine: String,
    pub model: String,
    pub arch: String,
    pub quant: String,
    pub gpu: String,
    pub method: String,
    pub notes: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApprovalRow {
    pub id: i64,
    pub action: String,
    pub detail: String,
    pub approved: bool,
    pub created_at: String,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        let db = Self { conn };
        db.init()?;
        Ok(db)
    }

    fn init(&self) -> Result<(), String> {
        self.conn
            .execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS sessions (
                    id TEXT PRIMARY KEY,
                    title TEXT NOT NULL,
                    agent TEXT NOT NULL DEFAULT 'kukia-master',
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS messages (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    session_id TEXT NOT NULL,
                    role TEXT NOT NULL,
                    content TEXT NOT NULL,
                    created_at TEXT NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id);
                CREATE TABLE IF NOT EXISTS memories (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    content TEXT NOT NULL,
                    tags TEXT NOT NULL DEFAULT '',
                    created_at TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS approvals (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    action TEXT NOT NULL,
                    detail TEXT NOT NULL,
                    approved INTEGER NOT NULL,
                    created_at TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS benchmarks (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    provider TEXT NOT NULL,
                    model TEXT NOT NULL,
                    prompt_tokens INTEGER NOT NULL,
                    completion_tokens INTEGER NOT NULL,
                    total_ms INTEGER NOT NULL,
                    tokens_per_s REAL NOT NULL,
                    created_at TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS compat (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    engine TEXT NOT NULL,
                    model TEXT NOT NULL,
                    arch TEXT NOT NULL DEFAULT '',
                    quant TEXT NOT NULL DEFAULT '',
                    gpu TEXT NOT NULL DEFAULT '',
                    method TEXT NOT NULL DEFAULT '',
                    notes TEXT NOT NULL DEFAULT '',
                    created_at TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS radar_cache (
                    source TEXT PRIMARY KEY,
                    version TEXT NOT NULL DEFAULT '',
                    published TEXT NOT NULL DEFAULT '',
                    url TEXT NOT NULL DEFAULT '',
                    note TEXT NOT NULL DEFAULT '',
                    checked_at TEXT NOT NULL
                );
                "#,
            )
            .map_err(|e| e.to_string())
    }

    fn now() -> String {
        Utc::now().to_rfc3339()
    }

    /* ------------------------- Sesiones ------------------------- */

    pub fn sessions_list(&self) -> Result<Vec<Session>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT id,title,agent,created_at,updated_at FROM sessions ORDER BY updated_at DESC")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(Session {
                    id: r.get(0)?,
                    title: r.get(1)?,
                    agent: r.get(2)?,
                    created_at: r.get(3)?,
                    updated_at: r.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn session_create(&self, title: &str, agent: &str) -> Result<Session, String> {
        let id = Uuid::new_v4().to_string();
        let now = Self::now();
        self.conn
            .execute(
                "INSERT INTO sessions (id,title,agent,created_at,updated_at) VALUES (?1,?2,?3,?4,?5)",
                params![id, title, agent, now, now],
            )
            .map_err(|e| e.to_string())?;
        Ok(Session {
            id,
            title: title.to_string(),
            agent: agent.to_string(),
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn session_rename(&self, id: &str, title: &str) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE sessions SET title=?2, updated_at=?3 WHERE id=?1",
                params![id, title, Self::now()],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn session_touch(&self, id: &str) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE sessions SET updated_at=?2 WHERE id=?1",
                params![id, Self::now()],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /* ------------------------- Mensajes ------------------------- */

    pub fn messages_list(&self, session_id: &str) -> Result<Vec<MessageRow>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id,session_id,role,content,created_at FROM messages WHERE session_id=?1 ORDER BY id ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![session_id], |r| {
                Ok(MessageRow {
                    id: r.get(0)?,
                    session_id: r.get(1)?,
                    role: r.get(2)?,
                    content: r.get(3)?,
                    created_at: r.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn message_add(
        &self,
        session_id: &str,
        role: &str,
        content: &str,
    ) -> Result<MessageRow, String> {
        let now = Self::now();
        self.conn
            .execute(
                "INSERT INTO messages (session_id,role,content,created_at) VALUES (?1,?2,?3,?4)",
                params![session_id, role, content, now],
            )
            .map_err(|e| e.to_string())?;
        let id = self.conn.last_insert_rowid();
        self.session_touch(session_id)?;
        Ok(MessageRow {
            id,
            session_id: session_id.to_string(),
            role: role.to_string(),
            content: content.to_string(),
            created_at: now,
        })
    }

    /* ------------------------- Memoria ------------------------- */

    pub fn memory_add(&self, content: &str, tags: &str) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO memories (content,tags,created_at) VALUES (?1,?2,?3)",
                params![content, tags, Self::now()],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn memory_search(&self, query: &str) -> Result<Vec<String>, String> {
        let like = format!("%{}%", query.trim());
        let mut stmt = self
            .conn
            .prepare(
                "SELECT content, tags, created_at FROM memories
                 WHERE content LIKE ?1 OR tags LIKE ?1
                 ORDER BY id DESC LIMIT 100",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![like], |r| {
                let content: String = r.get(0)?;
                let tags: String = r.get(1)?;
                let created: String = r.get(2)?;
                Ok(if tags.is_empty() {
                    format!("[{created}] {content}")
                } else {
                    format!("[{created}] ({tags}) {content}")
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    /* ------------------------- Aprobaciones ------------------------- */

    pub fn approval_log(&self, action: &str, detail: &str, approved: bool) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO approvals (action,detail,approved,created_at) VALUES (?1,?2,?3,?4)",
                params![action, detail, approved as i32, Self::now()],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn approvals_list(&self) -> Result<Vec<ApprovalRow>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT id,action,detail,approved,created_at FROM approvals ORDER BY id DESC LIMIT 200")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(ApprovalRow {
                    id: r.get(0)?,
                    action: r.get(1)?,
                    detail: r.get(2)?,
                    approved: r.get::<_, i32>(3)? != 0,
                    created_at: r.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    /* ------------------------- Benchmarks ------------------------- */

    pub fn benchmark_add(
        &self,
        provider: &str,
        model: &str,
        prompt_tokens: u64,
        completion_tokens: u64,
        total_ms: u64,
    ) -> Result<BenchmarkResult, String> {
        let tps = if total_ms > 0 {
            completion_tokens as f64 / (total_ms as f64 / 1000.0)
        } else {
            0.0
        };
        let now = Self::now();
        self.conn
            .execute(
                "INSERT INTO benchmarks (provider,model,prompt_tokens,completion_tokens,total_ms,tokens_per_s,created_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7)",
                params![provider, model, prompt_tokens as i64, completion_tokens as i64, total_ms as i64, tps, now],
            )
            .map_err(|e| e.to_string())?;
        Ok(BenchmarkResult {
            id: self.conn.last_insert_rowid(),
            provider: provider.to_string(),
            model: model.to_string(),
            prompt_tokens,
            completion_tokens,
            total_ms,
            tokens_per_s: tps,
            created_at: now,
        })
    }

    pub fn benchmarks_list(&self) -> Result<Vec<BenchmarkResult>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id,provider,model,prompt_tokens,completion_tokens,total_ms,tokens_per_s,created_at
                 FROM benchmarks ORDER BY id DESC LIMIT 200",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(BenchmarkResult {
                    id: r.get(0)?,
                    provider: r.get(1)?,
                    model: r.get(2)?,
                    prompt_tokens: r.get::<_, i64>(3)? as u64,
                    completion_tokens: r.get::<_, i64>(4)? as u64,
                    total_ms: r.get::<_, i64>(5)? as u64,
                    tokens_per_s: r.get(6)?,
                    created_at: r.get(7)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    /* ------------------------- Compatibilidad ------------------------- */

    pub fn compat_add(
        &self,
        engine: &str,
        model: &str,
        arch: &str,
        quant: &str,
        gpu: &str,
        method: &str,
        notes: &str,
    ) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO compat (engine,model,arch,quant,gpu,method,notes,created_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                params![engine, model, arch, quant, gpu, method, notes, Self::now()],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn compat_list(&self) -> Result<Vec<CompatRow>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id,engine,model,arch,quant,gpu,method,notes,created_at
                 FROM compat ORDER BY id DESC LIMIT 300",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(CompatRow {
                    id: r.get(0)?,
                    engine: r.get(1)?,
                    model: r.get(2)?,
                    arch: r.get(3)?,
                    quant: r.get(4)?,
                    gpu: r.get(5)?,
                    method: r.get(6)?,
                    notes: r.get(7)?,
                    created_at: r.get(8)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    /* ------------------------- Radar cache ------------------------- */

    pub fn radar_upsert(
        &self,
        source: &str,
        version: &str,
        published: &str,
        url: &str,
        note: &str,
    ) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO radar_cache (source,version,published,url,note,checked_at)
                 VALUES (?1,?2,?3,?4,?5,?6)
                 ON CONFLICT(source) DO UPDATE SET
                   version=excluded.version, published=excluded.published,
                   url=excluded.url, note=excluded.note, checked_at=excluded.checked_at",
                params![source, version, published, url, note, Self::now()],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_db() -> Database {
        let dir = std::env::temp_dir().join(format!("kukia-test-{}", Uuid::new_v4()));
        Database::open(&dir.join("test.db")).unwrap()
    }

    #[test]
    fn session_and_message_roundtrip() {
        let db = tmp_db();
        let s = db.session_create("prueba", "kukia-master").unwrap();
        db.message_add(&s.id, "user", "hola").unwrap();
        db.message_add(&s.id, "assistant", "buenas").unwrap();
        let msgs = db.messages_list(&s.id).unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].content, "hola");
        assert_eq!(msgs[1].role, "assistant");
    }

    #[test]
    fn memory_search_finds_content() {
        let db = tmp_db();
        db.memory_add("Kuki usa RTX 5070 y 5060 Ti", "hardware").unwrap();
        let hits = db.memory_search("5060").unwrap();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].contains("5060"));
    }

    #[test]
    fn benchmark_computes_tokens_per_second() {
        let db = tmp_db();
        let r = db.benchmark_add("ollama", "m", 10, 100, 2000).unwrap();
        assert!((r.tokens_per_s - 50.0).abs() < 0.001);
    }

    #[test]
    fn approvals_are_logged() {
        let db = tmp_db();
        db.approval_log("files.write", "H:\\x.txt", true).unwrap();
        let rows = db.approvals_list().unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].approved);
    }
}
