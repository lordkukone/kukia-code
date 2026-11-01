//! Ejecución de comandos con aprobación, *timeout*, tope de salida y cancelación.
//!
//! Reglas de Fase 3.0:
//! - Sin aprobación explícita no se ejecuta nada.
//! - Todo comando tiene un límite de tiempo; al agotarse se mata el árbol de procesos.
//! - La salida se recorta a `MAX_OUTPUT_BYTES` (sin dejar de drenar la tubería, para que el
//!   proceso hijo nunca se quede bloqueado escribiendo).
//! - Se devuelve el PID para que la interfaz pueda mostrar qué se está ejecutando.

use serde::Serialize;
use std::path::Path;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::sync::oneshot;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

/// Tope de salida por flujo (stdout y stderr por separado).
pub const MAX_OUTPUT_BYTES: usize = 1_000_000;
/// Tiempo máximo por defecto antes de matar el comando.
pub const DEFAULT_TIMEOUT_SECS: u64 = 60;

#[derive(Debug, Clone, Serialize)]
pub struct RunResult {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
    /// Cierto si la salida se recortó por superar `MAX_OUTPUT_BYTES`.
    pub truncated: bool,
    pub duration_ms: u64,
    /// Cierto si se agotó el tiempo y hubo que matar el proceso.
    pub timed_out: bool,
    /// Cierto si el usuario canceló la ejecución.
    pub cancelled: bool,
    pub pid: u32,
}

/// Lee un flujo con tope, drenando el resto para no bloquear al hijo.
async fn read_capped<R>(mut flujo: R) -> (String, bool)
where
    R: AsyncRead + Unpin,
{
    let mut guardado: Vec<u8> = Vec::new();
    let mut trozo = [0u8; 8192];
    let mut recortado = false;
    loop {
        match flujo.read(&mut trozo).await {
            Ok(0) => break,
            Ok(n) => {
                if guardado.len() < MAX_OUTPUT_BYTES {
                    let espacio = MAX_OUTPUT_BYTES - guardado.len();
                    let tomar = espacio.min(n);
                    guardado.extend_from_slice(&trozo[..tomar]);
                    if tomar < n {
                        recortado = true;
                    }
                } else {
                    recortado = true;
                }
            }
            Err(_) => break,
        }
    }
    (String::from_utf8_lossy(&guardado).to_string(), recortado)
}

/// Mata el proceso **y sus hijos** (en Windows, `taskkill /T /F`).
async fn matar_arbol(pid: u32) {
    if pid == 0 {
        return;
    }
    let _ = tokio::task::spawn_blocking(move || {
        let mut cmd = std::process::Command::new("taskkill");
        cmd.args(["/PID", &pid.to_string(), "/T", "/F"]);
        #[cfg(windows)]
        {
            cmd.creation_flags(0x0800_0000);
        }
        let _ = cmd.output();
    })
    .await;
}

enum Fin {
    Terminado(std::io::Result<std::process::ExitStatus>),
    Tiempo,
    Cancelado,
}

/// Ejecuta PowerShell con control total.
///
/// `cancel` recibe la señal de cancelación desde la interfaz; `on_start` se llama con el PID
/// justo después de lanzar el proceso (para poder ofrecer «Cancelar»).
pub async fn run_controlled(
    command: &str,
    cwd: &str,
    approved: bool,
    timeout_secs: u64,
    cancel: Option<oneshot::Receiver<()>>,
    on_start: impl FnOnce(u32),
) -> Result<RunResult, String> {
    if !approved {
        return Err("Operación bloqueada: falta aprobación explícita".into());
    }
    let command = command.trim();
    if command.is_empty() {
        return Err("Comando vacío".into());
    }
    let cwd_path = Path::new(cwd);
    if !cwd_path.is_dir() {
        return Err(format!("Directorio inexistente: {cwd}"));
    }
    let limite = if timeout_secs == 0 {
        DEFAULT_TIMEOUT_SECS
    } else {
        timeout_secs
    };

    let mut cmd = tokio::process::Command::new("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", command])
        .current_dir(cwd_path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000);

    let inicio = Instant::now();
    let mut hijo = cmd
        .spawn()
        .map_err(|e| format!("No se pudo lanzar PowerShell: {e}"))?;
    let pid = hijo.id().unwrap_or(0);
    on_start(pid);

    let salida = hijo.stdout.take();
    let errores = hijo.stderr.take();
    let t_out = tokio::spawn(async move {
        match salida {
            Some(s) => read_capped(s).await,
            None => (String::new(), false),
        }
    });
    let t_err = tokio::spawn(async move {
        match errores {
            Some(s) => read_capped(s).await,
            None => (String::new(), false),
        }
    });

    let fin = match cancel {
        Some(rx) => {
            tokio::select! {
                s = hijo.wait() => Fin::Terminado(s),
                _ = rx => Fin::Cancelado,
                _ = tokio::time::sleep(Duration::from_secs(limite)) => Fin::Tiempo,
            }
        }
        None => {
            tokio::select! {
                s = hijo.wait() => Fin::Terminado(s),
                _ = tokio::time::sleep(Duration::from_secs(limite)) => Fin::Tiempo,
            }
        }
    };

    let mut timed_out = false;
    let mut cancelled = false;
    let mut code = -1i32;
    match fin {
        Fin::Terminado(Ok(estado)) => {
            code = estado.code().unwrap_or(-1);
        }
        Fin::Terminado(Err(e)) => {
            let _ = matar_arbol(pid);
            return Err(format!("Fallo esperando al proceso: {e}"));
        }
        Fin::Tiempo => {
            timed_out = true;
            let _ = hijo.start_kill();
            matar_arbol(pid).await;
        }
        Fin::Cancelado => {
            cancelled = true;
            let _ = hijo.start_kill();
            matar_arbol(pid).await;
        }
    }
    let duration_ms = inicio.elapsed().as_millis() as u64;

    let (stdout, recortado_out) = t_out.await.unwrap_or_default();
    let (stderr, recortado_err) = t_err.await.unwrap_or_default();

    Ok(RunResult {
        code,
        stdout,
        stderr,
        truncated: recortado_out || recortado_err,
        duration_ms,
        timed_out,
        cancelled,
        pid,
    })
}

