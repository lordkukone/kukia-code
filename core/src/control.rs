//! Panel avanzado de modelo (contexto, KV cache, GPU, motor y generación).
//!
//! Todo lo que aquí se afirma está medido contra los motores de esta máquina
//! (01/11/2026), no copiado de una lista de deseos:
//!
//! - **`num_ctx` de Ollama se aplica y recarga el modelo.** Medido con
//!   `qwen3-vl:4b`: a 8.192 tokens declaró 3,45 GB de VRAM; a 2.048 recargó.
//! - **`num_gpu: 0` manda todo a CPU**: la VRAM del modelo bajó a 0,0 GB y la
//!   5060 Ti pasó de 4.443 MB a 212 MB. El control de capas existe de verdad.
//! - **`kv_cache_type` NO tiene efecto por petición** en Ollama 0.40.13: con
//!   `q8_0` la VRAM declarada fue idéntica (3,45 GB). Solo existe como variable
//!   de entorno del servidor, y KUKIA no reinicia procesos: se ofrece como dato,
//!   no como un botón que mienta.
//! - **LM Studio** expone `lms load --gpu off|max|0..1`, `-c`, `--parallel`,
//!   `--ttl` y `--speculative-draft-*`; no expone KV cache, flash attention ni
//!   RoPE.
//! - **RoPE/YaRN no está expuesto por ninguno de los dos.** Se informa del límite
//!   nativo del modelo en vez de ofrecer un control que no hace nada.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::engines::InferenceParams;

const OLLAMA_BASE: &str = "http://127.0.0.1:11434";
const LM_PORTS: [u16; 2] = [1234, 1235];

fn client(secs: u64) -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(secs))
        .user_agent("KUKIA-CODE/0.1")
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

/* ===================== Arquitectura del modelo y caché KV ===================== */

/// Lo que `/api/show` dice del modelo, en campos utilizables.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Arch {
    pub architecture: String,
    pub layers: u64,
    pub heads: u64,
    pub heads_kv: u64,
    pub embedding: u64,
    pub ctx_native: u64,
}

impl Arch {
    /// Dimensión por cabeza. Si el modelo no la publica se deduce del embedding.
    pub fn head_dim(&self) -> f64 {
        if self.heads == 0 {
            return 0.0;
        }
        self.embedding as f64 / self.heads as f64
    }

    /// Bytes de KV cache para un contexto y una precisión dados.
    ///
    /// Fórmula: `2 (K y V) × capas × cabezas_kv × dim_cabeza × tokens × bytes`.
    /// Validada con lo medido: 36 capas, 8 cabezas KV, dim 80, 8.192 tokens y
    /// fp16 dan 0,70 GB, que sumados a los ~2,75 GB de pesos cuadran con los
    /// 3,45 GB que declaró Ollama.
    pub fn kv_bytes(&self, ctx: u64, bits: f64) -> u64 {
        if self.layers == 0 || self.heads_kv == 0 || self.head_dim() == 0.0 {
            return 0;
        }
        let elementos = 2.0
            * self.layers as f64
            * self.heads_kv as f64
            * self.head_dim()
            * ctx as f64;
        (elementos * bits / 8.0).round() as u64
    }
}

/// Formatos de caché KV con su coste real en bits por valor.
///
/// `q8_0` y `q4_0` son bloques de 32 valores con una escala de 2 bytes:
/// 8 + 2/32 = 8,25 bits (se cuenta 8,5 por el redondeo del bloque).
pub const KV_FORMATOS: [(&str, f64); 6] = [
    ("fp16", 16.0),
    ("bf16", 16.0),
    ("q8_0", 8.5),
    ("q6_k", 6.5625),
    ("q5_k", 5.5),
    ("q4_0", 4.5),
];

#[derive(Debug, Clone, Serialize)]
pub struct KvOption {
    pub formato: String,
    pub gb: f64,
    pub ahorro_gb: f64,
    pub ahorro_pct: f64,
    /// `en_uso` | `disponible` | `no_aplicable` | `sin_verificar`
    pub estado: String,
    pub motivo: String,
}

/// Variable de entorno de Ollama que fija el formato de la caché KV.
///
/// En esta máquina vale `q4_0` (medido el 01/11/2026), y por eso el modelo de 39,8B
/// cabe a 262K de contexto. El formato por petición se ignora, así que el valor
/// activo solo se puede conocer leyendo el entorno del servidor.
pub fn kv_entorno() -> Option<(String, String)> {
    // Se lee del proceso y, si no está, del registro de Windows: la variable del
    // servidor puede no estar en la sesión del shell.
    let v = crate::lab::valor_entorno("OLLAMA_KV_CACHE_TYPE")?;
    let v = v.trim();
    if v.is_empty() {
        None
    } else {
        Some(("OLLAMA_KV_CACHE_TYPE".into(), v.to_string()))
    }
}

