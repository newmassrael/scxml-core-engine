// SCE-GENERATED — DO NOT EDIT
// source-hash: f3a316aacf7403325399aec899293549220a4c98b78ad03825aff15e55acb6a9
// SCE-MAP: algorithm_bytes_squeeze.scxml:17 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `AlgorithmBytesSqueeze`, a file of the
// module that calls it: a call is qualified by the enum
// (`AlgorithmBytesSqueeze.algorithmBytesSqueeze(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum AlgorithmBytesSqueeze {
    public static func algorithmBytesSqueeze(_ data: [UInt8]) throws(AlgorithmError) -> [UInt8] {
        var out: [UInt8] = []
        out.reserveCapacity(16)
        for b in data {
            let n: UInt32 = UInt32(truncatingIfNeeded: out.count)
            if (try Checked.or(n == 0, { (try Checked.at(out, (try Checked.sub(n, 1)))) != b })) {
                out.append(b)
            }
        }
        return out
    }
}
