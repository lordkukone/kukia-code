use crate::config::AppConfig;
use crate::control;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::UnboundedSender;

/// Bucle de herramientas: ofrecer, recoger, ejecutar y devolver.
pub mod herramientas;

/* ============================ Tipos públicos ============================ */

#[derive(Debug, Clone, Serialize)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub base_url: String,
    pub online: bool,
    pub models_count: usize,
    /// Cuántos de esos modelos son gratuitos (sufijo `-free` del catálogo).
    pub free_models: usize,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
    pub id: String,
    pub provider: String,
    pub name: String,
    pub context: Option<u64>,
    pub size_gb: Option<f64>,
    pub capabilities: Vec<String>,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatRequest {
    pub provider: String,
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub system_prompt: Option<String>,
    /// Usar herramientas durante el chat. Por defecto si: si el modelo no
    /// declara `tools`, no cambia absolutamente nada.
    #[serde(default = "si_por_defecto")]
    pub tools: bool,
    /// Aprobacion explicita para escribir y ejecutar en esta conversacion.
    #[serde(default)]
    pub aprobar: bool,
    /// Agente activo: de el sale la politica de permisos.
    #[serde(default)]
    pub agent: Option<String>,
}

fn si_por_defecto() -> bool {
    true
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatResponse {
    pub content: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ChatEvent {
    Token { text: String },
    /// El modelo ha pedido una herramienta (nombre y argumentos en JSON).
    ToolCall { nombre: String, argumentos: String },
    /// Resultado de esa herramienta, ya ejecutada bajo los permisos del agente.
    ToolResult {
        nombre: String,
        ok: bool,
        texto: String,
    },
    /// Cadena de pensamiento de modelos con razonamiento (DeepSeek-R1, JEV…).
    Reasoning { text: String },
    Done {
        prompt_tokens: u64,
        completion_tokens: u64,
        total_ms: u64,
    },
    Error { message: String },
}

/* ============================ Utilidades ============================ */

/// Modelos que el proveedor sirve gratis: es su propio catálogo el que los marca
/// con el sufijo `-free`. No se adivina por precio ni por nombre comercial.
pub fn es_gratis(id: &str) -> bool {
    let l = id.to_lowercase();
    l.ends_with("-free") || l.contains("-free-") || l.starts_with("free-")
}

fn client(timeout: u64) -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout))
        .user_agent("KUKIA-CODE/0.1")
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

fn base(provider: &str, cfg: &AppConfig) -> String {
    match provider {
        "lmstudio" => "http://127.0.0.1:1234".to_string(),
        // El endpoint lo pone el proveedor cloud activo del catálogo.
        "cloud" => {
            let activo = cfg.active_cloud();
            let url = if activo.base_url.trim().is_empty() {
                cfg.cloud_base_url.clone()
            } else {
                activo.base_url
            };
            url.trim_end_matches('/').to_string()
        }
        _ => "http://127.0.0.1:11434".to_string(),
    }
}

/// Une un endpoint base con una ruta de la API sin duplicar el `/v1`:
/// `https://opencode.ai/zen/v1` + `/chat/completions` → `…/zen/v1/chat/completions`,
/// y `http://127.0.0.1:1235` + `/chat/completions` → `http://127.0.0.1:1235/v1/chat/completions`.
fn join_api(base_url: &str, ruta: &str) -> String {
    let b = base_url.trim_end_matches('/');
    if b.ends_with("/v1") {
        format!("{b}{ruta}")
    } else {
        format!("{b}/v1{ruta}")
    }
}

/// LM Studio no siempre escucha en 1234: el usuario puede cambiarlo desde su
/// interfaz (en esta máquina está en 1235). Se sondean los puertos conocidos
/// y se usa el primero que responda, sin tocar la configuración de LM Studio.
const LMSTUDIO_PORTS: [u16; 2] = [1234, 1235];

