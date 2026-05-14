#![doc = "SCE-MAP: transform_multi_output:3"]
// SCE-MAP: transform_multi_output:3

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

pub fn compute_fahrenheit(celsius: f64) -> f64 {
    celsius * 9.0 / 5.0 + 32.0
}

pub fn compute_kelvin(celsius: f64) -> f64 {
    celsius + 273.15
}
