// SCE-GENERATED — DO NOT EDIT
// source-hash: 9e46d5ba618e2f5a559e7793b0e0ddd1d7ba8428fd3d96128f06266cf4750b56
// SCE-MAP: sync_response.scxml:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: sync.response
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum SyncResponse {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "sync.response"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 130
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct SyncResponsePayload: Equatable, Sendable, SceDefault {
    public var kind: UInt8 = 0
    public var status: Int32 = 0
    public var create: Bool = false
    public var davError: Bool = false
    public var now: Int64 = 0
    public var retryAfter: Int64 = 0

    public init() {}

    public init(kind: UInt8, status: Int32, create: Bool, davError: Bool, now: Int64, retryAfter: Int64) {
        self.kind = kind
        self.status = status
        self.create = create
        self.davError = davError
        self.now = now
        self.retryAfter = retryAfter
    }
}
