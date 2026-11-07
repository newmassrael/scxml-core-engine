// SCE-GENERATED — DO NOT EDIT
// source-hash: d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b
// SCE-MAP: order_customer.scxml:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: order.customer
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.order_customer

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "order.customer"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most UTF-8 bytes `full_name` holds (the bound its type writes), as this build resolved it. */
const val FULL_NAME_MAX_SIZE: UInt = 24u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 172uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:order_customer:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:order_customer:1\",\"x-sce-source-hash\":\"d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"full_name\":{\"type\":\"string\",\"maxLength\":24,\"x-sce-max-bytes\":24},\"vip\":{\"type\":\"boolean\"}},\"required\":[\"full_name\",\"vip\"],\"additionalProperties\":false}"

data class OrderCustomerPayload(    val full_name: String,    val vip: Boolean)
