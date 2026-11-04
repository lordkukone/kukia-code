//! Chat real con herramientas por el proveedor `llamacpp` (llama-server).
//! Uso: cargo run --example llamacpp_tools -- "pregunta"
use kukia_core::providers::{chat_stream_con, ChatEvent, ChatRequest};

#[tokio::main]
async fn main() {
    let pregunta = std::env::args().nth(1).unwrap_or_else(|| {
        "Usa la herramienta files_list sobre la carpeta de datos y dime cuantos archivos hay.".into()
    });
    let mut cfg = kukia_core::config::load_config();
    // Puerto opcional por argumento: permite probar contra un servidor suelto.
    if let Some(p) = std::env::args().nth(2).and_then(|v| v.parse().ok()) {
        cfg.local_gpu.puerto = p;
    }
    let req: ChatRequest = serde_json::from_value(serde_json::json!({
        "provider": "llamacpp",
        "model": "flint",
        "messages": [{ "role": "user", "content": pregunta }],
        "max_tokens": 900,
        "tools": true,
        "aprobar": false
    }))
    .expect("peticion");
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let h = tokio::spawn(async move { chat_stream_con(&req, &cfg, None, tx).await });
    let (mut texto, mut tools) = (String::new(), 0usize);
    while let Some(ev) = rx.recv().await {
        match ev {
            ChatEvent::Token { text } => texto.push_str(&text),
            ChatEvent::Reasoning { .. } => {}
            ChatEvent::ToolCall { nombre, argumentos } => {
                tools += 1;
                println!("\n[llamada] {nombre} {argumentos}");
            }
            ChatEvent::ToolResult { nombre, ok, texto } => println!(
                "[resultado] {nombre} ok={ok} -> {}",
                texto.chars().take(160).collect::<String>()
            ),
            ChatEvent::Done { completion_tokens, total_ms, .. } => {
                println!("\n[fin] tokens={completion_tokens} ms={total_ms}");
                break;
            }
            ChatEvent::Error { message } => {
                println!("\n[error] {message}");
                break;
            }
        }
    }
    let _ = h.await;
    println!("\n=== herramientas usadas: {tools} ===");
    println!("{}", texto.chars().take(700).collect::<String>());
}
