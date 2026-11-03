//! KUKIA MODEL HUB — descubrimiento, compatibilidad y descarga.
//!
//! Lo que se ha comprobado en esta máquina (02/11/2026), y por eso se hace así:
//!
//! - `https://ollama.com/api/search` **no existe** (404). La búsqueda de la librería
//!   se hace leyendo el HTML público de `ollama.com/search`, que sí responde 200.
//! - `registry.ollama.ai/.../tags/list` **no existe** (404), pero el **manifiesto**
//!   sí: de ahí salen los tamaños reales de cada capa.
//! - La API de Hugging Face responde sin clave y trae licencia, descargas y fecha.
//!
//! Nada de esto descarga ni instala por su cuenta: buscar es buscar.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

const OLLAMA_LOCAL: &str = "http://127.0.0.1:11434";

fn cliente(secs: u64) -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(secs))
        .user_agent("KUKIA-CODE/0.1 (model-hub)")
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

/* ============================ Ollama: búsqueda ============================ */

#[derive(Debug, Clone, Serialize)]
pub struct OllamaHit {
    pub name: String,
    pub url: String,
}

/// Busca en la librería pública de Ollama leyendo `ollama.com/search`.
/// Devuelve solo nombres y enlaces: el detalle (tamaños) va aparte.
pub async fn buscar_ollama(q: &str) -> Result<Vec<OllamaHit>, String> {
    let q = q.trim();
    if q.is_empty() {
        return Err("escribe algo para buscar en la librería de Ollama".into());
    }
    let url = format!("https://ollama.com/search?q={}", urlencoding(q));
    let html = cliente(20)
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("ollama.com no respondió: {e}"))?
        .error_for_status()
        .map_err(|e| format!("ollama.com devolvió un error: {e}"))?
        .text()
        .await
        .map_err(|e| e.to_string())?;

    let mut out: Vec<OllamaHit> = Vec::new();
    let aguja = "href=\"/library/";
    let mut resto = html.as_str();
    while let Some(pos) = resto.find(aguja) {
        resto = &resto[pos + aguja.len()..];
        let Some(fin) = resto.find('"') else { break };
        let name = &resto[..fin];
        // Se descartan las rutas con subcarpeta (`library/x/y` no es un modelo).
        if !name.is_empty() && !name.contains('/') && !name.contains('?') {
            if !out.iter().any(|h| h.name == name) {
                out.push(OllamaHit {
                    name: name.to_string(),
                    url: format!("https://ollama.com/library/{name}"),
                });
            }
        }
    }
    if out.is_empty() {
        return Err("la página de búsqueda no trajo resultados legibles (¿cambió el HTML?)".into());
    }
    Ok(out)
}

