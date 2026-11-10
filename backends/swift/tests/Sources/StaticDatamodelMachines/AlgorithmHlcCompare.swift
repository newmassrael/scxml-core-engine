// SCE-GENERATED — DO NOT EDIT
// source-hash: f3a316aacf7403325399aec899293549220a4c98b78ad03825aff15e55acb6a9
// SCE-MAP: algorithm_hlc_compare.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `AlgorithmHlcCompare`, a file of the
// module that calls it: a call is qualified by the enum
// (`AlgorithmHlcCompare.algorithmHlcCompare(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum AlgorithmHlcCompare {
    public static func algorithmHlcCompare(_ a: EventSchemaHlcPayload, _ b: EventSchemaHlcPayload) -> UInt8 {
        if a.wallTime != b.wallTime {
            return a.wallTime > b.wallTime ? 1 : 0
        }
        if a.counter != b.counter {
            return a.counter > b.counter ? 1 : 0
        }
        return a.nodeId > b.nodeId ? 1 : 0
    }
}
