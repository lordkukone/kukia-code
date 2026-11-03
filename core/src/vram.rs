//! Guardia de VRAM (Fase 3).
//!
//! Objetivo: evitar la pérdida brutal de rendimiento medida en Fase 2.1 cuando dos
//! motores de inferencia ocupan la misma GPU.
//!
//! Reglas:
//! - La guardia solo LEE: no cierra procesos, no toca configuraciones y no descarga
//!   nada por su cuenta.
//! - La descarga ocurre **solo** cuando el usuario pulsa «Expulsar» o «Vaciar VRAM»,
//!   y siempre por el mecanismo oficial de cada motor.
//! - Informa y avisa; la decisión es del usuario.
//! - La GPU 0 sigue siendo la principal de pantalla y juegos.

use serde::{Deserialize, Serialize};
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
    /// Trabajos que ComfyUI tiene en ejecución o en cola. Liberar su VRAM los
    /// interrumpe, así que la UI pide confirmación explícita cuando esto es > 0.
    #[serde(default)]
    pub comfy_busy: usize,
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
                        note: format!(
                            "puerto {port}: {} cargado(s) · LM Studio no publica la VRAM de cada \
                             modelo, así que las cifras por GPU salen de nvidia-smi",
                            models.len()
                        ),
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

/// Aviso de competencia listo para la interfaz.
///
/// Se calcula sin tocar la máquina, para poder probarlo sin red ni motores.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct VramAlert {
    /// Dos o más motores con modelos cargados a la vez.
    pub competing: bool,
    /// Motores con al menos un modelo cargado.
    pub count: usize,
    /// Detalle legible: `"ollama (1)"`.
    pub detalle: Vec<String>,
    /// Modelos cargados en total.
    pub models: usize,
    /// Texto listo para mostrar.
    pub message: String,
}

/// Decide si hay competencia de VRAM a partir del estado de los motores.
///
/// Regla: cuentan solo los motores **con modelos cargados**; un motor encendido pero
/// vacío no compite por la memoria.
pub fn evaluar_competencia(engines: &[EngineState]) -> VramAlert {
    let con_modelos: Vec<&EngineState> = engines.iter().filter(|e| !e.models.is_empty()).collect();
    let competing = con_modelos.len() > 1;
    let detalle: Vec<String> = con_modelos
        .iter()
        .map(|e| format!("{} ({})", e.engine, e.models.len()))
        .collect();
    let models = con_modelos.iter().map(|e| e.models.len()).sum();

    let message = if competing {
        format!(
            "COMPETENCIA DE VRAM: {} motores tienen modelos cargados a la vez: {}",
            con_modelos.len(),
            detalle.join(", ")
        )
    } else if let Some(uno) = detalle.first() {
        format!("Un solo motor con modelos cargados: {uno}")
    } else {
        "Ningún motor tiene modelos cargados.".to_string()
    };

    VramAlert {
        competing,
        count: con_modelos.len(),
        detalle,
        models,
        message,
    }
}

/* ================== Motores sin API de descarga (Wan2GP / Maestro) ================== */

fn base_wan2gp() -> String {
    std::env::var("KUKIA_WAN2GP").unwrap_or_else(|_| "http://127.0.0.1:7861".to_string())
}

fn base_maestro() -> String {
    std::env::var("KUKIA_MAESTRO").unwrap_or_else(|_| "http://127.0.0.1:7860".to_string())
}

/// Sondea un motor que **no** publica su inventario de modelos en memoria.
///
/// Wan2GP (Gradio) y Maestro no exponen ni qué modelo tienen cargado ni una ruta
/// para descargarlo: la única forma de liberar su VRAM sería cerrar el proceso, y
/// KUKIA no cierra procesos. Se dice tal cual en vez de inventar un botón que no
/// puede funcionar.
async fn motor_sin_api(engine: &str, etiqueta: &str, base: &str) -> EngineState {
    let vivo = match client(2).get(format!("{base}/" )).send().await {
        Ok(r) => r.status().is_success() || r.status().is_redirection(),
        Err(_) => false,
    };
    EngineState {
        engine: engine.into(),
        online: vivo,
        note: if vivo {
            format!("{etiqueta} responde en {base} · no publica qué modelo tiene cargado ni tiene descarga por API")
        } else {
            format!("{etiqueta} sin respuesta en {base}")
        },
        models: Vec::new(),
        vram_total_mb: 0,
    }
}

