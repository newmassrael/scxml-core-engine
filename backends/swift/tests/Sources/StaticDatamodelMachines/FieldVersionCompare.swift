// SCE-GENERATED — DO NOT EDIT
// source-hash: fb4f42a34e7ac40a28d8e5daee5d633c3c45dddac513816edf2bb83fdc85f91c
// SCE-MAP: field_version_compare.scxml:15 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `FieldVersionCompare`, a file of the
// module that calls it: a call is qualified by the enum
// (`FieldVersionCompare.fieldVersionCompare(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum FieldVersionCompare {
    public static func fieldVersionCompare(_ a: FieldVersionPayload, _ b: FieldVersionPayload) -> Int32 {
        let stampA: HlcTimestampPayload = HlcTimestampPayload(wallTime: a.wallTime, counter: a.counter, nodeId: a.nodeId)
        let stampB: HlcTimestampPayload = HlcTimestampPayload(wallTime: b.wallTime, counter: b.counter, nodeId: b.nodeId)
        return HlcCompare.hlcCompare(stampA, stampB)
    }
}
