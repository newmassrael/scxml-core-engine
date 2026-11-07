// SCE-GENERATED — DO NOT EDIT
// source-hash: d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b
// SCE-MAP: order_payment.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="union")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.order_payment

/** The revision of the union: its tag enum's, `payment_kind`'s (a union declares none of its own). */
const val REVISION: UInt = 1u

/** The most bytes a value's JSON text takes: its largest case, every list full, every text at its bound, every number at its longest spelling. */
const val DATA_MAX_LEN: ULong = 70uL

/** The id of the union's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `JSON_SCHEMA`. */
const val SCHEMA_ID: String = "urn:sce:schema:order_payment:1"

/** A value as a JSON Schema (2020-12): the `oneOf` of the cases, each its tag `kind` beside the fields of its record, every bound as this build resolved it. */
const val JSON_SCHEMA: String = "{\"\$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"\$id\":\"urn:sce:schema:order_payment:1\",\"x-sce-source-hash\":\"d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"oneOf\":[{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"card\"},\"last4\":{\"type\":\"string\",\"maxLength\":4,\"x-sce-max-bytes\":4},\"amount\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295}},\"required\":[\"kind\",\"last4\",\"amount\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"cash\"}},\"required\":[\"kind\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"voucher\"},\"voucher_code\":{\"type\":\"string\",\"maxLength\":4,\"pattern\":\"^[\\\\u0000-\\\\u00ff]*\$\"}},\"required\":[\"kind\",\"voucher_code\"],\"additionalProperties\":false}]}"

/**
 * A value of `order_payment`: exactly one case, told apart by its tag `kind`, a value of
 * [com.sce.generated.payment_kind.PaymentKind]. Two values are equal when they are in one case and their records are equal.
 */
sealed interface OrderPayment {
    /** The case this value is in: the tag its text writes under `kind`. */
    val kind: com.sce.generated.payment_kind.PaymentKind

    /** The case `card`, holding its record. */
    data class Card(val value: com.sce.generated.card_payment.CardPaymentPayload) : com.sce.generated.order_payment.OrderPayment {
        override val kind: com.sce.generated.payment_kind.PaymentKind
            get() = com.sce.generated.payment_kind.PaymentKind.CARD
    }

    /** The case `cash`, its tag alone. */
    data object Cash : com.sce.generated.order_payment.OrderPayment {
        override val kind: com.sce.generated.payment_kind.PaymentKind
            get() = com.sce.generated.payment_kind.PaymentKind.CASH
    }

    /** The case `voucher`, holding its record. */
    data class Voucher(val value: com.sce.generated.voucher_payment.VoucherPaymentPayload) : com.sce.generated.order_payment.OrderPayment {
        override val kind: com.sce.generated.payment_kind.PaymentKind
            get() = com.sce.generated.payment_kind.PaymentKind.VOUCHER
    }

    /** The record of the case `card`, read where a test of the tag proved the value is in it. */
    fun provenCard(): com.sce.generated.card_payment.CardPaymentPayload =
        (this as? Card)?.value
            ?: throw IllegalStateException("a order_payment read as card is in ${kind.declaredName()}")

    /** The record of the case `voucher`, read where a test of the tag proved the value is in it. */
    fun provenVoucher(): com.sce.generated.voucher_payment.VoucherPaymentPayload =
        (this as? Voucher)?.value
            ?: throw IllegalStateException("a order_payment read as voucher is in ${kind.declaredName()}")
}