async fn probe(url: &str) -> bool {
    client(2)
        .get(url)
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

async fn lmstudio_base() -> String {
    for port in LMSTUDIO_PORTS {
        let b = format!("http://127.0.0.1:{port}");
        if probe(&format!("{b}/v1/models")).await {
            return b;
        }
    }
    format!("http://127.0.0.1:{}", LMSTUDIO_PORTS[0])
}

/// Base efectiva: para LM Studio resuelve el puerto real; el resto, la fija.
async fn effective_base(provider: &str, cfg: &AppConfig) -> String {
    if provider == "lmstudio" {
        lmstudio_base().await
    } else {
        base(provider, cfg)
    }
}

/// Clave del proveedor cloud activo (entorno → secrets.json → auth.json de OpenCode).
fn cloud_key(cfg: &AppConfig) -> Option<String> {
    crate::config::cloud_credential(&cfg.active_cloud().id).0
}

/// OpenCode Go (`/zen/go/v1`) rechaza la petición con `400 MissingSessionID`
/// si no llega `x-opencode-session`. Se genera un id nuevo por petición; el
/// mismo valor se reutiliza en el reintento sin razonamiento.
fn con_sesion(builder: reqwest::RequestBuilder, cfg: &AppConfig, sesion: &str) -> reqwest::RequestBuilder {
    if cfg.active_cloud().session_header {
        builder.header("x-opencode-session", sesion)
    } else {
        builder
    }
}

async fn get_json(url: &str, key: Option<&str>) -> Result<Value, String> {
    let c = client(3);
    let mut req = c.get(url);
    if let Some(k) = key {
        req = req.bearer_auth(k);
    }
    let resp = req.send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    resp.json::<Value>().await.map_err(|e| e.to_string())
}

/* ============================ Proveedores ============================ */

pub async fn list_providers(cfg: &AppConfig) -> Vec<ProviderInfo> {
    let mut out = Vec::new();

    // Ollama
    let ollama_base = "http://127.0.0.1:11434";
    let (online, count, note) = match get_json(&format!("{ollama_base}/api/tags"), None).await {
        Ok(v) => {
            let n = v["models"].as_array().map(|a| a.len()).unwrap_or(0);
            (true, n, String::new())
        }
        Err(e) => (false, 0, format!("sin respuesta: {e}")),
    };
    out.push(ProviderInfo {
        id: "ollama".into(),
        name: "Ollama".into(),
        kind: "local · API nativa".into(),
        base_url: ollama_base.into(),
        online,
        models_count: count,
        free_models: 0,
        note,
    });

    // LM Studio
    let lm_base = lmstudio_base().await;
    let (online, count, note) = match get_json(&format!("{lm_base}/v1/models"), None).await {
        Ok(v) => {
            let n = v["data"].as_array().map(|a| a.len()).unwrap_or(0);
            (true, n, String::new())
        }
        Err(e) => (
            false,
            0,
            format!("servidor apagado o no accesible ({e})"),
        ),
    };
    out.push(ProviderInfo {
        id: "lmstudio".into(),
        name: "LM Studio".into(),
        kind: "local · OpenAI-compatible".into(),
        base_url: lm_base.into(),
        online,
        models_count: count,
        free_models: 0,
        note,
    });

    // Cloud OpenAI-compatible: el endpoint y la clave salen del proveedor activo.
    let activo = cfg.active_cloud();
    let cloud_base = base("cloud", cfg);
    let (key, origen) = crate::config::cloud_credential(&activo.id);
    let mut free = 0usize;
    let (online, count, note) = if key.is_none() {
        (
            false,
            0,
            format!(
                "{}: sin clave (KUKIA_CLOUD_KEY_{}, secrets.json o auth.json de OpenCode)",
                activo.label,
                activo.id.to_uppercase().replace('-', "_")
            ),
        )
    } else {
        match get_json(&format!("{cloud_base}/models"), key.as_deref()).await {
            Ok(v) => {
                let lista = v["data"].as_array().or_else(|| v["models"].as_array());
                let n = lista.map(|a| a.len()).unwrap_or(0);
                free = lista
                    .map(|a| {
                        a.iter()
                            .filter(|m| es_gratis(m["id"].as_str().unwrap_or("")))
                            .count()
                    })
                    .unwrap_or(0);
                (true, n, format!("{} · clave desde {}", activo.label, origen.label()))
            }
            Err(e) => (false, 0, format!("{}: sin respuesta ({e})", activo.label)),
        }
    };
    out.push(ProviderInfo {
        id: "cloud".into(),
        name: format!("Cloud · {}", activo.label),
        kind: "remoto · OpenAI-compatible".into(),
        base_url: cloud_base,
        online,
        models_count: count,
        free_models: free,
        note,
    });

    out
}

pub async fn list_models(provider: &str, cfg: &AppConfig) -> Result<Vec<ModelInfo>, String> {
    match provider {
        "lmstudio" | "cloud" => {
            let base_url = effective_base(provider, cfg).await;
            let url = join_api(&base_url, "/models");
            let (key, _origen) = if provider == "cloud" {
                crate::config::cloud_credential(&cfg.active_cloud().id)
            } else {
                (None, crate::config::KeySource::None)
            };
            let v = get_json(&url, key.as_deref()).await?;
            let mut out = Vec::new();
            if let Some(arr) = v["data"].as_array() {
                for m in arr {
                    let id = m["id"].as_str().unwrap_or("").to_string();
                    if id.is_empty() {
                        continue;
                    }
                    let mut caps = vec!["texto".to_string()];
                    if es_gratis(&id) {
                        caps.push("gratis".into());
                    }
                    out.push(ModelInfo {
                        id: id.clone(),
                        provider: provider.into(),
                        name: id,
                        context: m["context_length"].as_u64(),
                        size_gb: None,
                        capabilities: caps,
                        state: "disponible".into(),
                    });
                }
            }
            out.sort_by(|a, b| a.name.cmp(&b.name));
            Ok(out)
        }
        _ => {
            let url = format!("{}/api/tags", base(provider, cfg));
            let v = get_json(&url, None).await?;
            let mut out = Vec::new();
            if let Some(arr) = v["models"].as_array() {
                for m in arr {
                    let name = m["name"].as_str().unwrap_or("").to_string();
                    if name.is_empty() {
                        continue;
                    }
                    let family = m["details"]["family"].as_str().unwrap_or("");
                    let quant = m["details"]["quantization_level"].as_str().unwrap_or("");
                    let mut caps = vec!["texto".to_string()];
                    let lower = name.to_lowercase();
                    if lower.contains("vl") || lower.contains("llava") || family.contains("clip") {
                        caps.push("visión".into());
                    }
                    if lower.contains("coder") {
                        caps.push("código".into());
                    }
                    if lower.contains("embed") {
                        caps.push("embeddings".into());
                    }
                    let size = m["size"].as_u64().map(|b| b as f64 / 1e9);
                    out.push(ModelInfo {
                        id: name.clone(),
                        provider: "ollama".into(),
                        name: if quant.is_empty() {
                            name
                        } else {
                            format!("{name} · {quant}")
                        },
                        context: None,
                        size_gb: size,
                        capabilities: caps,
                        state: "disponible".into(),
                    });
                }
            }
            out.sort_by(|a, b| a.name.cmp(&b.name));
            Ok(out)
        }
    }
}

/* ============================ Chat ============================ */

fn build_messages(req: &ChatRequest) -> Vec<Value> {
    let mut msgs: Vec<Value> = Vec::new();
    if let Some(sys) = &req.system_prompt {
        if !sys.trim().is_empty() {
            msgs.push(json!({ "role": "system", "content": sys }));
        }
    }
    for m in &req.messages {
        msgs.push(json!({ "role": m.role, "content": m.content }));
    }
    msgs
}

/* ==================== Contexto de 256k para quien lo soporta ==================== */

/// Margen de VRAM que no se toca: el escritorio y los otros motores la usan.
const MARGEN_VRAM_GB: f64 = 1.5;

/// Corrección sobre la fórmula teórica de la caché KV.
///
/// Medido el 03/11/2026 con `qwen3-vl:4b` en esta máquina, leyendo `/api/ps`:
/// 2,92 GiB a 4k · 4,28 GiB a 32k · **16,99 GiB a 262.144** → 57,2 KB por token.
/// La fórmula `Arch::kv_bytes` con q4_0 predice 10,12 GiB a 256k y el servidor
/// reservó 14,29: Ollama pide un 41 % más de lo ideal. Se aplica 1,45 para no
/// prometer un contexto que después no cabe.
const CORRECCION_KV: f64 = 1.45;

/// Caché KV estimada en GB, ya corregida con lo medido.
fn kv_real_gb(bytes: u64) -> f64 {
    bytes as f64 / 1_073_741_824.0 * CORRECCION_KV
}

/// Cuánto contexto se le ha resuelto a cada modelo en esta sesión. La consulta
/// a `/api/show` y `/api/tags` cuesta unas decenas de ms: se hace una sola vez.
fn cache_contexto() -> &'static Mutex<HashMap<String, Option<u64>>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Option<u64>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cache_tamanos() -> &'static Mutex<HashMap<String, u64>> {
    static CACHE: OnceLock<Mutex<HashMap<String, u64>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// VRAM total de todas las GPU, en GB. No cambia durante la sesión.
fn vram_total_gb() -> f64 {
    static TOTAL: OnceLock<f64> = OnceLock::new();
    *TOTAL.get_or_init(|| {
        crate::gpu::gpus()
            .iter()
            .map(|g| g.memory_total_mb as f64 / 1024.0)
            .sum()
    })
}

/// ¿Caben el modelo y su caché KV a ese contexto? Regla explícita para que se
/// pueda probar: pesos + caché ≤ VRAM total − margen.
fn cabe_en_vram(peso_gb: f64, kv_gb: f64, vram_gb: f64) -> bool {
    vram_gb > 0.0 && peso_gb + kv_gb <= vram_gb - MARGEN_VRAM_GB
}

/// Bits por valor de la caché KV que Ollama tiene activa (`OLLAMA_KV_CACHE_TYPE`).
/// Si no hay variable, Ollama usa fp16.
fn bits_kv() -> f64 {
    match control::kv_entorno() {
        Some((_, v)) => control::KV_FORMATOS
            .iter()
            .find(|(nombre, _)| nombre.eq_ignore_ascii_case(v.trim()))
            .map(|(_, bits)| *bits)
            .unwrap_or(16.0),
        None => 16.0,
    }
}

/// Tamaño en disco del modelo, leído de `/api/tags` una vez por sesión.
async fn tamano_modelo(model: &str, cfg: &AppConfig) -> Option<u64> {
    if let Ok(c) = cache_tamanos().lock() {
        if let Some(v) = c.get(model) {
            return Some(*v);
        }
    }
    let url = format!("{}/api/tags", base("ollama", cfg));
    let r = client(10).get(&url).send().await.ok()?;
    let v: Value = r.json().await.ok()?;
    let mut cache = cache_tamanos().lock().ok()?;
    for m in v["models"].as_array().unwrap_or(&Vec::new()) {
        if let (Some(n), Some(sz)) = (m["name"].as_str(), m["size"].as_u64()) {
            cache.insert(n.to_string(), sz);
        }
    }
    cache.get(model).copied()
}

/// Decide si a este modelo se le puede pedir 256k de contexto en esta máquina.
///
/// Tres condiciones, las tres necesarias: el modelo declara el contexto nativo,
/// la caché KV a 256k cabe con sus pesos y hay VRAM para las dos cosas. La caché
/// se calcula con la fórmula de `control::Arch::kv_bytes`, la misma que usa el
/// panel de modelo, y con el formato KV que el servidor tiene activo.
/// Contexto máximo que cabe para un modelo: el mayor múltiplo de 1024 que entra
/// en la VRAM disponible sin pasar del contexto nativo que el modelo declara.
///
/// No se usa un número fijo: cada modelo tiene su propia caché KV por token, y
/// por eso un 4B llega al nativo mientras un 27B se queda donde cabe.
fn ctx_maximo(arch: &control::Arch, peso_gb: f64, vram_gb: f64) -> Option<u64> {
    if arch.layers == 0 || arch.heads_kv == 0 || arch.ctx_native == 0 {
        return None;
    }
    // Coste real por token, medido con qwen3-vl:4b (57,2 KB/token a 262.144).
    let gb_por_token = kv_real_gb(arch.kv_bytes(1_048_576, bits_kv())) / 1_048_576.0;
    if gb_por_token <= 0.0 {
        return None;
    }
    let disponible = vram_gb - MARGEN_VRAM_GB - peso_gb;
    if disponible <= 0.0 {
        return None;
    }
    let cabe = (disponible / gb_por_token) as u64;
    let objetivo = cabe.min(arch.ctx_native);
    // Múltiplo de 1024 hacia abajo. En modelos de contexto pequeño (2k-4k) el
    // resultado es su propio nativo, que es lo correcto: dejarlos con los 32.768
    // por defecto del motor los hacía trabajar fuera de rango (RoPE).
    let redondeado = ((objetivo / 1024) * 1024).max(1024);
    // Comprobación final con el contexto elegido, no con una suposición.
    if !cabe_en_vram(peso_gb, kv_real_gb(arch.kv_bytes(redondeado, bits_kv())), vram_gb) {
        return None;
    }
    Some(redondeado)
}

async fn calcular_contexto(model: &str, cfg: &AppConfig) -> Option<u64> {
    let arch = control::arch_ollama(model).await.ok()?;
    let peso_gb = tamano_modelo(model, cfg).await? as f64 / 1_073_741_824.0;
    ctx_maximo(&arch, peso_gb, vram_total_gb())
}

/// Contexto a pedir para el modelo, memorizado por sesión.
async fn contexto_para(model: &str, cfg: &AppConfig) -> Option<u64> {
    if let Ok(c) = cache_contexto().lock() {
        if let Some(v) = c.get(model) {
            return *v;
        }
    }
    let resuelto = calcular_contexto(model, cfg).await;
    if let Ok(mut c) = cache_contexto().lock() {
        c.insert(model.to_string(), resuelto);
    }
    resuelto
}

/// Opciones comunes de Ollama. `max_tokens` se traduce a `num_predict`: sin ese
/// mapeo el límite que pide el cliente se ignoraba, y un modelo con razonamiento
/// podía gastar todo el presupuesto pensando sin llegar a contestar.
/// `num_ctx` solo se envía cuando `contexto_para` ha confirmado que cabe.
fn opciones_ollama(req: &ChatRequest, cfg: &AppConfig, num_ctx: Option<u64>) -> Value {
    // Lo que el usuario haya guardado para ESTE modelo en el panel avanzado se
    // respeta y se envía. Antes el chat lo ignoraba: los parámetros guardados
    // (flash_attn, num_batch, num_gpu, num_ctx…) no llegaban nunca al motor.
    let guardados = crate::engines::params_get("ollama", &req.model);
    let mut opts = guardados.opciones_ollama();
    if !opts.is_object() {
        opts = json!({});
    }
    if opts.get("temperature").is_none() {
        opts["temperature"] = json!(req.temperature.unwrap_or(cfg.temperature));
    } else if let Some(t) = req.temperature {
        opts["temperature"] = json!(t);
    }
    if let Some(mt) = req.max_tokens {
        if mt > 0 {
            opts["num_predict"] = json!(mt);
        }
    }
    // El máximo calculado MANDA sobre lo guardado: es lo pedido, y un valor
    // viejo del panel (había un 2.048 guardado para qwen3-vl:4b) dejaría al
    // modelo sin contexto. Si no hay máximo calculado (arquitectura
    // desconocida o no cabe), se respeta lo que hubiera guardado.
    let ctx_guardado = opts.get("num_ctx").and_then(|v| v.as_u64()).unwrap_or(0);
    match num_ctx {
        Some(c) => opts["num_ctx"] = json!(c),
        None => {
            if ctx_guardado == 0 {
                if let Some(o) = opts.as_object_mut() {
                    o.remove("num_ctx");
                }
            }
        }
    }
    opts
}

/// Resultado de un intento de streaming: cuántos trozos de contenido y de
/// pensamiento llegaron, por qué terminó el motor y con qué consumo.
struct Intento {
    /// Llamadas a herramientas que el modelo pidio en este intento.
    llamadas: Vec<Value>,
    contenido: u64,
    pensamiento: u64,
    motivo: String,
    prompt_tokens: u64,
    completion_tokens: u64,
    total_ms: u64,
}

pub async fn chat(req: &ChatRequest, cfg: &AppConfig) -> Result<ChatResponse, String> {
    match req.provider.as_str() {
        "lmstudio" | "cloud" => chat_openai(req, cfg).await,
        _ => chat_ollama(req, cfg).await,
    }
}

async fn chat_ollama(req: &ChatRequest, cfg: &AppConfig) -> Result<ChatResponse, String> {
    let url = format!("{}/api/chat", base("ollama", cfg));
    let started = Instant::now();
    let mut body = json!({
        "model": req.model,
        "messages": build_messages(req),
        "stream": false,
        "options": opciones_ollama(req, cfg, contexto_para(&req.model, cfg).await)
    });
    if !cfg.reasoning {
        body["think"] = json!(false);
    }
    let resp = client(600)
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Ollama: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Ollama HTTP {status}: {text}"));
    }
    let v: Value = resp.json().await.map_err(|e| e.to_string())?;
    let content = {
        let msg = &v["message"];
        let normal = msg["content"].as_str().unwrap_or("");
        if normal.trim().is_empty() {
            // Ollama expone el pensamiento en `thinking` para modelos con razonamiento.
            msg["thinking"].as_str().unwrap_or("").to_string()
        } else {
            normal.to_string()
        }
    };
    let prompt_tokens = v["prompt_eval_count"].as_u64().unwrap_or(0);
    let completion_tokens = v["eval_count"].as_u64().unwrap_or(0);
    let total_ms = v["total_duration"]
        .as_u64()
        .map(|ns| ns / 1_000_000)
        .unwrap_or_else(|| started.elapsed().as_millis() as u64);
    Ok(ChatResponse {
        content,
        prompt_tokens,
        completion_tokens,
        total_ms,
    })
}

async fn chat_openai(req: &ChatRequest, cfg: &AppConfig) -> Result<ChatResponse, String> {
    let url = join_api(
        &effective_base(&req.provider, cfg).await,
        "/chat/completions",
    );
    let started = Instant::now();
    let mut body = json!({
        "model": req.model,
        "messages": build_messages(req),
        "temperature": req.temperature.unwrap_or(cfg.temperature),
        "stream": false
    });
    if let Some(mt) = req.max_tokens {
        body["max_tokens"] = json!(mt);
    }
    if !cfg.reasoning {
        // Solo se pide "no pensar" si el usuario lo desactiva en Configuración.
        body["chat_template_kwargs"] = json!({ "enable_thinking": false });
        body["reasoning_effort"] = json!("none");
    }
    let mut builder = client(600).post(&url).json(&body);
    if let Some(k) = cloud_key(cfg) {
        builder = builder.bearer_auth(k);
    }
    let sesion = uuid::Uuid::new_v4().to_string();
    builder = con_sesion(builder, cfg, &sesion);
    let mut resp = builder.send().await.map_err(|e| format!("{}: {e}", req.provider))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        // Algunos servidores rechazan los campos de razonamiento: se reintenta
        // sin ellos para no romper el chat por una preferencia opcional.
        if !cfg.reasoning && status.as_u16() == 400 {
            let mut retry = client(600).post(&url).json(&json!({
                "model": req.model,
                "messages": build_messages(req),
                "temperature": req.temperature.unwrap_or(cfg.temperature),
                "stream": false
            }));
            if let Some(k) = cloud_key(cfg) {
                retry = retry.bearer_auth(k);
            }
            retry = con_sesion(retry, cfg, &sesion);
            resp = retry
                .send()
                .await
                .map_err(|e| format!("{}: {e}", req.provider))?;
            if !resp.status().is_success() {
                let s2 = resp.status();
                let t2 = resp.text().await.unwrap_or_default();
                return Err(format!("{} HTTP {s2}: {t2}", req.provider));
            }
        } else {
            return Err(format!("{} HTTP {status}: {text}", req.provider));
        }
    }
    let v: Value = resp.json().await.map_err(|e| e.to_string())?;
    let content = {
        let msg = &v["choices"][0]["message"];
        let normal = msg["content"].as_str().unwrap_or("");
        if normal.trim().is_empty() {
            // Modelos con razonamiento (DeepSeek-R1, JEV, etc.) devuelven el texto
            // en `reasoning_content` y dejan `content` vacío.
            msg["reasoning_content"].as_str().unwrap_or("").to_string()
        } else {
            normal.to_string()
        }
    };
    let prompt_tokens = v["usage"]["prompt_tokens"].as_u64().unwrap_or(0);
    let completion_tokens = v["usage"]["completion_tokens"].as_u64().unwrap_or(0);
    Ok(ChatResponse {
        content,
        prompt_tokens,
        completion_tokens,
        total_ms: started.elapsed().as_millis() as u64,
    })
}

