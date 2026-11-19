// SCE-GENERATED — DO NOT EDIT
// source-hash: fc87f60788825a2accbefbd7d310802e57bdcedce19d2a650856b2530bfaac44
// source: envelope.scxml
// SCE-MAP: envelope.scxml:22 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec" sce:encoding="cbor")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.envelope

import com.sce.forge.runtime.Cbor
import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.pattern_kind.*
import com.sce.generated.payload_codec.*
import com.sce.generated.rpc_status.*
import com.sce.generated.causal_entry.*
import kotlin.jvm.JvmInline

/// The `id` element of [Envelope], 16 to 16 bytes, held to that bound where a
/// value of it is made (docs/adr/0077 §3, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of bytes within the bound. The value it
/// holds cannot change after, so a value holds its bound for as long as it lives.
@JvmInline
value class EnvelopeIdBytes private constructor(val bytes: ByteArray) {
    companion object {
        /// The fewest bytes a value holds.
        const val LEAST: Int = 16

        /// The most bytes a value holds.
        const val MOST: Int = 16

        /// `bytes` as a value of the bound, or `null` where it lies outside it.
        fun of(bytes: ByteArray): EnvelopeIdBytes? =
            if (bytes.size in LEAST..MOST) EnvelopeIdBytes(bytes) else null

        /// The fewest bytes the bound holds, each zero: what a value made by
        /// default holds.
        fun fewest(): EnvelopeIdBytes = EnvelopeIdBytes(ByteArray(LEAST))
    }
}

/// The `source` element of [Envelope], 0 to 262144 UTF-8 bytes, held to that bound where a
/// value of it is made (docs/adr/0077 §3, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of a text within the bound. The value it
/// holds cannot change after, so a value holds its bound for as long as it lives.
@JvmInline
value class EnvelopeSourceText private constructor(val text: String) {
    companion object {
        /// The fewest UTF-8 bytes a value holds.
        const val LEAST: Int = 0

        /// The most UTF-8 bytes a value holds.
        const val MOST: Int = 262144

        /// `text` as a value of the bound, or `null` where it lies outside it.
        fun of(text: String): EnvelopeSourceText? {
            val size = com.sce.forge.runtime.Utf8.size(text) ?: return null
            return if (size <= MOST) EnvelopeSourceText(text) else null
        }

        /// The fewest UTF-8 bytes the bound holds, each zero: what a value made by
        /// default holds.
        fun fewest(): EnvelopeSourceText = EnvelopeSourceText("\u0000".repeat(LEAST))
    }
}

/// The `event_type` element of [Envelope], 0 to 262144 UTF-8 bytes, held to that bound where a
/// value of it is made (docs/adr/0077 §3, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of a text within the bound. The value it
/// holds cannot change after, so a value holds its bound for as long as it lives.
@JvmInline
value class EnvelopeEventTypeText private constructor(val text: String) {
    companion object {
        /// The fewest UTF-8 bytes a value holds.
        const val LEAST: Int = 0

        /// The most UTF-8 bytes a value holds.
        const val MOST: Int = 262144

        /// `text` as a value of the bound, or `null` where it lies outside it.
        fun of(text: String): EnvelopeEventTypeText? {
            val size = com.sce.forge.runtime.Utf8.size(text) ?: return null
            return if (size <= MOST) EnvelopeEventTypeText(text) else null
        }

        /// The fewest UTF-8 bytes the bound holds, each zero: what a value made by
        /// default holds.
        fun fewest(): EnvelopeEventTypeText = EnvelopeEventTypeText("\u0000".repeat(LEAST))
    }
}

/// The `data` element of [Envelope], 0 to 16777216 bytes, held to that bound where a
/// value of it is made (docs/adr/0077 §3, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of bytes within the bound. The value it
/// holds cannot change after, so a value holds its bound for as long as it lives.
@JvmInline
value class EnvelopeDataBytes private constructor(val bytes: ByteArray) {
    companion object {
        /// The fewest bytes a value holds.
        const val LEAST: Int = 0

        /// The most bytes a value holds.
        const val MOST: Int = 16777216

        /// `bytes` as a value of the bound, or `null` where it lies outside it.
        fun of(bytes: ByteArray): EnvelopeDataBytes? =
            if (bytes.size <= MOST) EnvelopeDataBytes(bytes) else null

        /// The fewest bytes the bound holds, each zero: what a value made by
        /// default holds.
        fun fewest(): EnvelopeDataBytes = EnvelopeDataBytes(ByteArray(LEAST))
    }
}

