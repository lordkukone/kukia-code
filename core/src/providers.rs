use crate::config::AppConfig;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::UnboundedSender;

/* ============================ Tipos públicos ============================ */

#[derive(Debug, Clone, Serialize)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub base_url: String,
    pub online: bool,
    pub models_count: usize,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
    pub id: String,
    pub provider: String,
    pub name: String,
    pub context: Option<u64>,
    pub size_gb: Option<f64>,
    pub capabilities: Vec<String>,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatRequest {
    pub provider: String,
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub system_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatResponse {
    pub content: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ChatEvent {
    Token { text: String },
    Done {
        prompt_tokens: u64,
        completion_tokens: u64,
        total_ms: u64,
    },
    Error { message: String },
}

/* ============================ Utilidades ============================ */

fn client(timeout: u64) -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout))
        .user_agent("KUKIA-CODE/0.1")
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

fn base(provider: &str, cfg: &AppConfig) -> String {
    match provider {
        "lmstudio" => "http://127.0.0.1:1234".to_string(),
        "cloud" => cfg.cloud_base_url.trim_end_matches('/').to_string(),
        _ => "http://127.0.0.1:11434".to_string(),
    }
}

fn cloud_key() -> Option<String> {
    std::env::var("KUKIA_CLOUD_KEY")
        .ok()
        .filter(|k| !k.trim().is_empty())
}

async fn get_json(url: &str, key: Option<&str>) -> Result<Value, String> {
    let c = client(3);
    let mut req = c.get(url);
    if let Some(k) = key {
        req = req.bearer_auth(k);
    }
    let resp = req.send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    resp.json::<Value>().await.map_err(|e| e.to_string())
}

/* ============================ Proveedores ============================ */

pub async fn list_providers(cfg: &AppConfig) -> Vec<ProviderInfo> {
    let mut out = Vec::new();

    // Ollama
    let ollama_base = "http://127.0.0.1:11434";
    let (online, count, note) = match get_json(&format!("{ollama_base}/api/tags"), None).await {
        Ok(v) => {
            let n = v["models"].as_array().map(|a| a.len()).unwrap_or(0);
            (true, n, String::new())
        }
        Err(e) => (false, 0, format!("sin respuesta: {e}")),
    };
    out.push(ProviderInfo {
        id: "ollama".into(),
        name: "Ollama".into(),
        kind: "local · API nativa".into(),
        base_url: ollama_base.into(),
        online,
        models_count: count,
        note,
    });

    // LM Studio
    let lm_base = "http://127.0.0.1:1234";
    let (online, count, note) = match get_json(&format!("{lm_base}/v1/models"), None).await {
        Ok(v) => {
            let n = v["data"].as_array().map(|a| a.len()).unwrap_or(0);
            (true, n, String::new())
        }
        Err(e) => (
            false,
            0,
            format!("servidor apagado o no accesible ({e})"),
        ),
    };
    out.push(ProviderInfo {
        id: "lmstudio".into(),
        name: "LM Studio".into(),
        kind: "local · OpenAI-compatible".into(),
        base_url: lm_base.into(),
        online,
        models_count: count,
        note,
    });

    // Cloud OpenAI-compatible
    let cloud_base = cfg.cloud_base_url.trim_end_matches('/');
    let key = cloud_key();
    let (online, count, note) = if key.is_none() {
        (false, 0, "falta KUKIA_CLOUD_KEY".to_string())
    } else {
        match get_json(&format!("{cloud_base}/models"), key.as_deref()).await {
            Ok(v) => {
                let n = v["data"].as_array().map(|a| a.len()).unwrap_or(0);
                (true, n, String::new())
            }
            Err(e) => (false, 0, format!("sin respuesta: {e}")),
        }
    };
    out.push(ProviderInfo {
        id: "cloud".into(),
        name: "Cloud (OpenAI-compatible)".into(),
        kind: "remoto".into(),
        base_url: cloud_base.to_string(),
        online,
        models_count: count,
        note,
    });

    out
}

