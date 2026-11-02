// SCE-GENERATED — DO NOT EDIT
// source-hash: 814c7782483a2e1383636d59b115ce52cb83fc95e5c66e5ef4b90acbc7ce1d0e
// SCE-MAP: algorithm_single_sum.scxml:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `AlgorithmSingleSum`, a file of the
// module that calls it: a call is qualified by the enum
// (`AlgorithmSingleSum.algorithmSingleSum(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum AlgorithmSingleSum {
    public static func algorithmSingleSum(_ level: Float, _ step: Float) -> Float {
        var total: Float = 0.0
        total = level + step
        return total
    }
}
