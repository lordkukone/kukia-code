//! Herramientas del chat: el catálogo completo de un agente.
//!
//! Cubre lo mismo que hace un agente de código de verdad, en cuatro niveles:
//!
//! | Nivel | Permiso | Aprobación | Ejemplos |
//! |---|---|---|---|
//! | Lectura | `allow_read` | no | leer, listar, buscar por nombre, buscar dentro, árbol, memoria, GPU, logs, auditoría |
//! | Escritura | `allow_write` | **sí** | escribir, añadir, reemplazar, mover, copiar, borrar, recordar |
//! | Ejecución | `allow_shell` | **sí** | PowerShell, git status/diff/log, procesos, descargar de VRAM |
//! | Red | `allow_network` | **sí** | descargar una URL, POST JSON |
//! | MCP | `allow_tools` | **sí** | llamar a una herramienta de un servidor autorizado |
//!
//! Reglas que no se saltan:
//!
//! 1. Solo se ofrece lo que la política del agente permite.
//! 2. Lo que modifica exige, además, aprobación explícita del usuario: sin ella
//!    ni siquiera aparece en el esquema.
//! 3. Cada llamada pasa por `permissions::check` y queda en la tabla `audit`.

use super::ChatRequest;
use crate::config::AppConfig;
use crate::memory::{AgentPolicy, Database};
use crate::permissions::{self, Tool};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

/// Tope de vueltas del bucle por turno.
pub const MAX_PASOS: usize = 6;

/// Caracteres máximos que se devuelven al modelo por herramienta.
const TOPE_SALIDA: usize = 12_000;

/// Tamaño máximo de archivo que se lee de una vez.
const TOPE_ARCHIVO: u64 = 2_000_000;

/// Segundos que puede correr un comando antes de cancelarse.
const ESPERA_COMANDO: u64 = 60;

/// Base de datos compartida con la aplicación.
pub type Db = Arc<Mutex<Database>>;

/* ==================== ¿Sabe este modelo usar herramientas? ==================== */

fn cache_caps() -> &'static Mutex<HashMap<String, Vec<String>>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Vec<String>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Capacidades que el modelo declara en `/api/show`, memorizadas por sesión.
pub async fn capacidades(model: &str, cfg: &AppConfig) -> Vec<String> {
    if let Ok(c) = cache_caps().lock() {
        if let Some(v) = c.get(model) {
            return v.clone();
        }
    }
    let url = format!("{}/api/show", super::base("ollama", cfg));
    let mut caps: Vec<String> = Vec::new();
    if let Ok(r) = super::client(10)
        .post(&url)
        .json(&json!({ "model": model }))
        .send()
        .await
    {
        if let Ok(v) = r.json::<Value>().await {
            if let Some(arr) = v["capabilities"].as_array() {
                for c in arr {
                    if let Some(s) = c.as_str() {
                        caps.push(s.to_string());
                    }
                }
            }
        }
    }
    if let Ok(mut c) = cache_caps().lock() {
        c.insert(model.to_string(), caps.clone());
    }
    caps
}

/// ¿El modelo declara `tools`? Solo entonces se le ofrecen.
pub async fn soporta_tools(model: &str, cfg: &AppConfig) -> bool {
    capacidades(model, cfg).await.iter().any(|c| c == "tools")
}

/* ============================ Política del chat ============================ */

pub fn politica(req: &ChatRequest, db: Option<&Db>) -> AgentPolicy {
    let agente = req
        .agent
        .clone()
        .filter(|a| !a.trim().is_empty())
        .unwrap_or_else(|| "kukia".to_string());
    if let Some(m) = db {
        if let Ok(g) = m.lock() {
            if let Ok(p) = g.policy_get(&agente) {
                return p;
            }
        }
    }
    AgentPolicy::default_for(&agente)
}

/* ============================ Esquemas ofrecidos ============================ */

fn funcion(nombre: &str, descripcion: &str, props: Value, requeridos: &[&str]) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": nombre,
            "description": descripcion,
            "parameters": {
                "type": "object",
                "properties": props,
                "required": requeridos
            }
        }
    })
}

