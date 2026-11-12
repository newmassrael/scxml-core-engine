// SCE-GENERATED — DO NOT EDIT
// source-hash: fb4f42a34e7ac40a28d8e5daee5d633c3c45dddac513816edf2bb83fdc85f91c
// SCE-MAP: merkle_change_hash.scxml:40 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `MerkleChangeHash`, a file of the
// module that calls it: a call is qualified by the enum
// (`MerkleChangeHash.merkleChangeHash(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum MerkleChangeHash {
    public static func merkleChangeHash(_ change: OrsetEntryPayload, _ kind: UInt8) throws(AlgorithmError) -> UInt32 {
        if !(change.wallTime >= 0) {
            throw AlgorithmError.precondition
        }
        let element: UInt64 = change.element
        let wall: UInt64 = (try Checked.narrow(change.wallTime, to: UInt64.self))
        let counter: UInt64 = UInt64(change.counter)
        let node: UInt64 = change.nodeId
        var h: UInt64 = 0
        h = (try Checked.rem((try Checked.add((try Checked.mul(h, 1000003)), (try Checked.rem(element, 4294967296)))), 4294967296))
        h = (try Checked.rem((try Checked.add((try Checked.mul(h, 1000003)), (try Checked.div(element, 4294967296)))), 4294967296))
        h = (try Checked.rem((try Checked.add((try Checked.mul(h, 1000003)), (try Checked.rem(wall, 4294967296)))), 4294967296))
        h = (try Checked.rem((try Checked.add((try Checked.mul(h, 1000003)), (try Checked.div(wall, 4294967296)))), 4294967296))
        h = (try Checked.rem((try Checked.add((try Checked.mul(h, 1000003)), counter)), 4294967296))
        h = (try Checked.rem((try Checked.add((try Checked.mul(h, 1000003)), (try Checked.rem(node, 4294967296)))), 4294967296))
        h = (try Checked.rem((try Checked.add((try Checked.mul(h, 1000003)), (try Checked.div(node, 4294967296)))), 4294967296))
        h = (try Checked.rem((try Checked.add((try Checked.add((try Checked.mul(h, 1000003)), UInt64(kind))), 1)), 4294967296))
        h = h ^ (try Checked.div(h, 65536))
        h = (try Checked.rem((try Checked.mul(h, 2246822507)), 4294967296))
        h = h ^ (try Checked.div(h, 8192))
        h = (try Checked.rem((try Checked.mul(h, 3266489909)), 4294967296))
        h = h ^ (try Checked.div(h, 65536))
        return (try Checked.narrow(h, to: UInt32.self))
    }
}