/// Informe completo de la guardia.
pub async fn report() -> VramReport {
    let gpus = gpus_vram();
    let comfy_busy = comfyui_generando().await.unwrap_or(0);
    let processes = gpu_processes();
    let engines = vec![
        ollama_state().await,
        lmstudio_state().await,
        comfyui_state().await,
        motor_sin_api("wan2gp", "Wan2GP", &base_wan2gp()).await,
        motor_sin_api("maestro", "Maestro", &base_maestro()).await,
    ];

    let alerta = evaluar_competencia(&engines);
    let competition = alerta.competing;

    let mut recommendations = Vec::new();
    let warning = if competition {
        alerta.message.clone()
    } else {
        String::new()
    };

    if competition {
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
        recommendations.push(
            "Ningún motor responde: Ollama en 11434, LM Studio en 1234/1235, ComfyUI en 8188, \
             Wan2GP en 7861 y Maestro en 7860."
                .into(),
        );
    }

    if comfy_busy > 0 {
        recommendations.push(format!(
            "ComfyUI tiene {comfy_busy} trabajo(s) en marcha: liberar su VRAM los interrumpe. \
             Espera a que termine o confirma que quieres cortarlos."
        ));
    }

    VramReport {
        gpus,
        engines,
        processes,
        competition,
        warning,
        recommendations,
        checked_at: now(),
        comfy_busy,
    }
}

/* ============================ Expulsar modelos ============================ */

/// Resultado de intentar sacar un modelo de la memoria.
#[derive(Debug, Clone, Serialize)]
pub struct UnloadResult {
    pub engine: String,
    pub model: String,
    pub ok: bool,
    pub message: String,
}

fn resultado(engine: &str, model: &str, ok: bool, message: impl Into<String>) -> UnloadResult {
    UnloadResult {
        engine: engine.into(),
        model: model.into(),
        ok,
        message: message.into(),
    }
}

/// Recorta respuestas de error para no volcar cuerpos enormes en la interfaz.
fn recortar(s: &str) -> String {
    let t = s.trim().replace(['\n', '\r'], " ");
    if t.chars().count() > 160 {
        format!("{}…", t.chars().take(160).collect::<String>())
    } else {
        t
    }
}

/// Ollama descarga con `keep_alive: 0`: deja de retener el modelo al terminar la
/// petición. Comprobado en esta máquina: la ruta existe (un modelo inexistente
/// devuelve el 404 del modelo, no un 404 de ruta).
async fn unload_ollama(model: &str) -> UnloadResult {
    let body = serde_json::json!({ "model": model, "keep_alive": 0, "stream": false });
    match client(30)
        .post("http://127.0.0.1:11434/api/generate")
        .json(&body)
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => resultado(
            "ollama",
            model,
            true,
            "descargado de VRAM (keep_alive 0)",
        ),
        Ok(r) => {
            let st = r.status();
            let texto = r.text().await.unwrap_or_default();
            resultado("ollama", model, false, format!("HTTP {st}: {}", recortar(&texto)))
        }
        Err(e) => resultado("ollama", model, false, format!("sin respuesta: {e}")),
    }
}

/// `instance_id` de un modelo cargado, tal como lo publica LM Studio.
///
/// `GET /api/v1/models` devuelve `loaded_instances[].id` y ese es exactamente el
/// campo que exige `POST /api/v1/models/unload`. Verificado contra la instalación
/// de esta máquina (32 modelos, uno cargado).
fn instancia_cargada(v: &serde_json::Value, modelo: &str) -> Option<String> {
    let lista = v.get("models").and_then(|m| m.as_array())?;
    for m in lista {
        let clave = m.get("key").and_then(|k| k.as_str()).unwrap_or("");
        let id = m.get("id").and_then(|k| k.as_str()).unwrap_or("");
        if clave != modelo && id != modelo {
            continue;
        }
        let instancias = m.get("loaded_instances").and_then(|i| i.as_array())?;
        return instancias
            .first()
            .and_then(|i| i.get("id"))
            .and_then(|i| i.as_str())
            .map(|s| s.to_string());
    }
    None
}

/// Último recurso: el CLI oficial de LM Studio (`lms unload <modelo>`), que
/// resuelve el identificador por su cuenta si la API no lo encuentra.
fn lms_unload(model: &str) -> Result<String, String> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map_err(|_| "sin USERPROFILE".to_string())?;
    let exe = std::path::PathBuf::from(home)
        .join(".lmstudio")
        .join("bin")
        .join(if cfg!(windows) { "lms.exe" } else { "lms" });
    if !exe.exists() {
        return Err("el CLI lms no está instalado".into());
    }
    let mut cmd = Command::new(&exe);
    cmd.args(["unload", model]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let salida = cmd.output().map_err(|e| e.to_string())?;
    if salida.status.success() {
        Ok("descargado con el CLI oficial lms".into())
    } else {
        Err(recortar(&String::from_utf8_lossy(&salida.stderr)))
    }
}

