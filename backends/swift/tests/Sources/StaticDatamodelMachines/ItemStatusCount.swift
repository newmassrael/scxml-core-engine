// SCE-GENERATED — DO NOT EDIT
// source-hash: 138d54bef1f6b9b0e78600cb129e88342af00344f9fa58b6a90edf9adcaeb40d
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
