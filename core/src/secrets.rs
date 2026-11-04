//! Claves de proveedores cloud.
//!
//! El archivo `%APPDATA%\KukiaCode\secrets.json` se guarda **cifrado con DPAPI**
//! por usuario (`CryptProtectData`), que es lo que pedía C0: una clave legible
//! por cualquier proceso del usuario es un almacén de dinero.
//!
//! Se mantiene la **lectura del formato antiguo en claro** (`{"opencode": "…"}`)
//! para no perder nada, y `migrar()` lo reescribe cifrado dejando una copia
//! `secrets.json.legacy.bak` al lado.
//!
//! El valor de una clave nunca se registra en logs ni se devuelve a la interfaz:
//! solo `presente / no presente` y el origen.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Versión del sobre cifrado. Ausente o `< 2` significa JSON en claro antiguo.
const VERSION_SOBRE: u32 = 2;

/// Envoltorio que se escribe en disco: el mapa de claves va cifrado en `dpapi`.
#[derive(Debug, Serialize, Deserialize)]
struct Sobre {
    v: u32,
    dpapi: String,
}

pub fn ruta() -> PathBuf {
    crate::config::data_dir().join("secrets.json")
}

fn leer_crudo() -> Option<String> {
    std::fs::read_to_string(ruta()).ok()
}

/// `true` si el archivo existe y está cifrado con DPAPI.
pub fn esta_cifrado() -> bool {
    leer_crudo().map(|raw| es_sobre(&raw)).unwrap_or(false)
}

/// `true` si el archivo existe y sigue en texto plano (pendiente de migrar).
pub fn pendiente_de_migrar() -> bool {
    matches!(leer_crudo(), Some(raw) if !es_sobre(&raw))
}

fn es_sobre(raw: &str) -> bool {
    serde_json::from_str::<Sobre>(raw)
        .map(|s| s.v >= VERSION_SOBRE)
        .unwrap_or(false)
}

/// Mapa de claves en claro. Vacío si no hay archivo o no se puede descifrar.
pub fn leer() -> BTreeMap<String, String> {
    let Some(raw) = leer_crudo() else {
        return BTreeMap::new();
    };
    let texto = if es_sobre(&raw) {
        match descifrar_sobre(&raw) {
            Ok(t) => t,
            Err(_) => return BTreeMap::new(),
        }
    } else {
        raw
    };
    parse_mapa(&texto)
}

/// Clave de un proveedor (`opencode`, `freebuff`, `openai`…), o `None`.
pub fn buscar(id: &str) -> Option<String> {
    let id = id.trim().to_lowercase();
    if id.is_empty() {
        return None;
    }
    leer().get(&id).cloned()
}

/// Nombres de proveedor que tienen clave guardada. **Nunca** los valores.
pub fn proveedores_con_clave() -> Vec<String> {
    leer().keys().cloned().collect()
}

/// Guarda o reemplaza la clave de un proveedor, cifrando el archivo entero.
pub fn guardar(id: &str, clave: &str) -> Result<(), String> {
    let id = id.trim().to_lowercase();
    if id.is_empty() {
        return Err("proveedor vacío".to_string());
    }
    let clave = clave.trim();
    if clave.is_empty() {
        return Err("clave vacía".to_string());
    }
    let mut mapa = leer();
    mapa.insert(id, clave.to_string());
    escribir(&mapa)
}

/// Borra la clave de un proveedor. Devuelve error si no había ninguna.
pub fn borrar(id: &str) -> Result<(), String> {
    let id = id.trim().to_lowercase();
    let mut mapa = leer();
    if mapa.remove(&id).is_none() {
        return Err(format!("no había clave guardada para {id}"));
    }
    escribir(&mapa)
}

/// Migra el archivo en claro a DPAPI. `true` si migró algo.
///
/// Si la copia de seguridad falla **no** se toca el original.
pub fn migrar() -> Result<bool, String> {
    let Some(raw) = leer_crudo() else {
        return Ok(false);
    };
    if es_sobre(&raw) {
        return Ok(false);
    }
    let mapa = parse_mapa(&raw);
    if mapa.is_empty() {
        return Ok(false);
    }
    let destino = ruta();
    let copia = destino.with_extension("json.legacy.bak");
    std::fs::write(&copia, &raw)
        .map_err(|e| format!("no se pudo hacer la copia {}: {e}", copia.display()))?;
    escribir(&mapa)?;
    Ok(true)
}