/// The `subject` element of [Envelope], 0 to 262144 UTF-8 bytes, held to that bound where a
/// value of it is made (docs/adr/0077 §3, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of a text within the bound. The value it
/// holds cannot change after, so a value holds its bound for as long as it lives.
@JvmInline
value class EnvelopeSubjectText private constructor(val text: String) {
    companion object {
        /// The fewest UTF-8 bytes a value holds.
        const val LEAST: Int = 0

        /// The most UTF-8 bytes a value holds.
        const val MOST: Int = 262144

        /// `text` as a value of the bound, or `null` where it lies outside it.
        fun of(text: String): EnvelopeSubjectText? {
            val size = com.sce.forge.runtime.Utf8.size(text) ?: return null
            return if (size <= MOST) EnvelopeSubjectText(text) else null
        }

        /// The fewest UTF-8 bytes the bound holds, each zero: what a value made by
        /// default holds.
        fun fewest(): EnvelopeSubjectText = EnvelopeSubjectText("\u0000".repeat(LEAST))
    }
}

/// The `correlation_id` element of [Envelope], 16 to 16 bytes, held to that bound where a
/// value of it is made (docs/adr/0077 §3, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of bytes within the bound. The value it
/// holds cannot change after, so a value holds its bound for as long as it lives.
@JvmInline
value class EnvelopeCorrelationIdBytes private constructor(val bytes: ByteArray) {
    companion object {
        /// The fewest bytes a value holds.
        const val LEAST: Int = 16

        /// The most bytes a value holds.
        const val MOST: Int = 16

        /// `bytes` as a value of the bound, or `null` where it lies outside it.
        fun of(bytes: ByteArray): EnvelopeCorrelationIdBytes? =
            if (bytes.size in LEAST..MOST) EnvelopeCorrelationIdBytes(bytes) else null

        /// The fewest bytes the bound holds, each zero: what a value made by
        /// default holds.
        fun fewest(): EnvelopeCorrelationIdBytes = EnvelopeCorrelationIdBytes(ByteArray(LEAST))
    }
}

/// The `reply_to` element of [Envelope], 0 to 262144 UTF-8 bytes, held to that bound where a
/// value of it is made (docs/adr/0077 §3, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of a text within the bound. The value it
/// holds cannot change after, so a value holds its bound for as long as it lives.
@JvmInline
value class EnvelopeReplyToText private constructor(val text: String) {
    companion object {
        /// The fewest UTF-8 bytes a value holds.
        const val LEAST: Int = 0

        /// The most UTF-8 bytes a value holds.
        const val MOST: Int = 262144

        /// `text` as a value of the bound, or `null` where it lies outside it.
        fun of(text: String): EnvelopeReplyToText? {
            val size = com.sce.forge.runtime.Utf8.size(text) ?: return null
            return if (size <= MOST) EnvelopeReplyToText(text) else null
        }

        /// The fewest UTF-8 bytes the bound holds, each zero: what a value made by
        /// default holds.
        fun fewest(): EnvelopeReplyToText = EnvelopeReplyToText("\u0000".repeat(LEAST))
    }
}

/// The `invoke_id` element of [Envelope], 16 to 16 bytes, held to that bound where a
/// value of it is made (docs/adr/0077 §3, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of bytes within the bound. The value it
/// holds cannot change after, so a value holds its bound for as long as it lives.
@JvmInline
value class EnvelopeInvokeIdBytes private constructor(val bytes: ByteArray) {
    companion object {
        /// The fewest bytes a value holds.
        const val LEAST: Int = 16

        /// The most bytes a value holds.
        const val MOST: Int = 16

        /// `bytes` as a value of the bound, or `null` where it lies outside it.
        fun of(bytes: ByteArray): EnvelopeInvokeIdBytes? =
            if (bytes.size in LEAST..MOST) EnvelopeInvokeIdBytes(bytes) else null

        /// The fewest bytes the bound holds, each zero: what a value made by
        /// default holds.
        fun fewest(): EnvelopeInvokeIdBytes = EnvelopeInvokeIdBytes(ByteArray(LEAST))
    }
}

/// The `rpc_error_message` element of [Envelope], 0 to 262144 UTF-8 bytes, held to that bound where a
/// value of it is made (docs/adr/0077 §3, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of a text within the bound. The value it
/// holds cannot change after, so a value holds its bound for as long as it lives.
@JvmInline
value class EnvelopeRpcErrorMessageText private constructor(val text: String) {
    companion object {
        /// The fewest UTF-8 bytes a value holds.
        const val LEAST: Int = 0

        /// The most UTF-8 bytes a value holds.
        const val MOST: Int = 262144

        /// `text` as a value of the bound, or `null` where it lies outside it.
        fun of(text: String): EnvelopeRpcErrorMessageText? {
            val size = com.sce.forge.runtime.Utf8.size(text) ?: return null
            return if (size <= MOST) EnvelopeRpcErrorMessageText(text) else null
        }

        /// The fewest UTF-8 bytes the bound holds, each zero: what a value made by
        /// default holds.
        fun fewest(): EnvelopeRpcErrorMessageText = EnvelopeRpcErrorMessageText("\u0000".repeat(LEAST))
    }
}

