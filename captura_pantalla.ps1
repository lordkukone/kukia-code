param([string]$Salida = "pantalla.png", [int]$Izq=0, [int]$Arr=0, [int]$Ancho=0, [int]$Alto=0)
Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class W2 {
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
}
"@
$p = Get-Process kukia-code -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
if (-not $p) { "ERROR: sin ventana"; exit 1 }
$h = [IntPtr]$p.MainWindowHandle
[W2]::ShowWindow($h, 9) | Out-Null
Start-Sleep -Milliseconds 500
if ($Ancho -gt 0) {
  [W2]::SetWindowPos($h, [IntPtr]::Zero, $Izq, $Arr, $Ancho, $Alto, 0x0040) | Out-Null
  Start-Sleep -Milliseconds 800
}
[W2]::SetForegroundWindow($h) | Out-Null
Start-Sleep -Milliseconds 800
$r = New-Object W2+RECT
[W2]::GetWindowRect($h, [ref]$r) | Out-Null
$w = $r.Right - $r.Left; $hh = $r.Bottom - $r.Top
"RECT real: $w x $hh en $($r.Left),$($r.Top)"
if ($w -le 0 -or $hh -le 0) { "ERROR: rect invalido"; exit 1 }
$bmp = New-Object System.Drawing.Bitmap($w, $hh)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($r.Left, $r.Top, 0, 0, (New-Object System.Drawing.Size($w, $hh)))
$dir = Join-Path $PSScriptRoot "tmp_capturas"
if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Path $dir | Out-Null }
$ruta = Join-Path $dir $Salida
$bmp.Save($ruta, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
"OK: $ruta ($w x $hh)"
