//! Sensores de GPU universales: NVIDIA, AMD e Intel.
//!
//! **Por qué existe este módulo.** Hasta ahora todo salía de `nvidia-smi`, así que
//! en una tarjeta AMD o Intel la app no veía ninguna GPU: ni VRAM, ni memoria libre,
//! ni la guardia de «¿cabe este modelo?». Aquí se lee primero lo que Windows sabe de
//! **cualquier** tarjeta y solo después se añade lo que cada fabricante publique.
//!
//! - **Lista, nombre y VRAM total:** DXGI (`EnumAdapters1` + `GetDesc1`). Es la API
//!   de Windows para enumerar adaptadores gráficos; no depende del fabricante. Una
//!   iGPU no tiene VRAM dedicada, así que se informa su memoria compartida y se
//!   marca con `compartida`.
//! - **Memoria en uso:** contadores de rendimiento `\GPU Adapter Memory(*)\Dedicated
//!   Usage`, la misma fuente que el Administrador de tareas y que existe desde
//!   Windows 10 para todos los fabricantes. Se empareja con cada tarjeta por su
//!   LUID, que viene en el nombre del contador (`luid_0x..._0x..._phys_0`).
//! - **Temperatura, potencia, relojes y utilización:** solo si el fabricante los
//!   publica por una vía legible. Hoy eso es `nvidia-smi`. Cuando no hay, los campos
//!   quedan en 0 y `fuente` lo dice: no se rellenan con inventos.
//!
//! Nada de esto cierra procesos ni toca configuración: solo lee.

use serde::Serialize;

/// Una tarjeta gráfica tal como la ve el sistema, sea del fabricante que sea.
#[derive(Debug, Clone, Serialize)]
pub struct Tarjeta {
    pub index: u32,
    pub nombre: String,
    /// `nvidia`, `amd`, `intel`, `microsoft` o `desconocido`.
    pub fabricante: String,
    pub vendor_id: u32,
    /// VRAM dedicada; si la tarjeta no tiene (iGPU), su memoria compartida.
    pub total_mb: u64,
    /// Memoria en uso de **todo el sistema**, no solo de esta app.
    /// `None` cuando Windows no la publica para esa tarjeta.
    pub usada_mb: Option<u64>,
    /// Cierto si `total_mb` es memoria compartida con la RAM (gráfica integrada).
    pub compartida: bool,
    /// De dónde sale `usada_mb`, para poder decirlo en la interfaz.
    pub fuente_uso: String,
    /// LUID del adaptador en hexadecimal. Sirve para distinguir dos tarjetas
    /// iguales de la misma tarjeta enumerada dos veces; no se manda a la interfaz.
    #[serde(skip)]
    pub luid: String,
}

/// Lo que `nvidia-smi` sabe de una tarjeta NVIDIA.
///
/// Solo se usa cuando el driver NVIDIA está instalado. En AMD o Intel esta lista
/// viene vacía y nadie echa de menos esos campos: se dejan a 0 en vez de inventarlos.
#[derive(Debug, Clone, Serialize, Default)]
pub struct SmiTarjeta {
    pub index: u32,
    pub name: String,
    pub total_mb: u64,
    pub used_mb: u64,
    pub free_mb: u64,
    pub utilization: f64,
    pub temperature_c: f64,
    pub power_w: f64,
    pub clock_graphics_mhz: u64,
    pub clock_memory_mhz: u64,
}

/// Identificador de fabricante PCI → nombre legible.
pub fn fabricante(vendor_id: u32) -> &'static str {
    match vendor_id {
        0x10DE => "nvidia",
        0x1002 | 0x1022 => "amd",
        0x8086 => "intel",
        0x1414 => "microsoft",
        _ => "desconocido",
    }
}

