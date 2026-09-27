// SCE-GENERATED — DO NOT EDIT
// source-hash: 0ffd4f5aaee672eb45b262966e33f5e18902327fd6170e44dc26c067a1ddada4
#![doc = "SCE-MAP: outbound_sends_now.scxml:16 :: _forge_body"]
// SCE-MAP: outbound_sends_now.scxml:16 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn outbound_sends_now(ready: bool, depth: u32) -> bool {
    return ready && depth == 0;
}
