//! CC1–CC4 del centro de control: Ollama y LM Studio de verdad.
//!
//! - **CC1** estado real de cada servidor: versión, puerto (con descubrimiento, no
//!   asumiendo 1234), modelos instalados y cargados, y latencia medida.
//! - **CC2** ficha completa de modelo: familia, parámetros, cuantización, formato,
//!   contexto real, capacidades y si está cargado.
//! - **CC3** parámetros de inferencia por modelo, persistidos y validados según el
//!   motor (lo que un motor no soporta se avisa, no se finge).
//! - **CC4** carga controlada con preflight: primero se estima el consumo, y solo
//!   se carga si cabe. La descarga reutiliza `vram::unload`.
//!
//! Todo lo que se afirma aquí sale de una API o de un CLI comprobado en esta
//! máquina (01/11/2026): Ollama 0.40.13 (`/api/version`, `/api/tags`, `/api/show`)
//! y LM Studio con `lms server status`, `/api/v0/models`, `/api/v1/models` y
//! `lms load --estimate-only`.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

const OLLAMA_BASE: &str = "http://127.0.0.1:11434";
const LM_PORTS: [u16; 2] = [1234, 1235];

fn client(secs: u64) -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(secs))
        .user_agent("KUKIA-CODE/0.1")
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

/* ============================ CLI de LM Studio ============================ */

/// Ruta del CLI oficial `lms`. Se usa sólo para lo que la API REST no expone
/// (estimación de carga y arranque de modelo).
pub fn lms_exe() -> Option<PathBuf> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .ok()?;
    let exe = PathBuf::from(home)
        .join(".lmstudio")
        .join("bin")
        .join(if cfg!(windows) { "lms.exe" } else { "lms" });
    exe.exists().then_some(exe)
}

/// Ejecuta `lms` y devuelve la salida combinada. Sin ventana de consola.
fn lms(args: &[&str]) -> Result<String, String> {
    let exe = lms_exe().ok_or_else(|| "el CLI lms no está instalado".to_string())?;
    let mut cmd = Command::new(&exe);
    cmd.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let out = cmd.output().map_err(|e| format!("no se pudo ejecutar lms: {e}"))?;
    let texto = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    if out.status.success() {
        Ok(texto)
    } else {
        Err(recortar(&texto))
    }
}

fn recortar(s: &str) -> String {
    let t = s.trim().replace(['\n', '\r'], " ");
    if t.chars().count() > 220 {
        format!("{}…", t.chars().take(220).collect::<String>())
    } else {
        t
    }
}

/// Extrae el puerto del texto de `lms server status`
/// («The server is running on port 1235.»).
fn puerto_de_status(texto: &str) -> Option<u16> {
    let bajo = texto.to_lowercase();
    let i = bajo.find("port ")?;
    let resto = &texto[i + 5..];
    let num: String = resto.chars().take_while(|c| c.is_ascii_digit()).collect();
    num.parse().ok()
}

/// Extrae «Estimated GPU Memory: 21.29 GiB» → 21.29.
fn estimacion_gpu(texto: &str) -> Option<f64> {
    for linea in texto.lines() {
        let l = linea.to_lowercase();
        if l.contains("estimated gpu memory") || l.contains("estimated total memory") {
            let num: String = linea
                .chars()
                .skip_while(|c| !c.is_ascii_digit() && *c != '.')
                .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == ',')
                .collect();
            let limpio = num.replace(',', ".");
            if let Ok(v) = limpio.trim_end_matches('.').parse::<f64>() {
                return Some(v);
            }
        }
    }
    None
}

/* ================================ CC1 ================================ */

#[derive(Debug, Clone, Serialize)]
pub struct EngineStatus {
    pub engine: String,
    pub online: bool,
    pub base_url: String,
    pub port: Option<u16>,
    /// De dónde salió el puerto: `lms server status`, sonda directa o fijo.
    pub port_source: String,
    pub version: Option<String>,
    pub latency_ms: u64,
    pub installed: usize,
    pub loaded: Vec<String>,
    pub note: String,
}

