//! Dueño del proceso `llama-server`: arranque, lectura de salida y parada limpia.

use crate::localgpu::flags;
use crate::localgpu::{LocalGpuCfg, Perfil};
use serde::Serialize;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

/// Cuántas líneas de salida se guardan para enseñar en la interfaz.
pub const MAX_LINEAS: usize = 200;

#[derive(Debug, Clone, Serialize, Default)]
pub struct EstadoProceso {
    pub corriendo: bool,
    pub pid: Option<u32>,
    pub perfil: String,
    pub modelo: String,
    pub puerto: u16,
    pub iniciado_epoch_s: Option<u64>,
    pub lineas: Vec<String>,
    pub error: Option<String>,
    /// Nota de la última decisión (por ejemplo, por qué se bajó a 2 GPUs).
    pub aviso: Option<String>,
}

fn estado() -> &'static Mutex<EstadoProceso> {
    static E: OnceLock<Mutex<EstadoProceso>> = OnceLock::new();
    E.get_or_init(|| Mutex::new(EstadoProceso::default()))
}

fn hijo() -> &'static Mutex<Option<Child>> {
    static H: OnceLock<Mutex<Option<Child>>> = OnceLock::new();
    H.get_or_init(|| Mutex::new(None))
}

fn parada_pedida() -> &'static AtomicBool {
    static P: AtomicBool = AtomicBool::new(false);
    &P
}

fn con_estado<T>(f: impl FnOnce(&mut EstadoProceso) -> T) -> T {
    let mut g = estado().lock().unwrap_or_else(|e| e.into_inner());
    f(&mut g)
}

/// Guarda una línea de salida, recortando por arriba para no crecer sin fin.
fn apunta(linea: impl Into<String>) {
    let l = linea.into();
    con_estado(|st| {
        st.lineas.push(l);
        while st.lineas.len() > MAX_LINEAS {
            st.lineas.remove(0);
        }
    });
}

/// Estado actual. Si el proceso murió por su cuenta, se refleja aquí.
pub fn status() -> EstadoProceso {
    let muerto: Option<Option<i32>> = {
        let mut g = hijo().lock().unwrap_or_else(|e| e.into_inner());
        match g.as_mut() {
            Some(ch) => match ch.try_wait() {
                Ok(Some(salida)) => {
                    let codigo = salida.code();
                    *g = None;
                    Some(codigo)
                }
                Ok(None) => None,
                Err(_) => None,
            },
            None => None,
        }
    };
    con_estado(|st| {
        if let Some(codigo) = muerto {
            let texto = format!(
                "llama-server terminó por su cuenta (código {})",
                codigo.map(|c| c.to_string()).unwrap_or_else(|| "?".into())
            );
            st.corriendo = false;
            st.pid = None;
            st.error = Some(texto.clone());
            st.lineas.push(texto);
            if st.lineas.len() > MAX_LINEAS {
                st.lineas.remove(0);
            }
        }
        st.clone()
    })
}