/* ============================ Streaming ============================ */

pub async fn chat_stream(
    req: &ChatRequest,
    cfg: &AppConfig,
    tx: UnboundedSender<ChatEvent>,
) -> Result<(), String> {
    chat_stream_con(req, cfg, None, tx).await
}

/// Igual que `chat_stream`, pero con la base de datos: hace falta para ejecutar
/// herramientas (`memory_search`) y para dejar rastro en la auditoría. El candado
/// se toma por consulta, nunca mientras el modelo genera.
pub async fn chat_stream_con(
    req: &ChatRequest,
    cfg: &AppConfig,
    db: Option<&herramientas::Db>,
    tx: UnboundedSender<ChatEvent>,
) -> Result<(), String> {
    let result = match req.provider.as_str() {
        "lmstudio" | "cloud" => stream_openai(req, cfg, &tx).await,
        _ => stream_ollama(req, cfg, db, &tx).await,
    };
    if let Err(e) = &result {
        let _ = tx.send(ChatEvent::Error {
            message: e.clone(),
        });
    }
    result
}

/// Chat con Ollama. Si el modelo declara `tools`, se le ofrecen las herramientas
/// de su agente y se ejecutan las que pida, devolviéndole el resultado hasta que
/// conteste o se agoten los pasos.
async fn stream_ollama(
    req: &ChatRequest,
    cfg: &AppConfig,
    db: Option<&herramientas::Db>,
    tx: &UnboundedSender<ChatEvent>,
) -> Result<(), String> {
    let mut mensajes = build_messages(req);
    let politica = herramientas::politica(req, db);
    let mut esquemas: Option<Value> = None;

    if req.tools && herramientas::soporta_tools(&req.model, cfg).await {
        let defs = herramientas::esquemas(&politica, req.aprobar);
        if !defs.is_empty() {
            let nombres: Vec<String> = defs
                .iter()
                .filter_map(|d| d["function"]["name"].as_str().map(|s| s.to_string()))
                .collect();
            let _ = tx.send(ChatEvent::Reasoning {
                text: format!(
                    "\n[KUKIA] Herramientas activas ({}): {}.\n\n",
                    nombres.len(),
                    nombres.join(", ")
                ),
            });
            esquemas = Some(Value::Array(defs));
        }
    }

    let mut pasos = 0usize;
    let mut prompt_total = 0u64;
    let mut tokens_total = 0u64;
    let mut ms_total = 0u64;

    loop {
        let intento = stream_ollama_intento(
            req,
            cfg,
            tx,
            cfg.reasoning,
            &mensajes,
            esquemas.as_ref(),
        )
        .await?;
        prompt_total += intento.prompt_tokens;
        tokens_total += intento.completion_tokens;
        ms_total += intento.total_ms;

        // Sin herramientas pedidas: se cierra el turno como siempre.
        if intento.llamadas.is_empty() {
            // El caso que dejaba la burbuja vacía: el modelo agotó el presupuesto
            // pensando (`done_reason = "length"`) y nunca emitió `content`.
            // Medido contra Ollama 0.40.13 (03/11/2026, qwen3-vl:4b, 24 tokens):
            //   `think:false` → content vacío · `chat_template_kwargs` → vacío
            //   system `/no_think` → vacío · mismo prompt con 256 → "4" en 117
            // No hay forma fiable de pedirle que no piense; lo que sí funciona es
            // dar más margen, y solo si el límite no lo puso el usuario.
            if intento.contenido == 0
                && intento.motivo == "length"
                && req.max_tokens.is_none()
                && pasos == 0
                && intento.completion_tokens < 8_192
            {
                let nuevo = (intento.completion_tokens.saturating_mul(2))
                    .max(2048)
                    .min(8_192);
                let _ = tx.send(ChatEvent::Reasoning {
                    text: format!(
                        "\n\n[KUKIA] El modelo agotó {antes} tokens pensando y no llegó a \
                         responder. Se reintenta con {nuevo} tokens de margen.\n\n",
                        antes = intento.completion_tokens,
                    ),
                });
                let mut con_mas = req.clone();
                con_mas.max_tokens = Some(nuevo as u32);
                let segundo = stream_ollama_intento(
                    &con_mas,
                    cfg,
                    tx,
                    cfg.reasoning,
                    &mensajes,
                    esquemas.as_ref(),
                )
                .await?;
                prompt_total += segundo.prompt_tokens;
                tokens_total += segundo.completion_tokens;
                ms_total += segundo.total_ms;
                if segundo.contenido == 0 {
                    let _ = tx.send(ChatEvent::Error {
                        message: "Ollama: el modelo agotó el presupuesto pensando y no llegó \
                                  a responder. Sube «máx. tokens» en el mensaje o usa un modelo \
                                  sin razonamiento."
                            .to_string(),
                    });
                    return Ok(());
                }
                let _ = tx.send(ChatEvent::Done {
                    prompt_tokens: prompt_total,
                    completion_tokens: tokens_total,
                    total_ms: ms_total,
                });
                return Ok(());
            }

            if intento.contenido == 0 {
                let causa = if intento.motivo == "length" {
                    format!(
                        "agotó los {} tokens de salida pensando (done_reason=length) y no llegó \
                         a responder",
                        intento.completion_tokens
                    )
                } else if intento.pensamiento > 0 {
                    "solo devolvió pensamiento, sin texto final".to_string()
                } else {
                    "no devolvió texto".to_string()
                };
                let _ = tx.send(ChatEvent::Error {
                    message: format!(
                        "Ollama: el modelo {causa}. Sube «máx. tokens» en el mensaje o usa un \
                         modelo sin razonamiento."
                    ),
                });
                return Ok(());
            }

            let _ = tx.send(ChatEvent::Done {
                prompt_tokens: prompt_total,
                completion_tokens: tokens_total,
                total_ms: ms_total,
            });
            return Ok(());
        }

        // --- El modelo quiere herramientas: se ejecutan y se le devuelve el
        // resultado, que es lo que le permite seguir razonando con datos reales.
        pasos += 1;
        if pasos > herramientas::MAX_PASOS {
            let _ = tx.send(ChatEvent::Error {
                message: format!(
                    "el modelo encadenó más de {} pasos usando herramientas; se detiene aquí",
                    herramientas::MAX_PASOS
                ),
            });
            return Ok(());
        }
        mensajes.push(json!({
            "role": "assistant",
            "content": "",
            "tool_calls": intento.llamadas.clone(),
        }));
        for llamada in &intento.llamadas {
            let nombre = llamada["function"]["name"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let args = llamada["function"]["arguments"].clone();
            let args_txt = match &args {
                Value::String(s) => s.clone(),
                v => v.to_string(),
            };
            let _ = tx.send(ChatEvent::ToolCall {
                nombre: nombre.clone(),
                argumentos: args_txt,
            });
            let res = herramientas::ejecutar(&nombre, &args, &politica, req.aprobar, db).await;
            let (ok, texto) = match &res {
                Ok(t) => (true, t.clone()),
                Err(e) => (false, e.clone()),
            };
            let _ = tx.send(ChatEvent::ToolResult {
                nombre: nombre.clone(),
                ok,
                texto: texto.clone(),
            });
            mensajes.push(json!({
                "role": "tool",
                "tool_name": nombre,
                "content": texto,
            }));
        }
    }
}

/// Un intento completo de streaming contra Ollama. **No** emite `Done`: de eso se
/// encarga `stream_ollama`, porque puede haber un reintento detrás y un `Done`
/// prematuro es exactamente lo que guardaba mensajes vacíos en la interfaz.
async fn stream_ollama_intento(
    req: &ChatRequest,
    cfg: &AppConfig,
    tx: &UnboundedSender<ChatEvent>,
    pensar: bool,
    mensajes: &[Value],
    herramientas: Option<&Value>,
) -> Result<Intento, String> {
    let url = format!("{}/api/chat", base("ollama", cfg));
    let mut body = json!({
        "model": req.model,
        "messages": mensajes,
        "stream": true,
        "options": opciones_ollama(req, cfg, contexto_para(&req.model, cfg).await)
    });
    if let Some(t) = herramientas {
        body["tools"] = t.clone();
    }
    if !pensar {
        body["think"] = json!(false);
    }
    let resp = client(600)
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Ollama: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Ollama HTTP {status}: {text}"));
    }

    let mut salida = Intento {
        llamadas: Vec::new(),
        contenido: 0,
        pensamiento: 0,
        motivo: String::new(),
        prompt_tokens: 0,
        completion_tokens: 0,
        total_ms: 0,
    };
    let mut stream = resp.bytes_stream();
    let mut buf = String::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        buf.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(pos) = buf.find('\n') {
            let line: String = buf.drain(..=pos).collect();
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(v) = serde_json::from_str::<Value>(line) {
                // Ollama separa pensamiento (`thinking`) y respuesta (`content`).
                if let Some(pens) = v["message"]["thinking"].as_str() {
                    if !pens.is_empty() {
                        salida.pensamiento += 1;
                        let _ = tx.send(ChatEvent::Reasoning {
                            text: pens.to_string(),
                        });
                    }
                }
                if let Some(text) = v["message"]["content"].as_str() {
                    if !text.is_empty() {
                        salida.contenido += 1;
                        let _ = tx.send(ChatEvent::Token {
                            text: text.to_string(),
                        });
                    }
                }
                // Lo que el modelo pide hacer. Antes se descartaba por completo:
                // el motor sabía pedir y KUKIA no escuchaba.
                if let Some(arr) = v["message"]["tool_calls"].as_array() {
                    for c in arr {
                        salida.llamadas.push(c.clone());
                    }
                }
                if v["done"].as_bool().unwrap_or(false) {
                    salida.motivo = v["done_reason"].as_str().unwrap_or("").to_string();
                    salida.prompt_tokens = v["prompt_eval_count"].as_u64().unwrap_or(0);
                    salida.completion_tokens = v["eval_count"].as_u64().unwrap_or(0);
                    salida.total_ms = v["total_duration"]
                        .as_u64()
                        .map(|ns| ns / 1_000_000)
                        .unwrap_or(0);
                    return Ok(salida);
                }
            }
        }
    }
    Ok(salida)
}

