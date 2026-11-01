//! Permisos granulares por agente y protección por proyectos (Fase 3).
//!
//! Reglas de diseño:
//! - Denegar por defecto todo lo que no esté concedido expresamente.
//! - La escritura exige política + aprobación explícita + ruta dentro de la allowlist.
//! - Las rutas se comparan ya normalizadas, para que `..` no escape del proyecto.
//! - Los comandos destructivos exigen aprobación y política de escritura.

use crate::memory::{AgentPolicy, Project};
use std::path::{Component, Path, PathBuf};

/// Herramientas conocidas y qué permiso consumen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Read,
    Write,
    Shell,
    Network,
    Download,
    ExternalTools,
}

impl Tool {
    pub fn parse(name: &str) -> Option<Tool> {
        match name.trim().to_lowercase().as_str() {
            "files.read" | "read" => Some(Tool::Read),
            "files.write" | "write" => Some(Tool::Write),
            "terminal.run" | "shell" => Some(Tool::Shell),
            "network" => Some(Tool::Network),
            "download" | "files.download" => Some(Tool::Download),
            "tools" | "external_tools" => Some(Tool::ExternalTools),
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Tool::Read => "lectura",
            Tool::Write => "escritura",
            Tool::Shell => "ejecución",
            Tool::Network => "red",
            Tool::Download => "descargas",
            Tool::ExternalTools => "herramientas externas",
        }
    }
}

