import { useEffect, useState } from "react";
import { api, type ModelInfo, type ProviderInfo } from "../lib/api";

interface Props {
  providers: ProviderInfo[];
  models: ModelInfo[];
  provider: string;
  model: string;
  onSelectProvider: (id: string) => void;
  onSelectModel: (id: string) => void;
  onRefresh: () => Promise<ProviderInfo[]>;
  notify: (msg: string) => void;
}

export default function Models(p: Props) {
  const [filter, setFilter] = useState("");
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    setFilter("");
  }, [p.provider]);

  const refresh = async () => {
    setLoading(true);
    const list = await p.onRefresh();
    const current = list.find((x) => x.id === p.provider);
    if (current) await p.onSelectProvider(current.id);
    setLoading(false);
  };

  const shown = p.models.filter(
    (m) =>
      m.id.toLowerCase().includes(filter.toLowerCase()) ||
      m.name.toLowerCase().includes(filter.toLowerCase()),
  );

  return (
    <div className="page">
      <div className="page-head">
        <h1>Modelos</h1>
        <button className="btn" onClick={refresh} disabled={loading}>
          {loading ? "Escaneando…" : "Refrescar"}
        </button>
      </div>

      <div className="provider-grid">
        {p.providers.map((pr) => (
          <button
            key={pr.id}
            className={`provider-card ${pr.id === p.provider ? "active" : ""}`}
            onClick={() => p.onSelectProvider(pr.id)}
          >
            <div className="provider-top">
              <span className="provider-name">{pr.name}</span>
              <span className={`status-dot ${pr.online ? "online" : "offline"}`} />
            </div>
            <div className="provider-meta">{pr.kind}</div>
            <div className="provider-meta mono">{pr.base_url || "—"}</div>
            <div className="provider-bottom">
              <span>{pr.models_count} modelos</span>
              <span className={pr.online ? "ok-text" : "warn-text"}>
                {pr.online ? "online" : "offline"}
              </span>
            </div>
            {pr.note && <div className="provider-note">{pr.note}</div>}
          </button>
        ))}
      </div>

      <div className="card">
        <div className="card-head">
          <h2>Catálogo · {p.provider}</h2>
          <input
            className="search"
            placeholder="Filtrar modelos…"
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
          />
        </div>
        {shown.length === 0 ? (
          <div className="empty small">
            No hay modelos visibles. Si el proveedor está offline, arráncalo y pulsa Refrescar.
          </div>
        ) : (
          <table className="table">
            <thead>
              <tr>
                <th>Modelo</th>
                <th>Contexto</th>
                <th>Tamaño</th>
                <th>Capacidades</th>
                <th>Estado</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {shown.map((m) => (
                <tr key={m.id} className={m.id === p.model ? "selected" : ""}>
                  <td className="mono">{m.name}</td>
                  <td>{m.context ? m.context.toLocaleString() : "—"}</td>
                  <td>{m.size_gb ? `${m.size_gb.toFixed(1)} GB` : "—"}</td>
                  <td>{m.capabilities.join(", ") || "texto"}</td>
                  <td>{m.state}</td>
                  <td>
                    <button
                      className="btn small"
                      onClick={() => {
                        p.onSelectModel(m.id);
                        p.notify(`Modelo activo: ${m.name}`);
                      }}
                    >
                      Usar
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}
