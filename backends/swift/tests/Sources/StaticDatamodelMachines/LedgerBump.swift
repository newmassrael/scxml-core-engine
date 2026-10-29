// SCE-GENERATED — DO NOT EDIT
// source-hash: adbae20e9ca5d62b09870f4a9e5e88119b90dbcf69963958bb021c42c5c2c6a4
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
    public static func ledgerBump(_ total: SchemaTotalPayload, _ delta: Int32) throws -> SchemaTotalPayload {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        let next: SchemaTotalPayload = SchemaTotalPayload(count: Checked.add(sceFailure, total.count, 1), sum: Checked.add(sceFailure, total.sum, delta))
        try sceFailure.throwIfFailed()
        do {
            let sceReturned: SchemaTotalPayload = next
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
