#![doc = "SCE-MAP: transform_temperature:3 :: _forge_body"]
// SCE-MAP: transform_temperature:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

pub fn compute_temperature(raw: u16) -> f64 {
    raw as f64 * 0.1 - 40.0
}
