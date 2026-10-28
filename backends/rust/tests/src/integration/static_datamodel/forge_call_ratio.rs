// SCE-GENERATED — DO NOT EDIT
// source-hash: 8b4828d778fc1da05444ac6a052eb44c820b7a80e1f4075fc52bc6e97e8a070a
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
