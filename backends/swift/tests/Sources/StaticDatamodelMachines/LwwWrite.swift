// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
// SCE-MAP: lww_write.scxml:24 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `LwwWrite`, a file of the
// module that calls it: a call is qualified by the enum
// (`LwwWrite.lwwWrite(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum LwwWrite {
    public static func lwwWrite(_ cells: [LwwCellPayload], _ write: LwwCellPayload) throws -> [LwwCellPayload] {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        var out: [LwwCellPayload] = []
        out.reserveCapacity(256)
        try sceFailure.throwIfFailed()
        var held: Bool = false
        try sceFailure.throwIfFailed()
        let incoming: HlcTimestampPayload = HlcTimestampPayload(wallTime: write.wallTime, counter: write.counter, nodeId: write.nodeId)
        try sceFailure.throwIfFailed()
        for c in cells {
            do {
                let sceCond3 = c.row == write.row && c.col == write.col
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    held = true
                    try sceFailure.throwIfFailed()
                    let stored: HlcTimestampPayload = HlcTimestampPayload(wallTime: c.wallTime, counter: c.counter, nodeId: c.nodeId)
                    try sceFailure.throwIfFailed()
                    do {
                        let sceCond5 = HlcCompare.hlcCompare(incoming, stored) > 0
                        try sceFailure.throwIfFailed()
                        if sceCond5 {
                            out.append(write)
                            try sceFailure.throwIfFailed()
                        } else {
                            out.append(c)
                            try sceFailure.throwIfFailed()
                        }
                    }
                } else {
                    out.append(c)
                    try sceFailure.throwIfFailed()
                }
            }
        }
        do {
            let sceCond2 = !held
            try sceFailure.throwIfFailed()
            if sceCond2 {
                out.append(write)
                try sceFailure.throwIfFailed()
            }
        }
        do {
            let sceReturned: [LwwCellPayload] = out
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
