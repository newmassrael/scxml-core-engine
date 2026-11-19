// SCE-GENERATED — DO NOT EDIT
// source-hash: 32f3d7c11ac80be29d57e43f3bd96bf8c60dfd06eee3d5e4898056eaf3848397
// source: algorithm_ledger_larger.scxml
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
