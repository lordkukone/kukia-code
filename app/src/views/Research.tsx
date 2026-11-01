import { useEffect, useState } from "react";
import { api, type Discovery, type ResearchAlertRow } from "../lib/api";

export default function Research({ notify }: { notify: (m: string) => void }) {
  const [items, setItems] = useState<Discovery[]>([]);
  const [alerts, setAlerts] = useState<ResearchAlertRow[]>([]);
  const [lab, setLab] = useState("");
  const [busy, setBusy] = useState<string | null>(null);
  const [fetchUrl, setFetchUrl] = useState("");
  const [filter, setFilter] = useState("");

  const load = async () => {
    try {
      const [i, a] = await Promise.all([api.researchItems(), api.researchAlerts()]);
      setItems(i);
      setAlerts(a);
    } catch (e) {
      notify(`Error cargando research: ${e}`);
    }
  };

  useEffect(() => {
    load();
    api.researchLabPath().then(setLab).catch(() => setLab("H:\\KUKIA_RESEARCH\\AI_EXPERIMENTAL"));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const scan = async (source: string) => {
    setBusy(source);
    try {
      const list = await api.researchScan(source);
      setItems(list);
      await load();
      notify(`Escaneo ${source}: ${list.length} candidatos en base`);
    } catch (e) {
      notify(`Escaneo falló: ${e}`);
    } finally {
      setBusy(null);
    }
  };

  const report = async () => {
    try {
      const path = await api.researchReport();
      notify(`Informe generado: ${path}`);
    } catch (e) {
      notify(`No se pudo generar el informe: ${e}`);
    }
  };

  const fetchPublic = async () => {
    if (!fetchUrl.trim()) return;
    try {
      const path = await api.researchFetch(fetchUrl.trim());
      notify(`Descargado en laboratorio aislado: ${path}`);
      setFetchUrl("");
    } catch (e) {
      notify(`Descarga bloqueada: ${e}`);
    }
  };

  const shown = items.filter(
    (i) =>
      i.title.toLowerCase().includes(filter.toLowerCase()) ||
      i.summary.toLowerCase().includes(filter.toLowerCase()) ||
      i.source.toLowerCase().includes(filter.toLowerCase()),
  );

  return (
    <div className="page">
      <div className="page-head">
        <h1>KUKIA Experimental AI Radar</h1>
        <div className="row">
          <button className="btn" onClick={report}>
            Generar informe diario
          </button>
        </div>
      </div>

      <div className="card">
        <div className="card-head">
          <h2>Laboratorio aislado</h2>
          <span className="chip subtle mono">{lab}</span>
        </div>
        <div className="row wrap">
          <button className="btn primary" onClick={() => scan("all")} disabled={!!busy}>
            {busy === "all" ? "Escaneando…" : "Escanear todo"}
          </button>
          <button className="btn" onClick={() => scan("arxiv")} disabled={!!busy}>
            arXiv
          </button>
          <button className="btn" onClick={() => scan("huggingface")} disabled={!!busy}>
            Hugging Face
          </button>
          <button className="btn" onClick={() => scan("github")} disabled={!!busy}>
            GitHub
          </button>
          <span className="chip subtle">vigilancia automática cada 6 h (configurable)</span>
        </div>
        <p className="muted small">
          Solo se consultan APIs públicas. Nada se instala en el sistema: las descargas van a
          <code> downloads\</code> del laboratorio y los informes a <code>reports\</code>.
        </p>
      </div>

      <div className="card">
        <div className="card-head">
          <h2>Alertas ({alerts.length})</h2>
        </div>
        {alerts.length === 0 ? (
          <div className="empty small">Sin alertas registradas.</div>
        ) : (
          <div className="alerts">
            {alerts.slice(0, 30).map((a) => (
              <div key={a.id} className={`alert-item ${a.level === "alta" ? "high" : ""}`}>
                <span className={`tag ${a.level === "alta" ? "ok-tag" : ""}`}>{a.level}</span>
                <span>{a.message}</span>
                <span className="muted small mono">{a.created_at.slice(0, 16).replace("T", " ")}</span>
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="card">
        <div className="card-head">
          <h2>Descubrimientos ({shown.length})</h2>
          <input
            className="search"
            placeholder="Filtrar por título, fuente o resumen…"
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
          />
        </div>
        {shown.length === 0 ? (
          <div className="empty small">Sin descubrimientos. Pulsa «Escanear todo».</div>
        ) : (
          <div className="discoveries">
            {shown.map((d) => (
              <details key={d.id} className="discovery">
                <summary>
                  <span className={`tag ${d.score >= 0.6 ? "ok-tag" : ""}`}>
                    {d.score.toFixed(2)}
                  </span>
                  <strong>{d.title}</strong>
                  <span className="muted"> · {d.source} · {d.kind}</span>
                </summary>
                <div className="discovery-body">
                  <div className="kv"><span>Fuente</span><a href={d.url} target="_blank" rel="noreferrer">{d.url || "—"}</a></div>
                  <div className="kv"><span>Fecha</span><span>{d.published ? d.published.slice(0, 10) : "—"}</span></div>
                  <div className="kv"><span>Licencia</span><span>{d.license || "desconocida"}</span></div>
                  <div className="kv"><span>Windows</span><span>{d.windows}</span></div>
                  <div className="kv"><span>GPU</span><span>{d.gpu}</span></div>
                  <div className="kv"><span>VRAM</span><span>{d.vram}</span></div>
                  <div className="kv"><span>Mejora esperada</span><span>{d.expected}</span></div>
                  <div className="kv"><span>Medido</span><span>{d.measured}</span></div>
                  <div className="kv"><span>Riesgos</span><span>{d.risks || "sin evaluar"}</span></div>
                  <div className="kv"><span>Recomendación</span><span>{d.recommendation}</span></div>
                  <div className="kv span"><span>Resumen</span><span>{d.summary || "—"}</span></div>
                  <div className="row">
                    <button
                      className="btn small"
                      onClick={() => {
                        setFetchUrl(d.url);
                        notify("URL cargada en el campo de descarga aislada");
                      }}
                    >
                      Preparar descarga aislada
                    </button>
                  </div>
                </div>
              </details>
            ))}
          </div>
        )}
      </div>

      <div className="card">
        <h2>Descarga aislada (solo https, máx. 25 MB)</h2>
        <div className="row">
          <input
            className="mono"
            value={fetchUrl}
            onChange={(e) => setFetchUrl(e.target.value)}
            placeholder="https://…"
          />
          <button className="btn" onClick={fetchPublic}>
            Descargar al laboratorio
          </button>
        </div>
        <p className="muted small">
          Nunca se ejecuta nada automáticamente. El archivo queda en el laboratorio para revisión
          manual.
        </p>
      </div>
    </div>
  );
}
