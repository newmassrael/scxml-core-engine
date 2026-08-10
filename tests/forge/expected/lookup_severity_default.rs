#![doc = "SCE-MAP: lookup_severity_default:9 :: _forge_body"]
// SCE-MAP: lookup_severity_default:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
use sce_forge_runtime::lookup::lookup;

const KEYS: [i32; 5] = [100, 200, 300, 400, 500];
const VALUES: [i32; 5] = [1, 2, 3, 2, 4];

pub fn lookup_severity(code: i32) -> i32 {
    lookup(&KEYS, &VALUES, code).unwrap_or(0)
}
