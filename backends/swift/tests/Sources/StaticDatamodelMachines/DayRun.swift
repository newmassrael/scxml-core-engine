// SCE-GENERATED — DO NOT EDIT
// source-hash: c2e56e3837f180f07a5c044369108d947c57fa7c065dd063d76bf9f37c590bee
// SCE-MAP: algorithm_day_run.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `DayRun`, a file of the
// module that calls it: a call is qualified by the enum
// (`DayRun.dayRun(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum DayRun {
    public static func dayRun(_ first: UInt8, _ count: UInt8) throws(AlgorithmError) -> [UInt8] {
        var out: [UInt8] = []
        out.reserveCapacity(8)
        var i: UInt8 = 0
        while i < count {
            out.append((try Checked.add(first, i)))
            i = (try Checked.add(i, 1))
        }
        return out
    }
}
