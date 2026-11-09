// SCE-GENERATED — DO NOT EDIT
// source-hash: 9f0e28d53caaa29bc10122250abf2dd34aa356f984c7dd48a910705065c74b3a
// SCE-MAP: policy_settings.scxml:28 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: policy.settings
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The event this schema types and its revision, as a host names them. A caseless
/// enum of the module, as an algorithm's is, so two schemas never share a constant.
public enum PolicySettings {
    /// The event this schema types (`sce:event-name`).
    public static let eventName: String = "policy.settings"

    /// The revision of the schema (`sce:revision`, 1 when the document declares none).
    public static let revision: UInt32 = 1

    /// The most bytes the event's `_event.data` JSON text takes: every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 70

    /// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of
    /// `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:policy_settings:1"

    /// The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this
    /// build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:policy_settings:1\",\"x-sce-source-hash\":\"9f0e28d53caaa29bc10122250abf2dd34aa356f984c7dd48a910705065c74b3a\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"mode\":{\"enum\":[\"relaxed\",\"strict\"]},\"limitDays\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":65535},\"burst\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"locked\":{\"type\":\"boolean\"}},\"required\":[\"mode\",\"limitDays\",\"burst\",\"locked\"],\"additionalProperties\":false}"
}

/// The payload of the event: a struct of the schema's fields in the schema's order,
/// each at its zero until written. A value, so a host that reads one holds a copy.
public struct PolicySettingsPayload: Equatable, Sendable, SceDefault {
    public var mode: PolicyMode = PolicyMode.allCases[0]
    public var limitDays: UInt16 = 0
    public var burst: UInt32 = 0
    public var locked: Bool = false

    public init() {}

    public init(mode: PolicyMode, limitDays: UInt16, burst: UInt32, locked: Bool) {
        self.mode = mode
        self.limitDays = limitDays
        self.burst = burst
        self.locked = locked
    }
}
