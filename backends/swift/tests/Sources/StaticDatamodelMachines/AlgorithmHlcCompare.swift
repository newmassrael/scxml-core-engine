// SCE-GENERATED — DO NOT EDIT
// source-hash: 7b98612ed0f35af10eaffafb7dd60088b78d6093971dc622b4d3161a336da46c
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
