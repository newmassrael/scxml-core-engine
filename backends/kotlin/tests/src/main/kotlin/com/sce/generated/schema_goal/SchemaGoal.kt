// SCE-GENERATED — DO NOT EDIT
// source-hash: b67ac92c47dabab09bc66be64b0a4b4a6acfabe17f95412102ed640a717fc04f
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
