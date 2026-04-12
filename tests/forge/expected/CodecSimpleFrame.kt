// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_simple_frame

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecSimpleFrame()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecSimpleFrame(
    var msgId: UByte = 0.toUByte(),
    var length: UByte = 0.toUByte(),
    var payload: UShort = 0.toUShort()
) {
    fun encode(): ByteArray = byteArrayOf(
        msgId.toByte(),
        length.toByte(),
        (payload.toInt() ushr 8 and 0xFF).toByte(),
        (payload.toInt() and 0xFF).toByte()
    )

    companion object {
        fun decode(raw: ByteArray): CodecSimpleFrame? {
            if (raw.size < 4) return null
            return CodecSimpleFrame(
                msgId = raw[0].toUByte(),
                length = raw[1].toUByte(),
                payload = (((raw[2].toInt() and 0xFF) shl 8) or (raw[3].toInt() and 0xFF)).toUShort()
            )
        }
    }
}
