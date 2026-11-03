# -*- coding: utf-8 -*-
"""
Barrido Anime2Real / LTX 2.3 — más sigmas y más strength del IC-LoRA.

Qué hace:
  1. Parte del workflow medido `anime2real_ltx_api.json` (mismos nodos, mismo seed 42,
     mismos 49 frames, misma guía). Solo cambian tres cosas por intento:
        - número de sigmas (subdividiendo LA MISMA curva, no inventando otra)
        - strength del IC-LoRA anime2real (`LoraLoaderModelOnly.strength_model`)
       (el `strength` del nodo guía está capado a 1.0 por el propio nodo)
  2. Lanza cada intento por HTTP, mide tiempo de pared, pico de VRAM por GPU y pico
     de RAM con el mismo muestreador que `tools/medir_clip.py`.
  3. Guarda un JSON por intento y extrae los fotogramas 0/16/32/48 a PNG para poder
     mirarlos y decidir en qué punto deja de parecer anime.

No toca modelos, ni config de ComfyUI, ni drivers. Solo habla con 127.0.0.1:8188.

Uso:
  python tools/barrido_ltx_a2r.py
"""
from __future__ import annotations

import json
import os
import sys
import time
import urllib.error
import urllib.request
from datetime import datetime

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import medir_clip as mc  # Muestreador, vram_mib, ram_gb, get_json, post_json  (reutilizado)

BASE = "http://127.0.0.1:8188"
RAIZ = r"H:\KUKIA_CODE"
WORKFLOW = os.path.join(RAIZ, "workflows", "anime_real", "anime2real_ltx_api.json")
MEDIDAS = os.path.join(RAIZ, "output", "anime_real", "mediciones")
MUESTRAS = os.path.join(RAIZ, "output", "anime_real", "muestras", "barrido")
SALIDA_COMFY = r"I:\pinokio\api\comfy.git\app\output"

# Curva original del baseline (9 puntos = 8 pasos). Es la referencia a subdividir.
CURVA_BASE = [1.0, 0.99375, 0.9875, 0.98125, 0.975, 0.909375, 0.725, 0.421875, 0.0]
CABEZA = CURVA_BASE[:5]          # 1.0 → 0.975: se queda igual (calentamiento destilado)
COLA = CURVA_BASE[4:]            # 0.975 → 0.0: aquí se añaden pasos

# OJO: `LTXAddVideoICLoRAGuide.strength` está declarado con max=1.0 por el nodo.
# No se puede subir: 1.0 es el tope. El único mando libre es `strength_model`
# del LoraLoaderModelOnly (rango -100..100), que es el que se barre aquí.
INTENTOS = [
    # tag             sigmas  guide  lora
    ("A0_s09_l10", 9, 1.0, 1.0),     # referencia nueva, mismo backend de atención
    ("A_s13_l10", 13, 1.0, 1.0),     # solo más sigmas (aísla el efecto)
    ("B_s13_l16", 13, 1.0, 1.6),     # solo más LoRA (aísla el efecto)
    ("C_s17_l24", 17, 1.0, 2.4),     # subiendo los dos
    ("D_s25_l32", 25, 1.0, 3.2),     # al límite razonable
]

# Fase 4: régimen abierto. Sin imagen de entrada (strength 0,0) no hay dibujo que copiar,
# así se ve si el modelo sabe producir acción real o si el LoRA no aporta nada.
FASE4 = [
    ("J_t2v_l24", 13, 0.0, 2.4, 0.0, True),   # texto puro + LoRA anime2real 2.4
    ("K_t2v_l00", 13, 0.0, 0.0, 0.0, True),   # texto puro sin LoRA (control)
]

# Fase 2: la fase 1 demostró que subir sigmas/LoRA no cambia el estilo. El nodo que
# mete los fotogramas anime es el GUIDE, así que aquí se BAJA (su rango útil es 0..1).
FASE2 = [
    ("E_s13_g050_l20", 13, 0.50, 2.0),
    ("F_s13_g025_l24", 13, 0.25, 2.4),
    ("G_s13_g000_l24", 13, 0.00, 2.4),
]

# Fase 3: la fase 2 demostró que el strength del guide no cambia nada (0.0 da el mismo
# meme). Aquí se ANULA el nodo del guide y se baja la fuerza del primer fotograma (nodo
# 12), que es la otra vía por la que entra el anime en el latente.
FASE3 = [
    ("H_noguide_l24_i07", 13, 1.0, 2.4, 0.70, True),
    ("I_noguide_l24_i02", 13, 1.0, 2.4, 0.20, True),
]


