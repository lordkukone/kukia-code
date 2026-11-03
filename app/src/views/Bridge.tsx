import { useEffect, useMemo, useState } from "react";
import {
  api,
  type AgentePuente,
  type ModeloPuente,
  type PuenteEstado,
  type PuenteRespuesta,
  type ServeStatus,
} from "../lib/api";

const URL_POR_DEFECTO = "http://127.0.0.1:4096";

export default function Bridge({ notify }: { notify: (m: string) => void }) {
  const [estado, setEstado] = useState<PuenteEstado | null>(null);
  const [modelos, setModelos] = useState<ModeloPuente[]>([]);
  const [agentes, setAgentes] = useState<AgentePuente[]>([]);
  const [servidorKukia, setServidorKukia] = useState<ServeStatus | null>(null);
  const [snippet, setSnippet] = useState("");
  const [ocupado, setOcupado] = useState<string | null>(null);

  const [url, setUrl] = useState(URL_POR_DEFECTO);
  const [provider, setProvider] = useState("");
  const [modelo, setModelo] = useState("");
  const [agente, setAgente] = useState("");
  const [texto, setTexto] = useState("Responde solo: PUENTE OK");
  const [respuesta, setRespuesta] = useState<PuenteRespuesta | null>(null);
  const [verRazon, setVerRazon] = useState(false);

  const cargar = async (avisar = false) => {
    setOcupado("cargar");
    try {
      const [e, m, a, k] = await Promise.all([
        api.puenteEstado(url),
        api.puenteModelos(url),
        api.puenteAgentes(url),
        api.kukiaServerStatus(),
      ]);
      setEstado(e);
      setModelos(m);
      setAgentes(a);
      setServidorKukia(k);
      const primero = m.find((x) => x.provider === "ollama") ?? m[0];
      if (!provider && primero) {
        setProvider(primero.provider);
        setModelo(primero.id);
      }
      if (avisar) {
        notify(
          `OpenCode ${e.version || "no detectado"} · ${m.length} modelo(s) · ${a.length} agente(s)`,
        );
      }
    } catch (e) {
      notify(`No se pudo leer el puente: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  useEffect(() => {
    void cargar();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const proveedores = useMemo(() => {
    const cuenta = new Map<string, number>();
    for (const m of modelos) cuenta.set(m.provider, (cuenta.get(m.provider) ?? 0) + 1);
    return [...cuenta.entries()].sort((a, b) => b[1] - a[1]);
  }, [modelos]);

  const modelosDelProvider = useMemo(
    () => modelos.filter((m) => m.provider === provider),
    [modelos, provider],
  );

  const arrancarOpencode = async () => {
    setOcupado("srv-on");
    try {
      const msg = await api.puenteServidorArrancar();
      notify(msg);
      await cargar();
    } catch (e) {
      notify(`No se pudo arrancar el servidor de OpenCode: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  const pararOpencode = async () => {
    setOcupado("srv-off");
    try {
      notify(await api.puenteServidorParar());
      await cargar();
    } catch (e) {
      notify(`No se pudo parar: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  const preguntar = async () => {
    if (!provider || !modelo || !texto.trim()) return;
    setOcupado("preguntar");
    setRespuesta(null);
    try {
      const r = await api.puentePreguntar({
        url,
        provider,
        model: modelo,
        agent: agente || null,
        texto,
        timeout: 240,
      });
      setRespuesta(r);
      notify(`OpenCode respondió en ${(r.ms / 1000).toFixed(1)} s`);
    } catch (e) {
      notify(`OpenCode no respondió: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  const arrancarKukia = async () => {
    setOcupado("kukia-on");
    try {
      const st = await api.kukiaServerStart();
      setServidorKukia(st);
      setSnippet(await api.puenteSnippet());
      notify(`Servidor de KUKIA en ${st.url} (solo loopback)`);
    } catch (e) {
      notify(`No se pudo abrir el servidor de KUKIA: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  const pararKukia = async () => {
    setOcupado("kukia-off");
    try {
      await api.kukiaServerStop();
      setServidorKukia(await api.kukiaServerStatus());
      notify("Servidor de KUKIA detenido");
    } catch (e) {
      notify(`No se pudo detener: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  const generarSnippet = async () => {
    setOcupado("snippet");
    try {
      setSnippet(await api.puenteSnippet());
      notify("Fragmento generado. KUKIA no escribe en opencode.json.");
    } catch (e) {
      notify(`No se pudo generar el fragmento: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  const copiar = async (txt: string) => {
    try {
      await navigator.clipboard.writeText(txt);
      notify("Copiado al portapapeles");
    } catch {
      notify("El navegador no dejó copiar: selecciona el texto a mano");
    }
  };

  return (
    <div className="page">
      <div className="page-head">
        <h1>PUENTE OPENCODE</h1>
        <span className={`chip ${estado?.opencode_instalado ? "ok" : "warn"}`}>
          {estado
            ? estado.opencode_instalado
              ? `OpenCode ${estado.version}`
              : "OpenCode no detectado"
            : "leyendo…"}
        </span>
        <span className={`chip ${estado?.servidor.vivo ? "ok" : "subtle"}`}>
          servidor {estado?.servidor.vivo ? "vivo" : "parado"}
        </span>
      </div>

      <div className="row" style={{ gap: 8, marginBottom: 10, flexWrap: "wrap" }}>
        <label className="field" style={{ minWidth: 240 }}>
          <span>URL del servidor de OpenCode</span>
          <input className="input" value={url} onChange={(e) => setUrl(e.target.value)} />
        </label>
        <button
          className="btn ghost small"
          disabled={ocupado !== null}
          onClick={() => void arrancarOpencode()}
        >
          {ocupado === "srv-on" ? "Arrancando…" : "Arrancar opencode serve"}
        </button>
        <button
          className="btn ghost small"
          disabled={ocupado !== null}
          onClick={() => void pararOpencode()}
        >
          {ocupado === "srv-off" ? "Parando…" : "Parar el que arrancó KUKIA"}
        </button>
        <button
          className="btn ghost small"
          disabled={ocupado !== null}
          onClick={() => void cargar(true)}
        >
          {ocupado === "cargar" ? "Leyendo…" : "Releer"}
        </button>
      </div>

      <div className="card">
        <h2>1 · Lo que OpenCode aporta a KUKIA</h2>
        <div className="row" style={{ gap: 18, flexWrap: "wrap" }}>
          <span className="small">
            modelos <strong>{modelos.length}</strong>
          </span>
          <span className="small">
            agentes <strong>{agentes.length}</strong>
          </span>
          <span className="small">
            servidor <strong>{estado?.servidor.modelos ?? 0}</strong> modelos ·{" "}
            <strong>{estado?.servidor.sesiones ?? 0}</strong> sesiones
          </span>
          <span className="small">
            MCP declarados <strong>{estado?.mcp_config.length ?? 0}</strong>
          </span>
        </div>
        <div className="muted small mono" style={{ marginTop: 6 }}>
          {estado?.config_path}
        </div>
        {estado?.servidor.error && (
          <div className="muted small" style={{ marginTop: 4 }}>
            Sondeo del servidor: {estado.servidor.error}
          </div>
        )}
        {estado?.mcp_config.length ? (
          <div className="small" style={{ marginTop: 6 }}>
            MCP: <span className="mono">{estado.mcp_config.join(" · ")}</span>
          </div>
        ) : null}
        <div className="row" style={{ gap: 6, marginTop: 8, flexWrap: "wrap" }}>
          {proveedores.map(([p, n]) => (
            <span key={p} className="chip subtle">
              {p} · {n}
            </span>
          ))}
        </div>
      </div>

      <div className="card">
        <h2>2 · Pedir una respuesta a OpenCode</h2>
        <p className="muted small">
          KUKIA abre una sesión propia en OpenCode y le manda el mensaje. El catálogo y los agentes
          son los reales de tu instalación; si algo falla se muestra el error tal cual.
        </p>
        <div className="row" style={{ gap: 10, flexWrap: "wrap", alignItems: "flex-end" }}>
          <label className="field">
            <span>Proveedor</span>
            <select
              value={provider}
              onChange={(e) => {
                setProvider(e.target.value);
                const primero = modelos.find((m) => m.provider === e.target.value);
                setModelo(primero?.id ?? "");
              }}
            >
              {proveedores.map(([p, n]) => (
                <option key={p} value={p}>
                  {p} ({n})
                </option>
              ))}
            </select>
          </label>
          <label className="field" style={{ minWidth: 260 }}>
            <span>Modelo</span>
            <select value={modelo} onChange={(e) => setModelo(e.target.value)}>
              {modelosDelProvider.map((m) => (
                <option key={m.full} value={m.id}>
                  {m.id}
                </option>
              ))}
            </select>
          </label>
          <label className="field">
            <span>Agente</span>
            <select value={agente} onChange={(e) => setAgente(e.target.value)}>
              <option value="">(por defecto de OpenCode)</option>
              {agentes.map((a) => (
                <option key={a.nombre} value={a.nombre}>
                  {a.nombre} · {a.modo}
                </option>
              ))}
            </select>
          </label>
        </div>
        <textarea
          className="input"
          style={{ width: "100%", minHeight: 70, marginTop: 8 }}
          value={texto}
          onChange={(e) => setTexto(e.target.value)}
        />
        <div className="row" style={{ marginTop: 8, gap: 8 }}>
          <button
            className="btn"
            disabled={ocupado !== null || !provider || !modelo}
            onClick={() => void preguntar()}
          >
            {ocupado === "preguntar" ? "Esperando a OpenCode…" : "Preguntar"}
          </button>
          {estado?.agente_por_defecto && (
            <span className="muted small">agente por defecto: {estado.agente_por_defecto}</span>
          )}
        </div>

        {respuesta && (
          <div style={{ marginTop: 12 }}>
            <div className="row-between" style={{ flexWrap: "wrap" }}>
              <span className="small">
                <strong className="mono">{respuesta.modelo}</strong> · agente {respuesta.agente} ·{" "}
                {(respuesta.ms / 1000).toFixed(1)} s · {respuesta.tokens_out} tokens
              </span>
              <span className="muted small mono">{respuesta.session_id}</span>
            </div>
            <pre
              className="small"
              style={{ whiteSpace: "pre-wrap", marginTop: 6, maxHeight: 320, overflow: "auto" }}
            >
              {respuesta.texto}
            </pre>
            {respuesta.razonamiento && (
              <>
                <button className="btn ghost small" onClick={() => setVerRazon(!verRazon)}>
                  {verRazon ? "Ocultar razonamiento" : "Ver razonamiento"}
                </button>
                {verRazon && (
                  <pre
                    className="muted small"
                    style={{ whiteSpace: "pre-wrap", maxHeight: 240, overflow: "auto" }}
                  >
                    {respuesta.razonamiento}
                  </pre>
                )}
              </>
            )}
          </div>
        )}
      </div>

      <div className="card">
        <h2>3 · Lo que KUKIA aporta a OpenCode</h2>
        <p className="muted small">
          KUKIA abre un servidor <strong>solo en 127.0.0.1</strong> con la API de OpenAI
          (<code>/v1/models</code> y <code>/v1/chat/completions</code>) para que OpenCode use sus
          modelos. Exige token y nunca escribe en tu <code>opencode.json</code>: te da el fragmento
          exacto para pegarlo tú.
        </p>
        <div className="row" style={{ gap: 18, flexWrap: "wrap" }}>
          <span className={`chip ${servidorKukia?.running ? "ok" : "subtle"}`}>
            {servidorKukia?.running ? `escuchando en ${servidorKukia.url}` : "parado"}
          </span>
          <span className="small">
            peticiones <strong>{servidorKukia?.peticiones ?? 0}</strong> · errores{" "}
            <strong>{servidorKukia?.errores ?? 0}</strong>
          </span>
          {servidorKukia?.ultimo_error && (
            <span className="muted small">último error: {servidorKukia.ultimo_error}</span>
          )}
        </div>
        <div className="row" style={{ gap: 8, marginTop: 8, flexWrap: "wrap" }}>
          <button className="btn" disabled={ocupado !== null} onClick={() => void arrancarKukia()}>
            {ocupado === "kukia-on" ? "Abriendo…" : "Abrir servidor de KUKIA"}
          </button>
          <button
            className="btn ghost"
            disabled={ocupado !== null || !servidorKukia?.running}
            onClick={() => void pararKukia()}
          >
            Detener
          </button>
          <button className="btn ghost" disabled={ocupado !== null} onClick={() => void generarSnippet()}>
            {ocupado === "snippet" ? "Generando…" : "Generar fragmento para OpenCode"}
          </button>
          {snippet && (
            <button className="btn ghost" onClick={() => void copiar(snippet)}>
              Copiar
            </button>
          )}
        </div>
        {servidorKukia?.token && (
          <div className="muted small" style={{ marginTop: 6 }}>
            Token local de esta sesión: <span className="mono">{servidorKukia.token}</span>
          </div>
        )}
        {snippet && (
          <pre className="mono small" style={{ whiteSpace: "pre-wrap", marginTop: 8 }}>
            {snippet}
          </pre>
        )}
      </div>
    </div>
  );
}
