#![doc = "SCE-MAP: algorithm_bytes_equal:18 :: _forge_body"]
// SCE-MAP: algorithm_bytes_equal:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = false).

#[allow(clippy::all)]
#[allow(unused_assignments)]
#[inline]
pub fn bytes_equal(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut i: u32 = 0;
    while i < a.len() as u32 {
        if sce_forge_runtime::algorithm::proven::at(&a, i) != sce_forge_runtime::algorithm::proven::at(&b, i) {
            return false;
        }
        i = sce_forge_runtime::algorithm::proven::add::<u32>(i, 1);
    }
    return true;
}