/// Catálogo ofrecido según la política y la aprobación.
pub fn esquemas(policy: &AgentPolicy, aprobado: bool) -> Vec<Value> {
    let mut out: Vec<Value> = Vec::new();

    if policy.allow_read {
        // --- Archivos (solo lectura) ---
        out.push(funcion(
            "files_list",
            "Lista el contenido de una carpeta: tipo y tamaño.",
            json!({ "ruta": { "type": "string", "description": "Carpeta. Vacío = datos de KUKIA." } }),
            &[],
        ));
        out.push(funcion(
            "files_read",
            "Lee un archivo de texto completo.",
            json!({ "ruta": { "type": "string", "description": "Ruta absoluta del archivo." } }),
            &["ruta"],
        ));
        out.push(funcion(
            "files_read_many",
            "Lee varios archivos de una vez y los devuelve separados.",
            json!({ "rutas": { "type": "array", "items": { "type": "string" }, "description": "Rutas absolutas." } }),
            &["rutas"],
        ));
        out.push(funcion(
            "files_find",
            "Busca archivos y carpetas por nombre (* y ?), en profundidad.",
            json!({
                "patron": { "type": "string", "description": "Parte del nombre con comodines, por ejemplo *.rs" },
                "carpeta": { "type": "string", "description": "Carpeta donde buscar." },
                "max": { "type": "integer", "description": "Máximo de resultados (100 por defecto)." }
            }),
            &["patron", "carpeta"],
        ));
        out.push(funcion(
            "files_search",
            "Busca un texto dentro de los archivos y devuelve archivo:línea: contenido.",
            json!({
                "texto": { "type": "string", "description": "Texto a buscar, sin distinguir mayúsculas." },
                "carpeta": { "type": "string", "description": "Carpeta donde buscar." },
                "extension": { "type": "string", "description": "Filtro por extensión: rs, ts… (opcional)." },
                "max": { "type": "integer", "description": "Máximo de coincidencias (40 por defecto)." }
            }),
            &["texto", "carpeta"],
        ));
        out.push(funcion(
            "files_info",
            "Ficha de un archivo o carpeta: tipo, tamaño, fecha y número de líneas.",
            json!({ "ruta": { "type": "string" } }),
            &["ruta"],
        ));
        out.push(funcion(
            "dir_tree",
            "Árbol de una carpeta hasta N niveles, sin listar directorios enormes.",
            json!({
                "carpeta": { "type": "string" },
                "niveles": { "type": "integer", "description": "Profundidad (2 por defecto, máximo 5)." }
            }),
            &["carpeta"],
        ));
        // --- Memoria y estado ---
        out.push(funcion(
            "memory_search",
            "Busca en la memoria persistente de KUKIA.",
            json!({ "consulta": { "type": "string" } }),
            &["consulta"],
        ));
        out.push(funcion(
            "sessions_list",
            "Lista las conversaciones guardadas (título y fecha).",
            json!({}),
            &[],
        ));
        out.push(funcion(
            "tools_list",
            "Lista las herramientas de KUKIA con su estado comprobado.",
            json!({}),
            &[],
        ));
        out.push(funcion(
            "mcp_list",
            "Lista los servidores MCP declarados, si están autorizados y cuántas herramientas tienen.",
            json!({}),
            &[],
        ));
        out.push(funcion(
            "logs_read",
            "Últimas líneas del registro de KUKIA.",
            json!({ "lineas": { "type": "integer", "description": "Cuantas líneas (50 por defecto)." } }),
            &[],
        ));
        out.push(funcion(
            "gpu_stats",
            "VRAM, uso, temperatura y potencia de cada GPU, en tiempo real.",
            json!({}),
            &[],
        ));
        out.push(funcion(
            "sys_info",
            "Sistema: SO, CPU, núcleos, RAM y GPU.",
            json!({}),
            &[],
        ));
        out.push(funcion(
            "disk_usage",
            "Espacio total y libre de cada unidad.",
            json!({}),
            &[],
        ));
        out.push(funcion(
            "models_loaded",
            "Modelos cargados ahora mismo en Ollama, con su contexto.",
            json!({}),
            &[],
        ));
        out.push(funcion(
            "vram_report",
            "Informe de VRAM: quién ocupa cada GPU y qué se puede liberar.",
            json!({}),
            &[],
        ));
        out.push(funcion(
            "time_now",
            "Fecha y hora local del equipo.",
            json!({}),
            &[],
        ));
    }

    if policy.allow_write && aprobado {
        out.push(funcion(
            "files_write",
            "Escribe un archivo completo (crea carpetas). Requiere aprobación.",
            json!({
                "ruta": { "type": "string" },
                "contenido": { "type": "string" }
            }),
            &["ruta", "contenido"],
        ));
        out.push(funcion(
            "files_append",
            "Añade texto al final de un archivo. Requiere aprobación.",
            json!({
                "ruta": { "type": "string" },
                "contenido": { "type": "string" }
            }),
            &["ruta", "contenido"],
        ));
        out.push(funcion(
            "files_replace",
            "Reemplaza un texto exacto y único dentro de un archivo. Requiere aprobación.",
            json!({
                "ruta": { "type": "string" },
                "buscar": { "type": "string" },
                "reemplazar": { "type": "string" }
            }),
            &["ruta", "buscar", "reemplazar"],
        ));
        out.push(funcion(
            "files_copy",
            "Copia un archivo o carpeta. Requiere aprobación.",
            json!({ "origen": { "type": "string" }, "destino": { "type": "string" } }),
            &["origen", "destino"],
        ));
        out.push(funcion(
            "files_move",
            "Mueve o renombra un archivo o carpeta. Requiere aprobación.",
            json!({ "origen": { "type": "string" }, "destino": { "type": "string" } }),
            &["origen", "destino"],
        ));
        out.push(funcion(
            "files_delete",
            "Borra un archivo o una carpeta. Requiere aprobación.",
            json!({
                "ruta": { "type": "string" },
                "recursivo": { "type": "boolean", "description": "Para borrar una carpeta con contenido." }
            }),
            &["ruta"],
        ));
        out.push(funcion(
            "mkdir",
            "Crea una carpeta y las que falten. Requiere aprobación.",
            json!({ "ruta": { "type": "string" } }),
            &["ruta"],
        ));
        out.push(funcion(
            "memory_add",
            "Guarda un recuerdo en la memoria de KUKIA. Requiere aprobación.",
            json!({
                "contenido": { "type": "string" },
                "etiquetas": { "type": "string", "description": "Etiquetas separadas por comas (opcional)." }
            }),
            &["contenido"],
        ));
    }

    if policy.allow_shell && aprobado {
        out.push(funcion(
            "terminal_run",
            "Ejecuta un comando de PowerShell (60 s máximo). Requiere aprobación.",
            json!({
                "comando": { "type": "string" },
                "carpeta": { "type": "string", "description": "Carpeta de trabajo (opcional)." }
            }),
            &["comando"],
        ));
        out.push(funcion(
            "git_status",
            "Estado del repositorio git de una carpeta.",
            json!({ "carpeta": { "type": "string" } }),
            &["carpeta"],
        ));
        out.push(funcion(
            "git_diff",
            "Cambios sin confirmar en una carpeta, opcionalmente de un archivo.",
            json!({
                "carpeta": { "type": "string" },
                "archivo": { "type": "string", "description": "Ruta relativa (opcional)." }
            }),
            &["carpeta"],
        ));
        out.push(funcion(
            "git_log",
            "Últimos commits de una carpeta.",
            json!({
                "carpeta": { "type": "string" },
                "n": { "type": "integer", "description": "Cuantos commits (10 por defecto)." }
            }),
            &["carpeta"],
        ));
        out.push(funcion(
            "processes_list",
            "Procesos que más memoria usan, con su PID.",
            json!({ "max": { "type": "integer", "description": "Cuantos mostrar (20 por defecto)." } }),
            &[],
        ));
        out.push(funcion(
            "vram_unload",
            "Descarga un modelo de la VRAM. Requiere aprobación.",
            json!({
                "motor": { "type": "string", "description": "ollama, lmstudio, comfyui…" },
                "modelo": { "type": "string" }
            }),
            &["motor", "modelo"],
        ));
    }

    if policy.allow_network && aprobado {
        out.push(funcion(
            "web_fetch",
            "Descarga una URL https y devuelve su texto. Requiere aprobación.",
            json!({ "url": { "type": "string" } }),
            &["url"],
        ));
        out.push(funcion(
            "web_post",
            "Envía un JSON por POST y devuelve la respuesta. Requiere aprobación.",
            json!({
                "url": { "type": "string" },
                "cuerpo": { "type": "object", "description": "Objeto JSON que se envía." }
            }),
            &["url", "cuerpo"],
        ));
    }

    if policy.allow_tools && aprobado {
        out.push(funcion(
            "mcp_call",
            "Llama a una herramienta de un servidor MCP autorizado. Requiere aprobación.",
            json!({
                "servidor": { "type": "string" },
                "herramienta": { "type": "string" },
                "argumentos": { "type": "object", "description": "Argumentos según el esquema de la herramienta." }
            }),
            &["servidor", "herramienta"],
        ));
    }

    out
}

/* ============================ Ejecución ============================ */

