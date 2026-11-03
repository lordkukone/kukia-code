//! Catálogo universal de modelos (F4 · MODEL HUB).
//!
//! Enumera lo que hay de verdad en cada motor, **sin cargar, descargar ni
//! modificar nada**, y enruta una necesidad («texto», «vision», «imagen»…) al
//! modelo que mejor la cubre.
//!
//! Tres ideas mandan:
//!
//! 1. **Nada se inventa.** Cada modelo dice de qué motor sale y si sus
//!    capacidades las declaró el motor (`inferred = false`) o se dedujeron del
//!    nombre (`inferred = true`).
//! 2. **El cloud no se usa por accidente.** Solo entra si la necesidad lo
//!    autoriza (`allow_cloud`); si no, aparece como rechazado y con el motivo.
//! 3. **Lo fijado a mano manda**, pero si el modelo fijado ya no existe se avisa
//!    y se enruta igualmente, en vez de fallar en silencio.

use crate::config::AppConfig;
use crate::memory::Database;
use crate::providers;
use futures_util::StreamExt;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Instant;

/* ============================ Tipos ============================ */

/// Un modelo tal cual lo ve KUKIA, venga del motor que venga.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalModel {
    /// `motor::id` — la clave que usa toda la interfaz.
    pub key: String,
    pub engine: String,
    pub provider: String,
    pub id: String,
    /// chat · embeddings · imagen · audio · otro
    pub kind: String,
    pub capabilities: Vec<String>,
    pub context_length: Option<u64>,
    pub size_gb: Option<f64>,
    pub quantization: Option<String>,
    pub format: Option<String>,
    pub loaded: bool,
    pub local: bool,
    pub can_chat: bool,
    /// `true` cuando las capacidades se dedujeron del nombre y no las declaró el motor.
    pub inferred: bool,
    pub note: String,
}

/// Qué se pudo comprobar de cada motor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogCoverage {
    pub engine: String,
    pub name: String,
    pub reachable: bool,
    pub models: usize,
    /// El motor publica una API para enumerar modelos.
    pub catalog_api: bool,
    pub detail: String,
}

/// Todo lo enumerado en una pasada.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogReport {
    pub models: Vec<UniversalModel>,
    pub coverage: Vec<CatalogCoverage>,
    pub total: usize,
    pub generated_ms: u64,
}

/// Qué sabe hacer de verdad cada adaptador. Se declara a mano y se comprueba en
/// el resto de módulos: aquí no se promete nada que no exista.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterInfo {
    pub engine: String,
    pub chat: bool,
    pub stream: bool,
    pub embeddings: bool,
    pub image: bool,
    pub video: bool,
    pub audio: bool,
    pub unload: bool,
    pub status: bool,
    pub detail: String,
}

/// Lo que necesita una herramienta o un agente.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Need {
    pub capability: String,
    pub min_context: Option<u64>,
    pub allow_cloud: bool,
    pub prefer_loaded: bool,
    pub scope: Option<String>,
}

impl Default for Need {
    fn default() -> Self {
        Self {
            capability: "texto".into(),
            min_context: None,
            allow_cloud: false,
            prefer_loaded: true,
            scope: None,
        }
    }
}

/// Un candidato con su puntuación y el porqué.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub key: String,
    pub engine: String,
    pub id: String,
    pub kind: String,
    pub score: f64,
    pub reasons: Vec<String>,
}

/// Un modelo que se descartó, con el motivo exacto.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rejected {
    pub key: String,
    pub why: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    pub need: Need,
    pub chosen: Option<Candidate>,
    pub alternatives: Vec<Candidate>,
    pub rejected: Vec<Rejected>,
    pub note: String,
}

/// Modelo fijado a mano para un ámbito (`kukia-coder`, `kukia-gpu`…).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Binding {
    pub scope: String,
    pub engine: String,
    pub model: String,
    pub updated_at: String,
}

/// Respuesta de una llamada completa por el router.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedChat {
    pub engine: String,
    pub model: String,
    pub content: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_ms: u64,
    pub note: String,
}

/* ============================ Clasificación ============================ */

