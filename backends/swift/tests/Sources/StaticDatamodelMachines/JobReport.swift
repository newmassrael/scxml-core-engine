// SCE-GENERATED — DO NOT EDIT
// source-hash: 8957049d6e3664213f07710e16f42947c0b405de8fa5ea61d8821d57277f69f6
// SCE-MAP: job_report.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

import SceRuntime
import SceTestProfile

/// The test template `job_report` (docs/SCE_TEST_PROFILE.md §11.4): a caseless enum of
/// the module, as an algorithm's unit is, so the `Value`, `matches` and `value` of two templates
/// never share a name, and a template another one references is called through its enum.
public enum JobReport {
    /// The type the template matches: `record:JobStatus`, as the unit of its type declares it.
    public typealias Value = JobStatusPayload

    /// Whether `value` matches the template (ES 201 873-1 Annex B.1,
    /// docs/SCE_TEST_PROFILE.md §6.1).
    public static func matches(_ value: Value) -> Bool {
        return value.elapsed_ms == 0 && value.note.map { present_0 -> Bool in present_0 == "started on time" } ?? false && value.codes.count == 1 && value.codes[0] == 7 && value.tag == nil && value.retry == nil && value.count == 1
    }

    /// The value the specific template sends (docs/SCE_TEST_PROFILE.md §6.2): every field it
    /// names at its value, every field it omits absent. It matches the template by `matches`,
    /// given the same parameters.
    public static func value() -> Value {
        return JobStatusPayload(elapsed_ms: 0, note: "started on time", codes: [7], tag: nil, retry: nil, count: 1)
    }

    /// The name of the record `Value` is, as the types module of the suite's TTCN-3 export names it
    /// (docs/SCE_TEST_PROFILE.md §9.5): the `type` of a message of it in the test log (§12.5).
    public static let TYPE: String = "job_status"

    /// `value` in the canonical form of the test log (docs/SCE_TEST_PROFILE.md §12.5): each integer
    /// as the text of its digits, a byte string in upper-case hex, an enum and a case of a union by
    /// their declared names, an absent field omitted. A send of this template carries it beside the
    /// value it sends (`SceTestProfile.messageValue`).
    public static func canonical(_ value: Value) -> SceRuntime.JSONValue {
        return SceTestProfile.Canonical.record(("elapsed_ms", SceTestProfile.Canonical.integer(value.elapsed_ms)), ("note", value.note.map { present_0 -> SceRuntime.JSONValue in SceTestProfile.Canonical.text(present_0) }), ("codes", SceTestProfile.Canonical.list(value.codes.map { item_0 -> SceRuntime.JSONValue in SceTestProfile.Canonical.integer(item_0) })), ("tag", value.tag.map { present_0 -> SceRuntime.JSONValue in SceTestProfile.Canonical.integer(present_0) }), ("retry", value.retry.map { present_0 -> SceRuntime.JSONValue in SceTestProfile.Canonical.integer(present_0) }), ("count", SceTestProfile.Canonical.integer(value.count)))
    }
}
