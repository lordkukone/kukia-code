import { useEffect, useState } from "react";
import {
  api,
  type EngineStatus,
  type InferenceParams,
  type ModelCard,
  type ModelInfo,
  type Preflight,
  type ProviderInfo,
} from "../lib/api";
import ModelAdvanced from "./ModelAdvanced";

interface Props {
  providers: ProviderInfo[];
  models: ModelInfo[];
  provider: string;
  model: string;
  onSelectProvider: (id: string) => void;
  onSelectModel: (id: string) => void;
  onRefresh: () => Promise<ProviderInfo[]>;
  notify: (msg: string) => void;
}

const PARAMS_VACIOS: InferenceParams = {
  temperature: null,
  top_p: null,
  top_k: null,
  num_ctx: null,
  num_gpu: null,
  repeat_penalty: null,
  seed: null,
  max_tokens: null,
  keep_alive: null,
  min_p: null,
  repeat_last_n: null,
  presence_penalty: null,
  frequency_penalty: null,
  stop: [],
  num_batch: null,
  num_thread: null,
  flash_attn: null,
  use_mmap: null,
  use_mlock: null,
  gpu_offload: null,
  parallel: null,
  speculative: null,
};

/** Un motor local de los que se pueden controlar (CC1–CC4). */
const esLocal = (id: string) => id === "ollama" || id === "lmstudio";

const num = (v: string): number | null => {
  const t = v.trim();
  if (t === "") return null;
  const n = Number(t);
  return Number.isFinite(n) ? n : null;
};

