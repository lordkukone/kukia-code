use kukia_core::{
    agents, config, gaming, gpu, logs,
    memory::{AgentPolicy, AuditRow, Database, GamingAlertRow, GamingItem, Project},
    permissions, providers, radar, research, security, vram,
};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};
use tauri::ipc::Channel;
use tauri::{Emitter, State};
use tokio::sync::oneshot;

/// Comando en curso, para poder cancelarlo desde la interfaz.
struct RunningCmd {
    pid: u32,
    command: String,
    cwd: String,
    started_at: String,
    cancel: Option<oneshot::Sender<()>>,
}

struct AppState {
    db: Arc<Mutex<Database>>,
    config: Mutex<config::AppConfig>,
    running: Arc<Mutex<HashMap<String, RunningCmd>>>,
}

/// Candado a prueba de veneno: si un hilo entró en pánico con el mutex tomado,
/// se recupera el contenido en lugar de paniquear en cadena (E-07).
fn guard<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

#[derive(Serialize)]
struct FileEntry {
    name: String,
    path: String,
    is_dir: bool,
    size: u64,
    modified: String,
}

#[derive(Clone, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum StreamEvent {
    Started { id: String, pid: u32 },
    Done {
        id: String,
        result: security::RunResult,
    },
    Failed { id: String, message: String },
}

#[derive(Serialize)]
struct RunningInfo {
    id: String,
    pid: u32,
    command: String,
    cwd: String,
    started_at: String,
}

/* ------------------------- Utilidades internas ------------------------- */

const AGENTE_POR_DEFECTO: &str = "kukia-master";

fn agente(agent: &Option<String>) -> String {
    agent
        .as_ref()
        .map(|a| a.trim().to_string())
        .filter(|a| !a.is_empty())
        .unwrap_or_else(|| AGENTE_POR_DEFECTO.to_string())
}

/// Registra una entrada de auditoría. Nunca propaga el error: un fallo al auditar
/// no debe convertir una operación correcta en un fallo para el usuario, pero sí
/// queda en el log de la aplicación.
fn auditar(
    db: &Mutex<Database>,
    ag: &str,
    tool: &str,
    action: &str,
    targets: &str,
    approved: bool,
    result: &str,
    error: &str,
) {
    if let Err(e) = guard(db).audit_log(ag, tool, action, targets, approved, result, error) {
        logs::append("error", &format!("no se pudo escribir la auditoría: {e}"));
    }
}

/* ------------------------- Sistema / configuración ------------------------- */

#[tauri::command]
fn get_system_info() -> gpu::SystemInfo {
    gpu::system_info()
}

#[tauri::command]
fn config_get(state: State<'_, AppState>) -> config::AppConfig {
    guard(&state.config).clone()
}

#[tauri::command]
fn config_set(
    state: State<'_, AppState>,
    config: config::AppConfig,
) -> Result<config::AppConfig, String> {
    let saved = config::save_config(&config)?;
    *guard(&state.config) = saved.clone();
    logs::append("info", "configuración guardada");
    Ok(saved)
}

/* ------------------------- Proveedores ------------------------- */

#[tauri::command]
async fn list_providers(state: State<'_, AppState>) -> Result<Vec<providers::ProviderInfo>, String> {
    let cfg = guard(&state.config).clone();
    Ok(providers::list_providers(&cfg).await)
}

#[tauri::command]
async fn list_models(
    state: State<'_, AppState>,
    provider: String,
) -> Result<Vec<providers::ModelInfo>, String> {
    let cfg = guard(&state.config).clone();
    providers::list_models(&provider, &cfg).await
}

#[tauri::command]
async fn chat(
    state: State<'_, AppState>,
    request: providers::ChatRequest,
) -> Result<providers::ChatResponse, String> {
    let cfg = guard(&state.config).clone();
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
    let cfg = guard(&state.config).clone();
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
    guard(&state.db).sessions_list()
}

#[tauri::command]
fn session_create(
    state: State<'_, AppState>,
    title: String,
    agent: String,
) -> Result<kukia_core::memory::Session, String> {
    guard(&state.db).session_create(&title, &agent)
}

