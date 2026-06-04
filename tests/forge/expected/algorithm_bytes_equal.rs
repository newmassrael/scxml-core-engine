#![doc = "SCE-MAP: algorithm_bytes_equal:18"]
// SCE-MAP: algorithm_bytes_equal:18

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §5.A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = false).

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn bytes_equal(a: &[u8], b: &[u8]) -> bool {
    if (a).len() != (b).len() {
        return false;
    }
    let mut i: u32 = 0;
    while i < (a).len() {
        if a[(i) as usize] != b[(i) as usize] {
            return false;
        }
        i = i + 1;
    }
    return true;
}
