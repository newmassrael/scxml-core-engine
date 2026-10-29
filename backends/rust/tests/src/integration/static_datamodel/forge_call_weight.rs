// SCE-GENERATED — DO NOT EDIT
// source-hash: 047dc621dfd01b6348575becd26162c58fa20fa7680f076526766f91e97d2c5f
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
