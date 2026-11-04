// SCE-GENERATED — DO NOT EDIT
// source-hash: 8de0fa86739ee80dd1cde104a3417a50f438f9eccff41679327b28a2157e9381
// SCE-MAP: algorithm_ledger_amount.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `LedgerAmount`, a file of the
// module that calls it: a call is qualified by the enum
// (`LedgerAmount.ledgerAmount(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum LedgerAmount {
    public static func ledgerAmount(_ line: SchemaEntryPayload) -> Int32 {
        return line.amount
    }
}
