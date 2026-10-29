// SCE-GENERATED — DO NOT EDIT
// source-hash: 1178ba59796ca1f7c46f8569fd4bfda85c2b97db65f51728f3615d990d97dd1c
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
    public static func itemStatusCount(_ items: [SchemaItemPayload], _ status: EnumItemStatus) throws -> UInt32 {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        var count: UInt32 = 0
        try sceFailure.throwIfFailed()
        for e in items {
            do {
                let sceCond3 = e.status == status
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    count = Checked.add(sceFailure, count, 1)
                    try sceFailure.throwIfFailed()
                }
            }
        }
        do {
            let sceReturned: UInt32 = count
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
