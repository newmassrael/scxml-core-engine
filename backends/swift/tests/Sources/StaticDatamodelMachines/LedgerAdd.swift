// SCE-GENERATED — DO NOT EDIT
// source-hash: d8c75e4d0233623996dc9a7cc3bdbe850af8d4f91ab023212ebd9f938cf7955f
// SCE-MAP: algorithm_ledger_add.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `LedgerAdd`, a file of the
// module that calls it: a call is qualified by the enum
// (`LedgerAdd.ledgerAdd(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum LedgerAdd {
    public static func ledgerAdd(_ entries: [SchemaEntryPayload], _ id: UInt32, _ amount: Int32) throws(AlgorithmError) -> [SchemaEntryPayload] {
        let fresh: SchemaEntryPayload = SchemaEntryPayload(id: id, amount: amount)
        var out: [SchemaEntryPayload] = []
        out.reserveCapacity(8)
        var seen: Bool = false
        for e in entries {
            if e.id == id {
                seen = true
                out.append(fresh)
            } else {
                out.append(e)
            }
        }
        if !seen {
            out.append(fresh)
        }
        return out
    }
}
