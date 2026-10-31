// SCE-GENERATED — DO NOT EDIT
// source-hash: a026b55d57af6b3d6ae6556b172bd2bcd1c27127a3068fb36b62c3a2abbc391b
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
    public static func algorithmBytesSqueeze(_ data: [UInt8]) throws -> [UInt8] {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        var out: [UInt8] = []
        out.reserveCapacity(16)
        try sceFailure.throwIfFailed()
        for b in data {
            let n: UInt32 = UInt32(truncatingIfNeeded: out.count)
            try sceFailure.throwIfFailed()
            do {
                let sceCond3 = n == 0 || Checked.at(sceFailure, out, Checked.sub(sceFailure, n, 1), fallback: 0) != b
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    out.append(b)
                    try sceFailure.throwIfFailed()
                }
            }
        }
        do {
            let sceReturned: [UInt8] = out
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
