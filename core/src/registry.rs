//! Registro universal de sistemas del ecosistema.
//!
//! Cada programa se declara una vez y la detección es genérica: proceso,
//! puerto abierto, respuesta HTTP y ruta en disco. Nada se da por hecho.
//!
//! Estados, en este orden y sin saltarse ninguno:
//! - **instalado**: existe su ruta o su proceso.
//! - **detectado**: además se le ve un puerto escuchando.
//! - **conectado**: su API responde a la sonda declarada.
//! - **operativo**: conectado y, cuando se puede, con versión leída.
//!
//! Añadir un sistema nuevo es añadir una fila a [`SPECS`]: no hay lógica por
//! programa salvo la sonda HTTP, que es genérica.

use serde::Serialize;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

/// Declaración de un sistema. Todo lo que no se pueda comprobar, no se promete.
pub struct Spec {
    pub id: &'static str,
    pub nombre: &'static str,
    pub categoria: &'static str,
    /// Puertos donde suele escuchar, en orden de preferencia.
    pub puertos: &'static [u16],
    /// Ruta HTTP que identifica su API, si la tiene.
    pub sonda: Option<&'static str>,
    /// Nombres de proceso que lo delatan.
    pub procesos: &'static [&'static str],
    /// Rutas de instalación / datos.
    pub rutas: &'static [&'static str],
    /// `package.json` u otro archivo con `"version"`.
    pub version_file: Option<&'static str>,
    /// Qué puede hacer KUKIA con él, comprobado en este proyecto.
    pub capacidades: &'static [&'static str],
    /// Qué no hay. Se muestra siempre.
    pub limitaciones: &'static [&'static str],
}

const VACIO: &[&str] = &[];

