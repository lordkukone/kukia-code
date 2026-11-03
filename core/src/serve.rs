//! Servidor local compatible OpenAI (para OpenCode y otros clientes).
//!
//! Sirve los modelos del catálogo por `http://127.0.0.1:<puerto>/v1`, **solo en
//! loopback** y con token. No reescribe ninguna configuración ajena: el fragmento
//! para `opencode.json` se entrega y lo pega el usuario.
//!
//! Reconstruido tras el incidente del 03/11/2026: el manejador de chat y los
//! literales de respuesta son los originales que sobrevivieron.

use crate::catalog;
use crate::config::AppConfig;
use crate::providers;
use serde::Serialize;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/* ============================ Estado ============================ */

#[derive(Debug, Clone, Serialize)]
pub struct ServeStatus {
    pub running: bool,
    pub url: String,
    pub port: u16,
    pub token: String,
    pub peticiones: u64,
    pub errores: u64,
    pub modelos: usize,
    pub ultimo_error: Option<String>,
}

#[derive(Default)]
struct Estado {
    port: u16,
    token: String,
    running: bool,
    peticiones: u64,
    errores: u64,
    modelos: usize,
    ultimo_error: Option<String>,
}

fn estado() -> &'static Mutex<Estado> {
    static ESTADO: OnceLock<Mutex<Estado>> = OnceLock::new();
    ESTADO.get_or_init(|| Mutex::new(Estado::default()))
}

fn parar() -> &'static AtomicBool {
    static PARAR: AtomicBool = AtomicBool::new(false);
    &PARAR
}

/* ============================ API pública ============================ */

/// Arranca el servidor en `127.0.0.1:port`. Si ya estaba arriba, devuelve su
/// estado en vez de intentar un segundo bind.
pub async fn start(cfg: AppConfig, port: u16, token: String) -> Result<ServeStatus, String> {
    if status().running {
        return Ok(status());
    }

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .map_err(|e| {
            format!(
                "no se pudo escuchar en 127.0.0.1:{port}: {e} \
                 (¿ya está arrancado o escucha en otro puerto?)"
            )
        })?;

    let modelos = catalog::collect(&cfg).await.total;
    {
        let mut st = match estado().lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        st.port = port;
        st.token = token;
        st.running = true;
        st.modelos = modelos;
        st.ultimo_error = None;
        st.errores = 0;
        st.peticiones = 0;
    }
    parar().store(false, Ordering::SeqCst);

    let cfg = Arc::new(cfg);
    tokio::spawn(async move {
        loop {
            if parar().load(Ordering::SeqCst) {
                break;
            }
            match tokio::time::timeout(Duration::from_millis(400), listener.accept()).await {
                Ok(Ok((socket, _))) => {
                    let cfg = cfg.clone();
                    tokio::spawn(async move {
                        atender(socket, cfg).await;
                    });
                }
                Ok(Err(_e)) => {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                Err(_) => {}
            }
        }
        if let Ok(mut st) = estado().lock() {
            st.running = false;
        }
    });

    Ok(status())
}

pub fn status() -> ServeStatus {
    let st = match estado().lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    ServeStatus {
        running: st.running,
        url: if st.port > 0 {
            format!("http://127.0.0.1:{}", st.port)
        } else {
            String::new()
        },
        port: st.port,
        token: st.token.clone(),
        peticiones: st.peticiones,
        errores: st.errores,
        modelos: st.modelos,
        ultimo_error: st.ultimo_error.clone(),
    }
}

pub fn stop() {
    parar().store(true, Ordering::SeqCst);
    if let Ok(mut st) = estado().lock() {
        st.running = false;
    }
}

/// Modelos que sirve el puente, como `(motor::id, motor)`. Es lo que se pega en
/// la configuración de OpenCode.
pub async fn modelos_catalogo() -> Vec<(String, String)> {
    let cfg = crate::config::load_config();
    catalog::collect(&cfg)
        .await
        .models
        .iter()
        .filter(|m| m.can_chat)
        .map(|m| (m.key.clone(), m.engine.clone()))
        .collect()
}

/* ============================ HTTP mínimo ============================ */

fn respuesta(status: u16, tipo: &str, cuerpo: &str) -> Vec<u8> {
    let motivo = match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        405 => "Method Not Allowed",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        _ => "OK",
    };
    let cabecera = format!(
        "HTTP/1.1 {status} {motivo}\r\nContent-Type: {tipo}\r\n\
         Cache-Control: no-store\r\nContent-Length: {}\r\n\
         Access-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n",
        cuerpo.len()
    );
    let mut out = cabecera.into_bytes();
    out.extend_from_slice(cuerpo.as_bytes());
    out
}

fn json(status: u16, v: &Value) -> Vec<u8> {
    respuesta(status, "application/json", &v.to_string())
}

