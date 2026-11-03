//! El bucle de herramientas, contra un modelo real.
//!
//! Comprueba las cuatro piezas que antes no existían: que KUKIA ofrezca la
//! herramienta, que el modelo la pida, que se ejecute de verdad y que su
//! resultado vuelva al modelo hasta que conteste.
//!
//!   cargo test -p kukia-core --test validacion_herramientas_chat -- --ignored --nocapture

use kukia_core::config::AppConfig;
use kukia_core::providers::{chat_stream_con, ChatEvent, ChatMessage, ChatRequest};
use std::time::Instant;

#[tokio::test]
#[ignore = "necesita Ollama local con qwen3-vl:4b y su capacidad tools"]
async fn el_modelo_lee_un_archivo_y_lo_usa_para_responder() {
    let cfg = AppConfig::default();
    let ruta = std::env::temp_dir().join("kukia_bucle_herramientas.txt");
    std::fs::write(&ruta, "la clave secreta es 4711").unwrap();

    let req = ChatRequest {
        provider: "ollama".into(),
        model: "qwen3-vl:4b".into(),
        messages: vec![ChatMessage {
            role: "user".into(),
            content: format!(
                "Usa la herramienta files_read para leer el archivo {} y dime qué clave contiene.",
                ruta.display()
            ),
        }],
        temperature: Some(0.1),
        max_tokens: None,
        system_prompt: None,
        tools: true,
        aprobar: false,
        agent: Some("kukia".into()),
    };

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<ChatEvent>();
    let inicio = Instant::now();
    let hilo = tokio::spawn({
        let req = req.clone();
        let cfg = cfg.clone();
        async move { chat_stream_con(&req, &cfg, None, tx).await }
    });

    let mut texto = String::new();
    let mut llamadas: Vec<(String, String)> = Vec::new();
    let mut resultados: Vec<(String, bool, String)> = Vec::new();
    let mut hubo_done = false;
    let mut error = None;

    while let Some(ev) = rx.recv().await {
        match ev {
            ChatEvent::Token { text } => texto.push_str(&text),
            ChatEvent::ToolCall { nombre, argumentos } => llamadas.push((nombre, argumentos)),
            ChatEvent::ToolResult {
                nombre,
                ok,
                texto: t,
            } => resultados.push((nombre, ok, t)),
            ChatEvent::Done {
                completion_tokens, ..
            } => {
                hubo_done = true;
                println!("DONE · {completion_tokens} tokens");
            }
            ChatEvent::Error { message } => error = Some(message),
            ChatEvent::Reasoning { .. } => {}
        }
    }
    let res = hilo.await.expect("sin pánico");

    println!("--- herramientas pedidas ---");
    for (n, a) in &llamadas {
        println!("  {n}({a})");
    }
    println!("--- resultados ---");
    for (n, ok, t) in &resultados {
        println!("  {n} ok={ok} → {}", t.trim().replace('\n', " "));
    }
    println!("--- respuesta ---");
    println!("{}", texto.trim());
    println!("--- {} ms ---", inicio.elapsed().as_millis());

    let _ = std::fs::remove_file(&ruta);

    assert!(res.is_ok(), "chat_stream_con falló: {res:?}");
    assert!(error.is_none(), "la interfaz habría mostrado un error: {error:?}");
    assert!(
        llamadas.iter().any(|(n, _)| n == "files_read"),
        "el modelo no pidió leer el archivo; llamadas: {llamadas:?}"
    );
    assert!(
        resultados.iter().any(|(n, ok, t)| n == "files_read"
            && *ok
            && t.contains("4711")),
        "la herramienta no devolvió el contenido del archivo: {resultados:?}"
    );
    assert!(
        texto.contains("4711"),
        "el modelo no usó el resultado para responder: {texto:?}"
    );
    assert!(hubo_done, "no llegó Done: la interfaz no cerraría el mensaje");
}
