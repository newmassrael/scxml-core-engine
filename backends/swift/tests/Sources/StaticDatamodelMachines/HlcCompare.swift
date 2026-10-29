// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
// SCE-MAP: hlc_compare.scxml:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `HlcCompare`, a file of the
// module that calls it: a call is qualified by the enum
// (`HlcCompare.hlcCompare(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum HlcCompare {
    public static func hlcCompare(_ a: HlcTimestampPayload, _ b: HlcTimestampPayload) -> Int32 {
        if a.wallTime != b.wallTime {
            return a.wallTime < b.wallTime ? -1 : 1
        }
        if a.counter != b.counter {
            return a.counter < b.counter ? -1 : 1
        }
        if a.nodeId != b.nodeId {
            return a.nodeId < b.nodeId ? -1 : 1
        }
        return 0
    }
}
