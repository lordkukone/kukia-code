Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
$p = Get-Process kukia-code -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
if (-not $p) { "ERROR: sin ventana"; exit 1 }
$root = [System.Windows.Automation.AutomationElement]::FromHandle([IntPtr]$p.MainWindowHandle)
# Localizar el selector de proveedor cloud por su etiqueta cercana
$combos = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants,
  (New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::ComboBox)))
"combos: $($combos.Count)"
foreach ($c in $combos) {
  "valor actual: '" + $c.Current.Name + "' | patron: " + ($c.GetSupportedPatterns() | ForEach-Object { $_.ProgrammaticName }) -join ','
  try {
    $ec = $c.GetCurrentPattern([System.Windows.Automation.ExpandCollapsePattern]::Pattern)
    $ec.Expand()
    Start-Sleep -Milliseconds 900
    $items = $c.FindAll([System.Windows.Automation.TreeScope]::Descendants,
      (New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::ListItem)))
    "opciones ($($items.Count)):"
    foreach ($i in $items) { "   - " + $i.Current.Name }
    $ec.Collapse()
  } catch { "  (sin ExpandCollapse: " + $_.Exception.Message + ")" }
}
