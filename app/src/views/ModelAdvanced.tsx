import { useEffect, useState } from "react";
import {
  api,
  type Capacidades,
  type InferenceParams,
  type Metricas,
  type ParamSpec,
  type Perfil,
} from "../lib/api";

interface Props {
  engine: string;
  model: string;
  params: InferenceParams;
  onParams: (p: InferenceParams) => void;
  notify: (m: string) => void;
}

/** Presets de contexto pedidos, sin inventar: se recortan al máximo del modelo. */
const CONTEXTOS = [4096, 8192, 16384, 32768, 49152, 65536, 98304, 131072];

/** Color del estado de un parámetro: lo que se puede tocar y lo que no. */
const colorEstado = (aplica: string): string => {
  switch (aplica) {
    case "caliente":
      return "#15803d";
    case "recarga":
      return "#b45309";
    case "sesion":
      return "#7c3aed";
    case "sin_verificar":
      return "#6b7280";
    default:
      return "#b91c1c";
  }
};

const etiquetaEstado = (aplica: string): string => {
  switch (aplica) {
    case "caliente":
      return "cambio en caliente";
    case "recarga":
      return "recarga el modelo";
    case "sesion":
      return "nueva sesión";
    case "sin_verificar":
      return "sin verificar";
    default:
      return "no compatible";
  }
};

