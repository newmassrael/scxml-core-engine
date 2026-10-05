#![doc = "SCE-MAP: lookup_alarm_code:6 :: _forge_body"]
// SCE-MAP: lookup_alarm_code:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
use sce_forge_runtime::lookup::lookup as sce_lookup;

const KEYS: [i32; 5] = [100, 200, 300, 400, 500];
const VALUES: [i32; 5] = [1, 2, 3, 2, 4];

pub fn lookup_severity(code: i32) -> Option<i32> {
    sce_lookup(&KEYS, &VALUES, code)
}
