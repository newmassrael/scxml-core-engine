// SCE-GENERATED — DO NOT EDIT
// source-hash: 2ceb006f0889453c0736e5e54ecad34085b33b6cef4480cbb7d876536e0cecb6
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

    /// The most UTF-8 bytes `error` holds (`sce:max-size`), as this build resolved it.
    public static let errorMaxSize: UInt32 = 24

    /// The most UTF-8 bytes `headers` holds (`sce:max-size`), as this build resolved it.
    public static let headersMaxSize: UInt32 = 4096

    /// The most UTF-8 bytes `body` holds (`sce:max-size`), as this build resolved it.
    public static let bodyMaxSize: UInt32 = 16384
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
