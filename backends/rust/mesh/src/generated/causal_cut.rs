// SCE-GENERATED — DO NOT EDIT
// source-hash: f2f8682669670c7aaedbccb3102dd71d7b187eac29a32d3d20509a5bc070c12e
#![doc = "SCE-MAP: causal_cut.scxml:22 :: _forge_body"]
// SCE-MAP: causal_cut.scxml:22 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::causal_entry;
use super::causal_known;
use super::envelope_id;

use sce_portable_bytes::{CapacityExceeded, SceOwnedList};

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn causal_cut(
    members: u32,
    known: &[causal_known::CausalKnownPayload],
    out: &mut SceOwnedList<causal_entry::CausalEntryPayload, 64>,
) -> Result<(), CapacityExceeded> {
    let mut holders = SceOwnedList::<envelope_id::EnvelopeIdPayload, 64>::with_capacity(64);
    let mut counted: bool = false;
    for &mark in known.iter() {
        counted = false;
        for &holder in holders.iter() {
            if holder.hi == mark.holder_hi && holder.lo == mark.holder_lo {
                counted = true;
            }
        }
        if !counted {
            let member = envelope_id::EnvelopeIdPayload {
                hi: mark.holder_hi,
                lo: mark.holder_lo,
            };
            holders.push(member)?;
        }
    }
    out.clear();
    out.reserve(64);
    if (holders).len() as u32 >= members {
        let mut done: bool = false;
        let mut low: u64 = 0;
        let mut value: u64 = 0;
        for &author in known.iter() {
            done = false;
            for &cut in out.iter() {
                if cut.hi == author.hi && cut.lo == author.lo {
                    done = true;
                }
            }
            if !done {
                low = author.seq;
                for &peer in holders.iter() {
                    value = 0;
                    for &entry in known.iter() {
                        if entry.holder_hi == peer.hi
                            && entry.holder_lo == peer.lo
                            && entry.hi == author.hi
                            && entry.lo == author.lo
                        {
                            value = entry.seq;
                        }
                    }
                    if value < low {
                        low = value;
                    }
                }
                if low > 0 {
                    let stable = causal_entry::CausalEntryPayload {
                        hi: author.hi,
                        lo: author.lo,
                        seq: low,
                    };
                    out.push(stable)?;
                }
            }
        }
    }
    return Ok(());
}
