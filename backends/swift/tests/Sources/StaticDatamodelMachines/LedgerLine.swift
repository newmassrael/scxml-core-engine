// SCE-GENERATED — DO NOT EDIT
// source-hash: a4d51341b4c2faa9ecf7e674a095736d0b61e4dd0db21e43d561c13b65d4550d
// source: algorithm_ledger_line.scxml
// SCE-MAP: algorithm_ledger_line.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `LedgerLine`, a file of the
// module that calls it: a call is qualified by the enum
// (`LedgerLine.ledgerLine(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum LedgerLine {
    public static func ledgerLine(_ id: UInt32, _ amount: Int32) -> SchemaEntryPayload {
        let out: SchemaEntryPayload = SchemaEntryPayload(id: id, amount: amount)
        return out
    }
}
