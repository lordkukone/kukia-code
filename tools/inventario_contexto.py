"""Inventario de solo lectura del contexto real de cada modelo.

Para cada modelo de Ollama consulta `/api/show` y saca:
  - `*.context_length` de `model_info` (contexto nativo declarado por la arquitectura)
  - `num_ctx` del Modelfile si lo hubiera (contexto configurado en el modelo)
  - tamaño en disco, familia, cuantización y parámetros

Para LM Studio consulta `/api/v0/models` (si existe) y `/v1/models`.
No modifica nada: solo GET.
"""

import json
import os
import sys
import urllib.request
from concurrent.futures import ThreadPoolExecutor

OLLAMA = "http://127.0.0.1:11434"
LMSTUDIO = ["http://127.0.0.1:1235", "http://127.0.0.1:1234"]


def get(url, timeout=60):
    with urllib.request.urlopen(url, timeout=timeout) as r:
        return json.load(r)


def post(url, payload, timeout=60):
    datos = json.dumps(payload).encode()
    req = urllib.request.Request(
        url, data=datos, headers={"Content-Type": "application/json"}
    )
    with urllib.request.urlopen(req, timeout=timeout) as r:
        return json.load(r)


def conversaciones(valor):
    """Normaliza 131072 o '131072' o '128k' a entero de tokens."""
    if valor is None:
        return None
    if isinstance(valor, (int, float)):
        return int(valor)
    texto = str(valor).strip().lower()
    mult = 1
    if texto.endswith("k"):
        mult, texto = 1024, texto[:-1]
    elif texto.endswith("m"):
        mult, texto = 1024 * 1024, texto[:-1]
    try:
        return int(float(texto) * mult)
    except ValueError:
        return None


def show(nombre):
    try:
        d = post(f"{OLLAMA}/api/show", {"model": nombre})
    except Exception as e:  # noqa: BLE001
        return {"modelo": nombre, "error": str(e)}
    info = d.get("model_info") or {}
    ctx = {}
    for k, v in info.items():
        if k.endswith(".context_length"):
            ctx[k] = conversaciones(v)
    # Parámetros del Modelfile (pueden traer num_ctx ya fijado)
    num_ctx_modelo = None
    for linea in (d.get("parameters") or "").splitlines():
        partes = linea.split()
        if len(partes) == 2 and partes[0] == "num_ctx":
            num_ctx_modelo = conversaciones(partes[1])
    detalles = d.get("details") or {}

    def buscar_arch(sufijo):
        for k, v in info.items():
            if k.endswith(sufijo) and isinstance(v, (int, float)):
                return int(v)
        return 0

    capas = buscar_arch(".block_count")
    cabezas = buscar_arch(".attention.head_count")
    cabezas_kv = buscar_arch(".attention.head_count_kv") or cabezas
    emb = buscar_arch(".embedding_length")
    dim = (emb / cabezas) if cabezas else 0
    nativo = max(ctx.values()) if ctx else None

    def kv_gb(ctx_tokens, bits):
        if not (capas and cabezas_kv and dim and ctx_tokens):
            return None
        elementos = 2 * capas * cabezas_kv * dim * ctx_tokens
        return round(elementos * bits / 8 / 1024**3, 2)

    return {
        "modelo": nombre,
        "familia": detalles.get("family"),
        "parametros": detalles.get("parameter_size"),
        "cuantizacion": detalles.get("quantization_level"),
        "ctx_por_clave": ctx,
        "ctx_nativo": nativo,
        "num_ctx_modelo": num_ctx_modelo,
        "capas": capas,
        "cabezas": cabezas,
        "cabezas_kv": cabezas_kv,
        "embedding": emb,
        "kv_fp16_nativo_gb": kv_gb(nativo, 16.0),
        "kv_q4_nativo_gb": kv_gb(nativo, 4.5),
        "kv_q4_por_8k_gb": kv_gb(8192, 4.5),
        "capacidades": d.get("capabilities") or [],
    }


def main():
    salida = {"ollama": [], "lmstudio": {}, "cargados": []}

    try:
        etiquetas = get(f"{OLLAMA}/api/tags").get("models", [])
    except Exception as e:  # noqa: BLE001
        print("Ollama no responde:", e)
        etiquetas = []
    salida["total_ollama"] = len(etiquetas)

    nombres = [(m.get("name") or m.get("model")) for m in etiquetas]
    tamanos = {
        (m.get("name") or m.get("model")): m.get("size", 0) for m in etiquetas
    }

    with ThreadPoolExecutor(max_workers=6) as ex:
        filas = list(ex.map(show, nombres))

    for f in filas:
        f["tamano_gb"] = round(tamanos.get(f["modelo"], 0) / 1024**3, 2)
    salida["ollama"] = filas

    try:
        salida["cargados"] = get(f"{OLLAMA}/api/ps").get("models", [])
    except Exception:  # noqa: BLE001
        pass

    for base in LMSTUDIO:
        try:
            v0 = get(f"{base}/api/v0/models", timeout=10)
            salida["lmstudio"] = {"base": base, "v0": v0}
            break
        except Exception:  # noqa: BLE001
            continue

    destino = sys.argv[1] if len(sys.argv) > 1 else "contexto_inventario.json"
    with open(destino, "w", encoding="utf-8") as fh:
        json.dump(salida, fh, ensure_ascii=False, indent=1)
    print("guardado:", os.path.abspath(destino))

    filas = [f for f in salida["ollama"] if f.get("ctx_nativo")]
    sin = [f for f in salida["ollama"] if not f.get("ctx_nativo")]
    print(f"modelos: {salida['total_ollama']} | con contexto detectable: {len(filas)} | sin dato: {len(sin)}")
    for f in sorted(filas, key=lambda x: -x["ctx_nativo"])[:10]:
        print(f"  {f['modelo'][:44]:44} {f['ctx_nativo']:>9,} nativo  modelo:{f['num_ctx_modelo'] or '-'}")
    for f in sin[:8]:
        print("  sin dato:", f["modelo"], f.get("error") or f.get("ctx_por_clave"))


if __name__ == "__main__":
    main()
