Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class F { [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h); }
"@
$p = Get-Process kukia-code -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
$h = [IntPtr]$p.MainWindowHandle
[F]::SetForegroundWindow($h) | Out-Null
Start-Sleep -Milliseconds 700
$root = [System.Windows.Automation.AutomationElement]::FromHandle($h)
$cond = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::IsScrollPatternAvailableProperty, $true)
$panes = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $cond)
"paneles desplazables: $($panes.Count)"
foreach ($e in $panes) {
  try {
    $sp = $e.GetCurrentPattern([System.Windows.Automation.ScrollPattern]::Pattern)
    if ($sp.Current.VerticallyScrollable) {
      $sp.Scroll([System.Windows.Automation.ScrollAmount]::NoAmount, [System.Windows.Automation.ScrollAmount]::LargeIncrement)
      "desplazado: " + $e.Current.ControlType.ProgrammaticName
      Start-Sleep -Milliseconds 600
    }
  } catch {}
}
