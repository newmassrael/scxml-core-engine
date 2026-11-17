// SCE-GENERATED — DO NOT EDIT
// source-hash: a86b3b19a12dab1982345f4a46c1e37355ff902583da2c2f5d8395ee9c56f9e7
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
    public static func changesApply(_ state: [ItemVersionPayload], _ log: [ChangeEntryPayload]) throws(AlgorithmError) -> [ItemVersionPayload] {
        var prevItem: UInt64 = 0
        var first: Bool = true
        for held0 in state {
            if !(first || held0.item > prevItem) {
                throw AlgorithmError.precondition
            }
            prevItem = held0.item
            first = false
        }
        var prevToken: UInt64 = 0
        for row0 in log {
            if !(row0.token >= prevToken && row0.op >= 1 && row0.op <= 3) {
                throw AlgorithmError.precondition
            }
            prevToken = row0.token
        }
        var out: [ItemVersionPayload] = []
        out.reserveCapacity(256)
        var last: UInt64 = 0
        var started: Bool = false
        var more: Bool = true
        while more {
            var found: Bool = false
            var best: UInt64 = 0
            var bestToken: UInt64 = 0
            for s in state {
                if (!started || s.item > last) && (!found || s.item < best) {
                    var present: Bool = true
                    var token: UInt64 = s.token
                    for r in log {
                        if r.item == s.item {
                            present = r.op != 3
                            token = r.token
                        }
                    }
                    if present {
                        found = true
                        best = s.item
                        bestToken = token
                    }
                }
            }
            for c in log {
                if (!started || c.item > last) && (!found || c.item < best) {
                    var present2: Bool = false
                    var token2: UInt64 = 0
                    for d in log {
                        if d.item == c.item {
                            present2 = d.op != 3
                            token2 = d.token
                        }
                    }
                    if present2 {
                        found = true
                        best = c.item
                        bestToken = token2
                    }
                }
            }
            if found {
                let held: ItemVersionPayload = ItemVersionPayload(item: best, token: bestToken)
                out.append(held)
                last = best
                started = true
            }
            more = found
        }
        return out
    }
}
