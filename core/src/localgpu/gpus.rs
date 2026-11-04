//! Deteccion de las GPUs del equipo y reparto del modelo entre todas ellas.
//!
//! `nvidia-smi` es local: las GPUs del otro PC se cuentan como un dispositivo
//! mas por cada `rpc-server` que responda, con la VRAM que se declare en la
//! configuracion (`rpc_vram_mb`).

use serde::Serialize;

/// Una GPU vista por `nvidia-smi`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct GpuLocal {
    pub indice: u32,
    pub nombre: String,
    pub total_mb: u64,
    pub libre_mb: u64,
}

/// Interpreta la salida `index, name, memory.total, memory.free` (sin cabecera).
/// Las lineas que no encajan se ignoran: nunca se inventan tarjetas.
pub fn parsear(salida: &str) -> Vec<GpuLocal> {
    let mut v = Vec::new();
    for linea in salida.lines() {
        let campos: Vec<&str> = linea.split(',').map(|c| c.trim()).collect();
        if campos.len() < 4 {
            continue;
        }
        let (Ok(indice), Ok(total), Ok(libre)) = (
            campos[0].parse::<u32>(),
            campos[2].parse::<u64>(),
            campos[3].parse::<u64>(),
        ) else {
            continue;
        };
        v.push(GpuLocal {
            indice,
            nombre: campos[1].to_string(),
            total_mb: total,
            libre_mb: libre,
        });
    }
    v
}

/// Pregunta al sistema por las GPUs NVIDIA. Error claro si no hay `nvidia-smi`.
pub fn detectar() -> Result<Vec<GpuLocal>, String> {
    let salida = std::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=index,name,memory.total,memory.free",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .map_err(|e| format!("no se pudo ejecutar nvidia-smi: {e}"))?;
    if !salida.status.success() {
        return Err("nvidia-smi devolvio un error".into());
    }
    let texto = String::from_utf8_lossy(&salida.stdout).to_string();
    let gpus = parsear(&texto);
    if gpus.is_empty() {
        return Err("nvidia-smi no listo ninguna GPU".into());
    }
    Ok(gpus)
}

/// Reparto proporcional a la VRAM util de cada dispositivo, en porcentajes que
/// suman 100 y nunca bajan de 1. `reserva_mb` es el margen que se deja libre en
/// cada tarjeta (contexto de pantalla, otros procesos).
pub fn reparto(dispositivos: &[(String, u64)], reserva_mb: u64) -> Vec<u32> {
    if dispositivos.is_empty() {
        return Vec::new();
    }
    if dispositivos.len() == 1 {
        return vec![100];
    }
    let pesos: Vec<f64> = dispositivos
        .iter()
        .map(|(_, vram)| (vram.saturating_sub(reserva_mb)).max(512) as f64)
        .collect();
    let total: f64 = pesos.iter().sum();
    let mut pct: Vec<u32> = pesos
        .iter()
        .map(|p| (((p / total) * 100.0).round() as u32).max(1))
        .collect();
    // Ajuste fino para que sume exactamente 100: se corrige el mayor.
    let suma: i64 = pct.iter().map(|x| *x as i64).sum();
    if suma != 100 {
        let (i_mayor, _) = pct
            .iter()
            .enumerate()
            .max_by_key(|(_, x)| **x)
            .unwrap_or((0, &0));
        let corregido = pct[i_mayor] as i64 + (100 - suma);
        pct[i_mayor] = corregido.clamp(1, 100) as u32;
    }
    pct
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lee_dos_gpus_y_ignora_basura() {
        let salida = "0, NVIDIA GeForce RTX 5070, 12227, 9901\n\
                      1, NVIDIA GeForce RTX 5060 Ti, 16311, 16017\n\
                      algo que no es una gpu\n";
        let v = parsear(salida);
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].indice, 0);
        assert_eq!(v[1].total_mb, 16311);
        assert_eq!(v[1].libre_mb, 16017);
    }

    #[test]
    fn reparto_proporcional_a_la_vram() {
        let d = vec![
            ("5070".to_string(), 12227u64),
            ("5060 Ti".to_string(), 16311),
            ("2070 SUPER".to_string(), 8192),
        ];
        let r = reparto(&d, 700);
        assert_eq!(r.iter().sum::<u32>(), 100);
        // 11.527 / 15.611 / 7.492 MB utiles de 34.630 -> 33 / 45 / 22.
        assert_eq!(r, vec![33, 45, 22]);
    }

    #[test]
    fn cinco_dispositivos_suman_cien_y_ninguno_a_cero() {
        let d: Vec<(String, u64)> = (0..5).map(|i| (format!("g{i}"), 8192)).collect();
        let r = reparto(&d, 700);
        assert_eq!(r.len(), 5);
        assert_eq!(r.iter().sum::<u32>(), 100);
        assert!(r.iter().all(|x| *x >= 1));
    }

    #[test]
    fn una_sola_gpu_se_queda_todo() {
        assert_eq!(reparto(&[("unica".to_string(), 24576)], 700), vec![100]);
    }

    #[test]
    fn reserva_mayor_que_vram_no_revienta() {
        let d = vec![("a".to_string(), 4096u64), ("b".to_string(), 8192)];
        let r = reparto(&d, 99999);
        assert_eq!(r.iter().sum::<u32>(), 100);
        assert_eq!(r.len(), 2);
    }
}

/// Separa una entrada de `rpc_hosts` en destino y VRAM: `host`, `host:puerto` o
/// `host:puerto=MB`. Sin `=MB` se usa la VRAM por defecto de la configuracion.
/// Asi, al cambiar una tarjeta por otra mas grande, basta con anotar su tamano.
pub fn parsear_equipo(entrada: &str, vram_defecto: u64) -> (String, u64) {
    let t = entrada.trim();
    if let Some((host, mb)) = t.rsplit_once('=') {
        if let Ok(v) = mb.trim().parse::<u64>() {
            if v > 0 && !host.trim().is_empty() {
                return (host.trim().to_string(), v);
            }
        }
    }
    (t.to_string(), vram_defecto)
}

#[cfg(test)]
mod tests_equipos {
    use super::*;

    #[test]
    fn entrada_con_vram_propia() {
        let (h, mb) = parsear_equipo(" 192.168.50.2:50052=22528 ", 8192);
        assert_eq!(h, "192.168.50.2:50052");
        assert_eq!(mb, 22528);
    }

    #[test]
    fn entrada_sin_vram_usa_la_de_la_config() {
        assert_eq!(parsear_equipo("192.168.50.2", 8192), ("192.168.50.2".into(), 8192));
        assert_eq!(parsear_equipo("192.168.50.3:50052", 12288).1, 12288);
    }

    #[test]
    fn una_vram_absurda_no_rompe_la_entrada() {
        assert_eq!(parsear_equipo("host=0", 8192), ("host=0".into(), 8192));
        assert_eq!(parsear_equipo("=22528", 8192), ("=22528".into(), 8192));
    }

    #[test]
    fn reparto_con_una_tarjeta_de_22_gb() {
        // VRAM total declarada; la reserva de 700 MB la descuenta `reparto`.
        let d = vec![
            ("5070".to_string(), 12227u64),
            ("5060 Ti".to_string(), 16311),
            ("futura 22 GB".to_string(), 22528),
        ];
        let r = reparto(&d, 700);
        assert_eq!(r.iter().sum::<u32>(), 100);
        assert_eq!(r, vec![24, 32, 44]);
    }
}
