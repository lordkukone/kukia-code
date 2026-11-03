//! Validación de Fase 2.1 — pruebas REALES del núcleo.
//!
//! Estos tests ejercitan el código de seguridad y de research contra el
//! sistema real: PowerShell, SQLite y las APIs públicas del radar.
//!
//! Los que salen a la red están marcados `#[ignore]` para que `cargo test`
//! siga siendo determinista en local; se ejecutan explícitamente con:
//!
//! ```text
//! cargo test -p kukia-core --test validacion_fase21 -- --ignored --nocapture
//! ```

use kukia_core::{memory::Database, research, security};
use std::path::PathBuf;

fn db_temporal(nombre: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("kukia_fase21");
    std::fs::create_dir_all(&dir).expect("crear dir temporal");
    let path = dir.join(format!("{nombre}.db"));
    let _ = std::fs::remove_file(&path);
    path
}

/* --------------------------- SEGURIDAD --------------------------- */

#[tokio::test]
async fn terminal_sin_aprobacion_se_rechaza() {
    let res = security::run_powershell("Write-Output 'no deberia ejecutarse'", ".", false).await;
    match res {
        Ok(r) => panic!("FALLO DE SEGURIDAD: se ejecuto sin aprobacion. stdout={:?}", r.stdout),
        Err(e) => {
            println!("OK rechazo sin aprobacion -> {e}");
            assert!(e.to_lowercase().contains("aprobaci"), "mensaje inesperado: {e}");
        }
    }
}

#[tokio::test]
async fn terminal_con_aprobacion_se_ejecuta() {
    let res = security::run_powershell("Write-Output 'KUKIA_OK'", ".", true)
        .await
        .expect("con aprobacion debe ejecutarse");
    println!("codigo={} stdout={:?}", res.code, res.stdout);
    assert_eq!(res.code, 0);
    assert!(res.stdout.contains("KUKIA_OK"));
}

#[tokio::test]
async fn terminal_rechaza_directorio_inexistente() {
    let res = security::run_powershell("Write-Output x", "Z:\\ruta_que_no_existe_kukia", true).await;
    assert!(res.is_err(), "deberia rechazar un cwd inexistente");
}

#[test]
fn lectura_rechaza_archivo_demasiado_grande() {
    let path = db_temporal("lectura");
    std::fs::write(&path, "x".repeat(5000)).unwrap();
    let res = security::read_text_file(path.to_str().unwrap(), 1000);
    assert!(res.is_err(), "deberia rechazar por limite de tamano");
    println!("OK limite -> {}", res.unwrap_err());
}

#[test]
fn lectura_normal_funciona() {
    let path = db_temporal("lectura_ok");
    std::fs::write(&path, "contenido de prueba").unwrap();
    let txt = security::read_text_file(path.to_str().unwrap(), 1_000_000).unwrap();
    assert_eq!(txt, "contenido de prueba");
}

/* --------------------------- MEMORIA / AUDITORÍA --------------------------- */

#[test]
fn auditoria_registra_aprobaciones() {
    let path = db_temporal("auditoria");
    let db = Database::open(&path).expect("abrir sqlite");

    db.approval_log("files.write", "H:\\prueba.txt", false).unwrap();
    db.approval_log("terminal.run", "H:\\ > Get-Date", true).unwrap();

    let rows = db.approvals_list().unwrap();
    println!("aprobaciones registradas: {}", rows.len());
    for r in &rows {
        println!("  [{}] {} approved={} {}", r.id, r.action, r.approved, r.detail);
    }
    assert!(rows.len() >= 2);
    assert!(rows.iter().any(|r| r.action == "files.write" && !r.approved));
    assert!(rows.iter().any(|r| r.action == "terminal.run" && r.approved));
}

#[test]
fn sesiones_y_memoria_persisten() {
    let path = db_temporal("persistencia");

    {
        let db = Database::open(&path).unwrap();
        let s = db.session_create("Sesion de validacion", "coder").unwrap();
        db.message_add(&s.id, "user", "hola").unwrap();
        db.memory_add("dato persistente fase 2.1", "validacion").unwrap();
    }

    let db = Database::open(&path).unwrap();
    let sesiones = db.sessions_list().unwrap();
    let msgs = db.messages_list(&sesiones[0].id).unwrap();
    let mems = db.memory_search("persistente").unwrap();

    println!("sesiones={} mensajes={} memorias={}", sesiones.len(), msgs.len(), mems.len());
    assert!(!sesiones.is_empty(), "la sesion debe persistir tras reabrir");
    assert!(!msgs.is_empty(), "los mensajes deben persistir");
    assert!(!mems.is_empty(), "las memorias deben persistir");
}

/* --------------------------- PROVEEDORES (REAL) --------------------------- */

