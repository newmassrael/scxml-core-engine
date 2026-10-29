// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
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
    public static func merkleInsert(_ nodes: [MerkleNodePayload], _ minute: Int64, _ hash: UInt32) throws -> [MerkleNodePayload] {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        do {
            let sceCond2 = !(minute >= 0 && minute <= 153722867280912)
            try sceFailure.throwIfFailed()
            if sceCond2 {
                throw AlgorithmError.precondition
            }
        }
        var out: [MerkleNodePayload] = []
        out.reserveCapacity(2048)
        try sceFailure.throwIfFailed()
        var pd: UInt32 = 0
        try sceFailure.throwIfFailed()
        var pp: UInt64 = 0
        try sceFailure.throwIfFailed()
        var div: Int64 = 68630377364883
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
            while true {
                let sceCond3 = pd <= 30 && (pd < UInt32(x.depth) || pd == UInt32(x.depth) && pp < x.prefix)
                try sceFailure.throwIfFailed()
                if !sceCond3 { break }
                let created: MerkleNodePayload = MerkleNodePayload(depth: Checked.narrow(sceFailure, pd, to: UInt8.self), prefix: pp, hash: hash)
                try sceFailure.throwIfFailed()
                out.append(created)
                try sceFailure.throwIfFailed()
                pd = Checked.add(sceFailure, pd, 1)
                try sceFailure.throwIfFailed()
                pp = Checked.narrow(sceFailure, Checked.div(sceFailure, minute, div), to: UInt64.self)
                try sceFailure.throwIfFailed()
                do {
                    let sceCond4 = div > 1
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        div = Checked.div(sceFailure, div, 3)
                        try sceFailure.throwIfFailed()
                    }
                }
            }
            do {
                let sceCond3 = pd <= 30 && pd == UInt32(x.depth) && pp == x.prefix
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    let merged: MerkleNodePayload = MerkleNodePayload(depth: x.depth, prefix: x.prefix, hash: x.hash ^ hash)
                    try sceFailure.throwIfFailed()
                    out.append(merged)
                    try sceFailure.throwIfFailed()
                    pd = Checked.add(sceFailure, pd, 1)
                    try sceFailure.throwIfFailed()
                    pp = Checked.narrow(sceFailure, Checked.div(sceFailure, minute, div), to: UInt64.self)
                    try sceFailure.throwIfFailed()
                    do {
                        let sceCond5 = div > 1
                        try sceFailure.throwIfFailed()
                        if sceCond5 {
                            div = Checked.div(sceFailure, div, 3)
                            try sceFailure.throwIfFailed()
                        }
                    }
                } else {
                    out.append(x)
                    try sceFailure.throwIfFailed()
                }
            }
        }
        while true {
            let sceCond2 = pd <= 30
            try sceFailure.throwIfFailed()
            if !sceCond2 { break }
            let tail: MerkleNodePayload = MerkleNodePayload(depth: Checked.narrow(sceFailure, pd, to: UInt8.self), prefix: pp, hash: hash)
            try sceFailure.throwIfFailed()
            out.append(tail)
            try sceFailure.throwIfFailed()
            pd = Checked.add(sceFailure, pd, 1)
            try sceFailure.throwIfFailed()
            pp = Checked.narrow(sceFailure, Checked.div(sceFailure, minute, div), to: UInt64.self)
            try sceFailure.throwIfFailed()
            do {
                let sceCond3 = div > 1
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    div = Checked.div(sceFailure, div, 3)
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
