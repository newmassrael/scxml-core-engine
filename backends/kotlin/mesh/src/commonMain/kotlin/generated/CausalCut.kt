// SCE-GENERATED — DO NOT EDIT
// source-hash: f2f8682669670c7aaedbccb3102dd71d7b187eac29a32d3d20509a5bc070c12e
// SCE-MAP: causal_cut.scxml:22 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.causal_cut`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.causal_cut

import com.sce.generated.causal_entry.*
import com.sce.generated.causal_known.*
import com.sce.generated.envelope_id.*

fun causalCut(members: UInt, known: List<CausalKnownPayload>): List<CausalEntryPayload> {
    val holders = ArrayList<EnvelopeIdPayload>(64)
    var counted: Boolean = false
    for (mark in known) {
        counted = false
        for (holder in holders) {
            if (holder.hi == mark.holderHi && holder.lo == mark.holderLo) {
                counted = true
            }
        }
        if (!counted) {
            var member = EnvelopeIdPayload(hi = mark.holderHi, lo = mark.holderLo)
            holders.add(member)
        }
    }
    val out = ArrayList<CausalEntryPayload>(64)
    if ((holders).size.toUInt() >= members) {
        var done: Boolean = false
        var low: ULong = 0.toULong()
        var value: ULong = 0.toULong()
        for (author in known) {
            done = false
            for (cut in out) {
                if (cut.hi == author.hi && cut.lo == author.lo) {
                    done = true
                }
            }
            if (!done) {
                low = author.seq
                for (peer in holders) {
                    value = 0.toULong()
                    for (entry in known) {
                        if (entry.holderHi == peer.hi && entry.holderLo == peer.lo && entry.hi == author.hi && entry.lo == author.lo) {
                            value = entry.seq
                        }
                    }
                    if (value < low) {
                        low = value
                    }
                }
                if (low > 0.toULong()) {
                    var stable = CausalEntryPayload(hi = author.hi, lo = author.lo, seq = low)
                    out.add(stable)
                }
            }
        }
    }
    return out
}
