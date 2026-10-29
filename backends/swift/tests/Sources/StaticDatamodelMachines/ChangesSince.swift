// SCE-GENERATED — DO NOT EDIT
// source-hash: 1e96d54a1b4f9666059e38585db03a53d8ce664a6f48e0045468eeffc6fb717b
// SCE-MAP: changes_since.scxml:37 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `ChangesSince`, a file of the
// module that calls it: a call is qualified by the enum
// (`ChangesSince.changesSince(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum ChangesSince {
    public static func changesSince(_ log: [ChangeEntryPayload], _ since: UInt64, _ current: UInt64, _ lowWater: UInt64, _ limit: UInt32) throws -> [ChangeEntryPayload] {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        do {
            let sceCond2 = !(since >= lowWater && since <= current)
            try sceFailure.throwIfFailed()
            if sceCond2 {
                throw AlgorithmError.precondition
            }
        }
        do {
            let sceCond2 = !(limit >= 1 && limit <= 256)
            try sceFailure.throwIfFailed()
            if sceCond2 {
                throw AlgorithmError.precondition
            }
        }
        var out: [ChangeEntryPayload] = []
        out.reserveCapacity(256)
        try sceFailure.throwIfFailed()
        var taken: UInt32 = 0
        try sceFailure.throwIfFailed()
        var prevToken: UInt64 = 0
        try sceFailure.throwIfFailed()
        var i: UInt32 = 0
        try sceFailure.throwIfFailed()
        for e in log {
            do {
                let sceCond3 = !(e.token >= prevToken && e.op >= 1 && e.op <= 3)
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    throw AlgorithmError.precondition
                }
            }
            prevToken = e.token
            try sceFailure.throwIfFailed()
            do {
                let sceCond3 = e.token >= since && taken < limit
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    var superseded: Bool = false
                    try sceFailure.throwIfFailed()
                    var j: UInt32 = 0
                    try sceFailure.throwIfFailed()
                    for f in log {
                        do {
                            let sceCond6 = j > i && f.item == e.item
                            try sceFailure.throwIfFailed()
                            if sceCond6 {
                                superseded = true
                                try sceFailure.throwIfFailed()
                            }
                        }
                        j = Checked.add(sceFailure, j, 1)
                        try sceFailure.throwIfFailed()
                    }
                    do {
                        let sceCond5 = !superseded
                        try sceFailure.throwIfFailed()
                        if sceCond5 {
                            out.append(e)
                            try sceFailure.throwIfFailed()
                            taken = Checked.add(sceFailure, taken, 1)
                            try sceFailure.throwIfFailed()
                        }
                    }
                }
            }
            i = Checked.add(sceFailure, i, 1)
            try sceFailure.throwIfFailed()
        }
        do {
            let sceReturned: [ChangeEntryPayload] = out
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
