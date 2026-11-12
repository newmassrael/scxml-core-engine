// SCE-GENERATED — DO NOT EDIT
// source-hash: 4e65a893782852676bb9b2d81e2da91064cf31e9245b09ee7b72fd1a04b1ed23
#![doc = "SCE-MAP: job_status.scxml:4 :: _forge_body"]
// SCE-MAP: job_status.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: job.status
// Do not edit — regenerate from the source SCXML file.

/// The event this schema types (`sce:event-name`), as a host names it.
#[allow(dead_code)]
pub const EVENT_NAME: &str = "job.status";
/// The revision of the schema (`sce:revision`, 1 when the document declares none).
#[allow(dead_code)]
pub const REVISION: u32 = 1;
/// The most UTF-8 bytes `note` holds (the bound its type writes), as this build resolved it.
#[allow(dead_code)]
pub const NOTE_MAX_SIZE: u32 = 32;
/// The most elements `codes` holds (the bound its type writes), as this build resolved it.
#[allow(dead_code)]
pub const CODES_MAX_COUNT: u32 = 4;
/// The most bytes the event's `_event.data` JSON text takes: every list full, every
/// text at its bound, every number at its longest spelling.
#[allow(dead_code)]
pub const DATA_MAX_LEN: u64 = 296;
/// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
/// `$id` of `JSON_SCHEMA`.
#[allow(dead_code)]
pub const SCHEMA_ID: &str = "urn:sce:schema:job_status:1";
/// The payload as a JSON Schema (2020-12): one spelling of each value, every
/// bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`.
#[allow(dead_code)]
pub const JSON_SCHEMA: &str = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:job_status:1\",\"x-sce-source-hash\":\"4e65a893782852676bb9b2d81e2da91064cf31e9245b09ee7b72fd1a04b1ed23\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"elapsed_ms\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"note\":{\"anyOf\":[{\"type\":\"string\",\"maxLength\":32,\"x-sce-max-bytes\":32},{\"type\":\"null\"}]},\"codes\":{\"type\":\"array\",\"items\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"maxItems\":4},\"tag\":{\"anyOf\":[{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},{\"type\":\"null\"}]},\"retry\":{\"anyOf\":[{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},{\"type\":\"null\"}]},\"count\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295}},\"required\":[\"elapsed_ms\",\"note\",\"codes\",\"tag\",\"retry\",\"count\"],\"additionalProperties\":false}";

#[derive(Debug, Clone, PartialEq)]
pub struct JobStatusPayload {
    pub elapsed_ms: u32,
    pub note: Option<::sce_portable_bytes::SceString<32>>,
    pub codes: ::sce_portable_bytes::BoundedList<u8, 4>,
    pub tag: Option<u8>,
    pub retry: Option<u8>,
    pub count: u32,
}
