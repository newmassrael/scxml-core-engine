// SCE-GENERATED — DO NOT EDIT
// source-hash: 048fe3b13b4ba344e3f60c0d3a93bf8f8568ab7bd254b4d4763d7e631895e2f6
// SCE-MAP: schema_item.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: item.added
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.schema_item
import com.sce.generated.enum_item_status.*

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "item.added"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

data class SchemaItemPayload(    val id: UInt,    val status: EnumItemStatus)