/// Decide tipo y capacidades de un modelo.
///
/// Si el motor **declara** capacidades se usan tal cual (traducidas a los nombres
/// que entiende la interfaz) y `inferred` es `false`. Si no declara nada, se
/// deducen del nombre y se marca `inferred = true`: así la interfaz puede decir
/// «deducido» en vez de hacerlo pasar por un dato del motor.
pub fn clasificar(engine: &str, id: &str, declaradas: &[String]) -> (String, Vec<String>, bool) {
    let mut caps: Vec<String> = Vec::new();
    let inferred;

    for d in declaradas {
        for c in traducir_capacidad(d) {
            if !caps.contains(&c) {
                caps.push(c);
            }
        }
    }

    let l = id.to_lowercase();

    // El tipo manda sobre todo lo demás.
    let kind = if engine == "comfyui" || parece_imagen(&l) {
        "imagen"
    } else if l.contains("embed") {
        "embeddings"
    } else if caps.iter().any(|c| c == "embeddings") {
        "embeddings"
    } else if parece_audio(&l) {
        "audio"
    } else {
        "chat"
    };

    // Capacidades por tipo, siempre presentes.
    match kind {
        "imagen" => añadir(&mut caps, "imagen"),
        "embeddings" => añadir(&mut caps, "embeddings"),
        "audio" => añadir(&mut caps, "audio"),
        _ => añadir(&mut caps, "texto"),
    }

    // Deducidas del nombre: nunca sobrescriben lo que declaró el motor, solo
    // añaden lo que falta cuando no hay declaración.
    if declaradas.is_empty() {
        if l.contains("coder") || l.contains("code") {
            añadir(&mut caps, "codigo");
        }
        if l.contains("vl") || l.contains("llava") || l.contains("vision") || l.contains("vlm") {
            añadir(&mut caps, "vision");
        }
        if l.contains("whisper") || l.contains("audio") || l.contains("tts") {
            añadir(&mut caps, "audio");
        }
        if l.contains("think") || l.contains("reason") || l.contains("r1") {
            añadir(&mut caps, "razonamiento");
        }
        inferred = true;
    } else {
        inferred = false;
    }

    (kind.to_string(), caps, inferred)
}

fn añadir(caps: &mut Vec<String>, c: &str) {
    if !caps.iter().any(|x| x == c) {
        caps.push(c.to_string());
    }
}

/// Traduce una capacidad del motor a los nombres que usa el catálogo.
fn traducir_capacidad(decl: &str) -> Vec<String> {
    match decl.trim().to_lowercase().as_str() {
        "completion" | "chat" => vec!["texto".into()],
        "tools" | "tool" => vec!["herramientas".into()],
        "vision" => vec!["vision".into()],
        "embeddings" | "embedding" => vec!["embeddings".into()],
        "thinking" | "reasoning" => vec!["razonamiento".into()],
        "audio" | "speech" => vec!["audio".into()],
        "image" | "images" => vec!["imagen".into()],
        "insert" => vec!["autocompletado".into()],
        otro if otro.is_empty() => vec![],
        otro => vec![otro.to_string()],
    }
}

fn parece_imagen(l: &str) -> bool {
    l.ends_with(".safetensors")
        || l.contains("sdxl")
        || l.contains("stable-diffusion")
        || l.contains("flux")
        || l.contains("pony")
        || l.contains("illustrious")
        || l.contains("checkpoint")
}

fn parece_audio(l: &str) -> bool {
    l.contains("whisper") || l.contains("audio") || l.contains("tts") || l.contains("musicgen")
}

/* ============================ Enrutado ============================ */

