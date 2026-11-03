//! Comprobación manual del camino cloud real de KUKIA CORE.
//!
//! Uso:
//!   cargo run -p kukia-core --example probar_cloud -- opencode-go deepseek-v4.1-flash
//!   cargo run -p kukia-core --example probar_cloud -- opencode deepseek-v4.1-flash-free
//!
//! Imprime la respuesta o el error **tal cual** (incluido el código HTTP y el
//! tipo de error del servicio), sin claves ni cabeceras de autenticación.

use kukia_core::config::{load_config, CloudProviderCfg};
use kukia_core::providers::{self, ChatMessage, ChatRequest};

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let id = args.next().unwrap_or_else(|| "opencode-go".to_string());
    let modelo = args.next().unwrap_or_else(|| "deepseek-v4.1-flash".to_string());

    let mut cfg = load_config();
    cfg.cloud_provider = id.clone();
    cfg.merge_cloud_defaults();

    let activo: Option<&CloudProviderCfg> = cfg
        .cloud_providers
        .iter()
        .find(|p| p.id.eq_ignore_ascii_case(&id));
    match activo {
        Some(p) => println!(
            "proveedor: {} ({}) | base: {} | cabecera de sesión: {}",
            p.label, p.id, p.base_url, p.session_header
        ),
        None => println!("proveedor {id} no está en el catálogo"),
    }
    println!("modelo: {modelo}");
    println!(
        "clave disponible: {}",
        cfg.has_cloud_key || kukia_core::config::cloud_credential(&id).0.is_some()
    );

    let req = ChatRequest {
        provider: "cloud".into(),
        model: modelo.clone(),
        messages: vec![ChatMessage {
            role: "user".into(),
            content: "Responde solo: KUKIA OK".into(),
        }],
        temperature: Some(0.0),
        max_tokens: Some(16),
        system_prompt: None,
    
        tools: false,
        aprobar: false,
        agent: None,
};

    match providers::chat(&req, &cfg).await {
        Ok(r) => println!(
            "OK: {} | tokens {}/{} | {} ms",
            r.content.trim(),
            r.prompt_tokens,
            r.completion_tokens,
            r.total_ms
        ),
        Err(e) => println!("ERROR REAL: {e}"),
    }
}