/// Catálogo de sistemas. Los que no estén instalados saldrán como «no detectado»
/// y no se inventará nada sobre ellos.
pub const SPECS: &[Spec] = &[
    Spec {
        id: "kukia",
        nombre: "KUKIA CORE",
        categoria: "Núcleo",
        puertos: &[8311],
        sonda: Some("/health"),
        procesos: &[],
        rutas: VACIO,
        version_file: None,
        capacidades: &[
            "Catálogo, enrutador, memoria, herramientas y agentes propios",
            "Servidor compatible OpenAI en loopback para otros programas",
        ],
        limitaciones: &[],
    },
    Spec {
        id: "opencode",
        nombre: "OpenCode",
        categoria: "Programación",
        puertos: &[4096],
        sonda: Some("/api/health"),
        procesos: &["opencode"],
        rutas: &[r"C:\Users\kuki\AppData\Roaming\npm\opencode.cmd"],
        version_file: None,
        capacidades: &[
            "Catálogo real de modelos y agentes por su API",
            "Sesiones y respuestas con sus agentes",
        ],
        limitaciones: &[
            "Su CLI no se usa: `run` y `agent list` se cuelgan sin terminal",
            "Su opencode.json no se modifica desde KUKIA",
        ],
    },
    Spec {
        id: "freebuff",
        nombre: "Freebuff",
        categoria: "Programación",
        puertos: &[],
        sonda: None,
        procesos: &["Freebuff.exe"],
        rutas: &[r"C:\Users\kuki\AppData\Local\Programs\@codebufffreebuff-desktop\Freebuff.exe"],
        version_file: None,
        capacidades: &[
            "Proyectos, hilos y mensajes en solo lectura",
            "MCP compartidos y skills de la cuenta",
        ],
        limitaciones: &["Sin API local documentada: no se puede pilotar desde KUKIA"],
    },
    Spec {
        id: "n8n",
        nombre: "n8n",
        categoria: "Automatización",
        puertos: &[5678],
        sonda: Some("/rest/settings"),
        procesos: &["n8n"],
        rutas: &[r"C:\Users\kuki\.n8n"],
        version_file: Some(r"C:\Users\kuki\AppData\Roaming\npm\node_modules\n8n\package.json"),
        capacidades: &[
            "Base de workflows y ejecuciones localizable en solo lectura",
            "API REST en 5678 cuando está arrancado",
        ],
        limitaciones: &[
            "KUKIA no lanza workflows sin autorización ni los edita",
            "Detectado instalado ≠ API disponible: si no escucha, no hay conexión",
        ],
    },
    Spec {
        id: "dify",
        nombre: "Dify",
        categoria: "Aplicaciones IA",
        puertos: &[5001, 3000],
        sonda: Some("/console/api/setup"),
        procesos: &["dify"],
        rutas: &[r"C:\dify", r"D:\dify"],
        version_file: None,
        capacidades: &["Detección de la API de consola"],
        limitaciones: &["Requiere API key de Dify: KUKIA no la guarda ni la pide en claro"],
    },
    Spec {
        id: "flowise",
        nombre: "Flowise",
        categoria: "Agentes visuales",
        puertos: &[3000, 3001],
        sonda: Some("/api/v1/ping"),
        procesos: &["flowise"],
        rutas: &[
            r"C:\Users\kuki\AppData\Roaming\npm\node_modules\flowise",
            r"C:\flowise",
        ],
        version_file: Some(r"C:\Users\kuki\AppData\Roaming\npm\node_modules\flowise\package.json"),
        capacidades: &["Detección de su API"],
        limitaciones: &["Sin API key configurada no se invocan flujos"],
    },
    Spec {
        id: "anythingllm",
        nombre: "AnythingLLM",
        categoria: "Conocimiento",
        puertos: &[3001],
        sonda: Some("/api/ping"),
        procesos: &["AnythingLLM"],
        rutas: &[
            r"C:\Users\kuki\AppData\Local\Programs\AnythingLLM",
            r"C:\AnythingLLM",
        ],
        version_file: None,
        capacidades: &["Detección de su API de espacio de trabajo"],
        limitaciones: &["Necesita API key para consultar documentos"],
    },
    Spec {
        id: "open-webui",
        nombre: "Open WebUI",
        categoria: "Chat",
        puertos: &[8080, 8081],
        sonda: Some("/api/config"),
        procesos: &["open-webui"],
        rutas: &[r"C:\open-webui", r"C:\Users\kuki\AppData\Local\Programs\open-webui"],
        version_file: None,
        capacidades: &["Detección de su API"],
        limitaciones: &["Su base se gestiona fuera de KUKIA"],
    },
    Spec {
        id: "ollama",
        nombre: "Ollama",
        categoria: "Motor de modelos",
        puertos: &[11434],
        sonda: Some("/api/version"),
        procesos: &["ollama.exe", "ollama app.exe"],
        rutas: &[r"C:\Users\kuki\AppData\Local\Programs\Ollama"],
        version_file: None,
        capacidades: &[
            "Catálogo de modelos, chat y parámetros por modelo",
            "Descarga y liberación de VRAM por API oficial",
        ],
        limitaciones: &["No se modifica su instalación ni su almacén"],
    },
    Spec {
        id: "lmstudio",
        nombre: "LM Studio",
        categoria: "Motor de modelos",
        puertos: &[1234],
        sonda: Some("/v1/models"),
        procesos: &["LM Studio", "lms", "lm-studio"],
        rutas: &[r"C:\Users\kuki\AppData\Local\Programs\LM Studio"],
        version_file: None,
        capacidades: &["Servidor compatible OpenAI en 1234", "Carga y descarga de modelos"],
        limitaciones: &["Su catálogo externo se gestiona en la app, no desde KUKIA"],
    },
    Spec {
        id: "comfyui",
        nombre: "ComfyUI",
        categoria: "Multimodal · imagen",
        puertos: &[8188, 8189],
        sonda: Some("/system_stats"),
        // Sin procesos: `python.exe` lo comparten ComfyUI, Wan2GP y Fish, así que
        // contarlo daría un falso positivo. Aquí manda el puerto.
        procesos: VACIO,
        rutas: &[r"I:\pinokio\api\comfy.git", r"I:\pinokio\api\comfy.git\app"],
        version_file: None,
        capacidades: &[
            "Workflows, plantillas y conversión anime ↔ real",
            "Liberación de VRAM con /free (unload_models, free_memory)",
        ],
        limitaciones: &["No permite descargar un modelo individual: solo todos"],
    },
    Spec {
        id: "wan2gp",
        nombre: "Wan2GP",
        categoria: "Multimodal · vídeo",
        puertos: &[7861],
        sonda: Some("/"),
        // Corre dentro de `python.exe`: se detecta por puerto y por su carpeta.
        procesos: VACIO,
        rutas: &[r"I:\pinokio\api\wan.git"],
        version_file: None,
        capacidades: &["Modelos Wan en el M.2 (J:) y carga en GPU 1"],
        limitaciones: &["KUKIA no lanza sus generaciones: se abren en su interfaz"],
    },
    Spec {
        id: "maestro",
        nombre: "Maestro",
        categoria: "Multimodal · vídeo",
        puertos: &[7860],
        sonda: Some("/"),
        procesos: &["maestro"],
        rutas: VACIO,
        version_file: None,
        capacidades: &["Detección de su interfaz Gradio"],
        limitaciones: &["Siempre en GPU 1; no se comparte con otro modelo grande"],
    },
    Spec {
        id: "fish-speech",
        nombre: "Fish Speech",
        categoria: "Multimodal · voz",
        puertos: &[8080],
        sonda: Some("/"),
        procesos: VACIO,
        rutas: &[r"I:\pinokio\api\fish-speech.git"],
        version_file: None,
        capacidades: &["Voz clonada de Xoxete con referencia propia"],
        limitaciones: &["Tarda 90–120 s en calentar y no cabe con Maestro a la vez"],
    },
    Spec {
        id: "rvc",
        nombre: "RVC",
        categoria: "Multimodal · voz",
        puertos: &[7865],
        sonda: Some("/"),
        procesos: &["rvc"],
        rutas: &[r"I:\pinokio\api\rvc.git"],
        version_file: None,
        capacidades: &["Detección de su interfaz"],
        limitaciones: &["Conversión de voz no integrada por API en este proyecto"],
    },
    Spec {
        id: "qwen3-tts",
        nombre: "Qwen3-TTS",
        categoria: "Multimodal · voz",
        puertos: &[8000],
        sonda: Some("/"),
        procesos: &["qwen3_tts"],
        rutas: &[r"I:\pinokio\api\qwen3-tts.git"],
        version_file: None,
        capacidades: &["Detección de su servicio"],
        limitaciones: &["Sin adaptador de síntesis implementado todavía"],
    },
    Spec {
        id: "ltx",
        nombre: "LTX Desktop",
        categoria: "Multimodal · vídeo",
        puertos: &[],
        sonda: None,
        procesos: &["LTX", "ltx-desktop"],
        // `J:\models\video` es el almacén de modelos, no su instalación: usarlo
        // como prueba de instalación daría un falso positivo.
        rutas: VACIO,
        version_file: None,
        capacidades: &[r"Comparte el almacén de modelos con Wan2GP en J:\models\video"],
        limitaciones: &[
            "Funciona en GPU 0; KUKIA no lo lanza",
            "Su carpeta de instalación no se ha verificado en este equipo",
        ],
    },
    Spec {
        id: "pinokio",
        nombre: "Pinokio",
        categoria: "Plataforma",
        puertos: &[42000],
        sonda: Some("/api/status"),
        procesos: &["pinokio.exe", "Pinokio.exe"],
        rutas: &[r"I:\pinokio"],
        version_file: None,
        capacidades: &["Detección de la plataforma que aloja los motores"],
        limitaciones: &["No se tocan sus instalaciones: solo se detectan"],
    },
];

