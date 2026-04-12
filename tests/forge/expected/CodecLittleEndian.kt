// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_little_endian

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecLittleEndian()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecLittleEndian(
    var sensorId: UByte = 0.toUByte(),
    var value: UShort = 0.toUShort(),
    var status: UByte = 0.toUByte()
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
