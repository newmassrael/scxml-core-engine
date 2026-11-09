// SCE-GENERATED — DO NOT EDIT
// source-hash: d05ce240c5f3b48b3c83b6bee1ba58feef40d57c310e9a29ef5279604bbaaa42
// SCE-MAP: shape_path.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: shape.path
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum ShapePath {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "shape.path"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most elements `points` holds (the bound its type writes), as this build resolved it.
    public static let pointsMaxCount: UInt32 = 3

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 24

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:shape_path:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:shape_path:1\",\"x-sce-source-hash\":\"d05ce240c5f3b48b3c83b6bee1ba58feef40d57c310e9a29ef5279604bbaaa42\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"points\":{\"type\":\"array\",\"items\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"maxItems\":3}},\"required\":[\"points\"],\"additionalProperties\":false}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct ShapePathPayload: Equatable, Sendable, SceDefault {
    public var points: [UInt8] = []

    public init() {}

    public init(points: [UInt8]) {
        self.points = points
    }
}
