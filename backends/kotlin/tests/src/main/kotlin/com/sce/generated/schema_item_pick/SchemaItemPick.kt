// SCE-GENERATED — DO NOT EDIT
// source-hash: c4657e911a812ec796ce0b3c5be3eeef2b66326118a66c11d56342e555c6a85c
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

/** The most UTF-8 bytes `title` holds (the bound its type writes), as this build resolved it. */
const val TITLE_MAX_SIZE: UInt = 16u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 144uL

data class SchemaItemPickPayload(    val id: UInt,    val title: String,    val count: Int)
