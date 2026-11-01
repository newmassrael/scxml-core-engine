// SCE-GENERATED — DO NOT EDIT
// source-hash: a0ed8bd925c949021fa83a15dd0434623cdc2a31d67a1cec484ba6e7168d79a0
// SCE-MAP: event_schema_task.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: task.state
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum EventSchemaTask {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "task.state"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct EventSchemaTaskPayload: Equatable, Sendable, SceDefault {
    public var taskId: UInt32 = 0
    public var phase: EnumTaskPhase = EnumTaskPhase.allCases[0]

    public init() {}

    public init(taskId: UInt32, phase: EnumTaskPhase) {
        self.taskId = taskId
        self.phase = phase
    }
}
