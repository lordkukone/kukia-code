//! KUKIA CODE — núcleo
//!
//! Módulos independientes de la interfaz: proveedores, memoria, GPU,
//! agentes, seguridad, logs y radar de actualizaciones.

pub mod agents;
pub mod anime_real;
pub mod config;
pub mod gaming;
pub mod gpu;
pub mod logs;
pub mod memory;
pub mod permissions;
pub mod providers;
pub mod radar;
pub mod research;
pub mod security;
pub mod vram;

pub use anime_real::{Capability, ComfyStatus, ConversionResult, ConvertRequest};
pub use config::AppConfig;
pub use memory::Database;
pub use providers::{ChatEvent, ChatRequest, ChatResponse, ModelInfo, ProviderInfo};