#[tauri::command]
fn session_rename(state: State<'_, AppState>, id: String, title: String) -> Result<(), String> {
    guard(&state.db).session_rename(&id, &title)
}

#[tauri::command]
fn messages_list(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Vec<kukia_core::memory::MessageRow>, String> {
    guard(&state.db).messages_list(&session_id)
}

#[tauri::command]
fn message_add(
    state: State<'_, AppState>,
    session_id: String,
    role: String,
    content: String,
) -> Result<kukia_core::memory::MessageRow, String> {
    guard(&state.db).message_add(&session_id, &role, &content)
}

/* ------------------------- Memoria ------------------------- */

#[tauri::command]
fn memory_add(state: State<'_, AppState>, content: String, tags: String) -> Result<(), String> {
    guard(&state.db).memory_add(&content, &tags)
}

#[tauri::command]
fn memory_search(state: State<'_, AppState>, query: String) -> Result<Vec<String>, String> {
    guard(&state.db).memory_search(&query)
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
fn files_read(
    state: State<'_, AppState>,
    path: String,
    agent: Option<String>,
) -> Result<String, String> {
    let ag = agente(&agent);
    let (politica, proyecto) = politica_y_proyecto(&state, &ag)?;
    let chk = permissions::check_scoped(
        &politica,
        proyecto.as_ref(),
        permissions::Tool::Read,
        &path,
        false,
    );
    if !chk.allowed {
        auditar(
            &state.db,
            &ag,
            "files.read",
            "leer archivo",
            &path,
            false,
            "denegado",
            &chk.reason,
        );
        return Err(chk.reason);
    }
    match security::read_text_file(&path, 2_000_000) {
        Ok(texto) => {
            auditar(
                &state.db,
                &ag,
                "files.read",
                "leer archivo",
                &path,
                false,
                &format!("ok ({} bytes)", texto.len()),
                "",
            );
            Ok(texto)
        }
        Err(e) => {
            auditar(
                &state.db, &ag, "files.read", "leer archivo", &path, false, "error", &e,
            );
            Err(e)
        }
    }
}

/// Escribe un archivo. Requiere: política del agente + aprobación explícita + ruta
/// dentro de la allowlist y no marcada como solo lectura.
#[tauri::command]
fn files_write(
    state: State<'_, AppState>,
    path: String,
    content: String,
    approved: bool,
    agent: Option<String>,
) -> Result<(), String> {
    let ag = agente(&agent);
    let (politica, proyecto) = politica_y_proyecto(&state, &ag)?;
    let chk = permissions::check_scoped(
        &politica,
        proyecto.as_ref(),
        permissions::Tool::Write,
        &path,
        approved,
    );
    if !chk.allowed {
        auditar(
            &state.db,
            &ag,
            "files.write",
            "escribir archivo",
            &path,
            approved,
            "denegado",
            &chk.reason,
        );
        logs::append("warn", &format!("escritura denegada por política: {path}"));
        return Err(chk.reason);
    }

    // Backup si el archivo ya existe. Si el backup no se puede verificar,
    // la escritura se cancela: nunca se sobrescribe sin copia válida.
    if std::path::Path::new(&path).exists() {
        let dir = config::data_dir().join("backups");
        if let Err(e) = std::fs::create_dir_all(&dir) {
            let msg = format!("No se pudo crear el directorio de backups: {e}");
            auditar(
                &state.db,
                &ag,
                "files.write",
                "escribir archivo",
                &path,
                approved,
                "abortado",
                &msg,
            );
            return Err(msg);
        }
        let stamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let name = std::path::Path::new(&path)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "archivo".into());
        let backup = dir.join(format!("{name}.{stamp}.bak"));
        if let Err(e) = std::fs::copy(&path, &backup) {
            let msg = format!("Backup fallido, escritura cancelada: {e}");
            auditar(
                &state.db,
                &ag,
                "files.write",
                "escribir archivo",
                &path,
                approved,
                "abortado",
                &msg,
            );
            return Err(msg);
        }
        let origen = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let copia = std::fs::metadata(&backup).map(|m| m.len()).unwrap_or(0);
        if origen != copia {
            let msg = format!("Backup incompleto ({origen} != {copia} bytes), escritura cancelada");
            auditar(
                &state.db,
                &ag,
                "files.write",
                "escribir archivo",
                &path,
                approved,
                "abortado",
                &msg,
            );
            return Err(msg);
        }
        logs::append("info", &format!("backup verificado: {}", backup.display()));
    }

    if let Err(e) = std::fs::write(&path, &content) {
        auditar(
            &state.db,
            &ag,
            "files.write",
            "escribir archivo",
            &path,
            approved,
            "error",
            &e.to_string(),
        );
        return Err(e.to_string());
    }

    auditar(
        &state.db,
        &ag,
        "files.write",
        "escribir archivo",
        &path,
        approved,
        &format!("ok ({} bytes)", content.len()),
        "",
    );
    let _ = guard(&state.db).approval_log("files.write", &path, true);
    logs::append("warn", &format!("escritura aprobada: {path}"));
    Ok(())
}

/* ------------------------- Terminal ------------------------- */

#[tauri::command]
async fn run_command(
    state: State<'_, AppState>,
    command: String,
    cwd: String,
    approved: bool,
    agent: Option<String>,
    timeout_secs: Option<u64>,
) -> Result<security::RunResult, String> {
    let ag = agente(&agent);
    let (politica, proyecto) = politica_y_proyecto(&state, &ag)?;

    let chk = permissions::check_scoped(
        &politica,
        proyecto.as_ref(),
        permissions::Tool::Shell,
        &command,
        approved,
    );
    if !chk.allowed {
        auditar(
            &state.db,
            &ag,
            "terminal.run",
            "ejecutar comando",
            &format!("{cwd} > {command}"),
            approved,
            "denegado",
            &chk.reason,
        );
        return Err(chk.reason);
    }
    let alcance = permissions::check_cwd_in_scope_scoped(&politica, proyecto.as_ref(), &cwd);
    if !alcance.allowed {
        auditar(
            &state.db,
            &ag,
            "terminal.run",
            "ejecutar comando",
            &format!("{cwd} > {command}"),
            approved,
            "denegado",
            &alcance.reason,
        );
        return Err(alcance.reason);
    }

    let limite = timeout_secs.unwrap_or(security::DEFAULT_TIMEOUT_SECS);
    let res = security::run_controlled(&command, &cwd, approved, limite, None, |_| {}).await;

    let _ = guard(&state.db).approval_log(
        "terminal.run",
        &format!("{cwd} > {command}"),
        approved,
    );
    match &res {
        Ok(r) => {
            let estado = if r.timed_out {
                "tiempo agotado"
            } else if r.cancelled {
                "cancelado"
            } else if r.code == 0 {
                "ok"
            } else {
                "error"
            };
            auditar(
                &state.db,
                &ag,
                "terminal.run",
                "ejecutar comando",
                &format!("{cwd} > {command}"),
                approved,
                &format!("{estado} code={} {} ms", r.code, r.duration_ms),
                "",
            );
            logs::append(
                "info",
                &format!(
                    "comando: {command} → {estado} (code {}, {} ms, pid {})",
                    r.code, r.duration_ms, r.pid
                ),
            );
        }
        Err(e) => {
            auditar(
                &state.db,
                &ag,
                "terminal.run",
                "ejecutar comando",
                &format!("{cwd} > {command}"),
                approved,
                "error",
                e,
            );
            logs::append("error", &format!("comando: {command} → {e}"));
        }
    }
    res
}

/// Lanza un comando y devuelve el id **inmediatamente**, sin bloquear: así la
/// interfaz puede cancelarlo. El resultado llega por `on_event`.
#[tauri::command]
async fn run_command_stream(
    state: State<'_, AppState>,
    command: String,
    cwd: String,
    approved: bool,
    agent: Option<String>,
    timeout_secs: Option<u64>,
    on_event: Channel<StreamEvent>,
) -> Result<String, String> {
    let ag = agente(&agent);
    let (politica, proyecto) = politica_y_proyecto(&state, &ag)?;

    let chk = permissions::check_scoped(
        &politica,
        proyecto.as_ref(),
        permissions::Tool::Shell,
        &command,
        approved,
    );
    if !chk.allowed {
        auditar(
            &state.db,
            &ag,
            "terminal.run",
            "ejecutar comando",
            &format!("{cwd} > {command}"),
            approved,
            "denegado",
            &chk.reason,
        );
        return Err(chk.reason);
    }
    let alcance = permissions::check_cwd_in_scope_scoped(&politica, proyecto.as_ref(), &cwd);
    if !alcance.allowed {
        auditar(
            &state.db,
            &ag,
            "terminal.run",
            "ejecutar comando",
            &format!("{cwd} > {command}"),
            approved,
            "denegado",
            &alcance.reason,
        );
        return Err(alcance.reason);
    }

    let id = format!(
        "cmd-{}-{}",
        chrono::Utc::now().timestamp_millis(),
        guard(&state.running).len() + 1
    );
    let (tx_cancel, rx_cancel) = oneshot::channel::<()>();

    {
        let mut mapa = guard(&state.running);
        mapa.insert(
            id.clone(),
            RunningCmd {
                pid: 0,
                command: command.clone(),
                cwd: cwd.clone(),
                started_at: chrono::Utc::now().to_rfc3339(),
                cancel: Some(tx_cancel),
            },
        );
    }

    let running = state.running.clone();
    let db = state.db.clone();
    let id_tarea = id.clone();
    let canal = on_event.clone();
    let comando = command.clone();
    let directorio = cwd.clone();
    let limite = timeout_secs.unwrap_or(security::DEFAULT_TIMEOUT_SECS);

    tauri::async_runtime::spawn(async move {
        let canal_inicio = canal.clone();
        let running_inicio = running.clone();
        let id_pid = id_tarea.clone();
        let chk_ok = security::run_controlled(
            &comando,
            &directorio,
            true,
            limite,
            Some(rx_cancel),
            move |pid| {
                if let Some(entrada) = guard(&running_inicio).get_mut(&id_pid) {
                    entrada.pid = pid;
                }
                let _ = canal_inicio.send(StreamEvent::Started {
                    id: id_pid.clone(),
                    pid,
                });
            },
        )
        .await;

        {
            let mut mapa = guard(&running);
            mapa.remove(&id_tarea);
        }

        match chk_ok {
            Ok(r) => {
                let estado = if r.timed_out {
                    "tiempo agotado"
                } else if r.cancelled {
                    "cancelado"
                } else if r.code == 0 {
                    "ok"
                } else {
                    "error"
                };
                auditar(
                    &db,
                    &ag,
                    "terminal.run",
                    "ejecutar comando",
                    &format!("{directorio} > {comando}"),
                    true,
                    &format!("{estado} code={} {} ms", r.code, r.duration_ms),
                    "",
                );
                let _ = guard(&db).approval_log(
                    "terminal.run",
                    &format!("{directorio} > {comando}"),
                    true,
                );
                logs::append(
                    "info",
                    &format!(
                        "comando: {comando} → {estado} (code {}, {} ms, pid {})",
                        r.code, r.duration_ms, r.pid
                    ),
                );
                let _ = canal.send(StreamEvent::Done {
                    id: id_tarea.clone(),
                    result: r,
                });
            }
            Err(e) => {
                auditar(
                    &db,
                    &ag,
                    "terminal.run",
                    "ejecutar comando",
                    &format!("{directorio} > {comando}"),
                    true,
                    "error",
                    &e,
                );
                logs::append("error", &format!("comando: {comando} → {e}"));
                let _ = canal.send(StreamEvent::Failed {
                    id: id_tarea.clone(),
                    message: e,
                });
            }
        }
    });

    Ok(id)
}

/// Cancela un comando en ejecución por su id. Devuelve `true` si se envió la señal.
#[tauri::command]
fn command_cancel(state: State<'_, AppState>, id: String) -> Result<bool, String> {
    let mut mapa = guard(&state.running);
    match mapa.get_mut(&id) {
        Some(entrada) => {
            let enviado = entrada
                .cancel
                .take()
                .map(|tx| tx.send(()).is_ok())
                .unwrap_or(false);
            Ok(enviado)
        }
        None => Err(format!("no hay ningún comando en ejecución con id {id}")),
    }
}

#[tauri::command]
fn commands_running(state: State<'_, AppState>) -> Vec<RunningInfo> {
    guard(&state.running)
        .iter()
        .map(|(id, c)| RunningInfo {
            id: id.clone(),
            pid: c.pid,
            command: c.command.clone(),
            cwd: c.cwd.clone(),
            started_at: c.started_at.clone(),
        })
        .collect()
}

/* ------------------------- Permisos (Fase 3) ------------------------- */

#[tauri::command]
fn policy_list(state: State<'_, AppState>) -> Result<Vec<AgentPolicy>, String> {
    guard(&state.db).policies_list()
}

#[tauri::command]
fn policy_get(state: State<'_, AppState>, agent: String) -> Result<AgentPolicy, String> {
    guard(&state.db).policy_get(&agent)
}

/// Guarda una política. Exige aprobación explícita (un clic del usuario) y queda auditado.
/// Los agentes no pueden cambiar su propia política: solo la interfaz, con aprobación.
#[tauri::command]
fn policy_save(
    state: State<'_, AppState>,
    policy: AgentPolicy,
    approved: bool,
) -> Result<AgentPolicy, String> {
    if !approved {
        return Err(
            "Cambio de permisos bloqueado: falta aprobación explícita del usuario".to_string(),
        );
    }
    if policy.agent.trim().is_empty() {
        return Err("La política necesita un agente".to_string());
    }
    let antes = guard(&state.db).policy_get(&policy.agent)?;
    guard(&state.db).policy_set(&policy)?;
    let guardado = guard(&state.db).policy_get(&policy.agent)?;
    auditar(
        &state.db,
        "usuario",
        "policy.save",
        "cambiar permisos",
        &format!(
            "{} → write={} shell={} rutas_escritura={} solo_lectura={}",
            policy.agent,
            guardado.allow_write,
            guardado.allow_shell,
            guardado.write_paths,
            guardado.read_only_paths
        ),
        approved,
        &format!(
            "ok (antes: write={} shell={})",
            antes.allow_write, antes.allow_shell
        ),
        "",
    );
    let _ = guard(&state.db).approval_log("policy.save", &policy.agent, true);
    logs::append(
        "warn",
        &format!("permisos cambiados para {}", policy.agent),
    );
    Ok(guardado)
}

/* ------------------------- Auditoría (Fase 3) ------------------------- */

#[tauri::command]
fn audit_list(
    state: State<'_, AppState>,
    query: Option<String>,
    tool: Option<String>,
    approved: Option<bool>,
    limit: Option<usize>,
) -> Result<Vec<AuditRow>, String> {
    guard(&state.db).audit_search(
        &query.unwrap_or_default(),
        &tool.unwrap_or_default(),
        approved,
        limit.unwrap_or(300),
    )
}

#[tauri::command]
fn audit_export(
    state: State<'_, AppState>,
    path: String,
    approved: bool,
    agent: Option<String>,
) -> Result<usize, String> {
    let ag = agente(&agent);
    let (politica, proyecto) = politica_y_proyecto(&state, &ag)?;
    let chk = permissions::check_scoped(
        &politica,
        proyecto.as_ref(),
        permissions::Tool::Write,
        &path,
        approved,
    );
    if !chk.allowed {
        auditar(
            &state.db,
            &ag,
            "audit.export",
            "exportar auditoría",
            &path,
            approved,
            "denegado",
            &chk.reason,
        );
        return Err(chk.reason);
    }
    let filas = guard(&state.db).audit_export_csv(&path)?;
    auditar(
        &state.db,
        &ag,
        "audit.export",
        "exportar auditoría",
        &path,
        approved,
        &format!("ok ({filas} filas)"),
        "",
    );
    Ok(filas)
}

/* ------------------------- Benchmarks ------------------------- */

#[tauri::command]
async fn benchmark_run(
    state: State<'_, AppState>,
    provider: String,
    model: String,
    prompt: String,
) -> Result<kukia_core::memory::BenchmarkResult, String> {
    let cfg = guard(&state.config).clone();
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
    guard(&state.db).benchmark_add(
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
    guard(&state.db).benchmarks_list()
}

#[tauri::command]
fn approvals_list(
    state: State<'_, AppState>,
) -> Result<Vec<kukia_core::memory::ApprovalRow>, String> {
    guard(&state.db).approvals_list()
}

/* ------------------------- Radar / compatibilidad ------------------------- */

#[tauri::command]
async fn radar_check(state: State<'_, AppState>) -> Result<Vec<radar::UpdateInfo>, String> {
    let list = radar::check().await;
    {
        let db = guard(&state.db);
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
    guard(&state.db).compat_add(&engine, &model, &arch, &quant, &gpu, &method, &notes)
}

#[tauri::command]
fn compat_list(
    state: State<'_, AppState>,
) -> Result<Vec<kukia_core::memory::CompatRow>, String> {
    guard(&state.db).compat_list()
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
        let db = guard(&state.db);
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
    guard(&state.db).research_list(300)
}

#[tauri::command]
fn research_items(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<research::Discovery>, String> {
    guard(&state.db).research_list(limit.unwrap_or(300))
}

#[tauri::command]
fn research_alerts(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<kukia_core::memory::ResearchAlertRow>, String> {
    guard(&state.db).research_alerts_list(limit.unwrap_or(100))
}

#[tauri::command]
fn research_report(state: State<'_, AppState>) -> Result<String, String> {
    let items = guard(&state.db).research_list(300)?;
    let alerts = guard(&state.db).research_alerts_list(100)?;
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

/* ------------------------- FASE 3A: PERMISOS POR PROYECTO ------------------------- */

/// Política del agente + proyecto activo. Las dos se exigen a la vez.
fn politica_y_proyecto(
    state: &State<'_, AppState>,
    agent: &str,
) -> Result<(AgentPolicy, Option<Project>), String> {
    let db = guard(&state.db);
    let politica = db.policy_get(agent)?;
    let activo = config::load_config().active_project.unwrap_or_default();
    let proyecto = if activo.trim().is_empty() {
        None
    } else {
        db.project_get(activo.trim())?
    };
    Ok((politica, proyecto))
}

#[tauri::command]
fn projects_list(state: State<'_, AppState>) -> Result<Vec<Project>, String> {
    guard(&state.db).projects_list()
}

#[tauri::command]
fn project_create(
    state: State<'_, AppState>,
    name: String,
    root: String,
    allow_shell: Option<bool>,
) -> Result<Project, String> {
    if name.trim().is_empty() {
        return Err("El proyecto necesita un nombre".into());
    }
    if root.trim().is_empty() {
        return Err("El proyecto necesita una raíz".into());
    }
    let ruta = std::path::Path::new(root.trim());
    if !ruta.is_dir() {
        return Err(format!(
            "La raíz no existe o no es una carpeta: {}. Créala antes de dar de alta el proyecto.",
            ruta.display()
        ));
    }
    let mut p = Project::new_project(&name, &root);
    p.allow_shell = allow_shell.unwrap_or(false);
    let nuevo = guard(&state.db).project_upsert(&p)?;
    logs::append(
        "info",
        &format!("proyecto {}: {} ({})", if nuevo { "creado" } else { "actualizado" }, p.name, p.root),
    );
    auditar(
        &state.db,
        AGENTE_POR_DEFECTO,
        "projects.create",
        if nuevo { "crear proyecto" } else { "actualizar proyecto" },
        &format!("{} · {}", p.name, p.root),
        true,
        "ok",
        "",
    );
    Ok(p)
}

#[tauri::command]
fn project_update(
    state: State<'_, AppState>,
    project: Project,
    approved: bool,
) -> Result<Project, String> {
    if !approved {
        return Err("Cambio de ámbito bloqueado: falta aprobación explícita".into());
    }
    if project.id.trim().is_empty() || project.name.trim().is_empty() {
        return Err("Proyecto inválido: faltan id o nombre".into());
    }
    guard(&state.db).project_upsert(&project)?;
    auditar(
        &state.db,
        AGENTE_POR_DEFECTO,
        "projects.update",
        "cambiar ámbito del proyecto",
        &format!("{} · escritura={} · shell={}", project.name, project.write_paths, project.allow_shell),
        true,
        "ok",
        "",
    );
    guard(&state.db)
        .project_get(&project.id)?
        .ok_or_else(|| "el proyecto desapareció al guardarlo".to_string())
}

#[tauri::command]
fn project_activate(
    state: State<'_, AppState>,
    id: Option<String>,
) -> Result<config::AppConfig, String> {
    let activo = id.unwrap_or_default();
    let activo = activo.trim().to_string();
    if !activo.is_empty() {
        let p = guard(&state.db)
            .project_get(&activo)?
            .ok_or_else(|| format!("no existe el proyecto {activo}"))?;
        if !std::path::Path::new(&p.root).is_dir() {
            return Err(format!("la raíz del proyecto ya no existe: {}", p.root));
        }
        logs::append("info", &format!("proyecto activo: {}", p.name));
    } else {
        logs::append("info", "proyecto activo: ninguno (ámbito global del agente)");
    }
    let mut cfg = config::load_config();
    cfg.active_project = if activo.is_empty() {
        None
    } else {
        Some(activo.clone())
    };
    let guardado = config::save_config(&cfg)?;
    if let Ok(mut en_memoria) = state.config.lock() {
        *en_memoria = guardado.clone();
    }
    auditar(
        &state.db,
        AGENTE_POR_DEFECTO,
        "projects.activate",
        "cambiar proyecto activo",
        if activo.is_empty() { "ninguno" } else { &activo },
        true,
        "ok",
        "",
    );
    Ok(guardado)
}

#[tauri::command]
fn project_active(state: State<'_, AppState>) -> Result<Option<Project>, String> {
    let activo = config::load_config().active_project.unwrap_or_default();
    if activo.trim().is_empty() {
        return Ok(None);
    }
    guard(&state.db).project_get(activo.trim())
}

/* ------------------------- GUARDIA DE VRAM ------------------------- */

#[tauri::command]
async fn vram_report() -> vram::VramReport {
    vram::report().await
}

#[tauri::command]
fn vram_fits(size_gb: f64, gpu: u32) -> Option<(bool, String)> {
    vram::fits(size_gb, gpu)
}

/* ------------------------- KUKIA FUTURE GAMING RADAR ------------------------- */

#[derive(Serialize)]
struct GamingScan {
    encontrados: usize,
    nuevos: usize,
    alertas: usize,
    informe: String,
}

#[tauri::command]
async fn gaming_scan(state: State<'_, AppState>) -> Result<GamingScan, String> {
    let found = gaming::scan_all().await;
    let alertas = gaming::alerts_for(&found);
    let mut nuevos = 0usize;
    {
        let db = guard(&state.db);
        for g in &found {
            if db.gaming_upsert(g).unwrap_or(false) {
                nuevos += 1;
            }
        }
        for a in &alertas {
            let _ = db.gaming_alert_add(&a.item_id, &a.level, &a.message);
        }
        let _ = db.gaming_run_add("manual", found.len() as i64, nuevos as i64);
    }
    let informe = gaming::write_report(&found, &alertas, false)
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    logs::append(
        "info",
        &format!(
            "gaming radar: {} encontrados, {nuevos} nuevos, {} alertas",
            found.len(),
            alertas.len()
        ),
    );
    Ok(GamingScan {
        encontrados: found.len(),
        nuevos,
        alertas: alertas.len(),
        informe,
    })
}

#[tauri::command]
fn gaming_items(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<GamingItem>, String> {
    guard(&state.db).gaming_list(limit.unwrap_or(200))
}

#[tauri::command]
fn gaming_alerts(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<GamingAlertRow>, String> {
    guard(&state.db).gaming_alerts_list(limit.unwrap_or(100))
}

#[tauri::command]
fn gaming_report(state: State<'_, AppState>) -> Result<String, String> {
    let items = guard(&state.db).gaming_list(200)?;
    let alerts: Vec<gaming::GamingAlert> = guard(&state.db)
        .gaming_alerts_list(100)?
        .into_iter()
        .map(|a| gaming::GamingAlert {
            item_id: a.item_id,
            level: a.level,
            message: a.message,
        })
        .collect();
    let path = gaming::write_report(&items, &alerts, false)?;
    Ok(path.display().to_string())
}

#[tauri::command]
fn gaming_lab_path() -> String {
    gaming::ensure_lab()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| gaming::lab_dir().display().to_string())
}

/* ------------------------- KUKIA ANIME ↔ REAL STUDIO ------------------------- */

fn base_comfy(base: Option<String>) -> String {
    base.unwrap_or_else(|| kukia_core::anime_real::COMFY_DEFAULT.to_string())
}

#[tauri::command]
async fn anime_status(base: Option<String>) -> kukia_core::anime_real::ComfyStatus {
    kukia_core::anime_real::status(&base_comfy(base)).await
}

#[tauri::command]
fn anime_templates() -> Result<Vec<kukia_core::anime_real::Template>, String> {
    kukia_core::anime_real::load_templates()
}

#[tauri::command]
async fn anime_capabilities(base: Option<String>) -> Vec<kukia_core::anime_real::Capability> {
    kukia_core::anime_real::capabilities(&base_comfy(base)).await
}

#[tauri::command]
async fn anime_inputs(base: Option<String>) -> Vec<String> {
    kukia_core::anime_real::imagenes_entrada(&base_comfy(base)).await
}

#[tauri::command]
fn anime_output_path() -> String {
    kukia_core::anime_real::output_dir().display().to_string()
}

#[tauri::command]
fn anime_history(limit: Option<usize>) -> Vec<serde_json::Value> {
    kukia_core::anime_real::historial(limit.unwrap_or(50))
}

#[tauri::command]
async fn anime_convert(
    state: State<'_, AppState>,
    base: Option<String>,
    req: kukia_core::anime_real::ConvertRequest,
) -> Result<kukia_core::anime_real::ConversionResult, String> {
    let url = base_comfy(base);
    let plantilla = req.template_id.clone();
    let imagen = req.image.clone();
    let r = kukia_core::anime_real::convertir(&url, &req).await?;
    logs::append(
        "info",
        &format!(
            "anime-real: {plantilla} · {imagen} · {} archivo(s) en {:.1}s",
            r.archivos.len(),
            r.segundos
        ),
    );
    if let Ok(db) = state.db.lock() {
        let _ = db.audit_log(
            "studio",
            "anime_real.convert",
            "convertir",
            &format!("{plantilla} ← {imagen}"),
            true,
            &format!("{:.1}s · {} archivo(s)", r.segundos, r.archivos.len()),
            "",
        );
    }
    Ok(r)
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
            db: Arc::new(Mutex::new(db)),
            config: Mutex::new(cfg),
            running: Arc::new(Mutex::new(HashMap::new())),
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

            // VRAM GUARD DE FONDO — el aviso de competencia salta solo, sin abrir la vista.
            // Solo lectura: informa por evento y por log; no cierra procesos ni descarga modelos.
            let handle_vram = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut firma: Option<String> = None;
                loop {
                    let rep = vram::report().await;
                    let alerta = vram::evaluar_competencia(&rep.engines);
                    let nueva = if alerta.competing {
                        Some(alerta.detalle.join("|"))
                    } else {
                        None
                    };
                    if nueva != firma {
                        match &nueva {
                            Some(_) => {
                                logs::append("warn", &format!("vram guard: {}", alerta.message));
                                let _ = handle_vram.emit("vram://competencia", &alerta);
                            }
                            None => {
                                if firma.is_some() {
                                    logs::append("info", "vram guard: competencia resuelta");
                                    let _ = handle_vram.emit("vram://competencia_fin", ());
                                }
                            }
                        }
                        firma = nueva;
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(20)).await;
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
            run_command_stream,
            command_cancel,
            commands_running,
            policy_list,
            policy_get,
            policy_save,
            audit_list,
            audit_export,
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
            anime_status,
            anime_templates,
            anime_capabilities,
            anime_inputs,
            anime_output_path,
            anime_history,
            anime_convert,
            projects_list,
            project_create,
            project_update,
            project_activate,
            project_active,
            vram_report,
            vram_fits,
            gaming_scan,
            gaming_items,
            gaming_alerts,
            gaming_report,
            gaming_lab_path,
            logs_tail
        ])
        .run(tauri::generate_context!())
        .expect("error ejecutando KUKIA CODE");
}
