import { useEffect, useState } from "react";
import { api, type AgentPolicy, type AuditRow, type Project } from "../lib/api";

export default function Security({ notify }: { notify: (m: string) => void }) {
  const [politicas, setPoliticas] = useState<AgentPolicy[]>([]);
  const [edit, setEdit] = useState<AgentPolicy | null>(null);
  const [aprobado, setAprobado] = useState(false);
  const [auditoria, setAuditoria] = useState<AuditRow[]>([]);
  const [rutaCsv, setRutaCsv] = useState("H:\\KUKIA_CODE\\auditoria.csv");
  const [proyectos, setProyectos] = useState<Project[]>([]);
  const [activo, setActivo] = useState<Project | null>(null);
  const [nuevoNombre, setNuevoNombre] = useState("");
  const [nuevaRaiz, setNuevaRaiz] = useState("");
  const [shellProyecto, setShellProyecto] = useState(false);

  const cargarPoliticas = async () => {
    try {
      const p = await api.policyList();
      setPoliticas(p);
      if (!edit && p.length > 0) setEdit(p[0]);
    } catch (e) {
      notify(`Error cargando permisos: ${e}`);
    }
  };

  const cargarAuditoria = async () => {
    try {
      setAuditoria(await api.auditList(undefined, undefined, 60));
    } catch (e) {
      notify(`Error cargando auditoría: ${e}`);
    }
  };

  const cargarProyectos = async () => {
    try {
      const [lista, actual] = await Promise.all([api.projects(), api.activeProject()]);
      setProyectos(lista);
      setActivo(actual);
    } catch (e) {
      notify(`Error cargando proyectos: ${e}`);
    }
  };

  const crearProyecto = async () => {
    try {
      const p = await api.createProject(nuevoNombre, nuevaRaiz, shellProyecto);
      notify(`Proyecto dado de alta: ${p.name}`);
      setNuevoNombre("");
      setNuevaRaiz("");
      setShellProyecto(false);
      await cargarProyectos();
      await cargarAuditoria();
    } catch (e) {
      notify(`No se pudo crear: ${e}`);
    }
  };

  const activar = async (id: string | null) => {
    try {
      await api.activateProject(id);
      notify(id ? "Proyecto activado: sus rutas acotan los permisos" : "Sin proyecto: ámbito global del agente");
      await cargarProyectos();
      await cargarAuditoria();
    } catch (e) {
      notify(`No se pudo activar: ${e}`);
    }
  };

  useEffect(() => {
    cargarPoliticas();
    cargarProyectos();
    cargarAuditoria();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const guardar = async () => {
    if (!edit) return;
    if (!aprobado) {
      notify("Marca la confirmación antes de cambiar permisos");
      return;
    }
    try {
      const guardada = await api.policySave(edit, true);
      notify(`Permisos guardados para ${guardada.agent}`);
      setAprobado(false);
      await cargarPoliticas();
      await cargarAuditoria();
    } catch (e) {
      notify(`No se pudo guardar: ${e}`);
    }
  };

  const exportar = async () => {
    try {
      const filas = await api.auditExport(rutaCsv, true);
      notify(`Auditoría exportada: ${filas} filas en ${rutaCsv}`);
      await cargarAuditoria();
    } catch (e) {
      notify(`No se pudo exportar: ${e}`);
    }
  };

  const toggle = (campo: keyof AgentPolicy) => {
    if (!edit) return;
    setEdit({ ...edit, [campo]: !edit[campo] } as AgentPolicy);
  };

  return (
    <div className="page">
      <div className="page-head">
        <h1>Seguridad</h1>
        <span className="chip subtle">{politicas.length} políticas · {auditoria.length} acciones auditadas</span>
      </div>

      <div className="split">
        <div className="card file-list">
          <h2>Agentes</h2>
          {politicas.map((p) => (
            <button
              key={p.agent}
              className={`file-row ${edit?.agent === p.agent ? "selected" : ""}`}
              onClick={() => {
                setEdit(p);
                setAprobado(false);
              }}
            >
              <span>{p.allow_write ? "✍️" : "🔒"}</span>
              <span className="file-name mono">{p.agent}</span>
              <span className="muted small">{p.allow_shell ? "shell" : "sin shell"}</span>
            </button>
          ))}
          {politicas.length === 0 && (
            <div className="empty small">
              Sin políticas. Reinicia la app para sembrar las de por defecto.
            </div>
          )}
        </div>

        <div className="card">
          <h2>Permisos de {edit?.agent ?? "—"}</h2>
          {edit ? (
            <>
              <div className="policy-toggles">
                <label className="inline">
                  <input
                    type="checkbox"
                    checked={edit.allow_read}
                    onChange={() => toggle("allow_read")}
                  />
                  Lectura
                </label>
                <label className="inline">
                  <input
                    type="checkbox"
                    checked={edit.allow_write}
                    onChange={() => toggle("allow_write")}
                  />
                  Escritura
                </label>
                <label className="inline">
                  <input
                    type="checkbox"
                    checked={edit.allow_shell}
                    onChange={() => toggle("allow_shell")}
                  />
                  Ejecutar comandos
                </label>
                <label className="inline">
                  <input
                    type="checkbox"
                    checked={edit.allow_network}
                    onChange={() => toggle("allow_network")}
                  />
                  Red
                </label>
                <label className="inline">
                  <input
                    type="checkbox"
                    checked={edit.allow_download}
                    onChange={() => toggle("allow_download")}
                  />
                  Descargas
                </label>
                <label className="inline">
                  <input
                    type="checkbox"
                    checked={edit.allow_tools}
                    onChange={() => toggle("allow_tools")}
                  />
                  Herramientas externas
                </label>
              </div>

              <label className="inline">
                Rutas de escritura (separadas por ;)
                <input
                  className="path-input mono"
                  value={edit.write_paths}
                  onChange={(e) => setEdit({ ...edit, write_paths: e.target.value })}
                />
              </label>
              <label className="inline">
                Rutas de solo lectura
                <input
                  className="path-input mono"
                  value={edit.read_only_paths}
                  onChange={(e) => setEdit({ ...edit, read_only_paths: e.target.value })}
                />
              </label>
              <label className="inline">
                Rutas de lectura permitidas (vacío = todas)
                <input
                  className="path-input mono"
                  value={edit.read_paths}
                  onChange={(e) => setEdit({ ...edit, read_paths: e.target.value })}
                />
              </label>

              <label className="inline">
                <input
                  type="checkbox"
                  checked={aprobado}
                  onChange={(e) => setAprobado(e.target.checked)}
                />
                Confirmo el cambio de permisos (queda auditado)
              </label>
              <div className="composer">
                <button className="btn primary" onClick={guardar} disabled={!aprobado}>
                  Guardar permisos
                </button>
              </div>
              <p className="muted small">
                El terminal solo puede ejecutarse dentro de las rutas de escritura. Una operación
                destructiva exige además permiso de escritura y aprobación por comando.
              </p>
            </>
          ) : (
            <div className="empty small">Selecciona un agente.</div>
          )}
        </div>
      </div>

      <div className="card">
        <div className="page-head">
          <h2>Proyectos</h2>
          <span className={`chip ${activo ? "ok" : "subtle"}`}>
            {activo ? `activo: ${activo.name}` : "sin proyecto activo"}
          </span>
        </div>
        <p className="muted small">
          Con un proyecto activo las rutas se acotan dos veces: el agente debe permitirlo y el
          destino debe estar dentro del proyecto. El proyecto nunca amplía lo que el agente no
          tiene, solo recorta.
        </p>

        {proyectos.length === 0 && (
          <div className="empty small">Sin proyectos. Da de alta uno con su carpeta raíz.</div>
        )}
        <table className="table">
          <thead>
            <tr>
              <th>Proyecto</th>
              <th>Raíz</th>
              <th>Escritura</th>
              <th>Comandos</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {proyectos.map((p) => (
              <tr key={p.id}>
                <td>
                  {p.name}
                  {activo?.id === p.id && <span className="tag ok-tag"> activo</span>}
                </td>
                <td className="mono small">{p.root}</td>
                <td className="small">{p.allow_write ? "sí" : "no"}</td>
                <td className="small">{p.allow_shell ? "sí" : "no"}</td>
                <td>
                  {activo?.id === p.id ? (
                    <button className="btn ghost" onClick={() => activar(null)}>
                      Desactivar
                    </button>
                  ) : (
                    <button className="btn ghost" onClick={() => activar(p.id)}>
                      Activar
                    </button>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>

        <div className="form-grid" style={{ marginTop: 12 }}>
          <label>
            Nombre del proyecto
            <input
              value={nuevoNombre}
              onChange={(e) => setNuevoNombre(e.target.value)}
              placeholder="KUKIA CODE"
            />
          </label>
          <label>
            Carpeta raíz (debe existir)
            <input
              className="mono"
              value={nuevaRaiz}
              onChange={(e) => setNuevaRaiz(e.target.value)}
              placeholder="H:\\KUKIA_CODE"
            />
          </label>
          <label className="inline span-2">
            <input
              type="checkbox"
              checked={shellProyecto}
              onChange={(e) => setShellProyecto(e.target.checked)}
            />
            Permitir ejecutar comandos dentro del proyecto (se puede cambiar después)
          </label>
        </div>
        <div className="composer">
          <button
            className="btn primary"
            onClick={crearProyecto}
            disabled={!nuevoNombre.trim() || !nuevaRaiz.trim()}
          >
            Dar de alta
          </button>
        </div>
      </div>

      <div className="card">
        <div className="page-head">
          <h2>Auditoría</h2>
          <button className="btn" onClick={cargarAuditoria}>
            Recargar
          </button>
        </div>
        <label className="inline">
          Exportar CSV a
          <input className="path-input mono" value={rutaCsv} onChange={(e) => setRutaCsv(e.target.value)} />
        </label>
        <button className="btn" onClick={exportar}>
          Exportar
        </button>
        <table className="table">
          <thead>
            <tr>
              <th>Fecha</th>
              <th>Agente</th>
              <th>Herramienta</th>
              <th>Acción</th>
              <th>Objetivo</th>
              <th>Resultado</th>
            </tr>
          </thead>
          <tbody>
            {auditoria.map((a) => (
              <tr key={a.id}>
                <td className="mono">{a.created_at.slice(0, 19).replace("T", " ")}</td>
                <td className="mono">{a.agent}</td>
                <td className="mono">{a.tool}</td>
                <td>{a.action}</td>
                <td className="mono" title={a.targets}>
                  {a.targets.length > 60 ? `${a.targets.slice(0, 60)}…` : a.targets}
                </td>
                <td>
                  <span className={`tag ${a.result.startsWith("ok") ? "ok-tag" : ""}`}>
                    {a.error ? `${a.result}: ${a.error}` : a.result}
                  </span>
                </td>
              </tr>
            ))}
            {auditoria.length === 0 && (
              <tr>
                <td colSpan={6} className="muted small">
                  Nada auditado todavía. Cada lectura, escritura, comando y cambio de permisos
                  aparecerá aquí.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}
