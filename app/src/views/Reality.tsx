import { useEffect, useState } from "react";
import {
  api,
  type StudioDiagnostico,
  type StudioMotor,
  type StudioPreflight,
  type StudioPresupuesto,
} from "../lib/api";

/** Color de un estado del catálogo: verde solo si de verdad puede ejecutarse. */
function colorEstado(estado: string): string {
  switch (estado) {
    case "listo":
      return "#3fb950";
    case "listo_sin_probar":
      return "#d29922";
    case "experimental":
      return "#a371f7";
    case "faltan_modelos":
      return "#f85149";
    case "motor_offline":
      return "#8b949e";
    default:
      return "#8b949e";
  }
}

function Badge({ texto, color }: { texto: string; color: string }) {
  return (
    <span
      className="tag"
      style={{ borderColor: color, color, borderWidth: 1, borderStyle: "solid" }}
      title={texto}
    >
      {texto.replace(/_/g, " ")}
    </span>
  );
}

function Barra({ usado, total, nombre }: { usado: number; total: number; nombre: string }) {
  const pct = total > 0 ? Math.min(100, (usado / total) * 100) : 0;
  const color = pct > 92 ? "#f85149" : pct > 75 ? "#d29922" : "#3fb950";
  return (
    <div style={{ marginBottom: 8 }}>
      <div className="row-between small">
        <span>{nombre}</span>
        <span className="mono">
          {(usado / 1024).toFixed(1)} / {(total / 1024).toFixed(1)} GB usados
        </span>
      </div>
      <div style={{ height: 8, background: "#22262b", borderRadius: 4, overflow: "hidden" }}>
        <div style={{ width: `${pct}%`, height: "100%", background: color }} />
      </div>
    </div>
  );
}

