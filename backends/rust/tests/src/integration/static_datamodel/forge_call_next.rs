// SCE-GENERATED — DO NOT EDIT
// source-hash: d11fb6db69c27fed25f47a2f22a5bf7fb78da524d08f0c299e9cc553d32a6375
#![doc = "SCE-MAP: forge_call_next.scxml:7 :: _forge_body"]
// SCE-MAP: forge_call_next.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

use super::forge_call_status;

#[allow(clippy::comparison_to_empty)]
pub fn compute_after(
    status: forge_call_status::ForgeCallStatus,
) -> forge_call_status::ForgeCallStatus {
    if status == forge_call_status::ForgeCallStatus::Open {
        forge_call_status::ForgeCallStatus::Held
    } else {
        forge_call_status::ForgeCallStatus::Done
    }
}
