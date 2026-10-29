// SCE-GENERATED — DO NOT EDIT
// source-hash: 2ceb006f0889453c0736e5e54ecad34085b33b6cef4480cbb7d876536e0cecb6
// SCE-MAP: http_fetch_request.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.http.fetch.request
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum HttpFetchRequest {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "host.http.fetch.request"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most UTF-8 bytes `method` holds (`sce:max-size`), as this build resolved it.
    public static let methodMaxSize: UInt32 = 8

    /// The most UTF-8 bytes `url` holds (`sce:max-size`), as this build resolved it.
    public static let urlMaxSize: UInt32 = 2048

    /// The most UTF-8 bytes `headers` holds (`sce:max-size`), as this build resolved it.
    public static let headersMaxSize: UInt32 = 2048

    /// The most UTF-8 bytes `body` holds (`sce:max-size`), as this build resolved it.
    public static let bodyMaxSize: UInt32 = 16384
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct HttpFetchRequestPayload: Equatable, Sendable, SceDefault {
    public var method: String = ""
    public var url: String = ""
    public var headers: String = ""
    public var body: String = ""

    public init() {}

    public init(method: String, url: String, headers: String, body: String) {
        self.method = method
        self.url = url
        self.headers = headers
        self.body = body
    }
}
