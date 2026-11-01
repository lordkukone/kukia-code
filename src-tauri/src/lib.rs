use kukia_core::{
    agents, config, gpu, logs, memory::Database, providers, radar, research, security,
};
use serde::Serialize;
use std::sync::Mutex;
use tauri::ipc::Channel;
use tauri::{Emitter, State};

struct AppState {
    db: Mutex<Database>,
    config: Mutex<config::AppConfig>,
}

#[derive(Serialize)]
struct FileEntry {
    name: String,
    path: String,
    is_dir: bool,
    size: u64,
    modified: String,
}

/* ------------------------- Sistema / configuración ------------------------- */

#[tauri::command]
fn get_system_info() -> gpu::SystemInfo {
    gpu::system_info()
}

#[tauri::command]
fn config_get(state: State<'_, AppState>) -> config::AppConfig {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
fn config_set(
    state: State<'_, AppState>,
    config: config::AppConfig,
) -> Result<config::AppConfig, String> {
    let saved = config::save_config(&config)?;
    *state.config.lock().unwrap() = saved.clone();
    logs::append("info", "configuración guardada");
    Ok(saved)
}

/* ------------------------- Proveedores ------------------------- */

#[tauri::command]
async fn list_providers(state: State<'_, AppState>) -> Result<Vec<providers::ProviderInfo>, String> {
    let cfg = state.config.lock().unwrap().clone();
    Ok(providers::list_providers(&cfg).await)
}

#[tauri::command]
async fn list_models(
    state: State<'_, AppState>,
    provider: String,
) -> Result<Vec<providers::ModelInfo>, String> {
    let cfg = state.config.lock().unwrap().clone();
    providers::list_models(&provider, &cfg).await
}

#[tauri::command]
async fn chat(
    state: State<'_, AppState>,
    request: providers::ChatRequest,
) -> Result<providers::ChatResponse, String> {
    let cfg = state.config.lock().unwrap().clone();
    let res = providers::chat(&request, &cfg).await;
    match &res {
        Ok(r) => logs::append(
            "info",
            &format!(
                "chat {} / {} → {} tok en {} ms",
                request.provider, request.model, r.completion_tokens, r.total_ms
            ),
        ),
        Err(e) => logs::append("error", &format!("chat falló: {e}")),
    }
    res
}

#[tauri::command]
async fn chat_stream(
    state: State<'_, AppState>,
    request: providers::ChatRequest,
    on_event: Channel<providers::ChatEvent>,
) -> Result<(), String> {
    let cfg = state.config.lock().unwrap().clone();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<providers::ChatEvent>();
    let ch = on_event.clone();
    let forwarder = tauri::async_runtime::spawn(async move {
        while let Some(ev) = rx.recv().await {
            let _ = ch.send(ev);
        }
    });
    let res = providers::chat_stream(&request, &cfg, tx).await;
    let _ = forwarder.await;
    if let Err(e) = &res {
        logs::append("error", &format!("stream falló: {e}"));
    }
    res
}

/* ------------------------- GPU ------------------------- */

#[tauri::command]
fn gpu_stats() -> Vec<gpu::GpuStat> {
    gpu::stats()
}

/* ------------------------- Sesiones / mensajes ------------------------- */

#[tauri::command]
fn sessions_list(state: State<'_, AppState>) -> Result<Vec<kukia_core::memory::Session>, String> {
    state.db.lock().unwrap().sessions_list()
}

#[tauri::command]
fn session_create(
    state: State<'_, AppState>,
    title: String,
    agent: String,
) -> Result<kukia_core::memory::Session, String> {
    state.db.lock().unwrap().session_create(&title, &agent)
}

#[tauri::command]
fn session_rename(state: State<'_, AppState>, id: String, title: String) -> Result<(), String> {
    state.db.lock().unwrap().session_rename(&id, &title)
}

#[tauri::command]
fn messages_list(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Vec<kukia_core::memory::MessageRow>, String> {
    state.db.lock().unwrap().messages_list(&session_id)
}

#[tauri::command]
fn message_add(
    state: State<'_, AppState>,
    session_id: String,
    role: String,
    content: String,
) -> Result<kukia_core::memory::MessageRow, String> {
    state.db.lock().unwrap().message_add(&session_id, &role, &content)
}

/* ------------------------- Memoria ------------------------- */

#[tauri::command]
fn memory_add(state: State<'_, AppState>, content: String, tags: String) -> Result<(), String> {
    state.db.lock().unwrap().memory_add(&content, &tags)
}

#[tauri::command]
fn memory_search(state: State<'_, AppState>, query: String) -> Result<Vec<String>, String> {
    state.db.lock().unwrap().memory_search(&query)
}

/* ------------------------- Agentes ------------------------- */

#[tauri::command]
fn agents_list() -> Vec<agents::Agent> {
    agents::load_agents()
}

#[tauri::command]
fn agent_create(
    name: String,
    description: String,
    system_prompt: String,
    tools: String,
) -> Result<agents::Agent, String> {
    agents::create_agent(&name, &description, &system_prompt, &tools)
}

/* ------------------------- Archivos ------------------------- */

#[tauri::command]
fn files_list(path: String) -> Result<Vec<FileEntry>, String> {
    let dir = std::fs::read_dir(&path).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for entry in dir.flatten() {
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let modified = meta
            .modified()
            .ok()
            .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339())
            .unwrap_or_default();
        out.push(FileEntry {
            name: entry.file_name().to_string_lossy().to_string(),
            path: entry.path().display().to_string(),
            is_dir: meta.is_dir(),
            size: meta.len(),
            modified,
        });
    }
    out.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });
    Ok(out)
}

