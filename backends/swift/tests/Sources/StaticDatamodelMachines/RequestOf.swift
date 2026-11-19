// SCE-GENERATED — DO NOT EDIT
// source-hash: 945f3336b5577424df7d0d55b748b92fb98290481f319eb98f4c161a0343addf
// source: request_of.scxml
// SCE-MAP: request_of.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

import SceRuntime
import SceTestProfile

/// The test template `request_of` (docs/SCE_TEST_PROFILE.md §11.4): a caseless enum of
/// the module, as an algorithm's unit is, so the `Value`, `matches` and `value` of two templates
/// never share a name, and a template another one references is called through its enum.
public enum RequestOf {
    /// The type the template matches: `record:JobRequest`, as the unit of its type declares it.
    public typealias Value = JobRequestPayload

    /// Whether `value` matches the template (ES 201 873-1 Annex B.1,
    /// docs/SCE_TEST_PROFILE.md §6.1).
    ///
    /// `param_wanted` is the formal parameter `wanted`, of type `uint32`.
    public static func matches(_ value: Value, _ param_wanted: UInt32) -> Bool {
        return value.job == param_wanted
    }

    /// The value the specific template sends (docs/SCE_TEST_PROFILE.md §6.2): every field it
    /// names at its value, every field it omits absent. It matches the template by `matches`,
    /// given the same parameters.
    public static func value(_ param_wanted: UInt32) -> Value {
        return JobRequestPayload(job: param_wanted)
    }

    /// The name of the record `Value` is, as the types module of the suite's TTCN-3 export names it
    /// (docs/SCE_TEST_PROFILE.md §9.5): the `type` of a message of it in the test log (§12.5).
    public static let TYPE: String = "job_request"

    /// `value` in the canonical form of the test log (docs/SCE_TEST_PROFILE.md §12.5): each integer
    /// as the text of its digits, a byte string in upper-case hex, an enum and a case of a union by
    /// their declared names, an absent field omitted. A send of this template carries it beside the
    /// value it sends (`SceTestProfile.messageValue`).
    public static func canonical(_ value: Value) -> SceRuntime.JSONValue {
        return SceTestProfile.Canonical.record(("job", SceTestProfile.Canonical.integer(value.job)))
    }
}
