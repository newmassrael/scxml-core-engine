// SCE-GENERATED — DO NOT EDIT
// source-hash: 02a1f302f8985c8979205027eb5b236990d7a79fc8e74a6197aa1d9c5b50cead
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
