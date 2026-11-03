param([string]$Nombre = "Proveedor")
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
$p = Get-Process kukia-code -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
$root = [System.Windows.Automation.AutomationElement]::FromHandle([IntPtr]$p.MainWindowHandle)
$combos = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants,
  (New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::ComboBox)))
foreach ($c in $combos) {
  if ($c.Current.Name -like "*$Nombre*") {
    $vp = $c.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern)
    "combo '$($c.Current.Name)' -> valor seleccionado: '" + $vp.Current.Value + "'"
    # Opciones del desplegable
    try {
      $ec = $c.GetCurrentPattern([System.Windows.Automation.ExpandCollapsePattern]::Pattern)
      $ec.Expand(); Start-Sleep -Milliseconds 800
      $items = $c.FindAll([System.Windows.Automation.TreeScope]::Descendants,
        (New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty, [System.Windows.Automation.ControlType]::ListItem)))
      "opciones: " + (($items | ForEach-Object { $_.Current.Name }) -join ' | ')
      $ec.Collapse()
    } catch { "  no se pudo expandir: " + $_.Exception.Message }
  }
}
