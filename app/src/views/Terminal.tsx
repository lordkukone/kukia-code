import { useState } from "react";
import { api } from "../lib/api";

interface RunResult {
  code: number;
  stdout: string;
  stderr: string;
}

export default function TerminalView({ notify }: { notify: (m: string) => void }) {
  const [cwd, setCwd] = useState("H:\\KUKIA_CODE");
  const [command, setCommand] = useState("");
  const [approved, setApproved] = useState(false);
  const [history, setHistory] = useState<{ cmd: string; out: RunResult }[]>([]);
  const [busy, setBusy] = useState(false);

  const run = async () => {
    if (!command.trim() || busy) return;
    if (!approved) {
      notify("Marca la aprobación antes de ejecutar comandos");
      return;
    }
    setBusy(true);
    try {
      const out = await api.runCommand(command, cwd, true);
      setHistory((h) => [...h, { cmd: command, out }]);
      setCommand("");
    } catch (e) {
      notify(`Error ejecutando: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="page">
      <div className="page-head">
        <h1>Terminal</h1>
        <label className="inline">
          <input type="checkbox" checked={approved} onChange={(e) => setApproved(e.target.checked)} />
          Aprobar ejecución (queda registrada)
        </label>
      </div>

      <div className="card">
        <label className="inline">
          Directorio
          <input className="path-input mono" value={cwd} onChange={(e) => setCwd(e.target.value)} />
        </label>
      </div>

      <div className="terminal">
        {history.map((h, i) => (
          <div key={i} className="term-block">
            <div className="term-cmd">
              <span className="prompt">PS&gt;</span> {h.cmd}
            </div>
            {h.out.stdout && <pre className="term-out">{h.out.stdout}</pre>}
            {h.out.stderr && <pre className="term-err">{h.out.stderr}</pre>}
            <div className="term-code">exit code: {h.out.code}</div>
          </div>
        ))}
        {history.length === 0 && <div className="empty small">Sin comandos ejecutados.</div>}
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
