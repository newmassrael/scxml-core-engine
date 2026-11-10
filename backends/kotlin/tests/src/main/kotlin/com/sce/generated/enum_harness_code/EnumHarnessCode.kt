// SCE-GENERATED — DO NOT EDIT
// source-hash: d8c75e4d0233623996dc9a7cc3bdbe850af8d4f91ab023212ebd9f938cf7955f
// SCE-MAP: enum_harness_code.scxml:19 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.enum_harness_code

@OptIn(ExperimentalUnsignedTypes::class)
enum class EnumHarnessCode(val raw: UByte) {
    NOT_JSON(1u.toUByte()),
    NOT_OBJECT(2u.toUByte()),
    TOO_DEEP(3u.toUByte()),
    MISSING(4u.toUByte()),
    NULL_NOT_OPTIONAL(5u.toUByte()),
    WRONG_KIND(6u.toUByte()),
    BAD_NUMBER(7u.toUByte()),
    OUT_OF_RANGE(8u.toUByte()),
    TOO_LONG(9u.toUByte()),
    TOO_MANY(10u.toUByte()),
    UNKNOWN_VARIANT(11u.toUByte()),
    NOT_BYTE_TEXT(12u.toUByte()),
    NUL_IN_TEXT(13u.toUByte()),
    TOO_FEW(14u.toUByte()),
    NOT_OFFERED(15u.toUByte()),
    ;

    /** The carrier value this variant declares. */
    fun toUnderlying(): UByte = raw

    /** The name the document declares for this variant. */
    fun declaredName(): String = when (this) {
        NOT_JSON -> "notJson"
        NOT_OBJECT -> "notObject"
        TOO_DEEP -> "tooDeep"
        MISSING -> "missing"
        NULL_NOT_OPTIONAL -> "nullNotOptional"
        WRONG_KIND -> "wrongKind"
        BAD_NUMBER -> "badNumber"
        OUT_OF_RANGE -> "outOfRange"
        TOO_LONG -> "tooLong"
        TOO_MANY -> "tooMany"
        UNKNOWN_VARIANT -> "unknownVariant"
        NOT_BYTE_TEXT -> "notByteText"
        NUL_IN_TEXT -> "nulInText"
        TOO_FEW -> "tooFew"
        NOT_OFFERED -> "notOffered"
    }

    companion object {
        /**
         * The variant [raw] declares, or null — the declared set is closed,
         * so a value outside it is not a value of this type.
         */
        fun fromUnderlying(raw: UByte): EnumHarnessCode? =
            entries.firstOrNull { it.raw == raw }
    }
}
