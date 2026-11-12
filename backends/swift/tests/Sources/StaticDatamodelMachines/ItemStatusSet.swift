// SCE-GENERATED — DO NOT EDIT
// source-hash: d8c75e4d0233623996dc9a7cc3bdbe850af8d4f91ab023212ebd9f938cf7955f
// SCE-MAP: algorithm_item_status_set.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `ItemStatusSet`, a file of the
// module that calls it: a call is qualified by the enum
// (`ItemStatusSet.itemStatusSet(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum ItemStatusSet {
    public static func itemStatusSet(_ items: [SchemaItemPayload], _ id: UInt32, _ status: EnumItemStatus) throws(AlgorithmError) -> [SchemaItemPayload] {
        let moved: SchemaItemPayload = SchemaItemPayload(id: id, status: status)
        var out: [SchemaItemPayload] = []
        out.reserveCapacity(4)
        for e in items {
            if e.id == id {
                out.append(moved)
            } else {
                out.append(e)
            }
        }
        return out
    }
}
