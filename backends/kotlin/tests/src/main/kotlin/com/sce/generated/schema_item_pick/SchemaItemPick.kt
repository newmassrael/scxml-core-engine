// SCE-GENERATED — DO NOT EDIT
// source-hash: b67ac92c47dabab09bc66be64b0a4b4a6acfabe17f95412102ed640a717fc04f
// SCE-MAP: schema_item_pick.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: item.picked
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.schema_item_pick

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "item.picked"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most UTF-8 bytes `title` holds (`sce:max-size`), as this build resolved it. */
const val TITLE_MAX_SIZE: UInt = 16u

data class SchemaItemPickPayload(    val id: UInt,    val title: String,    val count: Int)
