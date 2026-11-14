// SCE-GENERATED — DO NOT EDIT
// source-hash: 1ee4c478fcd6af0bb675edd6c8e90348072aad4cd4e5fb9446a5e52706545b90
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
