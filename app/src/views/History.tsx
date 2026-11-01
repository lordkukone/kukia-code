import { useEffect, useState } from "react";
import { api, type MessageRow, type Session } from "../lib/api";

export default function History({ notify }: { notify: (m: string) => void }) {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [active, setActive] = useState<string>("");
  const [messages, setMessages] = useState<MessageRow[]>([]);

  useEffect(() => {
    api
      .sessions()
      .then((s) => {
        setSessions(s);
        if (s.length > 0) setActive(s[0].id);
      })
      .catch((e) => notify(`Error: ${e}`));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (!active) return;
    api.messages(active).then(setMessages).catch((e) => notify(`Error: ${e}`));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active]);

  return (
    <div className="page">
      <div className="page-head">
        <h1>Historial</h1>
        <span className="chip subtle">{sessions.length} conversaciones</span>
      </div>
      <div className="split">
        <div className="card file-list">
          {sessions.map((s) => (
            <button
              key={s.id}
              className={`file-row ${s.id === active ? "selected" : ""}`}
              onClick={() => setActive(s.id)}
            >
              <span>💬</span>
              <span className="file-name">{s.title}</span>
              <span className="muted">{s.updated_at.slice(0, 16).replace("T", " ")}</span>
            </button>
          ))}
          {sessions.length === 0 && <div className="empty small">Sin conversaciones.</div>}
        </div>
        <div className="card">
          <h2>Mensajes</h2>
          <div className="history-messages">
            {messages.map((m) => (
              <div key={m.id} className={`msg ${m.role}`}>
                <div className="msg-role">{m.role}</div>
                <div className="msg-body">{m.content}</div>
              </div>
            ))}
            {messages.length === 0 && <div className="empty small">Selecciona una conversación.</div>}
          </div>
        </div>
      </div>
    </div>
  );
}
