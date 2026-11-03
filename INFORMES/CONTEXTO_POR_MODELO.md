# Contexto máximo real por modelo — inventario y plan

Datos leídos de `/api/show` (arquitectura, `*.context_length`, `num_ctx` del
Modelfile) y `/api/tags` (tamaño en disco) de la instalación de Ollama de esta
máquina. `OLLAMA_KV_CACHE_TYPE` vale **q4_0** (variable de usuario), y es el
formato con el que se calcula la caché KV.

Presupuesto de VRAM usado para el máximo viable: **26.3 GB** (27.8 GB de las dos tarjetas menos 1.5 GB de margen).
No se suma la VRAM como si fuera memoria única: el presupuesto es el total.

## Resumen

- **CONFIGURADO AL MÁXIMO**: 4 modelo(s)
- **CONTEXTO AMPLIABLE**: 53 modelo(s)
- **LIMITADO POR HARDWARE**: 25 modelo(s)
- **NO VERIFICADO**: 0 modelo(s)

## Tabla completa

| Modelo | Proveedor | Nativo | Viables en este equipo | Configurado | Kv q4 al nativo | Clase |
|---|---|---:|---:|---:|---:|---|
| `xoxete-qwen25-32b:latest` | ollama | 32,768 | 32,768 | 32,768 | 2.25 GB | CONFIGURADO AL MÁXIMO |
| `qwen2.5-32b-ablit:latest` | ollama | 32,768 | 32,768 | 32,768 | 2.25 GB | CONFIGURADO AL MÁXIMO |
| `xoxete-mistral-7b:latest` | ollama | 32,768 | 32,768 | 32,768 | 1.12 GB | CONFIGURADO AL MÁXIMO |
| `llama3-ablit:latest` | ollama | 8,192 | 8,192 | 8,192 | 0.28 GB | CONFIGURADO AL MÁXIMO |
| `brianmatzelle/qwen3-coder-heretic:30b` | ollama | 262,144 | 262,144 | — | 3.38 GB | CONTEXTO AMPLIABLE |
| `qwen3-vl:4b` | ollama | 262,144 | 262,144 | — | 6.33 GB | CONTEXTO AMPLIABLE |
| `saluki-mtp-ablit:latest` | ollama | 262,144 | 262,144 | — | 15.23 GB | CONTEXTO AMPLIABLE |
| `cyber-tier-coder-hack:latest` | ollama | 262,144 | 262,144 | — | 3.38 GB | CONTEXTO AMPLIABLE |
| `cyber-tier-coder:latest` | ollama | 262,144 | 262,144 | — | 3.38 GB | CONTEXTO AMPLIABLE |
| `huihui_ai/qwen3-vl-abliterated:8b` | ollama | 262,144 | 262,144 | — | 10.12 GB | CONTEXTO AMPLIABLE |
| `qwen3-coder-ablit:latest` | ollama | 262,144 | 262,144 | 8,192 | 3.38 GB | CONTEXTO AMPLIABLE |
| `qwen3-vl-ablit:latest` | ollama | 262,144 | 262,144 | 8,192 | 6.33 GB | CONTEXTO AMPLIABLE |
| `kuki-hacker-ravenx-ablit:latest` | ollama | 262,144 | 262,144 | 8,192 | 2.88 GB | CONTEXTO AMPLIABLE |
| `kuki-hacker-endy-ablit:latest` | ollama | 262,144 | 262,144 | 8,192 | 2.88 GB | CONTEXTO AMPLIABLE |
| `kuki-hacker-ravenx:latest` | ollama | 262,144 | 262,144 | — | 2.88 GB | CONTEXTO AMPLIABLE |
| `kuki-hacker-endy:latest` | ollama | 262,144 | 262,144 | 32,768 | 2.88 GB | CONTEXTO AMPLIABLE |
| `endy-cybersec:latest` | ollama | 262,144 | 262,144 | 32,768 | 2.88 GB | CONTEXTO AMPLIABLE |
| `qwen3-coder:30b-a3b-q5_k_m` | ollama | 262,144 | 262,144 | — | 3.38 GB | CONTEXTO AMPLIABLE |
| `ravenx-tuned:latest` | ollama | 262,144 | 262,144 | 32,768 | 2.88 GB | CONTEXTO AMPLIABLE |
| `jev-9b:latest` | ollama | 262,144 | 262,144 | 32,768 | 9.0 GB | CONTEXTO AMPLIABLE |
| `ravenx-cyberagent:latest` | ollama | 262,144 | 262,144 | — | 2.88 GB | CONTEXTO AMPLIABLE |
| `xoxete-ravenx:latest` | ollama | 262,144 | 262,144 | — | 2.88 GB | CONTEXTO AMPLIABLE |
| `brianmatzelle/qwen3-coder-heretic:30b-normal` | ollama | 262,144 | 262,144 | — | 3.38 GB | CONTEXTO AMPLIABLE |
| `xoxete-glm47-heretic:latest` | ollama | 202,752 | 202,752 | 32,768 | 1.02 GB | CONTEXTO AMPLIABLE |
| `gpt-oss-20b-uncensored:latest` | ollama | 131,072 | 131,072 | 32,768 | 1.19 GB | CONTEXTO AMPLIABLE |
| `muse-glimmer-ablit:latest` | ollama | 131,072 | 131,072 | — | 2.97 GB | CONTEXTO AMPLIABLE |
| `glimmer-30b-ablit:latest` | ollama | 131,072 | 131,072 | — | 2.97 GB | CONTEXTO AMPLIABLE |
| `xoxete-glimmer:latest` | ollama | 131,072 | 131,072 | 32,768 | 2.97 GB | CONTEXTO AMPLIABLE |
| `glimmer-30b:latest` | ollama | 131,072 | 131,072 | 32,768 | 2.97 GB | CONTEXTO AMPLIABLE |
| `ia-mia:latest` | ollama | 40,960 | 40,960 | 16,384 | 1.76 GB | CONTEXTO AMPLIABLE |
| `qwen3-14b-abliterated:latest-normal` | ollama | 40,960 | 40,960 | 32,768 | 1.76 GB | CONTEXTO AMPLIABLE |
| `xoxete-qwen3-14b-abliterated-latest:latest-normal` | ollama | 40,960 | 40,960 | 32,768 | 1.76 GB | CONTEXTO AMPLIABLE |
| `xoxete-qwen3-14b-abliterated-latest:latest` | ollama | 40,960 | 40,960 | 32,768 | 1.76 GB | CONTEXTO AMPLIABLE |
| `xoxete-qwen14b:latest` | ollama | 40,960 | 40,960 | 32,768 | 1.76 GB | CONTEXTO AMPLIABLE |
| `xoxete-14b:latest-normal` | ollama | 40,960 | 40,960 | 32,768 | 1.76 GB | CONTEXTO AMPLIABLE |
| `xoxete-14b:latest` | ollama | 40,960 | 40,960 | 32,768 | 1.76 GB | CONTEXTO AMPLIABLE |
| `qwen3-14b-abliterated:latest` | ollama | 40,960 | 40,960 | 32,768 | 1.76 GB | CONTEXTO AMPLIABLE |
| `qwen3-32b-heretic:latest` | ollama | 40,960 | 40,960 | — | 1.76 GB | CONTEXTO AMPLIABLE |
| `xoxete-qwen32:latest` | ollama | 40,960 | 40,960 | — | 1.76 GB | CONTEXTO AMPLIABLE |
| `llava:latest` | ollama | 32,768 | 32,768 | — | 1.12 GB | CONTEXTO AMPLIABLE |
| `bakllava:latest` | ollama | 32,768 | 32,768 | 4,096 | 1.12 GB | CONTEXTO AMPLIABLE |
| `bakllava-ablit:latest` | ollama | 32,768 | 32,768 | 8,192 | 1.12 GB | CONTEXTO AMPLIABLE |
| `llava-ablit:latest` | ollama | 32,768 | 32,768 | 8,192 | 1.12 GB | CONTEXTO AMPLIABLE |
| `xoxete-mistral:latest-normal` | ollama | 32,768 | 32,768 | — | 1.12 GB | CONTEXTO AMPLIABLE |
| `dolphin-mistral:latest-normal` | ollama | 32,768 | 32,768 | — | 1.12 GB | CONTEXTO AMPLIABLE |
| `dolphin-mistral:latest` | ollama | 32,768 | 32,768 | — | 1.12 GB | CONTEXTO AMPLIABLE |
| `xoxete-mistral:latest` | ollama | 32,768 | 32,768 | — | 1.12 GB | CONTEXTO AMPLIABLE |
| `xoxete-wrn33b:latest` | ollama | 16,384 | 16,384 | — | 1.09 GB | CONTEXTO AMPLIABLE |
| `deepseek-coder:33b` | ollama | 16,384 | 16,384 | — | 1.09 GB | CONTEXTO AMPLIABLE |
| `deepseek-coder-ablit:latest` | ollama | 16,384 | 16,384 | 8,192 | 1.09 GB | CONTEXTO AMPLIABLE |
| `llama3:latest` | ollama | 8,192 | 8,192 | — | 0.28 GB | CONTEXTO AMPLIABLE |
| `prompt-forge:latest` | ollama | 8,192 | 8,192 | — | 0.65 GB | CONTEXTO AMPLIABLE |
| `dirty-muse-writer:latest` | ollama | 8,192 | 8,192 | — | 0.65 GB | CONTEXTO AMPLIABLE |
| `xoxete:latest` | ollama | 8,192 | 8,192 | — | 0.28 GB | CONTEXTO AMPLIABLE |
| `xoxete:latest-normal` | ollama | 8,192 | 8,192 | — | 0.28 GB | CONTEXTO AMPLIABLE |
| `llama3:latest-normal` | ollama | 8,192 | 8,192 | — | 0.28 GB | CONTEXTO AMPLIABLE |
| `llava:34b` | ollama | 4,096 | 4,096 | — | 0.26 GB | CONTEXTO AMPLIABLE |
| `aratan/Qwythos-9B-v2-1M-Uncensored-GGUF:Q4_K_M-normal` | ollama | 1,048,576 | 563,200 | 32,768 | 37.12 GB | LIMITADO POR HARDWARE |
| `aratan/Qwythos-9B-v2-1M-Uncensored-GGUF:Q4_K_M` | ollama | 1,048,576 | 563,200 | 32,768 | 37.12 GB | LIMITADO POR HARDWARE |
| `flint-local:latest` | ollama | 262,144 | 225,280 | — | 3.45 GB | LIMITADO POR HARDWARE |
| `kuki-hacker-cyber-ablit:latest` | ollama | 262,144 | 182,272 | 8,192 | 15.23 GB | LIMITADO POR HARDWARE |
| `xoxete-qwen3.8-27b-uncensored-latest:latest-normal` | ollama | 262,144 | 182,272 | 32,768 | 15.23 GB | LIMITADO POR HARDWARE |
| `xoxete-qwen3.8-27b-uncensored-latest:latest` | ollama | 262,144 | 182,272 | 32,768 | 15.23 GB | LIMITADO POR HARDWARE |
| `xoxete-qwen27b:latest` | ollama | 262,144 | 182,272 | 32,768 | 15.23 GB | LIMITADO POR HARDWARE |
| `qwen3.8-27b-uncensored:latest-normal` | ollama | 262,144 | 182,272 | — | 15.23 GB | LIMITADO POR HARDWARE |
| `kuki-hacker-cyber:latest` | ollama | 262,144 | 182,272 | 32,768 | 15.23 GB | LIMITADO POR HARDWARE |
| `cyber-preview:latest` | ollama | 262,144 | 182,272 | 32,768 | 15.23 GB | LIMITADO POR HARDWARE |
| `qwen3.8-27b-uncensored:latest` | ollama | 262,144 | 182,272 | 131,072 | 15.23 GB | LIMITADO POR HARDWARE |
| `huihui_ai/qwen3-vl-abliterated:32b` | ollama | 262,144 | 157,696 | — | 11.25 GB | LIMITADO POR HARDWARE |
| `glm-4.6v-jose-1m:latest` | ollama | 131,072 | 131,072 | 1,048,576 | 1.41 GB | LIMITADO POR HARDWARE |
| `qwen2.5-vl-32b-ablit:latest` | ollama | 128,000 | 126,976 | — | 8.79 GB | LIMITADO POR HARDWARE |
| `xoxete-vl32b:latest` | ollama | 128,000 | 126,976 | — | 8.79 GB | LIMITADO POR HARDWARE |
| `supergemma-unc:latest` | ollama | 262,144 | 118,784 | 32,768 | 23.2 GB | LIMITADO POR HARDWARE |
| `gemma31-unc:latest` | ollama | 262,144 | 24,576 | 32,768 | 88.59 GB | LIMITADO POR HARDWARE |
| `nemotron-bolt-ablit:latest` | ollama | 262,144 | 14,336 | 8,192 | 99.0 GB | LIMITADO POR HARDWARE |
| `xoxete-nemo-q5:latest` | ollama | 1,048,576 | 12,288 | 32,768 | 153.56 GB | LIMITADO POR HARDWARE |
| `nemotron-q5-heretic:latest` | ollama | 1,048,576 | 12,288 | 32,768 | 153.56 GB | LIMITADO POR HARDWARE |
| `xoxete-llama3-8b:latest` | ollama | 8,192 | 8,192 | 32,768 | 0.28 GB | LIMITADO POR HARDWARE |
| `llava13b-ablit:latest` | ollama | 4,096 | 4,096 | 8,192 | 0.88 GB | LIMITADO POR HARDWARE |
| `moondream-ablit:latest` | ollama | 2,048 | 2,048 | 8,192 | 0.11 GB | LIMITADO POR HARDWARE |
| `xoxete-nemo:latest` | ollama | 1,048,576 | — | 32,768 | 153.56 GB | LIMITADO POR HARDWARE |
| `nemo-heretic:latest` | ollama | 1,048,576 | — | 32,768 | 153.56 GB | LIMITADO POR HARDWARE |

## Anomalías encontradas

Modelos cuyo `num_ctx` del Modelfile **supera** el contexto nativo declarado por
su arquitectura. Ahí el motor atiende más tokens de los que el modelo fue
entrenado para manejar (RoPE fuera de rango): puede responder, pero degrada.

- `glm-4.6v-jose-1m:latest`: nativo 131,072 → Modelfile 1,048,576
- `xoxete-llama3-8b:latest`: nativo 8,192 → Modelfile 32,768
- `llava13b-ablit:latest`: nativo 4,096 → Modelfile 8,192
- `moondream-ablit:latest`: nativo 2,048 → Modelfile 8,192
