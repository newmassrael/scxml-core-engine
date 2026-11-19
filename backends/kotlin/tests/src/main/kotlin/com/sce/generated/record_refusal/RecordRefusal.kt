// SCE-GENERATED — DO NOT EDIT
// source-hash: 19a127f54c9deebd5f70030e23e03228c6cca2b28c17631c421836bbc812839a
// source: sce:std/shared/record_refusal.scxml
// SCE-MAP: record_refusal.scxml:39 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.record_refusal

@OptIn(ExperimentalUnsignedTypes::class)
enum class RecordRefusal(val raw: UByte) {
    NOT_WRITER(1u.toUByte()),
    NO_ROOM_FOR_VALUE(2u.toUByte()),
    NO_ROOM_FOR_HISTORY(3u.toUByte()),
    NO_ROOM_FOR_DIGEST(4u.toUByte()),
    CHANGE_TOO_LARGE(6u.toUByte()),
    REGISTER_NAMED_TWICE(7u.toUByte()),
    CLOCK_EXHAUSTED(8u.toUByte()),
    PEER_REFUSED(9u.toUByte()),
    ;

    /** The carrier value this variant declares. */
    fun toUnderlying(): UByte = raw

    /** The name the document declares for this variant. */
    fun declaredName(): String = when (this) {
        NOT_WRITER -> "notWriter"
        NO_ROOM_FOR_VALUE -> "noRoomForValue"
        NO_ROOM_FOR_HISTORY -> "noRoomForHistory"
        NO_ROOM_FOR_DIGEST -> "noRoomForDigest"
        CHANGE_TOO_LARGE -> "changeTooLarge"
        REGISTER_NAMED_TWICE -> "registerNamedTwice"
        CLOCK_EXHAUSTED -> "clockExhausted"
        PEER_REFUSED -> "peerRefused"
    }

    companion object {
        /**
         * The variant [raw] declares, or null — the declared set is closed,
         * so a value outside it is not a value of this type.
         */
        fun fromUnderlying(raw: UByte): RecordRefusal? =
            entries.firstOrNull { it.raw == raw }
    }
}
