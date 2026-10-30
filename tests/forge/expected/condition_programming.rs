#![doc = "SCE-MAP: condition_programming:3 :: _forge_body"]
// SCE-MAP: condition_programming:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="condition")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#[allow(
    clippy::bool_comparison,
    clippy::comparison_to_empty,
    clippy::manual_range_contains
)]
pub fn condition_programming(engine_stop: bool, ignition: bool) -> bool {
    engine_stop == true && ignition == true
}
