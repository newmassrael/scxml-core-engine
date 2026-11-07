// SCE-GENERATED — DO NOT EDIT
// source-hash: b2921c03007062914a1e69cc4069c6b54c8d66f97313a50f3246eed3d071c790
// SCE-MAP: algorithm_ledger_add.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `LedgerAdd`, a file of the
// module that calls it: a call is qualified by the enum
// (`LedgerAdd.ledgerAdd(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum LedgerAdd {
    public static func ledgerAdd(_ entries: [SchemaEntryPayload], _ id: UInt32, _ amount: Int32) throws -> [SchemaEntryPayload] {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        let fresh: SchemaEntryPayload = SchemaEntryPayload(id: id, amount: amount)
        try sceFailure.throwIfFailed()
        var out: [SchemaEntryPayload] = []
        out.reserveCapacity(8)
        try sceFailure.throwIfFailed()
        var seen: Bool = false
        try sceFailure.throwIfFailed()
        for e in entries {
            do {
                let sceCond3 = e.id == id
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    seen = true
                    try sceFailure.throwIfFailed()
                    out.append(fresh)
                    try sceFailure.throwIfFailed()
                } else {
                    out.append(e)
                    try sceFailure.throwIfFailed()
                }
            }
        }
        do {
            let sceCond2 = !seen
            try sceFailure.throwIfFailed()
            if sceCond2 {
                out.append(fresh)
                try sceFailure.throwIfFailed()
            }
        }
        do {
            let sceReturned: [SchemaEntryPayload] = out
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