/// Elige el mejor modelo para una necesidad. Puro: sin red, sin base de datos.
///
/// Puntuación: base 50, +30 si está cargado y se prefiere cargado, +10 si es
/// local, +5 si es Ollama (el motor con más adaptadores) y +10 si trae la
/// capacidad pedida. Los empates se rompen por clave, para que dos ejecuciones
/// den el mismo resultado.
pub fn enrutar(models: &[UniversalModel], need: &Need) -> Route {
    let mut validos: Vec<Candidate> = Vec::new();
    let mut rejected: Vec<Rejected> = Vec::new();

    for m in models {
        let mut reasons: Vec<String> = Vec::new();

        if m.engine == "cloud" && !need.allow_cloud {
            rejected.push(Rejected {
                key: m.key.clone(),
                why: "el modelo cloud no está autorizado para esta tarea (allow_cloud = false)"
                    .into(),
            });
            continue;
        }

        let pide_imagen = need.capability == "imagen";
        if pide_imagen && m.kind != "imagen" {
            rejected.push(Rejected {
                key: m.key.clone(),
                why: format!("no genera imágenes (tipo {})", m.kind),
            });
            continue;
        }
        if !pide_imagen && !m.can_chat {
            rejected.push(Rejected {
                key: m.key.clone(),
                why: "no tiene adaptador de chat".into(),
            });
            continue;
        }

        if !need.capability.trim().is_empty()
            && !m.capabilities.iter().any(|c| c == &need.capability)
        {
            rejected.push(Rejected {
                key: m.key.clone(),
                why: format!(
                    "no declara la capacidad «{}» (tiene: {})",
                    need.capability,
                    m.capabilities.join(", ")
                ),
            });
            continue;
        }

        if let Some(min) = need.min_context {
            let tiene = m.context_length.unwrap_or(0);
            if tiene < min {
                rejected.push(Rejected {
                    key: m.key.clone(),
                    why: format!("contexto insuficiente: {tiene} < {min}"),
                });
                continue;
            }
            reasons.push(format!("contexto {tiene} ≥ {min}"));
        }

        let mut score = 50.0;
        reasons.push("cubre la necesidad".into());

        if m.loaded && need.prefer_loaded {
            score += 30.0;
            reasons.push("ya está cargado".into());
        }
        if m.local {
            score += 10.0;
        }
        if m.engine == "ollama" {
            score += 5.0;
        }
        if m.capabilities.iter().any(|c| c == &need.capability) {
            score += 10.0;
        }

        validos.push(Candidate {
            key: m.key.clone(),
            engine: m.engine.clone(),
            id: m.id.clone(),
            kind: m.kind.clone(),
            score,
            reasons,
        });
    }

    validos.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.key.cmp(&b.key))
    });

    let chosen = validos.first().cloned();
    let alternatives = validos.iter().skip(1).take(5).cloned().collect::<Vec<_>>();

    let note = match &chosen {
        Some(c) => format!(
            "{} candidato(s) · elegido {} ({:.0} pts)",
            validos.len(),
            c.key,
            c.score
        ),
        None => format!(
            "ningún modelo cubre «{}» ({} descartado(s))",
            need.capability,
            rejected.len()
        ),
    };

    Route {
        need: need.clone(),
        chosen,
        alternatives,
        rejected,
        note,
    }
}

/// Enrutado con los modelos fijados a mano de por medio.
///
/// Si el ámbito tiene un modelo fijado y sigue existiendo, gana. Si ya no está,
/// se avisa en la nota y se enruta con normalidad: nunca se queda uno sin modelo
/// por una preferencia vieja.
pub fn resolver_en(models: &[UniversalModel], db: &Database, need: &Need) -> Route {
    if let Some(scope) = need.scope.as_ref().filter(|s| !s.trim().is_empty()) {
        if let Some(b) = bind_get(db, scope) {
            if let Some(m) = models
                .iter()
                .find(|m| m.engine == b.engine && m.id == b.model)
            {
                let cand = Candidate {
                    key: m.key.clone(),
                    engine: m.engine.clone(),
                    id: m.id.clone(),
                    kind: m.kind.clone(),
                    score: 100.0,
                    reasons: vec![format!("fijado a mano para «{scope}»")],
                };
                return Route {
                    need: need.clone(),
                    chosen: Some(cand),
                    alternatives: Vec::new(),
                    rejected: Vec::new(),
                    note: format!(
                        "fijado a mano: «{scope}» → {}::{}",
                        b.engine, b.model
                    ),
                };
            }
            let mut r = enrutar(models, need);
            r.note = format!(
                "el modelo fijado para «{scope}» ({}::{}) ya no está disponible; se usa el automático: {}",
                b.engine, b.model, r.note
            );
            return r;
        }
    }
    enrutar(models, need)
}

/// Enrutado completo: enumera y resuelve.
pub async fn resolver(db: &Database, cfg: &AppConfig, need: &Need) -> Route {
    let informe = collect(cfg).await;
    resolver_en(&informe.models, db, need)
}