#[derive(Debug, Clone, Serialize, Default)]
pub struct Sistema {
    pub id: String,
    pub nombre: String,
    pub categoria: String,
    pub instalado: bool,
    pub detectado: bool,
    pub conectado: bool,
    pub operativo: bool,
    pub version: String,
    pub ruta: String,
    pub puertos_abiertos: Vec<u16>,
    pub endpoint: String,
    pub detalle: String,
    pub error: Option<String>,
    pub capacidades: Vec<String>,
    pub limitaciones: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Inventario {
    pub sistemas: Vec<Sistema>,
    pub total: usize,
    pub instalados: usize,
    pub conectados: usize,
    pub operativos: usize,
    pub no_detectados: Vec<String>,
}

#[cfg(windows)]
fn oculto(cmd: &mut Command) -> &mut Command {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(0x0800_0000)
}

#[cfg(not(windows))]
fn oculto(cmd: &mut Command) -> &mut Command {
    cmd
}

/// Procesos vivos de la máquina, una sola vez para todos los sistemas.
fn procesos_vivos() -> Vec<String> {
    let mut cmd = Command::new("tasklist");
    cmd.args(["/FO", "CSV", "/NH"]);
    let Ok(out) = oculto(&mut cmd).output() else {
        return Vec::new();
    };
    let txt = String::from_utf8_lossy(&out.stdout).to_string();
    txt.lines()
        .filter_map(|l| l.split(',').next())
        .map(|s| s.trim_matches('"').to_lowercase())
        .collect()
}

async fn puerto_abierto(port: u16) -> bool {
    tokio::time::timeout(
        Duration::from_millis(600),
        tokio::net::TcpStream::connect(("127.0.0.1", port)),
    )
    .await
    .map(|r| r.is_ok())
    .unwrap_or(false)
}

/// Busca un campo de versión en un JSON, con profundidad acotada.
fn buscar_version(v: &serde_json::Value, nivel: usize) -> Option<String> {
    if nivel > 3 {
        return None;
    }
    if let Some(obj) = v.as_object() {
        for (k, val) in obj {
            if k.to_lowercase().contains("version") {
                if let Some(s) = val.as_str() {
                    if !s.trim().is_empty() {
                        return Some(s.to_string());
                    }
                }
                if let Some(n) = val.as_number() {
                    return Some(n.to_string());
                }
            }
        }
        for val in obj.values() {
            if let Some(s) = buscar_version(val, nivel + 1) {
                return Some(s);
            }
        }
    }
    None
}

async fn sondear(port: u16, ruta: &str) -> (bool, String, Option<String>) {
    let url = format!("http://127.0.0.1:{port}{ruta}");
    let cli = match reqwest::Client::builder()
        .timeout(Duration::from_millis(1500))
        .build()
    {
        Ok(c) => c,
        Err(e) => return (false, url, Some(format!("cliente HTTP: {e}"))),
    };
    match cli.get(&url).send().await {
        Ok(r) => {
            let estado = r.status();
            let texto = r.text().await.unwrap_or_default();
            if estado.is_success() {
                let version = serde_json::from_str::<serde_json::Value>(&texto)
                    .ok()
                    .and_then(|v| buscar_version(&v, 0));
                (true, url, version)
            } else {
                (false, url, Some(format!("HTTP {estado}")))
            }
        }
        Err(e) => (false, url, Some(format!("sin respuesta: {e}"))),
    }
}

fn leer_version_archivo(p: &str) -> Option<String> {
    let txt = std::fs::read_to_string(p).ok()?;
    let v: serde_json::Value = serde_json::from_str(txt.trim_start_matches('\u{feff}')).ok()?;
    v.get("version").and_then(|x| x.as_str()).map(|s| s.to_string())
}

/// Detecta un sistema según su declaración.
pub async fn detectar_uno(spec: &Spec, procesos: &[String]) -> Sistema {
    let mut s = Sistema {
        id: spec.id.into(),
        nombre: spec.nombre.into(),
        categoria: spec.categoria.into(),
        capacidades: spec.capacidades.iter().map(|c| c.to_string()).collect(),
        limitaciones: spec.limitaciones.iter().map(|c| c.to_string()).collect(),
        ..Default::default()
    };

    // 1) Rutas de instalación / datos.
    let ruta = spec.rutas.iter().find(|r| Path::new(r).exists());
    if let Some(r) = ruta {
        s.ruta = r.to_string();
    }

    // 2) Procesos (comparación por nombre base, sin extensión).
    let mut procesos_encontrados = 0usize;
    for p in spec.procesos {
        let buscado = p.to_lowercase();
        let base = buscado.split('.').next().unwrap_or(&buscado).to_string();
        procesos_encontrados += procesos
            .iter()
            .filter(|v| *v == &buscado || v.starts_with(&base))
            .count();
    }

    // 3) Puertos y sonda HTTP.
    let mut abiertos = Vec::new();
    for p in spec.puertos {
        if puerto_abierto(*p).await {
            abiertos.push(*p);
        }
    }
    s.puertos_abiertos = abiertos.clone();

    let mut version_sonda: Option<String> = None;
    if let (Some(port), Some(ruta_http)) = (abiertos.first(), spec.sonda) {
        let (ok, url, dato) = sondear(*port, ruta_http).await;
        s.endpoint = url;
        s.conectado = ok;
        if ok {
            version_sonda = dato.clone();
        } else {
            s.error = dato;
        }
    }

    // 4) Versión: del archivo declarado o de la API.
    s.version = spec
        .version_file
        .and_then(leer_version_archivo)
        .or(version_sonda)
        .unwrap_or_default();

    // KUKIA se detecta a sí misma: es el programa que ejecuta esto.
    if spec.id == "kukia" {
        s.instalado = true;
        s.detectado = true;
        s.version = env!("CARGO_PKG_VERSION").to_string();
        s.ruta = std::env::current_exe()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "kukia-code".into());
    }

