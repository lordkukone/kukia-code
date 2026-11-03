# Pulsa un botón real (ControlType.Button) de la ventana de KUKIA CODE por su nombre.
param(
    [Parameter(Mandatory = $true)][string]$Nombre,
    [int]$Indice = 0,
    [int]$Espera = 1500
)

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes

$p = Get-Process kukia-code -ErrorAction SilentlyContinue |
    Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
if (-not $p) { Write-Output "ERROR: no hay ventana de KUKIA CODE"; exit 1 }

$root = [System.Windows.Automation.AutomationElement]::FromHandle($p.MainWindowHandle)
# Se recorre todo el árbol y se filtra en memoria: la condición compuesta de
# UIA no siempre devuelve los botones del WebView, y así se ve el nombre real.
$todos = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants,
    [System.Windows.Automation.Condition]::TrueCondition)
# Los botones que el CSS parte en dos líneas traen un salto dentro del nombre
# accesible ("Descargar\ntodo"): se normaliza el espacio antes de comparar.
$patron = ($Nombre -replace '\s+', ' ').Trim()
$candidatos = @()
foreach ($e in $todos) {
    $t = $e.Current.ControlType.ProgrammaticName -replace 'ControlType\.', ''
    if ($t -ne 'Button') { continue }
    if (-not $e.Current.IsEnabled) { continue }
    $n = ($e.Current.Name -replace '\s+', ' ').Trim()
    if ($n -like "*$patron*") { $candidatos += $e }
}

Write-Output "BOTONES encontrados con '$Nombre': $($candidatos.Count)"
if ($candidatos.Count -eq 0) { exit 1 }

$el = $candidatos[[Math]::Min($Indice, $candidatos.Count - 1)]
Write-Output "PULSANDO: '$($el.Current.Name)'"
try {
    $el.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
    Write-Output "ACCION: InvokePattern ejecutado"
} catch {
    Write-Output "ACCION: fallo ($($_.Exception.Message))"
    exit 1
}
Start-Sleep -Milliseconds $Espera
