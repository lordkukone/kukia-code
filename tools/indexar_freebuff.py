"""Indexa las conversaciones de Freebuff en la memoria de KUKIA CODE.

Garantías:
  - La base de Freebuff se abre con `mode=ro&immutable=1`: SQLite no puede
    escribir en ella ni crear diarios. Se verifica su SHA-256 antes y después.
  - La memoria de KUKIA se escribe dentro de una transacción, con copia previa
    hecha por fuera de este script.
  - Es idempotente: cada fragmento lleva una marca `[fb:<hilo>:<i>/<n>]` y se
    salta si ya existe.

Uso:
    python tools/indexar_freebuff.py           # importa lo que falte
    python tools/indexar_freebuff.py --dry-run # solo cuenta, no escribe
"""

import argparse
import hashlib
import json
import os
import sqlite3
from datetime import datetime, timezone

FREEBBUF = (
    r"C:\Users\kuki\.config\freebuff-desktop\projects"
    r"\77aa033f-8bbe-43fc-b872-c43c04ed3b4a-9cda7a56-8a5b-45ca-b1dc-673a9c9e8d08"
    r"\desktop-v2.db"
)
KUKIA = os.path.join(os.environ["APPDATA"], "KukiaCode", "kukia.db")
CHUNK = 4000


def sha256(ruta):
    h = hashlib.sha256()
    with open(ruta, "rb") as fh:
        for bloque in iter(lambda: fh.read(4 * 1024 * 1024), b""):
            h.update(bloque)
    return h.hexdigest()


def iso(ms):
    try:
        return datetime.fromtimestamp(ms / 1000, tz=timezone.utc).isoformat()
    except Exception:  # noqa: BLE001
        return datetime.now(tz=timezone.utc).isoformat()


def texto_de(parts_json):
    """Solo las partes de texto: se descartan tool, reasoning, cambios y avisos."""
    try:
        partes = json.loads(parts_json or "[]")
    except Exception:  # noqa: BLE001
        return ""
    trozos = []
    for p in partes:
        if isinstance(p, dict) and p.get("kind") == "text":
            t = p.get("text") or ""
            if t.strip():
                trozos.append(t)
    return "\n".join(trozos)


def trocear(mensajes, hilo):
    """Agrupa mensajes en bloques de ~CHUNK caracteres sin partir mensajes."""
    bloques, actual, inicio_ts = [], [], None
    tam = 0
    for rol, ts, texto in mensajes:
        etiqueta = "TÚ" if rol == "user" else "BUFFY"
        pieza = f"[{etiqueta}] {texto}\n"
        if tam and tam + len(pieza) > CHUNK:
            bloques.append((inicio_ts, actual, tam))
            actual, tam, inicio_ts = [], 0, None
        if inicio_ts is None:
            inicio_ts = ts
        actual.append(pieza)
        tam += len(pieza)
    if actual:
        bloques.append((inicio_ts, actual, tam))
    return bloques


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--dry-run", action="store_true")
    args = ap.parse_args()

    if not os.path.exists(FREEBBUF):
        print("no existe la base de Freebuff:", FREEBBUF)
        return 1

    antes = sha256(FREEBBUF)
    fb = sqlite3.connect(f"file:{FREEBBUF}?mode=ro&immutable=1", uri=True)
    hilos = list(fb.execute("select id, title, created_at, project_path from threads order by created_at"))

    kukia = sqlite3.connect(KUKIA)
    previos = kukia.execute("select count(*) from memories").fetchone()[0]
    print(f"memorias antes: {previos}")

    insertados = 0
    saltados = 0
    plan = []
    for tid, titulo, creado, proyecto in hilos:
        mensajes = []
        for rol, pj, ts in fb.execute(
            "select role, parts_json, ts from messages where thread_id=? order by seq", (tid,)
        ):
            t = texto_de(pj)
            if t:
                mensajes.append((rol, ts, t))
        bloques = trocear(mensajes, tid)
        fecha = iso(creado)
        for i, (ts, trozos, _tam) in enumerate(bloques, start=1):
            marca = f"[fb:{tid[:8]}:{i}/{len(bloques)}]"
            existe = kukia.execute(
                "select count(*) from memories where content like ?", (f"%{marca}%",)
            ).fetchone()[0]
            if existe:
                saltados += 1
                continue
            cabecera = (
                f"{marca} Freebuff · «{titulo}» · {fecha[:10]} · "
                f"fragmento {i} de {len(bloques)}"
                + (f" · proyecto {os.path.basename(proyecto)}" if proyecto else "")
            )
            contenido = cabecera + "\n" + "".join(trozos)
            etiquetas = f"freebuff,hilo:{tid[:8]},fuente:freebuff"
            if proyecto:
                etiquetas += f",proyecto:{os.path.basename(proyecto) or 'raiz'}"
            plan.append((contenido, etiquetas, iso(ts)))

    print(f"fragmentos a insertar: {len(plan)} | ya presentes: {saltados}")
    if args.dry_run:
        if plan:
            print("\n--- muestra del primer fragmento ---")
            print(plan[0][0][:700])
        kukia.close()
        fb.close()
        return 0

    with kukia:
        for contenido, etiquetas, fecha in plan:
            kukia.execute(
                "INSERT INTO memories (content,tags,created_at) VALUES (?,?,?)",
                (contenido, etiquetas, fecha),
            )
            insertados += 1

    despues = kukia.execute("select count(*) from memories").fetchone()[0]
    kukia.close()
    fb.close()

    final = sha256(FREEBBUF)
    print(f"insertados: {insertados} | memorias después: {despues}")
    print("Freebuff intacto:", "SÍ" if antes == final else "NO (¡cambió el hash!)")
    print("  sha256 antes:", antes)
    print("  sha256 ahora:", final)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
