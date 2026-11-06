// SCE-GENERATED — DO NOT EDIT
// source-hash: 3e662d3bbddd658a2d3b6f6a012f7bf4d21db1a4e6f0de7d35c1bd71b309f5a0
#![doc = "SCE-MAP: forge_call_weight.scxml:8 :: _forge_body"]
// SCE-MAP: forge_call_weight.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

use super::forge_call_level;
use super::forge_call_status;
use sce_forge_runtime::lookup::lookup as sce_lookup;

const KEYS: [forge_call_status::ForgeCallStatus; 2] = [
    forge_call_status::ForgeCallStatus::Open,
    forge_call_status::ForgeCallStatus::Held,
];
const VALUES: [forge_call_level::ForgeCallLevel; 2] = [
    forge_call_level::ForgeCallLevel::Light,
    forge_call_level::ForgeCallLevel::Heavy,
];

pub fn lookup_weight(
    status: forge_call_status::ForgeCallStatus,
) -> Option<forge_call_level::ForgeCallLevel> {
    sce_lookup(&KEYS, &VALUES, status)
}
