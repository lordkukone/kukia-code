import { invoke, Channel } from "@tauri-apps/api/core";

/* ============ Tipos compartidos con el núcleo Rust ============ */

export interface GpuInfo {
  index: number;
  name: string;
  /** "nvidia", "amd", "intel", "microsoft" o "desconocido". */
  fabricante?: string;
  memory_total_mb: number;
}

export interface SystemInfo {
  os: string;
  cpu: string;
  cores: number;
  ram_gb: number;
  gpus: GpuInfo[];
  data_dir: string;
}

export interface GpuStat {
  index: number;
  name: string;
  /** "nvidia", "amd", "intel", "microsoft" o "desconocido". */
  fabricante?: string;
  /** De donde sale el detalle: nvidia-smi, contadores de Windows o no disponible. */
  fuente?: string;
  utilization: number;
  memory_used_mb: number;
  memory_total_mb: number;
  temperature_c: number;
  power_w: number;
  clock_graphics_mhz: number;
  clock_memory_mhz: number;
}

export interface ProviderInfo {
  id: string;
  name: string;
  kind: string;
  base_url: string;
  online: boolean;
  models_count: number;
  free_models: number;
  note: string;
}

export interface ModelInfo {
  id: string;
  provider: string;
  name: string;
  context: number | null;
  size_gb: number | null;
  capabilities: string[];
  state: string;
}

export interface ChatMessage {
  role: "system" | "user" | "assistant";
  content: string;
}

export interface ChatRequest {
  provider: string;
  model: string;
  messages: ChatMessage[];
  temperature?: number;
  max_tokens?: number;
  system_prompt?: string;
  /** Usar herramientas (por defecto sí, si el modelo declara `tools`). */
  tools?: boolean;
  /** Aprobación explícita para escribir y ejecutar. */
  aprobar?: boolean;
  /** Agente activo: de él sale la política de permisos. */
  agent?: string;
}

export interface ChatResponse {
  content: string;
  prompt_tokens: number;
  completion_tokens: number;
  total_ms: number;
}

export interface ChatEvent {
  kind: "token" | "reasoning" | "done" | "error" | "toolcall" | "toolresult";
  nombre?: string;
  argumentos?: string;
  ok?: boolean;
  texto?: string;
  text?: string;
  prompt_tokens?: number;
  completion_tokens?: number;
  total_ms?: number;
  message?: string;
}

export interface Session {
  id: string;
  title: string;
  agent: string;
  created_at: string;
  updated_at: string;
}

export interface MessageRow {
  id: number;
  session_id: string;
  role: string;
  content: string;
  created_at: string;
}

export interface Agent {
  id: string;
  name: string;
  description: string;
  system_prompt: string;
  tools: string[];
  color: string;
}

export interface BenchmarkResult {
  id: number;
  provider: string;
  model: string;
  prompt_tokens: number;
  completion_tokens: number;
  total_ms: number;
  tokens_per_s: number;
  created_at: string;
}

export interface CloudProviderCfg {
  id: string;
  label: string;
  base_url: string;
}

export interface AppConfig {
  default_provider: string;
  default_model: string;
  temperature: number;
  gpu_split: [number, number];
  theme: string;
  cloud_base_url: string;
  cloud_model: string;
  /** Proveedor cloud activo (opencode, freebuff, openai, custom). */
  cloud_provider: string;
  /** Catálogo de proveedores. Nunca contiene claves. */
  cloud_providers: CloudProviderCfg[];
  has_cloud_key: boolean;
  research_auto: boolean;
  reasoning: boolean;
  active_project?: string | null;
}

export interface Project {
  id: string;
  name: string;
  root: string;
  read_paths: string;
  write_paths: string;
  read_only_paths: string;
  allow_write: boolean;
  allow_shell: boolean;
  allow_network: boolean;
  created_at: string;
  updated_at: string;
}

export interface Discovery {
  id: string;
  kind: string;
  title: string;
  source: string;
  url: string;
  published: string;
  summary: string;
  license: string;
  windows: string;
  gpu: string;
  vram: string;
  expected: string;
  measured: string;
  risks: string;
  recommendation: string;
  status: string;
  score: number;
}

export interface ResearchAlertRow {
  id: number;
  item_id: string;
  level: string;
  message: string;
  created_at: string;
}

export interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
  modified: string;
}

export interface UpdateInfo {
  source: string;
  version: string;
  published: string;
  url: string;
  note: string;
}

export interface CompatRow {
  id: number;
  engine: string;
  model: string;
  arch: string;
  quant: string;
  gpu: string;
  method: string;
  notes: string;
  created_at: string;
}

export interface ApprovalRow {
  id: number;
  action: string;
  detail: string;
  approved: boolean;
  created_at: string;
}

export interface RunResult {
  code: number;
  stdout: string;
  stderr: string;
  truncated: boolean;
  duration_ms: number;
  timed_out: boolean;
  cancelled: boolean;
  pid: number;
}

export interface AgentPolicy {
  agent: string;
  allow_read: boolean;
  allow_write: boolean;
  allow_shell: boolean;
  allow_network: boolean;
  allow_download: boolean;
  allow_tools: boolean;
  read_paths: string;
  write_paths: string;
  read_only_paths: string;
  updated_at: string;
}

export interface AuditRow {
  id: number;
  agent: string;
  tool: string;
  action: string;
  targets: string;
  approved: boolean;
  result: string;
  error: string;
  created_at: string;
}

export interface RunningInfo {
  id: string;
  pid: number;
  command: string;
  cwd: string;
  started_at: string;
}

export interface RunStreamEvent {
  kind: "started" | "done" | "failed";
  id: string;
  pid?: number;
  result?: RunResult;
  message?: string;
}

