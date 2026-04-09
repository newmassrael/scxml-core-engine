// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_subbyte

data class CodecSubbyte(
    val priority: UByte,
    val channel: UByte,
    val direction: UByte
) {
    fun encode(): ByteArray = byteArrayOf(
        ((priority.toInt() and 0x07 shl 5) or (channel.toInt() and 0x07 shl 2) or (direction.toInt() and 0x03 shl 0)).toByte()
    )

    companion object {
        fun decode(raw: ByteArray): CodecSubbyte? {
            if (raw.size < 1) return null
            return CodecSubbyte(
                priority = ((raw[0].toInt() ushr 5) and 0x07).toUByte(),
                channel = ((raw[0].toInt() ushr 2) and 0x07).toUByte(),
                direction = ((raw[0].toInt() ushr 0) and 0x03).toUByte()
            )
        }
    }
}