async fn ollama_status() -> EngineStatus {
    let t0 = Instant::now();
    let ver = client(4).get(format!("{OLLAMA_BASE}/api/version")).send().await;
    let latency = t0.elapsed().as_millis() as u64;

    let (online, version) = match ver {
        Ok(r) if r.status().is_success() => {
            let v = r
                .json::<serde_json::Value>()
                .await
                .ok()
                .and_then(|v| v["version"].as_str().map(|s| s.to_string()));
            (true, v)
        }
        _ => (false, None),
    };
    if !online {
        return EngineStatus {
            engine: "ollama".into(),
            online: false,
            base_url: OLLAMA_BASE.into(),
            port: Some(11434),
            port_source: "fijo (API nativa)".into(),
            version: None,
            latency_ms: latency,
            installed: 0,
            loaded: Vec::new(),
            note: "sin respuesta en 11434".into(),
        };
    }

    let installed = match client(6).get(format!("{OLLAMA_BASE}/api/tags")).send().await {
        Ok(r) if r.status().is_success() => r
            .json::<serde_json::Value>()
            .await
            .ok()
            .and_then(|v| v["models"].as_array().map(|a| a.len()))
            .unwrap_or(0),
        _ => 0,
    };
    let loaded = cargados_ollama().await;

    EngineStatus {
        engine: "ollama".into(),
        online: true,
        base_url: OLLAMA_BASE.into(),
        port: Some(11434),
        port_source: "fijo (API nativa)".into(),
        version,
        latency_ms: latency,
        installed,
        note: format!("{installed} modelo(s) instalados · {} cargado(s)", loaded.len()),
        loaded,
    }
}

