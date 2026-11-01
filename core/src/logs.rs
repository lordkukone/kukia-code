use std::io::Write;
use std::path::{Path, PathBuf};

/// Carpeta de logs. `KUKIA_LOG_DIR` permite aislarla (tests, pruebas, entornos).
pub fn log_dir() -> PathBuf {
    if let Ok(custom) = std::env::var("KUKIA_LOG_DIR") {
        if !custom.trim().is_empty() {
            return PathBuf::from(custom);
        }
    }
    crate::config::data_dir().join("logs")
}

fn log_path() -> PathBuf {
    log_dir().join("kukia.log")
}

/// Escribe en la carpeta indicada. Es la pieza que usan los tests: nunca tocan
/// el log real, que hasta la Fase 3A recibía líneas `[test]` de cada ejecución.
pub fn append_to(dir: &Path, level: &str, message: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let line = format!(
        "{} [{}] {}\n",
        chrono::Utc::now().to_rfc3339(),
        level,
        message.replace('\n', " ")
    );
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("kukia.log"))?;
    f.write_all(line.as_bytes())
}

pub fn tail_from(dir: &Path, lines: usize) -> Vec<String> {
    let raw = std::fs::read_to_string(dir.join("kukia.log")).unwrap_or_default();
    let all: Vec<&str> = raw.lines().collect();
    let start = all.len().saturating_sub(lines);
    all[start..].iter().map(|s| s.to_string()).collect()
}

/// Registro en el log de la aplicación. Nunca falla hacia fuera.
pub fn append(level: &str, message: &str) {
    let _ = append_to(&log_dir(), level, message);
}

pub fn tail(lines: usize) -> Vec<String> {
    tail_from(&log_dir(), lines)
}

/// Ruta del log en uso (para mostrarla en la interfaz).
pub fn path() -> String {
    log_path().display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Carpeta temporal única: los tests jamás escriben en %APPDATA%.
    fn dir_temporal(nombre: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "kukia-log-test-{nombre}-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn append_and_tail_en_carpeta_temporal() {
        let dir = dir_temporal("append");
        append_to(&dir, "test", "linea de prueba aislada").unwrap();
        let t = tail_from(&dir, 50);
        assert!(t.iter().any(|l| l.contains("linea de prueba aislada")));
        // El archivo vive en la carpeta temporal, no en los datos de usuario.
        assert!(dir.join("kukia.log").is_file());
        let _ = std::fs::remove_file(dir.join("kukia.log"));
        let _ = std::fs::remove_dir(dir);
    }

    #[test]
    fn el_log_no_se_mezcla_entre_carpetas() {
        let a = dir_temporal("a");
        let b = dir_temporal("b");
        append_to(&a, "info", "solo en a").unwrap();
        append_to(&b, "info", "solo en b").unwrap();
        assert!(tail_from(&a, 10).iter().any(|l| l.contains("solo en a")));
        assert!(!tail_from(&a, 10).iter().any(|l| l.contains("solo en b")));
        for d in [a, b] {
            let _ = std::fs::remove_file(d.join("kukia.log"));
            let _ = std::fs::remove_dir(d);
        }
    }

    #[test]
    fn la_carpeta_respeta_la_variable_de_entorno() {
        // Sin variable, cuelga de los datos de la aplicación.
        let por_defecto = log_dir();
        assert!(por_defecto.ends_with("logs"));

        // Con variable, se usa la indicada (y no se toca la real).
        let antes = std::env::var("KUKIA_LOG_DIR").ok();
        std::env::set_var("KUKIA_LOG_DIR", r"H:\tmp\kukia-logs-prueba");
        let custom = log_dir();
        assert_eq!(custom, PathBuf::from(r"H:\tmp\kukia-logs-prueba"));
        assert!(path().ends_with("kukia.log"));
        match antes {
            Some(v) => std::env::set_var("KUKIA_LOG_DIR", v),
            None => std::env::remove_var("KUKIA_LOG_DIR"),
        }
    }
}
