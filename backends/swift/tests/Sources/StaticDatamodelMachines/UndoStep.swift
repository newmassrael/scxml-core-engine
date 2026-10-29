// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
// SCE-MAP: undo_step.scxml:19 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: merge.undo_step
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum UndoStep {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "merge.undo_step"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct UndoStepPayload: Equatable, Sendable, SceDefault {
    public var entityId: UInt64 = 0
    public var fieldId: UInt32 = 0
    public var verdict: UInt8 = 0
    public var restoreWallTime: Int64 = 0
    public var restoreCounter: UInt32 = 0
    public var restoreNodeId: UInt64 = 0
    public var winnerWallTime: Int64 = 0
    public var winnerCounter: UInt32 = 0
    public var winnerNodeId: UInt64 = 0

    public init() {}

    public init(entityId: UInt64, fieldId: UInt32, verdict: UInt8, restoreWallTime: Int64, restoreCounter: UInt32, restoreNodeId: UInt64, winnerWallTime: Int64, winnerCounter: UInt32, winnerNodeId: UInt64) {
        self.entityId = entityId
        self.fieldId = fieldId
        self.verdict = verdict
        self.restoreWallTime = restoreWallTime
        self.restoreCounter = restoreCounter
        self.restoreNodeId = restoreNodeId
        self.winnerWallTime = winnerWallTime
        self.winnerCounter = winnerCounter
        self.winnerNodeId = winnerNodeId
    }
}
