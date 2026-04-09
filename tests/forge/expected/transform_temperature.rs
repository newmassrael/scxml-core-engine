// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Do not edit — regenerate from the source SCXML file.

#![allow(dead_code, clippy::excessive_precision)]

pub fn compute_temperature(raw: u16) -> f64 {
    raw as f64 * 0.1 - 40.0
}
