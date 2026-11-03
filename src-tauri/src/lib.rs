use kukia_core::{
    agents, bridge, catalog, config, control, ecosystem, engines, entrenador, gaming, gpu, hub, jobs,
    lab, logs, mcp, registry, serve, tools, xoxete,
    memory::{AgentPolicy, AuditRow, Database, GamingAlertRow, GamingItem, Project},
    permissions, providers, radar, research, security, vram,
};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
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
    /// Parada global de la cola: si está activa, no se reclaman trabajos nuevos.
    halt_jobs: Arc<AtomicBool>,
    /// Cuántos workers están permitidos (1..=jobs::MAX_WORKERS).
    job_slots: Arc<AtomicUsize>,
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
    // La base se pasa al chat para ejecutar herramientas y dejar auditoría; el
    // candado se toma por consulta, así que no bloquea al resto de la interfaz.
    let db = state.db.clone();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<providers::ChatEvent>();
    let ch = on_event.clone();
    let forwarder = tauri::async_runtime::spawn(async move {
        while let Some(ev) = rx.recv().await {
            let _ = ch.send(ev);
        }
    });
    let res = providers::chat_stream_con(&request, &cfg, Some(&db), tx).await;
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
        // Un benchmark mide el modelo, no ejecuta herramientas.
        tools: false,
        aprobar: false,
        agent: None,
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

/// Expulsa un modelo concreto de la memoria del motor (botón de la vista VRAM).
#[tauri::command]
async fn vram_unload(engine: String, model: String, forzar: Option<bool>) -> vram::UnloadResult {
    let r = vram::unload(&engine, &model, forzar.unwrap_or(false)).await;
    logs::append(
        if r.ok { "info" } else { "warn" },
        &format!("vram unload {}/{}: {}", r.engine, r.model, r.message),
    );
    r
}

/* ------------- CENTRO DE CONTROL DE MOTORES (CC1–CC4) ------------- */

/// CC1: estado real de Ollama y LM Studio (versión, puerto, modelos, latencia).
#[tauri::command]
async fn engines_status() -> Vec<engines::EngineStatus> {
    engines::status_all().await
}

/// CC2: fichas de todos los modelos de un motor.
#[tauri::command]
async fn engine_cards(engine: String) -> Result<Vec<engines::ModelCard>, String> {
    engines::cards(&engine).await
}

/// CC2: ficha completa de un modelo concreto.
#[tauri::command]
async fn engine_card(engine: String, model: String) -> Result<engines::ModelCard, String> {
    engines::card(&engine, &model).await
}

/// CC3: parámetros guardados de un modelo.
#[tauri::command]
fn engine_params_get(engine: String, model: String) -> engines::InferenceParams {
    engines::params_get(&engine, &model)
}

/// CC3: guarda los parámetros y devuelve lo que el motor no soporta.
#[tauri::command]
fn engine_params_set(
    engine: String,
    model: String,
    params: engines::InferenceParams,
) -> Vec<String> {
    let avisos = engines::params_set(&engine, &model, params.clone());
    logs::append(
        "info",
        &format!(
            "parametros {}/{} guardados ({} aviso[s] de incompatibilidad)",
            engine,
            model,
            avisos.len()
        ),
    );
    avisos
}

/// CC4: estima el consumo antes de cargar (no toca la VRAM).
#[tauri::command]
async fn engine_preflight(engine: String, model: String) -> engines::Preflight {
    let p = engines::params_get(&engine, &model);
    engines::preflight(&engine, &model, &p).await
}

/// CC4: carga el modelo solo si el preflight lo autoriza.
#[tauri::command]
async fn engine_load(engine: String, model: String) -> vram::UnloadResult {
    let p = engines::params_get(&engine, &model);
    let pre = engines::preflight(&engine, &model, &p).await;
    if !pre.fits {
        logs::append(
            "warn",
            &format!("carga rechazada {}/{}: {}", engine, model, pre.detail),
        );
        return vram::UnloadResult {
            engine,
            model,
            ok: false,
            message: format!("El preflight no autoriza la carga. {}", pre.detail),
        };
    }
    let r = engines::load(&engine, &model, p).await;
    logs::append(
        if r.ok { "info" } else { "warn" },
        &format!("carga {}/{}: {}", r.engine, r.model, r.message),
    );
    r
}

/// Vacía la VRAM de todos los motores con modelos cargados.
/// Descarga una selección concreta de modelos (posiblemente de varios motores),
/// midiendo VRAM antes/después y verificando que siguen en disco.
#[tauri::command]
async fn vram_unload_seleccion(
    state: State<'_, AppState>,
    items: Vec<vram::Seleccion>,
) -> Result<vram::LoteResult, String> {
    if items.is_empty() {
        return Err("no has seleccionado ningún modelo".into());
    }
    let r = vram::descargar_lote(items.clone()).await;
    {
        let db = guard(&state.db);
        for it in &items {
            let res = r
                .resultados
                .iter()
                .find(|x| x.engine == it.engine && x.model == it.model);
            let (ok, nota) = match res {
                Some(x) => (x.ok, x.message.clone()),
                None => (false, "sin resultado del motor".to_string()),
            };
            let _ = vram::historial_add(&db, &it.engine, &it.model, ok, 0, &nota);
        }
        if r.liberado_mb > 0 {
            let _ = vram::historial_add(
                &db,
                "lote",
                &format!("{} modelo(s)", items.len()),
                true,
                r.liberado_mb,
                "VRAM medida recuperada",
            );
        }
    }
    logs::append(
        "info",
        &format!(
            "vram: {} modelo(s) descargados · {} MB recuperados · siguen cargados: {}",
            r.resultados.len(),
            r.liberado_mb,
            if r.siguen_cargados.is_empty() {
                "ninguno".to_string()
            } else {
                r.siguen_cargados.join(", ")
            }
        ),
    );
    Ok(r)
}

#[tauri::command]
fn vram_historial(state: State<'_, AppState>, limit: Option<usize>) -> Vec<vram::OpVram> {
    let db = guard(&state.db);
    vram::historial(&db, limit.unwrap_or(30))
}

#[tauri::command]
async fn vram_unload_all(forzar: Option<bool>) -> Vec<vram::UnloadResult> {
    let res = vram::unload_all(forzar.unwrap_or(false)).await;
    let ok = res.iter().filter(|r| r.ok).count();
    logs::append(
        "info",
        &format!("vram unload all: {} de {} descargados", ok, res.len()),
    );
    for r in &res {
        if !r.ok {
            logs::append(
                "warn",
                &format!("vram unload {}/{}: {}", r.engine, r.model, r.message),
            );
        }
    }
    res
}

