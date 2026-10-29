// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
// SCE-MAP: merkle_prune.scxml:26 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `MerklePrune`, a file of the
// module that calls it: a call is qualified by the enum
// (`MerklePrune.merklePrune(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum MerklePrune {
    public static func merklePrune(_ nodes: [MerkleNodePayload], _ keep: UInt32) throws -> [MerkleNodePayload] {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        do {
            let sceCond2 = !(keep >= 1)
            try sceFailure.throwIfFailed()
            if sceCond2 {
                throw AlgorithmError.precondition
            }
        }
        var out: [MerkleNodePayload] = []
        out.reserveCapacity(2048)
        try sceFailure.throwIfFailed()
        var prevDepth: Int32 = -1
        try sceFailure.throwIfFailed()
        var prevPrefix: UInt64 = 0
        try sceFailure.throwIfFailed()
        for x in nodes {
            do {
                let sceCond3 = !(x.depth <= 30 && (Int32(x.depth) > prevDepth || Int32(x.depth) == prevDepth && x.prefix > prevPrefix))
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    throw AlgorithmError.precondition
                }
            }
            prevDepth = Int32(x.depth)
            try sceFailure.throwIfFailed()
            prevPrefix = x.prefix
            try sceFailure.throwIfFailed()
            var kept: Bool = true
            try sceFailure.throwIfFailed()
            var d: UInt32 = UInt32(x.depth)
            try sceFailure.throwIfFailed()
            var p: UInt64 = x.prefix
            try sceFailure.throwIfFailed()
            while true {
                let sceCond3 = kept && d >= 1
                try sceFailure.throwIfFailed()
                if !sceCond3 { break }
                var after: UInt32 = 0
                try sceFailure.throwIfFailed()
                for y in nodes {
                    do {
                        let sceCond5 = UInt32(y.depth) == d && (d == 1 || Checked.div(sceFailure, y.prefix, 3) == Checked.div(sceFailure, p, 3)) && y.prefix > p
                        try sceFailure.throwIfFailed()
                        if sceCond5 {
                            after = Checked.add(sceFailure, after, 1)
                            try sceFailure.throwIfFailed()
                        }
                    }
                }
                do {
                    let sceCond4 = after >= keep
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        kept = false
                        try sceFailure.throwIfFailed()
                    }
                }
                p = d >= 2 ? Checked.div(sceFailure, p, 3) : 0
                try sceFailure.throwIfFailed()
                d = Checked.sub(sceFailure, d, 1)
                try sceFailure.throwIfFailed()
            }
            do {
                let sceCond3 = kept
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    out.append(x)
                    try sceFailure.throwIfFailed()
                }
            }
        }
        do {
            let sceReturned: [MerkleNodePayload] = out
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
