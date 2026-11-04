use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Proveedor cloud preconfigurado. **La clave NUNCA se guarda aquí**: solo la
/// identificación y el endpoint. El valor se resuelve en tiempo de uso.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudProviderCfg {
    pub id: String,
    pub label: String,
    pub base_url: String,
    /// `true` si el endpoint exige la cabecera `x-opencode-session` en cada
    /// petición. Lo pide **OpenCode Go** (`https://opencode.ai/zen/go/v1`): sin
    /// ella responde `400 MissingSessionID`. Comprobado contra el servicio real.
    #[serde(default)]
    pub session_header: bool,
}

pub fn default_cloud_providers() -> Vec<CloudProviderCfg> {
    vec![
        CloudProviderCfg {
            id: "opencode".into(),
            label: "OpenCode Zen".into(),
            base_url: "https://opencode.ai/zen/v1".into(),
            session_header: false,
        },
        // Plan de suscripción: endpoint propio, no el de Zen. Los modelos Go
        // (`deepseek-v4.1-flash`, `glm-5.3`, `gpt-6-luna`…) solo existen aquí.
        CloudProviderCfg {
            id: "opencode-go".into(),
            label: "OpenCode Go".into(),
            base_url: "https://opencode.ai/zen/go/v1".into(),
            session_header: true,
        },
        CloudProviderCfg {
            id: "freebuff".into(),
            label: "Freebuff".into(),
            base_url: "https://freebuff.com/api/v1/paid".into(),
            session_header: false,
        },
        CloudProviderCfg {
            id: "openai".into(),
            label: "OpenAI".into(),
            base_url: "https://api.openai.com/v1".into(),
            session_header: false,
        },
        CloudProviderCfg {
            id: "custom".into(),
            label: "Personalizado".into(),
            base_url: String::new(),
            session_header: false,
        },
    ]
}

/// De dónde salió la clave. Nunca se expone el valor, solo el origen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeySource {
    Env,
    SecretsFile,
    OpenCode,
    None,
}

impl KeySource {
    pub fn label(self) -> &'static str {
        match self {
            KeySource::Env => "variable de entorno",
            KeySource::SecretsFile => "secrets.json",
            KeySource::OpenCode => "auth.json de OpenCode",
            KeySource::None => "sin clave",
        }
    }
}

/// `%USERPROFILE%\.local\share\opencode\auth.json` (credenciales de OpenCode).
/// Se lee en solo lectura: KUKIA nunca escribe ni copia ese archivo.
pub fn auth_json_path() -> Option<PathBuf> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .ok()?;
    Some(
        PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("opencode")
            .join("auth.json"),
    )
}

fn non_empty(v: Option<String>) -> Option<String> {
    v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

/// Resuelve la clave de un proveedor cloud, en este orden:
/// 1. `KUKIA_CLOUD_KEY_<ID>` (específica del proveedor)
/// 2. `KUKIA_CLOUD_KEY` (genérica, compatibilidad)
/// 3. `%APPDATA%\KukiaCode\secrets.json` → `{"opencode": "…"}`
/// 4. `auth.json` de OpenCode (solo lectura)
///
/// Devuelve `(clave, origen)`. La clave **no se registra en logs ni se envía a la UI**.
pub fn cloud_credential(id: &str) -> (Option<String>, KeySource) {
    let norm = id.trim().to_lowercase();
    let env_id = format!(
        "KUKIA_CLOUD_KEY_{}",
        norm.to_uppercase()
            .replace(|c: char| !c.is_ascii_alphanumeric(), "_")
    );
    if let Some(k) = non_empty(std::env::var(&env_id).ok()) {
        return (Some(k), KeySource::Env);
    }
    if let Some(k) = non_empty(std::env::var("KUKIA_CLOUD_KEY").ok()) {
        return (Some(k), KeySource::Env);
    }
    // Claves cifradas con DPAPI (o el JSON en claro antiguo, si aún no se migró).
    if let Some(k) = crate::secrets::buscar(&norm) {
        return (Some(k), KeySource::SecretsFile);
    }
    if let Some(path) = auth_json_path() {
        if let Ok(raw) = std::fs::read_to_string(&path) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
                // `opencode-go` comparte clave con `opencode`; se prueba el propio primero.
                let mut ids = vec![norm.clone()];
                if norm != "opencode" {
                    ids.push("opencode".to_string());
                }
                for key_id in ids {
                    if let Some(k) = v
                        .get(&key_id)
                        .and_then(|e| e.get("key"))
                        .and_then(|x| x.as_str())
                    {
                        let k = k.trim();
                        if !k.is_empty() {
                            return (Some(k.to_string()), KeySource::OpenCode);
                        }
                    }
                }
            }
        }
    }
    (None, KeySource::None)
}

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
    /// Proveedor cloud activo (`opencode`, `freebuff`, `openai`, `custom`).
    #[serde(default)]
    pub cloud_provider: String,
    /// Catálogo de proveedores cloud con su endpoint. Nunca contiene claves.
    #[serde(default = "default_cloud_providers")]
    pub cloud_providers: Vec<CloudProviderCfg>,
    #[serde(default)]
    pub has_cloud_key: bool,
    #[serde(default = "default_true")]
    pub research_auto: bool,
    /// Permitir que los modelos con razonamiento (DeepSeek-R1, JEV, QwQ…)
    /// emitan su cadena de pensamiento. Si se desactiva, se pide al motor que
    /// no razone y solo se muestra la respuesta final.
    #[serde(default = "default_true")]
    pub reasoning: bool,
    /// Proyecto activo: cuando tiene valor, sus rutas acotan la política del agente.
    #[serde(default)]
    pub active_project: Option<String>,
    /// Motor local llama.cpp: 2 GPUs del i9 o 3 con el i7 por RPC.
    #[serde(default)]
    pub local_gpu: crate::localgpu::LocalGpuCfg,
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
            cloud_provider: "opencode".into(),
            cloud_providers: default_cloud_providers(),
            has_cloud_key: false,
            research_auto: true,
            reasoning: true,
            active_project: None,
            local_gpu: crate::localgpu::LocalGpuCfg::default(),
        }
    }
}