/// Descarga todos los modelos de un motor, midiendo la VRAM antes y después.
#[tauri::command]
async fn vram_release_engine(engine: String, forzar: Option<bool>) -> vram::ReleaseReport {
    let r = vram::release_engine(&engine, forzar.unwrap_or(false)).await;
    logs::append(
        if r.ok { "info" } else { "warn" },
        &format!(
            "vram liberar motor {}: {} MB recuperados · {}",
            engine, r.freed_mb, r.note
        ),
    );
    r
}

/// Libera la VRAM de una GPU, midiendo lo recuperado en cada una.
#[tauri::command]
async fn vram_release_gpu(gpu: u32, forzar: Option<bool>) -> vram::ReleaseReport {
    let r = vram::release_gpu(gpu, forzar.unwrap_or(false)).await;
    logs::append(
        if r.ok { "info" } else { "warn" },
        &format!("vram liberar GPU{}: {} MB recuperados · {}", gpu, r.freed_mb, r.note),
    );
    r
}

/// Libera la VRAM de las dos GPU (todos los motores), midiendo el resultado.
#[tauri::command]
async fn vram_release_everything(forzar: Option<bool>) -> vram::ReleaseReport {
    let r = vram::release_everything(forzar.unwrap_or(false)).await;
    logs::append(
        if r.ok { "info" } else { "warn" },
        &format!("vram liberar ambas GPU: {} MB recuperados · {}", r.freed_mb, r.note),
    );
    r
}

/* ---------- Panel avanzado de modelo (contexto, KV, GPU, perfiles) ---------- */

/// Contexto, tabla de KV, parámetros con su estado y uso de GPU de un modelo.
#[tauri::command]
async fn control_capacidades(engine: String, model: String) -> control::Capacidades {
    control::capacidades(&engine, &model).await
}

/// Medidores en tiempo real (VRAM por GPU, RAM, CPU y modelos cargados).
#[tauri::command]
async fn control_metricas() -> control::Metricas {
    control::metricas().await
}

#[tauri::command]
fn perfiles_listar() -> Vec<control::Perfil> {
    control::perfiles_listar()
}

#[tauri::command]
fn perfil_guardar(perfil: control::Perfil) -> Result<Vec<control::Perfil>, String> {
    let r = control::perfil_guardar(perfil);
    if let Ok(lista) = &r {
        logs::append("info", &format!("perfil guardado ({} perfiles)", lista.len()));
    }
    r
}

#[tauri::command]
fn perfil_borrar(nombre: String) -> Result<Vec<control::Perfil>, String> {
    let r = control::perfil_borrar(&nombre);
    if r.is_ok() {
        logs::append("info", &format!("perfil borrado: {nombre}"));
    }
    r
}

/// Aplica un perfil guardando antes los parámetros que había (deshacible).
#[tauri::command]
fn perfil_aplicar(
    engine: String,
    model: String,
    nombre: String,
) -> Result<(engines::InferenceParams, Vec<String>), String> {
    let r = control::perfil_aplicar(&engine, &model, &nombre);
    if let Ok((_, avisos)) = &r {
        logs::append(
            "info",
            &format!(
                "perfil {nombre} aplicado a {engine}/{model} ({} aviso[s])",
                avisos.len()
            ),
        );
    }
    r
}

/// Devuelve los parámetros que había antes de aplicar el último perfil.
#[tauri::command]
fn perfil_restaurar_anterior(
    engine: String,
    model: String,
) -> Option<engines::InferenceParams> {
    let r = control::perfil_restaurar_anterior(&engine, &model);
    if r.is_some() {
        logs::append("info", &format!("parametros anteriores restaurados en {engine}/{model}"));
    }
    r
}

/* ---------------- COLA DE TRABAJOS (SQ1) ---------------- */

/// Bucle de un worker: reclama, ejecuta y anota el resultado.
///
/// Los workers **no cargan modelos**: miden el que esté cargado, liberan lo que el
/// usuario pidió liberar o escanean fuentes públicas. Así nunca expulsan el modelo
/// que estés usando para meter otro por detrás.
async fn worker_loop(
    idx: usize,
    db: Arc<Mutex<Database>>,
    stop: Arc<AtomicBool>,
    activos: Arc<AtomicUsize>,
) {
    let nombre = format!("worker-{}", idx + 1);
    loop {
        if stop.load(Ordering::Relaxed) {
            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
            continue;
        }
        if idx >= activos.load(Ordering::Relaxed).max(1) {
            tokio::time::sleep(std::time::Duration::from_millis(700)).await;
            continue;
        }

        let reclamado = {
            let g = guard(&db);
            jobs::claim_next(&g, &nombre)
        };
        let Some(job) = reclamado else {
            tokio::time::sleep(std::time::Duration::from_millis(800)).await;
            continue;
        };

        let inicio = std::time::Instant::now();
        let salida = match tokio::time::timeout(
            std::time::Duration::from_secs(1800),
            ejecutar_job(&job, &db, &stop),
        )
        .await
        {
            Ok(r) => r,
            Err(_) => Err("superó el límite de 30 minutos y se dio por fallido".to_string()),
        };

        let g = guard(&db);
        match salida {
            Ok(v) => {
                let _ = jobs::finish(&g, job.id, v);
                logs::append(
                    "info",
                    &format!(
                        "cola: {} #{} terminado en {} ms ({})",
                        job.kind,
                        job.id,
                        inicio.elapsed().as_millis(),
                        nombre
                    ),
                );
            }
            Err(e) => {
                let reintento = jobs::fail(&g, job.id, &e).unwrap_or(false);
                logs::append(
                    "warn",
                    &format!(
                        "cola: {} #{} falló: {}{}",
                        job.kind,
                        job.id,
                        e,
                        if reintento { " (vuelve a la cola)" } else { "" }
                    ),
                );
            }
        }
    }
}

