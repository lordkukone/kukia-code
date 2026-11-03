param([string]$Texto = "n8n")
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
$p = Get-Process kukia-code -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
$root = [System.Windows.Automation.AutomationElement]::FromHandle([IntPtr]$p.MainWindowHandle)
$edits = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants,
  (New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::Edit)))
"campos de texto: $($edits.Count)"
$objetivo = $null
foreach ($e in $edits) {
  "  - '" + $e.Current.Name + "'"
  if ($e.Current.Name -like "*Buscar en memoria*") { $objetivo = $e }
}
if (-not $objetivo) { "ERROR: no encuentro el buscador"; exit 1 }
$vp = $objetivo.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern)
$vp.SetValue($Texto)
Start-Sleep -Milliseconds 400
"escrito: $Texto"
$botones = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants,
  (New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::Button)))
foreach ($b in $botones) {
  if ($b.Current.Name -eq "Buscar" -and $b.Current.IsEnabled) {
    $b.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
    "pulsado Buscar"
    break
  }
}
Start-Sleep -Milliseconds 2500
$textos = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants,
  (New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::Text)))
$cuenta = 0
foreach ($t in $textos) {
  if ($t.Current.Name -like "*fb:*") { $cuenta++; if ($cuenta -le 3) { "RESULTADO: " + $t.Current.Name.Substring(0, [Math]::Min(180, $t.Current.Name.Length)) } }
}
"resultados con marca [fb:]: $cuenta"
