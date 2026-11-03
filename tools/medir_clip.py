# -*- coding: utf-8 -*-
"""
Mide un clip en ComfyUI con números reales: tiempo, pico de VRAM por GPU y pico
de RAM del sistema.

Qué hace exactamente:
  1. Comprueba que ComfyUI responde y que la cola está vacía.
  2. Lanza el workflow con POST /prompt.
  3. Mientras corre, sondea cada 0,5 s la VRAM usada de cada GPU (nvidia-smi) y la
     RAM libre del sistema (GlobalMemoryStatusEx).
  4. Espera a que el trabajo salga en /history y recoge los archivos producidos.
  5. Escribe un JSON con todas las medidas en output/anime_real/mediciones/.

No modifica nada del sistema: solo lee contadores y habla con ComfyUI por HTTP.

Uso:
  python tools/medir_clip.py workflows/anime_real/anime2real_ltx_api.json --etiqueta anime2real_ltx
"""
from __future__ import annotations

import argparse
import ctypes
import json
import os
import subprocess
import sys
import threading
import time
import urllib.request
from datetime import datetime

BASE = "http://127.0.0.1:8188"
SALIDA_MEDIDAS = r"H:\KUKIA_CODE\output\anime_real\mediciones"


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


def ram_gb() -> tuple[float, float]:
    """(libre_GB, total_GB) del sistema."""
    st = MEMORYSTATUSEX()
    st.dwLength = ctypes.sizeof(MEMORYSTATUSEX)
    ctypes.windll.kernel32.GlobalMemoryStatusEx(ctypes.byref(st))
    g = 1024.0**3
    return st.ullAvailPhys / g, st.ullTotalPhys / g


def vram_mib() -> list[tuple[int, str, float, float]]:
    """[(indice, nombre, usada_MiB, total_MiB)] por GPU."""
    try:
        salida = subprocess.run(
            [
                "nvidia-smi",
                "--query-gpu=index,name,memory.used,memory.total",
                "--format=csv,noheader,nounits",
            ],
            capture_output=True,
            text=True,
            timeout=10,
        ).stdout
    except Exception:
        return []
    gpus = []
    for linea in salida.strip().splitlines():
        partes = [p.strip() for p in linea.split(",")]
        if len(partes) >= 4:
            try:
                gpus.append((int(partes[0]), partes[1], float(partes[2]), float(partes[3])))
            except ValueError:
                continue
    return gpus


def get_json(url: str, timeout: int = 30):
    with urllib.request.urlopen(url, timeout=timeout) as r:
        return json.load(r)


def post_json(url: str, datos: dict, timeout: int = 60):
    peticion = urllib.request.Request(
        url,
        data=json.dumps(datos).encode("utf-8"),
        headers={"Content-Type": "application/json"},
    )
    try:
        with urllib.request.urlopen(peticion, timeout=timeout) as r:
            return r.status, json.load(r)
    except urllib.error.HTTPError as e:
        cuerpo = e.read().decode("utf-8", "replace")
        try:
            return e.code, json.loads(cuerpo)
        except Exception:
            return e.code, {"raw": cuerpo}


