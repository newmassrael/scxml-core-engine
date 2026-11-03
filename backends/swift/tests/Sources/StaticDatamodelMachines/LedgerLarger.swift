// SCE-GENERATED — DO NOT EDIT
// source-hash: ef94c112b4026dfab705629195b17ee265cdc572de6e8fceff328262c2af0613
// SCE-MAP: algorithm_ledger_larger.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `LedgerLarger`, a file of the
// module that calls it: a call is qualified by the enum
// (`LedgerLarger.ledgerLarger(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum LedgerLarger {
    public static func ledgerLarger(_ a: SchemaEntryPayload, _ b: SchemaEntryPayload) -> SchemaEntryPayload {
        var out: SchemaEntryPayload = SchemaEntryPayload(id: a.id, amount: a.amount)
        if b.amount > a.amount {
            out.id = b.id
            out.amount = b.amount
        }
        return out
    }
}
