// SCE-GENERATED — DO NOT EDIT
// source-hash: f7347a8b98455a65e31ae2b3af12ef65d68a284242bd71a8102ab373827910fd
// SCE-MAP: text_slot.scxml:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: text.slot
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum TextSlot {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "text.slot"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most UTF-8 bytes `text` holds (`sce:max-size`), as this build resolved it.
    public static let textMaxSize: UInt32 = 32
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct TextSlotPayload: Equatable, Sendable, SceDefault {
    public var slot: UInt32 = 0
    public var version: Int64 = 0
    public var text: String = ""

    public init() {}

    public init(slot: UInt32, version: Int64, text: String) {
        self.slot = slot
        self.version = version
        self.text = text
    }
}
