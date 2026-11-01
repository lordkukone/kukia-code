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

export interface AppConfig {
  default_provider: string;
  default_model: string;
  temperature: number;
  gpu_split: [number, number];
  theme: string;
  cloud_base_url: string;
  cloud_model: string;
  has_cloud_key: boolean;
  research_auto: boolean;
  reasoning: boolean;
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
  runCommand: (command: string, cwd: string, approved: boolean) =>
    invoke<{ code: number; stdout: string; stderr: string }>("run_command", {
      command,
      cwd,
      approved,
    }),

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

  files: (path: string) => invoke<FileEntry[]>("files_list", { path }),
  readFile: (path: string) => invoke<string>("files_read", { path }),
  writeFile: (path: string, content: string, approved: boolean) =>
    invoke<void>("files_write", { path, content, approved }),
};

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