export default function Models(p: Props) {
  const [filter, setFilter] = useState("");
  const [loading, setLoading] = useState(false);
  // Los modelos gratuitos del proveedor vienen marcados con la capacidad «gratis».
  const [soloGratis, setSoloGratis] = useState(false);

  // CC1
  const [estados, setEstados] = useState<EngineStatus[]>([]);
  const [midiendo, setMidiendo] = useState(false);
  // CC2
  const [fichas, setFichas] = useState<ModelCard[]>([]);
  const [ficha, setFicha] = useState<ModelCard | null>(null);
  // CC3
  const [params, setParams] = useState<InferenceParams>(PARAMS_VACIOS);
  const [avisos, setAvisos] = useState<string[]>([]);
  // CC4
  const [pre, setPre] = useState<Preflight | null>(null);
  const [ocupado, setOcupado] = useState<string | null>(null);

  useEffect(() => {
    setFilter("");
    setFicha(null);
    setPre(null);
    if (esLocal(p.provider)) void cargarFichas(p.provider);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [p.provider]);

  useEffect(() => {
    void medirMotores();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const medirMotores = async () => {
    setMidiendo(true);
    try {
      setEstados(await api.enginesStatus());
    } catch (e) {
      p.notify(`Error midiendo motores: ${e}`);
    } finally {
      setMidiendo(false);
    }
  };

  const cargarFichas = async (engine: string) => {
    try {
      setFichas(await api.engineCards(engine));
    } catch (e) {
      setFichas([]);
      p.notify(`No se pudieron leer las fichas de ${engine}: ${e}`);
    }
  };

  const refresh = async () => {
    setLoading(true);
    try {
      const list = await p.onRefresh();
      const current = list.find((x) => x.id === p.provider);
      if (current) await p.onSelectProvider(current.id);
      await medirMotores();
      if (esLocal(p.provider)) await cargarFichas(p.provider);
    } finally {
      setLoading(false);
    }
  };

  /** CC2 + CC3: abre la ficha completa y sus parámetros guardados. */
  const abrirFicha = async (model: string) => {
    setOcupado(`ficha:${model}`);
    try {
      const [c, prm] = await Promise.all([
        api.engineCard(p.provider, model),
        api.engineParamsGet(p.provider, model),
      ]);
      setFicha(c);
      setParams(prm);
      setAvisos([]);
      setPre(null);
    } catch (e) {
      p.notify(`No se pudo leer la ficha: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  const guardarParams = async () => {
    if (!ficha) return;
    setOcupado("params");
    try {
      const w = await api.engineParamsSet(ficha.engine, ficha.id, params);
      setAvisos(w);
      p.notify(w.length ? "Parámetros guardados con avisos" : "Parámetros guardados");
    } catch (e) {
      p.notify(`Error guardando parámetros: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  /** CC4: primero estimar, y cargar solo si cabe. */
  const comprobar = async () => {
    if (!ficha) return;
    setOcupado("preflight");
    try {
      setPre(await api.enginePreflight(ficha.engine, ficha.id));
    } catch (e) {
      p.notify(`Error en el preflight: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  const cargarModelo = async (model: string) => {
    setOcupado(`carga:${model}`);
    try {
      const r = await api.engineLoad(p.provider, model);
      p.notify(r.ok ? `Cargado ${model}: ${r.message}` : `No se cargó ${model}: ${r.message}`);
      await medirMotores();
      await cargarFichas(p.provider);
    } catch (e) {
      p.notify(`Error cargando: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  const expulsar = async (model: string) => {
    setOcupado(`expulsa:${model}`);
    try {
      const r = await api.vramUnload(p.provider, model);
      p.notify(r.ok ? `Expulsado ${model}` : `No se pudo expulsar: ${r.message}`);
      await medirMotores();
      await cargarFichas(p.provider);
    } catch (e) {
      p.notify(`Error expulsando: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  const esFree = (m: ModelInfo) => m.capabilities.includes("gratis");
  const gratisDelProveedor = p.models.filter(esFree).length;
  const shown = p.models.filter(
    (m) =>
      (!soloGratis || esFree(m)) &&
      (m.id.toLowerCase().includes(filter.toLowerCase()) ||
        m.name.toLowerCase().includes(filter.toLowerCase())),
  );

  const cargado = (id: string) => fichas.find((f) => f.id === id)?.loaded ?? false;

  return (
    <div className="page">
      <div className="page-head">
        <h1>Modelos</h1>
        <div className="row" style={{ gap: 8 }}>
          <button className="btn ghost" onClick={medirMotores} disabled={midiendo}>
            {midiendo ? "Midiendo…" : "Medir motores"}
          </button>
          <button className="btn" onClick={refresh} disabled={loading}>
            {loading ? "Escaneando…" : "Refrescar"}
          </button>
        </div>
      </div>

      {/* CC1 — estado real de cada servidor */}
      <div className="provider-grid">
        {estados.map((e) => (
          <div key={e.engine} className="provider-card">
            <div className="provider-top">
              <span className="provider-name">
                {e.engine === "ollama" ? "Ollama" : "LM Studio"}
                {e.version ? ` · v${e.version}` : ""}
              </span>
              <span className={`status-dot ${e.online ? "online" : "offline"}`} />
            </div>
            <div className="provider-meta mono">
              {e.online ? e.base_url : "sin respuesta"}
              {e.port ? ` · puerto ${e.port}` : ""}
            </div>
            <div className="provider-meta">puerto por {e.port_source}</div>
            <div className="provider-bottom">
              <span>{e.installed} modelos</span>
              <span>{e.latency_ms} ms</span>
            </div>
            <div className={`provider-note ${e.loaded.length ? "ok-text" : ""}`}>
              {e.loaded.length ? `cargado: ${e.loaded.join(", ")}` : "nada cargado"}
            </div>
            {!e.online && <div className="provider-note warn-text">{e.note}</div>}
          </div>
        ))}
        {estados.length === 0 && (
          <div className="empty small">{midiendo ? "Midiendo servidores…" : "Sin lectura todavía."}</div>
        )}
      </div>

      <div className="provider-grid">
        {p.providers.map((pr) => (
          <button
            key={pr.id}
            className={`provider-card ${pr.id === p.provider ? "active" : ""}`}
            onClick={() => p.onSelectProvider(pr.id)}
          >
            <div className="provider-top">
              <span className="provider-name">{pr.name}</span>
              <span className={`status-dot ${pr.online ? "online" : "offline"}`} />
            </div>
            <div className="provider-meta">{pr.kind}</div>
            <div className="provider-meta mono">{pr.base_url || "—"}</div>
            <div className="provider-bottom">
              <span>
                {pr.models_count} modelos
                {pr.free_models > 0 ? ` · ${pr.free_models} gratis` : ""}
              </span>
              <span className={pr.online ? "ok-text" : "warn-text"}>
                {pr.online ? "online" : "offline"}
              </span>
            </div>
            {pr.note && <div className="provider-note">{pr.note}</div>}
          </button>
        ))}
      </div>

      <div className="card">
        <div className="card-head">
          <h2>
            Catálogo · {p.provider}
            {gratisDelProveedor > 0 && (
              <span className="chip ok" style={{ marginLeft: 8 }}>
                {gratisDelProveedor} gratis
              </span>
            )}
          </h2>
          <div className="row" style={{ gap: 8, alignItems: "center" }}>
            <label className="small" style={{ display: "flex", gap: 6, alignItems: "center" }}>
              <input
                type="checkbox"
                checked={soloGratis}
                onChange={(e) => setSoloGratis(e.target.checked)}
              />
              Solo gratis
            </label>
            <input
              className="search"
              placeholder="Filtrar modelos…"
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
            />
          </div>
        </div>
        {shown.length === 0 ? (
          <div className="empty small">
            {soloGratis && gratisDelProveedor === 0 ? (
              <>
                Este proveedor no ofrece modelos gratis.{" "}
                {(() => {
                  const con = p.providers.find((pr) => pr.free_models > 0);
                  return con ? (
                    <button
                      className="btn ghost small"
                      onClick={() => {
                        p.onSelectProvider(con.id);
                        setSoloGratis(true);
                      }}
                    >
                      Ver los {con.free_models} gratis de {con.name}
                    </button>
                  ) : (
                    <span>Ningún proveedor conectado anuncia modelos gratis.</span>
                  );
                })()}
              </>
            ) : (
              <>No hay modelos visibles. Si el proveedor está offline, arráncalo y pulsa Refrescar.</>
            )}
          </div>
        ) : (
          <table className="table">
            <thead>
              <tr>
                <th>Modelo</th>
                <th>Contexto</th>
                <th>Tamaño</th>
                <th>Capacidades</th>
                <th>Estado</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {shown.map((m) => (
                <tr key={m.id} className={m.id === p.model ? "selected" : ""}>
                  <td className="mono">
                    {m.name}
                    {esFree(m) && (
                      <span className="chip ok" style={{ marginLeft: 6 }}>
                        gratis
                      </span>
                    )}
                  </td>
                  <td>{m.context ? m.context.toLocaleString() : "—"}</td>
                  <td>{m.size_gb ? `${m.size_gb.toFixed(1)} GB` : "—"}</td>
                  <td>
                    {m.capabilities.filter((c) => c !== "gratis").join(", ") || "texto"}
                  </td>
                  <td>
                    {cargado(m.id) ? <span className="tag ok-tag">cargado</span> : m.state}
                  </td>
                  <td>
                    <div className="row" style={{ gap: 6 }}>
                      {esLocal(p.provider) && (
                        <>
                          <button
                            className="btn ghost small"
                            onClick={() => abrirFicha(m.id)}
                            disabled={ocupado === `ficha:${m.id}`}
                          >
                            {ocupado === `ficha:${m.id}` ? "…" : "Ficha"}
                          </button>
                          {cargado(m.id) ? (
                            <button
                              className="btn ghost small"
                              onClick={() => expulsar(m.id)}
                              disabled={ocupado === `expulsa:${m.id}`}
                            >
                              Expulsar
                            </button>
                          ) : (
                            <button
                              className="btn ghost small"
                              onClick={() => cargarModelo(m.id)}
                              disabled={ocupado === `carga:${m.id}`}
                              title="Lo carga solo si el preflight dice que cabe"
                            >
                              {ocupado === `carga:${m.id}` ? "…" : "Cargar"}
                            </button>
                          )}
                        </>
                      )}
                      <button
                        className="btn small"
                        onClick={() => {
                          p.onSelectModel(m.id);
                          p.notify(`Modelo activo: ${m.name}`);
                        }}
                      >
                        Usar
                      </button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      {/* CC2/CC3/CC4 — ficha, parámetros y carga controlada */}
      {ficha && (
        <div className="card">
          <div className="card-head">
            <h2>Ficha · {ficha.id}</h2>
            <button className="btn ghost small" onClick={() => setFicha(null)}>
              Cerrar
            </button>
          </div>

          <table className="table">
            <tbody>
              <tr>
                <td className="muted small">Motor</td>
                <td className="small">{ficha.engine}</td>
                <td className="muted small">Arquitectura</td>
                <td className="small">{ficha.architecture ?? "—"}</td>
              </tr>
              <tr>
                <td className="muted small">Parámetros</td>
                <td className="small">{ficha.parameter_size ?? "—"}</td>
                <td className="muted small">Cuantización</td>
                <td className="small">{ficha.quantization ?? "—"}</td>
              </tr>
              <tr>
                <td className="muted small">Formato</td>
                <td className="small">{ficha.format ?? "—"}</td>
                <td className="muted small">Tamaño</td>
                <td className="small">{ficha.size_gb ? `${ficha.size_gb.toFixed(2)} GB` : "—"}</td>
              </tr>
              <tr>
                <td className="muted small">Contexto</td>
                <td className="small">
                  {ficha.context_length ? ficha.context_length.toLocaleString() : "—"}
                </td>
                <td className="muted small">Cargado</td>
                <td className="small">{ficha.loaded ? "sí" : "no"}</td>
              </tr>
              <tr>
                <td className="muted small">Capacidades</td>
                <td className="small">{ficha.capabilities.join(", ") || "—"}</td>
                <td className="muted small">Origen</td>
                <td className="small">{ficha.publisher ?? "—"}</td>
              </tr>
            </tbody>
          </table>
          {ficha.note && <p className="muted small">{ficha.note}</p>}

          <h2 style={{ marginTop: 18 }}>Parámetros de inferencia</h2>
          <p className="muted small">
            Vacío = lo decide el motor. Solo se envía lo que escribas.
          </p>
          <div className="form-grid">
            {(
              [
                ["temperature", "Temperatura"],
                ["top_p", "top_p"],
                ["top_k", "top_k (solo Ollama)"],
                ["num_ctx", "num_ctx (contexto)"],
                ["num_gpu", "num_gpu (capas, solo Ollama)"],
                ["repeat_penalty", "repeat_penalty"],
                ["seed", "seed"],
                ["max_tokens", "max_tokens"],
              ] as [keyof InferenceParams, string][]
            ).map(([campo, etiqueta]) => (
              <label key={campo}>
                {etiqueta}
                <input
                  type="number"
                  step="any"
                  value={
                    typeof params[campo] === "number" || typeof params[campo] === "string"
                      ? (params[campo] as number | string)
                      : ""
                  }
                  onChange={(e) =>
                    setParams({ ...params, [campo]: num(e.target.value) })
                  }
                />
              </label>
            ))}
            <label>
              keep_alive (5m / 0)
              <input
                value={params.keep_alive ?? ""}
                onChange={(e) =>
                  setParams({ ...params, keep_alive: e.target.value.trim() || null })
                }
              />
            </label>
          </div>

          <div className="row" style={{ gap: 8, marginTop: 10 }}>
            <button className="btn" onClick={guardarParams} disabled={ocupado === "params"}>
              {ocupado === "params" ? "Guardando…" : "Guardar parámetros"}
            </button>
            <button className="btn ghost" onClick={comprobar} disabled={ocupado === "preflight"}>
              {ocupado === "preflight" ? "Estimando…" : "Preflight"}
            </button>
            {!ficha.loaded && (
              <button className="btn" onClick={() => cargarModelo(ficha.id)}>
                Cargar
              </button>
            )}
          </div>

          {avisos.length > 0 && (
            <div style={{ marginTop: 10 }}>
              <strong className="small">Este motor no soporta:</strong>
              {avisos.map((a) => (
                <p key={a} className="warn-text small">
                  {a}
                </p>
              ))}
            </div>
          )}

          {pre && (
            <div className="card" style={{ marginTop: 12 }}>
              <div className="row-between">
                <strong className={pre.fits ? "ok-text" : "warn-text"}>
                  {pre.fits ? "cabe y se puede cargar" : "NO cabe: no se cargará"}
                </strong>
                <span className="muted small">fuente: {pre.source}</span>
              </div>
              <p className="small">{pre.detail}</p>
              {pre.needed_gb != null && (
                <p className="muted small">
                  necesario ≈ {pre.needed_gb.toFixed(2)} GB
                  {pre.best_gpu
                    ? ` · GPU${pre.best_gpu.index} libre ${pre.best_gpu.free_gb.toFixed(2)} GB`
                    : ""}
                </p>
              )}
              {pre.warnings.map((w) => (
                <p key={w} className="muted small">
                  {w}
                </p>
              ))}
            </div>
          )}

          <ModelAdvanced
            engine={ficha.engine}
            model={ficha.id}
            params={params}
            onParams={setParams}
            notify={p.notify}
          />
        </div>
      )}
    </div>
  );
}
