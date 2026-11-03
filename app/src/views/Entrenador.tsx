import { useEffect, useRef, useState } from "react";
import {
  api,
  type EntrenadorImagen,
  type EntrenadorPreflight,
  type EntrenadorProyecto,
  type EntrenadorRun,
} from "../lib/api";

type Props = { notify: (msg: string) => void };

const EN_CURSO = ["iniciando", "entrenando"];

/**
 * Entrenador de LoRA (LA8). Une tres cosas que antes no existían juntas:
 * dataset con captions, comprobación de VRAM y un entrenamiento con progreso
 * real, leído del proceso que lo ejecuta.
 */
export default function Entrenador({ notify }: Props) {
  const [proyectos, setProyectos] = useState<EntrenadorProyecto[]>([]);
  const [sel, setSel] = useState<string>("");
  const [imagenes, setImagenes] = useState<EntrenadorImagen[]>([]);
  const [pf, setPf] = useState<EntrenadorPreflight | null>(null);
  const [runs, setRuns] = useState<EntrenadorRun[]>([]);
  const [run, setRun] = useState<EntrenadorRun | null>(null);

  const [nombre, setNombre] = useState("");
  const [clase, setClase] = useState("personaje");
  const [modeloBase, setModeloBase] = useState("");
  const [carpeta, setCarpeta] = useState("");
  const [modeloVision, setModeloVision] = useState("qwen3-vl:4b");

  const [pasos, setPasos] = useState(400);
  const [rango, setRango] = useState(8);
  const [resolucion, setResolucion] = useState(1024);

  const [borradores, setBorradores] = useState<Record<number, string>>({});
  const [ocupado, setOcupado] = useState(false);
  const timer = useRef<number | null>(null);

  const cargarProyectos = async () => {
    try {
      const p = await api.entrenadorProyectos();
      setProyectos(p);
      if (!sel && p.length > 0) setSel(p[0].id);
    } catch (e) {
      notify(`No se pudieron listar los proyectos: ${e}`);
    }
  };

  useEffect(() => {
    void cargarProyectos();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const cargarProyecto = async (id: string) => {
    try {
      const [img, pre, hist] = await Promise.all([
        api.entrenadorImagenes(id),
        api.entrenadorPreflight(id),
        api.entrenadorRuns(id),
      ]);
      setImagenes(img);
      setPf(pre);
      setRuns(hist);
      const activo = hist.find((r) => EN_CURSO.includes(r.estado));
      setRun(activo ?? null);
    } catch (e) {
      notify(`No se pudo abrir el proyecto: ${e}`);
    }
  };

  useEffect(() => {
    if (sel) void cargarProyecto(sel);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sel]);

  // Seguimiento del entrenamiento: se pregunta al núcleo, que es quien sabe.
  useEffect(() => {
    if (!run || !EN_CURSO.includes(run.estado)) return;
    timer.current = window.setInterval(async () => {
      try {
        const r = await api.entrenadorRun(run.id);
        if (r) {
          setRun(r);
          if (!EN_CURSO.includes(r.estado)) {
            notify(`Entrenamiento ${r.estado}${r.salida ? `: ${r.salida}` : ""}`);
            if (sel) await cargarProyecto(sel);
          }
        }
      } catch {
        /* si el sondeo falla, se reintenta en el siguiente ciclo */
      }
    }, 1500);
    return () => {
      if (timer.current) window.clearInterval(timer.current);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [run?.id, run?.estado]);

  const crear = async () => {
    if (!nombre.trim()) return notify("El proyecto necesita un nombre");
    setOcupado(true);
    try {
      const p = await api.entrenadorCrear(nombre.trim(), clase, modeloBase.trim());
      setNombre("");
      await cargarProyectos();
      setSel(p.id);
      notify(`Proyecto «${p.nombre}» creado`);
    } catch (e) {
      notify(`No se pudo crear: ${e}`);
    } finally {
      setOcupado(false);
    }
  };

  const importar = async () => {
    if (!sel) return notify("Elige un proyecto");
    if (!carpeta.trim()) return notify("Escribe la carpeta con las imágenes");
    setOcupado(true);
    try {
      const img = await api.entrenadorImportar(sel, carpeta.trim());
      setImagenes(img);
      await cargarProyecto(sel);
      await cargarProyectos();
      notify(`${img.length} imagen(es) en el dataset`);
    } catch (e) {
      notify(`No se pudo importar: ${e}`);
    } finally {
      setOcupado(false);
    }
  };

  const guardarCaption = async (img: EntrenadorImagen) => {
    const texto = borradores[img.id] ?? img.caption;
    try {
      await api.entrenadorCaption(img.id, texto);
      setImagenes((prev) => prev.map((i) => (i.id === img.id ? { ...i, caption: texto } : i)));
      setBorradores((b) => {
        const copia = { ...b };
        delete copia[img.id];
        return copia;
      });
      if (sel) setPf(await api.entrenadorPreflight(sel));
    } catch (e) {
      notify(`No se pudo guardar el caption: ${e}`);
    }
  };

  const describir = async (img: EntrenadorImagen) => {
    setOcupado(true);
    try {
      const texto = await api.entrenadorCaptionAuto(img.id, img.proyecto, modeloVision);
      setBorradores((b) => ({ ...b, [img.id]: texto }));
      notify("Caption propuesto: revísalo y guárdalo");
    } catch (e) {
      notify(`El modelo de visión falló: ${e}`);
    } finally {
      setOcupado(false);
    }
  };

  const iniciar = async (probar: boolean) => {
    if (!sel) return notify("Elige un proyecto");
    setOcupado(true);
    try {
      const r = await api.entrenadorIniciar({
        proyecto: sel,
        pasos,
        rango,
        resolucion,
        modelo: modeloBase.trim(),
        probar,
      });
      setRun(r);
      notify(probar ? "Prueba de humo lanzada" : "Entrenamiento lanzado");
      const hist = await api.entrenadorRuns(sel);
      setRuns(hist);
    } catch (e) {
      notify(`No se pudo lanzar: ${e}`);
    } finally {
      setOcupado(false);
    }
  };

  const cancelar = async () => {
    if (!run) return;
    try {
      await api.entrenadorCancelar(run.id);
      notify("Cancelado");
      if (sel) await cargarProyecto(sel);
    } catch (e) {
      notify(`No se pudo cancelar: ${e}`);
    }
  };

  const progreso = run && run.total > 0 ? Math.min(100, (run.paso / run.total) * 100) : 0;

  return (
    <div style={{ display: "grid", gap: 16 }}>
      <header>
        <h2 style={{ margin: 0 }}>Entrenador de LoRA</h2>
        <p style={{ margin: "4px 0 0", opacity: 0.75 }}>
          Dataset, captions y entrenamiento con PyTorch local. El progreso que ves viene del
          proceso que entrena, no de una estimación.
        </p>
      </header>

      <section className="card" style={{ padding: 12 }}>
        <strong>Nuevo proyecto</strong>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginTop: 8 }}>
          <input
            placeholder="Nombre (p. ej. Mika)"
            value={nombre}
            onChange={(e) => setNombre(e.target.value)}
          />
          <select value={clase} onChange={(e) => setClase(e.target.value)}>
            <option value="personaje">personaje</option>
            <option value="estilo">estilo</option>
            <option value="objeto">objeto</option>
            <option value="escenario">escenario</option>
          </select>
          <input
            placeholder="Modelo base (.safetensors o repo HF)"
            value={modeloBase}
            onChange={(e) => setModeloBase(e.target.value)}
            style={{ minWidth: 320 }}
          />
          <button className="btn" onClick={crear} disabled={ocupado}>
            Crear
          </button>
        </div>
      </section>

      <section style={{ display: "flex", gap: 16, alignItems: "flex-start", flexWrap: "wrap" }}>
        <div className="card" style={{ padding: 12, minWidth: 260 }}>
          <strong>Proyectos</strong>
          <div style={{ display: "grid", gap: 6, marginTop: 8 }}>
            {proyectos.length === 0 && <span style={{ opacity: 0.7 }}>ninguno todavía</span>}
            {proyectos.map((p) => (
              <button
                key={p.id}
                className={`btn small ${p.id === sel ? "active" : ""}`}
                onClick={() => setSel(p.id)}
                style={{ textAlign: "left", opacity: p.id === sel ? 1 : 0.8 }}
              >
                {p.nombre} · {p.imagenes} img · {p.con_caption} con caption
              </button>
            ))}
          </div>
        </div>

        {sel && (
          <div style={{ flex: 1, minWidth: 420, display: "grid", gap: 12 }}>
            <section className="card" style={{ padding: 12 }}>
              <strong>Dataset</strong>
              <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginTop: 8 }}>
                <input
                  placeholder="Carpeta con las imágenes (ruta completa)"
                  value={carpeta}
                  onChange={(e) => setCarpeta(e.target.value)}
                  style={{ minWidth: 340 }}
                />
                <button className="btn small" onClick={importar} disabled={ocupado}>
                  Importar
                </button>
                <input
                  value={modeloVision}
                  onChange={(e) => setModeloVision(e.target.value)}
                  title="Modelo de visión para describir"
                  style={{ width: 160 }}
                />
              </div>
              {pf && (
                <div style={{ marginTop: 8, fontSize: 13 }}>
                  <span className="chip">{pf.imagenes} imágenes</span>{" "}
                  <span className="chip">{pf.con_caption} con caption</span>{" "}
                  <span className="chip">
                    VRAM libre {(pf.vram_libre_mb / 1024).toFixed(1)} GB
                  </span>
                  <div style={{ opacity: 0.7, marginTop: 4 }}>{pf.dataset_dir}</div>
                  {pf.avisos.map((a, i) => (
                    <div key={i} style={{ color: "#f0b429" }}>
                      · {a}
                    </div>
                  ))}
                </div>
              )}
            </section>

            <section className="card" style={{ padding: 12, maxHeight: 360, overflow: "auto" }}>
              <strong>Imágenes y captions</strong>
              <div style={{ display: "grid", gap: 8, marginTop: 8 }}>
                {imagenes.length === 0 && <span style={{ opacity: 0.7 }}>dataset vacío</span>}
                {imagenes.map((img) => (
                  <div key={img.id} style={{ display: "grid", gap: 4 }}>
                    <div style={{ fontSize: 13, opacity: 0.8 }}>
                      {img.archivo} · {img.ancho}×{img.alto}
                    </div>
                    <div style={{ display: "flex", gap: 6 }}>
                      <input
                        value={borradores[img.id] ?? img.caption}
                        onChange={(e) =>
                          setBorradores((b) => ({ ...b, [img.id]: e.target.value }))
                        }
                        placeholder="caption: quién, ropa, pose, fondo, luz"
                        style={{ flex: 1 }}
                      />
                      <button className="btn small" onClick={() => guardarCaption(img)}>
                        Guardar
                      </button>
                      <button
                        className="btn small"
                        onClick={() => describir(img)}
                        disabled={ocupado}
                        title="Pide el caption a un modelo de visión local"
                      >
                        Describir
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            </section>

            <section className="card" style={{ padding: 12 }}>
              <strong>Entrenar</strong>
              <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginTop: 8, alignItems: "center" }}>
                <label>
                  pasos{" "}
                  <input
                    type="number"
                    value={pasos}
                    min={2}
                    onChange={(e) => setPasos(Number(e.target.value))}
                    style={{ width: 90 }}
                  />
                </label>
                <label>
                  rango{" "}
                  <input
                    type="number"
                    value={rango}
                    min={1}
                    max={128}
                    onChange={(e) => setRango(Number(e.target.value))}
                    style={{ width: 70 }}
                  />
                </label>
                <label>
                  resolución{" "}
                  <input
                    type="number"
                    value={resolucion}
                    min={256}
                    step={64}
                    onChange={(e) => setResolucion(Number(e.target.value))}
                    style={{ width: 90 }}
                  />
                </label>
                <button className="btn" onClick={() => iniciar(false)} disabled={ocupado}>
                  Entrenar
                </button>
                <button
                  className="btn small"
                  onClick={() => iniciar(true)}
                  disabled={ocupado}
                  title="UNet diminuta, 2 pasos, sin GPU: comprueba que la maquinaria funciona"
                >
                  Prueba de humo
                </button>
                {run && EN_CURSO.includes(run.estado) && (
                  <button className="btn small" onClick={cancelar}>
                    Cancelar
                  </button>
                )}
              </div>

              {run && (
                <div style={{ marginTop: 10, fontSize: 13 }}>
                  <div>
                    <strong>{run.estado}</strong> · paso {run.paso}/{run.total} · pérdida{" "}
                    {run.perdida.toFixed(4)}
                  </div>
                  <div
                    style={{
                      height: 8,
                      background: "rgba(255,255,255,0.12)",
                      borderRadius: 4,
                      marginTop: 6,
                      overflow: "hidden",
                    }}
                  >
                    <div
                      style={{
                        width: `${progreso}%`,
                        height: "100%",
                        background: "#4ade80",
                        transition: "width .3s",
                      }}
                    />
                  </div>
                  {run.salida && <div style={{ marginTop: 6 }}>LoRA: {run.salida}</div>}
                  {run.log && (
                    <pre style={{ fontSize: 11, opacity: 0.7, whiteSpace: "pre-wrap", marginTop: 6 }}>
                      {run.log.slice(-1200)}
                    </pre>
                  )}
                </div>
              )}
            </section>

            <section className="card" style={{ padding: 12 }}>
              <strong>Historial</strong>
              <div style={{ display: "grid", gap: 4, marginTop: 8, fontSize: 13 }}>
                {runs.length === 0 && <span style={{ opacity: 0.7 }}>sin entrenamientos</span>}
                {runs.map((r) => (
                  <div key={r.id} style={{ opacity: 0.85 }}>
                    #{r.id} · {r.estado} · {r.paso}/{r.total} pasos · pérdida{" "}
                    {r.perdida.toFixed(4)} · {r.iniciado}
                    {r.salida ? ` · ${r.salida}` : ""}
                  </div>
                ))}
              </div>
            </section>
          </div>
        )}
      </section>
    </div>
  );
}
