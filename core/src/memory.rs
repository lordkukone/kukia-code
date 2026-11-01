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
pub struct ResearchAlertRow {
    pub id: i64,
    pub item_id: String,
    pub level: String,
    pub message: String,
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

/* ==================== Fase 3: tipos de seguridad avanzada ==================== */

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct AuditRow {
    pub id: i64,
    pub agent: String,
    pub tool: String,
    pub action: String,
    pub targets: String,
    pub approved: bool,
    pub result: String,
    pub error: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct AgentPolicy {
    pub agent: String,
    pub allow_read: bool,
    pub allow_write: bool,
    pub allow_shell: bool,
    pub allow_network: bool,
    pub allow_download: bool,
    pub allow_tools: bool,
    /// Rutas permitidas para lectura, separadas por `;`
    pub read_paths: String,
    /// Rutas permitidas para escritura, separadas por `;`
    pub write_paths: String,
    /// Rutas de solo lectura (se ignoran para escritura), separadas por `;`
    pub read_only_paths: String,
    pub updated_at: String,
}

impl AgentPolicy {
    /// Política conservadora por defecto: leer sí, escribir y ejecutar no.
    pub fn default_for(agent: &str) -> Self {
        Self {
            agent: agent.to_string(),
            allow_read: true,
            allow_write: false,
            allow_shell: false,
            allow_network: true,
            allow_download: false,
            allow_tools: false,
            read_paths: String::new(),
            write_paths: String::new(),
            read_only_paths: String::new(),
            updated_at: String::new(),
        }
    }

    pub fn split(list: &str) -> Vec<String> {
        list.split(';')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
}

/// Proyecto con su propio ámbito de permisos (Fase 3A).
///
/// La política del agente y la del proyecto se aplican **las dos**: un permiso
/// no se gana por estar en una lista, se pierde por no estar en la otra.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    /// Raíz del proyecto; sirve de ámbito por defecto.
    pub root: String,
    pub read_paths: String,
    pub write_paths: String,
    pub read_only_paths: String,
    pub allow_write: bool,
    pub allow_shell: bool,
    pub allow_network: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl Project {
    /// Proyecto nuevo a partir de su raíz: escribe y lee dentro de ella, sin ejecutar
    /// comandos hasta que se conceda expresamente.
    pub fn new_project(name: &str, root: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.trim().to_string(),
            root: root.trim().to_string(),
            read_paths: root.trim().to_string(),
            write_paths: root.trim().to_string(),
            read_only_paths: String::new(),
            allow_write: true,
            allow_shell: false,
            allow_network: true,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    pub fn split(list: &str) -> Vec<String> {
        AgentPolicy::split(list)
    }
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct TaskRow {
    pub id: String,
    pub title: String,
    pub agent: String,
    pub kind: String,
    pub status: String,
    pub progress: i64,
    pub checkpoint: String,
    pub error: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct GamingItem {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub source: String,
    pub url: String,
    pub published: String,
    pub updated: String,
    pub status: String,
    pub opensource: String,
    pub license: String,
    pub os: String,
    pub gpus: String,
    pub games: String,
    pub requirements: String,
    pub measured: String,
    pub risks: String,
    pub conflicts: String,
    pub rtx5070: String,
    pub rtx5060ti: String,
    pub potential: String,
    pub recommendation: String,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct GamingAlertRow {
    pub id: i64,
    pub item_id: String,
    pub level: String,
    pub message: String,
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
        // Fase 3.0: sin política no se puede escribir ni ejecutar. Si la tabla está
        // vacía se siembra una política explícita (no la permisiva por defecto del código).
        db.seed_policies()?;
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
                CREATE TABLE IF NOT EXISTS research_items (
                    id TEXT PRIMARY KEY,
                    kind TEXT NOT NULL DEFAULT '',
                    title TEXT NOT NULL,
                    source TEXT NOT NULL,
                    url TEXT NOT NULL DEFAULT '',
                    published TEXT NOT NULL DEFAULT '',
                    summary TEXT NOT NULL DEFAULT '',
                    license TEXT NOT NULL DEFAULT '',
                    windows TEXT NOT NULL DEFAULT '',
                    gpu TEXT NOT NULL DEFAULT '',
                    vram TEXT NOT NULL DEFAULT '',
                    expected TEXT NOT NULL DEFAULT '',
                    measured TEXT NOT NULL DEFAULT '',
                    risks TEXT NOT NULL DEFAULT '',
                    recommendation TEXT NOT NULL DEFAULT '',
                    status TEXT NOT NULL DEFAULT '',
                    score REAL NOT NULL DEFAULT 0,
                    created_at TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS research_alerts (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    item_id TEXT NOT NULL,
                    level TEXT NOT NULL,
                    message TEXT NOT NULL,
                    created_at TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS research_runs (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    source TEXT NOT NULL,
                    found INTEGER NOT NULL,
                    new_items INTEGER NOT NULL,
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
                /* ---------------- Fase 3: seguridad avanzada ---------------- */
                CREATE TABLE IF NOT EXISTS audit (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    agent TEXT NOT NULL DEFAULT '',
                    tool TEXT NOT NULL DEFAULT '',
                    action TEXT NOT NULL DEFAULT '',
                    targets TEXT NOT NULL DEFAULT '',
                    approved INTEGER NOT NULL DEFAULT 0,
                    result TEXT NOT NULL DEFAULT '',
                    error TEXT NOT NULL DEFAULT '',
                    created_at TEXT NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_audit_created ON audit(created_at);
                CREATE TABLE IF NOT EXISTS agent_policies (
                    agent TEXT PRIMARY KEY,
                    allow_read INTEGER NOT NULL DEFAULT 1,
                    allow_write INTEGER NOT NULL DEFAULT 0,
                    allow_shell INTEGER NOT NULL DEFAULT 0,
                    allow_network INTEGER NOT NULL DEFAULT 0,
                    allow_download INTEGER NOT NULL DEFAULT 0,
                    allow_tools INTEGER NOT NULL DEFAULT 0,
                    read_paths TEXT NOT NULL DEFAULT '',
                    write_paths TEXT NOT NULL DEFAULT '',
                    read_only_paths TEXT NOT NULL DEFAULT '',
                    updated_at TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS tasks (
                    id TEXT PRIMARY KEY,
                    title TEXT NOT NULL,
                    agent TEXT NOT NULL DEFAULT '',
                    kind TEXT NOT NULL DEFAULT '',
                    status TEXT NOT NULL DEFAULT 'pendiente',
                    progress INTEGER NOT NULL DEFAULT 0,
                    checkpoint TEXT NOT NULL DEFAULT '',
                    error TEXT NOT NULL DEFAULT '',
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );
                /* ---------------- Fase 3: gaming radar ---------------- */
                CREATE TABLE IF NOT EXISTS gaming_items (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    category TEXT NOT NULL DEFAULT '',
                    description TEXT NOT NULL DEFAULT '',
                    source TEXT NOT NULL DEFAULT '',
                    url TEXT NOT NULL DEFAULT '',
                    published TEXT NOT NULL DEFAULT '',
                    updated TEXT NOT NULL DEFAULT '',
                    status TEXT NOT NULL DEFAULT 'INVESTIGACION',
                    opensource TEXT NOT NULL DEFAULT '',
                    license TEXT NOT NULL DEFAULT '',
                    os TEXT NOT NULL DEFAULT '',
                    gpus TEXT NOT NULL DEFAULT '',
                    games TEXT NOT NULL DEFAULT '',
                    requirements TEXT NOT NULL DEFAULT '',
                    measured TEXT NOT NULL DEFAULT '',
                    risks TEXT NOT NULL DEFAULT '',
                    conflicts TEXT NOT NULL DEFAULT '',
                    rtx5070 TEXT NOT NULL DEFAULT '',
                    rtx5060ti TEXT NOT NULL DEFAULT '',
                    potential TEXT NOT NULL DEFAULT '',
                    recommendation TEXT NOT NULL DEFAULT '',
                    score REAL NOT NULL DEFAULT 0,
                    created_at TEXT NOT NULL
                );
                /* ---------------- Fase 3A: proyectos y su ámbito ---------------- */
                CREATE TABLE IF NOT EXISTS projects (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    root TEXT NOT NULL,
                    read_paths TEXT NOT NULL DEFAULT '',
                    write_paths TEXT NOT NULL DEFAULT '',
                    read_only_paths TEXT NOT NULL DEFAULT '',
                    allow_write INTEGER NOT NULL DEFAULT 1,
                    allow_shell INTEGER NOT NULL DEFAULT 0,
                    allow_network INTEGER NOT NULL DEFAULT 1,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS gaming_alerts (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    item_id TEXT NOT NULL,
                    level TEXT NOT NULL,
                    message TEXT NOT NULL,
                    created_at TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS gaming_runs (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    source TEXT NOT NULL,
                    found INTEGER NOT NULL,
                    new_items INTEGER NOT NULL,
                    created_at TEXT NOT NULL
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

    /* ------------------------- Research radar ------------------------- */

    pub fn research_upsert(&self, d: &crate::research::Discovery) -> Result<bool, String> {
        let exists: bool = self
            .conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM research_items WHERE id=?1)",
                params![d.id],
                |r| r.get(0),
            )
            .unwrap_or(false);
        self.conn
            .execute(
                "INSERT INTO research_items
                 (id,kind,title,source,url,published,summary,license,windows,gpu,vram,expected,measured,risks,recommendation,status,score,created_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)
                 ON CONFLICT(id) DO UPDATE SET
                   title=excluded.title, source=excluded.source, url=excluded.url,
                   published=excluded.published, summary=excluded.summary, license=excluded.license,
                   windows=excluded.windows, gpu=excluded.gpu, vram=excluded.vram,
                   expected=excluded.expected, measured=excluded.measured, risks=excluded.risks,
                   recommendation=excluded.recommendation, status=excluded.status, score=excluded.score",
                params![
                    d.id, d.kind, d.title, d.source, d.url, d.published, d.summary, d.license,
                    d.windows, d.gpu, d.vram, d.expected, d.measured, d.risks, d.recommendation,
                    d.status, d.score, Self::now()
                ],
            )
            .map_err(|e| e.to_string())?;
        Ok(!exists)
    }

    pub fn research_list(&self, limit: usize) -> Result<Vec<crate::research::Discovery>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id,kind,title,source,url,published,summary,license,windows,gpu,vram,expected,measured,risks,recommendation,status,score
                 FROM research_items ORDER BY score DESC, published DESC LIMIT ?1",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![limit as i64], |r| {
                Ok(crate::research::Discovery {
                    id: r.get(0)?,
                    kind: r.get(1)?,
                    title: r.get(2)?,
                    source: r.get(3)?,
                    url: r.get(4)?,
                    published: r.get(5)?,
                    summary: r.get(6)?,
                    license: r.get(7)?,
                    windows: r.get(8)?,
                    gpu: r.get(9)?,
                    vram: r.get(10)?,
                    expected: r.get(11)?,
                    measured: r.get(12)?,
                    risks: r.get(13)?,
                    recommendation: r.get(14)?,
                    status: r.get(15)?,
                    score: r.get(16)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn research_alert_add(&self, item_id: &str, level: &str, message: &str) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO research_alerts (item_id,level,message,created_at) VALUES (?1,?2,?3,?4)",
                params![item_id, level, message, Self::now()],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn research_alerts_list(&self, limit: usize) -> Result<Vec<ResearchAlertRow>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT id,item_id,level,message,created_at FROM research_alerts ORDER BY id DESC LIMIT ?1")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![limit as i64], |r| {
                Ok(ResearchAlertRow {
                    id: r.get(0)?,
                    item_id: r.get(1)?,
                    level: r.get(2)?,
                    message: r.get(3)?,
                    created_at: r.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn research_run_add(&self, source: &str, found: usize, new_items: usize) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO research_runs (source,found,new_items,created_at) VALUES (?1,?2,?3,?4)",
                params![source, found as i64, new_items as i64, Self::now()],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
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

    /* ==================== Fase 3: auditoría avanzada ==================== */

    #[allow(clippy::too_many_arguments)]
    pub fn audit_log(
        &self,
        agent: &str,
        tool: &str,
        action: &str,
        targets: &str,
        approved: bool,
        result: &str,
        error: &str,
    ) -> Result<i64, String> {
        self.conn
            .execute(
                "INSERT INTO audit (agent,tool,action,targets,approved,result,error,created_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                params![
                    agent,
                    tool,
                    action,
                    targets,
                    approved as i32,
                    result,
                    error,
                    Self::now()
                ],
            )
            .map_err(|e| e.to_string())?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn audit_search(
        &self,
        query: &str,
        tool: &str,
        approved: Option<bool>,
        limit: usize,
    ) -> Result<Vec<AuditRow>, String> {
        let like = format!("%{}%", query.trim());
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id,agent,tool,action,targets,approved,result,error,created_at
                 FROM audit
                 WHERE (?1 = '' OR action LIKE ?2 OR targets LIKE ?2 OR agent LIKE ?2 OR result LIKE ?2)
                   AND (?3 = '' OR tool = ?3)
                   AND (?4 = -1 OR approved = ?4)
                 ORDER BY id DESC LIMIT ?5",
            )
            .map_err(|e| e.to_string())?;
        let tool_filter = tool.trim();
        let appr: i32 = match approved {
            Some(true) => 1,
            Some(false) => 0,
            None => -1,
        };
        // OJO: ?1 debe ser el texto de búsqueda (comprueba si está vacío), no la herramienta.
        // Antes iba `tool_filter` y por eso el filtro de texto solo funcionaba si además
        // se filtraba por herramienta.
        let rows = stmt
            .query_map(
                params![query.trim(), like, tool_filter, appr, limit as i64],
                |r| {
                    Ok(AuditRow {
                        id: r.get(0)?,
                        agent: r.get(1)?,
                        tool: r.get(2)?,
                        action: r.get(3)?,
                        targets: r.get(4)?,
                        approved: r.get::<_, i32>(5)? != 0,
                        result: r.get(6)?,
                        error: r.get(7)?,
                        created_at: r.get(8)?,
                    })
                },
            )
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    /// Exporta la auditoría a CSV. Devuelve el número de filas escritas.
    pub fn audit_export_csv(&self, path: &str) -> Result<usize, String> {
        let rows = self.audit_search("", "", None, 100_000)?;
        let mut out = String::from("id;fecha;agente;herramienta;accion;objetivos;aprobado;resultado;error\n");
        for r in &rows {
            out.push_str(&format!(
                "{};{};{};{};{};{};{};{};{}\n",
                r.id,
                r.created_at,
                r.agent,
                r.tool,
                r.action.replace(';', ","),
                r.targets.replace(';', ","),
                if r.approved { "si" } else { "no" },
                r.result.replace(';', ",").replace('\n', " "),
                r.error.replace(';', ",").replace('\n', " ")
            ));
        }
        std::fs::write(path, out).map_err(|e| e.to_string())?;
        Ok(rows.len())
    }

    /* ==================== Fase 3: permisos por agente ==================== */

    pub fn policy_get(&self, agent: &str) -> Result<AgentPolicy, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT agent,allow_read,allow_write,allow_shell,allow_network,allow_download,
                        allow_tools,read_paths,write_paths,read_only_paths,updated_at
                 FROM agent_policies WHERE agent = ?1",
            )
            .map_err(|e| e.to_string())?;
        let mut rows = stmt
            .query_map(params![agent], |r| {
                Ok(AgentPolicy {
                    agent: r.get(0)?,
                    allow_read: r.get::<_, i32>(1)? != 0,
                    allow_write: r.get::<_, i32>(2)? != 0,
                    allow_shell: r.get::<_, i32>(3)? != 0,
                    allow_network: r.get::<_, i32>(4)? != 0,
                    allow_download: r.get::<_, i32>(5)? != 0,
                    allow_tools: r.get::<_, i32>(6)? != 0,
                    read_paths: r.get(7)?,
                    write_paths: r.get(8)?,
                    read_only_paths: r.get(9)?,
                    updated_at: r.get(10)?,
                })
            })
            .map_err(|e| e.to_string())?;
        match rows.next() {
            Some(Ok(p)) => Ok(p),
            _ => Ok(AgentPolicy::default_for(agent)),
        }
    }

    pub fn policy_set(&self, p: &AgentPolicy) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO agent_policies
                 (agent,allow_read,allow_write,allow_shell,allow_network,allow_download,
                  allow_tools,read_paths,write_paths,read_only_paths,updated_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)
                 ON CONFLICT(agent) DO UPDATE SET
                   allow_read=excluded.allow_read, allow_write=excluded.allow_write,
                   allow_shell=excluded.allow_shell, allow_network=excluded.allow_network,
                   allow_download=excluded.allow_download, allow_tools=excluded.allow_tools,
                   read_paths=excluded.read_paths, write_paths=excluded.write_paths,
                   read_only_paths=excluded.read_only_paths, updated_at=excluded.updated_at",
                params![
                    p.agent,
                    p.allow_read as i32,
                    p.allow_write as i32,
                    p.allow_shell as i32,
                    p.allow_network as i32,
                    p.allow_download as i32,
                    p.allow_tools as i32,
                    p.read_paths,
                    p.write_paths,
                    p.read_only_paths,
                    Self::now()
                ],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /* ==================== Fase 3A: proyectos ==================== */

    pub fn project_upsert(&self, p: &Project) -> Result<bool, String> {
        let nuevo: bool = self
            .conn
            .query_row("SELECT COUNT(*) FROM projects WHERE id=?1", params![p.id], |r| {
                r.get::<_, i64>(0)
            })
            .map(|n| n == 0)
            .unwrap_or(true);
        let ahora = Self::now();
        let creado = if nuevo { ahora.clone() } else { p.created_at.clone() };
        self.conn
            .execute(
                "INSERT INTO projects
                 (id,name,root,read_paths,write_paths,read_only_paths,allow_write,allow_shell,
                  allow_network,created_at,updated_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)
                 ON CONFLICT(id) DO UPDATE SET
                   name=excluded.name, root=excluded.root, read_paths=excluded.read_paths,
                   write_paths=excluded.write_paths, read_only_paths=excluded.read_only_paths,
                   allow_write=excluded.allow_write, allow_shell=excluded.allow_shell,
                   allow_network=excluded.allow_network, updated_at=excluded.updated_at",
                params![
                    p.id,
                    p.name,
                    p.root,
                    p.read_paths,
                    p.write_paths,
                    p.read_only_paths,
                    p.allow_write as i32,
                    p.allow_shell as i32,
                    p.allow_network as i32,
                    creado,
                    ahora
                ],
            )
            .map_err(|e| e.to_string())?;
        Ok(nuevo)
    }

    pub fn projects_list(&self) -> Result<Vec<Project>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id,name,root,read_paths,write_paths,read_only_paths,allow_write,
                        allow_shell,allow_network,created_at,updated_at
                 FROM projects ORDER BY name",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(Project {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    root: r.get(2)?,
                    read_paths: r.get(3)?,
                    write_paths: r.get(4)?,
                    read_only_paths: r.get(5)?,
                    allow_write: r.get::<_, i32>(6)? != 0,
                    allow_shell: r.get::<_, i32>(7)? != 0,
                    allow_network: r.get::<_, i32>(8)? != 0,
                    created_at: r.get(9)?,
                    updated_at: r.get(10)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn project_get(&self, id: &str) -> Result<Option<Project>, String> {
        Ok(self.projects_list()?.into_iter().find(|p| p.id == id))
    }

    pub fn policies_list(&self) -> Result<Vec<AgentPolicy>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT agent,allow_read,allow_write,allow_shell,allow_network,allow_download,
                        allow_tools,read_paths,write_paths,read_only_paths,updated_at
                 FROM agent_policies ORDER BY agent",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(AgentPolicy {
                    agent: r.get(0)?,
                    allow_read: r.get::<_, i32>(1)? != 0,
                    allow_write: r.get::<_, i32>(2)? != 0,
                    allow_shell: r.get::<_, i32>(3)? != 0,
                    allow_network: r.get::<_, i32>(4)? != 0,
                    allow_download: r.get::<_, i32>(5)? != 0,
                    allow_tools: r.get::<_, i32>(6)? != 0,
                    read_paths: r.get(7)?,
                    write_paths: r.get(8)?,
                    read_only_paths: r.get(9)?,
                    updated_at: r.get(10)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    /// Siembra una política por cada agente conocido, una sola vez.
    ///
    /// Decisión explícita de Fase 3.0 (no es el `default_for` del código):
    /// - Escritura permitida **solo** en el perfil del usuario y en los dos directorios del proyecto.
    /// - `I:\CEREBRO` marcado como solo lectura: es la regla del CEREBRO y no se toca nunca.
    /// - Ejecución permitida, pero cada comando exige aprobación y su `cwd` debe estar en la lista.
    /// Devuelve cuántas políticas se han creado (0 si ya había alguna).
    pub fn seed_policies(&self) -> Result<usize, String> {
        if !self.policies_list()?.is_empty() {
            return Ok(0);
        }
        let mut rutas: Vec<String> = Vec::new();
        if let Ok(perfil) = std::env::var("USERPROFILE") {
            if !perfil.trim().is_empty() {
                rutas.push(perfil);
            }
        }
        rutas.push(r"H:\KUKIA_CODE".to_string());
        rutas.push(r"H:\KUKIA_RESEARCH".to_string());
        let write_paths = rutas.join(";");

        let mut creadas = 0usize;
        for a in crate::agents::default_agents() {
            let mut p = AgentPolicy::default_for(&a.id);
            p.allow_write = true;
            p.allow_shell = true;
            p.allow_network = true;
            p.write_paths = write_paths.clone();
            p.read_only_paths = r"I:\CEREBRO".to_string();
            self.policy_set(&p)?;
            creadas += 1;
        }
        Ok(creadas)
    }

    /* ==================== Fase 3: tareas y recuperación ==================== */

    pub fn task_upsert(
        &self,
        id: &str,
        title: &str,
        agent: &str,
        kind: &str,
        status: &str,
        progress: i64,
        checkpoint: &str,
        error: &str,
    ) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO tasks (id,title,agent,kind,status,progress,checkpoint,error,created_at,updated_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?9)
                 ON CONFLICT(id) DO UPDATE SET
                   title=excluded.title, agent=excluded.agent, kind=excluded.kind,
                   status=excluded.status, progress=excluded.progress,
                   checkpoint=excluded.checkpoint, error=excluded.error,
                   updated_at=excluded.updated_at",
                params![id, title, agent, kind, status, progress, checkpoint, error, Self::now()],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn tasks_list(&self, limit: usize) -> Result<Vec<TaskRow>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id,title,agent,kind,status,progress,checkpoint,error,created_at,updated_at
                 FROM tasks ORDER BY updated_at DESC LIMIT ?1",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![limit as i64], |r| {
                Ok(TaskRow {
                    id: r.get(0)?,
                    title: r.get(1)?,
                    agent: r.get(2)?,
                    kind: r.get(3)?,
                    status: r.get(4)?,
                    progress: r.get(5)?,
                    checkpoint: r.get(6)?,
                    error: r.get(7)?,
                    created_at: r.get(8)?,
                    updated_at: r.get(9)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn task_get(&self, id: &str) -> Result<Option<TaskRow>, String> {
        Ok(self.tasks_list(1000)?.into_iter().find(|t| t.id == id))
    }

    /// Tareas que quedaron "en_curso" de una sesión anterior: se consideran interrumpidas.
    pub fn tasks_interrupted(&self) -> Result<Vec<TaskRow>, String> {
        Ok(self
            .tasks_list(1000)?
            .into_iter()
            .filter(|t| t.status == "en_curso")
            .collect())
    }

    pub fn task_set_status(&self, id: &str, status: &str, error: &str) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE tasks SET status=?2, error=?3, updated_at=?4 WHERE id=?1",
                params![id, status, error, Self::now()],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /* ==================== Fase 3: gaming radar ==================== */

    /// Inserta o actualiza un descubrimiento. Devuelve true si era nuevo.
    pub fn gaming_upsert(&self, g: &GamingItem) -> Result<bool, String> {
        let existe: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM gaming_items WHERE id=?1",
                params![g.id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        self.conn
            .execute(
                "INSERT INTO gaming_items
                 (id,name,category,description,source,url,published,updated,status,opensource,license,
                  os,gpus,games,requirements,measured,risks,conflicts,rtx5070,rtx5060ti,potential,
                  recommendation,score,created_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24)
                 ON CONFLICT(id) DO UPDATE SET
                   name=excluded.name, category=excluded.category, description=excluded.description,
                   source=excluded.source, url=excluded.url, published=excluded.published,
                   updated=excluded.updated, status=excluded.status, opensource=excluded.opensource,
                   license=excluded.license, os=excluded.os, gpus=excluded.gpus, games=excluded.games,
                   requirements=excluded.requirements, measured=excluded.measured, risks=excluded.risks,
                   conflicts=excluded.conflicts, rtx5070=excluded.rtx5070, rtx5060ti=excluded.rtx5060ti,
                   potential=excluded.potential, recommendation=excluded.recommendation,
                   score=excluded.score",
                params![
                    g.id,
                    g.name,
                    g.category,
                    g.description,
                    g.source,
                    g.url,
                    g.published,
                    g.updated,
                    g.status,
                    g.opensource,
                    g.license,
                    g.os,
                    g.gpus,
                    g.games,
                    g.requirements,
                    g.measured,
                    g.risks,
                    g.conflicts,
                    g.rtx5070,
                    g.rtx5060ti,
                    g.potential,
                    g.recommendation,
                    g.score,
                    Self::now()
                ],
            )
            .map_err(|e| e.to_string())?;
        Ok(existe == 0)
    }

    pub fn gaming_list(&self, limit: usize) -> Result<Vec<GamingItem>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id,name,category,description,source,url,published,updated,status,opensource,
                        license,os,gpus,games,requirements,measured,risks,conflicts,rtx5070,rtx5060ti,
                        potential,recommendation,score
                 FROM gaming_items ORDER BY score DESC, name ASC LIMIT ?1",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![limit as i64], |r| {
                Ok(GamingItem {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    category: r.get(2)?,
                    description: r.get(3)?,
                    source: r.get(4)?,
                    url: r.get(5)?,
                    published: r.get(6)?,
                    updated: r.get(7)?,
                    status: r.get(8)?,
                    opensource: r.get(9)?,
                    license: r.get(10)?,
                    os: r.get(11)?,
                    gpus: r.get(12)?,
                    games: r.get(13)?,
                    requirements: r.get(14)?,
                    measured: r.get(15)?,
                    risks: r.get(16)?,
                    conflicts: r.get(17)?,
                    rtx5070: r.get(18)?,
                    rtx5060ti: r.get(19)?,
                    potential: r.get(20)?,
                    recommendation: r.get(21)?,
                    score: r.get(22)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn gaming_alert_add(&self, item_id: &str, level: &str, message: &str) -> Result<(), String> {
        let existe: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM gaming_alerts WHERE item_id=?1 AND message=?2",
                params![item_id, message],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if existe > 0 {
            return Ok(());
        }
        self.conn
            .execute(
                "INSERT INTO gaming_alerts (item_id,level,message,created_at) VALUES (?1,?2,?3,?4)",
                params![item_id, level, message, Self::now()],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn gaming_alerts_list(&self, limit: usize) -> Result<Vec<GamingAlertRow>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id,item_id,level,message,created_at FROM gaming_alerts
                 ORDER BY id DESC LIMIT ?1",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![limit as i64], |r| {
                Ok(GamingAlertRow {
                    id: r.get(0)?,
                    item_id: r.get(1)?,
                    level: r.get(2)?,
                    message: r.get(3)?,
                    created_at: r.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn gaming_run_add(&self, source: &str, found: i64, new_items: i64) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO gaming_runs (source,found,new_items,created_at) VALUES (?1,?2,?3,?4)",
                params![source, found, new_items, Self::now()],
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
