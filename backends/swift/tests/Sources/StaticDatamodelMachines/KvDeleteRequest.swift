// SCE-GENERATED — DO NOT EDIT
// source-hash: 2ceb006f0889453c0736e5e54ecad34085b33b6cef4480cbb7d876536e0cecb6
// SCE-MAP: kv_delete_request.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.kv.delete.request
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum KvDeleteRequest {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "host.kv.delete.request"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most UTF-8 bytes `key` holds (`sce:max-size`), as this build resolved it.
    public static let keyMaxSize: UInt32 = 256
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct KvDeleteRequestPayload: Equatable, Sendable, SceDefault {
    public var key: String = ""

    public init() {}

    public init(key: String) {
        self.key = key
    }
}
