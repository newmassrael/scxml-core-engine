// SCE-GENERATED — DO NOT EDIT
// source-hash: ea764ab5524bc7a7e7d4553640b3fb26547d13f0d2053c0f5e7214161efbc6d0
// SCE-MAP: algorithm_ledger_total.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `LedgerTotal`, a file of the
// module that calls it: a call is qualified by the enum
// (`LedgerTotal.ledgerTotal(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum LedgerTotal {
    public static func ledgerTotal(_ entries: [SchemaEntryPayload]) throws -> SchemaTotalPayload {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        var t: SchemaTotalPayload = SchemaTotalPayload(count: 0, sum: 0)
        try sceFailure.throwIfFailed()
        for e in entries {
            t.count = Checked.add(sceFailure, t.count, 1)
            try sceFailure.throwIfFailed()
            t.sum = Checked.add(sceFailure, t.sum, e.amount)
            try sceFailure.throwIfFailed()
        }
        do {
            let sceReturned: SchemaTotalPayload = t
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
