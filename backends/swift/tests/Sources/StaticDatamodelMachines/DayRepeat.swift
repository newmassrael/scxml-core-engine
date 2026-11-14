// SCE-GENERATED — DO NOT EDIT
// source-hash: 65fb42bdd588f958b0fbb29ac261cf039d02606d7afaae0720c045624bd679ba
// SCE-MAP: algorithm_day_repeat.scxml:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `DayRepeat`, a file of the
// module that calls it: a call is qualified by the enum
// (`DayRepeat.dayRepeat(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum DayRepeat {
    public static func dayRepeat(_ value: UInt8, _ count: UInt8) throws(AlgorithmError) -> [UInt8] {
        var out: [UInt8] = []
        out.reserveCapacity(4)
        var i: UInt8 = 0
        while i < count {
            out.append(value)
            i = (try Checked.add(i, 1))
        }
        return out
    }
}
