// SCE-GENERATED — DO NOT EDIT
// source-hash: f6c860c2693fcb3366bc871ff801c9b0ee9e491a1759830f8d83238267cc3b45
// SCE-MAP: policy_scalar_settings.scxml:21 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: policy.scalar_settings
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum PolicyScalarSettings {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "policy.scalar_settings"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct PolicyScalarSettingsPayload: Equatable, Sendable, SceDefault {
    public var strict: Bool = false
    public var limitDays: UInt16 = 0
    public var burst: UInt32 = 0
    public var locked: Bool = false

    public init() {}

    public init(strict: Bool, limitDays: UInt16, burst: UInt32, locked: Bool) {
        self.strict = strict
        self.limitDays = limitDays
        self.burst = burst
        self.locked = locked
    }
}
