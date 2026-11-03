//! Prueba de integración de la dirección A del puente: KUKIA pide una respuesta
//! al servidor real de OpenCode.
//!
//! Necesita un `opencode serve` vivo. Se le pasa por entorno para no depender de
//! un puerto fijo:
//!
//! ```text
//! KUKIA_TEST_OPENCODE_URL=http://127.0.0.1:4600 \
//! KUKIA_TEST_OPENCODE_MODEL=qwen3-vl:4b \
//! cargo test --test puente_opencode -- --nocapture
//! ```
//!
//! Sin esa variable la prueba lo dice y no finge nada.

use kukia_core::bridge;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn kukia_pregunta_a_opencode() {
    let Ok(url) = std::env::var("KUKIA_TEST_OPENCODE_URL") else {
        println!("sin KUKIA_TEST_OPENCODE_URL: prueba omitida (no se inventa resultado)");
        return;
    };
    let modelo = std::env::var("KUKIA_TEST_OPENCODE_MODEL")
        .unwrap_or_else(|_| "qwen3-vl:4b".to_string());

    let estado = bridge::estado(Some(url.clone())).await;
    println!(
        "opencode {} · instalado={} · modelos_cli={} · agentes_cli={} · servidor vivo={}",
        estado.version,
        estado.opencode_instalado,
        estado.modelos_cli,
        estado.agentes_cli,
        estado.servidor.vivo
    );
    assert!(estado.opencode_instalado, "opencode debe estar instalado");
    assert!(estado.servidor.vivo, "el servidor de prueba debe responder");
    assert!(
        estado.config_ok,
        "la configuración debe leerse: {}",
        estado.config_nota
    );

    let modelos = bridge::modelos(Some(&url)).await;
    println!("modelos por API: {}", modelos.len());
    assert!(!modelos.is_empty(), "el servidor debe listar modelos");

    let agentes = bridge::agentes(Some(&url)).await;
    println!("agentes por API: {}", agentes.len());

    let r = bridge::preguntar(&url, "ollama", &modelo, None, "Responde solo: PUENTE OK", 240)
        .await
        .expect("OpenCode debe contestar");
    println!("sesión {} · {} ms · {} tokens", r.session_id, r.ms, r.tokens_out);
    println!("texto: {:?}", r.texto);
    assert!(r.ok);
    assert!(r.session_id.starts_with("ses"), "id de sesión real: {}", r.session_id);
    assert!(!r.texto.trim().is_empty(), "la respuesta no puede venir vacía");
}
