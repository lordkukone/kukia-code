import { useEffect, useState } from "react";
import {
  api,
  type LoteResult,
  type OpVram,
  type ReleaseReport,
  type VramReport,
  type VramSeleccion,
} from "../lib/api";

/** Acción de liberación pendiente de confirmar. */
type Accion =
  | { tipo: "engine"; clave: string; etiqueta: string; estimado: number }
  | { tipo: "gpu"; clave: string; gpu: number; etiqueta: string; estimado: number }
  | { tipo: "all"; clave: string; etiqueta: string; estimado: number };

/** Motores que no tienen API de descarga: se dice, no se pone un botón que falle. */
const SIN_API = ["wan2gp", "maestro"];

export default function Vram({ notify }: { notify: (m: string) => void }) {
  const [report, setReport] = useState<VramReport | null>(null);
  const [cargando, setCargando] = useState(false);
  const [tamano, setTamano] = useState(13.5);
  const [gpu, setGpu] = useState(1);
  const [cabe, setCabe] = useState<[boolean, string] | null>(null);
  // Clave `motor::modelo` que se está expulsando, para no disparar dos veces el mismo.
  const [expulsando, setExpulsando] = useState<string | null>(null);
  const [vaciando, setVaciando] = useState(false);
  // Liberación pendiente de confirmar («descargar de este motor», «liberar esta GPU»…).
  const [accion, setAccion] = useState<Accion | null>(null);
  // Último resultado medido, para enseñar qué GPU se liberó y cuánto.
  const [ultimo, setUltimo] = useState<ReleaseReport | null>(null);
  // Selección múltiple: claves `motor::modelo`.
  const [sel, setSel] = useState<string[]>([]);
  const [lote, setLote] = useState<LoteResult | null>(null);
  const [hist, setHist] = useState<OpVram[]>([]);

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
      const [r, h] = await Promise.all([api.vramReport(), api.vramHistorial(20)]);
      setReport(r);
      setHist(h);
      // La selección se limpia de lo que ya no está cargado.
      const vivas = r.engines.flatMap((e) => e.models.map((m) => `${e.engine}::${m.name}`));
      setSel((prev) => prev.filter((c) => vivas.includes(c)));
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

  /** ComfyUI está generando: cualquier descarga de su VRAM corta el trabajo. */
  const comfyOcupado = report?.comfy_busy ?? 0;

  /** Pide permiso explícito si la acción puede interrumpir una generación. */
  const confirmarCorte = (afectaComfy: boolean): boolean => {
    if (!afectaComfy || comfyOcupado === 0) return true;
    return window.confirm(
      `ComfyUI tiene ${comfyOcupado} trabajo(s) en marcha o en cola.\n\n` +
        "Su única forma de liberar VRAM es /free, que descarga TODOS sus modelos y corta esa generación.\n\n" +
        "¿Seguro que quieres interrumpirla?",
    );
  };

  const expulsar = async (engine: string, model: string) => {
    const clave = `${engine}::${model}`;
    if (engine === "comfyui" && !confirmarCorte(true)) return;
    setExpulsando(clave);
    try {
      const r = await api.vramUnload(engine, model, engine === "comfyui" && comfyOcupado > 0);
      notify(r.ok ? `Expulsado ${model}: ${r.message}` : `No se pudo expulsar ${model}: ${r.message}`);
      // El motor tarda un instante en soltar la memoria: se le da un respiro
      // antes de volver a medir para no enseñar el modelo ya descargado.
      if (r.ok) await new Promise((listo) => setTimeout(listo, 700));
      await cargar();
    } catch (e) {
      notify(`Error expulsando ${model}: ${e}`);
    } finally {
      setExpulsando(null);
    }
  };

  const alternar = (clave: string) => {
    setSel((prev) => (prev.includes(clave) ? prev.filter((c) => c !== clave) : [...prev, clave]));
  };

  /** Descarga exactamente lo seleccionado, midiendo antes/después. */
  const descargarSeleccion = async () => {
    if (sel.length === 0) return;
    const items: VramSeleccion[] = sel.map((clave) => {
      const i = clave.indexOf("::");
      return { engine: clave.slice(0, i), model: clave.slice(i + 2) };
    });
    const comfy = items.some((i) => i.engine === "comfyui");
    const aviso = comfy
      ? `ComfyUI no permite descargar un modelo suelto: su API /free libera TODOS los modelos cargados.\n\nSe descargarán ${items.length} modelo(s) de la selección. ¿Continuar?`
      : `Se descargarán de VRAM ${items.length} modelo(s):\n\n${items
          .map((i) => `· ${i.engine}: ${i.model}`)
          .join("\n")}\n\nLos archivos del disco NO se tocan. ¿Continuar?`;
    if (!window.confirm(aviso)) return;
    if (comfy && !confirmarCorte(true)) return;
    const itemsFinales: VramSeleccion[] = items.map((i) =>
      i.engine === "comfyui" ? { ...i, forzar: comfyOcupado > 0 } : i,
    );
    setVaciando(true);
    setLote(null);
    try {
      const r = await api.vramUnloadSeleccion(itemsFinales);
      setLote(r);
      notify(
        r.liberado_mb > 0
          ? `${(r.liberado_mb / 1024).toFixed(2)} GB liberados de VRAM`
          : "Operación completada, sin VRAM medida recuperada",
      );
      setSel([]);
      await cargar();
    } catch (e) {
      notify(`Error en la descarga por lotes: ${e}`);
    } finally {
      setVaciando(false);
    }
  };

  /** MB que se estiman recuperables si se descarga lo cargado (por motor o todo). */
  const estimado = (motor?: string): number => {
    const engines = report?.engines ?? [];
    return engines
      .filter((e) => !motor || e.engine === motor)
      .reduce(
        (total, e) => total + e.models.reduce((s, m) => s + (m.vram_mb > 0 ? m.vram_mb : m.size_gb * 1024), 0),
        0,
      );
  };

  const ejecutar = async (a: Accion) => {
    const tocaComfy =
      a.tipo === "engine" ? a.clave === "comfyui" : (report?.engines.find((e) => e.engine === "comfyui")?.models.length ?? 0) > 0;
    if (!confirmarCorte(tocaComfy)) return;
    const forzar = tocaComfy && comfyOcupado > 0;
    setAccion(null);
    setVaciando(true);
    try {
      const r =
        a.tipo === "engine"
          ? await api.vramReleaseEngine(a.clave, forzar)
          : a.tipo === "gpu"
            ? await api.vramReleaseGpu(a.gpu, forzar)
            : await api.vramReleaseEverything(forzar);
      setUltimo(r);
      const libre = (r.freed_mb / 1024).toFixed(2);
      notify(
        r.ok
          ? `${a.etiqueta}: ${libre} GB recuperados · ${r.note}`
          : `${a.etiqueta}: no se pudo completar · ${r.note}`,
      );
      await cargar();
    } catch (e) {
      notify(`Error liberando ${a.etiqueta}: ${e}`);
    } finally {
      setVaciando(false);
    }
  };

  useEffect(() => {
    cargar();
    // Refresco automático: el estado de los motores cambia solo (un modelo se
    // carga, otro expira, ComfyUI termina). Se pausa mientras hay una descarga
    // en curso para no pisar el resultado que se está midiendo.
    const id = setInterval(() => {
      if (!vaciando) cargar();
    }, 6000);
    return () => clearInterval(id);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [vaciando]);

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
            : cargando
              ? "midiendo…"
              : "sin datos"}
        </span>
      </div>

      <div className="card">
        <div className="row-between">
          <div className="muted">
            Vigila y avisa; no cierra procesos ni descarga nada por su cuenta. Solo saca un modelo
            cuando pulsas <strong>Expulsar</strong> o <strong>Vaciar VRAM</strong>.
            {report && <> Última lectura: {horaLocal(report.checked_at)}.</>}
          </div>
          <div className="row" style={{ gap: 8 }}>
            <button className="btn ghost" onClick={cargar} disabled={cargando}>
              {cargando ? "Midiendo…" : "Volver a medir"}
            </button>
            <button
              className="btn ghost"
              onClick={() =>
                setAccion({ tipo: "all", clave: "ambas", etiqueta: "Liberar ambas GPU", estimado: estimado() })
              }
              disabled={vaciando}
              title="Descargar todos los modelos cargados de Ollama, LM Studio y ComfyUI"
            >
              {vaciando ? "Liberando…" : "Liberar VRAM de ambas GPU"}
            </button>
          </div>
        </div>
      </div>

      {accion && (
        <div className="card" style={{ borderLeft: "3px solid #b45309" }}>
          <h2>Confirmar: {accion.etiqueta}</h2>
          <table className="table">
            <tbody>
              <tr>
                <td className="small" style={{ width: 170 }}>
                  <strong>Qué se descarga</strong>
                </td>
                <td className="small">
                  {accion.tipo === "engine"
                    ? (report?.engines.find((e) => e.engine === accion.clave)?.models.map((m) => m.name).join(", ") || "nada cargado")
                    : accion.tipo === "gpu"
                      ? "todos los modelos de todos los motores con API de descarga"
                      : "todos los modelos cargados en todos los motores"}
                </td>
              </tr>
              <tr>
                <td className="small">
                  <strong>GPU afectada</strong>
                </td>
                <td className="small">
                  {accion.tipo === "gpu"
                    ? `GPU${accion.gpu} · ${report?.gpus.find((g) => g.index === accion.gpu)?.name.replace("NVIDIA GeForce ", "") ?? ""}`
                    : "las dos GPU (Ollama reparte un mismo modelo entre ambas; ver nota abajo)"}
                </td>
              </tr>
              <tr>
                <td className="small">
                  <strong>VRAM estimada a recuperar</strong>
                </td>
                <td className="small">
                  {(accion.estimado / 1024).toFixed(2)} GB
                  <span className="muted">
                    {" "}· estimación; después se mide con nvidia-smi
                  </span>
                </td>
              </tr>
              <tr>
                <td className="small">
                  <strong>Consecuencias</strong>
                </td>
                <td className="small">
                  Se descargan los modelos por el mecanismo oficial del motor. Los motores siguen
                  abiertos, los modelos se vuelven a cargar la próxima vez que se usen y no se
                  cierra ningún proceso ni se borra nada del disco. Si ComfyUI tiene una generación
                  en curso, KUKIA lo detecta antes y te pide confirmación aparte: su /free la corta.
                </td>
              </tr>
            </tbody>
          </table>
          <div className="row" style={{ gap: 8, marginTop: 10 }}>
            <button className="btn" onClick={() => ejecutar(accion)} disabled={vaciando}>
              {vaciando ? "Liberando…" : "Sí, liberar"}
            </button>
            <button className="btn ghost" onClick={() => setAccion(null)} disabled={vaciando}>
              Cancelar
            </button>
          </div>
          {accion.tipo === "gpu" && (
            <p className="muted small" style={{ marginTop: 8 }}>
              Ningún motor local descarga solo la parte de una GPU. Ollama, medido hoy, repartió
              <code> qwen3-vl:4b </code> entre las dos tarjetas (11,5 + 9,6 GB) y su API solo sabe
              descargar el modelo entero. Por eso esta acción descarga los modelos completos y luego
              mide qué se recuperó en cada GPU: en la elegida suele liberarse menos que el total.
            </p>
          )}
        </div>
      )}

      {ultimo && (
        <div className="card">
          <div className="row-between">
            <h2 style={{ margin: 0 }}>Última liberación · {ultimo.target}</h2>
            <span className={`tag ${ultimo.ok ? "ok-tag" : ""}`}>
              {(ultimo.freed_mb / 1024).toFixed(2)} GB recuperados
            </span>
          </div>
          <table className="table">
            <thead>
              <tr>
                <th>GPU</th>
                <th>Antes</th>
                <th>Ahora</th>
                <th>Recuperado</th>
              </tr>
            </thead>
            <tbody>
              {ultimo.gpus.map((g) => (
                <tr key={g.index}>
                  <td className="small mono">GPU{g.index}</td>
                  <td className="small">
                    {((g.used_mb + g.freed_mb) / 1024).toFixed(2)} GB
                  </td>
                  <td className="small">{(g.used_mb / 1024).toFixed(2)} GB</td>
                  <td className="small">
                    <strong>{(g.freed_mb / 1024).toFixed(2)} GB</strong>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          <p className="muted small">{ultimo.note}</p>
          {ultimo.results.filter((r) => !r.ok).length > 0 && (
            <p className="warn-text small">
              {ultimo.results.filter((r) => !r.ok).length} descarga(s) fallaron:{" "}
              {ultimo.results.filter((r) => !r.ok).map((r) => `${r.engine}/${r.model} (${r.message})`).join(" · ")}
            </p>
          )}
        </div>
      )}

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
              <div className="row-between" style={{ marginTop: 4 }}>
                <span className="muted small">
                  libres {(g.free_mb / 1024).toFixed(1)} GB · reservada{" "}
                  {(g.reserved_mb / 1024).toFixed(1)} GB · uso {g.utilization} %
                </span>
                <button
                  className="btn ghost small"
                  disabled={vaciando}
                  onClick={() =>
                    setAccion({
                      tipo: "gpu",
                      clave: `gpu${g.index}`,
                      gpu: g.index,
                      etiqueta: `Liberar VRAM de GPU${g.index}`,
                      estimado: estimado(),
                    })
                  }
                  title={"Descargar los modelos cargados y medir cuánto se recupera en esta GPU"}
                >
                  Liberar esta GPU
                </button>
              </div>
            </div>
          ))}
          {!report && (
            <div className="empty small">
              {cargando ? "Midiendo GPUs…" : "Sin lectura todavía."}
            </div>
          )}
        </div>

        <div className="card">
          <div className="row-between" style={{ flexWrap: "wrap", gap: 8 }}>
            <h2 style={{ margin: 0 }}>Motores vigilados</h2>
            <div className="row" style={{ gap: 8, flexWrap: "wrap" }}>
              <span className="muted small">{sel.length} seleccionado(s)</span>
              <button
                className="btn small"
                disabled={sel.length === 0 || vaciando}
                onClick={() => void descargarSeleccion()}
                title="Descargar de VRAM solo lo marcado"
              >
                Descargar seleccionados
              </button>
              <button
                className="btn ghost small"
                disabled={sel.length === 0}
                onClick={() => setSel([])}
              >
                Limpiar
              </button>
            </div>
          </div>
          <table className="table">
            <thead>
              <tr>
                <th></th>
                <th>Motor</th>
                <th>Modelos cargados</th>
                <th>VRAM</th>
                <th>Estado</th>
                <th>Descarga</th>
              </tr>
            </thead>
            <tbody>
              {report?.engines.map((e) => (
                <tr key={e.engine}>
                  <td className="small">
                    <input
                      type="checkbox"
                      checked={
                        e.models.length > 0 &&
                        e.models.every((m) => sel.includes(`${e.engine}::${m.name}`))
                      }
                      disabled={e.models.length === 0}
                      onChange={() => {
                        const claves = e.models.map((m) => `${e.engine}::${m.name}`);
                        const todas = claves.every((c) => sel.includes(c));
                        setSel((prev) =>
                          todas
                            ? prev.filter((c) => !claves.includes(c))
                            : [...new Set([...prev, ...claves])],
                        );
                      }}
                      title="Marcar todos los modelos de este motor"
                    />
                  </td>
                  <td className="mono">{e.engine}</td>
                  <td>
                    {e.models.length === 0 ? (
                      <span className="muted">—</span>
                    ) : (
                      e.models.map((m) => {
                        const clave = `${e.engine}::${m.name}`;
                        return (
                          <div
                            key={m.name}
                            className="row-between"
                            style={{ gap: 8, alignItems: "center" }}
                          >
                            <span className="small">
                              <input
                                type="checkbox"
                                checked={sel.includes(clave)}
                                onChange={() => alternar(clave)}
                                style={{ marginRight: 6 }}
                              />
                              {m.name}
                              {m.vram_mb > 0 ? (
                                <span className="muted small"> · {m.vram_mb} MB</span>
                              ) : m.size_gb > 0 ? (
                                <span className="muted small"> · {m.size_gb.toFixed(1)} GB en disco</span>
                              ) : null}
                            </span>
                            <button
                              className="btn ghost small"
                              onClick={() => expulsar(e.engine, m.name)}
                              disabled={expulsando === clave || !e.online}
                              title="Descargar este modelo de la VRAM"
                            >
                              {expulsando === clave ? "…" : "Expulsar"}
                            </button>
                          </div>
                        );
                      })
                    )}
                  </td>
                  <td className="small">
                    {e.vram_total_mb > 0 ? `${(e.vram_total_mb / 1024).toFixed(2)} GB` : "—"}
                  </td>
                  <td className="small">
                    <span className={`tag ${e.online ? "ok-tag" : ""}`}>
                      {e.online ? "activo" : "sin respuesta"}
                    </span>
                    {e.engine === "comfyui" && comfyOcupado > 0 && (
                      <span className="tag warn-tag" style={{ marginLeft: 6 }}>
                        generando · {comfyOcupado}
                      </span>
                    )}
                    <div className="muted small">{e.note}</div>
                  </td>
                  <td>
                    {SIN_API.includes(e.engine) ? (
                      <span
                        className="muted small"
                        title="No publica qué modelo tiene cargado ni tiene ruta de descarga: solo liberaría su VRAM al cerrar el proceso, y KUKIA no cierra procesos"
                      >
                        sin API
                      </span>
                    ) : (
                      <button
                        className="btn ghost small"
                        disabled={vaciando || e.models.length === 0}
                        title="Descargar por el mecanismo oficial todos los modelos de este motor"
                        onClick={() =>
                          setAccion({
                            tipo: "engine",
                            clave: e.engine,
                            etiqueta: `Descargar modelos de ${e.engine}`,
                            estimado: estimado(e.engine),
                          })
                        }
                      >
                        Descargar todo
                      </button>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {lote && (
        <div className="card">
          <div className="row-between" style={{ flexWrap: "wrap" }}>
            <h2 style={{ margin: 0 }}>Resultado de la descarga</h2>
            <span className={`chip ${lote.liberado_mb > 0 ? "ok" : "warn"}`}>
              {lote.liberado_mb > 0
                ? `${(lote.liberado_mb / 1024).toFixed(2)} GB recuperados`
                : "sin VRAM medida recuperada"}
            </span>
          </div>

          <table className="table" style={{ marginTop: 8 }}>
            <thead>
              <tr>
                <th>Modelo</th>
                <th>Descarga</th>
                <th>¿Sigue en disco?</th>
                <th>Mensaje</th>
              </tr>
            </thead>
            <tbody>
              {lote.resultados.map((r) => {
                const disco = lote.en_disco.find(
                  (d) => d.engine === r.engine && d.model === r.model,
                );
                return (
                  <tr key={`${r.engine}::${r.model}`}>
                    <td className="small">
                      <span className="mono">{r.engine}</span> · {r.model}
                    </td>
                    <td className="small">
                      <span className={`chip ${r.ok ? "ok" : "warn"}`}>
                        {r.ok ? "descargado" : "falló"}
                      </span>
                    </td>
                    <td className="small">
                      {disco?.presente === true
                        ? "sí, intacto"
                        : disco?.presente === false
                          ? "no aparece"
                          : "no comprobable"}
                    </td>
                    <td className="muted small">{r.message}</td>
                  </tr>
                );
              })}
            </tbody>
          </table>

          {lote.siguen_cargados.length > 0 && (
            <div className="warn-text small" style={{ marginTop: 8 }}>
              Siguen en memoria: {lote.siguen_cargados.join(", ")}
            </div>
          )}
          {lote.avisos.map((a) => (
            <div key={a} className="muted small" style={{ marginTop: 4 }}>
              {a}
            </div>
          ))}
        </div>
      )}

      <div className="card">
        <h2>Historial de operaciones</h2>
        {hist.length === 0 ? (
          <div className="empty small">Todavía no se ha descargado nada desde KUKIA.</div>
        ) : (
          <table className="table">
            <thead>
              <tr>
                <th>Cuándo</th>
                <th>Motor</th>
                <th>Modelo</th>
                <th>Resultado</th>
                <th>MB</th>
                <th>Nota</th>
              </tr>
            </thead>
            <tbody>
              {hist.map((h) => (
                <tr key={h.id}>
                  <td className="small">
                    {h.created_at.slice(0, 19).replace("T", " ")}
                  </td>
                  <td className="mono small">{h.engine}</td>
                  <td className="small">{h.model}</td>
                  <td className="small">
                    <span className={`chip ${h.ok ? "ok" : "warn"}`}>
                      {h.ok ? "ok" : "falló"}
                    </span>
                  </td>
                  <td className="mono small">{h.liberado_mb}</td>
                  <td className="muted small">{h.nota}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
        <p className="muted small">
          Solo se registran operaciones de descarga de memoria. Ningún modelo se ha borrado del
          disco.
        </p>
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
