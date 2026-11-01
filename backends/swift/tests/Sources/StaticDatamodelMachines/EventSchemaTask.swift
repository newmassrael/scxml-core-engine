// SCE-GENERATED — DO NOT EDIT
// source-hash: 88c3d475be4b704e681e435f769b6c1e874023c075725ce0d093a310a81ad11d
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
