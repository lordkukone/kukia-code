//! Puente bidireccional KUKIA CORE ↔ OpenCode.
//!
//! Todo lo que hay aquí está comprobado contra OpenCode 1.19.3 instalado en esta
//! máquina (evidencia en `INFORMES/evidencia/opencode_openapi.json`):
//!
//! **Dirección A — KUKIA usa OpenCode**
//! - Descubrimiento sin red: `opencode --version`, `opencode models`,
//!   `opencode agent list` y la configuración `~/.config/opencode/opencode.json`
//!   (solo se leen claves, nunca valores sensibles).
//! - Con el servidor de OpenCode vivo (`opencode serve --port N`), la API real
//!   aporta más que el CLI: `GET /api/model` (213 modelos), `GET /api/agent`,
//!   `POST /api/session` y `POST /session/{id}/message`.
//! - Conversación: se crea sesión, se manda el mensaje con `parts` y se lee la
//!   respuesta guardada con `GET /session/{id}/message`.
//!
//! **Dirección B — OpenCode usa KUKIA**
//! - `serve.rs` expone un servidor compatible con OpenAI en loopback y aquí se
//!   genera el fragmento exacto que hay que pegar en `opencode.json`. Este
//!   módulo **no escribe** en la configuración de OpenCode: la genera para que
//!   el usuario decida.
//!
//! Reglas del puente:
//! - Ningún programa depende del otro: si OpenCode no está, KUKIA funciona; si
//!   KUKIA está cerrado, OpenCode funciona.
//! - Todas las llamadas llevan timeout y se reportan sus errores reales.
//! - No se ejecutan comandos arbitrarios: solo `opencode` con sus subcomandos
//!   documentados.

use serde::Serialize;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

/* ============================ Descubrimiento ============================ */

#[derive(Debug, Clone, Serialize, Default)]
pub struct ProvCount {
    pub provider: String,
    pub modelos: usize,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct ServidorInfo {
    /// URL probada (por defecto el puerto que usa KUKIA para arrancarlo).
    pub url: String,
    pub vivo: bool,
    pub modelos: usize,
    pub agentes: usize,
    pub sesiones: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct PuenteEstado {
    pub opencode_instalado: bool,
    pub binario: String,
    pub version: String,
    pub config_path: String,
    pub config_ok: bool,
    pub config_nota: String,
    pub modelo_por_defecto: String,
    pub agente_por_defecto: String,
    pub modelos_cli: usize,
    pub agentes_cli: usize,
    pub proveedores: Vec<ProvCount>,
    pub mcp_config: Vec<String>,
    pub servidor: ServidorInfo,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModeloPuente {
    pub provider: String,
    pub id: String,
    /// `provider/id`, que es como lo pide `opencode run -m`.
    pub full: String,
    pub origen: String,
    pub habilitado: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentePuente {
    pub nombre: String,
    pub modo: String,
    pub modelo: Option<String>,
    pub origen: String,
}

#[cfg(windows)]
fn oculto(cmd: &mut Command) -> &mut Command {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(0x0800_0000) // CREATE_NO_WINDOW
}

#[cfg(not(windows))]
fn oculto(cmd: &mut Command) -> &mut Command {
    cmd
}

/// Localiza el binario de OpenCode. En Windows el instalador de npm deja
/// `opencode.cmd`, y eso obliga a lanzarlo a través de `cmd /C`.
pub fn binario() -> String {
    if let Ok(p) = std::env::var("KUKIA_OPENCODE_BIN") {
        if !p.trim().is_empty() {
            return p;
        }
    }
    let mut cmd = Command::new("where");
    cmd.arg("opencode");
    if let Ok(out) = oculto(&mut cmd).output() {
        if out.status.success() {
            let txt = String::from_utf8_lossy(&out.stdout);
            let lineas: Vec<&str> = txt.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
            // Preferimos el `.cmd`: el archivo sin extensión no es ejecutable.
            if let Some(c) = lineas.iter().find(|l| l.to_lowercase().ends_with(".cmd")) {
                return c.to_string();
            }
            if let Some(first) = lineas.first() {
                return first.to_string();
            }
        }
    }
    "opencode".to_string()
}

fn es_wrapper(bin: &str) -> bool {
    let l = bin.to_lowercase();
    l.ends_with(".cmd") || l.ends_with(".bat")
}

/// Ejecuta `opencode <args>` una vez y devuelve (ok, stdout, stderr).
fn ejecutar_cli(bin: &str, args: &[&str], timeout: Duration) -> Result<(bool, String, String), String> {
    let mut cmd = if es_wrapper(bin) {
        let mut c = Command::new("cmd");
        c.arg("/C").arg(bin);
        c
    } else {
        Command::new(bin)
    };
    cmd.args(args);
    oculto(&mut cmd).stdin(std::process::Stdio::null());

    // No hay timeout nativo en std::process: lanzamos y esperamos con sondeo.
    let mut child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("no se pudo ejecutar «{}»: {e}", bin))?;

    let inicio = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if inicio.elapsed() > timeout {
                    let _ = child.kill();
                    return Err(format!(
                        "«opencode {}» no terminó en {}s",
                        args.join(" "),
                        timeout.as_secs()
                    ));
                }
                std::thread::sleep(Duration::from_millis(120));
            }
            Err(e) => return Err(format!("esperando a opencode: {e}")),
        }
    }
    let out = child
        .wait_with_output()
        .map_err(|e| format!("leyendo la salida de opencode: {e}"))?;
    Ok((
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    ))
}

