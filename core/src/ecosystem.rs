//! Ecosistema IA: **KUKIA CORE + OpenCode + Freebuff** en un mismo panel.
//!
//! Este módulo no inventa integraciones. Cada capacidad que declara está
//! comprobada contra la instalación real de esta máquina:
//!
//! | Nodo | Qué hay de verdad |
//! |---|---|
//! | KUKIA CORE | catálogo local (Ollama/LM Studio/ComfyUI), agentes en disco, MCP autorizados y servidor OpenAI en loopback |
//! | OpenCode 1.19.3 | API local con token propio: `/api/model` (213), `/api/agent` (30), sesiones y mensajes |
//! | Freebuff 0.0.192 | procesos, proyectos, conversaciones en SQLite (solo lectura), MCP compartidos y skills |
//!
//! Lo que **no** existe y por eso se declara como limitación:
//! - Freebuff no publica API local documentada. Sus puertos `34147`/`34148`
//!   responden `401 missing or invalid token`: no se usan para pilotarlo.
//! - OpenCode no permite cambiar su configuración desde fuera sin editar su
//!   `opencode.json`; KUKIA genera el fragmento y no lo escribe.
//!
//! Regla de independencia: cada nodo se consulta con su propio timeout y, si
//! falla, el resto del ecosistema sigue respondiendo.

use crate::bridge;
use crate::catalog;
use crate::config::AppConfig;
use crate::memory::Database;
use crate::{agents, mcp, serve, tools};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

/* ============================ Tipos ============================ */