/// Lanza `llama-server` con el perfil indicado. No espera a que cargue el modelo:
/// el progreso llega por las líneas de salida.
pub fn arrancar(cfg: &LocalGpuCfg, perfil: Perfil) -> Result<EstadoProceso, String> {
    if con_estado(|st| st.corriendo) {
        return Err("ya hay un llama-server gestionado por KUKIA en marcha: páralo antes".into());
    }
    // Se miden las GPUs de verdad y se sondean los equipos remotos: entra en el
    // reparto todo lo que haya, sean dos, tres o mas.
    let locales = crate::localgpu::gpus::detectar().unwrap_or_default();
    let mut remotos: Vec<(String, u64)> = Vec::new();
    if perfil == Perfil::TresGpus {
        for (h, mb) in cfg.remotos() {
            let s = crate::localgpu::rpc::sondar(&h, cfg.timeout_rpc_s);
            if s.ok {
                remotos.push((s.destino.clone(), mb));
            }
        }
    }
    let mut disp: Vec<(String, u64)> = locales
        .iter()
        .map(|g| {
            let vram = if g.libre_mb > 0 { g.libre_mb } else { g.total_mb };
            (g.nombre.clone(), vram)
        })
        .collect();
    for (destino, mb) in &remotos {
        disp.push((destino.clone(), *mb));
    }
    let split: Vec<u32> = if disp.len() <= 1 {
        Vec::new()
    } else if cfg.forzar_split
        && cfg.tensor_split.len() == disp.len()
        && cfg.tensor_split.iter().any(|x| *x > 0)
    {
        cfg.tensor_split.clone()
    } else {
        crate::localgpu::gpus::reparto(&disp, cfg.reserva_vram_mb)
    };
    let exe = cfg.llama_server.trim().to_string();
    if exe.is_empty() {
        return Err("falta la ruta de llama-server.exe (local_gpu.llama_server)".into());
    }
    let destinos: Vec<String> = remotos.iter().map(|(d, _)| d.clone()).collect();
    let args = flags::argumentos_con(cfg, perfil, &split, &destinos)?;
    if !std::path::Path::new(&exe).exists() {
        return Err(format!(
            "no encuentro llama-server en «{exe}»: pon la ruta real en local_gpu.llama_server"
        ));
    }
    if !std::path::Path::new(cfg.modelo.trim()).exists() {
        return Err(format!("no encuentro el modelo «{}»", cfg.modelo.trim()));
    }

    parada_pedida().store(false, Ordering::SeqCst);
    let mut cmd = Command::new(&exe);
    cmd.args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("no se pudo lanzar «{exe}»: {e}"))?;
    let pid = child.id();

    if let Some(err) = child.stderr.take() {
        std::thread::spawn(move || {
            for linea in BufReader::new(err).lines().map_while(Result::ok) {
                let l = linea.trim_end().to_string();
                if l.is_empty() {
                    continue;
                }
                if l.to_lowercase().contains("error") {
                    apunta(format!("[stderr] {l}"));
                } else {
                    apunta(l);
                }
            }
        });
    }
    if let Some(out) = child.stdout.take() {
        std::thread::spawn(move || {
            for linea in BufReader::new(out).lines().map_while(Result::ok) {
                let l = linea.trim_end().to_string();
                if !l.is_empty() {
                    apunta(l);
                }
            }
        });
    }

    *hijo().lock().unwrap_or_else(|e| e.into_inner()) = Some(child);
    let iniciado = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .ok();
    Ok(con_estado(|st| {
        st.corriendo = true;
        st.pid = Some(pid);
        st.perfil = perfil.id().to_string();
        st.modelo = cfg.modelo.trim().to_string();
        st.puerto = cfg.puerto;
        st.iniciado_epoch_s = iniciado;
        st.error = None;
        st.aviso = None;
        st.lineas.clear();
        st.lineas.push(format!("lanzado: {exe} {}", args.join(" ")));
        st.lineas.push(format!(
            "dispositivos: {} locales + {} por RPC = {} (reparto {})",
            locales.len(),
            remotos.len(),
            disp.len(),
            if split.is_empty() { "-".to_string() } else { flags::split_texto(&split) }
        ));
        st.clone()
    }))
}

/// Para el proceso y su árbol. Seguro si no hay nada en marcha.
pub fn parar() -> Result<EstadoProceso, String> {
    parada_pedida().store(true, Ordering::SeqCst);
    let mut child = {
        let mut g = hijo().lock().unwrap_or_else(|e| e.into_inner());
        g.take()
    };
    let Some(mut ch) = child.take() else {
        return Ok(con_estado(|st| {
            st.corriendo = false;
            st.pid = None;
            st.clone()
        }));
    };
    let pid = ch.id();
    #[cfg(windows)]
    {
        // /T mata también a los hijos; /F evita que se quede pidiendo permiso.
        let args = vec![
            "/PID".to_string(),
            pid.to_string(),
            "/T".to_string(),
            "/F".to_string(),
        ];
        let _ = Command::new("taskkill")
            .args(&args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = ch.kill();
    let _ = ch.wait();
    apunta(format!("parado (pid {pid})"));
    Ok(con_estado(|st| {
        st.corriendo = false;
        st.pid = None;
        st.error = None;
        st.clone()
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parar_sin_nada_en_marcha_no_falla() {
        let st = parar().expect("parar sin proceso no debe fallar");
        assert!(!st.corriendo);
        assert!(st.pid.is_none());
    }

    #[test]
    fn arrancar_sin_ruta_de_llama_server_explica_que_falta() {
        let c = LocalGpuCfg::default();
        let e = arrancar(&c, Perfil::SoloI9).unwrap_err();
        assert!(e.contains("llama-server"), "mensaje poco claro: {e}");
    }

    #[test]
    fn arrancar_con_ruta_inexistente_no_lanza_nada() {
        let mut c = LocalGpuCfg::default();
        c.llama_server = "C:/no/existe/llama-server.exe".into();
        c.modelo = "C:/no/existe/modelo.gguf".into();
        let e = arrancar(&c, Perfil::SoloI9).unwrap_err();
        assert!(e.contains("no encuentro llama-server"), "mensaje: {e}");
        assert!(!status().corriendo);
    }

    #[test]
    fn el_estado_no_entra_en_panico_sin_proceso() {
        let st = status();
        assert!(!st.corriendo);
    }

    #[test]
    fn las_lineas_no_crecen_sin_limite() {
        for i in 0..(MAX_LINEAS + 40) {
            apunta(format!("linea {i}"));
        }
        let st = status();
        assert!(st.lineas.len() <= MAX_LINEAS, "se guardaron {}", st.lineas.len());
    }
}