/// Ejecuta un trabajo según su tipo. Devuelve el resultado que se guarda en la fila.
async fn ejecutar_job(
    job: &jobs::Job,
    db: &Arc<Mutex<Database>>,
    stop: &Arc<AtomicBool>,
) -> Result<serde_json::Value, String> {
    let payload = &job.payload;
    match job.kind.as_str() {
        "noop" => {
            let ms = payload["ms"].as_u64().unwrap_or(200).min(10_000);
            tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
            Ok(serde_json::json!({ "esperado_ms": ms, "nota": "trabajo de prueba" }))
        }

        "bench" => {
            let engine = payload["engine"].as_str().unwrap_or("ollama").to_string();
            let model = payload["model"]
                .as_str()
                .ok_or("falta el modelo en el trabajo de medición")?
                .to_string();
            let tokens = payload["tokens"].as_u64().unwrap_or(128).clamp(16, 2048);
            let prompt = payload["prompt"]
                .as_str()
                .map(|s| s.to_string())
                .unwrap_or_else(|| lab::PROMPT_PRUEBA.to_string());
            let params: engines::InferenceParams = payload
                .get("params")
                .cloned()
                .and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or_default();

            if stop.load(Ordering::Relaxed) {
                return Err("parada global activa antes de empezar".into());
            }
            let r = lab::benchmark(&engine, &model, &params, &prompt, tokens).await;
            let _ = lab::guardar_resultados(std::slice::from_ref(&r));
            if !r.ok {
                return Err(format!("la medición falló: {}", r.error));
            }
            Ok(serde_json::json!({
                "tok_s": r.tok_s,
                "prefill_tok_s": r.prefill_tok_s,
                "ttft_ms": r.ttft_ms,
                "tokens": r.tokens,
                "vram_pico": r.vram_pico,
                "ram_pico_gb": r.ram_pico_gb,
                "config": r.params_texto,
            }))
        }

        "vram_release" => {
            let modo = payload["target"].as_str().unwrap_or("all");
            let forzar = payload["forzar"].as_bool().unwrap_or(false);
            let informe = match modo {
                "engine" => {
                    let motor = payload["engine"]
                        .as_str()
                        .ok_or("falta el motor que se quiere liberar")?;
                    vram::release_engine(motor, forzar).await
                }
                "gpu" => {
                    let gpu = payload["gpu"].as_u64().unwrap_or(1) as u32;
                    vram::release_gpu(gpu, forzar).await
                }
                "all" => vram::release_everything(forzar).await,
                otro => return Err(format!("no sé liberar «{otro}»: usa engine, gpu o all")),
            };
            Ok(serde_json::json!({
                "target": informe.target,
                "freed_mb": informe.freed_mb,
                "ok": informe.ok,
                "note": informe.note,
                "gpus": informe.gpus,
            }))
        }

        "model_pull" => {
            let modelo = payload["model"]
                .as_str()
                .ok_or("falta el modelo que se quiere descargar")?
                .to_string();
            if stop.load(Ordering::Relaxed) {
                return Err("parada global activa antes de descargar".into());
            }
            hub::pull_ollama(db, job.id, &modelo).await
        }

        "research_auto" => {
            let objetivo = payload["objetivo"]
                .as_str()
                .ok_or("falta el objetivo de investigación")?
                .to_string();
            let limite = payload["limite"].as_u64().unwrap_or(10) as usize;
            research::validar_objetivo(&objetivo)?;
            let items = research::scan_all().await;
            if stop.load(Ordering::Relaxed) {
                return Err("parada global activa antes de guardar el informe".into());
            }
            let g = guard(db);
            let r = research::autonomous_con(&g, &objetivo, limite, items)?;
            Ok(serde_json::json!({
                "objetivo": r.objetivo,
                "relevantes": r.relevantes.len(),
                "nuevos": r.nuevos,
                "propuestas": r.propuestas.len(),
                "ruta": r.ruta,
            }))
        }

        "research_scan" => {
            let fuente = payload["source"].as_str().unwrap_or("all");
            let items = research::scan_one(fuente).await;
            let mut nuevos = 0usize;
            {
                let g = guard(db);
                for d in &items {
                    if g.research_upsert(d).unwrap_or(false) {
                        nuevos += 1;
                    }
                }
                for a in research::alerts_for(&items) {
                    let _ = g.research_alert_add(&a.item_id, &a.level, &a.message);
                }
                let _ = g.research_run_add(fuente, items.len(), nuevos);
            }
            Ok(serde_json::json!({
                "fuente": fuente,
                "encontrados": items.len(),
                "nuevos": nuevos,
                "titulos": items.iter().take(5).map(|d| d.title.clone()).collect::<Vec<_>>(),
            }))
        }

        otro => Err(format!("tipo de trabajo no implementado: {otro}")),
    }
}

/// Encola un trabajo. La interfaz usa esto para todo lo que puede tardar.
#[tauri::command]
fn jobs_enqueue(
    state: State<'_, AppState>,
    kind: String,
    payload: Option<serde_json::Value>,
    priority: Option<i64>,
) -> Result<i64, String> {
    let db = guard(&state.db);
    let id = jobs::enqueue(
        &db,
        &kind,
        payload.unwrap_or(serde_json::json!({})),
        priority.unwrap_or(0),
    )?;
    logs::append("info", &format!("cola: encolado {kind} #{id}"));
    Ok(id)
}

#[tauri::command]
fn jobs_list(state: State<'_, AppState>, limit: Option<i64>) -> Vec<jobs::Job> {
    let db = guard(&state.db);
    jobs::list(&db, limit.unwrap_or(100))
}

#[tauri::command]
fn jobs_stats(state: State<'_, AppState>) -> jobs::JobStats {
    let db = guard(&state.db);
    jobs::stats(&db)
}

#[tauri::command]
fn jobs_cancel(state: State<'_, AppState>, id: i64) -> Result<bool, String> {
    let db = guard(&state.db);
    jobs::cancel(&db, id)
}

#[tauri::command]
fn jobs_cancel_all(state: State<'_, AppState>) -> Result<usize, String> {
    let db = guard(&state.db);
    let n = jobs::cancel_all_queued(&db)?;
    logs::append("warn", &format!("cola: {n} trabajo(s) pendientes cancelados"));
    Ok(n)
}

#[tauri::command]
fn jobs_clear(state: State<'_, AppState>) -> Result<usize, String> {
    let db = guard(&state.db);
    jobs::clear_finished(&db)
}

/// Parada global: no se reclaman trabajos nuevos; el que esté en curso termina.
#[tauri::command]
fn jobs_stop(state: State<'_, AppState>) -> bool {
    state.halt_jobs.store(true, Ordering::Relaxed);
    logs::append("warn", "cola: PARADA GLOBAL activada");
    state.halt_jobs.load(Ordering::Relaxed)
}

#[tauri::command]
fn jobs_resume(state: State<'_, AppState>) -> bool {
    state.halt_jobs.store(false, Ordering::Relaxed);
    logs::append("info", "cola: reanudada");
    !state.halt_jobs.load(Ordering::Relaxed)
}

#[tauri::command]
fn jobs_estado(state: State<'_, AppState>) -> serde_json::Value {
    serde_json::json!({
        "parada": state.halt_jobs.load(Ordering::Relaxed),
        "activos": state.job_slots.load(Ordering::Relaxed),
        "max": jobs::MAX_WORKERS,
    })
}

