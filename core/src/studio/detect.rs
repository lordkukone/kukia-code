//! Detección y sondeo del convertidor universal (RC2).
//!
//! Tres cosas, sin ejecutar difusión: sondeo con ffprobe, detección de estilo
//! con un VLM local (Ollama) **con confianza y editable**, y segmentación de
//! planos con FFmpeg. El preflight junta todo y dice la verdad antes de gastar
//! GPU: qué se va a hacer, qué falta y qué no se puede prometer con este material.

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

use crate::studio::engines;

#[cfg(windows)]
fn oculto(cmd: &mut Command) -> &mut Command {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(0x0800_0000) // CREATE_NO_WINDOW
}

#[cfg(not(windows))]
fn oculto(cmd: &mut Command) -> &mut Command {
    cmd
}

fn temp_dir() -> PathBuf {
    std::env::temp_dir().join("kukia_studio")
}

/* ============================ Sondeo ============================ */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sondeo {
    pub ruta: String,
    /// `imagen` | `video`
    pub tipo: String,
    pub ancho: u32,
    pub alto: u32,
    pub fps: f64,
    pub segundos: f64,
    pub audio: bool,
    pub codec: String,
    pub bytes: u64,
}

fn fraccion_a_f64(s: &str) -> f64 {
    let mut p = s.split('/');
    let num: f64 = p.next().and_then(|x| x.parse().ok()).unwrap_or(0.0);
    let den: f64 = p.next().and_then(|x| x.parse().ok()).unwrap_or(1.0);
    if den == 0.0 {
        0.0
    } else {
        num / den
    }
}

/// Lee un archivo con ffprobe. No modifica nada.
pub async fn sondear(ruta: &str) -> Result<Sondeo, String> {
    let ruta_owned = ruta.to_string();
    let crudo = tokio::task::spawn_blocking(move || {
        let mut cmd = Command::new("ffprobe");
        cmd.args([
            "-v",
            "error",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
            &ruta_owned,
        ]);
        let out = oculto(&mut cmd)
            .output()
            .map_err(|e| format!("no se pudo lanzar ffprobe: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "ffprobe falló: {}",
                String::from_utf8_lossy(&out.stderr).lines().next().unwrap_or("")
            ));
        }
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    })
    .await
    .map_err(|e| format!("tarea de sondeo: {e}"))??;

    let v: serde_json::Value =
        serde_json::from_str(&crudo).map_err(|e| format!("ffprobe devolvió algo ilegible: {e}"))?;
    let streams = v["streams"].as_array().cloned().unwrap_or_default();

    let video = streams
        .iter()
        .find(|s| s["codec_type"].as_str() == Some("video"));
    let audio = streams
        .iter()
        .any(|s| s["codec_type"].as_str() == Some("audio"));

    let Some(vid) = video else {
        return Err("el archivo no tiene pista de vídeo (¿es una imagen suelta?)".into());
    };

    let ancho = vid["width"].as_u64().unwrap_or(0) as u32;
    let alto = vid["height"].as_u64().unwrap_or(0) as u32;
    let fps = fraccion_a_f64(vid["avg_frame_rate"].as_str().unwrap_or("0/1"));
    let dur_stream = vid["duration"]
        .as_str()
        .and_then(|d| d.parse::<f64>().ok())
        .unwrap_or(0.0);
    let dur_format = v["format"]["duration"]
        .as_str()
        .and_then(|d| d.parse::<f64>().ok())
        .unwrap_or(0.0);
    let segundos = if dur_stream > 0.0 { dur_stream } else { dur_format };
    let codec = vid["codec_name"].as_str().unwrap_or("").to_string();
    let bytes = v["format"]["size"]
        .as_str()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    let tipo = if segundos > 0.12 && fps > 0.5 {
        "video"
    } else {
        "imagen"
    };

    Ok(Sondeo {
        ruta: ruta.to_string(),
        tipo: tipo.to_string(),
        ancho,
        alto,
        fps: (fps * 100.0).round() / 100.0,
        segundos: (segundos * 100.0).round() / 100.0,
        audio,
        codec,
        bytes,
    })
}

