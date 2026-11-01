# KUKIA CODE — PLAN DE RELEASE

**Actualizado:** 01/11/2026 · **Autor:** Xoxete

## Estado del artefacto actual

| Campo | Valor |
|---|---|
| Producto | `KUKIA CODE` |
| Versión | `0.1.0` (sin incrementar desde el MVP) |
| Instalador | `KUKIA CODE_0.1.0_x64-setup.exe` · 2.934.300 B |
| SHA-256 | `509646014c93dd858c6f6fededccdb883c813423aaa2e6461bdf910faadf088f` |
| Empaquetado | Tauri 2 · NSIS · `installMode: perMachine` · idiomas ES/EN |
| Bundle | Sin firma de código, sin updater |
| Requisitos en destino | Windows 11 x64 + WebView2 (no requiere Node, Rust, Bun ni Visual Studio) |
| Modelos incluidos | **Ninguno** (se detectan los del sistema: Ollama 11434, LM Studio 1234/1235, ComfyUI 8188) |
| Secretos incluidos | **Ninguno** (verificado: `config.json` sin claves, exe sin credenciales) |

## Procedimiento reproducible de release

```bash
cd /h/KUKIA_CODE
cargo test --release -p kukia-core     # 98 pasan / 12 ignorados / 0 fallos
npm run build                          # tsc --noEmit + vite build
npx tauri build                        # ~3 min → target/release/bundle/nsis/
sha256sum target/release/bundle/nsis/*.exe
```

Instalación (requiere administrador):

```powershell
Start-Process "H:\KUKIA_CODE\target\release\bundle\nsis\KUKIA CODE_0.1.0_x64-setup.exe" -ArgumentList '/S' -Wait
```

## Checklist antes de publicar una versión

- [x] Tests del núcleo en verde
- [x] TypeScript compila sin errores
- [x] Instalador generado y hash calculado
- [x] Backup del binario instalado anterior, con hash idéntico verificado
- [x] La app instalada arranca y siembra políticas (`agent_policies` = 9)
- [x] La ventana pinta (decenas de elementos accesibles, no 1)
- [x] Sin claves ni modelos dentro del instalador
- [ ] Actualización sobre instalación previa conservando `%APPDATA%` (probada a mano, no automatizada)
- [ ] Desinstalación limpia que **no** borre datos del usuario (sin probar)
- [ ] Reparación de instalación (sin probar)
- [ ] Número de build visible en la interfaz (hoy solo «v0.1.0 · Fase 3»)
- [ ] Firma de código y verificación en SmartScreen (sin certificado)
- [ ] Updater firmado con rollback (sin implementar)

## Bloqueos para llamarlo «estable»

1. **Sin firma**: Windows avisará al instalar. Para un release serio hace falta certificado.
2. **Sin updater**: actualizar significa volver a bajar el instalador.
3. **Sin rollback automático**: la vuelta atrás es manual (copia en `backups/`).
4. **Versión congelada en 0.1.0**: con cambios de este tamaño debería ir a 0.2.0 (Fase 3) y
   0.3.0 (cloud). No se ha tocado para no invalidar los hashes documentados.
5. **`perMachine`**: exige administrador; valorar `currentUser` si no hace falta instalación global.

## Qué NO se incluirá nunca en el instalador

- Claves API, contraseñas o tokens.
- Modelos de varios GB.
- Bases de datos personales del usuario.
- Drivers NVIDIA ni CUDA.
- Instalaciones completas de Ollama o LM Studio.
- Componentes de terceros sin licencia compatible.