#[tauri::command]
fn jobs_workers(state: State<'_, AppState>, workers: Option<usize>) -> usize {
    if let Some(n) = workers {
        let acotado = n.clamp(1, jobs::MAX_WORKERS);
        state.job_slots.store(acotado, Ordering::Relaxed);
        logs::append("info", &format!("cola: {acotado} worker(s) activos"));
    }
    state.job_slots.load(Ordering::Relaxed)
}

/* ---------------- CEREBRO UNIVERSAL DE MODELOS ---------------- */

/// Catálogo real: lo que cada motor enumera por su API. Lo que no enumera, se
/// declara como «sin API de catálogo» en vez de inventarse.
#[tauri::command]
async fn catalog_list(state: State<'_, AppState>) -> Result<catalog::CatalogReport, String> {
    let cfg = guard(&state.config).clone();
    let r = catalog::collect(&cfg).await;
    logs::append(
        "info",
        &format!(
            "catálogo universal: {} modelo(s) de {} motor(es) en {} ms",
            r.total,
            r.coverage.iter().filter(|c| c.reachable).count(),
            r.generated_ms
        ),
    );
    Ok(r)
}

/// Qué sabe hacer de verdad cada adaptador.
#[tauri::command]
fn catalog_adapters() -> Vec<catalog::AdapterInfo> {
    catalog::adapters()
}

/// Enrutado para una herramienta: capacidad, contexto mínimo y cloud autorizado o no.
#[tauri::command]
async fn catalog_route(
    state: State<'_, AppState>,
    need: catalog::Need,
) -> Result<catalog::Route, String> {
    let cfg = guard(&state.config).clone();
    let report = catalog::collect(&cfg).await;
    let db = guard(&state.db);
    let r = catalog::resolver_en(&report.models, &db, &need);
    drop(db);
    logs::append(
        "info",
        &format!(
            "router «{}»: {}",
            need.capability,
            r.chosen
                .as_ref()
                .map(|c| format!("{} ({:.0} pts)", c.key, c.score))
                .unwrap_or_else(|| "sin candidato".into())
        ),
    );
    Ok(r)
}

/// Fija el modelo de una herramienta o agente (se respeta en el enrutado).
#[tauri::command]
fn catalog_bind_set(
    state: State<'_, AppState>,
    scope: String,
    engine: String,
    model: String,
) -> Result<(), String> {
    let db = guard(&state.db);
    catalog::bind_set(&db, &scope, &engine, &model)?;
    drop(db);
    logs::append(
        "info",
        &format!("catálogo: «{scope}» fijado a {engine}::{model}"),
    );
    Ok(())
}

#[tauri::command]
fn catalog_binds(state: State<'_, AppState>) -> Vec<catalog::Binding> {
    let db = guard(&state.db);
    catalog::binds(&db)
}

#[tauri::command]
fn catalog_bind_clear(state: State<'_, AppState>, scope: String) -> Result<bool, String> {
    let db = guard(&state.db);
    catalog::bind_clear(&db, &scope)
}

/// Ruta completa: elige el modelo y le habla por su adaptador. El cloud solo si
/// `allow_cloud` viene en true; por defecto no.
#[tauri::command]
async fn catalog_chat(
    state: State<'_, AppState>,
    prompt: String,
    capability: Option<String>,
    model_key: Option<String>,
    max_tokens: Option<u32>,
    allow_cloud: Option<bool>,
) -> Result<catalog::UnifiedChat, String> {
    let cfg = guard(&state.config).clone();
    let report = catalog::collect(&cfg).await;

    let (cand, nota) = if let Some(clave) = model_key {
        let m = report
            .models
            .iter()
            .find(|m| m.key == clave)
            .ok_or_else(|| format!("«{clave}» no está en el catálogo ahora mismo"))?;
        if !m.can_chat {
            return Err(format!("«{clave}» no tiene adaptador de chat"));
        }
        (
            catalog::Candidate {
                key: m.key.clone(),
                engine: m.engine.clone(),
                id: m.id.clone(),
                kind: m.kind.clone(),
                score: 100.0,
                reasons: vec!["elegido a mano".into()],
            },
            "elegido a mano: no pasa por el router".to_string(),
        )
    } else {
        let need = catalog::Need {
            capability: capability.unwrap_or_else(|| "texto".into()),
            min_context: None,
            allow_cloud: allow_cloud.unwrap_or(false),
            prefer_loaded: true,
            scope: None,
        };
        let db = guard(&state.db);
        let elegido = catalog::elegir_para(&report.models, &db, &need);
        drop(db);
        elegido?
    };

    let mut r = catalog::hablar(&cfg, &cand, &prompt, max_tokens).await?;
    r.note = nota;
    logs::append(
        "info",
        &format!(
            "router → {}/{} · {} tok salida · {} ms",
            r.engine, r.model, r.completion_tokens, r.total_ms
        ),
    );
    Ok(r)
}

/* ---------------- MODEL HUB ---------------- */

/// Búsqueda en la librería pública de Ollama (HTML de ollama.com).
#[tauri::command]
async fn hub_buscar_ollama(q: String) -> Result<Vec<hub::OllamaHit>, String> {
    let r = hub::buscar_ollama(&q).await?;
    logs::append("info", &format!("hub: «{q}» → {} en Ollama", r.len()));
    Ok(r)
}

/// Búsqueda de GGUF en Hugging Face con su API pública.
#[tauri::command]
async fn hub_buscar_hf(q: String, limite: Option<usize>) -> Result<Vec<hub::HubModelo>, String> {
    let r = hub::buscar_hf(&q, limite.unwrap_or(20)).await?;
    logs::append("info", &format!("hub: «{q}» → {} en Hugging Face", r.len()));
    Ok(r)
}

/// Manifiesto real de un modelo de la librería de Ollama (tamaño y digest).
#[tauri::command]
async fn hub_manifest(modelo: String) -> Result<hub::OllamaTag, String> {
    hub::manifiesto_ollama(&modelo).await
}

/// GGUF que ya están en los discos de este equipo.
#[tauri::command]
fn hub_gguf(max: Option<usize>) -> Vec<hub::GgufLocal> {
    let raices = hub::raices_gguf();
    let r = hub::gguf_locales(&raices, max.unwrap_or(400));
    logs::append("info", &format!("hub: {} GGUF locales", r.len()));
    r
}

/// ¿Cabe este modelo aquí? Con la VRAM y la RAM de ahora mismo.
#[tauri::command]
fn hub_compat(params_b: f64, quant: String, ctx: Option<u64>) -> hub::Compatibilidad {
    let (ram, gpus) = hub::hardware_actual();
    hub::evaluar(params_b, &quant, ctx.unwrap_or(8192), ram, gpus)
}

