// SCE-GENERATED — DO NOT EDIT
// source-hash: 16d76c0f9ed42e7e13706b7e291287d0861ab6ac1014d873b4e04ff755d59e01
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