/// The `routing_id` element of [Envelope], 16 to 16 bytes, held to that bound where a
/// value of it is made (docs/adr/0077 §3, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of bytes within the bound. The value it
/// holds cannot change after, so a value holds its bound for as long as it lives.
@JvmInline
value class EnvelopeRoutingIdBytes private constructor(val bytes: ByteArray) {
    companion object {
        /// The fewest bytes a value holds.
        const val LEAST: Int = 16

        /// The most bytes a value holds.
        const val MOST: Int = 16

        /// `bytes` as a value of the bound, or `null` where it lies outside it.
        fun of(bytes: ByteArray): EnvelopeRoutingIdBytes? =
            if (bytes.size in LEAST..MOST) EnvelopeRoutingIdBytes(bytes) else null

        /// The fewest bytes the bound holds, each zero: what a value made by
        /// default holds.
        fun fewest(): EnvelopeRoutingIdBytes = EnvelopeRoutingIdBytes(ByteArray(LEAST))
    }
}

/// The `parallel_id` element of [Envelope], 0 to 262144 UTF-8 bytes, held to that bound where a
/// value of it is made (docs/adr/0077 §3, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of a text within the bound. The value it
/// holds cannot change after, so a value holds its bound for as long as it lives.
@JvmInline
value class EnvelopeParallelIdText private constructor(val text: String) {
    companion object {
        /// The fewest UTF-8 bytes a value holds.
        const val LEAST: Int = 0

        /// The most UTF-8 bytes a value holds.
        const val MOST: Int = 262144

        /// `text` as a value of the bound, or `null` where it lies outside it.
        fun of(text: String): EnvelopeParallelIdText? {
            val size = com.sce.forge.runtime.Utf8.size(text) ?: return null
            return if (size <= MOST) EnvelopeParallelIdText(text) else null
        }

        /// The fewest UTF-8 bytes the bound holds, each zero: what a value made by
        /// default holds.
        fun fewest(): EnvelopeParallelIdText = EnvelopeParallelIdText("\u0000".repeat(LEAST))
    }
}

/// The `region_id` element of [Envelope], 0 to 262144 UTF-8 bytes, held to that bound where a
/// value of it is made (docs/adr/0077 §3, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of a text within the bound. The value it
/// holds cannot change after, so a value holds its bound for as long as it lives.
@JvmInline
value class EnvelopeRegionIdText private constructor(val text: String) {
    companion object {
        /// The fewest UTF-8 bytes a value holds.
        const val LEAST: Int = 0

        /// The most UTF-8 bytes a value holds.
        const val MOST: Int = 262144

        /// `text` as a value of the bound, or `null` where it lies outside it.
        fun of(text: String): EnvelopeRegionIdText? {
            val size = com.sce.forge.runtime.Utf8.size(text) ?: return null
            return if (size <= MOST) EnvelopeRegionIdText(text) else null
        }

        /// The fewest UTF-8 bytes the bound holds, each zero: what a value made by
        /// default holds.
        fun fewest(): EnvelopeRegionIdText = EnvelopeRegionIdText("\u0000".repeat(LEAST))
    }
}

/// The `child_session_id` element of [Envelope], 0 to 262144 UTF-8 bytes, held to that bound where a
/// value of it is made (docs/adr/0077 §3, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of a text within the bound. The value it
/// holds cannot change after, so a value holds its bound for as long as it lives.
@JvmInline
value class EnvelopeChildSessionIdText private constructor(val text: String) {
    companion object {
        /// The fewest UTF-8 bytes a value holds.
        const val LEAST: Int = 0

        /// The most UTF-8 bytes a value holds.
        const val MOST: Int = 262144

        /// `text` as a value of the bound, or `null` where it lies outside it.
        fun of(text: String): EnvelopeChildSessionIdText? {
            val size = com.sce.forge.runtime.Utf8.size(text) ?: return null
            return if (size <= MOST) EnvelopeChildSessionIdText(text) else null
        }

        /// The fewest UTF-8 bytes the bound holds, each zero: what a value made by
        /// default holds.
        fun fewest(): EnvelopeChildSessionIdText = EnvelopeChildSessionIdText("\u0000".repeat(LEAST))
    }
}