pub async fn list_models(provider: &str, cfg: &AppConfig) -> Result<Vec<ModelInfo>, String> {
    match provider {
        "lmstudio" | "cloud" => {
            let url = format!("{}/v1/models", base(provider, cfg));
            let v = get_json(&url, cloud_key().as_deref()).await?;
            let mut out = Vec::new();
            if let Some(arr) = v["data"].as_array() {
                for m in arr {
                    let id = m["id"].as_str().unwrap_or("").to_string();
                    if id.is_empty() {
                        continue;
                    }
                    out.push(ModelInfo {
                        id: id.clone(),
                        provider: provider.into(),
                        name: id,
                        context: m["context_length"].as_u64(),
                        size_gb: None,
                        capabilities: vec!["texto".into()],
                        state: "disponible".into(),
                    });
                }
            }
            out.sort_by(|a, b| a.name.cmp(&b.name));
            Ok(out)
        }
        _ => {
            let url = format!("{}/api/tags", base(provider, cfg));
            let v = get_json(&url, None).await?;
            let mut out = Vec::new();
            if let Some(arr) = v["models"].as_array() {
                for m in arr {
                    let name = m["name"].as_str().unwrap_or("").to_string();
                    if name.is_empty() {
                        continue;
                    }
                    let family = m["details"]["family"].as_str().unwrap_or("");
                    let quant = m["details"]["quantization_level"].as_str().unwrap_or("");
                    let mut caps = vec!["texto".to_string()];
                    let lower = name.to_lowercase();
                    if lower.contains("vl") || lower.contains("llava") || family.contains("clip") {
                        caps.push("visión".into());
                    }
                    if lower.contains("coder") {
                        caps.push("código".into());
                    }
                    if lower.contains("embed") {
                        caps.push("embeddings".into());
                    }
                    let size = m["size"].as_u64().map(|b| b as f64 / 1e9);
                    out.push(ModelInfo {
                        id: name.clone(),
                        provider: "ollama".into(),
                        name: if quant.is_empty() {
                            name
                        } else {
                            format!("{name} · {quant}")
                        },
                        context: None,
                        size_gb: size,
                        capabilities: caps,
                        state: "disponible".into(),
                    });
                }
            }
            out.sort_by(|a, b| a.name.cmp(&b.name));
            Ok(out)
        }
    }
}

/* ============================ Chat ============================ */

fn build_messages(req: &ChatRequest) -> Vec<Value> {
    let mut msgs: Vec<Value> = Vec::new();
    if let Some(sys) = &req.system_prompt {
        if !sys.trim().is_empty() {
            msgs.push(json!({ "role": "system", "content": sys }));
        }
    }
    for m in &req.messages {
        msgs.push(json!({ "role": m.role, "content": m.content }));
    }
    msgs
}

pub async fn chat(req: &ChatRequest, cfg: &AppConfig) -> Result<ChatResponse, String> {
    match req.provider.as_str() {
        "lmstudio" | "cloud" => chat_openai(req, cfg).await,
        _ => chat_ollama(req, cfg).await,
    }
}

async fn chat_ollama(req: &ChatRequest, cfg: &AppConfig) -> Result<ChatResponse, String> {
    let url = format!("{}/api/chat", base("ollama", cfg));
    let started = Instant::now();
    let body = json!({
        "model": req.model,
        "messages": build_messages(req),
        "stream": false,
        "options": { "temperature": req.temperature.unwrap_or(cfg.temperature) }
    });
    let resp = client(600)
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Ollama: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Ollama HTTP {status}: {text}"));
    }
    let v: Value = resp.json().await.map_err(|e| e.to_string())?;
    let content = v["message"]["content"].as_str().unwrap_or("").to_string();
    let prompt_tokens = v["prompt_eval_count"].as_u64().unwrap_or(0);
    let completion_tokens = v["eval_count"].as_u64().unwrap_or(0);
    let total_ms = v["total_duration"]
        .as_u64()
        .map(|ns| ns / 1_000_000)
        .unwrap_or_else(|| started.elapsed().as_millis() as u64);
    Ok(ChatResponse {
        content,
        prompt_tokens,
        completion_tokens,
        total_ms,
    })
}

async fn chat_openai(req: &ChatRequest, cfg: &AppConfig) -> Result<ChatResponse, String> {
    let url = format!("{}/v1/chat/completions", base(&req.provider, cfg));
    let started = Instant::now();
    let mut body = json!({
        "model": req.model,
        "messages": build_messages(req),
        "temperature": req.temperature.unwrap_or(cfg.temperature),
        "stream": false
    });
    if let Some(mt) = req.max_tokens {
        body["max_tokens"] = json!(mt);
    }
    let mut builder = client(600).post(&url).json(&body);
    if let Some(k) = cloud_key() {
        builder = builder.bearer_auth(k);
    }
    let resp = builder.send().await.map_err(|e| format!("{}: {e}", req.provider))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("{} HTTP {status}: {text}", req.provider));
    }
    let v: Value = resp.json().await.map_err(|e| e.to_string())?;
    let content = v["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let prompt_tokens = v["usage"]["prompt_tokens"].as_u64().unwrap_or(0);
    let completion_tokens = v["usage"]["completion_tokens"].as_u64().unwrap_or(0);
    Ok(ChatResponse {
        content,
        prompt_tokens,
        completion_tokens,
        total_ms: started.elapsed().as_millis() as u64,
    })
}

/* ============================ Streaming ============================ */

