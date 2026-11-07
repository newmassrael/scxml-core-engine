// SCE-GENERATED — DO NOT EDIT
// source-hash: 44e42be193b6ad6075c6d0b043abdcd82b148ab78d0b09d43794619a04e8d622
#![doc = "SCE-MAP: job_report.scxml:4 :: _forge_body"]
// SCE-MAP: job_report.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-template")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

/// The type the template `job_report` matches: `record:JobStatus`, as the
/// unit of its type declares it.
#[allow(dead_code)]
pub type Value = super::job_status::JobStatusPayload;

/// Whether `value` matches the template `job_report` (ES 201 873-1 Annex B.1,
/// docs/SCE_TEST_PROFILE.md §6.1).
#[allow(dead_code, unused_variables)]
pub fn matches(value: &Value) -> bool {
    value.elapsed_ms == 0
        && (match &value.note {
            Some(present_0) => *present_0 == "started on time",
            None => false,
        })
        && value.codes.len() == 1
        && value.codes[0] == 7
        && value.tag.is_none()
        && value.retry.is_none()
        && value.count == 1
}

/// The value the specific template `job_report` sends (docs/SCE_TEST_PROFILE.md
/// §6.2): every field it names at its value, every field it omits absent. It matches the
/// template by `matches`, given the same parameters.
#[allow(dead_code, unused_variables)]
pub fn value() -> Value {
    super::job_status::JobStatusPayload {
        elapsed_ms: 0,
        note: Some(::sce_portable_bytes::sce_text!(32, "started on time")),
        codes: ::sce_portable_bytes::BoundedList::proven(&[7]),
        tag: None,
        retry: None,
        count: 1,
    }
}