async fn unload_lmstudio(model: &str) -> UnloadResult {
    for puerto in [1234u16, 1235] {
        let base = format!("http://127.0.0.1:{puerto}");
        let Ok(r) = client(6).get(format!("{base}/api/v1/models")).send().await else {
            continue;
        };
        if !r.status().is_success() {
            continue;
        }
        let Ok(v) = r.json::<serde_json::Value>().await else {
            continue;
        };
        let Some(instancia) = instancia_cargada(&v, model) else {
            continue;
        };
        let body = serde_json::json!({ "instance_id": instancia });
        return match client(30)
            .post(format!("{base}/api/v1/models/unload"))
            .json(&body)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => resultado(
                "lmstudio",
                model,
                true,
                format!("descargado del puerto {puerto} (instancia {instancia})"),
            ),
            Ok(resp) => {
                let st = resp.status();
                let texto = resp.text().await.unwrap_or_default();
                resultado(
                    "lmstudio",
                    model,
                    false,
                    format!("HTTP {st}: {}", recortar(&texto)),
                )
            }
            Err(e) => resultado("lmstudio", model, false, format!("sin respuesta: {e}")),
        };
    }
    match lms_unload(model) {
        Ok(mensaje) => resultado("lmstudio", model, true, mensaje),
        Err(e) => resultado(
            "lmstudio",
            model,
            false,
            format!("la API no tiene esa instancia cargada y el CLI falló: {e}"),
        ),
    }
}

/// ComfyUI libera con `POST /free`. Definido en su `server.py`: las claves
/// `unload_models` y `free_memory` se pasan a la cola como banderas, así que la
/// descarga la ejecuta el worker y puede tardar unos segundos.
async fn unload_comfyui(forzar: bool) -> UnloadResult {
    // Si no se puede comprobar la cola, se falla del lado seguro: no se libera a ciegas.
    match comfyui_generando().await {
        Some(0) => {}
        Some(n) if !forzar => {
            return resultado(
                "comfyui",
                "modelo residente",
                false,
                format!(
                    "ComfyUI tiene {n} trabajo(s) en marcha: liberar la VRAM los interrumpe. \
                     Confirma la interrupción para seguir."
                ),
            );
        }
        None if !forzar => {
            return resultado(
                "comfyui",
                "modelo residente",
                false,
                "no se pudo comprobar si ComfyUI está generando (su /queue no respondió a \
                 tiempo): no se libera a ciegas; confirma la interrupción para forzar."
                    .to_string(),
            );
        }
        _ => {}
    }
    let body = serde_json::json!({ "unload_models": true, "free_memory": true });
    match client(30)
        .post("http://127.0.0.1:8188/free")
        .json(&body)
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => resultado(
            "comfyui",
            "modelo residente",
            true,
            "descarga pedida a ComfyUI (la ejecuta su worker; puede tardar unos segundos)",
        ),
        Ok(r) => {
            let st = r.status();
            let texto = r.text().await.unwrap_or_default();
            resultado("comfyui", "modelo residente", false, format!("HTTP {st}: {}", recortar(&texto)))
        }
        Err(e) => resultado("comfyui", "modelo residente", false, format!("sin respuesta: {e}")),
    }
}

/// ¿ComfyUI tiene trabajo en marcha? Devuelve ejecuciones + pendientes.
pub async fn comfyui_generando() -> Option<usize> {
    // Tiempo holgado: mientras genera, ComfyUI tarda en contestar y un falso «0»
    // dejaría liberar la VRAM en mitad de una generación.
    let v: serde_json::Value = client(8)
        .get("http://127.0.0.1:8188/queue")
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;
    let corriendo = v["queue_running"].as_array().map(|a| a.len()).unwrap_or(0);
    let esperando = v["queue_pending"].as_array().map(|a| a.len()).unwrap_or(0);
    Some(corriendo + esperando)
}

/// Saca un modelo concreto de la memoria del motor indicado.
///
/// `forzar` solo afecta a ComfyUI: su API libera todo y corta la generación activa,
/// así que sin permiso explícito se niega a tocarla mientras haya trabajo en marcha.
pub async fn unload(engine: &str, model: &str, forzar: bool) -> UnloadResult {
    match engine {
        "ollama" => unload_ollama(model).await,
        "lmstudio" => unload_lmstudio(model).await,
        "comfyui" => unload_comfyui(forzar).await,
        otro => resultado(
            otro,
            model,
            false,
            "motor desconocido: KUKIA no conoce un mecanismo de descarga para él",
        ),
    }
}

