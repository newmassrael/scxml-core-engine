// SCE-GENERATED — DO NOT EDIT
// source-hash: 3e662d3bbddd658a2d3b6f6a012f7bf4d21db1a4e6f0de7d35c1bd71b309f5a0
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