/// Normaliza una ruta sin tocar el disco: resuelve `.` y `..` de forma lógica.
pub fn normalize(path: &str) -> PathBuf {
    let p = Path::new(path);
    let mut out = PathBuf::new();
    for comp in p.components() {
        match comp {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Ruta real comparable: resuelve enlaces y *junctions* y quita el prefijo `\\?\`.
///
/// Si la ruta aún no existe (lo normal al escribir un archivo nuevo) se resuelve el
/// directorio padre, que es lo que impide escapar de la allowlist con un junction.
fn real_path(path: &str) -> String {
    let p = Path::new(path);
    let resuelto = match std::fs::canonicalize(p) {
        Ok(c) => c,
        Err(_) => match (p.parent(), p.file_name()) {
            (Some(padre), Some(nombre)) if !padre.as_os_str().is_empty() => {
                match std::fs::canonicalize(padre) {
                    Ok(cp) => cp.join(nombre),
                    Err(_) => normalize(path),
                }
            }
            _ => normalize(path),
        },
    };
    let mut s = resuelto.to_string_lossy().to_lowercase().replace('/', "\\");
    if let Some(resto) = s.strip_prefix("\\\\?\\unc\\") {
        s = format!("\\\\{resto}");
    } else if let Some(resto) = s.strip_prefix("\\\\?\\") {
        s = resto.to_string();
    }
    s.trim_end_matches('\\').to_string()
}

fn dentro_de(path: &str, base: &str) -> bool {
    let b = real_path(base);
    if b.is_empty() {
        return false;
    }
    let p = real_path(path);
    p == b || p.starts_with(&format!("{b}\\"))
}

/// ¿La ruta cae dentro de alguna de las rutas permitidas?
pub fn path_in_list(path: &str, list: &str) -> bool {
    AgentPolicy::split(list)
        .iter()
        .any(|base| dentro_de(path, base))
}

/// Comprueba si una ruta está protegida como solo lectura.
pub fn is_read_only(path: &str, policy: &AgentPolicy) -> bool {
    path_in_list(path, &policy.read_only_paths)
}

/// Verbos que pueden destruir datos (se buscan como token suelto, nunca como subcadena).
const VERBOS_DESTRUCTIVOS: [&str; 17] = [
    "remove-item",
    "ri",
    "rm",
    "del",
    "erase",
    "rd",
    "rmdir",
    "format",
    "format-volume",
    "clear-disk",
    "clear-content",
    "remove-partition",
    "initialize-disk",
    "diskpart",
    "sdelete",
    "clear-recyclebin",
    "takeown",
];

/// Trocea el comando respetando comillas: lo entrecomillado cuenta como un solo token,
/// así una ruta con la palabra «del» dentro no se confunde con el comando `del`.
fn tokens(command: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut actual = String::new();
    let mut comilla: Option<char> = None;
    for ch in command.chars() {
        match comilla {
            Some(q) => {
                if ch == q {
                    comilla = None;
                } else {
                    actual.push(ch);
                }
            }
            None => match ch {
                '\'' | '"' => comilla = Some(ch),
                c if c.is_whitespace() || "|;&(){},=".contains(c) => {
                    if !actual.is_empty() {
                        out.push(std::mem::take(&mut actual));
                    }
                }
                c => actual.push(c),
            },
        }
    }
    if !actual.is_empty() {
        out.push(actual);
    }
    out
}

/// Comandos que pueden destruir datos. Se detectan por **verbo**, no por subcadena:
/// `rd /s /q` (que antes se colaba) ahora se detecta, y una ruta con «del» no da falso positivo.
pub fn is_destructive(command: &str) -> bool {
    let c = command.to_lowercase();
    const SECUENCIAS: [&str; 7] = [
        "git reset --hard",
        "git clean -f",
        "git checkout --",
        "git restore",
        "drop table",
        "drop database",
        "truncate table",
    ];
    if SECUENCIAS.iter().any(|s| c.contains(s)) {
        return true;
    }
    if c.contains("robocopy") && (c.contains("/mir") || c.contains("/purge") || c.contains("/mov")) {
        return true;
    }
    tokens(&c)
        .iter()
        .any(|t| VERBOS_DESTRUCTIVOS.contains(&t.as_str()))
}

/// Resultado de una comprobación de permiso.
pub struct Check {
    pub allowed: bool,
    pub reason: String,
}

impl Check {
    fn ok() -> Self {
        Self {
            allowed: true,
            reason: String::new(),
        }
    }
    fn deny(reason: impl Into<String>) -> Self {
        Self {
            allowed: false,
            reason: reason.into(),
        }
    }
}

/// Comprobación central. `approved` es la aprobación explícita del usuario en la UI.
pub fn check(policy: &AgentPolicy, tool: Tool, target: &str, approved: bool) -> Check {
    match tool {
        Tool::Read => {
            if !policy.allow_read {
                return Check::deny("el agente no tiene permiso de lectura");
            }
            let allowed_paths = policy.read_paths.trim();
            if !allowed_paths.is_empty() && !path_in_list(target, allowed_paths) {
                return Check::deny(format!(
                    "ruta fuera de las permitidas para lectura: {target}"
                ));
            }
            Check::ok()
        }
        Tool::Write => {
            if !policy.allow_write {
                return Check::deny("el agente no tiene permiso de escritura");
            }
            if !approved {
                return Check::deny("escritura bloqueada: falta aprobación explícita");
            }
            // El solo-lectura se comprueba antes que la allowlist: si alguien intenta escribir
            // en el CEREBRO, el mensaje debe decir por qué (es de solo lectura), no que está
            // «fuera de las rutas».
            if is_read_only(target, policy) {
                return Check::deny(format!("escritura bloqueada: {target} es de solo lectura"));
            }
            if policy.write_paths.trim().is_empty() {
                return Check::deny(
                    "escritura bloqueada: el agente no tiene ninguna ruta autorizada",
                );
            }
            if !path_in_list(target, &policy.write_paths) {
                return Check::deny(format!(
                    "escritura bloqueada: {target} está fuera de las rutas autorizadas"
                ));
            }
            Check::ok()
        }
        Tool::Shell => {
            if !policy.allow_shell {
                return Check::deny("el agente no tiene permiso de ejecución");
            }
            if !approved {
                return Check::deny("ejecución bloqueada: falta aprobación explícita");
            }
            if is_destructive(target) {
                if !policy.allow_write {
                    return Check::deny(
                        "comando destructivo bloqueado: el agente no tiene permiso de escritura",
                    );
                }
                if !approved {
                    return Check::deny("comando destructivo bloqueado: requiere aprobación");
                }
            }
            Check::ok()
        }
        Tool::Network => {
            if !policy.allow_network {
                return Check::deny("el agente no tiene permiso de red");
            }
            Check::ok()
        }
        Tool::Download => {
            if !policy.allow_download {
                return Check::deny("el agente no tiene permiso de descargas");
            }
            if !target.starts_with("https://") {
                return Check::deny("solo se permiten descargas https");
            }
            if !approved {
                return Check::deny("descarga bloqueada: falta aprobación explícita");
            }
            Check::ok()
        }
        Tool::ExternalTools => {
            if !policy.allow_tools {
                return Check::deny("el agente no tiene permiso de herramientas externas");
            }
            Check::ok()
        }
    }
}

/// Comprobación con el ámbito de un proyecto activo (Fase 3A).
///
/// La política del agente y la del proyecto se exigen **las dos**: el proyecto no
/// puede ampliar lo que el agente no tiene, y el agente no puede salirse del proyecto.
pub fn check_scoped(
    policy: &AgentPolicy,
    scope: Option<&Project>,
    tool: Tool,
    target: &str,
    approved: bool,
) -> Check {
    let base = check(policy, tool, target, approved);
    if !base.allowed {
        return base;
    }
    let Some(p) = scope else {
        return base;
    };
    match tool {
        Tool::Read => {
            if !p.read_paths.trim().is_empty() && !path_in_list(target, &p.read_paths) {
                return Check::deny(format!(
                    "lectura fuera del proyecto «{}»: {target}",
                    p.name
                ));
            }
            Check::ok()
        }
        Tool::Write => {
            if !p.allow_write {
                return Check::deny(format!("el proyecto «{}» no permite escritura", p.name));
            }
            if !p.read_only_paths.trim().is_empty() && path_in_list(target, &p.read_only_paths) {
                return Check::deny(format!(
                    "escritura bloqueada: {target} es de solo lectura en el proyecto «{}»",
                    p.name
                ));
            }
            if p.write_paths.trim().is_empty() || !path_in_list(target, &p.write_paths) {
                return Check::deny(format!(
                    "escritura fuera del proyecto «{}»: {target}",
                    p.name
                ));
            }
            Check::ok()
        }
        Tool::Shell => {
            if !p.allow_shell {
                return Check::deny(format!(
                    "el proyecto «{}» no permite ejecutar comandos",
                    p.name
                ));
            }
            Check::ok()
        }
        Tool::Network => {
            if !p.allow_network {
                return Check::deny(format!("el proyecto «{}» no permite red", p.name));
            }
            Check::ok()
        }
        _ => Check::ok(),
    }
}

/// Como `check_cwd_in_scope`, pero exigiendo además estar dentro del proyecto activo.
pub fn check_cwd_in_scope_scoped(
    policy: &AgentPolicy,
    scope: Option<&Project>,
    cwd: &str,
) -> Check {
    let base = check_cwd_in_scope(policy, cwd);
    if !base.allowed {
        return base;
    }
    let Some(p) = scope else {
        return base;
    };
    if !p.allow_shell {
        return Check::deny(format!(
            "ejecución bloqueada: el proyecto «{}» no permite comandos",
            p.name
        ));
    }
    if p.write_paths.trim().is_empty() || !path_in_list(cwd, &p.write_paths) {
        return Check::deny(format!(
            "ejecución fuera del proyecto «{}»: {cwd}",
            p.name
        ));
    }
    if !p.read_only_paths.trim().is_empty() && path_in_list(cwd, &p.read_only_paths) {
        return Check::deny(format!(
            "ejecución bloqueada: {cwd} es de solo lectura en el proyecto «{}»",
            p.name
        ));
    }
    Check::ok()
}

/// El terminal solo puede ejecutarse dentro de las rutas autorizadas.
///
/// Sin esto, la allowlist de escritura sería decorativa: bastaría `Set-Content`
/// desde el terminal para escribir donde no se permite.
pub fn check_cwd_in_scope(policy: &AgentPolicy, cwd: &str) -> Check {
    if policy.write_paths.trim().is_empty() {
        return Check::deny(
            "ejecución bloqueada: el agente no tiene ninguna ruta autorizada. \
             Añádela en Configuración → Permisos",
        );
    }
    if !path_in_list(cwd, &policy.write_paths) {
        return Check::deny(format!(
            "ejecución bloqueada: {cwd} está fuera de las rutas autorizadas"
        ));
    }
    if is_read_only(cwd, policy) {
        return Check::deny(format!("ejecución bloqueada: {cwd} es de solo lectura"));
    }
    Check::ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn politica(escritura: bool, rutas: &str) -> AgentPolicy {
        let mut p = AgentPolicy::default_for("prueba");
        p.allow_write = escritura;
        p.write_paths = rutas.into();
        p
    }

    #[test]
    fn escritura_sin_politica_se_deniega() {
        let p = politica(false, "H:\\proyecto");
        let c = check(&p, Tool::Write, "H:\\proyecto\\a.txt", true);
        assert!(!c.allowed);
        assert!(c.reason.contains("no tiene permiso"));
    }

    #[test]
    fn escritura_sin_aprobacion_se_deniega() {
        let p = politica(true, "H:\\proyecto");
        let c = check(&p, Tool::Write, "H:\\proyecto\\a.txt", false);
        assert!(!c.allowed);
        assert!(c.reason.contains("aprobaci"));
    }

    #[test]
    fn escritura_dentro_de_ruta_se_permite() {
        let p = politica(true, "H:\\proyecto");
        let c = check(&p, Tool::Write, "H:\\proyecto\\sub\\a.txt", true);
        assert!(c.allowed, "{}", c.reason);
    }

    #[test]
    fn escritura_fuera_de_ruta_se_deniega() {
        let p = politica(true, "H:\\proyecto");
        let c = check(&p, Tool::Write, "C:\\Windows\\system32\\x.dll", true);
        assert!(!c.allowed);
        assert!(c.reason.contains("fuera"));
    }

    #[test]
    fn escape_con_puntos_suspensivos_se_bloquea() {
        let p = politica(true, "H:\\proyecto");
        let c = check(&p, Tool::Write, "H:\\proyecto\\..\\otro\\a.txt", true);
        assert!(!c.allowed, "el .. no debe permitir salir del proyecto");
    }

    #[test]
    fn solo_lectura_bloquea_escritura() {
        let mut p = politica(true, "H:\\proyecto");
        p.read_only_paths = "H:\\proyecto\\datos".into();
        let c = check(&p, Tool::Write, "H:\\proyecto\\datos\\a.txt", true);
        assert!(!c.allowed);
        assert!(c.reason.contains("solo lectura"));
    }

    #[test]
    fn comandos_destructivos_se_detectan() {
        assert!(is_destructive("Remove-Item H:\\x -Recurse"));
        assert!(is_destructive("del C:\\temp\\a.txt"));
        assert!(is_destructive("rm -rf /"));
        assert!(!is_destructive("Get-ChildItem H:\\"));
        assert!(!is_destructive("npm run build"));
    }

    #[test]
    fn shell_sin_permiso_se_deniega() {
        let p = AgentPolicy::default_for("prueba");
        let c = check(&p, Tool::Shell, "Get-Date", true);
        assert!(!c.allowed);
    }

    #[test]
    fn descarga_exige_https() {
        let mut p = AgentPolicy::default_for("prueba");
        p.allow_download = true;
        assert!(!check(&p, Tool::Download, "http://x/y.zip", true).allowed);
        assert!(check(&p, Tool::Download, "https://x/y.zip", true).allowed);
    }

    #[test]
    fn normalizacion_resuelve_padres() {
        let n = normalize("H:\\a\\b\\..\\c");
        assert_eq!(n.to_string_lossy().to_lowercase(), "h:\\a\\c");
    }

    /* -------- Fase 3.0: destructivos por verbo (E-05) -------- */

    #[test]
    fn detecta_rd_slash_s_que_antes_se_colaba() {
        // E-05: `rd /s /q` no llevaba espacio delante y no se detectaba.
        assert!(is_destructive("rd /s /q C:\\datos"));
        assert!(is_destructive("Rd /S /Q H:\\x"));
    }

    #[test]
    fn detecta_borrados_por_verbo_y_secuencia() {
        assert!(is_destructive("Remove-Item H:\\x -Recurse -Force"));
        assert!(is_destructive("Get-ChildItem C:\\tmp | Remove-Item -Force"));
        assert!(is_destructive("robocopy a b /MIR"));
        assert!(is_destructive("diskpart"));
        assert!(is_destructive("git clean -fdx"));
        assert!(is_destructive("git checkout -- ."));
        assert!(is_destructive("takeown /f C:\\Windows"));
    }

    #[test]
    fn no_marca_como_destructivo_lo_inocente() {
        assert!(!is_destructive("Get-ChildItem H:\\"));
        assert!(!is_destructive("npm run build"));
        assert!(!is_destructive("cargo test -p kukia-core"));
        assert!(!is_destructive("Get-Content \"C:\\model del dia.txt\""));
        assert!(!is_destructive("git status"));
        assert!(!is_destructive("robocopy a b /E"));
    }

    /* -------- Fase 3.0: alcance del terminal -------- */

    #[test]
    fn terminal_sin_rutas_se_deniega() {
        let p = AgentPolicy::default_for("prueba");
        let c = check_cwd_in_scope(&p, "C:\\");
        assert!(!c.allowed);
        assert!(c.reason.contains("ninguna ruta"), "{}", c.reason);
    }

    #[test]
    fn terminal_fuera_de_rutas_se_deniega() {
        let p = politica(true, "H:\\KUKIA_CODE");
        let c = check_cwd_in_scope(&p, "C:\\Windows\\System32");
        assert!(!c.allowed);
        assert!(c.reason.contains("fuera"), "{}", c.reason);
    }

    #[test]
    fn terminal_dentro_de_rutas_se_permite() {
        let p = politica(true, "H:\\KUKIA_CODE");
        let c = check_cwd_in_scope(&p, "H:\\KUKIA_CODE");
        assert!(c.allowed, "{}", c.reason);
    }

    #[test]
    fn terminal_en_ruta_de_solo_lectura_se_deniega() {
        let mut p = politica(true, "I:\\CEREBRO");
        p.read_only_paths = "I:\\CEREBRO".into();
        let c = check_cwd_in_scope(&p, "I:\\CEREBRO");
        assert!(!c.allowed);
        assert!(c.reason.contains("solo lectura"), "{}", c.reason);
    }

    /* -------- Fase 3.0: junction no escapa de la allowlist (E-04) -------- */

    #[test]
    fn junction_no_permite_escapar() {
        let base = std::env::temp_dir().join("kukia_perm_base");
        let fuera = std::env::temp_dir().join("kukia_perm_fuera");
        let enlace = base.join("link");
        let _ = std::fs::remove_dir_all(&base);
        let _ = std::fs::remove_dir_all(&fuera);
        std::fs::create_dir_all(&base).unwrap();
        std::fs::create_dir_all(&fuera).unwrap();

        let salida = std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(&enlace)
            .arg(&fuera)
            .output();
        let creado = salida.map(|o| o.status.success()).unwrap_or(false) && enlace.exists();
        if !creado {
            println!("AVISO: no se pudo crear el junction; prueba omitida");
            return;
        }

        let p = politica(true, base.to_str().unwrap());
        let intruso = enlace.join("intruso.txt");
        let c = check(&p, Tool::Write, intruso.to_str().unwrap(), true);
        assert!(
            !c.allowed,
            "el junction debe bloquearse, pero se permitió: {}",
            c.reason
        );

        let legitimo = base.join("legitimo.txt");
        let ok = check(&p, Tool::Write, legitimo.to_str().unwrap(), true);
        assert!(ok.allowed, "lo legítimo debe pasar: {}", ok.reason);

        let _ = std::fs::remove_dir_all(&base);
        let _ = std::fs::remove_dir_all(&fuera);
    }
}
