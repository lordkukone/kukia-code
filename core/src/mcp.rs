//! Cliente MCP de KUKIA CODE.
//!
//! Dos reglas mandan sobre todo lo demás:
//!
//! 1. **Nada se ejecuta sin permiso.** Los servidores `stdio` son procesos de
//!    terceros (`npx`, `uvx`…). Se listan y se explican, pero solo se lanzan si
//!    el usuario los ha autorizado uno a uno (`mcp_grants`).
//! 2. **Lo que se afirma está medido.** Un servidor sin conectar aparece como
//!    «no comprobado», no como «funciona».
//!
//! Transportes soportados: `stdio` (JSON-RPC por líneas) y `http` (POST JSON-RPC,
//! con respuesta JSON o SSE).
//!
//! Configuración leída (sin modificar ninguno): `.agents/mcp.json` del proyecto y
//! del padre, `~/.agents/mcp.json` y `%APPDATA%\KukiaCode\config\mcp.json`.

use crate::memory::Database;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Versión del protocolo MCP que se anuncia en el handshake.
pub const PROTOCOLO: &str = "2024-11-05";
const ESPERA: Duration = Duration::from_secs(25);

#[derive(Debug, Clone, Serialize)]
pub struct ServerInfo {
    pub name: String,
    pub transport: String,
    pub command: String,
    pub args: Vec<String>,
    pub url: Option<String>,
    pub source: String,
    pub authorized: bool,
    /// Herramientas guardadas en la última exploración (0 = sin explorar).
    pub tools: usize,
    pub last_seen: Option<String>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub server: String,
    pub name: String,
    pub description: String,
    pub schema: serde_json::Value,
}

#[derive(Debug, Clone)]
struct ServerCfg {
    name: String,
    command: String,
    args: Vec<String>,
    url: Option<String>,
    source: String,
}

impl ServerCfg {
    fn transport(&self) -> &'static str {
        if self.url.is_some() {
            "http"
        } else {
            "stdio"
        }
    }
}

/* ============================ Configuración ============================ */

fn candidatos() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        out.push(cwd.join(".agents").join("mcp.json"));
        if let Some(p) = cwd.parent() {
            out.push(p.join(".agents").join("mcp.json"));
        }
    }
    if let Some(home) = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .ok()
        .map(PathBuf::from)
    {
        out.push(home.join(".agents").join("mcp.json"));
    }
    out.push(crate::config::data_dir().join("config").join("mcp.json"));
    out
}

fn leer_cfg(texto: &str, origen: &str) -> Vec<ServerCfg> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(texto) else {
        return Vec::new();
    };
    let mapa = v
        .get("mcpServers")
        .or_else(|| v.get("servers"))
        .and_then(|m| m.as_object());
    let Some(mapa) = mapa else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (name, s) in mapa {
        let url = s["url"]
            .as_str()
            .or_else(|| s["endpoint"].as_str())
            .map(|x| x.to_string());
        let command = s["command"].as_str().unwrap_or("").to_string();
        let args: Vec<String> = s["args"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        if url.is_none() && command.is_empty() {
            continue;
        }
        out.push(ServerCfg {
            name: name.clone(),
            command,
            args,
            url,
            source: origen.to_string(),
        });
    }
    out
}

/// Todos los servidores declarados, sin duplicados por nombre (gana el primero).
fn configs() -> Vec<ServerCfg> {
    let mut out: Vec<ServerCfg> = Vec::new();
    for p in candidatos() {
        let Ok(texto) = std::fs::read_to_string(&p) else {
            continue;
        };
        let origen = p.display().to_string();
        for c in leer_cfg(&texto, &origen) {
            if !out.iter().any(|x| x.name == c.name) {
                out.push(c);
            }
        }
    }
    out
}

/* ============================ Permisos ============================ */