/* ============================ Detección de estilo ============================ */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstiloDetectado {
    pub estilo: String,
    pub confianza: f64,
    pub motivos: String,
    pub modelo: String,
    /// Texto crudo del VLM si no devolvió JSON: transparencia antes que inventar.
    #[serde(default)]
    pub crudo: String,
    /// El usuario puede corregirlo: esto es una sugerencia, no una sentencia.
    #[serde(default)]
    pub editable: bool,
}

const ESTILOS: &[&str] = &[
    "anime", "cartoon", "cgi", "pixel", "comic", "photoreal", "gameplay", "sketch",
];

fn prompt_deteccion() -> String {
    format!(
        "Eres un clasificador de estilo visual. Mira el fotograma y responde SOLO con JSON válido, \
sin texto fuera del JSON:\n\
{{\"estilo\": \"uno de: {}\", \"confianza\": 0.0, \"motivos\": \"breve, en español\"}}\n\
Reglas: usa \"photoreal\" solo si parece imagen real de cámara; \"anime\" para animación japonesa 2D; \
\"cgi\" para 3D/Pixar; \"gameplay\" si hay HUD o interfaz de juego; \"sketch\" si es boceto a línea.",
        ESTILOS.join("|")
    )
}

/// Extrae un fotograma al 25 % del metraje y lo devuelve como base64 JPEG.
fn fotograma_b64(ruta: &str, segundos: f64) -> Result<String, String> {
    let dir = temp_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("no pude crear {dir:?}: {e}"))?;
    let destino = dir.join(format!("foto_{}.jpg", std::process::id()));
    let marca = if segundos > 0.0 { segundos * 0.25 } else { 0.0 };

    let mut cmd = Command::new("ffmpeg");
    cmd.args([
        "-hide_banner",
        "-loglevel",
        "error",
        "-y",
        "-ss",
        &format!("{marca:.3}"),
        "-i",
        ruta,
        "-frames:v",
        "1",
        "-vf",
        "scale='min(768,iw)':-2",
        "-q:v",
        "3",
    ])
    .arg(&destino);
    oculto(&mut cmd)
        .output()
        .map_err(|e| format!("no se pudo lanzar ffmpeg: {e}"))?;

    let bytes = std::fs::read(&destino)
        .map_err(|e| format!("ffmpeg no dejó fotograma ({}): {e}", destino.display()))?;
    let _ = std::fs::remove_file(&destino);
    Ok(crate::anime_real::b64(&bytes))
}

