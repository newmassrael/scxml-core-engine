// SCE-GENERATED — DO NOT EDIT
// source-hash: 24f1c455257ea04f598b006a589c3dea642c1ea2e58b2dcf4b49b8cebf3f207e
// SCE-MAP: schema_item_pick.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: item.picked
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum SchemaItemPick {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "item.picked"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most UTF-8 bytes `title` holds (`sce:max-size`), as this build resolved it.
    public static let titleMaxSize: UInt32 = 16
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct SchemaItemPickPayload: Equatable, Sendable, SceDefault {
    public var id: UInt32 = 0
    public var title: String = ""
    public var count: Int32 = 0

    public init() {}

    public init(id: UInt32, title: String, count: Int32) {
        self.id = id
        self.title = title
        self.count = count
    }
}
