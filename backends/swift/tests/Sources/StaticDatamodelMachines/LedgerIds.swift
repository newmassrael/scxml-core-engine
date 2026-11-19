// SCE-GENERATED — DO NOT EDIT
// source-hash: 6a327a9ee376f39fed99384c28d6c7322bb1b8f2f648e5239005cc68b7e61b33
// source: algorithm_ledger_ids.scxml
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
    public static func ledgerIds(_ ids: [UInt32]) throws(AlgorithmError) -> UInt32 {
        var sum: UInt32 = 0
        for id in ids {
            sum = (try Checked.add(sum, id))
        }
        return sum
    }
}
