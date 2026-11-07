// SCE-GENERATED — DO NOT EDIT
// source-hash: d4646eb3233d0326b8220cebf3753ec7bf14af5b5faee8ffa467c7536202f42b
// SCE-MAP: payment_kind.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.payment_kind

@OptIn(ExperimentalUnsignedTypes::class)
enum class PaymentKind(val raw: UByte) {
    CARD(0u.toUByte()),
    CASH(1u.toUByte()),
    VOUCHER(2u.toUByte()),
    ;

    /** The carrier value this variant declares. */
    fun toUnderlying(): UByte = raw

    /** The name the document declares for this variant. */
    fun declaredName(): String = when (this) {
        CARD -> "card"
        CASH -> "cash"
        VOUCHER -> "voucher"
    }

    companion object {
        /**
         * The variant [raw] declares, or null — the declared set is closed,
         * so a value outside it is not a value of this type.
         */
        fun fromUnderlying(raw: UByte): PaymentKind? =
            entries.firstOrNull { it.raw == raw }
    }
}
