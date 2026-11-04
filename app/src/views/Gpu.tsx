import { useEffect, useRef, useState } from "react";
import {
  api,
  type GgufDisponible,
  type GpuStat,
  type LocalGpuCfg,
  type LocalGpuEstado,
  type RpcSonda,
} from "../lib/api";

const PERFILES = [
  { id: "solo_i9", nombre: "2 GPUs · solo el i9" },
  { id: "tres_gpus", nombre: "3 GPUs · i9 + i7 por RPC" },
];

export default function Gpu({ notify }: { notify: (m: string) => void }) {
  const [stats, setStats] = useState<GpuStat[]>([]);
  const [history, setHistory] = useState<Record<number, number[]>>({});
  const timer = useRef<number | null>(null);

  const [lg, setLg] = useState<LocalGpuCfg | null>(null);
  const [est, setEst] = useState<LocalGpuEstado | null>(null);
  const [ggufs, setGgufs] = useState<GgufDisponible[]>([]);
  const [sonda, setSonda] = useState<RpcSonda | null>(null);
  const [rpcVivo, setRpcVivo] = useState<boolean | null>(null);
  const [faena, setFaena] = useState("");

  useEffect(() => {
    const poll = async () => {
      try {
        const s = await api.gpuStats();
        setStats(s);
        setHistory((h) => {
          const next = { ...h };
          for (const g of s) {
            const arr = next[g.index] ? [...next[g.index], g.utilization] : [g.utilization];
            next[g.index] = arr.slice(-60);
          }
          return next;
        });
      } catch (e) {
        notify(`GPU no disponible: ${e}`);
      }
      try {
        setEst(await api.localgpuStatus());
      } catch {
        /* se reintenta en el siguiente ciclo */
      }
    };
    poll();
    timer.current = window.setInterval(poll, 2000);
    return () => {
      if (timer.current) window.clearInterval(timer.current);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    api.localgpuConfig().then(setLg).catch((e) => notify(`Motor local: ${e}`));
    api.localgpuGgufs().then(setGgufs).catch(() => setGgufs([]));
    // Vigila el i7 para poder sumar la tercera GPU sin reiniciar KUKIA.
    const mirarI7 = () =>
      api
        .localgpuProbeRpc()
        .then((r) => setRpcVivo(r.ok))
        .catch(() => setRpcVivo(false));
    mirarI7();
    const vigilante = window.setInterval(mirarI7, 10000);
    return () => window.clearInterval(vigilante);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const accion = async (
    nombre: string,
    fn: () => Promise<LocalGpuEstado>,
    exito?: string
  ) => {
    setFaena(nombre);
    try {
      setEst(await fn());
      if (exito) notify(exito);
    } catch (e) {
      notify(`${nombre}: ${e}`);
    } finally {
      setFaena("");
    }
  };

  const guardar = async () => {
    if (!lg) return;
    try {
      setLg(await api.localgpuSet(lg));
      notify("Motor local guardado");
    } catch (e) {
      notify(`Guardar: ${e}`);
    }
  };

  const spark = (values: number[]) => {
    const w = 220;
    const h = 40;
    const pts = values
      .map((v, i) => `${(i / Math.max(values.length - 1, 1)) * w},${h - (v / 100) * h}`)
      .join(" ");
    return (
      <svg width={w} height={h} className="spark">
        <polyline fill="none" stroke="#4ade80" strokeWidth="1.5" points={pts} />
      </svg>
    );
  };

  const corriendo = est?.corriendo ?? false;

  return (
    <div className="page">
      <div className="page-head">
        <h1>GPU Performance Center</h1>
        <span className="chip subtle">refresco 2 s</span>
      </div>

      {stats.length === 0 && (
        <div className="card empty small">
          No se han detectado GPUs (nvidia-smi sin respuesta).
        </div>
      )}

      <div className="gpu-grid">
        {stats.map((g) => (
          <div key={g.index} className="card gpu-card">
            <div className="gpu-head">
              <h3>
                GPU {g.index} · {g.name}
              </h3>
              <span className={`chip ${g.utilization > 5 ? "ok" : "subtle"}`}>
                {g.utilization.toFixed(0)}%
              </span>
            </div>
            <div className="meter">
              <div className="meter-label">
                <span>VRAM</span>
                <span>
                  {g.memory_used_mb} / {g.memory_total_mb} MB
                </span>
              </div>
              <div className="bar">
                <div
                  className="bar-fill"
                  style={{ width: `${(g.memory_used_mb / g.memory_total_mb) * 100}%` }}
                />
              </div>
            </div>
            <div className="meter">
              <div className="meter-label">
                <span>Utilización</span>
                <span>{g.utilization.toFixed(0)}%</span>
              </div>
              <div className="bar">
                <div className="bar-fill green" style={{ width: `${g.utilization}%` }} />
              </div>
            </div>
            <div className="gpu-metrics">
              <div>
                <span className="muted">Temp</span>
                <strong>{g.temperature_c.toFixed(0)} °C</strong>
              </div>
              <div>
                <span className="muted">Consumo</span>
                <strong>{g.power_w.toFixed(0)} W</strong>
              </div>
              <div>
                <span className="muted">Core</span>
                <strong>{g.clock_graphics_mhz} MHz</strong>
              </div>
              <div>
                <span className="muted">Mem</span>
                <strong>{g.clock_memory_mhz} MHz</strong>
              </div>
            </div>
            <div className="spark-wrap">{spark(history[g.index] ?? [])}</div>
          </div>
        ))}
      </div>
      <div className="card">
        <div className="gpu-head">
          <h2>Motor local · llama.cpp</h2>
          <span className={`chip ${corriendo ? "ok" : "subtle"}`}>
            {corriendo ? `corriendo · pid ${est?.pid}` : "parado"}
          </span>
          <span className="chip subtle">{est?.perfil ?? "—"}</span>
          className={`chip ${rpcVivo ? "ok" : "subtle"}`}>
            {rpcVivo === null ? "i7: ?" : rpcVivo ? "i7: listo" : "i7: sin respuesta"}
          </span>
        </div>
        <p className="muted">
          KUKIA arranca y para su propio llama-server. Con el perfil de 3 GPUs también lanza el RPC
          del i7; si no responde a tiempo, arranca en 2 GPUs y lo dice.
        </p>
        {lg && (
          <div className="gpu-metrics" style={{ flexWrap: "wrap", gap: 12 }}>
            <label>
              <span className="muted">Perfil</span>
              <select value={lg.perfil} onChange={(e) => setLg({ ...lg, perfil: e.target.value })}>
                {PERFILES.map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.nombre}
                  </option>
                ))}
              </select>
            </label>
            <label>
              <span className="muted">Modelo</span>
              <select value={lg.modelo} onChange={(e) => setLg({ ...lg, modelo: e.target.value })}>
                {!ggufs.some((g) => g.path === lg.modelo) && (
                  <option value={lg.modelo}>{lg.modelo || "(sin modelo)"}</option>
                )}
                {ggufs.map((g) => (
                  <option key={g.path} value={g.path}>
                    {g.nombre} · {g.size_gb.toFixed(1)} GB
                  </option>
                ))}
              </select>
            </label>
            <label>
              <span className="muted">Contexto</span>
              <input
                type="number"
                min={512}
                step={512}
                value={lg.ctx}
                onChange={(e) => setLg({ ...lg, ctx: Number(e.target.value) })}
                style={{ width: 90 }}
              />
            </label>
          </div>
        )}
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginTop: 12 }}>
          <button
            className="btn"
            disabled={!!faena || corriendo}
            onClick={() => accion("Arrancar", api.localgpuStart, "Motor local arrancado")}
          >
            {faena === "Arrancar" ? "Arrancando…" : "Arrancar"}
          </button>
          <button
            className="btn"
            disabled={!!faena || !corriendo}
            onClick={() => accion("Parar", api.localgpuStop, "Motor local parado")}
          >
            Parar
          </button>
          <button
            className="btn"
            disabled={!!faena}
            onClick={async () => {
              try {
                setSonda(await api.localgpuProbeRpc());
              } catch (e) {
                notify(`Sonda: ${e}`);
              }
            }}
          >
            Probar i7
          </button>
          <button className="btn" disabled={!!faena || !lg || corriendo} onClick={guardar}>
            Guardar
          </button>
          <button
            className="btn"
            disabled={!!faena || !corriendo || !rpcVivo || est?.perfil === "TresGpus"}
            title="El i7 responde: reinicia el motor repartiendo entre las tres GPUs"
            onClick={async () => {
              await accion("Parar", api.localgpuStop);
              await accion("Arrancar", api.localgpuStart, "Motor local en 3 GPUs");
            }}
          >
            Pasar a 3 GPUs
          </button>
        </div>
        {sonda && (
          <p className="muted">
            i7 {sonda.destino}: {sonda.ok ? "responde" : "no responde"} en {sonda.ms} ms —{" "}
            {sonda.detalle}
          </p>
        )}
        {est?.aviso && <p className="muted">Aviso: {est.aviso}</p>}
        {est?.error && <p className="muted">Error: {est.error}</p>}
        {est && est.lineas.length > 0 && (
          <pre
            className="card"
            style={{ maxHeight: 220, overflow: "auto", fontSize: 11, whiteSpace: "pre-wrap" }}
          >
            {est.lineas.slice(-40).join("\n")}
          </pre>
        )}
      </div>
    </div>
  );
}
