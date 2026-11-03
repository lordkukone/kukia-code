"""Monta el árbol de G:\\RTX_LAB para los proyectos aprobados (clase A del informe).

Qué hace y qué NO hace:
  - Crea las carpetas del diseño aprobado y, por proyecto, README + `origen.json`
    (rutas, tamaños y SHA-256 de los paquetes) + `uninstall.ps1`.
  - Escribe el manifiesto global (`MANIFIESTO.md` y `MANIFIESTO.json`), un
    verificador de hashes y un desinstalador global.
  - **No extrae ningún ZIP, no ejecuta ningún binario y no toca `G:\\descargas`.**
    La extracción y la prueba `nrcheck` siguen pendientes de autorización.

Uso:
    python tools/montar_rtx_lab.py            # crea lo que falte (idempotente)
    python tools/montar_rtx_lab.py --dry-run  # enseña lo que haría
"""

import argparse
import hashlib
import json
import os
from datetime import datetime, timezone

RAIZ = r"G:\RTX_LAB"
DESCARGAS = r"G:\descargas"

# Proyectos aprobados: clase A del informe RTX_DUAL_GPU_LAB_AUDITORIA.md.
# Cada uno con sus paquetes de origen reales (se comprueban antes de crear nada).
PROYECTOS = [
    {
        "carpeta": "01_TELEMETRIA",
        "titulo": "Telemetría (solo lectura)",
        "riesgo": "ninguno",
        "que": "GPU-Z 2.71 para leer sensores por tarjeta. PresentMon viene dentro "
               "del ZIP de Neural-coprocessor y se usará para medir latencia.",
        "faltan": "Extraer GPU-Z y PresentMon aquí. HWiNFO no aparece en las rutas "
                  "revisadas: si lo tienes en otra carpeta, se añade.",
        "archivos": [
            os.path.join(DESCARGAS, "GPU-Z.2.71.0.exe"),
            os.path.join(DESCARGAS, "GPU-Z-[Guru3D].zip"),
        ],
        "prueba": "Abrir GPU-Z y anotar VRAM, temperatura y vatios de cada tarjeta "
                  "antes de cualquier prueba.",
    },
    {
        "carpeta": "02_NRCHECK",
        "titulo": "nrcheck — ¿arranca DLSS-NR en la 5060 Ti?",
        "riesgo": "bajo (no instala nada)",
        "que": "Prueba de capacidad sin abrir ningún juego. `nrcheck.exe` dice si el "
               "Neural Rendering arranca en cada adaptador y `--bench` mide cuánto "
               "aguanta a 1440p.",
        "faltan": "AUTORIZACIÓN para extraer `nrcheck.exe`, `nvngx.dll_nrcheck.dll` "
                  "y `nvngx_dlssnr.dll` (del pack 310.8.0) y ejecutarlos.",
        "archivos": [
            os.path.join(DESCARGAS, "mgpu_bridge_launcher-0.4.0.zip"),
            os.path.join(DESCARGAS, "DLSS5-NR-v0.8.0.fxpkg"),
            os.path.join(DESCARGAS, "DLSS5-NR-SHA256SUMS.txt"),
            os.path.join(DESCARGAS, "DLSS 310.8.0 streamline 2.13 dll.zip"),
        ],
        "prueba": "1) .\\nrcheck.exe  → 2) .\\nrcheck.exe --bench --neural 1 --other 0 "
                  "--res 2560x1440 --passes 1 --frames 300",
    },
    {
        "carpeta": "03_MGPU_BRIDGE",
        "titulo": "MGPU Bridge (juego en una GPU, NR en la otra)",
        "riesgo": "medio",
        "que": "Add-on de ReShade que manda la etapa neuronal a la segunda tarjeta. "
               "Necesita ReShade 6.8.0 o superior con soporte de add-ons.",
        "faltan": "Extraer el add-on 0.4.0 + `mgpu.ini` + `mgpu_depth_tap.fx`. "
                  "ReShade NO está descargado: hay que pedirlo aparte.",
        "archivos": [
            os.path.join(DESCARGAS, "mgpu_bridge-0.4.0.zip"),
            os.path.join(DESCARGAS, "Neural-coprocessor-main ultimo.zip"),
            os.path.join(DESCARGAS, "mgpu_bridge.1.10.zip"),
        ],
        "prueba": "En una copia de juego: ReShade.log debe decir «Registered add-on "
                  "MGPU Bridge» y «SELECTED adapter» apuntando a la 5060 Ti.",
    },
    {
        "carpeta": "04_FOVEATED",
        "titulo": "Foveated DLSS5 (una sola GPU)",
        "riesgo": "bajo",
        "que": "DLSS-NR con foveación en los bordes, en una tarjeta. Alternativa si "
               "la doble GPU no rinde.",
        "faltan": "Extraer `renodx-dlss5.addon64` y probar la versión Standalone y la UEVR.",
        "archivos": [
            os.path.join(DESCARGAS, "FoveatedDLSS5", "renodx-dlss5.addon64"),
            os.path.join(DESCARGAS, "FoveatedDLSS5", "renodx-dlss5.addon64.sha256"),
            os.path.join(DESCARGAS, "CheekyFoveatedDLSS", "CheekyFoveatedDLSS-0.6.0-Standalone.zip"),
            os.path.join(DESCARGAS, "CheekyFoveatedDLSS", "CheekyFoveatedDLSS-0.6.0-UEVR.zip"),
        ],
        "prueba": "Comparar imagen y fps con NR de NVIDIA sin fovear.",
    },
    {
        "carpeta": "05_OPTISCALER",
        "titulo": "OptiScaler (aislado)",
        "riesgo": "medio",
        "que": "Sustituye el escalador del juego (DLSS/FSR/XeSS). La release "
               "DLSSNR v0.1.2 trae las DLLs ya compiladas.",
        "faltan": "Extraer la release en esta carpeta y probarla solo sobre copias.",
        "archivos": [
            os.path.join(DESCARGAS, "OptiScaler-DLSSNR-v0.1.2.zip"),
            os.path.join(DESCARGAS, "OptiScaler-master.zip"),
        ],
        "prueba": "Anotar versión de DLL original y resultado en 90_LOGS.",
    },
    {
        "carpeta": "06_SWAPPER",
        "titulo": "DLSS Swapper (portable, no instalador)",
        "riesgo": "bajo",
        "que": "Cambia la versión de `nvngx_dlss.dll` por juego. Se usa el portable "
               "para no instalar nada en el sistema.",
        "faltan": "Descomprimir el portable. El instalador 2.2.9 queda en el "
                  "manifiesto pero NO se ejecuta.",
        "archivos": [
            os.path.join(DESCARGAS, "DLSS.Swapper-1.2.7.0-portable.zip"),
            os.path.join(DESCARGAS, "DLSS5-Swapper-Setup-2.2.9.exe"),
        ],
        "prueba": "Antes de cambiar una DLL: copia en 99_BACKUPS.",
    },
    {
        "carpeta": "07_RTXNTC",
        "titulo": "Neural Texture Compression (demo compilada)",
        "riesgo": "bajo",
        "que": "`ntc-cli`, `ntc-explorer` y `ntc-renderer` para ver compresión "
               "neuronal de texturas en tus tarjetas.",
        "faltan": "Extraer y ejecutar la demo; no toca juegos.",
        "archivos": [
            os.path.join(DESCARGAS, "RTXNTC_windows_x64_v0.10.0b.zip"),
            os.path.join(DESCARGAS, "RTXNTC-main.zip"),
        ],
        "prueba": "Abrir ntc-explorer y comparar textura original vs NTC.",
    },
    {
        "carpeta": "08_RIFE",
        "titulo": "RIFE (interpolación de vídeo)",
        "riesgo": "bajo",
        "que": "Interpola fotogramas de vídeo ya grabado en la 5060 Ti. Es el reparto "
               "real de trabajo entre GPUs para vídeo, no para jugar.",
        "faltan": "Extraer y probar con un clip corto.",
        "archivos": [
            os.path.join(DESCARGAS, "rife-ncnn-vulkan-20221029-windows.zip"),
        ],
        "prueba": "Medir fps de interpolación y VRAM usada por la segunda tarjeta.",
    },
]