/// Normaliza un nombre de tarjeta para emparejar lo que dice DXGI con lo que dice
/// `nvidia-smi` (`NVIDIA GeForce RTX 5070` y `RTX 5070` son la misma tarjeta).
pub fn normaliza(nombre: &str) -> String {
    nombre
        .to_lowercase()
        .replace("nvidia", " ")
        .replace("geforce", " ")
        .replace("amd", " ")
        .replace("radeon", " ")
        .replace("(r)", " ")
        .replace("(tm)", " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Extrae el LUID de un nombre de instancia de contador de Windows.
///
/// Forma real: `luid_0x00000000_0x0000C4A3_phys_0` → `(0x0000C4A3, 0x00000000)`,
/// es decir `(LowPart, HighPart)`, que es como lo da DXGI.
pub fn parsear_luid(nombre: &str) -> Option<(u32, u32)> {
    let mut partes = nombre.split('_');
    if !partes.next()?.eq_ignore_ascii_case("luid") {
        return None;
    }
    let alto = u32::from_str_radix(partes.next()?.trim_start_matches("0x"), 16).ok()?;
    let bajo = u32::from_str_radix(partes.next()?.trim_start_matches("0x"), 16).ok()?;
    Some((bajo, alto))
}

/// Todas las tarjetas del equipo, con lo que se pueda medir de cada una.
///
/// DXGI puede enumerar dos veces el mismo adaptador: medido en esta máquina, la
/// 5070 sale dos veces, y la copia **no** existe en los contadores de memoria de
/// Windows (la crea un monitor virtual). Se descarta por dos criterios:
///
/// 1. Mismo LUID repetido: es la misma tarjeta, sin discusión.
/// 2. Mismo nombre y misma VRAM que otra que **sí** tiene contador, y sin contador
///    propio: es un duplicado fantasma. Dos tarjetas físicas iguales sí aparecen
///    las dos en los contadores, así que siguen contando como dos.
pub fn tarjetas() -> Vec<Tarjeta> {
    let base = plataforma::adaptadores();
    let uso = plataforma::uso_por_luid();
    let mut vistos: Vec<(u32, u32)> = Vec::new();
    let unicas: Vec<&Adaptador> = base
        .iter()
        .filter(|a| {
            if vistos.contains(&a.luid) {
                return false;
            }
            let fantasma = !uso.contains_key(&a.luid)
                && !uso.is_empty()
                && base.iter().any(|b| {
                    b.luid != a.luid
                        && uso.contains_key(&b.luid)
                        && b.total_mb == a.total_mb
                        && normaliza(&b.nombre) == normaliza(&a.nombre)
                });
            if fantasma {
                return false;
            }
            vistos.push(a.luid);
            true
        })
        .collect();
    unicas
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let usada = uso
                .get(&(a.luid.0, a.luid.1))
                .map(|bytes| bytes / (1024 * 1024))
                .map(|mb| mb.min(a.total_mb));
            Tarjeta {
                index: i as u32,
                nombre: a.nombre.clone(),
                fabricante: fabricante(a.vendor).to_string(),
                vendor_id: a.vendor,
                total_mb: a.total_mb,
                usada_mb: usada,
                compartida: a.compartida,
                fuente_uso: if usada.is_some() {
                    "contadores de Windows".to_string()
                } else {
                    "no disponible".to_string()
                },
                luid: format!("{:08x}:{:08x}", a.luid.1, a.luid.0),
            }
        })
        .collect()
}

/// Detalle de las tarjetas NVIDIA, si `nvidia-smi` está disponible.
///
/// En una máquina AMD o Intel devuelve una lista vacía sin errores: es lo normal,
/// no un fallo, y por eso no se registra como problema.
pub fn nvidia_smi() -> Vec<SmiTarjeta> {
    let query = "index,name,memory.total,memory.used,memory.free,utilization.gpu,temperature.gpu,power.draw,clocks.gr,clocks.mem";
    let Some(salida) = plataforma::ejecutar_smi(query) else {
        return Vec::new();
    };
    salida
        .lines()
        .filter_map(|linea| {
            let p: Vec<&str> = linea.split(',').map(|s| s.trim()).collect();
            if p.len() < 10 {
                return None;
            }
            Some(SmiTarjeta {
                index: p[0].parse().unwrap_or(0),
                name: p[1].to_string(),
                total_mb: p[2].parse().unwrap_or(0),
                used_mb: p[3].parse().unwrap_or(0),
                free_mb: p[4].parse().unwrap_or(0),
                utilization: p[5].parse().unwrap_or(0.0),
                temperature_c: p[6].parse().unwrap_or(0.0),
                power_w: p[7].parse().unwrap_or(0.0),
                clock_graphics_mhz: p[8].parse().unwrap_or(0),
                clock_memory_mhz: p[9].parse().unwrap_or(0),
            })
        })
        .collect()
}

/* ============================ Plataforma ============================ */

struct Adaptador {
    nombre: String,
    vendor: u32,
    total_mb: u64,
    compartida: bool,
    luid: (u32, u32),
}

