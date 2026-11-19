// SCE-GENERATED — DO NOT EDIT
// source-hash: 2bb0efdf4c49cba16891afcd8fa6d28ec26e10e5cb60b4e26e18ddf0238b7d94
// source: sce:std/merge/merkle_diff.scxml
// SCE-MAP: merkle_diff.scxml:39 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `MerkleDiff`, a file of the
// module that calls it: a call is qualified by the enum
// (`MerkleDiff.merkleDiff(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum MerkleDiff {
    public static func merkleDiff(_ a: [MerkleNodePayload], _ b: [MerkleNodePayload]) throws(AlgorithmError) -> MerkleDivergencePayload {
        var rootA: UInt32 = 0
        for rootOfA in a {
            if rootOfA.depth == 0 {
                rootA = rootOfA.hash
            }
        }
        var rootB: UInt32 = 0
        for rootOfB in b {
            if rootOfB.depth == 0 {
                rootB = rootOfB.hash
            }
        }
        var result: MerkleDivergencePayload = MerkleDivergencePayload(differs: rootA != rootB, millis: 0)
        if !result.differs {
            return result
        }
        var d: UInt32 = 0
        var k: UInt64 = 0
        var span: UInt64 = 205891132094649
        var descending: Bool = true
        while descending && d < 30 {
            descending = false
            var started: Bool = false
            var last: UInt64 = 0
            var scanning: Bool = true
            while scanning {
                var found: Bool = false
                var next: UInt64 = 0
                for childOfA in a {
                    if (try Checked.and(UInt32(childOfA.depth) == (try Checked.add(d, 1)), { () throws(AlgorithmError) -> Bool in (try Checked.or(d == 0, { () throws(AlgorithmError) -> Bool in (try Checked.div(childOfA.prefix, 3)) == k })) })) && (!started || childOfA.prefix > last) && (!found || childOfA.prefix < next) {
                        found = true
                        next = childOfA.prefix
                    }
                }
                for childOfB in b {
                    if (try Checked.and(UInt32(childOfB.depth) == (try Checked.add(d, 1)), { () throws(AlgorithmError) -> Bool in (try Checked.or(d == 0, { () throws(AlgorithmError) -> Bool in (try Checked.div(childOfB.prefix, 3)) == k })) })) && (!started || childOfB.prefix > last) && (!found || childOfB.prefix < next) {
                        found = true
                        next = childOfB.prefix
                    }
                }
                scanning = found
                if found {
                    var inA: Bool = false
                    var hashA: UInt32 = 0
                    for matchInA in a {
                        if UInt32(matchInA.depth) == (try Checked.add(d, 1)) && matchInA.prefix == next {
                            inA = true
                            hashA = matchInA.hash
                        }
                    }
                    var inB: Bool = false
                    var hashB: UInt32 = 0
                    for matchInB in b {
                        if UInt32(matchInB.depth) == (try Checked.add(d, 1)) && matchInB.prefix == next {
                            inB = true
                            hashB = matchInB.hash
                        }
                    }
                    if !inA || !inB {
                        scanning = false
                    } else {
                        if hashA != hashB {
                            scanning = false
                            descending = true
                            k = next
                        } else {
                            started = true
                            last = next
                        }
                    }
                }
            }
            if descending {
                d = (try Checked.add(d, 1))
                span = (try Checked.div(span, 3))
            }
        }
        let minute: Int64 = (try Checked.narrow((try Checked.mul(k, span)), to: Int64.self))
        result.millis = (try Checked.mul(minute, 60000))
        return result
    }
}
