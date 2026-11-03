import { useEffect, useState } from "react";
import { api, type MemoriaXoxete, type XoxeteEstado } from "../lib/api";

/**
 * XOXETE — su instalación real, vista desde KUKIA CODE.
 *
 * Lo que enseña viene de `H:\Xoxete`: la configuración de OpenCode (agente y
 * modelo por defecto), las instrucciones, los plugins presentes, los binarios y
 * el motor de memoria SQLite. La búsqueda y el guardado usan el mismo motor y la
 * misma base que emplea el agente, sin copiarla ni duplicarla.
 */
export default function Xoxete({ notify }: { notify: (m: string) => void }) {
  const [estado, setEstado] = useState<XoxeteEstado | null>(null);
  const [consulta, setConsulta] = useState("");
  const [memorias, setMemorias] = useState<MemoriaXoxete[] | null>(null);
  const [ocupado, setOcupado] = useState<string | null>(null);

  const [nueva, setNueva] = useState({
    tipo: "experiencia",
    contexto: "",
    accion: "",
    resultado: "",
    evidencia: "",
  });

  const cargar = async (avisar = false) => {
    setOcupado("estado");
    try {
      const e = await api.xoxeteEstado();
      setEstado(e);
      if (avisar) notify(`Xoxete leído · ${e.motor.total} recuerdo(s) en su base`);
    } catch (e) {
      notify(`No se pudo leer Xoxete: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  useEffect(() => {
    void cargar();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const buscar = async () => {
    if (consulta.trim().length < 3) {
      notify("Escribe al menos 3 caracteres para buscar");
      return;
    }
    setOcupado("buscar");
    try {
      const r = await api.xoxeteMemorias(consulta.trim(), 12);
      setMemorias(r);
      notify(r.length === 0 ? "Sin recuerdos para esa búsqueda" : `${r.length} recuerdo(s)`);
    } catch (e) {
      notify(`La búsqueda falló: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  const guardar = async () => {
    if (nueva.contexto.trim() === "" && nueva.resultado.trim() === "") {
      notify("Hace falta contexto o resultado");
      return;
    }
    if (
      !window.confirm(
        "Se escribirá en la base de memoria ORIGINAL de Xoxete (la misma que usa su agente). ¿Continuar?",
      )
    )
      return;
    setOcupado("guardar");
    try {
      const msg = await api.xoxeteRecordar(nueva);
      notify(msg);
      setNueva({ tipo: "experiencia", contexto: "", accion: "", resultado: "", evidencia: "" });
      await cargar();
    } catch (e) {
      notify(`No se pudo guardar: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  const m = estado?.motor;

  return (
    <div className="page">
      <div className="page-head">
        <h1>XOXETE</h1>
        <span className={`chip ${estado?.config_ok ? "ok" : "warn"}`}>
          {estado ? `${estado.agente || "sin agente"} · ${estado.modelo || "sin modelo"}` : "leyendo…"}
        </span>
      </div>

      <div className="row" style={{ gap: 8, marginBottom: 10, flexWrap: "wrap" }}>
        <button className="btn ghost small" disabled={ocupado !== null} onClick={() => void cargar(true)}>
          {ocupado === "estado" ? "Releyendo…" : "Releer estado"}
        </button>
        <span className="muted small">{estado?.raiz}</span>
      </div>

      {estado && (
        <div className="card">
          <h2>Instalación</h2>
          <div className="row" style={{ gap: 20, flexWrap: "wrap" }}>
            <span className="small">
              config <strong>{estado.config_ok ? "legible" : "ilegible"}</strong>
            </span>
            <span className="small">
              instrucciones <strong>{estado.instrucciones_ok}</strong>
            </span>
            <span className="small">
              plugins <strong>{estado.plugins.length}</strong>
            </span>
            <span className="small">
              motor memoria{" "}
              <strong>{m?.disponible ? "responde" : m?.error ? "error" : "no disponible"}</strong>
            </span>
          </div>
          <div className="mono small" style={{ marginTop: 6 }}>
            {estado.config}
          </div>
          {estado.instrucciones.length > 0 && (
            <ul className="muted small" style={{ marginTop: 8 }}>
              {estado.instrucciones.map((i) => (
                <li key={i}>{i}</li>
              ))}
            </ul>
          )}
        </div>
      )}

      {estado && (
        <div className="card">
          <h2>Plugins de su cerebro</h2>
          <table className="table">
            <thead>
              <tr>
                <th>Fichero</th>
                <th>Tamaño</th>
                <th>Estado</th>
              </tr>
            </thead>
            <tbody>
              {estado.plugins.map((p) => (
                <tr key={p.nombre}>
                  <td className="mono small">{p.nombre}</td>
                  <td className="small">{(p.bytes / 1024).toFixed(1)} KB</td>
                  <td className="small">
                    <span className={`chip ${p.presente ? "ok" : "warn"}`}>
                      {p.presente ? "presente" : "ausente"}
                    </span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {estado && (
        <div className="card">
          <h2>Binarios</h2>
          <table className="table">
            <thead>
              <tr>
                <th>Cuál</th>
                <th>Versión</th>
                <th>Ruta</th>
              </tr>
            </thead>
            <tbody>
              {estado.binarios.map((b) => (
                <tr key={b.nombre}>
                  <td className="small">
                    <strong>{b.nombre}</strong>
                  </td>
                  <td className="small">
                    <span className={`chip ${b.existe ? "ok" : "warn"}`}>
                      {b.existe ? b.version || "?" : "no está"}
                    </span>
                  </td>
                  <td className="mono small">{b.ruta}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {estado && (
        <div className="card">
          <h2>Memoria persistente ({m?.total ?? 0} recuerdos)</h2>
          <p className="muted small">
            Misma base y mismo motor que usa el agente: <span className="mono">{m?.base}</span> (
            {((m?.bytes ?? 0) / 1024).toFixed(0)} KB). Leer no la modifica.
          </p>
          {m && m.por_tipo.length > 0 && (
            <div className="row" style={{ gap: 12, flexWrap: "wrap", marginBottom: 8 }}>
              {m.por_tipo.map((c) => (
                <span key={c.nombre} className="small">
                  {c.nombre} <strong>{c.cantidad}</strong>
                </span>
              ))}
            </div>
          )}
          <div className="row" style={{ gap: 8, flexWrap: "wrap" }}>
            <input
              className="input"
              style={{ flex: 1, minWidth: 240 }}
              value={consulta}
              onChange={(e) => setConsulta(e.target.value)}
              placeholder="Buscar en sus recuerdos (ej. opencode, LTX, memoria)"
              onKeyDown={(e) => {
                if (e.key === "Enter") void buscar();
              }}
            />
            <button className="btn" disabled={ocupado !== null} onClick={() => void buscar()}>
              {ocupado === "buscar" ? "Buscando…" : "Buscar"}
            </button>
          </div>
          {memorias && (
            <table className="table" style={{ marginTop: 10 }}>
              <thead>
                <tr>
                  <th>#</th>
                  <th>Tipo</th>
                  <th>Recuerdo</th>
                  <th>Confianza</th>
                </tr>
              </thead>
              <tbody>
                {memorias.map((x) => (
                  <tr key={x.id}>
                    <td className="mono small">{x.id}</td>
                    <td className="small">
                      {x.tipo}
                      <div className="muted small">{x.estado}</div>
                    </td>
                    <td className="small">
                      {x.contexto}
                      {x.accion && <div className="muted small">→ {x.accion}</div>}
                      {x.resultado && <div className="muted small">{x.resultado}</div>}
                    </td>
                    <td className="small">{x.confianza}/5</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
          {memorias && memorias.length === 0 && (
            <div className="empty small">Nada guardado sobre eso.</div>
          )}
        </div>
      )}

      <div className="card">
        <h2>Añadir un recuerdo</h2>
        <p className="muted small">
          Escribe en la base original de Xoxete, igual que su herramienta `memoria_xoxete`. Se pide
          confirmación antes de guardar.
        </p>
        <div className="row" style={{ gap: 8, flexWrap: "wrap" }}>
          <label className="field">
            <span>Tipo</span>
            <select
              value={nueva.tipo}
              onChange={(e) => setNueva({ ...nueva, tipo: e.target.value })}
            >
              {["experiencia", "solucion", "conocimiento", "correccion", "error", "proyecto"].map(
                (t) => (
                  <option key={t} value={t}>
                    {t}
                  </option>
                ),
              )}
            </select>
          </label>
          <label className="field" style={{ flex: 1, minWidth: 240 }}>
            <span>Contexto</span>
            <input
              className="input"
              value={nueva.contexto}
              onChange={(e) => setNueva({ ...nueva, contexto: e.target.value })}
            />
          </label>
        </div>
        <div className="row" style={{ gap: 8, flexWrap: "wrap", marginTop: 6 }}>
          <label className="field" style={{ flex: 1, minWidth: 200 }}>
            <span>Qué se hizo</span>
            <input
              className="input"
              value={nueva.accion}
              onChange={(e) => setNueva({ ...nueva, accion: e.target.value })}
            />
          </label>
          <label className="field" style={{ flex: 1, minWidth: 200 }}>
            <span>Qué ocurrió</span>
            <input
              className="input"
              value={nueva.resultado}
              onChange={(e) => setNueva({ ...nueva, resultado: e.target.value })}
            />
          </label>
          <label className="field" style={{ flex: 1, minWidth: 160 }}>
            <span>Evidencia</span>
            <input
              className="input"
              value={nueva.evidencia}
              onChange={(e) => setNueva({ ...nueva, evidencia: e.target.value })}
            />
          </label>
        </div>
        <button
          className="btn"
          style={{ marginTop: 8 }}
          disabled={ocupado !== null}
          onClick={() => void guardar()}
        >
          {ocupado === "guardar" ? "Guardando…" : "Guardar en la memoria de Xoxete"}
        </button>
      </div>

      {estado && estado.notas.length > 0 && (
        <div className="card">
          <h2>Notas y límites reales</h2>
          <ul className="muted small">
            {estado.notas.map((n) => (
              <li key={n}>{n}</li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}
