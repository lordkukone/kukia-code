import { invoke, Channel } from "@tauri-apps/api/core";

/* ============ Tipos compartidos con el núcleo Rust ============ */

export interface GpuInfo {
  index: number;
  name: string;
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
}

export interface ChatResponse {
  content: string;
  prompt_tokens: number;
  completion_tokens: number;
  total_ms: number;
}

export interface ChatEvent {
  kind: "token" | "reasoning" | "done" | "error";
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

export interface VramReport {
  gpus: GpuVram[];
  engines: EngineState[];
  processes: ProcessOnGpu[];
  competition: boolean;
  warning: string;
  recommendations: string[];
  checked_at: string;
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
};

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
): Promise<void> {
  const channel = new Channel<ChatEvent>();
  channel.onmessage = (ev) => {
    if (ev.kind === "token" && ev.text) onToken(ev.text);
    else if (ev.kind === "reasoning" && ev.text) onReasoning?.(ev.text);
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