async fn stream_openai(
    req: &ChatRequest,
    cfg: &AppConfig,
    tx: &UnboundedSender<ChatEvent>,
) -> Result<(), String> {
    let url = join_api(
        &effective_base(&req.provider, cfg).await,
        "/chat/completions",
    );
    let started = Instant::now();
    let mut body = json!({
        "model": req.model,
        "messages": build_messages(req),
        "temperature": req.temperature.unwrap_or(cfg.temperature),
        "stream": true
    });
    if let Some(mt) = req.max_tokens {
        body["max_tokens"] = json!(mt);
    }
    let mut builder = client(600).post(&url).json(&body);
    if let Some(k) = cloud_key(cfg) {
        builder = builder.bearer_auth(k);
    }
    let sesion = uuid::Uuid::new_v4().to_string();
    builder = con_sesion(builder, cfg, &sesion);
    let resp = builder.send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("HTTP {status}: {text}"));
    }

    let mut stream = resp.bytes_stream();
    let mut buf = String::new();
    let mut completion_tokens = 0u64;
    // Mismo fallo que en Ollama, con otro nombre: si el modelo se queda sin
    // presupuesto antes de escribir, `finish_reason` es "length" y `content`
    // nunca llega. Aquí NO se reintenta (cada intento cuesta dinero).
    let mut hubo_contenido = false;
    let mut motivo = String::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        buf.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(pos) = buf.find('\n') {
            let line: String = buf.drain(..=pos).collect();
            let line = line.trim();
            if let Some(data) = line.strip_prefix("data:") {
                let data = data.trim();
                if data == "[DONE]" {
                    if !hubo_contenido {
                        let _ = tx.send(ChatEvent::Error {
                            message: format!(
                                "{}: el modelo no produjo texto final (finish_reason={}). \
                                 Sube «máx. tokens» o desactiva el razonamiento.",
                                req.provider,
                                if motivo.is_empty() { "—" } else { motivo.as_str() }
                            ),
                        });
                        return Ok(());
                    }
                    let _ = tx.send(ChatEvent::Done {
                        prompt_tokens: 0,
                        completion_tokens,
                        total_ms: started.elapsed().as_millis() as u64,
                    });
                    return Ok(());
                }
                if let Ok(v) = serde_json::from_str::<Value>(data) {
                    if let Some(fr) = v["choices"][0]["finish_reason"].as_str() {
                        motivo = fr.to_string();
                    }
                    let delta = &v["choices"][0]["delta"];
                    // El pensamiento va por su canal para que la UI lo muestre aparte.
                    if let Some(pens) = delta["reasoning_content"].as_str() {
                        if !pens.is_empty() {
                            let _ = tx.send(ChatEvent::Reasoning {
                                text: pens.to_string(),
                            });
                        }
                    }
                    if let Some(texto) = delta["content"].as_str() {
                        if !texto.is_empty() {
                            hubo_contenido = true;
                            completion_tokens += 1;
                            let _ = tx.send(ChatEvent::Token {
                                text: texto.to_string(),
                            });
                        }
                    }
                }
            }
        }
    }
    let _ = tx.send(ChatEvent::Done {
        prompt_tokens: 0,
        completion_tokens,
        total_ms: started.elapsed().as_millis() as u64,
    });
    Ok(())
}

