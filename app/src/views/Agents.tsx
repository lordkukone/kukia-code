import { useEffect, useState } from "react";
import { api, type Agent } from "../lib/api";

export default function Agents({ notify }: { notify: (m: string) => void }) {
  const [agents, setAgents] = useState<Agent[]>([]);
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [prompt, setPrompt] = useState("");
  const [tools, setTools] = useState("files.read, terminal.run");

  const load = () => api.agents().then(setAgents).catch((e) => notify(`Error: ${e}`));

  useEffect(() => {
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const create = async () => {
    if (!name.trim() || !prompt.trim()) {
      notify("Nombre y prompt son obligatorios");
      return;
    }
    try {
      await api.createAgent(name, description, prompt, tools);
      setName("");
      setDescription("");
      setPrompt("");
      await load();
      notify("Agente creado");
    } catch (e) {
      notify(`Error creando agente: ${e}`);
    }
  };

  return (
    <div className="page">
      <div className="page-head">
        <h1>Agentes</h1>
        <span className="chip subtle">{agents.length} configurados</span>
      </div>

      <div className="agent-grid">
        {agents.map((a) => (
          <div key={a.id} className="card agent-card" style={{ borderColor: a.color }}>
            <div className="agent-head">
              <span className="agent-dot" style={{ background: a.color }} />
              <h3>{a.name}</h3>
            </div>
            <p className="muted">{a.description || "Sin descripción"}</p>
            <div className="tags">
              {a.tools.map((t) => (
                <span key={t} className="tag">
                  {t}
                </span>
              ))}
            </div>
          </div>
        ))}
      </div>

      <div className="card">
        <h2>Crear agente</h2>
        <div className="form-grid">
          <label>
            Nombre
            <input value={name} onChange={(e) => setName(e.target.value)} placeholder="KUKIA AUDITOR" />
          </label>
          <label>
            Descripción
            <input
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="Qué hace este agente"
            />
          </label>
          <label className="span-2">
            Prompt de sistema
            <textarea
              rows={4}
              value={prompt}
              onChange={(e) => setPrompt(e.target.value)}
              placeholder="Eres un agente especializado en…"
            />
          </label>
          <label className="span-2">
            Herramientas (separadas por coma)
            <input value={tools} onChange={(e) => setTools(e.target.value)} />
          </label>
        </div>
        <button className="btn primary" onClick={create}>
          Crear agente
        </button>
      </div>
    </div>
  );
}