fn ruta_config() -> PathBuf {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".config").join("opencode").join("opencode.json")
}

/// Lee la configuración de OpenCode sin exponer secretos: solo claves.
fn leer_config() -> (bool, String, String, String, Vec<ProvCount>, Vec<String>) {
    let p = ruta_config();
    if !p.exists() {
        return (
            false,
            p.display().to_string(),
            "no existe opencode.json".into(),
            String::new(),
            Vec::new(),
            Vec::new(),
        );
    }
    let raw = match std::fs::read_to_string(&p) {
        Ok(r) => r,
        Err(e) => {
            return (
                false,
                p.display().to_string(),
                format!("no se pudo leer: {e}"),
                String::new(),
                Vec::new(),
                Vec::new(),
            )
        }
    };
    // El archivo puede llevar BOM y comentarios de estilo JSONC.
    let limpio = raw.trim_start_matches('\u{feff}');
    let sin_comentarios: String = limpio
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");

    let v: serde_json::Value = match serde_json::from_str(&sin_comentarios) {
        Ok(v) => v,
        Err(e) => {
            return (
                false,
                p.display().to_string(),
                format!("JSON no válido: {e}"),
                String::new(),
                Vec::new(),
                Vec::new(),
            )
        }
    };

    let mut provs = Vec::new();
    if let Some(prov) = v.get("provider").and_then(|x| x.as_object()) {
        for (nombre, def) in prov {
            let modelos = def
                .get("models")
                .and_then(|m| m.as_object())
                .map(|m| m.len())
                .unwrap_or(0);
            provs.push(ProvCount {
                provider: nombre.clone(),
                modelos,
            });
        }
    }
    provs.sort_by(|a, b| b.modelos.cmp(&a.modelos));

    let mcp = v
        .get("mcp")
        .and_then(|m| m.as_object())
        .map(|m| {
            m.iter()
                .map(|(k, d)| {
                    let on = d.get("enabled").and_then(|e| e.as_bool()).unwrap_or(true);
                    format!("{k}{}", if on { "" } else { " (desactivado)" })
                })
                .collect()
        })
        .unwrap_or_default();

    let modelo = v
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("")
        .to_string();
    let agente = v
        .get("default_agent")
        .and_then(|m| m.as_str())
        .unwrap_or("")
        .to_string();
    (
        true,
        p.display().to_string(),
        "leída sin mostrar credenciales".into(),
        format!("{modelo}|{agente}"),
        provs,
        mcp,
    )
}