def sigmas(n: int) -> list[float]:
    """Subdivide la MISMA curva del baseline en `n` puntos (n>=9 conserva la forma)."""
    if n <= len(CURVA_BASE):
        return CURVA_BASE[:n]
    segmentos = n - len(CABEZA)          # tramos en la cola
    origen = len(COLA) - 1               # tramos originales en la cola
    out = list(CABEZA[:-1])
    for i in range(segmentos + 1):
        t = i / segmentos
        pos = t * origen
        j = min(int(pos), origen - 1)
        frac = pos - j
        out.append(round(COLA[j] + (COLA[j + 1] - COLA[j]) * frac, 6))
    out[-1] = 0.0
    return out


def esperar_cola_vacia(timeout: int = 300) -> bool:
    t0 = time.time()
    while time.time() - t0 < timeout:
        try:
            q = mc.get_json(f"{BASE}/queue", timeout=30)
            if not (q.get("queue_running") or q.get("queue_pending")):
                return True
        except Exception:
            pass
        time.sleep(3)
    return False


def extraer_frames(nombre_webp: str, tag: str, subcarpeta: str = "") -> list[str]:
    """Saca los fotogramas 0/16/32/48 del webp animado a PNG (para mirarlos)."""
    try:
        from PIL import Image, ImageSequence
    except Exception as e:
        print(f"  [aviso] sin PIL, no extraigo frames: {e}")
        return []
    origen = os.path.join(SALIDA_COMFY, subcarpeta or "", nombre_webp)
    if not os.path.isfile(origen):
        print(f"  [aviso] no encuentro {origen}")
        return []
    os.makedirs(MUESTRAS, exist_ok=True)
    hechos = []
    with Image.open(origen) as im:
        frames = [f.convert("RGB") for f in ImageSequence.Iterator(im)]
    for idx in (0, 16, 32, len(frames) - 1):
        if idx >= len(frames):
            continue
        destino = os.path.join(MUESTRAS, f"{tag}_f{idx:02d}.png")
        frames[idx].save(destino)
        hechos.append(destino)
    print(f"  frames: {len(frames)} · extraidos {len(hechos)} a {MUESTRAS}")
    return hechos


def un_intento(tag: str, n_sigmas: int, guide: float, lora: float,
               img_strength: "float | None" = None, sin_guide: bool = False) -> dict:
    with open(WORKFLOW, encoding="utf-8-sig") as f:
        wf = json.load(f)

    lista = sigmas(n_sigmas)
    wf["20"]["inputs"]["sigmas"] = ", ".join(f"{s:g}" for s in lista)
    wf["13"]["inputs"]["strength"] = guide
    wf["3"]["inputs"]["strength_model"] = lora
    if img_strength is not None:
        wf["12"]["inputs"]["strength"] = img_strength
    if sin_guide:
        # Bypass real del IC-LoRA guide: sus 3 salidas pasan a ser sus 3 entradas.
        wf["16"]["inputs"]["video_latent"] = ["12", 0]
        wf["18"]["inputs"]["positive"] = ["7", 0]
        wf["18"]["inputs"]["negative"] = ["7", 1]
        wf["23"]["inputs"]["positive"] = ["7", 0]
        wf["23"]["inputs"]["negative"] = ["7", 1]
        del wf["13"]
    wf["25"]["inputs"]["filename_prefix"] = f"a2r_sweep/a2r_{tag}"

    print(f"\n===== {tag} · {len(lista)} sigmas · guide={guide} · lora={lora}"
          f" · img={img_strength} · sin_guide={sin_guide} =====")
    print("  sigmas: " + ", ".join(f"{s:g}" for s in lista))

    if not esperar_cola_vacia():
        return {"tag": tag, "error": "cola de ComfyUI ocupada, no lanzo"}

    libre0, total0 = mc.ram_gb()
    vram0 = mc.vram_mib()

    t0 = time.time()
    m = mc.Muestreador()
    m.start()
    estado, resp = mc.post_json(f"{BASE}/prompt", {"prompt": wf, "client_id": f"barrido-{tag}"})
    if estado != 200 or "prompt_id" not in resp:
        m.parar.set()
        print(f"  RECHAZADO HTTP {estado}: {json.dumps(resp, ensure_ascii=False)[:1500]}")
        return {"tag": tag, "error": f"HTTP {estado}", "respuesta": resp}
    pid = resp["prompt_id"]

    resultado, ultimo = None, 0.0
    while time.time() - t0 < 3600:
        time.sleep(2.0)
        try:
            h = mc.get_json(f"{BASE}/history/{pid}", timeout=30)
        except Exception:
            continue
        if pid not in h:
            continue
        est = (h[pid].get("status") or {}).get("status_str", "")
        if est == "error":
            m.parar.set()
            msg = json.dumps(h[pid].get("status"), ensure_ascii=False)[:2500]
            print(f"  ERROR en ComfyUI: {msg}")
            return {"tag": tag, "error": "error de ejecucion", "detalle": msg}
        # progreso aproximado por nodos ejecutados
        ejec = len(((h[pid].get("status") or {}).get("messages") or []))
        if ejec != ultimo:
            ultimo = ejec
        if (h[pid].get("status") or {}).get("completed"):
            resultado = h[pid]
            break
    m.parar.set()
    tiempo = time.time() - t0
    if resultado is None:
        print(f"  TIMEOUT tras {tiempo:.0f}s")
        return {"tag": tag, "error": "timeout", "segundos": round(tiempo, 1)}

    archivos = []
    for nodo in (resultado.get("outputs") or {}).values():
        for clave in ("images", "gifs", "videos"):
            for it in nodo.get(clave) or []:
                archivos.append(it)

    libre1, _ = mc.ram_gb()
    med = {
        "tag": tag,
        "fecha": datetime.now().isoformat(timespec="seconds"),
        "sigmas_n": len(lista),
        "sigmas": lista,
        "ic_guide_strength": guide,
        "lora_strength": lora,
        "segundos": round(tiempo, 1),
        "vram_antes": {f"GPU{i}": round(u) for i, _, u, _ in vram0},
        "vram_pico_mib": {
            f"GPU{i} ({m.nombre_gpu.get(i, '?')})": round(v)
            for i, v in sorted(m.pico_vram.items())
        },
        "vram_total_mib": {f"GPU{i}": round(t) for i, t in sorted(m.total_vram.items())},
        "ram_antes_gb_libres": round(libre0, 2),
        "ram_despues_gb_libres": round(libre1, 2),
        "ram_pico_usada_gb": round(m.ram_total - m.ram_min_libre, 2),
        "ram_total_gb": round(m.ram_total, 2),
        "archivos": [a.get("filename") for a in archivos],
    }
    med["frames_png"] = []
    for a in archivos:
        nombre = a.get("filename") or ""
        if nombre.lower().endswith(".webp"):
            med["frames_png"] += extraer_frames(nombre, tag, a.get("subfolder") or "")

    print(f"  tiempo: {med['segundos']} s")
    for k, v in med["vram_pico_mib"].items():
        print(f"  pico {k}: {v} MiB")
    print(f"  pico RAM usada: {med['ram_pico_usada_gb']} GB")
    return med


