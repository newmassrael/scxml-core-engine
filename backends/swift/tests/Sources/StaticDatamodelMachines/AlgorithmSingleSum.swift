// SCE-GENERATED — DO NOT EDIT
// source-hash: f52959f8eb76029a8e6cff0fae9c1d7f0469eb21cd52301f306b53afbc606d1a
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
