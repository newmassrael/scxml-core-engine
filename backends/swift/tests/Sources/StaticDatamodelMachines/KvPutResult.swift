// SCE-GENERATED — DO NOT EDIT
// source-hash: 3d0f676644ffcd9b4abbc6cf74de18978ed3b2243af3478ce8e8579253b96ea0
// SCE-MAP: kv_put_result.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: host.kv.put.result
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum KvPutResult {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "host.kv.put.result"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most UTF-8 bytes `error` holds (the bound its type writes), as this build resolved it.
    public static let errorMaxSize: UInt32 = 24

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 173
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct KvPutResultPayload: Equatable, Sendable, SceDefault {
    public var error: String = ""
    public var replaced: Bool = false

    public init() {}

    public init(error: String, replaced: Bool) {
        self.error = error
        self.replaced = replaced
    }
}
