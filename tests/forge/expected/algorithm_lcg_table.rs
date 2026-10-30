#![doc = "SCE-MAP: algorithm_lcg_table:13 :: _forge_body"]
// SCE-MAP: algorithm_lcg_table:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

pub static LCG_TABLE: [u32; 16] = [12345, 87628868, 71072467, 2332836374, 2726892157, 3908547000, 483019191, 2129828778, 2355140353, 2560230508, 3364893915, 171172990, 3194601925, 4148119648, 316399679, 3004788882];

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn algorithm_lcg_table(n: u8) -> u32 {
    return LCG_TABLE[(n & 15) as usize];
}
