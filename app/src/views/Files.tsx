import { useEffect, useState } from "react";
import { api, type FileEntry } from "../lib/api";

export default function Files({ notify }: { notify: (m: string) => void }) {
  const [path, setPath] = useState("H:\\KUKIA_CODE");
  const [entries, setEntries] = useState<FileEntry[]>([]);
  const [preview, setPreview] = useState<{ path: string; content: string } | null>(null);
  const [draft, setDraft] = useState("");
  const [approve, setApprove] = useState(false);

  const open = async (p: string) => {
    try {
      const list = await api.files(p);
      setEntries(list);
      setPath(p);
    } catch (e) {
      notify(`No se pudo abrir ${p}: ${e}`);
    }
  };

  useEffect(() => {
    open(path);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const readFile = async (p: string) => {
    try {
      const content = await api.readFile(p);
      setPreview({ path: p, content });
      setDraft(content);
    } catch (e) {
      notify(`No se pudo leer: ${e}`);
    }
  };

  const save = async () => {
    if (!preview) return;
    if (!approve) {
      notify("Marca la casilla de aprobación para modificar archivos");
      return;
    }
    try {
      await api.writeFile(preview.path, draft, true);
      notify("Archivo guardado (aprobado y registrado)");
      setApprove(false);
    } catch (e) {
      notify(`Error guardando: ${e}`);
    }
  };

  const parent = path.replace(/\\[^\\]+$/, "") || path;

  return (
    <div className="page">
      <div className="page-head">
        <h1>Archivos</h1>
        <div className="row">
          <input className="path-input mono" value={path} onChange={(e) => setPath(e.target.value)} />
          <button className="btn" onClick={() => open(path)}>
            Abrir
          </button>
        </div>
      </div>

      <div className="split">
        <div className="card file-list">
          <div className="card-head">
            <button className="btn small" onClick={() => open(parent)}>
              ↑ Subir
            </button>
            <span className="muted mono">{path}</span>
          </div>
          {entries.map((e) => (
            <button
              key={e.path}
              className="file-row"
              onClick={() => (e.is_dir ? open(e.path) : readFile(e.path))}
            >
              <span>{e.is_dir ? "📁" : "📄"}</span>
              <span className="file-name">{e.name}</span>
              <span className="muted">{e.is_dir ? "" : `${(e.size / 1024).toFixed(1)} KB`}</span>
            </button>
          ))}
          {entries.length === 0 && <div className="empty small">Carpeta vacía o inaccesible.</div>}
        </div>

        <div className="card editor">
          <div className="card-head">
            <h2 className="mono">{preview?.path ?? "Selecciona un archivo"}</h2>
            {preview && (
              <label className="inline">
                <input type="checkbox" checked={approve} onChange={(e) => setApprove(e.target.checked)} />
                Aprobar modificación
              </label>
            )}
          </div>
          <textarea
            className="code"
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            disabled={!preview}
            spellCheck={false}
          />
          {preview && (
            <button className="btn primary" onClick={save}>
              Guardar
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