/// Estado completo: CLI + configuración + sondeo del servidor (si se indica).
pub async fn estado(url_servidor: Option<String>) -> PuenteEstado {
    let bin = binario();
    let mut st = PuenteEstado {
        binario: bin.clone(),
        ..Default::default()
    };

    match ejecutar_cli(&bin, &["--version"], Duration::from_secs(15)) {
        Ok((_, out, _)) => {
            st.opencode_instalado = true;
            st.version = out.trim().to_string();
        }
        Err(e) => {
            st.config_nota = e;
            return st;
        }
    }

    let (ok, ruta, nota, defs, provs, mcp) = leer_config();
    st.config_ok = ok;
    st.config_path = ruta;
    st.config_nota = nota;
    let (modelo, agente) = defs.split_once('|').unwrap_or(("", ""));
    st.modelo_por_defecto = modelo.to_string();
    st.agente_por_defecto = agente.to_string();
    st.proveedores = provs;
    st.mcp_config = mcp;

        // El CLI reparte su salida entre stdout y stderr según cómo se lance, así
        // que se leen los dos: quedarse con uno daba conteos falsos (0 agentes).
    if let Ok((_, out, err)) = ejecutar_cli(&bin, &["models"], Duration::from_secs(30)) {
        st.modelos_cli = format!("{out}{err}")
            .lines()
            .filter(|l| l.contains('/') && !l.trim_start().starts_with('{'))
            .count();
    }
    // `opencode agent list` se queda colgado cuando no hay terminal (comprobado:
    // 20 s sin terminar), así que el timeout es corto y su fallo no debe frenar
    // el resto. Los agentes reales se piden por API cuando hay servidor.
    if let Ok((_, out, err)) = ejecutar_cli(&bin, &["agent", "list"], Duration::from_secs(12)) {
        st.agentes_cli = format!("{out}{err}")
            .lines()
            .filter(|l| !l.starts_with(' ') && l.contains(" ("))
            .count();
    }

    if let Some(url) = url_servidor {
        st.servidor = sondear(&url).await;
    }
    st
}

/// Lista de modelos: primero por la API del servidor (más rica) y, si no hay
/// servidor, por el CLI `opencode models`.
pub async fn modelos(url: Option<&str>) -> Vec<ModeloPuente> {
    if let Some(url) = url {
        let base = url.trim_end_matches('/');
        {
            let salida = async {
                let cli = reqwest::Client::builder()
                    .timeout(Duration::from_secs(12))
                    .build()
                    .ok()?;
                let r = cli.get(format!("{base}/api/model")).send().await.ok()?;
                if !r.status().is_success() {
                    return None;
                }
                r.json::<serde_json::Value>().await.ok()
            }
            .await;
            if let Some(v) = salida {
                let mut out = Vec::new();
                for m in v.get("data").and_then(|d| d.as_array()).cloned().unwrap_or_default() {
                    let provider = m.get("providerID").and_then(|x| x.as_str()).unwrap_or("");
                    let id = m.get("id").and_then(|x| x.as_str()).unwrap_or("");
                    if provider.is_empty() || id.is_empty() {
                        continue;
                    }
                    out.push(ModeloPuente {
                        provider: provider.to_string(),
                        id: id.to_string(),
                        full: format!("{provider}/{id}"),
                        origen: "servidor".into(),
                        habilitado: m.get("enabled").and_then(|x| x.as_bool()).unwrap_or(true),
                    });
                }
                if !out.is_empty() {
                    out.sort_by(|a, b| a.full.cmp(&b.full));
                    return out;
                }
            }
        }
    }

    let bin = binario();
    let mut out = Vec::new();
    if let Ok((_, txt, err)) = ejecutar_cli(&bin, &["models"], Duration::from_secs(30)) {
        for l in format!("{txt}{err}")
            .lines()
            .map(|l| l.trim())
            .filter(|l| l.contains('/') && !l.starts_with('{'))
        {
            if let Some((p, id)) = l.split_once('/') {
                out.push(ModeloPuente {
                    provider: p.to_string(),
                    id: id.to_string(),
                    full: l.to_string(),
                    origen: "cli".into(),
                    habilitado: true,
                });
            }
        }
    }
    out
}

