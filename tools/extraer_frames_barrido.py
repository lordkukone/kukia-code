# -*- coding: utf-8 -*-
"""
Extrae fotogramas de los webp del barrido y monta una hoja de contactos para comparar.

Por qué existe: `barrido_ltx_a2r.py` pedía el webp sin su subcarpeta (`a2r_sweep/`)
y no lo encontraba. Esto lo arregla a posteriori, sin repetir ni un segundo de GPU.

Uso:
  python tools/extraer_frames_barrido.py
"""
from __future__ import annotations

import glob
import os
import re
import subprocess

from PIL import Image, ImageDraw, ImageSequence

SALIDA_COMFY = r"I:\pinokio\api\comfy.git\app\output"
SUB = "a2r_sweep"
DESTINO = r"H:\KUKIA_CODE\output\anime_real\muestras\barrido"
GUIA_MP4 = r"I:\pinokio\api\comfy.git\app\input\a2r_guia.mp4"
FFMPEG = r"G:\descargas\ffmpeg-master-latest-win64-gpl-shared\bin\ffmpeg.exe"
FRAMES = (0, 16, 32)


def tag_de(nombre: str) -> str:
    """a2r_A0_s09_l10_00002_.webp -> A0_s09_l10"""
    base = os.path.splitext(os.path.basename(nombre))[0]
    m = re.match(r"a2r_(.+?)_\d+_?$", base)
    return m.group(1) if m else base


def extraer(webp: str) -> tuple[str, list[Image.Image]]:
    tag = tag_de(webp)
    with Image.open(webp) as im:
        frames = [f.convert("RGB") for f in ImageSequence.Iterator(im)]
    os.makedirs(DESTINO, exist_ok=True)
    for idx in FRAMES + (len(frames) - 1,):
        if idx < len(frames):
            frames[idx].save(os.path.join(DESTINO, f"{tag}_f{idx:02d}.png"))
    return tag, frames


def frame_guia() -> Image.Image | None:
    """Fotograma 16 del vídeo anime de entrada, como referencia de 'sigue siendo anime'."""
    if not os.path.isfile(GUIA_MP4):
        return None
    tmp = os.path.join(DESTINO, "_guia_f16.png")
    os.makedirs(DESTINO, exist_ok=True)
    cmd = [FFMPEG, "-y", "-loglevel", "error", "-i", GUIA_MP4,
           "-vf", "select=eq(n\\,16)", "-vframes", "1", tmp]
    try:
        if not os.path.isfile(tmp):
            subprocess.run(cmd, check=True, timeout=120)
        if os.path.isfile(tmp):
            return Image.open(tmp).convert("RGB")
    except Exception as e:
        print(f"  [aviso] sin frame de guía: {e}")
    return None


def hoja(filas: list[tuple[str, Image.Image]], destino: str, alto: int = 420) -> str | None:
    if not filas:
        return None
    miniaturas = []
    for tag, im in filas:
        w = int(im.width * alto / im.height)
        m = im.resize((w, alto), Image.LANCZOS)
        lienzo = Image.new("RGB", (w, alto + 26), (18, 18, 18))
        lienzo.paste(m, (0, 26))
        ImageDraw.Draw(lienzo).text((6, 7), tag, fill=(240, 240, 240))
        miniaturas.append(lienzo)
    total_w = sum(m.width for m in miniaturas) + 8 * (len(miniaturas) - 1)
    out = Image.new("RGB", (total_w, alto + 26), (10, 10, 10))
    x = 0
    for m in miniaturas:
        out.paste(m, (x, 0))
        x += m.width + 8
    out.save(destino)
    print(f"  hoja: {destino}  ({out.width}x{out.height})")
    return destino


def main() -> int:
    webps = sorted(glob.glob(os.path.join(SALIDA_COMFY, SUB, "*.webp")))
    if not webps:
        print("no hay webp en", os.path.join(SALIDA_COMFY, SUB))
        return 1
    filas = []
    for w in webps:
        try:
            tag, frames = extraer(w)
            print(f"{tag:16} {len(frames):3} frames  {os.path.basename(w)}")
            if len(frames) > 16:
                filas.append((tag, frames[16]))
        except Exception as e:
            print(f"  ERROR con {w}: {e}")
    ref = frame_guia()
    if ref is not None:
        filas.insert(0, ("ANIME (guia f16)", ref))
        print("  referencia de guia: OK")
    hoja(filas, os.path.join(DESTINO, "comparacion_f16.png"))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
