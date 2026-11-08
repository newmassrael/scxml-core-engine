// SCE-GENERATED — DO NOT EDIT
// source-hash: f37425ae3865d6dd180df44e52fd892d2910b6883720df85bc9f25b65e2d89d3
#![doc = "SCE-MAP: every_matcher.scxml:4 :: _forge_body"]
// SCE-MAP: every_matcher.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

/// The type the template `every_matcher` matches: `record:JobStatus`, as the
/// unit of its type declares it.
#[allow(dead_code)]
pub type Value = super::job_status::JobStatusPayload;

/// Whether `value` matches the template `every_matcher` (ES 201 873-1 Annex B.1,
/// docs/SCE_TEST_PROFILE.md §6.1).
/// - `param_limit`: the formal parameter `limit`, of type `uint32`.
#[allow(dead_code, unused_variables)]
pub fn matches(value: &Value, param_limit: u32) -> bool {
    (value.elapsed_ms == 1 || (value.elapsed_ms >= 10 && value.elapsed_ms <= param_limit))
        && (value.note.is_none()
            || (value.note.is_some()
                && (match &value.note {
                    Some(present_0) => present_0.chars().count() <= 8,
                    None => true,
                })))
        && value.codes.len() == 3
        && value.codes[0] == 3
        && value.codes[2] != 0
        && !value.codes.is_empty()
        && value.codes.len() <= 4
        && value.retry.is_none()
        && super::elapsed_at_most::matches(&value.count, param_limit)
}
