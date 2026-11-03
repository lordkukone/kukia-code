//! KUKIA UNIVERSAL REALITY CONVERTER — RC1 + RC2.
//!
//! - `engines`: registro de motores con diagnóstico real (RC1).
//! - `styles`: catálogo de estilos como dato, con estados honestos (RC1).
//! - `detect`: sondeo, detección de estilo con VLM y segmentación de planos (RC2).

pub mod detect;
pub mod engines;
pub mod styles;

pub use detect::{preflight, EstiloDetectado, Plano, Preflight, Sondeo};
pub use engines::{presupuesto, Motor, Presupuesto, VramLibre};
pub use styles::{cargar as estilos, diagnostico as diagnostico_estilos, Estilo, DiagnosticoEstilo};