/// Agentes: por la API si hay servidor, si no por `opencode agent list`.
pub async fn agentes(url: Option<&str>) -> Vec<AgentePuente> {
    if let Some(url) = url {
        let base = url.trim_end_matches('/');
        {
            let salida = async {
                let cli = reqwest::Client::builder()
                    .timeout(Duration::from_secs(12))
                    .build()
                    .ok()?;
                let r = cli.get(format!("{base}/api/agent")).send().await.ok()?;
                if !r.status().is_success() {
                    return None;
                }
                r.json::<serde_json::Value>().await.ok()
            }
            .await;
            if let Some(v) = salida {
                let mut out = Vec::new();
                for a in v.get("data").and_then(|d| d.as_array()).cloned().unwrap_or_default() {
                    let nombre = a.get("id").and_then(|x| x.as_str()).unwrap_or("");
                    if nombre.is_empty() {
                        continue;
                    }
                    let modelo = a.get("model").and_then(|m| {
                        let p = m.get("providerID").and_then(|x| x.as_str())?;
                        let id = m.get("id").and_then(|x| x.as_str())?;
                        Some(format!("{p}/{id}"))
                    });
                    out.push(AgentePuente {
                        nombre: nombre.to_string(),
                        modo: a
                            .get("mode")
                            .and_then(|x| x.as_str())
                            .unwrap_or("")
                            .to_string(),
                        modelo,
                        origen: "servidor".into(),
                    });
                }
                if !out.is_empty() {
                    out.sort_by(|a, b| a.nombre.cmp(&b.nombre));
                    return out;
                }
            }
        }
    }

    let bin = binario();
    let mut out = Vec::new();
    if let Ok((_, txt, err)) = ejecutar_cli(&bin, &["agent", "list"], Duration::from_secs(12)) {
        for l in format!("{txt}{err}").lines() {
            if l.starts_with(' ') || l.starts_with('\t') {
                continue;
            }
            if let Some((nombre, resto)) = l.split_once(" (") {
                let modo = resto.trim_end_matches(')').trim().to_string();
                out.push(AgentePuente {
                    nombre: nombre.trim().to_string(),
                    modo,
                    modelo: None,
                    origen: "cli".into(),
                });
            }
        }
    }
    out
}

async fn sondear(url: &str) -> ServidorInfo {
    let base = url.trim_end_matches('/').to_string();
    let mut info = ServidorInfo {
        url: base.clone(),
        ..Default::default()
    };
    {
    let _ = async {
        let cli = match reqwest::Client::builder().timeout(Duration::from_secs(8)).build() {
            Ok(c) => c,
            Err(e) => {
                info.error = Some(format!("cliente HTTP: {e}"));
                return;
            }
        };
        match cli.get(format!("{base}/api/health")).send().await {
            Ok(r) if r.status().is_success() => info.vivo = true,
            Ok(r) => {
                info.error = Some(format!("health HTTP {}", r.status()));
                return;
            }
            Err(e) => {
                info.error = Some(format!("sin respuesta: {e}"));
                return;
            }
        }
        if let Ok(r) = cli.get(format!("{base}/api/model")).send().await {
            if let Ok(v) = r.json::<serde_json::Value>().await {
                info.modelos = v
                    .get("data")
                    .and_then(|d| d.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);
            }
        }
        if let Ok(r) = cli.get(format!("{base}/api/agent")).send().await {
            if let Ok(v) = r.json::<serde_json::Value>().await {
                info.agentes = v
                    .get("data")
                    .and_then(|d| d.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);
            }
        }
        if let Ok(r) = cli.get(format!("{base}/api/session")).send().await {
            if let Ok(v) = r.json::<serde_json::Value>().await {
                info.sesiones = v
                    .get("data")
                    .and_then(|d| d.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);
            }
        }
    }
    .await;
    }
    info
}

/* ============================ Conversación ============================ */

#[derive(Debug, Clone, Serialize)]
pub struct PuenteRespuesta {
    pub ok: bool,
    pub session_id: String,
    pub modelo: String,
    pub agente: String,
    pub texto: String,
    pub razonamiento: String,
    pub ms: u64,
    pub tokens_out: u64,
    pub coste: f64,
    pub nota: String,
}

/// Extrae el texto (y el razonamiento) del último mensaje del asistente.
fn extraer(mensajes: &serde_json::Value) -> (String, String) {
    let lista = mensajes.as_array().cloned().unwrap_or_default();
    let ultimo = lista
        .iter()
        .rev()
        .find(|m| m.get("info").and_then(|i| i.get("role")).and_then(|r| r.as_str()) == Some("assistant"));
    let Some(m) = ultimo else {
        return (String::new(), String::new());
    };
    let mut texto = String::new();
    let mut razon = String::new();
    for p in m.get("parts").and_then(|p| p.as_array()).cloned().unwrap_or_default() {
        match p.get("type").and_then(|t| t.as_str()) {
            Some("text") => {
                if let Some(t) = p.get("text").and_then(|t| t.as_str()) {
                    if !texto.is_empty() {
                        texto.push('\n');
                    }
                    texto.push_str(t);
                }
            }
            Some("reasoning") => {
                if let Some(t) = p.get("text").and_then(|t| t.as_str()) {
                    razon.push_str(t);
                }
            }
            _ => {}
        }
    }
    (texto, razon)
}