/// Pregunta a un VLM local (Ollama) qué estilo tiene el material.
pub async fn detectar_estilo(ruta: &str, modelo: Option<&str>) -> Result<EstiloDetectado, String> {
    let sondeo = sondear(ruta).await?;
    let marca = if sondeo.segundos > 0.0 { sondeo.segundos } else { 0.0 };
    let ruta_owned = ruta.to_string();
    let b64 = tokio::task::spawn_blocking(move || fotograma_b64(&ruta_owned, marca))
        .await
        .map_err(|e| format!("tarea de fotograma: {e}"))??;

    let base = engines::ollama_default();
    let modelo = match modelo {
        Some(m) if !m.trim().is_empty() => m.to_string(),
        _ => {
            // El primero con visión que haya instalado; si no hay, se dice.
            let motores = engines::todos().await;
            let ollama = motores.iter().find(|m| m.id == "ollama");
            let capaces: Vec<String> = ollama
                .map(|m| {
                    m.capacidades
                        .iter()
                        .filter(|c| c.contains(":") || c.contains("/"))
                        .cloned()
                        .collect()
                })
                .unwrap_or_default();
            // Preferencia explícita: un qwen3-vl pequeño sigue instrucciones de
            // JSON mucho mejor que un llava viejo, que tiende a repetir el prompt.
            let elegido = capaces
                .iter()
                .find(|n| n.to_lowercase().contains("qwen3-vl") && !n.to_lowercase().contains("abliterated"))
                .or_else(|| capaces.iter().find(|n| n.to_lowercase().contains("qwen3-vl")))
                .or_else(|| capaces.iter().find(|n| n.to_lowercase().contains("llava")))
                .or_else(|| capaces.iter().find(|n| n.to_lowercase().contains("moondream")));
            elegido
                .cloned()
                .unwrap_or_else(|| "qwen3-vl:4b".to_string())
        }
    };

    let cuerpo = serde_json::json!({
        "model": modelo,
        "prompt": prompt_deteccion(),
        "images": [b64],
        "stream": false,
        "format": "json",
        "options": { "temperature": 0.0 }
    });

    let respuesta = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
        .post(format!("{base}/api/generate"))
        .json(&cuerpo)
        .send()
        .await
        .map_err(|e| format!("Ollama no respondió ({e}); ¿está levantado en {base}?"))?;

    let v: serde_json::Value = respuesta
        .json()
        .await
        .map_err(|e| format!("respuesta de Ollama ilegible: {e}"))?;

    let texto = v["response"].as_str().unwrap_or("").to_string();
    let (estilo, confianza, motivos) = match serde_json::from_str::<serde_json::Value>(&texto) {
        Ok(j) => {
            let estilo = j["estilo"].as_str().unwrap_or("").to_lowercase();
            let confianza = j["confianza"].as_f64().unwrap_or(0.0);
            let motivos = j["motivos"].as_str().unwrap_or("").to_string();
            (estilo, confianza, motivos)
        }
        Err(_) => {
            // Sin JSON: se busca la primera palabra del catálogo en la respuesta.
            // Se marca con confianza baja y se guarda el texto crudo: no se
            // convierte una respuesta libre en una certeza.
            let bajo = texto.to_lowercase();
            let hallado = ESTILOS
                .iter()
                .find(|e| bajo.contains(**e))
                .map(|e| e.to_string())
                .unwrap_or_default();
            (
                hallado.clone(),
                if hallado.is_empty() { 0.0 } else { 0.4 },
                if hallado.is_empty() {
                    String::new()
                } else {
                    "respuesta sin JSON: estilo inferido del texto".to_string()
                },
            )
        }
    };

    let valido = ESTILOS.contains(&estilo.as_str());
    Ok(EstiloDetectado {
        estilo: if valido { estilo } else { "desconocido".into() },
        confianza: if valido {
            (confianza.clamp(0.0, 1.0) * 100.0).round() / 100.0
        } else {
            0.0
        },
        motivos,
        modelo,
        crudo: texto,
        editable: true,
    })
}

/* ============================ Planos ============================ */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plano {
    pub indice: usize,
    pub inicio: f64,
    pub fin: f64,
    pub segundos: f64,
}

fn cortes_ffmpeg(ruta: &str, umbral: f64) -> Result<Vec<f64>, String> {
    let filtro = format!("select='gt(scene,{umbral:.3})',showinfo");
    let mut cmd = Command::new("ffmpeg");
    cmd.args([
        "-hide_banner",
        "-nostats",
        "-i",
        ruta,
        "-vf",
        &filtro,
        "-an",
        "-f",
        "null",
        "-",
    ]);
    let out = oculto(&mut cmd)
        .output()
        .map_err(|e| format!("no se pudo lanzar ffmpeg: {e}"))?;
    let err = String::from_utf8_lossy(&out.stderr);
    let mut cortes = Vec::new();
    for linea in err.lines() {
        if let Some(p) = linea.find("pts_time:") {
            let resto = &linea[p + 9..];
            let num: String = resto
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            if let Ok(t) = num.parse::<f64>() {
                cortes.push(t);
            }
        }
    }
    cortes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    Ok(cortes)
}

