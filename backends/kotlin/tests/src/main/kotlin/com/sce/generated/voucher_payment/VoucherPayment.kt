// SCE-GENERATED — DO NOT EDIT
// source-hash: d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b
// SCE-MAP: voucher_payment.scxml:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="event-schema")
// Runtime: none
// Event name: payment.voucher
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.voucher_payment

/** The event this schema types (`sce:event-name`), as a host names it. */
const val EVENT_NAME: String = "payment.voucher"

/** The revision of the schema (`sce:revision`, 1 when the document declares none). */
const val REVISION: UInt = 1u

/** The most bytes `voucher_code` holds (the bound its type writes), as this build resolved it. */
const val VOUCHER_CODE_MAX_SIZE: UInt = 4u

/** The most bytes the event's `_event.data` JSON text takes: every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 43uL

/** The id of the payload's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:voucher_payment:1"

/** The payload as a JSON Schema (2020-12): one spelling of each value, every bound as this build resolved it, a text's in UTF-8 bytes in `x-sce-max-bytes`. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:voucher_payment:1\",\"x-sce-source-hash\":\"d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"type\":\"object\",\"properties\":{\"voucher_code\":{\"type\":\"string\",\"maxLength\":4,\"pattern\":\"^[\\\\u0000-\\\\u00ff]*\$\"}},\"required\":[\"voucher_code\"],\"additionalProperties\":false}"

data class VoucherPaymentPayload(    val voucher_code: ByteArray)
