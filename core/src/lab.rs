//! LABORATORIO EXPERIMENTAL — catálogo, banco de pruebas y A/B.
//!
//! Regla de la casa: **nada se presenta como compatible sin comprobarlo**. Cada
//! experimento declara su nivel, su mecanismo real de activación y lo que se ha
//! verificado en esta máquina:
//!
//! - **Variables del servidor Ollama (medidas el 01/11/2026):**
//!   `OLLAMA_KV_CACHE_TYPE=q4_0` y `OLLAMA_CONTEXT_LENGTH=32768`. Por eso la KV
//!   cuantizada **sí** está activa aquí, aunque no se pueda cambiar por petición.
//! - **Por petición:** `num_ctx`, `num_gpu`, `flash_attn`, `num_batch`, `num_thread`,
//!   `use_mmap`, `use_mlock` hacen que Ollama **recargue** el modelo (medido: 30,6 s
//!   de `load_duration` al cambiar `flash_attn` y `num_batch`).
//! - **LM Studio:** `lms load --gpu`, `--parallel`, `--speculative-draft-mtp`.
//! - **No existe en ninguna de las dos APIs:** tensor/pipeline parallelism real,
//!   FP8/NVFP4/EXL2/AWQ/GPTQ, Medusa, EAGLE, paged attention configurable,
//!   K y V con precisiones distintas, chunked prefill configurable, NUMA.
//!
//! El banco de pruebas mide de verdad: Ollama devuelve `eval_count`,
//! `eval_duration`, `prompt_eval_duration` y `load_duration`, y mientras corre se
//! muestrean VRAM por GPU, RAM, temperatura y vatios.

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::control::Arch;
use crate::engines::InferenceParams;

const OLLAMA_BASE: &str = "http://127.0.0.1:11434";

fn client(secs: u64) -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(secs))
        .user_agent("KUKIA-CODE/0.1")
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

/* ============================== Catálogo ============================== */

/// `estable` | `experimental` | `muy_experimental` | `incompatible`
pub type Nivel = &'static str;

#[derive(Debug, Clone, Serialize)]
pub struct Experimento {
    pub id: String,
    pub nombre: String,
    pub categoria: String,
    pub nivel: Nivel,
    pub descripcion: String,
    pub requisitos: Vec<String>,
    pub riesgos: Vec<String>,
    /// `peticion` | `carga_cli` | `variable_servidor` | `ninguno`
    pub mecanismo: String,
    /// Motores donde se ha comprobado que existe el mecanismo.
    pub motores: Vec<String>,
    /// Se puede activar desde KUKIA sin reiniciar nada.
    pub activable: bool,
    /// Qué hace exactamente al activarse.
    pub efecto: String,
    /// Estado real hoy en esta máquina.
    pub estado_local: String,
    /// Nombre de la variable de entorno si el mecanismo es del servidor.
    pub variable: Option<String>,
    /// Valor actual de esa variable, si está definida.
    pub valor_actual: Option<String>,
}

