#!/usr/bin/env python
"""Entrenador de LoRA de KUKIA CODE (LA8 — Character LoRA Studio).

Entrena un LoRA de atencion sobre un modelo de difusion (SDXL o SD 1.5) con
diffusers + peft, a partir de una carpeta con imagenes y un `.txt` por imagen con
su caption.

Habla con KUKIA por stdout con **una linea JSON por evento**, para que el nucleo
pueda seguir el progreso sin adivinar:

    {"evento":"inicio","total":800,"modelo":"..."}
    {"evento":"paso","paso":10,"total":800,"perdida":0.0842,"seg":9.8}
    {"evento":"guardado","ruta":"...safetensors","claves":336}
    {"evento":"error","mensaje":"..."}

El LoRA se guarda con nombres al estilo sd-scripts/kohya
(`lora_unet_down_blocks_0_attentions_0_...`), que es lo que cargan ComfyUI,
sd-scripts y la mayoria de interfaces, en vez de los nombres internos de peft.

Uso:
    python trainer/entrenar_lora.py --dataset DIR --salida DIR --modelo RUTA \
        --pasos 800 --rango 8 --resolucion 1024

Modo de prueba (no descarga nada, no necesita GPU): `--probar`
"""

import argparse
import json
import math
import os
import random
import sys
import time
from pathlib import Path


def emitir(**campos):
    """Un evento JSON por linea. Sin buffering: KUKIA lee en tiempo real."""
    print(json.dumps(campos, ensure_ascii=False), flush=True)


def error(mensaje):
    emitir(evento="error", mensaje=str(mensaje))
    sys.exit(1)


# ---------------------------------------------------------------- utilidades


def imagenes_del_dataset(directorio):
    """Imagenes con su caption. Sin caption no se entrena: se avisa."""
    extensiones = {".png", ".jpg", ".jpeg", ".webp", ".bmp"}
    elementos, sin_caption = [], []
    for ruta in sorted(Path(directorio).iterdir()):
        if ruta.suffix.lower() not in extensiones:
            continue
        caption = ruta.with_suffix(".txt")
        if not caption.exists() or not caption.read_text(encoding="utf-8").strip():
            sin_caption.append(ruta.name)
            continue
        elementos.append((ruta, caption.read_text(encoding="utf-8").strip()))
    if sin_caption:
        emitir(evento="aviso", mensaje=f"{len(sin_caption)} imagen(es) sin caption se omiten",
               imagenes=sin_caption[:10])
    return elementos


def cargar_imagen(ruta, resolucion):
    """Recorte cuadrado al centro, que es lo que espera el entrenamiento."""
    from PIL import Image

    img = Image.open(ruta).convert("RGB")
    lado = min(img.size)
    izq = (img.width - lado) // 2
    arr = (img.height - lado) // 2
    img = img.crop((izq, arr, izq + lado, arr + lado))
    return img.resize((resolucion, resolucion), Image.LANCZOS)


def clave_kohya(clave, rango):
    """Traduce una clave de peft a la forma que cargan ComfyUI/sd-scripts.

    peft:  base_model.model.down_blocks.0.attentions.0...to_q.lora_A.weight
    kohya: lora_unet_down_blocks_0_attentions_0_..._to_q.lora_down.weight
    """
    for prefijo in ("base_model.model.", "model.diffusion_model.", "unet."):
        if clave.startswith(prefijo):
            clave = clave[len(prefijo):]
    if clave.endswith(".lora_A.weight"):
        base, lado = clave[: -len(".lora_A.weight")], "down"
    elif clave.endswith(".lora_B.weight"):
        base, lado = clave[: -len(".lora_B.weight")], "up"
    else:
        return None
    nombre = "lora_unet_" + base.replace(".", "_")
    return f"{nombre}.lora_{lado}.weight", f"{nombre}.alpha", rango


def exportar_lora(unet, salida, rango, meta):
    """Guarda el LoRA en safetensors con nombres kohya y un JSON de metadatos."""
    import torch
    from peft import get_peft_model_state_dict

    from safetensors.torch import save_file

    estado = get_peft_model_state_dict(unet)
    tensores, alfas = {}, {}
    for clave, valor in estado.items():
        traducida = clave_kohya(clave, rango)
        if traducida is None:
            continue
        nombre, alpha, r = traducida
        tensores[nombre] = valor.detach().to(torch.float32).contiguous().cpu()
        alfas[alpha] = torch.tensor(float(r), dtype=torch.float32)
    tensores.update(alfas)
    salida = Path(salida)
    salida.parent.mkdir(parents=True, exist_ok=True)
    save_file(tensores, str(salida), metadata={"kukia": "1", **{k: str(v) for k, v in meta.items()}})
    salida.with_suffix(".json").write_text(
        json.dumps(meta, ensure_ascii=False, indent=2), encoding="utf-8"
    )
    return len([k for k in tensores if k.endswith(".lora_down.weight")])


