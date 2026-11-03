import { useEffect, useMemo, useState } from "react";
import {
  api,
  type AnimeCapability,
  type AnimeJob,
  type AnimeResult,
  type ComfyStatus,
} from "../lib/api";

const DIRECCIONES = [
  { id: "anime2real", label: "Anime → Real" },
  { id: "real2anime", label: "Real → Anime" },
];

export default function AnimeReal({ notify }: { notify: (m: string) => void }) {
  const [estado, setEstado] = useState<ComfyStatus | null>(null);
  const [caps, setCaps] = useState<AnimeCapability[]>([]);
  const [imagenes, setImagenes] = useState<string[]>([]);
  const [historial, setHistorial] = useState<AnimeJob[]>([]);
  const [salida, setSalida] = useState("");

  const [direccion, setDireccion] = useState("anime2real");
  const [plantilla, setPlantilla] = useState("");
  const [imagen, setImagen] = useState("");
  const [prompt, setPrompt] = useState("");
  const [negativo, setNegativo] = useState("");
  const [denoise, setDenoise] = useState(0.55);
  const [seed, setSeed] = useState("");
  const [ancho, setAncho] = useState(1024);
  const [alto, setAlto] = useState(1024);

  const [trabajando, setTrabajando] = useState(false);
  const [resultado, setResultado] = useState<AnimeResult | null>(null);
  const [capsCargando, setCapsCargando] = useState(true);

  const cargar = async () => {
    // 1) El estado de ComfyUI va aparte y primero: si el análisis de modelos tarda o
    // falla, la vista no debe quedarse en blanco ni anunciar que ComfyUI está caído.
    try {
      setEstado(await api.animeStatus());
    } catch (e) {
      notify(`Error consultando ComfyUI: ${e}`);
    }

    // 2) Llamadas independientes: una que falle no borra las demás.
    setCapsCargando(true);
    const [c, imgs, h, dir] = await Promise.all([
      api.animeCapabilities().catch((e) => {
        notify(`No se pudieron analizar los modelos: ${e}`);
        return [] as AnimeCapability[];
      }),
      api.animeInputs().catch(() => [] as string[]),
      api.animeHistory(30).catch(() => [] as AnimeJob[]),
      api.animeOutputPath().catch(() => ""),
    ]);
    setCaps(c);
    setImagenes(imgs);
    setHistorial(h);
    setSalida(dir);
    setCapsCargando(false);
    if (imgs.length > 0 && !imagen) setImagen(imgs[0]);
    const primera = c.find((x) => x.direccion === direccion && x.disponible);
    if (primera && !plantilla) {
      setPlantilla(primera.id);
      setDenoise(primera.id.includes("id_") ? 0.55 : 0.55);
    }
  };

  useEffect(() => {
    cargar();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const deEstaDireccion = useMemo(
    () => caps.filter((c) => c.direccion === direccion),
    [caps, direccion],
  );

  const capActual = caps.find((c) => c.id === plantilla);

  const convertir = async () => {
    if (!plantilla) return notify("Elige una plantilla.");
    if (!imagen) return notify("Elige una imagen de entrada.");
    if (capActual && !capActual.disponible)
      return notify(`Faltan modelos: ${capActual.faltan.join(", ")}`);

    setTrabajando(true);
    setResultado(null);
    try {
      const r = await api.animeConvert({
        template_id: plantilla,
        image: imagen,
        prompt,
        negative: negativo,
        denoise,
        seed: seed.trim() === "" ? null : Number(seed),
        width: ancho,
        height: alto,
      });
      setResultado(r);
      setHistorial(await api.animeHistory(30));
      notify(`${r.direccion} · ${r.segundos.toFixed(1)} s · ${r.archivos.length} archivo(s)`);
    } catch (e) {
      notify(`Conversión fallida: ${e}`);
    } finally {
      setTrabajando(false);
    }
  };

  const ocupado = trabajando;

  return (
    <div className="page">
      <div className="page-head">
        <h1>KUKIA ANIME ↔ REAL STUDIO</h1>
        <span className={`chip ${estado?.online ? "ok" : estado ? "warn" : "subtle"}`}>
          {estado?.online
            ? `ComfyUI ${estado.version} · ${estado.device.split(":")[1]?.trim() ?? estado.device}`
            : estado
              ? "ComfyUI sin conexión"
              : "Comprobando ComfyUI…"}
        </span>
      </div>

      <div className="card">
        <div className="row-between">
          <div className="muted">
            {estado?.online ? (
              <>
                VRAM libre{" "}
                <strong>
                  {Math.round(estado.vram_free_mb / 1024)} / {Math.round(estado.vram_total_mb / 1024)} GB
                </strong>{" "}
                · cola {estado.en_cola} · resultados en <span className="mono">{salida}</span>
                {capsCargando && <> · analizando modelos…</>}
              </>
            ) : estado ? (
              <>Arranca ComfyUI en el puerto 8188 y vuelve a cargar. {estado.nota}</>
            ) : (
              <>Consultando ComfyUI en el puerto 8188…</>
            )}
          </div>
          <button className="btn ghost" onClick={cargar} disabled={ocupado}>
            Recargar
          </button>
        </div>
      </div>

      <div className="split">
        <div className="card">
          <h2>Origen → destino</h2>
          <div className="row">
            {DIRECCIONES.map((d) => (
              <button
                key={d.id}
                className={`btn ${direccion === d.id ? "" : "ghost"}`}
                onClick={() => {
                  setDireccion(d.id);
                  const p = caps.find((c) => c.direccion === d.id && c.disponible);
                  setPlantilla(p ? p.id : "");
                }}
                disabled={ocupado}
              >
                {d.label}
              </button>
            ))}
          </div>

          <label className="field">
            <span>Motor</span>
            <select
              value={plantilla}
              onChange={(e) => setPlantilla(e.target.value)}
              disabled={ocupado}
            >
              <option value="">— elige —</option>
              {deEstaDireccion.map((c) => (
                <option key={c.id} value={c.id} disabled={!c.disponible}>
                  {c.titulo}
                  {c.disponible ? ` · ${c.segundos_estimados}s` : " · faltan modelos"}
                </option>
              ))}
            </select>
          </label>

          {capActual && (
            <p className="muted small">
              {capActual.motor}
              {capActual.verificado && <> · verificado: {capActual.verificado}</>}
              {capActual.faltan.length > 0 && (
                <>
                  <br />
                  <span className="warn-text">Falta: {capActual.faltan.join(", ")}</span>
                </>
              )}
            </p>
          )}

          <label className="field">
            <span>Imagen de entrada (carpeta input de ComfyUI)</span>
            <select value={imagen} onChange={(e) => setImagen(e.target.value)} disabled={ocupado}>
              {imagenes.length === 0 && <option value="">sin imágenes</option>}
              {imagenes.map((i) => (
                <option key={i} value={i}>
                  {i}
                </option>
              ))}
            </select>
          </label>

          <label className="field">
            <span>Estilo (positivo) — vacío usa el del workflow</span>
            <textarea
              value={prompt}
              onChange={(e) => setPrompt(e.target.value)}
              placeholder="photorealistic portrait, natural skin, 85mm f/1.4…"
              rows={3}
              disabled={ocupado}
            />
          </label>

          <label className="field">
            <span>Negativo</span>
            <input
              value={negativo}
              onChange={(e) => setNegativo(e.target.value)}
              placeholder="anime, cartoon, plastic skin…"
              disabled={ocupado}
            />
          </label>

          <div className="row">
            <label className="field">
              <span>Fuerza ({denoise.toFixed(2)})</span>
              <input
                type="range"
                min={0.2}
                max={0.95}
                step={0.01}
                value={denoise}
                onChange={(e) => setDenoise(Number(e.target.value))}
                disabled={ocupado}
              />
            </label>
            <label className="field">
              <span>Semilla</span>
              <input
                value={seed}
                onChange={(e) => setSeed(e.target.value)}
                placeholder="aleatoria"
                disabled={ocupado}
              />
            </label>
          </div>
          <div className="row">
            <label className="field">
              <span>Ancho</span>
              <input
                type="number"
                value={ancho}
                onChange={(e) => setAncho(Number(e.target.value))}
                disabled={ocupado}
              />
            </label>
            <label className="field">
              <span>Alto</span>
              <input
                type="number"
                value={alto}
                onChange={(e) => setAlto(Number(e.target.value))}
                disabled={ocupado}
              />
            </label>
          </div>

          <button className="btn" onClick={convertir} disabled={ocupado || !estado?.online}>
            {ocupado ? "Convirtiendo…" : `Convertir (${denoise.toFixed(2)})`}
          </button>
          <p className="muted small">
            Menos fuerza conserva más la estructura original; más fuerza cambia más el estilo.
            Las de vídeo no se ofrecen hasta estar probadas: no hay interfaz falsa.
          </p>
        </div>

        <div className="card">
          <h2>Antes / después</h2>
          {!resultado && (
            <div className="empty small">
              {ocupado
                ? "Encolado en ComfyUI, esperando resultado…"
                : "Elige imagen y pulsa Convertir."}
            </div>
          )}
          {resultado && (
            <>
              <div className="preview-pair">
                <figure>
                  <img src={resultado.entrada_preview} alt="entrada" />
                  <figcaption className="muted small">entrada · {imagen}</figcaption>
                </figure>
                <figure>
                  <img src={resultado.preview} alt="salida" />
                  <figcaption className="muted small">
                    salida · {resultado.segundos.toFixed(1)} s ·{" "}
                    {(resultado.bytes / 1024 / 1024).toFixed(2)} MB
                  </figcaption>
                </figure>
              </div>
              <div className="muted small mono">
                {resultado.archivos.map((a) => (
                  <div key={a}>{a}</div>
                ))}
              </div>
            </>
          )}
        </div>
      </div>

      <div className="card">
        <h2>Qué se puede hacer de verdad</h2>
        <table className="table">
          <thead>
            <tr>
              <th>Conversión</th>
              <th>Tipo</th>
              <th>Motor</th>
              <th>Estado</th>
              <th>Medido</th>
            </tr>
          </thead>
          <tbody>
            {caps.map((c) => (
              <tr key={c.id}>
                <td>{c.titulo}</td>
                <td>{c.tipo}</td>
                <td className="mono small">{c.motor}</td>
                <td>
                  {c.disponible ? (
                    <span className="tag ok-tag">listo</span>
                  ) : c.estado === "pendiente_conversion" ? (
                    <span className="tag">sin convertir</span>
                  ) : (
                    <span className="tag">faltan modelos</span>
                  )}
                </td>
                <td className="small">{c.verificado || "—"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <div className="card">
        <h2>Historial de trabajos</h2>
        {historial.length === 0 && <div className="empty small">Sin trabajos todavía.</div>}
        <table className="table">
          <tbody>
            {historial.map((h) => (
              <tr key={h.id + h.fecha}>
                <td className="mono small">{h.fecha.slice(0, 16).replace("T", " ")}</td>
                <td className="small">{h.direccion}</td>
                <td className="mono small">{h.entrada}</td>
                <td className="small">{h.segundos.toFixed(1)} s</td>
                <td className="small">{h.archivos.length} archivo(s)</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