/// The `causal_vector` element of [Envelope], 0 to 64 entries, held to that bound where a
/// value of it is made (docs/adr/0065 §6 R2, docs/adr/0081 §2): its constructor is
/// private and [of] makes a value only of entries within the bound, which it
/// copies, so no list it was made of can grow it past its bound after.
@JvmInline
value class EnvelopeCausalVectorList private constructor(val entries: List<CausalEntryPayload>) : com.sce.forge.runtime.Counted {
    /// How many entries the list holds, the count a frame writes of it.
    override val size: Int
        get() = entries.size

    companion object {
        /// The most entries a value holds.
        const val MOST: Int = 64

        /// `entries` as a value of the bound, or `null` where they lie outside it.
        fun of(entries: List<CausalEntryPayload>): EnvelopeCausalVectorList? =
            if (entries.size <= MOST) EnvelopeCausalVectorList(entries.toList()) else null

        /// No entry: what a value made by default holds.
        fun fewest(): EnvelopeCausalVectorList = EnvelopeCausalVectorList(emptyList())
    }
}

/// The entries a value of [Envelope] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [Envelope.encodeSource] writes.
interface EnvelopeSource {
    val id: EnvelopeIdBytes
    val source: EnvelopeSourceText
    val event_type: EnvelopeEventTypeText
    val pattern: PatternKind
    val datacontenttype: PayloadCodec
    val data: EnvelopeDataBytes
    val subject: EnvelopeSubjectText?
    val correlation_id: EnvelopeCorrelationIdBytes?
    val reply_to: EnvelopeReplyToText?
    val invoke_id: EnvelopeInvokeIdBytes?
    val rpc_status: RpcStatus?
    val rpc_error_message: EnvelopeRpcErrorMessageText?
    val deadline_unix_ms: ULong?
    val sequence_no: ULong?
    val routing_id: EnvelopeRoutingIdBytes?
    val parallel_id: EnvelopeParallelIdText?
    val region_id: EnvelopeRegionIdText?
    val child_session_id: EnvelopeChildSessionIdText?
    val causal_epoch: ULong?
    val causal_vector: EnvelopeCausalVectorList?
}

/**
 * One CBOR map (SCE_FORGE.md §4.6.1). Decode takes the keys in any order and
 * skips one this codec does not declare; encode writes the entries present in
 * ascending key order, every head in its shortest form. A required entry
 * starts at its type's default and an optional one absent (`null`), so
 * `Envelope()` is the infallible constructor every codec offers.
 */
