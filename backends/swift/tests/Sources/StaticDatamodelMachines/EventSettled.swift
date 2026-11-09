// SCE-GENERATED — DO NOT EDIT
// source-hash: 39c188440acd2a30e1c96c90e825a582fc7011a2c558c89e70ea674bb5bda40e
// SCE-MAP: event_settled.scxml:17 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `EventSettled`, a file of the
// module that calls it: a call is qualified by the enum
// (`EventSettled.eventSettled(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum EventSettled {
    public static func eventSettled(_ adds: [OrsetEntryPayload], _ tombstones: [OrsetEntryPayload], _ before: Int64) throws -> UInt32 {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        var h: UInt32 = 0
        try sceFailure.throwIfFailed()
        for a in adds {
            do {
                let sceCond3 = a.wallTime <= before
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    h = h ^ Checked.take(sceFailure, fallback: 0, { try MerkleChangeHash.merkleChangeHash(a, 0) })
                    try sceFailure.throwIfFailed()
                }
            }
        }
        for t in tombstones {
            do {
                let sceCond3 = t.wallTime <= before
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    h = h ^ Checked.take(sceFailure, fallback: 0, { try MerkleChangeHash.merkleChangeHash(t, 1) })
                    try sceFailure.throwIfFailed()
                }
            }
        }
        do {
            let sceReturned: UInt32 = h
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
