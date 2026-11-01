import { useState } from "react";
import { api } from "../lib/api";

export default function MemoryView({ notify }: { notify: (m: string) => void }) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<string[]>([]);
  const [content, setContent] = useState("");
  const [tags, setTags] = useState("");

  const search = async () => {
    try {
      const r = await api.memorySearch(query);
      setResults(r);
      if (r.length === 0) notify("Sin resultados en memoria");
    } catch (e) {
      notify(`Error buscando: ${e}`);
    }
  };

  const add = async () => {
    if (!content.trim()) return;
    try {
      await api.memoryAdd(content, tags);
      setContent("");
      setTags("");
      notify("Memoria guardada");
      if (query) await search();
    } catch (e) {
      notify(`Error guardando: ${e}`);
    }
  };

  return (
    <div className="page">
      <div className="page-head">
        <h1>Memoria</h1>
        <span className="chip subtle">SQLite local</span>
      </div>

      <div className="card">
        <div className="row">
          <input
            className="search"
            placeholder="Buscar en memoria…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && search()}
          />
          <button className="btn" onClick={search}>
            Buscar
          </button>
        </div>
        <div className="memory-results">
          {results.map((r, i) => (
            <div key={i} className="memory-item">
              {r}
            </div>
          ))}
          {results.length === 0 && <div className="empty small">Sin búsqueda todavía.</div>}
        </div>
      </div>

      <div className="card">
        <h2>Añadir recuerdo</h2>
        <textarea
          rows={4}
          value={content}
          onChange={(e) => setContent(e.target.value)}
          placeholder="Hecho, decisión o contexto que debe persistir…"
        />
        <div className="row">
          <input
            className="search"
            value={tags}
            onChange={(e) => setTags(e.target.value)}
            placeholder="etiquetas (coma)"
          />
          <button className="btn primary" onClick={add}>
            Guardar
          </button>
        </div>
      </div>
    </div>
  );
}
