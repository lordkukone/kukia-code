import { useEffect, useRef, useState } from "react";
import { api, type GpuStat } from "../lib/api";

export default function Gpu({ notify }: { notify: (m: string) => void }) {
  const [stats, setStats] = useState<GpuStat[]>([]);
  const [history, setHistory] = useState<Record<number, number[]>>({});
  const timer = useRef<number | null>(null);

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
    };
    poll();
    timer.current = window.setInterval(poll, 2000);
    return () => {
      if (timer.current) window.clearInterval(timer.current);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const spark = (values: number[]) => {
    const w = 220;
    const h = 40;
    const max = 100;
    const pts = values
      .map((v, i) => `${(i / Math.max(values.length - 1, 1)) * w},${h - (v / max) * h}`)
      .join(" ");
    return (
      <svg width={w} height={h} className="spark">
        <polyline fill="none" stroke="#4ade80" strokeWidth="1.5" points={pts} />
      </svg>
    );
  };

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
        <h2>Reparto multi-GPU (perfil)</h2>
        <p className="muted">
          El reparto se guarda en Configuración. Los motores aplican sólo los métodos que soportan
          (Ollama usa offload por capas; LM Studio gestiona su propia VRAM).
        </p>
      </div>
    </div>
  );
}
