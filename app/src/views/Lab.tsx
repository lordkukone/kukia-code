import { useEffect, useState } from "react";
import {
  api,
  type BenchResult,
  type Comparacion,
  type Experimento,
  type InferenceParams,
  type ModelCard,
} from "../lib/api";

const PROMPT_PRUEBA =
  "Explica en cinco frases qué hace un gestor de memoria de VRAM en un sistema con dos GPU.";

/** Parámetros completos con todo a null: el motor decide. */
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

const NIVELES = ["todos", "estable", "experimental", "muy_experimental", "incompatible"];

const colorNivel = (nivel: string): string => {
  switch (nivel) {
    case "estable":
      return "#15803d";
    case "experimental":
      return "#b45309";
    case "muy_experimental":
      return "#7c3aed";
    default:
      return "#b91c1c";
  }
};

/** Ajustes rápidos de B: solo cambian una cosa respecto a A para que compare. */
const PRESETS_AB: { id: string; nombre: string; aplicar: (a: InferenceParams) => InferenceParams }[] = [
  {
    id: "num_gpu",
    nombre: "Reparto automático (A) vs todo en CPU (B)",
    aplicar: (a) => ({ ...a, num_gpu: 0 }),
  },
  {
    id: "ctx",
    nombre: "Contexto actual (A) vs 4K (B)",
    aplicar: (a) => ({ ...a, num_ctx: 4096 }),
  },
  {
    id: "flash",
    nombre: "Sin flash attention (A) vs con flash attention (B)",
    aplicar: (a) => ({ ...a, flash_attn: true }),
  },
  {
    id: "batch",
    nombre: "num_batch del motor (A) vs 512 (B)",
    aplicar: (a) => ({ ...a, num_batch: 512 }),
  },
];

const fmt = (v: number | null, ud = "", dec = 1) =>
  v === null || !Number.isFinite(v) ? "—" : `${v.toFixed(dec)}${ud}`;

