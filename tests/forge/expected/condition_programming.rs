#![doc = "SCE-MAP: condition_programming:3 :: _forge_body"]
// SCE-MAP: condition_programming:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="condition")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

pub fn condition_programming(engine_stop: bool, ignition: bool) -> bool {
    engine_stop == true && ignition == true
}
