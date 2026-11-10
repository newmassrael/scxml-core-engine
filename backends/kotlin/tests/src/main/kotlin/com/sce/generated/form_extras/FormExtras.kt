// SCE-GENERATED — DO NOT EDIT
// source-hash: 72891728be38ed1db13e4394197bad5a7eaecc0eeac062f21e8fdc061c4ae890
// SCE-MAP: form_extras.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: form.extras
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.form_extras
import com.sce.generated.shape_kind_open.*

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "form.extras"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most elements `slots` holds (the bound its type writes), as this build resolved it. */
const val SLOTS_MAX_COUNT: UInt = 3u

/** The most elements `flags` holds (the bound its type writes), as this build resolved it. */
const val FLAGS_MAX_COUNT: UInt = 2u

/** The most elements `spare` holds (the bound its type writes), as this build resolved it. */
const val SPARE_MAX_COUNT: UInt = 2u

/** The most elements `grid` holds (the bound its type writes), as this build resolved it. */
const val GRID_MAX_COUNT: UInt = 2u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 146uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:form_extras:1"

// No JSON_SCHEMA: the enum `Shape` is open (`sce:strict-variants="false"`): a value of it may be a number no name spells, and a grammar spells a value of an enum by its name.

data class FormExtrasPayload(    val shape: ShapeKindOpen,    val slots: List<UShort?>,    val least: Long,    val flags: List<Boolean>,    val spare: List<UByte>?,    val grid: List<List<UByte>>)
