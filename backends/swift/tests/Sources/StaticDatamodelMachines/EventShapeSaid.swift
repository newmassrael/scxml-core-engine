// SCE-GENERATED — DO NOT EDIT
// source-hash: 9c4f25c71477b8168828f3c8f9ccad85a4253ff428db93b5b34a11bbacc1faa1
// SCE-MAP: event_shape_said.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: shape.said
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum EventShapeSaid {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "shape.said"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most UTF-8 bytes `text` holds (the bound its type writes), as this build resolved it.
    public static let textMaxSize: UInt32 = 96

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 587

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:event_shape_said:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:event_shape_said:1\",\"x-sce-source-hash\":\"9c4f25c71477b8168828f3c8f9ccad85a4253ff428db93b5b34a11bbacc1faa1\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"text\":{\"type\":\"string\",\"maxLength\":96,\"x-sce-max-bytes\":96}},\"required\":[\"text\"],\"additionalProperties\":false}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct EventShapeSaidPayload: Equatable, Sendable, SceDefault {
    public var text: String = ""

    public init() {}

    public init(text: String) {
        self.text = text
    }
}