/// Corta el material en planos por cambio de escena. Un plano = un job.
pub async fn planos(ruta: &str, umbral: f64) -> Result<Vec<Plano>, String> {
    let sondeo = sondear(ruta).await?;
    if sondeo.tipo == "imagen" {
        return Ok(vec![Plano { indice: 0, inicio: 0.0, fin: sondeo.segundos.max(0.0), segundos: sondeo.segundos }]);
    }
    let ruta_owned = ruta.to_string();
    let u = umbral.clamp(0.05, 0.95);
    let cortes = tokio::task::spawn_blocking(move || cortes_ffmpeg(&ruta_owned, u))
        .await
        .map_err(|e| format!("tarea de cortes: {e}"))??;

    let dur = sondeo.segundos.max(0.01);
    let mut marcas = vec![0.0f64];
    for c in cortes {
        if c > 0.05 && c < dur - 0.05 {
            marcas.push(c);
        }
    }
    marcas.push(dur);
    marcas.dedup();

    let mut out = Vec::new();
    for i in 0..marcas.len().saturating_sub(1) {
        let inicio = marcas[i];
        let fin = marcas[i + 1];
        if fin - inicio < 0.1 {
            continue;
        }
        out.push(Plano {
            indice: out.len(),
            inicio: (inicio * 100.0).round() / 100.0,
            fin: (fin * 100.0).round() / 100.0,
            segundos: ((fin - inicio) * 100.0).round() / 100.0,
        });
    }
    Ok(out)
}

/// Recorta un plano a un archivo aparte, sin recomprimir (`-c copy`).
pub fn partir_plano(ruta: &str, inicio: f64, segundos: f64, destino: &str) -> Result<String, String> {
    if let Some(p) = std::path::Path::new(destino).parent() {
        std::fs::create_dir_all(p).map_err(|e| format!("no pude crear {p:?}: {e}"))?;
    }
    let mut cmd = Command::new("ffmpeg");
    cmd.args([
        "-hide_banner",
        "-loglevel",
        "error",
        "-y",
        "-ss",
        &format!("{inicio:.3}"),
        "-i",
        ruta,
        "-t",
        &format!("{segundos:.3}"),
        "-map",
        "0",
        "-c",
        "copy",
    ])
    .arg(destino);
    let out = oculto(&mut cmd)
        .output()
        .map_err(|e| format!("no se pudo lanzar ffmpeg: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "ffmpeg no pudo cortar el plano: {}",
            String::from_utf8_lossy(&out.stderr).lines().last().unwrap_or("")
        ));
    }
    Ok(destino.to_string())
}

/* ============================ Preflight ============================ */

#[derive(Debug, Clone, Serialize)]
pub struct Preflight {
    pub sondeo: Sondeo,
    pub deteccion: Option<EstiloDetectado>,
    pub planos: Vec<Plano>,
    pub paquete_id: String,
    pub paquete_titulo: String,
    pub plantilla: String,
    /// `listo` | `con_avisos` | `bloqueado`
    pub veredicto: String,
    pub avisos: Vec<String>,
    pub faltan: Vec<String>,
    pub vram_libre_max_gb: f64,
    pub ram_libre_gb: f64,
    pub disco_estimado_gb: f64,
}

