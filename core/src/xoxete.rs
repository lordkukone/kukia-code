//! **Xoxete** dentro de KUKIA CODE.
//!
//! No se reconstruye a Xoxete: se lee y se usa la instalación real de
//! `H:\Xoxete` (configurable con `KUKIA_XOXETE_ROOT`) y su motor de memoria.
//!
//! | Pieza real | Ruta | Uso desde aquí |
//! |---|---|---|
//! | Config de OpenCode | `xdg-config\opencode\opencode.json` | leer agente por defecto, modelo e instrucciones |
//! | Instrucciones | `AGENTS.md`, `AGENTS_AMPLIACION.md` | se muestran y se comprueban |
//! | Motor de memoria | `learning\xoxete_aprendiz.py` | `search`, `add`, `stats` por su CLI oficial |
//! | Base de memoria | `learning\memoria_aprendizaje.db` | la misma que usa el plugin; no se copia ni se duplica |
//! | Plugins | `xdg-config\opencode\plugin\` | se informa de cada uno y de si está presente |
//! | Binarios | `opencode-original`, `evil`, app 2.2.5 | versión real de cada uno |
//!
//! Reglas:
//! - Solo se ejecutan los subcomandos del motor de memoria y `--version`.
//! - Toda llamada al motor lleva tope de tiempo; si se pasa, se mata y se
//!   devuelve el error (nunca se queda la interfaz esperando).
//! - Leer es libre; escribir (`guardar`) es una acción explícita del usuario
//!   desde la interfaz y escribe en la misma base original, sin duplicarla.
//! - Si algo no está, se dice: `estado()` devuelve notas con lo que falta.

use serde::Serialize;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const TOPE_MOTOR: Duration = Duration::from_secs(10);

/* ============================ Tipos ============================ */

