// SCE-GENERATED — DO NOT EDIT
// source-hash: 9b33efdfdb5f3c283f92ca6bd6336ef9097b5ad8d1795089f76b27911b6fde36
// SCE-MAP: gcra_decision.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: rate.gcra_decision
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum GcraDecision {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "rate.gcra_decision"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct GcraDecisionPayload: Equatable, Sendable, SceDefault {
    public var admitted: Bool = false
    public var tat: Int64 = 0
    public var retryAfter: Int64 = 0

    public init() {}

    public init(admitted: Bool, tat: Int64, retryAfter: Int64) {
        self.admitted = admitted
        self.tat = tat
        self.retryAfter = retryAfter
    }
}
