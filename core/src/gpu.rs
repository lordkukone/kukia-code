//! Información de GPU y del sistema para la interfaz.
//!
//! La lectura de tarjetas vive en [`crate::gpus`], que funciona con **cualquier**
//! fabricante (DXGI + contadores de Windows) y solo rellena temperatura, potencia
//! y relojes cuando el fabricante los publica (`nvidia-smi`). Aquí se le da la
//! forma que ya consume la interfaz, sin suponer ninguna tarjeta concreta.

use serde::Serialize;

use crate::gpus;

#[derive(Debug, Clone, Serialize)]
pub struct GpuInfo {
    pub index: u32,
    pub name: String,
    /// `nvidia`, `amd`, `intel`, `microsoft` o `desconocido`.
    pub fabricante: String,
    pub memory_total_mb: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct GpuStat {
    pub index: u32,
    pub name: String,
    pub fabricante: String,
    pub utilization: f64,
    pub memory_used_mb: u64,
    pub memory_total_mb: u64,
    pub temperature_c: f64,
    pub power_w: f64,
    pub clock_graphics_mhz: u64,
    pub clock_memory_mhz: u64,
    /// De dónde sale el detalle de esta tarjeta. Se dice en vez de callarlo:
    /// en AMD o Intel no hay temperatura ni potencia, y eso no es un 0 real.
    pub fuente: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemInfo {
    pub os: String,
    pub cpu: String,
    pub cores: usize,
    pub ram_gb: f64,
    pub gpus: Vec<GpuInfo>,
    pub data_dir: String,
}

/// Tarjetas del equipo, de cualquier fabricante.
pub fn gpus() -> Vec<GpuInfo> {
    gpus::tarjetas()
        .into_iter()
        .map(|t| GpuInfo {
            index: t.index,
            name: t.nombre,
            fabricante: t.fabricante,
            memory_total_mb: t.total_mb,
        })
        .collect()
}

/// Uso por tarjeta: memoria siempre (DXGI + contadores), y temperatura, potencia,
/// relojes y utilización solo si el fabricante los publica.
pub fn stats() -> Vec<GpuStat> {
    let smi = gpus::nvidia_smi();
    gpus::tarjetas()
        .into_iter()
        .map(|t| {
            let d = smi
                .iter()
                .find(|s| gpus::normaliza(&s.name) == gpus::normaliza(&t.nombre));
            let usado = d.map(|s| s.used_mb).or(t.usada_mb).unwrap_or(0);
            GpuStat {
                index: t.index,
                name: t.nombre,
                fabricante: t.fabricante,
                utilization: d.map(|s| s.utilization).unwrap_or(0.0),
                memory_used_mb: usado.min(t.total_mb),
                memory_total_mb: t.total_mb,
                temperature_c: d.map(|s| s.temperature_c).unwrap_or(0.0),
                power_w: d.map(|s| s.power_w).unwrap_or(0.0),
                clock_graphics_mhz: d.map(|s| s.clock_graphics_mhz).unwrap_or(0),
                clock_memory_mhz: d.map(|s| s.clock_memory_mhz).unwrap_or(0),
                fuente: if d.is_some() {
                    "nvidia-smi + contadores de Windows".to_string()
                } else {
                    t.fuente_uso.clone()
                },
            }
        })
        .collect()
}

pub fn system_info() -> SystemInfo {
    use sysinfo::System;
    let mut sys = System::new_all();
    sys.refresh_all();

    let cpu = sys
        .cpus()
        .first()
        .map(|c| c.brand().trim().to_string())
        .unwrap_or_else(|| "CPU desconocida".into());
    let cores = sys.cpus().len();
    let ram_gb = sys.total_memory() as f64 / 1e9;

    SystemInfo {
        os: format!(
            "{} {}",
            System::name().unwrap_or_else(|| "Windows".into()),
            System::os_version().unwrap_or_default()
        )
        .trim()
        .to_string(),
        cpu,
        cores,
        ram_gb,
        gpus: gpus(),
        data_dir: crate::config::data_dir().display().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_info_has_cores() {
        let info = system_info();
        assert!(info.cores >= 1);
        assert!(info.ram_gb > 1.0);
    }

    #[test]
    fn stats_is_safe_without_gpu() {
        // Con o sin `nvidia-smi` (AMD, Intel, máquina sin GPU) esto no revienta, y
        // ninguna tarjeta puede tener más memoria usada que total.
        let s = stats();
        for g in &s {
            assert!(g.memory_used_mb <= g.memory_total_mb, "{}: usado > total", g.name);
            assert!(g.memory_total_mb > 0, "{}: total 0", g.name);
        }
        assert!(s.len() <= 32);
    }

    #[test]
    fn las_tarjetas_traen_nombre_y_fabricante() {
        for g in gpus() {
            assert!(!g.name.is_empty());
            assert!(!g.fabricante.is_empty());
            assert!(g.memory_total_mb > 0);
        }
    }
}
