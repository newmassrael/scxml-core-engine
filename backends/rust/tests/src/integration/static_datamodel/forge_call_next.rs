// SCE-GENERATED — DO NOT EDIT
// source-hash: a9c299ab96f6e6c2a694137962a3f363e82399bf22fbd9f4aa0018913d55ad58
#![doc = "SCE-MAP: forge_call_next.scxml:7 :: _forge_body"]
// SCE-MAP: forge_call_next.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

use super::forge_call_status;

#[allow(
    clippy::bool_comparison,
    clippy::comparison_to_empty,
    clippy::manual_range_contains
)]
pub fn compute_after(
    status: forge_call_status::ForgeCallStatus,
) -> forge_call_status::ForgeCallStatus {
    if status == forge_call_status::ForgeCallStatus::Open {
        forge_call_status::ForgeCallStatus::Held
    } else {
        forge_call_status::ForgeCallStatus::Done
    }
}