export default function Reality({ notify }: { notify: (m: string) => void }) {
  const [motores, setMotores] = useState<StudioMotor[]>([]);
  const [presupuesto, setPresupuesto] = useState<StudioPresupuesto | null>(null);
  const [catalogo, setCatalogo] = useState<StudioDiagnostico[]>([]);
  const [cargando, setCargando] = useState(false);

  const [ruta, setRuta] = useState("");
  const [destino, setDestino] = useState("");
  const [umbral, setUmbral] = useState(0.3);
  const [analizando, setAnalizando] = useState(false);
  const [preflight, setPreflight] = useState<StudioPreflight | null>(null);

  const cargarMotores = async () => {
    try {
      const [m, p] = await Promise.all([api.studioMotores(), api.studioPresupuesto()]);
      setMotores(m);
      setPresupuesto(p);
    } catch (e) {
      notify(`Error leyendo motores: ${e}`);
    }
  };

  const diagnosticar = async () => {
    setCargando(true);
    try {
      const [c, p] = await Promise.all([api.studioDiagnostico(), api.studioPresupuesto()]);
      setCatalogo(c);
      setPresupuesto(p);
      const listos = c.filter((x) => x.estado_real === "listo").length;
      notify(`Diagnóstico: ${listos} estilo(s) listos de ${c.length}`);
    } catch (e) {
      notify(`Error en el diagnóstico: ${e}`);
    } finally {
      setCargando(false);
    }
  };

  const analizar = async () => {
    if (!ruta.trim()) {
      notify("Escribe la ruta de un archivo de vídeo o imagen");
      return;
    }
    setAnalizando(true);
    setPreflight(null);
    try {
      const p = await api.studioPreflight(ruta.trim(), destino || undefined, undefined, umbral);
      setPreflight(p);
      notify(`Preflight: ${p.veredicto} · ${p.planos.length} plano(s)`);
    } catch (e) {
      notify(`No se pudo analizar: ${e}`);
    } finally {
      setAnalizando(false);
    }
  };

  useEffect(() => {
    void cargarMotores();
  }, []);

  return (
    <div className="page">
      <div className="page-head row-between">
        <div>
          <h1>Convertidor Universal</h1>
          <p className="muted small">
            RC1: registro de motores y catálogo de estilos con diagnóstico real. RC2: detección de
            estilo con VLM local, segmentación de planos y preflight. Nada de esto ejecuta difusión.
          </p>
        </div>
        <div className="row">
          <button className="btn ghost" onClick={() => void cargarMotores()}>
            Refrescar motores
          </button>
          <button className="btn" onClick={() => void diagnosticar()} disabled={cargando}>
            {cargando ? "Diagnosticando…" : "Diagnóstico"}
          </button>
        </div>
      </div>

      {presupuesto && (
        <div className="card">
          <h2>Presupuesto de memoria</h2>
          {presupuesto.vram.length === 0 && (
            <p className="empty small">nvidia-smi no devolvió tarjetas</p>
          )}
          {presupuesto.vram.map((g) => (
            <Barra key={g.index} nombre={`GPU${g.index} · ${g.nombre}`} usado={g.usada_mb} total={g.total_mb} />
          ))}
          <p className="small muted">
            RAM: {presupuesto.ram_libre_gb.toFixed(1)} GB libres de {presupuesto.ram_total_gb.toFixed(1)} GB
          </p>
        </div>
      )}

      <div className="card">
        <h2>Motores</h2>
        {motores.length === 0 ? (
          <p className="empty small">sin datos todavía</p>
        ) : (
          <div className="table">
            {motores.map((m) => (
              <div key={m.id} className="row-between" style={{ padding: "6px 0" }}>
                <div>
                  <strong>{m.nombre}</strong>{" "}
                  <Badge texto={m.online ? "online" : "offline"} color={m.online ? "#3fb950" : "#f85149"} />{" "}
                  <span className="mono small">{m.url}</span>
                  <div className="muted small">
                    {m.version ? `v${m.version} · ` : ""}
                    {m.detalle} · {m.gpu}
                  </div>
                  {m.capacidades.length > 0 && (
                    <div className="small mono" style={{ opacity: 0.8 }}>
                      {m.capacidades.join(" · ")}
                    </div>
                  )}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="card">
        <h2>Catálogo de estilos</h2>
        {catalogo.length === 0 ? (
          <p className="empty small">
            Pulsa «Diagnóstico» para cruzar el catálogo con los modelos instalados y la memoria libre.
          </p>
        ) : (
          <div className="table">
            <div className="row-between small muted" style={{ paddingBottom: 6 }}>
              <span>Estilo</span>
              <span>Estado real</span>
            </div>
            {catalogo.map((e) => (
              <div key={e.id} style={{ padding: "8px 0", borderTop: "1px solid #22262b" }}>
                <div className="row-between">
                  <div>
                    <strong>{e.titulo}</strong>{" "}
                    <span className="muted small">
                      {e.origen.join("/")} → {e.destino} · {e.modalidad} · {e.motor}
                    </span>
                  </div>
                  <div className="row" style={{ gap: 6 }}>
                    <Badge texto={e.estado_real} color={colorEstado(e.estado_real)} />
                    {e.estado_declarado !== e.estado_real && (
                      <span className="muted small" title="lo que declara el catálogo">
                        catálogo: {e.estado_declarado.replace(/_/g, " ")}
                      </span>
                    )}
                  </div>
                </div>
                <div className="small muted">{e.veredicto}</div>
                {e.faltan.length > 0 && (
                  <div className="small" style={{ color: "#f85149" }}>
                    faltan: {e.faltan.join(", ")}
                  </div>
                )}
                <div className="small mono" style={{ opacity: 0.75 }}>
                  {e.vram_min_gb > 0 ? `pide ${e.vram_min_gb} GB VRAM / ${e.ram_min_gb} GB RAM` : "sin requisitos declarados"}
                  {e.medido ? ` · medido: ${e.medido}` : " · sin medir"}
                </div>
                {e.limites && <div className="small muted">límite: {e.limites}</div>}
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="card">
        <h2>Analizar material (preflight)</h2>
        <div className="row">
          <label className="field" style={{ flex: 1 }}>
            <span>Ruta del archivo</span>
            <input
              value={ruta}
              onChange={(ev) => setRuta(ev.target.value)}
              placeholder="G:\\descargas\\clip.mp4"
            />
          </label>
          <label className="field">
            <span>Destino</span>
            <select value={destino} onChange={(ev) => setDestino(ev.target.value)}>
              <option value="">automático</option>
              <option value="photoreal">photoreal</option>
              <option value="anime">anime</option>
              <option value="cartoon">cartoon</option>
              <option value="game">game</option>
            </select>
          </label>
          <label className="field" style={{ width: 110 }}>
            <span>Umbral corte</span>
            <input
              type="number"
              step="0.05"
              min="0.05"
              max="0.95"
              value={umbral}
              onChange={(ev) => setUmbral(Number(ev.target.value))}
            />
          </label>
          <button className="btn" onClick={() => void analizar()} disabled={analizando}>
            {analizando ? "Analizando…" : "Analizar"}
          </button>
        </div>

        {preflight && (
          <div style={{ marginTop: 12 }}>
            <div className="row-between">
              <div className="small">
                <span className="mono">
                  {preflight.sondeo.tipo} · {preflight.sondeo.ancho}×{preflight.sondeo.alto} ·{" "}
                  {preflight.sondeo.fps} fps · {preflight.sondeo.segundos.toFixed(2)} s ·{" "}
                  {preflight.sondeo.codec}
                  {preflight.sondeo.audio ? " · con audio" : " · sin audio"}
                </span>
              </div>
              <Badge
                texto={preflight.veredicto}
                color={
                  preflight.veredicto === "listo"
                    ? "#3fb950"
                    : preflight.veredicto === "con_avisos"
                      ? "#d29922"
                      : "#f85149"
                }
              />
            </div>

            <div className="small" style={{ marginTop: 6 }}>
              {preflight.deteccion ? (
                <>
                  estilo detectado: <strong>{preflight.deteccion.estilo}</strong>{" "}
                  ({(preflight.deteccion.confianza * 100).toFixed(0)} %, {preflight.deteccion.modelo})
                  {preflight.deteccion.motivos ? ` — ${preflight.deteccion.motivos}` : ""}
                  {preflight.deteccion.confianza < 0.6 && (
                    <span className="warn-text"> · poca confianza: corrígelo a mano</span>
                  )}
                </>
              ) : (
                <span className="muted">sin detección automática</span>
              )}
            </div>

            <div className="small" style={{ marginTop: 4 }}>
              paquete: <strong>{preflight.paquete_titulo || "ninguno"}</strong>
              {preflight.plantilla ? ` · plantilla ${preflight.plantilla}` : ""}
            </div>

            <div className="small muted" style={{ marginTop: 4 }}>
              {preflight.planos.length} plano(s) · intermedios ~{preflight.disco_estimado_gb} GB · VRAM
              libre {preflight.vram_libre_max_gb} GB · RAM libre {preflight.ram_libre_gb} GB
            </div>

            {preflight.planos.length > 0 && (
              <div className="mono small" style={{ marginTop: 4, opacity: 0.8 }}>
                {preflight.planos
                  .slice(0, 10)
                  .map((p) => `#${p.indice} ${p.inicio.toFixed(2)}–${p.fin.toFixed(2)}s`)
                  .join("  ")}
                {preflight.planos.length > 10 ? "  …" : ""}
              </div>
            )}

            {preflight.avisos.map((a, i) => (
              <div key={i} className="small warn-text">
                · {a}
              </div>
            ))}
            {preflight.faltan.map((f, i) => (
              <div key={i} className="small" style={{ color: "#f85149" }}>
                falta: {f}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