/* ============ KUKIA ANIME ↔ REAL STUDIO ============ */

export interface ComfyStatus {
  online: boolean;
  base_url: string;
  version: string;
  device: string;
  vram_total_mb: number;
  vram_free_mb: number;
  en_cola: number;
  nota: string;
}

export interface AnimeCapability {
  id: string;
  titulo: string;
  direccion: string;
  tipo: string;
  motor: string;
  disponible: boolean;
  faltan: string[];
  verificado: string;
  segundos_estimados: number;
  nota: string;
  estado: string;
}

export interface AnimeTemplate {
  id: string;
  titulo: string;
  direccion: string;
  tipo: string;
  motor: string;
  archivo: string;
  verificado: string;
  requiere: string[];
  segundos_estimados: number;
  nota: string;
  estado: string;
}

export interface AnimeResult {
  id: string;
  prompt_id: string;
  template_id: string;
  direccion: string;
  segundos: number;
  archivos: string[];
  bytes: number;
  preview: string;
  entrada_preview: string;
}

export interface AnimeJob {
  id: string;
  fecha: string;
  plantilla: string;
  direccion: string;
  entrada: string;
  segundos: number;
  archivos: string[];
}

/* ============ CONVERTIDOR UNIVERSAL (RC1 + RC2) ============ */

export interface StudioMotor {
  id: string;
  nombre: string;
  clase: string;
  url: string;
  online: boolean;
  version: string;
  detalle: string;
  capacidades: string[];
  gpu: string;
}

export interface StudioVram {
  index: number;
  nombre: string;
  total_mb: number;
  usada_mb: number;
  libre_mb: number;
}

export interface StudioPresupuesto {
  vram: StudioVram[];
  ram_libre_gb: number;
  ram_total_gb: number;
}

export interface StudioEstilo {
  id: string;
  titulo: string;
  origen: string[];
  destino: string;
  modalidad: string;
  motor: string;
  plantilla: string;
  modelos: string[];
  resolucion: number[];
  frames: number;
  pasos: number;
  vram_min_gb: number;
  ram_min_gb: number;
  estado: string;
  limites: string;
  licencia: string;
  medido: string;
}

export interface StudioDiagnostico {
  id: string;
  titulo: string;
  origen: string[];
  destino: string;
  modalidad: string;
  motor: string;
  plantilla: string;
  estado_declarado: string;
  estado_real: string;
  veredicto: string;
  motivo: string;
  listos: string[];
  faltan: string[];
  motor_online: boolean;
  vram_min_gb: number;
  ram_min_gb: number;
  vram_libre_max_gb: number;
  ram_libre_gb: number;
  medido: string;
  limites: string;
  licencia: string;
}

export interface StudioSondeo {
  ruta: string;
  tipo: string;
  ancho: number;
  alto: number;
  fps: number;
  segundos: number;
  audio: boolean;
  codec: string;
  bytes: number;
}

export interface StudioDeteccion {
  estilo: string;
  confianza: number;
  motivos: string;
  modelo: string;
  crudo: string;
  editable: boolean;
}

export interface StudioPlano {
  indice: number;
  inicio: number;
  fin: number;
  segundos: number;
}

export interface StudioPreflight {
  sondeo: StudioSondeo;
  deteccion: StudioDeteccion | null;
  planos: StudioPlano[];
  paquete_id: string;
  paquete_titulo: string;
  plantilla: string;
  veredicto: string;
  avisos: string[];
  faltan: string[];
  vram_libre_max_gb: number;
  ram_libre_gb: number;
  disco_estimado_gb: number;
}

/* ============ GUARDIA DE VRAM ============ */

export interface GpuVram {
  index: number;
  name: string;
  total_mb: number;
  used_mb: number;
  free_mb: number;
  reserved_mb: number;
  utilization: number;
}

export interface EngineModel {
  name: string;
  vram_mb: number;
  size_gb: number;
}

export interface EngineState {
  engine: string;
  online: boolean;
  note: string;
  models: EngineModel[];
  vram_total_mb: number;
}

export interface ProcessOnGpu {
  pid: number;
  name: string;
  vram_mb: number;
}

/** Aviso de competencia entre motores, emitido por el vigilante de fondo. */
export interface VramAlert {
  competing: boolean;
  count: number;
  detalle: string[];
  models: number;
  message: string;
}

export interface VramReport {
  gpus: GpuVram[];
  engines: EngineState[];
  processes: ProcessOnGpu[];
  competition: boolean;
  warning: string;
  recommendations: string[];
  checked_at: string;
  /** Trabajos en marcha o en cola en ComfyUI: liberar su VRAM los interrumpe. */
  comfy_busy: number;
}

/** Resultado de sacar un modelo de la memoria (botón Expulsar / Vaciar VRAM). */
export interface UnloadResult {
  engine: string;
  model: string;
  ok: boolean;
  message: string;
}

/** Estado de una GPU después de una liberación, con lo recuperado en ella. */
export interface GpuUsed {
  index: number;
  name: string;
  used_mb: number;
  total_mb: number;
  freed_mb: number;
}

/** Liberación medida: motor, GPU o ambas. `freed_mb` sale de nvidia-smi, no de una estimación. */
export interface ReleaseReport {
  target: string;
  ok: boolean;
  freed_mb: number;
  gpus: GpuUsed[];
  results: UnloadResult[];
  note: string;
  checked_at: string;
}

/* ---------- Centro de control de motores (CC1–CC4) ---------- */

/** CC1: estado real de un servidor de inferencia. */
export interface EngineStatus {
  engine: string;
  online: boolean;
  base_url: string;
  port: number | null;
  port_source: string;
  version: string | null;
  latency_ms: number;
  installed: number;
  loaded: string[];
  note: string;
}