#[cfg(windows)]
mod plataforma {
    use super::Adaptador;
    use std::collections::HashMap;
    use std::ffi::c_void;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Guid {
        d1: u32,
        d2: u16,
        d3: u16,
        d4: [u8; 8],
    }

    /// `{770aae78-f26f-4dba-a829-253c83d1b387}` = IID_IDXGIFactory1.
    const IID_FACTORY1: Guid = Guid {
        d1: 0x770a_ae78,
        d2: 0xf26f,
        d3: 0x4dba,
        d4: [0xa8, 0x29, 0x25, 0x3c, 0x83, 0xd1, 0xb3, 0x87],
    };
    const DXGI_ERROR_NOT_FOUND: i32 = 0x887A_0002u32 as i32;
    const ADAPTER_FLAG_SOFTWARE: u32 = 2;

    /// Posiciones en la vtable (COM: cada interfaz añade sus métodos detrás).
    const SLOT_RELEASE: usize = 2;
    const SLOT_GET_DESC1: usize = 10; // IDXGIAdapter1
    const SLOT_ENUM_ADAPTERS1: usize = 12; // IDXGIFactory1

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct Luid {
        low: u32,
        high: i32,
    }

    #[repr(C)]
    struct AdapterDesc1 {
        description: [u16; 128],
        vendor_id: u32,
        device_id: u32,
        subsys_id: u32,
        revision: u32,
        dedicated_video_memory: usize,
        dedicated_system_memory: usize,
        shared_system_memory: usize,
        adapter_luid: Luid,
        flags: u32,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct FmtCounterValue {
        status: u32,
        _pad: u32,
        double: f64,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct FmtCounterValueItemW {
        name: *const u16,
        value: FmtCounterValue,
    }

    const PDH_FMT_DOUBLE: u32 = 0x0000_0200;
    const PDH_MORE_DATA: u32 = 0x8000_07D2;

    #[link(name = "dxgi")]
    extern "system" {
        fn CreateDXGIFactory1(riid: *const Guid, fabrica: *mut *mut c_void) -> i32;
    }

    #[link(name = "pdh")]
    extern "system" {
        fn PdhOpenQueryW(fuente: *const u16, datos: usize, consulta: *mut isize) -> u32;
        fn PdhAddEnglishCounterW(
            consulta: isize,
            ruta: *const u16,
            datos: usize,
            contador: *mut isize,
        ) -> u32;
        fn PdhCollectQueryData(consulta: isize) -> u32;
        fn PdhGetFormattedCounterArrayW(
            contador: isize,
            formato: u32,
            tam: *mut u32,
            n: *mut u32,
            buffer: *mut FmtCounterValueItemW,
        ) -> u32;
        fn PdhCloseQuery(consulta: isize) -> u32;
    }

    unsafe fn metodo(obj: *mut c_void, indice: usize) -> *const c_void {
        let tabla = *(obj as *const *const *const c_void);
        *tabla.add(indice)
    }

    unsafe fn liberar(obj: *mut c_void) {
        let f: unsafe extern "system" fn(*mut c_void) -> u32 =
            std::mem::transmute(metodo(obj, SLOT_RELEASE));
        f(obj);
    }

    /// Enumeración de adaptadores con DXGI: vale para NVIDIA, AMD e Intel.
    pub fn adaptadores() -> Vec<Adaptador> {
        let mut out = Vec::new();
        unsafe {
            let mut fabrica: *mut c_void = std::ptr::null_mut();
            if CreateDXGIFactory1(&IID_FACTORY1, &mut fabrica) < 0 || fabrica.is_null() {
                return out;
            }
            let enum_adapters: unsafe extern "system" fn(*mut c_void, u32, *mut *mut c_void) -> i32 =
                std::mem::transmute(metodo(fabrica, SLOT_ENUM_ADAPTERS1));

            for i in 0..32u32 {
                let mut adaptador: *mut c_void = std::ptr::null_mut();
                let hr = enum_adapters(fabrica, i, &mut adaptador);
                if hr == DXGI_ERROR_NOT_FOUND || hr < 0 || adaptador.is_null() {
                    break;
                }
                let get_desc: unsafe extern "system" fn(*mut c_void, *mut AdapterDesc1) -> i32 =
                    std::mem::transmute(metodo(adaptador, SLOT_GET_DESC1));
                let mut d: AdapterDesc1 = std::mem::zeroed();
                if get_desc(adaptador, &mut d) >= 0 && d.flags & ADAPTER_FLAG_SOFTWARE == 0 {
                    let fin = d
                        .description
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(d.description.len());
                    let nombre = String::from_utf16_lossy(&d.description[..fin]);
                    let dedicada = (d.dedicated_video_memory / (1024 * 1024)) as u64;
                    let compartida = (d.shared_system_memory / (1024 * 1024)) as u64;
                    // Una iGPU (Intel, APU AMD) no tiene VRAM propia: se informa su
                    // memoria compartida, marcada como tal, en vez de un 0 inútil.
                    let (total_mb, es_compartida) = if dedicada > 0 {
                        (dedicada, false)
                    } else {
                        (compartida, true)
                    };
                    if total_mb > 0 {
                        out.push(Adaptador {
                            nombre,
                            vendor: d.vendor_id,
                            total_mb,
                            compartida: es_compartida,
                            luid: (d.adapter_luid.low, d.adapter_luid.high as u32),
                        });
                    }
                }
                liberar(adaptador);
            }
            liberar(fabrica);
        }
        out
    }

    fn leer_utf16(ptr: *const u16) -> String {
        if ptr.is_null() {
            return String::new();
        }
        let mut n = 0usize;
        unsafe {
            while *ptr.add(n) != 0 && n < 256 {
                n += 1;
            }
            String::from_utf16_lossy(std::slice::from_raw_parts(ptr, n))
        }
    }

    /// Memoria dedicada en uso, por LUID, en bytes (suma de todos sus segmentos).
    ///
    /// Es la misma cifra que enseña el Administrador de tareas en «Memoria de GPU
    /// dedicada»: incluye lo que ocupan Ollama, LM Studio, el navegador y demás.
    pub fn uso_por_luid() -> HashMap<(u32, u32), u64> {
        let mut mapa = HashMap::new();
        unsafe {
            let mut consulta: isize = 0;
            if PdhOpenQueryW(std::ptr::null(), 0, &mut consulta) != 0 {
                return mapa;
            }
            let ruta: Vec<u16> = "\\GPU Adapter Memory(*)\\Dedicated Usage\0"
                .encode_utf16()
                .collect();
            let mut contador: isize = 0;
            if PdhAddEnglishCounterW(consulta, ruta.as_ptr(), 0, &mut contador) != 0 {
                PdhCloseQuery(consulta);
                return mapa;
            }
            // Dos lecturas: la primera solo abre la muestra; sin la segunda el
            // contador deja de valer 0 y parecería que no hay nada cargado.
            PdhCollectQueryData(consulta);
            std::thread::sleep(std::time::Duration::from_millis(120));
            if PdhCollectQueryData(consulta) != 0 {
                PdhCloseQuery(consulta);
                return mapa;
            }

            let mut tam: u32 = 0;
            let mut n: u32 = 0;
            let rc = PdhGetFormattedCounterArrayW(
                contador,
                PDH_FMT_DOUBLE,
                &mut tam,
                &mut n,
                std::ptr::null_mut(),
            );
            if rc == PDH_MORE_DATA && tam > 0 {
                let mut buffer: Vec<u8> = vec![0; tam as usize];
                let items = buffer.as_mut_ptr() as *mut FmtCounterValueItemW;
                let rc2 =
                    PdhGetFormattedCounterArrayW(contador, PDH_FMT_DOUBLE, &mut tam, &mut n, items);
                if rc2 == 0 && !items.is_null() {
                    for item in std::slice::from_raw_parts(items, n as usize) {
                        if item.value.status != 0 {
                            continue;
                        }
                        let Some((bajo, alto)) = super::parsear_luid(&leer_utf16(item.name)) else {
                            continue;
                        };
                        let bytes = item.value.double.max(0.0) as u64;
                        *mapa.entry((bajo, alto)).or_insert(0u64) += bytes;
                    }
                }
            }
            PdhCloseQuery(consulta);
        }
        mapa
    }

    /// Ejecuta `nvidia-smi` sin abrir ventana. `None` si no está instalado.
    pub fn ejecutar_smi(query: &str) -> Option<String> {
        use std::os::windows::process::CommandExt;
        let salida = std::process::Command::new("nvidia-smi")
            .args(["--query-gpu", query, "--format=csv,noheader,nounits"])
            .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
            .output()
            .ok()?;
        if !salida.status.success() {
            return None;
        }
        Some(String::from_utf8_lossy(&salida.stdout).to_string())
    }
}

#[cfg(not(windows))]
mod plataforma {
    use super::Adaptador;
    use std::collections::HashMap;