pub fn esta_autorizado(db: &Database, server: &str) -> bool {
    db.conn()
        .query_row(
            "SELECT granted FROM mcp_grants WHERE server = ?1",
            rusqlite::params![server],
            |r| r.get::<_, i64>(0),
        )
        .map(|n| n > 0)
        .unwrap_or(false)
}

/// Autoriza o revoca un servidor. Autorizar es lo que permite lanzarlo.
pub fn autorizar(db: &Database, server: &str, permitido: bool) -> Result<(), String> {
    let ahora = chrono::Utc::now().to_rfc3339();
    db.conn()
        .execute(
            "INSERT INTO mcp_grants(server, granted, updated_at) VALUES (?1, ?2, ?3) \
             ON CONFLICT(server) DO UPDATE SET granted = excluded.granted, \
             updated_at = excluded.updated_at",
            rusqlite::params![server, if permitido { 1i64 } else { 0i64 }, ahora],
        )
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn tools_guardadas(db: &Database, server: &str) -> Vec<McpTool> {
    let Ok(mut st) = db.conn().prepare(
        "SELECT server, name, description, schema FROM mcp_tools WHERE server = ?1 ORDER BY name",
    ) else {
        return Vec::new();
    };
    let filas = st.query_map(rusqlite::params![server], |r| {
        let raw: String = r.get(3)?;
        Ok(McpTool {
            server: r.get(0)?,
            name: r.get(1)?,
            description: r.get(2)?,
            schema: serde_json::from_str(&raw).unwrap_or(serde_json::Value::Null),
        })
    });
    match filas {
        Ok(it) => it.filter_map(|r| r.ok()).collect(),
        Err(_) => Vec::new(),
    }
}

fn guardar_tools(db: &Database, server: &str, tools: &[McpTool]) -> Result<(), String> {
    let ahora = chrono::Utc::now().to_rfc3339();
    let c = db.conn();
    c.execute("DELETE FROM mcp_tools WHERE server = ?1", rusqlite::params![server])
        .map_err(|e| e.to_string())?;
    for t in tools {
        c.execute(
            "INSERT OR REPLACE INTO mcp_tools(server, name, description, schema, discovered_at) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                server,
                t.name,
                t.description,
                t.schema.to_string(),
                ahora
            ],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn ultima_exploracion(db: &Database, server: &str) -> Option<String> {
    db.conn()
        .query_row(
            "SELECT MAX(discovered_at) FROM mcp_tools WHERE server = ?1",
            rusqlite::params![server],
            |r| r.get::<_, Option<String>>(0),
        )
        .ok()
        .flatten()
}

/* ============================ Listado ============================ */

pub fn servidores(db: &Database) -> Vec<ServerInfo> {
    let mut out = Vec::new();
    for c in configs() {
        let authorized = esta_autorizado(db, &c.name);
        let tools = tools_guardadas(db, &c.name).len();
        let last_seen = ultima_exploracion(db, &c.name);
        let note = if c.transport() == "http" {
            if authorized {
                "http: se consulta bajo demanda".into()
            } else {
                "http sin autorizar: no se consulta".into()
            }
        } else if !authorized {
            format!(
                "stdio sin autorizar: lanzaría «{}» con {} argumento(s)",
                c.command,
                c.args.len()
            )
        } else if tools == 0 {
            "autorizado, sin explorar todavía".into()
        } else {
            format!("{tools} herramienta(s) descubierta(s)")
        };
        let transport = c.transport().to_string();
        out.push(ServerInfo {
            name: c.name,
            transport,
            command: c.command,
            args: c.args,
            url: c.url,
            source: c.source,
            authorized,
            tools,
            last_seen,
            note,
        });
    }
    out
}

/* ============================ Handshake ============================ */

fn peticion(id: i64, method: &str) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": {}
    })
}

