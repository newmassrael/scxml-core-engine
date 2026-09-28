// SCE-GENERATED — DO NOT EDIT
// source-hash: e2a3a8b4e1357d9d4d92950925a9e4c7d6137a4246c0000110af1331c99ac086
// SCE-MAP: envelope.scxml:22 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec" sce:encoding="cbor")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.envelope

import com.sce.forge.runtime.Cbor
import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.pattern_kind.*
import com.sce.generated.payload_codec.*
import com.sce.generated.rpc_status.*

/**
 * One CBOR map (SCE_FORGE.md §4.6.1). Decode takes the keys in any order and
 * skips one this codec does not declare; encode writes the entries present in
 * ascending key order, every head in its shortest form. A required entry
 * starts at its type's default and an optional one absent (`null`), so
 * `Envelope()` is the infallible constructor every codec offers.
 */
data class Envelope(
    /** Map key 0, required. */
    var id: ByteArray = byteArrayOf(),
    /** Map key 1, required. */
    var source: String = "",
    /** Map key 2, required. */
    var event_type: String = "",
    /** Map key 3, required. */
    var pattern: PatternKind = PatternKind.FIRE_FORGET,
    /** Map key 4, required. */
    var datacontenttype: PayloadCodec = PayloadCodec.NONE,
    /** Map key 5, required. */
    var data: ByteArray = byteArrayOf(),
    /** Map key 6. */
    var subject: String? = null,
    /** Map key 7. */
    var correlation_id: ByteArray? = null,
    /** Map key 8. */
    var reply_to: String? = null,
    /** Map key 9. */
    var invoke_id: ByteArray? = null,
    /** Map key 10. */
    var rpc_status: RpcStatus? = null,
    /** Map key 11. */
    var rpc_error_message: String? = null,
    /** Map key 12. */
    var deadline_unix_ms: ULong? = null,
    /** Map key 14. */
    var sequence_no: ULong? = null,
    /** Map key 15. */
    var routing_id: ByteArray? = null,
    /** Map key 16. */
    var parallel_id: String? = null,
    /** Map key 17. */
    var region_id: String? = null,
    /** Map key 18. */
    var child_session_id: String? = null,
) {
    /**
     * Encode this map into [w]; `null` on success. An entry whose value breaks
     * its declared bound — an exact length, a maximum size — is refused, not
     * written.
     */
    fun encode(w: SceSink): CodecError? {
        var count = 6UL
        if (subject != null) count++
        if (correlation_id != null) count++
        if (reply_to != null) count++
        if (invoke_id != null) count++
        if (rpc_status != null) count++
        if (rpc_error_message != null) count++
        if (deadline_unix_ms != null) count++
        if (sequence_no != null) count++
        if (routing_id != null) count++
        if (parallel_id != null) count++
        if (region_id != null) count++
        if (child_session_id != null) count++
        Cbor.writeMapHead(w, count)?.let { return it }
        run {
            val v = id
            Cbor.writeUint(w, 0UL)?.let { return it }
            if (v.size != 16) return CodecError.CborWrongLength
            Cbor.writeBytes(w, v)?.let { return it }
        }
        run {
            val v = source
            Cbor.writeUint(w, 1UL)?.let { return it }
            if (v.encodeToByteArray().size > 262144) return CodecError.CborOutOfRange
            Cbor.writeText(w, v)?.let { return it }
        }
        run {
            val v = event_type
            Cbor.writeUint(w, 2UL)?.let { return it }
            if (v.encodeToByteArray().size > 262144) return CodecError.CborOutOfRange
            Cbor.writeText(w, v)?.let { return it }
        }
        run {
            val v = pattern
            Cbor.writeUint(w, 3UL)?.let { return it }
            Cbor.writeUint(w, (v.toUnderlying()).toULong())?.let { return it }
        }
        run {
            val v = datacontenttype
            Cbor.writeUint(w, 4UL)?.let { return it }
            Cbor.writeUint(w, (v.toUnderlying()).toULong())?.let { return it }
        }
        run {
            val v = data
            Cbor.writeUint(w, 5UL)?.let { return it }
            if (v.size > 16777216) return CodecError.CborOutOfRange
            Cbor.writeBytes(w, v)?.let { return it }
        }
        subject?.let { v ->
            Cbor.writeUint(w, 6UL)?.let { return it }
            if (v.encodeToByteArray().size > 262144) return CodecError.CborOutOfRange
            Cbor.writeText(w, v)?.let { return it }
        }
        correlation_id?.let { v ->
            Cbor.writeUint(w, 7UL)?.let { return it }
            if (v.size != 16) return CodecError.CborWrongLength
            Cbor.writeBytes(w, v)?.let { return it }
        }
        reply_to?.let { v ->
            Cbor.writeUint(w, 8UL)?.let { return it }
            if (v.encodeToByteArray().size > 262144) return CodecError.CborOutOfRange
            Cbor.writeText(w, v)?.let { return it }
        }
        invoke_id?.let { v ->
            Cbor.writeUint(w, 9UL)?.let { return it }
            if (v.size != 16) return CodecError.CborWrongLength
            Cbor.writeBytes(w, v)?.let { return it }
        }
        rpc_status?.let { v ->
            Cbor.writeUint(w, 10UL)?.let { return it }
            Cbor.writeUint(w, (v.toUnderlying()).toULong())?.let { return it }
        }
        rpc_error_message?.let { v ->
            Cbor.writeUint(w, 11UL)?.let { return it }
            if (v.encodeToByteArray().size > 262144) return CodecError.CborOutOfRange
            Cbor.writeText(w, v)?.let { return it }
        }
        deadline_unix_ms?.let { v ->
            Cbor.writeUint(w, 12UL)?.let { return it }
            Cbor.writeUint(w, v.toULong())?.let { return it }
        }
        sequence_no?.let { v ->
            Cbor.writeUint(w, 14UL)?.let { return it }
            Cbor.writeUint(w, v.toULong())?.let { return it }
        }
        routing_id?.let { v ->
            Cbor.writeUint(w, 15UL)?.let { return it }
            if (v.size != 16) return CodecError.CborWrongLength
            Cbor.writeBytes(w, v)?.let { return it }
        }
        parallel_id?.let { v ->
            Cbor.writeUint(w, 16UL)?.let { return it }
            if (v.encodeToByteArray().size > 262144) return CodecError.CborOutOfRange
            Cbor.writeText(w, v)?.let { return it }
        }
        region_id?.let { v ->
            Cbor.writeUint(w, 17UL)?.let { return it }
            if (v.encodeToByteArray().size > 262144) return CodecError.CborOutOfRange
            Cbor.writeText(w, v)?.let { return it }
        }
        child_session_id?.let { v ->
            Cbor.writeUint(w, 18UL)?.let { return it }
            if (v.encodeToByteArray().size > 262144) return CodecError.CborOutOfRange
            Cbor.writeText(w, v)?.let { return it }
        }
        return null
    }

    /** [encode] into a new array; `null` when encode refuses the value. */
    fun encodeToByteArray(): ByteArray? {
        val out = mutableListOf<Byte>()
        if (encode(MutableListSink(out)) != null) return null
        return out.toByteArray()
    }

    companion object {
        /**
         * Decode one map from [cursor]. On success the cursor advances past
         * it; `null` on any refusal, with the cursor left where it was.
         */
        fun decode(cursor: SceCursor): Envelope? {
            val mark = cursor.mark()
            val value = decodeAt(cursor)
            if (value == null) cursor.reset(mark)
            return value
        }

        private fun decodeAt(c: SceCursor): Envelope? {
            val count = Cbor.readMapLen(c) ?: return null
            var id: ByteArray? = null
            var source: String? = null
            var event_type: String? = null
            var pattern: PatternKind? = null
            var datacontenttype: PayloadCodec? = null
            var data: ByteArray? = null
            var subject: String? = null
            var correlation_id: ByteArray? = null
            var reply_to: String? = null
            var invoke_id: ByteArray? = null
            var rpc_status: RpcStatus? = null
            var rpc_error_message: String? = null
            var deadline_unix_ms: ULong? = null
            var sequence_no: ULong? = null
            var routing_id: ByteArray? = null
            var parallel_id: String? = null
            var region_id: String? = null
            var child_session_id: String? = null
            var i = 0UL
            while (i < count) {
                i++
                when (Cbor.readUint(c) ?: return null) {
                    0UL -> {
                        // A key given twice is not one map entry.
                        if (id != null) return null
                        id = Cbor.readBytesExact(c, 16) ?: return null
                    }
                    1UL -> {
                        // A key given twice is not one map entry.
                        if (source != null) return null
                        source = Cbor.readText(c, 262144) ?: return null
                    }
                    2UL -> {
                        // A key given twice is not one map entry.
                        if (event_type != null) return null
                        event_type = Cbor.readText(c, 262144) ?: return null
                    }
                    3UL -> {
                        // A key given twice is not one map entry.
                        if (pattern != null) return null
                        pattern = PatternKind.fromUnderlying((Cbor.readUintUpto(c, UShort.MAX_VALUE.toULong()) ?: return null).toUShort()) ?: return null
                    }
                    4UL -> {
                        // A key given twice is not one map entry.
                        if (datacontenttype != null) return null
                        datacontenttype = PayloadCodec.fromUnderlying((Cbor.readUintUpto(c, UByte.MAX_VALUE.toULong()) ?: return null).toUByte()) ?: return null
                    }
                    5UL -> {
                        // A key given twice is not one map entry.
                        if (data != null) return null
                        data = Cbor.readBytes(c, 16777216) ?: return null
                    }
                    6UL -> {
                        // A key given twice is not one map entry.
                        if (subject != null) return null
                        subject = Cbor.readText(c, 262144) ?: return null
                    }
                    7UL -> {
                        // A key given twice is not one map entry.
                        if (correlation_id != null) return null
                        correlation_id = Cbor.readBytesExact(c, 16) ?: return null
                    }
                    8UL -> {
                        // A key given twice is not one map entry.
                        if (reply_to != null) return null
                        reply_to = Cbor.readText(c, 262144) ?: return null
                    }
                    9UL -> {
                        // A key given twice is not one map entry.
                        if (invoke_id != null) return null
                        invoke_id = Cbor.readBytesExact(c, 16) ?: return null
                    }
                    10UL -> {
                        // A key given twice is not one map entry.
                        if (rpc_status != null) return null
                        rpc_status = RpcStatus.fromUnderlying((Cbor.readUintUpto(c, UByte.MAX_VALUE.toULong()) ?: return null).toUByte()) ?: return null
                    }
                    11UL -> {
                        // A key given twice is not one map entry.
                        if (rpc_error_message != null) return null
                        rpc_error_message = Cbor.readText(c, 262144) ?: return null
                    }
                    12UL -> {
                        // A key given twice is not one map entry.
                        if (deadline_unix_ms != null) return null
                        deadline_unix_ms = Cbor.readUint(c) ?: return null
                    }
                    14UL -> {
                        // A key given twice is not one map entry.
                        if (sequence_no != null) return null
                        sequence_no = Cbor.readUint(c) ?: return null
                    }
                    15UL -> {
                        // A key given twice is not one map entry.
                        if (routing_id != null) return null
                        routing_id = Cbor.readBytesExact(c, 16) ?: return null
                    }
                    16UL -> {
                        // A key given twice is not one map entry.
                        if (parallel_id != null) return null
                        parallel_id = Cbor.readText(c, 262144) ?: return null
                    }
                    17UL -> {
                        // A key given twice is not one map entry.
                        if (region_id != null) return null
                        region_id = Cbor.readText(c, 262144) ?: return null
                    }
                    18UL -> {
                        // A key given twice is not one map entry.
                        if (child_session_id != null) return null
                        child_session_id = Cbor.readText(c, 262144) ?: return null
                    }
                    else -> if (!Cbor.skip(c)) return null
                }
            }
            return Envelope(
                id = id ?: return null,
                source = source ?: return null,
                event_type = event_type ?: return null,
                pattern = pattern ?: return null,
                datacontenttype = datacontenttype ?: return null,
                data = data ?: return null,
                subject = subject,
                correlation_id = correlation_id,
                reply_to = reply_to,
                invoke_id = invoke_id,
                rpc_status = rpc_status,
                rpc_error_message = rpc_error_message,
                deadline_unix_ms = deadline_unix_ms,
                sequence_no = sequence_no,
                routing_id = routing_id,
                parallel_id = parallel_id,
                region_id = region_id,
                child_session_id = child_session_id,
            )
        }
    }
}
