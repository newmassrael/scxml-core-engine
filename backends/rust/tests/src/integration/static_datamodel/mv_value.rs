// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
#![doc = "SCE-MAP: mv_value.scxml:27 :: _forge_body"]
// SCE-MAP: mv_value.scxml:27 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: merge.mv_value
// Do not edit — regenerate from the source SCXML file.

/// The event this schema types (`sce:event-name`), as a host names it.
#[allow(dead_code)]
pub const EVENT_NAME: &str = "merge.mv_value";
/// The revision of the schema (`sce:revision`, 1 when the document declares none).
#[allow(dead_code)]
pub const REVISION: u32 = 1;
/// The most UTF-8 bytes `text` holds (`sce:max-size`, the bound `sce:limit="merge_text"` names), as this build resolved it.
#[allow(dead_code)]
pub const TEXT_MAX_SIZE: u32 = 96;

#[derive(Debug, Clone, PartialEq)]
pub struct MvValuePayload {
    pub id: u64,
    pub field: u64,
    pub value: i64,
    pub text: ::sce_portable_bytes::SceString<96>,
    pub wall_time: i64,
    pub counter: u32,
    pub node_id: u64,
}
