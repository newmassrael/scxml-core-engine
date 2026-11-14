// SCE-GENERATED — DO NOT EDIT
// source-hash: 1f01cd741ca0d625188a4b5d3fc338154b34cfdf32194edda139c3bc09e0016b
// SCE-MAP: http_fetch_result.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.http.fetch.result
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum HttpFetchResult {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "host.http.fetch.result"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most UTF-8 bytes `error` holds (the bound its type writes), as this build resolved it.
    public static let errorMaxSize: UInt32 = 24

    /// The most UTF-8 bytes `headers` holds (the bound its type writes), as this build resolved it.
    public static let headersMaxSize: UInt32 = 4096

    /// The most UTF-8 bytes `body` holds (the bound its type writes), as this build resolved it.
    public static let bodyMaxSize: UInt32 = 16384

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 123074

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:http_fetch_result:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:http_fetch_result:1\",\"x-sce-source-hash\":\"1f01cd741ca0d625188a4b5d3fc338154b34cfdf32194edda139c3bc09e0016b\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"error\":{\"type\":\"string\",\"maxLength\":24,\"x-sce-max-bytes\":24},\"status\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535},\"headers\":{\"type\":\"string\",\"maxLength\":4096,\"x-sce-max-bytes\":4096},\"body\":{\"type\":\"string\",\"maxLength\":16384,\"x-sce-max-bytes\":16384}},\"required\":[\"error\",\"status\",\"headers\",\"body\"],\"additionalProperties\":false}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order, built
/// whole by its initialiser, every field written: no type has a value nobody wrote
/// (docs/adr/0099, section 4). A value, so a host that reads one holds a copy.
public struct HttpFetchResultPayload: Equatable, Sendable {
    public var error: String
    public var status: UInt16
    public var headers: String
    public var body: String

    public init(error: String, status: UInt16, headers: String, body: String) {
        self.error = error
        self.status = status
        self.headers = headers
        self.body = body
    }
}