data class Envelope(
    /** Map key 0, required. */
    override var id: EnvelopeIdBytes = EnvelopeIdBytes.fewest(),
    /** Map key 1, required. */
    override var source: EnvelopeSourceText = EnvelopeSourceText.fewest(),
    /** Map key 2, required. */
    override var event_type: EnvelopeEventTypeText = EnvelopeEventTypeText.fewest(),
    /** Map key 3, required. */
    override var pattern: PatternKind = PatternKind.FIRE_FORGET,
    /** Map key 4, required. */
    override var datacontenttype: PayloadCodec = PayloadCodec.NONE,
    /** Map key 5, required. */
    override var data: EnvelopeDataBytes = EnvelopeDataBytes.fewest(),
    /** Map key 6. */
    override var subject: EnvelopeSubjectText? = null,
    /** Map key 7. */
    override var correlation_id: EnvelopeCorrelationIdBytes? = null,
    /** Map key 8. */
    override var reply_to: EnvelopeReplyToText? = null,
    /** Map key 9. */
    override var invoke_id: EnvelopeInvokeIdBytes? = null,
    /** Map key 10. */
    override var rpc_status: RpcStatus? = null,
    /** Map key 11. */
    override var rpc_error_message: EnvelopeRpcErrorMessageText? = null,
    /** Map key 12. */
    override var deadline_unix_ms: ULong? = null,
    /** Map key 14. */
    override var sequence_no: ULong? = null,
    /** Map key 15. */
    override var routing_id: EnvelopeRoutingIdBytes? = null,
    /** Map key 16. */
    override var parallel_id: EnvelopeParallelIdText? = null,
    /** Map key 17. */
    override var region_id: EnvelopeRegionIdText? = null,
    /** Map key 18. */
    override var child_session_id: EnvelopeChildSessionIdText? = null,
    /** Map key 19. */
    override var causal_epoch: ULong? = null,
    /** Map key 20. */
    override var causal_vector: EnvelopeCausalVectorList? = null,
) : EnvelopeSource {
    /**
     * Encode this map into [w]; `null` on success. The encode over any source
     * ([encodeSource]) refuses, before the first byte, what no class of an
     * entry holds that the map cannot carry.
     */
    fun encode(w: SceSink): CodecError? = encodeSource(this, w)

    /** [encode] into a new array; `null` when encode refuses the value. */
    fun encodeToByteArray(): ByteArray? {
        val out = mutableListOf<Byte>()
        if (encode(MutableListSink(out)) != null) return null
        return out.toByteArray()
    }

    companion object {
        /// Write the map [src] presents into [w]: the entries present in ascending key
        /// order, every head in its shortest form (RFC 8949 §4.2.1). `null` on
        /// success; a value no class of an entry holds that the map cannot carry is
        /// refused with [CodecError.CborOutOfRange] before the first byte.
        fun encodeSource(src: EnvelopeSource, w: SceSink): CodecError? {
            val _e_id = src.id
            val _e_source = src.source
            val _e_event_type = src.event_type
            val _e_pattern = src.pattern
            val _e_datacontenttype = src.datacontenttype
            val _e_data = src.data
            val _e_subject = src.subject
            val _e_correlation_id = src.correlation_id
            val _e_reply_to = src.reply_to
            val _e_invoke_id = src.invoke_id
            val _e_rpc_status = src.rpc_status
            val _e_rpc_error_message = src.rpc_error_message
            val _e_deadline_unix_ms = src.deadline_unix_ms
            val _e_sequence_no = src.sequence_no
            val _e_routing_id = src.routing_id
            val _e_parallel_id = src.parallel_id
            val _e_region_id = src.region_id
            val _e_child_session_id = src.child_session_id
            val _e_causal_epoch = src.causal_epoch
            val _e_causal_vector = src.causal_vector
            var count = 6UL
            if (_e_subject != null) count++
            if (_e_correlation_id != null) count++
            if (_e_reply_to != null) count++
            if (_e_invoke_id != null) count++
            if (_e_rpc_status != null) count++
            if (_e_rpc_error_message != null) count++
            if (_e_deadline_unix_ms != null) count++
            if (_e_sequence_no != null) count++
            if (_e_routing_id != null) count++
            if (_e_parallel_id != null) count++
            if (_e_region_id != null) count++
            if (_e_child_session_id != null) count++
            if (_e_causal_epoch != null) count++
            if (_e_causal_vector != null) count++
            Cbor.writeMapHead(w, count)?.let { return it }
            Cbor.writeUint(w, 0UL)?.let { return it }
            Cbor.writeBytes(w, _e_id.bytes)?.let { return it }
            Cbor.writeUint(w, 1UL)?.let { return it }
            Cbor.writeText(w, _e_source.text)?.let { return it }
            Cbor.writeUint(w, 2UL)?.let { return it }
            Cbor.writeText(w, _e_event_type.text)?.let { return it }
            Cbor.writeUint(w, 3UL)?.let { return it }
            Cbor.writeUint(w, _e_pattern.toUnderlying().toULong())?.let { return it }
            Cbor.writeUint(w, 4UL)?.let { return it }
            Cbor.writeUint(w, _e_datacontenttype.toUnderlying().toULong())?.let { return it }
            Cbor.writeUint(w, 5UL)?.let { return it }
            Cbor.writeBytes(w, _e_data.bytes)?.let { return it }
            if (_e_subject != null) {
                Cbor.writeUint(w, 6UL)?.let { return it }
                Cbor.writeText(w, _e_subject.text)?.let { return it }
            }
            if (_e_correlation_id != null) {
                Cbor.writeUint(w, 7UL)?.let { return it }
                Cbor.writeBytes(w, _e_correlation_id.bytes)?.let { return it }
            }
            if (_e_reply_to != null) {
                Cbor.writeUint(w, 8UL)?.let { return it }
                Cbor.writeText(w, _e_reply_to.text)?.let { return it }
            }
            if (_e_invoke_id != null) {
                Cbor.writeUint(w, 9UL)?.let { return it }
                Cbor.writeBytes(w, _e_invoke_id.bytes)?.let { return it }
            }
            if (_e_rpc_status != null) {
                Cbor.writeUint(w, 10UL)?.let { return it }
                Cbor.writeUint(w, _e_rpc_status.toUnderlying().toULong())?.let { return it }
            }
            if (_e_rpc_error_message != null) {
                Cbor.writeUint(w, 11UL)?.let { return it }
                Cbor.writeText(w, _e_rpc_error_message.text)?.let { return it }
            }
            if (_e_deadline_unix_ms != null) {
                Cbor.writeUint(w, 12UL)?.let { return it }
                Cbor.writeUint(w, _e_deadline_unix_ms)?.let { return it }
            }
            if (_e_sequence_no != null) {
                Cbor.writeUint(w, 14UL)?.let { return it }
                Cbor.writeUint(w, _e_sequence_no)?.let { return it }
            }
            if (_e_routing_id != null) {
                Cbor.writeUint(w, 15UL)?.let { return it }
                Cbor.writeBytes(w, _e_routing_id.bytes)?.let { return it }
            }
            if (_e_parallel_id != null) {
                Cbor.writeUint(w, 16UL)?.let { return it }
                Cbor.writeText(w, _e_parallel_id.text)?.let { return it }
            }
            if (_e_region_id != null) {
                Cbor.writeUint(w, 17UL)?.let { return it }
                Cbor.writeText(w, _e_region_id.text)?.let { return it }
            }
            if (_e_child_session_id != null) {
                Cbor.writeUint(w, 18UL)?.let { return it }
                Cbor.writeText(w, _e_child_session_id.text)?.let { return it }
            }
            if (_e_causal_epoch != null) {
                Cbor.writeUint(w, 19UL)?.let { return it }
                Cbor.writeUint(w, _e_causal_epoch)?.let { return it }
            }
            if (_e_causal_vector != null) {
                Cbor.writeUint(w, 20UL)?.let { return it }
                Cbor.writeArrayHead(w, _e_causal_vector.size.toULong())?.let { return it }
                for (_record in _e_causal_vector.entries) {
                    encodeRecordCausalEntry(w, _record)?.let { return it }
                }
            }
            return null
        }

        /// Write one record of `causal_entry` (`record:CausalEntry`) into [w] as one map of
        /// every field, the keys in the order RFC 8949 §4.2.1 sorts their encodings: a
        /// shorter key first. The encode held the record to its bounds before the
        /// first byte.
        private fun encodeRecordCausalEntry(w: SceSink, record: CausalEntryPayload): CodecError? {
            Cbor.writeMapHead(w, 3UL)?.let { return it }
            Cbor.writeText(w, "hi")?.let { return it }
            Cbor.writeUint(w, record.hi)?.let { return it }
            Cbor.writeText(w, "lo")?.let { return it }
            Cbor.writeUint(w, record.lo)?.let { return it }
            Cbor.writeText(w, "seq")?.let { return it }
            Cbor.writeUint(w, record.seq)?.let { return it }
            return null
        }

        /**
         * Decode one map from [cursor]. On success the cursor advances past
         * it; a refusal answers the [CodecError] that refused the map
         * (docs/adr/0099, amendment of 2026-11-16), with the cursor left where
         * it was.
         */
        fun decode(cursor: SceCursor): Decoded<Envelope> {
            val mark = cursor.mark()
            val value = decodeAt(cursor)
            if (value is Decoded.Refused) cursor.reset(mark)
            return value
        }

        private fun decodeAt(c: SceCursor): Decoded<Envelope> {
            val count = Cbor.readMapLen(c).valueOr { _refused -> return _refused }
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
            var causal_epoch: ULong? = null
            var causal_vector: List<CausalEntryPayload>? = null
            var i = 0UL
            while (i < count) {
                i++
                when (Cbor.readUint(c).valueOr { _refused -> return _refused }) {
                    0UL -> {
                        // A key given twice is not one map entry.
                        if (id != null) return Decoded.Refused(CodecError.CborDuplicateKey)
                        id = Cbor.readBytesExact(c, 16).valueOr { _refused -> return _refused }
                    }
                    1UL -> {
                        // A key given twice is not one map entry.
                        if (source != null) return Decoded.Refused(CodecError.CborDuplicateKey)
                        source = Cbor.readText(c, 262144).valueOr { _refused -> return _refused }
                    }
                    2UL -> {
                        // A key given twice is not one map entry.
                        if (event_type != null) return Decoded.Refused(CodecError.CborDuplicateKey)
                        event_type = Cbor.readText(c, 262144).valueOr { _refused -> return _refused }
                    }
                    3UL -> {
                        // A key given twice is not one map entry.
                        if (pattern != null) return Decoded.Refused(CodecError.CborDuplicateKey)
                        pattern = PatternKind.fromUnderlying((Cbor.readUintUpto(c, UShort.MAX_VALUE.toULong()).valueOr { _refused -> return _refused }).toUShort()) ?: return Decoded.Refused(CodecError.UndeclaredEnumValue)
                    }
                    4UL -> {
                        // A key given twice is not one map entry.
                        if (datacontenttype != null) return Decoded.Refused(CodecError.CborDuplicateKey)
                        datacontenttype = PayloadCodec.fromUnderlying((Cbor.readUintUpto(c, UByte.MAX_VALUE.toULong()).valueOr { _refused -> return _refused }).toUByte()) ?: return Decoded.Refused(CodecError.UndeclaredEnumValue)
                    }
                    5UL -> {
                        // A key given twice is not one map entry.
                        if (data != null) return Decoded.Refused(CodecError.CborDuplicateKey)
                        data = Cbor.readBytes(c, 16777216).valueOr { _refused -> return _refused }
                    }
                    6UL -> {
                        // A key given twice is not one map entry.
                        if (subject != null) return Decoded.Refused(CodecError.CborDuplicateKey)
                        subject = Cbor.readText(c, 262144).valueOr { _refused -> return _refused }
                    }
                    7UL -> {
                        // A key given twice is not one map entry.
                        if (correlation_id != null) return Decoded.Refused(CodecError.CborDuplicateKey)
                        correlation_id = Cbor.readBytesExact(c, 16).valueOr { _refused -> return _refused }
                    }
                    8UL -> {
                        // A key given twice is not one map entry.
                        if (reply_to != null) return Decoded.Refused(CodecError.CborDuplicateKey)
                        reply_to = Cbor.readText(c, 262144).valueOr { _refused -> return _refused }
                    }
                    9UL -> {
                        // A key given twice is not one map entry.
                        if (invoke_id != null) return Decoded.Refused(CodecError.CborDuplicateKey)
                        invoke_id = Cbor.readBytesExact(c, 16).valueOr { _refused -> return _refused }
                    }
                    10UL -> {
                        // A key given twice is not one map entry.
                        if (rpc_status != null) return Decoded.Refused(CodecError.CborDuplicateKey)
                        rpc_status = RpcStatus.fromUnderlying((Cbor.readUintUpto(c, UByte.MAX_VALUE.toULong()).valueOr { _refused -> return _refused }).toUByte()) ?: return Decoded.Refused(CodecError.UndeclaredEnumValue)
                    }
                    11UL -> {
                        // A key given twice is not one map entry.
                        if (rpc_error_message != null) return Decoded.Refused(CodecError.CborDuplicateKey)
                        rpc_error_message = Cbor.readText(c, 262144).valueOr { _refused -> return _refused }
                    }
                    12UL -> {
                        // A key given twice is not one map entry.
                        if (deadline_unix_ms != null) return Decoded.Refused(CodecError.CborDuplicateKey)
                        deadline_unix_ms = Cbor.readUint(c).valueOr { _refused -> return _refused }
                    }
                    14UL -> {
                        // A key given twice is not one map entry.
                        if (sequence_no != null) return Decoded.Refused(CodecError.CborDuplicateKey)
                        sequence_no = Cbor.readUint(c).valueOr { _refused -> return _refused }
                    }
                    15UL -> {
                        // A key given twice is not one map entry.
                        if (routing_id != null) return Decoded.Refused(CodecError.CborDuplicateKey)
                        routing_id = Cbor.readBytesExact(c, 16).valueOr { _refused -> return _refused }
                    }
                    16UL -> {
                        // A key given twice is not one map entry.
                        if (parallel_id != null) return Decoded.Refused(CodecError.CborDuplicateKey)
                        parallel_id = Cbor.readText(c, 262144).valueOr { _refused -> return _refused }
                    }
                    17UL -> {
                        // A key given twice is not one map entry.
                        if (region_id != null) return Decoded.Refused(CodecError.CborDuplicateKey)
                        region_id = Cbor.readText(c, 262144).valueOr { _refused -> return _refused }
                    }
                    18UL -> {
                        // A key given twice is not one map entry.
                        if (child_session_id != null) return Decoded.Refused(CodecError.CborDuplicateKey)
                        child_session_id = Cbor.readText(c, 262144).valueOr { _refused -> return _refused }
                    }
                    19UL -> {
                        // A key given twice is not one map entry.
                        if (causal_epoch != null) return Decoded.Refused(CodecError.CborDuplicateKey)
                        causal_epoch = Cbor.readUint(c).valueOr { _refused -> return _refused }
                    }
                    20UL -> {
                        // A key given twice is not one map entry.
                        if (causal_vector != null) return Decoded.Refused(CodecError.CborDuplicateKey)
                        causal_vector = run {
                            // A definite-length array within the list's counts,
                            // each item one record's map.
                            val len = Cbor.readArrayLen(c).valueOr { _refused -> return _refused }
                            if (len > 64UL) return Decoded.Refused(CodecError.CborOutOfRange)
                            val list = ArrayList<CausalEntryPayload>(len.toInt())
                            var n = 0UL
                            while (n < len) {
                                n++
                                list.add(decodeRecordCausalEntry(c).valueOr { _refused -> return _refused })
                            }
                            list
                        }
                    }
                    else -> Cbor.skip(c).valueOr { _refused -> return _refused }
                }
            }
            return Decoded.Ok(
                Envelope(
                    id = EnvelopeIdBytes.of(id ?: return Decoded.Refused(CodecError.CborRequiredKeyMissing)) ?: return Decoded.Refused(CodecError.CborWrongLength),
                    source = EnvelopeSourceText.of(source ?: return Decoded.Refused(CodecError.CborRequiredKeyMissing)) ?: return Decoded.Refused(CodecError.CborOutOfRange),
                    event_type = EnvelopeEventTypeText.of(event_type ?: return Decoded.Refused(CodecError.CborRequiredKeyMissing)) ?: return Decoded.Refused(CodecError.CborOutOfRange),
                    pattern = pattern ?: return Decoded.Refused(CodecError.CborRequiredKeyMissing),
                    datacontenttype = datacontenttype ?: return Decoded.Refused(CodecError.CborRequiredKeyMissing),
                    data = EnvelopeDataBytes.of(data ?: return Decoded.Refused(CodecError.CborRequiredKeyMissing)) ?: return Decoded.Refused(CodecError.CborOutOfRange),
                    subject = subject?.let { EnvelopeSubjectText.of(it) ?: return Decoded.Refused(CodecError.CborOutOfRange) },
                    correlation_id = correlation_id?.let { EnvelopeCorrelationIdBytes.of(it) ?: return Decoded.Refused(CodecError.CborWrongLength) },
                    reply_to = reply_to?.let { EnvelopeReplyToText.of(it) ?: return Decoded.Refused(CodecError.CborOutOfRange) },
                    invoke_id = invoke_id?.let { EnvelopeInvokeIdBytes.of(it) ?: return Decoded.Refused(CodecError.CborWrongLength) },
                    rpc_status = rpc_status,
                    rpc_error_message = rpc_error_message?.let { EnvelopeRpcErrorMessageText.of(it) ?: return Decoded.Refused(CodecError.CborOutOfRange) },
                    deadline_unix_ms = deadline_unix_ms,
                    sequence_no = sequence_no,
                    routing_id = routing_id?.let { EnvelopeRoutingIdBytes.of(it) ?: return Decoded.Refused(CodecError.CborWrongLength) },
                    parallel_id = parallel_id?.let { EnvelopeParallelIdText.of(it) ?: return Decoded.Refused(CodecError.CborOutOfRange) },
                    region_id = region_id?.let { EnvelopeRegionIdText.of(it) ?: return Decoded.Refused(CodecError.CborOutOfRange) },
                    child_session_id = child_session_id?.let { EnvelopeChildSessionIdText.of(it) ?: return Decoded.Refused(CodecError.CborOutOfRange) },
                    causal_epoch = causal_epoch,
                    causal_vector = causal_vector?.let { EnvelopeCausalVectorList.of(it) ?: return Decoded.Refused(CodecError.CborOutOfRange) },
                ),
            )
        }

        /**
         * Read one record of the event schema `causal_entry` (`record:CausalEntry`)
         * from [c], as a list of this map holds it (docs/adr/0085, Landing order
         * step 3a), into the payload type the schema's own unit declares: one
         * CBOR map of every field, each under its id as text (docs/adr/0086,
         * section 9.2). Its keys in any order, a key the schema does not declare
         * skipped whole; a key given twice, a field missing and a text or byte
         * string past its bound are refused, each by its kind.
         */
        private fun decodeRecordCausalEntry(c: SceCursor): Decoded<CausalEntryPayload> {
            val count = Cbor.readMapLen(c).valueOr { _refused -> return _refused }
            var held_hi: ULong? = null
            var held_lo: ULong? = null
            var held_seq: ULong? = null
            var i = 0UL
            while (i < count) {
                i++
                when (Cbor.readText(c, null).valueOr { _refused -> return _refused }) {
                    "hi" -> {
                        // A key given twice is not one map entry.
                        if (held_hi != null) return Decoded.Refused(CodecError.CborDuplicateKey)
                        held_hi = Cbor.readUint(c).valueOr { _refused -> return _refused }
                    }
                    "lo" -> {
                        // A key given twice is not one map entry.
                        if (held_lo != null) return Decoded.Refused(CodecError.CborDuplicateKey)
                        held_lo = Cbor.readUint(c).valueOr { _refused -> return _refused }
                    }
                    "seq" -> {
                        // A key given twice is not one map entry.
                        if (held_seq != null) return Decoded.Refused(CodecError.CborDuplicateKey)
                        held_seq = Cbor.readUint(c).valueOr { _refused -> return _refused }
                    }
                    else -> Cbor.skip(c).valueOr { _refused -> return _refused }
                }
            }
            return Decoded.Ok(
                CausalEntryPayload(
                    hi = held_hi ?: return Decoded.Refused(CodecError.CborRequiredKeyMissing),
                    lo = held_lo ?: return Decoded.Refused(CodecError.CborRequiredKeyMissing),
                    seq = held_seq ?: return Decoded.Refused(CodecError.CborRequiredKeyMissing),
                ),
            )
        }
    }
}
