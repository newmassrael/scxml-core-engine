// SCE-GENERATED — DO NOT EDIT
// source-hash: 982a23a5ec15b8216f29e9e8e4ba6c1047dee0d66671c7835c2776a4de5a13d2
// SCE-MAP: signed_reading_sent.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

import SceRuntime
import SceTestProfile

/// The test template `signed_reading_sent` (docs/SCE_TEST_PROFILE.md §11.4): a caseless enum of
/// the module, as an algorithm's unit is, so the `Value`, `matches` and `value` of two templates
/// never share a name, and a template another one references is called through its enum.
public enum SignedReadingSent {
    /// The type the template matches: `record:SignedReading`, as the unit of its type declares it.
    public typealias Value = SignedReadingPayload

    /// Whether `value` matches the template (ES 201 873-1 Annex B.1,
    /// docs/SCE_TEST_PROFILE.md §6.1).
    public static func matches(_ value: Value) -> Bool {
        return value.delta == -7 && value.offset == -300 && value.level == -70000 && value.stamp == -5000000000 && value.total == 10000000000000000000
    }

    /// The value the specific template sends (docs/SCE_TEST_PROFILE.md §6.2): every field it
    /// names at its value, every field it omits absent. It matches the template by `matches`,
    /// given the same parameters.
    public static func value() -> Value {
        return SignedReadingPayload(delta: -7, offset: -300, level: -70000, stamp: -5000000000, total: 10000000000000000000)
    }

    /// The name of the record `Value` is, as the types module of the suite's TTCN-3 export names it
    /// (docs/SCE_TEST_PROFILE.md §9.5): the `type` of a message of it in the test log (§12.5).
    public static let TYPE: String = "signed_reading"

    /// `value` in the canonical form of the test log (docs/SCE_TEST_PROFILE.md §12.5): each integer
    /// as the text of its digits, a byte string in upper-case hex, an enum and a case of a union by
    /// their declared names, an absent field omitted. A send of this template carries it beside the
    /// value it sends (`SceTestProfile.messageValue`).
    public static func canonical(_ value: Value) -> SceRuntime.JSONValue {
        return SceTestProfile.Canonical.record(("delta", SceTestProfile.Canonical.integer(value.delta)), ("offset", SceTestProfile.Canonical.integer(value.offset)), ("level", SceTestProfile.Canonical.integer(value.level)), ("stamp", SceTestProfile.Canonical.integer(value.stamp)), ("total", SceTestProfile.Canonical.integer(value.total)))
    }
}
