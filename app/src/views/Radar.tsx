import { useEffect, useState } from "react";
import { api, type CompatRow, type UpdateInfo } from "../lib/api";

export default function Radar({ notify }: { notify: (m: string) => void }) {
  const [updates, setUpdates] = useState<UpdateInfo[]>([]);
  const [compat, setCompat] = useState<CompatRow[]>([]);
  const [busy, setBusy] = useState(false);
  const [engine, setEngine] = useState("ollama");
  const [model, setModel] = useState("");
  const [arch, setArch] = useState("");
  const [quant, setQuant] = useState("");
  const [gpu, setGpu] = useState("RTX 5070 + RTX 5060 Ti");
  const [method, setMethod] = useState("layer-offload");
  const [notes, setNotes] = useState("");

  const loadCompat = () => api.compatList().then(setCompat).catch((e) => notify(`Error: ${e}`));

  useEffect(() => {
    loadCompat();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const check = async () => {
    setBusy(true);
    try {
      const list = await api.radarCheck();
      setUpdates(list);
      notify(`Radar: ${list.length} fuentes consultadas`);
    } catch (e) {
      notify(`Radar falló: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  const addCompat = async () => {
    if (!model.trim()) {
      notify("Indica al menos el modelo");
      return;
    }
    try {
      await api.compatAdd(engine, model, arch, quant, gpu, method, notes);
      setModel("");
      setArch("");
      setQuant("");
      setNotes("");
      await loadCompat();
      notify("Entrada de compatibilidad registrada");
    } catch (e) {
      notify(`Error: ${e}`);
    }
  };

  return (
    <div className="page">
      <div className="page-head">
        <h1>KUKIA Technology Radar</h1>
        <button className="btn primary" onClick={check} disabled={busy}>
          {busy ? "Consultando…" : "Comprobar novedades"}
        </button>
      </div>

      <div className="card">
        <h2>Fuentes oficiales vigiladas</h2>
        <p className="muted small">
          Sólo se consultan APIs públicas (GitHub Releases y changelogs). KUKIA CODE nunca descarga
          ni instala actualizaciones sin tu autorización.
        </p>
        <table className="table">
          <thead>
            <tr>
              <th>Fuente</th>
              <th>Última versión</th>
              <th>Publicada</th>
              <th>Estado</th>
              <th>Origen</th>
            </tr>
          </thead>
          <tbody>
            {updates.map((u) => (
              <tr key={u.source}>
                <td>{u.source}</td>
                <td className="mono">{u.version || "—"}</td>
                <td>{u.published ? u.published.slice(0, 10) : "—"}</td>
                <td>{u.note}</td>
                <td>
                  {u.url ? (
                    <a href={u.url} target="_blank" rel="noreferrer">
                      oficial
                    </a>
                  ) : (
                    "—"
                  )}
                </td>
              </tr>
            ))}
            {updates.length === 0 && (
              <tr>
                <td colSpan={5} className="muted">
                  Pulsa «Comprobar novedades» para consultar las fuentes.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      <div className="card">
        <h2>Matriz de compatibilidad</h2>
        <p className="muted small">
          Registro de lo realmente probado o documentado. No se marca compatible nada sin evidencia.
        </p>
        <table className="table">
          <thead>
            <tr>
              <th>Motor</th>
              <th>Modelo</th>
              <th>Arquitectura</th>
              <th>Cuantización</th>
              <th>GPU</th>
              <th>Reparto</th>
              <th>Notas</th>
            </tr>
          </thead>
          <tbody>
            {compat.map((c) => (
              <tr key={c.id}>
                <td>{c.engine}</td>
                <td className="mono">{c.model}</td>
                <td>{c.arch || "—"}</td>
                <td>{c.quant || "—"}</td>
                <td>{c.gpu || "—"}</td>
                <td>{c.method || "—"}</td>
                <td>{c.notes || "—"}</td>
              </tr>
            ))}
            {compat.length === 0 && (
              <tr>
                <td colSpan={7} className="muted">
                  Sin entradas todavía.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      <div className="card">
        <h2>Registrar compatibilidad verificada</h2>
        <div className="form-grid">
          <label>
            Motor
            <select value={engine} onChange={(e) => setEngine(e.target.value)}>
              <option>ollama</option>
              <option>lmstudio</option>
              <option>llamacpp</option>
              <option>polystrata</option>
              <option>cloud</option>
            </select>
          </label>
          <label>
            Modelo
            <input value={model} onChange={(e) => setModel(e.target.value)} placeholder="qwen3-coder:30b" />
          </label>
          <label>
            Arquitectura
            <input value={arch} onChange={(e) => setArch(e.target.value)} placeholder="qwen3moe" />
          </label>
          <label>
            Cuantización
            <input value={quant} onChange={(e) => setQuant(e.target.value)} placeholder="Q4_K_M" />
          </label>
          <label>
            GPU
            <input value={gpu} onChange={(e) => setGpu(e.target.value)} />
          </label>
          <label>
            Reparto
            <select value={method} onChange={(e) => setMethod(e.target.value)}>
              <option>layer-offload</option>
              <option>tensor-split</option>
              <option>row-split</option>
              <option>expert-split</option>
              <option>pipeline-parallel</option>
              <option>single-gpu</option>
            </select>
          </label>
          <label className="span-2">
            Notas / evidencia
            <input value={notes} onChange={(e) => setNotes(e.target.value)} placeholder="probado en Fase 2, 42 tok/s…" />
          </label>
        </div>
        <button className="btn" onClick={addCompat}>
          Añadir a la matriz
        </button>
      </div>

      <div className="card">
        <h2>Actualizador modular</h2>
        <p className="muted">
          Detecta versiones y consulta novedades, pero <strong>no instala nada</strong>. El ciclo
          completo (backup → instalación aislada → tests → rollback) se implementará en fases
          posteriores y siempre con tu aprobación explícita.
        </p>
      </div>
    </div>
  );
}
