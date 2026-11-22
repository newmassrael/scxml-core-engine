// SCE-GENERATED — DO NOT EDIT
// source-hash: da008e408cd55b290322e8155517ad19a85661d68eef0def5f8178d4e21288f7
// source: algorithm_ledger_settles.scxml
// SCE-MAP: algorithm_ledger_settles.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `LedgerSettles`, a file of the
// module that calls it: a call is qualified by the enum
// (`LedgerSettles.ledgerSettles(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum LedgerSettles {
    public static func ledgerSettles(_ status: EnumItemStatus, _ amount: Int32) -> Bool {
        return status == EnumItemStatus.done && amount > 0
    }
}
