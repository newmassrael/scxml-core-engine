// SCE-GENERATED — DO NOT EDIT
// source-hash: fb4f42a34e7ac40a28d8e5daee5d633c3c45dddac513816edf2bb83fdc85f91c
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
    public static func undoFieldPlan(_ versions: [FieldVersionPayload], _ batch: UInt64, _ floor: UInt64) throws(AlgorithmError) -> [UndoStepPayload] {
        if !(batch != 0) {
            throw AlgorithmError.precondition
        }
        var members: UInt32 = 0
        for m in versions {
            if m.batchId == batch {
                members = (try Checked.add(members, 1))
            }
        }
        if !(members <= 256) {
            throw AlgorithmError.precondition
        }
        var out: [UndoStepPayload] = []
        out.reserveCapacity(256)
        for v in versions {
            if v.batchId == batch {
                var first: Bool = true
                for u in versions {
                    if u.batchId == batch && u.entityId == v.entityId && u.fieldId == v.fieldId {
                        let before: Int32 = FieldVersionCompare.fieldVersionCompare(u, v)
                        if before < 0 {
                            first = false
                        }
                    }
                }
                if first {
                    var hasWinner: Bool = false
                    var winner: FieldVersionPayload = FieldVersionPayload(entityId: 0, fieldId: 0, wallTime: 0, counter: 0, nodeId: 0, arrival: 0, batchId: 0, origin: 0, revertsBatch: 0, valueWallTime: 0, valueCounter: 0, valueNodeId: 0)
                    var hasBase: Bool = false
                    var base: FieldVersionPayload = FieldVersionPayload(entityId: 0, fieldId: 0, wallTime: 0, counter: 0, nodeId: 0, arrival: 0, batchId: 0, origin: 0, revertsBatch: 0, valueWallTime: 0, valueCounter: 0, valueNodeId: 0)
                    for w in versions {
                        if w.entityId == v.entityId && w.fieldId == v.fieldId {
                            var againstWinner: Int32 = 0
                            if hasWinner {
                                againstWinner = FieldVersionCompare.fieldVersionCompare(w, winner)
                            }
                            if !hasWinner || againstWinner > 0 {
                                winner.wallTime = w.wallTime
                                winner.counter = w.counter
                                winner.nodeId = w.nodeId
                                winner.batchId = w.batchId
                                winner.origin = w.origin
                                winner.revertsBatch = w.revertsBatch
                                hasWinner = true
                            }
                            let againstFirst: Int32 = FieldVersionCompare.fieldVersionCompare(w, v)
                            if againstFirst < 0 {
                                var againstBase: Int32 = 0
                                if hasBase {
                                    againstBase = FieldVersionCompare.fieldVersionCompare(w, base)
                                }
                                if !hasBase || againstBase > 0 {
                                    base.wallTime = w.wallTime
                                    base.counter = w.counter
                                    base.nodeId = w.nodeId
                                    base.valueWallTime = w.valueWallTime
                                    base.valueCounter = w.valueCounter
                                    base.valueNodeId = w.valueNodeId
                                    hasBase = true
                                }
                            }
                        }
                    }
                    var code: UInt8 = 1
                    var restoreWall: Int64 = 0
                    var restoreCount: UInt32 = 0
                    var restoreNode: UInt64 = 0
                    if v.arrival <= floor {
                        code = 3
                    } else {
                        if winner.origin == 2 && winner.revertsBatch == batch {
                            code = 2
                        } else {
                            if winner.batchId == batch {
                                code = 0
                                if hasBase {
                                    restoreWall = base.valueWallTime
                                    restoreCount = base.valueCounter
                                    restoreNode = base.valueNodeId
                                }
                            }
                        }
                    }
                    let step: UndoStepPayload = UndoStepPayload(entityId: v.entityId, fieldId: v.fieldId, verdict: code, restoreWallTime: restoreWall, restoreCounter: restoreCount, restoreNodeId: restoreNode, winnerWallTime: code == 1 ? winner.wallTime : 0, winnerCounter: code == 1 ? winner.counter : 0, winnerNodeId: code == 1 ? winner.nodeId : 0)
                    out.append(step)
                }
            }
        }
        return out
    }
}
