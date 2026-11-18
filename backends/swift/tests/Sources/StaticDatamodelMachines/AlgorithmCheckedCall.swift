// SCE-GENERATED — DO NOT EDIT
// source-hash: 2e14e61a19815f19d3d0dbb6fe1326691e48aee7d4809e1595ab8383fb1a9fb7
// SCE-MAP: algorithm_checked_call.scxml:16 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `AlgorithmCheckedCall`, a file of the
// module that calls it: a call is qualified by the enum
// (`AlgorithmCheckedCall.algorithmCheckedCall(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum AlgorithmCheckedCall {
    public static func algorithmCheckedCall(_ a: Int32, _ b: Int32, _ op: UInt8) throws(AlgorithmError) -> Int32 {
        (try AlgorithmCheckedArith.algorithmCheckedArith(a, b, op))
        let s: Int32 = (try AlgorithmCheckedArith.algorithmCheckedArith(a, b, 0))
        if (try AlgorithmCheckedArith.algorithmCheckedArith(s, s, 2)) > 100 {
            return 0
        }
        return (try Checked.add(s, 1))
    }
}
