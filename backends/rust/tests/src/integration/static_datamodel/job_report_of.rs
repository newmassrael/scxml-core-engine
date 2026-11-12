// SCE-GENERATED — DO NOT EDIT
// source-hash: 4e65a893782852676bb9b2d81e2da91064cf31e9245b09ee7b72fd1a04b1ed23
#![doc = "SCE-MAP: job_report_of.scxml:6 :: _forge_body"]
// SCE-MAP: job_report_of.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

/// The type the template `job_report_of` matches: `record:JobStatus`, as the
/// unit of its type declares it.
#[allow(dead_code)]
pub type Value = super::job_status::JobStatusPayload;

/// Whether `value` matches the template `job_report_of` (ES 201 873-1 Annex B.1,
/// docs/SCE_TEST_PROFILE.md §6.1).
/// - `param_elapsed`: the formal parameter `elapsed`, of type `uint32`.
/// - `param_note`: the formal parameter `note`, of type `string[32]`.
/// - `param_code`: the formal parameter `code`, of type `uint8`.
#[allow(dead_code, unused_variables)]
pub fn matches(
    value: &Value,
    param_elapsed: u32,
    param_note: &::sce_portable_bytes::SceString<32>,
    param_code: u8,
) -> bool {
    value.elapsed_ms == param_elapsed
        && (match &value.note {
            Some(present_0) => *present_0 == *param_note,
            None => false,
        })
        && value.codes.len() == 2
        && value.codes[0] == param_code
        && value.codes[1] == 255
        && value.tag.is_none()
        && (match &value.retry {
            Some(present_0) => *present_0 == 2,
            None => false,
        })
        && value.count == 4294967295
}

/// The value the specific template `job_report_of` sends (docs/SCE_TEST_PROFILE.md
/// §6.2): every field it names at its value, every field it omits absent. It matches the
/// template by `matches`, given the same parameters.
#[allow(dead_code, unused_variables)]
pub fn value(
    param_elapsed: u32,
    param_note: &::sce_portable_bytes::SceString<32>,
    param_code: u8,
) -> Value {
    super::job_status::JobStatusPayload {
        elapsed_ms: param_elapsed,
        note: Some(param_note.clone()),
        codes: ::sce_portable_bytes::BoundedList::proven(&[param_code, 255]),
        tag: None,
        retry: Some(2),
        count: 4294967295,
    }
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