/// Vacía la VRAM de todos los motores que tengan algo cargado.
pub async fn unload_all(forzar: bool) -> Vec<UnloadResult> {
    let engines = vec![
        ollama_state().await,
        lmstudio_state().await,
        comfyui_state().await,
    ];
    let mut out = Vec::new();
    for e in engines {
        if !e.online || e.models.is_empty() {
            continue;
        }
        if e.engine == "comfyui" {
            out.push(unload_comfyui(forzar).await);
            continue;
        }
        for m in &e.models {
            out.push(unload(&e.engine, &m.name, forzar).await);
        }
    }
    out
}

/* ============ Descarga selectiva por lotes + verificación ============ */

/// Un modelo concreto de un motor concreto, elegido a mano.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Seleccion {
    pub engine: String,
    pub model: String,
    /// Permiso explícito para interrumpir una generación de ComfyUI.
    #[serde(default)]
    pub forzar: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiscoCheck {
    pub engine: String,
    pub model: String,
    /// `None` = este motor no publica una forma fiable de comprobarlo.
    pub presente: Option<bool>,
    pub nota: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoteResult {
    pub resultados: Vec<UnloadResult>,
    pub gpus_antes: Vec<GpuVram>,
    pub gpus_despues: Vec<GpuVram>,
    /// VRAM medida que se ha recuperado (suma de todas las GPU, solo si bajó).
    pub liberado_mb: u64,
    /// Modelos que se pidieron descargar y siguen residiendo.
    pub siguen_cargados: Vec<String>,
    pub en_disco: Vec<DiscoCheck>,
    pub avisos: Vec<String>,
}

/// ¿Sigue el modelo instalado en el motor? (No se borra nada: solo se consulta.)
pub async fn sigue_en_disco(engine: &str, model: &str) -> Option<bool> {
    match engine {
        "ollama" => {
            let r = client(4)
                .get("http://127.0.0.1:11434/api/tags")
                .send()
                .await
                .ok()?;
            if !r.status().is_success() {
                return None;
            }
            let v: serde_json::Value = r.json().await.ok()?;
            let nombres: Vec<&str> = v["models"]
                .as_array()?
                .iter()
                .filter_map(|m| m["name"].as_str())
                .collect();
            Some(nombres.iter().any(|n| *n == model || n.split(':').next() == Some(model)))
        }
        "lmstudio" => {
            for port in [1234u16, 1235] {
                let base = format!("http://127.0.0.1:{port}");
                let Ok(r) = client(4).get(format!("{base}/api/v1/models")).send().await else {
                    continue;
                };
                if !r.status().is_success() {
                    continue;
                }
                let Ok(v) = r.json::<serde_json::Value>().await else {
                    continue;
                };
                let claves: Vec<&str> = v["models"]
                    .as_array()
                    .map(|a| a.iter().filter_map(|m| m["key"].as_str()).collect())
                    .unwrap_or_default();
                return Some(claves.contains(&model));
            }
            None
        }
        // ComfyUI no expone un catálogo fiable de “modelo residente” por nombre.
        _ => None,
    }
}

/// Descarga una selección de modelos, midiendo VRAM antes y después de cada lote y
/// verificando que los archivos siguen en disco. No borra ni cierra nada.
pub async fn descargar_lote(items: Vec<Seleccion>) -> LoteResult {
    let antes = gpus_vram();
    let mut resultados = Vec::new();
    let mut avisos: Vec<String> = Vec::new();
    let mut comfy_avisado = false;

    for it in &items {
        if it.engine == "comfyui" && !comfy_avisado {
            avisos.push(
                "ComfyUI no permite descargar un modelo suelto: su API /free libera TODOS los modelos cargados y la memoria libre. Es lo que se hace al pedirlo."
                    .into(),
            );
            comfy_avisado = true;
        }
        resultados.push(unload(&it.engine, &it.model, it.forzar).await);
        tokio::time::sleep(std::time::Duration::from_millis(900)).await;
    }

    let despues = gpus_vram();
    let usado_antes: u64 = antes.iter().map(|g| g.used_mb).sum();
    let usado_despues: u64 = despues.iter().map(|g| g.used_mb).sum();
    let liberado_mb = usado_antes.saturating_sub(usado_despues);

    // ¿Se quedó algo de lo pedido en memoria?
    let estados = vec![
        ollama_state().await,
        lmstudio_state().await,
        comfyui_state().await,
    ];
    let mut cargados: Vec<String> = Vec::new();
    for e in &estados {
        for m in &e.models {
            cargados.push(format!("{}::{}", e.engine, m.name));
        }
    }
    let mut siguen_cargados: Vec<String> = Vec::new();
    for it in &items {
        let clave = format!("{}::{}", it.engine, it.model);
        if cargados.iter().any(|c| *c == clave) && !siguen_cargados.contains(&clave) {
            siguen_cargados.push(clave);
        }
    }
    if items.iter().any(|i| i.engine == "comfyui") {
        // En ComfyUI el nombre del modelo residente no coincide con el que se pidió.
        if let Some(e) = estados.iter().find(|e| e.engine == "comfyui") {
            for m in &e.models {
                let clave = format!("comfyui::{}", m.name);
                if !siguen_cargados.contains(&clave) {
                    siguen_cargados.push(clave);
                }
            }
        }
    }

    let mut en_disco = Vec::new();
    for it in &items {
        let presente = sigue_en_disco(&it.engine, &it.model).await;
        let nota = match (it.engine.as_str(), presente) {
            (_, Some(true)) => "sigue instalado: se puede volver a cargar cuando quieras".to_string(),
            (_, Some(false)) => "el motor no lo lista entre sus modelos instalados".to_string(),
            ("comfyui", _) => "ComfyUI no publica un listado fiable por nombre".to_string(),
            _ => "no se pudo comprobar (motor sin respuesta)".to_string(),
        };
        en_disco.push(DiscoCheck {
            engine: it.engine.clone(),
            model: it.model.clone(),
            presente,
            nota,
        });
    }

    if liberado_mb == 0 && !resultados.is_empty() {
        avisos.push(
            "No se ha medido VRAM recuperada: el motor puede haberla soltado sin cambiar el contador de nvidia-smi, o Windows y otros procesos siguen ocupándola."
                .into(),
        );
    }

    LoteResult {
        resultados,
        gpus_antes: antes,
        gpus_despues: despues,
        liberado_mb,
        siguen_cargados,
        en_disco,
        avisos,
    }
}

/* ============================ Historial ============================ */

#[derive(Debug, Clone, Serialize)]
pub struct OpVram {
    pub id: i64,
    pub engine: String,
    pub model: String,
    pub ok: bool,
    pub liberado_mb: u64,
    pub nota: String,
    pub created_at: String,
}

pub fn historial_add(
    db: &crate::memory::Database,
    engine: &str,
    model: &str,
    ok: bool,
    liberado_mb: u64,
    nota: &str,
) -> Result<(), String> {
    db.conn()
        .execute(
            "INSERT INTO vram_ops(engine, model, ok, liberado_mb, nota, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![engine, model, ok as i64, liberado_mb as i64, nota, now()],
        )
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn historial(db: &crate::memory::Database, limit: usize) -> Vec<OpVram> {
    let Ok(mut st) = db.conn().prepare(
        "SELECT id, engine, model, ok, liberado_mb, nota, created_at FROM vram_ops \
         ORDER BY id DESC LIMIT ?1",
    ) else {
        return Vec::new();
    };
    let filas = st.query_map(rusqlite::params![limit as i64], |r| {
        Ok(OpVram {
            id: r.get(0)?,
            engine: r.get(1)?,
            model: r.get(2)?,
            ok: r.get::<_, i64>(3)? > 0,
            liberado_mb: r.get::<_, i64>(4)? as u64,
            nota: r.get(5)?,
            created_at: r.get(6)?,
        })
    });
    match filas {
        Ok(it) => it.filter_map(|r| r.ok()).collect(),
        Err(_) => Vec::new(),
    }
}

/* ============ Gestor de liberación: medido, por motor y por GPU ============ */

/// Estado de una GPU con lo que la operación recuperó en ella.
#[derive(Debug, Clone, Serialize)]
pub struct GpuUsed {
    pub index: u32,
    pub name: String,
    pub used_mb: u64,
    pub total_mb: u64,
    /// MB recuperados en esta GPU durante la operación (0 si no cambió).
    pub freed_mb: u64,
}

/// Resultado de una liberación con medición antes/después.
#[derive(Debug, Clone, Serialize)]
pub struct ReleaseReport {
    /// `"ollama"`, `"gpu1"` o `"ambas"`.
    pub target: String,
    /// Cierto si no hubo ningún fallo (también si no había nada que descargar).
    pub ok: bool,
    pub freed_mb: u64,
    pub gpus: Vec<GpuUsed>,
    pub results: Vec<UnloadResult>,
    pub note: String,
    pub checked_at: String,
}

fn usada_por_gpu() -> Vec<(u32, String, u64, u64)> {
    gpus_vram()
        .into_iter()
        .map(|g| (g.index, g.name, g.used_mb, g.total_mb))
        .collect()
}

/// Diferencia de VRAM usada entre dos lecturas, GPU a GPU.
///
/// `saturating_sub`: si la memoria **sube** (otro proceso carga algo a la vez)
/// se informa 0 recuperado en vez de un número negativo que no significa nada.
fn comparar(antes: &[(u32, String, u64, u64)], despues: &[(u32, String, u64, u64)]) -> Vec<GpuUsed> {
    despues
        .iter()
        .map(|(idx, name, used, total)| {
            let previa = antes.iter().find(|(i, _, _, _)| i == idx).map(|(_, _, u, _)| *u);
            GpuUsed {
                index: *idx,
                name: name.clone(),
                used_mb: *used,
                total_mb: *total,
                freed_mb: previa.map(|p| p.saturating_sub(*used)).unwrap_or(0),
            }
        })
        .collect()
}

/// Diferencia medida, pero solo si la operación llegó a pedir algo al motor.
///
/// Si todos los resultados son negativos (por ejemplo ComfyUI negándose porque está
/// generando) no se le apunta a KUKIA una memoria que soltó otro proceso: la cifra
/// se queda en 0 aunque nvidia-smi marque menos que antes.
fn comparar_si_se_actuo(
    antes: &[(u32, String, u64, u64)],
    hubo_orden: bool,
) -> Vec<GpuUsed> {
    let mut gpus = comparar(antes, &usada_por_gpu());
    if !hubo_orden {
        for g in &mut gpus {
            g.freed_mb = 0;
        }
    }
    gpus
}

fn total_recuperado(gpus: &[GpuUsed]) -> u64 {
    gpus.iter().map(|g| g.freed_mb).sum()
}

/// Los motores tardan en soltar la memoria: se les da un respiro antes de medir.
async fn esperar_liberacion() {
    tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
}

/// Descarga **todos** los modelos de un motor concreto, por su mecanismo oficial.
pub async fn release_engine(engine: &str, forzar: bool) -> ReleaseReport {
    let antes = usada_por_gpu();
    let mut results: Vec<UnloadResult> = Vec::new();
    #[allow(unused_assignments)]
    let mut note = String::new();
    let mut ok = true;

    match engine {
        "ollama" | "lmstudio" => {
            let e = if engine == "ollama" {
                ollama_state().await
            } else {
                lmstudio_state().await
            };
            if !e.online {
                ok = false;
                note = format!("{} no responde: {}", engine, e.note);
            } else if e.models.is_empty() {
                note = format!("{} no tiene modelos cargados", engine);
            } else {
                for m in &e.models {
                    results.push(unload(engine, &m.name, forzar).await);
                }
                note = format!("{} modelo(s) descargado(s) de {}", results.len(), engine);
            }
        }
        "comfyui" => {
            let r = unload_comfyui(forzar).await;
            ok = r.ok;
            note = if r.ok {
                "descarga pedida a ComfyUI; su worker la ejecuta y se mide el resultado".into()
            } else {
                r.message.clone()
            };
            results.push(r);
        }
        "wan2gp" | "maestro" => {
            ok = false;
            note = format!(
                "{engine} no ofrece API para descargar modelos: solo libera su VRAM al cerrar el \
                 proceso, y KUKIA no cierra procesos de Windows"
            );
            results.push(resultado(engine, "modelo residente", false, note.clone()));
        }
        otro => {
            ok = false;
            note = format!(
                "motor desconocido ({otro}): KUKIA no conoce un mecanismo de descarga para él"
            );
        }
    }

    if !results.is_empty() {
        ok = ok && results.iter().all(|r| r.ok);
    }
    if results.iter().any(|r| r.ok) {
        esperar_liberacion().await;
    }

    let hubo_orden = results.iter().any(|r| r.ok);
    let gpus = comparar_si_se_actuo(&antes, hubo_orden);
    ReleaseReport {
        target: engine.to_string(),
        ok,
        freed_mb: total_recuperado(&gpus),
        gpus,
        results,
        note,
        checked_at: now(),
    }
}

/// Libera la VRAM de una GPU.
///
/// **Límite real medido (01/11/2026):** ningún motor local permite descargar solo la
/// parte de un modelo que vive en una GPU. Ollama repartió `qwen3-vl:4b` entre las dos
/// (11.528 + 9.661 MB) y su API solo sabe descargar el modelo entero. Por eso esta
/// operación descarga los modelos completos y **mide después cuánto se recuperó en
/// cada GPU**, en vez de fingir una selectividad que no existe.
pub async fn release_gpu(gpu: u32, forzar: bool) -> ReleaseReport {
    let antes = usada_por_gpu();
    if !antes.iter().any(|(i, _, _, _)| *i == gpu) {
        return ReleaseReport {
            target: format!("gpu{gpu}"),
            ok: false,
            freed_mb: 0,
            gpus: Vec::new(),
            results: Vec::new(),
            note: format!("la GPU {gpu} no existe en esta máquina"),
            checked_at: now(),
        };
    }

    let results = unload_all(forzar).await;
    if !results.is_empty() {
        esperar_liberacion().await;
    }
    let hubo_orden = results.iter().any(|r| r.ok);
    let gpus = comparar_si_se_actuo(&antes, hubo_orden);
    let propio = gpus.iter().find(|g| g.index == gpu).map(|g| g.freed_mb).unwrap_or(0);
    let note = if results.is_empty() {
        "no había modelos cargados que descargar".to_string()
    } else {
        format!(
            "ningún motor local descarga solo la parte de una GPU: se descargaron {} modelo(s) \
             enteros y en GPU{gpu} se recuperaron {:.2} GB (el total de ambas puede ser mayor si \
             algún modelo estaba repartido)",
            results.iter().filter(|r| r.ok).count(),
            propio as f64 / 1024.0
        )
    };

    ReleaseReport {
        target: format!("gpu{gpu}"),
        ok: results.iter().all(|r| r.ok),
        freed_mb: total_recuperado(&gpus),
        gpus,
        results,
        note,
        checked_at: now(),
    }
}

/// Libera las dos GPU (todos los motores), midiendo el resultado.
pub async fn release_everything(forzar: bool) -> ReleaseReport {
    let antes = usada_por_gpu();
    let results = unload_all(forzar).await;
    if !results.is_empty() {
        esperar_liberacion().await;
    }
    let hubo_orden = results.iter().any(|r| r.ok);
    let gpus = comparar_si_se_actuo(&antes, hubo_orden);
    let note = if results.is_empty() {
        "no había modelos cargados que descargar".to_string()
    } else {
        format!("{} modelo(s) descargado(s)", results.iter().filter(|r| r.ok).count())
    };
    ReleaseReport {
        target: "ambas".into(),
        ok: results.iter().all(|r| r.ok),
        freed_mb: total_recuperado(&gpus),
        gpus,
        results,
        note,
        checked_at: now(),
    }
}

/// MB que se podrían recuperar si se descargara todo lo cargado ahora mismo.
///
/// Es una estimación: en LM Studio la API no publica la VRAM del modelo cargado, así
/// que se usa su tamaño de archivo como suelo, y en ComfyUI se usa lo que declara torch.
pub fn recuperable_mb(engines: &[EngineState]) -> u64 {
    engines
        .iter()
        .map(|e| {
            e.models
                .iter()
                .map(|m| {
                    if m.vram_mb > 0 {
                        m.vram_mb
                    } else {
                        (m.size_gb * 1024.0) as u64
                    }
                })
                .sum::<u64>()
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn motor(engine: &str, modelos: usize) -> EngineState {
        EngineState {
            engine: engine.into(),
            online: true,
            note: String::new(),
            models: (0..modelos)
                .map(|i| EngineModel {
                    name: format!("modelo-{i}"),
                    vram_mb: 1024,
                    size_gb: 1.0,
                })
                .collect(),
            vram_total_mb: 1024,
        }
    }

    #[test]
    fn dos_motores_con_modelos_disparan_el_aviso() {
        let a = evaluar_competencia(&[motor("ollama", 1), motor("comfyui", 1)]);
        assert!(a.competing, "dos motores con modelos deben competir");
        assert_eq!(a.count, 2);
        assert_eq!(a.models, 2);
        assert!(a.message.starts_with("COMPETENCIA DE VRAM"));
        assert!(a.message.contains("ollama (1)"));
        assert!(a.message.contains("comfyui (1)"));
    }

    #[test]
    fn un_solo_motor_no_es_competencia() {
        let a = evaluar_competencia(&[motor("ollama", 2), motor("lmstudio", 0)]);
        assert!(!a.competing, "un motor solo no puede competir consigo mismo");
        assert_eq!(a.count, 1);
        assert_eq!(a.models, 2);
        assert!(a.message.contains("ollama (2)"));
    }

    #[test]
    fn motores_encendidos_pero_vacios_no_compiten() {
        let a = evaluar_competencia(&[motor("ollama", 0), motor("lmstudio", 0), motor("comfyui", 0)]);
        assert!(!a.competing);
        assert_eq!(a.count, 0);
        assert_eq!(a.detalle.len(), 0);
        assert!(a.message.contains("Ningún motor"));
    }

    #[test]
    fn el_aviso_cuenta_modelos_de_todos_los_motores() {
        let a = evaluar_competencia(&[motor("ollama", 2), motor("lmstudio", 1), motor("comfyui", 3)]);
        assert!(a.competing);
        assert_eq!(a.count, 3);
        assert_eq!(a.models, 6);
        assert_eq!(a.detalle.len(), 3);
    }

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
    fn lee_el_instance_id_de_lm_studio() {
        // Forma real de `GET /api/v1/models` en esta máquina.
        let v: serde_json::Value = serde_json::from_str(
            r#"{"models":[
                {"key":"flint-40b-abliterated","id":"flint-40b-abliterated",
                 "loaded_instances":[{"id":"flint-40b-abliterated","config":{}}]},
                {"key":"otro","id":"otro","loaded_instances":[]}
            ]}"#,
        )
        .unwrap();
        assert_eq!(
            instancia_cargada(&v, "flint-40b-abliterated"),
            Some("flint-40b-abliterated".to_string())
        );
        assert_eq!(instancia_cargada(&v, "otro"), None, "sin instancias no se descarga");
        assert_eq!(instancia_cargada(&v, "no-existe"), None);
    }

    #[test]
    fn un_motor_desconocido_no_finge_exito() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let r = rt.block_on(unload("vllm", "lo-que-sea", false));
        assert!(!r.ok, "no se puede decir que se descargó lo que no se sabe descargar");
        assert_eq!(r.engine, "vllm");
        assert!(r.message.contains("desconocido"));
    }

    #[test]
    fn la_diferencia_mide_lo_recuperado_por_gpu() {
        let antes = vec![(0u32, "RTX 5070".to_string(), 11528u64, 12282u64), (1, "RTX 5060 Ti".into(), 9661, 16311)];
        let despues = vec![(0u32, "RTX 5070".to_string(), 2376u64, 12282u64), (1, "RTX 5060 Ti".into(), 212, 16311)];
        let g = comparar(&antes, &despues);
        assert_eq!(g[0].freed_mb, 9152);
        assert_eq!(g[1].freed_mb, 9449);
        assert_eq!(total_recuperado(&g), 18601, "fue lo medido de verdad al descargar qwen3-vl:4b");
    }

    #[test]
    fn si_la_memoria_sube_no_se_informa_negativo() {
        let antes = vec![(1u32, "RTX 5060 Ti".to_string(), 200u64, 16311u64)];
        let despues = vec![(1u32, "RTX 5060 Ti".to_string(), 5000u64, 16311u64)];
        let g = comparar(&antes, &despues);
        assert_eq!(g[0].freed_mb, 0, "otro proceso puede cargar mientras se mide");
    }

    #[test]
    fn una_gpu_que_no_existe_no_finge_exito() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let r = rt.block_on(release_gpu(9, false));
        assert!(!r.ok);
        assert_eq!(r.freed_mb, 0);
        assert!(r.note.contains("no existe"));
    }

    #[test]
    fn wan2gp_y_maestro_se_declaran_sin_api_de_descarga() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let r = rt.block_on(release_engine("wan2gp", false));
        assert!(!r.ok, "no se puede afirmar que se descargó algo que no se puede descargar");
        assert!(r.note.contains("no ofrece API"));
        assert_eq!(r.results.len(), 1);
        assert!(!r.results[0].ok);
    }

    #[test]
    fn el_recuperable_suma_modelos_y_usa_el_tamano_si_falta_la_vram() {
        let engines = vec![
            motor("ollama", 2),  // 2 × 1024 MB
            EngineState {
                engine: "lmstudio".into(),
                online: true,
                note: String::new(),
                models: vec![EngineModel { name: "m".into(), vram_mb: 0, size_gb: 2.0 }],
                vram_total_mb: 0,
            },
        ];
        assert_eq!(recuperable_mb(&engines), 4096);
    }

    #[test]
    fn el_historial_registra_y_devuelve_lo_ultimo() {
        let p = std::env::temp_dir().join(format!("kukia_vram_hist_{}.db", std::process::id()));
        let _ = std::fs::remove_file(&p);
        let db = crate::memory::Database::open(&p).expect("db de prueba");
        historial_add(&db, "ollama", "flint:latest", true, 0, "descargado de VRAM").unwrap();
        historial_add(&db, "lote", "2 modelos", true, 4096, "VRAM medida").unwrap();
        let h = historial(&db, 10);
        assert_eq!(h.len(), 2);
        assert_eq!(h[0].engine, "lote");
        assert_eq!(h[0].liberado_mb, 4096);
        assert!(h[1].ok);
        drop(db);
        let _ = std::fs::remove_file(p);
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
