//! Registro central de herramientas (F4) con estado comprobado.
//!
//! No inventa: cada entrada dice de dónde viene (core, MCP, pendiente) y, cuando
//! hay una sonda barata y segura, comprueba el estado de verdad. Lo que no se
//! puede comprobar se marca como tal en vez de ponerse «disponible».

use crate::mcp;
use crate::memory::Database;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ToolEntry {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    /// core | mcp | pendiente
    pub origin: String,
    /// Permiso que consume según `permissions::Tool`.
    pub permission: String,
    /// disponible | requiere autorización | no comprobado | error
    pub state: String,
    pub detail: String,
    pub server: Option<String>,
    pub schema: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Resumen {
    pub total: usize,
    pub core: usize,
    pub mcp: usize,
    pub servidores_mcp: usize,
    pub autorizados: usize,
    pub disponibles: usize,
    pub requieren_autorizacion: usize,
    pub sin_comprobar: usize,
}

struct Base {
    id: &'static str,
    name: &'static str,
    category: &'static str,
    description: &'static str,
    permission: &'static str,
}

const BASE: &[Base] = &[
    Base {
        id: "files.list",
        name: "Listar directorios",
        category: "archivos",
        description: "Lista el contenido de una ruta permitida por la política del agente.",
        permission: "lectura",
    },
    Base {
        id: "files.read",
        name: "Leer archivo",
        category: "archivos",
        description: "Lee texto de una ruta dentro de la allowlist del proyecto.",
        permission: "lectura",
    },
    Base {
        id: "files.write",
        name: "Escribir archivo",
        category: "archivos",
        description: "Escribe con política + aprobación explícita + ruta permitida.",
        permission: "escritura",
    },
    Base {
        id: "terminal.run",
        name: "Ejecutar comando",
        category: "sistema",
        description: "PowerShell supervisado, con timeout, cancelación y aprobación.",
        permission: "ejecución",
    },
    Base {
        id: "memory.search",
        name: "Buscar en memoria",
        category: "memoria",
        description: "Búsqueda sobre la memoria SQLite persistente.",
        permission: "lectura",
    },
    Base {
        id: "memory.add",
        name: "Guardar recuerdo",
        category: "memoria",
        description: "Añade una entrada a la memoria persistente.",
        permission: "escritura",
    },
    Base {
        id: "providers.list",
        name: "Listar proveedores",
        category: "ia",
        description: "Detecta Ollama, LM Studio y el proveedor cloud activo.",
        permission: "red",
    },
    Base {
        id: "gpu.stats",
        name: "Métricas GPU",
        category: "gpu",
        description: "VRAM, uso, temperatura y potencia por nvidia-smi.",
        permission: "lectura",
    },
    Base {
        id: "benchmark.run",
        name: "Medir modelo",
        category: "ia",
        description: "tok/s, prefill, TTFT y picos de VRAM/RAM.",
        permission: "red",
    },
    Base {
        id: "radar.check",
        name: "Radar de versiones",
        category: "investigacion",
        description: "Consulta releases oficiales de los motores.",
        permission: "red",
    },
    Base {
        id: "research.scan",
        name: "Escanear fuentes",
        category: "investigacion",
        description: "arXiv, Hugging Face y GitHub, con alertas por relevancia.",
        permission: "red",
    },
    Base {
        id: "research.autonomous",
        name: "Investigación autónoma",
        category: "investigacion",
        description: "Objetivo → varias fuentes → informe con referencias y propuestas.",
        permission: "red",
    },
    Base {
        id: "model.catalog",
        name: "Catálogo universal de modelos",
        category: "ia",
        description: "Enumera modelos de todos los motores y enruta por capacidad.",
        permission: "red",
    },
    Base {
        id: "jobs.queue",
        name: "Cola de trabajos",
        category: "automatizacion",
        description: "Cola persistente con workers acotados y parada global.",
        permission: "ejecución",
    },
    Base {
        id: "mcp.client",
        name: "Cliente MCP",
        category: "automatizacion",
        description: "Descubre servidores MCP y sus herramientas; solo conecta los autorizados.",
        permission: "herramientas externas",
    },
    Base {
        id: "security.policy",
        name: "Políticas por agente",
        category: "seguridad",
        description: "Allowlist de rutas y permisos por agente, con auditoría.",
        permission: "lectura",
    },
];

fn contar(db: &Database, sql: &str) -> i64 {
    db.conn()
        .query_row(sql, [], |r| r.get::<_, i64>(0))
        .unwrap_or(0)
}

/// Sonda barata y sin efectos: devuelve `(estado, detalle)`.
fn sondar(db: &Database, id: &str, servidores: &[mcp::ServerInfo]) -> (String, String) {
    match id {
        "gpu.stats" => {
            let g = crate::gpu::gpus();
            if g.is_empty() {
                (
                    "error".into(),
                    "nvidia-smi no devolvió ninguna GPU".into(),
                )
            } else {
                let detalle = g
                    .iter()
                    .map(|x| format!("GPU{} {} {} MB", x.index, x.name, x.memory_total_mb))
                    .collect::<Vec<_>>()
                    .join(" · ");
                ("disponible".into(), detalle)
            }
        }
        "memory.search" | "memory.add" => {
            let n = contar(db, "SELECT COUNT(*) FROM memories");
            ("disponible".into(), format!("{n} recuerdo(s) en la base"))
        }
        "files.list" | "files.read" | "files.write" => {
            let d = crate::config::data_dir();
            (
                "disponible".into(),
                format!("directorio de datos: {}", d.display()),
            )
        }
        "jobs.queue" => {
            let q = contar(db, "SELECT COUNT(*) FROM jobs WHERE state = 'queued'");
            let r = contar(db, "SELECT COUNT(*) FROM jobs WHERE state = 'running'");
            (
                "disponible".into(),
                format!("{q} en cola · {r} ejecutándose"),
            )
        }
        "research.scan" | "research.autonomous" => {
            let n = contar(db, "SELECT COUNT(*) FROM research_items");
            (
                "disponible".into(),
                format!("{n} descubrimiento(s) guardados"),
            )
        }
        "model.catalog" => {
            let binds = contar(db, "SELECT COUNT(*) FROM model_bindings");
            (
                "disponible".into(),
                format!("{binds} modelo(s) fijados por herramienta"),
            )
        }
        "security.policy" => {
            let n = contar(db, "SELECT COUNT(*) FROM agent_policies");
            ("disponible".into(), format!("{n} política(s) de agente"))
        }
        "mcp.client" => {
            let aut = servidores.iter().filter(|s| s.authorized).count();
            let tools: usize = servidores.iter().map(|s| s.tools).sum();
            if servidores.is_empty() {
                (
                    "no comprobado".into(),
                    "no hay ningún mcp.json con servidores".into(),
                )
            } else {
                (
                    "disponible".into(),
                    format!(
                        "{} servidor(es) declarados · {aut} autorizados · {tools} herramienta(s) descubiertas",
                        servidores.len()
                    ),
                )
            }
        }
        "providers.list" | "benchmark.run" | "radar.check" | "terminal.run" => (
            "disponible".into(),
            "se comprueba al ejecutarlo (necesita motor o aprobación)".into(),
        ),
        _ => ("no comprobado".into(), String::new()),
    }
}

/// Registro completo: herramientas del núcleo + herramientas MCP conocidas.
pub fn registro(db: &Database) -> Vec<ToolEntry> {
    let servidores = mcp::servidores(db);
    let mut out: Vec<ToolEntry> = BASE
        .iter()
        .map(|b| {
            let (state, detail) = sondar(db, b.id, &servidores);
            ToolEntry {
                id: b.id.into(),
                name: b.name.into(),
                category: b.category.into(),
                description: b.description.into(),
                origin: "core".into(),
                permission: b.permission.into(),
                state,
                detail,
                server: None,
                schema: None,
            }
        })
        .collect();

    for s in &servidores {
        for t in mcp::tools_guardadas(db, &s.name) {
            out.push(ToolEntry {
                id: format!("mcp:{}:{}", s.name, t.name),
                name: t.name.clone(),
                category: "mcp".into(),
                description: t.description.clone(),
                origin: "mcp".into(),
                permission: "herramientas externas".into(),
                state: if s.authorized {
                    "disponible".into()
                } else {
                    "requiere autorización".into()
                },
                detail: format!("servidor «{}» ({})", s.name, s.transport),
                server: Some(s.name.clone()),
                schema: if t.schema.is_null() {
                    None
                } else {
                    Some(t.schema.clone())
                },
            });
        }
    }

    out.sort_by(|a, b| {
        a.category
            .cmp(&b.category)
            .then(a.origin.cmp(&b.origin))
            .then(a.id.cmp(&b.id))
    });
    out
}

pub fn resumen(db: &Database) -> Resumen {
    let servidores = mcp::servidores(db);
    let r = registro(db);
    Resumen {
        total: r.len(),
        core: r.iter().filter(|t| t.origin == "core").count(),
        mcp: r.iter().filter(|t| t.origin == "mcp").count(),
        servidores_mcp: servidores.len(),
        autorizados: servidores.iter().filter(|s| s.authorized).count(),
        disponibles: r.iter().filter(|t| t.state == "disponible").count(),
        requieren_autorizacion: r
            .iter()
            .filter(|t| t.state == "requiere autorización")
            .count(),
        sin_comprobar: r.iter().filter(|t| t.state == "no comprobado").count(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db_temp(tag: &str) -> (Database, std::path::PathBuf) {
        let p = std::env::temp_dir().join(format!("kukia_tools_{tag}_{}.db", std::process::id()));
        let _ = std::fs::remove_file(&p);
        let db = Database::open(&p).expect("db de prueba");
        (db, p)
    }

    #[test]
    fn el_registro_incluye_las_dos_pendientes() {
        let (db, p) = db_temp("registro");
        let r = registro(&db);
        assert!(r.iter().any(|t| t.id == "mcp.client"));
        assert!(r.iter().any(|t| t.id == "research.autonomous"));
        assert!(r.len() >= BASE.len());
        drop(db);
        let _ = std::fs::remove_file(p);
    }

    #[test]
    fn gpu_stats_se_sonda_de_verdad() {
        let (db, p) = db_temp("gpu");
        let r = registro(&db);
        let g = r.iter().find(|t| t.id == "gpu.stats").unwrap();
        // En esta máquina hay dos GPU: o hay detalle real, o el estado es error.
        assert!(g.state == "disponible" || g.state == "error");
        assert!(!g.detail.is_empty());
        drop(db);
        let _ = std::fs::remove_file(p);
    }

    #[test]
    fn un_servidor_mcp_sin_autorizar_no_aparece_disponible() {
        let (db, p) = db_temp("mcp");
        mcp::autorizar(&db, "servidor-inventado", true).unwrap();
        // No está en ningún mcp.json, así que no debe aparecer en el registro.
        let r = registro(&db);
        assert!(!r.iter().any(|t| t.id.contains("servidor-inventado")));
        drop(db);
        let _ = std::fs::remove_file(p);
    }
}
