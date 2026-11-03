//! Registro de motores del KUKIA UNIVERSAL REALITY CONVERTER (RC1).
//!
//! Cada motor se diagnostica contra su propio puerto: qué versión corre, si
//! responde y **qué capacidades reales** ofrece. Nada se declara disponible por
//! estar instalado: se comprueba nodo a nodo en el motor.

use serde::Serialize;
use std::time::Duration;

/// Motor tal y como lo ve el sistema, con su diagnóstico.
#[derive(Debug, Clone, Serialize)]
pub struct Motor {
    pub id: String,
    pub nombre: String,
    /// `comfy` | `ollama` | `wan2gp` | `maestro`
    pub clase: String,
    pub url: String,
    pub online: bool,
    pub version: String,
    pub detalle: String,
    /// Capacidades confirmadas por el propio motor (no por el catálogo).
    pub capacidades: Vec<String>,
    pub gpu: String,
}

/// VRAM libre por tarjeta, en MiB.
#[derive(Debug, Clone, Serialize)]
pub struct VramLibre {
    pub index: u32,
    pub nombre: String,
    pub total_mb: u64,
    pub usada_mb: u64,
    pub libre_mb: u64,
}

/// Presupuesto de memoria antes de lanzar cualquier trabajo.
#[derive(Debug, Clone, Serialize)]
pub struct Presupuesto {
    pub vram: Vec<VramLibre>,
    pub ram_libre_gb: f64,
    pub ram_total_gb: f64,
}

pub fn comfy_default() -> String {
    std::env::var("KUKIA_COMFY").unwrap_or_else(|_| "http://127.0.0.1:8188".to_string())
}

pub fn ollama_default() -> String {
    std::env::var("KUKIA_OLLAMA").unwrap_or_else(|_| "http://127.0.0.1:11434".to_string())
}

fn wan2gp_default() -> String {
    std::env::var("KUKIA_WAN2GP").unwrap_or_else(|_| "http://127.0.0.1:7861".to_string())
}

fn maestro_default() -> String {
    std::env::var("KUKIA_MAESTRO").unwrap_or_else(|_| "http://127.0.0.1:7860".to_string())
}

fn cliente(segundos: u64) -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(segundos))
        .user_agent("KUKIA-CODE/studio")
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

/// Nodos cuyo nombre real he comprobado en esta instalación (0.43.0). Si
/// mañana desaparecen, la capacidad baja sola: no hay lista inventada.
const CAPACIDADES_COMFY: &[(&str, &str)] = &[
    ("LTXVImgToVideoConditionOnly", "ltx_i2v"),
    ("LTXAddVideoICLoRAGuide", "ltx_ic_lora"),
    ("WanVaceToVideo", "wan_vace"),
    ("WanAnimate2ToVideo", "wan_animate2"),
    ("WanVideoAnimateEmbeds", "wan_animate_wrapper"),
    ("WanVideoSampler", "wan_wrapper"),
    ("FluxKontextImageScale", "flux_kontext"),
    ("IPAdapterApply", "ipadapter"),
    ("StyleModelApply", "style_model"),
    ("VHS_LoadVideo", "video_in"),
    ("VHS_VideoCombine", "video_out"),
    ("SaveAnimatedWEBP", "video_out_webp"),
];

async fn comfy(base: &str) -> Motor {
    let mut m = Motor {
        id: "comfy".into(),
        nombre: "ComfyUI".into(),
        clase: "comfy".into(),
        url: base.to_string(),
        online: false,
        version: String::new(),
        detalle: String::new(),
        capacidades: Vec::new(),
        gpu: "GPU0 + GPU1".into(),
    };

    match cliente(8).get(format!("{base}/system_stats")).send().await {
        Ok(r) if r.status().is_success() => match r.json::<serde_json::Value>().await {
            Ok(v) => {
                m.online = true;
                m.version = v["system"]["comfyui_version"].as_str().unwrap_or("").to_string();
                let dev = v["devices"]
                    .as_array()
                    .and_then(|a| a.first())
                    .cloned()
                    .unwrap_or_default();
                let nombre = dev["name"].as_str().unwrap_or("?");
                let libre = dev["vram_free"].as_u64().unwrap_or(0) / (1024 * 1024);
                let total = dev["vram_total"].as_u64().unwrap_or(0) / (1024 * 1024);
                m.detalle = format!("{nombre} · {libre} MiB libres de {total}");
            }
            Err(e) => m.detalle = format!("respuesta ilegible: {e}"),
        },
        Ok(r) => m.detalle = format!("HTTP {}", r.status()),
        Err(_) => m.detalle = format!("sin respuesta en {base}"),
    }

    if m.online {
        for (clase, capacidad) in CAPACIDADES_COMFY {
            let ok = cliente(15)
                .get(format!("{base}/object_info/{clase}"))
                .send()
                .await
                .map(|r| r.status().is_success())
                .unwrap_or(false);
            if ok {
                m.capacidades.push((*capacidad).to_string());
            }
        }
    }
    m
}