#[derive(Debug, Clone, Serialize, Default)]
pub struct Cuenta {
    pub nombre: String,
    pub cantidad: usize,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Nodo {
    pub id: String,
    pub nombre: String,
    pub instalado: bool,
    pub conectado: bool,
    pub version: String,
    pub ruta: String,
    pub detalle: String,
    pub procesos: usize,
    pub puertos: Vec<String>,
    pub modelos: usize,
    pub agentes: usize,
    pub herramientas: usize,
    pub sesiones: usize,
    /// Capacidades comprobadas (se muestran tal cual al usuario).
    pub puede: Vec<String>,
    /// Lo que ese nodo no ofrece: sin esto el panel mentiría.
    pub limitaciones: Vec<String>,
    pub ultimo_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Catalogo {
    pub modelos_locales: usize,
    pub modelos_opencode: usize,
    pub agentes_kukia: usize,
    pub agentes_opencode: usize,
    pub herramientas_kukia: usize,
    pub servidores_mcp: usize,
    pub por_motor: Vec<Cuenta>,
    pub por_proveedor_opencode: Vec<Cuenta>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Ecosistema {
    pub nodos: Vec<Nodo>,
    pub catalogo: Catalogo,
    pub notas: Vec<String>,
}

/// Datos de KUKIA que se leen de la base. Se extraen en una fase bloqueante y
/// se suelta la conexión antes de cualquier `await`: la conexión de rusqlite no
/// es `Send` y retenerla rompería los comandos async de la interfaz.
#[derive(Debug, Clone, Default)]
pub struct DatosLocales {
    pub herramientas: usize,
    pub agentes: usize,
    pub mcp: usize,
    pub error: Option<String>,
}

pub fn datos_locales(db_path: &Path) -> DatosLocales {
    let agentes = agents::load_agents().len();
    match Database::open(db_path) {
        Ok(db) => {
            let r = tools::resumen(&db);
            let m = mcp::servidores(&db).len();
            DatosLocales {
                herramientas: r.total,
                agentes,
                mcp: m,
                error: None,
            }
        }
        Err(e) => DatosLocales {
            herramientas: 0,
            agentes,
            mcp: 0,
            error: Some(e),
        },
    }
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Recomendacion {
    pub sistema: String,
    pub motivo: String,
    /// `true` si KUKIA puede lanzar la tarea por sí misma en ese sistema.
    pub automatizable: bool,
    pub como: String,
    pub alternativas: Vec<String>,
}

/* ============================ Utilidades ============================ */

#[cfg(windows)]
fn oculto(cmd: &mut Command) -> &mut Command {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(0x0800_0000)
}

#[cfg(not(windows))]
fn oculto(cmd: &mut Command) -> &mut Command {
    cmd
}

fn home() -> PathBuf {
    PathBuf::from(
        std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".into()),
    )
}

fn local_appdata() -> PathBuf {
    PathBuf::from(
        std::env::var("LOCALAPPDATA").unwrap_or_else(|_| home().join("AppData\\Local").display().to_string()),
    )
}

/// Ruta del ejecutable de Freebuff (instalador real de esta máquina).
pub fn freebuff_exe() -> PathBuf {
    if let Ok(p) = std::env::var("KUKIA_FREEBUFF_EXE") {
        if !p.trim().is_empty() {
            return PathBuf::from(p);
        }
    }
    local_appdata()
        .join("Programs")
        .join("@codebufffreebuff-desktop")
        .join("Freebuff.exe")
}

pub fn freebuff_config() -> PathBuf {
    home().join(".config").join("freebuff-desktop")
}

fn procesos(nombre: &str) -> Vec<u32> {
    let mut cmd = Command::new("tasklist");
    cmd.args(["/FI", &format!("IMAGENAME eq {nombre}"), "/FO", "CSV", "/NH"]);
    let Ok(out) = oculto(&mut cmd).output() else {
        return Vec::new();
    };
    let txt = String::from_utf8_lossy(&out.stdout);
    txt.lines()
        .filter(|l| l.to_lowercase().contains(&nombre.to_lowercase()))
        .filter_map(|l| {
            // "Freebuff.exe","42680","Console",...
            l.split(',').nth(1).map(|s| s.trim_matches('"').to_string())
        })
        .filter_map(|s| s.parse::<u32>().ok())
        .collect()
}

fn puertos_de(pids: &[u32]) -> Vec<String> {
    if pids.is_empty() {
        return Vec::new();
    }
    let mut cmd = Command::new("netstat");
    cmd.args(["-ano"]);
    let Ok(out) = oculto(&mut cmd).output() else {
        return Vec::new();
    };
    let txt = String::from_utf8_lossy(&out.stdout);
    let mut out: Vec<String> = txt
        .lines()
        .filter(|l| l.to_uppercase().contains("LISTENING"))
        .filter_map(|l| {
            let campos: Vec<&str> = l.split_whitespace().collect();
            if campos.len() < 5 {
                return None;
            }
            let pid: u32 = campos[4].parse().ok()?;
            if pids.contains(&pid) {
                Some(campos[1].to_string())
            } else {
                None
            }
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

fn contar_archivos(dir: &Path, nombre: &str, profundo: bool) -> usize {
    let Ok(entradas) = std::fs::read_dir(dir) else {
        return 0;
    };
    let mut total = 0;
    for e in entradas.flatten() {
        let p = e.path();
        if p.is_dir() {
            if profundo {
                total += contar_archivos(&p, nombre, true);
            }
        } else if p.file_name().and_then(|n| n.to_str()) == Some(nombre) {
            total += 1;
        }
    }
    total
}

/* ============================ Freebuff ============================ */

/// Freebuff es una app de escritorio: se detecta por su proceso y su ejecutable,
/// y lo que se puede leer de ella son sus datos en disco (siempre en solo
/// lectura) más los MCP que comparte con KUKIA en `~/.agents/mcp.json`.
pub fn freebuff() -> Nodo {
    let exe = freebuff_exe();
    let instalado = exe.exists();
    let pids = procesos("Freebuff.exe");
    let cfg = freebuff_config();
    let mut nodo = Nodo {
        id: "freebuff".into(),
        nombre: "Freebuff".into(),
        instalado,
        ruta: exe.display().to_string(),
        procesos: pids.len(),
        ..Default::default()
    };

    if !instalado && pids.is_empty() {
        nodo.detalle = "No se encuentra la instalación de Freebuff.".into();
        nodo.limitaciones
            .push("Sin instalación no hay nada que integrar".into());
        return nodo;
    }

    if instalado {
        let mut cmd = Command::new("powershell");
        cmd.args([
            "-NoProfile",
            "-Command",
            &format!(
                "(Get-Item '{}').VersionInfo.ProductVersion",
                exe.display()
            ),
        ]);
        if let Ok(out) = oculto(&mut cmd).output() {
            if out.status.success() {
                nodo.version = String::from_utf8_lossy(&out.stdout).trim().to_string();
            }
        }
    }

    nodo.conectado = nodo.procesos > 0;
    nodo.puertos = puertos_de(&pids);
    nodo.puertos = nodo
        .puertos
        .into_iter()
        .map(|p| format!("{p} (con token)").replace("127.0.0.1:", ""))
        .collect();

    // Proyectos y conversaciones: SQLite en solo lectura, nunca escritura.
    let proyectos = cfg.join("projects");
    let mut n_proyectos = 0usize;
    let mut n_mensajes = 0i64;
    let mut n_hilos = 0i64;
    if let Ok(dirs) = std::fs::read_dir(&proyectos) {
        for d in dirs.flatten() {
            let db = d.path().join("desktop-v2.db");
            if !db.exists() {
                continue;
            }
            n_proyectos += 1;
            if let Ok(con) = rusqlite::Connection::open_with_flags(
                &db,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            ) {
                if let Ok(n) = con.query_row("select count(*) from messages", [], |r| r.get::<_, i64>(0)) {
                    n_mensajes += n;
                }
                if let Ok(n) = con.query_row("select count(*) from threads", [], |r| r.get::<_, i64>(0)) {
                    n_hilos += n;
                }
            }
        }
    }

    let mcp_json = home().join(".agents").join("mcp.json");
    let mcp_nombres: Vec<String> = std::fs::read_to_string(&mcp_json)
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(t.trim_start_matches('\u{feff}')).ok())
        .and_then(|v| v.get("mcpServers").and_then(|m| m.as_object()).map(|m| m.keys().cloned().collect()))
        .unwrap_or_default();

    let skills = contar_archivos(&cfg.join("account-skills"), "SKILL.md", true);

    nodo.modelos = 0; // Freebuff no publica su catálogo de modelos.
    nodo.agentes = 0; // Tampoco sus agentes.
    nodo.herramientas = mcp_nombres.len() + skills;
    nodo.sesiones = n_hilos as usize;
    nodo.detalle = format!(
        "{n_proyectos} proyecto(s) · {n_hilos} hilo(s) · {n_mensajes} mensaje(s) · {} MCP · {skills} skill(s)",
        mcp_nombres.len()
    );

    nodo.puede = vec![
        "Leer proyectos, hilos y mensajes (SQLite en solo lectura)".into(),
        "Compartir los servidores MCP de ~/.agents/mcp.json".into(),
        "Descubrir skills de la cuenta".into(),
        "Abrir sus modelos como proveedor compatible OpenAI desde KUKIA".into(),
    ];
    nodo.limitaciones = vec![
        "No publica API local documentada: sus puertos 34147/34148 exigen token propio y no se usan".into(),
        "KUKIA no puede lanzar agentes ni workflows dentro de Freebuff".into(),
        "No se expone ni se lee ninguna credencial suya".into(),
    ];
    if !nodo.conectado {
        nodo.ultimo_error = Some("Freebuff no está en ejecución ahora mismo".into());
    }
    nodo
}

/* ============================ OpenCode ============================ */

pub async fn opencode(url: Option<String>) -> Nodo {
    let mut nodo = Nodo {
        id: "opencode".into(),
        nombre: "OpenCode".into(),
        ..Default::default()
    };
    let estado = bridge::estado(url.clone()).await;
    nodo.instalado = estado.opencode_instalado;
    nodo.version = estado.version.clone();
    nodo.ruta = estado.binario.clone();
    nodo.conectado = estado.servidor.vivo;
    nodo.sesiones = estado.servidor.sesiones;
    nodo.modelos = if estado.servidor.vivo {
        estado.servidor.modelos
    } else {
        estado.modelos_cli
    };
    nodo.agentes = if estado.servidor.vivo {
        estado.servidor.agentes
    } else {
        estado.agentes_cli
    };
    nodo.herramientas = estado.mcp_config.len();
    nodo.puertos = if estado.servidor.vivo {
        vec![estado.servidor.url.clone()]
    } else {
        Vec::new()
    };
    nodo.detalle = format!(
        "{} modelo(s) · {} agente(s) · {} MCP · config {}",
        nodo.modelos,
        nodo.agentes,
        estado.mcp_config.len(),
        if estado.config_ok { "legible" } else { "no legible" }
    );
    nodo.ultimo_error = estado.servidor.error.clone();

    nodo.puede = vec![
        "Consultar su catálogo real de modelos y agentes".into(),
        "Crear sesiones y pedir respuestas con su API local".into(),
        "Ejecutar agentes suyos (por ejemplo xoxete) con su propio modelo".into(),
        "Recibir los modelos de KUKIA como proveedor compatible OpenAI".into(),
    ];
    nodo.limitaciones = vec![
        "Su configuración no se toca desde KUKIA: el fragmento se genera y lo pegas tú".into(),
        "La ruta v2 /api/session/{id}/prompt devuelve 500 en 1.19.3; se usa /session/{id}/message".into(),
        "Su CLI sin terminal se cuelga (`run` y `agent list`): KUKIA usa su servidor, no el CLI".into(),
    ];
    nodo
}

/* ============================ KUKIA ============================ */

pub async fn kukia(cfg: &AppConfig, datos: &DatosLocales) -> Nodo {
    let informe = catalog::collect(cfg).await;
    let mut por_motor: Vec<Cuenta> = Vec::new();
    for m in &informe.models {
        match por_motor.iter_mut().find(|c| c.nombre == m.engine) {
            Some(c) => c.cantidad += 1,
            None => por_motor.push(Cuenta {
                nombre: m.engine.clone(),
                cantidad: 1,
            }),
        }
    }
    por_motor.sort_by(|a, b| b.cantidad.cmp(&a.cantidad));

    let srv = serve::status();

    let mut nodo = Nodo {
        id: "kukia".into(),
        nombre: "KUKIA CORE".into(),
        instalado: true,
        conectado: true,
        version: env!("CARGO_PKG_VERSION").to_string(),
        ruta: cfg_ruta(),
        modelos: informe.models.len(),
        agentes: datos.agentes,
        herramientas: datos.herramientas,
        sesiones: 0,
        detalle: format!(
            "{} motor(es) · {} herramienta(s) · {} servidor(es) MCP · /v1 {}",
            por_motor.len(),
            datos.herramientas,
            datos.mcp,
            if srv.running {
                format!("en {}", srv.url)
            } else {
                "parado".to_string()
            }
        ),
        ..Default::default()
    };
    nodo.puertos = if srv.running { vec![srv.url.clone()] } else { Vec::new() };
    nodo.puede = vec![
        "Enrutar tareas a cualquier modelo local del catálogo".into(),
        "Servir sus modelos a OpenCode por /v1 (token local)".into(),
        "Leer la memoria de proyectos de Freebuff en solo lectura".into(),
        "Coordinar a los otros dos nodos y registrar el origen".into(),
    ];
    nodo.limitaciones = vec![
        "No puede pilotar Freebuff: no hay API oficial".into(),
        "No modifica la configuración de OpenCode".into(),
    ];
    nodo
}

fn cfg_ruta() -> String {
    std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "kukia-code".into())
}

/* ============================ Ecosistema ============================ */

/// Panel completo. Abre su **propia** conexión a la base de KUKIA por ruta: así
/// no se retiene ningún candado de la aplicación a través de un `await`, que es
/// lo que bloquearía la interfaz.
pub async fn todo(cfg: &AppConfig, db_path: &Path, opencode_url: Option<String>) -> Ecosistema {
    let datos = datos_locales(db_path);
    let k = kukia(cfg, &datos).await;
    let o = opencode(opencode_url.clone()).await;
    let f = freebuff();

    let mut catalogo = Catalogo {
        modelos_locales: k.modelos,
        modelos_opencode: o.modelos,
        agentes_kukia: k.agentes,
        agentes_opencode: o.agentes,
        herramientas_kukia: k.herramientas,
        servidores_mcp: datos.mcp,
        ..Default::default()
    };

    // Desglose por motor de KUKIA y por proveedor de OpenCode.
    let informe = catalog::collect(cfg).await;
    let mut por_motor: Vec<Cuenta> = Vec::new();
    for m in &informe.models {
        match por_motor.iter_mut().find(|c| c.nombre == m.engine) {
            Some(c) => c.cantidad += 1,
            None => por_motor.push(Cuenta {
                nombre: m.engine.clone(),
                cantidad: 1,
            }),
        }
    }
    por_motor.sort_by(|a, b| b.cantidad.cmp(&a.cantidad));
    catalogo.por_motor = por_motor;

    let mut por_proveedor: Vec<Cuenta> = Vec::new();
    for m in bridge::modelos(Some(opencode_url.as_deref().unwrap_or("http://127.0.0.1:4096"))).await {
        match por_proveedor.iter_mut().find(|c| c.nombre == m.provider) {
            Some(c) => c.cantidad += 1,
            None => por_proveedor.push(Cuenta {
                nombre: m.provider.clone(),
                cantidad: 1,
            }),
        }
    }
    por_proveedor.sort_by(|a, b| b.cantidad.cmp(&a.cantidad));
    catalogo.por_proveedor_opencode = por_proveedor;

    let mut notas = Vec::new();
    if let Some(e) = &datos.error {
        notas.push(format!("La base de KUKIA no se pudo abrir ({e}): herramientas MCP fuera del recuento."));
    }
    notas.push(
        "Cada nodo se consulta por separado: si uno falla, los demás siguen respondiendo.".into(),
    );
    if !o.conectado {
        notas.push("OpenCode no tiene servidor vivo: se usa su CLI para el catálogo y no se puede conversar hasta arrancarlo.".into());
    }
    if !f.conectado {
        notas.push("Freebuff no está abierto: sus datos se leen igual de disco.".into());
    }
    notas.push("Ningún nodo comparte credenciales con otro.".into());

    Ecosistema {
        nodos: vec![k, o, f],
        catalogo,
        notas,
    }
}

/// Selecciona el entorno más apropiado para una tarea, según capacidades reales.
///
/// No promete lo que no hay: si la mejor opción es Freebuff, avisa de que hay
/// que abrirlo a mano porque no existe API para lanzarlo desde KUKIA.
pub fn enrutar(tarea: &str, nodos: &[Nodo]) -> Recomendacion {
    let t = tarea.to_lowercase();
    let tiene = |id: &str| nodos.iter().find(|n| n.id == id);
    let opencode_ok = tiene("opencode").map(|n| n.conectado).unwrap_or(false);
    let freebuff_ok = tiene("freebuff").map(|n| n.conectado).unwrap_or(false);

    let clave_codigo = ["codigo", "código", "refactor", "compilar", "test", "bug", "reparar", "git", "parche"];
    let clave_auto = ["automat", "workflow", "flujo", "agente", "tarea larga", "investiga", "busca", "web"];
    let clave_memoria = ["memoria", "recuerdo", "contexto", "coordinar", "orquest", "historial"];

    let es_codigo = clave_codigo.iter().any(|k| t.contains(k));
    let es_auto = clave_auto.iter().any(|k| t.contains(k));
    let es_memoria = clave_memoria.iter().any(|k| t.contains(k));

    if es_codigo {
        return Recomendacion {
            sistema: "opencode".into(),
            motivo: "La tarea es de código: OpenCode tiene agentes y herramientas de desarrollo.".into(),
            automatizable: opencode_ok,
            como: if opencode_ok {
                "Desde el Puente: elige proveedor y modelo, y su agente de programación.".into()
            } else {
                "Arranca su servidor desde el Puente y repite la petición.".into()
            },
            alternativas: vec!["KUKIA con un modelo local de código".into()],
        };
    }
    if es_auto && !es_memoria {
        return Recomendacion {
            sistema: if freebuff_ok { "freebuff".into() } else { "kukia".into() },
            motivo: "Es una tarea de automatización larga con herramientas.".into(),
            automatizable: false,
            como: if freebuff_ok {
                "Freebuff está abierto: lánzala allí. KUKIA no puede pilotarlo (no hay API oficial).".into()
            } else {
                "KUKIA puede preparar los datos y el contexto; para ejecutarla, abre Freebuff.".into()
            },
            alternativas: vec!["OpenCode con su agente general".into(), "KUKIA + tools y jobs".into()],
        };
    }
    Recomendacion {
        sistema: "kukia".into(),
        motivo: if es_memoria {
            "La tarea es de memoria y coordinación: es el centro del ecosistema.".into()
        } else {
            "Por defecto manda KUKIA: enruta al modelo local que mejor encaje.".into()
        },
        automatizable: true,
        como: "Catálogo → enrutador de KUKIA con el modelo elegido.".into(),
        alternativas: vec![
            "OpenCode si necesitas editar código".into(),
            "Freebuff si necesitas sus workflows".into(),
        ],
    }
}

/* ============================ Tests ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_router_manda_codigo_a_opencode() {
        let nodos = vec![
            Nodo { id: "opencode".into(), conectado: true, ..Default::default() },
            Nodo { id: "freebuff".into(), conectado: true, ..Default::default() },
        ];
        let r = enrutar("repara este bug y compila los tests", &nodos);
        assert_eq!(r.sistema, "opencode");
        assert!(r.automatizable);
    }

    #[test]
    fn sin_servidor_de_opencode_no_se_promete_automatizar() {
        let nodos = vec![Nodo { id: "opencode".into(), conectado: false, ..Default::default() }];
        let r = enrutar("refactor de codigo", &nodos);
        assert_eq!(r.sistema, "opencode");
        assert!(!r.automatizable, "sin servidor vivo no hay automatización");
    }

    #[test]
    fn freebuff_nunca_se_declara_automatizable() {
        let nodos = vec![Nodo { id: "freebuff".into(), conectado: true, ..Default::default() }];
        let r = enrutar("automatiza un workflow largo", &nodos);
        assert_eq!(r.sistema, "freebuff");
        assert!(!r.automatizable, "no hay API oficial de Freebuff");
    }

    #[test]
    fn memoria_se_queda_en_kukia() {
        let nodos = vec![Nodo { id: "opencode".into(), conectado: true, ..Default::default() }];
        let r = enrutar("recupera el contexto y coordina", &nodos);
        assert_eq!(r.sistema, "kukia");
        assert!(r.automatizable);
    }

    #[test]
    fn las_rutas_por_defecto_apuntan_a_sitios_reales() {
        assert!(freebuff_exe().to_string_lossy().ends_with("Freebuff.exe"));
        assert!(freebuff_config().to_string_lossy().contains("freebuff-desktop"));
    }

    #[test]
    fn contar_archivos_no_revienta_sin_directorio() {
        assert_eq!(contar_archivos(Path::new("Z:\\no\\existe"), "SKILL.md", true), 0);
    }
}
