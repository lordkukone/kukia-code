import { useEffect, useState } from "react";
import { api, type AppConfig, type ModelInfo, type ProviderInfo, type SystemInfo } from "./lib/api";
import Chat from "./views/Chat";
import Models from "./views/Models";
import Agents from "./views/Agents";
import Files from "./views/Files";
import TerminalView from "./views/Terminal";
import MemoryView from "./views/Memory";
import Gpu from "./views/Gpu";
import Benchmarks from "./views/Benchmarks";
import Settings from "./views/Settings";
import History from "./views/History";
import Tools from "./views/Tools";
import Tasks from "./views/Tasks";
import Radar from "./views/Radar";
import Research from "./views/Research";
import AnimeReal from "./views/AnimeReal";
import Vram from "./views/Vram";
import Gaming from "./views/Gaming";
import Security from "./views/Security";

export type ViewId =
  | "chat" | "models" | "agents" | "files" | "terminal" | "tasks"
  | "memory" | "tools" | "gpu" | "benchmarks" | "settings" | "history" | "radar"
  | "research" | "security" | "anime" | "vram" | "gaming";

const NAV: { id: ViewId; label: string; icon: string }[] = [
  { id: "chat", label: "Chat", icon: "💬" },
  { id: "models", label: "Modelos", icon: "🧠" },
  { id: "agents", label: "Agentes", icon: "🤖" },
  { id: "files", label: "Archivos", icon: "📁" },
  { id: "terminal", label: "Terminal", icon: "⌨️" },
  { id: "tasks", label: "Tareas", icon: "📋" },
  { id: "security", label: "Seguridad", icon: "🛡️" },
  { id: "memory", label: "Memoria", icon: "🗄️" },
  { id: "tools", label: "Herramientas", icon: "🧰" },
  { id: "gpu", label: "GPU", icon: "📊" },
  { id: "benchmarks", label: "Benchmarks", icon: "⏱️" },
  { id: "radar", label: "Radar", icon: "📡" },
  { id: "research", label: "Research", icon: "🔬" },
  { id: "anime", label: "Anime ↔ Real", icon: "🎬" },
  { id: "vram", label: "VRAM Guard", icon: "🧯" },
  { id: "gaming", label: "Gaming Radar", icon: "🎮" },
  { id: "history", label: "Historial", icon: "🕘" },
  { id: "settings", label: "Configuración", icon: "⚙️" },
];

export default function App() {
  const [view, setView] = useState<ViewId>("chat");
  const [system, setSystem] = useState<SystemInfo | null>(null);
  const [providers, setProviders] = useState<ProviderInfo[]>([]);
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [provider, setProvider] = useState("ollama");
  const [model, setModel] = useState("");
  const [toast, setToast] = useState<string | null>(null);

  const notify = (msg: string) => {
    setToast(msg);
    window.setTimeout(() => setToast(null), 4000);
  };

  const refreshProviders = async () => {
    try {
      const p = await api.providers();
      setProviders(p);
      return p;
    } catch (e) {
      notify(`Error listando proveedores: ${e}`);
      return [];
    }
  };

  const loadModels = async (providerId: string) => {
    try {
      const m = await api.models(providerId);
      setModels(m);
      if (m.length > 0 && !m.some((x) => x.id === model)) setModel(m[0].id);
      return m;
    } catch (e) {
      notify(`Error listando modelos: ${e}`);
      return [];
    }
  };

  useEffect(() => {
    (async () => {
      try {
        const [sys, cfg] = await Promise.all([api.systemInfo(), api.config()]);
        setSystem(sys);
        setConfig(cfg);
        setProvider(cfg.default_provider);
        setModel(cfg.default_model);
        const ps = await refreshProviders();
        const active = ps.find((p) => p.id === cfg.default_provider) ?? ps[0];
        if (active) {
          setProvider(active.id);
          await loadModels(active.id);
        }
      } catch (e) {
        notify(`Arranque incompleto: ${e}`);
      }
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const selectProvider = async (id: string) => {
    setProvider(id);
    await loadModels(id);
  };

  const persistSelection = async (p: string, m: string) => {
    if (!config) return;
    try {
      const next = { ...config, default_provider: p, default_model: m };
      const saved = await api.setConfig(next);
      setConfig(saved);
    } catch (e) {
      notify(`No se pudo guardar la selección: ${e}`);
    }
  };

  const headerModel = model || config?.default_model || "sin modelo";

  return (
    <div className="app">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-logo">K</div>
          <div>
            <div className="brand-name">KUKIA CODE</div>
            <div className="brand-sub">v0.1.0 · Fase 2</div>
          </div>
        </div>
        <nav>
          {NAV.map((n) => (
            <button
              key={n.id}
              className={`nav-item ${view === n.id ? "active" : ""}`}
              onClick={() => setView(n.id)}
            >
              <span className="nav-icon">{n.icon}</span>
              {n.label}
            </button>
          ))}
        </nav>
        <div className="sidebar-footer">
          <div className="status-dot online" /> núcleo local
        </div>
      </aside>

      <main className="main">
        <header className="topbar">
          <div className="topbar-left">
            <span className="chip">{provider}</span>
            <span className="chip model">{headerModel}</span>
          </div>
          <div className="topbar-right">
            {system && (
              <span className="chip subtle">
                {system.gpus.length} GPU · {system.ram_gb.toFixed(0)} GB RAM
              </span>
            )}
            <span className={`chip ${providers.some((p) => p.online) ? "ok" : "warn"}`}>
              {providers.filter((p) => p.online).length}/{providers.length} proveedores
            </span>
          </div>
        </header>

        <section className="content">
          {view === "chat" && (
            <Chat provider={provider} model={model} notify={notify} />
          )}
          {view === "models" && (
            <Models
              providers={providers}
              models={models}
              provider={provider}
              model={model}
              onSelectProvider={selectProvider}
              onSelectModel={(m) => {
                setModel(m);
                persistSelection(provider, m);
              }}
              onRefresh={refreshProviders}
              notify={notify}
            />
          )}
          {view === "agents" && <Agents notify={notify} />}
          {view === "files" && <Files notify={notify} />}
          {view === "terminal" && <TerminalView notify={notify} />}
          {view === "memory" && <MemoryView notify={notify} />}
          {view === "gpu" && <Gpu notify={notify} />}
          {view === "benchmarks" && (
            <Benchmarks provider={provider} model={model} notify={notify} />
          )}
          {view === "settings" && config && (
            <Settings config={config} onSaved={setConfig} notify={notify} />
          )}
          {view === "history" && <History notify={notify} />}
          {view === "tools" && <Tools />}
          {view === "tasks" && <Tasks />}
          {view === "security" && <Security notify={notify} />}
          {view === "radar" && <Radar notify={notify} />}
          {view === "research" && <Research notify={notify} />}
          {view === "anime" && <AnimeReal notify={notify} />}
          {view === "vram" && <Vram notify={notify} />}
          {view === "gaming" && <Gaming notify={notify} />}
        </section>
      </main>

      {toast && <div className="toast">{toast}</div>}
    </div>
  );
}
