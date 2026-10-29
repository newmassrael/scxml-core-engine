// SCE-GENERATED — DO NOT EDIT
// source-hash: 047dc621dfd01b6348575becd26162c58fa20fa7680f076526766f91e97d2c5f
// SCE-MAP: schema_goal.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: goal.added
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.schema_goal

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "goal.added"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

data class SchemaGoalPayload(    val id: UInt,    val day: Long?,    val weight: Int)
