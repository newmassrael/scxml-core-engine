// SCE-GENERATED — DO NOT EDIT
// source-hash: 0834e4c26bd46d9ebaf430da8e162e38dd9e4bbf623718c722dcacef2754df7f
#![doc = "SCE-MAP: every_matcher.scxml:4 :: _forge_body"]
// SCE-MAP: every_matcher.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: sce_runtime
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

/// The name of the record `Value` is, as the types module of the suite's TTCN-3 export names it
/// (docs/SCE_TEST_PROFILE.md §9.5): the `type` of a message of it in the test log (§12.5).
#[allow(dead_code)]
pub const TYPE: &str = "job_status";

/// `value` in the canonical form of the test log (docs/SCE_TEST_PROFILE.md §12.5): each
/// integer as the text of its digits, a byte string in upper-case hex, an enum and a case
/// of a union by their declared names, an absent field omitted. A send of this template
/// carries it beside the value it sends (`sce_rust_runtime::test::message_value`).
#[allow(dead_code)]
pub fn canonical(value: &Value) -> ::sce_rust_runtime::json::Value {
    ::sce_rust_runtime::test::canonical::record([
        (
            "elapsed_ms",
            Some(::sce_rust_runtime::test::canonical::integer(
                value.elapsed_ms,
            )),
        ),
        (
            "note",
            value
                .note
                .as_ref()
                .map(|present_0| ::sce_rust_runtime::test::canonical::text(present_0.as_str())),
        ),
        (
            "codes",
            Some(::sce_rust_runtime::test::canonical::list(
                value
                    .codes
                    .as_slice()
                    .iter()
                    .map(|item_0| ::sce_rust_runtime::test::canonical::integer(*item_0)),
            )),
        ),
        (
            "tag",
            value
                .tag
                .as_ref()
                .map(|present_0| ::sce_rust_runtime::test::canonical::integer(*present_0)),
        ),
        (
            "retry",
            value
                .retry
                .as_ref()
                .map(|present_0| ::sce_rust_runtime::test::canonical::integer(*present_0)),
        ),
        (
            "count",
            Some(::sce_rust_runtime::test::canonical::integer(value.count)),
        ),
    ])
}