/** CC2: ficha completa de un modelo. */
export interface ModelCard {
  engine: string;
  id: string;
  family: string | null;
  parameter_size: string | null;
  quantization: string | null;
  format: string | null;
  size_gb: number | null;
  context_length: number | null;
  capabilities: string[];
  loaded: boolean;
  publisher: string | null;
  architecture: string | null;
  note: string;
}

/** CC3: parámetros de inferencia por modelo. `null` = lo decide el motor. */
export interface InferenceParams {
  temperature: number | null;
  top_p: number | null;
  top_k: number | null;
  num_ctx: number | null;
  num_gpu: number | null;
  repeat_penalty: number | null;
  seed: number | null;
  max_tokens: number | null;
  keep_alive: string | null;
  /* Panel avanzado */
  min_p: number | null;
  repeat_last_n: number | null;
  presence_penalty: number | null;
  frequency_penalty: number | null;
  stop: string[];
  num_batch: number | null;
  num_thread: number | null;
  flash_attn: boolean | null;
  use_mmap: boolean | null;
  use_mlock: boolean | null;
  gpu_offload: string | null;
  parallel: number | null;
  speculative: boolean | null;
}

/* -------- Panel avanzado de modelo (contexto, KV, GPU, perfiles) -------- */

/** Un parámetro con el estado real de aplicación que admite su motor. */
export interface ParamSpec {
  key: string;
  etiqueta: string;
  grupo: string;
  tipo: string;
  /** caliente | recarga | sesion | no_compatible | sin_verificar */
  aplica: string;
  motivo: string;
  valor: unknown;
  defecto: unknown;
  min: number | null;
  max: number | null;
  opciones: string[];
}

export interface KvOption {
  formato: string;
  gb: number;
  ahorro_gb: number;
  ahorro_pct: number;
  estado: string;
  motivo: string;
}

export interface ContextoInfo {
  actual: number;
  maximo_modelo: number;
  maximo_servido: number;
  fuente: string;
  gb_por_8k: number;
}

export interface GpuUso {
  index: number;
  name: string;
  total_mb: number;
  used_mb: number;
  free_mb: number;
  utilization: number;
  temperatura_c: number;
  potencia_w: number;
}

export interface Capacidades {
  engine: string;
  model: string;
  arch: {
    architecture: string;
    layers: number;
    heads: number;
    heads_kv: number;
    embedding: number;
    ctx_native: number;
  };
  contexto: ContextoInfo;
  kv: KvOption[];
  parametros: ParamSpec[];
  gpus: GpuUso[];
  avisos: string[];
}

export interface Metricas {
  gpus: GpuUso[];
  ram_total_gb: number;
  ram_usada_gb: number;
  ram_libre_gb: number;
  cpu_uso_pct: number;
  cargados: string[];
  tomado_en: string;
}

export interface Perfil {
  nombre: string;
  descripcion: string;
  params: InferenceParams;
  creado: string;
}

/* -------- Laboratorio experimental -------- */

export interface Experimento {
  id: string;
  nombre: string;
  categoria: string;
  /** estable | experimental | muy_experimental | incompatible */
  nivel: string;
  descripcion: string;
  requisitos: string[];
  riesgos: string[];
  /** peticion | carga_cli | variable_servidor | ninguno */
  mecanismo: string;
  motores: string[];
  activable: boolean;
  efecto: string;
  estado_local: string;
  variable: string | null;
  valor_actual: string | null;
}

export interface PicoGpu {
  index: number;
  name: string;
  pico_mb: number;
}

export interface BenchResult {
  id: string;
  etiqueta: string;
  engine: string;
  model: string;
  params_texto: string;
  params: InferenceParams;
  tokens: number;
  tok_s: number;
  prefill_tok_s: number | null;
  ttft_ms: number | null;
  itl_ms: number | null;
  carga_ms: number | null;
  total_ms: number;
  vram_pico: PicoGpu[];
  ram_pico_gb: number;
  temp_pico_c: number;
  potencia_pico_w: number;
  ok: boolean;
  error: string;
  tomado_en: string;
}

/* -------- Cola de trabajos (SQ1) -------- */

export interface Job {
  id: number;
  kind: string;
  payload: unknown;
  /** queued | running | done | failed | cancelled */
  state: string;
  priority: number;
  attempts: number;
  max_attempts: number;
  worker: string | null;
  progress: number | null;
  result: unknown;
  error: string;
  created_at: string;
  started_at: string | null;
  finished_at: string | null;
}

export interface JobStats {
  queued: number;
  running: number;
  done: number;
  failed: number;
  cancelled: number;
  espera_max_s: number;
  total: number;
}

export interface JobsEstado {
  parada: boolean;
  activos: number;
  max: number;
}

export interface Comparacion {
  engine: string;
  model: string;
  a: BenchResult;
  b: BenchResult;
  delta_tok_s: number;
  delta_tok_s_pct: number;
  delta_vram_mb: [number, number][];
  delta_ram_gb: number;
  veredicto: string;
}

/** CC4: resultado del preflight de carga. */
export interface Preflight {
  engine: string;
  model: string;
  needed_gb: number | null;
  best_gpu: { index: number; name: string; free_gb: number } | null;
  fits: boolean;
  source: string;
  detail: string;
  warnings: string[];
}

/* ============ KUKIA FUTURE GAMING RADAR ============ */

export interface GamingItem {
  id: string;
  name: string;
  category: string;
  description: string;
  source: string;
  url: string;
  published: string;
  updated: string;
  status: string;
  opensource: string;
  license: string;
  os: string;
  gpus: string;
  games: string;
  requirements: string;
  measured: string;
  risks: string;
  conflicts: string;
  rtx5070: string;
  rtx5060ti: string;
  potential: string;
  recommendation: string;
  score: number;
}

