//! La prueba que faltaba: que la MISMA función que usa la interfaz para el chat
//! (`providers::chat_stream`) emita texto con un modelo que razona.
//!
//! Necesita Ollama local con `qwen3-vl:4b`, así que se ejecuta a mano:
//!
//!   cargo test -p kukia-core --test validacion_chat_responde -- --ignored --nocapture

use kukia_core::config::AppConfig;
use kukia_core::providers::{chat_stream, ChatEvent, ChatMessage, ChatRequest};
use std::time::Instant;

#[tokio::test]
#[ignore = "necesita Ollama local con qwen3-vl:4b"]
async fn un_modelo_que_razona_acaba_respondiendo() {
    let cfg = AppConfig::default();
    let req = ChatRequest {
        provider: "ollama".into(),
        model: "qwen3-vl:4b".into(),
        messages: vec![ChatMessage {
            role: "user".into(),
            content: "Responde solo con una palabra: ok".into(),
        }],
        temperature: Some(0.2),
        // Sin límite explícito: así se ejerce también el reintento automático
        // cuando el modelo se gasta el presupuesto pensando, que era justo el
        // caso que dejaba la burbuja vacía en la interfaz.
        max_tokens: None,
        system_prompt: None,
        // Esta prueba mide la respuesta; el bucle de herramientas tiene la suya.
        tools: false,
        aprobar: false,
        agent: None,
    };

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<ChatEvent>();
    let inicio = Instant::now();
    let hilo = tokio::spawn({
        let req = req.clone();
        let cfg = cfg.clone();
        async move { chat_stream(&req, &cfg, tx).await }
    });

    let mut texto = String::new();
    let mut pensamiento = String::new();
    let mut hubo_done = false;
    let mut error = None;
    while let Some(ev) = rx.recv().await {
        match ev {
            ChatEvent::Token { text } => texto.push_str(&text),
            ChatEvent::Reasoning { text } => pensamiento.push_str(&text),
            ChatEvent::Done {
                completion_tokens,
                total_ms,
                ..
            } => {
                hubo_done = true;
                println!("DONE: {completion_tokens} tokens de salida en {total_ms} ms");
            }
            ChatEvent::ToolCall { .. } | ChatEvent::ToolResult { .. } => {}
            ChatEvent::Error { message } => error = Some(message),
        }
    }
    let res = hilo.await.expect("la tarea no debe entrar en pánico");

    println!(
        "--- respuesta ({} caracteres) ---\n{}",
        texto.len(),
        texto.trim()
    );
    if !pensamiento.trim().is_empty() {
        println!(
            "--- pensamiento / avisos ({} caracteres) ---\n{}",
            pensamiento.len(),
            pensamiento.trim()
        );
    }
    println!("--- total: {} ms ---", inicio.elapsed().as_millis());

    assert!(res.is_ok(), "chat_stream devolvió error: {res:?}");
    assert!(
        error.is_none(),
        "la interfaz habría mostrado un error: {error:?}"
    );
    assert!(
        !texto.trim().is_empty(),
        "burbuja vacía: el modelo no escribió nada"
    );
    assert!(
        hubo_done,
        "no llegó el evento Done: la interfaz no cerraría el mensaje"
    );
}