fn descifrar_sobre(raw: &str) -> Result<String, String> {
    let sobre: Sobre = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    let blob = hex_a_bytes(&sobre.dpapi)?;
    let claro = descifrar(&blob)?;
    String::from_utf8(claro).map_err(|e| e.to_string())
}

fn escribir(mapa: &BTreeMap<String, String>) -> Result<(), String> {
    let json = serde_json::to_string_pretty(mapa).map_err(|e| e.to_string())?;
    let blob = cifrar(json.as_bytes())?;
    let sobre = Sobre {
        v: VERSION_SOBRE,
        dpapi: bytes_a_hex(&blob),
    };
    let raw = serde_json::to_string_pretty(&sobre).map_err(|e| e.to_string())?;
    let destino = ruta();
    if let Some(dir) = destino.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    // Escritura atómica: si falla a mitad, el archivo anterior sigue intacto.
    let tmp = destino.with_extension("json.tmp");
    std::fs::write(&tmp, raw).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &destino).map_err(|e| e.to_string())
}

/// Acepta tanto `{"proveedor": "clave"}` como `{"providers": {"proveedor": "clave"}}`.
fn parse_mapa(texto: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let Ok(v) = serde_json::from_str::<serde_json::Value>(texto) else {
        return out;
    };
    let Some(obj) = v.as_object() else {
        return out;
    };
    for (k, val) in obj {
        if k.eq_ignore_ascii_case("providers") {
            if let Some(p) = val.as_object() {
                for (k2, v2) in p {
                    if let Some(s) = v2.as_str() {
                        insertar(&mut out, k2, s);
                    }
                }
            }
            continue;
        }
        if let Some(s) = val.as_str() {
            insertar(&mut out, k, s);
        }
    }
    out
}

fn insertar(out: &mut BTreeMap<String, String>, k: &str, v: &str) {
    let k = k.trim().to_lowercase();
    let v = v.trim();
    if !k.is_empty() && !v.is_empty() {
        out.insert(k, v.to_string());
    }
}

fn bytes_a_hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for byte in b {
        s.push_str(&format!("{byte:02x}"));
    }
    s
}

fn hex_a_bytes(s: &str) -> Result<Vec<u8>, String> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        return Err("hex con longitud impar".to_string());
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let par = std::str::from_utf8(&bytes[i..i + 2]).map_err(|e| e.to_string())?;
        out.push(u8::from_str_radix(par, 16).map_err(|e| e.to_string())?);
        i += 2;
    }
    Ok(out)
}

/// `CRYPTPROTECT_UI_FORBIDDEN`: nunca mostrar un diálogo desde un proceso de fondo.
const UI_FORBIDDEN: u32 = 0x1;

#[cfg(windows)]
mod dpapi {
    use std::ffi::c_void;

    #[repr(C)]
    pub struct Blob {
        pub cb_data: u32,
        pub pb_data: *mut u8,
    }

    #[link(name = "crypt32")]
    extern "system" {
        pub fn CryptProtectData(
            data_in: *const Blob,
            descripcion: *const u16,
            entropia: *const Blob,
            reservado: *const c_void,
            prompt: *const c_void,
            flags: u32,
            data_out: *mut Blob,
        ) -> i32;
        pub fn CryptUnprotectData(
            data_in: *const Blob,
            descripcion: *mut *mut u16,
            entropia: *const Blob,
            reservado: *const c_void,
            prompt: *const c_void,
            flags: u32,
            data_out: *mut Blob,
        ) -> i32;
    }

    #[link(name = "kernel32")]
    extern "system" {
        pub fn LocalFree(h: *mut c_void) -> *mut c_void;
    }
}

