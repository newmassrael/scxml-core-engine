// SCE-GENERATED — DO NOT EDIT
// source-hash: db91e467a409f671155f800587da9a1dc9cebc08ae59c6337f154a5d7f66ec96
#![doc = "SCE-MAP: sync_delete_outcome.scxml:28 :: _forge_body"]
// SCE-MAP: sync_delete_outcome.scxml:28 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn sync_delete_outcome(
    kind: u8,
    status: i32,
) -> Result<u8, sce_forge_runtime::algorithm::AlgorithmError> {
    if !(kind >= 1 && kind <= 9) {
        return Err(sce_forge_runtime::algorithm::AlgorithmError::Precondition);
    }
    if !(if kind == 4 {
        status >= 200 && status <= 599
    } else {
        status == 0
    }) {
        return Err(sce_forge_runtime::algorithm::AlgorithmError::Precondition);
    }
    let mut outcome: u8 = 2;
    if kind == 4 && status < 500 {
        outcome = if status < 300 || status == 404 || status == 410 {
            0
        } else {
            1
        };
    }
    return Ok(outcome);
}
