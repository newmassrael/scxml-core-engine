// SCE-GENERATED — DO NOT EDIT
// source-hash: 8b4828d778fc1da05444ac6a052eb44c820b7a80e1f4075fc52bc6e97e8a070a
#![doc = "SCE-MAP: event_schedule.scxml:18 :: _forge_body"]
// SCE-MAP: event_schedule.scxml:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: calendar.schedule
// Do not edit — regenerate from the source SCXML file.

/// The event this schema types (`sce:event-name`), as a host names it.
#[allow(dead_code)]
pub const EVENT_NAME: &str = "calendar.schedule";
/// The revision of the schema (`sce:revision`, 1 when the document declares none).
#[allow(dead_code)]
pub const REVISION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EventSchedulePayload {
    pub zone: i64,
    pub start: i64,
    pub length: i64,
    pub freq: i64,
    pub interval: i64,
    pub days: i64,
    pub count: i64,
    pub until: i64,
    pub bymonth: i64,
    pub md_pos: i64,
    pub md_neg: i64,
    pub invalid: i64,
    pub op0: i64,
    pub op1: i64,
    pub op2: i64,
    pub op3: i64,
    pub op4: i64,
    pub op5: i64,
    pub op6: i64,
    pub on0: i64,
    pub on1: i64,
    pub on2: i64,
    pub on3: i64,
    pub on4: i64,
    pub on5: i64,
    pub on6: i64,
    pub sp_pos: i64,
    pub sp_neg: i64,
    pub wkst: i64,
    pub wn_pos: i64,
    pub wn_neg: i64,
    pub yd0: i64,
    pub yd1: i64,
    pub yd2: i64,
    pub yd3: i64,
    pub yd4: i64,
    pub yd5: i64,
    pub ydn0: i64,
    pub ydn1: i64,
    pub ydn2: i64,
    pub ydn3: i64,
    pub ydn4: i64,
    pub ydn5: i64,
    pub gone: i64,
    pub flag: i64,
}
