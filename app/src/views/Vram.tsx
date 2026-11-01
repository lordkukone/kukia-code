import { useEffect, useState } from "react";
import { api, type VramReport } from "../lib/api";

export default function Vram({ notify }: { notify: (m: string) => void }) {
  const [report, setReport] = useState<VramReport | null>(null);
  const [cargando, setCargando] = useState(false);
  const [tamano, setTamano] = useState(13.5);
  const [gpu, setGpu] = useState(1);
  const [cabe, setCabe] = useState<[boolean, string] | null>(null);

  // checked_at viene en RFC3339 con offset 0 (UTC). Se muestra en hora local:
  // antes se cortaba la cadena y se enseñaba la hora UTC como si fuera local.
  const horaLocal = (iso: string): string => {
    const d = new Date(iso);
    if (isNaN(d.getTime())) return iso.length >= 19 ? iso.slice(11, 19) : iso;
    return d.toLocaleTimeString("es-ES", {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
      hour12: false,
    });
  };

  const cargar = async () => {
    setCargando(true);
    try {
      const r = await api.vramReport();
      setReport(r);
      // Si la GPU elegida no existe en esta máquina, se selecciona la primera
      // (con una sola GPU el desplegable quedaba vacío y «Comprobar» fallaba).
      const gpus = r.gpus ?? [];
      setGpu((prev) => (gpus.some((g) => g.index === prev) ? prev : (gpus[0]?.index ?? prev)));
    } catch (e) {
      notify(`Error leyendo VRAM: ${e}`);
    } finally {
      setCargando(false);
    }
  };

  const comprobar = async () => {
    try {
      setCabe(await api.vramFits(tamano, gpu));
    } catch (e) {
      notify(`Error calculando: ${e}`);
    }
  };

  useEffect(() => {
    cargar();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const pct = (usado: number, total: number) => (total > 0 ? Math.min(100, (usado / total) * 100) : 0);

  return (
    <div className="page">
      <div className="page-head">
        <h1>Guardia de VRAM</h1>
        <span className={`chip ${report?.competition ? "warn" : report ? "ok" : "subtle"}`}>
          {report
            ? report.competition
              ? "competencia de VRAM"
              : "sin competencia"
            : "sin datos"}
        </span>
      </div>

      <div className="card">
        <div className="row-between">
          <div className="muted">
            Solo lectura: informa y avisa, no cierra procesos ni descarga modelos.
            {report && <> Última lectura: {horaLocal(report.checked_at)}.</>}
          </div>
          <button className="btn ghost" onClick={cargar} disabled={cargando}>
            {cargando ? "Midiendo…" : "Volver a medir"}
          </button>
        </div>
      </div>

      {report?.competition && (
        <div className="card" style={{ borderLeft: "3px solid #b45309" }}>
          <strong>{report.warning}</strong>
          {report.recommendations.map((r) => (
            <p key={r} className="muted small">
              {r}
            </p>
          ))}
        </div>
      )}

      <div className="split">
        <div className="card">
          <h2>GPUs</h2>
          {report?.gpus.map((g) => (
            <div key={g.index} style={{ marginBottom: 14 }}>
              <div className="row-between">
                <span className="small">
                  <strong>GPU{g.index}</strong> · {g.name.replace("NVIDIA GeForce ", "")}
                </span>
                <span className="muted small">
                  {(g.used_mb / 1024).toFixed(1)} / {(g.total_mb / 1024).toFixed(1)} GB
                </span>
              </div>
              <div
                style={{
                  height: 8,
                  background: "var(--bg-3)",
                  borderRadius: 4,
                  overflow: "hidden",
                  marginTop: 6,
                }}
              >
                <div
                  style={{
                    width: `${pct(g.used_mb, g.total_mb)}%`,
                    height: "100%",
                    background: pct(g.used_mb, g.total_mb) > 85 ? "#b45309" : "var(--accent)",
                  }}
                />
              </div>
              <div className="muted small" style={{ marginTop: 4 }}>
                libres {(g.free_mb / 1024).toFixed(1)} GB · reservada{" "}
                {(g.reserved_mb / 1024).toFixed(1)} GB · uso {g.utilization} %
              </div>
            </div>
          ))}
          {!report && <div className="empty small">Sin lectura todavía.</div>}
        </div>

        <div className="card">
          <h2>Motores vigilados</h2>
          <table className="table">
            <thead>
              <tr>
                <th>Motor</th>
                <th>Modelos</th>
                <th>VRAM</th>
                <th>Estado</th>
              </tr>
            </thead>
            <tbody>
              {report?.engines.map((e) => (
                <tr key={e.engine}>
                  <td className="mono">{e.engine}</td>
                  <td>
                    {e.models.length === 0 ? (
                      <span className="muted">—</span>
                    ) : (
                      e.models.map((m) => (
                        <div key={m.name} className="small">
                          {m.name}
                        </div>
                      ))
                    )}
                  </td>
                  <td className="small">
                    {e.vram_total_mb > 0 ? `${(e.vram_total_mb / 1024).toFixed(2)} GB` : "—"}
                  </td>
                  <td className="small">
                    <span className={`tag ${e.online ? "ok-tag" : ""}`}>
                      {e.online ? "activo" : "sin respuesta"}
                    </span>
                    <div className="muted small">{e.note}</div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      <div className="card">
        <h2>¿Cabe este modelo?</h2>
        <div className="row">
          <label className="field">
            <span>Tamaño del modelo (GB)</span>
            <input
              type="number"
              step={0.5}
              value={tamano}
              onChange={(e) => setTamano(Number(e.target.value))}
            />
          </label>
          <label className="field">
            <span>GPU</span>
            <select value={gpu} onChange={(e) => setGpu(Number(e.target.value))}>
              {report?.gpus.map((g) => (
                <option key={g.index} value={g.index}>
                  GPU{g.index} · {g.name.replace("NVIDIA GeForce ", "")}
                </option>
              ))}
            </select>
          </label>
          <button className="btn" onClick={comprobar} style={{ alignSelf: "flex-end" }}>
            Comprobar
          </button>
        </div>
        {cabe && (
          <p className={cabe[0] ? "muted" : "warn-text"}>
            <span className={`tag ${cabe[0] ? "ok-tag" : ""}`}>{cabe[0] ? "cabe" : "no cabe"}</span>{" "}
            {cabe[1]}
          </p>
        )}
        <p className="muted small">
          Incluye un 12 % de margen para contexto y caché KV. La decisión sigue siendo tuya.
        </p>
      </div>

      <div className="card">
        <h2>Procesos con memoria en GPU</h2>
        {report && report.processes.filter((p) => p.vram_mb > 0).length === 0 && (
          <div className="empty small">
            El driver no atribuye MB por proceso en esta máquina (WDDM): nvidia-smi lista los
            procesos pero devuelve N/A en memoria. Por eso se mide por motor, no por proceso.
          </div>
        )}
        <table className="table">
          <tbody>
            {report?.processes
              .filter((p) => p.vram_mb > 0)
              .map((p) => (
                <tr key={`${p.pid}-${p.name}`}>
                  <td className="mono small">PID {p.pid}</td>
                  <td className="small">{p.name.split("\\").pop()}</td>
                  <td className="small">{p.vram_mb} MB</td>
                </tr>
              ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
