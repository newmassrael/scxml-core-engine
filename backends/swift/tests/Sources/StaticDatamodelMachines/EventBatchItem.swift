// SCE-GENERATED — DO NOT EDIT
// source-hash: d947250ac8dbe681be6c82964fb8feadd2bb3cbd6b46adffc9d553a7fc0e7ce6
// SCE-MAP: event_batch_item.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: batch.item
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum EventBatchItem {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "batch.item"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most UTF-8 bytes `label` holds (the bound its type writes), as this build resolved it.
    public static let labelMaxSize: UInt32 = 8

    /// The most bytes `blob` holds (the bound its type writes), as this build resolved it.
    public static let blobMaxSize: UInt32 = 4

    /// The most elements `tags` holds (the bound its type writes), as this build resolved it.
    public static let tagsMaxCount: UInt32 = 4

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 403
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct EventBatchItemPayload: Equatable, Sendable, SceDefault {
    public var id: UInt32 = 0
    public var amount: Int64 = 0
    public var label: String = ""
    public var level: EnumSignalLevel = EnumSignalLevel.allCases[0]
    public var due: Int64? = nil
    public var blob: [UInt8] = []
    public var tags: [String] = []

    public init() {}

    public init(id: UInt32, amount: Int64, label: String, level: EnumSignalLevel, due: Int64?, blob: [UInt8], tags: [String]) {
        self.id = id
        self.amount = amount
        self.label = label
        self.level = level
        self.due = due
        self.blob = blob
        self.tags = tags
    }
}