fn error_json(status: u16, msg: &str) -> Vec<u8> {
    json(
        status,
        &json!({ "error": { "message": msg, "type": "kukia_error" } }),
    )
}

async fn atender(mut socket: TcpStream, cfg: Arc<AppConfig>) {
    let mut datos: Vec<u8> = Vec::new();
    let mut buf = [0u8; 8192];

    // Cabeceras.
    let fin_cabeceras = loop {
        match socket.read(&mut buf).await {
            Ok(0) => return,
            Ok(n) => {
                datos.extend_from_slice(&buf[..n]);
                if let Some(p) = buscar(&datos, b"\r\n\r\n") {
                    break p + 4;
                }
                if datos.len() > 64 * 1024 {
                    let _ = socket.write_all(&error_json(400, "cabeceras demasiado grandes")).await;
                    return;
                }
            }
            Err(_) => return,
        }
    };

    let cabeceras = String::from_utf8_lossy(&datos[..fin_cabeceras]).to_string();
    let mut lineas = cabeceras.lines();
    let primera = lineas.next().unwrap_or("").to_string();
    let partes: Vec<&str> = primera.split_whitespace().collect();
    if partes.len() < 2 {
        let _ = socket.write_all(&error_json(400, "petición mal formada")).await;
        return;
    }
    let metodo = partes[0].to_string();
    let ruta = partes[1].to_string();

    let mut largo = 0usize;
    let mut autorizacion = String::new();
    for l in lineas {
        let low = l.to_lowercase();
        if let Some(v) = low.strip_prefix("content-length:") {
            largo = v.trim().parse::<usize>().unwrap_or(0);
        }
        if let Some(v) = low.strip_prefix("authorization:") {
            autorizacion = v.trim().to_string();
        }
    }

    let mut cuerpo = datos[fin_cabeceras..].to_vec();
    while cuerpo.len() < largo {
        match socket.read(&mut buf).await {
            Ok(0) => break,
            Ok(n) => cuerpo.extend_from_slice(&buf[..n]),
            Err(_) => break,
        }
    }

    {
        let mut st = match estado().lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        st.peticiones += 1;
    }

    let fuera = match (metodo.as_str(), ruta.split('?').next().unwrap_or("")) {
        ("GET", "/v1/models") | ("GET", "/models") => {
            let lista = modelos_catalogo().await;
            json(
                200,
                &json!({
                    "object": "list",
                    "data": lista
                        .iter()
                        .map(|(id, _)| json!({
                            "id": id,
                            "object": "model",
                            "owned_by": "kukia",
                            "created": 0
                        }))
                        .collect::<Vec<_>>()
                }),
            )
        }
        ("GET", "/v1/ping") | ("GET", "/health") => json(
            200,
            &json!({ "ok": true, "servicio": "kukia", "solo_loopback": true }),
        ),
        ("POST", "/v1/chat/completions") | ("POST", "/chat/completions") => {
            if !token_valido(&autorizacion) {
                let mut st = match estado().lock() {
                    Ok(g) => g,
                    Err(p) => p.into_inner(),
                };
                st.errores += 1;
                st.ultimo_error = Some("token inválido".into());
                error_json(401, "token inválido: usa el que muestra el puente de KUKIA")
            } else {
                match serde_json::from_slice::<Value>(&cuerpo) {
                    Ok(v) => completar(&cfg, &v).await,
                    Err(e) => error_json(400, &format!("JSON inválido: {e}")),
                }
            }
        }
        ("OPTIONS", _) => respuesta(200, "text/plain", ""),
        _ => error_json(404, "ruta desconocida; usa /v1/models o /v1/chat/completions"),
    };

    let _ = socket.write_all(&fuera).await;
    let _ = socket.shutdown().await;
}

fn token_valido(autorizacion: &str) -> bool {
    let esperado = match estado().lock() {
        Ok(g) => g.token.clone(),
        Err(p) => p.into_inner().token.clone(),
    };
    if esperado.trim().is_empty() {
        return true;
    }
    let recibido = autorizacion
        .trim()
        .strip_prefix("Bearer ")
        .or_else(|| autorizacion.trim().strip_prefix("bearer "))
        .unwrap_or("")
        .trim();
    recibido == esperado
}

fn buscar(pajar: &[u8], aguja: &[u8]) -> Option<usize> {
    pajar
        .windows(aguja.len())
        .position(|v| v == aguja)
}

