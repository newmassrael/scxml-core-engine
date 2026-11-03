// SCE-GENERATED — DO NOT EDIT
// source-hash: 848898b5bbb016bf799fdf5a1235f8e512e7516e53f20e55bfd5887716ace728
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