/// Valor de una variable de entorno del sistema.
///
/// `std::env::var` solo ve lo heredado por este proceso; en Windows las variables
/// de usuario/máquina viven en el registro y pueden no estar en la sesión del
/// shell. Se consultan las dos: primero el proceso y después el registro, para no
/// informar de un valor que no es el que tiene el servidor.
fn entorno(nombre: &str) -> Option<String> {
    if let Ok(v) = std::env::var(nombre) {
        if !v.trim().is_empty() {
            return Some(v.trim().to_string());
        }
    }
    #[cfg(windows)]
    {
        for raiz in ["HKCU\\Environment", "HKLM\\SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment"] {
            let mut cmd = std::process::Command::new("reg");
            cmd.args(["query", raiz, "/v", nombre]);
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x0800_0000);
            }
            if let Ok(salida) = cmd.output() {
                if salida.status.success() {
                    let texto = String::from_utf8_lossy(&salida.stdout).to_string();
                    // Formato: "    NOMBRE    REG_SZ    valor"
                    if let Some(linea) = texto.lines().find(|l| l.contains(nombre)) {
                        let valor = linea.split_whitespace().last().unwrap_or("").trim();
                        if !valor.is_empty() && !valor.contains("REG_") {
                            return Some(valor.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

/// Helper de pruebas: fuerza una lectura desde el registro (o el proceso).
pub fn valor_entorno(nombre: &str) -> Option<String> {
    entorno(nombre)
}

fn exp(
    id: &str,
    nombre: &str,
    categoria: &str,
    nivel: Nivel,
    descripcion: &str,
    mecanismo: &str,
    motores: &[&str],
    efecto: &str,
    estado_local: String,
) -> Experimento {
    Experimento {
        id: id.into(),
        nombre: nombre.into(),
        categoria: categoria.into(),
        nivel,
        descripcion: descripcion.into(),
        requisitos: Vec::new(),
        riesgos: Vec::new(),
        mecanismo: mecanismo.into(),
        motores: motores.iter().map(|s| s.to_string()).collect(),
        activable: mecanismo == "peticion" || mecanismo == "carga_cli",
        efecto: efecto.into(),
        estado_local,
        variable: None,
        valor_actual: None,
    }
}

/// Catálogo completo, con el estado real de cada técnica en ESTA máquina.
pub fn catalogo() -> Vec<Experimento> {
    let kv_env = entorno("OLLAMA_KV_CACHE_TYPE");
    let ctx_env = entorno("OLLAMA_CONTEXT_LENGTH");
    let flash_env = entorno("OLLAMA_FLASH_ATTENTION");
    let par_env = entorno("OLLAMA_NUM_PARALLEL");

    let mut v = Vec::new();

    /* ------------------------- 1. KV cache ------------------------- */
    let mut kv = |id: &str, nombre: &str, formato: &str, desc: &str| {
        let efecto = format!("Ollama usa {formato} para toda la caché KV del servidor");
        let mut e = exp(
            id,
            nombre,
            "KV cache",
            "experimental",
            desc,
            "variable_servidor",
            &["ollama"],
            &efecto,
            match &kv_env {
                Some(a) if a.eq_ignore_ascii_case(formato) => {
                    format!("ACTIVO ahora mismo (OLLAMA_KV_CACHE_TYPE={a})")
                }
                Some(a) => format!("inactivo: el servidor está en {a}"),
                None => String::from("inactivo: sin OLLAMA_KV_CACHE_TYPE el servidor usa fp16"),
            },
        );
        e.variable = Some("OLLAMA_KV_CACHE_TYPE".into());
        e.valor_actual = kv_env.clone();
        e.requisitos = vec![
            "definir la variable de entorno del servidor".into(),
            "reiniciar Ollama para que la tome".into(),
        ];
        e.riesgos = vec![
            "puede degradar la calidad en contextos muy largos".into(),
            "medido: la opción por petición se ignora; solo cuenta el entorno".into(),
        ];
        v.push(e);
    };
    kv(
        "kv_q4",
        "KV Cache Q4_0",
        "q4_0",
        "Caché KV a 4,5 bits por valor: la que más VRAM ahorra y la que está activa aquí.",
    );
    kv(
        "kv_q8",
        "KV Cache Q8_0",
        "q8_0",
        "Caché KV a 8,5 bits por valor: casi la mitad que fp16 con pérdida mínima.",
    );

    v.push(exp(
        "kv_fp8",
        "KV Cache FP8",
        "KV cache",
        "incompatible",
        "Formato FP8 de la caché de atención (E4M3/E5M2).",
        "ninguno",
        &[],
        "no existe en la API de Ollama 0.40.13 ni en la de LM Studio",
        "no disponible en los motores instalados".into(),
    ));
    v.push(exp(
        "kv_kv_mixta",
        "K y V con precisiones distintas",
        "KV cache",
        "incompatible",
        "Guardar claves en una precisión y valores en otra.",
        "ninguno",
        &[],
        "ningún motor local instalado lo expone",
        "no disponible".into(),
    ));
    v.push(exp(
        "kv_offload_ram",
        "KV Cache en RAM",
        "KV cache",
        "muy_experimental",
        "Mantener la caché KV en RAM en vez de VRAM; libera GPU a cambio de latencia.",
        "ninguno",
        &["lmstudio"],
        "LM Studio lo ofrece en su interfaz, pero no en `lms load` ni en su API REST",
        "solo con la interfaz de LM Studio; KUKIA no lo puede activar".into(),
    ));
    v.push(exp(
        "prefix_cache",
        "Prefix caching (prefijos compartidos)",
        "KV cache",
        "estable",
        "Reutilizar la parte común del prompt ya procesada.",
        "ninguno",
        &["ollama", "lmstudio"],
        "Ollama cachea prefijos automáticamente; no hay interruptor ni ajuste",
        "activo por diseño del motor, sin control".into(),
    ));
    v.push(exp(
        "paged_attention",
        "Paged Attention",
        "KV cache",
        "muy_experimental",
        "Gestión de la caché por páginas, como vLLM.",
        "ninguno",
        &[],
        "llama.cpp gestiona su memoria de forma interna; no hay ajuste expuesto",
        "no configurable en los motores instalados".into(),
    ));

    /* ------------------------- 2. Atención ------------------------- */
    let mut flash = exp(
        "flash_attn",
        "Flash Attention",
        "Atención",
        "experimental",
        "Atención con memoria lineal en vez de cuadrática en la ventana de contexto.",
        "peticion",
        &["ollama", "lmstudio"],
        "Ollama: `options.flash_attn` por petición (recarga el modelo). También existe \
         OLLAMA_FLASH_ATTENTION a nivel de servidor",
        match &flash_env {
            Some(x) => format!("ACTIVO en el servidor (OLLAMA_FLASH_ATTENTION={x})"),
            None => "inactivo a nivel de servidor; activable por petición (recarga)".into(),
        },
    );
    flash.variable = Some("OLLAMA_FLASH_ATTENTION".into());
    flash.valor_actual = flash_env.clone();
    flash.requisitos = vec!["reinicia el modelo al aplicarse".into()];
    flash.riesgos = vec!["en algunas GPUs/quantizaciones puede no estar soportado y caer a la atención normal".into()];
    v.push(flash);

    for (id, nombre, desc) in [
        ("flash_attn_v3", "Flash Attention 3", "Variante Hopper/Blackwell con solapamiento de kernels."),
        ("flashinfer", "FlashInfer", "Backend de kernels de atención (ideal para serving)."),
        ("chunked_prefill", "Chunked Prefill", "Partir el prefill en trozos para no bloquear el decode."),
        ("split_k", "Split-K", "Repartir la reducción de atención entre bloques."),
        ("fused_attn", "Fused Attention", "Fusionar operaciones de atención en un solo kernel."),
        ("hybrid_attn", "Atención híbrida", "Mezclar atención completa y lineal por capas."),
    ] {
        v.push(exp(
            id,
            nombre,
            "Atención",
            "incompatible",
            desc,
            "ninguno",
            &[],
            "no existe ajuste en la API de Ollama ni en la de LM Studio; lo decide el binario del motor",
            "no disponible: no se puede ni activar ni medir desde aquí".into(),
        ));
    }

    /* ------------------------- 3. Multi-GPU ------------------------- */
    v.push(exp(
        "layer_split",
        "Reparto de capas entre GPU",
        "Multi-GPU",
        "estable",
        "Colocar unas capas en GPU y otras en CPU o en la otra tarjeta.",
        "peticion",
        &["ollama"],
        "Ollama: `options.num_gpu` (número de capas). Medido: 0 manda todo a CPU y la VRAM del modelo baja a 0",
        "disponible: es lo que ya usa la Guardia de VRAM".into(),
    ));
    v.push(exp(
        "tensor_parallel",
        "Tensor Parallelism",
        "Multi-GPU",
        "incompatible",
        "Partir cada capa entre las dos GPU (lo que hace vLLM).",
        "ninguno",
        &[],
        "ni Ollama ni LM Studio hacen TP: reparten capas, que no es lo mismo (con PCIe 3.0 x8 el TP rinde peor)",
        "no disponible".into(),
    ));
    v.push(exp(
        "pipeline_parallel",
        "Pipeline Parallelism",
        "Multi-GPU",
        "muy_experimental",
        "Encadenar etapas del modelo entre tarjetas.",
        "ninguno",
        &[],
        "requiere planificador propio (vLLM/DeepSpeed); no está en estos motores",
        "no disponible".into(),
    ));
    v.push(exp(
        "modelo_por_gpu",
        "Un modelo distinto en cada GPU",
        "Multi-GPU",
        "experimental",
        "Dos servidores, cada uno viendo una sola tarjeta con CUDA_VISIBLE_DEVICES.",
        "ninguno",
        &["ollama", "lmstudio"],
        "posible, pero exige dos procesos con CUDA_VISIBLE_DEVICES distintos: no se puede hacer sin arrancar servidores",
        "no configurable desde KUKIA (no reinicia procesos)".into(),
    ));
    v.push(exp(
        "reparto_uniforme_vs_custom",
        "Uniforme vs reparto personalizado",
        "Multi-GPU",
        "estable",
        "Comparar reparto automático del motor contra `num_gpu` fijado a mano.",
        "peticion",
        &["ollama"],
        "se mide con el banco de pruebas A/B cambiando solo num_gpu",
        "medible hoy: usa A/B con y sin num_gpu".into(),
    ));

    /* ------------------------- 4. Cuantización ------------------------- */
    v.push(exp(
        "gguf_q4_q8",
        "GGUF Q4_K_M … Q8_0",
        "Cuantización",
        "estable",
        "Los niveles de cuantización que ya tienen tus modelos instalados.",
        "ninguno",
        &["ollama", "lmstudio"],
        "se elige al descargar el modelo; no se convierte nada desde aquí",
        "depende del archivo que cargues".into(),
    ));
    for (id, nombre) in [
        ("exl2", "EXL2"),
        ("awq", "AWQ"),
        ("gptq", "GPTQ"),
        ("fp8_model", "Modelos FP8"),
        ("fp4", "FP4 / NVFP4"),
        ("int4", "INT4"),
        ("mxfp4", "MXFP4"),
    ] {
        v.push(exp(
            id,
            nombre,
            "Cuantización",
            "incompatible",
            &format!("{nombre} como formato de pesos."),
            "ninguno",
            &[],
            "los motores instalados (Ollama/llama.cpp y LM Studio) no cargan este formato; haría falta vLLM o TensorRT-LLM, que no están instalados",
            "no disponible y no se instala nada sin autorización".into(),
        ));
    }

    /* ------------------------- 5. Aceleración ------------------------- */
    let mut spec = exp(
        "speculative",
        "Speculative decoding",
        "Aceleración",
        "experimental",
        "Un modelo pequeño propone tokens y el grande los valida.",
        "carga_cli",
        &["lmstudio"],
        "LM Studio: `lms load --speculative-draft-mtp` o `--speculative-draft-simple --speculative-draft-model`",
        "existe en el CLI; solo funciona si el modelo trae cabeza MTP o hay un draft compatible".into(),
    );
    spec.requisitos = vec![
        "modelo con cabeza MTP (o un draft model del mismo vocabulario)".into(),
        "recarga el modelo".into(),
    ];
    spec.riesgos = vec!["si el draft acierta poco, puede ir MÁS LENTO".into()];
    v.push(spec);

    v.push(exp(
        "parallel_batching",
        "Batching continuo / peticiones en paralelo",
        "Aceleración",
        "experimental",
        "Atender varias generaciones a la vez repartiendo la GPU.",
        "carga_cli",
        &["lmstudio", "ollama"],
        "LM Studio: `lms load --parallel N`. Ollama: OLLAMA_NUM_PARALLEL en el servidor",
        match &par_env {
            Some(x) => format!("ACTIVO en el servidor (OLLAMA_NUM_PARALLEL={x})"),
            None => "sin definir en Ollama; configurable al cargar en LM Studio".into(),
        },
    ));
    for (id, nombre, desc) in [
        ("medusa", "Medusa", "Cabezas de decodificación especulativa entrenadas."),
        ("eagle", "EAGLE", "Decodificación especulativa con modelo draft entrenado."),
        ("ngram", "N-gram speculation", "Sugerir tokens por coincidencia de n-gramas."),
    ] {
        v.push(exp(
            id,
            nombre,
            "Aceleración",
            "incompatible",
            desc,
            "ninguno",
            &[],
            "no soportado por Ollama ni por LM Studio",
            "no disponible".into(),
        ));
    }

    /* ------------------------- 6. Memoria ------------------------- */
    let mut mmap = exp(
        "mmap",
        "Memory mapping (mmap)",
        "Memoria",
        "estable",
        "Mapear el archivo del modelo en vez de copiarlo entero a RAM.",
        "peticion",
        &["ollama"],
        "Ollama: `options.use_mmap` (recarga el modelo)",
        "disponible por petición".into(),
    );
    mmap.riesgos = vec!["con mmap desactivado la carga es más lenta y pide toda la RAM de golpe".into()];
    v.push(mmap);

    let mut mlock = exp(
        "mlock",
        "mlock (fijar en RAM)",
        "Memoria",
        "experimental",
        "Impedir que Windows pagine el modelo a disco.",
        "peticion",
        &["ollama"],
        "Ollama: `options.use_mlock` (recarga el modelo)",
        "disponible por petición".into(),
    );
    mlock.riesgos = vec!["puede consumir mucha RAM de golpe si el modelo es grande".into()];
    v.push(mlock);

    for (id, nombre, desc) in [
        ("numa", "NUMA y afinidad de CPU", "Fijar el modelo a un nodo de memoria."),
        ("pinned", "Pinned memory", "Memoria no paginable para transferencias más rápidas."),
        ("vram_pool", "GPU memory pool", "Reservar un bloque de VRAM por adelantado."),
        ("fragmentacion", "Anti-fragmentación de VRAM", "Reutilizar buffers para no fragmentar."),
    ] {
        v.push(exp(
            id,
            nombre,
            "Memoria",
            "incompatible",
            desc,
            "ninguno",
            &[],
            "no expuesto por las APIs de Ollama ni de LM Studio",
            "no disponible".into(),
        ));
    }

    /* ------------------------- 7. Modelos ------------------------- */
    for (id, nombre, desc) in [
        ("moe", "Mixture of Experts (MoE)", "Solo se activan algunos expertos por token."),
        ("expert_parallel", "Expert Parallelism", "Repartir expertos entre GPU."),
        ("long_context", "Contexto largo", "Modelos entrenados para 128K o más."),
        ("multimodal", "Multimodal", "Modelos que ven imágenes."),
        ("recurrentes", "Arquitecturas recurrentes / atención lineal", "Modelos híbridos tipo Mamba."),
    ] {
        let nivel: Nivel = match id {
            "moe" | "long_context" | "multimodal" => "estable",
            _ => "muy_experimental",
        };
        v.push(exp(
            id,
            nombre,
            "Modelos",
            nivel,
            desc,
            "ninguno",
            &["ollama", "lmstudio"],
            "es una propiedad del modelo, no un ajuste: se informa en la ficha del modelo",
            "depende del modelo cargado".into(),
        ));
    }

    let ctx_estado = match &ctx_env {
        Some(c) => format!("contexto por defecto del servidor: OLLAMA_CONTEXT_LENGTH={c}"),
        None => String::from("contexto por defecto: 4096 en Ollama si no se define"),
    };
    let mut ctxe = exp(
        "ctx_default",
        "Contexto por defecto del servidor",
        "Contexto",
        "estable",
        "Cuántos tokens reserva Ollama cuando la petición no dice otra cosa.",
        "variable_servidor",
        &["ollama"],
        "OLLAMA_CONTEXT_LENGTH; cambiarlo exige reiniciar el servidor",
        ctx_estado,
    );
    ctxe.variable = Some("OLLAMA_CONTEXT_LENGTH".into());
    ctxe.valor_actual = ctx_env;
    v.push(ctxe);

    v
}

/* ============================ Banco de pruebas ============================ */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PicoGpu {
    pub index: u32,
    pub name: String,
    pub pico_mb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchResult {
    pub id: String,
    pub etiqueta: String,
    pub engine: String,
    pub model: String,
    pub params_texto: String,
    pub params: InferenceParams,
    pub tokens: u64,
    pub tok_s: f64,
    /// `None` cuando el motor no publica tiempos (LM Studio).
    pub prefill_tok_s: Option<f64>,
    pub ttft_ms: Option<f64>,
    pub itl_ms: Option<f64>,
    pub carga_ms: Option<f64>,
    pub total_ms: f64,
    pub vram_pico: Vec<PicoGpu>,
    pub ram_pico_gb: f64,
    pub temp_pico_c: f64,
    pub potencia_pico_w: f64,
    pub ok: bool,
    pub error: String,
    pub tomado_en: String,
}

/// Resumen legible de los parámetros que se han usado en una prueba.
pub fn resumen_params(p: &InferenceParams) -> String {
    let mut partes = Vec::new();
    if let Some(v) = p.num_ctx {
        partes.push(format!("ctx {v}"));
    }
    if let Some(v) = p.num_gpu {
        partes.push(format!(
            "num_gpu {}",
            if v < 0 { "auto".to_string() } else { v.to_string() }
        ));
    }
    if let Some(v) = p.flash_attn {
        partes.push(format!("flash_attn {v}"));
    }
    if let Some(v) = p.num_batch {
        partes.push(format!("num_batch {v}"));
    }
    if let Some(v) = p.num_thread {
        partes.push(format!("num_thread {v}"));
    }
    if let Some(v) = p.use_mmap {
        partes.push(format!("mmap {v}"));
    }
    if let Some(v) = p.use_mlock {
        partes.push(format!("mlock {v}"));
    }
    if let Some(v) = p.parallel {
        partes.push(format!("parallel {v}"));
    }
    if let Some(v) = &p.gpu_offload {
        partes.push(format!("gpu {v}"));
    }
    if p.speculative == Some(true) {
        partes.push("speculative".into());
    }
    if partes.is_empty() {
        "configuración del motor (sin overrides)".into()
    } else {
        partes.join(" · ")
    }
}

struct Muestras {
    vram: Vec<PicoGpu>,
    ram_pico_gb: f64,
    temp: f64,
    potencia: f64,
}

fn muestrear() -> Muestras {
    let gpus: Vec<PicoGpu> = crate::gpu::stats()
        .into_iter()
        .map(|g| PicoGpu {
            index: g.index,
            name: g.name,
            pico_mb: g.memory_used_mb,
        })
        .collect();
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    Muestras {
        vram: gpus,
        ram_pico_gb: sys.used_memory() as f64 / 1_073_741_824.0,
        temp: crate::gpu::stats().iter().map(|g| g.temperature_c).fold(0.0, f64::max),
        potencia: crate::gpu::stats().iter().map(|g| g.power_w).sum(),
    }
}

/// Ejecuta una generación controlada y devuelve métricas medidas.
///
/// Si `engine` no es Ollama, usa la API compatible OpenAI de LM Studio y mide el
/// tiempo a reloj; en ese caso `prefill_tok_s`, `ttft_ms` e `itl_ms` van a `None`
/// porque su API no publica esos tiempos y no se inventan.
pub async fn benchmark(
    engine: &str,
    model: &str,
    p: &InferenceParams,
    prompt: &str,
    tokens: u64,
) -> BenchResult {
    let pico = Arc::new(Mutex::new(Muestras {
        vram: Vec::new(),
        ram_pico_gb: 0.0,
        temp: 0.0,
        potencia: 0.0,
    }));
    let muestreando = Arc::new(Mutex::new(true));
    let (pico_c, flag) = (pico.clone(), muestreando.clone());
    let hilo = tokio::task::spawn_blocking(move || {
        let mut acumulado: Option<Muestras> = None;
        while *flag.lock().unwrap() {
            let m = muestrear();
            // Cada vuelta reemplaza la anterior: se guarda el pico por GPU.
            acumulado = Some(match acumulado {
                None => m,
                Some(prev) => Muestras {
                    vram: prev
                        .vram
                        .iter()
                        .map(|g| PicoGpu {
                            index: g.index,
                            name: g.name.clone(),
                            pico_mb: g
                                .pico_mb
                                .max(m.vram.iter().find(|x| x.index == g.index).map(|x| x.pico_mb).unwrap_or(0)),
                        })
                        .collect(),
                    ram_pico_gb: prev.ram_pico_gb.max(m.ram_pico_gb),
                    temp: prev.temp.max(m.temp),
                    potencia: prev.potencia.max(m.potencia),
                },
            });
            std::thread::sleep(Duration::from_millis(250));
        }
        acumulado
    });
    *pico_c.lock().unwrap() = muestrear();

    let inicio = Instant::now();
    let mut resultado = if engine == "lmstudio" {
        bench_lmstudio(model, p, prompt, tokens).await
    } else {
        bench_ollama(model, p, prompt, tokens).await
    };
    resultado.total_ms = inicio.elapsed().as_millis() as f64;

    *muestreando.lock().unwrap() = false;
    if let Ok(Some(m)) = hilo.await {
        resultado.vram_pico = m.vram;
        resultado.ram_pico_gb = m.ram_pico_gb;
        resultado.temp_pico_c = m.temp;
        resultado.potencia_pico_w = m.potencia;
    }
    resultado.etiqueta = resumen_params(p);
    resultado.params = p.clone();
    resultado.tomado_en = chrono::Utc::now().to_rfc3339();
    resultado
}

async fn bench_ollama(model: &str, p: &InferenceParams, prompt: &str, tokens: u64) -> BenchResult {
    let mut body = serde_json::json!({
        "model": model,
        "prompt": prompt,
        "stream": false,
        "keep_alive": "5m",
    });
    let mut opciones = p.opciones_ollama();
    if let Some(o) = opciones.as_object_mut() {
        // Se prueban siempre los mismos valores de muestreo para que A/B sea comparable.
        o.insert("temperature".into(), serde_json::json!(0.0));
        o.insert("seed".into(), serde_json::json!(42));
        o.insert("num_predict".into(), serde_json::json!(tokens));
    }
    body["options"] = opciones;

    let mut r = BenchResult {
        id: String::new(),
        etiqueta: String::new(),
        engine: "ollama".into(),
        model: model.into(),
        params_texto: resumen_params(p),
        params: p.clone(),
        tokens: 0,
        tok_s: 0.0,
        prefill_tok_s: None,
        ttft_ms: None,
        itl_ms: None,
        carga_ms: None,
        total_ms: 0.0,
        vram_pico: Vec::new(),
        ram_pico_gb: 0.0,
        temp_pico_c: 0.0,
        potencia_pico_w: 0.0,
        ok: false,
        error: String::new(),
        tomado_en: String::new(),
    };

    match client(600)
        .post(format!("{OLLAMA_BASE}/api/generate"))
        .json(&body)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => match resp.json::<serde_json::Value>().await {
            Ok(v) => {
                let eval_count = v["eval_count"].as_u64().unwrap_or(0);
                let eval_ns = v["eval_duration"].as_u64().unwrap_or(0);
                let prompt_count = v["prompt_eval_count"].as_u64().unwrap_or(0);
                let prompt_ns = v["prompt_eval_duration"].as_u64().unwrap_or(0);
                let load_ns = v["load_duration"].as_u64().unwrap_or(0);
                r.tokens = eval_count;
                if eval_count > 0 && eval_ns > 0 {
                    r.tok_s = eval_count as f64 / (eval_ns as f64 / 1e9);
                    r.itl_ms = Some(eval_ns as f64 / eval_count as f64 / 1e6);
                }
                if prompt_count > 0 && prompt_ns > 0 {
                    r.prefill_tok_s = Some(prompt_count as f64 / (prompt_ns as f64 / 1e9));
                    r.ttft_ms = Some((load_ns + prompt_ns) as f64 / 1e6);
                }
                if load_ns > 0 {
                    r.carga_ms = Some(load_ns as f64 / 1e6);
                }
                r.ok = eval_count > 0;
                if !r.ok {
                    r.error = "el motor respondió sin tokens generados".into();
                }
            }
            Err(e) => r.error = format!("respuesta ilegible: {e}"),
        },
        Ok(resp) => {
            let st = resp.status();
            let t = resp.text().await.unwrap_or_default();
            r.error = format!("HTTP {st}: {}", t.trim().chars().take(180).collect::<String>());
        }
        Err(e) => r.error = format!("sin respuesta: {e}"),
    }
    r
}

async fn bench_lmstudio(model: &str, p: &InferenceParams, prompt: &str, tokens: u64) -> BenchResult {
    let mut r = BenchResult {
        id: String::new(),
        etiqueta: String::new(),
        engine: "lmstudio".into(),
        model: model.into(),
        params_texto: resumen_params(p),
        params: p.clone(),
        tokens: 0,
        tok_s: 0.0,
        prefill_tok_s: None,
        ttft_ms: None,
        itl_ms: None,
        carga_ms: None,
        total_ms: 0.0,
        vram_pico: Vec::new(),
        ram_pico_gb: 0.0,
        temp_pico_c: 0.0,
        potencia_pico_w: 0.0,
        ok: false,
        error: String::new(),
        tomado_en: String::new(),
    };
    let mut base = None;
    for port in [1234u16, 1235] {
        let b = format!("http://127.0.0.1:{port}");
        if client(2)
            .get(format!("{b}/v1/models"))
            .send()
            .await
            .map(|x| x.status().is_success())
            .unwrap_or(false)
        {
            base = Some(b);
            break;
        }
    }
    let Some(base) = base else {
        r.error = "LM Studio no responde en 1234 ni 1235".into();
        return r;
    };
    let body = serde_json::json!({
        "model": model,
        "messages": [{ "role": "user", "content": prompt }],
        "max_tokens": tokens,
        "temperature": p.temperature.unwrap_or(0.0),
        "stream": false
    });
    let inicio = Instant::now();
    match client(600)
        .post(format!("{base}/v1/chat/completions"))
        .json(&body)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => match resp.json::<serde_json::Value>().await {
            Ok(v) => {
                let t = v["usage"]["completion_tokens"].as_u64().unwrap_or(0);
                r.tokens = t;
                let seg = inicio.elapsed().as_secs_f64();
                if t > 0 && seg > 0.0 {
                    r.tok_s = t as f64 / seg;
                    r.ok = true;
                } else {
                    r.error = "sin tokens en la respuesta".into();
                }
            }
            Err(e) => r.error = format!("respuesta ilegible: {e}"),
        },
        Ok(resp) => {
            let st = resp.status();
            let t = resp.text().await.unwrap_or_default();
            r.error = format!("HTTP {st}: {}", t.trim().chars().take(180).collect::<String>());
        }
        Err(e) => r.error = format!("sin respuesta: {e}"),
    }
    r
}

/* ============================== A/B ============================== */

#[derive(Debug, Clone, Serialize)]
pub struct Comparacion {
    pub engine: String,
    pub model: String,
    pub a: BenchResult,
    pub b: BenchResult,
    /// Diferencias B − A (positivo = B mejor en velocidad).
    pub delta_tok_s: f64,
    pub delta_tok_s_pct: f64,
    pub delta_vram_mb: Vec<(u32, i64)>,
    pub delta_ram_gb: f64,
    pub veredicto: String,
}

/// Compara dos configuraciones del mismo modelo con la misma petición.
pub async fn comparar(
    engine: &str,
    model: &str,
    a: &InferenceParams,
    b: &InferenceParams,
    prompt: &str,
    tokens: u64,
) -> Comparacion {
    let ra = benchmark(engine, model, a, prompt, tokens).await;
    let rb = benchmark(engine, model, b, prompt, tokens).await;

    let delta_tok_s = rb.tok_s - ra.tok_s;
    let delta_tok_s_pct = if ra.tok_s > 0.0 {
        delta_tok_s / ra.tok_s * 100.0
    } else {
        0.0
    };
    let delta_vram_mb: Vec<(u32, i64)> = rb
        .vram_pico
        .iter()
        .map(|g| {
            let antes = ra
                .vram_pico
                .iter()
                .find(|x| x.index == g.index)
                .map(|x| x.pico_mb as i64)
                .unwrap_or(0);
            (g.index, g.pico_mb as i64 - antes)
        })
        .collect();
    let delta_ram_gb = rb.ram_pico_gb - ra.ram_pico_gb;

    let veredicto = if !ra.ok || !rb.ok {
        "no se puede comparar: alguna de las dos pruebas falló".to_string()
    } else if delta_tok_s_pct.abs() < 3.0 {
        format!(
            "sin diferencia clara ({:+.1} %): vuelve a medir antes de sacar conclusiones",
            delta_tok_s_pct
        )
    } else if delta_tok_s_pct > 0.0 {
        format!(
            "B va {:+.1} % más rápido ({:.1} → {:.1} tok/s)",
            delta_tok_s_pct, ra.tok_s, rb.tok_s
        )
    } else {
        format!(
            "B va {:+.1} % más lento ({:.1} → {:.1} tok/s)",
            delta_tok_s_pct, ra.tok_s, rb.tok_s
        )
    };

    Comparacion {
        engine: engine.into(),
        model: model.into(),
        a: ra,
        b: rb,
        delta_tok_s,
        delta_tok_s_pct,
        delta_vram_mb,
        delta_ram_gb,
        veredicto,
    }
}

/// Texto fijo de las pruebas: mismo prompt para que A/B compare solo la config.
pub const PROMPT_PRUEBA: &str =
    "Explica en cinco frases qué hace un gestor de memoria de VRAM en un sistema con dos GPU.";

/* ============================ Historial ============================ */

fn historial_path() -> std::path::PathBuf {
    crate::config::data_dir().join("laboratorio.json")
}

pub fn guardar_resultados(nuevos: &[BenchResult]) -> Result<Vec<BenchResult>, String> {
    let mut todo: Vec<BenchResult> = std::fs::read_to_string(historial_path())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    todo.extend_from_slice(nuevos);
    // Se conservan las últimas 100 pruebas para no crecer sin control.
    if todo.len() > 100 {
        let sobra = todo.len() - 100;
        todo.drain(0..sobra);
    }
    if let Some(dir) = historial_path().parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let t = serde_json::to_string_pretty(&todo).map_err(|e| e.to_string())?;
    std::fs::write(historial_path(), t).map_err(|e| e.to_string())?;
    Ok(todo)
}

pub fn historial() -> Vec<BenchResult> {
    std::fs::read_to_string(historial_path())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/* ============================ Activación ============================ */

fn previos_path() -> std::path::PathBuf {
    crate::config::data_dir().join("laboratorio_previo.json")
}

/// Activa un experimento aplicando parámetros reales, guardando antes los que
/// había. Devuelve el mensaje para la interfaz.
pub fn activar(
    experimento: &str,
    engine: &str,
    model: &str,
) -> Result<(InferenceParams, String), String> {
    let catalogo = catalogo();
    let e = catalogo
        .iter()
        .find(|x| x.id == experimento)
        .ok_or_else(|| format!("no existe el experimento {experimento}"))?;
    if !e.activable {
        return Err(format!(
            "«{}» no se puede activar desde KUKIA: {}",
            e.nombre, e.efecto
        ));
    }
    if !e.motores.iter().any(|m| m == engine) {
        return Err(format!("{} no existe para el motor {engine}", e.nombre));
    }

    let anterior = crate::engines::params_get(engine, model);
    let mut nuevo = anterior.clone();
    match experimento {
        "flash_attn" => nuevo.flash_attn = Some(true),
        "layer_split" => nuevo.num_gpu = Some(0),
        "mmap" => nuevo.use_mmap = Some(false),
        "mlock" => nuevo.use_mlock = Some(true),
        "speculative" => nuevo.speculative = Some(true),
        "parallel_batching" => nuevo.parallel = Some(2),
        otro => {
            return Err(format!(
                "el experimento {otro} aún no tiene un mapeo a parámetros; no se toca nada"
            ))
        }
    }

    // Copia de seguridad del estado previo, por modelo.
    let clave = format!("{}::{}", engine, model);
    let mut previos: std::collections::BTreeMap<String, InferenceParams> =
        std::fs::read_to_string(previos_path())
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
    previos.insert(clave, anterior);
    if let Ok(t) = serde_json::to_string_pretty(&previos) {
        if let Some(dir) = previos_path().parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(previos_path(), t);
    }

    let avisos = crate::engines::params_set(engine, model, nuevo.clone());
    let mut msg = format!(
        "«{}» activado en {}/{}. {}",
        e.nombre, engine, model, e.efecto
    );
    if !avisos.is_empty() {
        msg.push_str(&format!(" Avisos: {}", avisos.join(" · ")));
    }
    Ok((nuevo, msg))
}

/// Devuelve la configuración que había antes de activar el experimento.
pub fn restaurar(engine: &str, model: &str) -> Option<InferenceParams> {
    let clave = format!("{}::{}", engine, model);
    let previos: std::collections::BTreeMap<String, InferenceParams> =
        std::fs::read_to_string(previos_path())
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
    let anterior = previos.get(&clave).cloned();
    if let Some(a) = &anterior {
        crate::engines::params_set(engine, model, a.clone());
    }
    anterior
}

/// Estimación de KV con el formato activo del servidor, para el laboratorio.
pub fn kv_activo_gb(arch: &Arch, ctx: u64) -> Option<(String, f64)> {
    let (_, formato) = crate::control::kv_entorno()?;
    let bits = crate::control::KV_FORMATOS
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(&formato))
        .map(|(_, b)| *b)
        .unwrap_or(16.0);
    Some((formato, arch.kv_bytes(ctx, bits) as f64 / 1_073_741_824.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn todo_experimento_declara_nivel_mecanismo_y_estado() {
        let c = catalogo();
        assert!(c.len() > 25, "el catálogo debe cubrir las 7 secciones: {}", c.len());
        for e in &c {
            assert!(
                ["estable", "experimental", "muy_experimental", "incompatible"]
                    .contains(&e.nivel),
                "nivel inválido en {}: {}",
                e.id,
                e.nivel
            );
            assert!(
                ["peticion", "carga_cli", "variable_servidor", "ninguno"].contains(&e.mecanismo.as_str()),
                "mecanismo inválido en {}",
                e.id
            );
            assert!(!e.estado_local.is_empty(), "{} no dice su estado", e.id);
            assert!(!e.efecto.is_empty(), "{} no dice qué hace", e.id);
            if e.mecanismo == "variable_servidor" {
                assert!(e.variable.is_some(), "{} debe nombrar su variable", e.id);
                assert!(!e.activable, "{} exige reiniciar el servidor", e.id);
            }
            if e.nivel == "incompatible" {
                assert!(!e.activable, "{} no puede ser activable siendo incompatible", e.id);
            }
        }
    }

    #[test]
    fn las_categorias_pedidas_estan_todas() {
        let c = catalogo();
        let cats: Vec<&str> = c.iter().map(|e| e.categoria.as_str()).collect();
        for esperada in [
            "KV cache",
            "Atención",
            "Multi-GPU",
            "Cuantización",
            "Aceleración",
            "Memoria",
            "Modelos",
        ] {
            assert!(cats.contains(&esperada), "falta la categoría {esperada}");
        }
    }

    #[test]
    fn lo_que_no_existe_se_marca_incompatible() {
        let c = catalogo();
        for id in ["tensor_parallel", "medusa", "eagle", "kv_fp8", "exl2", "numa"] {
            let e = c.iter().find(|x| x.id == id).unwrap_or_else(|| panic!("falta {id}"));
            assert_eq!(e.nivel, "incompatible", "{id} no existe en estos motores");
        }
    }

    #[test]
    fn el_entorno_real_se_refleja_en_el_catalogo() {
        let c = catalogo();
        let kv4 = c.iter().find(|e| e.id == "kv_q4").unwrap();
        let kv8 = c.iter().find(|e| e.id == "kv_q8").unwrap();
        match entorno("OLLAMA_KV_CACHE_TYPE") {
            Some(v) if v.eq_ignore_ascii_case("q4_0") => {
                assert!(kv4.estado_local.starts_with("ACTIVO"), "{}", kv4.estado_local);
                assert!(kv8.estado_local.starts_with("inactivo"), "{}", kv8.estado_local);
            }
            _ => {
                assert!(!kv4.estado_local.is_empty());
                assert!(!kv8.estado_local.is_empty());
            }
        }
    }

    #[test]
    fn no_se_activa_lo_incompatible() {
        let r = activar("tensor_parallel", "ollama", "cualquiera");
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("no se puede activar"));
    }

    #[test]
    fn un_id_inventado_no_toca_nada() {
        assert!(activar("existe_esto", "ollama", "m").is_err());
    }

    #[test]
    fn el_resumen_de_parametros_es_legible() {
        let p = InferenceParams {
            num_ctx: Some(8192),
            num_gpu: Some(-1),
            flash_attn: Some(true),
            ..Default::default()
        };
        let t = resumen_params(&p);
        assert!(t.contains("ctx 8192"));
        assert!(t.contains("num_gpu auto"));
        assert!(t.contains("flash_attn true"));
        assert_eq!(
            resumen_params(&InferenceParams::default()),
            "configuración del motor (sin overrides)"
        );
    }
}
