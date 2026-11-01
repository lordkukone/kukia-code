import { useEffect, useState } from "react";
import { api, type BenchmarkResult } from "../lib/api";

interface Props {
  provider: string;
  model: string;
  notify: (m: string) => void;
}

export default function Benchmarks({ provider, model, notify }: Props) {
  const [prompt, setPrompt] = useState("Explica en 3 frases qué es una GPU y para qué sirve.");
  const [busy, setBusy] = useState(false);
  const [results, setResults] = useState<BenchmarkResult[]>([]);

  const load = () => api.benchmarks().then(setResults).catch((e) => notify(`Error: ${e}`));

  useEffect(() => {
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const run = async () => {
    if (!model) {
      notify("Selecciona un modelo primero");
      return;
    }
    setBusy(true);
    try {
      const r = await api.benchmark(provider, model, prompt);
      notify(`✓ ${r.tokens_per_s.toFixed(1)} tok/s · ${r.total_ms} ms`);
      await load();
    } catch (e) {
      notify(`Benchmark fallido: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  const exportResults = async () => {
    try {
      await api.runCommand(
        `python -c "import json;print('export pendiente')"`,
        "H:\\KUKIA_CODE",
        true,
      );
      notify("Exportación CSV/JSON disponible en la próxima iteración");
    } catch {
      notify("Exportación no disponible");
    }
  };

  return (
    <div className="page">
      <div className="page-head">
        <h1>Benchmarks</h1>
        <button className="btn" onClick={exportResults}>
          Exportar CSV/JSON
        </button>
      </div>

      <div className="card">
        <h2>Ejecutar benchmark</h2>
        <div className="row">
          <span className="chip">{provider}</span>
          <span className="chip model">{model || "sin modelo"}</span>
        </div>
        <textarea rows={3} value={prompt} onChange={(e) => setPrompt(e.target.value)} />
        <button className="btn primary" onClick={run} disabled={busy}>
          {busy ? "Midiendo…" : "Medir tokens/s"}
        </button>
      </div>

      <div className="card">
        <h2>Historial</h2>
        {results.length === 0 ? (
          <div className="empty small">Sin benchmarks todavía.</div>
        ) : (
          <table className="table">
            <thead>
              <tr>
                <th>Fecha</th>
                <th>Proveedor</th>
                <th>Modelo</th>
                <th>Prompt tok</th>
                <th>Salida tok</th>
                <th>ms</th>
                <th>tok/s</th>
              </tr>
            </thead>
            <tbody>
              {results.map((r) => (
                <tr key={r.id}>
                  <td className="mono">{r.created_at.replace("T", " ").slice(0, 19)}</td>
                  <td>{r.provider}</td>
                  <td className="mono">{r.model}</td>
                  <td>{r.prompt_tokens}</td>
                  <td>{r.completion_tokens}</td>
                  <td>{r.total_ms}</td>
                  <td>
                    <strong>{r.tokens_per_s.toFixed(1)}</strong>
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
