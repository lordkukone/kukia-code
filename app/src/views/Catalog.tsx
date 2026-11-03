import { useEffect, useMemo, useState } from "react";
import {
  api,
  type AdapterInfo,
  type Agent,
  type CatalogBinding,
  type CatalogNeed,
  type CatalogReport,
  type CatalogRoute,
  type UnifiedChat,
  type UniversalModel,
} from "../lib/api";

/** Herramientas del proyecto que pueden pedir modelos por su nombre. */
const HERRAMIENTAS = [
  { id: "chat", nombre: "Chat" },
  { id: "terminal", nombre: "Terminal" },
  { id: "research", nombre: "Investigación" },
  { id: "tasks", nombre: "Tareas / bench" },
  { id: "agents", nombre: "Agentes (general)" },
  { id: "reality", nombre: "Convertidor" },
  { id: "lab", nombre: "Laboratorio" },
];

const CAPS = [
  "texto",
  "codigo",
  "vision",
  "razonamiento",
  "herramientas",
  "imagen",
  "video",
  "audio",
  "embeddings",
];

const ctxTexto = (n: number | null): string => {
  if (!n) return "—";
  if (n >= 1024 && n % 1024 === 0) return `${n / 1024}K`;
  return n.toLocaleString("es-ES");
};

export default function Catalog({ notify }: { notify: (m: string) => void }) {
  const [report, setReport] = useState<CatalogReport | null>(null);
  const [adapters, setAdapters] = useState<AdapterInfo[]>([]);
  const [binds, setBinds] = useState<CatalogBinding[]>([]);
  const [agents, setAgents] = useState<Agent[]>([]);
  const [ocupado, setOcupado] = useState<string | null>(null);

  // Filtros del catálogo
  const [q, setQ] = useState("");
  const [cap, setCap] = useState("");
  const [engine, setEngine] = useState("");
  const [soloCargados, setSoloCargados] = useState(false);

  // Router
  const [needCap, setNeedCap] = useState("texto");
  const [minCtx, setMinCtx] = useState(0);
  const [allowCloud, setAllowCloud] = useState(false);
  const [route, setRoute] = useState<CatalogRoute | null>(null);

  // Fijar modelo / chat
  const [scope, setScope] = useState("chat");
  const [prompt, setPrompt] = useState("En una frase: ¿qué dos GPU tiene este equipo?");
  const [resp, setResp] = useState<UnifiedChat | null>(null);

  const cargar = async (callarAviso = false) => {
    setOcupado("cargar");
    try {
      const [r, a, b] = await Promise.all([
        api.catalogList(),
        api.catalogAdapters(),
        api.catalogBinds(),
      ]);
      setReport(r);
      setAdapters(a);
      setBinds(b);
      if (!callarAviso) notify(`${r.total} modelo(s) en ${r.generated_ms} ms`);
    } catch (e) {
      notify(`No se pudo leer el catálogo: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  useEffect(() => {
    void cargar(true);
    void api
      .agents()
      .then(setAgents)
      .catch(() => setAgents([]));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const capsDisponibles = useMemo(() => {
    const s = new Set<string>();
    for (const m of report?.models ?? []) for (const c of m.capabilities) s.add(c);
    return [...s].sort();
  }, [report]);

  const filtrados = useMemo(() => {
    const texto = q.trim().toLowerCase();
    return (report?.models ?? []).filter((m) => {
      if (soloCargados && !m.loaded) return false;
      if (cap && !m.capabilities.includes(cap)) return false;
      if (engine && m.engine !== engine) return false;
      if (!texto) return true;
      return (
        m.id.toLowerCase().includes(texto) ||
        m.engine.toLowerCase().includes(texto) ||
        m.capabilities.join(" ").toLowerCase().includes(texto)
      );
    });
  }, [report, q, cap, engine, soloCargados]);

  const probarRouter = async () => {
    setOcupado("router");
    const need: CatalogNeed = {
      capability: needCap,
      min_context: minCtx > 0 ? minCtx : null,
      allow_cloud: allowCloud,
      prefer_loaded: true,
      scope: null,
    };
    try {
      const r = await api.catalogRoute(need);
      setRoute(r);
      notify(r.chosen ? `Router: ${r.chosen.key}` : "El router no encontró candidato");
    } catch (e) {
      notify(`Error del router: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  const hablar = async () => {
    setOcupado("chat");
    try {
      const r = await api.catalogChat(prompt, needCap, undefined, 256, allowCloud);
      setResp(r);
      notify(`Respondió ${r.engine}/${r.model}`);
    } catch (e) {
      notify(`El modelo no respondió: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  const fijar = async (m: UniversalModel) => {
    setOcupado(`bind:${m.key}`);
    try {
      await api.catalogBindSet(scope, m.engine, m.id);
      setBinds(await api.catalogBinds());
      notify(`«${scope}» queda fijado a ${m.id}`);
    } catch (e) {
      notify(`No se pudo fijar: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  const soltar = async (s: string) => {
    setOcupado(`clear:${s}`);
    try {
      await api.catalogBindClear(s);
      setBinds(await api.catalogBinds());
      notify(`«${s}» vuelve al enrutado automático`);
    } catch (e) {
      notify(`No se pudo soltar: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  const nombreScope = (id: string): string =>
    HERRAMIENTAS.find((h) => h.id === id)?.nombre ??
    agents.find((a) => a.id === id)?.name ??
    id;

  return (
    <div className="page">
      <div className="page-head">
        <h1>Cerebro universal</h1>
        <span className={`chip ${report ? "ok" : "subtle"}`}>
          {report
            ? `${report.total} modelo(s) · ${report.generated_ms} ms`
            : ocupado === "cargar"
              ? "leyendo motores…"
              : "sin datos"}
        </span>
      </div>

      <div className="card">
        <div className="row-between" style={{ flexWrap: "wrap", gap: 10 }}>
          <div className="muted small">
            Un solo catálogo para todas las herramientas. Solo aparece lo que cada motor enumera
            por su API; lo que no publica catálogo se declara abajo en vez de inventarse.
          </div>
          <button className="btn ghost small" onClick={() => void cargar()} disabled={ocupado !== null}>
            {ocupado === "cargar" ? "Leyendo…" : "Releer motores"}
          </button>
        </div>

        <div className="row" style={{ gap: 10, marginTop: 10, flexWrap: "wrap" }}>
          {(report?.coverage ?? []).map((c) => (
            <span
              key={c.engine}
              className={`chip ${c.reachable ? "ok" : c.catalog_api ? "warn" : "subtle"}`}
              title={c.detail}
            >
              {c.name}: {c.catalog_api ? `${c.models}` : "sin API"}
            </span>
          ))}
        </div>

        <table className="table" style={{ marginTop: 10 }}>
          <thead>
            <tr>
              <th>Motor</th>
              <th>Alcanzable</th>
              <th>Catálogo por API</th>
              <th>Modelos</th>
              <th>Detalle</th>
            </tr>
          </thead>
          <tbody>
            {(report?.coverage ?? []).map((c) => (
              <tr key={c.engine}>
                <td className="small">{c.name}</td>
                <td className="small">{c.reachable ? "sí" : "no"}</td>
                <td className="small">{c.catalog_api ? "sí" : "no"}</td>
                <td className="mono small">{c.catalog_api ? c.models : "—"}</td>
                <td className="muted small">{c.detail}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <div className="card">
        <h2>Router universal</h2>
        <div className="row" style={{ gap: 10, flexWrap: "wrap", alignItems: "flex-end" }}>
          <label className="field">
            <span>Capacidad</span>
            <select value={needCap} onChange={(e) => setNeedCap(e.target.value)}>
              {CAPS.map((c) => (
                <option key={c} value={c}>
                  {c}
                </option>
              ))}
            </select>
          </label>
          <label className="field" style={{ maxWidth: 160 }}>
            <span>Contexto mínimo</span>
            <input
              type="number"
              min={0}
              step={1024}
              value={minCtx}
              onChange={(e) => setMinCtx(Number(e.target.value))}
            />
          </label>
          <label className="field" style={{ flexDirection: "row", gap: 6, alignItems: "center" }}>
            <input
              type="checkbox"
              checked={allowCloud}
              onChange={(e) => setAllowCloud(e.target.checked)}
            />
            <span>Permitir cloud (consume créditos)</span>
          </label>
          <button className="btn" onClick={() => void probarRouter()} disabled={ocupado !== null}>
            {ocupado === "router" ? "Enrutando…" : "Probar router"}
          </button>
        </div>

        {route && (
          <div style={{ marginTop: 12 }}>
            <div className="small">{route.note}</div>
            {route.chosen && (
              <div className="card" style={{ marginTop: 8 }}>
                <div className="row" style={{ gap: 8, flexWrap: "wrap" }}>
                  <span className="chip ok">elegido</span>
                  <strong className="mono small">{route.chosen.key}</strong>
                  <span className="muted small">{route.chosen.score.toFixed(0)} puntos</span>
                  {route.chosen.reasons.map((r) => (
                    <span key={r} className="chip subtle">
                      {r}
                    </span>
                  ))}
                </div>
              </div>
            )}

            {route.alternatives.length > 0 && (
              <>
                <div className="muted small" style={{ marginTop: 8 }}>
                  Alternativas
                </div>
                <ul className="small">
                  {route.alternatives.map((a) => (
                    <li key={a.key} className="mono">
                      {a.key} · {a.score.toFixed(0)} pts
                    </li>
                  ))}
                </ul>
              </>
            )}

            {route.rejected.length > 0 && (
              <>
                <div className="muted small" style={{ marginTop: 8 }}>
                  Descartados y por qué ({route.rejected.length})
                </div>
                <ul className="small">
                  {route.rejected.slice(0, 8).map((r) => (
                    <li key={r.key}>
                      <span className="mono">{r.key}</span> — <span className="muted">{r.why}</span>
                    </li>
                  ))}
                </ul>
              </>
            )}
          </div>
        )}

        <div className="row" style={{ gap: 8, marginTop: 12, flexWrap: "wrap" }}>
          <input
            className="input"
            style={{ flex: 1, minWidth: 260 }}
            value={prompt}
            onChange={(e) => setPrompt(e.target.value)}
            placeholder="Prueba de punta a punta: el router elige y el adaptador responde"
          />
          <button className="btn ghost" onClick={() => void hablar()} disabled={ocupado !== null}>
            {ocupado === "chat" ? "Pensando…" : "Enviar por el router"}
          </button>
        </div>
        {resp && (
          <div className="card" style={{ marginTop: 8 }}>
            <div className="muted small">
              {resp.engine}/{resp.model} · {resp.completion_tokens} tok · {resp.total_ms} ms ·{" "}
              {resp.note}
            </div>
            <div className="small" style={{ whiteSpace: "pre-wrap", marginTop: 6 }}>
              {resp.content}
            </div>
          </div>
        )}
      </div>

      <div className="card">
        <h2>Modelo fijado por herramienta</h2>
        <div className="row" style={{ gap: 10, flexWrap: "wrap", alignItems: "flex-end" }}>
          <label className="field" style={{ minWidth: 220 }}>
            <span>Herramienta / agente</span>
            <select value={scope} onChange={(e) => setScope(e.target.value)}>
              {HERRAMIENTAS.map((h) => (
                <option key={h.id} value={h.id}>
                  {h.nombre}
                </option>
              ))}
              {agents.map((a) => (
                <option key={a.id} value={a.id}>
                  {a.name}
                </option>
              ))}
            </select>
          </label>
          <span className="muted small">
            Al fijar, esa herramienta usa siempre ese modelo. Si desaparece, avisa y vuelve al
            automático.
          </span>
        </div>

        {binds.length === 0 ? (
          <div className="empty small" style={{ marginTop: 8 }}>
            Ninguna herramienta tiene modelo fijado.
          </div>
        ) : (
          <table className="table" style={{ marginTop: 8 }}>
            <thead>
              <tr>
                <th>Herramienta</th>
                <th>Motor</th>
                <th>Modelo</th>
                <th>Actualizado</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {binds.map((b) => (
                <tr key={b.scope}>
                  <td className="small">{nombreScope(b.scope)}</td>
                  <td className="small">{b.engine}</td>
                  <td className="mono small">{b.model}</td>
                  <td className="small">{b.updated_at.slice(0, 19).replace("T", " ")}</td>
                  <td>
                    <button
                      className="btn ghost small"
                      disabled={ocupado !== null}
                      onClick={() => void soltar(b.scope)}
                    >
                      Soltar
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      <div className="card">
        <h2>Catálogo ({filtrados.length})</h2>
        <div className="row" style={{ gap: 10, flexWrap: "wrap", alignItems: "flex-end" }}>
          <label className="field" style={{ flex: 1, minWidth: 200 }}>
            <span>Buscar</span>
            <input
              className="input"
              value={q}
              onChange={(e) => setQ(e.target.value)}
              placeholder="modelo, motor o capacidad"
            />
          </label>
          <label className="field">
            <span>Capacidad</span>
            <select value={cap} onChange={(e) => setCap(e.target.value)}>
              <option value="">todas</option>
              {capsDisponibles.map((c) => (
                <option key={c} value={c}>
                  {c}
                </option>
              ))}
            </select>
          </label>
          <label className="field">
            <span>Motor</span>
            <select value={engine} onChange={(e) => setEngine(e.target.value)}>
              <option value="">todos</option>
              {(report?.coverage ?? []).map((c) => (
                <option key={c.engine} value={c.engine}>
                  {c.name}
                </option>
              ))}
            </select>
          </label>
          <label className="field" style={{ flexDirection: "row", gap: 6, alignItems: "center" }}>
            <input
              type="checkbox"
              checked={soloCargados}
              onChange={(e) => setSoloCargados(e.target.checked)}
            />
            <span>Solo cargados</span>
          </label>
        </div>

        {filtrados.length === 0 ? (
          <div className="empty small" style={{ marginTop: 8 }}>
            {report ? "Ningún modelo cumple el filtro." : "Leyendo motores…"}
          </div>
        ) : (
          <table className="table" style={{ marginTop: 8 }}>
            <thead>
              <tr>
                <th>Modelo</th>
                <th>Motor</th>
                <th>Tipo</th>
                <th>Capacidades</th>
                <th>Contexto</th>
                <th>Tamaño</th>
                <th>Estado</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {filtrados.map((m) => (
                <tr key={m.key}>
                  <td className="small">
                    <strong className="mono">{m.id}</strong>
                    {m.quantization && <span className="muted small"> · {m.quantization}</span>}
                    {m.inferred && (
                      <div className="muted small" style={{ fontSize: 11 }}>
                        capacidades deducidas del nombre
                      </div>
                    )}
                  </td>
                  <td className="small">{m.engine}</td>
                  <td className="small">{m.kind}</td>
                  <td className="small">{m.capabilities.join(", ")}</td>
                  <td className="mono small">{ctxTexto(m.context_length)}</td>
                  <td className="mono small">
                    {m.size_gb ? `${m.size_gb.toFixed(1)} GB` : "—"}
                  </td>
                  <td className="small">
                    <span className={`chip ${m.loaded ? "ok" : m.local ? "subtle" : "warn"}`}>
                      {m.loaded ? "cargado" : m.local ? "en disco" : "remoto"}
                    </span>
                  </td>
                  <td>
                    <button
                      className="btn ghost small"
                      disabled={ocupado !== null}
                      onClick={() => void fijar(m)}
                      title={`Fijar a «${nombreScope(scope)}»`}
                    >
                      Fijar
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      <div className="card">
        <h2>Adaptadores: qué hace cada motor de verdad</h2>
        <table className="table">
          <thead>
            <tr>
              <th>Motor</th>
              <th>Chat</th>
              <th>Stream</th>
              <th>Imagen</th>
              <th>Vídeo</th>
              <th>Audio</th>
              <th>Embeddings</th>
              <th>Descarga</th>
              <th>Detalle</th>
            </tr>
          </thead>
          <tbody>
            {adapters.map((a) => (
              <tr key={a.engine}>
                <td className="small">{a.engine}</td>
                <td className="small">{a.chat ? "sí" : "no"}</td>
                <td className="small">{a.stream ? "sí" : "no"}</td>
                <td className="small">{a.image ? "sí" : "no"}</td>
                <td className="small">{a.video ? "sí" : "no"}</td>
                <td className="small">{a.audio ? "sí" : "no"}</td>
                <td className="small">{a.embeddings ? "sí" : "no"}</td>
                <td className="small">{a.unload ? "sí" : "no"}</td>
                <td className="muted small">{a.detail}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
