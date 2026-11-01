"""Envía un workflow (formato API) a ComfyUI y verifica que produce un archivo.

Uso:  python comfy_verificar_workflow.py <workflow.json> [imagen_entrada] [segundos]

No instala nada, no modifica nada de ComfyUI: solo encola y espera.
"""
import json
import pathlib
import sys
import time
import urllib.error
import urllib.parse
import urllib.request

BASE = "http://127.0.0.1:8188"


def post(ruta, cuerpo):
    datos = json.dumps(cuerpo).encode()
    req = urllib.request.Request(
        BASE + ruta, data=datos, headers={"Content-Type": "application/json"}
    )
    with urllib.request.urlopen(req, timeout=30) as r:
        return json.loads(r.read().decode())


def get(ruta):
    with urllib.request.urlopen(BASE + ruta, timeout=30) as r:
        return json.loads(r.read().decode())


def main():
    if len(sys.argv) < 2:
        print("falta la ruta del workflow")
        return 2
    wf_path = pathlib.Path(sys.argv[1])
    imagen = sys.argv[2] if len(sys.argv) > 2 else None
    limite = int(sys.argv[3]) if len(sys.argv) > 3 else 300

    wf = json.loads(wf_path.read_text(encoding="utf-8"))
    if imagen:
        for nid, nodo in wf.items():
            if nodo.get("class_type") == "LoadImage":
                nodo["inputs"]["image"] = imagen
                print(f"entrada: {imagen} (nodo {nid})")

    print(f"workflow: {wf_path.name} · {len(wf)} nodos")
    try:
        r = post("/prompt", {"prompt": wf, "client_id": "kukia-code-verify"})
    except urllib.error.HTTPError as e:
        cuerpo = json.loads(e.read().decode())
        print(f"RECHAZADO ({e.code})")
        print(json.dumps(cuerpo, indent=2, ensure_ascii=False)[:4000])
        return 1

    pid = r.get("prompt_id")
    print(f"encolado: {pid}")

    t0 = time.time()
    while time.time() - t0 < limite:
        hist = get(f"/history/{pid}")
        if pid in hist:
            h = hist[pid]
            estado = h.get("status", {})
            print(f"estado: {estado.get('status_str')} · completed={estado.get('completed')}")
            if not estado.get("completed", False):
                for m in estado.get("messages", [])[-3:]:
                    print("   ", str(m)[:300])
                return 1
            salidas = h.get("outputs", {})
            encontrados = []
            for nid, out in salidas.items():
                for img in out.get("images", []):
                    encontrados.append(img)
                    print(
                        f"  salida nodo {nid}: {img.get('filename')} "
                        f"({img.get('type')}, {img.get('subfolder') or '.'})"
                    )
            if not encontrados:
                print("sin imágenes en outputs")
                return 1
            print(f"OK en {time.time() - t0:.1f} s")
            for img in encontrados:
                sub = img.get("subfolder") or ""
                url = f"/view?filename={urllib.parse.quote(img['filename'])}&type={img.get('type','output')}&subfolder={urllib.parse.quote(sub)}"
                print("   descarga:", BASE + url)
            return 0
        time.sleep(2)
    print(f"TIEMPO AGOTADO ({limite}s)")
    return 1


if __name__ == "__main__":
    sys.exit(main())