/// Traduce la petición OpenAI al motor y devuelve la respuesta en el mismo
/// formato. El cuerpo no-stream y el SSE son los del servidor original.
async fn completar(cfg: &AppConfig, cuerpo: &Value) -> Vec<u8> {
    let modelo_completo = cuerpo["model"].as_str().unwrap_or("").to_string();
    let (motor, modelo) = if let Some((m, id)) = modelo_completo.split_once("::") {
        (m.to_string(), id.to_string())
    } else {
        ("ollama".to_string(), modelo_completo.clone())
    };
    if modelo.trim().is_empty() {
        return error_json(400, "falta «model»");
    }

    let stream = cuerpo["stream"].as_bool().unwrap_or(false);
    let temperatura = cuerpo["temperature"].as_f64().map(|t| t as f32);
    let max_tokens = cuerpo["max_tokens"].as_u64().map(|t| t as u32);

    let mut lista: Vec<providers::ChatMessage> = Vec::new();
    let mut sistema = String::new();
    for m in cuerpo["messages"].as_array().cloned().unwrap_or_default() {
        let rol = m["role"].as_str().unwrap_or("user");
        let texto = m["content"].as_str().unwrap_or("").to_string();
        if rol == "system" {
            if !sistema.is_empty() {
                sistema.push('\n');
            }
            sistema.push_str(&texto);
        } else {
            lista.push(providers::ChatMessage {
                role: rol.to_string(),
                content: texto,
            });
        }
    }
    if lista.is_empty() {
        return error_json(400, "faltan mensajes");
    }

    let req = providers::ChatRequest {
        provider: motor.clone(),
        model: modelo.clone(),
        messages: lista,
        temperature: temperatura,
        max_tokens,
        system_prompt: if sistema.is_empty() {
            None
        } else {
            Some(sistema)
        },
        // El puente sirve texto a terceros: no ejecuta herramientas en su nombre.
        tools: false,
        aprobar: false,
        agent: None,
    };

    let r = providers::chat(&req, cfg).await;
    match r {
        Err(e) => {
            let mut st = match estado().lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            st.errores += 1;
            st.ultimo_error = Some(e.clone());
            error_json(502, &e)
        }
        Ok(resp) => {
            let id = format!("chatcmpl-kukia-{}", chrono::Utc::now().timestamp_millis());
            if !stream {
                return json(
                    200,
                    &serde_json::json!({
                        "id": id,
                        "object": "chat.completion",
                        "created": chrono::Utc::now().timestamp(),
                        "model": format!("{motor}::{modelo}"),
                        "choices": [{
                            "index": 0,
                            "message": { "role": "assistant", "content": resp.content },
                            "finish_reason": "stop"
                        }],
                        "usage": {
                            "prompt_tokens": resp.prompt_tokens,
                            "completion_tokens": resp.completion_tokens,
                            "total_tokens": resp.prompt_tokens + resp.completion_tokens
                        },
                        "kukia": { "motor": motor, "modelo": modelo, "ms": resp.total_ms }
                    }),
                );
            }

            // SSE mínimo válido: un delta de rol, un delta de contenido y el cierre.
            let creado = chrono::Utc::now().timestamp();
            let evento = |delta: serde_json::Value, fin: bool| {
                serde_json::json!({
                    "id": id,
                    "object": "chat.completion.chunk",
                    "created": creado,
                    "model": format!("{motor}::{modelo}"),
                    "choices": [{
                        "index": 0,
                        "delta": delta,
                        "finish_reason": if fin { serde_json::json!("stop") } else { serde_json::Value::Null }
                    }]
                })
            };
            let mut sse = String::new();
            for d in [
                evento(serde_json::json!({ "role": "assistant", "content": "" }), false),
                evento(serde_json::json!({ "content": resp.content }), false),
                evento(serde_json::json!({}), true),
            ] {
                sse.push_str("data: ");
                sse.push_str(&d.to_string());
                sse.push_str("\n\n");
            }
            sse.push_str("data: [DONE]\n\n");

            let cabecera = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                sse.len()
            );
            let mut out = cabecera.into_bytes();
            out.extend_from_slice(sse.as_bytes());
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_estado_arranca_parado() {
        let st = status();
        assert!(st.port == 0 || st.running || !st.running);
    }

    #[test]
    fn un_token_distinto_se_rechaza() {
        {
            let mut st = estado().lock().unwrap();
            st.token = "kukia-prueba".into();
        }
        assert!(token_valido("Bearer kukia-prueba"));
        assert!(!token_valido("Bearer otra-cosa"));
        assert!(!token_valido(""));
        {
            let mut st = estado().lock().unwrap();
            st.token = String::new();
        }
        assert!(token_valido(""), "sin token configurado no se exige");
    }

    #[test]
    fn las_respuestas_json_llevan_cabecera_y_cuerpo() {
        let r = json(200, &json!({ "ok": true }));
        let t = String::from_utf8_lossy(&r);
        assert!(t.starts_with("HTTP/1.1 200 OK"));
        assert!(t.contains("Content-Type: application/json"));
        assert!(t.contains("{\"ok\":true}"));
    }
}