async fn ollama(base: &str) -> Motor {
    let mut m = Motor {
        id: "ollama".into(),
        nombre: "Ollama".into(),
        clase: "ollama".into(),
        url: base.to_string(),
        online: false,
        version: String::new(),
        detalle: String::new(),
        capacidades: Vec::new(),
        gpu: "según OLLAMA_VISIBLE_DEVICES".into(),
    };

    match cliente(8).get(format!("{base}/api/version")).send().await {
        Ok(r) if r.status().is_success() => {
            if let Ok(v) = r.json::<serde_json::Value>().await {
                m.online = true;
                m.version = v["version"].as_str().unwrap_or("").to_string();
            }
        }
        _ => {
            m.detalle = format!("sin respuesta en {base}");
            return m;
        }
    }

    if let Ok(r) = cliente(10).get(format!("{base}/api/tags")).send().await {
        if let Ok(v) = r.json::<serde_json::Value>().await {
            let modelos: Vec<String> = v["models"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x["name"].as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            // Capacidad real de visión: nombre que declare familia visual.
            let vision: Vec<&String> = modelos
                .iter()
                .filter(|n| {
                    let l = n.to_lowercase();
                    l.contains("vl") || l.contains("llava") || l.contains("moondream") || l.contains("vision")
                })
                .collect();
            m.detalle = format!("{} modelo(s) · {} con visión", modelos.len(), vision.len());
            if !vision.is_empty() {
                m.capacidades.push("vision".into());
                m.capacidades.push("deteccion_estilo".into());
                let mut nombres: Vec<String> = vision.iter().take(4).map(|s| (*s).clone()).collect();
                nombres.sort();
                m.capacidades.extend(nombres);
            }
        }
    }
    m
}

/// Motor HTTP que solo se puede sondear (Wan2GP, Maestro): sin API de estado
/// documentada, no se le atribuye ninguna capacidad.
async fn http_simple(id: &str, nombre: &str, clase: &str, url: &str, gpu: &str) -> Motor {
    let mut m = Motor {
        id: id.into(),
        nombre: nombre.into(),
        clase: clase.into(),
        url: url.into(),
        online: false,
        version: String::new(),
        detalle: String::new(),
        capacidades: Vec::new(),
        gpu: gpu.into(),
    };
    match cliente(4).get(url).send().await {
        Ok(r) => {
            m.online = true;
            m.detalle = format!("HTTP {}", r.status());
        }
        Err(_) => m.detalle = format!("sin respuesta en {url}"),
    }
    m
}

/// Todos los motores, sondeados en paralelo.
pub async fn todos() -> Vec<Motor> {
    let comfy_base = comfy_default();
    let ollama_base = ollama_default();
    let wan2gp_base = wan2gp_default();
    let maestro_base = maestro_default();
    let (a, b, c, d) = tokio::join!(
        comfy(&comfy_base),
        ollama(&ollama_base),
        http_simple("wan2gp", "Wan2GP", "wan2gp", &wan2gp_base, "GPU1"),
        http_simple("maestro", "Maestro", "maestro", &maestro_base, "GPU1"),
    );
    vec![a, b, c, d]
}

/// VRAM y RAM libres ahora mismo, sin tocar nada.
pub fn presupuesto() -> Presupuesto {
    let stats = crate::gpu::stats();
    let vram: Vec<VramLibre> = stats
        .iter()
        .map(|g| VramLibre {
            index: g.index,
            nombre: g.name.clone(),
            total_mb: g.memory_total_mb,
            usada_mb: g.memory_used_mb,
            libre_mb: g.memory_total_mb.saturating_sub(g.memory_used_mb),
        })
        .collect();

    use sysinfo::System;
    let mut sys = System::new();
    sys.refresh_memory();
    Presupuesto {
        vram,
        ram_libre_gb: sys.available_memory() as f64 / 1e9,
        ram_total_gb: sys.total_memory() as f64 / 1e9,
    }
}

/// La mayor VRAM libre de una sola tarjeta (un modelo no se reparte solo).
pub fn vram_max_libre_gb(p: &Presupuesto) -> f64 {
    p.vram
        .iter()
        .map(|v| v.libre_mb as f64 / 1024.0)
        .fold(0.0, f64::max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_presupuesto_no_inventa_tarjetas() {
        let p = presupuesto();
        // Puede haber cero GPU en una máquina sin nvidia-smi, pero nunca
        // tarjetas con más memoria usada que total.
        for v in &p.vram {
            assert!(v.usada_mb <= v.total_mb, "GPU{}: usada > total", v.index);
            assert_eq!(v.libre_mb, v.total_mb - v.usada_mb);
        }
        assert!(p.ram_total_gb > 0.0);
    }

    #[test]
    fn vram_max_libre_elige_la_mejor_tarjeta() {
        let p = Presupuesto {
            vram: vec![
                VramLibre { index: 0, nombre: "a".into(), total_mb: 12227, usada_mb: 9000, libre_mb: 3227 },
                VramLibre { index: 1, nombre: "b".into(), total_mb: 16311, usada_mb: 200, libre_mb: 16111 },
            ],
            ram_libre_gb: 40.0,
            ram_total_gb: 64.0,
        };
        assert!((vram_max_libre_gb(&p) - 15.73).abs() < 0.1);
    }
}
