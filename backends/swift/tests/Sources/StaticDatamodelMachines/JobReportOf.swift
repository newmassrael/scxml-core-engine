// SCE-GENERATED — DO NOT EDIT
// source-hash: 4e65a893782852676bb9b2d81e2da91064cf31e9245b09ee7b72fd1a04b1ed23
// SCE-MAP: job_report_of.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

import SceRuntime
import SceTestProfile

/// The test template `job_report_of` (docs/SCE_TEST_PROFILE.md §11.4): a caseless enum of
/// the module, as an algorithm's unit is, so the `Value`, `matches` and `value` of two templates
/// never share a name, and a template another one references is called through its enum.
public enum JobReportOf {
    /// The type the template matches: `record:JobStatus`, as the unit of its type declares it.
    public typealias Value = JobStatusPayload

    /// Whether `value` matches the template (ES 201 873-1 Annex B.1,
    /// docs/SCE_TEST_PROFILE.md §6.1).
    ///
    /// `param_elapsed` is the formal parameter `elapsed`, of type `uint32`.
    ///
    /// `param_note` is the formal parameter `note`, of type `string[32]`.
    ///
    /// `param_code` is the formal parameter `code`, of type `uint8`.
    public static func matches(_ value: Value, _ param_elapsed: UInt32, _ param_note: String, _ param_code: UInt8) -> Bool {
        return value.elapsed_ms == param_elapsed && value.note.map { present_0 -> Bool in present_0 == param_note } ?? false && value.codes.count == 2 && value.codes[0] == param_code && value.codes[1] == 255 && value.tag == nil && value.retry.map { present_0 -> Bool in present_0 == 2 } ?? false && value.count == 4294967295
    }

    /// The value the specific template sends (docs/SCE_TEST_PROFILE.md §6.2): every field it
    /// names at its value, every field it omits absent. It matches the template by `matches`,
    /// given the same parameters.
    public static func value(_ param_elapsed: UInt32, _ param_note: String, _ param_code: UInt8) -> Value {
        return JobStatusPayload(elapsed_ms: param_elapsed, note: param_note, codes: [param_code, 255], tag: nil, retry: 2, count: 4294967295)
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
