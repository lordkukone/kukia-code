//! Ficha del binario que está corriendo.
//!
//! Existe porque el campo de versión llevaba congelado en `0.1.0` desde el MVP y
//! varios binarios distintos se anunciaban igual: en el equipo había uno del
//! 02/11 y el repo producía otro del 03/11, los dos «0.1.0». La interfaz necesita
//! algo que distinga uno de otro sin abrir el explorador de archivos.
//!
//! La fecha de «build» es la de **modificación del ejecutable**. No es un número
//! de compilación grabado en el binario (eso pediría un `build.rs`), pero sí es
//! verificable y no miente: cambia en cada compilación y en cada instalación.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct AppInfo {
    /// Versión declarada (`CARGO_PKG_VERSION`).
    pub version: String,
    /// Fecha y hora de modificación del ejecutable, `AAAA-MM-DD HH:MM`.
    pub build: String,
    /// `true` si `secrets.json` está cifrado con DPAPI.
    pub secrets_dpapi: bool,
    /// `true` si hay claves guardadas pero el archivo sigue en texto plano.
    pub secrets_en_claro: bool,
    /// Número de proveedores con clave guardada (nunca los valores).
    pub proveedores_con_clave: usize,
}

pub fn app_info() -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        build: fecha_del_ejecutable(),
        secrets_dpapi: crate::secrets::esta_cifrado(),
        secrets_en_claro: crate::secrets::pendiente_de_migrar(),
        proveedores_con_clave: crate::secrets::proveedores_con_clave().len(),
    }
}

fn fecha_del_ejecutable() -> String {
    let Ok(ruta) = std::env::current_exe() else {
        return "desconocido".to_string();
    };
    let Ok(meta) = std::fs::metadata(ruta) else {
        return "desconocido".to_string();
    };
    let Ok(modificado) = meta.modified() else {
        return "desconocido".to_string();
    };
    let local: chrono::DateTime<chrono::Local> = modificado.into();
    local.format("%Y-%m-%d %H:%M").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_version_no_esta_vacia() {
        assert!(!app_info().version.trim().is_empty());
    }

    #[test]
    fn la_fecha_del_ejecutable_tiene_forma_de_fecha() {
        let b = fecha_del_ejecutable();
        assert!(
            b == "desconocido" || (b.len() == 16 && b.contains('-') && b.contains(':')),
            "formato inesperado: {b}"
        );
    }
}
