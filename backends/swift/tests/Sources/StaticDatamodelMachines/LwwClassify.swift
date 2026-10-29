// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
// SCE-MAP: lww_classify.scxml:24 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `LwwClassify`, a file of the
// module that calls it: a call is qualified by the enum
// (`LwwClassify.lwwClassify(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum LwwClassify {
    public static func lwwClassify(_ logged: [HlcTimestampPayload], _ msg: HlcTimestampPayload) -> UInt8 {
        var duplicate: Bool = false
        var later: Bool = false
        for x in logged {
            let c: Int32 = HlcCompare.hlcCompare(x, msg)
            if c == 0 {
                duplicate = true
            }
            if c > 0 {
                later = true
            }
        }
        return duplicate ? 2 : (later ? 1 : 0)
    }
}
