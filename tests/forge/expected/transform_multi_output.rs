#![doc = "SCE-MAP: transform_multi_output:3 :: _forge_body"]
// SCE-MAP: transform_multi_output:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#[allow(
    clippy::bool_comparison,
    clippy::comparison_to_empty,
    clippy::manual_range_contains
)]
pub fn compute_fahrenheit(celsius: f64) -> f64 {
    celsius * 9.0 / 5.0 + 32.0
}

#[allow(
    clippy::bool_comparison,
    clippy::comparison_to_empty,
    clippy::manual_range_contains
)]
pub fn compute_kelvin(celsius: f64) -> f64 {
    celsius + 273.15
}