export default function ModelAdvanced(p: Props) {
  const [cap, setCap] = useState<Capacidades | null>(null);
  const [metricas, setMetricas] = useState<Metricas | null>(null);
  const [perfiles, setPerfiles] = useState<Perfil[]>([]);
  const [elegido, setElegido] = useState("");
  const [nombrePerfil, setNombrePerfil] = useState("");
  const [ocupado, setOcupado] = useState<string | null>(null);
  const [ctx, setCtx] = useState<number>(0);

  const analizar = async () => {
    setOcupado("cap");
    try {
      const [c, m] = await Promise.all([
        api.controlCapacidades(p.engine, p.model),
        api.controlMetricas(),
      ]);
      setCap(c);
      setMetricas(m);
      setCtx(c.contexto.actual || p.params.num_ctx || 8192);
    } catch (e) {
      p.notify(`No se pudo analizar el modelo: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  useEffect(() => {
    setCap(null);
    setMetricas(null);
    void analizar();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [p.engine, p.model]);

  useEffect(() => {
    void api
      .perfilesListar()
      .then(setPerfiles)
      .catch((e) => p.notify(`No se pudieron leer los perfiles: ${e}`));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const guardar = async (cambios: Partial<InferenceParams>, clave: string) => {
    setOcupado(clave);
    try {
      const nuevo = { ...p.params, ...cambios };
      const avisos = await api.engineParamsSet(p.engine, p.model, nuevo);
      p.onParams(nuevo);
      p.notify(
        avisos.length
          ? `Guardado con ${avisos.length} aviso(s) de incompatibilidad`
          : "Parámetros guardados",
      );
      await analizar();
    } catch (e) {
      p.notify(`Error guardando: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  const aplicarPerfil = async () => {
    if (!elegido) return;
    setOcupado("perfil");
    try {
      const [params, avisos] = await api.perfilAplicar(p.engine, p.model, elegido);
      p.onParams(params);
      p.notify(
        `Perfil ${elegido} aplicado${avisos.length ? ` con ${avisos.length} aviso(s)` : ""}`,
      );
      await analizar();
    } catch (e) {
      p.notify(`No se pudo aplicar el perfil: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  const restaurar = async () => {
    setOcupado("restaurar");
    try {
      const anterior = await api.perfilRestaurarAnterior(p.engine, p.model);
      if (anterior) {
        p.onParams(anterior);
        p.notify("Restaurados los parámetros anteriores");
        await analizar();
      } else {
        p.notify("No hay parámetros anteriores guardados para este modelo");
      }
    } catch (e) {
      p.notify(`No se pudo restaurar: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  const guardarPerfil = async () => {
    const nombre = nombrePerfil.trim();
    if (!nombre) {
      p.notify("Escribe un nombre para el perfil");
      return;
    }
    setOcupado("guardarPerfil");
    try {
      const lista = await api.perfilGuardar({
        nombre,
        descripcion: `Guardado desde ${p.engine}/${p.model}`,
        params: p.params,
        creado: new Date().toISOString(),
      });
      setPerfiles(lista);
      setNombrePerfil("");
      p.notify(`Perfil ${nombre} guardado`);
    } catch (e) {
      p.notify(`No se pudo guardar el perfil: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  const borrarPerfil = async () => {
    if (!elegido) return;
    setOcupado("borrarPerfil");
    try {
      setPerfiles(await api.perfilBorrar(elegido));
      p.notify(`Perfil ${elegido} borrado`);
      setElegido("");
    } catch (e) {
      p.notify(`No se pudo borrar: ${e}`);
    } finally {
      setOcupado(null);
    }
  };

  const campo = (s: ParamSpec) => {
    const valor = p.params[s.key as keyof InferenceParams];
    const deshabilitado = s.aplica === "no_compatible" || s.aplica === "sin_verificar";
    if (s.tipo === "bool") {
      return (
        <label key={s.key} title={s.motivo}>
          <span>
            {s.etiqueta}{" "}
            <em className="small" style={{ color: colorEstado(s.aplica) }}>
              ({etiquetaEstado(s.aplica)})
            </em>
          </span>
          <select
            value={valor === true ? "on" : valor === false ? "off" : "auto"}
            disabled={deshabilitado || ocupado === s.key}
            onChange={(e) =>
              void guardar(
                {
                  [s.key]:
                    e.target.value === "auto" ? null : e.target.value === "on",
                } as Partial<InferenceParams>,
                s.key,
              )
            }
          >
            <option value="auto">por defecto del motor</option>
            <option value="on">activado</option>
            <option value="off">desactivado</option>
          </select>
        </label>
      );
    }
    if (s.opciones.length > 0) {
      return (
        <label key={s.key} title={s.motivo}>
          <span>
            {s.etiqueta}{" "}
            <em className="small" style={{ color: colorEstado(s.aplica) }}>
              ({etiquetaEstado(s.aplica)})
            </em>
          </span>
          <select
            value={typeof valor === "string" ? valor : "auto"}
            disabled={deshabilitado}
            onChange={(e) =>
              void guardar(
                {
                  [s.key]: e.target.value === "auto" ? null : e.target.value,
                } as Partial<InferenceParams>,
                s.key,
              )
            }
          >
            {s.opciones.map((o) => (
              <option key={o} value={o}>
                {o}
              </option>
            ))}
          </select>
        </label>
      );
    }
    return (
      <label key={s.key} title={s.motivo}>
        <span>
          {s.etiqueta}{" "}
          <em className="small" style={{ color: colorEstado(s.aplica) }}>
            ({etiquetaEstado(s.aplica)})
          </em>
        </span>
        <input
          type="number"
          step="any"
          min={s.min ?? undefined}
          max={s.max ?? undefined}
          disabled={deshabilitado}
          value={typeof valor === "number" ? valor : ""}
          placeholder={deshabilitado ? "no disponible" : "motor"}
          onChange={(e) => {
            const t = e.target.value.trim();
            const n = t === "" ? null : Number(t);
            void guardar(
              { [s.key]: n !== null && Number.isFinite(n) ? n : null } as Partial<InferenceParams>,
              s.key,
            );
          }}
        />
      </label>
    );
  };

  const grupo = (nombre: string, titulo: string) => {
    const lista = (cap?.parametros ?? []).filter((x) => x.grupo === nombre);
    if (lista.length === 0) return null;
    return (
      <div style={{ marginTop: 14 }}>
        <h3 className="small" style={{ marginBottom: 6 }}>
          {titulo}
        </h3>
        <div className="form-grid">{lista.map(campo)}</div>
      </div>
    );
  };

  const maxCtx = cap?.contexto.maximo_modelo || cap?.contexto.maximo_servido || 262144;

  return (
    <div className="card" style={{ marginTop: 14 }}>
      <div className="row-between">
        <h2 style={{ margin: 0 }}>Control avanzado</h2>
        <button className="btn ghost small" onClick={analizar} disabled={ocupado === "cap"}>
          {ocupado === "cap" ? "Analizando…" : "Volver a analizar"}
        </button>
      </div>

      {cap && (
        <>
          <p className="muted small">
            arquitectura {cap.arch.architecture || "—"} · {cap.arch.layers} capas ·{" "}
            {cap.arch.heads_kv} cabezas KV · dim{" "}
            {cap.arch.embedding > 0 && cap.arch.heads > 0
              ? Math.round(cap.arch.embedding / cap.arch.heads)
              : "—"}{" "}
            · contexto nativo {cap.arch.ctx_native.toLocaleString()}
          </p>

          {/* 1. Contexto */}
          <h3 className="small" style={{ marginTop: 12 }}>
            Contexto
          </h3>
          <div className="row" style={{ gap: 8, flexWrap: "wrap" }}>
            {CONTEXTOS.filter((c) => c <= maxCtx).map((c) => (
              <button
                key={c}
                className={`btn ghost small ${ctx === c ? "active" : ""}`}
                onClick={() => setCtx(c)}
              >
                {c >= 1024 ? `${Math.round(c / 1024)}K` : c}
              </button>
            ))}
          </div>
          <div className="row" style={{ gap: 10, marginTop: 8, alignItems: "center" }}>
            <input
              type="range"
              min={2048}
              max={maxCtx}
              step={1024}
              value={ctx}
              onChange={(e) => setCtx(Number(e.target.value))}
              style={{ flex: 1 }}
            />
            <input
              type="number"
              value={ctx}
              min={2048}
              max={maxCtx}
              step={1024}
              onChange={(e) => setCtx(Number(e.target.value))}
              style={{ width: 110 }}
            />
            <button
              className="btn small"
              disabled={ocupado === "ctx" || ctx === p.params.num_ctx}
              onClick={() => void guardar({ num_ctx: ctx }, "ctx")}
            >
              {ocupado === "ctx" ? "…" : "Aplicar"}
            </button>
          </div>
          <p className="muted small">
            actual {cap.contexto.actual.toLocaleString()} · máximo del modelo{" "}
            {cap.contexto.maximo_modelo.toLocaleString()} · fuente: {cap.contexto.fuente}
            <br />
            cada 8K extra de contexto añaden ≈ {cap.contexto.gb_por_8k.toFixed(2)} GB de caché KV
            (fp16). Cambiar el contexto <strong>recarga el modelo</strong>.
          </p>

          {/* 2. KV cache */}
          <h3 className="small" style={{ marginTop: 12 }}>
            Caché KV
          </h3>
          <table className="table">
            <thead>
              <tr>
                <th>Formato</th>
                <th>Coste al contexto actual</th>
                <th>Ahorro</th>
                <th>Estado</th>
              </tr>
            </thead>
            <tbody>
              {cap.kv.map((k) => (
                <tr key={k.formato}>
                  <td className="mono small">{k.formato}</td>
                  <td className="small">{k.gb.toFixed(2)} GB</td>
                  <td className="small">
                    {k.ahorro_gb > 0
                      ? `${k.ahorro_gb.toFixed(2)} GB (${k.ahorro_pct.toFixed(0)} %)`
                      : "—"}
                  </td>
                  <td className="small">
                    <span
                      className={`tag ${
                        k.estado === "por_defecto" || k.estado === "activo_entorno" ? "ok-tag" : ""
                      }`}
                    >
                      {k.estado.replace("_", " ")}
                    </span>
                    <div className="muted small">{k.motivo}</div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>

          {/* 3. GPU */}
          <h3 className="small" style={{ marginTop: 12 }}>
            GPU y reparto de capas
          </h3>
          <div className="split">
            {(metricas?.gpus ?? cap.gpus).map((g) => (
              <div key={g.index} className="card" style={{ margin: 0 }}>
                <div className="row-between">
                  <strong className="small">
                    GPU{g.index} · {g.name.replace("NVIDIA GeForce ", "")}
                  </strong>
                  <span className="muted small">
                    {g.used_mb} / {g.total_mb} MB
                  </span>
                </div>
                <div className="muted small" style={{ marginTop: 4 }}>
                  libres {(g.free_mb / 1024).toFixed(1)} GB · uso {g.utilization} % ·{" "}
                  {g.temperatura_c.toFixed(0)} °C
                  {g.potencia_w > 0 ? ` · ${g.potencia_w.toFixed(0)} W` : ""}
                  <br />
                  capas asignadas por el motor: no se pueden fijar desde aquí (ver abajo)
                </div>
              </div>
            ))}
          </div>
          {metricas && (
            <p className="muted small">
              RAM {metricas.ram_usada_gb.toFixed(1)} / {metricas.ram_total_gb.toFixed(1)} GB · CPU{" "}
              {metricas.cpu_uso_pct.toFixed(0)} % · cargados:{" "}
              {metricas.cargados.join(", ") || "ninguno"}
            </p>
          )}
          {grupo("gpu", "Capas en GPU")}

          {/* 4. Motor */}
          {grupo("motor", "Motor")}
          {/* 5. Generación */}
          {grupo("generacion", "Generación")}

          {/* Avisos */}
          {cap.avisos.length > 0 && (
            <div style={{ marginTop: 12 }}>
              {cap.avisos.map((a) => (
                <p key={a} className="muted small">
                  · {a}
                </p>
              ))}
            </div>
          )}

          {/* 6. Perfiles */}
          <h3 className="small" style={{ marginTop: 16 }}>
            Perfiles
          </h3>
          <div className="row" style={{ gap: 8, flexWrap: "wrap", alignItems: "flex-end" }}>
            <label className="field">
              <span>Perfil guardado</span>
              <select value={elegido} onChange={(e) => setElegido(e.target.value)}>
                <option value="">— elegir —</option>
                {perfiles.map((x) => (
                  <option key={x.nombre} value={x.nombre}>
                    {x.nombre}
                  </option>
                ))}
              </select>
            </label>
            <button
              className="btn small"
              onClick={aplicarPerfil}
              disabled={!elegido || ocupado === "perfil"}
            >
              {ocupado === "perfil" ? "…" : "Aplicar"}
            </button>
            <button className="btn ghost small" onClick={restaurar} disabled={ocupado === "restaurar"}>
              {ocupado === "restaurar" ? "…" : "Restaurar anterior"}
            </button>
            <button
              className="btn ghost small"
              onClick={borrarPerfil}
              disabled={!elegido || ocupado === "borrarPerfil"}
            >
              Borrar
            </button>
          </div>
          <div className="row" style={{ gap: 8, marginTop: 8 }}>
            <input
              placeholder="Nombre del perfil nuevo"
              value={nombrePerfil}
              onChange={(e) => setNombrePerfil(e.target.value)}
              style={{ maxWidth: 260 }}
            />
            <button
              className="btn ghost small"
              onClick={guardarPerfil}
              disabled={ocupado === "guardarPerfil"}
            >
              Guardar como perfil
            </button>
          </div>
          {perfiles.length > 0 && (
            <p className="muted small" style={{ marginTop: 6 }}>
              {perfiles.length} perfil(es): {perfiles.map((x) => x.nombre).join(" · ")}
            </p>
          )}
        </>
      )}

      {!cap && (
        <p className="muted small">
          {ocupado === "cap" ? "Analizando modelo y GPUs…" : "Sin datos todavía."}
        </p>
      )}
    </div>
  );
}
