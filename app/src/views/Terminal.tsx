import { useRef, useState } from "react";
import { api, runCommandStream, type RunResult } from "../lib/api";

interface Entrada {
  cmd: string;
  out?: RunResult;
  id?: string;
  pid?: number;
  error?: string;
  running: boolean;
}

export default function TerminalView({ notify }: { notify: (m: string) => void }) {
  const [cwd, setCwd] = useState("H:\\KUKIA_CODE");
  const [command, setCommand] = useState("");
  const [approved, setApproved] = useState(false);
  const [timeout, setTimeoutSecs] = useState(60);
  const [entradas, setEntradas] = useState<Entrada[]>([]);
  const [busy, setBusy] = useState(false);
  const enCurso = useRef<string | null>(null);

  const actualizar = (indice: number, cambio: Partial<Entrada>) =>
    setEntradas((prev) => prev.map((e, i) => (i === indice ? { ...e, ...cambio } : e)));

  const run = async () => {
    if (!command.trim() || busy) return;
    if (!approved) {
      notify("Marca la aprobación antes de ejecutar comandos");
      return;
    }
    const indice = entradas.length;
    const cmd = command;
    setEntradas((prev) => [...prev, { cmd, running: true }]);
    setCommand("");
    setBusy(true);
    try {
      const id = await runCommandStream(
        cmd,
        cwd,
        true,
        (eid, pid) => {
          enCurso.current = eid;
          actualizar(indice, { id: eid, pid });
        },
        (result) => {
          enCurso.current = null;
          actualizar(indice, { out: result, running: false });
          setBusy(false);
        },
        (msg) => {
          enCurso.current = null;
          actualizar(indice, { error: msg, running: false });
          setBusy(false);
        },
        timeout,
      );
      enCurso.current = id;
      actualizar(indice, { id });
    } catch (e) {
      enCurso.current = null;
      actualizar(indice, { error: String(e), running: false });
      notify(`Error ejecutando: ${e}`);
      setBusy(false);
    }
  };

  const cancelar = async () => {
    const id = enCurso.current;
    if (!id) {
      notify("No hay ningún comando en ejecución");
      return;
    }
    try {
      const enviado = await api.commandCancel(id);
      notify(enviado ? "Cancelación enviada" : "El comando ya había terminado");
    } catch (e) {
      notify(`No se pudo cancelar: ${e}`);
    }
  };

  const estado = (e: Entrada) => {
    if (e.running) return { texto: "en ejecución…", clase: "term-state running" };
    if (e.error) return { texto: `error: ${e.error}`, clase: "term-state bad" };
    const o = e.out;
    if (!o) return { texto: "", clase: "term-state" };
    if (o.timed_out)
      return {
        texto: `tiempo agotado a los ${(o.duration_ms / 1000).toFixed(1)} s (proceso terminado)`,
        clase: "term-state bad",
      };
    if (o.cancelled) return { texto: "cancelado por el usuario", clase: "term-state bad" };
    return {
      texto: `exit code: ${o.code} · ${o.duration_ms} ms · pid ${o.pid}${
        o.truncated ? " · salida recortada a 1 MB" : ""
      }`,
      clase: "term-state",
    };
  };

  return (
    <div className="page">
      <div className="page-head">
        <h1>Terminal</h1>
        <label className="inline">
          <input type="checkbox" checked={approved} onChange={(e) => setApproved(e.target.checked)} />
          Aprobar ejecución (queda registrada y auditada)
        </label>
      </div>

      <div className="card">
        <label className="inline">
          Directorio
          <input className="path-input mono" value={cwd} onChange={(e) => setCwd(e.target.value)} />
        </label>
        <label className="inline">
          Límite de tiempo (s)
          <input
            className="mono timeout-input"
            type="number"
            min={1}
            max={3600}
            value={timeout}
            onChange={(e) => setTimeoutSecs(Number(e.target.value) || 60)}
          />
        </label>
        <p className="muted small">
          El directorio debe estar dentro de las rutas autorizadas del agente en Seguridad → Permisos.
          Al agotarse el tiempo se mata el proceso y todos sus hijos.
        </p>
      </div>

      <div className="terminal">
        {entradas.map((e, i) => {
          const s = estado(e);
          return (
            <div key={i} className="term-block">
              <div className="term-cmd">
                <span className="prompt">PS&gt;</span> {e.cmd}
                {e.pid ? <span className="muted small"> · pid {e.pid}</span> : null}
                {e.running && (
                  <button className="btn small danger" onClick={cancelar}>
                    Cancelar
                  </button>
                )}
              </div>
              {e.out?.stdout && <pre className="term-out">{e.out.stdout}</pre>}
              {e.out?.stderr && <pre className="term-err">{e.out.stderr}</pre>}
              {s.texto && <div className={s.clase}>{s.texto}</div>}
            </div>
          );
        })}
        {entradas.length === 0 && <div className="empty small">Sin comandos ejecutados.</div>}
      </div>

      <div className="composer">
        <input
          className="mono"
          value={command}
          placeholder="Escribe un comando de PowerShell…"
          onChange={(e) => setCommand(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && run()}
        />
        <button className="btn primary" onClick={run} disabled={busy || !approved}>
          {busy ? "Ejecutando…" : "Ejecutar"}
        </button>
      </div>
    </div>
  );
}