#[cfg(windows)]
fn cifrar(datos: &[u8]) -> Result<Vec<u8>, String> {
    unsafe {
        let entrada = dpapi::Blob {
            cb_data: datos.len() as u32,
            pb_data: datos.as_ptr() as *mut u8,
        };
        let mut salida = dpapi::Blob {
            cb_data: 0,
            pb_data: std::ptr::null_mut(),
        };
        let ok = dpapi::CryptProtectData(
            &entrada,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            UI_FORBIDDEN,
            &mut salida,
        );
        if ok == 0 {
            return Err(format!(
                "CryptProtectData falló: {}",
                std::io::Error::last_os_error()
            ));
        }
        let bytes = std::slice::from_raw_parts(salida.pb_data, salida.cb_data as usize).to_vec();
        dpapi::LocalFree(salida.pb_data as *mut std::ffi::c_void);
        Ok(bytes)
    }
}

#[cfg(windows)]
fn descifrar(datos: &[u8]) -> Result<Vec<u8>, String> {
    unsafe {
        let entrada = dpapi::Blob {
            cb_data: datos.len() as u32,
            pb_data: datos.as_ptr() as *mut u8,
        };
        let mut salida = dpapi::Blob {
            cb_data: 0,
            pb_data: std::ptr::null_mut(),
        };
        let ok = dpapi::CryptUnprotectData(
            &entrada,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            UI_FORBIDDEN,
            &mut salida,
        );
        if ok == 0 {
            return Err(format!(
                "CryptUnprotectData falló: {}",
                std::io::Error::last_os_error()
            ));
        }
        let bytes = std::slice::from_raw_parts(salida.pb_data, salida.cb_data as usize).to_vec();
        dpapi::LocalFree(salida.pb_data as *mut std::ffi::c_void);
        Ok(bytes)
    }
}

#[cfg(not(windows))]
fn cifrar(_datos: &[u8]) -> Result<Vec<u8>, String> {
    Err("DPAPI solo existe en Windows".to_string())
}

#[cfg(not(windows))]
fn descifrar(_datos: &[u8]) -> Result<Vec<u8>, String> {
    Err("DPAPI solo existe en Windows".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_ida_y_vuelta() {
        let datos = b"clave-secreta-\x00\xff";
        assert_eq!(hex_a_bytes(&bytes_a_hex(datos)).unwrap(), datos.to_vec());
    }

    #[test]
    fn hex_impar_es_error() {
        assert!(hex_a_bytes("abc").is_err());
    }

    #[test]
    fn el_formato_plano_antiguo_se_lee() {
        let mapa = parse_mapa(r#"{"opencode":"sk-prueba","freebuff":"otra"}"#);
        assert_eq!(mapa.get("opencode").map(String::as_str), Some("sk-prueba"));
        assert_eq!(mapa.get("freebuff").map(String::as_str), Some("otra"));
    }

    #[test]
    fn el_formato_con_providers_tambien_se_lee() {
        let mapa = parse_mapa(r#"{"providers":{"openai":"sk-1"}}"#);
        assert_eq!(mapa.get("openai").map(String::as_str), Some("sk-1"));
    }

    #[test]
    fn las_claves_vacias_no_cuentan() {
        let mapa = parse_mapa(r#"{"opencode":"   "}"#);
        assert!(mapa.is_empty(), "una clave en blanco no es una clave");
    }

    #[test]
    fn el_sobre_solo_es_sobre_con_version_2_o_mas() {
        assert!(es_sobre(r#"{"v":2,"dpapi":"00"}"#));
        assert!(!es_sobre(r#"{"v":1,"dpapi":"00"}"#));
        assert!(!es_sobre(r#"{"opencode":"sk"}"#));
    }

    #[cfg(windows)]
    #[test]
    fn dpapi_cifra_y_descifra_de_verdad() {
        let claro = br#"{"opencode":"sk-no-debe-verse"}"#;
        let blob = cifrar(claro).expect("cifrar con DPAPI");
        assert_ne!(blob.as_slice(), claro.as_slice(), "el blob no puede ser el claro");
        assert!(
            !String::from_utf8_lossy(&blob).contains("sk-no-debe-verse"),
            "la clave no puede aparecer en claro dentro del blob"
        );
        let vuelta = descifrar(&blob).expect("descifrar con DPAPI");
        assert_eq!(vuelta, claro.to_vec());
    }
}
