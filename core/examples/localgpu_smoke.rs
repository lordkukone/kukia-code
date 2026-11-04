//! Prueba de humo REAL del motor local: arranca `llama-server`, espera a que
//! escuche, enseña sus últimas líneas y lo para. No forma parte de la app.
//!
//! Uso: cargo run --example localgpu_smoke -p kukia-core -- <llama-server.exe> <modelo.gguf> [solo_i9|tres_gpus]

use std::time::{Duration, Instant};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        println!("uso: localgpu_smoke <llama-server.exe> <modelo.gguf> [solo_i9|tres_gpus]");
        std::process::exit(2);
    }
    let mut cfg = kukia_core::localgpu::LocalGpuCfg::default();
    cfg.llama_server = args[0].clone();
    cfg.modelo = args[1].clone();
    cfg.perfil = args.get(2).cloned().unwrap_or_else(|| "solo_i9".into());
    cfg.ctx = 2048;
    cfg.puerto = 8099;
    cfg.timeout_rpc_s = 5;
    let perfil = cfg.perfil();
    println!("perfil={perfil:?} puerto={} ctx={}", cfg.puerto, cfg.ctx);

    match kukia_core::localgpu::proceso::arrancar(&cfg, perfil) {
        Ok(est) => println!("arrancado pid={:?}", est.pid),
        Err(e) => {
            println!("NO ARRANCO: {e}");
            std::process::exit(3);
        }
    }

    let t0 = Instant::now();
    let mut escucha = false;
    while t0.elapsed() < Duration::from_secs(180) {
        let st = kukia_core::localgpu::proceso::status();
        if let Some(e) = &st.error {
            println!("el proceso murio: {e}");
            break;
        }
        if st.lineas.iter().any(|l| {
            l.contains("listening") || l.contains("HTTP server") || l.contains("server is listening")
        }) {
            escucha = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }

    let st = kukia_core::localgpu::proceso::status();
    println!("--- ultimas lineas ({}) ---", st.lineas.len());
    for l in st.lineas.iter().rev().take(14).rev() {
        println!("   {l}");
    }
    let dir: std::net::SocketAddr = format!("127.0.0.1:{}", cfg.puerto).parse().unwrap();
    let tcp = std::net::TcpStream::connect_timeout(&dir, Duration::from_secs(3)).is_ok();
    println!(
        "escucha_log={escucha} tcp_8099={tcp} en {:.1}s",
        t0.elapsed().as_secs_f32()
    );

    let parado = kukia_core::localgpu::proceso::parar().expect("no se pudo parar");
    println!("parado corriendo={} pid={:?}", parado.corriendo, parado.pid);
    std::process::exit(if tcp { 0 } else { 1 });
}