NO_APROBADOS = [
    ("laurensanp/siris", "no descargado"),
    ("Kizzuwatnaa/DLSS5-Autopilot", "exe presente, mezcla A/B: requiere decisión"),
    ("Abhito/Glasir", "no descargado"),
    ("GTA-5-LEGACY Intel XeSS/DLSS/FG", "no descargado"),
    ("teemupynnonen/mgsv-phantom-fx", "no descargado"),
    ("Xchen991/RTX30-Broadcast-FrameGen", "no descargado"),
    ("Kaldaien/SpecialK debug_sessions", "no descargado"),
    ("SpartanEngine, three-gpu-pathtracer, Path-Tracer, XRay-Render, rayzee-renderer, "
     "PATHTRACER, Real-Time-Path-Tracer, DXRDemo, vk_gltf_renderer, vk-rt-cuda-raytracing",
     "motores de render: no descargados (investigación)"),
    ("Ablation neural upscaling, ReShadeUpscaler, video-upscaler-with-dlss5, ternoise",
     "no descargados"),
    ("NVIDIA/Model-Optimizer 0.47.0", "solo fuente (clase B): exige compilación"),
    ("NVIDIAGameWorks/Streamline 2.14.16", "solo hay DLLs de 2.13: versión distinta"),
    ("NVIDIAGameWorks/rtx-remix 1.6.1", "no descargado"),
    ("harpjo60-lgtm/rdr2-mac-framegen", "no descargado"),
    ("OpticalFlowSDK_5.0.zip / OpticalFlowSDK.exe", "paquetes ROTOS (HTML de login / 0 bytes)"),
    ("dlss5-bridge-v1.4.13-pre6.zip", "ROTO: 9 bytes con el texto «Not Found»"),
    ("DDU (6 copias)", "no se usa: cambiar driver está prohibido en el encargo"),
]

