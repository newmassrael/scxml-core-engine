#![doc = "SCE-MAP: condition_threshold:3 :: _forge_body"]
// SCE-MAP: condition_threshold:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="condition")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#[allow(
    clippy::bool_comparison,
    clippy::comparison_to_empty,
    clippy::manual_range_contains
)]
pub fn condition_threshold(coolant_temp: f64, oil_temp: f64, max_temp: f64) -> bool {
    coolant_temp > max_temp || oil_temp > max_temp
}
