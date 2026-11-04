// SCE-GENERATED — DO NOT EDIT
// source-hash: 3d0f676644ffcd9b4abbc6cf74de18978ed3b2243af3478ce8e8579253b96ea0
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
    public static let jsonSchema: String = "{\"$id\":\"urn:sce:schema:http_fetch_result:1\",\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"additionalProperties\":false,\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"properties\":{\"body\":{\"maxLength\":16384,\"type\":\"string\",\"x-sce-max-bytes\":16384},\"error\":{\"maxLength\":24,\"type\":\"string\",\"x-sce-max-bytes\":24},\"headers\":{\"maxLength\":4096,\"type\":\"string\",\"x-sce-max-bytes\":4096},\"status\":{\"maximum\":65535,\"minimum\":0,\"type\":\"integer\"}},\"required\":[\"body\",\"error\",\"headers\",\"status\"],\"type\":\"object\",\"x-sce-source-hash\":\"3d0f676644ffcd9b4abbc6cf74de18978ed3b2243af3478ce8e8579253b96ea0\"}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct HttpFetchResultPayload: Equatable, Sendable, SceDefault {
    public var error: String = ""
    public var status: UInt16 = 0
    public var headers: String = ""
    public var body: String = ""

    public init() {}

    public init(error: String, status: UInt16, headers: String, body: String) {
        self.error = error
        self.status = status
        self.headers = headers
        self.body = body
    }
}
