// SCE-GENERATED — DO NOT EDIT
// source-hash: 9cad705a6b3f095f94a61c5a4662900a418cfa5bd31654ab943cef88874fa80b
// source: algorithm_shift_count_checked.scxml
// SCE-MAP: algorithm_shift_count_checked.scxml:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `AlgorithmShiftCountChecked`, a file of the
// module that calls it: a call is qualified by the enum
// (`AlgorithmShiftCountChecked.algorithmShiftCountChecked(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum AlgorithmShiftCountChecked {
    public static func algorithmShiftCountChecked(_ s: Int32, _ k: Int32, _ op: UInt8) throws(AlgorithmError) -> Int32 {
        var r: Int32 = 0
        if op == 0 {
            r = (try Checked.shl(s, k))
        }
        if op == 1 {
            r = (try Checked.shr(s, k))
        }
        if op == 2 {
            r = (try Checked.ushr(s, k))
        }
        return r
    }
}