/// Encola la descarga de un modelo de Ollama. El progreso real se ve en Tareas.
#[tauri::command]
fn hub_pull(state: State<'_, AppState>, modelo: String) -> Result<i64, String> {
    let db = guard(&state.db);
    let payload = serde_json::json!({ "model": modelo });
    let id = jobs::enqueue(&db, "model_pull", payload, 5)?;
    drop(db);
    logs::append("info", &format!("hub: descarga de «{modelo}» encolada (#{id})"));
    Ok(id)
}

/* ---------------- PUENTE BIDIRECCIONAL KUKIA CORE ↔ OPENCODE ---------------- */
//
// Dirección A: KUKIA lee el catálogo real de OpenCode y le pide respuestas por
// su API (`/api/session` + `/session/{id}/message`).
// Dirección B: `serve.rs` expone los modelos de KUKIA en `/v1` para que OpenCode
// los consuma. KUKIA nunca escribe en la configuración de OpenCode.

/// Proceso `opencode serve` que haya arrancado KUKIA (solo se para el nuestro).
static OPENCODE_HIJO: Mutex<Option<std::process::Child>> = Mutex::new(None);

const PUENTE_URL_POR_DEFECTO: &str = "http://127.0.0.1:4096";
const KUKIA_PORT_POR_DEFECTO: u16 = 8311;

fn puente_url(v: Option<String>) -> String {
    v.map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| PUENTE_URL_POR_DEFECTO.to_string())
}

/// Estado del puente: OpenCode instalado, su catálogo, sus agentes y si su
/// servidor responde. Todo se lee de la instalación real, sin modificar nada.
#[tauri::command]
async fn puente_estado(url: Option<String>) -> bridge::PuenteEstado {
    let u = puente_url(url);
    bridge::estado(Some(u)).await
}

/// Modelos de OpenCode. Con servidor vivo usa `GET /api/model` (más datos);
/// si no, cae a `opencode models`.
#[tauri::command]
async fn puente_modelos(url: Option<String>) -> Vec<bridge::ModeloPuente> {
    let u = puente_url(url);
    bridge::modelos(Some(&u)).await
}

#[tauri::command]
async fn puente_agentes(url: Option<String>) -> Vec<bridge::AgentePuente> {
    let u = puente_url(url);
    bridge::agentes(Some(&u)).await
}

/// Pide una respuesta a OpenCode. Crea sesión propia para no mezclar contextos.
#[tauri::command]
async fn puente_preguntar(
    url: Option<String>,
    provider: String,
    model: String,
    agent: Option<String>,
    texto: String,
    timeout: Option<u64>,
) -> Result<bridge::PuenteRespuesta, String> {
    let u = puente_url(url);
    if texto.trim().is_empty() {
        return Err("escribe un mensaje".into());
    }
    let r = bridge::preguntar(
        &u,
        &provider,
        &model,
        agent.as_deref(),
        &texto,
        timeout.unwrap_or(180),
    )
    .await?;
    logs::append(
        "info",
        &format!(
            "puente: opencode respondió con {} en {} ms (sesión {})",
            r.modelo, r.ms, r.session_id
        ),
    );
    Ok(r)
}

/// Arranca `opencode serve` como proceso hijo de KUKIA. Devuelve la URL lista.
#[tauri::command]
async fn puente_servidor_arrancar(port: Option<u16>) -> Result<String, String> {
    {
        let mut h = guard(&OPENCODE_HIJO);
        if let Some(c) = h.as_mut() {
            if matches!(c.try_wait(), Ok(None)) {
                return Ok(format!(
                    "Ya estaba arrancado por KUKIA (pid {}).",
                    c.id()
                ));
            }
        }
    }

    let bin = bridge::binario();
    let puerto = port.unwrap_or(4096);
    let mut cmd = if bin.to_lowercase().ends_with(".cmd") || bin.to_lowercase().ends_with(".bat") {
        let mut c = std::process::Command::new("cmd");
        c.arg("/C").arg(&bin);
        c
    } else {
        std::process::Command::new(&bin)
    };
    cmd.args(["serve", "--port", &puerto.to_string(), "--hostname", "127.0.0.1"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let child = cmd
        .spawn()
        .map_err(|e| format!("no se pudo arrancar «{bin} serve»: {e}"))?;
    let pid = child.id();
    *guard(&OPENCODE_HIJO) = Some(child);

    // Espera activa a que conteste, máximo 25 s.
    let url = format!("http://127.0.0.1:{puerto}");
    let cliente = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .map_err(|e| e.to_string())?;
    for _ in 0..25 {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        if let Ok(r) = cliente.get(format!("{url}/api/health")).send().await {
            if r.status().is_success() {
                logs::append("info", &format!("puente: opencode serve arriba en {url} (pid {pid})"));
                return Ok(url);
            }
        }
    }
    Err(format!(
        "«opencode serve» arrancó (pid {pid}) pero no respondió en {url} en 25 s."
    ))
}

/// Para el servidor de OpenCode **solo si lo arrancó KUKIA**.
#[tauri::command]
fn puente_servidor_parar() -> Result<String, String> {
    let mut h = guard(&OPENCODE_HIJO);
    match h.as_mut() {
        None => Ok("KUKIA no ha arrancado ningún servidor de OpenCode.".into()),
        Some(c) => {
            let pid = c.id();
            let _ = c.kill();
            let _ = c.wait();
            *h = None;
            logs::append("info", &format!("puente: servidor de OpenCode parado (pid {pid})"));
            Ok(format!("Servidor de OpenCode parado (pid {pid})."))
        }
    }
}

/// Servidor local compatible OpenAI con los modelos de KUKIA (para OpenCode).
#[tauri::command]
async fn kukia_server_start(
    state: State<'_, AppState>,
    port: Option<u16>,
    token: Option<String>,
) -> Result<serve::ServeStatus, String> {
    let cfg = guard(&state.config).clone();
    let tok = token
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            format!("kukia-{nanos:x}")
        });
    let st = serve::start(cfg, port.unwrap_or(KUKIA_PORT_POR_DEFECTO), tok).await?;
    logs::append(
        "info",
        &format!("puente: servidor KUKIA arriba en {} (solo loopback)", st.url),
    );
    Ok(st)
}

#[tauri::command]
fn kukia_server_status() -> serve::ServeStatus {
    serve::status()
}

#[tauri::command]
fn kukia_server_stop() -> bool {
    serve::stop();
    logs::append("info", "puente: servidor KUKIA detenido");
    true
}

/// Fragmento JSON para pegar en `opencode.json`. KUKIA no lo escribe solo.
#[tauri::command]
async fn puente_snippet(port: Option<u16>, token: Option<String>) -> String {
    let st = serve::status();
    let p = port.unwrap_or(if st.port > 0 { st.port } else { KUKIA_PORT_POR_DEFECTO });
    let tok = token
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .unwrap_or(st.token);
    let modelos: Vec<String> = serve::modelos_catalogo()
        .await
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    bridge::snippet_opencode(&format!("http://127.0.0.1:{p}"), &tok, &modelos)
}

