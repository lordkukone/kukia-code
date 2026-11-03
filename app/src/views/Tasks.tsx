import { useEffect, useRef, useState } from "react";
import {
  api,
  type Job,
  type JobsEstado,
  type JobStats,
  type ModelCard,
} from "../lib/api";

/** Trabajos que la cola sabe ejecutar de verdad. */
const TIPOS: { id: string; nombre: string; detalle: string }[] = [
  { id: "bench", nombre: "Medir un modelo", detalle: "banco de pruebas: tok/s, prefill, TTFT, picos" },
  { id: "vram_release", nombre: "Liberar VRAM", detalle: "motor, una GPU o ambas (medido antes/después)" },
  { id: "research_scan", nombre: "Escanear fuentes", detalle: "arXiv, Hugging Face o GitHub" },
  { id: "noop", nombre: "Trabajo de prueba", detalle: "espera unos milisegundos y termina" },
];

const colorEstado = (s: string): string => {
  switch (s) {
    case "queued":
      return "subtle";
    case "running":
      return "warn";
    case "done":
      return "ok";
    case "failed":
      return "warn";
    default:
      return "subtle";
  }
};

const etiquetaEstado: Record<string, string> = {
  queued: "en cola",
  running: "ejecutándose",
  done: "terminado",
  failed: "fallido",
  cancelled: "cancelado",
};

const reloj = (iso: string | null): string => {
  if (!iso) return "—";
  const d = new Date(iso);
  return isNaN(d.getTime()) ? iso.slice(11, 19) : d.toLocaleTimeString("es-ES", { hour12: false });
};