/// Como `resolver_en`, pero devolviendo el elegido y la nota (lo que usa el chat).
pub fn elegir_para(
    models: &[UniversalModel],
    db: &Database,
    need: &Need,
) -> Result<(Candidate, String), String> {
    let r = resolver_en(models, db, need);
    match r.chosen {
        Some(c) => Ok((c, r.note)),
        None => Err(format!(
            "ningún modelo cubre «{}»: {}",
            need.capability, r.note
        )),
    }
}

/* ============================ Fijados a mano ============================ */

fn ahora() -> String {
    chrono::Utc::now().to_rfc3339()
}

pub fn bind_set(db: &Database, scope: &str, engine: &str, model: &str) -> Result<(), String> {
    if scope.trim().is_empty() || engine.trim().is_empty() || model.trim().is_empty() {
        return Err("hacen falta ámbito, motor y modelo".into());
    }
    db.conn()
        .execute(
            "INSERT INTO model_bindings (scope,engine,model,updated_at) VALUES (?1,?2,?3,?4)
             ON CONFLICT(scope) DO UPDATE SET engine=?2, model=?3, updated_at=?4",
            params![scope.trim(), engine.trim(), model.trim(), ahora()],
        )
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn bind_get(db: &Database, scope: &str) -> Option<Binding> {
    db.conn()
        .query_row(
            "SELECT scope,engine,model,updated_at FROM model_bindings WHERE scope=?1",
            params![scope],
            |r| {
                Ok(Binding {
                    scope: r.get(0)?,
                    engine: r.get(1)?,
                    model: r.get(2)?,
                    updated_at: r.get(3)?,
                })
            },
        )
        .ok()
}

pub fn binds(db: &Database) -> Vec<Binding> {
    let mut out = Vec::new();
    let Ok(mut st) = db.conn().prepare(
        "SELECT scope,engine,model,updated_at FROM model_bindings ORDER BY scope",
    ) else {
        return out;
    };
    if let Ok(filas) = st.query_map([], |r| {
        Ok(Binding {
            scope: r.get(0)?,
            engine: r.get(1)?,
            model: r.get(2)?,
            updated_at: r.get(3)?,
        })
    }) {
        for f in filas.flatten() {
            out.push(f);
        }
    }
    out
}

pub fn bind_clear(db: &Database, scope: &str) -> Result<bool, String> {
    db.conn()
        .execute("DELETE FROM model_bindings WHERE scope=?1", params![scope])
        .map(|n| n > 0)
        .map_err(|e| e.to_string())
}

/* ============================ Adaptadores ============================ */

/// Qué sabe hacer cada motor. Es la respuesta al «¿esto sirve para lo mío?» sin
/// tener que probarlo a ciegas.
pub fn adapters() -> Vec<AdapterInfo> {
    vec![
        AdapterInfo {
            engine: "ollama".into(),
            chat: true,
            stream: true,
            embeddings: true,
            image: false,
            video: false,
            audio: false,
            unload: true,
            status: true,
            detail: "API nativa: /api/chat, /api/embeddings, /api/ps y /api/show".into(),
        },
        AdapterInfo {
            engine: "lmstudio".into(),
            chat: true,
            stream: true,
            embeddings: true,
            image: false,
            video: false,
            audio: false,
            unload: true,
            status: true,
            detail: "OpenAI-compatible en 1234/1235; el puerto se sondea, no se supone".into(),
        },
        AdapterInfo {
            engine: "cloud".into(),
            chat: true,
            stream: true,
            embeddings: false,
            image: false,
            video: false,
            audio: false,
            unload: false,
            status: true,
            detail: "OpenAI-compatible con la clave del proveedor activo; solo si se autoriza"
                .into(),
        },
        AdapterInfo {
            engine: "comfyui".into(),
            chat: false,
            stream: false,
            embeddings: false,
            image: true,
            video: true,
            audio: false,
            unload: true,
            status: true,
            detail: "Genera imágenes y vídeo por flujo; no conversa".into(),
        },
    ]
}

/* ============================ Enumeración ============================ */

/// Enumera todos los motores. Nunca falla: lo que no responde queda como no
/// alcanzable, con el motivo, en vez de desaparecer sin dejar rastro.
pub async fn collect(cfg: &AppConfig) -> CatalogReport {
    let inicio = Instant::now();
    let mut models: Vec<UniversalModel> = Vec::new();
    let mut coverage: Vec<CatalogCoverage> = Vec::new();

    let (lista, detalle, ok) = ollama_modelos().await;
    coverage.push(CatalogCoverage {
        engine: "ollama".into(),
        name: "Ollama".into(),
        reachable: ok,
        models: lista.len(),
        catalog_api: true,
        detail: detalle,
    });
    models.extend(lista);

    let (lista, detalle, ok) = lmstudio_modelos().await;
    coverage.push(CatalogCoverage {
        engine: "lmstudio".into(),
        name: "LM Studio".into(),
        reachable: ok,
        models: lista.len(),
        catalog_api: true,
        detail: detalle,
    });
    models.extend(lista);

    let (lista, detalle, ok) = cloud_modelos(cfg).await;
    coverage.push(CatalogCoverage {
        engine: "cloud".into(),
        name: "Cloud".into(),
        reachable: ok,
        models: lista.len(),
        catalog_api: true,
        detail: detalle,
    });
    models.extend(lista);

    let (lista, detalle, ok) = comfyui_modelos().await;
    coverage.push(CatalogCoverage {
        engine: "comfyui".into(),
        name: "ComfyUI".into(),
        reachable: ok,
        models: lista.len(),
        catalog_api: true,
        detail: detalle,
    });
    models.extend(lista);

    models.sort_by(|a, b| a.key.cmp(&b.key));
    let total = models.len();

    CatalogReport {
        models,
        coverage,
        total,
        generated_ms: inicio.elapsed().as_millis() as u64,
    }
}

async fn get_json(url: &str) -> Result<Value, String> {
    let r = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(4))
        .user_agent("KUKIA-CODE/0.1 (catálogo)")
        .build()
        .map_err(|e| e.to_string())?
        .get(url)
        .send()
        .await
        .map_err(|e| format!("sin respuesta: {e}"))?;
    if !r.status().is_success() {
        return Err(format!("HTTP {}", r.status()));
    }
    r.json::<Value>().await.map_err(|e| e.to_string())
}

