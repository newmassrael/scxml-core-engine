// SCE-GENERATED — DO NOT EDIT
// source-hash: 75f53f6deefa1a9e19646e018ad72cacc30c4c7ca7f7f96f21be9f17f58cfdae
#![doc = "SCE-MAP: form_extras.scxml:7 :: _forge_body"]
// SCE-MAP: form_extras.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: form.extras
// Do not edit — regenerate from the source SCXML file.

use super::shape_kind_open;

/// The event this schema types (`sce:event-name`), as a host names it.
#[allow(dead_code)]
pub const EVENT_NAME: &str = "form.extras";
/// The revision of the schema (`sce:revision`, 1 when the document declares none).
#[allow(dead_code)]
pub const REVISION: u32 = 1;
/// The most elements `slots` holds (the bound its type writes), as this build resolved it.
#[allow(dead_code)]
pub const SLOTS_MAX_COUNT: u32 = 3;
/// The most elements `flags` holds (the bound its type writes), as this build resolved it.
#[allow(dead_code)]
pub const FLAGS_MAX_COUNT: u32 = 2;
/// The most elements `spare` holds (the bound its type writes), as this build resolved it.
#[allow(dead_code)]
pub const SPARE_MAX_COUNT: u32 = 2;
/// The most elements `grid` holds (the bound its type writes), as this build resolved it.
#[allow(dead_code)]
pub const GRID_MAX_COUNT: u32 = 2;
/// The most bytes the event's `_event.data` JSON text takes: every list full, every
/// text at its bound, every number at its longest spelling.
#[allow(dead_code)]
pub const DATA_MAX_LEN: u64 = 146;
/// The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the
/// `$id` of `JSON_SCHEMA`.
#[allow(dead_code)]
pub const SCHEMA_ID: &str = "urn:sce:schema:form_extras:1";
// No JSON_SCHEMA: the enum `Shape` is open (`sce:strict-variants="false"`): a value of it may be a number no name spells, and a grammar spells a value of an enum by its name.

#[derive(Debug, Clone, PartialEq)]
pub struct FormExtrasPayload {
    pub shape: shape_kind_open::ShapeKindOpen,
    pub slots: ::sce_portable_bytes::BoundedList<Option<u16>, 3>,
    pub least: i64,
    pub flags: ::sce_portable_bytes::BoundedList<bool, 2>,
    pub spare: Option<::sce_portable_bytes::BoundedList<u8, 2>>,
    pub grid: ::sce_portable_bytes::BoundedList<::sce_portable_bytes::BoundedList<u8, 2>, 2>,
}
