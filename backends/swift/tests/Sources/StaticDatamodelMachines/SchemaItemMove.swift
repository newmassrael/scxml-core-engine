// SCE-GENERATED — DO NOT EDIT
// source-hash: 047dc621dfd01b6348575becd26162c58fa20fa7680f076526766f91e97d2c5f
// SCE-MAP: schema_item_move.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: item.moved
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum SchemaItemMove {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "item.moved"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct SchemaItemMovePayload: Equatable, Sendable, SceDefault {
    public var id: UInt32 = 0
    public var status: EnumItemStatus = EnumItemStatus.allCases[0]

    public init() {}

    public init(id: UInt32, status: EnumItemStatus) {
        self.id = id
        self.status = status
    }
}
