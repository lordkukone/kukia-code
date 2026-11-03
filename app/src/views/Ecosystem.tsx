import { useEffect, useState } from "react";
import { api, type Ecosistema, type NodoEcosistema, type Recomendacion } from "../lib/api";

const ICONO: Record<string, string> = {
  kukia: "🧿",
  opencode: "⌨️",
  freebuff: "🐃",
};

function Tarjeta({ nodo, onDiag }: { nodo: NodoEcosistema; onDiag: () => void }) {
  const color = !nodo.instalado ? "subtle" : nodo.conectado ? "ok" : "warn";
  const estado = !nodo.instalado
    ? "no instalado"
    : nodo.conectado
      ? "conectado"
      : "no configurado / cerrado";
  return (
    <div className="card" style={{ flex: "1 1 320px", minWidth: 300 }}>
      <div className="row-between" style={{ flexWrap: "wrap", gap: 6 }}>
        <h2 style={{ margin: 0 }}>
          {ICONO[nodo.id] ?? "•"} {nodo.nombre}
        </h2>
        <span className={`chip ${color}`}>{estado}</span>
      </div>

      <div className="muted small mono" style={{ marginTop: 4 }}>
        {nodo.version ? `v${nodo.version} · ` : ""}
        {nodo.ruta}
      </div>

      <div className="row" style={{ gap: 14, marginTop: 8, flexWrap: "wrap" }}>
        <span className="small">
          modelos <strong>{nodo.modelos}</strong>
        </span>
        <span className="small">
          agentes <strong>{nodo.agentes}</strong>
        </span>
        <span className="small">
          herramientas <strong>{nodo.herramientas}</strong>
        </span>
        {nodo.procesos > 0 && (
          <span className="small">
            procesos <strong>{nodo.procesos}</strong>
          </span>
        )}
        {nodo.puertos.length > 0 && (
          <span className="mono small">{nodo.puertos.join(" · ")}</span>
        )}
      </div>

      <div className="small" style={{ marginTop: 6 }}>
        {nodo.detalle}
      </div>

      {nodo.puede.length > 0 && (
        <>
          <div className="muted small" style={{ marginTop: 10 }}>
            Verificado
          </div>
          <ul className="small" style={{ margin: "4px 0 0 16px" }}>
            {nodo.puede.map((p) => (
              <li key={p}>{p}</li>
            ))}
          </ul>
        </>
      )}

      {nodo.limitaciones.length > 0 && (
        <>
          <div className="muted small" style={{ marginTop: 8 }}>
            Limitaciones reales
          </div>
          <ul className="muted small" style={{ margin: "4px 0 0 16px" }}>
            {nodo.limitaciones.map((p) => (
              <li key={p}>{p}</li>
            ))}
          </ul>
        </>
      )}

      {nodo.ultimo_error && (
        <div className="small" style={{ marginTop: 8, color: "var(--warn, #c60)" }}>
          ⚠ {nodo.ultimo_error}
        </div>
      )}

      <div className="row" style={{ marginTop: 10, gap: 6 }}>
        <button className="btn ghost small" onClick={onDiag}>
          Diagnóstico
        </button>
      </div>
    </div>
  );
}