export interface GamingAlertRow {
  id: number;
  item_id: string;
  level: string;
  message: string;
  created_at: string;
}

export interface GamingScan {
  encontrados: number;
  nuevos: number;
  alertas: number;
  informe: string;
}

/* ============ Invocaciones ============ */

export const api = {
  systemInfo: () => invoke<SystemInfo>("get_system_info"),
  providers: () => invoke<ProviderInfo[]>("list_providers"),
  models: (provider: string) => invoke<ModelInfo[]>("list_models", { provider }),
  chat: (request: ChatRequest) => invoke<ChatResponse>("chat", { request }),
  gpuStats: () => invoke<GpuStat[]>("gpu_stats"),

  sessions: () => invoke<Session[]>("sessions_list"),
  createSession: (title: string, agent: string) =>
    invoke<Session>("session_create", { title, agent }),
  renameSession: (id: string, title: string) => invoke<void>("session_rename", { id, title }),
  messages: (sessionId: string) => invoke<MessageRow[]>("messages_list", { sessionId }),
  addMessage: (sessionId: string, role: string, content: string) =>
    invoke<MessageRow>("message_add", { sessionId, role, content }),

  memorySearch: (query: string) => invoke<string[]>("memory_search", { query }),
  memoryAdd: (content: string, tags: string) => invoke<void>("memory_add", { content, tags }),

  agents: () => invoke<Agent[]>("agents_list"),
  createAgent: (name: string, description: string, systemPrompt: string, tools: string) =>
    invoke<Agent>("agent_create", { name, description, systemPrompt, tools }),

  config: () => invoke<AppConfig>("config_get"),
  setConfig: (config: AppConfig) => invoke<AppConfig>("config_set", { config }),

  logs: (lines: number) => invoke<string[]>("logs_tail", { lines }),
  runCommand: (command: string, cwd: string, approved: boolean, timeoutSecs?: number) =>
    invoke<RunResult>("run_command", {
      command,
      cwd,
      approved,
      agent: null,
      timeoutSecs: timeoutSecs ?? null,
    }),
  commandCancel: (id: string) => invoke<boolean>("command_cancel", { id }),
  commandsRunning: () => invoke<RunningInfo[]>("commands_running"),

  policyList: () => invoke<AgentPolicy[]>("policy_list"),
  policyGet: (agent: string) => invoke<AgentPolicy>("policy_get", { agent }),
  policySave: (policy: AgentPolicy, approved: boolean) =>
    invoke<AgentPolicy>("policy_save", { policy, approved }),
  auditList: (query?: string, tool?: string, limit?: number) =>
    invoke<AuditRow[]>("audit_list", {
      query: query ?? null,
      tool: tool ?? null,
      approved: null,
      limit: limit ?? 100,
    }),
  projects: () => invoke<Project[]>("projects_list"),
  createProject: (name: string, root: string, allowShell?: boolean) =>
    invoke<Project>("project_create", { name, root, allowShell: allowShell ?? null }),
  updateProject: (project: Project, approved: boolean) =>
    invoke<Project>("project_update", { project, approved }),
  activateProject: (id: string | null) => invoke<AppConfig>("project_activate", { id }),
  activeProject: () => invoke<Project | null>("project_active"),

  auditExport: (path: string, approved: boolean) =>
    invoke<number>("audit_export", { path, approved, agent: null }),

  benchmark: (provider: string, model: string, prompt: string) =>
    invoke<BenchmarkResult>("benchmark_run", { provider, model, prompt }),
  benchmarks: () => invoke<BenchmarkResult[]>("benchmarks_list"),
  approvals: () => invoke<ApprovalRow[]>("approvals_list"),

  researchScan: (source?: string) =>
    invoke<Discovery[]>("research_scan", { source: source ?? null }),
  researchItems: (limit?: number) => invoke<Discovery[]>("research_items", { limit: limit ?? null }),
  researchAlerts: (limit?: number) =>
    invoke<ResearchAlertRow[]>("research_alerts", { limit: limit ?? null }),
  researchReport: () => invoke<string>("research_report"),
  researchFetch: (url: string) => invoke<string>("research_fetch", { url }),
  researchLabPath: () => invoke<string>("research_lab_path"),

  radarCheck: () => invoke<UpdateInfo[]>("radar_check"),
  compatList: () => invoke<CompatRow[]>("compat_list"),
  compatAdd: (
    engine: string,
    model: string,
    arch: string,
    quant: string,
    gpu: string,
    method: string,
    notes: string,
  ) => invoke<void>("compat_add", { engine, model, arch, quant, gpu, method, notes }),

  vramReport: () => invoke<VramReport>("vram_report"),
  vramFits: (sizeGb: number, gpu: number) =>
    invoke<[boolean, string] | null>("vram_fits", { sizeGb, gpu }),
  vramUnload: (engine: string, model: string, forzar = false) =>
    invoke<UnloadResult>("vram_unload", { engine, model, forzar }),
  vramUnloadAll: (forzar = false) => invoke<UnloadResult[]>("vram_unload_all", { forzar }),
  vramUnloadSeleccion: (items: VramSeleccion[]) =>
    invoke<LoteResult>("vram_unload_seleccion", { items }),
  vramHistorial: (limit?: number) =>
    invoke<OpVram[]>("vram_historial", { limit: limit ?? null }),
  vramReleaseEngine: (engine: string, forzar = false) =>
    invoke<ReleaseReport>("vram_release_engine", { engine, forzar }),
  vramReleaseGpu: (gpu: number, forzar = false) =>
    invoke<ReleaseReport>("vram_release_gpu", { gpu, forzar }),
  vramReleaseEverything: (forzar = false) =>
    invoke<ReleaseReport>("vram_release_everything", { forzar }),

  enginesStatus: () => invoke<EngineStatus[]>("engines_status"),
  engineCards: (engine: string) => invoke<ModelCard[]>("engine_cards", { engine }),
  engineCard: (engine: string, model: string) =>
    invoke<ModelCard>("engine_card", { engine, model }),
  engineParamsGet: (engine: string, model: string) =>
    invoke<InferenceParams>("engine_params_get", { engine, model }),
  engineParamsSet: (engine: string, model: string, params: InferenceParams) =>
    invoke<string[]>("engine_params_set", { engine, model, params }),
  enginePreflight: (engine: string, model: string) =>
    invoke<Preflight>("engine_preflight", { engine, model }),
  engineLoad: (engine: string, model: string) =>
    invoke<UnloadResult>("engine_load", { engine, model }),
  controlCapacidades: (engine: string, model: string) =>
    invoke<Capacidades>("control_capacidades", { engine, model }),
  controlMetricas: () => invoke<Metricas>("control_metricas"),
  perfilesListar: () => invoke<Perfil[]>("perfiles_listar"),
  perfilGuardar: (perfil: Perfil) => invoke<Perfil[]>("perfil_guardar", { perfil }),
  perfilBorrar: (nombre: string) => invoke<Perfil[]>("perfil_borrar", { nombre }),
  perfilAplicar: (engine: string, model: string, nombre: string) =>
    invoke<[InferenceParams, string[]]>("perfil_aplicar", { engine, model, nombre }),
  perfilRestaurarAnterior: (engine: string, model: string) =>
    invoke<InferenceParams | null>("perfil_restaurar_anterior", { engine, model }),
  jobsEnqueue: (kind: string, payload?: unknown, priority?: number) =>
    invoke<number>("jobs_enqueue", { kind, payload, priority }),
  jobsList: (limit?: number) => invoke<Job[]>("jobs_list", { limit }),
  jobsStats: () => invoke<JobStats>("jobs_stats"),
  jobsCancel: (id: number) => invoke<boolean>("jobs_cancel", { id }),
  jobsCancelAll: () => invoke<number>("jobs_cancel_all"),
  jobsClear: () => invoke<number>("jobs_clear"),
  jobsStop: () => invoke<boolean>("jobs_stop"),
  jobsResume: () => invoke<boolean>("jobs_resume"),
  jobsEstado: () => invoke<JobsEstado>("jobs_estado"),
  jobsWorkers: (workers?: number) => invoke<number>("jobs_workers", { workers }),

  // Ecosistema de herramientas: MCP, registro y investigación autónoma.
  // Model Hub: descubrir, comprobar y descargar.
  hubBuscarOllama: (q: string) => invoke<OllamaHit[]>("hub_buscar_ollama", { q }),
  hubBuscarHf: (q: string, limite?: number) =>
    invoke<HubModelo[]>("hub_buscar_hf", { q, limite: limite ?? null }),
  hubManifest: (modelo: string) => invoke<OllamaTag>("hub_manifest", { modelo }),
  hubGguf: (max?: number) => invoke<GgufLocal[]>("hub_gguf", { max: max ?? null }),
  hubCompat: (paramsB: number, quant: string, ctx?: number) =>
    invoke<Compatibilidad>("hub_compat", { paramsB, quant, ctx: ctx ?? null }),
  hubPull: (modelo: string) => invoke<number>("hub_pull", { modelo }),

  mcpServers: () => invoke<McpServerInfo[]>("mcp_servers"),
  mcpGrant: (server: string, granted: boolean) =>
    invoke<boolean>("mcp_grant", { server, granted }),
  mcpDiscover: (server: string) => invoke<McpTool[]>("mcp_discover", { server }),
  mcpCall: (server: string, tool: string, args: unknown) =>
    invoke<unknown>("mcp_call", { server, tool, arguments: args }),
  mcpTools: (server: string) => invoke<McpTool[]>("mcp_tools", { server }),
  toolsRegistry: () => invoke<ToolEntry[]>("tools_registry"),
  toolsResumen: () => invoke<ToolsResumen>("tools_resumen"),
  researchAuto: (objetivo: string, limite?: number) =>
    invoke<AutoInforme>("research_auto", { objetivo, limite: limite ?? null }),

  // Cerebro universal de modelos: catálogo, router y adaptadores.
  catalogList: () => invoke<CatalogReport>("catalog_list"),
  catalogAdapters: () => invoke<AdapterInfo[]>("catalog_adapters"),
  catalogRoute: (need: CatalogNeed) => invoke<CatalogRoute>("catalog_route", { need }),
  catalogBindSet: (scope: string, engine: string, model: string) =>
    invoke<void>("catalog_bind_set", { scope, engine, model }),
  catalogBinds: () => invoke<CatalogBinding[]>("catalog_binds"),
  catalogBindClear: (scope: string) => invoke<boolean>("catalog_bind_clear", { scope }),
  catalogChat: (
    prompt: string,
    capability?: string,
    modelKey?: string,
    maxTokens?: number,
    allowCloud?: boolean,
  ) =>
    invoke<UnifiedChat>("catalog_chat", {
      prompt,
      capability: capability ?? null,
      modelKey: modelKey ?? null,
      maxTokens: maxTokens ?? null,
      allowCloud: allowCloud ?? null,
    }),
  labCatalogo: () => invoke<Experimento[]>("lab_catalogo"),
  labBench: (
    engine: string,
    model: string,
    params: InferenceParams,
    prompt?: string,
    tokens?: number,
  ) => invoke<BenchResult>("lab_bench", { engine, model, params, prompt, tokens }),
  labAb: (
    engine: string,
    model: string,
    a: InferenceParams,
    b: InferenceParams,
    prompt?: string,
    tokens?: number,
  ) => invoke<Comparacion>("lab_ab", { engine, model, a, b, prompt, tokens }),
  labHistorial: () => invoke<BenchResult[]>("lab_historial"),
  labActivar: (experimento: string, engine: string, model: string) =>
    invoke<[InferenceParams, string]>("lab_activar", { experimento, engine, model }),
  labRestaurar: (engine: string, model: string) =>
    invoke<InferenceParams | null>("lab_restaurar", { engine, model }),

  gamingScan: () => invoke<GamingScan>("gaming_scan"),
  gamingItems: (limit?: number) =>
    invoke<GamingItem[]>("gaming_items", { limit: limit ?? 200 }),
  gamingAlerts: (limit?: number) =>
    invoke<GamingAlertRow[]>("gaming_alerts", { limit: limit ?? 100 }),
  gamingReport: () => invoke<string>("gaming_report"),
  gamingLabPath: () => invoke<string>("gaming_lab_path"),

  animeStatus: () => invoke<ComfyStatus>("anime_status", { base: null }),
  animeTemplates: () => invoke<AnimeTemplate[]>("anime_templates"),
  animeCapabilities: () => invoke<AnimeCapability[]>("anime_capabilities", { base: null }),
  animeInputs: () => invoke<string[]>("anime_inputs", { base: null }),
  animeOutputPath: () => invoke<string>("anime_output_path"),
  studioMotores: () => invoke<StudioMotor[]>("studio_motores"),
  studioPresupuesto: () => invoke<StudioPresupuesto>("studio_presupuesto"),
  studioEstilos: () => invoke<StudioEstilo[]>("studio_estilos"),
  studioDiagnostico: () => invoke<StudioDiagnostico[]>("studio_diagnostico"),
  studioPreflight: (ruta: string, destino?: string, modelo?: string, umbral?: number) =>
    invoke<StudioPreflight>("studio_preflight", {
      ruta,
      destino: destino ?? null,
      modelo: modelo ?? null,
      umbral: umbral ?? 0.3,
    }),
  studioPlanos: (ruta: string, umbral?: number) =>
    invoke<StudioPlano[]>("studio_planos", { ruta, umbral: umbral ?? 0.3 }),
  studioDetectar: (ruta: string, modelo?: string) =>
    invoke<StudioDeteccion>("studio_detectar", { ruta, modelo: modelo ?? null }),
  studioCortarPlano: (ruta: string, inicio: number, segundos: number, destino: string) =>
    invoke<string>("studio_cortar_plano", { ruta, inicio, segundos, destino }),
  animeHistory: (limit?: number) => invoke<AnimeJob[]>("anime_history", { limit: limit ?? 30 }),
  animeConvert: (req: {
    template_id: string;
    image: string;
    prompt: string;
    negative: string;
    denoise: number | null;
    seed: number | null;
    width: number | null;
    height: number | null;
  }) => invoke<AnimeResult>("anime_convert", { base: null, req }),

  files: (path: string) => invoke<FileEntry[]>("files_list", { path }),
  readFile: (path: string) => invoke<string>("files_read", { path }),
  writeFile: (path: string, content: string, approved: boolean) =>
    invoke<void>("files_write", { path, content, approved }),

  /* ---------------- Puente KUKIA CORE ↔ OPENCODE ---------------- */
  puenteEstado: (url?: string) => invoke<PuenteEstado>("puente_estado", { url: url ?? null }),
  puenteModelos: (url?: string) =>
    invoke<ModeloPuente[]>("puente_modelos", { url: url ?? null }),
  puenteAgentes: (url?: string) =>
    invoke<AgentePuente[]>("puente_agentes", { url: url ?? null }),
  puentePreguntar: (req: {
    url?: string;
    provider: string;
    model: string;
    agent?: string | null;
    texto: string;
    timeout?: number | null;
  }) =>
    invoke<PuenteRespuesta>("puente_preguntar", {
      url: req.url ?? null,
      provider: req.provider,
      model: req.model,
      agent: req.agent ?? null,
      texto: req.texto,
      timeout: req.timeout ?? null,
    }),
  puenteServidorArrancar: (port?: number) =>
    invoke<string>("puente_servidor_arrancar", { port: port ?? null }),
  puenteServidorParar: () => invoke<string>("puente_servidor_parar"),
  kukiaServerStart: (port?: number, token?: string) =>
    invoke<ServeStatus>("kukia_server_start", { port: port ?? null, token: token ?? null }),
  kukiaServerStatus: () => invoke<ServeStatus>("kukia_server_status"),
  kukiaServerStop: () => invoke<boolean>("kukia_server_stop"),
  puenteSnippet: (port?: number, token?: string) =>
    invoke<string>("puente_snippet", { port: port ?? null, token: token ?? null }),

  /* ---------------- Ecosistema IA ---------------- */
  ecosistemaEstado: (opencodeUrl?: string) =>
    invoke<Ecosistema>("ecosistema_estado", { opencodeUrl: opencodeUrl ?? null }),
  ecosistemaEnrutar: (tarea: string, opencodeUrl?: string) =>
    invoke<Recomendacion>("ecosistema_enrutar", { tarea, opencodeUrl: opencodeUrl ?? null }),
  ecosistemaUniversal: () => invoke<InventarioUniversal>("ecosistema_universal"),

  /* ---------------- Xoxete (instalación real y memoria propia) ---------------- */
  xoxeteEstado: () => invoke<XoxeteEstado>("xoxete_estado"),
  xoxeteMemorias: (consulta: string, limite?: number) =>
    invoke<MemoriaXoxete[]>("xoxete_memorias", { consulta, limite: limite ?? null }),
  xoxeteRecordar: (req: {
    tipo: string;
    contexto: string;
    accion: string;
    resultado: string;
    evidencia?: string;
  }) =>
    invoke<string>("xoxete_recordar", {
      tipo: req.tipo,
      contexto: req.contexto,
      accion: req.accion,
      resultado: req.resultado,
      evidencia: req.evidencia ?? null,
    }),
};

