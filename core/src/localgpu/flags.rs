//! Argumentos de `llama-server`. Todo puro: se prueba sin binarios ni red.

use crate::localgpu::{LocalGpuCfg, Perfil};

/// llama-server escucha solo en loopback: KUKIA habla con él por 127.0.0.1.
pub const HOST: &str = "127.0.0.1";

/// Puerta del `rpc-server` cuando el destino no trae puerto.
pub const RPC_PUERTO: u16 = 50052;

/// Comprobaciones que no tocan el disco: sirven antes de intentar lanzar nada.
pub fn validar(cfg: &LocalGpuCfg, perfil: Perfil) -> Result<(), String> {
    if cfg.modelo.trim().is_empty() {
        return Err("falta el modelo GGUF (local_gpu.modelo)".into());
    }
    if cfg.ctx == 0 {
        return Err("el contexto no puede ser 0".into());
    }
    if cfg.puerto == 0 {
        return Err("el puerto no puede ser 0".into());
    }
    if perfil == Perfil::TresGpus && cfg.rpc_host.trim().is_empty() {
        return Err("el perfil de 3 GPUs necesita local_gpu.rpc_host (por ejemplo 192.168.50.2:50052)".into());
    }
    Ok(())
}

/// Reparto de capas. Se recorta o se rellena según el perfil y nunca queda a cero.
pub fn tensor_split(cfg: &LocalGpuCfg, perfil: Perfil) -> Vec<u32> {
    let mut v = cfg.tensor_split.clone();
    match perfil {
        Perfil::SoloI9 => {
            v.truncate(2);
            while v.len() < 2 {
                v.push(25);
            }
        }
        Perfil::TresGpus => {
            while v.len() < 3 {
                v.push(25);
            }
            v.truncate(3);
        }
    }
    if v.iter().all(|x| *x == 0) {
        v = match perfil {
            Perfil::SoloI9 => vec![50, 50],
            Perfil::TresGpus => vec![40, 40, 20],
        };
    }
    v
}

pub fn split_texto(v: &[u32]) -> String {
    v.iter()
        .map(|x| x.to_string())
        .collect::<Vec<String>>()
        .join(",")
}

/// Argumentos completos, en el orden en que los espera llama.cpp.
pub fn argumentos(cfg: &LocalGpuCfg, perfil: Perfil) -> Result<Vec<String>, String> {
    validar(cfg, perfil)?;
    let mut a: Vec<String> = Vec::new();
    a.push("-m".into());
    a.push(cfg.modelo.trim().to_string());
    a.push("-ngl".into());
    a.push(cfg.ngl.to_string());
    a.push("-c".into());
    a.push(cfg.ctx.to_string());
    a.push("--host".into());
    a.push(HOST.into());
    a.push("--port".into());
    a.push(cfg.puerto.to_string());
    a.push("--tensor-split".into());
    a.push(split_texto(&tensor_split(cfg, perfil)));
    if perfil == Perfil::TresGpus {
        a.push("--rpc".into());
        a.push(cfg.rpc_host.trim().to_string());
    }
    Ok(a)
}

/// Ejecutable y argumentos. Valida que la ruta del binario esté configurada.
pub fn comando(cfg: &LocalGpuCfg, perfil: Perfil) -> Result<(String, Vec<String>), String> {
    let exe = cfg.llama_server.trim().to_string();
    if exe.is_empty() {
        return Err("falta la ruta de llama-server.exe (local_gpu.llama_server)".into());
    }
    Ok((exe, argumentos(cfg, perfil)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg_base() -> LocalGpuCfg {
        let mut c = LocalGpuCfg::default();
        c.modelo = "C:/modelos/prueba.gguf".into();
        c
    }

    #[test]
    fn sin_modelo_no_hay_argumentos() {
        let c = LocalGpuCfg::default();
        assert!(argumentos(&c, Perfil::SoloI9).is_err());
    }

    #[test]
    fn contexto_o_puerto_cero_se_rechazan() {
        let mut c = cfg_base();
        c.ctx = 0;
        assert!(argumentos(&c, Perfil::SoloI9).is_err());
        c.ctx = 4096;
        c.puerto = 0;
        assert!(argumentos(&c, Perfil::SoloI9).is_err());
    }

    #[test]
    fn el_perfil_de_dos_gpus_no_lleva_rpc() {
        let a = argumentos(&cfg_base(), Perfil::SoloI9).unwrap();
        assert!(!a.iter().any(|x| x == "--rpc"), "solo_i9 no debe pedir RPC");
        let split = a.iter().position(|x| x == "--tensor-split").unwrap();
        assert_eq!(a[split + 1].split(',').count(), 2);
    }

    #[test]
    fn el_perfil_de_tres_gpus_lleva_rpc_y_tres_valores() {
        let mut c = cfg_base();
        c.rpc_host = "192.168.50.2:50052".into();
        let a = argumentos(&c, Perfil::TresGpus).unwrap();
        assert!(a.iter().any(|x| x == "--rpc"));
        let i = a.iter().position(|x| x == "--rpc").unwrap();
        assert_eq!(a[i + 1], "192.168.50.2:50052");
        let split = a.iter().position(|x| x == "--tensor-split").unwrap();
        assert_eq!(a[split + 1].split(',').count(), 3);
    }

    #[test]
    fn tres_gpus_sin_host_se_rechaza() {
        let mut c = cfg_base();
        c.rpc_host = "   ".into();
        assert!(crate::localgpu::flags::argumentos(&c, Perfil::TresGpus).is_err());
    }

    #[test]
    fn un_split_todo_cero_se_corrige() {
        let mut c = cfg_base();
        c.tensor_split = vec![0, 0, 0];
        assert_eq!(tensor_split(&c, Perfil::SoloI9), vec![50, 50]);
        assert_eq!(tensor_split(&c, Perfil::TresGpus), vec![40, 40, 20]);
    }

    #[test]
    fn sin_ruta_de_llama_server_no_se_puede_lanzar() {
        let c = cfg_base();
        assert!(comando(&c, Perfil::SoloI9).is_err());
    }

    #[test]
    fn los_argumentos_llevan_modelo_contexto_puerto_y_ngl() {
        let mut c = cfg_base();
        c.puerto = 8080;
        c.ctx = 8192;
        c.ngl = 99;
        let a = argumentos(&c, Perfil::SoloI9).unwrap();
        for esperado in ["-m", "-ngl", "99", "-c", "8192", "--port", "8080"] {
            assert!(a.iter().any(|x| x == esperado), "falta {esperado} en {a:?}");
        }
        assert!(a.iter().any(|x| x == HOST));
    }
}
