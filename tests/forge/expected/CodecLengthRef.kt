// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_length_ref

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecLengthRef()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecLengthRef(
    var msgId: UByte = 0.toUByte(),
    var len: UByte = 0.toUByte(),
    var payload: ByteArray = byteArrayOf()
) {
    fun encode(): ByteArray {
        val r = mutableListOf<Byte>()
        r.add(msgId.toByte())
        r.add(len.toByte())
        r.addAll(payload.toList())
        return r.toByteArray()
    }

    companion object {
        fun decode(raw: ByteArray): CodecLengthRef? {
            if (raw.size < 2) return null
            return CodecLengthRef(
                msgId = raw[0].toUByte(),
                len = raw[1].toUByte(),
                payload = raw.copyOfRange(2, 2 + raw[1].toInt())
            )
        }
    }
}