/* ============ Xoxete ============ */

export interface XoxeteBinario {
  nombre: string;
  ruta: string;
  existe: boolean;
  version: string;
}

export interface XoxetePlugin {
  nombre: string;
  presente: boolean;
  bytes: number;
}

export interface XoxeteCuenta {
  nombre: string;
  cantidad: number;
}

export interface XoxeteMotor {
  disponible: boolean;
  ruta: string;
  base: string;
  bytes: number;
  total: number;
  por_tipo: XoxeteCuenta[];
  por_estado: XoxeteCuenta[];
  error: string | null;
}

export interface XoxeteEstado {
  raiz: string;
  config: string;
  config_ok: boolean;
  agente: string;
  modelo: string;
  modelo_pequeno: string;
  instrucciones: string[];
  instrucciones_ok: number;
  motor: XoxeteMotor;
  plugins: XoxetePlugin[];
  binarios: XoxeteBinario[];
  notas: string[];
}

export interface MemoriaXoxete {
  id: number;
  tipo: string;
  estado: string;
  contexto: string;
  accion: string;
  resultado: string;
  confianza: number;
}

/* ============ Registro universal ============ */

export interface SistemaUniversal {
  id: string;
  nombre: string;
  categoria: string;
  instalado: boolean;
  detectado: boolean;
  conectado: boolean;
  operativo: boolean;
  version: string;
  ruta: string;
  puertos_abiertos: number[];
  endpoint: string;
  detalle: string;
  error: string | null;
  capacidades: string[];
  limitaciones: string[];
}

