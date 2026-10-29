#![doc = "SCE-MAP: algorithm_tick_until_due:12 :: _forge_body"]
// SCE-MAP: algorithm_tick_until_due:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn algorithm_tick_until_due(now: u32, last: u32, period: u32) -> u32 {
    return u32::saturating_sub(period, u32::wrapping_sub(now, last));
}
