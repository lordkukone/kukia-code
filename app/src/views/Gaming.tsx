import { useEffect, useState } from "react";
import { api, type GamingAlertRow, type GamingItem, type GamingScan } from "../lib/api";

export default function Gaming({ notify }: { notify: (m: string) => void }) {
  const [items, setItems] = useState<GamingItem[]>([]);
  const [alerts, setAlerts] = useState<GamingAlertRow[]>([]);
  const [lab, setLab] = useState("");
  const [resumen, setResumen] = useState<GamingScan | null>(null);
  const [escaneando, setEscaneando] = useState(false);

  const cargar = async () => {
    try {
      const [i, a, l] = await Promise.all([
        api.gamingItems(200),
        api.gamingAlerts(100),
        api.gamingLabPath(),
      ]);
      setItems(i);
      setAlerts(a);
      setLab(l);
    } catch (e) {
      notify(`Error cargando el radar: ${e}`);
    }
  };

  const escanear = async () => {
    setEscaneando(true);
    try {
      const r = await api.gamingScan();
      setResumen(r);
      notify(`${r.encontrados} encontrados · ${r.nuevos} nuevos · ${r.alertas} alertas`);
      await cargar();
    } catch (e) {
      notify(`Escaneo fallido: ${e}`);
    } finally {
      setEscaneando(false);
    }
  };

  useEffect(() => {
    cargar();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const colorNivel = (nivel: string) =>
    nivel === "riesgo" ? "warn" : nivel === "alta" ? "ok" : "subtle";

  return (
    <div className="page">
      <div className="page-head">
        <h1>KUKIA Future Gaming Radar</h1>
        <span className="chip subtle">{items.length} fichas</span>
      </div>

      <div className="card">
        <div className="row-between">
          <div className="muted">
            Consulta releases, búsqueda de GitHub y arXiv. Solo lectura: no descarga ejecutables ni
            toca juegos, drivers o configuraciones.
            <br />
            Laboratorio: <span className="mono">{lab}</span>
          </div>
          <button className="btn" onClick={escanear} disabled={escaneando}>
            {escaneando ? "Escaneando…" : "Escanear ahora"}
          </button>
        </div>
        {resumen && (
          <p className="muted small">
            Último escaneo: <strong>{resumen.encontrados}</strong> encontrados ·{" "}
            <strong>{resumen.nuevos}</strong> nuevos · <strong>{resumen.alertas}</strong> alertas
            {resumen.informe && (
              <>
                {" "}
                · informe <span className="mono">{resumen.informe}</span>
              </>
            )}
          </p>
        )}
      </div>

      <div className="card">
        <h2>Alertas</h2>
        {alerts.length === 0 && (
          <div className="empty small">Sin alertas todavía. Escanea para generarlas.</div>
        )}
        {alerts.slice(0, 25).map((a) => (
          <div key={a.id} className="row" style={{ marginBottom: 6 }}>
            <span className={`chip ${colorNivel(a.level)}`}>{a.level}</span>
            <span className="small">{a.message}</span>
            <span className="muted small">{a.created_at.slice(0, 10)}</span>
          </div>
        ))}
      </div>

      <div className="card">
        <h2>Descubrimientos</h2>
        {items.length === 0 && (
          <div className="empty small">
            Sin fichas. Pulsa Escanear: consulta release de 9 repos, 6 búsquedas y 5 términos de
            arXiv.
          </div>
        )}
        <table className="table">
          <thead>
            <tr>
              <th>Nombre</th>
              <th>Categoría</th>
              <th>Estado</th>
              <th>5070</th>
              <th>5060 Ti</th>
              <th>Puntos</th>
            </tr>
          </thead>
          <tbody>
            {items.map((it) => (
              <tr key={it.id}>
                <td>
                  {it.url ? (
                    <a href={it.url} target="_blank" rel="noreferrer">
                      {it.name}
                    </a>
                  ) : (
                    it.name
                  )}
                  <div className="muted small">{it.source}</div>
                </td>
                <td className="small">{it.category}</td>
                <td className="small">
                  <span
                    className={`tag ${it.status === "RIESGO_ALTO" ? "warn" : it.status === "ESTABLE" ? "ok-tag" : ""}`}
                  >
                    {it.status}
                  </span>
                </td>
                <td className="small">{it.rtx5070}</td>
                <td className="small">{it.rtx5060ti}</td>
                <td className="small">{it.score.toFixed(1)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {items.some((i) => i.recommendation) && (
        <div className="card">
          <h2>Recomendaciones del radar</h2>
          {items
            .filter((i) => i.recommendation)
            .slice(0, 12)
            .map((i) => (
              <p key={i.id} className="small">
                <strong>{i.name}</strong>: {i.recommendation}
                {i.risks && <span className="muted"> · riesgos: {i.risks}</span>}
              </p>
            ))}
        </div>
      )}
    </div>
  );
}
