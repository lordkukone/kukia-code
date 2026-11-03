//! Catálogo de estilos del convertidor universal (RC1).
//!
//! Un estilo es **un dato** (`workflows/studio/styles.json`), no código: añadir
//! uno nuevo no toca el núcleo. El diagnóstico compara los modelos que pide cada
//! paquete contra los que el motor ve de verdad, y el presupuesto de VRAM/RAM
//! contra lo que hay libre ahora.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::studio::engines;

pub fn dir() -> PathBuf {
    if let Ok(p) = std::env::var("KUKIA_STUDIO_DIR") {
        return PathBuf::from(p);
    }
    PathBuf::from(r"H:\KUKIA_CODE\workflows\studio")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Estilo {
    pub id: String,
    pub titulo: String,
    /// Orígenes que este paquete sabe tratar (`anime`, `cartoon`, `cgi`, ...).
    #[serde(default)]
    pub origen: Vec<String>,
    /// Destino: `photoreal`, `anime`, `cartoon`, `game`, ...
    pub destino: String,
    /// `imagen` | `video`
    pub modalidad: String,
    pub motor: String,
    /// Plantilla concreta del estudio (`anime2real_ltx_video`, etc.).
    #[serde(default)]
    pub plantilla: String,
    /// Nombres exactos de fichero que deben existir en el motor.
    #[serde(default)]
    pub modelos: Vec<String>,
    #[serde(default)]
    pub resolucion: Vec<u32>,
    #[serde(default)]
    pub frames: u32,
    #[serde(default)]
    pub pasos: u32,
    #[serde(default)]
    pub vram_min_gb: f64,
    #[serde(default)]
    pub ram_min_gb: f64,
    /// Lo que se declara de antemano: `disponible`, `requiere_prueba`, `experimental`.
    pub estado: String,
    #[serde(default)]
    pub limites: String,
    #[serde(default)]
    pub licencia: String,
    /// Medida real si ya se ha probado (`905,6 s · 15.995 MiB · 49 frames`).
    #[serde(default)]
    pub medido: String,
}

#[derive(Debug, Clone, Deserialize)]
struct Catalogo {
    #[serde(default)]
    version: String,
    #[serde(default)]
    estilos: Vec<Estilo>,
}

pub fn cargar() -> Result<Vec<Estilo>, String> {
    let ruta = dir().join("styles.json");
    let crudo = std::fs::read_to_string(&ruta)
        .map_err(|e| format!("no se pudo leer {}: {e}", ruta.display()))?;
    let c: Catalogo =
        serde_json::from_str(&crudo).map_err(|e| format!("styles.json inválido: {e}"))?;
    Ok(c.estilos)
}

pub fn version_catalogo() -> String {
    let ruta = dir().join("styles.json");
    std::fs::read_to_string(ruta)
        .ok()
        .and_then(|c| serde_json::from_str::<Catalogo>(&c).ok())
        .map(|c| c.version)
        .unwrap_or_default()
}

#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticoEstilo {
    pub id: String,
    pub titulo: String,
    pub origen: Vec<String>,
    pub destino: String,
    pub modalidad: String,
    pub motor: String,
    pub plantilla: String,
    /// Lo que el catálogo declara.
    pub estado_declarado: String,
    /// Lo que se deduce de lo que hay instalado.
    pub estado_real: String,
    pub veredicto: String,
    pub motivo: String,
    pub listos: Vec<String>,
    pub faltan: Vec<String>,
    pub motor_online: bool,
    pub vram_min_gb: f64,
    pub ram_min_gb: f64,
    pub vram_libre_max_gb: f64,
    pub ram_libre_gb: f64,
    pub medido: String,
    pub limites: String,
    pub licencia: String,
}