# ---------------------------------------------------------------- pruebas


def probar(args):
    """Prueba de humo: UNet diminuta, 2 pasos, sin descargas ni GPU.

    Comprueba de verdad la maquinaria: inyeccion del adaptador, paso de
    optimizador, guardado y formato de las claves. No dice nada de la calidad.
    """
    import torch
    from diffusers import UNet2DConditionModel
    from peft import LoraConfig

    emitir(evento="inicio", total=args.pasos, modelo="unet-diminuta", prueba=True)
    unet = UNet2DConditionModel(
        sample_size=8,
        in_channels=4,
        out_channels=4,
        layers_per_block=1,
        block_out_channels=(32, 64),
        down_block_types=("DownBlock2D", "CrossAttnDownBlock2D"),
        up_block_types=("CrossAttnUpBlock2D", "UpBlock2D"),
        cross_attention_dim=32,
        attention_head_dim=8,
    )
    unet.requires_grad_(False)
    unet.add_adapter(
        LoraConfig(
            r=args.rango,
            lora_alpha=args.rango,
            init_lora_weights="gaussian",
            target_modules=["to_q", "to_k", "to_v", "to_out.0"],
        )
    )
    entrenables = [p for p in unet.parameters() if p.requires_grad]
    emitir(evento="adaptador", parametros=sum(p.numel() for p in entrenables))
    optimizador = torch.optim.AdamW(entrenables, lr=1e-3)
    latentes = torch.randn(1, 4, 8, 8)
    contexto = torch.randn(1, 12, 32)
    inicio = time.time()
    for paso in range(1, args.pasos + 1):
        ruido = torch.randn_like(latentes)
        t = torch.tensor([paso * 10], dtype=torch.long)
        pred = unet(latentes + ruido * 0.1, t, contexto).sample
        perdida = torch.nn.functional.mse_loss(pred, ruido)
        perdida.backward()
        optimizador.step()
        optimizador.zero_grad()
        emitir(evento="paso", paso=paso, total=args.pasos, perdida=round(perdida.item(), 6),
               seg=round(time.time() - inicio, 2))
    salida = Path(args.salida) / "kukia_prueba.safetensors"
    claves = exportar_lora(unet, salida, args.rango, {"prueba": True, "pasos": args.pasos})
    emitir(evento="guardado", ruta=str(salida), claves=claves)
    emitir(evento="fin", ok=True)


# ---------------------------------------------------------------- real