export default function Tasks({ notify }: { notify: (m: string) => void }) {
  const [jobs, setJobs] = useState<Job[]>([]);
  const [stats, setStats] = useState<JobStats | null>(null);
  const [estado, setEstado] = useState<JobsEstado | null>(null);
  const [modelos, setModelos] = useState<ModelCard[]>([]);
  const [ocupado, setOcupado] = useState<string | null>(null);

  // Formulario
  const [tipo, setTipo] = useState("bench");
  const [engine, setEngine] = useState("ollama");
  const [model, setModel] = useState("");
  const [tokens, setTokens] = useState(128);
  const [destino, setDestino] = useState("all");
  const [gpu, setGpu] = useState(1);
  const [motorVram, setMotorVram] = useState("ollama");
  const [fuente, setFuente] = useState("all");
  const [ms, setMs] = useState(1500);
  const [prioridad, setPrioridad] = useState(0);

  const vivo = useRef(true);

  const refrescar = async () => {
    try {
      const [l, s, e] = await Promise.all([api.jobsList(60), api.jobsStats(), api.jobsEstado()]);
      if (!vivo.current) return;
      setJobs(l);
      setStats(s);
      setEstado(e);
    } catch (e) {
      notify(`No se pudo leer la cola: ${e}`);
    }
  };

  useEffect(() => {
    vivo.current = true;
    void refrescar();
    void api
      .engineCards("ollama")
      .then((f) => {
        setModelos(f);
        const primero = f.find((x) => x.loaded) ?? f[0];
        if (primero) setModel(primero.id);
      })
      .catch(() => setModelos([]));
    return () => {
      vivo.current = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Mientras haya trabajo vivo, se refresca sola cada 2 s.
  const hayVivo = (stats?.queued ?? 0) > 0 || (stats?.running ?? 0) > 0;
  useEffect(() => {
    if (!hayVivo) return;
    const id = setInterval(() => void refrescar(), 2000);
    return () => clearInterval(id);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [hayVivo]);

  const encolar = async () => {
    let payload: Record<string, unknown> = {};
    if (tipo === "bench") {
      if (!model) {
        notify("Elige un modelo para medir");
        return;
      }
      payload = { engine, model, tokens };
    } else if (tipo === "vram_release") {
      payload =
        destino === "engine"
          ? { target: "engine", engine: motorVram }
          : destino === "gpu"
            ? { target: "gpu", gpu }
            : { target: "all" };
    } else if (tipo === "research_scan") {
      payload = { source: fuente };
    } else {
      payload = { ms };
    }
    setOcupado("encolar");
    try {
      const id = await api.jobsEnqueue(tipo, payload, prioridad);
      notify(`Encolado ${tipo} #${id}`);
      await refrescar();
    } catch (e) {
      notify(`No se pudo encolar: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  const accion = async (clave: string, fn: () => Promise<unknown>, exito: (r: unknown) => string) => {
    setOcupado(clave);
    try {
      const r = await fn();
      notify(exito(r));
      await refrescar();
    } catch (e) {
      notify(`Error: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  return (
    <div className="page">
      <div className="page-head">
        <h1>Tareas</h1>
        <span className={`chip ${estado?.parada ? "warn" : hayVivo ? "ok" : "subtle"}`}>
          {estado?.parada
            ? "PARADA GLOBAL"
            : hayVivo
              ? `${stats?.running ?? 0} ejecutándose · ${stats?.queued ?? 0} en cola`
              : "cola en reposo"}
        </span>
      </div>

      <div className="card">
        <div className="row-between" style={{ flexWrap: "wrap", gap: 10 }}>
          <div className="muted small">
            Cola persistente en SQLite: si cierras la app, los trabajos siguen ahí y lo que estaba a
            medias vuelve a la cola al arrancar. Los workers <strong>no cargan modelos</strong>: miden
            el que esté cargado, liberan lo que tú pidas o escanean fuentes públicas.
          </div>
          <div className="row" style={{ gap: 8, flexWrap: "wrap" }}>
            <label className="field">
              <span>Workers ({estado?.max ?? 4} máx.)</span>
              <select
                value={estado?.activos ?? 2}
                onChange={(e) =>
                  accion(
                    "workers",
                    () => api.jobsWorkers(Number(e.target.value)),
                    (r) => `Workers activos: ${r}`,
                  )
                }
              >
                {[1, 2, 3, 4].map((n) => (
                  <option key={n} value={n}>
                    {n}
                  </option>
                ))}
              </select>
            </label>
            <button className="btn ghost small" onClick={() => void refrescar()}>
              Refrescar
            </button>
            {estado?.parada ? (
              <button
                className="btn"
                disabled={ocupado === "resume"}
                onClick={() => accion("resume", () => api.jobsResume(), () => "Cola reanudada")}
              >
                Reanudar
              </button>
            ) : (
              <button
                className="btn ghost small"
                disabled={ocupado === "stop"}
                onClick={() =>
                  accion("stop", () => api.jobsStop(), () => "Parada global activada")
                }
              >
                Parada global
              </button>
            )}
            <button
              className="btn ghost small"
              onClick={() =>
                accion(
                  "cancelAll",
                  () => api.jobsCancelAll(),
                  (r) => `${r} trabajo(s) pendientes cancelados`,
                )
              }
            >
              Cancelar pendientes
            </button>
            <button
              className="btn ghost small"
              onClick={() => accion("clear", () => api.jobsClear(), () => "Historial limpiado")}
            >
              Limpiar historial
            </button>
          </div>
        </div>

        {stats && (
          <div className="row" style={{ gap: 16, marginTop: 10, flexWrap: "wrap" }}>
            <span className="small">en cola <strong>{stats.queued}</strong></span>
            <span className="small">ejecutándose <strong>{stats.running}</strong></span>
            <span className="small">terminados <strong>{stats.done}</strong></span>
            <span className="small">fallidos <strong>{stats.failed}</strong></span>
            <span className="small">cancelados <strong>{stats.cancelled}</strong></span>
            <span className="muted small">
              espera máxima en cola: {stats.espera_max_s} s
            </span>
          </div>
        )}
      </div>

      <div className="card">
        <h2>Encolar trabajo</h2>
        <div className="row" style={{ gap: 10, flexWrap: "wrap", alignItems: "flex-end" }}>
          <label className="field" style={{ minWidth: 200 }}>
            <span>Tipo</span>
            <select value={tipo} onChange={(e) => setTipo(e.target.value)}>
              {TIPOS.map((t) => (
                <option key={t.id} value={t.id}>
                  {t.nombre}
                </option>
              ))}
            </select>
          </label>

          {tipo === "bench" && (
            <>
              <label className="field">
                <span>Motor</span>
                <select value={engine} onChange={(e) => setEngine(e.target.value)}>
                  <option value="ollama">Ollama</option>
                  <option value="lmstudio">LM Studio</option>
                </select>
              </label>
              <label className="field" style={{ minWidth: 240 }}>
                <span>Modelo</span>
                <select value={model} onChange={(e) => setModel(e.target.value)}>
                  {modelos.length === 0 && <option value="">sin modelos</option>}
                  {modelos.map((m) => (
                    <option key={m.id} value={m.id}>
                      {m.id}
                    </option>
                  ))}
                </select>
              </label>
              <label className="field">
                <span>Tokens</span>
                <input
                  type="number"
                  min={16}
                  max={2048}
                  value={tokens}
                  onChange={(e) => setTokens(Number(e.target.value))}
                />
              </label>
            </>
          )}

          {tipo === "vram_release" && (
            <>
              <label className="field">
                <span>Qué liberar</span>
                <select value={destino} onChange={(e) => setDestino(e.target.value)}>
                  <option value="all">Ambas GPU (todos los motores)</option>
                  <option value="gpu">Una GPU</option>
                  <option value="engine">Un motor</option>
                </select>
              </label>
              {destino === "gpu" && (
                <label className="field">
                  <span>GPU</span>
                  <select value={gpu} onChange={(e) => setGpu(Number(e.target.value))}>
                    <option value={0}>GPU0</option>
                    <option value={1}>GPU1</option>
                  </select>
                </label>
              )}
              {destino === "engine" && (
                <label className="field">
                  <span>Motor</span>
                  <select value={motorVram} onChange={(e) => setMotorVram(e.target.value)}>
                    <option value="ollama">Ollama</option>
                    <option value="lmstudio">LM Studio</option>
                    <option value="comfyui">ComfyUI</option>
                  </select>
                </label>
              )}
            </>
          )}

          {tipo === "research_scan" && (
            <label className="field">
              <span>Fuente</span>
              <select value={fuente} onChange={(e) => setFuente(e.target.value)}>
                <option value="all">Todas</option>
                <option value="arxiv">arXiv</option>
                <option value="huggingface">Hugging Face</option>
                <option value="github">GitHub</option>
              </select>
            </label>
          )}

          {tipo === "noop" && (
            <label className="field">
              <span>Espera (ms)</span>
              <input
                type="number"
                min={100}
                max={10000}
                value={ms}
                onChange={(e) => setMs(Number(e.target.value))}
              />
            </label>
          )}

          <label className="field" style={{ maxWidth: 130 }}>
            <span>Prioridad</span>
            <input
              type="number"
              min={-10}
              max={10}
              value={prioridad}
              onChange={(e) => setPrioridad(Number(e.target.value))}
            />
          </label>

          <button className="btn" onClick={encolar} disabled={ocupado === "encolar"}>
            {ocupado === "encolar" ? "Encolando…" : "Encolar"}
          </button>
        </div>
        <p className="muted small" style={{ marginTop: 8 }}>
          {TIPOS.find((t) => t.id === tipo)?.detalle}. Sale primero lo de prioridad más alta y, a
          igualdad, lo más antiguo.
        </p>
      </div>

      <div className="card">
        <h2>Cola</h2>
        {jobs.length === 0 ? (
          <div className="empty small">No hay trabajos. Encola uno arriba.</div>
        ) : (
          <table className="table">
            <thead>
              <tr>
                <th>#</th>
                <th>Trabajo</th>
                <th>Estado</th>
                <th>Progreso</th>
                <th>Intentos</th>
                <th>Worker</th>
                <th>Creado</th>
                <th>Fin</th>
                <th>Resultado</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {jobs.map((j) => (
                <tr key={j.id}>
                  <td className="mono small">{j.id}</td>
                  <td className="small">
                    <strong>{TIPOS.find((t) => t.id === j.kind)?.nombre ?? j.kind}</strong>
                    <div className="muted small mono">
                      {JSON.stringify(j.payload).slice(0, 90)}
                    </div>
                  </td>
                  <td className="small">
                    <span className={`chip ${colorEstado(j.state)}`}>
                      {etiquetaEstado[j.state] ?? j.state}
                    </span>
                  </td>
                  <td className="small">
                    {j.progress === null || j.progress === undefined ? (
                      <span className="muted">no disponible</span>
                    ) : (
                      `${Math.round(j.progress * 100)} %`
                    )}
                  </td>
                  <td className="small">
                    {j.attempts}/{j.max_attempts}
                  </td>
                  <td className="mono small">{j.worker ?? "—"}</td>
                  <td className="small">{reloj(j.created_at)}</td>
                  <td className="small">{reloj(j.finished_at)}</td>
                  <td className="small">
                    {j.error ? (
                      <span className="warn-text">{j.error.slice(0, 80)}</span>
                    ) : j.result ? (
                      <span className="mono">{JSON.stringify(j.result).slice(0, 90)}</span>
                    ) : (
                      "—"
                    )}
                  </td>
                  <td>
                    {j.state === "queued" && (
                      <button
                        className="btn ghost small"
                        disabled={ocupado === `cancel:${j.id}`}
                        onClick={() =>
                          accion(
                            `cancel:${j.id}`,
                            () => api.jobsCancel(j.id),
                            () => `Trabajo #${j.id} cancelado`,
                          )
                        }
                      >
                        Cancelar
                      </button>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
        <p className="muted small">
          Un trabajo en ejecución no se cancela: se deja terminar para no cortar una generación a
          medias. El progreso solo aparece si el motor lo publica; cuando no, dice «no disponible» en
          vez de inventar un porcentaje.
        </p>
      </div>
    </div>
  );
}
