// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
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
    public static func merkleChangeHash(_ change: OrsetEntryPayload, _ kind: UInt8) throws -> UInt32 {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        do {
            let sceCond2 = !(change.wallTime >= 0)
            try sceFailure.throwIfFailed()
            if sceCond2 {
                throw AlgorithmError.precondition
            }
        }
        let element: UInt64 = change.element
        try sceFailure.throwIfFailed()
        let wall: UInt64 = Checked.narrow(sceFailure, change.wallTime, to: UInt64.self)
        try sceFailure.throwIfFailed()
        let counter: UInt64 = UInt64(change.counter)
        try sceFailure.throwIfFailed()
        let node: UInt64 = change.nodeId
        try sceFailure.throwIfFailed()
        var h: UInt64 = 0
        try sceFailure.throwIfFailed()
        h = Checked.rem(sceFailure, Checked.add(sceFailure, Checked.mul(sceFailure, h, 1000003), Checked.rem(sceFailure, element, 4294967296)), 4294967296)
        try sceFailure.throwIfFailed()
        h = Checked.rem(sceFailure, Checked.add(sceFailure, Checked.mul(sceFailure, h, 1000003), Checked.div(sceFailure, element, 4294967296)), 4294967296)
        try sceFailure.throwIfFailed()
        h = Checked.rem(sceFailure, Checked.add(sceFailure, Checked.mul(sceFailure, h, 1000003), Checked.rem(sceFailure, wall, 4294967296)), 4294967296)
        try sceFailure.throwIfFailed()
        h = Checked.rem(sceFailure, Checked.add(sceFailure, Checked.mul(sceFailure, h, 1000003), Checked.div(sceFailure, wall, 4294967296)), 4294967296)
        try sceFailure.throwIfFailed()
        h = Checked.rem(sceFailure, Checked.add(sceFailure, Checked.mul(sceFailure, h, 1000003), counter), 4294967296)
        try sceFailure.throwIfFailed()
        h = Checked.rem(sceFailure, Checked.add(sceFailure, Checked.mul(sceFailure, h, 1000003), Checked.rem(sceFailure, node, 4294967296)), 4294967296)
        try sceFailure.throwIfFailed()
        h = Checked.rem(sceFailure, Checked.add(sceFailure, Checked.mul(sceFailure, h, 1000003), Checked.div(sceFailure, node, 4294967296)), 4294967296)
        try sceFailure.throwIfFailed()
        h = Checked.rem(sceFailure, Checked.add(sceFailure, Checked.add(sceFailure, Checked.mul(sceFailure, h, 1000003), UInt64(kind)), 1), 4294967296)
        try sceFailure.throwIfFailed()
        h = h ^ Checked.div(sceFailure, h, 65536)
        try sceFailure.throwIfFailed()
        h = Checked.rem(sceFailure, Checked.mul(sceFailure, h, 2246822507), 4294967296)
        try sceFailure.throwIfFailed()
        h = h ^ Checked.div(sceFailure, h, 8192)
        try sceFailure.throwIfFailed()
        h = Checked.rem(sceFailure, Checked.mul(sceFailure, h, 3266489909), 4294967296)
        try sceFailure.throwIfFailed()
        h = h ^ Checked.div(sceFailure, h, 65536)
        try sceFailure.throwIfFailed()
        do {
            let sceReturned: UInt32 = Checked.narrow(sceFailure, h, to: UInt32.self)
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
