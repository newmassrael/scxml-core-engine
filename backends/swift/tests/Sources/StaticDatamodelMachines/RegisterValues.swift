// SCE-GENERATED — DO NOT EDIT
// source-hash: 634c022b7eb332297dcb6115177eef8cafdb472ead988f0bdc39838cb7f8d8ae
// SCE-MAP: register_values.scxml:21 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `RegisterValues`, a file of the
// module that calls it: a call is qualified by the enum
// (`RegisterValues.registerValues(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum RegisterValues {
    public static func registerValues(_ values: [MvValuePayload], _ id: UInt64, _ field: UInt64) throws -> [MvValuePayload] {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        var out: [MvValuePayload] = []
        out.reserveCapacity(512)
        try sceFailure.throwIfFailed()
        var started: Bool = false
        try sceFailure.throwIfFailed()
        var lastWall: Int64 = 0
        try sceFailure.throwIfFailed()
        var lastCounter: UInt32 = 0
        try sceFailure.throwIfFailed()
        var lastNode: UInt64 = 0
        try sceFailure.throwIfFailed()
        var more: Bool = true
        try sceFailure.throwIfFailed()
        while true {
            let sceCond2 = more
            try sceFailure.throwIfFailed()
            if !sceCond2 { break }
            var chosen: Bool = false
            try sceFailure.throwIfFailed()
            var bestWall: Int64 = 0
            try sceFailure.throwIfFailed()
            var bestCounter: UInt32 = 0
            try sceFailure.throwIfFailed()
            var bestNode: UInt64 = 0
            try sceFailure.throwIfFailed()
            for v in values {
                do {
                    let sceCond4 = v.id == id && v.field == field && (!started || v.wallTime > lastWall || v.wallTime == lastWall && (v.counter > lastCounter || v.counter == lastCounter && v.nodeId > lastNode)) && (!chosen || v.wallTime < bestWall || v.wallTime == bestWall && (v.counter < bestCounter || v.counter == bestCounter && v.nodeId < bestNode))
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        chosen = true
                        try sceFailure.throwIfFailed()
                        bestWall = v.wallTime
                        try sceFailure.throwIfFailed()
                        bestCounter = v.counter
                        try sceFailure.throwIfFailed()
                        bestNode = v.nodeId
                        try sceFailure.throwIfFailed()
                    }
                }
            }
            do {
                let sceCond3 = chosen
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    for w in values {
                        do {
                            let sceCond6 = w.id == id && w.field == field && w.wallTime == bestWall && w.counter == bestCounter && w.nodeId == bestNode
                            try sceFailure.throwIfFailed()
                            if sceCond6 {
                                out.append(w)
                                try sceFailure.throwIfFailed()
                            }
                        }
                    }
                    started = true
                    try sceFailure.throwIfFailed()
                    lastWall = bestWall
                    try sceFailure.throwIfFailed()
                    lastCounter = bestCounter
                    try sceFailure.throwIfFailed()
                    lastNode = bestNode
                    try sceFailure.throwIfFailed()
                } else {
                    more = false
                    try sceFailure.throwIfFailed()
                }
            }
        }
        do {
            let sceReturned: [MvValuePayload] = out
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
