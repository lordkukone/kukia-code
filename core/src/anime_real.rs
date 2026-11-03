//! KUKIA ANIME ↔ REAL STUDIO.
//!
//! Cliente real de ComfyUI: no simula nada. Cada plantilla es un workflow en formato
//! API que se envía a `/prompt`, se sigue por `/history` y se descarga por `/view`.
//!
//! Reglas:
//! - Las plantillas y lo que requieren viven en `workflows/anime_real/` (manifest.json).
//! - Antes de ofrecer una conversión se comprueba que ComfyUI ve **todos** sus modelos.
//!   Si falta alguno, la capacidad sale `disponible: false` con la lista exacta de lo que falta.
//! - No se instala, no se descarga y no se modifica nada de ComfyUI: solo se encola.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::time::Duration;

pub const COMFY_DEFAULT: &str = "http://127.0.0.1:8188";
/// Tope de espera por conversión (las de vídeo pueden tardar mucho más).
pub const TIMEOUT_CONVERSION_S: u64 = 1800;

/* ============================ Rutas ============================ */

pub fn studio_dir() -> PathBuf {
    if let Ok(p) = std::env::var("KUKIA_ANIME_DIR") {
        return PathBuf::from(p);
    }
    PathBuf::from(r"H:\KUKIA_CODE\workflows\anime_real")
}

pub fn output_dir() -> PathBuf {
    if let Ok(p) = std::env::var("KUKIA_ANIME_OUT") {
        return PathBuf::from(p);
    }
    PathBuf::from(r"H:\KUKIA_CODE\output\anime_real")
}

fn cliente(segundos: u64) -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(segundos))
        .user_agent("KUKIA-CODE/anime-real")
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

/* ============================ Plantillas ============================ */

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Nodes {
    #[serde(default)]
    pub imagen: String,
    #[serde(default)]
    pub positivo: String,
    #[serde(default)]
    pub negativo: String,
    #[serde(default)]
    pub muestreo: String,
    #[serde(default)]
    pub escalado: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub id: String,
    pub titulo: String,
    /// `anime2real` o `real2anime`
    pub direccion: String,
    /// `imagen` o `video`
    pub tipo: String,
    pub motor: String,
    pub archivo: String,
    #[serde(default)]
    pub verificado: String,
    #[serde(default)]
    pub nodos: Nodes,
    #[serde(default)]
    pub requiere: Vec<String>,
    #[serde(default)]
    pub segundos_estimados: u64,
    #[serde(default)]
    pub nota: String,
    /// `listo` | `pendiente_conversion`
    #[serde(default)]
    pub estado: String,
}

#[derive(Debug, Clone, Deserialize)]
struct Manifest {
    #[serde(default)]
    templates: Vec<Template>,
}

pub fn load_templates() -> Result<Vec<Template>, String> {
    let ruta = studio_dir().join("manifest.json");
    let crudo = std::fs::read_to_string(&ruta)
        .map_err(|e| format!("no se pudo leer {}: {e}", ruta.display()))?;
    let m: Manifest =
        serde_json::from_str(&crudo).map_err(|e| format!("manifest.json inválido: {e}"))?;
    Ok(m.templates)
}

fn template_por_id(id: &str) -> Result<Template, String> {
    load_templates()?
        .into_iter()
        .find(|t| t.id == id)
        .ok_or_else(|| format!("plantilla desconocida: {id}"))
}

/* ============================ Estado de ComfyUI ============================ */

#[derive(Debug, Clone, Serialize)]
pub struct ComfyStatus {
    pub online: bool,
    pub base_url: String,
    pub version: String,
    pub device: String,
    pub vram_total_mb: u64,
    pub vram_free_mb: u64,
    pub en_cola: usize,
    pub nota: String,
}

