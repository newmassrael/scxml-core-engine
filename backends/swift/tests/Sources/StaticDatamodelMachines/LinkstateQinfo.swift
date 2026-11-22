// SCE-GENERATED — DO NOT EDIT
// source-hash: 9174c9c67334c6d6929c080f6a44797beb306a162b0dc103e5985893b05100ab
// source: linkstate_qinfo.scxml
// SCE-MAP: linkstate_qinfo.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: interest.value
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum LinkstateQinfo {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "interest.value"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 35

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:linkstate_qinfo:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:linkstate_qinfo:1\",\"x-sce-source-hash\":\"9174c9c67334c6d6929c080f6a44797beb306a162b0dc103e5985893b05100ab\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"complete\":{\"type\":\"boolean\"},\"distance\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535}},\"required\":[\"complete\",\"distance\"],\"additionalProperties\":false}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order, built
/// whole by its initialiser, every field written: no type has a value nobody wrote
/// (docs/adr/0099, section 4). A value, so a host that reads one holds a copy.
public struct LinkstateQinfoPayload: Equatable, Sendable {
    public var complete: Bool
    public var distance: UInt16

    public init(complete: Bool, distance: UInt16) {
        self.complete = complete
        self.distance = distance
    }
}
