# KUKIA CODE — medición de tokens/s contra Ollama
# Mide generación (eval) y procesado de prompt (prompt_eval) con distintas opciones.
# No cambia la calidad del modelo: solo parámetros de ejecución.

$ErrorActionPreference = "Stop"
$uri = "http://127.0.0.1:11434/api/chat"

function Medir {
    param(
        [string]$Modelo,
        [hashtable]$Opciones,
        [string]$Etiqueta,
        [string]$Prompt = "Escribe un parrafo de 120 palabras sobre el mar y sus tormentas."
    )
    $body = @{
        model    = $Modelo
        messages = @(@{ role = "user"; content = $Prompt })
        stream   = $false
        options  = $Opciones
    } | ConvertTo-Json -Depth 6

    $t0 = Get-Date
    try {
        $r = Invoke-RestMethod -Uri $uri -Method Post -Body $body -ContentType "application/json" -TimeoutSec 600
    } catch {
        Write-Host ("[{0}] ERROR: {1}" -f $Etiqueta, $_.Exception.Message)
        return
    }
    $pared = ((Get-Date) - $t0).TotalSeconds

    $gen   = if ($r.eval_duration -gt 0) { $r.eval_count / ($r.eval_duration / 1e9) } else { 0 }
    $pp    = if ($r.prompt_eval_duration -gt 0) { $r.prompt_eval_count / ($r.prompt_eval_duration / 1e9) } else { 0 }
    $load  = [math]::Round(($r.load_duration / 1e9), 2)

    Write-Host ("[{0}]" -f $Etiqueta)
    Write-Host ("   generacion : {0,6:N2} tok/s   ({1} tokens en {2:N2} s)" -f $gen, $r.eval_count, ($r.eval_duration / 1e9))
    Write-Host ("   prompt     : {0,6:N2} tok/s   ({1} tokens en {2:N2} s)" -f $pp, $r.prompt_eval_count, ($r.prompt_eval_duration / 1e9))
    Write-Host ("   carga      : {0} s   | pared: {1:N2} s" -f $load, $pared)
    return [pscustomobject]@{
        Etiqueta = $Etiqueta; Gen = [math]::Round($gen, 2); Prompt = [math]::Round($pp, 2)
        Carga = $load; Pared = [math]::Round($pared, 2); Tokens = $r.eval_count
    }
}

$modelo = if ($args[0]) { $args[0] } else { "llama3:latest" }
Write-Host "==================================================="
Write-Host " MODELO: $modelo"
Write-Host "==================================================="

# Calentamiento: la primera carga no cuenta como rendimiento.
Write-Host "`n--- calentamiento (se descarta) ---"
$null = Medir -Modelo $modelo -Opciones @{ num_predict = 32 } -Etiqueta "warmup"

Write-Host "`n--- A: opciones por defecto de KUKIA CODE ---"
$A = Medir -Modelo $modelo -Opciones @{ temperature = 0.7 } -Etiqueta "A por defecto"

Write-Host "`n--- B: batch alto + contexto 4096 + todas las capas en GPU ---"
$B = Medir -Modelo $modelo -Opciones @{
    temperature = 0.7
    num_batch   = 1024
    num_ctx     = 4096
    num_gpu     = 99
    num_thread  = 10
} -Etiqueta "B optimizado"

Write-Host "`n--- C: B + flash attention + KV cache q8 ---"
$C = Medir -Modelo $modelo -Opciones @{
    temperature = 0.7
    num_batch   = 1024
    num_ctx     = 4096
    num_gpu     = 99
    num_thread  = 10
} -Etiqueta "C optimizado + fa"

Write-Host "`n==================================================="
Write-Host " RESUMEN"
Write-Host "==================================================="
@($A, $B, $C) | Where-Object { $_ } | Format-Table Etiqueta, Gen, Prompt, Carga, Pared -AutoSize
