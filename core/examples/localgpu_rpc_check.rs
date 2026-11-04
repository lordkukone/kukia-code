//! Verifica la sonda al i7, su tiempo y la decision de perfil (lo mismo que hace
//! `localgpu_start` antes de arrancar nada).
use std::time::Instant;
use kukia_core::localgpu::{self, Perfil};

fn main() {
    let cfg = localgpu::cfg();
    println!("rpc_host={} timeout={}s perfil_config={:?}", cfg.rpc_host, cfg.timeout_rpc_s, cfg.perfil());
    let destino = if cfg.rpc_host.contains(':') {
        cfg.rpc_host.clone()
    } else {
        format!("{}:{}", cfg.rpc_host, kukia_core::localgpu::flags::RPC_PUERTO)
    };
    let t0 = Instant::now();
    let sonda = localgpu::rpc::sondar(&destino, cfg.timeout_rpc_s);
    let ms = t0.elapsed().as_millis();
    println!("sonda ok={} destino={} ms={} detalle={}", sonda.ok, sonda.destino, ms, sonda.detalle);
    let (perfil, motivo) = localgpu::perfil_efectivo(Perfil::TresGpus, sonda.ok);
    println!("decision: {:?} | motivo: {}", perfil, motivo);
    println!("criterio timeout<=10s: {}", if ms <= 10_500 { "CUMPLE" } else { "NO CUMPLE" });
}
