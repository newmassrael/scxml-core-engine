// SCE-GENERATED — DO NOT EDIT
// source-hash: 69b0ebe2b258c0c641661c1efeb701cca9163d999b33bc95a7ea77aa603a45be
// SCE-MAP: job_report_of.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

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
}