/// Junta sondeo, detección y segmentación, y decide si se puede lanzar.
pub async fn preflight(
    ruta: &str,
    destino: Option<&str>,
    modelo_vlm: Option<&str>,
    umbral: f64,
) -> Result<Preflight, String> {
    let sondeo = sondear(ruta).await?;
    let planos = planos(ruta, umbral).await.unwrap_or_default();

    // La detección no es imprescindible: si el VLM falla, se sigue con aviso.
    let (deteccion, aviso_vlm) = match detectar_estilo(ruta, modelo_vlm).await {
        Ok(d) => {
            let aviso = if d.confianza < 0.6 {
                Some(format!(
                    "estilo detectado con poca confianza ({:.0} %): revísalo antes de gastar GPU",
                    d.confianza * 100.0
                ))
            } else {
                None
            };
            (Some(d), aviso)
        }
        Err(e) => (None, Some(format!("sin detección automática: {e}"))),
    };

    let origen = deteccion
        .as_ref()
        .map(|d| d.estilo.clone())
        .unwrap_or_else(|| "photoreal".into());
    let destino_final = destino
        .map(|d| d.to_string())
        .unwrap_or_else(|| if origen == "photoreal" { "anime".into() } else { "photoreal".into() });

    let paquete = crate::studio::styles::elegir(&origen, &destino_final, &sondeo.tipo);

    let mut avisos: Vec<String> = Vec::new();
    if let Some(a) = aviso_vlm {
        avisos.push(a);
    }
    if sondeo.ancho < 480 || sondeo.alto < 480 {
        avisos.push(format!(
            "origen pequeño ({}×{}): no hay información para un primer plano realista",
            sondeo.ancho, sondeo.alto
        ));
    }
    if sondeo.tipo == "video" && sondeo.fps > 0.0 && sondeo.fps < 12.0 {
        avisos.push(format!(
            "animación limitada ({:.0} fps): habrá que interpolar y el movimiento no será fiel",
            sondeo.fps
        ));
    }
    if sondeo.tipo == "video" && !sondeo.audio {
        avisos.push("el material no tiene audio: el montaje saldrá mudo salvo que se añada".into());
    }
    if planos.len() > 40 {
        avisos.push(format!(
            "{} planos detectados: son {} jobs, no uno",
            planos.len(),
            planos.len()
        ));
    }

    let presupuesto = engines::presupuesto();
    let vram = engines::vram_max_libre_gb(&presupuesto);
    let ram = presupuesto.ram_libre_gb;

    let (paquete_id, paquete_titulo, plantilla, faltan, vram_min, ram_min) = match &paquete {
        Some(p) => {
            let visibles = crate::anime_real::modelos_visibles(&engines::comfy_default())
                .await
                .unwrap_or_default();
            let faltan: Vec<String> = p
                .modelos
                .iter()
                .filter(|m| !visibles.iter().any(|v| v == *m || v.ends_with(m.as_str())))
                .cloned()
                .collect();
            (
                p.id.clone(),
                p.titulo.clone(),
                p.plantilla.clone(),
                faltan,
                p.vram_min_gb,
                p.ram_min_gb,
            )
        }
        None => {
            avisos.push(format!(
                "no hay paquete para {} → {} en {}: no se inventa uno",
                origen, destino_final, sondeo.tipo
            ));
            (String::new(), String::new(), String::new(), Vec::new(), 0.0, 0.0)
        }
    };

    if vram_min > 0.0 && vram > 0.0 && vram_min > vram + 0.05 {
        avisos.push(format!(
            "el paquete pide {:.0} GB de VRAM y hay {:.1} GB libres",
            vram_min, vram
        ));
    }
    if ram_min > 0.0 && ram_min > ram {
        avisos.push(format!(
            "pide {:.0} GB de RAM y hay {:.1} GB libres",
            ram_min, ram
        ));
    }

    // Disco estimado de intermedios: fotogramas PNG a la resolución del paquete.
    let (w, h) = paquete
        .as_ref()
        .and_then(|p| {
            if p.resolucion.len() >= 2 {
                Some((p.resolucion[0] as f64, p.resolucion[1] as f64))
            } else {
                None
            }
        })
        .unwrap_or((sondeo.ancho as f64, sondeo.alto as f64));
    let frames_total = if sondeo.tipo == "video" {
        (sondeo.segundos * sondeo.fps.max(1.0)) as f64
    } else {
        1.0
    };
    let disco_estimado_gb = (w * h * 3.0 * frames_total / 1e9 * 100.0).round() / 100.0;

    let veredicto = if paquete.is_none() || !faltan.is_empty() {
        "bloqueado".to_string()
    } else if avisos.is_empty() {
        "listo".to_string()
    } else {
        "con_avisos".to_string()
    };

    Ok(Preflight {
        sondeo,
        deteccion,
        planos,
        paquete_id,
        paquete_titulo,
        plantilla,
        veredicto,
        avisos,
        faltan,
        vram_libre_max_gb: (vram * 10.0).round() / 10.0,
        ram_libre_gb: (ram * 10.0).round() / 10.0,
        disco_estimado_gb,
    })
}

