// SCE-GENERATED — DO NOT EDIT
// source-hash: a9c299ab96f6e6c2a694137962a3f363e82399bf22fbd9f4aa0018913d55ad58
// SCE-MAP: algorithm_item_status_count.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `ItemStatusCount`, a file of the
// module that calls it: a call is qualified by the enum
// (`ItemStatusCount.itemStatusCount(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum ItemStatusCount {
    public static func itemStatusCount(_ items: [SchemaItemPayload], _ status: EnumItemStatus) throws(AlgorithmError) -> UInt32 {
        var count: UInt32 = 0
        for e in items {
            if e.status == status {
                count = (try Checked.add(count, 1))
            }
        }
        return count
    }
}