/// Diagnóstico completo: estilos + motores + presupuesto.
pub async fn diagnostico() -> Result<Vec<DiagnosticoEstilo>, String> {
    let estilos = cargar()?;
    let base = engines::comfy_default();
    let visibles = crate::anime_real::modelos_visibles(&base)
        .await
        .unwrap_or_default();
    let motores = engines::todos().await;
    let comfy_online = motores
        .iter()
        .find(|m| m.id == "comfy")
        .map(|m| m.online)
        .unwrap_or(false);
    let presupuesto = engines::presupuesto();
    let vram_libre = engines::vram_max_libre_gb(&presupuesto);

    Ok(estilos
        .into_iter()
        .map(|e| {
            let (listos, faltan): (Vec<String>, Vec<String>) = e
                .modelos
                .iter()
                .cloned()
                .partition(|m| visibles.iter().any(|v| v == m || v.ends_with(m.as_str())));

            let motor_online = match e.motor.as_str() {
                "comfy_ltx" | "comfy_wan" | "comfy_image" | "comfy" => comfy_online,
                "ollama" => motores.iter().any(|m| m.id == "ollama" && m.online),
                _ => false,
            };

            let mut avisos: Vec<String> = Vec::new();
            let estado_real;
            if !motor_online {
                estado_real = "motor_offline".to_string();
                avisos.push("el motor no responde".into());
            } else if !faltan.is_empty() {
                estado_real = "faltan_modelos".to_string();
                avisos.push(format!("faltan {} modelo(s) en el motor", faltan.len()));
            } else if e.estado == "experimental" {
                estado_real = "experimental".to_string();
                avisos.push("todas las piezas están, pero nadie lo ha aprobado a la vista".into());
            } else if e.estado == "requiere_prueba" {
                estado_real = "listo_sin_probar".to_string();
                avisos.push("piezas completas; falta medir el resultado".into());
            } else {
                estado_real = "listo".to_string();
            }

            if e.vram_min_gb > 0.0 && vram_libre > 0.0 && e.vram_min_gb > vram_libre + 0.05 {
                avisos.push(format!(
                    "pide {:.0} GB de VRAM y hay {:.1} GB libres en la mejor tarjeta",
                    e.vram_min_gb, vram_libre
                ));
            }
            if e.ram_min_gb > 0.0 && e.ram_min_gb > presupuesto.ram_libre_gb {
                avisos.push(format!(
                    "pide {:.0} GB de RAM y hay {:.1} GB libres",
                    e.ram_min_gb, presupuesto.ram_libre_gb
                ));
            }

            let veredicto = if estado_real == "listo" && avisos.is_empty() {
                "puede ejecutarse".to_string()
            } else if estado_real == "listo" || estado_real == "listo_sin_probar" {
                format!("puede ejecutarse con avisos: {}", avisos.join("; "))
            } else {
                format!("no ejecutable ahora: {}", avisos.join("; "))
            };

            DiagnosticoEstilo {
                id: e.id,
                titulo: e.titulo,
                origen: e.origen,
                destino: e.destino,
                modalidad: e.modalidad,
                motor: e.motor,
                plantilla: e.plantilla,
                estado_declarado: e.estado,
                estado_real,
                veredicto,
                motivo: if faltan.is_empty() {
                    String::new()
                } else {
                    format!("faltan: {}", faltan.join(", "))
                },
                listos,
                faltan,
                motor_online,
                vram_min_gb: e.vram_min_gb,
                ram_min_gb: e.ram_min_gb,
                vram_libre_max_gb: (vram_libre * 10.0).round() / 10.0,
                ram_libre_gb: (presupuesto.ram_libre_gb * 10.0).round() / 10.0,
                medido: e.medido,
                limites: e.limites,
                licencia: e.licencia,
            }
        })
        .collect())
}

/// Elige el paquete que mejor encaja con un origen detectado y un destino.
/// Devuelve `None` si no hay ninguno: no se inventa un estilo de emergencia.
pub fn elegir(origen: &str, destino: &str, modalidad: &str) -> Option<Estilo> {
    let estilos = cargar().ok()?;
    let o = origen.to_lowercase();
    let d = destino.to_lowercase();
    let mut candidatos: Vec<Estilo> = estilos
        .into_iter()
        .filter(|e| e.modalidad == modalidad)
        .filter(|e| d.is_empty() || e.destino.to_lowercase() == d)
        .collect();
    // Preferencia por coincidencia exacta de origen, luego cualquiera del destino.
    candidatos.sort_by_key(|e| {
        let exacto = e.origen.iter().any(|x| x.to_lowercase() == o);
        (!exacto) as u8
    });
    candidatos.into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_catalogo_se_lee_y_es_coherente() {
        let e = cargar().expect("styles.json legible");
        assert!(e.len() >= 10, "el catálogo debe cubrir las conversiones");
        for s in &e {
            assert!(!s.id.is_empty());
            assert!(!s.titulo.is_empty());
            assert!(!s.destino.is_empty());
            assert!(
                s.modalidad == "imagen" || s.modalidad == "video",
                "{}: modalidad inválida '{}'",
                s.id,
                s.modalidad
            );
            assert!(
                ["disponible", "requiere_prueba", "experimental"].contains(&s.estado.as_str()),
                "{}: estado declarado desconocido '{}'",
                s.id,
                s.estado
            );
            assert!(
                !s.origen.is_empty(),
                "{}: sin orígenes no se puede enrutar",
                s.id
            );
        }
        let ids: Vec<&str> = e.iter().map(|s| s.id.as_str()).collect();
        let mut unicos = ids.clone();
        unicos.sort();
        unicos.dedup();
        assert_eq!(ids.len(), unicos.len(), "hay ids repetidos");
    }

    #[test]
    fn no_se_inventa_un_estilo_que_no_existe() {
        assert!(elegir("anime", "photoreal", "video").is_some());
        assert!(elegir("anime", "no_existe_este_destino", "video").is_none());
        assert!(elegir("anime", "photoreal", "holograma").is_none());
    }
}
