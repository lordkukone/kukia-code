//! Entrenador de LoRA (LA8 — Character LoRA Studio).
//!
//! Un proyecto guarda un personaje o estilo, su dataset (imágenes + caption) y sus
//! entrenamientos. KUKIA **no** reimplementa el entrenamiento: lanza el script
//! `trainer/entrenar_lora.py` con PyTorch + diffusers + peft y sigue su progreso
//! por líneas JSON, de modo que el progreso es un dato medido y no una estimación.
//!
//! Reglas que se respetan aquí:
//!
//! - Nada de descargas automáticas: el modelo base lo pone el usuario.
//! - Nada se declara entrenado sin que el script diga `fin` y exista el fichero.
//! - Si no hay ninguna imagen con caption, no se lanza: se dice por qué.
//! - El proceso se puede cancelar; se mata **ese** pid, no por nombre.

use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

use crate::memory::Database;

fn ahora() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[derive(Debug, Clone, Serialize)]
pub struct Proyecto {
    pub id: String,
    pub nombre: String,
    pub clase: String,
    pub modelo_base: String,
    pub notas: String,
    pub creado: String,
    /// Imágenes del dataset y cuántas tienen caption.
    pub imagenes: usize,
    pub con_caption: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Imagen {
    pub id: i64,
    pub proyecto: String,
    pub archivo: String,
    pub caption: String,
    pub ancho: u32,
    pub alto: u32,
    pub creado: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Run {
    pub id: i64,
    pub proyecto: String,
    pub estado: String,
    pub paso: u32,
    pub total: u32,
    pub perdida: f64,
    pub salida: String,
    pub log: String,
    pub iniciado: String,
    pub terminado: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Preflight {
    pub proyecto: String,
    pub imagenes: usize,
    pub con_caption: usize,
    pub dataset_dir: String,
    pub vram_total_mb: u64,
    pub vram_libre_mb: u64,
    pub puede: bool,
    pub avisos: Vec<String>,
    pub comprobado: String,
}

/// Lo que se le pide a un entrenamiento.
#[derive(Debug, Clone, Deserialize)]
pub struct RunCfg {
    pub proyecto: String,
    #[serde(default = "pasos_def")]
    pub pasos: u32,
    #[serde(default = "rango_def")]
    pub rango: u32,
    #[serde(default = "res_def")]
    pub resolucion: u32,
    #[serde(default)]
    pub modelo: String,
    /// Prueba de humo: UNet diminuta, sin GPU ni descargas.
    #[serde(default)]
    pub probar: bool,
}

fn pasos_def() -> u32 {
    800
}
fn rango_def() -> u32 {
    8
}
fn res_def() -> u32 {
    1024
}

pub fn init(db: &Database) -> Result<(), String> {
    db.conn()
        .execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS lora_proyectos (
                id TEXT PRIMARY KEY,
                nombre TEXT NOT NULL,
                clase TEXT NOT NULL DEFAULT '',
                modelo_base TEXT NOT NULL DEFAULT '',
                notas TEXT NOT NULL DEFAULT '',
                creado TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS lora_imagenes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                proyecto TEXT NOT NULL,
                archivo TEXT NOT NULL,
                caption TEXT NOT NULL DEFAULT '',
                ancho INTEGER NOT NULL DEFAULT 0,
                alto INTEGER NOT NULL DEFAULT 0,
                creado TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_lora_img ON lora_imagenes(proyecto);
            CREATE TABLE IF NOT EXISTS lora_runs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                proyecto TEXT NOT NULL,
                estado TEXT NOT NULL DEFAULT 'preparando',
                paso INTEGER NOT NULL DEFAULT 0,
                total INTEGER NOT NULL DEFAULT 0,
                perdida REAL NOT NULL DEFAULT 0,
                salida TEXT NOT NULL DEFAULT '',
                log TEXT NOT NULL DEFAULT '',
                iniciado TEXT NOT NULL,
                terminado TEXT NOT NULL DEFAULT ''
            );
            CREATE INDEX IF NOT EXISTS idx_lora_run ON lora_runs(proyecto);
            "#,
        )
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/* ============================ Rutas ============================ */

pub fn dir_proyecto(id: &str) -> PathBuf {
    crate::config::data_dir().join("entrenador").join(id)
}

pub fn dir_dataset(id: &str) -> PathBuf {
    dir_proyecto(id).join("dataset")
}

pub fn dir_salidas(id: &str) -> PathBuf {
    dir_proyecto(id).join("lora")
}

/// Ruta del script de entrenamiento.
///
/// `KUKIA_TRAINER` manda si está; si no, junto al proyecto, como el resto de
/// rutas del estudio (`studio::styles::dir` usa la misma convención).
pub fn script() -> PathBuf {
    if let Ok(p) = std::env::var("KUKIA_TRAINER") {
        return PathBuf::from(p);
    }
    PathBuf::from(r"H:\KUKIA_CODE\trainer\entrenar_lora.py")
}

fn python() -> String {
    std::env::var("KUKIA_PYTHON").unwrap_or_else(|_| "python".to_string())
}

/* ============================ Proyectos ============================ */

pub fn proyecto_crear(
    db: &Database,
    nombre: &str,
    clase: &str,
    modelo_base: &str,
) -> Result<Proyecto, String> {
    let nombre = nombre.trim();
    if nombre.is_empty() {
        return Err("el proyecto necesita un nombre".into());
    }
    init(db)?;
    let id = uuid::Uuid::new_v4().to_string();
    db.conn()
        .execute(
            "INSERT INTO lora_proyectos(id, nombre, clase, modelo_base, notas, creado) \
             VALUES (?1, ?2, ?3, ?4, '', ?5)",
            rusqlite::params![id, nombre, clase.trim(), modelo_base.trim(), ahora()],
        )
        .map_err(|e| e.to_string())?;
    std::fs::create_dir_all(dir_dataset(&id)).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(dir_salidas(&id)).map_err(|e| e.to_string())?;
    let mut p = proyecto(db, &id)?.ok_or_else(|| "el proyecto no se guardó".to_string())?;
    // El nombre se devuelve tal cual se guardó.
    p.nombre = nombre.to_string();
    Ok(p)
}

pub fn proyecto(db: &Database, id: &str) -> Result<Option<Proyecto>, String> {
    let mut st = db
        .conn()
        .prepare(
            "SELECT p.id, p.nombre, p.clase, p.modelo_base, p.notas, p.creado, \
             (SELECT COUNT(*) FROM lora_imagenes i WHERE i.proyecto = p.id), \
             (SELECT COUNT(*) FROM lora_imagenes i WHERE i.proyecto = p.id AND TRIM(i.caption) <> '') \
             FROM lora_proyectos p WHERE p.id = ?1",
        )
        .map_err(|e| e.to_string())?;
    let fila = st
        .query_row(rusqlite::params![id], |r| {
            Ok(Proyecto {
                id: r.get(0)?,
                nombre: r.get(1)?,
                clase: r.get(2)?,
                modelo_base: r.get(3)?,
                notas: r.get(4)?,
                creado: r.get(5)?,
                imagenes: r.get::<_, i64>(6)? as usize,
                con_caption: r.get::<_, i64>(7)? as usize,
            })
        })
        .ok();
    Ok(fila)
}

pub fn proyectos(db: &Database) -> Vec<Proyecto> {
    let Ok(mut st) = db.conn().prepare(
        "SELECT p.id, p.nombre, p.clase, p.modelo_base, p.notas, p.creado, \
         (SELECT COUNT(*) FROM lora_imagenes i WHERE i.proyecto = p.id), \
         (SELECT COUNT(*) FROM lora_imagenes i WHERE i.proyecto = p.id AND TRIM(i.caption) <> '') \
         FROM lora_proyectos p ORDER BY p.creado DESC",
    ) else {
        return Vec::new();
    };
    let filas = st.query_map([], |r| {
        Ok(Proyecto {
            id: r.get(0)?,
            nombre: r.get(1)?,
            clase: r.get(2)?,
            modelo_base: r.get(3)?,
            notas: r.get(4)?,
            creado: r.get(5)?,
            imagenes: r.get::<_, i64>(6)? as usize,
            con_caption: r.get::<_, i64>(7)? as usize,
        })
    });
    match filas {
        Ok(it) => it.filter_map(|r| r.ok()).collect(),
        Err(_) => Vec::new(),
    }
}

/* ============================ Imágenes ============================ */

/// Lee ancho y alto sin dependencias: cabecera IHDR del PNG y SOF del JPEG.
pub fn dimensiones(ruta: &Path) -> Option<(u32, u32)> {
    let datos = std::fs::read(ruta).ok()?;
    // PNG: firma de 8 bytes y luego "IHDR" con ancho y alto big-endian.
    if datos.len() > 24 && datos.starts_with(&[0x89, b'P', b'N', b'G']) {
        let ancho = u32::from_be_bytes([datos[16], datos[17], datos[18], datos[19]]);
        let alto = u32::from_be_bytes([datos[20], datos[21], datos[22], datos[23]]);
        return Some((ancho, alto));
    }
    // JPEG: recorrer segmentos hasta un SOF (0xC0..0xCF, salvo C4/C8/CC).
    if datos.len() > 4 && datos[0] == 0xFF && datos[1] == 0xD8 {
        let mut i = 2usize;
        while i + 9 < datos.len() {
            if datos[i] != 0xFF {
                i += 1;
                continue;
            }
            let marcador = datos[i + 1];
            if (0xC0..=0xCF).contains(&marcador)
                && marcador != 0xC4
                && marcador != 0xC8
                && marcador != 0xCC
            {
                let alto = u16::from_be_bytes([datos[i + 5], datos[i + 6]]) as u32;
                let ancho = u16::from_be_bytes([datos[i + 7], datos[i + 8]]) as u32;
                return Some((ancho, alto));
            }
            let largo = u16::from_be_bytes([datos[i + 2], datos[i + 3]]) as usize;
            if largo < 2 {
                break;
            }
            i += 2 + largo;
        }
    }
    None
}

/// Copia imágenes al dataset del proyecto. No toca los originales.
pub fn importar(db: &Database, proyecto: &str, rutas: &[String]) -> Result<Vec<Imagen>, String> {
    init(db)?;
    if crate::entrenador::proyecto(db, proyecto)?.is_none() {
        return Err("ese proyecto no existe".into());
    }
    let destino = dir_dataset(proyecto);
    std::fs::create_dir_all(&destino).map_err(|e| e.to_string())?;
    let validas = ["png", "jpg", "jpeg", "webp", "bmp"];
    let mut importadas = 0usize;
    let mut omitidas = Vec::new();
    for ruta in rutas {
        let p = PathBuf::from(ruta);
        let ext = p
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if !p.is_file() || !validas.contains(&ext.as_str()) {
            omitidas.push(ruta.clone());
            continue;
        }
        let Some(nombre) = p.file_name() else {
            omitidas.push(ruta.clone());
            continue;
        };
        let archivo = nombre.to_string_lossy().to_string();
        let final_ = destino.join(&archivo);
        if final_ != p {
            std::fs::copy(&p, &final_).map_err(|e| format!("{}: {e}", p.display()))?;
        }
        let (ancho, alto) = dimensiones(&final_).unwrap_or((0, 0));
        db.conn()
            .execute(
                "INSERT INTO lora_imagenes(proyecto, archivo, caption, ancho, alto, creado) \
                 VALUES (?1, ?2, '', ?3, ?4, ?5)",
                rusqlite::params![proyecto, archivo, ancho as i64, alto as i64, ahora()],
            )
            .map_err(|e| e.to_string())?;
        importadas += 1;
    }
    if importadas == 0 {
        return Err(format!(
            "no se importó ninguna imagen válida ({} ruta(s) omitidas)",
            omitidas.len()
        ));
    }
    Ok(imagenes(db, proyecto))
}

/// Importa todas las imágenes de una carpeta del disco.
pub fn importar_carpeta(db: &Database, proyecto: &str, carpeta: &str) -> Result<Vec<Imagen>, String> {
    let dir = PathBuf::from(carpeta);
    if !dir.is_dir() {
        return Err(format!("no es una carpeta: {carpeta}"));
    }
    let mut rutas = Vec::new();
    for e in std::fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let Ok(e) = e else { continue };
        let p = e.path();
        if p.is_file() {
            rutas.push(p.display().to_string());
        }
    }
    if rutas.is_empty() {
        return Err("la carpeta no tiene ficheros".into());
    }
    importar(db, proyecto, &rutas)
}

pub fn imagenes(db: &Database, proyecto: &str) -> Vec<Imagen> {
    let Ok(mut st) = db.conn().prepare(
        "SELECT id, proyecto, archivo, caption, ancho, alto, creado FROM lora_imagenes \
         WHERE proyecto = ?1 ORDER BY id",
    ) else {
        return Vec::new();
    };
    let filas = st.query_map(rusqlite::params![proyecto], |r| {
        Ok(Imagen {
            id: r.get(0)?,
            proyecto: r.get(1)?,
            archivo: r.get(2)?,
            caption: r.get(3)?,
            ancho: r.get::<_, i64>(4)? as u32,
            alto: r.get::<_, i64>(5)? as u32,
            creado: r.get(6)?,
        })
    });
    match filas {
        Ok(it) => it.filter_map(|r| r.ok()).collect(),
        Err(_) => Vec::new(),
    }
}

pub fn caption(db: &Database, id: i64, texto: &str) -> Result<(), String> {
    let n = db
        .conn()
        .execute(
            "UPDATE lora_imagenes SET caption = ?1 WHERE id = ?2",
            rusqlite::params![texto.trim(), id],
        )
        .map_err(|e| e.to_string())?;
    if n == 0 {
        return Err(format!("no hay imagen con id {id}"));
    }
    Ok(())
}

/// Guarda un caption solo si la imagen existe; devuelve el texto aplicado.
pub fn caption_de(db: &Database, id: i64) -> Option<String> {
    db.conn()
        .query_row(
            "SELECT caption FROM lora_imagenes WHERE id = ?1",
            rusqlite::params![id],
            |r| r.get::<_, String>(0),
        )
        .ok()
}

/// Escribe los `.txt` junto a cada imagen. Devuelve (imágenes, caption escritos).
pub fn dataset_exportar(db: &Database, proyecto: &str) -> Result<(usize, usize), String> {
    let destino = dir_dataset(proyecto);
    std::fs::create_dir_all(&destino).map_err(|e| e.to_string())?;
    let lista = imagenes(db, proyecto);
    let mut con_caption = 0usize;
    for img in &lista {
        let ruta = destino.join(&img.archivo);
        if !ruta.exists() {
            continue;
        }
        let txt = destino.join(Path::new(&img.archivo).with_extension("txt"));
        if img.caption.trim().is_empty() {
            // Un .txt vacío confundiría al script: se borra si quedó de antes.
            let _ = std::fs::remove_file(&txt);
            continue;
        }
        std::fs::write(&txt, format!("{}\n", img.caption.trim())).map_err(|e| e.to_string())?;
        con_caption += 1;
    }
    Ok((lista.len(), con_caption))
}

/* ============================ Captions ============================ */

/// Lo que se le pide al modelo de visión. Corto y en una línea: un caption de
/// entrenamiento no debe ser un ensayo, porque se repite en cada paso.
const PROMPT_CAPTION: &str = "Describe esta imagen para entrenar un LoRA. Responde en una sola \
frase en español, sin markdown y sin adornos: quién o qué se ve, ropa o detalles \
característicos, pose o acción, encuadre, fondo y luz. No inventes lo que no se ve.";

/// Base64 sin dependencias: solo lo justo para mandar una imagen a Ollama.
fn base64(datos: &[u8]) -> String {
    const TABLA: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut salida = String::with_capacity(datos.len().div_ceil(3) * 4);
    for trozo in datos.chunks(3) {
        let b0 = trozo[0] as u32;
        let b1 = *trozo.get(1).unwrap_or(&0) as u32;
        let b2 = *trozo.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        salida.push(TABLA[((n >> 18) & 63) as usize] as char);
        salida.push(TABLA[((n >> 12) & 63) as usize] as char);
        salida.push(if trozo.len() > 1 {
            TABLA[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        salida.push(if trozo.len() > 2 {
            TABLA[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    salida
}

/// Pide a un modelo de visión local (Ollama) un caption para la imagen.
///
/// No guarda nada: devuelve el texto y la interfaz decide si lo acepta. Si Ollama
/// no responde o no hay modelo de visión, se dice el motivo en vez de inventar un
/// caption vacío.
pub async fn caption_auto(ruta: &Path, modelo: &str) -> Result<String, String> {
    if modelo.trim().is_empty() {
        return Err("elige un modelo de visión (por ejemplo qwen3-vl) para describir".into());
    }
    let bytes = std::fs::read(ruta)
        .map_err(|e| format!("no se pudo leer {}: {e}", ruta.display()))?;
    let cliente = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build()
        .map_err(|e| e.to_string())?;
    let cuerpo = serde_json::json!({
        "model": modelo,
        "prompt": PROMPT_CAPTION,
        "images": [base64(&bytes)],
        "stream": false,
        "options": { "temperature": 0.2, "num_predict": 180 }
    });
    let r = cliente
        .post("http://127.0.0.1:11434/api/generate")
        .json(&cuerpo)
        .send()
        .await
        .map_err(|e| format!("Ollama no responde: {e}"))?;
    if !r.status().is_success() {
        return Err(format!("Ollama devolvió HTTP {}", r.status()));
    }
    let v: serde_json::Value = r.json().await.map_err(|e| e.to_string())?;
    let texto = v["response"].as_str().unwrap_or("").trim().to_string();
    if texto.is_empty() {
        return Err("el modelo no devolvió texto".into());
    }
    Ok(texto)
}

/* ============================ Preflight ============================ */

pub fn preflight(db: &Database, proyecto: &str) -> Result<Preflight, String> {
    init(db)?;
    let p = crate::entrenador::proyecto(db, proyecto)?
        .ok_or_else(|| "ese proyecto no existe".to_string())?;
    let tarjetas = crate::gpus::tarjetas();
    let vram_total: u64 = tarjetas.iter().map(|t| t.total_mb).sum();
    let vram_libre: u64 = tarjetas
        .iter()
        .map(|t| t.total_mb.saturating_sub(t.usada_mb.unwrap_or(0)))
        .max()
        .unwrap_or(0);

    let mut avisos = Vec::new();
    if p.imagenes == 0 {
        avisos.push("no hay ninguna imagen en el dataset".to_string());
    }
    if p.con_caption < p.imagenes {
        avisos.push(format!(
            "{} de {} imágenes no tienen caption y se omitirán",
            p.imagenes - p.con_caption,
            p.imagenes
        ));
    }
    if p.con_caption < 5 && p.con_caption > 0 {
        avisos.push(
            "menos de 5 imágenes con caption: el LoRA saldrá muy pegado a esas fotos".to_string(),
        );
    }
    if p.modelo_base.trim().is_empty() {
        avisos.push("falta el modelo base (ruta al .safetensors o repo de Hugging Face)".to_string());
    }
    if tarjetas.is_empty() {
        avisos.push("no se detectó ninguna GPU: se entrenaría en CPU, muy lento".to_string());
    } else if vram_libre < 8000 {
        avisos.push(format!(
            "la tarjeta con más VRAM libre tiene {:.1} GB: por debajo de 8 GB conviene bajar resolución o rango",
            vram_libre as f64 / 1024.0
        ));
    }
    let puede = p.con_caption > 0 && !p.modelo_base.trim().is_empty();

    Ok(Preflight {
        proyecto: p.id,
        imagenes: p.imagenes,
        con_caption: p.con_caption,
        dataset_dir: dir_dataset(proyecto).display().to_string(),
        vram_total_mb: vram_total,
        vram_libre_mb: vram_libre,
        puede,
        avisos,
        comprobado: ahora(),
    })
}

/* ============================ Entrenamiento ============================ */

fn hijos() -> &'static Mutex<std::collections::HashMap<i64, Child>> {
    static HIJOS: std::sync::OnceLock<Mutex<std::collections::HashMap<i64, Child>>> =
        std::sync::OnceLock::new();
    HIJOS.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

/// Un evento del script, ya interpretado.
#[derive(Debug, Clone, PartialEq)]
pub enum Evento {
    Inicio { total: u32, imagenes: Option<u32>, dispositivo: Option<String> },
    Paso { paso: u32, total: u32, perdida: f64, seg: Option<f64> },
    Guardado { ruta: String, claves: u32 },
    Aviso(String),
    Fin,
    Error(String),
    Otro(String),
}

/// Traduce una línea del script. Las que no son JSON se guardan como log.
pub fn leer_evento(linea: &str) -> Evento {
    let texto = linea.trim();
    if !texto.starts_with('{') {
        return Evento::Otro(texto.to_string());
    }
    let Ok(v) = serde_json::from_str::<serde_json::Value>(texto) else {
        return Evento::Otro(texto.to_string());
    };
    match v.get("evento").and_then(|e| e.as_str()).unwrap_or("") {
        "inicio" => Evento::Inicio {
            total: v.get("total").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
            imagenes: v.get("imagenes").and_then(|x| x.as_u64()).map(|x| x as u32),
            dispositivo: v
                .get("dispositivo")
                .and_then(|x| x.as_str())
                .map(|s| s.to_string()),
        },
        "paso" => Evento::Paso {
            paso: v.get("paso").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
            total: v.get("total").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
            perdida: v.get("perdida").and_then(|x| x.as_f64()).unwrap_or(0.0),
            seg: v.get("seg").and_then(|x| x.as_f64()),
        },
        "guardado" => Evento::Guardado {
            ruta: v
                .get("ruta")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            claves: v.get("claves").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
        },
        "fin" => Evento::Fin,
        "error" => Evento::Error(
            v.get("mensaje")
                .and_then(|x| x.as_str())
                .unwrap_or("error sin mensaje")
                .to_string(),
        ),
        "aviso" => Evento::Aviso(
            v.get("mensaje")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
        ),
        _ => Evento::Otro(texto.to_string()),
    }
}

/// Lanza un entrenamiento y devuelve el registro del run (ya con su id).
pub fn entrenar(db: Arc<Mutex<Database>>, cfg: &RunCfg) -> Result<Run, String> {
    let (dataset, salidas, total_imagenes, con_caption, modelo) = {
        let g = db.lock().unwrap_or_else(|e| e.into_inner());
        init(&g)?;
        let p = proyecto(&g, &cfg.proyecto)?.ok_or_else(|| "ese proyecto no existe".to_string())?;
        let modelo = if cfg.modelo.trim().is_empty() {
            p.modelo_base.clone()
        } else {
            cfg.modelo.clone()
        };
        if con_caption_de(&g, &cfg.proyecto) == 0 {
            return Err("no hay ninguna imagen con caption: no se lanza el entrenamiento".into());
        }
        if !cfg.probar && modelo.trim().is_empty() {
            return Err("falta el modelo base: sin él no hay nada que entrenar".into());
        }
        let (_, _) = dataset_exportar(&g, &cfg.proyecto)?;
        (
            dir_dataset(&cfg.proyecto),
            dir_salidas(&cfg.proyecto),
            p.imagenes,
            p.con_caption,
            modelo,
        )
    };

    std::fs::create_dir_all(&salidas).map_err(|e| e.to_string())?;
    let _ = (total_imagenes, con_caption);

    let run_id = {
        let g = db.lock().unwrap_or_else(|e| e.into_inner());
        g.conn()
            .execute(
                "INSERT INTO lora_runs(proyecto, estado, paso, total, perdida, salida, log, iniciado, terminado) \
                 VALUES (?1, 'iniciando', 0, ?2, 0, '', '', ?3, '')",
                rusqlite::params![cfg.proyecto, cfg.pasos as i64, ahora()],
            )
            .map_err(|e| e.to_string())?;
        g.conn().last_insert_rowid()
    };

    let mut cmd = Command::new(python());
    cmd.arg(script())
        .arg("--dataset")
        .arg(&dataset)
        .arg("--salida")
        .arg(&salidas)
        .arg("--pasos")
        .arg(cfg.pasos.to_string())
        .arg("--rango")
        .arg(cfg.rango.to_string())
        .arg("--resolucion")
        .arg(cfg.resolucion.to_string())
        .current_dir(crate::config::data_dir());
    if cfg.probar {
        cmd.arg("--probar");
    } else {
        cmd.arg("--modelo").arg(&modelo);
    }
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }

    let mut hijo = cmd
        .spawn()
        .map_err(|e| format!("no se pudo lanzar python: {e}"))?;
    let stdout = hijo.stdout.take();
    let stderr = hijo.stderr.take();
    let pid = hijo.id();

    if let Some(out) = stdout {
        let db_hilo = db.clone();
        std::thread::spawn(move || {
            let mut exito = false;
            for linea in BufReader::new(out).lines().map_while(Result::ok) {
                let guard = db_hilo.lock().unwrap_or_else(|e| e.into_inner());
                match leer_evento(&linea) {
                    Evento::Inicio { total, .. } => {
                        let _ = guard.conn().execute(
                            "UPDATE lora_runs SET estado = 'entrenando', total = ?1 WHERE id = ?2",
                            rusqlite::params![total as i64, run_id],
                        );
                    }
                    Evento::Paso {
                        paso,
                        total,
                        perdida,
                        ..
                    } => {
                        let _ = guard.conn().execute(
                            "UPDATE lora_runs SET paso = ?1, total = ?2, perdida = ?3 WHERE id = ?4",
                            rusqlite::params![paso as i64, total as i64, perdida, run_id],
                        );
                    }
                    Evento::Guardado { ruta, .. } => {
                        let _ = guard.conn().execute(
                            "UPDATE lora_runs SET salida = ?1 WHERE id = ?2",
                            rusqlite::params![ruta, run_id],
                        );
                    }
                    Evento::Fin => exito = true,
                    Evento::Error(m) => {
                        let _ = guard.conn().execute(
                            "UPDATE lora_runs SET estado = 'error', log = ?1, terminado = ?2 WHERE id = ?3",
                            rusqlite::params![m, ahora(), run_id],
                        );
                    }
                    Evento::Aviso(m) | Evento::Otro(m) => {
                        let _ = guard.conn().execute(
                            "UPDATE lora_runs SET log = TRIM(log || char(10) || ?1) WHERE id = ?2",
                            rusqlite::params![m, run_id],
                        );
                    }
                }
            }
            // Sin `fin` explícito no se declara terminado: puede haber muerto.
            let guard = db_hilo.lock().unwrap_or_else(|e| e.into_inner());
            let estado_actual: String = guard
                .conn()
                .query_row(
                    "SELECT estado FROM lora_runs WHERE id = ?1",
                    rusqlite::params![run_id],
                    |r| r.get(0),
                )
                .unwrap_or_default();
            if estado_actual != "error" {
                let estado = if exito { "terminado" } else { "interrumpido" };
                let _ = guard.conn().execute(
                    "UPDATE lora_runs SET estado = ?1, terminado = ?2 WHERE id = ?3",
                    rusqlite::params![estado, ahora(), run_id],
                );
            }
        });
    }

    if let Some(err) = stderr {
        let db_hilo = db.clone();
        std::thread::spawn(move || {
            for linea in BufReader::new(err).lines().map_while(Result::ok) {
                let guard = db_hilo.lock().unwrap_or_else(|e| e.into_inner());
                let _ = guard.conn().execute(
                    "UPDATE lora_runs SET log = TRIM(log || char(10) || ?1) WHERE id = ?2",
                    rusqlite::params![linea, run_id],
                );
            }
        });
    }

    hijos()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(run_id, hijo);

    Ok(run(db.clone(), run_id)?.ok_or_else(|| "el run no se guardó".to_string())?
        .con_pid_hint(pid))
}

fn con_caption_de(db: &Database, proyecto: &str) -> usize {
    db.conn()
        .query_row(
            "SELECT COUNT(*) FROM lora_imagenes WHERE proyecto = ?1 AND TRIM(caption) <> ''",
            rusqlite::params![proyecto],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(0) as usize
}

impl Run {
    /// El pid no se guarda en la base (no sirve tras reiniciar); solo se usa para
    /// dejar constancia en el log de que se lanzó algo de verdad.
    fn con_pid_hint(mut self, pid: u32) -> Run {
        if !self.log.contains("pid ") {
            self.log = format!("{}\npid {pid}", self.log.trim_end())
                .trim_start()
                .to_string();
        }
        self
    }
}

pub fn run(db: Arc<Mutex<Database>>, id: i64) -> Result<Option<Run>, String> {
    let g = db.lock().unwrap_or_else(|e| e.into_inner());
    let mut st = g
        .conn()
        .prepare(
            "SELECT id, proyecto, estado, paso, total, perdida, salida, log, iniciado, terminado \
             FROM lora_runs WHERE id = ?1",
        )
        .map_err(|e| e.to_string())?;
    Ok(st
        .query_row(rusqlite::params![id], |r| {
            Ok(Run {
                id: r.get(0)?,
                proyecto: r.get(1)?,
                estado: r.get(2)?,
                paso: r.get::<_, i64>(3)? as u32,
                total: r.get::<_, i64>(4)? as u32,
                perdida: r.get(5)?,
                salida: r.get(6)?,
                log: r.get(7)?,
                iniciado: r.get(8)?,
                terminado: r.get(9)?,
            })
        })
        .ok())
}

pub fn runs(db: &Database, proyecto: &str) -> Vec<Run> {
    let Ok(mut st) = db.conn().prepare(
        "SELECT id, proyecto, estado, paso, total, perdida, salida, log, iniciado, terminado \
         FROM lora_runs WHERE proyecto = ?1 ORDER BY id DESC LIMIT 40",
    ) else {
        return Vec::new();
    };
    let filas = st.query_map(rusqlite::params![proyecto], |r| {
        Ok(Run {
            id: r.get(0)?,
            proyecto: r.get(1)?,
            estado: r.get(2)?,
            paso: r.get::<_, i64>(3)? as u32,
            total: r.get::<_, i64>(4)? as u32,
            perdida: r.get(5)?,
            salida: r.get(6)?,
            log: r.get(7)?,
            iniciado: r.get(8)?,
            terminado: r.get(9)?,
        })
    });
    match filas {
        Ok(it) => it.filter_map(|r| r.ok()).collect(),
        Err(_) => Vec::new(),
    }
}

/// Mata el proceso de **ese** run. No se busca por nombre en todo el sistema.
pub fn cancelar(db: Arc<Mutex<Database>>, id: i64) -> bool {
    let mut mapa = hijos().lock().unwrap_or_else(|e| e.into_inner());
    let matado = match mapa.get_mut(&id) {
        Some(hijo) => {
            let _ = hijo.kill();
            let _ = hijo.wait();
            true
        }
        None => false,
    };
    drop(mapa);
    let g = db.lock().unwrap_or_else(|e| e.into_inner());
    let _ = g.conn().execute(
        "UPDATE lora_runs SET estado = 'cancelado', terminado = ?1 WHERE id = ?2 AND estado <> 'terminado'",
        rusqlite::params![ahora(), id],
    );
    matado
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> (Database, PathBuf) {
        let p = std::env::temp_dir().join(format!("kukia_lora_{}", uuid::Uuid::new_v4()));
        let db = Database::open(&p.join("t.db")).expect("db");
        init(&db).expect("tablas");
        (db, p)
    }

    fn png_1x1(ruta: &Path) {
        // PNG 1x1 transparente, escrito a mano: sin dependencias de imagen.
        let bytes: &[u8] = &[
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13, 0x49, 0x48, 0x44, 0x52,
            0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0, 0x1F, 0x15, 0xC4, 0x89, 0, 0, 0, 10, 0x49,
            0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0, 0, 0, 0, 1, 0, 1, 0, 5, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        std::fs::write(ruta, bytes).expect("png");
    }

    #[test]
    fn proyecto_se_crea_y_se_lista() {
        let (db, dir) = base();
        let p = proyecto_crear(&db, "Mika", "personaje", "H:/modelos/sdxl.safetensors").unwrap();
        assert_eq!(p.nombre, "Mika");
        assert_eq!(p.imagenes, 0);
        let lista = proyectos(&db);
        assert_eq!(lista.len(), 1);
        assert!(dir_dataset(&p.id).is_dir(), "el dataset debe crearse vacío");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn un_proyecto_sin_nombre_no_se_crea() {
        let (db, dir) = base();
        assert!(proyecto_crear(&db, "   ", "personaje", "").is_err());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn importar_copia_y_lee_el_tamano() {
        let (db, dir) = base();
        let p = proyecto_crear(&db, "Mika", "personaje", "m").unwrap();
        let origen = dir.join("mika.png");
        png_1x1(&origen);
        let lista = importar(&db, &p.id, &[origen.display().to_string()]).unwrap();
        assert_eq!(lista.len(), 1);
        assert_eq!(lista[0].ancho, 1);
        assert_eq!(lista[0].alto, 1);
        assert!(dir_dataset(&p.id).join("mika.png").exists(), "se copia, no se mueve");
        assert!(origen.exists(), "el original no se toca");
        let _ = std::fs::remove_dir_all(dir);
        let _ = std::fs::remove_dir_all(dir_proyecto(&p.id));
    }

    #[test]
    fn sin_caption_no_se_exporta_txt_y_con_caption_si() {
        let (db, dir) = base();
        let p = proyecto_crear(&db, "Mika", "personaje", "m").unwrap();
        let origen = dir.join("mika.png");
        png_1x1(&origen);
        let lista = importar(&db, &p.id, &[origen.display().to_string()]).unwrap();
        let (total, con) = dataset_exportar(&db, &p.id).unwrap();
        assert_eq!((total, con), (1, 0));
        assert!(!dir_dataset(&p.id).join("mika.txt").exists());

        caption(&db, lista[0].id, "  una chica de pelo azul  ").unwrap();
        let (_, con) = dataset_exportar(&db, &p.id).unwrap();
        assert_eq!(con, 1);
        let texto = std::fs::read_to_string(dir_dataset(&p.id).join("mika.txt")).unwrap();
        assert_eq!(texto.trim(), "una chica de pelo azul");
        let _ = std::fs::remove_dir_all(dir);
        let _ = std::fs::remove_dir_all(dir_proyecto(&p.id));
    }

    #[test]
    fn preflight_no_dice_que_se_puede_sin_caption_ni_modelo() {
        let (db, dir) = base();
        let p = proyecto_crear(&db, "Mika", "personaje", "").unwrap();
        let pf = preflight(&db, &p.id).unwrap();
        assert!(!pf.puede, "sin caption y sin modelo no se puede entrenar");
        assert!(pf.avisos.iter().any(|a| a.contains("imagen")));
        assert!(pf.avisos.iter().any(|a| a.contains("modelo base")));
        let _ = std::fs::remove_dir_all(dir);
        let _ = std::fs::remove_dir_all(dir_proyecto(&p.id));
    }

    #[test]
    fn el_base64_es_correcto() {
        // Casos conocidos: sin relleno, con uno y con dos `=`.
        assert_eq!(base64(b"M"), "TQ==");
        assert_eq!(base64(b"Ma"), "TWE=");
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64(b"hola"), "aG9sYQ==");
    }

    #[test]
    fn los_eventos_del_script_se_entienden() {
        assert_eq!(
            leer_evento(r#"{"evento":"paso","paso":3,"total":10,"perdida":0.125,"seg":1.5}"#),
            Evento::Paso { paso: 3, total: 10, perdida: 0.125, seg: Some(1.5) }
        );
        assert_eq!(
            leer_evento(r#"{"evento":"guardado","ruta":"C:/x/y.safetensors","claves":48}"#),
            Evento::Guardado { ruta: "C:/x/y.safetensors".into(), claves: 48 }
        );
        assert_eq!(leer_evento(r#"{"evento":"fin","ok":true}"#), Evento::Fin);
        assert_eq!(
            leer_evento(r#"{"evento":"error","mensaje":"sin CUDA"}"#),
            Evento::Error("sin CUDA".into())
        );
        assert!(matches!(leer_evento("Cargando modelo..."), Evento::Otro(_)));
        assert!(matches!(leer_evento("{roto"), Evento::Otro(_)));
    }

    /// De punta a punta: proyecto → imagen → caption → python → fichero LoRA.
    /// Es lento (arranca torch) y necesita Python en el PATH, así que va ignorada.
    #[test]
    #[ignore = "lanza python de verdad: requiere torch instalado"]
    fn entrenamiento_completo_hasta_fichero() {
        let (db, dir) = base();
        let p = proyecto_crear(&db, "Prueba", "personaje", "modelo-de-prueba").unwrap();
        let origen = dir.join("uno.png");
        png_1x1(&origen);
        let lista = importar(&db, &p.id, &[origen.display().to_string()]).unwrap();
        caption(&db, lista[0].id, "una figura de prueba").unwrap();

        let db = Arc::new(Mutex::new(db));
        let cfg = RunCfg {
            proyecto: p.id.clone(),
            pasos: 2,
            rango: 4,
            resolucion: 64,
            modelo: String::new(),
            probar: true,
        };
        let lanzado = entrenar(db.clone(), &cfg).expect("debe lanzar python");

        let limite = std::time::Instant::now() + std::time::Duration::from_secs(180);
        let mut final_ = lanzado.clone();
        while std::time::Instant::now() < limite {
            std::thread::sleep(std::time::Duration::from_millis(500));
            if let Some(r) = run(db.clone(), lanzado.id).unwrap() {
                final_ = r;
                if !["iniciando", "entrenando"].contains(&final_.estado.as_str()) {
                    break;
                }
            }
        }
        println!(
            "estado={} paso={}/{} perdida={:.4} salida={}",
            final_.estado, final_.paso, final_.total, final_.perdida, final_.salida
        );
        assert_eq!(final_.estado, "terminado", "log: {}", final_.log);
        assert_eq!(final_.paso, 2, "deben quedar registrados los 2 pasos");
        assert!(
            Path::new(&final_.salida).exists(),
            "el .safetensors debe existir en {}",
            final_.salida
        );
        let tam = std::fs::metadata(&final_.salida).unwrap().len();
        assert!(tam > 1024, "un LoRA de 0 bytes no es un LoRA");
        println!("fichero: {} bytes", tam);

        let _ = std::fs::remove_dir_all(dir);
        let _ = std::fs::remove_dir_all(dir_proyecto(&p.id));
    }

    #[test]
    fn un_dataset_vacio_no_lanza_entrenamiento() {
        let (db, dir) = base();
        let p = proyecto_crear(&db, "Vacio", "estilo", "modelito").unwrap();
        let db = Arc::new(Mutex::new(db));
        let r = entrenar(
            db,
            &RunCfg { proyecto: p.id.clone(), pasos: 2, rango: 4, resolucion: 64, modelo: "m".into(), probar: true },
        );
        assert!(r.is_err(), "sin imágenes no se lanza");
        assert!(r.unwrap_err().contains("caption"));
        let _ = std::fs::remove_dir_all(dir);
        let _ = std::fs::remove_dir_all(dir_proyecto(&p.id));
    }
}