/* ============================ Tests ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn los_gratis_se_detectan_por_el_sufijo_del_catalogo() {
        assert!(es_gratis("flint-40b-free"));
        assert!(es_gratis("glm-5.3-flash-free"));
        assert!(es_gratis("free-model"));
        assert!(!es_gratis("gpt-5.5"));
        assert!(!es_gratis("claude-sonnet-5"));
        assert!(!es_gratis("freedom-7b"));
    }

    #[test]
    fn max_tokens_se_traduce_a_num_predict_en_ollama() {
        let cfg = AppConfig::default();
        let mut req = ChatRequest {
            provider: "ollama".into(),
            model: "m".into(),
            messages: vec![],
            temperature: None,
            max_tokens: Some(512),
            system_prompt: None,
            tools: false,
            aprobar: false,
            agent: None,
        };
        let con = opciones_ollama(&req, &cfg, None);
        assert_eq!(con["num_predict"], 512, "max_tokens debe llegar a num_predict");

        // Sin límite del cliente no se inventa uno: Ollama decide.
        req.max_tokens = None;
        let sin = opciones_ollama(&req, &cfg, None);
        assert!(sin.get("num_predict").is_none());

        // El contexto solo viaja cuando se ha comprobado que cabe.
        assert!(sin.get("num_ctx").is_none());
        let con_ctx = opciones_ollama(&req, &cfg, Some(131_072));
        assert_eq!(con_ctx["num_ctx"], 131_072);
    }

    #[test]
    fn la_guarda_de_vram_rechaza_contextos_que_no_caben() {
        // Un 31B a 256k pedía 88 GB de caché KV según el inventario: imposible.
        assert!(!cabe_en_vram(18.0, 88.0, 27.8));
        // Un 30B con caché KV q4 de 3,4 GB sí cabe con margen.
        assert!(cabe_en_vram(18.0, 3.4, 27.8));
        // Justo en el límite del margen: no se acepta por los pelos.
        assert!(!cabe_en_vram(25.0, 1.4, 27.8));
        // Sin GPU no se promete nada.
        assert!(!cabe_en_vram(1.0, 1.0, 0.0));
    }

    #[test]
    fn el_contexto_maximo_se_ajusta_a_lo_que_cabe() {
        let vram = 27.9;
        // Forma de qwen3-vl:4b: 36 capas, 8 cabezas KV, dim 128, nativo 256k.
        let pequeno = control::Arch {
            architecture: "qwen3vl".into(),
            layers: 36,
            heads: 32,
            heads_kv: 8,
            embedding: 4096,
            ctx_native: 262_144,
        };
        let ctx = ctx_maximo(&pequeno, 3.1, vram).expect("un 4B debe tener contexto");
        assert_eq!(ctx, 262_144, "con 3,1 GB de pesos cabe el nativo entero");
        assert_eq!(ctx % 1024, 0, "siempre múltiplo de 1024");

        // Un 27B con la misma cabeza KV no llega al nativo: se queda donde cabe.
        let grande = control::Arch {
            architecture: "qwen3next".into(),
            layers: 64,
            heads: 40,
            heads_kv: 8,
            embedding: 5120,
            ctx_native: 262_144,
        };
        let ctx2 = ctx_maximo(&grande, 15.7, vram).expect("un 27B debe tener contexto");
        assert!(ctx2 < 262_144, "un 27B no cabe a 256k, salió {ctx2}");
        assert!(ctx2 > 8_192, "y no se queda en el mínimo, salió {ctx2}");

        // Modelo de contexto pequeño: se pide su nativo, no los 32.768 del motor
        // (que lo harían trabajar fuera de rango).
        let chico = control::Arch {
            architecture: "llama".into(),
            layers: 40,
            heads: 64,
            heads_kv: 8,
            embedding: 8192,
            ctx_native: 4_096,
        };
        assert_eq!(ctx_maximo(&chico, 20.0, vram), Some(4_096));

        // Sin arquitectura no se inventa nada.
        let vacio = control::Arch::default();
        assert_eq!(ctx_maximo(&vacio, 3.0, vram), None);
    }

    #[test]
    fn la_correccion_de_kv_no_se_queda_corta_frente_a_lo_medido() {
        // Medido con qwen3-vl:4b a 262.144 tokens: 14,29 GiB reales de caché.
        // La fórmula ideal (q4_0) daba 10,12 GiB. La corrección nunca debe
        // estimar menos de lo que el servidor reserva de verdad.
        let medido_gb = 14.29_f64;
        let ideal_bytes = (10.12_f64 * 1_073_741_824.0) as u64;
        assert!(
            kv_real_gb(ideal_bytes) >= medido_gb,
            "la corrección se queda corta: {} < {medido_gb}",
            kv_real_gb(ideal_bytes)
        );
    }

    #[test]
    fn messages_include_system_prompt() {
        let req = ChatRequest {
            provider: "ollama".into(),
            model: "m".into(),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: "hola".into(),
            }],
            temperature: None,
            max_tokens: None,
            system_prompt: Some("eres kukia".into()),
            tools: false,
            aprobar: false,
            agent: None,
        };
        let msgs = build_messages(&req);
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0]["role"], "system");
    }

    #[test]
    fn base_urls_are_correct() {
        let cfg = AppConfig::default();
        assert_eq!(base("ollama", &cfg), "http://127.0.0.1:11434");
        assert_eq!(base("lmstudio", &cfg), "http://127.0.0.1:1234");
        // Por defecto el proveedor cloud activo es OpenCode Zen.
        assert_eq!(base("cloud", &cfg), "https://opencode.ai/zen/v1");
    }

    #[test]
    fn el_proveedor_cloud_activo_manda_en_la_base() {
        let mut cfg = AppConfig::default();
        cfg.cloud_provider = "openai".into();
        assert_eq!(base("cloud", &cfg), "https://api.openai.com/v1");
        cfg.cloud_provider = "freebuff".into();
        assert_eq!(base("cloud", &cfg), "https://freebuff.com/api/v1/paid");
    }

    #[test]
    fn join_api_no_duplica_el_v1() {
        assert_eq!(
            join_api("https://opencode.ai/zen/v1", "/models"),
            "https://opencode.ai/zen/v1/models"
        );
        assert_eq!(
            join_api("https://opencode.ai/zen/v1/", "/chat/completions"),
            "https://opencode.ai/zen/v1/chat/completions"
        );
        assert_eq!(
            join_api("http://127.0.0.1:1235", "/chat/completions"),
            "http://127.0.0.1:1235/v1/chat/completions"
        );
        assert_eq!(
            join_api("https://api.openai.com/v1", "/models"),
            "https://api.openai.com/v1/models"
        );
    }

    #[test]
    fn chat_event_serializes_with_kind() {
        let ev = ChatEvent::Token {
            text: "hola".into(),
        };
        let raw = serde_json::to_string(&ev).unwrap();
        assert!(raw.contains("\"kind\":\"token\""));
        assert!(raw.contains("hola"));
    }
}
