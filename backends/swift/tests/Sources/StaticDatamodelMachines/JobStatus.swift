// SCE-GENERATED — DO NOT EDIT
// source-hash: 0834e4c26bd46d9ebaf430da8e162e38dd9e4bbf623718c722dcacef2754df7f
// SCE-MAP: job_status.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: job.status
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum JobStatus {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "job.status"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most UTF-8 bytes `note` holds (the bound its type writes), as this build resolved it.
    public static let noteMaxSize: UInt32 = 32

    /// The most elements `codes` holds (the bound its type writes), as this build resolved it.
    public static let codesMaxCount: UInt32 = 4

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 296

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:job_status:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:job_status:1\",\"x-sce-source-hash\":\"0834e4c26bd46d9ebaf430da8e162e38dd9e4bbf623718c722dcacef2754df7f\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"elapsed_ms\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"note\":{\"anyOf\":[{\"type\":\"string\",\"maxLength\":32,\"x-sce-max-bytes\":32},{\"type\":\"null\"}]},\"codes\":{\"type\":\"array\",\"items\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"maxItems\":4},\"tag\":{\"anyOf\":[{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},{\"type\":\"null\"}]},\"retry\":{\"anyOf\":[{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},{\"type\":\"null\"}]},\"count\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295}},\"required\":[\"elapsed_ms\",\"note\",\"codes\",\"tag\",\"retry\",\"count\"],\"additionalProperties\":false}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct JobStatusPayload: Equatable, Sendable, SceDefault {
    public var elapsed_ms: UInt32 = 0
    public var note: String? = nil
    public var codes: [UInt8] = []
    public var tag: UInt8? = nil
    public var retry: UInt8? = nil
    public var count: UInt32 = 0

    public init() {}

    public init(elapsed_ms: UInt32, note: String?, codes: [UInt8], tag: UInt8?, retry: UInt8?, count: UInt32) {
        self.elapsed_ms = elapsed_ms
        self.note = note
        self.codes = codes
        self.tag = tag
        self.retry = retry
        self.count = count
    }
}
