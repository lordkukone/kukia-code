//! Sonda del `rpc-server` del i7. Solo TCP: si el puerto abre no se garantiza
//! que sirva capas, así que el texto lo dice tal cual.

use serde::Serialize;
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

/// Resultado de la sonda, listo para la interfaz.
#[derive(Debug, Clone, Serialize)]
pub struct RpcSonda {
    pub ok: bool,
    pub destino: String,
    pub ms: u64,
    pub detalle: String,
}

/// Normaliza `host` o `host:puerto`. Sin puerto se asume el de llama.cpp (50052).
pub fn normalizar(destino: &str) -> String {
    let d = destino.trim();
    if d.is_empty() {
        return String::new();
    }
    if d.contains(':') {
        d.to_string()
    } else {
        format!("{d}:{}", crate::localgpu::flags::RPC_PUERTO)
    }
}

/// Intenta conectar. Nunca tarda más de `timeout_s` (entre 1 y 60 s).
pub fn sondar(destino: &str, timeout_s: u64) -> RpcSonda {
    let destino = normalizar(destino);
    if destino.is_empty() {
        return RpcSonda {
            ok: false,
            destino,
            ms: 0,
            detalle: "sin destino configurado (local_gpu.rpc_host)".into(),
        };
    }
    let t0 = Instant::now();
    let tope = Duration::from_millis(timeout_s.clamp(1, 60) * 1000);
    let dir = match destino.to_socket_addrs() {
        Ok(mut it) => match it.next() {
            Some(a) => a,
            None => {
                return RpcSonda {
                    ok: false,
                    destino,
                    ms: t0.elapsed().as_millis() as u64,
                    detalle: "el destino no resolvió a ninguna dirección".into(),
                }
            }
        },
        Err(e) => {
            return RpcSonda {
                ok: false,
                destino,
                ms: t0.elapsed().as_millis() as u64,
                detalle: format!("no se pudo resolver: {e}"),
            }
        }
    };
    match TcpStream::connect_timeout(&dir, tope) {
        Ok(_) => RpcSonda {
            ok: true,
            destino,
            ms: t0.elapsed().as_millis() as u64,
            detalle: format!(
                "el rpc-server acepta conexión en {dir} (puerto abierto; que sirva capas se ve al cargar)"
            ),
        },
        Err(e) => RpcSonda {
            ok: false,
            destino,
            ms: t0.elapsed().as_millis() as u64,
            detalle: format!(
                "sin respuesta en {dir}: {e} (¿rpc-server apagado, firewall o IP distinta?)"
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_host_sin_puerto_recibe_el_de_llama_cpp() {
        assert_eq!(normalizar("192.168.50.2"), "192.168.50.2:50052");
        assert_eq!(normalizar(" 10.0.0.5:6000 "), "10.0.0.5:6000");
        assert_eq!(normalizar("   "), "");
    }

    #[test]
    fn sin_destino_no_se_intenta_nada() {
        let s = sondar("", 1);
        assert!(!s.ok);
        assert!(s.detalle.contains("sin destino"));
    }

    #[test]
    fn un_puerto_cerrado_devuelve_fallo_sin_colgarse() {
        let t0 = Instant::now();
        let s = sondar("127.0.0.1:1", 2);
        assert!(!s.ok);
        assert!(t0.elapsed().as_secs() < 5, "la sonda no debe quedarse colgada");
        assert!(!s.detalle.is_empty());
    }
}
