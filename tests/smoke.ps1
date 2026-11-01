# KUKIA CODE — smoke test de entrega
# Comprueba artefactos reales sin modificar nada.

$ErrorActionPreference = "Stop"
$root = "H:\KUKIA_CODE"
$fails = 0

function Check($name, $condition) {
    if ($condition) { Write-Host "[OK]   $name" -ForegroundColor Green }
    else { Write-Host "[FAIL] $name" -ForegroundColor Red; $script:fails++ }
}

Check "package.json existe" (Test-Path "$root\package.json")
Check "dist/index.html generado" (Test-Path "$root\dist\index.html")
Check "core/src/lib.rs existe" (Test-Path "$root\core\src\lib.rs")
Check "src-tauri/src/lib.rs existe" (Test-Path "$root\src-tauri\src\lib.rs")
Check "tauri.conf.json existe" (Test-Path "$root\src-tauri\tauri.conf.json")
Check "icono ico existe" (Test-Path "$root\src-tauri\icons\icon.ico")
Check "documentación arquitectura" (Test-Path "$root\docs\ARQUITECTURA.md")
Check "manual de uso" (Test-Path "$root\docs\MANUAL.md")

$exe = Get-ChildItem "$root\src-tauri\target\release\*.exe" -ErrorAction SilentlyContinue |
       Where-Object { $_.Name -notlike "*setup*" } | Select-Object -First 1
Check "binario release compilado" ($null -ne $exe)

$setup = Get-ChildItem "$root\src-tauri\target\release\bundle\nsis\*.exe" -ErrorAction SilentlyContinue |
         Select-Object -First 1
Check "instalador NSIS generado" ($null -ne $setup)

if ($setup) { Write-Host "`nInstalador: $($setup.FullName) ($([math]::Round($setup.Length/1MB,1)) MB)" }
if ($exe) { Write-Host "Binario:    $($exe.FullName) ($([math]::Round($exe.Length/1MB,1)) MB)" }

if ($fails -gt 0) { Write-Host "`n$fails comprobaciones fallidas" -ForegroundColor Red; exit 1 }
Write-Host "`nSmoke test OK" -ForegroundColor Green
