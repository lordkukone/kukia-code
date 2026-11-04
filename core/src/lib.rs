//! KUKIA CODE — núcleo
//!
//! Módulos independientes de la interfaz: proveedores, memoria, GPU,
//! agentes, seguridad, logs y radar de actualizaciones.

pub mod agents;
pub mod anime_real;
pub mod bridge;
pub mod buildinfo;
pub mod catalog;
pub mod config;
pub mod control;
pub mod ecosystem;
pub mod engines;
pub mod entrenador;
pub mod gaming;
pub mod gpu;
pub mod gpus;
pub mod hub;
pub mod jobs;
pub mod lab;
pub mod logs;
pub mod mcp;
pub mod memory;
pub mod permissions;
pub mod providers;
pub mod radar;
pub mod registry;
pub mod research;
pub mod secrets;
pub mod security;
pub mod serve;
pub mod studio;
pub mod tools;
pub mod vram;
pub mod xoxete;

pub use anime_real::{Capability, ComfyStatus, ConversionResult, ConvertRequest};
pub use config::AppConfig;
pub use engines::{EngineStatus, InferenceParams, ModelCard, Preflight};
pub use memory::Database;
pub use providers::{ChatEvent, ChatRequest, ChatResponse, ModelInfo, ProviderInfo};
