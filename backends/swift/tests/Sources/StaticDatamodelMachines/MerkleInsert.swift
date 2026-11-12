// SCE-GENERATED — DO NOT EDIT
// source-hash: fb4f42a34e7ac40a28d8e5daee5d633c3c45dddac513816edf2bb83fdc85f91c
// SCE-MAP: merkle_insert.scxml:33 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `MerkleInsert`, a file of the
// module that calls it: a call is qualified by the enum
// (`MerkleInsert.merkleInsert(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum MerkleInsert {
    public static func merkleInsert(_ nodes: [MerkleNodePayload], _ minute: Int64, _ hash: UInt32) throws(AlgorithmError) -> [MerkleNodePayload] {
        if !(minute >= 0 && minute <= 153722867280912) {
            throw AlgorithmError.precondition
        }
        var out: [MerkleNodePayload] = []
        out.reserveCapacity(2048)
        var pd: UInt32 = 0
        var pp: UInt64 = 0
        var div: Int64 = 68630377364883
        var prevDepth: Int32 = -1
        var prevPrefix: UInt64 = 0
        for x in nodes {
            if !(x.depth <= 30 && (Int32(x.depth) > prevDepth || Int32(x.depth) == prevDepth && x.prefix > prevPrefix)) {
                throw AlgorithmError.precondition
            }
            prevDepth = Int32(x.depth)
            prevPrefix = x.prefix
            while pd <= 30 && (pd < UInt32(x.depth) || pd == UInt32(x.depth) && pp < x.prefix) {
                let created: MerkleNodePayload = MerkleNodePayload(depth: (try Checked.narrow(pd, to: UInt8.self)), prefix: pp, hash: hash)
                out.append(created)
                pd = (try Checked.add(pd, 1))
                pp = (try Checked.narrow((try Checked.div(minute, div)), to: UInt64.self))
                if div > 1 {
                    div = (try Checked.div(div, 3))
                }
            }
            if pd <= 30 && pd == UInt32(x.depth) && pp == x.prefix {
                let merged: MerkleNodePayload = MerkleNodePayload(depth: x.depth, prefix: x.prefix, hash: x.hash ^ hash)
                out.append(merged)
                pd = (try Checked.add(pd, 1))
                pp = (try Checked.narrow((try Checked.div(minute, div)), to: UInt64.self))
                if div > 1 {
                    div = (try Checked.div(div, 3))
                }
            } else {
                out.append(x)
            }
        }
        while pd <= 30 {
            let tail: MerkleNodePayload = MerkleNodePayload(depth: (try Checked.narrow(pd, to: UInt8.self)), prefix: pp, hash: hash)
            out.append(tail)
            pd = (try Checked.add(pd, 1))
            pp = (try Checked.narrow((try Checked.div(minute, div)), to: UInt64.self))
            if div > 1 {
                div = (try Checked.div(div, 3))
            }
        }
        return out
    }
}
