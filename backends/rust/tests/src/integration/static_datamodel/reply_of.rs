// SCE-GENERATED — DO NOT EDIT
// source-hash: 16d76c0f9ed42e7e13706b7e291287d0861ab6ac1014d873b4e04ff755d59e01
#![doc = "SCE-MAP: reply_of.scxml:4 :: _forge_body"]
// SCE-MAP: reply_of.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

/// The type the template `reply_of` matches: `record:JobReply`, as the
/// unit of its type declares it.
#[allow(dead_code)]
pub type Value = super::job_reply::JobReplyPayload;

/// Whether `value` matches the template `reply_of` (ES 201 873-1 Annex B.1,
/// docs/SCE_TEST_PROFILE.md §6.1).
/// - `param_answers`: the formal parameter `answers`, of type `uint32`.
/// - `param_code`: the formal parameter `code`, of type `uint32`.
#[allow(dead_code, unused_variables)]
pub fn matches(value: &Value, param_answers: u32, param_code: u32) -> bool {
    value.job == param_answers && value.code == param_code
}

/// The value the specific template `reply_of` sends (docs/SCE_TEST_PROFILE.md
/// §6.2): every field it names at its value, every field it omits absent. It matches the
/// template by `matches`, given the same parameters.
#[allow(dead_code, unused_variables)]
pub fn value(param_answers: u32, param_code: u32) -> Value {
    super::job_reply::JobReplyPayload {
        job: param_answers,
        code: param_code,
    }
}
