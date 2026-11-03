//! Prueba de integración del puente inverso: KUKIA sirve sus modelos por HTTP
//! compatible con OpenAI, que es lo que OpenCode necesita.
//!
//! Se ejecuta de verdad: abre el servidor en un puerto libre de loopback, habla
//! con él por HTTP como haría OpenCode y, si Ollama tiene modelos, comprueba que
//! el chat devuelve texto. Si no hay ningún motor local encendido, la prueba lo
//! dice y no inventa un resultado.

use kukia_core::config::AppConfig;
use kukia_core::serve;

const TOKEN: &str = "token-de-prueba";

fn cfg() -> AppConfig {
    AppConfig::default()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn el_servidor_de_kukia_habla_openai() {
    let st = serve::start(cfg(), 0, TOKEN.to_string())
        .await
        .expect("el servidor debe arrancar en un puerto libre");
    let base = st.url.clone();
    assert!(st.running, "debe quedar marcado como vivo");
    assert!(base.starts_with("http://127.0.0.1:"), "solo loopback: {base}");

    let cli = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(150))
        .build()
        .unwrap();

    // 1) /health es público.
    let r = cli.get(format!("{base}/health")).send().await.unwrap();
    assert_eq!(r.status(), 200);
    let v: serde_json::Value = r.json().await.unwrap();
    assert_eq!(v["ok"], true);

    // 2) Sin token, todo lo demás da 401.
    let r = cli.get(format!("{base}/v1/models")).send().await.unwrap();
    assert_eq!(r.status(), 401, "sin token debe rechazar");

    // 3) Con token, la lista de modelos.
    let r = cli
        .get(format!("{base}/v1/models"))
        .bearer_auth(TOKEN)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let v: serde_json::Value = r.json().await.unwrap();
    let modelos: Vec<String> = v["data"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|m| m["id"].as_str().map(|s| s.to_string()))
        .collect();
    println!("modelos servidos por KUKIA: {}", modelos.len());

    // 4) Modelo desconocido: error claro, no un 200 vacío.
    let r = cli
        .post(format!("{base}/v1/chat/completions"))
        .bearer_auth(TOKEN)
        .json(&serde_json::json!({
            "model": "no_existe::tampoco",
            "messages": [{ "role": "user", "content": "hola" }]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 404);

    // 5) Chat real contra el primer modelo local disponible.
    let elegido = modelos
        .iter()
        .find(|m| m.starts_with("ollama::"))
        .cloned();
    match elegido {
        None => println!(
            "sin modelos de ollama en el catálogo: se omite el chat real (¿motor apagado?)"
        ),
        Some(modelo) => {
            println!("chat de prueba con {modelo}");
            let r = cli
                .post(format!("{base}/v1/chat/completions"))
                .bearer_auth(TOKEN)
                .json(&serde_json::json!({
                    "model": modelo,
                    "messages": [{ "role": "user", "content": "Responde solo: KUKIA OK" }],
                    "max_tokens": 24
                }))
                .send()
                .await
                .unwrap();
            let estado = r.status();
            let v: serde_json::Value = r.json().await.unwrap();
            assert_eq!(estado, 200, "el chat debe responder 200: {v}");
            let texto = v["choices"][0]["message"]["content"].as_str().unwrap_or("");
            println!("respuesta: {:?}", texto);
            assert!(
                !texto.trim().is_empty(),
                "la respuesta no puede venir vacía: {v}"
            );
            assert_eq!(v["object"], "chat.completion");
        }
    }

    serve::stop();
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert!(!serve::status().running, "stop debe dejar el servidor parado");
}