#[tauri::command]
fn files_read(path: String) -> Result<String, String> {
    security::read_text_file(&path, 2_000_000)
}

#[tauri::command]
fn files_write(
    state: State<'_, AppState>,
    path: String,
    content: String,
    approved: bool,
) -> Result<(), String> {
    if !approved {
        return Err("Escritura bloqueada: falta aprobación explícita".into());
    }
    // Backup si el archivo ya existe. Si el backup no se puede verificar,
    // la escritura se cancela: nunca se sobrescribe sin copia válida.
    if std::path::Path::new(&path).exists() {
        let dir = config::data_dir().join("backups");
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("No se pudo crear el directorio de backups: {e}"))?;
        let stamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let name = std::path::Path::new(&path)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "archivo".into());
        let backup = dir.join(format!("{name}.{stamp}.bak"));
        std::fs::copy(&path, &backup)
            .map_err(|e| format!("Backup fallido, escritura cancelada: {e}"))?;
        let origen = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let copia = std::fs::metadata(&backup).map(|m| m.len()).unwrap_or(0);
        if origen != copia {
            return Err(format!(
                "Backup incompleto ({origen} != {copia} bytes), escritura cancelada"
            ));
        }
        logs::append("info", &format!("backup verificado: {}", backup.display()));
    }
    std::fs::write(&path, content).map_err(|e| e.to_string())?;
    state
        .db
        .lock()
        .unwrap()
        .approval_log("files.write", &path, true)?;
    logs::append("warn", &format!("escritura aprobada: {path}"));
    Ok(())
}

/* ------------------------- Terminal ------------------------- */

#[tauri::command]
fn run_command(
    state: State<'_, AppState>,
    command: String,
    cwd: String,
    approved: bool,
) -> Result<security::RunResult, String> {
    let res = security::run_powershell(&command, &cwd, approved);
    state
        .db
        .lock()
        .unwrap()
        .approval_log("terminal.run", &format!("{cwd} > {command}"), approved)?;
    logs::append(
        if res.is_ok() { "info" } else { "error" },
        &format!("comando: {command}"),
    );
    res
}

/* ------------------------- Benchmarks ------------------------- */

#[tauri::command]
async fn benchmark_run(
    state: State<'_, AppState>,
    provider: String,
    model: String,
    prompt: String,
) -> Result<kukia_core::memory::BenchmarkResult, String> {
    let cfg = state.config.lock().unwrap().clone();
    let req = providers::ChatRequest {
        provider: provider.clone(),
        model: model.clone(),
        messages: vec![providers::ChatMessage {
            role: "user".into(),
            content: prompt,
        }],
        temperature: Some(0.2),
        max_tokens: Some(256),
        system_prompt: None,
    };
    let resp = providers::chat(&req, &cfg).await?;
    state.db.lock().unwrap().benchmark_add(
        &provider,
        &model,
        resp.prompt_tokens,
        resp.completion_tokens,
        resp.total_ms,
    )
}

#[tauri::command]
fn benchmarks_list(
    state: State<'_, AppState>,
) -> Result<Vec<kukia_core::memory::BenchmarkResult>, String> {
    state.db.lock().unwrap().benchmarks_list()
}

#[tauri::command]
fn approvals_list(
    state: State<'_, AppState>,
) -> Result<Vec<kukia_core::memory::ApprovalRow>, String> {
    state.db.lock().unwrap().approvals_list()
}

/* ------------------------- Radar / compatibilidad ------------------------- */

#[tauri::command]
async fn radar_check(state: State<'_, AppState>) -> Result<Vec<radar::UpdateInfo>, String> {
    let list = radar::check().await;
    {
        let db = state.db.lock().unwrap();
        for u in &list {
            let _ = db.radar_upsert(&u.source, &u.version, &u.published, &u.url, &u.note);
        }
    }
    logs::append("info", "radar: fuentes consultadas");
    Ok(list)
}

#[tauri::command]
fn compat_add(
    state: State<'_, AppState>,
    engine: String,
    model: String,
    arch: String,
    quant: String,
    gpu: String,
    method: String,
    notes: String,
) -> Result<(), String> {
    state
        .db
        .lock()
        .unwrap()
        .compat_add(&engine, &model, &arch, &quant, &gpu, &method, &notes)
}

