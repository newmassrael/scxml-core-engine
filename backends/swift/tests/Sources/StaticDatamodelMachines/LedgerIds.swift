// SCE-GENERATED — DO NOT EDIT
// source-hash: 047dc621dfd01b6348575becd26162c58fa20fa7680f076526766f91e97d2c5f
// SCE-MAP: algorithm_ledger_ids.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `LedgerIds`, a file of the
// module that calls it: a call is qualified by the enum
// (`LedgerIds.ledgerIds(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum LedgerIds {
    public static func ledgerIds(_ ids: [UInt32]) throws -> UInt32 {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        var sum: UInt32 = 0
        try sceFailure.throwIfFailed()
        for id in ids {
            sum = Checked.add(sceFailure, sum, id)
            try sceFailure.throwIfFailed()
        }
        do {
            let sceReturned: UInt32 = sum
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