    s.instalado = s.instalado || ruta.is_some() || procesos_encontrados > 0 || !abiertos.is_empty();
    s.detectado = s.detectado || !abiertos.is_empty() || procesos_encontrados > 0;
    s.operativo = s.conectado && !s.version.is_empty();

    s.detalle = {
        let mut partes = Vec::new();
        if let Some(r) = ruta {
            partes.push(format!("ruta {}", r));
        }
        if procesos_encontrados > 0 {
            partes.push(format!("{procesos_encontrados} proceso(s)"));
        }
        if abiertos.is_empty() {
            partes.push("sin puerto abierto".into());
        } else {
            partes.push(format!(
                "escucha en {}",
                abiertos.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", ")
            ));
        }
        if s.conectado {
            partes.push("API responde".into());
        }
        partes.join(" · ")
    };

    s
}

/// Inventario completo. Cada sistema se detecta en paralelo y con su propio
/// timeout: uno lento no bloquea al resto.
pub async fn inventario() -> Inventario {
    let procesos = procesos_vivos();
    // `detectar_uno` necesita la lista de procesos por valor compartido.
    let mut tareas = Vec::new();
    for spec in SPECS {
        let procesos = procesos.clone();
        tareas.push(async move { detectar_uno(spec, &procesos).await });
    }
    let sistemas = futures_util::future::join_all(tareas).await;

    let instalados = sistemas.iter().filter(|s| s.instalado).count();
    let conectados = sistemas.iter().filter(|s| s.conectado).count();
    let operativos = sistemas.iter().filter(|s| s.operativo).count();
    let no_detectados = sistemas
        .iter()
        .filter(|s| !s.instalado)
        .map(|s| s.nombre.clone())
        .collect();

    Inventario {
        total: sistemas.len(),
        instalados,
        conectados,
        operativos,
        no_detectados,
        sistemas,
    }
}

