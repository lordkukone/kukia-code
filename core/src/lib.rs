//! KUKIA CODE — núcleo
//!
//! Módulos independientes de la interfaz: proveedores, memoria, GPU,
//! agentes, seguridad, logs y radar de actualizaciones.

pub mod agents;
pub mod config;
pub mod gpu;
pub mod logs;
pub mod memory;
pub mod providers;
pub mod radar;
pub mod research;
pub mod security;

pub use config::AppConfig;
pub use memory::Database;
pub use providers::{ChatEvent, ChatRequest, ChatResponse, ModelInfo, ProviderInfo};