pub async fn ejecutar(
    nombre: &str,
    args: &Value,
    policy: &AgentPolicy,
    aprobado: bool,
    db: Option<&Db>,
) -> Result<String, String> {
    let res = match nombre {
        /* ---------------- Lectura de archivos ---------------- */
        "files_list" => {
            let ruta = texto(args, "ruta");
            let ruta = if ruta.trim().is_empty() {
                crate::config::data_dir().display().to_string()
            } else {
                ruta
            };
            permitir(policy, Tool::Read, &ruta, aprobado)?;
            listar(&ruta)
        }
        "files_read" => {
            let ruta = texto(args, "ruta");
            if ruta.trim().is_empty() {
                return Err("falta el argumento «ruta»".into());
            }
            permitir(policy, Tool::Read, &ruta, aprobado)?;
            leer(&ruta)
        }
        "files_read_many" => {
            let rutas: Vec<String> = args
                .get("rutas")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            if rutas.is_empty() {
                return Err("falta el argumento «rutas»".into());
            }
            let mut out = String::new();
            for r in rutas.iter().take(12) {
                permitir(policy, Tool::Read, r, aprobado)?;
                out.push_str(&format!("\n===== {r} =====\n"));
                match leer(r) {
                    Ok(t) => out.push_str(&t),
                    Err(e) => out.push_str(&format!("(error: {e})")),
                }
            }
            Ok(recortar(out.trim_start()))
        }
        "files_find" => {
            let patron = texto(args, "patron");
            let carpeta = texto(args, "carpeta");
            if patron.trim().is_empty() || carpeta.trim().is_empty() {
                return Err("hacen falta «patron» y «carpeta»".into());
            }
            permitir(policy, Tool::Read, &carpeta, aprobado)?;
            buscar_nombres(&carpeta, &patron, numero(args, "max", 100))
        }
        "files_search" => {
            let buscado = texto(args, "texto");
            let carpeta = texto(args, "carpeta");
            if buscado.trim().is_empty() || carpeta.trim().is_empty() {
                return Err("hacen falta «texto» y «carpeta»".into());
            }
            permitir(policy, Tool::Read, &carpeta, aprobado)?;
            buscar_dentro(&carpeta, &buscado, &texto(args, "extension"), numero(args, "max", 40))
        }
        "files_info" => {
            let ruta = texto(args, "ruta");
            if ruta.trim().is_empty() {
                return Err("falta el argumento «ruta»".into());
            }
            permitir(policy, Tool::Read, &ruta, aprobado)?;
            ficha(&ruta)
        }
        "dir_tree" => {
            let carpeta = texto(args, "carpeta");
            if carpeta.trim().is_empty() {
                return Err("falta el argumento «carpeta»".into());
            }
            permitir(policy, Tool::Read, &carpeta, aprobado)?;
            arbol(&carpeta, numero(args, "niveles", 2).min(5))
        }

        /* ---------------- Memoria y estado del sistema ---------------- */
        "memory_search" => {
            let q = texto(args, "consulta");
            if q.trim().is_empty() {
                return Err("falta el argumento «consulta»".into());
            }
            let Some(m) = db else {
                return Err("la memoria no está disponible en este contexto".into());
            };
            let g = m.lock().map_err(|_| "la base de datos está ocupada".to_string())?;
            let mut v = g.memory_search(&q)?;
            if v.is_empty() {
                Ok("(sin resultados)".to_string())
            } else {
                v.truncate(20);
                Ok(recortar(&v.join("\n---\n")))
            }
        }
        "sessions_list" => {
            let Some(m) = db else {
                return Err("las sesiones no están disponibles en este contexto".into());
            };
            let g = m.lock().map_err(|_| "la base de datos está ocupada".to_string())?;
            let s = g.sessions_list()?;
            let lineas: Vec<String> = s
                .iter()
                .take(30)
                .map(|x| format!("{} · {}", x.title, x.updated_at))
                .collect();
            Ok(if lineas.is_empty() {
                "(sin conversaciones)".to_string()
            } else {
                recortar(&lineas.join("\n"))
            })
        }
        "tools_list" => {
            let Some(m) = db else {
                return Err("el registro de herramientas necesita la base".into());
            };
            let g = m.lock().map_err(|_| "la base de datos está ocupada".to_string())?;
            let r = crate::tools::registro(&g);
            let lineas: Vec<String> = r
                .iter()
                .map(|t| format!("{} · {} · {}", t.id, t.state, t.detail))
                .collect();
            Ok(recortar(&lineas.join("\n")))
        }
        "mcp_list" => {
            let Some(m) = db else {
                return Err("los servidores MCP necesitan la base".into());
            };
            let g = m.lock().map_err(|_| "la base de datos está ocupada".to_string())?;
            let s = crate::mcp::servidores(&g);
            let lineas: Vec<String> = s
                .iter()
                .map(|x| {
                    format!(
                        "{} ({}) · {} · autorizado={} · {} herramienta(s)",
                        x.name, x.transport, x.source, x.authorized, x.tools
                    )
                })
                .collect();
            Ok(if lineas.is_empty() {
                "no hay ningún mcp.json con servidores".to_string()
            } else {
                recortar(&lineas.join("\n"))
            })
        }
        "logs_read" => {
            let n = numero(args, "lineas", 50).min(300);
            let l = crate::logs::tail(n);
            Ok(if l.is_empty() {
                "(registro vacío)".to_string()
            } else {
                recortar(&l.join("\n"))
            })
        }
        "gpu_stats" => {
            let st = crate::gpu::stats();
            if st.is_empty() {
                Err("nvidia-smi no devolvió ninguna GPU".into())
            } else {
                serde_json::to_string_pretty(&st).map_err(|e| e.to_string())
            }
        }
        "sys_info" => {
            let i = crate::gpu::system_info();
            serde_json::to_string_pretty(&i).map_err(|e| e.to_string())
        }
        "disk_usage" => {
            let discos = sysinfo::Disks::new_with_refreshed_list();
            let mut lineas = Vec::new();
            for d in discos.list() {
                let total = d.total_space() as f64 / 1e9;
                let libre = d.available_space() as f64 / 1e9;
                lineas.push(format!(
                    "{} · {:.1} GB libres de {:.1} GB",
                    d.mount_point().display(),
                    libre,
                    total
                ));
            }
            Ok(if lineas.is_empty() {
                "(sin unidades)".to_string()
            } else {
                lineas.join("\n")
            })
        }
        "models_loaded" => cargados().await,
        "vram_report" => {
            let r = crate::vram::report().await;
            serde_json::to_string_pretty(&r).map_err(|e| e.to_string())
        }
        "time_now" => Ok(chrono::Local::now()
            .format("%Y-%m-%d %H:%M:%S %A")
            .to_string()),

        /* ---------------- Escritura ---------------- */
        "files_write" => {
            let ruta = texto(args, "ruta");
            let contenido = texto(args, "contenido");
            if ruta.trim().is_empty() {
                return Err("falta el argumento «ruta»".into());
            }
            permitir(policy, Tool::Write, &ruta, aprobado)?;
            escribir(&ruta, &contenido)
        }
        "files_append" => {
            let ruta = texto(args, "ruta");
            let contenido = texto(args, "contenido");
            if ruta.trim().is_empty() {
                return Err("falta el argumento «ruta»".into());
            }
            permitir(policy, Tool::Write, &ruta, aprobado)?;
            anadir(&ruta, &contenido)
        }
        "files_replace" => {
            let ruta = texto(args, "ruta");
            let buscar = texto(args, "buscar");
            let reemplazar = texto(args, "reemplazar");
            if ruta.trim().is_empty() || buscar.is_empty() {
                return Err("hacen falta «ruta» y «buscar»".into());
            }
            permitir(policy, Tool::Write, &ruta, aprobado)?;
            reemplazar_en(&ruta, &buscar, &reemplazar)
        }
        "files_copy" => {
            let origen = texto(args, "origen");
            let destino = texto(args, "destino");
            permitir(policy, Tool::Read, &origen, aprobado)?;
            permitir(policy, Tool::Write, &destino, aprobado)?;
            copiar(&origen, &destino)
        }
        "files_move" => {
            let origen = texto(args, "origen");
            let destino = texto(args, "destino");
            permitir(policy, Tool::Read, &origen, aprobado)?;
            permitir(policy, Tool::Write, &destino, aprobado)?;
            mover(&origen, &destino)
        }
        "files_delete" => {
            let ruta = texto(args, "ruta");
            if ruta.trim().is_empty() {
                return Err("falta el argumento «ruta»".into());
            }
            if es_raiz(&ruta) {
                return Err("no se borra una raíz de unidad".into());
            }
            permitir(policy, Tool::Write, &ruta, aprobado)?;
            borrar(&ruta, args.get("recursivo").and_then(|v| v.as_bool()).unwrap_or(false))
        }
        "mkdir" => {
            let ruta = texto(args, "ruta");
            if ruta.trim().is_empty() {
                return Err("falta el argumento «ruta»".into());
            }
            permitir(policy, Tool::Write, &ruta, aprobado)?;
            std::fs::create_dir_all(&ruta)
                .map(|_| format!("carpeta creada: {ruta}"))
                .map_err(|e| format!("no se pudo crear {ruta}: {e}"))
        }
        "memory_add" => {
            if !policy.allow_write || !aprobado {
                return Err("guardar recuerdos requiere permiso de escritura y aprobación".into());
            }
            let contenido = texto(args, "contenido");
            if contenido.trim().is_empty() {
                return Err("falta el argumento «contenido»".into());
            }
            let Some(m) = db else {
                return Err("la memoria no está disponible en este contexto".into());
            };
            let etiquetas = texto(args, "etiquetas");
            let g = m.lock().map_err(|_| "la base de datos está ocupada".to_string())?;
            g.memory_add(&contenido, &etiquetas)?;
            Ok("recuerdo guardado".to_string())
        }

        /* ---------------- Ejecución ---------------- */
        "terminal_run" => {
            let cmd = texto(args, "comando");
            if cmd.trim().is_empty() {
                return Err("falta el argumento «comando»".into());
            }
            permitir(policy, Tool::Shell, &cmd, aprobado)?;
            let cwd = texto(args, "carpeta");
            if !cwd.trim().is_empty() {
                let c = permissions::check_cwd_in_scope(policy, &cwd);
                if !c.allowed {
                    return Err(c.reason);
                }
            }
            comando(&cmd, &cwd).await
        }
        "git_status" => {
            let carpeta = texto(args, "carpeta");
            permitir(policy, Tool::Shell, "git status", aprobado)?;
            programa("git", &["status", "--short", "--branch"], &carpeta).await
        }
        "git_diff" => {
            let carpeta = texto(args, "carpeta");
            permitir(policy, Tool::Shell, "git diff", aprobado)?;
            let archivo = texto(args, "archivo");
            let mut a = vec!["diff", "--stat", "-p"];
            if !archivo.trim().is_empty() {
                a.push(archivo.trim());
            }
            programa("git", &a, &carpeta).await
        }
        "git_log" => {
            let carpeta = texto(args, "carpeta");
            permitir(policy, Tool::Shell, "git log", aprobado)?;
            let n = numero(args, "n", 10).to_string();
            programa("git", &["log", "-n", &n, "--pretty=format:%h %ad %s", "--date=short"], &carpeta).await
        }
        "processes_list" => procesos(numero(args, "max", 20)),
        "vram_unload" => {
            let motor = texto(args, "motor");
            let modelo = texto(args, "modelo");
            if motor.trim().is_empty() || modelo.trim().is_empty() {
                return Err("hacen falta «motor» y «modelo»".into());
            }
            permitir(policy, Tool::Shell, &format!("descargar {motor}/{modelo}"), aprobado)?;
            let r = crate::vram::unload(&motor, &modelo, false).await;
            serde_json::to_string_pretty(&r).map_err(|e| e.to_string())
        }

        /* ---------------- Red ---------------- */
        "web_fetch" => {
            let url = texto(args, "url");
            if !url.starts_with("https://") {
                return Err("solo se permite https".into());
            }
            permitir(policy, Tool::Network, &url, aprobado)?;
            traer(&url).await
        }
        "web_post" => {
            let url = texto(args, "url");
            if !url.starts_with("https://") {
                return Err("solo se permite https".into());
            }
            permitir(policy, Tool::Network, &url, aprobado)?;
            let cuerpo = args.get("cuerpo").cloned().unwrap_or(json!({}));
            enviar(&url, &cuerpo).await
        }

        /* ---------------- MCP ---------------- */
        "mcp_call" => {
            let servidor = texto(args, "servidor");
            let herramienta = texto(args, "herramienta");
            if servidor.trim().is_empty() || herramienta.trim().is_empty() {
                return Err("hacen falta «servidor» y «herramienta»".into());
            }
            permitir(policy, Tool::ExternalTools, &servidor, aprobado)?;
            let Some(m) = db else {
                return Err("MCP necesita la base para comprobar la autorización".into());
            };
            {
                let g = m.lock().map_err(|_| "la base de datos está ocupada".to_string())?;
                if !crate::mcp::esta_autorizado(&g, &servidor) {
                    return Err(format!(
                        "«{servidor}» no está autorizado en KUKIA: no se lanza un proceso de terceros sin permiso"
                    ));
                }
            }
            let argumentos = args.get("argumentos").cloned().unwrap_or(json!({}));
            let r = crate::mcp::llamar(&servidor, &herramienta, argumentos).await?;
            Ok(recortar(&r.to_string()))
        }

        otro => Err(format!("herramienta desconocida: {otro}")),
    };

    auditar(db, policy, nombre, args, aprobado, &res);
    res
}

