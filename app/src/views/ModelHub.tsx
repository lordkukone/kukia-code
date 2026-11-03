import { useEffect, useState } from "react";
import {
  api,
  type Compatibilidad,
  type GgufLocal,
  type HubModelo,
  type Job,
  type ModelCard,
  type OllamaHit,
  type OllamaTag,
} from "../lib/api";

type Pestana = "explorar" | "ollama" | "gguf" | "compat";

const CUANTS = ["Q2_K", "Q3_K_M", "Q4_K_M", "Q5_K_M", "Q6_K", "Q8_0", "F16"];

const colorVeredicto = (v: string): string => {
  if (v === "COMPATIBLE") return "ok";
  if (v === "COMPATIBLE CON LIMITACIONES") return "warn";
  if (v === "NO COMPATIBLE") return "warn";
  return "subtle";
};

export default function ModelHub({ notify }: { notify: (m: string) => void }) {
  const [pestana, setPestana] = useState<Pestana>("explorar");
  const [ocupado, setOcupado] = useState<string | null>(null);

  const [q, setQ] = useState("qwen3");
  const [ollamaHits, setOllamaHits] = useState<OllamaHit[] | null>(null);
  const [hf, setHf] = useState<HubModelo[] | null>(null);
  const [manifest, setManifest] = useState<OllamaTag | null>(null);

  const [instalados, setInstalados] = useState<ModelCard[]>([]);
  const [gguf, setGguf] = useState<GgufLocal[] | null>(null);
  const [pull, setPull] = useState<Job[]>([]);

  const [params, setParams] = useState(8);
  const [quant, setQuant] = useState("Q4_K_M");
  const [ctx, setCtx] = useState(8192);
  const [compat, setCompat] = useState<Compatibilidad | null>(null);

  const refrescarDescargas = async () => {
    try {
      const lista = await api.jobsList(60);
      const mios = lista.filter((j) => j.kind === "model_pull");
      setPull(mios);
      return mios;
    } catch {
      return [];
    }
  };

  useEffect(() => {
    void refrescarDescargas();
    void api
      .engineCards("ollama")
      .then(setInstalados)
      .catch(() => setInstalados([]));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Mientras haya descargas vivas, se refresca sola.
  const hayVivas = pull.some((j) => j.state === "queued" || j.state === "running");
  useEffect(() => {
    if (!hayVivas) return;
    const id = setInterval(() => void refrescarDescargas(), 2500);
    return () => clearInterval(id);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [hayVivas]);

  const buscar = async () => {
    setOcupado("buscar");
    setOllamaHits(null);
    setHf(null);
    const [o, h] = await Promise.allSettled([api.hubBuscarOllama(q), api.hubBuscarHf(q, 20)]);
    if (o.status === "fulfilled") setOllamaHits(o.value);
    else notify(`Ollama: ${o.reason}`);
    if (h.status === "fulfilled") setHf(h.value);
    else notify(`Hugging Face: ${h.reason}`);
    setOcupado(null);
  };

  const verManifiesto = async (modelo: string) => {
    setOcupado(`manifest:${modelo}`);
    try {
      const m = await api.hubManifest(modelo);
      setManifest(m);
      notify(`${m.name}: ${m.size_gb ? m.size_gb.toFixed(1) + " GB" : "sin tamaño publicado"}`);
    } catch (e) {
      notify(`No se pudo leer el manifiesto: ${e}`);
      setManifest(null);
    } finally {
      setOcupado(null);
    }
  };

  const descargar = async (modelo: string) => {
    if (!window.confirm(`Se encolará la descarga de «${modelo}» con la API de Ollama.\n\nPuede ocupar varios GB en disco. ¿Continuar?`)) {
      return;
    }
    setOcupado(`pull:${modelo}`);
    try {
      const id = await api.hubPull(modelo);
      notify(`Descarga #${id} encolada. El progreso está abajo y en Tareas.`);
      setPestana("ollama");
      await refrescarDescargas();
    } catch (e) {
      notify(`No se pudo encolar: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  const escanearGguf = async () => {
    setOcupado("gguf");
    try {
      const r = await api.hubGguf(400);
      setGguf(r);
      notify(`${r.length} archivo(s) GGUF encontrados`);
    } catch (e) {
      notify(`No se pudo escanear: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  const calcular = async () => {
    setOcupado("compat");
    try {
      const c = await api.hubCompat(params, quant, ctx);
      setCompat(c);
      notify(c.veredicto);
    } catch (e) {
      notify(`No se pudo evaluar: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  return (
    <div className="page">
      <div className="page-head">
        <h1>MODEL HUB</h1>
        <span className="chip ok">{instalados.length} modelo(s) en Ollama</span>
      </div>

      <div className="row" style={{ gap: 8, marginBottom: 10, flexWrap: "wrap" }}>
        {(
          [
            ["explorar", "Explorar"],
            ["ollama", "Ollama"],
            ["gguf", "GGUF locales"],
            ["compat", "Compatibilidad"],
          ] as [Pestana, string][]
        ).map(([id, label]) => (
          <button
            key={id}
            className={`btn small ${pestana === id ? "" : "ghost"}`}
            onClick={() => setPestana(id)}
          >
            {label}
          </button>
        ))}
      </div>

      {(pestana === "explorar" || pestana === "ollama") && (
        <div className="card">
          <div className="row" style={{ gap: 8, flexWrap: "wrap" }}>
            <input
              className="input"
              style={{ flex: 1, minWidth: 260 }}
              value={q}
              onChange={(e) => setQ(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") void buscar();
              }}
              placeholder="qwen3, llama, deepseek, mistral…"
            />
            <button className="btn" disabled={ocupado !== null} onClick={() => void buscar()}>
              {ocupado === "buscar" ? "Buscando…" : "Buscar en Ollama + Hugging Face"}
            </button>
          </div>
          <p className="muted small" style={{ marginTop: 8 }}>
            Ollama no tiene API pública de búsqueda (comprobado: 404), así que se lee su página
            pública. Hugging Face se consulta por su API oficial. Buscar no descarga nada.
          </p>
        </div>
      )}

      {pestana === "explorar" && (
        <>
          <div className="card">
            <h2>Librería de Ollama ({ollamaHits?.length ?? 0})</h2>
            {ollamaHits === null ? (
              <div className="empty small">Busca algo arriba.</div>
            ) : ollamaHits.length === 0 ? (
              <div className="empty small">Sin resultados.</div>
            ) : (
              <table className="table">
                <thead>
                  <tr>
                    <th>Modelo</th>
                    <th></th>
                  </tr>
                </thead>
                <tbody>
                  {ollamaHits.map((h) => (
                    <tr key={h.name}>
                      <td className="mono small">{h.name}</td>
                      <td>
                        <div className="row" style={{ gap: 6 }}>
                          <button
                            className="btn ghost small"
                            disabled={ocupado !== null}
                            onClick={() => void verManifiesto(h.name)}
                          >
                            Tamaño
                          </button>
                          <button
                            className="btn ghost small"
                            disabled={ocupado !== null}
                            onClick={() => void descargar(h.name)}
                          >
                            Descargar
                          </button>
                        </div>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
            {manifest && (
              <div className="muted small" style={{ marginTop: 8 }}>
                {manifest.name} ·{" "}
                {manifest.size_gb ? `${manifest.size_gb.toFixed(2)} GB` : "sin tamaño"} ·{" "}
                {manifest.note}
              </div>
            )}
          </div>

          <div className="card">
            <h2>Hugging Face · GGUF ({hf?.length ?? 0})</h2>
            {hf === null ? (
              <div className="empty small">Busca algo arriba.</div>
            ) : hf.length === 0 ? (
              <div className="empty small">Sin resultados.</div>
            ) : (
              <table className="table">
                <thead>
                  <tr>
                    <th>Modelo</th>
                    <th>Autor</th>
                    <th>Licencia</th>
                    <th>Descargas</th>
                    <th>Actualizado</th>
                  </tr>
                </thead>
                <tbody>
                  {hf.map((m) => (
                    <tr key={m.id}>
                      <td className="small">
                        <strong className="mono">{m.id}</strong>
                        <div className="muted small" style={{ fontSize: 11 }}>
                          {m.url}
                        </div>
                      </td>
                      <td className="small">{m.autor}</td>
                      <td className="small">{m.licencia ?? "sin declarar"}</td>
                      <td className="mono small">{m.descargas.toLocaleString("es-ES")}</td>
                      <td className="small">{m.modificado?.slice(0, 10) ?? "—"}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>
        </>
      )}

      {pestana === "ollama" && (
        <>
          <div className="card">
            <div className="row-between" style={{ flexWrap: "wrap" }}>
              <h2 style={{ margin: 0 }}>Instalados ({instalados.length})</h2>
              <button
                className="btn ghost small"
                onClick={() =>
                  void api
                    .engineCards("ollama")
                    .then(setInstalados)
                    .catch((e) => notify(`Ollama: ${e}`))
                }
              >
                Releer
              </button>
            </div>
            <table className="table">
              <thead>
                <tr>
                  <th>Modelo</th>
                  <th>Tamaño</th>
                  <th>Cuantización</th>
                  <th>Contexto</th>
                  <th>Estado</th>
                </tr>
              </thead>
              <tbody>
                {instalados.map((m) => (
                  <tr key={m.id}>
                    <td className="mono small">{m.id}</td>
                    <td className="mono small">
                      {m.size_gb ? `${m.size_gb.toFixed(1)} GB` : "—"}
                    </td>
                    <td className="small">{m.quantization ?? "—"}</td>
                    <td className="mono small">{m.context_length ?? "—"}</td>
                    <td className="small">
                      <span className={`chip ${m.loaded ? "ok" : "subtle"}`}>
                        {m.loaded ? "cargado" : "en disco"}
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          <div className="card">
            <h2>Descargar por nombre exacto</h2>
            <div className="row" style={{ gap: 8, flexWrap: "wrap" }}>
              <input
                className="input"
                style={{ flex: 1, minWidth: 240 }}
                value={q}
                onChange={(e) => setQ(e.target.value)}
                placeholder="llama3.2:3b, qwen3:8b, gemma3:4b…"
              />
              <button
                className="btn"
                disabled={ocupado !== null || !q.trim()}
                onClick={() => void descargar(q.trim())}
              >
                Descargar
              </button>
            </div>
          </div>

          <div className="card">
            <h2>Descargas ({pull.length})</h2>
            {pull.length === 0 ? (
              <div className="empty small">Ninguna descarga en el historial.</div>
            ) : (
              <table className="table">
                <thead>
                  <tr>
                    <th>#</th>
                    <th>Modelo</th>
                    <th>Estado</th>
                    <th>Progreso</th>
                    <th>Error</th>
                  </tr>
                </thead>
                <tbody>
                  {pull.map((j) => {
                    const modelo = (j.payload as { model?: string }).model ?? "—";
                    return (
                      <tr key={j.id}>
                        <td className="mono small">{j.id}</td>
                        <td className="mono small">{modelo}</td>
                        <td className="small">
                          <span className={`chip ${j.state === "done" ? "ok" : j.state === "failed" ? "warn" : "subtle"}`}>
                            {j.state}
                          </span>
                        </td>
                        <td className="small">
                          {j.progress === null || j.progress === undefined
                            ? "—"
                            : `${Math.round(j.progress * 100)} %`}
                        </td>
                        <td className="small warn-text">{j.error.slice(0, 90)}</td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            )}
            <p className="muted small">
              El progreso es el que publica Ollama capa a capa; se actualiza cada pocos segundos.
            </p>
          </div>
        </>
      )}

      {pestana === "gguf" && (
        <div className="card">
          <div className="row-between" style={{ flexWrap: "wrap" }}>
            <h2 style={{ margin: 0 }}>GGUF en disco ({gguf?.length ?? 0})</h2>
            <button className="btn small" disabled={ocupado !== null} onClick={() => void escanearGguf()}>
              {ocupado === "gguf" ? "Escaneando…" : "Escanear discos"}
            </button>
          </div>
          <p className="muted small" style={{ marginTop: 6 }}>
            Lee la cabecera GGUF de cada archivo (magia, versión y arquitectura). Solo lectura: no
            mueve ni copia nada.
          </p>
          {gguf === null ? (
            <div className="empty small">Pulsa «Escanear discos».</div>
          ) : gguf.length === 0 ? (
            <div className="empty small">
              No se encontró ningún .gguf en las rutas conocidas (G:\descargas, J:\models…).
            </div>
          ) : (
            <table className="table">
              <thead>
                <tr>
                  <th>Archivo</th>
                  <th>Tamaño</th>
                  <th>Arquitectura</th>
                  <th>Versión</th>
                  <th>Cuantización</th>
                  <th>Dividido</th>
                  <th>Estado</th>
                </tr>
              </thead>
              <tbody>
                {gguf.map((g) => (
                  <tr key={g.path}>
                    <td className="small">
                      <strong className="mono">{g.nombre}</strong>
                      <div className="muted small" style={{ fontSize: 11 }}>
                        {g.path}
                      </div>
                    </td>
                    <td className="mono small">{g.size_gb.toFixed(2)} GB</td>
                    <td className="small">{g.arquitectura ?? "no legible"}</td>
                    <td className="mono small">{g.version ?? "—"}</td>
                    <td className="small">{g.cuantizacion ?? "—"}</td>
                    <td className="small">{g.dividido ? `sí (${g.parte})` : "no"}</td>
                    <td className="small">
                      {g.valido ? (
                        <span className="chip ok">válido</span>
                      ) : (
                        <span className="chip warn" title={g.nota}>
                          inválido
                        </span>
                      )}
                      {!g.valido && (
                        <div className="muted small" style={{ fontSize: 11 }}>
                          {g.nota}
                        </div>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      )}

      {pestana === "compat" && (
        <>
          <div className="card">
            <h2>¿Cabe aquí?</h2>
            <div className="row" style={{ gap: 10, flexWrap: "wrap", alignItems: "flex-end" }}>
              <label className="field">
                <span>Parámetros (miles de millones)</span>
                <input
                  type="number"
                  min={0.5}
                  step={0.5}
                  value={params}
                  onChange={(e) => setParams(Number(e.target.value))}
                />
              </label>
              <label className="field">
                <span>Cuantización</span>
                <select value={quant} onChange={(e) => setQuant(e.target.value)}>
                  {CUANTS.map((c) => (
                    <option key={c} value={c}>
                      {c}
                    </option>
                  ))}
                </select>
              </label>
              <label className="field">
                <span>Contexto</span>
                <input
                  type="number"
                  min={2048}
                  step={2048}
                  value={ctx}
                  onChange={(e) => setCtx(Number(e.target.value))}
                />
              </label>
              <button className="btn" disabled={ocupado !== null} onClick={() => void calcular()}>
                {ocupado === "compat" ? "Calculando…" : "Evaluar"}
              </button>
            </div>
          </div>

          {compat && (
            <div className="card">
              <div className="row" style={{ gap: 8, flexWrap: "wrap" }}>
                <span className={`chip ${colorVeredicto(compat.veredicto)}`}>{compat.veredicto}</span>
                {compat.total_gb && (
                  <span className="muted small">≈ {compat.total_gb.toFixed(1)} GB en total</span>
                )}
              </div>
              <div className="small" style={{ marginTop: 8 }}>
                {compat.explicacion}
              </div>
              <div className="row" style={{ gap: 16, marginTop: 8, flexWrap: "wrap" }}>
                <span className="small">
                  pesos <strong>{compat.pesos_gb?.toFixed(1) ?? "?"} GB</strong>
                </span>
                <span className="small">
                  KV (estimada) <strong>{compat.kv_gb?.toFixed(1) ?? "?"} GB</strong>
                </span>
                <span className="small">
                  RAM libre <strong>{compat.ram_libre_gb.toFixed(1)} GB</strong>
                </span>
                {compat.requiere_offload && <span className="chip warn">necesita offload</span>}
              </div>
              <table className="table" style={{ marginTop: 10 }}>
                <thead>
                  <tr>
                    <th>GPU</th>
                    <th>Total</th>
                    <th>Libre</th>
                    <th>¿Cabe entero?</th>
                  </tr>
                </thead>
                <tbody>
                  {compat.gpus.map((g) => (
                    <tr key={g.index}>
                      <td className="small">
                        GPU{g.index} · {g.name}
                      </td>
                      <td className="mono small">{g.total_gb.toFixed(1)} GB</td>
                      <td className="mono small">{g.free_gb.toFixed(1)} GB</td>
                      <td className="small">
                        {compat.cabe_en_una_gpu === g.index ? "sí" : "no"}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
              <p className="muted small">
                Los pesos salen de parámetros × bits por peso. La KV es una estimación gruesa
                (GQA aproximado): no se conoce la arquitectura completa antes de descargar. Nunca se
                suman las VRAM como si fueran una sola.
              </p>
            </div>
          )}
        </>
      )}
    </div>
  );
}