/// Tabla de KV para un contexto: cuánto ocupa cada formato y si se puede usar.
///
/// El estado sale de la auditoría: en Ollama 0.40.13 `kv_cache_type` por petición
/// no cambia nada, así que el formato lo decide el **entorno del servidor**: el
/// que esté en `OLLAMA_KV_CACHE_TYPE` se marca `activo_entorno`, y el resto se
/// muestra con su ahorro para que se vea lo que se ganaría, pero marcado.
pub fn kv_tabla(arch: &Arch, ctx: u64, engine: &str) -> Vec<KvOption> {
    let base = arch.kv_bytes(ctx, 16.0) as f64 / 1_073_741_824.0;
    let activo = if engine == "ollama" { kv_entorno() } else { None };
    KV_FORMATOS
        .iter()
        .map(|(nombre, bits)| {
            let gb = arch.kv_bytes(ctx, *bits) as f64 / 1_073_741_824.0;
            let ahorro = (base - gb).max(0.0);
            let es_activo = activo
                .as_ref()
                .map(|(_, v)| v.eq_ignore_ascii_case(nombre))
                .unwrap_or(false);
            let (estado, motivo) = if es_activo {
                (
                    "activo_entorno",
                    format!(
                        "es el formato que Ollama tiene activo ahora mismo: OLLAMA_KV_CACHE_TYPE={}. \
                         No cambia por petición",
                        activo.as_ref().map(|(_, v)| v.as_str()).unwrap_or("")
                    ),
                )
            } else if activo.is_some() {
                (
                    "no_aplicable",
                    format!(
                        "el servidor está en {}: cambiarlo exige otra OLLAMA_KV_CACHE_TYPE y \
                         reiniciar Ollama, y KUKIA no reinicia procesos",
                        activo.as_ref().map(|(_, v)| v.as_str()).unwrap_or("")
                    ),
                )
            } else if *nombre == "fp16" || *nombre == "bf16" {
                (
                    "por_defecto",
                    "sin OLLAMA_KV_CACHE_TYPE definida, Ollama usa fp16 para la caché KV".to_string(),
                )
            } else if engine == "ollama" {
                (
                    "no_aplicable",
                    "Ollama 0.40.13 ignora kv_cache_type por petición (medido: misma VRAM con \
                     q8_0). Solo existe como OLLAMA_KV_CACHE_TYPE en el servidor, y KUKIA no \
                     reinicia procesos"
                        .to_string(),
                )
            } else {
                (
                    "no_aplicable",
                    "la API de LM Studio no expone el formato de la caché KV".to_string(),
                )
            };
            KvOption {
                formato: (*nombre).to_string(),
                gb,
                ahorro_gb: ahorro,
                ahorro_pct: if base > 0.0 { ahorro / base * 100.0 } else { 0.0 },
                estado: estado.to_string(),
                motivo,
            }
        })
        .collect()
}

/* ============================ Capacidades por motor ============================ */

#[derive(Debug, Clone, Serialize)]
pub struct ParamSpec {
    pub key: String,
    pub etiqueta: String,
    pub grupo: String,
    pub tipo: String,
    /// `caliente` | `recarga` | `sesion` | `no_compatible` | `sin_verificar`
    pub aplica: String,
    pub motivo: String,
    pub valor: serde_json::Value,
    pub defecto: serde_json::Value,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub opciones: Vec<String>,
}