fn extraer_tools(server: &str, v: &serde_json::Value) -> Vec<McpTool> {
    v["result"]["tools"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|t| {
                    let name = t["name"].as_str()?.to_string();
                    Some(McpTool {
                        server: server.to_string(),
                        name,
                        description: t["description"].as_str().unwrap_or("").to_string(),
                        schema: t
                            .get("inputSchema")
                            .cloned()
                            .unwrap_or(serde_json::Value::Null),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Handshake completo + una petición concreta por stdio. Devuelve la respuesta entera.
async fn stdio_rpc(
    cfg: &ServerCfg,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::process::{ChildStdout, Command};

    // Los `.cmd`/`.bat` de Windows no los ejecuta CreateProcess: hay que pasar por cmd /C.
    let bajo = cfg.command.to_lowercase();
    let (prog, mut prefix) = if bajo.ends_with(".cmd") || bajo.ends_with(".bat") {
        ("cmd".to_string(), vec!["/C".to_string(), cfg.command.clone()])
    } else {
        (cfg.command.clone(), Vec::new())
    };
    prefix.extend(cfg.args.iter().cloned());

    let mut cmd = Command::new(&prog);
    cmd.args(&prefix)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000);

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("no se pudo lanzar «{}»: {e}", cfg.command))?;
    let mut stdin = child.stdin.take().ok_or("el proceso no tiene stdin")?;
    let stdout: ChildStdout = child.stdout.take().ok_or("el proceso no tiene stdout")?;
    let mut lineas = BufReader::new(stdout).lines();

    let init = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": PROTOCOLO,
            "capabilities": {},
            "clientInfo": { "name": "KUKIA-CODE", "version": env!("CARGO_PKG_VERSION") }
        }
    });
    stdin
        .write_all(format!("{init}\n").as_bytes())
        .await
        .map_err(|e| format!("no se pudo escribir al servidor: {e}"))?;
    stdin.flush().await.ok();

    let _ = esperar(&mut lineas, 1).await?;

    stdin
        .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n")
        .await
        .ok();
    let pide = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": method,
        "params": params
    });
    stdin
        .write_all(format!("{pide}\n").as_bytes())
        .await
        .map_err(|e| format!("no se pudo enviar «{method}»: {e}"))?;
    stdin.flush().await.ok();

    let resp = esperar(&mut lineas, 2).await;
    let _ = child.kill().await;
    resp
}

async fn stdio_descubrir(cfg: &ServerCfg) -> Result<Vec<McpTool>, String> {
    let resp = stdio_rpc(cfg, "tools/list", serde_json::json!({})).await?;
    Ok(extraer_tools(&cfg.name, &resp))
}

async fn esperar(
    lineas: &mut tokio::io::Lines<tokio::io::BufReader<tokio::process::ChildStdout>>,
    id: i64,
) -> Result<serde_json::Value, String> {
    loop {
        let linea = tokio::time::timeout(ESPERA, lineas.next_line())
            .await
            .map_err(|_| {
                format!(
                    "el servidor MCP no respondió en {} s (¿descarga inicial de npx?)",
                    ESPERA.as_secs()
                )
            })?
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "el servidor MCP cerró la conexión".to_string())?;
        let Ok(v) = serde_json::from_str::<serde_json::Value>(linea.trim()) else {
            continue;
        };
        if v["id"].as_i64() == Some(id) {
            if let Some(err) = v.get("error") {
                return Err(format!(
                    "error MCP {}: {}",
                    err["code"],
                    err["message"].as_str().unwrap_or("sin detalle")
                ));
            }
            return Ok(v);
        }
    }
}