    pub fn adaptadores() -> Vec<Adaptador> {
        Vec::new()
    }

    pub fn uso_por_luid() -> HashMap<(u32, u32), u64> {
        HashMap::new()
    }

    pub fn ejecutar_smi(_query: &str) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_luid_se_lee_de_la_instancia_del_contador() {
        // Nombre real de un contador de Windows.
        assert_eq!(
            parsear_luid("luid_0x00000000_0x0000C4A3_phys_0"),
            Some((0x0000_C4A3, 0))
        );
        assert_eq!(
            parsear_luid("luid_0x00000001_0x00001234_phys_1"),
            Some((0x0000_1234, 1))
        );
        assert_eq!(parsear_luid("total"), None, "lo que no es un LUID no se interpreta");
        assert_eq!(parsear_luid("luid_sin_hex"), None);
    }

    #[test]
    fn el_fabricante_se_reconoce_por_su_id() {
        assert_eq!(fabricante(0x10DE), "nvidia");
        assert_eq!(fabricante(0x1002), "amd");
        assert_eq!(fabricante(0x8086), "intel");
        assert_eq!(fabricante(0x1234), "desconocido");
    }

    #[test]
    fn los_nombres_se_normalizan_para_emparejarlos() {
        assert_eq!(normaliza("NVIDIA GeForce RTX 5070"), "rtx 5070");
        assert_eq!(normaliza("RTX 5070"), "rtx 5070");
        assert_eq!(normaliza("AMD Radeon RX 7800 XT"), "rx 7800 xt");
    }