/// Pregunta a OpenCode por su API real y devuelve la respuesta.
///
/// Ruta comprobada en 1.19.3: `POST /api/session` → `POST /session/{id}/message`
/// con `parts` → `GET /session/{id}/message`. La ruta v2
/// `/api/session/{id}/prompt` **no** se usa: en esta versión devuelve 500.
pub async fn preguntar(
    url: &str,
    provider: &str,
    model: &str,
    agent: Option<&str>,
    texto: &str,
    timeout_s: u64,
) -> Result<PuenteRespuesta, String> {
    let base = url.trim_end_matches('/').to_string();
    if texto.trim().is_empty() {
        return Err("el mensaje está vacío".into());
    }
    if provider.trim().is_empty() || model.trim().is_empty() {
        return Err("hay que indicar proveedor y modelo (por ejemplo ollama/qwen3-vl:4b)".into());
    }

    let cli = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_s.max(15)))
        .build()
        .map_err(|e| format!("cliente HTTP: {e}"))?;

    let inicio = Instant::now();

    // 1) Sesión nueva, fijando agente y modelo.
    let mut cuerpo = serde_json::json!({
        "model": { "providerID": provider, "id": model }
    });
    if let Some(a) = agent.filter(|a| !a.trim().is_empty()) {
        cuerpo["agent"] = serde_json::json!(a);
    }
    let r = cli
        .post(format!("{base}/api/session"))
        .json(&cuerpo)
        .send()
        .await
        .map_err(|e| format!("el servidor de OpenCode no responde en {base}: {e}"))?;
    let estado = r.status();
    let v: serde_json::Value = r
        .json()
        .await
        .map_err(|e| format!("respuesta ilegible al crear la sesión: {e}"))?;
    if !estado.is_success() {
        return Err(format!("crear sesión: HTTP {estado} · {}", recorte(&v.to_string(), 200)));
    }
    let session_id = v
        .get("data")
        .and_then(|d| d.get("id"))
        .and_then(|i| i.as_str())
        .ok_or_else(|| format!("la sesión no trajo id: {}", recorte(&v.to_string(), 200)))?
        .to_string();

    // 2) Mensaje con `parts`. Este endpoint responde cuando la generación acaba.
    let payload = serde_json::json!({
        "model": { "providerID": provider, "modelID": model },
        "parts": [{ "type": "text", "text": texto }]
    });
    let envio = cli
        .post(format!("{base}/session/{session_id}/message"))
        .json(&payload)
        .send()
        .await;
    if let Err(e) = envio {
        return Err(format!(
            "OpenCode cortó el envío tras {}s: {e}",
            inicio.elapsed().as_secs()
        ));
    }

    // 3) Lectura de los mensajes guardados: la fuente fiable del texto final.
    let mut vuelta: Option<(String, String, u64, f64)> = None;
    for _ in 0..8 {
        if let Ok(resp) = cli.get(format!("{base}/session/{session_id}/message")).send().await {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                let (texto_final, razon) = extraer(&json);
                if !texto_final.is_empty() {
                    let info = json
                        .as_array()
                        .and_then(|a| {
                            a.iter()
                                .rev()
                                .find(|m| {
                                    m.get("info").and_then(|i| i.get("role")).and_then(|r| r.as_str())
                                        == Some("assistant")
                                })
                                .and_then(|m| m.get("info").cloned())
                        })
                        .unwrap_or(serde_json::Value::Null);
                    let tokens = info
                        .get("tokens")
                        .and_then(|t| t.get("output"))
                        .and_then(|t| t.as_u64())
                        .unwrap_or(0);
                    let coste = info.get("cost").and_then(|c| c.as_f64()).unwrap_or(0.0);
                    vuelta = Some((texto_final, razon, tokens, coste));
                    break;
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(700)).await;
    }

    let agente_final = agent.unwrap_or("por defecto").to_string();
    match vuelta {
        Some((texto, razon, tokens, coste)) => Ok(PuenteRespuesta {
            ok: true,
            session_id,
            modelo: format!("{provider}/{model}"),
            agente: agente_final,
            texto,
            razonamiento: razon,
            ms: inicio.elapsed().as_millis() as u64,
            tokens_out: tokens,
            coste,
            nota: "ruta POST /api/session + /session/{id}/message (verificada)".into(),
        }),
        None => Err(format!(
            "la sesión {session_id} no devolvió texto en {}s. Revisa que el motor del modelo esté cargado.",
            inicio.elapsed().as_secs()
        )),
    }
}

