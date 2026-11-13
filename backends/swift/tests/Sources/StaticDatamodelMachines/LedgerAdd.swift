// SCE-GENERATED — DO NOT EDIT
// source-hash: 138d54bef1f6b9b0e78600cb129e88342af00344f9fa58b6a90edf9adcaeb40d
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
    public static func ledgerAdd(_ entries: [SchemaEntryPayload], _ id: UInt32, _ amount: Int32) throws(AlgorithmError) -> [SchemaEntryPayload] {
        let fresh: SchemaEntryPayload = SchemaEntryPayload(id: id, amount: amount)
        var out: [SchemaEntryPayload] = []
        out.reserveCapacity(8)
        var seen: Bool = false
        for e in entries {
            if e.id == id {
                seen = true
                out.append(fresh)
            } else {
                out.append(e)
            }
        }
        if !seen {
            out.append(fresh)
        }
        return out
    }
}
