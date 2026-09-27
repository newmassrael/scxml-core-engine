// SCE-GENERATED — DO NOT EDIT
// source-hash: 201ac60c6ba911f84b8077668d3ff26a61f49af962c8e5a4b48dfbfc2397df5a
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
