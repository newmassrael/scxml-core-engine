// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
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
    public static func merkleDiff(_ a: [MerkleNodePayload], _ b: [MerkleNodePayload]) throws -> MerkleDivergencePayload {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        var rootA: UInt32 = 0
        try sceFailure.throwIfFailed()
        for rootOfA in a {
            do {
                let sceCond3 = rootOfA.depth == 0
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    rootA = rootOfA.hash
                    try sceFailure.throwIfFailed()
                }
            }
        }
        var rootB: UInt32 = 0
        try sceFailure.throwIfFailed()
        for rootOfB in b {
            do {
                let sceCond3 = rootOfB.depth == 0
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    rootB = rootOfB.hash
                    try sceFailure.throwIfFailed()
                }
            }
        }
        var result: MerkleDivergencePayload = MerkleDivergencePayload(differs: rootA != rootB, millis: 0)
        try sceFailure.throwIfFailed()
        do {
            let sceCond2 = !result.differs
            try sceFailure.throwIfFailed()
            if sceCond2 {
                do {
                    let sceReturned: MerkleDivergencePayload = result
                    try sceFailure.throwIfFailed()
                    return sceReturned
                }
            }
        }
        var d: UInt32 = 0
        try sceFailure.throwIfFailed()
        var k: UInt64 = 0
        try sceFailure.throwIfFailed()
        var span: UInt64 = 205891132094649
        try sceFailure.throwIfFailed()
        var descending: Bool = true
        try sceFailure.throwIfFailed()
        while true {
            let sceCond2 = descending && d < 30
            try sceFailure.throwIfFailed()
            if !sceCond2 { break }
            descending = false
            try sceFailure.throwIfFailed()
            var started: Bool = false
            try sceFailure.throwIfFailed()
            var last: UInt64 = 0
            try sceFailure.throwIfFailed()
            var scanning: Bool = true
            try sceFailure.throwIfFailed()
            while true {
                let sceCond3 = scanning
                try sceFailure.throwIfFailed()
                if !sceCond3 { break }
                var found: Bool = false
                try sceFailure.throwIfFailed()
                var next: UInt64 = 0
                try sceFailure.throwIfFailed()
                for childOfA in a {
                    do {
                        let sceCond5 = UInt32(childOfA.depth) == Checked.add(sceFailure, d, 1) && (d == 0 || Checked.div(sceFailure, childOfA.prefix, 3) == k) && (!started || childOfA.prefix > last) && (!found || childOfA.prefix < next)
                        try sceFailure.throwIfFailed()
                        if sceCond5 {
                            found = true
                            try sceFailure.throwIfFailed()
                            next = childOfA.prefix
                            try sceFailure.throwIfFailed()
                        }
                    }
                }
                for childOfB in b {
                    do {
                        let sceCond5 = UInt32(childOfB.depth) == Checked.add(sceFailure, d, 1) && (d == 0 || Checked.div(sceFailure, childOfB.prefix, 3) == k) && (!started || childOfB.prefix > last) && (!found || childOfB.prefix < next)
                        try sceFailure.throwIfFailed()
                        if sceCond5 {
                            found = true
                            try sceFailure.throwIfFailed()
                            next = childOfB.prefix
                            try sceFailure.throwIfFailed()
                        }
                    }
                }
                scanning = found
                try sceFailure.throwIfFailed()
                do {
                    let sceCond4 = found
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        var inA: Bool = false
                        try sceFailure.throwIfFailed()
                        var hashA: UInt32 = 0
                        try sceFailure.throwIfFailed()
                        for matchInA in a {
                            do {
                                let sceCond7 = UInt32(matchInA.depth) == Checked.add(sceFailure, d, 1) && matchInA.prefix == next
                                try sceFailure.throwIfFailed()
                                if sceCond7 {
                                    inA = true
                                    try sceFailure.throwIfFailed()
                                    hashA = matchInA.hash
                                    try sceFailure.throwIfFailed()
                                }
                            }
                        }
                        var inB: Bool = false
                        try sceFailure.throwIfFailed()
                        var hashB: UInt32 = 0
                        try sceFailure.throwIfFailed()
                        for matchInB in b {
                            do {
                                let sceCond7 = UInt32(matchInB.depth) == Checked.add(sceFailure, d, 1) && matchInB.prefix == next
                                try sceFailure.throwIfFailed()
                                if sceCond7 {
                                    inB = true
                                    try sceFailure.throwIfFailed()
                                    hashB = matchInB.hash
                                    try sceFailure.throwIfFailed()
                                }
                            }
                        }
                        do {
                            let sceCond6 = !inA || !inB
                            try sceFailure.throwIfFailed()
                            if sceCond6 {
                                scanning = false
                                try sceFailure.throwIfFailed()
                            } else {
                                do {
                                    let sceCond8 = hashA != hashB
                                    try sceFailure.throwIfFailed()
                                    if sceCond8 {
                                        scanning = false
                                        try sceFailure.throwIfFailed()
                                        descending = true
                                        try sceFailure.throwIfFailed()
                                        k = next
                                        try sceFailure.throwIfFailed()
                                    } else {
                                        started = true
                                        try sceFailure.throwIfFailed()
                                        last = next
                                        try sceFailure.throwIfFailed()
                                    }
                                }
                            }
                        }
                    }
                }
            }
            do {
                let sceCond3 = descending
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    d = Checked.add(sceFailure, d, 1)
                    try sceFailure.throwIfFailed()
                    span = Checked.div(sceFailure, span, 3)
                    try sceFailure.throwIfFailed()
                }
            }
        }
        let minute: Int64 = Checked.narrow(sceFailure, Checked.mul(sceFailure, k, span), to: Int64.self)
        try sceFailure.throwIfFailed()
        result.millis = Checked.mul(sceFailure, minute, 60000)
        try sceFailure.throwIfFailed()
        do {
            let sceReturned: MerkleDivergencePayload = result
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