/// Ejecución simple (sin cancelación) con el timeout por defecto.
pub async fn run_powershell(command: &str, cwd: &str, approved: bool) -> Result<RunResult, String> {
    run_controlled(command, cwd, approved, DEFAULT_TIMEOUT_SECS, None, |_| {}).await
}

/// Lectura de archivos con límite de tamaño para evitar cargar binarios enormes.
pub fn read_text_file(path: &str, max_bytes: u64) -> Result<String, String> {
    let p = Path::new(path);
    let meta = std::fs::metadata(p).map_err(|e| e.to_string())?;
    if !meta.is_file() {
        return Err("No es un archivo".into());
    }
    if meta.len() > max_bytes {
        return Err(format!(
            "Archivo demasiado grande ({} bytes, límite {})",
            meta.len(),
            max_bytes
        ));
    }
    std::fs::read_to_string(p).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn blocks_without_approval() {
        let r = run_powershell("Write-Output hola", "C:\\", false).await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn read_rejects_missing() {
        assert!(read_text_file("C:\\no-existe-kukia-xyz.txt", 1024).is_err());
    }

    #[tokio::test]
    async fn ejecuta_y_devuelve_codigo() {
        let pid_visto = std::sync::Arc::new(std::sync::Mutex::new(0u32));
        let pista = pid_visto.clone();
        let r = run_controlled(
            "Write-Output KUKIA_FASE30",
            "C:\\",
            true,
            DEFAULT_TIMEOUT_SECS,
            None,
            move |pid| *pista.lock().unwrap() = pid,
        )
        .await
        .expect("debe ejecutar");
        assert_eq!(r.code, 0);
        assert!(r.stdout.contains("KUKIA_FASE30"), "stdout={:?}", r.stdout);
        assert!(!r.timed_out && !r.cancelled && !r.truncated);
        assert!(*pid_visto.lock().unwrap() > 0, "debe informar del PID");
    }

    #[tokio::test]
    async fn el_timeout_mata_el_proceso() {
        let r = run_controlled(
            "Start-Sleep -Seconds 30; Write-Output 'no deberia llegar'",
            "C:\\",
            true,
            1,
            None,
            |_| {},
        )
        .await
        .expect("timeout no es error de seguridad");
        assert!(r.timed_out, "deberia marcar timed_out");
        assert!(!r.cancelled);
        assert!(r.duration_ms < 15_000, "debe cortar rapido: {} ms", r.duration_ms);
    }

    #[tokio::test]
    async fn la_cancelacion_corta_la_ejecucion() {
        let (tx, rx) = oneshot::channel::<()>();
        let hilo = tokio::spawn(async move {
            run_controlled(
                "Start-Sleep -Seconds 30; Write-Output 'no deberia llegar'",
                "C:\\",
                true,
                120,
                Some(rx),
                |_| {},
            )
            .await
        });
        tokio::time::sleep(Duration::from_millis(900)).await;
        let _ = tx.send(());
        let r = hilo.await.unwrap().expect("cancelar no es error de seguridad");
        assert!(r.cancelled, "deberia marcar cancelled");
        assert!(!r.timed_out);
    }

    #[tokio::test]
    async fn recorta_la_salida_enorme() {
        let r = run_controlled(
            "[Console]::Out.Write('x' * 2000000)",
            "C:\\",
            true,
            DEFAULT_TIMEOUT_SECS,
            None,
            |_| {},
        )
        .await
        .expect("debe ejecutar");
        assert!(r.truncated, "deberia marcar truncated");
        assert_eq!(r.stdout.len(), MAX_OUTPUT_BYTES);
    }
}