async fn http_llamar(url: &str, cuerpo: &serde_json::Value) -> Result<serde_json::Value, String> {
    let c = reqwest::Client::builder()
        .timeout(ESPERA)
        .user_agent("KUKIA-CODE/0.1")
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    let r = c
        .post(url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .json(cuerpo)
        .send()
        .await
        .map_err(|e| format!("HTTP MCP: {e}"))?;
    let estado = r.status();
    let texto = r.text().await.unwrap_or_default();
    if !estado.is_success() {
        return Err(format!("HTTP MCP {estado}"));
    }
    // Algunos servidores responden SSE: se toma la primera línea `data:`.
    if texto.trim_start().starts_with("data:") {
        for l in texto.lines() {
            if let Some(resto) = l.strip_prefix("data:") {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(resto.trim()) {
                    return Ok(v);
                }
            }
        }
        return Err("respuesta SSE ilegible".into());
    }
    serde_json::from_str(&texto).map_err(|e| format!("respuesta ilegible: {e}"))
}

async fn http_descubrir(cfg: &ServerCfg) -> Result<Vec<McpTool>, String> {
    let url = cfg.url.clone().ok_or("servidor sin URL")?;
    let init = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": PROTOCOLO,
            "capabilities": {},
            "clientInfo": { "name": "KUKIA-CODE", "version": env!("CARGO_PKG_VERSION") }
        }
    });
    let _ = http_llamar(&url, &init).await?;
    let v = http_llamar(&url, &peticion(2, "tools/list")).await?;
    Ok(extraer_tools(&cfg.name, &v))
}

/// Ejecuta una herramienta MCP ya descubierta. Requiere autorización del servidor:
/// el llamante debe comprobarla antes con `esta_autorizado`.
pub async fn llamar(
    server: &str,
    tool: &str,
    argumentos: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let cfg = configs()
        .into_iter()
        .find(|c| c.name == server)
        .ok_or_else(|| format!("«{server}» no está en ningún mcp.json"))?;
    let params = serde_json::json!({ "name": tool, "arguments": argumentos });
    if cfg.url.is_some() {
        let url = cfg.url.clone().ok_or("servidor sin URL")?;
        let init = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": PROTOCOLO,
                "capabilities": {},
                "clientInfo": { "name": "KUKIA-CODE", "version": env!("CARGO_PKG_VERSION") }
            }
        });
        let _ = http_llamar(&url, &init).await?;
        let mut body = peticion(2, "tools/call");
        body["params"] = params;
        let v = http_llamar(&url, &body).await?;
        return Ok(v["result"].clone());
    }
    let v = stdio_rpc(&cfg, "tools/call", params).await?;
    Ok(v["result"].clone())
}

/// Habla con un servidor concreto y devuelve sus herramientas, **sin tocar la base
/// de datos** (para no retener el candado durante la espera de red o de proceso).
/// El permiso se comprueba aparte, con `esta_autorizado`.
pub async fn conectar(server: &str) -> Result<Vec<McpTool>, String> {
    let cfg = configs()
        .into_iter()
        .find(|c| c.name == server)
        .ok_or_else(|| format!("«{server}» no está en ningún mcp.json"))?;
    if cfg.url.is_some() {
        http_descubrir(&cfg).await
    } else {
        stdio_descubrir(&cfg).await
    }
}

/// Guarda el resultado de una exploración.
pub fn guardar_en(db: &Database, server: &str, tools: &[McpTool]) -> Result<(), String> {
    guardar_tools(db, server, tools)
}

/// Conectar y guardar en una sola llamada (para uso interno del core).
pub async fn descubrir(db: &Database, server: &str) -> Result<Vec<McpTool>, String> {
    if !esta_autorizado(db, server) {
        return Err(format!(
            "«{server}» no está autorizado: es un proceso externo que debe autorizar el usuario"
        ));
    }
    let tools = conectar(server).await?;
    guardar_en(db, server, &tools)?;
    Ok(tools)
}

/// Descubre todos los autorizados. Devuelve `(servidor, resultado legible)`.
pub async fn descubrir_autorizados(db: &Database) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for c in configs() {
        if !esta_autorizado(db, &c.name) {
            continue;
        }
        match descubrir(db, &c.name).await {
            Ok(t) => out.push((c.name.clone(), format!("{} herramienta(s)", t.len()))),
            Err(e) => out.push((c.name.clone(), format!("error: {e}"))),
        }
    }
    out
}
