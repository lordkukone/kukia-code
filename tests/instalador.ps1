# KUKIA CODE — Validación del instalador NSIS (Fase 2.1)
# Instala en modo usuario (sin admin), arranca la app instalada y verifica persistencia.
# NO desinstala y NO borra datos del usuario.

$ErrorActionPreference = "Stop"
$root    = "H:\KUKIA_CODE"
$setup   = Get-ChildItem "$root\target\release\bundle\nsis\*.exe" | Select-Object -First 1
$appData = "$env:APPDATA\KukiaCode"

Write-Host "=== INSTALADOR ==="
Write-Host "Ruta:  $($setup.FullName)"
Write-Host "Bytes: $($setup.Length)"
Write-Host "SHA256: $((Get-FileHash $setup.FullName -Algorithm SHA256).Hash)"
Write-Host "Fecha: $($setup.LastWriteTime)"

Write-Host "`n=== DATOS ANTES DE INSTALAR ==="
$dbBefore = if (Test-Path "$appData\kukia.db") { (Get-Item "$appData\kukia.db").Length } else { 0 }
$logBefore = if (Test-Path "$appData\logs\kukia.log") { (Get-Item "$appData\logs\kukia.log").Length } else { 0 }
Write-Host "kukia.db: $dbBefore bytes | kukia.log: $logBefore bytes"

Write-Host "`n=== INSTALACION SILENCIOSA (per-user) ==="
$p = Start-Process -FilePath $setup.FullName -ArgumentList "/S","/CURRENTUSER" -PassThru -Wait
Write-Host "Codigo de salida del instalador: $($p.ExitCode)"

Write-Host "`n=== BUSQUEDA DEL EJECUTABLE INSTALADO ==="
$candidatos = @(
    "$env:LOCALAPPDATA\KUKIA CODE\kukia-code.exe",
    "$env:LOCALAPPDATA\Programs\KUKIA CODE\kukia-code.exe",
    "$env:ProgramFiles\KUKIA CODE\kukia-code.exe",
    "${env:ProgramFiles(x86)}\KUKIA CODE\kukia-code.exe"
)
$instalado = $candidatos | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $instalado) {
    $instalado = Get-ChildItem "$env:LOCALAPPDATA","$env:ProgramFiles" -Recurse -Filter "kukia-code.exe" -ErrorAction SilentlyContinue |
                 Select-Object -First 1 -ExpandProperty FullName
}
if ($instalado) {
    Write-Host "[OK] Instalado en: $instalado"
    Write-Host "     Version: $((Get-Item $instalado).VersionInfo.FileVersion)"
    Write-Host "     SHA256:  $((Get-FileHash $instalado -Algorithm SHA256).Hash)"
} else {
    Write-Host "[FAIL] No se encontro el ejecutable instalado"
    exit 1
}

Write-Host "`n=== ARRANQUE DE LA APP INSTALADA ==="
$proc = Start-Process -FilePath $instalado -PassThru
Start-Sleep -Seconds 8
$proc.Refresh()
Write-Host "Proceso vivo: $(-not $proc.HasExited) (PID $($proc.Id))"
Write-Host "Titulo ventana: '$($proc.MainWindowTitle)'"
Write-Host "RAM: $([math]::Round($proc.WorkingSet64/1MB,1)) MB"

$dbAfter = if (Test-Path "$appData\kukia.db") { (Get-Item "$appData\kukia.db").Length } else { 0 }
$logAfter = if (Test-Path "$appData\logs\kukia.log") { (Get-Item "$appData\logs\kukia.log").Length } else { 0 }
Write-Host "kukia.db: $dbAfter bytes | kukia.log: $logAfter bytes"

$closed = $proc.CloseMainWindow()
Write-Host "CloseMainWindow: $closed"
Start-Sleep -Seconds 3
if (-not $proc.HasExited) { $proc.Kill(); Write-Host "Cerrado forzado (prueba)" } else { Write-Host "Cerrado limpiamente" }

Write-Host "`n=== SEGUNDO ARRANQUE (persistencia) ==="
$proc2 = Start-Process -FilePath $instalado -PassThru
Start-Sleep -Seconds 6
$proc2.Refresh()
Write-Host "Proceso vivo: $(-not $proc2.HasExited) (PID $($proc2.Id))"
Write-Host "Titulo: '$($proc2.MainWindowTitle)'"
$dbFinal = if (Test-Path "$appData\kukia.db") { (Get-Item "$appData\kukia.db").Length } else { 0 }
Write-Host "kukia.db tras segundo arranque: $dbFinal bytes"
$proc2.CloseMainWindow() | Out-Null
Start-Sleep -Seconds 2
if (-not $proc2.HasExited) { $proc2.Kill() }

Write-Host "`n=== LOG FINAL ==="
Get-Content "$appData\logs\kukia.log" -Tail 8 -ErrorAction SilentlyContinue

Write-Host "`n=== RESUMEN ==="
Write-Host "Instalado:    $instalado"
Write-Host "AppData:      $appData"
Write-Host "BD inicial:   $dbBefore bytes -> final: $dbFinal bytes"
Write-Host "La app queda instalada. Desinstalar solo con permiso explicito de Kuki."
