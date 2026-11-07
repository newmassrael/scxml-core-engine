// SCE-GENERATED — DO NOT EDIT
// source-hash: b2921c03007062914a1e69cc4069c6b54c8d66f97313a50f3246eed3d071c790
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
    public static func itemStatusSet(_ items: [SchemaItemPayload], _ id: UInt32, _ status: EnumItemStatus) throws -> [SchemaItemPayload] {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        let moved: SchemaItemPayload = SchemaItemPayload(id: id, status: status)
        try sceFailure.throwIfFailed()
        var out: [SchemaItemPayload] = []
        out.reserveCapacity(4)
        try sceFailure.throwIfFailed()
        for e in items {
            do {
                let sceCond3 = e.id == id
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    out.append(moved)
                    try sceFailure.throwIfFailed()
                } else {
                    out.append(e)
                    try sceFailure.throwIfFailed()
                }
            }
        }
        do {
            let sceReturned: [SchemaItemPayload] = out
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
