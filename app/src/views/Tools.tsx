const TOOLS = [
  { name: "files.list", desc: "Listar directorios", state: "implementado" },
  { name: "files.read", desc: "Leer archivos de texto", state: "implementado" },
  { name: "files.write", desc: "Escribir archivos (requiere aprobación)", state: "implementado" },
  { name: "terminal.run", desc: "Ejecutar PowerShell (requiere aprobación)", state: "implementado" },
  { name: "memory.search", desc: "Buscar en memoria SQLite", state: "implementado" },
  { name: "memory.add", desc: "Guardar recuerdo", state: "implementado" },
  { name: "providers.list", desc: "Detectar Ollama / LM Studio / cloud", state: "implementado" },
  { name: "gpu.stats", desc: "Métricas NVIDIA vía nvidia-smi", state: "implementado" },
  { name: "benchmark.run", desc: "Medir tokens/s reales", state: "implementado" },
  { name: "radar.check", desc: "Consultar releases oficiales", state: "implementado" },
  { name: "mcp.client", desc: "Cliente MCP genérico", state: "pendiente F6" },
  { name: "research.autonomous", desc: "Investigación autónoma supervisada", state: "pendiente F9" },
];

export default function Tools() {
  return (
    <div className="page">
      <div className="page-head">
        <h1>Herramientas</h1>
        <span className="chip subtle">estado real del MVP</span>
      </div>
      <div className="card">
        <table className="table">
          <thead>
            <tr>
              <th>Herramienta</th>
              <th>Descripción</th>
              <th>Estado</th>
            </tr>
          </thead>
          <tbody>
            {TOOLS.map((t) => (
              <tr key={t.name}>
                <td className="mono">{t.name}</td>
                <td>{t.desc}</td>
                <td>
                  <span className={`tag ${t.state.startsWith("implementado") ? "ok-tag" : ""}`}>
                    {t.state}
                  </span>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
