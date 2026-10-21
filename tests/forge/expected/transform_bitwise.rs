#![doc = "SCE-MAP: transform_bitwise:3 :: _forge_body"]
// SCE-MAP: transform_bitwise:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#[allow(clippy::comparison_to_empty)]
pub fn compute_high_nibble(byte: u8) -> u8 {
    byte >> 4 & 0x0F
}

#[allow(clippy::comparison_to_empty)]
pub fn compute_low_nibble(byte: u8) -> u8 {
    byte & 0x0F
}
