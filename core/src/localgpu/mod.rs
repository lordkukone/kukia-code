//! Motor local con llama.cpp: modelos GGUF repartidos entre las dos GPUs del i9
//! (perfil `solo_i9`) o esas dos mas la del i7 por RPC (perfil `tres_gpus`).
//!
//! KUKIA no toca VRAM a mano: lanza `llama-server` y habla con el por HTTP
//! (compatible OpenAI). Si el i7 no responde, se usa solo el i9.

pub mod flags;
pub mod proceso;
pub mod rpc;

use serde::{Deserialize, Serialize};

pub use proceso::EstadoProceso;
pub use rpc::RpcSonda;

/// Perfil de GPUs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Perfil {
    SoloI9,
    TresGpus,
}

impl Perfil {
    /// Acepta lo que el usuario escriba en la configuracion sin romperse.
    pub fn desde(texto: &str) -> Self {
        match texto.trim().to_lowercase().as_str() {
            "tres_gpus" | "tres" | "3gpu" | "3-gpu" | "lan" | "tres gpus" => Perfil::TresGpus,
            _ => Perfil::SoloI9,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Perfil::SoloI9 => "solo_i9",
            Perfil::TresGpus => "tres_gpus",
        }
    }

    pub fn gpus(self) -> usize {
        match self {
            Perfil::SoloI9 => 2,
            Perfil::TresGpus => 3,
        }
    }

    pub fn etiqueta(self) -> &'static str {
        match self {
            Perfil::SoloI9 => "2 GPUs (i9)",
            Perfil::TresGpus => "3 GPUs (i9 + i7 por RPC)",
        }
    }
}

/// Configuracion del motor local. **Sin secretos**: rutas, puertos y un numero
/// de GB declarado a mano para la GPU del i7 (hasta que haya agente o WinRM).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LocalGpuCfg {
    pub llama_server: String,
    pub rpc_server: String,
    pub modelo: String,
    /// `solo_i9` o `tres_gpus`.
    pub perfil: String,
    pub puerto: u16,
    pub ctx: u32,
    pub ngl: u32,
    /// Proporciones por GPU; para 2 GPUs se usan los dos primeros valores.
    pub tensor_split: Vec<u32>,
    pub rpc_host: String,
    pub vram_i7_gb: f64,
    pub rutas_gguf: Vec<String>,
    pub timeout_rpc_s: u64,
    pub auto_arranque: bool,
}

impl Default for LocalGpuCfg {
    fn default() -> Self {
        Self {
            llama_server: String::new(),
            rpc_server: String::new(),
            modelo: String::new(),
            perfil: "solo_i9".into(),
            puerto: 8080,
            ctx: 8192,
            ngl: 0, // 0 = que llama.cpp reparta las capas (necesario para contexto grande)
            tensor_split: vec![35, 40, 25],
            rpc_host: "192.168.50.2:50052".into(),
            vram_i7_gb: 8.0,
            rutas_gguf: Vec::new(),
            timeout_rpc_s: 10,
            auto_arranque: false,
        }
    }
}

impl LocalGpuCfg {
    pub fn perfil(&self) -> Perfil {
        Perfil::desde(&self.perfil)
    }
}

/// Perfil realmente usable: si se pidio `tres_gpus` y el RPC no responde, se
/// baja a `solo_i9` con el motivo. Nunca se queda esperando al i7.
pub fn perfil_efectivo(pedido: Perfil, rpc_ok: bool) -> (Perfil, String) {
    match (pedido, rpc_ok) {
        (Perfil::TresGpus, true) => (
            Perfil::TresGpus,
            "el rpc-server del i7 responde: se usan 3 GPUs".into(),
        ),
        (Perfil::TresGpus, false) => (
            Perfil::SoloI9,
            "el rpc-server del i7 no responde: se cae a las 2 GPUs del i9".into(),
        ),
        (Perfil::SoloI9, _) => (Perfil::SoloI9, "perfil de 2 GPUs del i9".into()),
    }
}

