// SCE-GENERATED — DO NOT EDIT
// source-hash: f7347a8b98455a65e31ae2b3af12ef65d68a284242bd71a8102ab373827910fd
// SCE-MAP: text_slot.scxml:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: text.slot
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.text_slot

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "text.slot"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most UTF-8 bytes `text` holds (`sce:max-size`), as this build resolved it. */
const val TEXT_MAX_SIZE: UInt = 32u

data class TextSlotPayload(    val slot: UInt,    val version: Long,    val text: String)