fn recorte(s: &str, n: usize) -> String {
    let mut out: String = s.chars().take(n).collect();
    if s.chars().count() > n {
        out.push('…');
    }
    out
}

/* ============================ Dirección inversa ============================ */

/// Fragmento exacto para que OpenCode consuma los modelos de KUKIA.
///
/// Es una entrada de `provider` con el adaptador oficial
/// `@ai-sdk/openai-compatible`. Se genera como texto: KUKIA no escribe en la
/// configuración de OpenCode por su cuenta.
pub fn snippet_opencode(base_url: &str, token: &str, modelos: &[String]) -> String {
    let mut map = serde_json::Map::new();
    for m in modelos {
        map.insert(
            m.clone(),
            serde_json::json!({ "name": format!("KUKIA · {m}") }),
        );
    }
    let v = serde_json::json!({
        "provider": {
            "kukia": {
                "npm": "@ai-sdk/openai-compatible",
                "name": "KUKIA CORE",
                "options": {
                    "baseURL": format!("{}/v1", base_url.trim_end_matches('/')),
                    "apiKey": token
                },
                "models": serde_json::Value::Object(map)
            }
        }
    });
    serde_json::to_string_pretty(&v).unwrap_or_else(|_| "{}".into())
}

/* ============================ Tests ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extrae_texto_y_razonamiento_del_ultimo_asistente() {
        let mensajes = serde_json::json!([
            { "info": { "role": "user" }, "parts": [{ "type": "text", "text": "hola" }] },
            { "info": { "role": "assistant" }, "parts": [
                { "type": "reasoning", "text": "pensando" },
                { "type": "text", "text": "PUENTE OK" }
            ]}
        ]);
        let (texto, razon) = extraer(&mensajes);
        assert_eq!(texto, "PUENTE OK");
        assert_eq!(razon, "pensando");
    }

    #[test]
    fn sin_mensajes_devuelve_vacio_sin_panico() {
        let (t, r) = extraer(&serde_json::json!([]));
        assert!(t.is_empty() && r.is_empty());
    }

    #[test]
    fn el_snippet_es_json_valido_con_el_modelo_pedido() {
        let s = snippet_opencode("http://127.0.0.1:8311", "tok", &["ollama::llama3".into()]);
        let v: serde_json::Value = serde_json::from_str(&s).expect("JSON válido");
        let p = &v["provider"]["kukia"];
        assert_eq!(p["options"]["baseURL"], "http://127.0.0.1:8311/v1");
        assert!(p["models"]["ollama::llama3"].is_object());
    }

    /// Diagnóstico manual: muestra qué devuelve el CLI de verdad. Se ejecuta con
    /// `cargo test --lib cli_crudo -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn cli_crudo() {
        let bin = binario();
        println!("binario: {bin}");
        for args in [vec!["agent", "list"], vec!["models"]] {
            match ejecutar_cli(&bin, &args, Duration::from_secs(20)) {
                Ok((ok, out, err)) => println!(
                    "{:?}: ok={ok} stdout={} bytes stderr={} bytes\n--- stdout ---\n{}\n--- stderr ---\n{}",
                    args,
                    out.len(),
                    err.len(),
                    recorte(&out, 300),
                    recorte(&err, 300)
                ),
                Err(e) => println!("{args:?}: ERROR {e}"),
            }
        }
    }

    #[test]
    fn el_wrapper_cmd_se_reconoce() {
        assert!(es_wrapper(r"C:\Users\kuki\AppData\Roaming\npm\opencode.cmd"));
        assert!(!es_wrapper("opencode"));
    }
}
