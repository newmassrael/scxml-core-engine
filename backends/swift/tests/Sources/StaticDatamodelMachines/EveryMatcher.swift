// SCE-GENERATED — DO NOT EDIT
// source-hash: 69b0ebe2b258c0c641661c1efeb701cca9163d999b33bc95a7ea77aa603a45be
// SCE-MAP: every_matcher.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceTestProfile

/// The test template `every_matcher` (docs/SCE_TEST_PROFILE.md §11.4): a caseless enum of
/// the module, as an algorithm's unit is, so the `Value`, `matches` and `value` of two templates
/// never share a name, and a template another one references is called through its enum.
public enum EveryMatcher {
    /// The type the template matches: `record:JobStatus`, as the unit of its type declares it.
    public typealias Value = JobStatusPayload

    /// Whether `value` matches the template (ES 201 873-1 Annex B.1,
    /// docs/SCE_TEST_PROFILE.md §6.1).
    ///
    /// `param_limit` is the formal parameter `limit`, of type `uint32`.
    public static func matches(_ value: Value, _ param_limit: UInt32) -> Bool {
        return (value.elapsed_ms == 1 || (value.elapsed_ms >= 10 && value.elapsed_ms <= param_limit)) && (value.note == nil || (value.note != nil && value.note.map { present_0 -> Bool in SceTestProfile.textLength(present_0) <= 8 } ?? true)) && value.codes.count == 3 && value.codes[0] == 3 && value.codes[2] != 0 && value.codes.count >= 1 && value.codes.count <= 4 && value.retry == nil && ElapsedAtMost.matches(value.count, param_limit)
    }
}
