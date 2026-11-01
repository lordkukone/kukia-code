use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuración persistente de KUKIA CODE.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub default_provider: String,
    pub default_model: String,
    pub temperature: f32,
    pub gpu_split: [u32; 2],
    pub theme: String,
    pub cloud_base_url: String,
    pub cloud_model: String,
    #[serde(default)]
    pub has_cloud_key: bool,
    #[serde(default = "default_true")]
    pub research_auto: bool,
    /// Permitir que los modelos con razonamiento (DeepSeek-R1, JEV, QwQ…)
    /// emitan su cadena de pensamiento. Si se desactiva, se pide al motor que
    /// no razone y solo se muestra la respuesta final.
    #[serde(default = "default_true")]
    pub reasoning: bool,
}

fn default_true() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            default_provider: "ollama".into(),
            default_model: String::new(),
            temperature: 0.7,
            gpu_split: [35, 65],
            theme: "dark".into(),
            cloud_base_url: "https://api.openai.com/v1".into(),
            cloud_model: String::new(),
            has_cloud_key: false,
            research_auto: true,
            reasoning: true,
        }
    }
}

impl AppConfig {
    pub fn refresh_flags(&mut self) {
        self.has_cloud_key = std::env::var("KUKIA_CLOUD_KEY")
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false);
    }
}

/// Directorio de datos por usuario (compatible con instalación).
pub fn data_dir() -> PathBuf {
    if let Ok(custom) = std::env::var("KUKIA_DATA_DIR") {
        let p = PathBuf::from(custom);
        let _ = std::fs::create_dir_all(&p);
        return p;
    }
    let base = std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());
    let p = base.join("KukiaCode");
    let _ = std::fs::create_dir_all(&p);
    p
}

pub fn ensure_dirs() -> PathBuf {
    let dir = data_dir();
    for sub in ["config", "logs", "benchmarks", "backups"] {
        let _ = std::fs::create_dir_all(dir.join(sub));
    }
    dir
}

fn config_path() -> PathBuf {
    data_dir().join("config").join("config.json")
}

pub fn load_config() -> AppConfig {
    let path = config_path();
    let mut cfg = match std::fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str::<AppConfig>(&raw).unwrap_or_default(),
        Err(_) => AppConfig::default(),
    };
    cfg.refresh_flags();
    cfg
}

pub fn save_config(cfg: &AppConfig) -> Result<AppConfig, String> {
    let dir = data_dir().join("config");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let mut out = cfg.clone();
    out.refresh_flags();
    let raw = serde_json::to_string_pretty(&out).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("config.json"), raw).map_err(|e| e.to_string())?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_split_is_35_65() {
        let c = AppConfig::default();
        assert_eq!(c.gpu_split, [35, 65]);
    }

    #[test]
    fn default_provider_is_ollama() {
        assert_eq!(AppConfig::default().default_provider, "ollama");
    }

    #[test]
    fn config_roundtrip_json() {
        let c = AppConfig::default();
        let raw = serde_json::to_string(&c).unwrap();
        let back: AppConfig = serde_json::from_str(&raw).unwrap();
        assert_eq!(back.temperature, c.temperature);
    }
}