pub async fn chat_stream(
    req: &ChatRequest,
    cfg: &AppConfig,
    tx: UnboundedSender<ChatEvent>,
) -> Result<(), String> {
    let result = match req.provider.as_str() {
        "lmstudio" | "cloud" => stream_openai(req, cfg, &tx).await,
        _ => stream_ollama(req, cfg, &tx).await,
    };
    if let Err(e) = &result {
        let _ = tx.send(ChatEvent::Error {
            message: e.clone(),
        });
    }
    result
}

async fn stream_ollama(
    req: &ChatRequest,
    cfg: &AppConfig,
    tx: &UnboundedSender<ChatEvent>,
) -> Result<(), String> {
    let url = format!("{}/api/chat", base("ollama", cfg));
    let body = json!({
        "model": req.model,
        "messages": build_messages(req),
        "stream": true,
        "options": { "temperature": req.temperature.unwrap_or(cfg.temperature) }
    });
    let resp = client(600)
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Ollama: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Ollama HTTP {status}: {text}"));
    }

    let mut stream = resp.bytes_stream();
    let mut buf = String::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        buf.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(pos) = buf.find('\n') {
            let line: String = buf.drain(..=pos).collect();
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(v) = serde_json::from_str::<Value>(line) {
                if let Some(text) = v["message"]["content"].as_str() {
                    if !text.is_empty() {
                        let _ = tx.send(ChatEvent::Token {
                            text: text.to_string(),
                        });
                    }
                }
                if v["done"].as_bool().unwrap_or(false) {
                    let _ = tx.send(ChatEvent::Done {
                        prompt_tokens: v["prompt_eval_count"].as_u64().unwrap_or(0),
                        completion_tokens: v["eval_count"].as_u64().unwrap_or(0),
                        total_ms: v["total_duration"]
                            .as_u64()
                            .map(|ns| ns / 1_000_000)
                            .unwrap_or(0),
                    });
                    return Ok(());
                }
            }
        }
    }
    let _ = tx.send(ChatEvent::Done {
        prompt_tokens: 0,
        completion_tokens: 0,
        total_ms: 0,
    });
    Ok(())
}

async fn stream_openai(
    req: &ChatRequest,
    cfg: &AppConfig,
    tx: &UnboundedSender<ChatEvent>,
) -> Result<(), String> {
    let url = format!("{}/v1/chat/completions", base(&req.provider, cfg));
    let started = Instant::now();
    let body = json!({
        "model": req.model,
        "messages": build_messages(req),
        "temperature": req.temperature.unwrap_or(cfg.temperature),
        "stream": true
    });
    let mut builder = client(600).post(&url).json(&body);
    if let Some(k) = cloud_key() {
        builder = builder.bearer_auth(k);
    }
    let resp = builder.send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("HTTP {status}: {text}"));
    }

    let mut stream = resp.bytes_stream();
    let mut buf = String::new();
    let mut completion_tokens = 0u64;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        buf.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(pos) = buf.find('\n') {
            let line: String = buf.drain(..=pos).collect();
            let line = line.trim();
            if let Some(data) = line.strip_prefix("data:") {
                let data = data.trim();
                if data == "[DONE]" {
                    let _ = tx.send(ChatEvent::Done {
                        prompt_tokens: 0,
                        completion_tokens,
                        total_ms: started.elapsed().as_millis() as u64,
                    });
                    return Ok(());
                }
                if let Ok(v) = serde_json::from_str::<Value>(data) {
                    if let Some(text) = v["choices"][0]["delta"]["content"].as_str() {
                        if !text.is_empty() {
                            completion_tokens += 1;
                            let _ = tx.send(ChatEvent::Token {
                                text: text.to_string(),
                            });
                        }
                    }
                }
            }
        }
    }
    let _ = tx.send(ChatEvent::Done {
        prompt_tokens: 0,
        completion_tokens,
        total_ms: started.elapsed().as_millis() as u64,
    });
    Ok(())
}

/* ============================ Tests ============================ */

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_include_system_prompt() {
        let req = ChatRequest {
            provider: "ollama".into(),
            model: "m".into(),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: "hola".into(),
            }],
            temperature: None,
            max_tokens: None,
            system_prompt: Some("eres kukia".into()),
        };
        let msgs = build_messages(&req);
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0]["role"], "system");
    }

    #[test]
    fn base_urls_are_correct() {
        let cfg = AppConfig::default();
        assert_eq!(base("ollama", &cfg), "http://127.0.0.1:11434");
        assert_eq!(base("lmstudio", &cfg), "http://127.0.0.1:1234");
        assert_eq!(base("cloud", &cfg), "https://api.openai.com/v1");
    }

    #[test]
    fn chat_event_serializes_with_kind() {
        let ev = ChatEvent::Token {
            text: "hola".into(),
        };
        let raw = serde_json::to_string(&ev).unwrap();
        assert!(raw.contains("\"kind\":\"token\""));
        assert!(raw.contains("hola"));
    }
}
