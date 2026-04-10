// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Do not edit — regenerate from the source SCXML file.

pub fn compute_high_nibble(byte: u8) -> u8 {
    byte >> 4 & 0x0F
}

pub fn compute_low_nibble(byte: u8) -> u8 {
    byte & 0x0F
}
