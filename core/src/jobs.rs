//! Cola persistente de trabajos (SQ1 del Omega).
//!
//! Reglas de diseño, aprendidas de lo que ya falló en este proyecto:
//!
//! - **Persistente**: los trabajos viven en SQLite (`jobs`), así que cerrar la app
//!   no los pierde. Al arrancar, lo que quedó en `running` se devuelve a `queued`
//!   porque no hay ningún worker vivo que lo esté haciendo.
//! - **Reclamo atómico**: un solo `UPDATE ... RETURNING` elige y marca el trabajo,
//!   así dos workers no pueden coger el mismo.
//! - **Acotada**: como máximo `MAX_WORKERS`; la app decide cuántos están activos y
//!   la vista lo muestra.
//! - **Reintentos limitados**: un trabajo que falla vuelve a la cola hasta
//!   `max_attempts`; después queda `failed` con su error real.
//! - **Sin efectos ocultos**: los workers **no cargan modelos**. Solo miden, liberan
//!   VRAM que el usuario pidió liberar o escanean fuentes públicas.
//! - **Parada global**: la bandera de parada impide reclamar trabajos nuevos; el que
//!   esté en curso termina (no se mata a mitad de una generación).

use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::memory::Database;

/// Techo de trabajos simultáneos. Más de cuatro a la vez solo sirve para que se
/// peleen por la misma GPU.
pub const MAX_WORKERS: usize = 4;

/// Tipos de trabajo que la cola sabe ejecutar de verdad.
pub const TIPOS: [(&str, &str); 6] = [
    ("bench", "Medir un modelo (banco de pruebas)"),
    ("vram_release", "Liberar VRAM (motor, GPU o ambas)"),
    ("research_scan", "Escanear fuentes de descubrimiento"),
    ("research_auto", "Investigación autónoma supervisada"),
    ("model_pull", "Descargar modelo de Ollama"),
    ("noop", "Trabajo de prueba (espera N ms)"),
];

