import { useEffect, useMemo, useState } from "react";
import {
  api,
  type AutoInforme,
  type McpServerInfo,
  type McpTool,
  type ToolEntry,
  type ToolsResumen,
} from "../lib/api";

type Pestana = "todas" | "mcp" | "investigacion";

const colorEstado = (s: string): string => {
  if (s === "disponible") return "ok";
  if (s === "requiere autorización") return "warn";
  if (s === "error") return "warn";
  return "subtle";
};

export default function Tools({ notify }: { notify: (m: string) => void }) {
  const [pestana, setPestana] = useState<Pestana>("todas");
  const [registro, setRegistro] = useState<ToolEntry[]>([]);
  const [resumen, setResumen] = useState<ToolsResumen | null>(null);
  const [servidores, setServidores] = useState<McpServerInfo[]>([]);
  const [tools, setTools] = useState<Record<string, McpTool[]>>({});
  const [ocupado, setOcupado] = useState<string | null>(null);

  const [q, setQ] = useState("");
  const [cat, setCat] = useState("");

  const [resultado, setResultado] = useState<{ etiqueta: string; texto: string } | null>(null);

  const [objetivo, setObjetivo] = useState(
    "Buscar herramientas MCP y frameworks de agentes compatibles con Windows y GPU NVIDIA",
  );
  const [informe, setInforme] = useState<AutoInforme | null>(null);

  const cargar = async (avisar = false) => {
    setOcupado("cargar");
    try {
      const [r, s, m] = await Promise.all([
        api.toolsRegistry(),
        api.toolsResumen(),
        api.mcpServers(),
      ]);
      setRegistro(r);
      setResumen(s);
      setServidores(m);
      if (avisar) notify(`${r.length} herramienta(s) en el registro`);
    } catch (e) {
      notify(`No se pudo leer el registro: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  useEffect(() => {
    void cargar();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const categorias = useMemo(
    () => [...new Set(registro.map((t) => t.category))].sort(),
    [registro],
  );

  const filtradas = useMemo(() => {
    const texto = q.trim().toLowerCase();
    return registro.filter((t) => {
      if (cat && t.category !== cat) return false;
      if (!texto) return true;
      return (
        t.id.toLowerCase().includes(texto) ||
        t.description.toLowerCase().includes(texto) ||
        t.name.toLowerCase().includes(texto)
      );
    });
  }, [registro, q, cat]);

  const autorizar = async (s: McpServerInfo) => {
    const accion = s.authorized ? "revocar" : "autorizar";
    const aviso = s.authorized
      ? `Se revocará «${s.name}»: dejará de poder lanzarse.`
      : `Se autorizará «${s.name}», que ejecutará:\n\n${s.command} ${s.args.join(" ")}\n\nEs un proceso externo. ¿Continuar?`;
    if (!window.confirm(aviso)) return;
    setOcupado(`grant:${s.name}`);
    try {
      await api.mcpGrant(s.name, !s.authorized);
      notify(`«${s.name}» ${accion === "revocar" ? "revocado" : "autorizado"}`);
      setServidores(await api.mcpServers());
    } catch (e) {
      notify(`No se pudo cambiar el permiso: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  const descubrir = async (s: McpServerInfo) => {
    setOcupado(`disc:${s.name}`);
    try {
      const lista = await api.mcpDiscover(s.name);
      setTools((prev) => ({ ...prev, [s.name]: lista }));
      setServidores(await api.mcpServers());
      setRegistro(await api.toolsRegistry());
      notify(`«${s.name}»: ${lista.length} herramienta(s)`);
    } catch (e) {
      notify(`No se pudo conectar con «${s.name}»: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  const verTools = async (s: McpServerInfo) => {
    setOcupado(`tools:${s.name}`);
    try {
      const lista = await api.mcpTools(s.name);
      setTools((prev) => ({ ...prev, [s.name]: lista }));
    } catch (e) {
      notify(`No se pudieron leer las herramientas: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  const ejecutar = async (server: string, tool: string) => {
    const crudo = window.prompt(
      `Argumentos JSON para «${tool}» (deja {} si no necesita):`,
      "{}",
    );
    if (crudo === null) return;
    let args: unknown = {};
    try {
      args = crudo.trim() === "" ? {} : JSON.parse(crudo);
    } catch (e) {
      notify(`JSON no válido: ${e}`);
      return;
    }
    setOcupado(`call:${server}:${tool}`);
    try {
      const r = await api.mcpCall(server, tool, args);
      setResultado({ etiqueta: `${server}::${tool}`, texto: JSON.stringify(r, null, 2) });
      notify(`${tool}: ejecutada`);
    } catch (e) {
      notify(`Falló «${tool}»: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  const investigar = async () => {
    setOcupado("investigar");
    try {
      const r = await api.researchAuto(objetivo, 10);
      setInforme(r);
      notify(`${r.relevantes.length} hallazgo(s) · ${r.nuevos} nuevo(s)`);
    } catch (e) {
      notify(`La investigación falló: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  return (
    <div className="page">
      <div className="page-head">
        <h1>KUKIA TOOL CENTER</h1>
        <span className="chip ok">
          {resumen
            ? `${resumen.total} herramienta(s) · ${resumen.disponibles} disponibles · ${resumen.servidores_mcp} servidor(es) MCP`
            : "leyendo…"}
        </span>
      </div>

      <div className="row" style={{ gap: 8, marginBottom: 10, flexWrap: "wrap" }}>
        <button
          className={`btn small ${pestana === "todas" ? "" : "ghost"}`}
          onClick={() => setPestana("todas")}
        >
          Todas ({registro.length})
        </button>
        <button
          className={`btn small ${pestana === "mcp" ? "" : "ghost"}`}
          onClick={() => setPestana("mcp")}
        >
          Servidores MCP ({servidores.length})
        </button>
        <button
          className={`btn small ${pestana === "investigacion" ? "" : "ghost"}`}
          onClick={() => setPestana("investigacion")}
        >
          Investigación autónoma
        </button>
        <button
          className="btn ghost small"
          disabled={ocupado !== null}
          onClick={() => void cargar(true)}
        >
          Releer
        </button>
      </div>

      {pestana === "todas" && (
        <>
          <div className="card">
            <div className="row" style={{ gap: 10, flexWrap: "wrap", alignItems: "flex-end" }}>
              <label className="field" style={{ flex: 1, minWidth: 200 }}>
                <span>Buscar</span>
                <input
                  className="input"
                  value={q}
                  onChange={(e) => setQ(e.target.value)}
                  placeholder="id, nombre o descripción"
                />
              </label>
              <label className="field">
                <span>Categoría</span>
                <select value={cat} onChange={(e) => setCat(e.target.value)}>
                  <option value="">todas</option>
                  {categorias.map((c) => (
                    <option key={c} value={c}>
                      {c}
                    </option>
                  ))}
                </select>
              </label>
            </div>
            {resumen && (
              <div className="row" style={{ gap: 16, marginTop: 10, flexWrap: "wrap" }}>
                <span className="small">núcleo <strong>{resumen.core}</strong></span>
                <span className="small">MCP <strong>{resumen.mcp}</strong></span>
                <span className="small">disponibles <strong>{resumen.disponibles}</strong></span>
                <span className="small">
                  requieren autorización <strong>{resumen.requieren_autorizacion}</strong>
                </span>
                <span className="muted small">sin comprobar <strong>{resumen.sin_comprobar}</strong></span>
              </div>
            )}
          </div>

          <div className="card">
            <h2>Registro ({filtradas.length})</h2>
            <table className="table">
              <thead>
                <tr>
                  <th>Herramienta</th>
                  <th>Categoría</th>
                  <th>Origen</th>
                  <th>Permiso</th>
                  <th>Estado</th>
                  <th>Comprobación</th>
                </tr>
              </thead>
              <tbody>
                {filtradas.map((t) => (
                  <tr key={t.id}>
                    <td className="small">
                      <strong className="mono">{t.id}</strong>
                      <div className="muted small">{t.description}</div>
                    </td>
                    <td className="small">{t.category}</td>
                    <td className="small">
                      {t.origin}
                      {t.server && <span className="muted small"> · {t.server}</span>}
                    </td>
                    <td className="small">{t.permission}</td>
                    <td className="small">
                      <span className={`chip ${colorEstado(t.state)}`}>{t.state}</span>
                    </td>
                    <td className="muted small">{t.detail}</td>
                  </tr>
                ))}
              </tbody>
            </table>
            {filtradas.length === 0 && (
              <div className="empty small">Ninguna herramienta cumple el filtro.</div>
            )}
          </div>
        </>
      )}

      {pestana === "mcp" && (
        <div className="card">
          <h2>Servidores MCP</h2>
          <p className="muted small">
            Los servidores <strong>stdio</strong> son procesos externos (<code>npx</code>,{" "}
            <code>uvx</code>…). Se listan siempre, pero solo se lanzan si los autorizas aquí. Sin
            autorización no se ejecuta nada.
          </p>
          {servidores.length === 0 ? (
            <div className="empty small">No hay ningún mcp.json con servidores.</div>
          ) : (
            <table className="table">
              <thead>
                <tr>
                  <th>Servidor</th>
                  <th>Transporte</th>
                  <th>Comando</th>
                  <th>Permiso</th>
                  <th>Herramientas</th>
                  <th>Estado</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                {servidores.map((s) => (
                  <tr key={s.name}>
                    <td className="small">
                      <strong>{s.name}</strong>
                      <div className="muted small" style={{ fontSize: 11 }}>
                        {s.source}
                      </div>
                    </td>
                    <td className="small">{s.transport}</td>
                    <td className="mono small" title={`${s.command} ${s.args.join(" ")}`}>
                      {s.url ?? `${s.command.split(/[\\/]/).pop()} ${s.args.slice(0, 2).join(" ")}…`}
                    </td>
                    <td className="small">
                      <span className={`chip ${s.authorized ? "ok" : "warn"}`}>
                        {s.authorized ? "autorizado" : "sin autorizar"}
                      </span>
                    </td>
                    <td className="mono small">{s.tools}</td>
                    <td className="muted small">{s.note}</td>
                    <td>
                      <div className="row" style={{ gap: 6 }}>
                        <button
                          className="btn ghost small"
                          disabled={ocupado !== null}
                          onClick={() => void autorizar(s)}
                        >
                          {s.authorized ? "Revocar" : "Autorizar"}
                        </button>
                        <button
                          className="btn ghost small"
                          disabled={!s.authorized || ocupado !== null}
                          onClick={() => void descubrir(s)}
                          title={s.authorized ? "Conectar y listar" : "Autorízalo primero"}
                        >
                          {ocupado === `disc:${s.name}` ? "Conectando…" : "Descubrir"}
                        </button>
                        <button
                          className="btn ghost small"
                          disabled={ocupado !== null}
                          onClick={() => void verTools(s)}
                        >
                          Ver
                        </button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}

          {resultado && (
            <div className="card" style={{ marginTop: 12 }}>
              <div className="row-between">
                <strong className="mono small">{resultado.etiqueta}</strong>
                <button className="btn ghost small" onClick={() => setResultado(null)}>
                  Cerrar
                </button>
              </div>
              <pre className="small" style={{ whiteSpace: "pre-wrap", marginTop: 6 }}>
                {resultado.texto}
              </pre>
            </div>
          )}

          {Object.entries(tools).map(([server, lista]) => (
            <div key={server} style={{ marginTop: 12 }}>
              <h3 className="small">{server}</h3>
              {lista.length === 0 ? (
                <div className="muted small">Sin herramientas descubiertas.</div>
              ) : (
                <table className="table">
                  <thead>
                    <tr>
                      <th>Herramienta</th>
                      <th>Descripción</th>
                      <th></th>
                    </tr>
                  </thead>
                  <tbody>
                    {lista.map((t) => (
                      <tr key={`${server}:${t.name}`}>
                        <td className="mono small">{t.name}</td>
                        <td className="small">{t.description || "—"}</td>
                        <td>
                          <button
                            className="btn ghost small"
                            disabled={ocupado !== null}
                            onClick={() => void ejecutar(server, t.name)}
                          >
                            {ocupado === `call:${server}:${t.name}` ? "Ejecutando…" : "Ejecutar"}
                          </button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </div>
          ))}
        </div>
      )}

      {pestana === "investigacion" && (
        <>
          <div className="card">
            <h2>Investigación autónoma supervisada</h2>
            <p className="muted small">
              Lee arXiv, Hugging Face y GitHub, puntúa por relevancia respecto al objetivo, guarda lo
              nuevo y escribe un informe con sus enlaces. No instala, no descarga modelos y no cambia
              permisos. Los textos de las fuentes son datos, nunca instrucciones.
            </p>
            <div className="row" style={{ gap: 8, marginTop: 8, flexWrap: "wrap" }}>
              <input
                className="input"
                style={{ flex: 1, minWidth: 300 }}
                value={objetivo}
                onChange={(e) => setObjetivo(e.target.value)}
                placeholder="¿Qué quieres averiguar?"
              />
              <button
                className="btn"
                disabled={ocupado !== null || objetivo.trim().length < 8}
                onClick={() => void investigar()}
              >
                {ocupado === "investigar" ? "Investigando… (puede tardar)" : "Investigar"}
              </button>
            </div>
          </div>

          {informe && (
            <div className="card">
              <div className="row-between" style={{ flexWrap: "wrap" }}>
                <h2 style={{ margin: 0 }}>Informe</h2>
                <span className="chip ok">
                  {informe.encontrados} encontrados · {informe.relevantes.length} relevantes ·{" "}
                  {informe.nuevos} nuevos
                </span>
              </div>
              <div className="mono small" style={{ marginTop: 6 }}>
                {informe.ruta}
              </div>

              {informe.contradicciones.length > 0 && (
                <>
                  <div className="muted small" style={{ marginTop: 10 }}>
                    Posibles contradicciones o duplicados
                  </div>
                  <ul className="small">
                    {informe.contradicciones.map((c) => (
                      <li key={c}>{c}</li>
                    ))}
                  </ul>
                </>
              )}

              {informe.propuestas.length > 0 && (
                <>
                  <div className="muted small" style={{ marginTop: 10 }}>
                    Propuestas de integración
                  </div>
                  <ul className="small">
                    {informe.propuestas.map((p) => (
                      <li key={p}>{p}</li>
                    ))}
                  </ul>
                </>
              )}

              <table className="table" style={{ marginTop: 10 }}>
                <thead>
                  <tr>
                    <th>Hallazgo</th>
                    <th>Fuente</th>
                    <th>Tipo</th>
                    <th>Recomendación</th>
                  </tr>
                </thead>
                <tbody>
                  {informe.relevantes.map((d) => (
                    <tr key={d.id}>
                      <td className="small">
                        <strong>{d.title}</strong>
                        <div className="muted small" style={{ fontSize: 11 }}>
                          {d.url}
                        </div>
                      </td>
                      <td className="small">{d.source}</td>
                      <td className="small">{d.kind}</td>
                      <td className="muted small">{d.recommendation || "—"}</td>
                    </tr>
                  ))}
                </tbody>
              </table>

              <ul className="muted small" style={{ marginTop: 8 }}>
                {informe.notas.map((n) => (
                  <li key={n}>{n}</li>
                ))}
              </ul>
            </div>
          )}
        </>
      )}
    </div>
  );
}