UNINSTALL = r"""param([switch]$Confirmar)
# Desinstalador de {carpeta}. No toca G:\descargas ni nada fuera de G:\RTX_LAB.
$ErrorActionPreference = 'Stop'
$raiz = '{raiz}'
$destino = Join-Path $raiz '{carpeta}'

if (-not (Test-Path -LiteralPath $destino)) {{
    Write-Output "Nada que desinstalar: $destino no existe."
    exit 0
}}

$abs = (Resolve-Path -LiteralPath $destino).Path
if (-not $abs.StartsWith($raiz, [System.StringComparison]::OrdinalIgnoreCase)) {{
    throw "Ruta fuera del laboratorio, no se borra: $abs"
}}

$tam = 0
Get-ChildItem -LiteralPath $abs -Recurse -File -ErrorAction SilentlyContinue |
    ForEach-Object {{ $tam += $_.Length }}
$mb = [math]::Round($tam / 1MB, 2)

if (-not $Confirmar) {{
    Write-Output "Se borraría: $abs ($mb MB)"
    Write-Output "Los paquetes originales de G:\descargas NO se tocan."
    Write-Output "Para ejecutarlo: .\uninstall.ps1 -Confirmar"
    exit 0
}}

Remove-Item -LiteralPath $abs -Recurse -Force
$log = Join-Path $raiz '90_LOGS\desinstalaciones.log'
New-Item -ItemType Directory -Force -Path (Split-Path $log) | Out-Null
Add-Content -LiteralPath $log -Value ("{{0}}  desinstalado  {{1}}  ({{2}} MB)" -f (Get-Date -Format o), $abs, $mb)
Write-Output "Desinstalado: $abs ($mb MB)"
"""


