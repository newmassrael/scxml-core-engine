// SCE-GENERATED — DO NOT EDIT
// source-hash: ef94c112b4026dfab705629195b17ee265cdc572de6e8fceff328262c2af0613
// SCE-MAP: event_batch.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: batch.sent
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum EventBatch {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "batch.sent"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 2

    /// The most elements `items` holds (the bound its type writes, named `batch_items`), as this build resolved it.
    public static let itemsMaxCount: UInt32 = 3

    /// The most elements `grid` holds (the bound its type writes), as this build resolved it.
    public static let gridMaxCount: UInt32 = 2

    /// The most elements `levels` holds (the bound its type writes), as this build resolved it.
    public static let levelsMaxCount: UInt32 = 2

    /// The most elements `maybe` holds (the bound its type writes), as this build resolved it.
    public static let maybeMaxCount: UInt32 = 2

    /// The most elements `spare` holds (the bound its type writes), as this build resolved it.
    public static let spareMaxCount: UInt32 = 2

    /// The most UTF-8 bytes `note` holds (the bound its type writes), as this build resolved it.
    public static let noteMaxSize: UInt32 = 4

    /// The most elements `late` holds (the bound its type writes), as this build resolved it.
    public static let lateMaxCount: UInt32 = 2

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 1376
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct EventBatchPayload: Equatable, Sendable, SceDefault {
    public var items: [EventBatchItemPayload] = []
    public var grid: [[UInt8]] = []
    public var levels: [EnumSignalLevel] = []
    public var maybe: [UInt16?] = []
    public var spare: [UInt8]? = nil
    public var note: String = ""
    public var late: [UInt8] = []

    public init() {}

    public init(items: [EventBatchItemPayload], grid: [[UInt8]], levels: [EnumSignalLevel], maybe: [UInt16?], spare: [UInt8]?, note: String, late: [UInt8]) {
        self.items = items
        self.grid = grid
        self.levels = levels
        self.maybe = maybe
        self.spare = spare
        self.note = note
        self.late = late
    }
}
