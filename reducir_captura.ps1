# Reduce una captura para poder verla en el chat (sin depender de rutas raras).
param(
    [Parameter(Mandatory = $true)][string]$Entrada,
    [Parameter(Mandatory = $true)][string]$Salida,
    [int]$Ancho = 1100
)

Add-Type -AssemblyName System.Drawing

$img = [System.Drawing.Image]::FromFile($Entrada)
$alto = [int](($img.Height * $Ancho) / $img.Width)
$bmp = New-Object System.Drawing.Bitmap $Ancho, $alto
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$g.DrawImage($img, 0, 0, $Ancho, $alto)
$bmp.Save($Salida, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose()
$bmp.Dispose()
$img.Dispose()

Write-Output "OK: $Salida ($Ancho x $alto)"