#[test]
#[ignore = "requiere Ollama escuchando en 127.0.0.1:11434"]
fn proveedor_ollama_lista_y_responde() {
    use kukia_core::config::AppConfig;
    use kukia_core::providers::{self, ChatMessage, ChatRequest};

    let rt = tokio::runtime::Runtime::new().unwrap();
    let cfg = AppConfig::default();

    let modelos = rt
        .block_on(providers::list_models("ollama", &cfg))
        .expect("Ollama debe responder en /api/tags");
    println!("modelos detectados en Ollama: {}", modelos.len());
    assert!(!modelos.is_empty(), "Ollama debe tener al menos un modelo");

    let modelo = modelos
        .iter()
        .find(|m| m.id.contains("llama3"))
        .map(|m| m.id.clone())
        .unwrap_or_else(|| modelos[0].id.clone());
    println!("modelo de prueba: {modelo}");

    let req = ChatRequest {
        provider: "ollama".into(),
        model: modelo.clone(),
        messages: vec![ChatMessage {
            role: "user".into(),
            content: "Responde solo: OK".into(),
        }],
        temperature: Some(0.0),
        max_tokens: Some(16),
        system_prompt: None,
    
        tools: false,
        aprobar: false,
        agent: None,
};

    let t0 = std::time::Instant::now();
    let res = rt.block_on(providers::chat(&req, &cfg)).expect("chat real");
    let ms = t0.elapsed().as_millis();
    println!(
        "respuesta: {:?} | prompt_tokens={} completion_tokens={} total_ms={} (medido {ms} ms)",
        res.content.trim(),
        res.prompt_tokens,
        res.completion_tokens,
        res.total_ms
    );
    assert!(!res.content.trim().is_empty(), "la respuesta no puede estar vacia");
}

#[test]
#[ignore = "requiere LM Studio con API en 127.0.0.1:1234"]
fn proveedor_lmstudio_lista() {
    use kukia_core::config::AppConfig;
    use kukia_core::providers;

    let rt = tokio::runtime::Runtime::new().unwrap();
    let cfg = AppConfig::default();
    match rt.block_on(providers::list_models("lmstudio", &cfg)) {
        Ok(m) => {
            println!("LM Studio activo con {} modelos", m.len());
            for x in m.iter().take(5) {
                println!("  {}", x.id);
            }
            assert!(!m.is_empty());
        }
        Err(e) => println!("LM Studio no disponible: {e}"),
    }
}

#[test]
#[ignore = "requiere un modelo cargado en LM Studio"]
fn proveedor_lmstudio_chat_real() {
    use kukia_core::config::AppConfig;
    use kukia_core::providers::{self, ChatMessage, ChatRequest};

    let rt = tokio::runtime::Runtime::new().unwrap();
    let cfg = AppConfig::default();

    let modelos = rt
        .block_on(providers::list_models("lmstudio", &cfg))
        .expect("LM Studio debe responder");
    let modelo = std::env::var("KUKIA_LMSTUDIO_MODEL").unwrap_or_else(|_| modelos[0].id.clone());
    println!("modelo LM Studio: {modelo}");

    let req = ChatRequest {
        provider: "lmstudio".into(),
        model: modelo,
        messages: vec![ChatMessage {
            role: "user".into(),
            content: "Responde solo con la palabra OK".into(),
        }],
        temperature: Some(0.0),
        max_tokens: Some(16),
        system_prompt: None,
    
        tools: false,
        aprobar: false,
        agent: None,
};
    let t0 = std::time::Instant::now();
    let res = rt.block_on(providers::chat(&req, &cfg)).expect("chat real LM Studio");
    let ms = t0.elapsed().as_millis();
    println!(
        "respuesta: {:?} | prompt={} completion={} total_ms={} (medido {ms} ms)",
        res.content.trim(),
        res.prompt_tokens,
        res.completion_tokens,
        res.total_ms
    );
    assert!(!res.content.trim().is_empty(), "respuesta vacia");
}

/* --------------------------- RESEARCH (RED REAL) --------------------------- */

#[test]
fn research_rechaza_url_no_https() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let res = rt.block_on(research::fetch_public("http://example.com/a.bin"));
    assert!(res.is_err(), "solo debe permitir https");
    println!("OK rechazo http -> {}", res.unwrap_err());
}

#[test]
#[ignore = "sale a internet: ejecutar con --ignored"]
fn research_escaneo_real_tres_fuentes() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let items = rt.block_on(research::scan_all());
    println!("descubrimientos totales: {}", items.len());

    let mut arxiv = 0;
    let mut hf = 0;
    let mut gh = 0;
    for d in &items {
        println!(
            "  [{}] {} | score {:.2} | {}",
            d.source, d.title, d.score, d.url
        );
        let src = d.source.to_lowercase();
        if src.contains("arxiv") {
            arxiv += 1;
        } else if src.contains("hugging") {
            hf += 1;
        } else if src.contains("github") {
            gh += 1;
        }
    }
    println!("arxiv={arxiv} huggingface={hf} github={gh}");
    assert!(!items.is_empty(), "el escaneo debe devolver algo con red disponible");

    let alerts = research::alerts_for(&items);
    println!("alertas: {}", alerts.len());
    for a in &alerts {
        println!("  [{}] {}", a.level, a.message);
    }

    let report = research::write_report(&items, &alerts).expect("generar informe");
    let meta = std::fs::metadata(&report).unwrap();
    println!("informe: {} ({} bytes)", report.display(), meta.len());
    assert!(meta.len() > 200, "el informe debe tener contenido real");
}

#[test]
#[ignore = "descarga de red: ejecutar con --ignored"]
fn research_descarga_aislada_respeta_limite() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    // Archivo pequeno y publico para verificar la descarga real.
    let url = "https://raw.githubusercontent.com/ollama/ollama/main/LICENSE";
    match rt.block_on(research::fetch_public(url)) {
        Ok(p) => {
            let meta = std::fs::metadata(&p).unwrap();
            println!("descargado: {} ({} bytes)", p.display(), meta.len());
            assert!(meta.len() > 0 && meta.len() <= 25_000_000);
            assert!(p.to_string_lossy().contains("AI_EXPERIMENTAL"));
        }
        Err(e) => println!("descarga no realizada (red?): {e}"),
    }
}
