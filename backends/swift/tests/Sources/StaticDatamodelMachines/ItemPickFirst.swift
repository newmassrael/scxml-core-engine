// SCE-GENERATED — DO NOT EDIT
// source-hash: 24f1c455257ea04f598b006a589c3dea642c1ea2e58b2dcf4b49b8cebf3f207e
// SCE-MAP: algorithm_item_pick_first.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `ItemPickFirst`, a file of the
// module that calls it: a call is qualified by the enum
// (`ItemPickFirst.itemPickFirst(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum ItemPickFirst {
    public static func itemPickFirst(_ items: [SchemaItemPickPayload], _ least: Int32) -> SchemaItemPickPayload? {
        var found: SchemaItemPickPayload? = nil
        for it in items {
            if found == nil && it.count >= least {
                found = it
            }
        }
        return found
    }
}
