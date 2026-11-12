// SCE-GENERATED — DO NOT EDIT
// source-hash: d5a1192f784ff99a5aa95877e35eef81f703dd6fbaf723312b776f25706b4354
// SCE-MAP: utc_offset_at.scxml:25 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `UtcOffsetAt`, a file of the
// module that calls it: a call is qualified by the enum
// (`UtcOffsetAt.utcOffsetAt(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum UtcOffsetAt {
    public static func utcOffsetAt(_ starts: [Int64], _ offsets: [Int32], _ t: Int64) throws(AlgorithmError) -> Int32 {
        if !(starts.count == offsets.count && starts.count > 0) {
            throw AlgorithmError.precondition
        }
        if !(t >= (try Checked.at(starts, 0))) {
            throw AlgorithmError.precondition
        }
        var k: UInt32 = 0
        var i: UInt32 = 1
        while i < UInt32(truncatingIfNeeded: starts.count) {
            if !((try Checked.at(starts, i)) > (try Checked.at(starts, (try Checked.sub(i, 1))))) {
                throw AlgorithmError.precondition
            }
            if (try Checked.at(starts, i)) <= t {
                k = i
            }
            i = (try Checked.add(i, 1))
        }
        return (try Checked.at(offsets, k))
    }
}
