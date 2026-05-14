#![doc = "SCE-MAP: lookup_unit_scale:6"]
// SCE-MAP: lookup_unit_scale:6

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
use sce_forge_runtime::lookup::lookup;

const KEYS: [i32; 6] = [1, 2, 3, 4, 5, 6];
const VALUES: [f64; 6] = [0.001, 0.01, 0.1, 1.0, 10.0, 100.0];

pub fn lookup_scale(unit: i32) -> Option<f64> {
    lookup(&KEYS, &VALUES, unit)
}
