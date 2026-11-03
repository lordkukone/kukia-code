param([string]$Nombre, [int]$Indice = 0, [int]$Espera = 2500)
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
$p = Get-Process kukia-code -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
if (-not $p) { "ERROR: sin ventana"; exit 1 }
$root = [System.Windows.Automation.AutomationElement]::FromHandle([IntPtr]$p.MainWindowHandle)
$cond = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::Button)
$botones = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $cond)
$coinc = @()
foreach ($b in $botones) {
  $n = $b.Current.Name
  if ($n -and $n.ToLower().Contains($Nombre.ToLower()) -and $b.Current.IsEnabled) { $coinc += $b }
}
"encontrados habilitados con '$Nombre': $($coinc.Count)"
if ($coinc.Count -le $Indice) { "ERROR: indice $Indice fuera de rango"; exit 2 }
for ($i=0; $i -lt $coinc.Count; $i++) { "  [$i] " + $coinc[$i].Current.Name }
$obj = $coinc[$Indice].GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
$obj.Invoke()
"PULSADO [$Indice] " + $coinc[$Indice].Current.Name
Start-Sleep -Milliseconds $Espera
