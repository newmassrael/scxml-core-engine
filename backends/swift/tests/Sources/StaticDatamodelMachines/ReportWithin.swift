// SCE-GENERATED — DO NOT EDIT
// source-hash: 84c1b8f490600d7eecaa6c31326cf34f41eda9322dd585954c1b6ff6e016fe19
// SCE-MAP: report_within.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

/// The test template `report_within` (docs/SCE_TEST_PROFILE.md §11.4): a caseless enum of
/// the module, as an algorithm's unit is, so the `Value`, `matches` and `value` of two templates
/// never share a name, and a template another one references is called through its enum.
public enum ReportWithin {
    /// The type the template matches: `record:JobStatus`, as the unit of its type declares it.
    public typealias Value = JobStatusPayload

    /// Whether `value` matches the template (ES 201 873-1 Annex B.1,
    /// docs/SCE_TEST_PROFILE.md §6.1).
    ///
    /// `param_limit` is the formal parameter `limit`, of type `uint32`.
    public static func matches(_ value: Value, _ param_limit: UInt32) -> Bool {
        return value.elapsed_ms == 0 && value.note.map { present_0 -> Bool in present_0 == "started on time" } ?? false && value.codes.count == 1 && value.codes[0] == 7 && value.tag == nil && value.retry == nil && ElapsedAtMost.matches(value.count, param_limit)
    }
}