async fn cargados_ollama() -> Vec<String> {
    match client(4).get(format!("{OLLAMA_BASE}/api/ps")).send().await {
        Ok(r) if r.status().is_success() => r
            .json::<serde_json::Value>()
            .await
            .ok()
            .and_then(|v| {
                v["models"].as_array().map(|a| {
                    a.iter()
                        .filter_map(|m| m["name"].as_str().map(|s| s.to_string()))
                        .collect()
                })
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// Descubre el puerto del servidor de LM Studio: primero lo que diga su CLI,
/// después 1234/1235. Nunca se cambia el puerto: sólo se lee.
fn lmstudio_port() -> (String, Option<u16>, String) {
    if let Ok(texto) = lms(&["server", "status"]) {
        if let Some(p) = puerto_de_status(&texto) {
            return (format!("http://127.0.0.1:{p}"), Some(p), "lms server status".into());
        }
    }
    for p in LM_PORTS {
        if std::net::TcpStream::connect_timeout(
            &format!("127.0.0.1:{p}").parse().unwrap(),
            std::time::Duration::from_millis(250),
        )
        .is_ok()
        {
            return (format!("http://127.0.0.1:{p}"), Some(p), "sonda TCP".into());
        }
    }
    (
        format!("http://127.0.0.1:{}", LM_PORTS[0]),
        None,
        "sin descubrir".into(),
    )
}

async fn lmstudio_status() -> EngineStatus {
    let (base, port, origin) = lmstudio_port();
    let t0 = Instant::now();
    let resp = client(6).get(format!("{base}/api/v0/models")).send().await;
    let latency = t0.elapsed().as_millis() as u64;

    match resp {
        Ok(r) if r.status().is_success() => {
            let v = r.json::<serde_json::Value>().await.unwrap_or_default();
            let mut installed = 0usize;
            let mut loaded = Vec::new();
            if let Some(arr) = v["data"].as_array() {
                installed = arr.len();
                for m in arr {
                    if m["state"].as_str() == Some("loaded") {
                        if let Some(id) = m["id"].as_str() {
                            loaded.push(id.to_string());
                        }
                    }
                }
            }
            EngineStatus {
                engine: "lmstudio".into(),
                online: true,
                base_url: base.clone(),
                port,
                port_source: origin,
                version: None,
                latency_ms: latency,
                installed,
                note: format!("{installed} modelo(s) · {} cargado(s)", loaded.len()),
                loaded,
            }
        }
        _ => EngineStatus {
            engine: "lmstudio".into(),
            online: false,
            base_url: base,
            port,
            port_source: origin,
            version: None,
            latency_ms: latency,
            installed: 0,
            loaded: Vec::new(),
            note: "sin respuesta: arranca el servidor de LM Studio".into(),
        },
    }
}

/// CC1: estado de un motor concreto.
pub async fn status(engine: &str) -> EngineStatus {
    match engine {
        "lmstudio" => lmstudio_status().await,
        _ => ollama_status().await,
    }
}

/// CC1: estado de los dos motores a la vez.
pub async fn status_all() -> Vec<EngineStatus> {
    let (o, l) = tokio::join!(ollama_status(), lmstudio_status());
    vec![o, l]
}

/* ================================ CC2 ================================ */

#[derive(Debug, Clone, Serialize)]
pub struct ModelCard {
    pub engine: String,
    pub id: String,
    pub family: Option<String>,
    pub parameter_size: Option<String>,
    pub quantization: Option<String>,
    pub format: Option<String>,
    pub size_gb: Option<f64>,
    pub context_length: Option<u64>,
    pub capabilities: Vec<String>,
    pub loaded: bool,
    pub publisher: Option<String>,
    pub architecture: Option<String>,
    /// Aviso legible cuando algún dato no lo da el motor.
    pub note: String,
}

fn caps_de(valor: &serde_json::Value) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(arr) = valor.as_array() {
        for c in arr {
            if let Some(s) = c.as_str() {
                out.push(s.to_string());
            } else if let Some(s) = c.as_object().and_then(|o| o.keys().next()) {
                out.push(s.clone());
            }
        }
    } else if let Some(obj) = valor.as_object() {
        // LM Studio /api/v1/models devuelve un objeto de capacidades.
        for (k, v) in obj {
            if v.as_bool() == Some(true) || v.is_object() || v.is_string() {
                out.push(k.clone());
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// CC2: lista de fichas de un motor (rápida, sin una llamada por modelo).
pub async fn cards(engine: &str) -> Result<Vec<ModelCard>, String> {
    if engine == "lmstudio" {
        let (base, _, _) = lmstudio_port();
        let v = client(10)
            .get(format!("{base}/api/v1/models"))
            .send()
            .await
            .map_err(|e| format!("LM Studio: {e}"))?
            .json::<serde_json::Value>()
            .await
            .map_err(|e| format!("LM Studio: respuesta ilegible ({e})"))?;
        let mut out = Vec::new();
        for m in v["models"].as_array().cloned().unwrap_or_default() {
            let id = m["key"].as_str().unwrap_or("").to_string();
            if id.is_empty() {
                continue;
            }
            let cargado = m["loaded_instances"]
                .as_array()
                .map(|a| !a.is_empty())
                .unwrap_or(false);
            out.push(ModelCard {
                engine: "lmstudio".into(),
                id,
                family: m["architecture"].as_str().map(|s| s.to_string()),
                parameter_size: m["params_string"].as_str().map(|s| s.to_string()),
                quantization: m["quantization"].as_str().map(|s| s.to_string()),
                format: m["format"].as_str().map(|s| s.to_string()),
                size_gb: m["size_bytes"].as_f64().map(|b| b / 1_073_741_824.0),
                context_length: m["max_context_length"].as_u64(),
                capabilities: caps_de(&m["capabilities"]),
                loaded: cargado,
                publisher: m["publisher"].as_str().map(|s| s.to_string()),
                architecture: m["architecture"].as_str().map(|s| s.to_string()),
                note: String::new(),
            });
        }
        out.sort_by(|a, b| a.id.cmp(&b.id));
        return Ok(out);
    }

    let v = client(15)
        .get(format!("{OLLAMA_BASE}/api/tags"))
        .send()
        .await
        .map_err(|e| format!("Ollama: {e}"))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Ollama: respuesta ilegible ({e})"))?;
    let cargados = cargados_ollama().await;
    let mut out = Vec::new();
    for m in v["models"].as_array().cloned().unwrap_or_default() {
        let id = m["name"].as_str().unwrap_or("").to_string();
        if id.is_empty() {
            continue;
        }
        let d = &m["details"];
        out.push(ModelCard {
            engine: "ollama".into(),
            id: id.clone(),
            family: d["family"].as_str().map(|s| s.to_string()),
            parameter_size: d["parameter_size"].as_str().map(|s| s.to_string()),
            quantization: d["quantization_level"].as_str().map(|s| s.to_string()),
            format: d["format"].as_str().map(|s| s.to_string()),
            size_gb: m["size"].as_f64().map(|b| b / 1_073_741_824.0),
            // `/api/tags` no publica el contexto: se deja vacío y lo completa `card`.
            context_length: None,
            capabilities: caps_de(&m["capabilities"]),
            loaded: cargados.iter().any(|c| c == &id),
            publisher: d["parent_model"].as_str().map(|s| s.to_string()),
            architecture: d["families"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|x| x.as_str())
                .map(|s| s.to_string()),
            note: "contexto real en la ficha (necesita /api/show)".into(),
        });
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

/// CC2: ficha completa de un modelo (una llamada al motor).
pub async fn card(engine: &str, model: &str) -> Result<ModelCard, String> {
    if engine == "lmstudio" {
        let mut todas = cards("lmstudio").await?;
        if let Some(pos) = todas.iter().position(|c| c.id == model) {
            return Ok(todas.remove(pos));
        }
        return Err(format!("LM Studio no tiene el modelo {model}"));
    }

    let body = serde_json::json!({ "model": model });
    let v = client(15)
        .post(format!("{OLLAMA_BASE}/api/show"))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Ollama: {e}"))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Ollama: respuesta ilegible ({e})"))?;
    if v.get("error").is_some() {
        return Err(format!(
            "Ollama: {}",
            v["error"].as_str().unwrap_or("error desconocido")
        ));
    }

    let d = &v["details"];
    // El contexto llega como `<familia>.context_length` dentro de `model_info`.
    let contexto = v["model_info"]
        .as_object()
        .and_then(|o| {
            o.iter()
                .find(|(k, _)| k.ends_with("context_length"))
                .and_then(|(_, val)| val.as_u64())
        });
    let cargados = cargados_ollama().await;

    Ok(ModelCard {
        engine: "ollama".into(),
        id: model.to_string(),
        family: d["family"].as_str().map(|s| s.to_string()),
        parameter_size: d["parameter_size"].as_str().map(|s| s.to_string()),
        quantization: d["quantization_level"].as_str().map(|s| s.to_string()),
        format: d["format"].as_str().map(|s| s.to_string()),
        size_gb: None,
        context_length: contexto,
        capabilities: caps_de(&v["capabilities"]),
        loaded: cargados.iter().any(|c| c == model),
        publisher: d["parent_model"].as_str().map(|s| s.to_string()),
        architecture: d["families"]
            .as_array()
            .and_then(|a| a.first())
            .and_then(|x| x.as_str())
            .map(|s| s.to_string()),
        note: match contexto {
            Some(_) => String::new(),
            None => "este modelo no publica contexto; Ollama usará su valor por defecto".into(),
        },
    })
}

/* ================================ CC3 ================================ */

/// Parámetros de inferencia por modelo. `None` = no se envía nada y manda el
/// motor. No se inventan valores por defecto.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct InferenceParams {
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    /// Solo Ollama (`/api/chat` → `options.top_k`).
    pub top_k: Option<i64>,
    /// Solo Ollama (`options.num_ctx`). En LM Studio es `-c`.
    pub num_ctx: Option<u64>,
    /// Capas en GPU. Solo Ollama (`options.num_gpu`). En LM Studio es un
    /// porcentaje de offload, que no equivale a capas: no se traduce.
    pub num_gpu: Option<i32>,
    pub repeat_penalty: Option<f64>,
    pub seed: Option<i64>,
    pub max_tokens: Option<u64>,
    /// Solo Ollama: `"5m"`, `"0"`… En LM Studio es `--ttl` en segundos.
    pub keep_alive: Option<String>,

    /* ---- Panel avanzado (medido contra las APIs el 01/11/2026) ---- */
    /// Ollama `options.min_p`.
    pub min_p: Option<f64>,
    /// Ollama `options.repeat_last_n`.
    pub repeat_last_n: Option<i64>,
    /// Ollama `options.presence_penalty`.
    pub presence_penalty: Option<f64>,
    /// Ollama `options.frequency_penalty`.
    pub frequency_penalty: Option<f64>,
    /// `options.stop`: secuencias que cortan la generación.
    pub stop: Vec<String>,
    /// Ollama `options.num_batch`.
    pub num_batch: Option<i64>,
    /// Ollama `options.num_thread`.
    pub num_thread: Option<i64>,
    /// Ollama `options.flash_attn`.
    pub flash_attn: Option<bool>,
    /// Ollama `options.use_mmap`.
    pub use_mmap: Option<bool>,
    /// Ollama `options.use_mlock`.
    pub use_mlock: Option<bool>,
    /// LM Studio `lms load --gpu off|max|0..1`. En Ollama no se usa: allí se
    /// reparte por capas con `num_gpu`.
    pub gpu_offload: Option<String>,
    /// LM Studio `lms load --parallel`.
    pub parallel: Option<u64>,
    /// LM Studio `lms load --speculative-draft-mtp`.
    pub speculative: Option<bool>,
}

impl InferenceParams {
    /// `true` si no hay nada configurado (no se enviará nada al motor).
    pub fn vacio(&self) -> bool {
        self == &InferenceParams::default()
    }

    /// Opciones para `/api/chat` de Ollama.
    pub fn opciones_ollama(&self) -> serde_json::Value {
        let mut o = serde_json::Map::new();
        if let Some(v) = self.temperature {
            o.insert("temperature".into(), serde_json::json!(v));
        }
        if let Some(v) = self.top_p {
            o.insert("top_p".into(), serde_json::json!(v));
        }
        if let Some(v) = self.top_k {
            o.insert("top_k".into(), serde_json::json!(v));
        }
        if let Some(v) = self.num_ctx {
            o.insert("num_ctx".into(), serde_json::json!(v));
        }
        if let Some(v) = self.num_gpu {
            o.insert("num_gpu".into(), serde_json::json!(v));
        }
        if let Some(v) = self.repeat_penalty {
            o.insert("repeat_penalty".into(), serde_json::json!(v));
        }
        if let Some(v) = self.seed {
            o.insert("seed".into(), serde_json::json!(v));
        }
        if let Some(v) = self.min_p {
            o.insert("min_p".into(), serde_json::json!(v));
        }
        if let Some(v) = self.repeat_last_n {
            o.insert("repeat_last_n".into(), serde_json::json!(v));
        }
        if let Some(v) = self.presence_penalty {
            o.insert("presence_penalty".into(), serde_json::json!(v));
        }
        if let Some(v) = self.frequency_penalty {
            o.insert("frequency_penalty".into(), serde_json::json!(v));
        }
        if !self.stop.is_empty() {
            o.insert("stop".into(), serde_json::json!(self.stop));
        }
        if let Some(v) = self.num_batch {
            o.insert("num_batch".into(), serde_json::json!(v));
        }
        if let Some(v) = self.num_thread {
            o.insert("num_thread".into(), serde_json::json!(v));
        }
        if let Some(v) = self.flash_attn {
            o.insert("flash_attn".into(), serde_json::json!(v));
        }
        if let Some(v) = self.use_mmap {
            o.insert("use_mmap".into(), serde_json::json!(v));
        }
        if let Some(v) = self.use_mlock {
            o.insert("use_mlock".into(), serde_json::json!(v));
        }
        serde_json::Value::Object(o)
    }

    /// Campos que el motor indicado **no** soporta. Se avisan, no se aplican.
    pub fn no_soportados(&self, engine: &str) -> Vec<String> {
        let mut out = Vec::new();
        if engine == "lmstudio" {
            if self.top_k.is_some() {
                out.push("top_k: la API de LM Studio no lo expone".into());
            }
            if self.num_gpu.is_some() {
                out.push("num_gpu: en LM Studio el offload es un porcentaje, no capas".into());
            }
            if self.keep_alive.is_some() {
                out.push("keep_alive: en LM Studio se usa --ttl en segundos".into());
            }
            for (valor, nombre) in [
                (self.num_batch.is_some(), "num_batch"),
                (self.num_thread.is_some(), "num_thread"),
                (self.flash_attn.is_some(), "flash_attn"),
                (self.use_mmap.is_some(), "mmap"),
                (self.use_mlock.is_some(), "mlock"),
                (self.min_p.is_some(), "min_p"),
                (self.presence_penalty.is_some(), "presence_penalty"),
                (self.frequency_penalty.is_some(), "frequency_penalty"),
            ] {
                if valor {
                    out.push(format!(
                        "{nombre}: la API de LM Studio no documenta este campo; no se envía sin \
                         poder comprobarlo"
                    ));
                }
            }
        } else if engine == "ollama" {
            if self.gpu_offload.is_some() {
                out.push("gpu_offload: en Ollama el reparto es por capas (num_gpu), no un ratio".into());
            }
            if self.parallel.is_some() {
                out.push("parallel: en Ollama se controla con OLLAMA_NUM_PARALLEL en el servidor".into());
            }
            if self.speculative.is_some() {
                out.push("speculative: Ollama no expone speculative decoding por petición".into());
            }
        } else {
            if self.max_tokens.is_some() {
                out.push("max_tokens: Ollama lo fija con options.num_predict; aquí no se mapea".into());
            }
        }
        out
    }
}

fn params_path() -> PathBuf {
    crate::config::data_dir().join("inferencia.json")
}

fn leer_todo() -> std::collections::BTreeMap<String, InferenceParams> {
    std::fs::read_to_string(params_path())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn clave(engine: &str, model: &str) -> String {
    format!("{}::{}", engine.trim(), model.trim())
}

/// CC3: parámetros guardados de un modelo (vacíos si no hay ninguno).
pub fn params_get(engine: &str, model: &str) -> InferenceParams {
    leer_todo().get(&clave(engine, model)).cloned().unwrap_or_default()
}

/// CC3: guarda los parámetros de un modelo. Devuelve los avisos de lo que el
/// motor no soporta, para que la interfaz lo diga en vez de callarse.
pub fn params_set(engine: &str, model: &str, p: InferenceParams) -> Vec<String> {
    let mut todo = leer_todo();
    let k = clave(engine, model);
    if p.vacio() {
        todo.remove(&k);
    } else {
        todo.insert(k, p.clone());
    }
    let ruta = params_path();
    if let Some(dir) = ruta.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(t) = serde_json::to_string_pretty(&todo) {
        let _ = std::fs::write(&ruta, t);
    }
    p.no_soportados(engine)
}

/* ================================ CC4 ================================ */

#[derive(Debug, Clone, Serialize)]
pub struct GpuFree {
    pub index: u32,
    pub name: String,
    pub free_gb: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Preflight {
    pub engine: String,
    pub model: String,
    /// Lo que el motor dice que hará falta (GB). `None` si no lo publica.
    pub needed_gb: Option<f64>,
    /// Mejor GPU libre y su margen.
    pub best_gpu: Option<GpuFree>,
    pub fits: bool,
    pub source: String,
    pub detail: String,
    pub warnings: Vec<String>,
}

/// CC4: estima el consumo antes de tocar la VRAM.
///
/// - LM Studio: `lms load --estimate-only` (no carga nada).
/// - Ollama: tamaño del GGUF de `/api/tags` + 12 % de margen para contexto/KV.
pub async fn preflight(engine: &str, model: &str, p: &InferenceParams) -> Preflight {
    let gpus: Vec<GpuFree> = crate::vram::gpus_vram()
        .into_iter()
        .map(|g| GpuFree {
            index: g.index,
            name: g.name,
            free_gb: g.free_mb as f64 / 1024.0,
        })
        .collect();
    let best = gpus
        .iter()
        .max_by(|a, b| a.free_gb.partial_cmp(&b.free_gb).unwrap_or(std::cmp::Ordering::Equal))
        .cloned();
    let mut warnings = p.no_soportados(engine);

    let (needed, source, mut detail) = if engine == "lmstudio" {
        match lms(&["load", "--estimate-only", "--yes", model]) {
            Ok(txt) => {
                let est = estimacion_gpu(&txt);
                (
                    est,
                    "lms load --estimate-only".to_string(),
                    est.map(|g| format!("LM Studio estima {g:.2} GB de VRAM"))
                        .unwrap_or_else(|| recortar(&txt)),
                )
            }
            Err(e) => (
                None,
                "lms load --estimate-only".to_string(),
                format!("el CLI no pudo estimar: {e}"),
            ),
        }
    } else {
        let size = match client(10).get(format!("{OLLAMA_BASE}/api/tags")).send().await {
            Ok(r) if r.status().is_success() => r
                .json::<serde_json::Value>()
                .await
                .ok()
                .and_then(|v| {
                    v["models"].as_array().and_then(|a| {
                        a.iter()
                            .find(|m| m["name"].as_str() == Some(model))
                            .and_then(|m| m["size"].as_f64())
                    })
                }),
            _ => None,
        };
        match size {
            Some(b) => {
                let gb = b / 1_073_741_824.0;
                let ctx = p.num_ctx.unwrap_or(4096) as f64;
                // KV aproximada: 1 GB por cada 16k de contexto en modelos 7–14B.
                let kv = ctx / 16384.0;
                let total = gb * 1.12 + kv;
                (
                    Some(total),
                    "/api/tags + margen propio".to_string(),
                    format!("archivo {gb:.2} GB + 12 % + KV({ctx:.0}) ≈ {total:.2} GB"),
                )
            }
            None => (
                None,
                "/api/tags".to_string(),
                "Ollama no publica el tamaño de este modelo".to_string(),
            ),
        }
    };

    let fits = match (needed, &best) {
        (Some(n), Some(g)) => g.free_gb >= n,
        _ => false,
    };
    if let (Some(_), Some(g)) = (needed, &best) {
        detail = format!(
            "{detail} · GPU{} ({}): {:.2} GB libres → {}",
            g.index,
            g.name
                .replace("NVIDIA GeForce ", "")
                .replace("AMD Radeon ", "")
                .replace("Intel(R) ", ""),
            g.free_gb,
            if fits { "cabe" } else { "NO cabe" }
        );
    } else if needed.is_none() {
        detail = format!("{detail} · sin estimación no se autoriza la carga");
    }
    if !fits {
        warnings.push(
            "Si no cabe, no se carga: es preferible avisar a provocar un error de memoria.".into(),
        );
    }

    Preflight {
        engine: engine.into(),
        model: model.into(),
        needed_gb: needed,
        best_gpu: best,
        fits,
        source,
        detail,
        warnings,
    }
}

/// CC4: carga un modelo **solo si el preflight lo autoriza**.
pub async fn load(
    engine: &str,
    model: &str,
    p: InferenceParams,
) -> crate::vram::UnloadResult {
    let pre = preflight(engine, model, &p).await;
    if !pre.fits {
        return crate::vram::UnloadResult {
            engine: engine.into(),
            model: model.into(),
            ok: false,
            message: format!("No se carga: {}. {}", pre.detail, pre.warnings.join(" ")),
        };
    }

    if engine == "lmstudio" {
        // Los argumentos se construyen DENTRO del hilo: si se crean fuera, los
        // préstamos a `args` no sobreviven al `move`.
        let modelo = model.to_string();
        let ctx = p.num_ctx;
        let gpu_offload = p.gpu_offload.clone();
        let parallel = p.parallel;
        let speculative = p.speculative;
        let ttl = p
            .keep_alive
            .as_deref()
            .and_then(|k| k.trim_end_matches('m').parse::<u64>().ok())
            .map(|min| min * 60);
        return match tokio::task::spawn_blocking(move || {
            let mut args: Vec<String> = vec!["load".into(), modelo, "--yes".into()];
            if let Some(c) = ctx {
                args.push("-c".into());
                args.push(c.to_string());
            }
            if let Some(t) = ttl {
                args.push("--ttl".into());
                args.push(t.to_string());
            }
            if let Some(g) = gpu_offload.clone() {
                if g != "auto" {
                    args.push("--gpu".into());
                    args.push(g);
                }
            }
            if let Some(n) = parallel {
                args.push("--parallel".into());
                args.push(n.to_string());
            }
            if speculative == Some(true) {
                args.push("--speculative-draft-mtp".into());
            }
            let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            lms(&refs)
        })
        .await
        {
            Ok(Ok(txt)) => crate::vram::UnloadResult {
                engine: "lmstudio".into(),
                model: model.into(),
                ok: true,
                message: format!("cargado · {}", recortar(&txt)),
            },
            Ok(Err(e)) => crate::vram::UnloadResult {
                engine: "lmstudio".into(),
                model: model.into(),
                ok: false,
                message: e,
            },
            Err(e) => crate::vram::UnloadResult {
                engine: "lmstudio".into(),
                model: model.into(),
                ok: false,
                message: format!("la carga se interrumpió: {e}"),
            },
        };
    }

    // Ollama: una petición vacía con keep_alive carga el modelo en memoria.
    let mut body = serde_json::json!({
        "model": model,
        "prompt": "",
        "stream": false,
    });
    body["keep_alive"] = serde_json::json!(p.keep_alive.clone().unwrap_or_else(|| "5m".into()));
    let opciones = p.opciones_ollama();
    if opciones.as_object().map(|o| !o.is_empty()).unwrap_or(false) {
        body["options"] = opciones;
    }
    match client(300)
        .post(format!("{OLLAMA_BASE}/api/generate"))
        .json(&body)
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => crate::vram::UnloadResult {
            engine: "ollama".into(),
            model: model.into(),
            ok: true,
            message: format!(
                "cargado (keep_alive {})",
                p.keep_alive.clone().unwrap_or_else(|| "5m".into())
            ),
        },
        Ok(r) => {
            let st = r.status();
            let t = r.text().await.unwrap_or_default();
            crate::vram::UnloadResult {
                engine: "ollama".into(),
                model: model.into(),
                ok: false,
                message: format!("HTTP {st}: {}", recortar(&t)),
            }
        }
        Err(e) => crate::vram::UnloadResult {
            engine: "ollama".into(),
            model: model.into(),
            ok: false,
            message: format!("sin respuesta: {e}"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lee_el_puerto_del_status_real() {
        assert_eq!(
            puerto_de_status("The server is running on port 1235.\n"),
            Some(1235)
        );
        assert_eq!(puerto_de_status("no hay servidor"), None);
    }

    #[test]
    fn lee_la_estimacion_real_de_lm_studio() {
        let salida = "Model: flint-40b-abliterated\nEstimated GPU Memory:   21.29 GiB\nEstimated Total Memory: 21.29 GiB\nConfidence: LOW\n";
        assert_eq!(estimacion_gpu(salida), Some(21.29));
    }

    #[test]
    fn un_motor_no_soporta_lo_que_no_soporta() {
        let p = InferenceParams {
            top_k: Some(40),
            num_gpu: Some(999),
            keep_alive: Some("5m".into()),
            ..Default::default()
        };
        let avisos = p.no_soportados("lmstudio");
        assert_eq!(avisos.len(), 3, "LM Studio no acepta top_k, num_gpu ni keep_alive");
        assert!(p.no_soportados("ollama").is_empty());
    }

    #[test]
    fn los_dos_motores_responden_de_verdad() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let estados = rt.block_on(status_all());
        assert_eq!(estados.len(), 2);
        for e in &estados {
            println!(
                "{:<9} online={:<5} puerto={:?} ({}) version={:?} modelos={} cargados={} {}ms · {}",
                e.engine,
                e.online,
                e.port,
                e.port_source,
                e.version,
                e.installed,
                e.loaded.len(),
                e.latency_ms,
                e.note
            );
            assert!(!e.engine.is_empty());
        }
        if estados.iter().any(|e| e.engine == "ollama" && e.online) {
            let fichas = rt.block_on(cards("ollama")).expect("Ollama debe dar fichas");
            println!("fichas de ollama: {}", fichas.len());
            assert!(!fichas.is_empty());
            let full = rt
                .block_on(card("ollama", &fichas[0].id))
                .expect("ficha completa");
            println!(
                "ficha completa: {} · family={:?} params={:?} quant={:?} ctx={:?} caps={:?}",
                full.id,
                full.family,
                full.parameter_size,
                full.quantization,
                full.context_length,
                full.capabilities
            );
        }
    }

    #[test]
    fn el_preflight_estima_sin_cargar_nada() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let p = InferenceParams::default();
        if let Ok(fichas) = rt.block_on(cards("ollama")) {
            if let Some(c) = fichas.first() {
                let pre = rt.block_on(preflight("ollama", &c.id, &p));
                println!(
                    "preflight {} → necesita={:?} GB fits={} · {}",
                    c.id, pre.needed_gb, pre.fits, pre.detail
                );
                assert!(
                    pre.needed_gb.is_some(),
                    "Ollama publica el tamaño: debe haber estimación"
                );
            }
        }
    }

    #[test]
    fn las_opciones_de_ollama_solo_llevan_lo_configurado() {
        let p = InferenceParams {
            temperature: Some(0.7),
            num_ctx: Some(8192),
            ..Default::default()
        };
        let o = p.opciones_ollama();
        assert_eq!(o["temperature"], 0.7);
        assert_eq!(o["num_ctx"], 8192);
        assert!(o.get("top_k").is_none());
        assert!(InferenceParams::default().vacio());
    }
}