export interface InventarioUniversal {
  sistemas: SistemaUniversal[];
  total: number;
  instalados: number;
  conectados: number;
  operativos: number;
  no_detectados: string[];
}

/* ============ Ecosistema IA ============ */

export interface Cuenta {
  nombre: string;
  cantidad: number;
}

export interface NodoEcosistema {
  id: string;
  nombre: string;
  instalado: boolean;
  conectado: boolean;
  version: string;
  ruta: string;
  detalle: string;
  procesos: number;
  puertos: string[];
  modelos: number;
  agentes: number;
  herramientas: number;
  sesiones: number;
  puede: string[];
  limitaciones: string[];
  ultimo_error: string | null;
}

export interface CatalogoEcosistema {
  modelos_locales: number;
  modelos_opencode: number;
  agentes_kukia: number;
  agentes_opencode: number;
  herramientas_kukia: number;
  servidores_mcp: number;
  por_motor: Cuenta[];
  por_proveedor_opencode: Cuenta[];
}

export interface Ecosistema {
  nodos: NodoEcosistema[];
  catalogo: CatalogoEcosistema;
  notas: string[];
}

export interface Recomendacion {
  sistema: string;
  motivo: string;
  automatizable: boolean;
  como: string;
  alternativas: string[];
}

/* ============ Puente con OpenCode ============ */

