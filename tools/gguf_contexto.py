"""Lee solo la cabecera de un GGUF (metadatos) sin cargar tensores.

Devuelve arquitectura, contexto nativo, tamaño de capas/cabezas y nombre. Se usa
para inventariar modelos que **no** están en Ollama: LM Studio, descargas y el
M.2. Lee únicamente el bloque de pares clave-valor, así que tarda milisegundos
por archivo en vez de segundos.
"""

import os
import struct
import sys

TIPOS = {
    0: ("<B", 1),
    1: ("<b", 1),
    2: ("<H", 2),
    3: ("<h", 2),
    4: ("<I", 4),
    5: ("<i", 4),
    6: ("<f", 4),
    7: ("<?", 1),
    10: ("<Q", 8),
    11: ("<q", 8),
    12: ("<d", 8),
}


def _cadena(fh):
    (n,) = struct.unpack("<Q", fh.read(8))
    return fh.read(n).decode("utf-8", "replace")


def _valor(fh, tipo):
    if tipo == 8:
        return _cadena(fh)
    if tipo == 9:  # array
        tipo_hijo, n = struct.unpack("<IQ", fh.read(12))
        return [_valor(fh, tipo_hijo) for _ in range(n)]
    fmt, tam = TIPOS[tipo]
    return struct.unpack(fmt, fh.read(tam))[0]


def leer(ruta):
    with open(ruta, "rb") as fh:
        magia = fh.read(4)
        if magia != b"GGUF":
            return None
        version, n_tensores, n_kv = struct.unpack("<IQQ", fh.read(20))
        meta = {}
        for _ in range(n_kv):
            clave = _cadena(fh)
            (tipo,) = struct.unpack("<I", fh.read(4))
            try:
                meta[clave] = _valor(fh, tipo)
            except Exception:  # noqa: BLE001
                break
    arq = meta.get("general.architecture") or ""
    ctx = meta.get(f"{arq}.context_length")
    return {
        "ruta": ruta,
        "gguf_version": version,
        "nombre": meta.get("general.name"),
        "arquitectura": arq,
        "ctx_nativo": ctx,
        "capas": meta.get(f"{arq}.block_count"),
        "cabezas": meta.get(f"{arq}.attention.head_count"),
        "cabezas_kv": meta.get(f"{arq}.attention.head_count_kv"),
        "embedding": meta.get(f"{arq}.embedding_length"),
        "tamano_gb": round(os.path.getsize(ruta) / 1024**3, 2),
        "tensores": n_tensores,
        "metadatos": len(meta),
    }


def main():
    import json

    raices = sys.argv[1:] or [r"C:\Users\kuki\.lmstudio\models"]
    salida = []
    for raiz in raices:
        for base, _, ficheros in os.walk(raiz):
            for f in ficheros:
                if f.lower().endswith(".gguf"):
                    ruta = os.path.join(base, f)
                    try:
                        d = leer(ruta)
                    except Exception as e:  # noqa: BLE001
                        d = {"ruta": ruta, "error": str(e)}
                    if d:
                        salida.append(d)
    destino = r"H:\KUKIA_CODE\INFORMES\contexto_gguf_disco.json"
    with open(destino, "w", encoding="utf-8") as fh:
        json.dump(salida, fh, ensure_ascii=False, indent=1)
    ok = [d for d in salida if d.get("ctx_nativo")]
    print(f"archivos: {len(salida)} | con contexto legible: {len(ok)} | guardado: {destino}")
    for d in sorted(ok, key=lambda x: -(x["ctx_nativo"] or 0))[:15]:
        print(
            f"  {os.path.basename(d['ruta'])[:52]:52} {d['ctx_nativo']:>9,} "
            f"{d['arquitectura']:10} {d['tamano_gb']:>6.2f} GB"
        )


if __name__ == "__main__":
    main()