#[tauri::command]
fn compat_list(
    state: State<'_, AppState>,
) -> Result<Vec<kukia_core::memory::CompatRow>, String> {
    state.db.lock().unwrap().compat_list()
}

/* ------------------------- KUKIA EXPERIMENTAL AI RADAR ------------------------- */

#[tauri::command]
async fn research_scan(
    state: State<'_, AppState>,
    source: Option<String>,
) -> Result<Vec<research::Discovery>, String> {
    let src = source.unwrap_or_else(|| "all".into());
    let found = research::scan_one(&src).await;
    let mut new_count = 0usize;
    {
        let db = state.db.lock().unwrap();
        for d in &found {
            if db.research_upsert(d).unwrap_or(false) {
                new_count += 1;
            }
        }
        for a in research::alerts_for(&found) {
            let _ = db.research_alert_add(&a.item_id, &a.level, &a.message);
        }
        let _ = db.research_run_add(&src, found.len(), new_count);
    }
    logs::append(
        "info",
        &format!("research: {src} → {} encontrados, {new_count} nuevos", found.len()),
    );
    state.db.lock().unwrap().research_list(300)
}

#[tauri::command]
fn research_items(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<research::Discovery>, String> {
    state.db.lock().unwrap().research_list(limit.unwrap_or(300))
}

#[tauri::command]
fn research_alerts(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<kukia_core::memory::ResearchAlertRow>, String> {
    state
        .db
        .lock()
        .unwrap()
        .research_alerts_list(limit.unwrap_or(100))
}

#[tauri::command]
fn research_report(state: State<'_, AppState>) -> Result<String, String> {
    let items = state.db.lock().unwrap().research_list(300)?;
    let alerts = state.db.lock().unwrap().research_alerts_list(100)?;
    let alerts_core: Vec<research::Alert> = alerts
        .into_iter()
        .map(|a| research::Alert {
            item_id: a.item_id,
            level: a.level,
            message: a.message,
        })
        .collect();
    let path = research::write_report(&items, &alerts_core)?;
    Ok(path.display().to_string())
}

#[tauri::command]
async fn research_fetch(url: String) -> Result<String, String> {
    let path = research::fetch_public(&url).await?;
    logs::append("info", &format!("research fetch: {url} → {}", path.display()));
    Ok(path.display().to_string())
}

#[tauri::command]
fn research_lab_path() -> String {
    research::ensure_lab()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| research::lab_dir().display().to_string())
}

/* ------------------------- Logs ------------------------- */

#[tauri::command]
fn logs_tail(lines: usize) -> Vec<String> {
    logs::tail(lines)
}

/* ------------------------- Arranque ------------------------- */

pub fn run() {
    let dir = config::ensure_dirs();
    let db = Database::open(&dir.join("kukia.db")).expect("no se pudo abrir la base de datos");
    let cfg = config::load_config();
    logs::append("info", "KUKIA CODE arrancando");

    tauri::Builder::default()
        .manage(AppState {
            db: Mutex::new(db),
            config: Mutex::new(cfg),
        })
        .setup(|app| {
            // KUKIA EXPERIMENTAL AI RADAR — vigilancia periódica (solo metadatos).
            // No instala, no descarga binarios y respeta el interruptor de configuración.
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(6 * 3600)).await;
                    let cfg = config::load_config();
                    if !cfg.research_auto {
                        continue;
                    }
                    let items = research::scan_all().await;
                    if items.is_empty() {
                        continue;
                    }
                    let db_path = config::data_dir().join("kukia.db");
                    if let Ok(db) = Database::open(&db_path) {
                        let mut new_count = 0usize;
                        for d in &items {
                            if db.research_upsert(d).unwrap_or(false) {
                                new_count += 1;
                            }
                        }
                        for a in research::alerts_for(&items) {
                            let _ = db.research_alert_add(&a.item_id, &a.level, &a.message);
                        }
                        let _ = db.research_run_add("auto-6h", items.len(), new_count);
                        logs::append(
                            "info",
                            &format!("research auto: {} encontrados, {new_count} nuevos", items.len()),
                        );
                    }
                    let _ = handle.emit("research://update", items.len());
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_system_info,
            config_get,
            config_set,
            list_providers,
            list_models,
            chat,
            chat_stream,
            gpu_stats,
            sessions_list,
            session_create,
            session_rename,
            messages_list,
            message_add,
            memory_add,
            memory_search,
            agents_list,
            agent_create,
            files_list,
            files_read,
            files_write,
            run_command,
            benchmark_run,
            benchmarks_list,
            approvals_list,
            radar_check,
            compat_add,
            compat_list,
            research_scan,
            research_items,
            research_alerts,
            research_report,
            research_fetch,
            research_lab_path,
            logs_tail
        ])
        .run(tauri::generate_context!())
        .expect("error ejecutando KUKIA CODE");
}