export interface ProvCount {
  provider: string;
  modelos: number;
}

export interface ServidorOpencode {
  url: string;
  vivo: boolean;
  modelos: number;
  agentes: number;
  sesiones: number;
  error: string | null;
}

export interface PuenteEstado {
  opencode_instalado: boolean;
  binario: string;
  version: string;
  config_path: string;
  config_ok: boolean;
  config_nota: string;
  modelo_por_defecto: string;
  agente_por_defecto: string;
  modelos_cli: number;
  agentes_cli: number;
  proveedores: ProvCount[];
  mcp_config: string[];
  servidor: ServidorOpencode;
}

export interface ModeloPuente {
  provider: string;
  id: string;
  full: string;
  origen: string;
  habilitado: boolean;
}

export interface AgentePuente {
  nombre: string;
  modo: string;
  modelo: string | null;
  origen: string;
}

export interface PuenteRespuesta {
  ok: boolean;
  session_id: string;
  modelo: string;
  agente: string;
  texto: string;
  razonamiento: string;
  ms: number;
  tokens_out: number;
  coste: number;
  nota: string;
}

export interface ServeStatus {
  running: boolean;
  url: string;
  port: number;
  token: string;
  peticiones: number;
  errores: number;
  modelos: number;
  ultimo_error: string | null;
}

/* Ejecución de comandos con id inmediato, timeout y cancelación (Fase 3.0) */
export function runCommandStream(
  command: string,
  cwd: string,
  approved: boolean,
  onStarted: (id: string, pid: number) => void,
  onDone: (result: RunResult) => void,
  onFailed: (msg: string) => void,
  timeoutSecs?: number,
): Promise<string> {
  const channel = new Channel<RunStreamEvent>();
  channel.onmessage = (ev) => {
    if (ev.kind === "started") onStarted(ev.id, ev.pid ?? 0);
    else if (ev.kind === "done" && ev.result) onDone(ev.result);
    else if (ev.kind === "failed") onFailed(ev.message ?? "error desconocido");
  };
  return invoke<string>("run_command_stream", {
    command,
    cwd,
    approved,
    agent: null,
    timeoutSecs: timeoutSecs ?? null,
    onEvent: channel,
  });
}