export default function Lab({ notify }: { notify: (m: string) => void }) {
  const [catalogo, setCatalogo] = useState<Experimento[]>([]);
  const [nivel, setNivel] = useState("todos");
  const [busqueda, setBusqueda] = useState("");
  const [detalle, setDetalle] = useState<Experimento | null>(null);

  const [engine, setEngine] = useState("ollama");
  const [modelos, setModelos] = useState<ModelCard[]>([]);
  const [model, setModel] = useState("");
  const [params, setParams] = useState<InferenceParams | null>(null);

  const [prompt, setPrompt] = useState(PROMPT_PRUEBA);
  const [tokens, setTokens] = useState(128);
  const [bench, setBench] = useState<BenchResult | null>(null);
  const [preset, setPreset] = useState(PRESETS_AB[0].id);
  const [comp, setComp] = useState<Comparacion | null>(null);
  const [historial, setHistorial] = useState<BenchResult[]>([]);
  const [ocupado, setOcupado] = useState<string | null>(null);

  useEffect(() => {
    void api
      .labCatalogo()
      .then(setCatalogo)
      .catch((e) => notify(`No se pudo leer el catálogo: ${e}`));
    void api.labHistorial().then(setHistorial).catch(() => setHistorial([]));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const cargarModelos = async (motor: string) => {
    try {
      const fichas = await api.engineCards(motor);
      setModelos(fichas);
      const primero = fichas.find((f) => f.loaded) ?? fichas[0];
      if (primero) await elegir(motor, primero.id);
    } catch (e) {
      setModelos([]);
      notify(`No se pudieron listar los modelos de ${motor}: ${e}`);
    }
  };

  const elegir = async (motor: string, id: string) => {
    setModel(motor === engine ? id : id);
    try {
      setParams(await api.engineParamsGet(motor, id));
    } catch {
      setParams(null);
    }
  };

  useEffect(() => {
    void cargarModelos(engine);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [engine]);

  const ejecutarPrueba = async () => {
    if (!model) {
      notify("Elige un modelo antes de medir");
      return;
    }
    setOcupado("bench");
    try {
      const r = await api.labBench(engine, model, params ?? PARAMS_VACIOS, prompt, tokens);
      setBench(r);
      setHistorial(await api.labHistorial());
      notify(
        r.ok
          ? `Medido: ${r.tok_s.toFixed(1)} tok/s (${r.tokens} tokens)`
          : `La prueba falló: ${r.error}`,
      );
    } catch (e) {
      notify(`Error en el banco de pruebas: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  const comparar = async () => {
    if (!model) {
      notify("Elige un modelo con parámetros cargados antes de comparar");
      return;
    }
    const p = PRESETS_AB.find((x) => x.id === preset);
    if (!p) return;
    setOcupado("ab");
    const base = params ?? PARAMS_VACIOS;
    try {
      const c = await api.labAb(engine, model, base, p.aplicar(base), prompt, tokens);
      setComp(c);
      setHistorial(await api.labHistorial());
      notify(c.veredicto);
    } catch (e) {
      notify(`Error comparando: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  const activar = async (e: Experimento) => {
    if (!model) {
      notify("Elige un modelo antes de activar un experimento");
      return;
    }
    setOcupado(`activar:${e.id}`);
    try {
      const [nuevo, mensaje] = await api.labActivar(e.id, engine, model);
      setParams(nuevo);
      notify(mensaje);
    } catch (err) {
      notify(`No se activó: ${err}`);
    } finally {
      setOcupado(null);
    }
  };

  const restaurar = async () => {
    if (!model) return;
    setOcupado("restaurar");
    try {
      const anterior = await api.labRestaurar(engine, model);
      if (anterior) {
        setParams(anterior);
        notify("Configuración anterior restaurada");
      } else {
        notify("No hay configuración anterior guardada para este modelo");
      }
    } catch (e) {
      notify(`No se pudo restaurar: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  const filtrado = catalogo.filter(
    (e) =>
      (nivel === "todos" || e.nivel === nivel) &&
      (busqueda.trim() === "" ||
        `${e.nombre} ${e.categoria} ${e.descripcion}`
          .toLowerCase()
          .includes(busqueda.toLowerCase())),
  );

  const cuentas = (n: string) => catalogo.filter((e) => e.nivel === n).length;

  return (
    <div className="page">
      <div className="page-head">
        <h1>Laboratorio experimental</h1>
        <span className="chip subtle">
          {cuentas("estable")} estables · {cuentas("experimental")} experimentales ·{" "}
          {cuentas("muy_experimental")} muy experimentales · {cuentas("incompatible")} incompatibles
        </span>
      </div>

      <div className="card">
        <p className="muted small">
          Nada se activa solo y nada se presenta como compatible sin comprobarlo. Los experimentos
          con mecanismo <code>variable_servidor</code> no se pueden aplicar desde aquí: exigen
          cambiar una variable de Ollama y reiniciarlo, y KUKIA no reinicia procesos. Ahí verás el
          valor real que tiene tu servidor ahora y la orden exacta para cambiarlo tú.
        </p>
        <div className="row" style={{ gap: 10, flexWrap: "wrap", alignItems: "flex-end" }}>
          <label className="field">
            <span>Motor</span>
            <select value={engine} onChange={(e) => setEngine(e.target.value)}>
              <option value="ollama">Ollama</option>
              <option value="lmstudio">LM Studio</option>
            </select>
          </label>
          <label className="field" style={{ minWidth: 260 }}>
            <span>Modelo</span>
            <select
              value={model}
              onChange={(e) => {
                setModel(e.target.value);
                void elegir(engine, e.target.value);
              }}
            >
              {modelos.length === 0 && <option value="">sin modelos</option>}
              {modelos.map((m) => (
                <option key={m.id} value={m.id}>
                  {m.id}
                  {m.loaded ? " · cargado" : ""}
                </option>
              ))}
            </select>
          </label>
          <button className="btn ghost small" onClick={() => cargarModelos(engine)}>
            Recargar modelos
          </button>
          <button className="btn ghost small" onClick={restaurar} disabled={ocupado === "restaurar"}>
            Restaurar configuración
          </button>
        </div>
      </div>

      {/* Catálogo */}
      <div className="card">
        <div className="row-between">
          <h2 style={{ margin: 0 }}>Catálogo de experimentos</h2>
          <div className="row" style={{ gap: 8 }}>
            <select value={nivel} onChange={(e) => setNivel(e.target.value)}>
              {NIVELES.map((n) => (
                <option key={n} value={n}>
                  {n}
                </option>
              ))}
            </select>
            <input
              className="search"
              placeholder="Buscar técnica…"
              value={busqueda}
              onChange={(e) => setBusqueda(e.target.value)}
            />
          </div>
        </div>
        <table className="table">
          <thead>
            <tr>
              <th>Técnica</th>
              <th>Categoría</th>
              <th>Nivel</th>
              <th>Mecanismo</th>
              <th>Estado en esta máquina</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {filtrado.map((e) => (
              <tr key={e.id} className={detalle?.id === e.id ? "selected" : ""}>
                <td className="small">{e.nombre}</td>
                <td className="small muted">{e.categoria}</td>
                <td className="small">
                  <span className="tag" style={{ color: colorNivel(e.nivel) }}>
                    {e.nivel.replace("_", " ")}
                  </span>
                </td>
                <td className="small mono">{e.mecanismo}</td>
                <td className="small">{e.estado_local}</td>
                <td>
                  <button className="btn ghost small" onClick={() => setDetalle(e)}>
                    Ver
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        {filtrado.length === 0 && <div className="empty small">Ningún experimento coincide.</div>}
      </div>

      {/* Detalle */}
      {detalle && (
        <div className="card">
          <div className="row-between">
            <h2 style={{ margin: 0 }}>
              {detalle.nombre}{" "}
              <span className="tag" style={{ color: colorNivel(detalle.nivel) }}>
                {detalle.nivel.replace("_", " ")}
              </span>
            </h2>
            <button className="btn ghost small" onClick={() => setDetalle(null)}>
              Cerrar
            </button>
          </div>
          <p className="small">{detalle.descripcion}</p>
          <table className="table">
            <tbody>
              <tr>
                <td className="muted small" style={{ width: 160 }}>
                  Qué hace al activarse
                </td>
                <td className="small">{detalle.efecto}</td>
              </tr>
              <tr>
                <td className="muted small">Motores donde existe</td>
                <td className="small">
                  {detalle.motores.length > 0 ? detalle.motores.join(", ") : "ninguno instalado"}
                </td>
              </tr>
              <tr>
                <td className="muted small">Estado real</td>
                <td className="small">{detalle.estado_local}</td>
              </tr>
              {detalle.variable && (
                <tr>
                  <td className="muted small">Variable del servidor</td>
                  <td className="small mono">
                    {detalle.variable}={detalle.valor_actual ?? "(sin definir)"}
                  </td>
                </tr>
              )}
              {detalle.requisitos.length > 0 && (
                <tr>
                  <td className="muted small">Requisitos</td>
                  <td className="small">{detalle.requisitos.join(" · ")}</td>
                </tr>
              )}
              {detalle.riesgos.length > 0 && (
                <tr>
                  <td className="muted small">Riesgos</td>
                  <td className="small warn-text">{detalle.riesgos.join(" · ")}</td>
                </tr>
              )}
            </tbody>
          </table>

          <div className="row" style={{ gap: 8, marginTop: 10 }}>
            {detalle.activable ? (
              <button
                className="btn"
                onClick={() => activar(detalle)}
                disabled={!model || ocupado === `activar:${detalle.id}`}
              >
                {ocupado === `activar:${detalle.id}` ? "Activando…" : "Activar experimento"}
              </button>
            ) : (
              <span className="muted small">
                No se puede activar desde KUKIA: {detalle.efecto}.
              </span>
            )}
            <button className="btn ghost" onClick={restaurar} disabled={ocupado === "restaurar"}>
              Restaurar configuración
            </button>
          </div>
        </div>
      )}

      {/* Banco de pruebas */}
      <div className="card">
        <h2>Banco de pruebas</h2>
        <p className="muted small">
          Genera con una petición fija y mide: tokens/s, prefill, latencia del primer token,
          latencia entre tokens, pico de VRAM por GPU, pico de RAM, temperatura y vatios. Ollama
          publica esos tiempos; LM Studio no, así que allí solo se mide el tiempo total real.
        </p>
        <textarea
          value={prompt}
          onChange={(e) => setPrompt(e.target.value)}
          rows={2}
          style={{ width: "100%" }}
        />
        <div className="row" style={{ gap: 10, marginTop: 8, alignItems: "flex-end" }}>
          <label className="field">
            <span>Tokens a generar</span>
            <input
              type="number"
              min={16}
              max={1024}
              value={tokens}
              onChange={(e) => setTokens(Number(e.target.value))}
            />
          </label>
          <button className="btn" onClick={ejecutarPrueba} disabled={ocupado === "bench" || !model}>
            {ocupado === "bench" ? "Midiendo…" : "Ejecutar prueba"}
          </button>
          <span className="muted small">
            parámetros actuales: {params ? JSON.stringify(params).slice(0, 0) : ""}
            {params?.num_ctx ? `ctx ${params.num_ctx}` : "ctx del motor"}
            {params?.num_gpu !== null && params?.num_gpu !== undefined
              ? ` · num_gpu ${params.num_gpu < 0 ? "auto" : params.num_gpu}`
              : ""}
          </span>
        </div>

        {bench && (
          <div className="card" style={{ marginTop: 12 }}>
            <div className="row-between">
              <strong className={bench.ok ? "ok-text" : "warn-text"}>
                {bench.ok ? `${bench.tok_s.toFixed(1)} tok/s` : bench.error}
              </strong>
              <span className="muted small">{bench.params_texto}</span>
            </div>
            <table className="table">
              <tbody>
                <tr>
                  <td className="muted small">tokens generados</td>
                  <td className="small">{bench.tokens}</td>
                  <td className="muted small">prefill</td>
                  <td className="small">{fmt(bench.prefill_tok_s, " tok/s")}</td>
                </tr>
                <tr>
                  <td className="muted small">primer token (TTFT)</td>
                  <td className="small">{fmt(bench.ttft_ms, " ms", 0)}</td>
                  <td className="muted small">entre tokens</td>
                  <td className="small">{fmt(bench.itl_ms, " ms", 2)}</td>
                </tr>
                <tr>
                  <td className="muted small">carga del modelo</td>
                  <td className="small">{fmt(bench.carga_ms, " ms", 0)}</td>
                  <td className="muted small">total</td>
                  <td className="small">{fmt(bench.total_ms, " ms", 0)}</td>
                </tr>
                <tr>
                  <td className="muted small">pico de VRAM</td>
                  <td className="small">
                    {bench.vram_pico.length === 0
                      ? "—"
                      : bench.vram_pico
                          .map((g) => `GPU${g.index} ${(g.pico_mb / 1024).toFixed(1)} GB`)
                          .join(" · ")}
                  </td>
                  <td className="muted small">pico de RAM</td>
                  <td className="small">{bench.ram_pico_gb.toFixed(1)} GB</td>
                </tr>
                <tr>
                  <td className="muted small">temperatura pico</td>
                  <td className="small">{bench.temp_pico_c.toFixed(0)} °C</td>
                  <td className="muted small">potencia pico</td>
                  <td className="small">{bench.potencia_pico_w.toFixed(0)} W</td>
                </tr>
              </tbody>
            </table>
          </div>
        )}
      </div>

      {/* A/B */}
      <div className="card">
        <h2>Comparación A/B</h2>
        <p className="muted small">
          A = configuración actual del modelo. B = la misma con un solo cambio. Se ejecutan las dos
          seguidas con el mismo prompt y los mismos tokens.
        </p>
        <div className="row" style={{ gap: 8, flexWrap: "wrap", alignItems: "flex-end" }}>
          <label className="field" style={{ minWidth: 380 }}>
            <span>Cambio de B</span>
            <select value={preset} onChange={(e) => setPreset(e.target.value)}>
              {PRESETS_AB.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.nombre}
                </option>
              ))}
            </select>
          </label>
          <button className="btn" onClick={comparar} disabled={ocupado === "ab" || !model}>
            {ocupado === "ab" ? "Comparando…" : "Comparar A/B"}
          </button>
        </div>

        {comp && (
          <div style={{ marginTop: 12 }}>
            <p className={comp.delta_tok_s_pct > 2 ? "ok-text" : comp.delta_tok_s_pct < -2 ? "warn-text" : "muted"}>
              <strong>{comp.veredicto}</strong>
            </p>
            <table className="table">
              <thead>
                <tr>
                  <th>Configuración</th>
                  <th>tok/s</th>
                  <th>prefill</th>
                  <th>TTFT</th>
                  <th>VRAM pico</th>
                  <th>RAM pico</th>
                  <th>Temp.</th>
                </tr>
              </thead>
              <tbody>
                {[comp.a, comp.b].map((r, i) => (
                  <tr key={i}>
                    <td className="small">
                      <strong>{i === 0 ? "A" : "B"}</strong> · {r.params_texto}
                    </td>
                    <td className="small">{r.tok_s.toFixed(1)}</td>
                    <td className="small">{fmt(r.prefill_tok_s, " tok/s")}</td>
                    <td className="small">{fmt(r.ttft_ms, " ms", 0)}</td>
                    <td className="small">
                      {r.vram_pico.map((g) => `${(g.pico_mb / 1024).toFixed(1)}`).join(" / ")} GB
                    </td>
                    <td className="small">{r.ram_pico_gb.toFixed(1)} GB</td>
                    <td className="small">{r.temp_pico_c.toFixed(0)} °C</td>
                  </tr>
                ))}
              </tbody>
            </table>
            <p className="muted small">
              VRAM: {comp.delta_vram_mb.map(([i, d]) => `GPU${i} ${(d / 1024).toFixed(2)} GB`).join(" · ")}{" "}
              (B − A) · RAM {(comp.delta_ram_gb).toFixed(2)} GB · diferencia{" "}
              {comp.delta_tok_s >= 0 ? "+" : ""}
              {comp.delta_tok_s.toFixed(1)} tok/s
            </p>
            <p className="muted small">
              Una sola medida no demuestra nada: repite la comparación antes de dar un cambio por
              bueno, y no uses el resultado para elegir cuantización sin mirar la calidad.
            </p>
          </div>
        )}
      </div>

      {/* Historial */}
      <div className="card">
        <h2>Historial de medidas</h2>
        {historial.length === 0 ? (
          <div className="empty small">Todavía no hay pruebas guardadas.</div>
        ) : (
          <table className="table">
            <thead>
              <tr>
                <th>Cuándo</th>
                <th>Modelo</th>
                <th>Configuración</th>
                <th>tok/s</th>
                <th>tokens</th>
                <th>VRAM pico</th>
              </tr>
            </thead>
            <tbody>
              {[...historial].reverse().slice(0, 15).map((r, i) => (
                <tr key={`${r.tomado_en}-${i}`}>
                  <td className="small">{new Date(r.tomado_en).toLocaleString("es-ES")}</td>
                  <td className="small mono">
                    {r.engine}/{r.model}
                  </td>
                  <td className="small">{r.params_texto}</td>
                  <td className="small">{r.tok_s.toFixed(1)}</td>
                  <td className="small">{r.tokens}</td>
                  <td className="small">
                    {r.vram_pico.map((g) => `${g.index}:${(g.pico_mb / 1024).toFixed(1)}`).join(" ")}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}
