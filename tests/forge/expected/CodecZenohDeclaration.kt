// SCE-MAP: codec_zenoh_declaration:54

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_declaration

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.codec_zenoh_decl_kexpr.*
import com.sce.generated.codec_zenoh_undecl_kexpr.*
import com.sce.generated.codec_zenoh_decl_subscriber.*
import com.sce.generated.codec_zenoh_undecl_subscriber.*
import com.sce.generated.codec_zenoh_decl_queryable.*
import com.sce.generated.codec_zenoh_undecl_queryable.*
import com.sce.generated.codec_zenoh_decl_token.*
import com.sce.generated.codec_zenoh_undecl_token.*
import com.sce.generated.codec_zenoh_decl_final.*

// RFC §5.B variant primitive (B1-β): discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body. Arm body types are referenced by FQN
// (defensive — wildcard imports could otherwise surface an ambiguity if
// two imported codecs declare same-named inner classes).
sealed class CodecZenohDeclarationVariant {
    data class CodecZenohDeclKexpr(val body: com.sce.generated.codec_zenoh_decl_kexpr.CodecZenohDeclKexpr) : CodecZenohDeclarationVariant()
    data class CodecZenohUndeclKexpr(val body: com.sce.generated.codec_zenoh_undecl_kexpr.CodecZenohUndeclKexpr) : CodecZenohDeclarationVariant()
    data class CodecZenohDeclSubscriber(val body: com.sce.generated.codec_zenoh_decl_subscriber.CodecZenohDeclSubscriber) : CodecZenohDeclarationVariant()
    data class CodecZenohUndeclSubscriber(val body: com.sce.generated.codec_zenoh_undecl_subscriber.CodecZenohUndeclSubscriber) : CodecZenohDeclarationVariant()
    data class CodecZenohDeclQueryable(val body: com.sce.generated.codec_zenoh_decl_queryable.CodecZenohDeclQueryable) : CodecZenohDeclarationVariant()
    data class CodecZenohUndeclQueryable(val body: com.sce.generated.codec_zenoh_undecl_queryable.CodecZenohUndeclQueryable) : CodecZenohDeclarationVariant()
    data class CodecZenohDeclToken(val body: com.sce.generated.codec_zenoh_decl_token.CodecZenohDeclToken) : CodecZenohDeclarationVariant()
    data class CodecZenohUndeclToken(val body: com.sce.generated.codec_zenoh_undecl_token.CodecZenohUndeclToken) : CodecZenohDeclarationVariant()
    data class CodecZenohDeclFinal(val body: com.sce.generated.codec_zenoh_decl_final.CodecZenohDeclFinal) : CodecZenohDeclarationVariant()
    data class Default(val tag: UByte, val body: com.sce.generated.codec_zenoh_decl_final.CodecZenohDeclFinal) : CodecZenohDeclarationVariant()
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohDeclaration()` before any encode()/decode() call. Defaults
// mirror the zero-initialized shape that decode() fills in on success.
data class CodecZenohDeclaration(
    var header: UByte = 0.toUByte(),
    // RFC variant-default-uniformity Atomic β-kotlin: pick the declared
    // default arm (`<sce:arm default="true"/>`) instead of the first
    // alternative so a freshly-constructed envelope round-trips byte-
    // exactly through `encode() -> decode()`. Paired with the inner
    // codec's `<sce:flag value=>`-baked default fields above.
    var body: CodecZenohDeclarationVariant = CodecZenohDeclarationVariant.CodecZenohDeclFinal(com.sce.generated.codec_zenoh_decl_final.CodecZenohDeclFinal())
) {
    // RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
    // the carrier field. Single-bit (width=1) reads as Boolean; multi-
    // bit (width>=2) reads as the smallest unsigned Kotlin type that
    // fits (UByte / UShort / UInt / ULong). UByte/UShort widen through
    // `.toInt()` and UInt/ULong through `.toLong()` for the bitwise
    // ops; the result narrows back via the carrier's `toU*` ctor.
    fun mid(): UByte {
        val _carrier = this.header.toInt()
        return ((_carrier shr 0) and 0x1F).toUByte()
    }

    fun setMid(v: UByte) {
        val _carrier = this.header.toInt()
        val _shifted_mask = 0x1F shl 0
        val _val = (v.toInt() and 0x1F) shl 0
        this.header = ((_carrier and _shifted_mask.inv()) or _val).toUByte()
    }

    fun N(): Boolean = (this.header.toInt() and 0x20) != 0

    fun setN(v: Boolean) {
        this.header = if (v) {
            (this.header.toInt() or 0x20).toUByte()
        } else {
            (this.header.toInt() and 0x20.inv()).toUByte()
        }
    }

    fun M(): Boolean = (this.header.toInt() and 0x40) != 0

    fun setM(v: Boolean) {
        this.header = if (v) {
            (this.header.toInt() or 0x40).toUByte()
        } else {
            (this.header.toInt() and 0x40.inv()).toUByte()
        }
    }

    fun Z(): Boolean = (this.header.toInt() and 0x80) != 0

    fun setZ(v: Boolean) {
        this.header = if (v) {
            (this.header.toInt() or 0x80).toUByte()
        } else {
            (this.header.toInt() and 0x80.inv()).toUByte()
        }
    }

    /// RFC §5.B B1-α encode-side primary: write `self` into the
    /// caller-owned `w` sink. Returns `null` on success;
    /// `CodecError.BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `MutableListSink`) are effectively infallible.
    fun encode(w: SceSink): CodecError? {
        // Encode fixed prefix (tag field bytes are part of the prefix).
        // The tag value is read from the struct field, NOT derived from
        // the body discriminant — keeping author-set tag / body in sync
        // is the caller's responsibility (v1 keeps the layout simple).
        w.writeU8(header.toByte())?.let { return it }
        // Append the active arm body's encoded bytes via the same sink.
        when (val _b = this.body) {
            is CodecZenohDeclarationVariant.CodecZenohDeclKexpr -> _b.body.encode(w, (((this.header.toInt() shr 5) and 0x1).toUByte()))?.let { return it }
            is CodecZenohDeclarationVariant.CodecZenohUndeclKexpr -> _b.body.encode(w)?.let { return it }
            is CodecZenohDeclarationVariant.CodecZenohDeclSubscriber -> _b.body.encode(w, (((this.header.toInt() shr 5) and 0x1).toUByte()))?.let { return it }
            is CodecZenohDeclarationVariant.CodecZenohUndeclSubscriber -> _b.body.encode(w, (((this.header.toInt() shr 7) and 0x1).toUByte()))?.let { return it }
            is CodecZenohDeclarationVariant.CodecZenohDeclQueryable -> _b.body.encode(w, (((this.header.toInt() shr 5) and 0x1).toUByte()), (((this.header.toInt() shr 7) and 0x1).toUByte()))?.let { return it }
            is CodecZenohDeclarationVariant.CodecZenohUndeclQueryable -> _b.body.encode(w, (((this.header.toInt() shr 7) and 0x1).toUByte()))?.let { return it }
            is CodecZenohDeclarationVariant.CodecZenohDeclToken -> _b.body.encode(w, (((this.header.toInt() shr 5) and 0x1).toUByte()))?.let { return it }
            is CodecZenohDeclarationVariant.CodecZenohUndeclToken -> _b.body.encode(w, (((this.header.toInt() shr 7) and 0x1).toUByte()))?.let { return it }
            is CodecZenohDeclarationVariant.CodecZenohDeclFinal -> _b.body.encode(w)?.let { return it }
            is CodecZenohDeclarationVariant.Default -> _b.body.encode(w)?.let { return it }
        }
        return null
    }

    /// Heap-backed convenience facade. Runs `encode` over a
    /// `MutableListSink` and returns the freshly-encoded ByteArray.
    /// Callers targeting zero-alloc hot paths should call `encode`
    /// directly against a caller-owned sink (e.g. `ByteArraySink`).
    fun encodeToByteArray(): ByteArray {
        val _list = mutableListOf<Byte>()
        encode(MutableListSink(_list))
        return _list.toByteArray()
    }

    companion object {
        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes; returns `null` when the
        /// cursor's tail is shorter than the declared minimum frame
        /// (RFC §5.B L494-519).
        fun decode(cursor: SceCursor): CodecZenohDeclaration? {
            // Decode fixed prefix (RFC §5.B variant B1-β: fields before tag suffix).
            val raw = cursor.peekSlice(1) ?: return null
            val header = raw[0].toUByte()
            if (!cursor.advance(1)) return null
            // Dispatch on the tag field; each arm decodes its body codec
            // from the cursor. The default arm (when declared) carries
            // the runtime tag value so encode can round-trip it back
            // onto the wire.
            val body: CodecZenohDeclarationVariant = when (((header.toInt() shr 0) and 0x1F)) {
                0 -> {
                    val _arm = com.sce.generated.codec_zenoh_decl_kexpr.CodecZenohDeclKexpr.decode(cursor, (((header.toInt() shr 5) and 0x1).toUByte())) ?: return null
                    CodecZenohDeclarationVariant.CodecZenohDeclKexpr(_arm)
                }
                1 -> {
                    val _arm = com.sce.generated.codec_zenoh_undecl_kexpr.CodecZenohUndeclKexpr.decode(cursor) ?: return null
                    CodecZenohDeclarationVariant.CodecZenohUndeclKexpr(_arm)
                }
                2 -> {
                    val _arm = com.sce.generated.codec_zenoh_decl_subscriber.CodecZenohDeclSubscriber.decode(cursor, (((header.toInt() shr 5) and 0x1).toUByte())) ?: return null
                    CodecZenohDeclarationVariant.CodecZenohDeclSubscriber(_arm)
                }
                3 -> {
                    val _arm = com.sce.generated.codec_zenoh_undecl_subscriber.CodecZenohUndeclSubscriber.decode(cursor, (((header.toInt() shr 7) and 0x1).toUByte())) ?: return null
                    CodecZenohDeclarationVariant.CodecZenohUndeclSubscriber(_arm)
                }
                4 -> {
                    val _arm = com.sce.generated.codec_zenoh_decl_queryable.CodecZenohDeclQueryable.decode(cursor, (((header.toInt() shr 5) and 0x1).toUByte()), (((header.toInt() shr 7) and 0x1).toUByte())) ?: return null
                    CodecZenohDeclarationVariant.CodecZenohDeclQueryable(_arm)
                }
                5 -> {
                    val _arm = com.sce.generated.codec_zenoh_undecl_queryable.CodecZenohUndeclQueryable.decode(cursor, (((header.toInt() shr 7) and 0x1).toUByte())) ?: return null
                    CodecZenohDeclarationVariant.CodecZenohUndeclQueryable(_arm)
                }
                6 -> {
                    val _arm = com.sce.generated.codec_zenoh_decl_token.CodecZenohDeclToken.decode(cursor, (((header.toInt() shr 5) and 0x1).toUByte())) ?: return null
                    CodecZenohDeclarationVariant.CodecZenohDeclToken(_arm)
                }
                7 -> {
                    val _arm = com.sce.generated.codec_zenoh_undecl_token.CodecZenohUndeclToken.decode(cursor, (((header.toInt() shr 7) and 0x1).toUByte())) ?: return null
                    CodecZenohDeclarationVariant.CodecZenohUndeclToken(_arm)
                }
                26 -> {
                    val _arm = com.sce.generated.codec_zenoh_decl_final.CodecZenohDeclFinal.decode(cursor) ?: return null
                    CodecZenohDeclarationVariant.CodecZenohDeclFinal(_arm)
                }
                else -> {
                    val _arm = com.sce.generated.codec_zenoh_decl_final.CodecZenohDeclFinal.decode(cursor) ?: return null
                    CodecZenohDeclarationVariant.Default(tag = ((header.toInt() shr 0) and 0x1F).toUByte(), body = _arm)
                }
            }
            return CodecZenohDeclaration(
                header = header,
                body = body
            )
        }
    }
}