def entrenar(args):
    import torch
    from diffusers import StableDiffusionPipeline, StableDiffusionXLPipeline, DDPMScheduler
    from peft import LoraConfig

    datos = imagenes_del_dataset(args.dataset)
    if len(datos) < 1:
        error("no hay ninguna imagen con caption en el dataset")

    dispositivo = "cuda" if torch.cuda.is_available() else "cpu"
    tipo = torch.float16 if dispositivo == "cuda" else torch.float32
    emitir(evento="inicio", total=args.pasos, modelo=str(args.modelo),
           imagenes=len(datos), dispositivo=dispositivo)

    modelo = str(args.modelo)
    if Path(modelo).exists() and modelo.lower().endswith((".safetensors", ".ckpt")):
        pipe = StableDiffusionXLPipeline.from_single_file(modelo, torch_dtype=tipo)
        es_sdxl = True
    else:
        pipe = StableDiffusionXLPipeline.from_pretrained(modelo, torch_dtype=tipo)
        es_sdxl = True
    pipe.to(dispositivo)
    pipe.set_progress_bar_config(disable=True)

    vae = pipe.vae.eval()
    unet = pipe.unet
    unet.requires_grad_(False)
    unet.add_adapter(
        LoraConfig(
            r=args.rango,
            lora_alpha=args.rango,
            init_lora_weights="gaussian",
            target_modules=["to_q", "to_k", "to_v", "to_out.0"],
        )
    )
    if hasattr(unet, "enable_gradient_checkpointing"):
        unet.enable_gradient_checkpointing()
    entrenables = [p for p in unet.parameters() if p.requires_grad]
    emitir(evento="adaptador", parametros=sum(p.numel() for p in entrenables))
    optimizador = torch.optim.AdamW(entrenables, lr=args.lr)

    ruido_programa = DDPMScheduler.from_config(pipe.scheduler.config)
    escalado = getattr(vae.config, "scaling_factor", 0.18215)
    factor = 0.13025 if es_sdxl else escalado

    def preparar(ruta, caption):
        img = cargar_imagen(ruta, args.resolucion)
        with torch.no_grad():
            pixel = torch.tensor(list(img.getdata()), dtype=tipo).reshape(
                args.resolucion, args.resolucion, 3
            ).permute(2, 0, 1)[None] / 127.5 - 1.0
            pixel = pixel.to(dispositivo)
            latente = vae.encode(pixel).latent_dist.sample() * factor
            if es_sdxl:
                tok1, tok2 = pipe.tokenizer, pipe.tokenizer_2
                emb = pipe.text_encoder(tok1(caption, padding="max_length", truncation=True,
                                             max_length=tok1.model_max_length, return_tensors="pt"
                                             ).input_ids.to(dispositivo))[0]
                emb2 = pipe.text_encoder_2(tok2(caption, padding="max_length", truncation=True,
                                                max_length=tok2.model_max_length, return_tensors="pt"
                                                ).input_ids.to(dispositivo), output_hidden_states=True)
                prompt = torch.cat([emb, emb2.hidden_states[-2]], dim=-1)
                pooled = emb2[0]
            else:
                tok = pipe.tokenizer
                prompt = pipe.text_encoder(tok(caption, padding="max_length", truncation=True,
                                               max_length=tok.model_max_length, return_tensors="pt"
                                               ).input_ids.to(dispositivo))[0]
                pooled = None
        return latente, prompt, pooled

    cache = [(ruta, caption) for ruta, caption in datos]
    inicio = time.time()
    perdida_media = 0.0
    for paso in range(1, args.pasos + 1):
        ruta, caption = cache[(paso - 1) % len(cache)]
        latente, prompt, pooled = preparar(ruta, caption)
        ruido = torch.randn_like(latente)
        pasos_ruido = torch.randint(0, ruido_programa.config.num_train_timesteps, (1,),
                                    device=dispositivo).long()
        ruidoso = ruido_programa.add_noise(latente, ruido, pasos_ruido)
        extra = {"pooled_projections": pooled} if pooled is not None else {}
        pred = unet(ruidoso, pasos_ruido, prompt, added_cond_kwargs=extra).sample
        perdida = torch.nn.functional.mse_loss(pred.float(), ruido.float())
        perdida.backward()
        torch.nn.utils.clip_grad_norm_(entrenables, 1.0)
        optimizador.step()
        optimizador.zero_grad()
        perdida_media = perdida.item() if paso == 1 else 0.9 * perdida_media + 0.1 * perdida.item()
        if paso % args.cada == 0 or paso == args.pasos:
            emitir(evento="paso", paso=paso, total=args.pasos,
                   perdida=round(perdida.item(), 6), media=round(perdida_media, 6),
                   seg=round(time.time() - inicio, 2))
            if hasattr(torch.cuda, "max_memory_allocated"):
                emitir(evento="vram", mb=round(torch.cuda.max_memory_allocated() / 1048576))

    nombre = args.nombre or Path(args.dataset).name
    salida = Path(args.salida) / f"{nombre}_r{args.rango}_{args.pasos}pasos.safetensors"
    claves = exportar_lora(
        unet, salida, args.rango,
        {"nombre": nombre, "modelo": str(args.modelo), "pasos": args.pasos,
         "rango": args.rango, "resolucion": args.resolucion, "imagenes": len(datos)},
    )
    emitir(evento="guardado", ruta=str(salida), claves=claves)
    emitir(evento="fin", ok=True)


def main():
    p = argparse.ArgumentParser(description="Entrenador de LoRA de KUKIA CODE")
    p.add_argument("--dataset", required=True, help="carpeta con imagenes y sus .txt")
    p.add_argument("--salida", required=True, help="carpeta donde dejar el .safetensors")
    p.add_argument("--modelo", default="", help="ruta al .safetensors o repo de Hugging Face")
    p.add_argument("--pasos", type=int, default=800)
    p.add_argument("--rango", type=int, default=8)
    p.add_argument("--resolucion", type=int, default=1024)
    p.add_argument("--lr", type=float, default=1e-4)
    p.add_argument("--cada", type=int, default=10, help="cada cuantos pasos avisar")
    p.add_argument("--nombre", default="")
    p.add_argument("--probar", action="store_true", help="prueba de humo sin GPU ni descargas")
    args = p.parse_args()

    try:
        if args.probar:
            probar(args)
        else:
            entrenar(args)
    except KeyboardInterrupt:
        emitir(evento="cancelado")
    except Exception as e:  # noqa: BLE001 - el nucleo necesita el motivo tal cual
        error(f"{type(e).__name__}: {e}")


if __name__ == "__main__":
    main()