#[derive(Debug, Clone, Serialize, Default)]
pub struct Binario {
    pub nombre: String,
    pub ruta: String,
    pub existe: bool,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Plugin {
    pub nombre: String,
    pub presente: bool,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Cuenta {
    pub nombre: String,
    pub cantidad: usize,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Motor {
    pub disponible: bool,
    pub ruta: String,
    pub base: String,
    pub bytes: u64,
    pub total: usize,
    pub por_tipo: Vec<Cuenta>,
    pub por_estado: Vec<Cuenta>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct XoxeteEstado {
    pub raiz: String,
    pub config: String,
    pub config_ok: bool,
    pub agente: String,
    pub modelo: String,
    pub modelo_pequeno: String,
    pub instrucciones: Vec<String>,
    pub instrucciones_ok: usize,
    pub motor: Motor,
    pub plugins: Vec<Plugin>,
    pub binarios: Vec<Binario>,
    /// Lo que falta o está roto, en claro.
    pub notas: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Memoria {
    pub id: i64,
    pub tipo: String,
    pub estado: String,
    pub contexto: String,
    pub accion: String,
    pub resultado: String,
    pub confianza: i64,
}

/* ============================ Rutas ============================ */

pub fn raiz() -> PathBuf {
    if let Ok(p) = std::env::var("KUKIA_XOXETE_ROOT") {
        if !p.trim().is_empty() {
            return PathBuf::from(p);
        }
    }
    PathBuf::from(r"H:\Xoxete")
}

fn cfg_json() -> PathBuf {
    raiz().join("xdg-config").join("opencode").join("opencode.json")
}

fn dir_plugins() -> PathBuf {
    raiz().join("xdg-config").join("opencode").join("plugin")
}

fn motor_py() -> PathBuf {
    raiz().join("learning").join("xoxete_aprendiz.py")
}

fn base_memoria() -> PathBuf {
    raiz().join("learning").join("memoria_aprendizaje.db")
}

fn python() -> String {
    std::env::var("KUKIA_XOXETE_PYTHON")
        .ok()
        .filter(|p| !p.trim().is_empty())
        .unwrap_or_else(|| "python".to_string())
}

/* ============================ Procesos ============================ */

#[cfg(windows)]
fn oculto(cmd: &mut Command) -> &mut Command {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(0x0800_0000) // CREATE_NO_WINDOW
}

#[cfg(not(windows))]
fn oculto(cmd: &mut Command) -> &mut Command {
    cmd
}

/// Ejecuta y **no espera más de `TOPE_MOTOR`**: si el proceso se pasa, se mata
/// y se informa. Evita que un motor atascado congele la interfaz.
fn ejecutar(mut cmd: Command) -> Result<String, String> {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut hijo = oculto(&mut cmd).spawn().map_err(|e| format!("no se pudo lanzar: {e}"))?;
    let inicio = Instant::now();
    loop {
        match hijo.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if inicio.elapsed() > TOPE_MOTOR {
                    let _ = hijo.kill();
                    let _ = hijo.wait();
                    return Err(format!(
                        "el proceso no terminó en {} s y se detuvo",
                        TOPE_MOTOR.as_secs()
                    ));
                }
                std::thread::sleep(Duration::from_millis(60));
            }
            Err(e) => return Err(format!("error esperando al proceso: {e}")),
        }
    }
    let salida = hijo
        .wait_with_output()
        .map_err(|e| format!("no se pudo leer la salida: {e}"))?;
    let texto = String::from_utf8_lossy(&salida.stdout).to_string();
    if texto.trim().is_empty() {
        let err = String::from_utf8_lossy(&salida.stderr).trim().to_string();
        return Err(if err.is_empty() {
            "sin salida".to_string()
        } else {
            err
        });
    }
    Ok(texto)
}

/// Última línea que sea JSON (el motor imprime logs y luego el objeto).
fn ultimo_json(texto: &str) -> Result<serde_json::Value, String> {
    for linea in texto.lines().rev() {
        let l = linea.trim();
        if l.starts_with('{') {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(l) {
                return Ok(v);
            }
        }
    }
    Err("respuesta no reconocida del motor de memoria".to_string())
}

/// Llama al motor de Xoxete con sus subcomandos oficiales.
fn motor(args: &[&str]) -> Result<serde_json::Value, String> {
    let script = motor_py();
    if !script.exists() {
        return Err(format!("motor no encontrado en {}", script.display()));
    }
    let mut cmd = Command::new(python());
    cmd.arg(&script).args(args).current_dir(raiz());
    let texto = ejecutar(cmd)?;
    let valor = ultimo_json(&texto)?;
    if valor.get("ok").and_then(|v| v.as_bool()) == Some(false) {
        return Err(valor
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("el motor respondió ok=false")
            .to_string());
    }
    Ok(valor)
}

fn cuenta_de(valor: Option<&serde_json::Value>) -> Vec<Cuenta> {
    let mut out: Vec<Cuenta> = valor
        .and_then(|v| v.as_object())
        .map(|m| {
            m.iter()
                .map(|(k, v)| Cuenta {
                    nombre: k.clone(),
                    cantidad: v.as_u64().unwrap_or(0) as usize,
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort_by(|a, b| b.cantidad.cmp(&a.cantidad).then(a.nombre.cmp(&b.nombre)));
    out
}

/* ============================ Estado ============================ */

fn version_de(ruta: &PathBuf) -> String {
    if !ruta.exists() {
        return String::new();
    }
    let mut cmd = Command::new(ruta);
    cmd.arg("--version");
    match ejecutar(cmd) {
        Ok(t) => t
            .lines()
            .map(|l| l.trim())
            .find(|l| !l.is_empty())
            .unwrap_or("")
            .to_string(),
        Err(_) => String::new(),
    }
}

fn plugins_presentes() -> Vec<Plugin> {
    let dir = dir_plugins();
    let mut out = Vec::new();
    if !dir.is_dir() {
        return out;
    }
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            let p = e.path();
            let nombre = e.file_name().to_string_lossy().to_string();
            if nombre == "__pycache__" {
                continue;
            }
            let bytes = e.metadata().map(|m| m.len()).unwrap_or(0);
            out.push(Plugin {
                nombre,
                presente: p.exists(),
                bytes,
            });
        }
    }
    out.sort_by(|a, b| a.nombre.cmp(&b.nombre));
    out
}

/// Estado real de la instalación de Xoxete y de su memoria.
pub fn estado() -> XoxeteEstado {
    let raiz = raiz();
    let config = cfg_json();
    let mut st = XoxeteEstado {
        raiz: raiz.display().to_string(),
        config: config.display().to_string(),
        ..Default::default()
    };

    // --- configuración -------------------------------------------------
    match std::fs::read(&config) {
        Ok(bytes) => {
            // La config lleva BOM: se descarta antes de parsear.
            let texto = String::from_utf8_lossy(&bytes);
            let limpio = texto.trim_start_matches('\u{feff}');
            match serde_json::from_str::<serde_json::Value>(limpio) {
                Ok(v) => {
                    st.config_ok = true;
                    let s = |k: &str| {
                        v.get(k)
                            .and_then(|x| x.as_str())
                            .unwrap_or("")
                            .to_string()
                    };
                    st.agente = s("default_agent");
                    st.modelo = s("model");
                    st.modelo_pequeno = s("small_model");
                    if let Some(arr) = v.get("instructions").and_then(|x| x.as_array()) {
                        st.instrucciones = arr
                            .iter()
                            .filter_map(|x| x.as_str())
                            .map(|x| x.to_string())
                            .collect();
                    }
                    // El agente Xoxete declara su propio modelo; si difiere del
                    // global, manda el del agente. Se refleja tal cual.
                    if let Some(a) = v
                        .get("agent")
                        .and_then(|a| a.get(&st.agente))
                        .and_then(|a| a.get("model"))
                        .and_then(|m| m.as_str())
                    {
                        if !a.is_empty() {
                            st.modelo = a.to_string();
                        }
                    }
                }
                Err(e) => st.notas.push(format!("opencode.json no se pudo leer: {e}")),
            }
        }
        Err(e) => st
            .notas
            .push(format!("no se encontró {}: {e}", config.display())),
    }

    let dir_cfg = config.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    for ruta in &st.instrucciones {
        if PathBuf::from(ruta).exists() {
            st.instrucciones_ok += 1;
        }
    }
    if dir_cfg.join("AGENTS.md").exists() {
        st.instrucciones_ok += 1;
    } else {
        st.notas.push("falta AGENTS.md en la configuración de Xoxete".into());
    }

    // --- motor de memoria ---------------------------------------------
    let base = base_memoria();
    let mut m = Motor {
        ruta: motor_py().display().to_string(),
        base: base.display().to_string(),
        bytes: std::fs::metadata(&base).map(|x| x.len()).unwrap_or(0),
        ..Default::default()
    };
    match motor(&["stats"]) {
        Ok(v) => {
            m.disponible = true;
            m.total = v.get("total").and_then(|x| x.as_u64()).unwrap_or(0) as usize;
            m.por_tipo = cuenta_de(v.get("por_tipo"));
            m.por_estado = cuenta_de(v.get("por_estado"));
        }
        Err(e) => {
            m.error = Some(e.clone());
            st.notas.push(format!("motor de memoria no responde: {e}"));
        }
    }
    st.motor = m;

    // --- plugins -------------------------------------------------------
    st.plugins = plugins_presentes();
    let esperados = [
        "xoxete-learning.ts",
        "xoxete-router.ts",
        "xoxete-web.ts",
        "security-agent.ts",
    ];
    for e in esperados {
        if !st.plugins.iter().any(|p| p.nombre == e) {
            st.notas.push(format!("plugin esperado no encontrado: {e}"));
        }
    }

    // --- binarios ------------------------------------------------------
    let binarios = [
        ("CLI oficial", raiz.join("opencode-original").join("opencode.exe")),
        ("CLI Evil", raiz.join("evil").join("opencode-evil.exe")),
    ];
    for (nombre, ruta) in binarios {
        let existe = ruta.exists();
        let version = if existe { version_de(&ruta) } else { String::new() };
        st.binarios.push(Binario {
            nombre: nombre.to_string(),
            ruta: ruta.display().to_string(),
            existe,
            version,
        });
    }

    st.notas.push(
        "En OpenCode 2.2.5 cualquier fichero en opencode\\plugin deja la activación de plugins sin terminar (503 en /api/provider); el cerebro completo va por el CLI 1.19.3.".into(),
    );
    st
}

/* ============================ Memoria ============================ */

fn memoria_de(v: &serde_json::Value) -> Memoria {
    let s = |k: &str| {
        v.get(k)
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string()
    };
    Memoria {
        id: v.get("id").and_then(|x| x.as_i64()).unwrap_or(0),
        tipo: s("tipo"),
        estado: s("estado"),
        contexto: s("contexto"),
        accion: s("accion"),
        resultado: s("resultado"),
        confianza: v.get("confianza").and_then(|x| x.as_i64()).unwrap_or(0),
    }
}

/// Busca recuerdos en la base original. Solo lectura.
pub fn buscar(consulta: &str, limite: usize) -> Result<Vec<Memoria>, String> {
    let texto = consulta.trim();
    if texto.is_empty() {
        return Err("falta el texto a buscar".into());
    }
    let tope = limite.clamp(1, 30).to_string();
    let v = motor(&["search", texto, "--limit", &tope])?;
    Ok(v.get("resultados")
        .and_then(|x| x.as_array())
        .map(|arr| arr.iter().map(memoria_de).collect())
        .unwrap_or_default())
}

/// Guarda una experiencia en la misma base de Xoxete (acción explícita).
pub fn guardar(
    tipo: &str,
    contexto: &str,
    accion: &str,
    resultado: &str,
    evidencia: &str,
) -> Result<String, String> {
    if contexto.trim().is_empty() && resultado.trim().is_empty() {
        return Err("hace falta al menos contexto o resultado".into());
    }
    let tipo = if tipo.trim().is_empty() {
        "experiencia"
    } else {
        tipo.trim()
    };
    let v = motor(&[
        "add",
        "--tipo",
        tipo,
        "--contexto",
        contexto.trim(),
        "--accion",
        accion.trim(),
        "--resultado",
        resultado.trim(),
        "--evidencia",
        evidencia.trim(),
        "--fuente",
        "kukia-code",
        "--estado",
        "pendiente",
    ])?;
    let id = v.get("id").and_then(|x| x.as_i64()).unwrap_or(0);
    ok(format!("Guardado como memoria #{id} ({tipo}) en la base de Xoxete."))
}

fn ok(msg: String) -> Result<String, String> {
    Ok(msg)
}

/* ============================ Pruebas ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raiz_respeta_entorno() {
        std::env::set_var("KUKIA_XOXETE_ROOT", r"D:\prueba");
        assert_eq!(raiz(), PathBuf::from(r"D:\prueba"));
        std::env::remove_var("KUKIA_XOXETE_ROOT");
    }

    #[test]
    fn ultimo_json_ignora_logs() {
        let t = "ruido\n{\"ok\":true,\"total\":3}\n";
        let v = ultimo_json(t).expect("json");
        assert_eq!(v["total"], 3);
    }

    #[test]
    fn ultimo_json_falla_sin_json() {
        assert!(ultimo_json("solo texto").is_err());
    }

    /// Si la instalación no está en esta máquina, no se inventa nada: se
    /// comprueba que el estado se construye y lo declara.
    #[test]
    fn estado_no_entra_en_panico() {
        let st = estado();
        assert!(!st.raiz.is_empty());
    }
}


