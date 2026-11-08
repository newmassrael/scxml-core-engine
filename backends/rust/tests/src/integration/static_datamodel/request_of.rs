// SCE-GENERATED — DO NOT EDIT
// source-hash: 011d804f887126b888ea6ba5b4213d56e0f5c8740555a154c83e9b8d0105c4b6
#![doc = "SCE-MAP: request_of.scxml:4 :: _forge_body"]
// SCE-MAP: request_of.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

/// The type the template `request_of` matches: `record:JobRequest`, as the
/// unit of its type declares it.
#[allow(dead_code)]
pub type Value = super::job_request::JobRequestPayload;

/// Whether `value` matches the template `request_of` (ES 201 873-1 Annex B.1,
/// docs/SCE_TEST_PROFILE.md §6.1).
/// - `param_job`: the formal parameter `job`, of type `uint32`.
#[allow(dead_code, unused_variables)]
pub fn matches(value: &Value, param_job: u32) -> bool {
    value.job == param_job
}

/// The value the specific template `request_of` sends (docs/SCE_TEST_PROFILE.md
/// §6.2): every field it names at its value, every field it omits absent. It matches the
/// template by `matches`, given the same parameters.
#[allow(dead_code, unused_variables)]
pub fn value(param_job: u32) -> Value {
    super::job_request::JobRequestPayload { job: param_job }
}