export default function Ecosystem({ notify }: { notify: (m: string) => void }) {
  const [eco, setEco] = useState<Ecosistema | null>(null);
  const [ocupado, setOcupado] = useState<string | null>(null);
  const [tarea, setTarea] = useState("Repara el bug del parser y compila los tests");
  const [rec, setRec] = useState<Recomendacion | null>(null);

  const cargar = async (avisar = false) => {
    setOcupado("cargar");
    try {
      const e = await api.ecosistemaEstado();
      setEco(e);
      if (avisar) {
        notify(
          e.nodos
            .map((n) => `${n.nombre}: ${n.conectado ? "conectado" : "no conectado"}`)
            .join(" · "),
        );
      }
    } catch (e) {
      notify(`No se pudo leer el ecosistema: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  useEffect(() => {
    void cargar();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const enrutar = async () => {
    if (!tarea.trim()) return;
    setOcupado("enrutar");
    try {
      const r = await api.ecosistemaEnrutar(tarea);
      setRec(r);
      notify(`Recomendado: ${r.sistema}${r.automatizable ? " (lo puedo lanzar)" : " (a mano)"}`);
    } catch (e) {
      notify(`No se pudo enrutar: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  const conectados = eco?.nodos.filter((n) => n.conectado).length ?? 0;

  return (
    <div className="page">
      <div className="page-head">
        <h1>ECOSISTEMA IA — INTEGRACIONES</h1>
        <span className="chip ok">
          {eco ? `${conectados}/${eco.nodos.length} conectados` : "leyendo…"}
        </span>
        <button
          className="btn ghost small"
          disabled={ocupado !== null}
          onClick={() => void cargar(true)}
        >
          {ocupado === "cargar" ? "Releyendo…" : "Actualizar todo"}
        </button>
      </div>

      <p className="muted small">
        KUKIA consulta a los tres por separado. Un nodo caído no bloquea a los demás, y cada tarjeta
        distingue lo <strong>verificado</strong> de lo que ese programa <strong>no ofrece</strong>.
      </p>

      <div className="row" style={{ gap: 12, flexWrap: "wrap", alignItems: "stretch" }}>
        {eco?.nodos.map((n) => (
          <Tarjeta key={n.id} nodo={n} onDiag={() => void cargar(true)} />
        ))}
        {!eco && <div className="card">Leyendo los tres nodos…</div>}
      </div>

      <div className="card">
        <h2>Catálogo compartido</h2>
        {eco ? (
          <>
            <div className="row" style={{ gap: 22, flexWrap: "wrap" }}>
              <span className="small">
                modelos locales <strong>{eco.catalogo.modelos_locales}</strong>
              </span>
              <span className="small">
                modelos OpenCode <strong>{eco.catalogo.modelos_opencode}</strong>
              </span>
              <span className="small">
                agentes KUKIA <strong>{eco.catalogo.agentes_kukia}</strong> · OpenCode{" "}
                <strong>{eco.catalogo.agentes_opencode}</strong>
              </span>
              <span className="small">
                herramientas <strong>{eco.catalogo.herramientas_kukia}</strong> · MCP{" "}
                <strong>{eco.catalogo.servidores_mcp}</strong>
              </span>
            </div>
            <div className="row" style={{ gap: 6, marginTop: 8, flexWrap: "wrap" }}>
              {eco.catalogo.por_motor.map((c) => (
                <span key={c.nombre} className="chip subtle">
                  local · {c.nombre} {c.cantidad}
                </span>
              ))}
              {eco.catalogo.por_proveedor_opencode.map((c) => (
                <span key={`oc-${c.nombre}`} className="chip subtle">
                  opencode · {c.nombre} {c.cantidad}
                </span>
              ))}
            </div>
          </>
        ) : (
          <div className="muted small">Sin datos todavía.</div>
        )}
      </div>

      <div className="card">
        <h2>Enrutador de tareas</h2>
        <p className="muted small">
          Elige el entorno según capacidades reales. Si la mejor opción no se puede lanzar desde
          KUKIA, lo dice en vez de fingir que la ejecuta.
        </p>
        <div className="row" style={{ gap: 8, flexWrap: "wrap" }}>
          <input
            className="input"
            style={{ flex: 1, minWidth: 280 }}
            value={tarea}
            onChange={(e) => setTarea(e.target.value)}
            placeholder="Describe la tarea…"
          />
          <button className="btn" disabled={ocupado !== null} onClick={() => void enrutar()}>
            {ocupado === "enrutar" ? "Analizando…" : "¿Dónde la lanzo?"}
          </button>
        </div>

        {rec && (
          <div style={{ marginTop: 12 }}>
            <div className="row" style={{ gap: 8, flexWrap: "wrap", alignItems: "center" }}>
              <span className={`chip ${rec.automatizable ? "ok" : "warn"}`}>
                {ICONO[rec.sistema] ?? "•"} {rec.sistema}
              </span>
              <span className="small">
                {rec.automatizable
                  ? "KUKIA puede lanzarla"
                  : "hay que lanzarla a mano en ese programa"}
              </span>
            </div>
            <div className="small" style={{ marginTop: 6 }}>
              {rec.motivo}
            </div>
            <div className="muted small" style={{ marginTop: 4 }}>
              Cómo: {rec.como}
            </div>
            {rec.alternativas.length > 0 && (
              <div className="muted small" style={{ marginTop: 4 }}>
                Alternativas: {rec.alternativas.join(" · ")}
              </div>
            )}
          </div>
        )}
      </div>

      {eco && eco.notas.length > 0 && (
        <div className="card">
          <h2>Notas del entorno</h2>
          <ul className="muted small" style={{ margin: "4px 0 0 16px" }}>
            {eco.notas.map((n) => (
              <li key={n}>{n}</li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}