/* ============================ Tests ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn todos_los_sistemas_tienen_id_unico_y_nombre() {
        let mut ids = std::collections::HashSet::new();
        for s in SPECS {
            assert!(!s.nombre.is_empty());
            assert!(ids.insert(s.id), "id repetido: {}", s.id);
        }
        assert!(SPECS.len() >= 15, "el registro debe cubrir la lista pedida");
    }

    #[test]
    fn la_version_se_encuentra_en_json_anidado() {
        let v = serde_json::json!({ "system": { "comfyui_version": "0.3.40" } });
        assert_eq!(buscar_version(&v, 0).as_deref(), Some("0.3.40"));
    }

    #[test]
    fn sin_version_devuelve_none() {
        let v = serde_json::json!({ "ok": true });
        assert!(buscar_version(&v, 0).is_none());
    }

    #[test]
    fn cada_sistema_declara_algo_que_puede_y_sus_limites() {
        for s in SPECS {
            assert!(
                !s.capacidades.is_empty(),
                "{} no declara ninguna capacidad",
                s.id
            );
            // Todo sistema con API propia debe decir qué no se puede hacer con él.
            if s.sonda.is_some() && s.id != "kukia" {
                assert!(
                    !s.limitaciones.is_empty(),
                    "{} tiene API pero no declara limitaciones",
                    s.id
                );
            }
        }
    }
}
