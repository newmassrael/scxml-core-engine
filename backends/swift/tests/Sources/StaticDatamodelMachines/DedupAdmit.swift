// SCE-GENERATED — DO NOT EDIT
// source-hash: 3e850e348999a821018727a6eab60f939f8f51a6e8133ca476ace471d64c40a6
// SCE-MAP: dedup_admit.scxml:26 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `DedupAdmit`, a file of the
// module that calls it: a call is qualified by the enum
// (`DedupAdmit.dedupAdmit(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum DedupAdmit {
    public static func dedupAdmit(_ window: [EnvelopeIdPayload], _ id: EnvelopeIdPayload, _ capacity: UInt32) throws -> [EnvelopeIdPayload] {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        do {
            let sceCond2 = !(capacity > 0 && UInt32(truncatingIfNeeded: window.count) <= capacity)
            try sceFailure.throwIfFailed()
            if sceCond2 {
                throw AlgorithmError.precondition
            }
        }
        var held: Bool = false
        try sceFailure.throwIfFailed()
        for x in window {
            do {
                let sceCond3 = x.hi == id.hi && x.lo == id.lo
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    held = true
                    try sceFailure.throwIfFailed()
                }
            }
        }
        var out: [EnvelopeIdPayload] = []
        out.reserveCapacity(256)
        try sceFailure.throwIfFailed()
        let skip: UInt32 = held || UInt32(truncatingIfNeeded: window.count) < capacity ? 0 : 1
        try sceFailure.throwIfFailed()
        var i: UInt32 = 0
        try sceFailure.throwIfFailed()
        for kept in window {
            do {
                let sceCond3 = i >= skip
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    out.append(kept)
                    try sceFailure.throwIfFailed()
                }
            }
            i = Checked.add(sceFailure, i, 1)
            try sceFailure.throwIfFailed()
        }
        do {
            let sceCond2 = !held
            try sceFailure.throwIfFailed()
            if sceCond2 {
                out.append(id)
                try sceFailure.throwIfFailed()
            }
        }
        do {
            let sceReturned: [EnvelopeIdPayload] = out
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