def sha256(ruta):
    h = hashlib.sha256()
    with open(ruta, "rb") as fh:
        for bloque in iter(lambda: fh.read(4 * 1024 * 1024), b""):
            h.update(bloque)
    return h.hexdigest()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--dry-run", action="store_true")
    args = ap.parse_args()

    # 1. Comprobar que existen todos los paquetes antes de crear nada.
    faltan = []
    for p in PROYECTOS:
        for ruta in p["archivos"]:
            if not os.path.exists(ruta):
                faltan.append(ruta)
    if faltan:
        print("FALTAN PAQUETES, no se crea el árbol:")
        for f in faltan:
            print("  -", f)
        return 1

    if args.dry_run:
        print("Se crearían:", RAIZ)
        for p in PROYECTOS:
            print(f"  {p['carpeta']:18} {len(p['archivos'])} paquete(s)")
        return 0

    # 2. Carpetas base.
    for carpeta in ["90_LOGS", "99_BACKUPS", "09_BANCO_JUEGOS"]:
        os.makedirs(os.path.join(RAIZ, carpeta), exist_ok=True)

    manifiesto = {
        "generado": datetime.now(tz=timezone.utc).isoformat(),
        "raiz": RAIZ,
        "origen_paquetes": DESCARGAS,
        "extraido": False,
        "ejecutado": False,
        "nota": "Ningún paquete se ha extraído ni ejecutado. Solo se ha creado la "
                "estructura y se han calculado hashes de los originales.",
        "proyectos": [],
    }

    print("calculando hashes (puede tardar unos segundos)…")
    for p in PROYECTOS:
        destino = os.path.join(RAIZ, p["carpeta"])
        os.makedirs(os.path.join(destino, "datos"), exist_ok=True)
        os.makedirs(os.path.join(destino, "logs"), exist_ok=True)

        archivos = []
        for ruta in p["archivos"]:
            archivos.append(
                {
                    "ruta": ruta,
                    "nombre": os.path.basename(ruta),
                    "bytes": os.path.getsize(ruta),
                    "sha256": sha256(ruta),
                }
            )
            print(f"  {os.path.basename(ruta)[:56]:56} {archivos[-1]['sha256'][:16]}…")

        origen = {
            "carpeta": p["carpeta"],
            "titulo": p["titulo"],
            "riesgo": p["riesgo"],
            "estado": "estructura creada; paquetes sin extraer",
            "archivos": archivos,
        }
        with open(os.path.join(destino, "origen.json"), "w", encoding="utf-8") as fh:
            json.dump(origen, fh, ensure_ascii=False, indent=2)

        with open(os.path.join(destino, "uninstall.ps1"), "w", encoding="utf-8") as fh:
            fh.write(UNINSTALL.format(carpeta=p["carpeta"], raiz=RAIZ))

        readme = [
            f"# {p['titulo']}",
            "",
            f"- **Carpeta:** `{destino}`",
            f"- **Riesgo:** {p['riesgo']}",
            f"- **Estado:** estructura creada. **Nada extraído ni ejecutado.**",
            "",
            "## Qué es",
            p["que"],
            "",
            "## Qué falta para poder probarlo",
            p["faltan"],
            "",
            "## Paquetes de origen (sin mover)",
            "",
            "| Archivo | Tamaño | SHA-256 |",
            "|---|---:|---|",
        ]
        for a in archivos:
            readme.append(
                f"| `{a['nombre']}` | {a['bytes'] / 1024**2:.1f} MB | `{a['sha256']}` |"
            )
        readme += [
            "",
            "## Cómo se prueba",
            p["prueba"],
            "",
            "## Cómo se desinstala",
            "```powershell",
            ".\\uninstall.ps1            # enseña qué borraría",
            ".\\uninstall.ps1 -Confirmar # lo borra",
            "```",
            "Solo borra esta carpeta. Los originales de `G:\\descargas` no se tocan.",
            "",
        ]
        with open(os.path.join(destino, "README.md"), "w", encoding="utf-8") as fh:
            fh.write("\n".join(readme))

        manifiesto["proyectos"].append(
            {
                "carpeta": p["carpeta"],
                "titulo": p["titulo"],
                "riesgo": p["riesgo"],
                "estado": "estructura creada; sin extraer",
                "archivos": archivos,
            }
        )

    # 3. Manifiesto global.
    with open(os.path.join(RAIZ, "MANIFIESTO.json"), "w", encoding="utf-8") as fh:
        json.dump(manifiesto, fh, ensure_ascii=False, indent=2)

    total_bytes = sum(a["bytes"] for p in manifiesto["proyectos"] for a in p["archivos"])
    md = [
        "# RTX LAB — manifiesto",
        "",
        f"Generado: `{manifiesto['generado']}`",
        f"Raíz: `{RAIZ}` · Origen de los paquetes: `{DESCARGAS}` (no se mueve nada)",
        f"Paquetes registrados: **{sum(len(p['archivos']) for p in manifiesto['proyectos'])}** · "
        f"**{total_bytes / 1024**3:.2f} GB** (contando duplicados por nombre)",
        "",
        "**Estado:** estructura y hashes hechos. Ningún ZIP extraído, ningún binario ejecutado.",
        "",
        "| Carpeta | Proyecto | Riesgo | Paquetes | Estado |",
        "|---|---|---|---:|---|",
    ]
    for p in manifiesto["proyectos"]:
        md.append(
            f"| `{p['carpeta']}` | {p['titulo']} | {p['riesgo']} | {len(p['archivos'])} | sin extraer |"
        )
    md += [
        "",
        "## Hashes",
        "",
        "| Archivo | Bytes | SHA-256 |",
        "|---|---:|---|",
    ]
    for p in manifiesto["proyectos"]:
        for a in p["archivos"]:
            md.append(f"| `{a['nombre']}` | {a['bytes']} | `{a['sha256']}` |")
    md += [
        "",
        "## Fuera del laboratorio (no aprobados)",
        "",
        "| Proyecto | Motivo |",
        "|---|---|",
    ]
    for nombre, motivo in NO_APROBADOS:
        md.append(f"| {nombre} | {motivo} |")
    with open(os.path.join(RAIZ, "MANIFIESTO.md"), "w", encoding="utf-8") as fh:
        fh.write("\n".join(md) + "\n")

    # 4. Verificador de hashes.
    verificador = [
        "# Comprueba que los paquetes de origen siguen intactos (solo lectura).",
        "$ErrorActionPreference = 'Stop'",
        f"$manifiesto = Get-Content -Raw '{os.path.join(RAIZ, 'MANIFIESTO.json')}' | ConvertFrom-Json",
        "$malos = 0",
        "foreach ($p in $manifiesto.proyectos) {",
        "  foreach ($a in $p.archivos) {",
        "    if (-not (Test-Path -LiteralPath $a.ruta)) { Write-Output ('FALTA  ' + $a.ruta); $malos++; continue }",
        "    $h = (Get-FileHash -Algorithm SHA256 -LiteralPath $a.ruta).Hash.ToLower()",
        "    if ($h -eq $a.sha256.ToLower()) { Write-Output ('OK     ' + $a.nombre) }",
        "    else { Write-Output ('CAMBIADO ' + $a.nombre + ' -> ' + $h); $malos++ }",
        "  }",
        "}",
        "if ($malos -eq 0) { Write-Output 'Todos los paquetes coinciden con el manifiesto.' }",
        "else { Write-Output \"$malos problema(s) encontrados.\"; exit 1 }",
    ]
    with open(os.path.join(RAIZ, "VERIFICAR_HASHES.ps1"), "w", encoding="utf-8") as fh:
        fh.write("\n".join(verificador) + "\n")

    # 5. Desinstalador global.
    global_un = [
        "param([switch]$Confirmar)",
        "# Desinstala el laboratorio entero, carpeta por carpeta, sin tocar G:\\descargas.",
        "$ErrorActionPreference = 'Stop'",
        f"$raiz = '{RAIZ}'",
        "if (-not $Confirmar) {",
        "  Write-Output 'Esto borraría todas las carpetas de proyecto de G:\\RTX_LAB.'",
        "  Write-Output 'Los logs (90_LOGS) y las copias (99_BACKUPS) se conservan.'",
        "  Write-Output 'Para ejecutarlo: .\\DESINSTALAR_TODO.ps1 -Confirmar'",
        "  exit 0",
        "}",
        "Get-ChildItem -LiteralPath $raiz -Directory | Where-Object { $_.Name -match '^[0-9]{2}_' -and $_.Name -notin @('90_LOGS','99_BACKUPS') } | ForEach-Object {",
        "  $u = Join-Path $_.FullName 'uninstall.ps1'",
        "  if (Test-Path -LiteralPath $u) { & $u -Confirmar } else { Write-Output \"sin desinstalador: $($_.Name)\" }",
        "}",
        "Write-Output 'Laboratorio desinstalado. Nada de G:\\descargas se ha tocado.'",
    ]
    with open(os.path.join(RAIZ, "DESINSTALAR_TODO.ps1"), "w", encoding="utf-8") as fh:
        fh.write("\n".join(global_un) + "\n")

    # 6. README del banco de juegos y raíz.
    with open(os.path.join(RAIZ, "09_BANCO_JUEGOS", "README.md"), "w", encoding="utf-8") as fh:
        fh.write(
            "# Banco de juegos (vacío a propósito)\n\n"
            "Aquí va **una copia** de un juego para inyectar ReShade/MGPU, nunca el original "
            "instalado y nunca un juego online con anticheat. Recomendados por el informe: "
            "*They Are Billions* o *WorldBox* (DX11, ligeros, fáciles de revertir).\n\n"
            "No se ha copiado ningún juego: hace falta autorización.\n"
        )

    with open(os.path.join(RAIZ, "README.md"), "w", encoding="utf-8") as fh:
        fh.write(
            "# G:\\RTX_LAB — laboratorio de doble GPU\n\n"
            "Árbol creado a partir de `INFORMES/RTX_DUAL_GPU_LAB_AUDITORIA.md`.\n\n"
            "**Estado ahora mismo: solo estructura, manifiesto y hashes. Nada extraído "
            "ni ejecutado.**\n\n"
            "## Reglas\n\n"
            "- Cada carpeta es autónoma: su `datos\\`, sus `logs\\`, su `uninstall.ps1`.\n"
            "- Los paquetes originales viven en `G:\\descargas` y **no se mueven**.\n"
            "- Nunca se escribe en la carpeta de un juego instalado, ni en juegos online.\n"
            "- Cada prueba deja su informe en `90_LOGS`.\n\n"
            "## Puntos de entrada\n\n"
            "- `MANIFIESTO.md` / `MANIFIESTO.json`: qué hay, de dónde salió y su hash.\n"
            "- `VERIFICAR_HASHES.ps1`: comprueba que los originales siguen intactos.\n"
            "- `DESINSTALAR_TODO.ps1`: borra el laboratorio (pide `-Confirmar`).\n\n"
            "## Orden de pruebas\n\n"
            "1. `01_TELEMETRIA` (riesgo cero).\n"
            "2. `02_NRCHECK`: la prueba que decide si el proyecto tiene sentido.\n"
            "3. `03_MGPU_BRIDGE` sobre una copia de juego.\n"
            "4. `04_FOVEATED`, `05_OPTISCALER`, `06_SWAPPER`, `07_RTXNTC`, `08_RIFE`.\n\n"
            "**Pendiente de autorización:** extraer los paquetes y ejecutar `nrcheck`.\n"
        )

    print()
    print("árbol creado en", RAIZ)
    print("proyectos:", len(manifiesto["proyectos"]))
    print("paquetes registrados:", sum(len(p["archivos"]) for p in manifiesto["proyectos"]))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
