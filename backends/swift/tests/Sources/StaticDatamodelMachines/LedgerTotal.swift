// SCE-GENERATED — DO NOT EDIT
// source-hash: 138d54bef1f6b9b0e78600cb129e88342af00344f9fa58b6a90edf9adcaeb40d
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
    public static func ledgerTotal(_ entries: [SchemaEntryPayload]) throws(AlgorithmError) -> SchemaTotalPayload {
        var t: SchemaTotalPayload = SchemaTotalPayload(count: 0, sum: 0)
        for e in entries {
            t.count = (try Checked.add(t.count, 1))
            t.sum = (try Checked.add(t.sum, e.amount))
        }
        return t
    }
}