/* ============================ Permisos y auditoría ============================ */

fn permitir(policy: &AgentPolicy, tool: Tool, target: &str, aprobado: bool) -> Result<(), String> {
    let c = permissions::check(policy, tool, target, aprobado);
    if c.allowed {
        Ok(())
    } else {
        Err(c.reason)
    }
}

fn auditar(
    db: Option<&Db>,
    policy: &AgentPolicy,
    nombre: &str,
    args: &Value,
    aprobado: bool,
    res: &Result<String, String>,
) {
    let Some(m) = db else { return };
    let target = args
        .get("ruta")
        .or_else(|| args.get("carpeta"))
        .or_else(|| args.get("origen"))
        .or_else(|| args.get("comando"))
        .or_else(|| args.get("url"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let (estado, err) = match res {
        Ok(_) => ("ok", ""),
        Err(e) => ("error", e.as_str()),
    };
    if let Ok(g) = m.lock() {
        let _ = g.audit_log(
            &policy.agent,
            nombre,
            "tool.call",
            target,
            aprobado,
            estado,
            err,
        );
    }
}

/* ============================ Utilidades ============================ */

fn texto(args: &Value, clave: &str) -> String {
    args.get(clave)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

fn numero(args: &Value, clave: &str, por_defecto: usize) -> usize {
    args.get(clave)
        .and_then(|v| v.as_u64())
        .map(|n| n as usize)
        .filter(|n| *n > 0)
        .unwrap_or(por_defecto)
        .min(500)
}

fn recortar(s: &str) -> String {
    if s.chars().count() <= TOPE_SALIDA {
        return s.to_string();
    }
    let corto: String = s.chars().take(TOPE_SALIDA).collect();
    format!("{corto}\n…(recortado a {TOPE_SALIDA} caracteres)")
}

fn es_raiz(ruta: &str) -> bool {
    let t = ruta.trim().trim_end_matches(['\\', '/']);
    t.len() <= 2 || t.is_empty()
}

/* ============================ Archivos ============================ */

fn listar(ruta: &str) -> Result<String, String> {
    let p = Path::new(ruta);
    let rd = std::fs::read_dir(p).map_err(|e| format!("no se pudo listar {ruta}: {e}"))?;
    let mut lineas: Vec<String> = Vec::new();
    for e in rd.flatten() {
        let (tipo, tam) = match e.metadata() {
            Ok(m) => (if m.is_dir() { "dir " } else { "file" }, m.len()),
            Err(_) => ("?   ", 0),
        };
        lineas.push(format!("{tipo} {tam:>12}  {}", e.file_name().to_string_lossy()));
    }
    lineas.sort();
    if lineas.is_empty() {
        return Ok("(carpeta vacía)".into());
    }
    Ok(recortar(&lineas.join("\n")))
}

fn leer(ruta: &str) -> Result<String, String> {
    let meta = std::fs::metadata(ruta).map_err(|e| format!("no se pudo leer {ruta}: {e}"))?;
    if meta.len() > TOPE_ARCHIVO {
        return Err(format!(
            "{ruta} es demasiado grande para el chat ({} bytes)",
            meta.len()
        ));
    }
    let bytes = std::fs::read(ruta).map_err(|e| format!("no se pudo leer {ruta}: {e}"))?;
    Ok(recortar(&String::from_utf8_lossy(&bytes)))
}

fn ficha(ruta: &str) -> Result<String, String> {
    let meta = std::fs::metadata(ruta).map_err(|e| format!("no se pudo leer {ruta}: {e}"))?;
    let tipo = if meta.is_dir() { "carpeta" } else { "archivo" };
    let modificado = meta
        .modified()
        .ok()
        .map(|t| chrono::DateTime::<chrono::Local>::from(t).format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "?".into());
    let mut out = format!(
        "{tipo} · {} bytes · modificado {modificado} · {ruta}",
        meta.len()
    );
    if meta.is_file() && meta.len() <= TOPE_ARCHIVO {
        if let Ok(b) = std::fs::read(ruta) {
            let t = String::from_utf8_lossy(&b);
            if !t.contains('\u{0}') {
                out.push_str(&format!(" · {} línea(s)", t.lines().count()));
            } else {
                out.push_str(" · binario");
            }
        }
    }
    if meta.is_dir() {
        if let Ok(rd) = std::fs::read_dir(ruta) {
            let n = rd.count();
            out.push_str(&format!(" · {n} entrada(s)"));
        }
    }
    Ok(out)
}

fn arbol(carpeta: &str, niveles: usize) -> Result<String, String> {
    let raiz = PathBuf::from(carpeta);
    if !raiz.is_dir() {
        return Err(format!("{carpeta} no es una carpeta"));
    }
    let mut out = String::new();
    let mut pila: Vec<(PathBuf, usize)> = vec![(raiz.clone(), 0)];
    let mut contados = 0usize;
    while let Some((dir, nivel)) = pila.pop() {
        if nivel >= niveles || contados > 400 {
            continue;
        }
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        let mut entradas: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
        entradas.sort();
        for p in entradas {
            if p.file_name()
                .map(|n| n.to_string_lossy().starts_with('.'))
                .unwrap_or(false)
            {
                continue;
            }
            let rel = p.strip_prefix(&raiz).unwrap_or(&p);
            let sangria = "  ".repeat(nivel);
            let marca = if p.is_dir() { "/" } else { "" };
            out.push_str(&format!("{sangria}{}{marca}\n", rel.display()));
            contados += 1;
            if p.is_dir() {
                pila.push((p, nivel + 1));
            }
            if contados > 400 {
                break;
            }
        }
    }
    if out.is_empty() {
        return Ok("(carpeta vacía)".into());
    }
    Ok(recortar(out.trim_end()))
}

fn escribir(ruta: &str, contenido: &str) -> Result<String, String> {
    if let Some(dir) = Path::new(ruta).parent() {
        if !dir.as_os_str().is_empty() {
            std::fs::create_dir_all(dir)
                .map_err(|e| format!("no se pudo crear {}: {e}", dir.display()))?;
        }
    }
    std::fs::write(ruta, contenido).map_err(|e| format!("no se pudo escribir {ruta}: {e}"))?;
    Ok(format!("escritos {} bytes en {ruta}", contenido.len()))
}

fn anadir(ruta: &str, contenido: &str) -> Result<String, String> {
    use std::io::Write;
    if let Some(dir) = Path::new(ruta).parent() {
        if !dir.as_os_str().is_empty() {
            let _ = std::fs::create_dir_all(dir);
        }
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(ruta)
        .map_err(|e| format!("no se pudo abrir {ruta}: {e}"))?;
    f.write_all(contenido.as_bytes())
        .map_err(|e| format!("no se pudo escribir en {ruta}: {e}"))?;
    Ok(format!("añadidos {} bytes a {ruta}", contenido.len()))
}

fn reemplazar_en(ruta: &str, buscar: &str, nuevo: &str) -> Result<String, String> {
    let bytes = std::fs::read(ruta).map_err(|e| format!("no se pudo leer {ruta}: {e}"))?;
    let actual = String::from_utf8_lossy(&bytes).to_string();
    let veces = actual.matches(buscar).count();
    if veces == 0 {
        return Err(format!("«{buscar}» no aparece en {ruta}"));
    }
    if veces > 1 {
        return Err(format!(
            "«{buscar}» aparece {veces} veces en {ruta}: incluye más contexto para que sea único"
        ));
    }
    let cambiado = actual.replacen(buscar, nuevo, 1);
    std::fs::write(ruta, &cambiado).map_err(|e| format!("no se pudo escribir {ruta}: {e}"))?;
    Ok(format!("reemplazado 1 fragmento en {ruta}"))
}

fn copiar(origen: &str, destino: &str) -> Result<String, String> {
    let o = Path::new(origen);
    if !o.exists() {
        return Err(format!("{origen} no existe"));
    }
    if o.is_dir() {
        copiar_dir(o, Path::new(destino))?;
        Ok(format!("carpeta copiada: {origen} → {destino}"))
    } else {
        if let Some(d) = Path::new(destino).parent() {
            let _ = std::fs::create_dir_all(d);
        }
        std::fs::copy(o, destino).map_err(|e| format!("no se pudo copiar: {e}"))?;
        Ok(format!("archivo copiado: {origen} → {destino}"))
    }
}

fn copiar_dir(origen: &Path, destino: &Path) -> Result<(), String> {
    std::fs::create_dir_all(destino).map_err(|e| e.to_string())?;
    for e in std::fs::read_dir(origen)
        .map_err(|e| e.to_string())?
        .flatten()
    {
        let p = e.path();
        let d = destino.join(e.file_name());
        if p.is_dir() {
            copiar_dir(&p, &d)?;
        } else {
            std::fs::copy(&p, &d).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn mover(origen: &str, destino: &str) -> Result<String, String> {
    if !Path::new(origen).exists() {
        return Err(format!("{origen} no existe"));
    }
    if let Some(d) = Path::new(destino).parent() {
        if !d.as_os_str().is_empty() {
            let _ = std::fs::create_dir_all(d);
        }
    }
    std::fs::rename(origen, destino).map_err(|e| format!("no se pudo mover: {e}"))?;
    Ok(format!("movido: {origen} → {destino}"))
}

fn borrar(ruta: &str, recursivo: bool) -> Result<String, String> {
    let p = Path::new(ruta);
    if !p.exists() {
        return Err(format!("{ruta} no existe"));
    }
    if p.is_dir() {
        let vacia = std::fs::read_dir(p)
            .map(|mut r| r.next().is_none())
            .unwrap_or(false);
        if !vacia && !recursivo {
            return Err(format!(
                "{ruta} no está vacía: pide «recursivo» si de verdad quieres borrarla"
            ));
        }
        if recursivo {
            std::fs::remove_dir_all(p).map_err(|e| format!("no se pudo borrar {ruta}: {e}"))?;
        } else {
            std::fs::remove_dir(p).map_err(|e| format!("no se pudo borrar {ruta}: {e}"))?;
        }
    } else {
        std::fs::remove_file(p).map_err(|e| format!("no se pudo borrar {ruta}: {e}"))?;
    }
    Ok(format!("borrado: {ruta}"))
}

fn coincide(nombre: &str, patron: &str) -> bool {
    let n: Vec<char> = nombre.to_lowercase().chars().collect();
    let p: Vec<char> = patron.to_lowercase().chars().collect();
    let (mut i, mut j) = (0usize, 0usize);
    let mut estrella: Option<usize> = None;
    let mut marca = 0usize;
    while i < n.len() {
        if j < p.len() && (p[j] == '?' || p[j] == n[i]) {
            i += 1;
            j += 1;
        } else if j < p.len() && p[j] == '*' {
            estrella = Some(j);
            marca = i;
            j += 1;
        } else if let Some(s) = estrella {
            j = s + 1;
            marca += 1;
            i = marca;
        } else {
            return false;
        }
    }
    while j < p.len() && p[j] == '*' {
        j += 1;
    }
    j == p.len()
}

fn recorrer(carpeta: &str, tope: usize) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let mut archivos = Vec::new();
    let mut carpetas = Vec::new();
    let raiz = PathBuf::from(carpeta);
    let mut pila: Vec<(PathBuf, usize)> = vec![(raiz, 0)];
    while let Some((dir, nivel)) = pila.pop() {
        if nivel > 6 {
            continue;
        }
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            let nombre = e.file_name().to_string_lossy().to_string();
            if nombre.starts_with('.') {
                continue;
            }
            if p.is_dir() {
                carpetas.push(p.clone());
                pila.push((p, nivel + 1));
            } else {
                archivos.push(p);
            }
            if archivos.len() + carpetas.len() > tope * 40 {
                return (archivos, carpetas);
            }
        }
    }
    (archivos, carpetas)
}

fn buscar_nombres(carpeta: &str, patron: &str, max: usize) -> Result<String, String> {
    let (archivos, carpetas) = recorrer(carpeta, max);
    let mut out: Vec<String> = Vec::new();
    for p in carpetas.iter().chain(archivos.iter()) {
        let nombre = p
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if coincide(&nombre, patron) {
            let tipo = if p.is_dir() { "dir " } else { "file" };
            out.push(format!("{tipo} {}", p.display()));
            if out.len() >= max {
                break;
            }
        }
    }
    if out.is_empty() {
        return Ok(format!("nada coincide con «{patron}» en {carpeta}"));
    }
    Ok(recortar(&out.join("\n")))
}

fn buscar_dentro(carpeta: &str, buscado: &str, extension: &str, max: usize) -> Result<String, String> {
    let (archivos, _) = recorrer(carpeta, max);
    let aguja = buscado.to_lowercase();
    let mut out: Vec<String> = Vec::new();
    let mut revisados = 0usize;

    for p in &archivos {
        if revisados > 2_000 || out.len() >= max {
            break;
        }
        if !extension.trim().is_empty() {
            let quieren = extension.trim().trim_start_matches('.').to_lowercase();
            let tiene = p
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if tiene != quieren {
                continue;
            }
        }
        let Ok(meta) = p.metadata() else { continue };
        if meta.len() > TOPE_ARCHIVO {
            continue;
        }
        revisados += 1;
        let Ok(bytes) = std::fs::read(p) else { continue };
        let contenido = String::from_utf8_lossy(&bytes);
        if contenido.contains('\u{0}') {
            continue;
        }
        for (n, linea) in contenido.lines().enumerate() {
            if linea.to_lowercase().contains(&aguja) {
                out.push(format!(
                    "{}:{}: {}",
                    p.display(),
                    n + 1,
                    linea.trim().chars().take(200).collect::<String>()
                ));
                break;
            }
        }
    }

    if out.is_empty() {
        return Ok(format!(
            "«{buscado}» no aparece en {revisados} archivo(s) de {carpeta}"
        ));
    }
    Ok(recortar(&out.join("\n")))
}

/* ============================ Sistema ============================ */

async fn cargados() -> Result<String, String> {
    let r = super::client(6)
        .get("http://127.0.0.1:11434/api/ps")
        .send()
        .await
        .map_err(|e| format!("Ollama no responde: {e}"))?;
    let v: Value = r.json().await.map_err(|e| e.to_string())?;
    let mut lineas = Vec::new();
    for m in v["models"].as_array().cloned().unwrap_or_default() {
        lineas.push(format!(
            "{} · {} tokens · {:.1} GiB en VRAM",
            m["name"].as_str().unwrap_or("?"),
            m["context_length"].as_u64().unwrap_or(0),
            m["size_vram"].as_u64().unwrap_or(0) as f64 / 1_073_741_824.0
        ));
    }
    Ok(if lineas.is_empty() {
        "ningún modelo cargado".to_string()
    } else {
        lineas.join("\n")
    })
}

fn procesos(max: usize) -> Result<String, String> {
    use sysinfo::System;
    let mut sys = System::new_all();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    let mut lista: Vec<(u64, String, u32)> = sys
        .processes()
        .iter()
        .map(|(pid, p)| {
            (
                p.memory(),
                p.name().to_string_lossy().to_string(),
                pid.as_u32(),
            )
        })
        .collect();
    lista.sort_by(|a, b| b.0.cmp(&a.0));
    lista.truncate(max);
    let lineas: Vec<String> = lista
        .iter()
        .map(|(m, n, pid)| format!("{pid:>7}  {:>8.1} MB  {n}", *m as f64 / 1_048_576.0))
        .collect();
    Ok(lineas.join("\n"))
}

async fn comando(cmd: &str, cwd: &str) -> Result<String, String> {
    let mut c = tokio::process::Command::new("powershell");
    c.args(["-NoProfile", "-NonInteractive", "-Command", cmd]);
    if !cwd.trim().is_empty() {
        c.current_dir(cwd);
    }
    salida(c).await
}

async fn programa(prog: &str, args: &[&str], cwd: &str) -> Result<String, String> {
    let mut c = tokio::process::Command::new(prog);
    c.args(args);
    if !cwd.trim().is_empty() {
        c.current_dir(cwd);
    }
    salida(c).await
}

async fn salida(mut c: tokio::process::Command) -> Result<String, String> {
    c.stdout(Stdio::piped()).stderr(Stdio::piped());
    let out = tokio::time::timeout(Duration::from_secs(ESPERA_COMANDO), c.output())
        .await
        .map_err(|_| format!("el comando pasó de {ESPERA_COMANDO} s y se canceló"))?
        .map_err(|e| format!("no se pudo ejecutar: {e}"))?;
    let mut s = String::from_utf8_lossy(&out.stdout).to_string();
    let err = String::from_utf8_lossy(&out.stderr).to_string();
    if !err.trim().is_empty() {
        s.push_str("\n[stderr]\n");
        s.push_str(&err);
    }
    if !out.status.success() {
        s.push_str(&format!("\n[código de salida: {:?}]", out.status.code()));
    }
    if s.trim().is_empty() {
        s = "(sin salida)".into();
    }
    Ok(recortar(&s))
}

/* ============================ Red ============================ */

async fn traer(url: &str) -> Result<String, String> {
    let r = super::client(30)
        .get(url)
        .header("User-Agent", "KUKIA-CODE/0.1 (herramienta web)")
        .send()
        .await
        .map_err(|e| format!("no se pudo descargar {url}: {e}"))?;
    if !r.status().is_success() {
        return Err(format!("{url} respondió HTTP {}", r.status()));
    }
    let cuerpo = r.text().await.map_err(|e| e.to_string())?;
    Ok(recortar(&limpiar_html(&cuerpo)))
}

async fn enviar(url: &str, cuerpo: &Value) -> Result<String, String> {
    let r = super::client(30)
        .post(url)
        .header("User-Agent", "KUKIA-CODE/0.1 (herramienta web)")
        .json(cuerpo)
        .send()
        .await
        .map_err(|e| format!("no se pudo enviar a {url}: {e}"))?;
    let estado = r.status();
    let texto = r.text().await.unwrap_or_default();
    if !estado.is_success() {
        return Err(format!("{url} respondió HTTP {estado}: {}", &texto.chars().take(300).collect::<String>()));
    }
    Ok(recortar(&texto))
}

fn limpiar_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut dentro = false;
    let mut saltos = 0;
    for c in s.chars() {
        match c {
            '<' => dentro = true,
            '>' => {
                dentro = false;
                if saltos < 1 {
                    out.push(' ');
                }
            }
            _ if dentro => {}
            c if c.is_whitespace() => {
                if saltos < 1 {
                    out.push(' ');
                    saltos += 1;
                }
            }
            c => {
                out.push(c);
                saltos = 0;
            }
        }
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p() -> AgentPolicy {
        AgentPolicy::default_for("prueba")
    }

    fn nombres(policy: &AgentPolicy, aprobado: bool) -> Vec<String> {
        esquemas(policy, aprobado)
            .iter()
            .filter_map(|d| d["function"]["name"].as_str().map(|s| s.to_string()))
            .collect()
    }

    fn tmp_dir(tag: &str) -> PathBuf {
        let p = tmp_dir_path(tag);
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn tmp_dir_path(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("kukia_tools_{tag}_{}", std::process::id()))
    }

    #[test]
    fn el_catalogo_de_lectura_es_amplio() {
        let n = nombres(&p(), false);
        for esperado in [
            "files_list",
            "files_read",
            "files_read_many",
            "files_find",
            "files_search",
            "files_info",
            "dir_tree",
            "memory_search",
            "logs_read",
            "gpu_stats",
            "sys_info",
            "disk_usage",
            "models_loaded",
            "vram_report",
            "time_now",
        ] {
            assert!(n.contains(&esperado.to_string()), "falta {esperado}: {n:?}");
        }
        assert!(n.len() >= 15, "el catálogo de lectura es corto: {n:?}");
    }

    #[test]
    fn sin_aprobacion_no_hay_ninguna_accion() {
        let mut po = p();
        po.allow_write = true;
        po.allow_shell = true;
        po.allow_network = true;
        po.allow_tools = true;
        let n = nombres(&po, false);
        for prohibido in [
            "files_write",
            "files_delete",
            "terminal_run",
            "git_status",
            "web_fetch",
            "mcp_call",
        ] {
            assert!(
                !n.contains(&prohibido.to_string()),
                "{prohibido} no debe ofrecerse sin aprobación"
            );
        }
    }

    #[test]
    fn con_permisos_y_aprobacion_se_ofrece_todo() {
        let mut po = p();
        po.allow_write = true;
        po.allow_shell = true;
        po.allow_network = true;
        po.allow_tools = true;
        let n = nombres(&po, true);
        for esperado in [
            "files_write",
            "files_append",
            "files_replace",
            "files_copy",
            "files_move",
            "files_delete",
            "mkdir",
            "memory_add",
            "terminal_run",
            "git_status",
            "git_diff",
            "git_log",
            "processes_list",
            "vram_unload",
            "web_fetch",
            "web_post",
            "mcp_call",
        ] {
            assert!(n.contains(&esperado.to_string()), "falta {esperado}: {n:?}");
        }
        assert!(n.len() >= 34, "el catálogo completo es corto: {n:?}");
    }

    #[test]
    fn sin_lectura_no_hay_herramientas_de_lectura() {
        let mut po = p();
        po.allow_read = false;
        let n = nombres(&po, true);
        assert!(!n.contains(&"files_read".to_string()));
        assert!(!n.contains(&"files_search".to_string()));
    }

    #[test]
    fn el_patron_admite_asteriscos_e_interrogaciones() {
        assert!(coincide("providers.rs", "*.rs"));
        assert!(coincide("providers.rs", "prov*"));
        assert!(coincide("providers", "provider?"));
        assert!(!coincide("providers.rs", "provider?"));
        assert!(!coincide("providers.rs", "*.ts"));
        assert!(coincide("Cargo.toml", "cargo*"));
    }

    #[tokio::test]
    async fn escribir_sin_aprobacion_se_rechaza() {
        let mut po = p();
        po.allow_write = true;
        let ruta = std::env::temp_dir().join("kukia_no_debe_escribirse.txt");
        let _ = std::fs::remove_file(&ruta);
        let r = ejecutar(
            "files_write",
            &json!({ "ruta": ruta.to_string_lossy(), "contenido": "x" }),
            &po,
            false,
            None,
        )
        .await;
        assert!(r.is_err(), "sin aprobación no se escribe");
        assert!(!ruta.exists(), "no debe existir el archivo");
    }

    #[tokio::test]
    async fn lectura_y_busquedas_funcionan() {
        let dir = tmp_dir("lecturas");
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("a.rs"), "fn uno() {}\nconst CLAVE_SECRETA: u32 = 4711;\n").unwrap();
        std::fs::write(dir.join("sub").join("objetivo.rs"), "x").unwrap();
        std::fs::write(dir.join("b.ts"), "const otra = 1;\n").unwrap();
        let po = p();
        let d = dir.to_string_lossy().to_string();

        let r = ejecutar(
            "files_search",
            &json!({ "texto": "clave_secreta", "carpeta": d, "extension": "rs" }),
            &po,
            false,
            None,
        )
        .await
        .expect("búsqueda dentro");
        assert!(r.contains("a.rs:2"), "debe indicar archivo y línea: {r}");
        assert!(!r.contains("b.ts"), "el filtro de extensión debe respetarse");

        let f = ejecutar(
            "files_find",
            &json!({ "patron": "objetivo*", "carpeta": d }),
            &po,
            false,
            None,
        )
        .await
        .expect("búsqueda por nombre");
        assert!(f.contains("objetivo.rs"));
        assert!(!f.contains("otro"));

        let info = ejecutar("files_info", &json!({ "ruta": dir.join("a.rs").to_string_lossy() }), &po, false, None)
            .await
            .expect("ficha");
        assert!(info.contains("archivo"));
        assert!(info.contains("línea"));

        let arbol_txt = ejecutar("dir_tree", &json!({ "carpeta": d, "niveles": 2 }), &po, false, None)
            .await
            .expect("árbol");
        assert!(arbol_txt.contains("a.rs"));
        assert!(arbol_txt.contains("sub/"));

        let varios = ejecutar(
            "files_read_many",
            &json!({ "rutas": [dir.join("a.rs").to_string_lossy(), dir.join("b.ts").to_string_lossy()] }),
            &po,
            false,
            None,
        )
        .await
        .expect("lectura múltiple");
        assert!(varios.contains("CLAVE_SECRETA") && varios.contains("otra"));

        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn escritura_completa_con_permiso_y_rutas() {
        let dir = tmp_dir("escritura");
        let mut po = p();
        po.allow_write = true;
        po.write_paths = dir.to_string_lossy().to_string();
        let d = dir.to_string_lossy().to_string();

        let f = dir.join("c.txt");
        ejecutar(
            "files_write",
            &json!({ "ruta": f.to_string_lossy(), "contenido": "uno uno dos" }),
            &po,
            true,
            None,
        )
        .await
        .expect("escribir");

        // Repetido: no se toca.
        let r = ejecutar(
            "files_replace",
            &json!({ "ruta": f.to_string_lossy(), "buscar": "uno", "reemplazar": "tres" }),
            &po,
            true,
            None,
        )
        .await;
        assert!(r.is_err() && r.unwrap_err().contains("2 veces"));

        ejecutar(
            "files_replace",
            &json!({ "ruta": f.to_string_lossy(), "buscar": "dos", "reemplazar": "cuatro" }),
            &po,
            true,
            None,
        )
        .await
        .expect("reemplazo único");
        assert!(std::fs::read_to_string(&f).unwrap().contains("cuatro"));

        ejecutar("files_append", &json!({ "ruta": f.to_string_lossy(), "contenido": "\ncinco" }), &po, true, None)
            .await
            .expect("añadir");
        assert!(std::fs::read_to_string(&f).unwrap().contains("cinco"));

        let copia = dir.join("sub").join("copia.txt");
        ejecutar(
            "files_copy",
            &json!({ "origen": f.to_string_lossy(), "destino": copia.to_string_lossy() }),
            &po,
            true,
            None,
        )
        .await
        .expect("copiar");
        assert!(copia.exists());

        let movido = dir.join("movido.txt");
        ejecutar(
            "files_move",
            &json!({ "origen": copia.to_string_lossy(), "destino": movido.to_string_lossy() }),
            &po,
            true,
            None,
        )
        .await
        .expect("mover");
        assert!(movido.exists() && !copia.exists());

        ejecutar("mkdir", &json!({ "ruta": dir.join("nueva").to_string_lossy() }), &po, true, None)
            .await
            .expect("carpeta");
        assert!(dir.join("nueva").is_dir());

        ejecutar(
            "files_delete",
            &json!({ "ruta": movido.to_string_lossy() }),
            &po,
            true,
            None,
        )
        .await
        .expect("borrar");
        assert!(!movido.exists());

        // Una carpeta con contenido exige «recursivo»: se deja algo dentro
        // (al mover el archivo de prueba, «sub» se había quedado vacía).
        std::fs::write(dir.join("sub").join("relleno.txt"), "x").unwrap();
        let r = ejecutar(
            "files_delete",
            &json!({ "ruta": dir.join("sub").to_string_lossy() }),
            &po,
            true,
            None,
        )
        .await;
        assert!(r.is_err());
        ejecutar(
            "files_delete",
            &json!({ "ruta": dir.join("sub").to_string_lossy(), "recursivo": true }),
            &po,
            true,
            None,
        )
        .await
        .expect("borrar carpeta recursiva");

        // Y no se borra una raíz.
        assert!(ejecutar("files_delete", &json!({ "ruta": "C:\\" }), &po, true, None)
            .await
            .is_err());

        let _ = d;
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn git_y_procesos_responden() {
        let mut po = p();
        po.allow_shell = true;
        let repo = PathBuf::from(r"H:\KUKIA_CODE");
        let st = ejecutar("git_status", &json!({ "carpeta": repo.to_string_lossy() }), &po, true, None)
            .await
            .expect("git status");
        assert!(st.contains("##") || st.contains("fatal"), "salida inesperada: {st}");

        let pr = ejecutar("processes_list", &json!({ "max": 5 }), &po, true, None)
            .await
            .expect("procesos");
        assert!(pr.lines().count() >= 1);
    }

    #[tokio::test]
    async fn la_red_y_mcp_exigen_aprobacion() {
        let mut po = p();
        po.allow_network = true;
        po.allow_tools = true;
        let r = ejecutar("web_fetch", &json!({ "url": "https://example.com" }), &po, false, None).await;
        assert!(r.is_err(), "sin aprobación no se sale a la red");
        let m = ejecutar(
            "mcp_call",
            &json!({ "servidor": "x", "herramienta": "y" }),
            &po,
            false,
            None,
        )
        .await;
        assert!(m.is_err(), "sin aprobación no se llama a MCP");
    }

    #[test]
    fn el_limpiador_de_html_quita_etiquetas() {
        let t = limpiar_html("<p>Hola <b>mundo</b></p>");
        assert!(t.contains("Hola") && t.contains("mundo") && !t.contains('<'));
    }
}
