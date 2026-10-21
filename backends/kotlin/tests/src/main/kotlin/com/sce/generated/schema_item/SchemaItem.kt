// SCE-GENERATED — DO NOT EDIT
// source-hash: 9ec1a747bc5501f93d66b3929221786d0a45ece3c23f323814ee50eb28f290d9
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
