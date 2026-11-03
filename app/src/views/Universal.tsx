import { useEffect, useMemo, useState } from "react";
import { api, type InventarioUniversal, type SistemaUniversal } from "../lib/api";

type Filtro = "todos" | "instalados" | "conectados" | "ausentes";

function estadoDe(s: SistemaUniversal): { texto: string; color: string } {
  if (s.operativo) return { texto: "operativo", color: "ok" };
  if (s.conectado) return { texto: "conectado", color: "ok" };
  if (s.detectado) return { texto: "detectado (sin API)", color: "warn" };
  if (s.instalado) return { texto: "instalado (parado)", color: "warn" };
  return { texto: "no detectado", color: "subtle" };
}

function Ficha({ s, abierta, onToggle }: { s: SistemaUniversal; abierta: boolean; onToggle: () => void }) {
  const e = estadoDe(s);
  return (
    <div className="card" style={{ minWidth: 320, flex: "1 1 340px" }}>
      <div className="row-between" style={{ gap: 6, flexWrap: "wrap" }}>
        <strong>{s.nombre}</strong>
        <span className={`chip ${e.color}`}>{e.texto}</span>
      </div>
      <div className="muted small">
        {s.categoria}
        {s.version ? ` · v${s.version}` : ""}
        {s.puertos_abiertos.length > 0 ? ` · puerto ${s.puertos_abiertos.join(", ")}` : ""}
      </div>
      <div className="small" style={{ marginTop: 6 }}>
        {s.detalle}
      </div>
      {s.endpoint && <div className="mono small muted">{s.endpoint}</div>}
      {s.error && <div className="small" style={{ marginTop: 4, color: "var(--warn, #c60)" }}>⚠ {s.error}</div>}
      <button className="btn ghost small" style={{ marginTop: 8 }} onClick={onToggle}>
        {abierta ? "Ocultar detalle" : "Capacidades y límites"}
      </button>
      {abierta && (
        <>
          <div className="muted small" style={{ marginTop: 8 }}>
            KUKIA puede
          </div>
          <ul className="small" style={{ margin: "4px 0 0 16px" }}>
            {s.capacidades.map((c) => (
              <li key={c}>{c}</li>
            ))}
          </ul>
          {s.limitaciones.length > 0 && (
            <>
              <div className="muted small" style={{ marginTop: 6 }}>
                No hay / no se hace
              </div>
              <ul className="muted small" style={{ margin: "4px 0 0 16px" }}>
                {s.limitaciones.map((c) => (
                  <li key={c}>{c}</li>
                ))}
              </ul>
            </>
          )}
        </>
      )}
    </div>
  );
}

export default function Universal({ notify }: { notify: (m: string) => void }) {
  const [inv, setInv] = useState<InventarioUniversal | null>(null);
  const [ocupado, setOcupado] = useState(false);
  const [filtro, setFiltro] = useState<Filtro>("todos");
  const [q, setQ] = useState("");
  const [abierta, setAbierta] = useState<string | null>(null);

  const cargar = async (avisar = false) => {
    setOcupado(true);
    try {
      const r = await api.ecosistemaUniversal();
      setInv(r);
      if (avisar) {
        notify(`${r.instalados}/${r.total} instalados · ${r.conectados} con API viva`);
      }
    } catch (e) {
      notify(`No se pudo sondear el ecosistema: ${e}`);
    } finally {
      setOcupado(false);
    }
  };

  useEffect(() => {
    void cargar();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const lista = useMemo(() => {
    if (!inv) return [];
    const texto = q.trim().toLowerCase();
    return inv.sistemas.filter((s) => {
      if (filtro === "instalados" && !s.instalado) return false;
      if (filtro === "conectados" && !s.conectado) return false;
      if (filtro === "ausentes" && s.instalado) return false;
      if (!texto) return true;
      return (
        s.nombre.toLowerCase().includes(texto) ||
        s.categoria.toLowerCase().includes(texto) ||
        s.id.toLowerCase().includes(texto)
      );
    });
  }, [inv, filtro, q]);

  return (
    <div className="page">
      <div className="page-head">
        <h1>KUKIA UNIVERSAL ECOSYSTEM</h1>
        <span className="chip ok">
          {inv ? `${inv.instalados}/${inv.total} instalados · ${inv.conectados} conectados` : "sondeando…"}
        </span>
        <button className="btn ghost small" disabled={ocupado} onClick={() => void cargar(true)}>
          {ocupado ? "Sondeando…" : "Volver a sondear"}
        </button>
      </div>

      <p className="muted small">
        Detección por <strong>proceso</strong>, <strong>puerto</strong>, <strong>sonda HTTP</strong> y{" "}
        <strong>ruta</strong>, cada sistema con su propio timeout. «Instalado» no es «conectado»: solo
        se marca conectado cuando su API contesta de verdad.
      </p>

      <div className="row" style={{ gap: 8, marginBottom: 10, flexWrap: "wrap" }}>
        <input
          className="input"
          style={{ minWidth: 240, flex: 1 }}
          placeholder="Buscar por nombre o categoría…"
          value={q}
          onChange={(e) => setQ(e.target.value)}
        />
        {(["todos", "instalados", "conectados", "ausentes"] as Filtro[]).map((f) => (
          <button
            key={f}
            className={`btn small ${filtro === f ? "" : "ghost"}`}
            onClick={() => setFiltro(f)}
          >
            {f}
          </button>
        ))}
      </div>

      <div className="row" style={{ gap: 12, flexWrap: "wrap", alignItems: "stretch" }}>
        {lista.map((s) => (
          <Ficha
            key={s.id}
            s={s}
            abierta={abierta === s.id}
            onToggle={() => setAbierta(abierta === s.id ? null : s.id)}
          />
        ))}
      </div>

      {inv && inv.no_detectados.length > 0 && (
        <div className="card">
          <h2>Sin detectar en este equipo</h2>
          <p className="muted small">
            No están instalados o no se han encontrado: no hay integración que fingir.
          </p>
          <div className="row" style={{ gap: 6, flexWrap: "wrap" }}>
            {inv.no_detectados.map((n) => (
              <span key={n} className="chip subtle">
                {n}
              </span>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
