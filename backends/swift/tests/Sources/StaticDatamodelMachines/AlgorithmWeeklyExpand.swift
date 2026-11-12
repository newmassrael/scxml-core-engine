// SCE-GENERATED — DO NOT EDIT
// source-hash: f3a316aacf7403325399aec899293549220a4c98b78ad03825aff15e55acb6a9
// SCE-MAP: algorithm_weekly_expand.scxml:19 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `AlgorithmWeeklyExpand`, a file of the
// module that calls it: a call is qualified by the enum
// (`AlgorithmWeeklyExpand.algorithmWeeklyExpand(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum AlgorithmWeeklyExpand {
    public static func algorithmWeeklyExpand(_ dtstart: Int64, _ mask: UInt8, _ interval: Int64, _ window_start: Int64, _ window_end: Int64) throws(AlgorithmError) -> [Int64] {
        var out: [Int64] = []
        out.reserveCapacity(366)
        var d: Int64 = window_start > dtstart ? window_start : dtstart
        while d < window_end {
            let weekday: Int64 = (try Checked.rem((try Checked.add(d, 3)), 7))
            let week: Int64 = (try Checked.div((try Checked.sub(d, dtstart)), 7))
            if (try Checked.and(((Int64(mask) >> weekday) & 1) == 1, { () throws(AlgorithmError) -> Bool in (try Checked.rem(week, interval)) == 0 })) {
                out.append(d)
            }
            d = (try Checked.add(d, 1))
        }
        return out
    }
}
