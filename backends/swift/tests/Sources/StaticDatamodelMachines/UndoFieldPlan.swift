// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
// SCE-MAP: undo_field_plan.scxml:50 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `UndoFieldPlan`, a file of the
// module that calls it: a call is qualified by the enum
// (`UndoFieldPlan.undoFieldPlan(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum UndoFieldPlan {
    public static func undoFieldPlan(_ versions: [FieldVersionPayload], _ batch: UInt64, _ floor: UInt64) throws -> [UndoStepPayload] {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        do {
            let sceCond2 = !(batch != 0)
            try sceFailure.throwIfFailed()
            if sceCond2 {
                throw AlgorithmError.precondition
            }
        }
        var members: UInt32 = 0
        try sceFailure.throwIfFailed()
        for m in versions {
            do {
                let sceCond3 = m.batchId == batch
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    members = Checked.add(sceFailure, members, 1)
                    try sceFailure.throwIfFailed()
                }
            }
        }
        do {
            let sceCond2 = !(members <= 256)
            try sceFailure.throwIfFailed()
            if sceCond2 {
                throw AlgorithmError.precondition
            }
        }
        var out: [UndoStepPayload] = []
        out.reserveCapacity(256)
        try sceFailure.throwIfFailed()
        for v in versions {
            do {
                let sceCond3 = v.batchId == batch
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    var first: Bool = true
                    try sceFailure.throwIfFailed()
                    for u in versions {
                        do {
                            let sceCond6 = u.batchId == batch && u.entityId == v.entityId && u.fieldId == v.fieldId
                            try sceFailure.throwIfFailed()
                            if sceCond6 {
                                let before: Int32 = FieldVersionCompare.fieldVersionCompare(u, v)
                                try sceFailure.throwIfFailed()
                                do {
                                    let sceCond8 = before < 0
                                    try sceFailure.throwIfFailed()
                                    if sceCond8 {
                                        first = false
                                        try sceFailure.throwIfFailed()
                                    }
                                }
                            }
                        }
                    }
                    do {
                        let sceCond5 = first
                        try sceFailure.throwIfFailed()
                        if sceCond5 {
                            var hasWinner: Bool = false
                            try sceFailure.throwIfFailed()
                            var winner: FieldVersionPayload = FieldVersionPayload(entityId: 0, fieldId: 0, wallTime: 0, counter: 0, nodeId: 0, arrival: 0, batchId: 0, origin: 0, revertsBatch: 0, valueWallTime: 0, valueCounter: 0, valueNodeId: 0)
                            try sceFailure.throwIfFailed()
                            var hasBase: Bool = false
                            try sceFailure.throwIfFailed()
                            var base: FieldVersionPayload = FieldVersionPayload(entityId: 0, fieldId: 0, wallTime: 0, counter: 0, nodeId: 0, arrival: 0, batchId: 0, origin: 0, revertsBatch: 0, valueWallTime: 0, valueCounter: 0, valueNodeId: 0)
                            try sceFailure.throwIfFailed()
                            for w in versions {
                                do {
                                    let sceCond8 = w.entityId == v.entityId && w.fieldId == v.fieldId
                                    try sceFailure.throwIfFailed()
                                    if sceCond8 {
                                        var againstWinner: Int32 = 0
                                        try sceFailure.throwIfFailed()
                                        do {
                                            let sceCond10 = hasWinner
                                            try sceFailure.throwIfFailed()
                                            if sceCond10 {
                                                againstWinner = FieldVersionCompare.fieldVersionCompare(w, winner)
                                                try sceFailure.throwIfFailed()
                                            }
                                        }
                                        do {
                                            let sceCond10 = !hasWinner || againstWinner > 0
                                            try sceFailure.throwIfFailed()
                                            if sceCond10 {
                                                winner.wallTime = w.wallTime
                                                try sceFailure.throwIfFailed()
                                                winner.counter = w.counter
                                                try sceFailure.throwIfFailed()
                                                winner.nodeId = w.nodeId
                                                try sceFailure.throwIfFailed()
                                                winner.batchId = w.batchId
                                                try sceFailure.throwIfFailed()
                                                winner.origin = w.origin
                                                try sceFailure.throwIfFailed()
                                                winner.revertsBatch = w.revertsBatch
                                                try sceFailure.throwIfFailed()
                                                hasWinner = true
                                                try sceFailure.throwIfFailed()
                                            }
                                        }
                                        let againstFirst: Int32 = FieldVersionCompare.fieldVersionCompare(w, v)
                                        try sceFailure.throwIfFailed()
                                        do {
                                            let sceCond10 = againstFirst < 0
                                            try sceFailure.throwIfFailed()
                                            if sceCond10 {
                                                var againstBase: Int32 = 0
                                                try sceFailure.throwIfFailed()
                                                do {
                                                    let sceCond12 = hasBase
                                                    try sceFailure.throwIfFailed()
                                                    if sceCond12 {
                                                        againstBase = FieldVersionCompare.fieldVersionCompare(w, base)
                                                        try sceFailure.throwIfFailed()
                                                    }
                                                }
                                                do {
                                                    let sceCond12 = !hasBase || againstBase > 0
                                                    try sceFailure.throwIfFailed()
                                                    if sceCond12 {
                                                        base.wallTime = w.wallTime
                                                        try sceFailure.throwIfFailed()
                                                        base.counter = w.counter
                                                        try sceFailure.throwIfFailed()
                                                        base.nodeId = w.nodeId
                                                        try sceFailure.throwIfFailed()
                                                        base.valueWallTime = w.valueWallTime
                                                        try sceFailure.throwIfFailed()
                                                        base.valueCounter = w.valueCounter
                                                        try sceFailure.throwIfFailed()
                                                        base.valueNodeId = w.valueNodeId
                                                        try sceFailure.throwIfFailed()
                                                        hasBase = true
                                                        try sceFailure.throwIfFailed()
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            var code: UInt8 = 1
                            try sceFailure.throwIfFailed()
                            var restoreWall: Int64 = 0
                            try sceFailure.throwIfFailed()
                            var restoreCount: UInt32 = 0
                            try sceFailure.throwIfFailed()
                            var restoreNode: UInt64 = 0
                            try sceFailure.throwIfFailed()
                            do {
                                let sceCond7 = v.arrival <= floor
                                try sceFailure.throwIfFailed()
                                if sceCond7 {
                                    code = 3
                                    try sceFailure.throwIfFailed()
                                } else {
                                    do {
                                        let sceCond9 = winner.origin == 2 && winner.revertsBatch == batch
                                        try sceFailure.throwIfFailed()
                                        if sceCond9 {
                                            code = 2
                                            try sceFailure.throwIfFailed()
                                        } else {
                                            do {
                                                let sceCond11 = winner.batchId == batch
                                                try sceFailure.throwIfFailed()
                                                if sceCond11 {
                                                    code = 0
                                                    try sceFailure.throwIfFailed()
                                                    do {
                                                        let sceCond13 = hasBase
                                                        try sceFailure.throwIfFailed()
                                                        if sceCond13 {
                                                            restoreWall = base.valueWallTime
                                                            try sceFailure.throwIfFailed()
                                                            restoreCount = base.valueCounter
                                                            try sceFailure.throwIfFailed()
                                                            restoreNode = base.valueNodeId
                                                            try sceFailure.throwIfFailed()
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            let step: UndoStepPayload = UndoStepPayload(entityId: v.entityId, fieldId: v.fieldId, verdict: code, restoreWallTime: restoreWall, restoreCounter: restoreCount, restoreNodeId: restoreNode, winnerWallTime: code == 1 ? winner.wallTime : 0, winnerCounter: code == 1 ? winner.counter : 0, winnerNodeId: code == 1 ? winner.nodeId : 0)
                            try sceFailure.throwIfFailed()
                            out.append(step)
                            try sceFailure.throwIfFailed()
                        }
                    }
                }
            }
        }
        do {
            let sceReturned: [UndoStepPayload] = out
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
