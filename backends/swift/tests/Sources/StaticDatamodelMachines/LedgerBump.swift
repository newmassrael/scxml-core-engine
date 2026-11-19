// SCE-GENERATED — DO NOT EDIT
// source-hash: c2e56e3837f180f07a5c044369108d947c57fa7c065dd063d76bf9f37c590bee
// SCE-MAP: algorithm_ledger_bump.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `LedgerBump`, a file of the
// module that calls it: a call is qualified by the enum
// (`LedgerBump.ledgerBump(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum LedgerBump {
    public static func ledgerBump(_ total: SchemaTotalPayload, _ delta: Int32) throws(AlgorithmError) -> SchemaTotalPayload {
        let next: SchemaTotalPayload = SchemaTotalPayload(count: (try Checked.add(total.count, 1)), sum: (try Checked.add(total.sum, delta)))
        return next
    }
}
