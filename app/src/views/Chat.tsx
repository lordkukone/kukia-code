import { useEffect, useRef, useState } from "react";
import { api, chatStream, type Agent, type MessageRow, type Session } from "../lib/api";

interface Props {
  provider: string;
  model: string;
  notify: (msg: string) => void;
}

export default function Chat({ provider, model, notify }: Props) {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [agents, setAgents] = useState<Agent[]>([]);
  const [sessionId, setSessionId] = useState<string>("");
  const [messages, setMessages] = useState<MessageRow[]>([]);
  const [input, setInput] = useState("");
  const [busy, setBusy] = useState(false);
  const [streamText, setStreamText] = useState("");
  const [agentId, setAgentId] = useState("kukia-master");
  const bottomRef = useRef<HTMLDivElement>(null);

  const loadSessions = async () => {
    const list = await api.sessions();
    setSessions(list);
    return list;
  };

  useEffect(() => {
    (async () => {
      try {
        const [list, ag] = await Promise.all([loadSessions(), api.agents()]);
        setAgents(ag);
        if (list.length > 0) setSessionId(list[0].id);
        else {
          const s = await api.createSession("Nueva conversación", "kukia-master");
          setSessions([s]);
          setSessionId(s.id);
        }
      } catch (e) {
        notify(`Error cargando sesiones: ${e}`);
      }
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (!sessionId) return;
    api.messages(sessionId).then(setMessages).catch((e) => notify(`Error cargando mensajes: ${e}`));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sessionId]);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, streamText]);

  const newSession = async () => {
    const s = await api.createSession("Nueva conversación", agentId);
    setSessions([s, ...sessions]);
    setSessionId(s.id);
    setMessages([]);
  };

  const send = async () => {
    const text = input.trim();
    if (!text || busy || !model) return;
    setInput("");
    setBusy(true);
    setStreamText("");

    try {
      const agent = agents.find((a) => a.id === agentId);
      const history = messages
        .filter((m) => m.role === "user" || m.role === "assistant")
        .slice(-20)
        .map((m) => ({ role: m.role as "user" | "assistant", content: m.content }));

      const userMsg = await api.addMessage(sessionId, "user", text);
      setMessages((prev) => [...prev, userMsg]);

      let acc = "";
      await chatStream(
        {
          provider,
          model,
          messages: [...history, { role: "user", content: text }],
          system_prompt: agent?.system_prompt,
        },
        (tok) => {
          acc += tok;
          setStreamText(acc);
        },
        async (r) => {
          const saved = await api.addMessage(sessionId, "assistant", acc);
          setMessages((prev) => [...prev, saved]);
          setStreamText("");
          setBusy(false);
          if (sessions.length > 0) {
            const first = sessions.find((s) => s.id === sessionId);
            if (first && first.title === "Nueva conversación") {
              const title = text.slice(0, 48);
              await api.renameSession(sessionId, title);
              await loadSessions();
            }
          }
          notify(
            `✓ ${r.completion_tokens} tok · ${r.total_ms} ms · ${r.completion_tokens > 0 && r.total_ms > 0 ? (r.completion_tokens / (r.total_ms / 1000)).toFixed(1) : "?"} tok/s`,
          );
        },
        (err) => {
          setBusy(false);
          setStreamText("");
          notify(`Error del modelo: ${err}`);
        },
      );
    } catch (e) {
      setBusy(false);
      notify(`Fallo al enviar: ${e}`);
    }
  };

  return (
    <div className="chat-layout">
      <aside className="chat-sessions">
        <button className="btn primary block" onClick={newSession}>
          + Nueva conversación
        </button>
        <div className="session-list">
          {sessions.map((s) => (
            <button
              key={s.id}
              className={`session-item ${s.id === sessionId ? "active" : ""}`}
              onClick={() => setSessionId(s.id)}
              title={s.title}
            >
              <span className="session-title">{s.title}</span>
              <span className="session-date">{s.updated_at.slice(0, 16).replace("T", " ")}</span>
            </button>
          ))}
        </div>
      </aside>

      <div className="chat-main">
        <div className="chat-toolbar">
          <label className="inline">
            Agente
            <select value={agentId} onChange={(e) => setAgentId(e.target.value)}>
              {agents.map((a) => (
                <option key={a.id} value={a.id}>
                  {a.name}
                </option>
              ))}
            </select>
          </label>
          <span className="chip subtle">
            {provider} · {model || "sin modelo"}
          </span>
        </div>

        <div className="messages">
          {messages.length === 0 && !streamText && (
            <div className="empty">
              <h2>KUKIA CODE</h2>
              <p>Selecciona un modelo y escribe. Todo se ejecuta en local.</p>
            </div>
          )}
          {messages.map((m) => (
            <div key={m.id} className={`msg ${m.role}`}>
              <div className="msg-role">{m.role === "user" ? "Tú" : "KUKIA"}</div>
              <div className="msg-body">{m.content}</div>
            </div>
          ))}
          {streamText && (
            <div className="msg assistant">
              <div className="msg-role">KUKIA</div>
              <div className="msg-body">{streamText}<span className="caret">▌</span></div>
            </div>
          )}
          <div ref={bottomRef} />
        </div>

        <div className="composer">
          <textarea
            value={input}
            placeholder={model ? "Escribe tu mensaje…" : "No hay modelo seleccionado"}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                send();
              }
            }}
            rows={3}
          />
          <button className="btn primary" disabled={busy || !input.trim() || !model} onClick={send}>
            {busy ? "Generando…" : "Enviar"}
          </button>
        </div>
      </div>
    </div>
  );
}
