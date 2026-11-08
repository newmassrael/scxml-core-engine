// SCE-GENERATED — DO NOT EDIT
// source-hash: a0d514a5bb9842543c3a83f1c232fcee900a165f2a3ed19fb29e42d249c8aa22
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