/* Streaming con Channel de Tauri 2 */
export function chatStream(
  request: ChatRequest,
  onToken: (text: string) => void,
  onDone: (r: ChatResponse) => void,
  onError: (msg: string) => void,
  onReasoning?: (text: string) => void,
  onTool?: (nombre: string, detalle: string) => void,
): Promise<void> {
  const channel = new Channel<ChatEvent>();
  channel.onmessage = (ev) => {
    if (ev.kind === "token" && ev.text) onToken(ev.text);
    else if (ev.kind === "reasoning" && ev.text) onReasoning?.(ev.text);
    else if (ev.kind === "toolcall")
      onTool?.(ev.nombre ?? "herramienta", ev.argumentos ?? "");
    else if (ev.kind === "toolresult")
      onTool?.(ev.nombre ?? "herramienta", `${ev.ok ? "ok" : "error"}: ${ev.texto ?? ""}`);
    else if (ev.kind === "done")
      onDone({
        content: "",
        prompt_tokens: ev.prompt_tokens ?? 0,
        completion_tokens: ev.completion_tokens ?? 0,
        total_ms: ev.total_ms ?? 0,
      });
    else if (ev.kind === "error") onError(ev.message ?? "error desconocido");
  };
  return invoke<void>("chat_stream", { request, onEvent: channel });
}

/* ================= Cerebro universal de modelos ================= */

export interface UniversalModel {
  key: string;
  engine: string;
  provider: string;
  id: string;
  kind: string;
  capabilities: string[];
  context_length: number | null;
  size_gb: number | null;
  quantization: string | null;
  format: string | null;
  loaded: boolean;
  local: boolean;
  can_chat: boolean;
  inferred: boolean;
  note: string;
}

export interface CatalogCoverage {
  engine: string;
  name: string;
  reachable: boolean;
  models: number;
  catalog_api: boolean;
  detail: string;
}

export interface CatalogReport {
  models: UniversalModel[];
  coverage: CatalogCoverage[];
  total: number;
  generated_ms: number;
}

export interface AdapterInfo {
  engine: string;
  chat: boolean;
  stream: boolean;
  embeddings: boolean;
  image: boolean;
  video: boolean;
  audio: boolean;
  unload: boolean;
  status: boolean;
  detail: string;
}

export interface CatalogNeed {
  capability: string;
  min_context?: number | null;
  allow_cloud?: boolean;
  prefer_loaded?: boolean;
  scope?: string | null;
}

export interface CatalogCandidate {
  key: string;
  engine: string;
  id: string;
  kind: string;
  score: number;
  reasons: string[];
}

export interface CatalogRejected {
  key: string;
  why: string;
}

export interface CatalogRoute {
  need: CatalogNeed;
  chosen: CatalogCandidate | null;
  alternatives: CatalogCandidate[];
  rejected: CatalogRejected[];
  note: string;
}

export interface CatalogBinding {
  scope: string;
  engine: string;
  model: string;
  updated_at: string;
}

export interface UnifiedChat {
  engine: string;
  model: string;
  content: string;
  prompt_tokens: number;
  completion_tokens: number;
  total_ms: number;
  note: string;
}

/* ================= Ecosistema de herramientas ================= */

export interface McpServerInfo {
  name: string;
  transport: string;
  command: string;
  args: string[];
  url: string | null;
  source: string;
  authorized: boolean;
  tools: number;
  last_seen: string | null;
  note: string;
}

export interface McpTool {
  server: string;
  name: string;
  description: string;
  schema: unknown;
}

export interface ToolEntry {
  id: string;
  name: string;
  category: string;
  description: string;
  origin: string;
  permission: string;
  state: string;
  detail: string;
  server: string | null;
  schema: unknown;
}

export interface ToolsResumen {
  total: number;
  core: number;
  mcp: number;
  servidores_mcp: number;
  autorizados: number;
  disponibles: number;
  requieren_autorizacion: number;
  sin_comprobar: number;
}

export interface AutoInforme {
  objetivo: string;
  encontrados: number;
  nuevos: number;
  relevantes: Discovery[];
  propuestas: string[];
  contradicciones: string[];
  ruta: string;
  notas: string[];
}

/* ================= Model Hub ================= */

export interface OllamaHit {
  name: string;
  url: string;
}

export interface OllamaTag {
  name: string;
  size_gb: number | null;
  digest: string | null;
  note: string;
}

export interface HubModelo {
  id: string;
  url: string;
  autor: string;
  descargas: number;
  likes: number;
  licencia: string | null;
  modificado: string | null;
  pipeline: string | null;
  etiquetas: string[];
  fuente: string;
}

export interface GgufLocal {
  path: string;
  nombre: string;
  size_gb: number;
  dividido: boolean;
  parte: string | null;
  arquitectura: string | null;
  version: number | null;
  cuantizacion: string | null;
  valido: boolean;
  nota: string;
}

export interface GpuLibre {
  index: number;
  name: string;
  total_gb: number;
  free_gb: number;
}

export interface Compatibilidad {
  veredicto: string;
  pesos_gb: number | null;
  kv_gb: number | null;
  margen_gb: number | null;
  total_gb: number | null;
  gpus: GpuLibre[];
  ram_libre_gb: number;
  cabe_en_una_gpu: number | null;
  requiere_offload: boolean;
  explicacion: string;
}

/* ================= Descarga selectiva de VRAM ================= */

export interface VramSeleccion {
  engine: string;
  model: string;
  /** Permiso explícito para cortar una generación de ComfyUI. */
  forzar?: boolean;
}

export interface DiscoCheck {
  engine: string;
  model: string;
  presente: boolean | null;
  nota: string;
}

export interface LoteResult {
  resultados: UnloadResult[];
  gpus_antes: GpuVram[];
  gpus_despues: GpuVram[];
  liberado_mb: number;
  siguen_cargados: string[];
  en_disco: DiscoCheck[];
  avisos: string[];
}

export interface OpVram {
  id: number;
  engine: string;
  model: string;
  ok: boolean;
  liberado_mb: number;
  nota: string;
  created_at: string;
}
