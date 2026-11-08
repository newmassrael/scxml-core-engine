// SCE-GENERATED — DO NOT EDIT
// source-hash: a0d514a5bb9842543c3a83f1c232fcee900a165f2a3ed19fb29e42d249c8aa22
// SCE-MAP: algorithm_ledger_has.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `LedgerHas`, a file of the
// module that calls it: a call is qualified by the enum
// (`LedgerHas.ledgerHas(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum LedgerHas {
    public static func ledgerHas(_ entries: [SchemaEntryPayload], _ id: UInt32) -> Bool {
        var found: Bool = false
        for e in entries {
            if e.id == id {
                found = true
            }
        }
        return found
    }
}