    #[test]
    fn la_lista_de_tarjetas_es_coherente() {
        // Con o sin GPU, esto no puede mentir: si hay tarjeta, tiene nombre y memoria,
        // y lo usado nunca supera lo que tiene.
        for t in tarjetas() {
            assert!(!t.nombre.is_empty(), "una tarjeta sin nombre no es una tarjeta");
            assert!(t.total_mb > 0, "{}: memoria total 0", t.nombre);
            if let Some(u) = t.usada_mb {
                assert!(u <= t.total_mb, "{}: usada {} > total {}", t.nombre, u, t.total_mb);
            }
            println!(
                "GPU{} {} [{}] luid {} {} MB{} · uso: {}",
                t.index,
                t.nombre,
                t.fabricante,
                t.luid,
                t.total_mb,
                if t.compartida { " (compartida)" } else { "" },
                match t.usada_mb {
                    Some(u) => format!("{u} MB ({})", t.fuente_uso),
                    None => "no disponible".to_string(),
                }
            );
        }
    }

    #[test]
    fn no_hay_tarjetas_repetidas() {
        // DXGI puede devolver el mismo adaptador dos veces; la lista final no.
        let t = tarjetas();
        let mut luids: Vec<&str> = t.iter().map(|x| x.luid.as_str()).collect();
        luids.sort_unstable();
        let antes = luids.len();
        luids.dedup();
        assert_eq!(antes, luids.len(), "una tarjeta aparece dos veces en la lista");
        // Dos tarjetas iguales de verdad existen (mismo nombre y VRAM), pero no
        // pueden compartir LUID ni ser una copia sin medición de otra.
        for (i, a) in t.iter().enumerate() {
            for b in t.iter().skip(i + 1) {
                let iguales = a.nombre == b.nombre && a.total_mb == b.total_mb;
                assert!(!iguales, "duplicado sin distinguir: {} x2", a.nombre);
            }
        }
        for (i, x) in t.iter().enumerate() {
            assert_eq!(x.index as usize, i, "los indices deben ser correlativos");
        }
    }

    #[test]
    fn nvidia_smi_no_revienta_si_no_existe() {
        // En AMD o Intel devuelve vacío: es lo esperado, no un fallo.
        let smi = nvidia_smi();
        for s in &smi {
            assert!(s.used_mb <= s.total_mb, "nvidia-smi no puede dar usado > total");
        }
        println!("nvidia-smi: {} tarjeta(s)", smi.len());
    }
}