async fn ollama_modelos() -> (Vec<UniversalModel>, String, bool) {
    const BASE: &str = "http://127.0.0.1:11434";
    let tags = match get_json(&format!("{BASE}/api/tags")).await {
        Ok(v) => v,
        Err(e) => return (Vec::new(), e, false),
    };
    let cargados: Vec<String> = match get_json(&format!("{BASE}/api/ps")).await {
        Ok(v) => v["models"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(|m| m["name"].as_str().map(|s| s.to_string()))
            .collect(),
        Err(_) => Vec::new(),
    };

    let mut base: Vec<(String, u64, String, String)> = Vec::new();
    for m in tags["models"].as_array().cloned().unwrap_or_default() {
        let Some(nombre) = m["name"].as_str().map(|s| s.to_string()) else {
            continue;
        };
        let size = m["size"].as_u64().unwrap_or(0);
        let quant = m["details"]["quantization_level"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let family = m["details"]["family"].as_str().unwrap_or("").to_string();
        base.push((nombre, size, quant, family));
    }

    let tareas = base.into_iter().map(|(nombre, size, quant, family)| {
        let cargados = cargados.clone();
        async move {
            let (decl, ctx) = capacidades_ollama(&nombre).await;
            let (kind, capabilities, inferred) = clasificar("ollama", &nombre, &decl);
            let loaded = cargados.iter().any(|c| c == &nombre);
            let can_chat = kind == "chat";
            UniversalModel {
                key: format!("ollama::{nombre}"),
                engine: "ollama".into(),
                provider: "ollama".into(),
                id: nombre.clone(),
                kind,
                capabilities,
                context_length: ctx,
                size_gb: if size > 0 {
                    Some(size as f64 / 1e9)
                } else {
                    None
                },
                quantization: if quant.is_empty() { None } else { Some(quant) },
                format: Some("gguf".into()),
                loaded,
                local: true,
                can_chat,
                inferred,
                note: if family.is_empty() {
                    String::new()
                } else {
                    format!("familia {family}")
                },
            }
        }
    });

    let models: Vec<UniversalModel> = futures_util::stream::iter(tareas)
        .buffer_unordered(8)
        .collect()
        .await;

    let detalle = format!(
        "{} modelo(s) · {} cargado(s) · /api/tags + /api/show",
        models.len(),
        models.iter().filter(|m| m.loaded).count()
    );
    (models, detalle, true)
}

/// Capacidades declaradas y contexto nativo, leídos de `/api/show`.
async fn capacidades_ollama(model: &str) -> (Vec<String>, Option<u64>) {
    let cuerpo = serde_json::json!({ "model": model });
    let r = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(6))
        .build()
    {
        Ok(c) => c.post("http://127.0.0.1:11434/api/show").json(&cuerpo).send().await,
        Err(_) => return (Vec::new(), None),
    };
    let Ok(resp) = r else {
        return (Vec::new(), None);
    };
    let Ok(v) = resp.json::<Value>().await else {
        return (Vec::new(), None);
    };

    let decl: Vec<String> = v["capabilities"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|c| c.as_str().map(|s| s.to_string()))
        .collect();

    // El contexto nativo viene en `model_info` con claves tipo
    // `qwen3.context_length`: se busca por sufijo, como hace `control`.
    let ctx = v["model_info"]
        .as_object()
        .and_then(|o| {
            o.iter()
                .find(|(k, _)| k.ends_with(".context_length"))
                .and_then(|(_, val)| val.as_u64())
        })
        .filter(|c| *c > 0);

    (decl, ctx)
}

async fn lmstudio_modelos() -> (Vec<UniversalModel>, String, bool) {
    for port in crate::control::lmstudio_puertos() {
        let base = format!("http://127.0.0.1:{port}");
        let Ok(v) = get_json(&format!("{base}/v1/models")).await else {
            continue;
        };
        let mut models = Vec::new();
        for m in v["data"].as_array().cloned().unwrap_or_default() {
            let Some(id) = m["id"].as_str().map(|s| s.to_string()) else {
                continue;
            };
            let (kind, capabilities, inferred) = clasificar("lmstudio", &id, &[]);
            models.push(UniversalModel {
                key: format!("lmstudio::{id}"),
                engine: "lmstudio".into(),
                provider: "lmstudio".into(),
                id: id.clone(),
                kind: kind.clone(),
                capabilities,
                context_length: m["context_length"].as_u64(),
                size_gb: None,
                quantization: None,
                format: None,
                loaded: false,
                local: true,
                can_chat: kind == "chat",
                inferred,
                note: String::new(),
            });
        }
        let detalle = format!("{} modelo(s) en {base} · LM Studio no publica capas ni contexto", models.len());
        return (models, detalle, true);
    }
    (
        Vec::new(),
        "LM Studio no responde en 1234 ni 1235".into(),
        false,
    )
}

async fn cloud_modelos(cfg: &AppConfig) -> (Vec<UniversalModel>, String, bool) {
    match providers::list_models("cloud", cfg).await {
        Ok(lista) => {
            let mut models = Vec::new();
            for m in lista {
                let (kind, capabilities, inferred) = clasificar("cloud", &m.id, &[]);
                models.push(UniversalModel {
                    key: format!("cloud::{}", m.id),
                    engine: "cloud".into(),
                    provider: "cloud".into(),
                    id: m.id.clone(),
                    kind: kind.clone(),
                    capabilities,
                    context_length: m.context,
                    size_gb: None,
                    quantization: None,
                    format: None,
                    loaded: false,
                    local: false,
                    can_chat: kind == "chat",
                    inferred,
                    note: "remoto: solo con allow_cloud".into(),
                });
            }
            let detalle = format!("{} modelo(s) · {}", models.len(), m_note(&models));
            (models, detalle, true)
        }
        Err(e) => (Vec::new(), e, false),
    }
}

fn m_note(models: &[UniversalModel]) -> String {
    if models.is_empty() {
        "sin modelos".into()
    } else {
        "requiere clave y autorización".into()
    }
}

async fn comfyui_modelos() -> (Vec<UniversalModel>, String, bool) {
    const BASE: &str = "http://127.0.0.1:8188";
    let mut nombres: Vec<String> = Vec::new();

    if let Ok(v) = get_json(&format!("{BASE}/models/checkpoints")).await {
        for n in v.as_array().cloned().unwrap_or_default() {
            if let Some(s) = n.as_str() {
                nombres.push(s.to_string());
            }
        }
    }
    if nombres.is_empty() {
        if let Ok(v) = get_json(&format!("{BASE}/object_info/CheckpointLoaderSimple")).await {
            if let Some(arr) = v["CheckpointLoaderSimple"]["input"]["required"]["ckpt_name"][0]
                .as_array()
            {
                for n in arr {
                    if let Some(s) = n.as_str() {
                        nombres.push(s.to_string());
                    }
                }
            }
        }
    }

    if nombres.is_empty() {
        return (
            Vec::new(),
            "ComfyUI no responde en 8188 o no publica checkpoints".into(),
            false,
        );
    }

    let models = nombres
        .into_iter()
        .map(|id| UniversalModel {
            key: format!("comfyui::{id}"),
            engine: "comfyui".into(),
            provider: "comfyui".into(),
            id: id.clone(),
            kind: "imagen".into(),
            capabilities: vec!["imagen".into()],
            context_length: None,
            size_gb: None,
            quantization: None,
            format: Some("safetensors".into()),
            loaded: false,
            local: true,
            can_chat: false,
            inferred: true,
            note: "checkpoint de imagen: no conversa".into(),
        })
        .collect::<Vec<_>>();

    (
        models,
        "checkpoints de ComfyUI; se generan por flujo, no por chat".into(),
        true,
    )
}

/* ============================ Hablar ============================ */

/// Habla con el modelo elegido por su adaptador. El proveedor se deduce del
/// motor, no del nombre del modelo.
pub async fn hablar(
    cfg: &AppConfig,
    c: &Candidate,
    prompt: &str,
    max_tokens: Option<u32>,
) -> Result<UnifiedChat, String> {
    let provider = match c.engine.as_str() {
        "lmstudio" => "lmstudio",
        "cloud" => "cloud",
        "ollama" => "ollama",
        otro => {
            return Err(format!(
                "«{otro}» no tiene adaptador de chat; usa otro modelo del catálogo"
            ))
        }
    };

    let req = providers::ChatRequest {
        provider: provider.to_string(),
        model: c.id.clone(),
        messages: vec![providers::ChatMessage {
            role: "user".into(),
            content: prompt.to_string(),
        }],
        temperature: None,
        max_tokens,
        system_prompt: None,
        tools: false,
        aprobar: false,
        agent: None,
    };

    let r = providers::chat(&req, cfg).await?;
    Ok(UnifiedChat {
        engine: c.engine.clone(),
        model: c.id.clone(),
        content: r.content,
        prompt_tokens: r.prompt_tokens,
        completion_tokens: r.completion_tokens,
        total_ms: r.total_ms,
        note: String::new(),
    })
}

/* ============================ Tests ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    fn modelo(
        engine: &str,
        id: &str,
        caps: &[&str],
        ctx: Option<u64>,
        loaded: bool,
    ) -> UniversalModel {
        let decl: Vec<String> = caps.iter().map(|s| s.to_string()).collect();
        let (kind, capabilities, inferred) = clasificar(engine, id, &decl);
        UniversalModel {
            key: format!("{engine}::{id}"),
            engine: engine.into(),
            provider: if engine == "cloud" { "cloud" } else { engine }.into(),
            id: id.into(),
            kind: kind.clone(),
            capabilities,
            context_length: ctx,
            size_gb: Some(4.0),
            quantization: Some("Q4_K_M".into()),
            format: Some("gguf".into()),
            loaded,
            local: engine != "cloud",
            can_chat: kind == "chat" && engine != "comfyui",
            inferred,
            note: String::new(),
        }
    }

    fn db_temp(tag: &str) -> (Database, std::path::PathBuf) {
        let p = std::env::temp_dir().join(format!(
            "kukia_cat_test_{tag}_{}.db",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&p);
        let db = Database::open(&p).expect("db de prueba");
        (db, p)
    }

    #[test]
    fn un_embedder_no_es_de_chat() {
        let (kind, caps, _) = clasificar("ollama", "nomic-embed-text:latest", &[]);
        assert_eq!(kind, "embeddings");
        assert!(caps.contains(&"embeddings".to_string()));
    }

    #[test]
    fn coder_y_vision_se_deducen_del_nombre() {
        let (_, caps, inferido) = clasificar("ollama", "qwen2.5-coder:7b", &[]);
        assert!(caps.contains(&"codigo".to_string()));
        assert!(inferido);

        let (_, caps2, inf2) = clasificar("ollama", "llava:13b", &[]);
        assert!(caps2.contains(&"vision".to_string()));
        assert!(inf2);
    }

    #[test]
    fn lo_que_declara_el_motor_no_es_inferido() {
        let decl = vec!["completion".to_string(), "tools".to_string()];
        let (_, caps, inferido) = clasificar("ollama", "llama3.1:8b", &decl);
        assert!(caps.contains(&"herramientas".to_string()));
        assert!(!inferido);
    }

    #[test]
    fn comfyui_es_imagen_y_no_chatea() {
        let m = modelo("comfyui", "sdxl_base.safetensors", &[], None, false);
        assert_eq!(m.kind, "imagen");
        assert!(!m.can_chat);
    }

    #[test]
    fn el_router_prefiere_el_cargado() {
        let models = vec![
            modelo("ollama", "modelo-frio", &["completion"], Some(8192), false),
            modelo("ollama", "modelo-cargado", &["completion"], Some(8192), true),
        ];
        let r = enrutar(&models, &Need::default());
        assert_eq!(r.chosen.unwrap().id, "modelo-cargado");
    }

    #[test]
    fn el_cloud_no_se_usa_sin_permiso() {
        let models = vec![
            modelo("cloud", "gpt-remoto", &["completion"], Some(128_000), false),
            modelo("ollama", "local", &["completion"], Some(8192), false),
        ];
        let r = enrutar(&models, &Need::default());
        assert_eq!(r.chosen.unwrap().id, "local");
        assert!(r
            .rejected
            .iter()
            .any(|x| x.key == "cloud::gpt-remoto" && x.why.contains("no está autorizado")));
    }

    #[test]
    fn sin_contexto_suficiente_se_descarta() {
        let models = vec![modelo("ollama", "corto", &["completion"], Some(4096), true)];
        let need = Need {
            min_context: Some(32_768),
            ..Default::default()
        };
        let r = enrutar(&models, &need);
        assert!(r.chosen.is_none());
        assert!(r.rejected[0].why.contains("4096"));
    }

    #[test]
    fn el_fijado_a_mano_gana_al_automatico() {
        let (db, path) = db_temp("bind");
        let models = vec![
            modelo("ollama", "preferido", &["completion"], Some(8192), true),
            modelo("ollama", "fijado", &["completion"], Some(8192), false),
        ];
        bind_set(&db, "kukia-coder", "ollama", "fijado").unwrap();
        let need = Need {
            scope: Some("kukia-coder".into()),
            ..Default::default()
        };
        let r = resolver_en(&models, &db, &need);
        assert_eq!(r.chosen.unwrap().id, "fijado");
        assert!(r.note.contains("fijado"));
        drop(db);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn un_fijado_que_ya_no_existe_avisa_y_enruta() {
        let (db, path) = db_temp("bind_roto");
        let models = vec![modelo("ollama", "vivo", &["completion"], Some(8192), true)];
        bind_set(&db, "kukia-gpu", "ollama", "borrado").unwrap();
        let need = Need {
            scope: Some("kukia-gpu".into()),
            ..Default::default()
        };
        let r = resolver_en(&models, &db, &need);
        assert_eq!(r.chosen.unwrap().id, "vivo");
        assert!(r.note.contains("ya no está disponible"));
        drop(db);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn las_capacidades_son_legibles_por_la_herramienta() {
        let models = vec![
            modelo("ollama", "m1", &["completion", "vision"], Some(8192), false),
            modelo("ollama", "m2", &["completion"], Some(8192), true),
        ];
        let need = Need {
            capability: "vision".into(),
            ..Default::default()
        };
        let r = enrutar(&models, &need);
        assert_eq!(r.chosen.unwrap().id, "m1");
        assert_eq!(r.alternatives.len(), 0);
    }

    #[test]
    fn los_fijados_se_listan_y_se_borran() {
        let (db, path) = db_temp("bind_lista");
        bind_set(&db, "b", "ollama", "m2").unwrap();
        bind_set(&db, "a", "ollama", "m1").unwrap();
        let l = binds(&db);
        assert_eq!(l.len(), 2);
        assert_eq!(l[0].scope, "a", "se listan ordenados por ámbito");
        assert!(bind_clear(&db, "a").unwrap());
        assert!(!bind_clear(&db, "a").unwrap(), "borrar dos veces no inventa filas");
        drop(db);
        let _ = std::fs::remove_file(path);
    }
}
