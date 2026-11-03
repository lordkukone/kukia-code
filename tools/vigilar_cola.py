# -*- coding: utf-8 -*-
"""
Vigila un trabajo de ComfyUI en primer plano y mide tiempo, pico de VRAM por GPU
y pico de RAM. Sirve cuando el trabajo ya está dentro de la cola (no necesita
conocer el prompt_id: lo saca de /queue) o cuando se le pasa --prompt-id.

No modifica nada: solo lee /queue, /history, nvidia-smi y los contadores de RAM.

Uso:
  python tools/vigilar_cola.py --etiqueta anime2real_ltx --salida output/anime_real/mediciones
"""
from __future__ import annotations

import argparse
import ctypes
import json
import os
import subprocess
import time
import urllib.request
from datetime import datetime

BASE = "http://127.0.0.1:8188"


class MEMORYSTATUSEX(ctypes.Structure):
    _fields_ = [
        ("dwLength", ctypes.c_ulong),
        ("dwMemoryLoad", ctypes.c_ulong),
        ("ullTotalPhys", ctypes.c_ulonglong),
        ("ullAvailPhys", ctypes.c_ulonglong),
        ("ullTotalPageFile", ctypes.c_ulonglong),
        ("ullAvailPageFile", ctypes.c_ulonglong),
        ("ullTotalVirtual", ctypes.c_ulonglong),
        ("ullAvailVirtual", ctypes.c_ulonglong),
        ("ullAvailExtendedVirtual", ctypes.c_ulonglong),
    ]


def ram() -> tuple[float, float]:
    st = MEMORYSTATUSEX()
    st.dwLength = ctypes.sizeof(MEMORYSTATUSEX)
    ctypes.windll.kernel32.GlobalMemoryStatusEx(ctypes.byref(st))
    g = 1024.0**3
    return st.ullAvailPhys / g, st.ullTotalPhys / g


def vram() -> list[tuple[int, str, float, float]]:
    try:
        txt = subprocess.run(
            ["nvidia-smi", "--query-gpu=index,name,memory.used,memory.total",
             "--format=csv,noheader,nounits"],
            capture_output=True, text=True, timeout=10,
        ).stdout
    except Exception:
        return []
    out = []
    for linea in txt.strip().splitlines():
        p = [x.strip() for x in linea.split(",")]
        if len(p) >= 4:
            try:
                out.append((int(p[0]), p[1], float(p[2]), float(p[3])))
            except ValueError:
                pass
    return out


def get(url: str, timeout: int = 30):
    with urllib.request.urlopen(url, timeout=timeout) as r:
        return json.load(r)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--etiqueta", default="clip")
    ap.add_argument("--salida", default=r"H:\KUKIA_CODE\output\anime_real\mediciones")
    ap.add_argument("--prompt-id", default="")
    ap.add_argument("--max-segundos", type=int, default=540)
    ap.add_argument("--intervalo", type=float, default=2.0)
    args = ap.parse_args()

    pid = args.prompt_id
    t0 = time.time()
    pico: dict[int, float] = {}
    nombre: dict[int, str] = {}
    total: dict[int, float] = {}
    ram_min, ram_total = 1e9, 0.0
    muestras = 0
    estado_final = ""
    ultimo_latido = 0.0

    try:
        while time.time() - t0 < args.max_segundos:
            for i, n, u, t in vram():
                nombre[i], total[i] = n, t
                pico[i] = max(pico.get(i, 0.0), u)
            libre, tot = ram()
            ram_min, ram_total = min(ram_min, libre), tot
            muestras += 1

            try:
                cola = get(f"{BASE}/queue", timeout=20)
            except Exception:
                cola = {"queue_running": [], "queue_pending": []}
            corriendo = cola.get("queue_running") or []
            pendiente = cola.get("queue_pending") or []
            if not pid and corriendo:
                primer = corriendo[0]
                pid = str(primer[1]) if isinstance(primer, (list, tuple)) and len(primer) > 1 else str(primer)

            if pid:
                # ¿ya ha terminado?
                if not corriendo and not pendiente:
                    try:
                        h = get(f"{BASE}/history/{pid}", timeout=20)
                    except Exception:
                        h = {}
                    if pid in h:
                        estado_final = (h[pid].get("status") or {}).get("status_str", "")
                        if (h[pid].get("status") or {}).get("completed") or estado_final == "error":
                            break
                if time.time() - ultimo_latido > 30:
                    import json as _j
                    try:
                        prog = get(f"{BASE}/prompt", timeout=15)
                    except Exception:
                        prog = {}
                    ultimo_latido = time.time()
                    print(f"  t={time.time()-t0:5.0f}s · cola={len(corriendo)} · "
                          f"VRAM=" + ",".join(f"{pico.get(i,0):.0f}" for i in sorted(pico)) + " MiB", flush=True)
            elif not corriendo and not pendiente and time.time() - t0 > 5:
                print("Nada en la cola y sin prompt_id: el trabajo ya no está.")
                break
            time.sleep(args.intervalo)
    except KeyboardInterrupt:
        print("interrumpido por el usuario")

    tiempo = time.time() - t0
    resultado: dict = {}
    archivos = []
    if pid:
        try:
            h = get(f"{BASE}/history/{pid}", timeout=30)
            if pid in h:
                resultado = h[pid]
                for nodo in (resultado.get("outputs") or {}).values():
                    for clave in ("images", "gifs", "videos"):
                        for item in nodo.get(clave) or []:
                            archivos.append({
                                "nombre": item.get("filename"),
                                "tipo": item.get("type"),
                                "subcarpeta": item.get("subfolder"),
                            })
                estado_final = (resultado.get("status") or {}).get("status_str", estado_final) or estado_final
        except Exception:
            pass

    medidas = {
        "etiqueta": args.etiqueta,
        "fecha": datetime.now().isoformat(timespec="seconds"),
        "prompt_id": pid,
        "segundos_vigilados": round(tiempo, 1),
        "estado": estado_final or "(sin confirmar)",
        "vram_pico_mib": {f"GPU{i} ({nombre.get(i,'?')})": round(v) for i, v in sorted(pico.items())},
        "vram_total_mib": {f"GPU{i}": round(t) for i, t in sorted(total.items())},
        "ram_pico_usada_gb": round(ram_total - ram_min, 2),
        "ram_total_gb": round(ram_total, 2),
        "ram_min_libre_gb": round(ram_min, 2),
        "muestras": muestras,
        "archivos": archivos,
    }
    os.makedirs(args.salida, exist_ok=True)
    destino = os.path.join(args.salida, f"{datetime.now().strftime('%Y%m%d_%H%M%S')}_{args.etiqueta}.json")
    with open(destino, "w", encoding="utf-8") as f:
        json.dump(medidas, f, indent=1, ensure_ascii=False)

    print("\n===== RESULTADO =====")
    print(f"estado: {medidas['estado']} · vigilado {tiempo:.0f} s · prompt {pid}")
    for k, v in medidas["vram_pico_mib"].items():
        print(f"pico VRAM {k}: {v} MiB")
    print(f"pico RAM usada: {medidas['ram_pico_usada_gb']} GB de {medidas['ram_total_gb']} GB")
    print(f"archivos: {archivos}")
    print(f"guardado: {destino}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
