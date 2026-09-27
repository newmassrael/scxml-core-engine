// SCE-GENERATED — DO NOT EDIT
// source-hash: f6148b213f9ed141b381c5b54aabc8b1a585f1b96413497e91bf4f50d5cda1ce
#![doc = "SCE-MAP: dedup_admit.scxml:26 :: _forge_body"]
// SCE-MAP: dedup_admit.scxml:26 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::envelope_id;

use sce_portable_bytes::SceOwnedList;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn dedup_admit(
    window: &[envelope_id::EnvelopeIdPayload],
    id: envelope_id::EnvelopeIdPayload,
    capacity: u32,
) -> Result<
    SceOwnedList<envelope_id::EnvelopeIdPayload, 256>,
    sce_forge_runtime::algorithm::AlgorithmError,
> {
    if !(capacity > 0 && (window).len() as u32 <= capacity) {
        return Err(sce_forge_runtime::algorithm::AlgorithmError::Precondition);
    }
    let mut held: bool = false;
    for &x in window.iter() {
        if x.hi == id.hi && x.lo == id.lo {
            held = true;
        }
    }
    let mut out: SceOwnedList<envelope_id::EnvelopeIdPayload, 256> = SceOwnedList::new();
    let skip: u32 = if held || ((window).len() as u32) < capacity {
        0
    } else {
        1
    };
    let mut i: u32 = 0;
    for &kept in window.iter() {
        if i >= skip {
            out.push(kept)?;
        }
        i = sce_forge_runtime::algorithm::add::<u32>(i, 1)?;
    }
    if !held {
        out.push(id)?;
    }
    return Ok(out);
}
