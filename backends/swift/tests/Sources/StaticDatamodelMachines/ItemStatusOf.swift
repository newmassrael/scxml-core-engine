// SCE-GENERATED — DO NOT EDIT
// source-hash: b2921c03007062914a1e69cc4069c6b54c8d66f97313a50f3246eed3d071c790
// SCE-MAP: algorithm_item_status_of.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `ItemStatusOf`, a file of the
// module that calls it: a call is qualified by the enum
// (`ItemStatusOf.itemStatusOf(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum ItemStatusOf {
    public static func itemStatusOf(_ items: [SchemaItemPayload], _ id: UInt32) -> EnumItemStatus {
        var found: EnumItemStatus = EnumItemStatus.open
        for e in items {
            if e.id == id {
                found = e.status
            }
        }
        return found
    }
}
