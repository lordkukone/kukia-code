//! Guardia de VRAM (Fase 3).
//!
//! Objetivo: evitar la pérdida brutal de rendimiento medida en Fase 2.1 cuando dos
//! motores de inferencia ocupan la misma GPU.
//!
//! Reglas:
//! - Solo lectura: no cierra procesos, no descarga modelos, no toca configuraciones.
//! - Informa y avisa; la decisión es del usuario.
//! - La GPU 0 sigue siendo la principal de pantalla y juegos.

use serde::Serialize;
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct GpuVram {
    pub index: u32,
    pub name: String,
    pub total_mb: u64,
    pub used_mb: u64,
    pub free_mb: u64,
    /// Memoria reservada/no contabilizada, cuando nvidia-smi la distingue.
    pub reserved_mb: u64,
    pub utilization: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct EngineModel {
    pub name: String,
    pub vram_mb: u64,
    pub size_gb: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct EngineState {
    pub engine: String,
    pub online: bool,
    pub note: String,
    pub models: Vec<EngineModel>,
    pub vram_total_mb: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcessOnGpu {
    pub pid: u32,
    pub name: String,
    pub vram_mb: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct VramReport {
    pub gpus: Vec<GpuVram>,
    pub engines: Vec<EngineState>,
    pub processes: Vec<ProcessOnGpu>,
    pub competition: bool,
    pub warning: String,
    pub recommendations: Vec<String>,
    pub checked_at: String,
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn run_smi(args: &[&str]) -> String {
    let mut cmd = Command::new("nvidia-smi");
    cmd.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    match cmd.output() {
        Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
        Err(_) => String::new(),
    }
}

/// Lee VRAM por GPU con memoria total, usada y libre.
pub fn gpus_vram() -> Vec<GpuVram> {
    let out = run_smi(&[
        "--query-gpu=index,name,memory.total,memory.used,memory.free,utilization.gpu",
        "--format=csv,noheader,nounits",
    ]);
    let mut list = Vec::new();
    for line in out.lines() {
        let p: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        if p.len() < 6 {
            continue;
        }
        let total: u64 = p[2].parse().unwrap_or(0);
        let used: u64 = p[3].parse().unwrap_or(0);
        let free: u64 = p[4].parse().unwrap_or(0);
        let reserved = total.saturating_sub(used + free);
        list.push(GpuVram {
            index: p[0].parse().unwrap_or(0),
            name: p[1].to_string(),
            total_mb: total,
            used_mb: used,
            free_mb: free,
            reserved_mb: reserved,
            utilization: p[5].parse().unwrap_or(0),
        });
    }
    list
}

/// Procesos con memoria en GPU (si el driver lo permite).
pub fn gpu_processes() -> Vec<ProcessOnGpu> {
    let out = run_smi(&[
        "--query-compute-apps=pid,process_name,used_memory",
        "--format=csv,noheader,nounits",
    ]);
    let mut list = Vec::new();
    for line in out.lines() {
        let p: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        if p.len() < 3 {
            continue;
        }
        list.push(ProcessOnGpu {
            pid: p[0].parse().unwrap_or(0),
            name: p[1].to_string(),
            vram_mb: p[2].parse().unwrap_or(0),
        });
    }
    list
}

fn client(timeout: u64) -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout))
        .user_agent("KUKIA-CODE/0.1")
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

async fn ollama_state() -> EngineState {
    let url = "http://127.0.0.1:11434/api/ps";
    match client(2).get(url).send().await {
        Ok(r) if r.status().is_success() => match r.json::<serde_json::Value>().await {
            Ok(v) => {
                let mut models = Vec::new();
                let mut total = 0u64;
                if let Some(arr) = v["models"].as_array() {
                    for m in arr {
                        let name = m["name"].as_str().unwrap_or("").to_string();
                        if name.is_empty() {
                            continue;
                        }
                        let vram = m["size_vram"].as_u64().unwrap_or(0);
                        let size = m["size"].as_u64().unwrap_or(0);
                        total += vram;
                        models.push(EngineModel {
                            name,
                            vram_mb: vram / (1024 * 1024),
                            size_gb: size as f64 / (1024.0 * 1024.0 * 1024.0),
                        });
                    }
                }
                EngineState {
                    engine: "ollama".into(),
                    online: true,
                    note: if models.is_empty() {
                        "activo, sin modelos cargados".into()
                    } else {
                        format!("{} modelo(s) cargado(s)", models.len())
                    },
                    models,
                    vram_total_mb: total / (1024 * 1024),
                }
            }
            Err(e) => EngineState {
                engine: "ollama".into(),
                online: true,
                note: format!("respuesta ilegible: {e}"),
                models: Vec::new(),
                vram_total_mb: 0,
            },
        },
        _ => EngineState {
            engine: "ollama".into(),
            online: false,
            note: "sin respuesta en 11434".into(),
            models: Vec::new(),
            vram_total_mb: 0,
        },
    }
}

async fn lmstudio_state() -> EngineState {
    // LM Studio expone el estado de carga en /api/v0/models.
    for port in [1234u16, 1235] {
        let base = format!("http://127.0.0.1:{port}");
        if let Ok(r) = client(2).get(format!("{base}/api/v0/models")).send().await {
            if r.status().is_success() {
                if let Ok(v) = r.json::<serde_json::Value>().await {
                    let mut models = Vec::new();
                    if let Some(arr) = v["data"].as_array() {
                        for m in arr {
                            let state = m["state"].as_str().unwrap_or("");
                            if state != "loaded" {
                                continue;
                            }
                            let name = m["id"].as_str().unwrap_or("").to_string();
                            if name.is_empty() {
                                continue;
                            }
                            let size = m["size"].as_f64().unwrap_or(0.0);
                            models.push(EngineModel {
                                name,
                                vram_mb: 0,
                                size_gb: size / 1_073_741_824.0,
                            });
                        }
                    }
                    return EngineState {
                        engine: "lmstudio".into(),
                        online: true,
                        note: format!("puerto {port}: {} cargado(s)", models.len()),
                        models,
                        vram_total_mb: 0,
                    };
                }
            }
        }
    }
    EngineState {
        engine: "lmstudio".into(),
        online: false,
        note: "sin respuesta en 1234 ni 1235".into(),
        models: Vec::new(),
        vram_total_mb: 0,
    }
}

/// ComfyUI no publica qué checkpoint tiene cargado, pero sí su VRAM real.
/// Se informa la memoria medida y se deja claro que el nombre no se puede saber.
async fn comfyui_state() -> EngineState {
    let url = "http://127.0.0.1:8188/system_stats";
    match client(3).get(url).send().await {
        Ok(r) if r.status().is_success() => match r.json::<serde_json::Value>().await {
            Ok(v) => {
                let d = v["devices"].as_array().and_then(|a| a.first());
                let total = d
                    .and_then(|x| x["vram_total"].as_u64())
                    .unwrap_or(0)
                    / (1024 * 1024);
                let libre = d
                    .and_then(|x| x["vram_free"].as_u64())
                    .unwrap_or(0)
                    / (1024 * 1024);
                let gpu = d
                    .and_then(|x| x["name"].as_str())
                    .unwrap_or("GPU")
                    .to_string();
                let usado_mb = total.saturating_sub(libre);

                // ComfyUI no expone qué checkpoint tiene cargado, así que se deduce de la VRAM.
                // Umbral 800 MB: por debajo solo queda el contexto de torch; por encima hay modelo
                // retenido. Medido en esta máquina: tras generar marca 1.177 MB aunque nvidia-smi
                // cuente 5.642 MiB en la misma GPU, porque torch reserva más de lo que declara.
                let mut models = Vec::new();
                if usado_mb >= 800 {
                    models.push(EngineModel {
                        name: "modelo residente (ComfyUI no expone su nombre)".into(),
                        vram_mb: usado_mb,
                        size_gb: usado_mb as f64 / 1024.0,
                    });
                }
                EngineState {
                    engine: "comfyui".into(),
                    online: true,
                    note: format!(
                        "{gpu} · {usado_mb} MB ocupados de {total} MB (cifra de ComfyUI; torch reserva \
                         más de lo que declara)"
                    ),
                    models,
                    vram_total_mb: usado_mb,
                }
            }
            Err(e) => EngineState {
                engine: "comfyui".into(),
                online: true,
                note: format!("respuesta ilegible: {e}"),
                models: Vec::new(),
                vram_total_mb: 0,
            },
        },
        _ => EngineState {
            engine: "comfyui".into(),
            online: false,
            note: "sin respuesta en 8188".into(),
            models: Vec::new(),
            vram_total_mb: 0,
        },
    }
}

/// ¿Cabe un modelo de `size_gb` en la GPU indicada, dejando margen?
pub fn fits(size_gb: f64, gpu_index: u32) -> Option<(bool, String)> {
    let gpus = gpus_vram();
    let g = gpus.iter().find(|g| g.index == gpu_index)?;
    let necesario = (size_gb * 1024.0 * 1.12) as u64; // 12 % de margen para contexto y caché
    let libre = g.free_mb;
    Some((
        libre >= necesario,
        format!(
            "{}: necesita ~{} MB, libres {} MB → {}",
            g.name,
            necesario,
            libre,
            if libre >= necesario { "cabe" } else { "NO cabe" }
        ),
    ))
}

/// Informe completo de la guardia.
pub async fn report() -> VramReport {
    let gpus = gpus_vram();
    let processes = gpu_processes();
    let engines = vec![
        ollama_state().await,
        lmstudio_state().await,
        comfyui_state().await,
    ];

    let con_modelos: Vec<&EngineState> = engines.iter().filter(|e| !e.models.is_empty()).collect();
    let competition = con_modelos.len() > 1;

    let mut recommendations = Vec::new();
    let mut warning = String::new();

    if competition {
        let nombres: Vec<String> = con_modelos
            .iter()
            .map(|e| format!("{} ({})", e.engine, e.models.len()))
            .collect();
        warning = format!(
            "COMPETENCIA DE VRAM: {} motores tienen modelos cargados a la vez: {}",
            con_modelos.len(),
            nombres.join(", ")
        );
        recommendations.push(
            "Descarga los modelos de uno de los dos motores antes de seguir: en Fase 2.1 esta \
             situación bajó el rendimiento de 83 tok/s a 0,13 tok/s."
                .into(),
        );
    }

    for g in &gpus {
        if g.free_mb < 1024 {
            recommendations.push(format!(
                "{}: solo {} MB libres de {} MB. No cargues otro modelo ahí.",
                g.name, g.free_mb, g.total_mb
            ));
        }
    }

    if let Some(g) = gpus.iter().find(|g| g.index == 0) {
        if g.used_mb > g.total_mb / 2 {
            recommendations.push(format!(
                "GPU0 ({}): {} MB ocupados por el escritorio y aplicaciones. Es la GPU de pantalla \
                 y juegos; no la uses para inferencia pesada.",
                g.name, g.used_mb
            ));
        }
    }

    if !engines.iter().any(|e| e.online) {
        recommendations.push("Ningún motor responde: Ollama en 11434, LM Studio en 1234/1235 y ComfyUI en 8188.".into());
    }

    VramReport {
        gpus,
        engines,
        processes,
        competition,
        warning,
        recommendations,
        checked_at: now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detecta_gpus_reales() {
        let gpus = gpus_vram();
        println!("GPUs detectadas: {}", gpus.len());
        for g in &gpus {
            println!(
                "  {} {} total={}MB usada={}MB libre={}MB reservada={}MB",
                g.index, g.name, g.total_mb, g.used_mb, g.free_mb, g.reserved_mb
            );
        }
        assert!(!gpus.is_empty(), "nvidia-smi debe devolver al menos una GPU");
        assert!(gpus.iter().any(|g| g.name.contains("5070")));
        assert!(gpus.iter().any(|g| g.name.contains("5060")));
    }

    #[test]
    fn el_informe_incluye_los_tres_motores() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let r = rt.block_on(report());
        let nombres: Vec<&str> = r.engines.iter().map(|e| e.engine.as_str()).collect();
        println!("motores vigilados: {nombres:?}");
        assert!(nombres.contains(&"ollama"));
        assert!(nombres.contains(&"lmstudio"));
        assert!(nombres.contains(&"comfyui"), "ComfyUI debe vigilarse: es el que más VRAM usa");
        for e in &r.engines {
            println!(
                "  {:<9} online={:<5} modelos={} · {} MB · {}",
                e.engine,
                e.online,
                e.models.len(),
                e.vram_total_mb,
                e.note
            );
        }
        println!("competencia: {} · {}", r.competition, r.warning);
    }

    #[test]
    fn margen_de_cabeza_de_modelo() {
        // 1000 GB nunca cabe; 0 GB siempre cabe.
        if let Some((ok, msg)) = fits(1000.0, 0) {
            println!("{msg}");
            assert!(!ok);
        }
        if let Some((ok, msg)) = fits(0.0, 0) {
            println!("{msg}");
            assert!(ok);
        }
    }
}
