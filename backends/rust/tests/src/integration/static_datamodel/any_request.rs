// SCE-GENERATED — DO NOT EDIT
// source-hash: 021fdf0d0c3ee151c664e2ecf6a7e5a797d68aaf3d399831e8dfd49b9e10bec9
#![doc = "SCE-MAP: any_request.scxml:3 :: _forge_body"]
// SCE-MAP: any_request.scxml:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

/// The type the template `any_request` matches: `record:JobRequest`, as the
/// unit of its type declares it.
#[allow(dead_code)]
pub type Value = super::job_request::JobRequestPayload;

/// Whether `value` matches the template `any_request` (ES 201 873-1 Annex B.1,
/// docs/SCE_TEST_PROFILE.md §6.1).
#[allow(dead_code, unused_variables)]
pub fn matches(value: &Value) -> bool {
    true
}
