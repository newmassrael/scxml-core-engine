// SCE-GENERATED — DO NOT EDIT
// source-hash: d05ce240c5f3b48b3c83b6bee1ba58feef40d57c310e9a29ef5279604bbaaa42
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