def main() -> int:
    global INTENTOS
    if len(sys.argv) > 1 and sys.argv[1].lower() in ("fase2", "2"):
        INTENTOS = FASE2
        print("### FASE 2: bajando el guide (los sigmas se quedan en 13)")
    elif len(sys.argv) > 1 and sys.argv[1].lower() in ("fase4", "4"):
        INTENTOS = FASE4
        print("### FASE 4: régimen abierto, sin imagen de entrada (strength 0,0)")
    elif len(sys.argv) > 1 and sys.argv[1].lower() in ("fase3", "3"):
        INTENTOS = FASE3
        print("### FASE 3: anulando el guide y bajando la fuerza del primer fotograma")
    # Permite reanudar solo algunos intentos: `barrido_ltx_a2r.py fase2 F_s13_g025_l24 ...`
    filtro = [a for a in sys.argv[2:] if not a.startswith("-")]
    if filtro:
        INTENTOS = [t for t in INTENTOS if t[0] in filtro]
        print(f"### reanudando solo: {[t[0] for t in INTENTOS]}")
    try:
        mc.get_json(f"{BASE}/system_stats", timeout=20)
    except Exception as e:
        print(f"ComfyUI no responde en {BASE}: {e}")
        return 1

    os.makedirs(MEDIDAS, exist_ok=True)
    os.makedirs(MUESTRAS, exist_ok=True)
    salida = []
    for intento in INTENTOS:
        tag, ns, g, l = intento[:4]
        extra = intento[4:]
        try:
            salida.append(un_intento(tag, ns, g, l, *extra))
        except Exception as e:
            print(f"  EXCEPCION en {tag}: {e}")
            salida.append({"tag": tag, "error": str(e)})
        ruta = os.path.join(MEDIDAS, f"barrido_a2r_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json")
        with open(ruta, "w", encoding="utf-8") as f:
            json.dump(salida, f, indent=1, ensure_ascii=False)

    print("\n================= RESUMEN =================")
    print(f"{'tag':18} {'sigmas':>6} {'guide':>5} {'lora':>5} {'seg':>8} {'GPU1 MiB':>9}")
    for r in salida:
        if "error" in r:
            print(f"{r['tag']:18} ERROR: {r['error']}")
            continue
        g1 = next((v for k, v in r["vram_pico_mib"].items() if "GPU1" in k), 0)
        print(f"{r['tag']:18} {r['sigmas_n']:6} {r['ic_guide_strength']:5} {r['lora_strength']:5} "
              f"{r['segundos']:8.1f} {g1:9}")
    print(f"\nJSON: {ruta}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
