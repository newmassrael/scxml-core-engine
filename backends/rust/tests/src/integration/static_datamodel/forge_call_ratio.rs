// SCE-GENERATED — DO NOT EDIT
// source-hash: 6fe63bbaa76604f09a48d24750e75e0a784e90fcb598cdca75c3ac643915bb8d
#![doc = "SCE-MAP: forge_call_ratio.scxml:6 :: _forge_body"]
// SCE-MAP: forge_call_ratio.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
use sce_forge_runtime::lookup::lookup as sce_lookup;

const KEYS: [u8; 3] = [1, 2, 3];
const VALUES: [i32; 3] = [35, 20, 14];

pub fn lookup_ratio(gear: u8) -> i32 {
    sce_lookup(&KEYS, &VALUES, gear).unwrap_or(0)
}
