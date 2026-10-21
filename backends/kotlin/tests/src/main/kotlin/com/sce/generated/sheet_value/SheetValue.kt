// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
// SCE-MAP: sheet_value.scxml:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: merge.sheet_value
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.sheet_value

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "merge.sheet_value"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

data class SheetValuePayload(    val value: Long,    val fault: UByte)
