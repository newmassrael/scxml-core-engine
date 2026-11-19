// SCE-GENERATED — DO NOT EDIT
// source-hash: ffaccc84c5109acb797ef9f96f66211c2ea59fe0a393779364fe2a3bdbc13238
// source: sce:std/merge/merkle_prune.scxml
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
    public static func merklePrune(_ nodes: [MerkleNodePayload], _ keep: UInt32) throws(AlgorithmError) -> [MerkleNodePayload] {
        if !(keep >= 1) {
            throw AlgorithmError.precondition
        }
        var out: [MerkleNodePayload] = []
        out.reserveCapacity(2048)
        var prevDepth: Int32 = -1
        var prevPrefix: UInt64 = 0
        for x in nodes {
            if !(x.depth <= 30 && (Int32(x.depth) > prevDepth || Int32(x.depth) == prevDepth && x.prefix > prevPrefix)) {
                throw AlgorithmError.precondition
            }
            prevDepth = Int32(x.depth)
            prevPrefix = x.prefix
            var kept: Bool = true
            var d: UInt32 = UInt32(x.depth)
            var p: UInt64 = x.prefix
            while kept && d >= 1 {
                var after: UInt32 = 0
                for y in nodes {
                    if (try Checked.and(UInt32(y.depth) == d, { () throws(AlgorithmError) -> Bool in (try Checked.or(d == 1, { () throws(AlgorithmError) -> Bool in (try Checked.div(y.prefix, 3)) == (try Checked.div(p, 3)) })) })) && y.prefix > p {
                        after = (try Checked.add(after, 1))
                    }
                }
                if after >= keep {
                    kept = false
                }
                p = d >= 2 ? (try Checked.div(p, 3)) : 0
                d = (try Checked.sub(d, 1))
            }
            if kept {
                out.append(x)
            }
        }
        return out
    }
}