fn spec(
    key: &str,
    etiqueta: &str,
    grupo: &str,
    tipo: &str,
    aplica: &str,
    motivo: &str,
    valor: serde_json::Value,
    defecto: serde_json::Value,
) -> ParamSpec {
    ParamSpec {
        key: key.into(),
        etiqueta: etiqueta.into(),
        grupo: grupo.into(),
        tipo: tipo.into(),
        aplica: aplica.into(),
        motivo: motivo.into(),
        valor,
        defecto,
        min: None,
        max: None,
        opciones: Vec::new(),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextoInfo {
    pub actual: u64,
    pub maximo_modelo: u64,
    pub maximo_servido: u64,
    pub fuente: String,
    /// Memoria extra de KV por cada 8K adicionales (GB, fp16).
    pub gb_por_8k: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Capacidades {
    pub engine: String,
    pub model: String,
    pub arch: Arch,
    pub contexto: ContextoInfo,
    pub kv: Vec<KvOption>,
    pub parametros: Vec<ParamSpec>,
    pub gpus: Vec<GpuUso>,
    pub avisos: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GpuUso {
    pub index: u32,
    pub name: String,
    pub total_mb: u64,
    pub used_mb: u64,
    pub free_mb: u64,
    pub utilization: u64,
    pub temperatura_c: f64,
    pub potencia_w: f64,
}

fn gpus_uso() -> Vec<GpuUso> {
    crate::gpu::stats()
        .into_iter()
        .map(|g| GpuUso {
            index: g.index,
            name: g.name,
            total_mb: g.memory_total_mb,
            used_mb: g.memory_used_mb,
            free_mb: g.memory_total_mb.saturating_sub(g.memory_used_mb),
            utilization: g.utilization as u64,
            temperatura_c: g.temperature_c,
            potencia_w: g.power_w,
        })
        .collect()
}

/// Arquitectura real del modelo según `/api/show` (Ollama).
pub async fn arch_ollama(model: &str) -> Result<Arch, String> {
    let r = client(10)
        .post(format!("{OLLAMA_BASE}/api/show"))
        .json(&serde_json::json!({ "model": model }))
        .send()
        .await
        .map_err(|e| format!("Ollama no responde: {e}"))?;
    if !r.status().is_success() {
        return Err(format!("HTTP {}", r.status()));
    }
    let v: serde_json::Value = r.json().await.map_err(|e| e.to_string())?;
    Ok(arch_de_model_info(&v))
}

/// Extrae los campos útiles de `model_info` sin asumir el nombre de la familia.
pub fn arch_de_model_info(v: &serde_json::Value) -> Arch {
    let info = v.get("model_info").cloned().unwrap_or(serde_json::Value::Null);
    let architecture = info
        .get("general.architecture")
        .and_then(|a| a.as_str())
        .unwrap_or("")
        .to_string();
    let buscar = |sufijo: &str| -> u64 {
        info.as_object()
            .and_then(|o| {
                o.iter()
                    .find(|(k, _)| k.ends_with(sufijo))
                    .and_then(|(_, val)| val.as_u64())
            })
            .unwrap_or(0)
    };
    let heads = buscar(".attention.head_count").max(1);
    let heads_kv = {
        let kv = buscar(".attention.head_count_kv");
        if kv == 0 {
            heads
        } else {
            kv
        }
    };
    Arch {
        architecture,
        layers: buscar(".block_count"),
        heads,
        heads_kv,
        embedding: buscar(".embedding_length"),
        ctx_native: buscar(".context_length"),
    }
}

/// Contexto que el motor declara ahora mismo para el modelo, si está cargado.
async fn contexto_cargado_ollama(model: &str) -> Option<u64> {
    let r = client(4).get(format!("{OLLAMA_BASE}/api/ps")).send().await.ok()?;
    let v: serde_json::Value = r.json().await.ok()?;
    v.get("models")?
        .as_array()?
        .iter()
        .find(|m| m.get("name").and_then(|n| n.as_str()) == Some(model))
        .and_then(|m| m.get("context_length"))
        .and_then(|c| c.as_u64())
}

/// Panel completo de un modelo: contexto, KV, GPU y parámetros con su estado.
pub async fn capacidades(engine: &str, model: &str) -> Capacidades {
    let p = crate::engines::params_get(engine, model);
    let mut avisos = Vec::new();

    let arch = if engine == "ollama" {
        match arch_ollama(model).await {
            Ok(a) => a,
            Err(e) => {
                avisos.push(format!("no se pudo leer la arquitectura del modelo: {e}"));
                Arch::default()
            }
        }
    } else {
        avisos.push(
            "LM Studio no publica capas ni cabezas del modelo: la tabla de KV y el máximo de \
             contexto no se pueden calcular aquí"
                .to_string(),
        );
        Arch::default()
    };

    let cargado = if engine == "ollama" {
        contexto_cargado_ollama(model).await
    } else {
        None
    };
    let actual = cargado.or(p.num_ctx).unwrap_or(0);
    let maximo_modelo = arch.ctx_native;
    // Ollama sirve hasta 262.144 por defecto en esta instalación; se informa el
    // nativo del modelo como tope real, y el "servido" solo si es menor.
    let maximo_servido = if maximo_modelo > 0 {
        maximo_modelo
    } else {
        p.num_ctx.unwrap_or(0)
    };
    let gb_por_8k = arch.kv_bytes(8192, 16.0) as f64 / 1_073_741_824.0;

    let contexto = ContextoInfo {
        actual,
        maximo_modelo,
        maximo_servido,
        fuente: if cargado.is_some() {
            "Ollama /api/ps (context_length del modelo cargado)".into()
        } else if p.num_ctx.is_some() {
            "parámetro guardado en KUKIA (aún sin cargar)".into()
        } else {
            "por defecto del motor".into()
        },
        gb_por_8k,
    };

    if maximo_modelo > 0 && gb_por_8k > 0.0 {
        let kv_max = arch.kv_bytes(maximo_modelo, 16.0) as f64 / 1_073_741_824.0;
        avisos.push(format!(
            "con el contexto nativo ({} tokens) la caché KV sola pediría {kv_max:.1} GB",
            maximo_modelo
        ));
    }

    let parametros = if engine == "ollama" {
        params_ollama(&p, &arch)
    } else if engine == "lmstudio" {
        params_lmstudio(&p)
    } else {
        vec![spec(
            "ninguno",
            "Sin control",
            "motor",
            "texto",
            "no_compatible",
            "este motor no publica parámetros ajustables (no tiene API de inferencia)",
            serde_json::Value::Null,
            serde_json::Value::Null,
        )]
    };

    Capacidades {
        engine: engine.into(),
        model: model.into(),
        kv: kv_tabla(&arch, if actual > 0 { actual } else { 8192 }, engine),
        arch,
        contexto,
        parametros,
        gpus: gpus_uso(),
        avisos,
    }
}

fn num(v: Option<f64>) -> serde_json::Value {
    v.map(|x| serde_json::json!(x)).unwrap_or(serde_json::Value::Null)
}

/// Parámetros de Ollama con el estado **medido** de cada uno.
fn params_ollama(p: &InferenceParams, arch: &Arch) -> Vec<ParamSpec> {
    let mut out = Vec::new();

    // --- Contexto ---
    let mut c = spec(
        "num_ctx",
        "Contexto (tokens)",
        "contexto",
        "numero",
        "recarga",
        "medido: cambiar num_ctx hace que Ollama recargue el modelo (3,45 GB a 8K)",
        serde_json::json!(p.num_ctx.unwrap_or(0)),
        serde_json::json!(0),
    );
    c.min = Some(2048.0);
    c.max = Some(if arch.ctx_native > 0 { arch.ctx_native as f64 } else { 262144.0 });
    out.push(c);

    // --- GPU y capas ---
    let mut g = spec(
        "num_gpu",
        "Capas en GPU (num_gpu)",
        "gpu",
        "numero",
        "recarga",
        "medido: num_gpu 0 manda todo a CPU (VRAM del modelo 0,0 GB). El cambio recarga",
        serde_json::json!(p.num_gpu.unwrap_or(-1)),
        serde_json::json!(-1),
    );
    g.min = Some(-1.0);
    g.max = Some(arch.layers as f64);
    out.push(g);
    out.push(spec(
        "gpu_split",
        "Reparto entre las dos GPU",
        "gpu",
        "texto",
        "no_compatible",
        "Ollama reparte solo; elegir tarjeta exige CUDA_VISIBLE_DEVICES, que es del proceso \
         del servidor: KUKIA no lo reinicia (medido: repartió 11,5 + 9,6 GB sin preguntar)",
        serde_json::Value::Null,
        serde_json::Value::Null,
    ));
    out.push(spec(
        "tensor_split",
        "Tensor Split / GPU Split",
        "gpu",
        "texto",
        "no_compatible",
        "no expuesto por la API de Ollama 0.40.13; en LM Studio el offload es un ratio, no un reparto",
        serde_json::Value::Null,
        serde_json::Value::Null,
    ));

    // --- Motor ---
    out.push(spec(
        "flash_attn",
        "Flash Attention",
        "motor",
        "bool",
        "recarga",
        "opción de carga de Ollama (options.flash_attn); se aplica al recargar el modelo",
        serde_json::json!(p.flash_attn.unwrap_or(false)),
        serde_json::json!(false),
    ));
    out.push(spec(
        "num_batch",
        "Batch size (num_batch)",
        "motor",
        "numero",
        "recarga",
        "opción de carga de Ollama: cambia con la recarga, no en caliente",
        serde_json::json!(p.num_batch.unwrap_or(0)),
        serde_json::json!(0),
    ));
    out.push(spec(
        "num_thread",
        "Hilos de CPU (num_thread)",
        "motor",
        "numero",
        "recarga",
        "opción de carga; 0 deja que el motor decida",
        serde_json::json!(p.num_thread.unwrap_or(0)),
        serde_json::json!(0),
    ));
    out.push(spec(
        "use_mmap",
        "mmap",
        "motor",
        "bool",
        "recarga",
        "opción de carga del runner de Ollama",
        serde_json::json!(p.use_mmap.unwrap_or(true)),
        serde_json::json!(true),
    ));
    out.push(spec(
        "use_mlock",
        "mlock (fijar en RAM)",
        "motor",
        "bool",
        "recarga",
        "opción de carga del runner de Ollama",
        serde_json::json!(p.use_mlock.unwrap_or(false)),
        serde_json::json!(false),
    ));
    out.push(spec(
        "kv_cache_type",
        "Formato de caché KV",
        "motor",
        "texto",
        "no_compatible",
        "medido: Ollama 0.40.13 ignora kv_cache_type por petición (q8_0 dio la misma VRAM). \
         Solo OLLAMA_KV_CACHE_TYPE en el servidor, y KUKIA no reinicia procesos",
        serde_json::Value::Null,
        serde_json::Value::Null,
    ));
    out.push(spec(
        "rope",
        "RoPE / YaRN",
        "motor",
        "texto",
        "no_compatible",
        "ni Ollama ni LM Studio exponen rope_freq_base/scale por API: el contexto se limita al \
         nativo del modelo",
        serde_json::Value::Null,
        serde_json::Value::Null,
    ));

    // --- Generación (caliente) ---
    let mut gen = |key: &str, etiqueta: &str, valor: serde_json::Value, defecto: serde_json::Value, min: Option<f64>, max: Option<f64>| {
        let mut s = spec(
            key,
            etiqueta,
            "generacion",
            "numero",
            "caliente",
            "opción de muestreo por petición: no recarga el modelo",
            valor,
            defecto,
        );
        s.min = min;
        s.max = max;
        out.push(s);
    };
    gen("temperature", "Temperature", num(p.temperature), serde_json::json!(0.8), Some(0.0), Some(2.0));
    gen("top_p", "Top P", num(p.top_p), serde_json::json!(0.9), Some(0.0), Some(1.0));
    gen("top_k", "Top K", num(p.top_k.map(|v| v as f64)), serde_json::json!(40), Some(0.0), Some(200.0));
    gen("min_p", "Min P", num(p.min_p), serde_json::json!(0.0), Some(0.0), Some(1.0));
    gen("repeat_penalty", "Repetition penalty", num(p.repeat_penalty), serde_json::json!(1.1), Some(0.0), Some(4.0));
    gen("repeat_last_n", "Repeat last N", num(p.repeat_last_n.map(|v| v as f64)), serde_json::json!(64), Some(0.0), Some(4096.0));
    gen("presence_penalty", "Presence penalty", num(p.presence_penalty), serde_json::json!(0.0), Some(-2.0), Some(2.0));
    gen("frequency_penalty", "Frequency penalty", num(p.frequency_penalty), serde_json::json!(0.0), Some(-2.0), Some(2.0));
    gen("max_tokens", "Máx. tokens de salida", num(p.max_tokens.map(|v| v as f64)), serde_json::json!(0), Some(0.0), Some(32768.0));
    gen("seed", "Semilla", num(p.seed.map(|v| v as f64)), serde_json::json!(0), None, None);
    out.push(spec(
        "stop",
        "Secuencias de parada",
        "generacion",
        "texto",
        "caliente",
        "lista separada por comas; se aplica por petición",
        serde_json::json!(p.stop.clone()),
        serde_json::json!([]),
    ));
    out
}

/// Parámetros de LM Studio: solo lo que su CLI y su API aceptan de verdad.
fn params_lmstudio(p: &InferenceParams) -> Vec<ParamSpec> {
    let mut out = Vec::new();
    let mut ctx = spec(
        "num_ctx",
        "Contexto (tokens)",
        "contexto",
        "numero",
        "recarga",
        "se aplica con `lms load -c`: recarga el modelo",
        serde_json::json!(p.num_ctx.unwrap_or(0)),
        serde_json::json!(0),
    );
    ctx.min = Some(512.0);
    ctx.max = Some(262144.0);
    out.push(ctx);
    let mut off = spec(
        "gpu_offload",
        "Offload a GPU",
        "gpu",
        "texto",
        "recarga",
        "`lms load --gpu off|max|0..1`: es un ratio de offload, no un número de capas",
        serde_json::json!(p.gpu_offload.clone().unwrap_or_else(|| "auto".into())),
        serde_json::json!("auto"),
    );
    off.opciones = vec!["auto".into(), "off".into(), "max".into(), "0.5".into()];
    out.push(off);
    out.push(spec(
        "parallel",
        "Peticiones en paralelo",
        "motor",
        "numero",
        "recarga",
        "`lms load --parallel`: se fija al cargar",
        serde_json::json!(p.parallel.unwrap_or(0)),
        serde_json::json!(0),
    ));
    out.push(spec(
        "speculative",
        "Speculative decoding",
        "motor",
        "bool",
        "recarga",
        "`lms load --speculative-draft-mtp`; solo si el modelo lo soporta",
        serde_json::json!(p.speculative.unwrap_or(false)),
        serde_json::json!(false),
    ));
    out.push(spec(
        "kv_cache_type",
        "Formato de caché KV",
        "motor",
        "texto",
        "no_compatible",
        "ni la API ni el CLI de LM Studio exponen el formato de la caché KV",
        serde_json::Value::Null,
        serde_json::Value::Null,
    ));
    out.push(spec(
        "flash_attn",
        "Flash Attention",
        "motor",
        "bool",
        "no_compatible",
        "LM Studio lo decide por dentro; no hay campo en la API ni en `lms load`",
        serde_json::Value::Null,
        serde_json::Value::Null,
    ));
    out.push(spec(
        "rope",
        "RoPE / YaRN",
        "motor",
        "texto",
        "no_compatible",
        "no expuesto por LM Studio",
        serde_json::Value::Null,
        serde_json::Value::Null,
    ));
    for (key, etiqueta, valor, min, max) in [
        ("temperature", "Temperature", num(p.temperature), 0.0, 2.0),
        ("top_p", "Top P", num(p.top_p), 0.0, 1.0),
        ("max_tokens", "Máx. tokens de salida", num(p.max_tokens.map(|v| v as f64)), 0.0, 32768.0),
        ("seed", "Semilla", num(p.seed.map(|v| v as f64)), 0.0, 0.0),
    ] {
        let mut s = spec(
            key,
            etiqueta,
            "generacion",
            "numero",
            "caliente",
            "campo estándar de su API compatible OpenAI",
            valor,
            serde_json::json!(0),
        );
        s.min = Some(min);
        s.max = Some(max);
        out.push(s);
    }
    for key in ["top_k", "min_p", "repeat_penalty"] {
        out.push(spec(
            key,
            key,
            "generacion",
            "numero",
            "sin_verificar",
            "no confirmado en esta instalación: no se envía sin poder comprobarlo",
            serde_json::Value::Null,
            serde_json::Value::Null,
        ));
    }
    out
}

/* ================================ Perfiles ================================ */

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Perfil {
    pub nombre: String,
    pub descripcion: String,
    pub params: InferenceParams,
    pub creado: String,
}

fn perfiles_path() -> PathBuf {
    crate::config::data_dir().join("perfiles_modelo.json")
}

fn leer_perfiles() -> Vec<Perfil> {
    std::fs::read_to_string(perfiles_path())
        .ok()
        .and_then(|t| serde_json::from_str::<Vec<Perfil>>(&t).ok())
        .unwrap_or_default()
}

fn escribir_perfiles(lista: &[Perfil]) -> Result<(), String> {
    let ruta = perfiles_path();
    if let Some(dir) = ruta.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let t = serde_json::to_string_pretty(lista).map_err(|e| e.to_string())?;
    std::fs::write(&ruta, t).map_err(|e| e.to_string())
}

pub fn perfiles_listar() -> Vec<Perfil> {
    let mut lista = leer_perfiles();
    if lista.is_empty() {
        lista = perfiles_de_fabrica();
        let _ = escribir_perfiles(&lista);
    }
    lista
}

fn p(
    temp: f64,
    top_p: f64,
    top_k: i64,
    num_ctx: u64,
    num_gpu: i32,
) -> InferenceParams {
    InferenceParams {
        temperature: Some(temp),
        top_p: Some(top_p),
        top_k: Some(top_k),
        num_ctx: Some(num_ctx),
        num_gpu: Some(num_gpu),
        ..Default::default()
    }
}

/// Perfiles que el panel ofrece de entrada. Los valores son puntos de partida
/// editables, no promesas: cada uno se puede duplicar y ajustar.
fn perfiles_de_fabrica() -> Vec<Perfil> {
    let ahora = chrono::Utc::now().to_rfc3339();
    let f = |nombre: &str, descripcion: &str, params: InferenceParams| Perfil {
        nombre: nombre.into(),
        descripcion: descripcion.into(),
        params,
        creado: ahora.clone(),
    };
    vec![
        f(
            "EQUILIBRADO",
            "8K de contexto, muestreo suave, todas las capas en GPU si caben",
            p(0.7, 0.9, 40, 8192, -1),
        ),
        f(
            "MÁXIMA VELOCIDAD",
            "4K de contexto y penalización mínima: lo más rápido para charlar",
            p(0.6, 0.85, 20, 4096, -1),
        ),
        f(
            "MÁXIMA CALIDAD",
            "32K de contexto y muestreo estrecho para respuestas más cuidadas",
            p(0.3, 0.95, 60, 32768, -1),
        ),
        f(
            "AHORRO DE VRAM",
            "4K de contexto y flash attention: la caché KV baja mucho; el reparto de capas \
             lo decide el motor (un número fijo de capas no vale para todos los modelos)",
            InferenceParams {
                flash_attn: Some(true),
                ..p(0.7, 0.9, 40, 4096, -1)
            },
        ),
        f(
            "CONTEXTO EXTENDIDO",
            "64K de contexto; aviso: la caché KV crece y puede no caber",
            p(0.6, 0.9, 40, 65536, -1),
        ),
        f(
            "GPU + RAM OFFLOAD",
            "2K de contexto y cero capas en GPU: todo a CPU para no tocar la VRAM",
            p(0.7, 0.9, 40, 2048, 0),
        ),
    ]
}

pub fn perfil_guardar(mut perfil: Perfil) -> Result<Vec<Perfil>, String> {
    if perfil.nombre.trim().is_empty() {
        return Err("el perfil necesita un nombre".into());
    }
    perfil.nombre = perfil.nombre.trim().to_string();
    if perfil.creado.trim().is_empty() {
        perfil.creado = chrono::Utc::now().to_rfc3339();
    }
    let mut lista = perfiles_listar();
    match lista.iter_mut().find(|x| x.nombre.eq_ignore_ascii_case(&perfil.nombre)) {
        Some(existente) => *existente = perfil,
        None => lista.push(perfil),
    }
    escribir_perfiles(&lista)?;
    Ok(lista)
}

pub fn perfil_borrar(nombre: &str) -> Result<Vec<Perfil>, String> {
    let mut lista = perfiles_listar();
    lista.retain(|x| !x.nombre.eq_ignore_ascii_case(nombre));
    escribir_perfiles(&lista)?;
    Ok(lista)
}

/// Aplica un perfil a un modelo guardando **antes** lo que había, para poder
/// deshacerlo. Devuelve los avisos de lo que el motor no soporta.
pub fn perfil_aplicar(
    engine: &str,
    model: &str,
    nombre: &str,
) -> Result<(InferenceParams, Vec<String>), String> {
    let perfil = perfiles_listar()
        .into_iter()
        .find(|x| x.nombre.eq_ignore_ascii_case(nombre))
        .ok_or_else(|| format!("no existe el perfil {nombre}"))?;
    let anterior = crate::engines::params_get(engine, model);

    // Copia de seguridad del estado previo, por modelo.
    let respaldo = crate::config::data_dir().join("perfiles_anterior.json");
    let mut previos: std::collections::BTreeMap<String, InferenceParams> = std::fs::read_to_string(&respaldo)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    previos.insert(format!("{}::{}", engine, model), anterior);
    if let Ok(t) = serde_json::to_string_pretty(&previos) {
        let _ = std::fs::write(&respaldo, t);
    }

    let avisos = crate::engines::params_set(engine, model, perfil.params.clone());
    Ok((perfil.params, avisos))
}

/// Devuelve los parámetros que había antes de aplicar el último perfil.
pub fn perfil_restaurar_anterior(engine: &str, model: &str) -> Option<InferenceParams> {
    let respaldo = crate::config::data_dir().join("perfiles_anterior.json");
    let previos: std::collections::BTreeMap<String, InferenceParams> =
        std::fs::read_to_string(&respaldo)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
    let clave = format!("{}::{}", engine, model);
    let anterior = previos.get(&clave).cloned();
    if let Some(a) = &anterior {
        crate::engines::params_set(engine, model, a.clone());
    }
    anterior
}

/* ================================ Medidores ================================ */

#[derive(Debug, Clone, Serialize)]
pub struct Metricas {
    pub gpus: Vec<GpuUso>,
    pub ram_total_gb: f64,
    pub ram_usada_gb: f64,
    pub ram_libre_gb: f64,
    pub cpu_uso_pct: f64,
    pub cargados: Vec<String>,
    pub tomado_en: String,
}

/// Medidores en tiempo real. No inventa tokens/s: eso exige generar.
pub async fn metricas() -> Metricas {
    use sysinfo::System;
    let mut sys = System::new();
    sys.refresh_memory();
    sys.refresh_cpu_usage();
    let total = sys.total_memory() as f64 / 1_073_741_824.0;
    let usada = sys.used_memory() as f64 / 1_073_741_824.0;
    let cargados = match client(3).get(format!("{OLLAMA_BASE}/api/ps")).send().await {
        Ok(r) => r
            .json::<serde_json::Value>()
            .await
            .ok()
            .and_then(|v| {
                v.get("models").and_then(|m| m.as_array()).map(|a| {
                    a.iter()
                        .filter_map(|m| m.get("name").and_then(|n| n.as_str()).map(String::from))
                        .collect::<Vec<_>>()
                })
            })
            .unwrap_or_default(),
        Err(_) => Vec::new(),
    };
    Metricas {
        gpus: gpus_uso(),
        ram_total_gb: total,
        ram_usada_gb: usada,
        ram_libre_gb: (total - usada).max(0.0),
        cpu_uso_pct: sys.global_cpu_usage() as f64,
        cargados,
        tomado_en: chrono::Utc::now().to_rfc3339(),
    }
}

/// Puertos reales de LM Studio, para que la interfaz no asuma 1234.
pub fn lmstudio_puertos() -> Vec<u16> {
    LM_PORTS.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arch_prueba() -> Arch {
        Arch {
            architecture: "qwen3vl".into(),
            layers: 36,
            heads: 32,
            heads_kv: 8,
            embedding: 2560,
            ctx_native: 262144,
        }
    }

    #[test]
    fn la_cacne_kv_cuadra_con_lo_medido() {
        let a = arch_prueba();
        assert_eq!(a.head_dim(), 80.0);
        let gb = a.kv_bytes(8192, 16.0) as f64 / 1_073_741_824.0;
        println!("KV @8192 fp16 = {gb:.3} GB");
        // Medido: Ollama declaró 3,45 GB totales con ~2,75 GB de pesos.
        assert!((gb - 0.703).abs() < 0.01, "esperado ~0,70 GB, salió {gb}");
    }

    #[test]
    fn q8_cuesta_la_mitad_que_fp16() {
        let a = arch_prueba();
        let fp16 = a.kv_bytes(32768, 16.0);
        let q8 = a.kv_bytes(32768, 8.5);
        assert!(q8 < fp16);
        let pct = q8 as f64 / fp16 as f64;
        assert!((pct - 0.53125).abs() < 0.001, "q8_0 debe ahorrar ~47 %: {pct}");
    }

    #[test]
    fn el_contexto_nativo_encarece_la_cache() {
        let a = arch_prueba();
        let gb_max = a.kv_bytes(a.ctx_native, 16.0) as f64 / 1_073_741_824.0;
        println!("KV al contexto nativo ({}): {gb_max:.1} GB", a.ctx_native);
        assert!(gb_max > 20.0, "262K de contexto debe pasar de 20 GB de KV");
    }

    #[test]
    fn un_modelo_sin_datos_no_divide_por_cero() {
        let a = Arch::default();
        assert_eq!(a.head_dim(), 0.0);
        assert_eq!(a.kv_bytes(8192, 16.0), 0);
    }    #[test]
    fn la_tabla_marca_lo_no_aplicable_sin_esconder_el_ahorro() {
        let t = kv_tabla(&arch_prueba(), 8192, "ollama");
        let q8 = t.iter().find(|o| o.formato == "q8_0").unwrap();
        assert!(q8.ahorro_gb > 0.2, "el ahorro se muestra aunque no se pueda usar");
        // Si el servidor tiene definida OLLAMA_KV_CACHE_TYPE, ese formato es el
        // activo y el resto no se puede aplicar por petición.
        let activo = kv_entorno();
        match &activo {
            Some((_, v)) if v.eq_ignore_ascii_case("q8_0") => {
                assert_eq!(q8.estado, "activo_entorno");
            }
            Some((_, v)) => {
                assert_eq!(q8.estado, "no_aplicable");
                assert!(
                    q8.motivo.contains("OLLAMA_KV_CACHE_TYPE") && q8.motivo.contains(v),
                    "el motivo debe nombrar el formato real del servidor: {}",
                    q8.motivo
                );
            }
            None => assert_eq!(q8.estado, "no_aplicable"),
        }
        // Nunca se puede afirmar que un formato por petición está activo si no lo está.
        let activos = t.iter().filter(|o| o.estado == "activo_entorno").count();
        assert!(activos <= 1, "solo un formato puede estar activo en el servidor");
    }

    #[test]
    fn los_perfiles_de_fabrica_cubren_lo_pedido() {
        let lista = perfiles_de_fabrica();
        let nombres: Vec<&str> = lista.iter().map(|p| p.nombre.as_str()).collect();
        for esperado in [
            "EQUILIBRADO",
            "MÁXIMA VELOCIDAD",
            "MÁXIMA CALIDAD",
            "AHORRO DE VRAM",
            "CONTEXTO EXTENDIDO",
            "GPU + RAM OFFLOAD",
        ] {
            assert!(nombres.contains(&esperado), "falta el perfil {esperado}");
        }
        let offload = lista.iter().find(|p| p.nombre == "GPU + RAM OFFLOAD").unwrap();
        assert_eq!(offload.params.num_gpu, Some(0), "offload total usa num_gpu 0");
    }

    #[test]
    fn todo_parametro_declara_su_estado_y_su_motivo() {
        let p = InferenceParams::default();
        for s in params_ollama(&p, &arch_prueba()) {
            assert!(
                ["caliente", "recarga", "sesion", "no_compatible", "sin_verificar"]
                    .contains(&s.aplica.as_str()),
                "estado desconocido en {}: {}",
                s.key,
                s.aplica
            );
            assert!(!s.motivo.is_empty(), "{} no explica por qué", s.key);
        }
        for s in params_lmstudio(&p) {
            assert!(!s.motivo.is_empty(), "{} no explica por qué", s.key);
        }
    }

    #[test]
    fn lm_studio_marca_sin_verificar_lo_que_no_se_ha_probado() {
        let p = InferenceParams::default();
        let lista = params_lmstudio(&p);
        let top_k = lista.iter().find(|s| s.key == "top_k").unwrap();
        assert_eq!(top_k.aplica, "sin_verificar");
        assert!(top_k.valor.is_null(), "no se envía un valor no confirmado");
    }

    #[test]
    fn el_contexto_de_ollama_no_pasa_del_nativo() {
        let p = InferenceParams::default();
        let c = params_ollama(&p, &arch_prueba())
            .into_iter()
            .find(|s| s.key == "num_ctx")
            .unwrap();
        assert_eq!(c.max, Some(262144.0));
        assert_eq!(c.aplica, "recarga");
    }
}
