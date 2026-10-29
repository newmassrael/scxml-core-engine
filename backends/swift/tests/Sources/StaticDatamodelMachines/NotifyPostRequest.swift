// SCE-GENERATED — DO NOT EDIT
// source-hash: 2ceb006f0889453c0736e5e54ecad34085b33b6cef4480cbb7d876536e0cecb6
// SCE-MAP: notify_post_request.scxml:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.notify.post.request
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum NotifyPostRequest {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "host.notify.post.request"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most UTF-8 bytes `id` holds (`sce:max-size`), as this build resolved it.
    public static let idMaxSize: UInt32 = 64

    /// The most UTF-8 bytes `title` holds (`sce:max-size`), as this build resolved it.
    public static let titleMaxSize: UInt32 = 128

    /// The most UTF-8 bytes `body` holds (`sce:max-size`), as this build resolved it.
    public static let bodyMaxSize: UInt32 = 512
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct NotifyPostRequestPayload: Equatable, Sendable, SceDefault {
    public var id: String = ""
    public var title: String = ""
    public var body: String = ""
    public var at_ms: Int64 = 0

    public init() {}

    public init(id: String, title: String, body: String, at_ms: Int64) {
        self.id = id
        self.title = title
        self.body = body
        self.at_ms = at_ms
    }
}
