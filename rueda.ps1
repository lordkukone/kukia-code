param([int]$X=600, [int]$Y=300, [int]$Veces=3)
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class RW {
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, int d, IntPtr e);
}
"@
[RW]::SetCursorPos($X, $Y) | Out-Null
Start-Sleep -Milliseconds 200
for ($i=0; $i -lt $Veces; $i++) { [RW]::mouse_event(0x0800, 0, 0, -120, [IntPtr]::Zero); Start-Sleep -Milliseconds 220 }
"rueda x$Veces en $X,$Y"
