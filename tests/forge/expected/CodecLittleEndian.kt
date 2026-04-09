// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_little_endian

data class CodecLittleEndian(
    val sensorId: UByte,
    val value: UShort,
    val status: UByte
) {
    fun encode(): ByteArray = byteArrayOf(
        sensorId.toByte(),
        (value.toInt() and 0xFF).toByte(),
        (value.toInt() ushr 8 and 0xFF).toByte(),
        status.toByte()
    )

    companion object {
        fun decode(raw: ByteArray): CodecLittleEndian? {
            if (raw.size < 4) return null
            return CodecLittleEndian(
                sensorId = raw[0].toUByte(),
                value = ((raw[1].toInt() and 0xFF) or ((raw[2].toInt() and 0xFF) shl 8)).toUShort(),
                status = raw[3].toUByte()
            )
        }
    }
}