/* ---------------- ECOSISTEMA IA: KUKIA + OPENCODE + FREEBUFF ---------------- */

/// Los tres nodos con sus capacidades comprobadas. Abre su propia conexión a la
/// base para no retener candados durante los `await`.
#[tauri::command]
async fn ecosistema_estado(
    state: State<'_, AppState>,
    opencode_url: Option<String>,
) -> Result<ecosystem::Ecosistema, String> {
    let cfg = guard(&state.config).clone();
    let data = config::data_dir().join("kukia.db");
    let url = opencode_url.filter(|u| !u.trim().is_empty());
    Ok(ecosystem::todo(&cfg, &data, url).await)
}

/// Recomienda el entorno adecuado para una tarea según lo que esté conectado.
#[tauri::command]
async fn ecosistema_enrutar(
    state: State<'_, AppState>,
    tarea: String,
    opencode_url: Option<String>,
) -> Result<ecosystem::Recomendacion, String> {
    let cfg = guard(&state.config).clone();
    let data = config::data_dir().join("kukia.db");
    let url = opencode_url.filter(|u| !u.trim().is_empty());
    let ec = ecosystem::todo(&cfg, &data, url).await;
    Ok(ecosystem::enrutar(&tarea, &ec.nodos))
}

/// Inventario universal: 18 sistemas sondeados en paralelo (proceso, puerto,
/// HTTP y ruta). No modifica ni arranca nada.
#[tauri::command]
async fn ecosistema_universal() -> Result<registry::Inventario, String> {
    Ok(registry::inventario().await)
}

/* ---------------- XOXETE: instalación real y memoria propia ---------------- */

/// Estado real de la instalación de Xoxete: config, agente, modelo,
/// instrucciones, plugins, binarios y motor de memoria. Solo lectura.
#[tauri::command]
async fn xoxete_estado() -> Result<xoxete::XoxeteEstado, String> {
    tauri::async_runtime::spawn_blocking(xoxete::estado)
        .await
        .map_err(|e| format!("tarea cancelada: {e}"))
}

/// Busca recuerdos en la base original de Xoxete (motor oficial, solo lectura).
#[tauri::command]
async fn xoxete_memorias(
    consulta: String,
    limite: Option<usize>,
) -> Result<Vec<xoxete::Memoria>, String> {
    let tope = limite.unwrap_or(10);
    tauri::async_runtime::spawn_blocking(move || xoxete::buscar(&consulta, tope))
        .await
        .map_err(|e| format!("tarea cancelada: {e}"))?
}

/// Guarda una experiencia en la misma base que usa Xoxete. Acción explícita:
/// la interfaz solo la lanza cuando el usuario lo pide.
#[tauri::command]
async fn xoxete_recordar(
    tipo: String,
    contexto: String,
    accion: String,
    resultado: String,
    evidencia: Option<String>,
) -> Result<String, String> {
    let ev = evidencia.unwrap_or_default();
    tauri::async_runtime::spawn_blocking(move || {
        xoxete::guardar(&tipo, &contexto, &accion, &resultado, &ev)
    })
    .await
    .map_err(|e| format!("tarea cancelada: {e}"))?
}

/* ---------------- MCP Y CENTRO DE HERRAMIENTAS ---------------- */

/// Servidores MCP declarados en los mcp.json, con su permiso y su estado real.
#[tauri::command]
fn mcp_servers(state: State<'_, AppState>) -> Vec<mcp::ServerInfo> {
    let db = guard(&state.db);
    mcp::servidores(&db)
}

/// Autoriza o revoca un servidor. Autorizar permite lanzarlo: es un proceso externo.
#[tauri::command]
fn mcp_grant(state: State<'_, AppState>, server: String, granted: bool) -> Result<bool, String> {
    let db = guard(&state.db);
    mcp::autorizar(&db, &server, granted)?;
    drop(db);
    logs::append(
        "warn",
        &format!(
            "mcp: «{server}» {}",
            if granted {
                "AUTORIZADO (podrá lanzarse como proceso)"
            } else {
                "revocado"
            }
        ),
    );
    Ok(granted)
}

/// Conecta con un servidor autorizado y guarda sus herramientas.
#[tauri::command]
async fn mcp_discover(
    state: State<'_, AppState>,
    server: String,
) -> Result<Vec<mcp::McpTool>, String> {
    let autorizado = {
        let db = guard(&state.db);
        mcp::esta_autorizado(&db, &server)
    };
    if !autorizado {
        return Err(format!(
            "«{server}» no está autorizado: autorízalo antes de conectarlo"
        ));
    }
    let lista = mcp::conectar(&server).await?;
    {
        let db = guard(&state.db);
        mcp::guardar_en(&db, &server, &lista)?;
    }
    logs::append(
        "info",
        &format!("mcp «{server}»: {} herramienta(s) descubierta(s)", lista.len()),
    );
    Ok(lista)
}

/// Ejecuta una herramienta MCP ya descubierta. Exige que su servidor esté autorizado.
#[tauri::command]
async fn mcp_call(
    state: State<'_, AppState>,
    server: String,
    tool: String,
    arguments: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let autorizado = {
        let db = guard(&state.db);
        mcp::esta_autorizado(&db, &server)
    };
    if !autorizado {
        return Err(format!(
            "«{server}» no está autorizado: no se ejecuta nada de un servidor sin permiso"
        ));
    }
    logs::append(
        "warn",
        &format!("mcp: ejecutando {server}::{tool}"),
    );
    mcp::llamar(&server, &tool, arguments).await
}

#[tauri::command]
fn mcp_tools(state: State<'_, AppState>, server: String) -> Vec<mcp::McpTool> {
    let db = guard(&state.db);
    mcp::tools_guardadas(&db, &server)
}

/// Registro central: núcleo + MCP, con estado comprobado donde se puede.
#[tauri::command]
fn tools_registry(state: State<'_, AppState>) -> Vec<tools::ToolEntry> {
    let db = guard(&state.db);
    tools::registro(&db)
}

#[tauri::command]
fn tools_resumen(state: State<'_, AppState>) -> tools::Resumen {
    let db = guard(&state.db);
    tools::resumen(&db)
}