class Muestreador(threading.Thread):
    """Sondea VRAM y RAM en paralelo al trabajo de ComfyUI."""

    def __init__(self, intervalo: float = 0.5):
        super().__init__(daemon=True)
        self.intervalo = intervalo
        self.parar = threading.Event()
        self.pico_vram: dict[int, float] = {}
        self.nombre_gpu: dict[int, str] = {}
        self.total_vram: dict[int, float] = {}
        self.ram_min_libre = 1e9
        self.ram_total = 0.0
        self.muestras = 0

    def run(self) -> None:
        while not self.parar.is_set():
            for idx, nombre, usada, total in vram_mib():
                self.nombre_gpu[idx] = nombre
                self.total_vram[idx] = total
                self.pico_vram[idx] = max(self.pico_vram.get(idx, 0.0), usada)
            libre, total = ram_gb()
            self.ram_min_libre = min(self.ram_min_libre, libre)
            self.ram_total = total
            self.muestras += 1
            self.parar.wait(self.intervalo)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("workflow")
    ap.add_argument("--etiqueta", default="clip")
    ap.add_argument("--timeout", type=int, default=3600)
    args = ap.parse_args()

    with open(args.workflow, encoding="utf-8-sig") as f:
        wf = json.load(f)

    # 1) ComfyUI vivo y cola limpia
    try:
        stats = get_json(f"{BASE}/system_stats", timeout=15)
    except Exception as e:
        print(f"FALLO: ComfyUI no responde en {BASE} ({e})")
        return 1
    version = (stats.get("system") or {}).get("comfyui_version", "?")
    cola = get_json(f"{BASE}/queue", timeout=30)
    en_cola = len(cola.get("queue_running") or []) + len(cola.get("queue_pending") or [])
    print(f"ComfyUI {version} · cola={en_cola}")
    if en_cola:
        print("Hay trabajo en cola; espera o vacíala antes de medir (no la toco yo).")
        return 1

    libre0, total0 = ram_gb()
    vram0 = vram_mib()
    print(f"RAM antes: {libre0:.1f} GB libres de {total0:.1f} GB")
    print("VRAM antes: " + ", ".join(f"GPU{i}={u:.0f}/{t:.0f}MiB" for i, _, u, t in vram0))

    # 2) Lanzar
    t0 = time.time()
    muestreo = Muestreador()
    muestreo.start()
    estado, respuesta = post_json(f"{BASE}/prompt", {"prompt": wf, "client_id": "kukia-medicion"})
    if estado != 200 or "prompt_id" not in respuesta:
        muestreo.parar.set()
        print(f"ComfyUI RECHAZÓ el workflow (HTTP {estado}):")
        print(json.dumps(respuesta, indent=1, ensure_ascii=False)[:4000])
        return 1
    pid = respuesta["prompt_id"]
    print(f"en cola: {pid}")

    # 3) Esperar
    resultado = None
    while time.time() - t0 < args.timeout:
        time.sleep(1.5)
        try:
            h = get_json(f"{BASE}/history/{pid}", timeout=30)
        except Exception:
            continue
        if pid in h:
            est = (h[pid].get("status") or {}).get("status_str", "")
            if est == "error":
                muestreo.parar.set()
                print("ComfyUI FALLÓ ejecutando el workflow:")
                print(json.dumps(h[pid].get("status"), indent=1, ensure_ascii=False)[:4000])
                return 1
            if (h[pid].get("status") or {}).get("completed"):
                resultado = h[pid]
                break
    muestreo.parar.set()
    tiempo = time.time() - t0
    if resultado is None:
        print(f"tiempo agotado tras {args.timeout}s (prompt {pid} sigue dentro)")
        return 1

    # 4) Archivos producidos
    archivos = []
    for nodo in (resultado.get("outputs") or {}).values():
        for clave in ("images", "gifs", "videos"):
            for item in nodo.get(clave) or []:
                archivos.append(
                    {
                        "nombre": item.get("filename"),
                        "tipo": item.get("type"),
                        "subcarpeta": item.get("subfolder"),
                        "formato": item.get("format"),
                    }
                )

    libre1, _ = ram_gb()
    medidas = {
        "etiqueta": args.etiqueta,
        "fecha": datetime.now().isoformat(timespec="seconds"),
        "workflow": os.path.basename(args.workflow),
        "prompt_id": pid,
        "segundos": round(tiempo, 2),
        "ram_antes_gb_libres": round(libre0, 2),
        "ram_despues_gb_libres": round(libre1, 2),
        "ram_pico_usada_gb": round(muestreo.ram_total - muestreo.ram_min_libre, 2),
        "ram_total_gb": round(muestreo.ram_total, 2),
        "vram_antes": {f"GPU{i}": round(u, 0) for i, _, u, _ in vram0},
        "vram_pico_mib": {
            f"GPU{i} ({muestreo.nombre_gpu.get(i,'?')})": round(v, 0)
            for i, v in sorted(muestreo.pico_vram.items())
        },
        "vram_total_mib": {f"GPU{i}": round(t, 0) for i, t in sorted(muestreo.total_vram.items())},
        "muestras": muestreo.muestras,
        "archivos": archivos,
    }

    os.makedirs(SALIDA_MEDIDAS, exist_ok=True)
    destino = os.path.join(
        SALIDA_MEDIDAS, f"{datetime.now().strftime('%Y%m%d_%H%M%S')}_{args.etiqueta}.json"
    )
    with open(destino, "w", encoding="utf-8") as f:
        json.dump(medidas, f, indent=1, ensure_ascii=False)

    print("\n===== RESULTADO =====")
    print(f"tiempo: {tiempo:.1f} s")
    for k, v in medidas["vram_pico_mib"].items():
        print(f"pico VRAM {k}: {v:.0f} MiB de {medidas['vram_total_mib'].get(k.split(' ')[0], 0):.0f} MiB")
    print(f"pico RAM usada: {medidas['ram_pico_usada_gb']} GB de {medidas['ram_total_gb']} GB")
    print(f"archivos: {archivos}")
    print(f"medidas guardadas en {destino}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
