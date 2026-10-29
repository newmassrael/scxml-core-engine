// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
// SCE-MAP: undo_set_readds.scxml:23 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `UndoSetReadds`, a file of the
// module that calls it: a call is qualified by the enum
// (`UndoSetReadds.undoSetReadds(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum UndoSetReadds {
    public static func undoSetReadds(_ batchTombstones: [OrsetEntryPayload], _ live: [OrsetEntryPayload]) throws -> [UInt64] {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        do {
            let sceCond2 = !(batchTombstones.count <= 256)
            try sceFailure.throwIfFailed()
            if sceCond2 {
                throw AlgorithmError.precondition
            }
        }
        var out: [UInt64] = []
        out.reserveCapacity(256)
        try sceFailure.throwIfFailed()
        for t in batchTombstones {
            var held: Bool = false
            try sceFailure.throwIfFailed()
            for l in live {
                do {
                    let sceCond4 = l.element == t.element
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        held = true
                        try sceFailure.throwIfFailed()
                    }
                }
            }
            var listed: Bool = false
            try sceFailure.throwIfFailed()
            var i: UInt32 = 0
            try sceFailure.throwIfFailed()
            while true {
                let sceCond3 = i < UInt32(truncatingIfNeeded: out.count)
                try sceFailure.throwIfFailed()
                if !sceCond3 { break }
                do {
                    let sceCond4 = Checked.at(sceFailure, out, i, fallback: 0) == t.element
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        listed = true
                        try sceFailure.throwIfFailed()
                    }
                }
                i = Checked.add(sceFailure, i, 1)
                try sceFailure.throwIfFailed()
            }
            do {
                let sceCond3 = !held && !listed
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    out.append(t.element)
                    try sceFailure.throwIfFailed()
                }
            }
        }
        do {
            let sceReturned: [UInt64] = out
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