pub async fn status(base: &str) -> ComfyStatus {
    let mut s = ComfyStatus {
        online: false,
        base_url: base.to_string(),
        version: String::new(),
        device: String::new(),
        vram_total_mb: 0,
        vram_free_mb: 0,
        en_cola: 0,
        nota: String::new(),
    };
    match cliente(5).get(format!("{base}/system_stats")).send().await {
        Ok(r) if r.status().is_success() => match r.json::<Value>().await {
            Ok(v) => {
                s.online = true;
                s.version = v["system"]["comfyui_version"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                if let Some(d) = v["devices"].as_array().and_then(|a| a.first()) {
                    s.device = d["name"].as_str().unwrap_or("").to_string();
                    s.vram_total_mb = d["vram_total"].as_u64().unwrap_or(0) / (1024 * 1024);
                    s.vram_free_mb = d["vram_free"].as_u64().unwrap_or(0) / (1024 * 1024);
                }
                s.nota = "ComfyUI responde".into();
            }
            Err(e) => s.nota = format!("respuesta ilegible: {e}"),
        },
        Ok(r) => s.nota = format!("HTTP {}", r.status()),
        Err(_) => s.nota = "sin respuesta en 8188".into(),
    }
    if s.online {
        if let Ok(v) = cliente(5)
            .get(format!("{base}/queue"))
            .send()
            .await
            .and_then(|r| r.error_for_status())
        {
            if let Ok(q) = v.json::<Value>().await {
                let run = q["queue_running"].as_array().map(|a| a.len()).unwrap_or(0);
                let pend = q["queue_pending"].as_array().map(|a| a.len()).unwrap_or(0);
                s.en_cola = run + pend;
            }
        }
    }
    s
}

/// Nombres de modelos que ComfyUI ve ahora mismo (checkpoints, LoRAs, unets, VAEs, clip vision).
///
/// Se pregunta nodo por nodo (`/object_info/<clase>`) en vez de pedir el volcado completo:
/// en esta instalación `/object_info` son 22 MB y el análisis tardaba 32 s, con lo que la
/// vista entera se quedaba esperando. Por nodo son unos pocos KB y responde al instante.
pub async fn modelos_visibles(base: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut fallos = 0usize;

    for (clase, campo) in [
        ("CheckpointLoaderSimple", "ckpt_name"),
        ("LoraLoaderModelOnly", "lora_name"),
        ("LoraLoader", "lora_name"),
        ("UNETLoader", "unet_name"),
        ("UnetLoaderGGUF", "unet_name"),
        ("VAELoader", "vae_name"),
        // Los text encoders también son modelos que faltan o están: sin esto el
        // diagnóstico del convertidor daba por ausente el umt5 que sí está.
        ("CLIPLoader", "clip_name"),
        ("CLIPVisionLoader", "clip_name"),
        ("IPAdapterModelLoader", "ipadapter_file"),
    ] {
        let respuesta = match cliente(15)
            .get(format!("{base}/object_info/{clase}"))
            .send()
            .await
        {
            Ok(r) if r.status().is_success() => r,
            Ok(_) => continue,
            Err(_) => {
                fallos += 1;
                continue;
            }
        };
        let obj: Value = match respuesta.json().await {
            Ok(v) => v,
            Err(_) => {
                fallos += 1;
                continue;
            }
        };
        if let Some(arr) = obj
            .get(clase)
            .and_then(|c| c.get("input"))
            .and_then(|i| i.get("required"))
            .and_then(|r| r.get(campo))
            .and_then(|s| s.get(0))
            .and_then(|a| a.as_array())
        {
            for x in arr {
                if let Some(t) = x.as_str() {
                    out.push(t.to_string());
                }
            }
        }
    }

    // Si no se pudo preguntar a ningún nodo, es que ComfyUI no responde: se propaga
    // el error para que la interfaz diga la verdad en vez de «faltan todos los modelos».
    if out.is_empty() && fallos > 0 {
        return Err(format!("ComfyUI no respondió al listar modelos ({fallos} consultas fallidas)"));
    }

    out.sort();
    out.dedup();
    Ok(out)
}

/// Imágenes disponibles en la carpeta `input` de ComfyUI (para elegir la fuente).
pub async fn imagenes_entrada(base: &str) -> Vec<String> {
    match cliente(30).get(format!("{base}/object_info/LoadImage")).send().await {
        Ok(r) => match r.json::<Value>().await {
            Ok(v) => v
                .get("LoadImage")
                .and_then(|c| c.get("input"))
                .and_then(|i| i.get("required"))
                .and_then(|r| r.get("image"))
                .and_then(|s| s.get(0))
                .and_then(|a| a.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
            Err(_) => Vec::new(),
        },
        Err(_) => Vec::new(),
    }
}

/* ============================ Capacidades ============================ */

#[derive(Debug, Clone, Serialize)]
pub struct Capability {
    pub id: String,
    pub titulo: String,
    pub direccion: String,
    pub tipo: String,
    pub motor: String,
    pub disponible: bool,
    pub faltan: Vec<String>,
    pub verificado: String,
    pub segundos_estimados: u64,
    pub nota: String,
    pub estado: String,
}

pub async fn capabilities(base: &str) -> Vec<Capability> {
    let plantillas = load_templates().unwrap_or_default();
    let visibles = modelos_visibles(base).await.unwrap_or_default();
    plantillas
        .into_iter()
        .map(|t| {
            let faltan: Vec<String> = if visibles.is_empty() {
                t.requiere.clone()
            } else {
                t.requiere
                    .iter()
                    .filter(|m| !visibles.iter().any(|v| v.ends_with(m.as_str())))
                    .cloned()
                    .collect()
            };
            Capability {
                disponible: t.estado != "pendiente_conversion" && faltan.is_empty(),
                id: t.id,
                titulo: t.titulo,
                direccion: t.direccion,
                tipo: t.tipo,
                motor: t.motor,
                faltan,
                verificado: t.verificado,
                segundos_estimados: t.segundos_estimados,
                nota: t.nota,
                estado: t.estado,
            }
        })
        .collect()
}

/* ============================ Conversión ============================ */

#[derive(Debug, Clone, Deserialize)]
pub struct ConvertRequest {
    pub template_id: String,
    pub image: String,
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub negative: String,
    pub denoise: Option<f64>,
    pub seed: Option<i64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConversionResult {
    pub id: String,
    pub prompt_id: String,
    pub template_id: String,
    pub direccion: String,
    pub segundos: f64,
    pub archivos: Vec<String>,
    pub bytes: u64,
    /// Data URL del primer resultado (para la comparación antes/después).
    pub preview: String,
    /// Data URL de la imagen de origen.
    pub entrada_preview: String,
}

fn ahora_id() -> String {
    chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string()
}

/// Aplica los parámetros del usuario sobre el workflow.
fn parchear(wf: &mut Value, t: &Template, req: &ConvertRequest, seed: i64) -> Result<(), String> {
    let n = &t.nodos;
    if !n.imagen.is_empty() {
        if let Some(nodo) = wf.get_mut(&n.imagen) {
            nodo["inputs"]["image"] = json!(req.image);
        }
    }
    if !req.prompt.trim().is_empty() && !n.positivo.is_empty() {
        if let Some(nodo) = wf.get_mut(&n.positivo) {
            nodo["inputs"]["text"] = json!(req.prompt);
        }
    }
    if !req.negative.trim().is_empty() && !n.negativo.is_empty() {
        if let Some(nodo) = wf.get_mut(&n.negativo) {
            nodo["inputs"]["text"] = json!(req.negative);
        }
    }
    if !n.muestreo.is_empty() {
        if let Some(nodo) = wf.get_mut(&n.muestreo) {
            nodo["inputs"]["seed"] = json!(seed);
            if let Some(d) = req.denoise {
                nodo["inputs"]["denoise"] = json!(d.clamp(0.05, 1.0));
            }
        }
    }
    if !n.escalado.is_empty() {
        if let Some(nodo) = wf.get_mut(&n.escalado) {
            if let Some(w) = req.width {
                nodo["inputs"]["width"] = json!(w);
            }
            if let Some(h) = req.height {
                nodo["inputs"]["height"] = json!(h);
            }
        }
    }
    Ok(())
}

pub fn b64(datos: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((datos.len() + 2) / 3 * 4);
    for trozo in datos.chunks(3) {
        let b = [
            trozo[0],
            *trozo.get(1).unwrap_or(&0),
            *trozo.get(2).unwrap_or(&0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if trozo.len() > 1 {
            T[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if trozo.len() > 2 {
            T[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

fn data_url(bytes: &[u8], nombre: &str) -> String {
    let mime = if nombre.to_lowercase().ends_with(".png") {
        "image/png"
    } else if nombre.to_lowercase().ends_with(".webp") {
        "image/webp"
    } else {
        "image/jpeg"
    };
    format!("data:{mime};base64,{}", b64(bytes))
}

/// Descarga una imagen del servidor ComfyUI.
async fn descargar(base: &str, img: &Value) -> Result<(String, Vec<u8>), String> {
    let nombre = img["filename"].as_str().unwrap_or("salida.png").to_string();
    let tipo = img["type"].as_str().unwrap_or("output");
    let sub = img["subfolder"].as_str().unwrap_or("");
    let url = format!(
        "{base}/view?filename={}&type={}&subfolder={}",
        urlencoding(&nombre),
        urlencoding(tipo),
        urlencoding(sub)
    );
    let r = cliente(120)
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("descarga: {e}"))?;
    let bytes = r
        .bytes()
        .await
        .map_err(|e| format!("descarga ilegible: {e}"))?
        .to_vec();
    Ok((nombre, bytes))
}

fn urlencoding(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Ejecuta una conversión completa contra ComfyUI.
pub async fn convertir(base: &str, req: &ConvertRequest) -> Result<ConversionResult, String> {
    let t = template_por_id(&req.template_id)?;
    if t.estado == "pendiente_conversion" {
        return Err(format!(
            "{} todavía no está convertido a formato API: ábrelo en ComfyUI",
            t.titulo
        ));
    }
    let ruta = studio_dir().join(&t.archivo);
    let crudo = std::fs::read_to_string(&ruta)
        .map_err(|e| format!("no se pudo leer {}: {e}", ruta.display()))?;
    let mut wf: Value =
        serde_json::from_str(&crudo).map_err(|e| format!("workflow inválido: {e}"))?;

    let seed = req
        .seed
        .unwrap_or_else(|| (chrono::Utc::now().timestamp_millis() % 1_000_000_000) as i64);
    parchear(&mut wf, &t, req, seed)?;

    let inicio = std::time::Instant::now();
    let resp: Value = cliente(30)
        .post(format!("{base}/prompt"))
        .json(&json!({ "prompt": wf, "client_id": "kukia-code-studio" }))
        .send()
        .await
        .map_err(|e| format!("no se pudo encolar: {e}"))?
        .json()
        .await
        .map_err(|e| format!("respuesta de /prompt ilegible: {e}"))?;

    let prompt_id = resp["prompt_id"]
        .as_str()
        .ok_or_else(|| {
            format!(
                "ComfyUI rechazó el workflow: {}",
                serde_json::to_string(&resp).unwrap_or_default()
            )
        })?
        .to_string();

    // Sondeo del historial.
    let h: Value = loop {
        if inicio.elapsed().as_secs() > TIMEOUT_CONVERSION_S {
            return Err(format!(
                "tiempo agotado tras {TIMEOUT_CONVERSION_S}s (prompt {prompt_id} sigue en ComfyUI)"
            ));
        }
        tokio::time::sleep(Duration::from_millis(1500)).await;
        let v: Value = cliente(15)
            .get(format!("{base}/history/{prompt_id}"))
            .send()
            .await
            .map_err(|e| format!("history: {e}"))?
            .json()
            .await
            .map_err(|e| format!("history ilegible: {e}"))?;
        if let Some(h) = v.get(&prompt_id) {
            let completado = h["status"]["completed"].as_bool().unwrap_or(false);
            let estado = h["status"]["status_str"].as_str().unwrap_or("");
            if estado == "error" {
                return Err(format!(
                    "ComfyUI falló ejecutando el workflow: {}",
                    serde_json::to_string(&h["status"]).unwrap_or_default()
                ));
            }
            if completado {
                break h.clone();
            }
        }
    };
    let segundos = inicio.elapsed().as_secs_f64();

    // Descargar resultados.
    let vacio = Vec::new();
    let salidas: Vec<&Value> = h["outputs"]
        .as_object()
        .map(|o| o.values().collect())
        .unwrap_or_default();
    let mut archivos = Vec::new();
    let mut total = 0u64;
    let mut preview = String::new();

    std::fs::create_dir_all(output_dir()).map_err(|e| e.to_string())?;
    let id = ahora_id();
    for out in salidas {
        let imgs = out
            .get("images")
            .and_then(|i| i.as_array())
            .unwrap_or(&vacio);
        for img in imgs {
            let (nombre, bytes) = descargar(base, img).await?;
            let destino = output_dir().join(format!("{id}_{nombre}"));
            std::fs::write(&destino, &bytes).map_err(|e| e.to_string())?;
            total += bytes.len() as u64;
            if preview.is_empty() {
                preview = data_url(&bytes, &nombre);
            }
            archivos.push(destino.display().to_string());
        }
    }
    if archivos.is_empty() {
        return Err("el workflow terminó sin imágenes".to_string());
    }

    // Vista previa de la entrada.
    let mut entrada_preview = String::new();
    let url_in = format!(
        "{base}/view?filename={}&type=input&subfolder=",
        urlencoding(&req.image)
    );
    if let Ok(r) = cliente(30).get(&url_in).send().await {
        if r.status().is_success() {
            if let Ok(b) = r.bytes().await {
                entrada_preview = data_url(&b, &req.image);
            }
        }
    }

    // Historial en disco (una línea JSON por trabajo).
    let linea = json!({
        "id": id,
        "fecha": chrono::Utc::now().to_rfc3339(),
        "plantilla": t.id,
        "direccion": t.direccion,
        "entrada": req.image,
        "segundos": segundos,
        "archivos": archivos,
    });
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(output_dir().join("historial.jsonl"))
    {
        use std::io::Write;
        let _ = writeln!(f, "{linea}");
    }

    Ok(ConversionResult {
        id,
        prompt_id,
        template_id: t.id,
        direccion: t.direccion,
        segundos,
        archivos,
        bytes: total,
        preview,
        entrada_preview,
    })
}

/// Historial de trabajos, del más reciente al más antiguo.
pub fn historial(limite: usize) -> Vec<Value> {
    let Ok(crudo) = std::fs::read_to_string(output_dir().join("historial.jsonl")) else {
        return Vec::new();
    };
    let mut v: Vec<Value> = crudo
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .collect();
    v.reverse();
    v.truncate(limite);
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn las_plantillas_se_cargan_y_estan_completas() {
        let t = load_templates().expect("manifest legible");
        assert!(!t.is_empty(), "debe haber plantillas");
        for p in &t {
            assert!(!p.id.is_empty());
            assert!(!p.archivo.is_empty());
            if p.estado != "pendiente_conversion" {
                let ruta = studio_dir().join(&p.archivo);
                assert!(ruta.is_file(), "falta el workflow {}", ruta.display());
                let wf: Value =
                    serde_json::from_str(&std::fs::read_to_string(&ruta).unwrap()).unwrap();
                assert!(wf.is_object(), "{} no es un workflow API", p.archivo);
                // Todos los nodos que la app parchea deben existir de verdad.
                for nid in [
                    &p.nodos.imagen,
                    &p.nodos.positivo,
                    &p.nodos.negativo,
                    &p.nodos.muestreo,
                    &p.nodos.escalado,
                ] {
                    if !nid.is_empty() {
                        assert!(
                            wf.get(nid).is_some(),
                            "{}: el nodo {nid} no existe en {}",
                            p.id,
                            p.archivo
                        );
                    }
                }
            }
        }
        println!("plantillas: {}", t.len());
    }

    #[test]
    fn el_parcheo_cambia_solo_lo_que_debe() {
        let t = Template {
            id: "x".into(),
            titulo: "x".into(),
            direccion: "anime2real".into(),
            tipo: "imagen".into(),
            motor: "comfyui".into(),
            archivo: "x.json".into(),
            verificado: String::new(),
            nodos: Nodes {
                imagen: "2".into(),
                positivo: "5".into(),
                negativo: "6".into(),
                muestreo: "7".into(),
                escalado: "3".into(),
            },
            requiere: vec![],
            segundos_estimados: 0,
            nota: String::new(),
            estado: "listo".into(),
        };
        let mut wf = json!({
            "1": {"class_type":"CheckpointLoaderSimple","inputs":{"ckpt_name":"a.safetensors"}},
            "2": {"class_type":"LoadImage","inputs":{"image":"orig.png"}},
            "3": {"class_type":"ImageScale","inputs":{"width":1024,"height":1024}},
            "5": {"class_type":"CLIPTextEncode","inputs":{"text":"viejo"}},
            "6": {"class_type":"CLIPTextEncode","inputs":{"text":"neg"}},
            "7": {"class_type":"KSampler","inputs":{"seed":1,"denoise":0.5}}
        });
        let req = ConvertRequest {
            template_id: "x".into(),
            image: "nueva.png".into(),
            prompt: "retrato realista".into(),
            negative: String::new(),
            denoise: Some(0.65),
            seed: Some(777),
            width: Some(832),
            height: Some(1216),
        };
        parchear(&mut wf, &t, &req, 777).unwrap();
        assert_eq!(wf["2"]["inputs"]["image"], "nueva.png");
        assert_eq!(wf["5"]["inputs"]["text"], "retrato realista");
        assert_eq!(wf["6"]["inputs"]["text"], "neg", "negativo vacío no debe pisarse");
        assert_eq!(wf["7"]["inputs"]["seed"], 777);
        assert_eq!(wf["7"]["inputs"]["denoise"], 0.65);
        assert_eq!(wf["3"]["inputs"]["width"], 832);
        assert_eq!(wf["1"]["inputs"]["ckpt_name"], "a.safetensors", "no se toca el modelo");
    }

    #[test]
    fn el_denoise_se_acota() {
        let t = Template {
            id: "x".into(),
            titulo: "x".into(),
            direccion: "anime2real".into(),
            tipo: "imagen".into(),
            motor: "comfyui".into(),
            archivo: "x.json".into(),
            verificado: String::new(),
            nodos: Nodes {
                muestreo: "7".into(),
                ..Default::default()
            },
            requiere: vec![],
            segundos_estimados: 0,
            nota: String::new(),
            estado: "listo".into(),
        };
        let mut wf = json!({"7": {"inputs": {"denoise": 0.5}}});
        let req = ConvertRequest {
            template_id: "x".into(),
            image: "a.png".into(),
            prompt: String::new(),
            negative: String::new(),
            denoise: Some(9.0),
            seed: None,
            width: None,
            height: None,
        };
        parchear(&mut wf, &t, &req, 1).unwrap();
        assert_eq!(wf["7"]["inputs"]["denoise"], 1.0);
    }

    #[test]
    fn base64_y_data_url() {
        assert_eq!(b64(b"hola"), "aG9sYQ==");
        assert_eq!(b64(b"a"), "YQ==");
        assert_eq!(b64(b"ab"), "YWI=");
        assert!(data_url(b"x", "f.png").starts_with("data:image/png;base64,"));
    }
}
