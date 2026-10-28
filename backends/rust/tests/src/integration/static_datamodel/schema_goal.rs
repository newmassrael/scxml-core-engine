// SCE-GENERATED — DO NOT EDIT
// source-hash: b67ac92c47dabab09bc66be64b0a4b4a6acfabe17f95412102ed640a717fc04f
#![doc = "SCE-MAP: schema_goal.scxml:10 :: _forge_body"]
// SCE-MAP: schema_goal.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: goal.added
// Do not edit — regenerate from the source SCXML file.

/// The event this schema types (`sce:event-name`), as a host names it.
#[allow(dead_code)]
pub const EVENT_NAME: &str = "goal.added";
/// The revision of the schema (`sce:revision`, 1 when the document declares none).
#[allow(dead_code)]
pub const REVISION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SchemaGoalPayload {
    pub id: u32,
    pub day: Option<i64>,
    pub weight: i32,
}
