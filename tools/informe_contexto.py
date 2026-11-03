"""Genera el informe de contexto por modelo a partir del inventario medido.

No toca nada: lee `contexto_inventario.json` y escribe un Markdown con la tabla
completa, el máximo viable en este equipo y la clasificación pedida.

Presupuesto usado (declarado, no mágico):
  - VRAM total de las dos tarjetas: 11,9 + 15,9 = 27,8 GB; se reserva 1,5 GB
    para pantalla, contexto del sistema y margen de la caché de cómputo.
  - KV con el formato activo del servidor (OLLAMA_KV_CACHE_TYPE = q4_0).
  - El máximo viable se redondea a múltiplos de 2.048 tokens y nunca supera el
    contexto nativo que declara la arquitectura del modelo.
"""

import json
from pathlib import Path

RAIZ = Path(__file__).resolve().parent.parent
INVENTARIO = RAIZ / "INFORMES" / "contexto_inventario.json"
SALIDA = RAIZ / "INFORMES" / "CONTEXTO_POR_MODELO.md"

VRAM_GB = 27.8
RESERVA_GB = 1.5
PRESUPUESTO = VRAM_GB - RESERVA_GB


def redondear(ctx):
    """Baja el contexto a múltiplo de 2.048 sin pasarse."""
    return (int(ctx) // 2048) * 2048


def main():
    datos = json.load(open(INVENTARIO, encoding="utf-8"))
    filas = datos["ollama"]

    informe = []
    for f in filas:
        nativo = f.get("ctx_nativo") or 0
        pesos = f.get("tamano_gb") or 0.0
        kv_nativo = f.get("kv_q4_nativo_gb")
        kv_token_gb = (kv_nativo / nativo) if (kv_nativo and nativo) else None
        libre = PRESUPUESTO - pesos
        if kv_token_gb and kv_token_gb > 0 and libre > 0.2:
            viable = redondear(min(nativo, libre / kv_token_gb))
        else:
            viable = 0
        viable = max(0, min(viable, nativo)) if nativo else viable
        configurado = f.get("num_ctx_modelo")
        f["viable"] = viable
        if viable == 0:
            # Ni los pesos caben en la VRAM presupuestada: solo correría con
            # offload a RAM, y ahí el contexto no es el límite.
            clase = "LIMITADO POR HARDWARE"
        elif viable < nativo:
            clase = "LIMITADO POR HARDWARE"
        elif configurado and configurado > viable:
            clase = "LIMITADO POR HARDWARE"
        elif configurado == viable:
            clase = "CONFIGURADO AL MÁXIMO"
        else:
            clase = "CONTEXTO AMPLIABLE"
        f["clase"] = clase
        f["kv_token_gb"] = kv_token_gb
        informe.append(f)

    orden = {
        "CONFIGURADO AL MÁXIMO": 0,
        "CONTEXTO AMPLIABLE": 1,
        "LIMITADO POR HARDWARE": 2,
        "NO VERIFICADO": 3,
    }
    informe.sort(key=lambda x: (orden[x["clase"]], -(x.get("viable") or 0)))

    lineas = [
        "# Contexto máximo real por modelo — inventario y plan",
        "",
        "Datos leídos de `/api/show` (arquitectura, `*.context_length`, `num_ctx` del",
        "Modelfile) y `/api/tags` (tamaño en disco) de la instalación de Ollama de esta",
        "máquina. `OLLAMA_KV_CACHE_TYPE` vale **q4_0** (variable de usuario), y es el",
        "formato con el que se calcula la caché KV.",
        "",
        f"Presupuesto de VRAM usado para el máximo viable: **{PRESUPUESTO:.1f} GB** "
        f"({VRAM_GB} GB de las dos tarjetas menos {RESERVA_GB} GB de margen).",
        "No se suma la VRAM como si fuera memoria única: el presupuesto es el total.",
        "",
        "## Resumen",
        "",
    ]
    for clase in orden:
        n = sum(1 for f in informe if f["clase"] == clase)
        lineas.append(f"- **{clase}**: {n} modelo(s)")
    lineas += [
        "",
        "## Tabla completa",
        "",
        "| Modelo | Proveedor | Nativo | Viables en este equipo | Configurado | Kv q4 al nativo | Clase |",
        "|---|---|---:|---:|---:|---:|---|",
    ]
    for f in informe:
        nat = f"{f['ctx_nativo']:,}" if f["ctx_nativo"] else "—"
        via = f"{f['viable']:,}" if f["viable"] else "—"
        con = f"{f['num_ctx_modelo']:,}" if f.get("num_ctx_modelo") else "—"
        kv = f"{f['kv_q4_nativo_gb']} GB" if f.get("kv_q4_nativo_gb") else "—"
        lineas.append(
            f"| `{f['modelo']}` | ollama | {nat} | {via} | {con} | {kv} | {f['clase']} |"
        )

    anom = [
        f
        for f in informe
        if f.get("num_ctx_modelo") and f["ctx_nativo"] and f["num_ctx_modelo"] > f["ctx_nativo"]
    ]
    lineas += [
        "",
        "## Anomalías encontradas",
        "",
        "Modelos cuyo `num_ctx` del Modelfile **supera** el contexto nativo declarado por",
        "su arquitectura. Ahí el motor atiende más tokens de los que el modelo fue",
        "entrenado para manejar (RoPE fuera de rango): puede responder, pero degrada.",
        "",
    ]
    for f in anom:
        lineas.append(
            f"- `{f['modelo']}`: nativo {f['ctx_nativo']:,} → Modelfile {f['num_ctx_modelo']:,}"
        )

    SALIDA.write_text("\n".join(lineas) + "\n", encoding="utf-8")
    print("informe:", SALIDA)
    print("filas:", len(informe))
    for clase in orden:
        print(" ", clase, sum(1 for f in informe if f["clase"] == clase))


if __name__ == "__main__":
    main()