/// VRAM con la que cuenta el perfil, en GB. `locales` son las libres de cada
/// GPU del i9, inyectables para poder probarlo sin tarjetas.
pub fn presupuesto_gb(locales: &[f64], i7_gb: f64, perfil: Perfil) -> f64 {
    let total: f64 = locales.iter().copied().filter(|g| *g > 0.0).sum();
    if perfil == Perfil::TresGpus {
        total + i7_gb.max(0.0)
    } else {
        total
    }
}

/// Configuracion guardada del motor local.
pub fn cfg() -> LocalGpuCfg {
    crate::config::load_config().local_gpu
}

/// GGUF elegibles, con lo justo para que la interfaz los enseñe.
#[derive(Debug, Clone, Serialize)]
pub struct GgufDisponible {
    pub path: String,
    pub nombre: String,
    pub size_gb: f64,
    pub cuantizacion: Option<String>,
    pub arquitectura: Option<String>,
    pub valido: bool,
}

/// GGUF de las rutas de siempre mas las que anada el usuario.
pub fn ggufs(extra: &[String], max: usize) -> Vec<GgufDisponible> {
    let mut raices = crate::hub::raices_gguf();
    for r in extra {
        let p = std::path::PathBuf::from(r.trim());
        if !r.trim().is_empty() && p.exists() && !raices.contains(&p) {
            raices.push(p);
        }
    }
    crate::hub::gguf_locales(&raices, max)
        .into_iter()
        .map(|g| GgufDisponible {
            path: g.path,
            nombre: g.nombre,
            size_gb: g.size_gb,
            cuantizacion: g.cuantizacion,
            arquitectura: g.arquitectura,
            valido: g.valido,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_perfil_por_defecto_es_el_i9_solo() {
        assert_eq!(LocalGpuCfg::default().perfil(), Perfil::SoloI9);
        assert_eq!(LocalGpuCfg::default().puerto, 8080);
    }

    #[test]
    fn un_texto_raro_cae_en_solo_i9_sin_romper() {
        assert_eq!(Perfil::desde(""), Perfil::SoloI9);
        assert_eq!(Perfil::desde("lo que sea"), Perfil::SoloI9);
        assert_eq!(Perfil::desde("TRES_GPUS"), Perfil::TresGpus);
        assert_eq!(Perfil::desde(" 3gpu "), Perfil::TresGpus);
    }

    #[test]
    fn sin_rpc_se_baja_a_dos_gpus_con_motivo() {
        let (p, motivo) = perfil_efectivo(Perfil::TresGpus, false);
        assert_eq!(p, Perfil::SoloI9);
        assert!(motivo.contains("2 GPUs"), "motivo: {motivo}");
    }

    #[test]
    fn con_rpc_se_quedan_las_tres() {
        let (p, _) = perfil_efectivo(Perfil::TresGpus, true);
        assert_eq!(p, Perfil::TresGpus);
        assert_eq!(p.gpus(), 3);
    }

    #[test]
    fn el_presupuesto_suma_la_gpu_del_i7_solo_si_hay_tres() {
        let locales = [11.0, 15.0];
        assert!((presupuesto_gb(&locales, 8.0, Perfil::SoloI9) - 26.0).abs() < 0.01);
        assert!((presupuesto_gb(&locales, 8.0, Perfil::TresGpus) - 34.0).abs() < 0.01);
    }

    #[test]
    fn una_config_vacia_del_motor_local_toma_los_valores_por_defecto() {
        let c: LocalGpuCfg = serde_json::from_str("{}").unwrap();
        assert_eq!(c.perfil, "solo_i9");
        assert_eq!(c.ctx, 8192);
        assert_eq!(c.timeout_rpc_s, 10);
    }

    #[test]
    fn el_perfil_se_guarda_en_snake_case() {
        let raw = serde_json::to_string(&Perfil::TresGpus).unwrap();
        assert_eq!(raw, "\"tres_gpus\"");
    }
}
