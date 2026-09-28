// SCE-GENERATED — DO NOT EDIT
// source-hash: c0802ab29f5b9eace062e478a717aaf037a02f7f1368f1ee6542066106d79c77
#![doc = "SCE-MAP: sync_upload_outcome.scxml:24 :: _forge_body"]
// SCE-MAP: sync_upload_outcome.scxml:24 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn sync_upload_outcome(
    create: bool,
    status: i32,
    dav_error: bool,
) -> Result<u8, sce_forge_runtime::algorithm::AlgorithmError> {
    if !(status >= 200 && status <= 599) {
        return Err(sce_forge_runtime::algorithm::AlgorithmError::Precondition);
    }
    let mut outcome: u8 = 2;
    if status < 300 {
        outcome = 0;
    }
    if status == 403 || status == 412 || status == 409 && dav_error {
        outcome = 1;
    }
    if !create && (status == 404 || status == 410 || status == 409) {
        outcome = 1;
    }
    return Ok(outcome);
}
