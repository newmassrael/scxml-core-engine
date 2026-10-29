// SCE-GENERATED — DO NOT EDIT
// source-hash: 1e96d54a1b4f9666059e38585db03a53d8ce664a6f48e0045468eeffc6fb717b
// SCE-MAP: changes_apply.scxml:37 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `ChangesApply`, a file of the
// module that calls it: a call is qualified by the enum
// (`ChangesApply.changesApply(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum ChangesApply {
    public static func changesApply(_ state: [ItemVersionPayload], _ log: [ChangeEntryPayload]) throws -> [ItemVersionPayload] {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        var prevItem: UInt64 = 0
        try sceFailure.throwIfFailed()
        var first: Bool = true
        try sceFailure.throwIfFailed()
        for held0 in state {
            do {
                let sceCond3 = !(first || held0.item > prevItem)
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    throw AlgorithmError.precondition
                }
            }
            prevItem = held0.item
            try sceFailure.throwIfFailed()
            first = false
            try sceFailure.throwIfFailed()
        }
        var prevToken: UInt64 = 0
        try sceFailure.throwIfFailed()
        for row0 in log {
            do {
                let sceCond3 = !(row0.token >= prevToken && row0.op >= 1 && row0.op <= 3)
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    throw AlgorithmError.precondition
                }
            }
            prevToken = row0.token
            try sceFailure.throwIfFailed()
        }
        var out: [ItemVersionPayload] = []
        out.reserveCapacity(256)
        try sceFailure.throwIfFailed()
        var last: UInt64 = 0
        try sceFailure.throwIfFailed()
        var started: Bool = false
        try sceFailure.throwIfFailed()
        var more: Bool = true
        try sceFailure.throwIfFailed()
        while true {
            let sceCond2 = more
            try sceFailure.throwIfFailed()
            if !sceCond2 { break }
            var found: Bool = false
            try sceFailure.throwIfFailed()
            var best: UInt64 = 0
            try sceFailure.throwIfFailed()
            var bestToken: UInt64 = 0
            try sceFailure.throwIfFailed()
            for s in state {
                do {
                    let sceCond4 = (!started || s.item > last) && (!found || s.item < best)
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        var present: Bool = true
                        try sceFailure.throwIfFailed()
                        var token: UInt64 = s.token
                        try sceFailure.throwIfFailed()
                        for r in log {
                            do {
                                let sceCond7 = r.item == s.item
                                try sceFailure.throwIfFailed()
                                if sceCond7 {
                                    present = r.op != 3
                                    try sceFailure.throwIfFailed()
                                    token = r.token
                                    try sceFailure.throwIfFailed()
                                }
                            }
                        }
                        do {
                            let sceCond6 = present
                            try sceFailure.throwIfFailed()
                            if sceCond6 {
                                found = true
                                try sceFailure.throwIfFailed()
                                best = s.item
                                try sceFailure.throwIfFailed()
                                bestToken = token
                                try sceFailure.throwIfFailed()
                            }
                        }
                    }
                }
            }
            for c in log {
                do {
                    let sceCond4 = (!started || c.item > last) && (!found || c.item < best)
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        var present2: Bool = false
                        try sceFailure.throwIfFailed()
                        var token2: UInt64 = 0
                        try sceFailure.throwIfFailed()
                        for d in log {
                            do {
                                let sceCond7 = d.item == c.item
                                try sceFailure.throwIfFailed()
                                if sceCond7 {
                                    present2 = d.op != 3
                                    try sceFailure.throwIfFailed()
                                    token2 = d.token
                                    try sceFailure.throwIfFailed()
                                }
                            }
                        }
                        do {
                            let sceCond6 = present2
                            try sceFailure.throwIfFailed()
                            if sceCond6 {
                                found = true
                                try sceFailure.throwIfFailed()
                                best = c.item
                                try sceFailure.throwIfFailed()
                                bestToken = token2
                                try sceFailure.throwIfFailed()
                            }
                        }
                    }
                }
            }
            do {
                let sceCond3 = found
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    let held: ItemVersionPayload = ItemVersionPayload(item: best, token: bestToken)
                    try sceFailure.throwIfFailed()
                    out.append(held)
                    try sceFailure.throwIfFailed()
                    last = best
                    try sceFailure.throwIfFailed()
                    started = true
                    try sceFailure.throwIfFailed()
                }
            }
            more = found
            try sceFailure.throwIfFailed()
        }
        do {
            let sceReturned: [ItemVersionPayload] = out
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