pub fn tipo_valido(kind: &str) -> bool {
    TIPOS.iter().any(|(k, _)| *k == kind)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: i64,
    pub kind: String,
    pub payload: serde_json::Value,
    /// `queued` | `running` | `done` | `failed` | `cancelled`
    pub state: String,
    pub priority: i64,
    pub attempts: i64,
    pub max_attempts: i64,
    pub worker: Option<String>,
    pub progress: Option<f64>,
    pub result: Option<serde_json::Value>,
    pub error: String,
    pub created_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct JobStats {
    pub queued: i64,
    pub running: i64,
    pub done: i64,
    pub failed: i64,
    pub cancelled: i64,
    /// Segundos que lleva esperando el trabajo más antiguo en cola.
    pub espera_max_s: i64,
    pub total: i64,
}

fn ahora() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn fila(r: &rusqlite::Row<'_>) -> rusqlite::Result<Job> {
    let payload: String = r.get("payload")?;
    let result: Option<String> = r.get("result")?;
    Ok(Job {
        id: r.get("id")?,
        kind: r.get("kind")?,
        payload: serde_json::from_str(&payload).unwrap_or(serde_json::Value::Null),
        state: r.get("state")?,
        priority: r.get("priority")?,
        attempts: r.get("attempts")?,
        max_attempts: r.get("max_attempts")?,
        worker: r.get("worker").ok(),
        progress: r.get("progress").ok(),
        result: result.and_then(|t| serde_json::from_str(&t).ok()),
        error: r.get("error")?,
        created_at: r.get("created_at")?,
        started_at: r.get("started_at").ok(),
        finished_at: r.get("finished_at").ok(),
    })
}

/// Añade un trabajo a la cola. Devuelve su id.
pub fn enqueue(
    db: &Database,
    kind: &str,
    payload: serde_json::Value,
    priority: i64,
) -> Result<i64, String> {
    if !tipo_valido(kind) {
        return Err(format!(
            "tipo de trabajo desconocido: {kind}. Válidos: {}",
            TIPOS.iter().map(|(k, _)| *k).collect::<Vec<_>>().join(", ")
        ));
    }
    db.conn()
        .execute(
            "INSERT INTO jobs (kind, payload, state, priority, created_at)
             VALUES (?1, ?2, 'queued', ?3, ?4)",
            params![kind, payload.to_string(), priority, ahora()],
        )
        .map_err(|e| e.to_string())?;
    Ok(db.conn().last_insert_rowid())
}

/// Reclama el siguiente trabajo para `worker`, de forma atómica.
///
/// `UPDATE ... RETURNING` en una sola sentencia: si dos workers lo intentan a la
/// vez, solo uno recibe la fila.
pub fn claim_next(db: &Database, worker: &str) -> Option<Job> {
    let id: Option<i64> = db
        .conn()
        .query_row(
            "UPDATE jobs
                SET state = 'running', worker = ?1, started_at = ?2, attempts = attempts + 1
              WHERE id = (
                    SELECT id FROM jobs
                     WHERE state = 'queued'
                     ORDER BY priority DESC, id ASC
                     LIMIT 1
              )
          RETURNING id",
            params![worker, ahora()],
            |r| r.get(0),
        )
        .ok();
    let id = id?;
    db.conn()
        .query_row("SELECT * FROM jobs WHERE id = ?1", params![id], |r| fila(r))
        .ok()
}

/// Marca un trabajo como terminado con su resultado.
pub fn finish(db: &Database, id: i64, result: serde_json::Value) -> Result<(), String> {
    db.conn()
        .execute(
            "UPDATE jobs SET state = 'done', progress = 1.0, result = ?2,
                    error = '', finished_at = ?3
              WHERE id = ?1",
            params![id, result.to_string(), ahora()],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Marca un fallo: reintenta si quedan intentos, si no lo deja `failed`.
///
/// Devuelve `true` si ha vuelto a la cola.
pub fn fail(db: &Database, id: i64, error: &str) -> Result<bool, String> {
    let (attempts, max): (i64, i64) = db
        .conn()
        .query_row(
            "SELECT attempts, max_attempts FROM jobs WHERE id = ?1",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|e| e.to_string())?;
    let recortado: String = error.chars().take(400).collect();
    if attempts < max {
        db.conn()
            .execute(
                "UPDATE jobs SET state = 'queued', worker = NULL, started_at = NULL,
                        error = ?2
                  WHERE id = ?1",
                params![id, recortado],
            )
            .map_err(|e| e.to_string())?;
        Ok(true)
    } else {
        db.conn()
            .execute(
                "UPDATE jobs SET state = 'failed', error = ?2, finished_at = ?3
                  WHERE id = ?1",
                params![id, recortado, ahora()],
            )
            .map_err(|e| e.to_string())?;
        Ok(false)
    }
}

/// Cancela un trabajo **en cola**. Uno en ejecución no se mata: se avisa.
pub fn cancel(db: &Database, id: i64) -> Result<bool, String> {
    let n = db
        .conn()
        .execute(
            "UPDATE jobs SET state = 'cancelled', finished_at = ?2
              WHERE id = ?1 AND state = 'queued'",
            params![id, ahora()],
        )
        .map_err(|e| e.to_string())?;
    if n > 0 {
        return Ok(true);
    }
    let estado: Option<String> = db
        .conn()
        .query_row("SELECT state FROM jobs WHERE id = ?1", params![id], |r| r.get(0))
        .ok();
    match estado.as_deref() {
        Some("running") => Err("el trabajo está en ejecución: no se interrumpe a mitad".into()),
        Some(_) => Ok(false),
        None => Err(format!("no existe el trabajo {id}")),
    }
}

/// Cancela todo lo que esté en cola. Devuelve cuántos.
pub fn cancel_all_queued(db: &Database) -> Result<usize, String> {
    db.conn()
        .execute(
            "UPDATE jobs SET state = 'cancelled', finished_at = ?1 WHERE state = 'queued'",
            params![ahora()],
        )
        .map_err(|e| e.to_string())
}

/// Al arrancar: lo que quedó `running` no lo está haciendo nadie. Vuelve a la cola.
pub fn requeue_stale(db: &Database) -> Result<usize, String> {
    let n = db
        .conn()
        .execute(
            "UPDATE jobs SET state = 'queued', worker = NULL, started_at = NULL,
                    error = 'interrumpido: la app se cerró mientras se ejecutaba'
              WHERE state = 'running'",
            [],
        )
        .map_err(|e| e.to_string())?;
    Ok(n)
}

pub fn list(db: &Database, limit: i64) -> Vec<Job> {
    let mut stmt = match db
        .conn()
        .prepare("SELECT * FROM jobs ORDER BY id DESC LIMIT ?1")
    {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let filas = stmt.query_map(params![limit.clamp(1, 500)], |r| fila(r));
    match filas {
        Ok(it) => it.filter_map(|x| x.ok()).collect(),
        Err(_) => Vec::new(),
    }
}

pub fn by_id(db: &Database, id: i64) -> Option<Job> {
    db.conn()
        .query_row("SELECT * FROM jobs WHERE id = ?1", params![id], |r| fila(r))
        .ok()
}

pub fn stats(db: &Database) -> JobStats {
    let mut s = JobStats::default();
    let contar = |estado: &str| -> i64 {
        db.conn()
            .query_row(
                "SELECT COUNT(*) FROM jobs WHERE state = ?1",
                params![estado],
                |r| r.get(0),
            )
            .unwrap_or(0)
    };
    s.queued = contar("queued");
    s.running = contar("running");
    s.done = contar("done");
    s.failed = contar("failed");
    s.cancelled = contar("cancelled");
    s.total = s.queued + s.running + s.done + s.failed + s.cancelled;

    s.espera_max_s = db
        .conn()
        .query_row(
            "SELECT CAST((julianday('now') - julianday(MIN(created_at))) * 86400 AS INTEGER)
               FROM jobs WHERE state = 'queued'",
            [],
            |r| r.get::<_, Option<i64>>(0),
        )
        .ok()
        .flatten()
        .unwrap_or(0)
        .max(0);
    s
}

/// Borra el historial de trabajos ya terminados (no toca los vivos).
pub fn clear_finished(db: &Database) -> Result<usize, String> {
    db.conn()
        .execute(
            "DELETE FROM jobs WHERE state IN ('done','failed','cancelled')",
            [],
        )
        .map_err(|e| e.to_string())
}

/// Actualiza el progreso (0..1) si el motor lo permite. Los trabajos que no
/// publican progreso lo dejan en `None` y la interfaz dice «no disponible».
pub fn set_progress(db: &Database, id: i64, progress: Option<f64>) -> Result<(), String> {
    db.conn()
        .execute(
            "UPDATE jobs SET progress = ?2 WHERE id = ?1",
            params![id, progress.map(|p| p.clamp(0.0, 1.0))],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn db_temporal(nombre: &str) -> (Database, PathBuf) {
        let ruta = std::env::temp_dir().join(format!("kukia_jobs_{nombre}_{}.db", std::process::id()));
        let _ = std::fs::remove_file(&ruta);
        let db = Database::open(&ruta).expect("se debe abrir la base de datos de prueba");
        (db, ruta)
    }

    #[test]
    fn la_cola_respeta_prioridad_y_orden() {
        let (db, ruta) = db_temporal("orden");
        enqueue(&db, "noop", serde_json::json!({"ms": 1}), 0).unwrap();
        enqueue(&db, "noop", serde_json::json!({"ms": 1}), 5).unwrap();
        enqueue(&db, "noop", serde_json::json!({"ms": 1}), 1).unwrap();

        let primero = claim_next(&db, "w1").expect("hay trabajo");
        assert_eq!(primero.priority, 5, "el de más prioridad sale primero");
        assert_eq!(primero.state, "running");
        assert_eq!(primero.attempts, 1);

        let segundo = claim_next(&db, "w2").unwrap();
        assert_eq!(segundo.priority, 1);
        let tercero = claim_next(&db, "w3").unwrap();
        assert_eq!(tercero.priority, 0);
        assert!(claim_next(&db, "w4").is_none(), "ya no queda nada");
        let _ = std::fs::remove_file(ruta);
    }

    #[test]
    fn dos_workers_no_cogen_el_mismo_trabajo() {
        let (db, ruta) = db_temporal("atomic");
        let id = enqueue(&db, "noop", serde_json::json!({}), 0).unwrap();
        let a = claim_next(&db, "w1");
        let b = claim_next(&db, "w2");
        assert!(a.is_some());
        assert!(b.is_none(), "el segundo worker no puede coger el mismo trabajo");
        assert_eq!(a.unwrap().id, id);
        let _ = std::fs::remove_file(ruta);
    }

    #[test]
    fn un_fallo_reintenta_y_luego_queda_failed() {
        let (db, ruta) = db_temporal("reintentos");
        let id = enqueue(&db, "noop", serde_json::json!({}), 0).unwrap();
        db.conn()
            .execute("UPDATE jobs SET max_attempts = 2 WHERE id = ?1", params![id])
            .unwrap();

        claim_next(&db, "w1").unwrap();
        assert!(fail(&db, id, "primera caída").unwrap(), "debe volver a la cola");
        assert_eq!(by_id(&db, id).unwrap().state, "queued");

        claim_next(&db, "w1").unwrap();
        assert!(!fail(&db, id, "segunda caída").unwrap(), "ya no quedan intentos");
        let j = by_id(&db, id).unwrap();
        assert_eq!(j.state, "failed");
        assert!(j.error.contains("segunda caída"));
        let _ = std::fs::remove_file(ruta);
    }

    #[test]
    fn terminar_guarda_resultado_y_limpia_error() {
        let (db, ruta) = db_temporal("fin");
        let id = enqueue(&db, "noop", serde_json::json!({"ms": 1}), 0).unwrap();
        claim_next(&db, "w1").unwrap();
        finish(&db, id, serde_json::json!({"ok": true, "tok_s": 64.5})).unwrap();
        let j = by_id(&db, id).unwrap();
        assert_eq!(j.state, "done");
        assert_eq!(j.progress, Some(1.0));
        assert_eq!(j.result.unwrap()["tok_s"], 64.5);
        assert!(j.finished_at.is_some());
        let _ = std::fs::remove_file(ruta);
    }

    #[test]
    fn no_se_cancela_lo_que_esta_corriendo() {
        let (db, ruta) = db_temporal("cancel");
        let id = enqueue(&db, "noop", serde_json::json!({}), 0).unwrap();
        claim_next(&db, "w1").unwrap();
        let r = cancel(&db, id);
        assert!(r.is_err(), "no se debe fingir que se ha cancelado algo en curso");
        assert!(r.unwrap_err().contains("en ejecución"));

        let otro = enqueue(&db, "noop", serde_json::json!({}), 0).unwrap();
        assert!(cancel(&db, otro).unwrap());
        assert_eq!(by_id(&db, otro).unwrap().state, "cancelled");
        let _ = std::fs::remove_file(ruta);
    }

    #[test]
    fn al_arrancar_se_recupera_lo_que_quedo_a_medias() {
        let (db, ruta) = db_temporal("stale");
        let id = enqueue(&db, "noop", serde_json::json!({}), 0).unwrap();
        claim_next(&db, "w1").unwrap();
        assert_eq!(by_id(&db, id).unwrap().state, "running");

        let recuperados = requeue_stale(&db).unwrap();
        assert_eq!(recuperados, 1);
        let j = by_id(&db, id).unwrap();
        assert_eq!(j.state, "queued");
        assert!(j.worker.is_none());
        assert!(j.error.contains("se cerró"));
        let _ = std::fs::remove_file(ruta);
    }

    #[test]
    fn las_estadisticas_cuentan_cada_estado() {
        let (db, ruta) = db_temporal("stats");
        enqueue(&db, "noop", serde_json::json!({}), 0).unwrap();
        enqueue(&db, "noop", serde_json::json!({}), 0).unwrap();
        let reclamado = claim_next(&db, "w1").expect("hay trabajo");
        finish(&db, reclamado.id, serde_json::json!({})).unwrap();
        let s = stats(&db);
        assert_eq!(s.queued, 1, "queda el segundo en cola");
        assert_eq!(s.running, 0, "el reclamado ya terminó");
        assert_eq!(s.done, 1);
        assert_eq!(s.total, 2);
        assert!(s.espera_max_s >= 0);
        let _ = std::fs::remove_file(ruta);
    }

    #[test]
    fn un_tipo_inventado_se_rechaza() {
        let (db, ruta) = db_temporal("tipo");
        let r = enqueue(&db, "formatear_disco", serde_json::json!({}), 0);
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("desconocido"));
        let _ = std::fs::remove_file(ruta);
    }

    #[test]
    fn limpiar_solo_borra_lo_terminado() {
        let (db, ruta) = db_temporal("limpiar");
        let hecho = enqueue(&db, "noop", serde_json::json!({}), 0).unwrap();
        claim_next(&db, "w1").unwrap();
        finish(&db, hecho, serde_json::json!({})).unwrap();
        enqueue(&db, "noop", serde_json::json!({}), 0).unwrap();
        let borrados = clear_finished(&db).unwrap();
        assert_eq!(borrados, 1);
        assert_eq!(stats(&db).queued, 1);
        let _ = std::fs::remove_file(ruta);
    }
}
