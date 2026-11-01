import { useState } from "react";
import { api, type AppConfig } from "../lib/api";

interface Props {
  config: AppConfig;
  onSaved: (c: AppConfig) => void;
  notify: (m: string) => void;
}

export default function Settings({ config, onSaved, notify }: Props) {
  const [form, setForm] = useState<AppConfig>(config);
  const [saving, setSaving] = useState(false);

  const save = async () => {
    setSaving(true);
    try {
      const saved = await api.setConfig(form);
      onSaved(saved);
      notify("Configuración guardada");
    } catch (e) {
      notify(`Error guardando: ${e}`);
    } finally {
      setSaving(false);
    }
  };

  const set = <K extends keyof AppConfig>(key: K, value: AppConfig[K]) =>
    setForm((f) => ({ ...f, [key]: value }));

  return (
    <div className="page">
      <div className="page-head">
        <h1>Configuración</h1>
        <button className="btn primary" onClick={save} disabled={saving}>
          {saving ? "Guardando…" : "Guardar"}
        </button>
      </div>

      <div className="card">
        <h2>Modelo por defecto</h2>
        <div className="form-grid">
          <label>
            Proveedor
            <input value={form.default_provider} onChange={(e) => set("default_provider", e.target.value)} />
          </label>
          <label>
            Modelo
            <input value={form.default_model} onChange={(e) => set("default_model", e.target.value)} />
          </label>
          <label>
            Temperatura
            <input
              type="number"
              step="0.1"
              min="0"
              max="2"
              value={form.temperature}
              onChange={(e) => set("temperature", Number(e.target.value))}
            />
          </label>
          <label>
            Tema
            <select value={form.theme} onChange={(e) => set("theme", e.target.value)}>
              <option value="dark">Oscuro</option>
              <option value="midnight">Midnight</option>
            </select>
          </label>
        </div>
      </div>

      <div className="card">
        <h2>Reparto multi-GPU</h2>
        <p className="muted">
          Perfil inicial recomendado: RTX 5070 (GPU 0) 35 % · RTX 5060 Ti (GPU 1) 65 %. La GPU 0
          sigue siendo la principal para pantalla y juegos.
        </p>
        <div className="form-grid">
          <label>
            GPU 0 — RTX 5070 (%)
            <input
              type="number"
              min="0"
              max="100"
              value={form.gpu_split[0]}
              onChange={(e) => set("gpu_split", [Number(e.target.value), form.gpu_split[1]])}
            />
          </label>
          <label>
            GPU 1 — RTX 5060 Ti (%)
            <input
              type="number"
              min="0"
              max="100"
              value={form.gpu_split[1]}
              onChange={(e) => set("gpu_split", [form.gpu_split[0], Number(e.target.value)])}
            />
          </label>
        </div>
        <div className="split-bar">
          <div className="split-a" style={{ width: `${form.gpu_split[0]}%` }}>
            {form.gpu_split[0]}%
          </div>
          <div className="split-b" style={{ width: `${form.gpu_split[1]}%` }}>
            {form.gpu_split[1]}%
          </div>
        </div>
        <p className="muted small">
          Nota: repartir VRAM no es repartir cálculo. Cada motor aplica sólo lo que soporta
          (offload por capas en Ollama/llama.cpp; gestión propia en LM Studio).
        </p>
      </div>

      <div className="card">
        <h2>Proveedor cloud (OpenAI-compatible)</h2>
        <div className="form-grid">
          <label>
            Base URL
            <input
              value={form.cloud_base_url}
              onChange={(e) => set("cloud_base_url", e.target.value)}
              placeholder="https://api.openai.com/v1"
            />
          </label>
          <label>
            Modelo cloud
            <input
              value={form.cloud_model}
              onChange={(e) => set("cloud_model", e.target.value)}
              placeholder="gpt-4o-mini"
            />
          </label>
        </div>
        <p className="muted small">
          La clave API se lee de la variable de entorno <code>KUKIA_CLOUD_KEY</code>. No se guarda en
          el archivo de configuración. Estado: {form.has_cloud_key ? "clave detectada ✓" : "sin clave"}.
        </p>
      </div>

      <div className="card">
        <h2>Razonamiento</h2>
        <label className="inline">
          <input
            type="checkbox"
            checked={form.reasoning}
            onChange={(e) => set("reasoning", e.target.checked)}
          />
          Permitir que los modelos razonen antes de responder (recomendado)
        </label>
        <p className="muted small">
          Activado, los modelos con cadena de pensamiento (DeepSeek-R1, JEV, QwQ…) trabajan a pleno
          rendimiento y su pensamiento se muestra plegado en el chat. Desactivado, se les pide que
          respondan sin pensar: más rápido y más barato en tokens, pero peor en problemas complejos.
        </p>
      </div>

      <div className="card">
        <h2>KUKIA Experimental AI Radar</h2>
        <label className="inline">
          <input
            type="checkbox"
            checked={form.research_auto}
            onChange={(e) => set("research_auto", e.target.checked)}
          />
          Vigilancia automática cada 6 h (solo metadatos públicos, nunca instala nada)
        </label>
        <p className="muted small">
          Consulta arXiv, Hugging Face y GitHub. Las descargas manuales van al laboratorio aislado
          <code> H:\KUKIA_RESEARCH\AI_EXPERIMENTAL\downloads</code>.
        </p>
      </div>

      <div className="card">
        <h2>Datos</h2>
        <p className="muted small">
          Configuración, memoria, logs y benchmarks se guardan por usuario en{" "}
          <code>%APPDATA%\KukiaCode</code>. Nada se escribe fuera de ese directorio salvo los
          archivos que tú abras explícitamente.
        </p>
      </div>
    </div>
  );
}