fn urlencoding(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[derive(Debug, Clone, Serialize)]
pub struct OllamaTag {
    pub name: String,
    /// Tamaño total real del manifiesto en GB, si el registry lo publica.
    pub size_gb: Option<f64>,
    pub digest: Option<String>,
    pub note: String,
}

/// Consulta el manifiesto de un modelo concreto (`nombre:tag` o `nombre`).
pub async fn manifiesto_ollama(modelo: &str) -> Result<OllamaTag, String> {
    let (nombre, tag) = match modelo.split_once(':') {
        Some((n, t)) => (n.to_string(), t.to_string()),
        None => (modelo.to_string(), "latest".to_string()),
    };
    let url = format!("https://registry.ollama.ai/v2/library/{nombre}/manifests/{tag}");
    let r = cliente(20)
        .get(&url)
        .header(
            "Accept",
            "application/vnd.docker.distribution.manifest.v2+json",
        )
        .send()
        .await
        .map_err(|e| format!("registry.ollama.ai no respondió: {e}"))?;
    let estado = r.status();
    if !estado.is_success() {
        return Err(format!(
            "«{modelo}» no está en la librería de Ollama (HTTP {estado})"
        ));
    }
    let v: serde_json::Value = r.json().await.map_err(|e| e.to_string())?;
    let bytes: f64 = v["layers"]
        .as_array()
        .map(|a| a.iter().filter_map(|l| l["size"].as_f64()).sum())
        .unwrap_or(0.0);
    Ok(OllamaTag {
        name: format!("{nombre}:{tag}"),
        size_gb: if bytes > 0.0 {
            Some(bytes / 1_073_741_824.0)
        } else {
            None
        },
        digest: v["config"]["digest"].as_str().map(|s| s.to_string()),
        note: "tamaño real del manifiesto del registry".into(),
    })
}

/* ============================ Hugging Face ============================ */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HubModelo {
    pub id: String,
    pub url: String,
    pub autor: String,
    pub descargas: u64,
    pub likes: u64,
    /// Licencia declarada en las etiquetas (`license:…`), si la hay.
    pub licencia: Option<String>,
    pub modificado: Option<String>,
    pub pipeline: Option<String>,
    pub etiquetas: Vec<String>,
    pub fuente: String,
}

/// Busca modelos GGUF en Hugging Face con su API pública.
pub async fn buscar_hf(q: &str, limite: usize) -> Result<Vec<HubModelo>, String> {
    let q = q.trim();
    if q.is_empty() {
        return Err("escribe algo para buscar en Hugging Face".into());
    }
    let n = limite.clamp(1, 50);
    let url = format!(
        "https://huggingface.co/api/models?search={}&filter=gguf&sort=downloads&direction=-1&limit={n}",
        urlencoding(q)
    );
    let lista: Vec<serde_json::Value> = cliente(25)
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Hugging Face no respondió: {e}"))?
        .error_for_status()
        .map_err(|e| format!("Hugging Face devolvió un error: {e}"))?
        .json()
        .await
        .map_err(|e| format!("respuesta ilegible de Hugging Face: {e}"))?;

    let mut out = Vec::new();
    for m in lista {
        let Some(id) = m["id"].as_str() else { continue };
        let etiquetas: Vec<String> = m["tags"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|t| t.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        let licencia = etiquetas
            .iter()
            .find(|t| t.starts_with("license:"))
            .map(|t| t.trim_start_matches("license:").to_string());
        let autor = id.split('/').next().unwrap_or("").to_string();
        out.push(HubModelo {
            id: id.to_string(),
            url: format!("https://huggingface.co/{id}"),
            autor,
            descargas: m["downloads"].as_u64().unwrap_or(0),
            likes: m["likes"].as_u64().unwrap_or(0),
            licencia,
            modificado: m["lastModified"]
                .as_str()
                .or_else(|| m["createdAt"].as_str())
                .map(|s| s.to_string()),
            pipeline: m["pipeline_tag"].as_str().map(|s| s.to_string()),
            etiquetas: etiquetas.into_iter().take(12).collect(),
            fuente: "huggingface".into(),
        });
    }
    Ok(out)
}

/* ============================ GGUF locales ============================ */

#[derive(Debug, Clone, Serialize)]
pub struct GgufLocal {
    pub path: String,
    pub nombre: String,
    pub size_gb: f64,
    /// `true` cuando es una parte de un archivo dividido (`-00001-of-00003`).
    pub dividido: bool,
    pub parte: Option<String>,
    /// Arquitectura leída de la cabecera GGUF, si se ha podido.
    pub arquitectura: Option<String>,
    pub version: Option<u32>,
    pub cuantizacion: Option<String>,
    /// `false` si no es un GGUF real (puntero de LFS, error de descarga, vacío…).
    pub valido: bool,
    pub nota: String,
}

/// Raíces por defecto donde este equipo guarda modelos.
pub fn raices_gguf() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for p in [
        "G:\\descargas",
        "J:\\models",
        "J:\\models\\video",
        "H:\\models",
        "C:\\Users\\kuki\\models",
    ] {
        let p = PathBuf::from(p);
        if p.exists() {
            out.push(p);
        }
    }
    let datos = crate::config::data_dir().join("models");
    if datos.exists() {
        out.push(datos);
    }
    out
}

/// Parser mínimo de cabecera GGUF: magia, versión y arquitectura.
fn leer_cabecera(path: &Path) -> (Option<u32>, Option<String>, Option<String>) {
    use std::io::Read;
    let Ok(mut f) = std::fs::File::open(path) else {
        return (None, None, None);
    };
    let mut cab = [0u8; 4];
    if f.read_exact(&mut cab).is_err() || &cab != b"GGUF" {
        return (None, None, None);
    }
    let mut b4 = [0u8; 4];
    if f.read_exact(&mut b4).is_err() {
        return (None, None, None);
    }
    let version = u32::from_le_bytes(b4);
    let mut b8 = [0u8; 8];
    if f.read_exact(&mut b8).is_err() {
        return (Some(version), None, None);
    }
    let _tensores = u64::from_le_bytes(b8);
    if f.read_exact(&mut b8).is_err() {
        return (Some(version), None, None);
    }
    let kv_count = u64::from_le_bytes(b8);
    let leer_u64 = |f: &mut std::fs::File| -> Option<u64> {
        let mut b = [0u8; 8];
        f.read_exact(&mut b).ok().map(|_| u64::from_le_bytes(b))
    };
    // Tipos GGUF: 4=string, 8=string, 9=array…
    let leer_string = |f: &mut std::fs::File, len: u64| -> Option<String> {
        if len > 4096 {
            return None;
        }
        let mut buf = vec![0u8; len as usize];
        f.read_exact(&mut buf).ok()?;
        Some(String::from_utf8_lossy(&buf).to_string())
    };

    let mut arquitectura = None;
    let mut nombre = None;
    for _ in 0..kv_count.min(64) {
        let Some(klen) = leer_u64(&mut f) else { break };
        let Some(clave) = leer_string(&mut f, klen) else {
            break;
        };
        let mut tipo = [0u8; 4];
        if f.read_exact(&mut tipo).is_err() {
            break;
        }
        match u32::from_le_bytes(tipo) {
            4 | 5 | 6 | 7 | 10 | 11 | 12 => break, // números: se corta sin seguir
            8 => {
                let Some(vlen) = leer_u64(&mut f) else { break };
                let Some(valor) = leer_string(&mut f, vlen) else {
                    break;
                };
                if clave == "general.architecture" {
                    arquitectura = Some(valor.clone());
                }
                if clave == "general.name" && nombre.is_none() {
                    nombre = Some(valor);
                }
            }
            _ => break, // arrays y tipos complejos: no hacen falta aquí
        }
        if arquitectura.is_some() && nombre.is_some() {
            break;
        }
    }
    (Some(version), arquitectura, nombre)
}

/// Primeros bytes legibles de un archivo que no es un GGUF, para saber qué es.
fn pista_de(path: &Path, size: u64) -> String {
    use std::io::Read;
    if size < 4096 {
        if let Ok(mut f) = std::fs::File::open(path) {
            let mut buf = vec![0u8; 120];
            let n = f.read(&mut buf).unwrap_or(0);
            let texto = String::from_utf8_lossy(&buf[..n]).trim().to_string();
            if !texto.is_empty() {
                return format!("{size} bytes, no es un GGUF: «{texto}»");
            }
        }
        return format!("{size} bytes: archivo vacío o incompleto");
    }
    "la cabecera no empieza por GGUF: archivo corrupto o de otro formato".into()
}

fn cuantizacion_de(nombre: &str) -> Option<String> {
    let bajo = nombre.to_uppercase();
    for q in [
        "IQ4_XS", "Q8_0", "Q6_K", "Q5_K_M", "Q5_K_S", "Q4_K_M", "Q4_K_S", "Q4_0", "Q3_K_M",
        "Q3_K_S", "Q2_K", "MXFP4", "BF16", "F16",
    ] {
        if bajo.contains(q) {
            return Some(q.to_string());
        }
    }
    None
}

/// Recorre las raíces (sin seguir enlaces simbólicos) y devuelve los GGUF vistos.
pub fn gguf_locales(raices: &[PathBuf], max: usize) -> Vec<GgufLocal> {
    let mut out = Vec::new();
    let mut pila: Vec<PathBuf> = raices.to_vec();
    let mut vistos = 0usize;
    while let Some(dir) = pila.pop() {
        if vistos > 20_000 {
            break;
        }
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.flatten() {
            vistos += 1;
            let p = e.path();
            let Ok(meta) = e.metadata() else { continue };
            if meta.is_dir() {
                pila.push(p);
                continue;
            }
            if !p
                .extension()
                .map(|x| x.eq_ignore_ascii_case("gguf"))
                .unwrap_or(false)
            {
                continue;
            }
            if out.len() >= max {
                return out;
            }
            let nombre = p
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            let (version, arquitectura, _nombre_interno) = leer_cabecera(&p);
            let parte = nombre
                .split('-')
                .find(|s| s.len() == 5 && s.chars().all(|c| c.is_ascii_digit()))
                .map(|s| s.to_string());
            let valido = version.is_some() && meta.len() > 4096;
            let nota = if valido {
                String::new()
            } else {
                pista_de(&p, meta.len())
            };
            out.push(GgufLocal {
                path: p.display().to_string(),
                nombre: nombre.clone(),
                size_gb: meta.len() as f64 / 1_073_741_824.0,
                dividido: parte.is_some(),
                parte,
                arquitectura,
                version,
                cuantizacion: cuantizacion_de(&nombre),
                valido,
                nota,
            });
        }
    }
    out.sort_by(|a, b| b.size_gb.partial_cmp(&a.size_gb).unwrap_or(std::cmp::Ordering::Equal));
    out
}

/* ============================ Compatibilidad ============================ */

#[derive(Debug, Clone, Serialize)]
pub struct GpuLibre {
    pub index: u32,
    pub name: String,
    pub total_gb: f64,
    pub free_gb: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Compatibilidad {
    /// COMPATIBLE | COMPATIBLE CON LIMITACIONES | NO COMPATIBLE | DESCONOCIDO
    pub veredicto: String,
    pub pesos_gb: Option<f64>,
    pub kv_gb: Option<f64>,
    pub margen_gb: Option<f64>,
    pub total_gb: Option<f64>,
    pub gpus: Vec<GpuLibre>,
    pub ram_libre_gb: f64,
    pub cabe_en_una_gpu: Option<u32>,
    pub requiere_offload: bool,
    pub explicacion: String,
}

/// Bits efectivos por peso, aproximados por cuantización conocida.
pub fn bits_por_peso(quant: &str) -> Option<f64> {
    let q = quant.to_uppercase();
    let v = if q.contains("Q2") {
        3.0
    } else if q.contains("Q3") {
        3.9
    } else if q.contains("Q4") {
        4.8
    } else if q.contains("Q5") {
        5.6
    } else if q.contains("Q6") {
        6.6
    } else if q.contains("Q8") {
        8.5
    } else if q.contains("MXFP4") {
        4.25
    } else if q.contains("BF16") || q.contains("F16") {
        16.0
    } else if q.contains("F32") {
        32.0
    } else {
        return None;
    };
    Some(v)
}

/// Estima si un modelo cabe. La KV es una **estimación gruesa** (no se conoce la
/// arquitectura completa antes de descargar) y se dice tal cual.
pub fn evaluar(
    params_b: f64,
    quant: &str,
    ctx: u64,
    ram_libre_gb: f64,
    gpus: Vec<GpuLibre>,
) -> Compatibilidad {
    let Some(bits) = bits_por_peso(quant) else {
        return Compatibilidad {
            veredicto: "DESCONOCIDO".into(),
            pesos_gb: None,
            kv_gb: None,
            margen_gb: None,
            total_gb: None,
            gpus,
            ram_libre_gb,
            cabe_en_una_gpu: None,
            requiere_offload: false,
            explicacion: format!("no sé cuántos bits ocupa «{quant}»"),
        };
    };
    if params_b <= 0.0 {
        return Compatibilidad {
            veredicto: "DESCONOCIDO".into(),
            pesos_gb: None,
            kv_gb: None,
            margen_gb: None,
            total_gb: None,
            gpus,
            ram_libre_gb,
            cabe_en_una_gpu: None,
            requiere_offload: false,
            explicacion: "faltan los parámetros del modelo".into(),
        };
    }

    let pesos = params_b * bits / 8.0;
    // KV: ~0,07 GB por cada 1B de parámetros a 4k de contexto (orden de magnitud
    // de Llama/Qwen con GQA). Es una estimación, no una medida.
    let kv = params_b * 0.07 * (ctx.max(2048) as f64 / 4096.0);
    let extra = 1.5; // buffers, gráfico y margen
    let total = pesos + kv + extra;

    let cabe_en_una = gpus
        .iter()
        .filter(|g| g.free_gb * 0.92 >= total)
        .map(|g| g.index)
        .min();

    if let Some(gpu) = cabe_en_una {
        let (gnombre, glibre) = {
            let g = gpus.iter().find(|x| x.index == gpu).unwrap();
            (g.name.clone(), g.free_gb)
        };
        return Compatibilidad {
            veredicto: "COMPATIBLE".into(),
            pesos_gb: Some(pesos),
            kv_gb: Some(kv),
            margen_gb: Some(glibre - total),
            total_gb: Some(total),
            gpus,
            ram_libre_gb,
            cabe_en_una_gpu: Some(gpu),
            requiere_offload: false,
            explicacion: format!(
                "cabe entero en GPU{gpu} ({gnombre}): {total:.1} GB de {glibre:.1} GB libres"
            ),
        };
    }

    let vram_suma: f64 = gpus.iter().map(|g| g.free_gb).sum();
    if vram_suma * 0.92 >= total {
        return Compatibilidad {
            veredicto: "COMPATIBLE CON LIMITACIONES".into(),
            pesos_gb: Some(pesos),
            kv_gb: Some(kv),
            margen_gb: Some(vram_suma - total),
            total_gb: Some(total),
            gpus,
            ram_libre_gb,
            cabe_en_una_gpu: None,
            requiere_offload: false,
            explicacion: format!(
                "no cabe en una tarjeta, pero suma {vram_suma:.1} GB libres entre las dos. Repartir capas entre GPU es más lento que una sola."
            ),
        };
    }

    // Offload realista: en VRAM entra lo que quepa (92 % de lo libre) y el resto
    // tiene que caber en RAM. No se suman las VRAM como si fueran una sola: se
    // usa la mayor de las dos y el resto va a RAM.
    let mayor_vram = gpus.iter().map(|g| g.free_gb).fold(0.0, f64::max) * 0.92;
    let en_ram = total - mayor_vram;
    if mayor_vram > 0.0 && en_ram > 0.0 && ram_libre_gb * 0.9 >= en_ram {
        return Compatibilidad {
            veredicto: "COMPATIBLE CON LIMITACIONES".into(),
            pesos_gb: Some(pesos),
            kv_gb: Some(kv),
            margen_gb: Some(ram_libre_gb * 0.9 - en_ram),
            total_gb: Some(total),
            gpus,
            ram_libre_gb,
            cabe_en_una_gpu: None,
            requiere_offload: true,
            explicacion: format!(
                "offload: {mayor_vram:.1} GB en la GPU más libre y {en_ram:.1} GB en RAM ({ram_libre_gb:.1} GB libres). Espera bastantes menos tok/s y más latencia de primer token."
            ),
        };
    }

    Compatibilidad {
        veredicto: "NO COMPATIBLE".into(),
        pesos_gb: Some(pesos),
        kv_gb: Some(kv),
        margen_gb: None,
        total_gb: Some(total),
        gpus,
        ram_libre_gb,
        cabe_en_una_gpu: None,
        requiere_offload: false,
        explicacion: format!(
            "necesita ~{total:.1} GB y hay {vram_suma:.1} GB de VRAM y {ram_libre_gb:.1} GB de RAM"
        ),
    }
}

/// Estado actual de VRAM libre por tarjeta y de RAM disponible.
pub fn hardware_actual() -> (f64, Vec<GpuLibre>) {
    let stats = crate::gpu::stats();
    let gpus: Vec<GpuLibre> = stats
        .iter()
        .map(|g| GpuLibre {
            index: g.index,
            name: g.name.clone(),
            total_gb: g.memory_total_mb as f64 / 1024.0,
            free_gb: (g.memory_total_mb.saturating_sub(g.memory_used_mb)) as f64 / 1024.0,
        })
        .collect();
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    let ram = sys.available_memory() as f64 / 1_073_741_824.0;
    (ram, gpus)
}

/* ============================ Descarga Ollama ============================ */

/// Descarga un modelo con `POST /api/pull` y va escribiendo el progreso real en la
/// fila del trabajo. Devuelve un resumen al terminar.
pub async fn pull_ollama(
    db: &std::sync::Arc<std::sync::Mutex<crate::memory::Database>>,
    job_id: i64,
    modelo: &str,
) -> Result<serde_json::Value, String> {
    use futures_util::StreamExt;
    let cuerpo = serde_json::json!({ "model": modelo, "stream": true });
    let resp = cliente(86_400)
        .post(format!("{OLLAMA_LOCAL}/api/pull"))
        .json(&cuerpo)
        .send()
        .await
        .map_err(|e| format!("Ollama no respondió al pull: {e}"))?;
    if !resp.status().is_success() {
        let estado = resp.status();
        let texto = resp.text().await.unwrap_or_default();
        return Err(format!("pull HTTP {estado}: {}", texto.trim()));
    }

    let mut stream = resp.bytes_stream();
    let mut buf = String::new();
    let mut ultimo = String::new();
    let mut capas: usize = 0;

    while let Some(chunk) = stream.next().await {
        let trozo = chunk.map_err(|e| format!("se cortó la descarga: {e}"))?;
        buf.push_str(&String::from_utf8_lossy(&trozo));
        while let Some(pos) = buf.find('\n') {
            let linea: String = buf.drain(..=pos).collect();
            let Ok(v) = serde_json::from_str::<serde_json::Value>(linea.trim()) else {
                continue;
            };
            if let Some(err) = v["error"].as_str() {
                return Err(format!("Ollama: {err}"));
            }
            if let Some(estado) = v["status"].as_str() {
                ultimo = estado.to_string();
            }
            if let (Some(done), Some(tot)) = (v["completed"].as_u64(), v["total"].as_u64()) {
                if tot > 0 {
                    let p = done as f64 / tot as f64;
                    let g = std::sync::Mutex::new(p);
                    let _ = g; // el candado lo toma `jobs` en cada escritura
                    if let Ok(guard) = db.lock() {
                        let _ = crate::jobs::set_progress(&guard, job_id, Some(p));
                    }
                }
            }
            if ultimo.starts_with("pulling") && v["total"].is_u64() {
                capas = capas.max(1);
            }
        }
    }

    if let Ok(guard) = db.lock() {
        let _ = crate::jobs::set_progress(&guard, job_id, Some(1.0));
    }
    Ok(serde_json::json!({
        "modelo": modelo,
        "estado_final": ultimo,
        "capas": capas,
        "nota": "descarga hecha por la API oficial de Ollama",
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_cuantizacion_manda_en_los_bits() {
        assert!(bits_por_peso("Q4_K_M").unwrap() < bits_por_peso("Q8_0").unwrap());
        assert_eq!(bits_por_peso("F16"), Some(16.0));
        assert!(bits_por_peso("inventada").is_none());
    }

    #[test]
    fn un_8b_q4_cabe_en_la_tarjeta_grande() {
        let gpus = vec![
            GpuLibre {
                index: 0,
                name: "Tarjeta A".into(),
                total_gb: 12.0,
                free_gb: 8.0,
            },
            GpuLibre {
                index: 1,
                name: "Tarjeta B".into(),
                total_gb: 16.0,
                free_gb: 15.0,
            },
        ];
        let c = evaluar(8.0, "Q4_K_M", 8192, 40.0, gpus);
        assert_eq!(c.veredicto, "COMPATIBLE");
        assert_eq!(c.cabe_en_una_gpu, Some(1));
    }

    #[test]
    fn un_70b_no_cabe_y_lo_dice() {
        let gpus = vec![
            GpuLibre {
                index: 0,
                name: "Tarjeta A".into(),
                total_gb: 12.0,
                free_gb: 10.0,
            },
            GpuLibre {
                index: 1,
                name: "Tarjeta B".into(),
                total_gb: 16.0,
                free_gb: 15.0,
            },
        ];
        let c = evaluar(70.0, "Q4_K_M", 8192, 20.0, gpus);
        assert_eq!(c.veredicto, "NO COMPATIBLE");
        assert!(c.explicacion.contains("VRAM"));
    }

    #[test]
    fn sin_params_el_veredicto_es_desconocido() {
        let c = evaluar(0.0, "Q4_K_M", 4096, 30.0, vec![]);
        assert_eq!(c.veredicto, "DESCONOCIDO");
    }

    #[test]
    fn la_cabecera_gguf_se_lee_de_verdad() {
        // Cabecera GGUF mínima con general.architecture = "llama".
        let mut b: Vec<u8> = Vec::new();
        b.extend_from_slice(b"GGUF");
        b.extend_from_slice(&3u32.to_le_bytes()); // versión
        b.extend_from_slice(&0u64.to_le_bytes()); // tensores
        b.extend_from_slice(&1u64.to_le_bytes()); // 1 clave
        let clave = b"general.architecture";
        b.extend_from_slice(&(clave.len() as u64).to_le_bytes());
        b.extend_from_slice(clave);
        b.extend_from_slice(&8u32.to_le_bytes()); // tipo string
        let valor = b"llama";
        b.extend_from_slice(&(valor.len() as u64).to_le_bytes());
        b.extend_from_slice(valor);

        let p = std::env::temp_dir().join(format!("kukia_hdr_{}.gguf", std::process::id()));
        std::fs::write(&p, &b).unwrap();
        let (version, arch, _n) = leer_cabecera(&p);
        let _ = std::fs::remove_file(&p);
        assert_eq!(version, Some(3));
        assert_eq!(arch.as_deref(), Some("llama"));
    }

    #[test]
    fn un_archivo_que_no_es_gguf_no_se_confunde() {
        let p = std::env::temp_dir().join(format!("kukia_nog_gguf_{}.gguf", std::process::id()));
        std::fs::write(&p, b"NOGGUF....").unwrap();
        let (version, arch, _n) = leer_cabecera(&p);
        let _ = std::fs::remove_file(&p);
        assert!(version.is_none() && arch.is_none());
    }
}
