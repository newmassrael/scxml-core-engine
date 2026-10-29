// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
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
