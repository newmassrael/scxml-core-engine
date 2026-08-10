#![doc = "SCE-MAP: lookup_state_action:5 :: _forge_body"]
// SCE-MAP: lookup_state_action:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
use sce_forge_runtime::lookup::lookup;

const KEYS: [i32; 4] = [0, 1, 2, 3];
const VALUES: [i32; 4] = [10, 20, 30, 40];

pub fn lookup_action(state: i32) -> Option<i32> {
    lookup(&KEYS, &VALUES, state)
}