/// Investigación autónoma: lee fuentes públicas y escribe informe. No instala nada.
#[tauri::command]
async fn research_auto(
    state: State<'_, AppState>,
    objetivo: String,
    limite: Option<usize>,
) -> Result<research::AutoInforme, String> {
    research::validar_objetivo(&objetivo)?;
    let items = research::scan_all().await;
    let informe = {
        let db = guard(&state.db);
        research::autonomous_con(&db, &objetivo, limite.unwrap_or(10), items)?
    };
    logs::append(
        "info",
        &format!(
            "investigación autónoma «{}»: {} relevante(s), {} nuevo(s) → {}",
            informe.objetivo,
            informe.relevantes.len(),
            informe.nuevos,
            informe.ruta
        ),
    );
    Ok(informe)
}

/* ---------------- LABORATORIO EXPERIMENTAL ---------------- */

/// Catálogo de técnicas con su nivel, mecanismo y estado real en esta máquina.
#[tauri::command]
fn lab_catalogo() -> Vec<lab::Experimento> {
    lab::catalogo()
}

/// Banco de pruebas: una generación controlada con métricas medidas.
#[tauri::command]
async fn lab_bench(
    engine: String,
    model: String,
    params: engines::InferenceParams,
    prompt: Option<String>,
    tokens: Option<u64>,
) -> lab::BenchResult {
    let texto = prompt.unwrap_or_else(|| lab::PROMPT_PRUEBA.to_string());
    let n = tokens.unwrap_or(128);
    let r = lab::benchmark(&engine, &model, &params, &texto, n).await;
    let _ = lab::guardar_resultados(std::slice::from_ref(&r));
    logs::append(
        if r.ok { "info" } else { "warn" },
        &format!(
            "lab bench {}/{}: {:.1} tok/s · {} MB pico · {}",
            engine,
            model,
            r.tok_s,
            r.vram_pico.iter().map(|g| g.pico_mb).max().unwrap_or(0),
            if r.ok { r.etiqueta.as_str() } else { r.error.as_str() }
        ),
    );
    r
}

/// A/B: misma petición con dos configuraciones del mismo modelo.
#[tauri::command]
async fn lab_ab(
    engine: String,
    model: String,
    a: engines::InferenceParams,
    b: engines::InferenceParams,
    prompt: Option<String>,
    tokens: Option<u64>,
) -> lab::Comparacion {
    let texto = prompt.unwrap_or_else(|| lab::PROMPT_PRUEBA.to_string());
    let n = tokens.unwrap_or(128);
    let c = lab::comparar(&engine, &model, &a, &b, &texto, n).await;
    let _ = lab::guardar_resultados(&[c.a.clone(), c.b.clone()]);
    logs::append(
        "info",
        &format!("lab A/B {}/{}: {}", engine, model, c.veredicto),
    );
    c
}

#[tauri::command]
fn lab_historial() -> Vec<lab::BenchResult> {
    lab::historial()
}

/// Activa un experimento guardando antes la configuración que había.
#[tauri::command]
fn lab_activar(
    experimento: String,
    engine: String,
    model: String,
) -> Result<(engines::InferenceParams, String), String> {
    let r = lab::activar(&experimento, &engine, &model);
    match &r {
        Ok((_, msg)) => logs::append("info", &format!("lab activar: {msg}")),
        Err(e) => logs::append("warn", &format!("lab activar rechazado: {e}")),
    }
    r
}

