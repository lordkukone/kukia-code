use serde::Serialize;
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct GpuInfo {
    pub index: u32,
    pub name: String,
    pub memory_total_mb: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct GpuStat {
    pub index: u32,
    pub name: String,
    pub utilization: f64,
    pub memory_used_mb: u64,
    pub memory_total_mb: u64,
    pub temperature_c: f64,
    pub power_w: f64,
    pub clock_graphics_mhz: u64,
    pub clock_memory_mhz: u64,
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

#[cfg(windows)]
fn hidden(cmd: &mut Command) -> &mut Command {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(0x0800_0000) // CREATE_NO_WINDOW
}

#[cfg(not(windows))]
fn hidden(cmd: &mut Command) -> &mut Command {
    cmd
}

fn nvidia_smi(query: &str) -> Option<String> {
    let mut cmd = Command::new("nvidia-smi");
    cmd.args(["--query-gpu", query, "--format=csv,noheader,nounits"]);
    let out = hidden(&mut cmd).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).to_string())
}

pub fn gpus() -> Vec<GpuInfo> {
    let Some(raw) = nvidia_smi("index,name,memory.total") else {
        return Vec::new();
    };
    raw.lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            if parts.len() < 3 {
                return None;
            }
            Some(GpuInfo {
                index: parts[0].parse().ok()?,
                name: parts[1].to_string(),
                memory_total_mb: parts[2].parse().unwrap_or(0),
            })
        })
        .collect()
}

pub fn stats() -> Vec<GpuStat> {
    let query = "index,name,utilization.gpu,memory.used,memory.total,temperature.gpu,power.draw,clocks.gr,clocks.mem";
    let Some(raw) = nvidia_smi(query) else {
        return Vec::new();
    };
    raw.lines()
        .filter_map(|line| {
            let p: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            if p.len() < 9 {
                return None;
            }
            Some(GpuStat {
                index: p[0].parse().ok()?,
                name: p[1].to_string(),
                utilization: p[2].parse().unwrap_or(0.0),
                memory_used_mb: p[3].parse().unwrap_or(0),
                memory_total_mb: p[4].parse().unwrap_or(0),
                temperature_c: p[5].parse().unwrap_or(0.0),
                power_w: p[6].parse().unwrap_or(0.0),
                clock_graphics_mhz: p[7].parse().unwrap_or(0),
                clock_memory_mhz: p[8].parse().unwrap_or(0),
            })
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
    fn stats_parse_is_safe_without_gpu() {
        // No debe entrar en pánico aunque nvidia-smi no exista.
        let s = stats();
        assert!(s.len() <= 8);
    }
}
