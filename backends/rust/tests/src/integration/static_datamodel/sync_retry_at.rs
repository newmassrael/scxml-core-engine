// SCE-GENERATED — DO NOT EDIT
// source-hash: c0802ab29f5b9eace062e478a717aaf037a02f7f1368f1ee6542066106d79c77
#![doc = "SCE-MAP: sync_retry_at.scxml:28 :: _forge_body"]
// SCE-MAP: sync_retry_at.scxml:28 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn sync_retry_at(
    previous: i64,
    kind: u8,
    status: i32,
    now: i64,
    retry_after: i64,
) -> Result<i64, sce_forge_runtime::algorithm::AlgorithmError> {
    if !(kind >= 1 && kind <= 9) {
        return Err(sce_forge_runtime::algorithm::AlgorithmError::Precondition);
    }
    if !(if kind == 4 {
        status >= 300 && status <= 599
    } else {
        status == 0
    }) {
        return Err(sce_forge_runtime::algorithm::AlgorithmError::Precondition);
    }
    if !(previous >= 0 && now >= 0 && retry_after >= 0) {
        return Err(sce_forge_runtime::algorithm::AlgorithmError::Precondition);
    }
    let mut asked: i64 = 0;
    if kind == 4 && status == 503 && retry_after > 0 {
        asked = sce_forge_runtime::algorithm::add::<i64>(now, retry_after)?;
    }
    if kind == 4 && status == 502 {
        asked = sce_forge_runtime::algorithm::add::<i64>(now, 900)?;
    }
    return Ok(if asked > previous { asked } else { previous });
}
