// SCE-GENERATED — DO NOT EDIT
// source-hash: 69b0ebe2b258c0c641661c1efeb701cca9163d999b33bc95a7ea77aa603a45be
#![doc = "SCE-MAP: report_within.scxml:5 :: _forge_body"]
// SCE-MAP: report_within.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

/// The type the template `report_within` matches: `record:JobStatus`, as the
/// unit of its type declares it.
#[allow(dead_code)]
pub type Value = super::job_status::JobStatusPayload;

/// Whether `value` matches the template `report_within` (ES 201 873-1 Annex B.1,
/// docs/SCE_TEST_PROFILE.md §6.1).
/// - `param_limit`: the formal parameter `limit`, of type `uint32`.
#[allow(dead_code, unused_variables)]
pub fn matches(value: &Value, param_limit: u32) -> bool {
    value.elapsed_ms == 0
        && (match &value.note {
            Some(present_0) => *present_0 == "started on time",
            None => false,
        })
        && value.codes.len() == 1
        && value.codes[0] == 7
        && value.tag.is_none()
        && value.retry.is_none()
        && super::elapsed_at_most::matches(&value.count, param_limit)
}
