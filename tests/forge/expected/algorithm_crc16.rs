#![doc = "SCE-MAP: algorithm_crc16:11 :: _forge_body"]
// SCE-MAP: algorithm_crc16:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = false).

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn algorithm_crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &b in data.iter() {
        let hi: u16 = b as u16;
        crc = crc ^ hi << 8;
        let mut i: u8 = 0;
        while i < 8 {
            if crc & 0x8000 != 0 {
                crc = crc << 1 ^ 0x1021;
            } else {
                crc = crc << 1;
            }
            i = i + 1;
        }
    }
    return crc;
}
