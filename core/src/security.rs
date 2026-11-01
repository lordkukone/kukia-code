use serde::Serialize;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct RunResult {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Ejecuta un comando PowerShell.
///
/// `approved` debe venir de una aprobación explícita del usuario en la UI.
/// El llamador registra la aprobación en la base de datos de auditoría.
pub fn run_powershell(command: &str, cwd: &str, approved: bool) -> Result<RunResult, String> {
    if !approved {
        return Err("Operación bloqueada: falta aprobación explícita".into());
    }
    if command.trim().is_empty() {
        return Err("Comando vacío".into());
    }
    let cwd_path = Path::new(cwd);
    if !cwd_path.is_dir() {
        return Err(format!("Directorio inexistente: {cwd}"));
    }

    let mut cmd = Command::new("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", command])
        .current_dir(cwd_path);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }

    let out = cmd.output().map_err(|e| e.to_string())?;
    Ok(RunResult {
        code: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).to_string(),
        stderr: String::from_utf8_lossy(&out.stderr).to_string(),
    })
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

    #[test]
    fn blocks_without_approval() {
        let r = run_powershell("echo hola", "C:\\", false);
        assert!(r.is_err());
    }

    #[test]
    fn read_rejects_missing() {
        assert!(read_text_file("C:\\no-existe-kukia-xyz.txt", 1024).is_err());
    }
}
