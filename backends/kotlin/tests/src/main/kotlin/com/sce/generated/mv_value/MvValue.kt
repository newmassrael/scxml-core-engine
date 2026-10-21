// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
// SCE-MAP: mv_value.scxml:27 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: merge.mv_value
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.mv_value

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "merge.mv_value"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most UTF-8 bytes `text` holds (`sce:max-size`, the bound `sce:limit="merge_text"` names), as this build resolved it. */
const val TEXT_MAX_SIZE: UInt = 96u

data class MvValuePayload(    val id: ULong,    val field: ULong,    val value: Long,    val text: String,    val wallTime: Long,    val counter: UInt,    val nodeId: ULong)