/// Devuelve la configuración que había antes de activar el experimento.
#[tauri::command]
fn lab_restaurar(engine: String, model: String) -> Option<engines::InferenceParams> {
    let r = lab::restaurar(&engine, &model);
    if r.is_some() {
        logs::append("info", &format!("lab restaurar: {engine}/{model}"));
    }
    r
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

/* ------------- KUKIA UNIVERSAL REALITY CONVERTER (RC1 + RC2) ------------- */

/// Registro de motores con diagnóstico real (ComfyUI, Ollama, Wan2GP, Maestro).
#[tauri::command]
async fn studio_motores() -> Vec<kukia_core::studio::Motor> {
    kukia_core::studio::engines::todos().await
}

/// VRAM libre por tarjeta y RAM disponible, sin tocar nada.
#[tauri::command]
fn studio_presupuesto() -> kukia_core::studio::Presupuesto {
    kukia_core::studio::presupuesto()
}

/// Catálogo de estilos tal cual está en el JSON (sin diagnosticar).
#[tauri::command]
fn studio_estilos() -> Result<Vec<kukia_core::studio::Estilo>, String> {
    kukia_core::studio::estilos()
}

/// Catálogo cruzado con lo que hay instalado y con la memoria libre.
#[tauri::command]
async fn studio_diagnostico() -> Result<Vec<kukia_core::studio::DiagnosticoEstilo>, String> {
    kukia_core::studio::diagnostico_estilos().await
}

/// Sondeo + estilo detectado + planos + veredicto, sin ejecutar difusión.
#[tauri::command]
async fn studio_preflight(
    ruta: String,
    destino: Option<String>,
    modelo: Option<String>,
    umbral: Option<f64>,
) -> Result<kukia_core::studio::Preflight, String> {
    kukia_core::studio::preflight(
        &ruta,
        destino.as_deref(),
        modelo.as_deref(),
        umbral.unwrap_or(0.30),
    )
    .await
}

/// Solo segmentación de planos (un plano = un job).
#[tauri::command]
async fn studio_planos(
    ruta: String,
    umbral: Option<f64>,
) -> Result<Vec<kukia_core::studio::Plano>, String> {
    kukia_core::studio::detect::planos(&ruta, umbral.unwrap_or(0.30)).await
}

/// Solo detección de estilo con VLM local, con confianza y texto crudo.
#[tauri::command]
async fn studio_detectar(
    ruta: String,
    modelo: Option<String>,
) -> Result<kukia_core::studio::EstiloDetectado, String> {
    kukia_core::studio::detect::detectar_estilo(&ruta, modelo.as_deref()).await
}

/// Recorta un plano a un archivo aparte, sin recomprimir.
#[tauri::command]
fn studio_cortar_plano(
    ruta: String,
    inicio: f64,
    segundos: f64,
    destino: String,
) -> Result<String, String> {
    kukia_core::studio::detect::partir_plano(&ruta, inicio, segundos, &destino)
}

/* ------------------------- Entrenador de LoRA (LA8) ------------------------- */

#[tauri::command]
fn entrenador_proyectos(state: State<'_, AppState>) -> Vec<entrenador::Proyecto> {
    entrenador::proyectos(&guard(&state.db))
}

#[tauri::command]
fn entrenador_proyecto_crear(
    state: State<'_, AppState>,
    nombre: String,
    clase: String,
    modelo_base: String,
) -> Result<entrenador::Proyecto, String> {
    entrenador::proyecto_crear(&guard(&state.db), &nombre, &clase, &modelo_base)
}

#[tauri::command]
fn entrenador_imagenes(state: State<'_, AppState>, proyecto: String) -> Vec<entrenador::Imagen> {
    entrenador::imagenes(&guard(&state.db), &proyecto)
}

#[tauri::command]
fn entrenador_importar_carpeta(
    state: State<'_, AppState>,
    proyecto: String,
    carpeta: String,
) -> Result<Vec<entrenador::Imagen>, String> {
    entrenador::importar_carpeta(&guard(&state.db), &proyecto, &carpeta)
}

#[tauri::command]
fn entrenador_caption(state: State<'_, AppState>, id: i64, texto: String) -> Result<(), String> {
    entrenador::caption(&guard(&state.db), id, &texto)
}

/// Describe una imagen con un modelo de visión local y la propone como caption.
/// No la guarda: la interfaz decide si aceptarla.
#[tauri::command]
async fn entrenador_caption_auto(
    state: State<'_, AppState>,
    id: i64,
    proyecto: String,
    modelo: String,
) -> Result<String, String> {
    let (archivo, actual) = {
        let db = guard(&state.db);
        let lista = entrenador::imagenes(&db, &proyecto);
        let img = lista
            .iter()
            .find(|i| i.id == id)
            .ok_or_else(|| format!("no hay imagen con id {id}"))?;
        (img.archivo.clone(), entrenador::caption_de(&db, id).unwrap_or_default())
    };
    let _ = actual;
    let ruta = entrenador::dir_dataset(&proyecto).join(archivo);
    entrenador::caption_auto(&ruta, &modelo).await
}

#[tauri::command]
fn entrenador_preflight(
    state: State<'_, AppState>,
    proyecto: String,
) -> Result<entrenador::Preflight, String> {
    entrenador::preflight(&guard(&state.db), &proyecto)
}

#[tauri::command]
fn entrenador_iniciar(
    state: State<'_, AppState>,
    cfg: entrenador::RunCfg,
) -> Result<entrenador::Run, String> {
    entrenador::entrenar(state.db.clone(), &cfg)
}

#[tauri::command]
fn entrenador_run(state: State<'_, AppState>, id: i64) -> Result<Option<entrenador::Run>, String> {
    entrenador::run(state.db.clone(), id)
}

#[tauri::command]
fn entrenador_runs(state: State<'_, AppState>, proyecto: String) -> Vec<entrenador::Run> {
    entrenador::runs(&guard(&state.db), &proyecto)
}

#[tauri::command]
fn entrenador_cancelar(state: State<'_, AppState>, id: i64) -> bool {
    entrenador::cancelar(state.db.clone(), id)
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

    // Nombres distintos a los de los comandos: dentro de `run()` el macro
    // `generate_handler!` los invoca por nombre y un `let` los taparía.
    let halt = Arc::new(AtomicBool::new(false));
    let slots = Arc::new(AtomicUsize::new(2));
    {
        // Lo que quedó «running» de la sesión anterior no lo está haciendo nadie.
        match jobs::requeue_stale(&db) {
            Ok(0) => {}
            Ok(n) => logs::append(
                "warn",
                &format!("cola: {n} trabajo(s) interrumpidos devueltos a la cola"),
            ),
            Err(e) => logs::append("warn", &format!("cola: no se pudo recuperar el estado: {e}")),
        }
    }
    let db_jobs = Arc::new(Mutex::new(db));
    let stop_spawn = halt.clone();
    let workers_spawn = slots.clone();
    let db_spawn = db_jobs.clone();

    tauri::Builder::default()
        .manage(AppState {
            db: db_jobs,
            config: Mutex::new(cfg),
            running: Arc::new(Mutex::new(HashMap::new())),
            halt_jobs: halt,
            job_slots: slots,
        })

        .setup(move |app| {
            // SQ1 — pool acotado de workers de la cola. Como máximo `MAX_WORKERS`
            // tareas, y solo están activas las que el usuario permita.
            for idx in 0..jobs::MAX_WORKERS {
                let db = db_spawn.clone();
                let stop = stop_spawn.clone();
                let activos = workers_spawn.clone();
                tauri::async_runtime::spawn(async move {
                    worker_loop(idx, db, stop, activos).await;
                });
            }

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
            studio_motores,
            studio_presupuesto,
            studio_estilos,
            studio_diagnostico,
            studio_preflight,
            studio_planos,
            studio_detectar,
            studio_cortar_plano,
            projects_list,
            project_create,
            project_update,
            project_activate,
            project_active,
            vram_report,
            vram_fits,
            vram_unload,
            vram_unload_seleccion,
            vram_historial,
            vram_unload_all,
            vram_release_engine,
            vram_release_gpu,
            vram_release_everything,
            control_capacidades,
            control_metricas,
            perfiles_listar,
            perfil_guardar,
            perfil_borrar,
            perfil_aplicar,
            perfil_restaurar_anterior,
            jobs_enqueue,
            jobs_list,
            jobs_stats,
            jobs_cancel,
            jobs_cancel_all,
            jobs_clear,
            jobs_stop,
            jobs_resume,
            jobs_estado,
            jobs_workers,
            catalog_list,
            catalog_adapters,
            catalog_route,
            catalog_bind_set,
            catalog_binds,
            catalog_bind_clear,
            catalog_chat,
            entrenador_proyectos,
            entrenador_proyecto_crear,
            entrenador_imagenes,
            entrenador_importar_carpeta,
            entrenador_caption,
            entrenador_caption_auto,
            entrenador_preflight,
            entrenador_iniciar,
            entrenador_run,
            entrenador_runs,
            entrenador_cancelar,
            mcp_servers,
            mcp_grant,
            mcp_discover,
            mcp_call,
            mcp_tools,
            tools_registry,
            tools_resumen,
            research_auto,
            hub_buscar_ollama,
            hub_buscar_hf,
            hub_manifest,
            hub_gguf,
            hub_compat,
            hub_pull,
            lab_catalogo,
            lab_bench,
            lab_ab,
            lab_historial,
            lab_activar,
            lab_restaurar,
            engines_status,
            engine_cards,
            engine_card,
            engine_params_get,
            engine_params_set,
            engine_preflight,
            engine_load,
            gaming_scan,
            gaming_items,
            gaming_alerts,
            gaming_report,
            gaming_lab_path,
            logs_tail,
            puente_estado,
            puente_modelos,
            puente_agentes,
            puente_preguntar,
            puente_servidor_arrancar,
            puente_servidor_parar,
            kukia_server_start,
            kukia_server_status,
            kukia_server_stop,
            puente_snippet,
            ecosistema_estado,
            ecosistema_enrutar,
            ecosistema_universal,
            xoxete_estado,
            xoxete_memorias,
            xoxete_recordar
        ])
        .run(tauri::generate_context!())
        .expect("error ejecutando KUKIA CODE");
}
