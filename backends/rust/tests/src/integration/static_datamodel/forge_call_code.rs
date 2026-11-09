// SCE-GENERATED — DO NOT EDIT
// source-hash: d05ce240c5f3b48b3c83b6bee1ba58feef40d57c310e9a29ef5279604bbaaa42
#![doc = "SCE-MAP: forge_call_code.scxml:7 :: _forge_body"]
// SCE-MAP: forge_call_code.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
use sce_forge_runtime::lookup::lookup as sce_lookup;

const KEYS: [u8; 2] = [1, 2];
const VALUES: [i32; 2] = [100, 200];

pub fn lookup_code(gear: u8) -> Option<i32> {
    sce_lookup(&KEYS, &VALUES, gear)
}