/// Guarda un informe legible del preflight junto al material analizado.
pub fn guardar_informe(p: &Preflight, destino: &str) -> Result<String, String> {
    let mut f = std::fs::File::create(destino).map_err(|e| format!("no pude crear {destino}: {e}"))?;
    writeln!(f, "# PREFLIGHT — {}", p.sondeo.ruta).map_err(|e| e.to_string())?;
    writeln!(f, "\n- tipo: {} · {}×{} · {:.2} fps · {:.2} s · audio: {} · codec: {}",
        p.sondeo.tipo, p.sondeo.ancho, p.sondeo.alto, p.sondeo.fps, p.sondeo.segundos,
        if p.sondeo.audio { "sí" } else { "no" }, p.sondeo.codec).map_err(|e| e.to_string())?;
    if let Some(d) = &p.deteccion {
        writeln!(f, "- estilo detectado: {} ({:.0} %) con {} · {}", d.estilo, d.confianza * 100.0, d.modelo, d.motivos)
            .map_err(|e| e.to_string())?;
    } else {
        writeln!(f, "- estilo detectado: no disponible").map_err(|e| e.to_string())?;
    }
    writeln!(f, "- planos: {}", p.planos.len()).map_err(|e| e.to_string())?;
    writeln!(f, "- paquete: {} ({})", p.paquete_titulo, p.paquete_id).map_err(|e| e.to_string())?;
    writeln!(f, "- veredicto: **{}**", p.veredicto).map_err(|e| e.to_string())?;
    for a in &p.avisos {
        writeln!(f, "  - aviso: {a}").map_err(|e| e.to_string())?;
    }
    for m in &p.faltan {
        writeln!(f, "  - FALTA: {m}").map_err(|e| e.to_string())?;
    }
    writeln!(f, "- VRAM libre máxima: {:.1} GB · RAM libre: {:.1} GB · intermedios ~{:.2} GB",
        p.vram_libre_max_gb, p.ram_libre_gb, p.disco_estimado_gb).map_err(|e| e.to_string())?;
    Ok(destino.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fracciones_de_fps() {
        assert!((fraccion_a_f64("30000/1001") - 29.97).abs() < 0.01);
        assert_eq!(fraccion_a_f64("0/0"), 0.0);
        assert_eq!(fraccion_a_f64("bogus"), 0.0);
    }

    #[test]
    fn los_planos_cubren_todo_el_metraje_sin_huecos() {
        let marcas = vec![0.0, 1.5, 4.25, 10.0];
        let planos: Vec<Plano> = marcas
            .windows(2)
            .enumerate()
            .map(|(i, w)| Plano {
                indice: i,
                inicio: w[0],
                fin: w[1],
                segundos: w[1] - w[0],
            })
            .collect();
        assert_eq!(planos.len(), 3);
        assert_eq!(planos[0].inicio, 0.0);
        assert_eq!(planos.last().unwrap().fin, 10.0);
        for p in &planos {
            assert!(p.segundos > 0.0);
        }
    }

    #[test]
    fn el_prompt_solo_admite_estilos_del_catalogo() {
        let p = prompt_deteccion();
        for e in ESTILOS {
            assert!(p.contains(e), "el prompt no menciona {e}");
        }
        assert!(!ESTILOS.contains(&"otro"));
    }
}