impl AppConfig {
    /// Proveedor cloud activo, resuelto contra el catálogo. Si el activo no está
    /// en el catálogo se usa `cloud_base_url` (compatibilidad con configs viejas).
    pub fn active_cloud(&self) -> CloudProviderCfg {
        let id = if self.cloud_provider.trim().is_empty() {
            "opencode".to_string()
        } else {
            self.cloud_provider.trim().to_lowercase()
        };
        if let Some(c) = self
            .cloud_providers
            .iter()
            .find(|c| c.id.eq_ignore_ascii_case(&id))
        {
            if c.id == "custom" && !self.cloud_base_url.trim().is_empty() {
                return CloudProviderCfg {
                    id: c.id.clone(),
                    label: c.label.clone(),
                    base_url: self.cloud_base_url.clone(),
                    session_header: c.session_header,
                };
            }
            if !c.base_url.trim().is_empty() {
                return c.clone();
            }
        }
        CloudProviderCfg {
            id,
            label: "Cloud".into(),
            base_url: self.cloud_base_url.clone(),
            session_header: false,
        }
    }

    /// Añade al catálogo guardado los proveedores que trae esta versión y aún no
    /// estaban (una config antigua no recibe los nuevos si no se fusiona).
    pub fn merge_cloud_defaults(&mut self) {
        for d in default_cloud_providers() {
            if !self
                .cloud_providers
                .iter()
                .any(|c| c.id.eq_ignore_ascii_case(&d.id))
            {
                self.cloud_providers.push(d);
            }
        }
    }

    /// `true` si el proveedor activo tiene clave disponible, venga de donde venga.
    pub fn refresh_flags(&mut self) {
        let activo = self.active_cloud();
        self.has_cloud_key = cloud_credential(&activo.id).0.is_some();
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
    cfg.merge_cloud_defaults();
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
    fn proyecto_activo_por_defecto_es_ninguno() {
        assert!(AppConfig::default().active_project.is_none());
    }

    #[test]
    fn config_roundtrip_json() {
        let c = AppConfig::default();
        let raw = serde_json::to_string(&c).unwrap();
        let back: AppConfig = serde_json::from_str(&raw).unwrap();
        assert_eq!(back.temperature, c.temperature);
    }

    #[test]
    fn el_catalogo_cloud_trae_los_proveedores_conocidos() {
        let c = AppConfig::default();
        let ids: Vec<&str> = c.cloud_providers.iter().map(|p| p.id.as_str()).collect();
        for esperado in ["opencode", "opencode-go", "freebuff", "openai", "custom"] {
            assert!(ids.contains(&esperado), "falta {esperado} en el catálogo");
        }
    }

    #[test]
    fn opencode_go_tiene_su_endpoint_y_pide_cabecera_de_sesion() {
        let c = AppConfig::default();
        let go = c
            .cloud_providers
            .iter()
            .find(|p| p.id == "opencode-go")
            .expect("falta opencode-go");
        assert_eq!(go.base_url, "https://opencode.ai/zen/go/v1");
        assert!(go.session_header, "Go exige x-opencode-session");
        let zen = c.cloud_providers.iter().find(|p| p.id == "opencode").unwrap();
        assert!(!zen.session_header, "Zen no usa cabecera de sesión");
    }

    #[test]
    fn una_config_vieja_recibe_los_proveedores_nuevos() {
        let mut c = AppConfig::default();
        c.cloud_providers.retain(|p| p.id != "opencode-go");
        c.merge_cloud_defaults();
        assert!(c.cloud_providers.iter().any(|p| p.id == "opencode-go"));
        // No duplica lo que ya estaba.
        let n = c.cloud_providers.iter().filter(|p| p.id == "opencode").count();
        assert_eq!(n, 1);
    }

    #[test]
    fn el_proveedor_activo_por_defecto_es_opencode() {
        let c = AppConfig::default();
        assert_eq!(c.active_cloud().id, "opencode");
        assert_eq!(c.active_cloud().base_url, "https://opencode.ai/zen/v1");
    }

    #[test]
    fn proveedor_desconocido_cae_en_cloud_base_url() {
        let mut c = AppConfig::default();
        c.cloud_provider = "inexistente".into();
        c.cloud_base_url = "https://ejemplo.test/v1".into();
        let activo = c.active_cloud();
        assert_eq!(activo.id, "inexistente");
        assert_eq!(activo.base_url, "https://ejemplo.test/v1");
    }

    #[test]
    fn la_config_no_guarda_ninguna_clave() {
        let raw = serde_json::to_string(&AppConfig::default()).unwrap();
        assert!(!raw.contains("api_key") && !raw.contains("\"key\""), "la config no debe tener claves");
    }
}
