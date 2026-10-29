// SCE-GENERATED — DO NOT EDIT
// source-hash: 2ceb006f0889453c0736e5e54ecad34085b33b6cef4480cbb7d876536e0cecb6
// SCE-MAP: kv_delete_result.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.kv.delete.result
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum KvDeleteResult {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "host.kv.delete.result"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most UTF-8 bytes `error` holds (`sce:max-size`), as this build resolved it.
    public static let errorMaxSize: UInt32 = 24
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct KvDeleteResultPayload: Equatable, Sendable, SceDefault {
    public var error: String = ""
    public var existed: Bool = false

    public init() {}

    public init(error: String, existed: Bool) {
        self.error = error
        self.existed = existed
    }
}
