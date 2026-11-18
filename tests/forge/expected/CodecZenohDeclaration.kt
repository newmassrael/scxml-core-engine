// SCE-MAP: codec_zenoh_declaration:54 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_declaration

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
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
import kotlin.jvm.JvmInline

// RFC §synth-5-B variant primitive: discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body. Arm body types are referenced by FQN
// (defensive — wildcard imports could otherwise surface an ambiguity if
// two imported codecs declare same-named inner classes).
sealed class CodecZenohDeclarationVariant {
    data class CodecZenohDeclKexpr(val body: com.sce.generated.codec_zenoh_decl_kexpr.CodecZenohDeclKexprSource) : CodecZenohDeclarationVariant()
    data class CodecZenohUndeclKexpr(val body: com.sce.generated.codec_zenoh_undecl_kexpr.CodecZenohUndeclKexprSource) : CodecZenohDeclarationVariant()
    data class CodecZenohDeclSubscriber(val body: com.sce.generated.codec_zenoh_decl_subscriber.CodecZenohDeclSubscriberSource) : CodecZenohDeclarationVariant()
    data class CodecZenohUndeclSubscriber(val body: com.sce.generated.codec_zenoh_undecl_subscriber.CodecZenohUndeclSubscriberSource) : CodecZenohDeclarationVariant()
    data class CodecZenohDeclQueryable(val body: com.sce.generated.codec_zenoh_decl_queryable.CodecZenohDeclQueryableSource) : CodecZenohDeclarationVariant()
    data class CodecZenohUndeclQueryable(val body: com.sce.generated.codec_zenoh_undecl_queryable.CodecZenohUndeclQueryableSource) : CodecZenohDeclarationVariant()
    data class CodecZenohDeclToken(val body: com.sce.generated.codec_zenoh_decl_token.CodecZenohDeclTokenSource) : CodecZenohDeclarationVariant()
    data class CodecZenohUndeclToken(val body: com.sce.generated.codec_zenoh_undecl_token.CodecZenohUndeclTokenSource) : CodecZenohDeclarationVariant()
    data class CodecZenohDeclFinal(val body: com.sce.generated.codec_zenoh_decl_final.CodecZenohDeclFinalSource) : CodecZenohDeclarationVariant()
    data class Default(val tag: CodecZenohDeclarationUnknownTag, val body: com.sce.generated.codec_zenoh_decl_final.CodecZenohDeclFinalSource) : CodecZenohDeclarationVariant()
}

/// The tag the catch-all arm of [CodecZenohDeclaration]'s variant holds: one the bits
/// the tag is read from hold and no arm names, held to that where a value of
/// it is made (docs/adr/0065 §6 R3, docs/adr/0081 §2), so the catch-all a value
/// holds is written and read back as the catch-all.
@JvmInline
value class CodecZenohDeclarationUnknownTag private constructor(val tag: UByte) {
    /// The tag as a `ULong`, the type an encode derives a tag in.
    val wide: ULong
        get() = tag.toULong()

    companion object {
        /// `tag` as a value of the catch-all, or `null` where an arm names it or the
        /// bits the tag is read from do not hold it.
        fun of(tag: UByte): CodecZenohDeclarationUnknownTag? =
            when (tag.toUInt()) {
                in 0x0u..0x7u, 0x1Au -> null
                else -> if (tag.toUInt() <= 0x1Fu) CodecZenohDeclarationUnknownTag(tag) else null
            }

        /// The least tag no arm names: what a catch-all made by default holds.
        fun fewest(): CodecZenohDeclarationUnknownTag = CodecZenohDeclarationUnknownTag(0x8u.toUByte())
    }
}

/// One-bit flags of [CodecZenohDeclaration] a parent may set in place of the ones a value
/// presents, such as the flag a chain whose entries are [CodecZenohDeclaration] ends them
/// on, which the chain derives from where each entry stands.
class CodecZenohDeclarationOverrides(
    /// The `M` flag of `header`, where a parent sets it.
    val header_M: Boolean? = null,
)

/// The elements a value of [CodecZenohDeclaration] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [CodecZenohDeclaration.encodeSource] writes. The encode derives every other element
/// of the frame.
interface CodecZenohDeclarationSource {
    val header: UByte
    val body: CodecZenohDeclarationVariant
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecZenohDeclaration()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecZenohDeclaration(
    override var header: UByte = 0.toUByte(),
    // RFC variant-default-uniformity (Kotlin): pick the declared
    // default arm (`<sce:arm default="true"/>`) instead of the first
    // alternative so a freshly-constructed envelope round-trips byte-
    // exactly through `encode() -> decode()`. Paired with the inner
    // codec's `<sce:flag value=>`-baked default fields above.
    override var body: CodecZenohDeclarationVariant = CodecZenohDeclarationVariant.CodecZenohDeclFinal(com.sce.generated.codec_zenoh_decl_final.CodecZenohDeclFinal())
) : CodecZenohDeclarationSource {
    // RFC §synth-5-B flags primitive: per-bit-range accessors over
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

    /// RFC §synth-5-B encode-side primary: write `self` into the
    /// caller-owned `w` sink. Returns `null` on success;
    /// `CodecError.BufferOverflow` from a bounded sink when the
    /// destination has insufficient remaining capacity; growable
    /// sinks (e.g. `MutableListSink`) are effectively infallible.
    fun encode(w: SceSink): CodecError? {
        // docs/adr/0081 step 4: the encode over any source, which derives
        // every element the value does not present. It is handed the flag
        // inputs the value does not say; one the value says is read off it,
        // and what a caller hands this `encode` for it is held to that
        // (docs/adr/0065 §6 R4a).
        return encodeSource(this, w)
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
        /// Write any source of [CodecZenohDeclaration] into `w`: each element read once off
        /// `src` and written where the codec's encode plan lays it out
        /// (docs/adr/0081). A chain that holds it
        /// sets the flag its entries end on by `overrides`, in place of the value's. It refuses no value: it answers `null`, or
        /// [CodecError.BufferOverflow] from a bounded sink with no room left.
        fun encodeSource(src: CodecZenohDeclarationSource, w: SceSink, overrides: CodecZenohDeclarationOverrides? = null): CodecError? {
            val _e_header = src.header
            val _r_body = src.body
            val _t_body: ULong = when (val _b = _r_body) {
                is CodecZenohDeclarationVariant.CodecZenohDeclKexpr -> 0x0uL
                is CodecZenohDeclarationVariant.CodecZenohUndeclKexpr -> 0x1uL
                is CodecZenohDeclarationVariant.CodecZenohDeclSubscriber -> 0x2uL
                is CodecZenohDeclarationVariant.CodecZenohUndeclSubscriber -> 0x3uL
                is CodecZenohDeclarationVariant.CodecZenohDeclQueryable -> 0x4uL
                is CodecZenohDeclarationVariant.CodecZenohUndeclQueryable -> 0x5uL
                is CodecZenohDeclarationVariant.CodecZenohDeclToken -> 0x6uL
                is CodecZenohDeclarationVariant.CodecZenohUndeclToken -> 0x7uL
                is CodecZenohDeclarationVariant.CodecZenohDeclFinal -> 0x1AuL
                is CodecZenohDeclarationVariant.Default -> _b.tag.wide
            }
            var _u_header = _e_header.toUInt()
            _u_header = (_u_header and 0x1Fu.inv()) or ((_t_body and 0x1FuL).toUInt() shl 0)
            if ((_r_body is CodecZenohDeclarationVariant.CodecZenohDeclKexpr || _r_body is CodecZenohDeclarationVariant.CodecZenohDeclSubscriber || _r_body is CodecZenohDeclarationVariant.CodecZenohDeclQueryable || _r_body is CodecZenohDeclarationVariant.CodecZenohDeclToken)) {
                _u_header = (_u_header and 0x20u.inv()) or (((when (val _b = _r_body) { is CodecZenohDeclarationVariant.CodecZenohDeclKexpr -> CodecZenohDeclKexpr.inputN(_b.body).toInt(); is CodecZenohDeclarationVariant.CodecZenohUndeclKexpr -> 0; is CodecZenohDeclarationVariant.CodecZenohDeclSubscriber -> CodecZenohDeclSubscriber.inputN(_b.body).toInt(); is CodecZenohDeclarationVariant.CodecZenohUndeclSubscriber -> 0; is CodecZenohDeclarationVariant.CodecZenohDeclQueryable -> CodecZenohDeclQueryable.inputN(_b.body).toInt(); is CodecZenohDeclarationVariant.CodecZenohUndeclQueryable -> 0; is CodecZenohDeclarationVariant.CodecZenohDeclToken -> CodecZenohDeclToken.inputN(_b.body).toInt(); is CodecZenohDeclarationVariant.CodecZenohUndeclToken -> 0; is CodecZenohDeclarationVariant.CodecZenohDeclFinal -> 0; is CodecZenohDeclarationVariant.Default -> 0 })).toUInt() shl 5)
            }
            if ((_r_body is CodecZenohDeclarationVariant.CodecZenohUndeclSubscriber || _r_body is CodecZenohDeclarationVariant.CodecZenohDeclQueryable || _r_body is CodecZenohDeclarationVariant.CodecZenohUndeclQueryable || _r_body is CodecZenohDeclarationVariant.CodecZenohUndeclToken)) {
                _u_header = (_u_header and 0x80u.inv()) or (((when (val _b = _r_body) { is CodecZenohDeclarationVariant.CodecZenohDeclKexpr -> 0; is CodecZenohDeclarationVariant.CodecZenohUndeclKexpr -> 0; is CodecZenohDeclarationVariant.CodecZenohDeclSubscriber -> 0; is CodecZenohDeclarationVariant.CodecZenohUndeclSubscriber -> CodecZenohUndeclSubscriber.inputZ(_b.body).toInt(); is CodecZenohDeclarationVariant.CodecZenohDeclQueryable -> CodecZenohDeclQueryable.inputZ(_b.body).toInt(); is CodecZenohDeclarationVariant.CodecZenohUndeclQueryable -> CodecZenohUndeclQueryable.inputZ(_b.body).toInt(); is CodecZenohDeclarationVariant.CodecZenohDeclToken -> 0; is CodecZenohDeclarationVariant.CodecZenohUndeclToken -> CodecZenohUndeclToken.inputZ(_b.body).toInt(); is CodecZenohDeclarationVariant.CodecZenohDeclFinal -> 0; is CodecZenohDeclarationVariant.Default -> 0 })).toUInt() shl 7)
            }
            overrides?.header_M?.let { _u_header = (_u_header and 0x40u.inv()) or (if (it) 0x40u else 0x0u) }
            val _c_header = _u_header.toUByte()
            w.writeU8(_c_header.toByte())?.let { return it }
            when (val _b = _r_body) {
                is CodecZenohDeclarationVariant.CodecZenohDeclKexpr -> CodecZenohDeclKexpr.encodeSource(_b.body, w)?.let { return it }
                is CodecZenohDeclarationVariant.CodecZenohUndeclKexpr -> CodecZenohUndeclKexpr.encodeSource(_b.body, w)?.let { return it }
                is CodecZenohDeclarationVariant.CodecZenohDeclSubscriber -> CodecZenohDeclSubscriber.encodeSource(_b.body, w)?.let { return it }
                is CodecZenohDeclarationVariant.CodecZenohUndeclSubscriber -> CodecZenohUndeclSubscriber.encodeSource(_b.body, w)?.let { return it }
                is CodecZenohDeclarationVariant.CodecZenohDeclQueryable -> CodecZenohDeclQueryable.encodeSource(_b.body, w)?.let { return it }
                is CodecZenohDeclarationVariant.CodecZenohUndeclQueryable -> CodecZenohUndeclQueryable.encodeSource(_b.body, w)?.let { return it }
                is CodecZenohDeclarationVariant.CodecZenohDeclToken -> CodecZenohDeclToken.encodeSource(_b.body, w)?.let { return it }
                is CodecZenohDeclarationVariant.CodecZenohUndeclToken -> CodecZenohUndeclToken.encodeSource(_b.body, w)?.let { return it }
                is CodecZenohDeclarationVariant.CodecZenohDeclFinal -> CodecZenohDeclFinal.encodeSource(_b.body, w)?.let { return it }
                is CodecZenohDeclarationVariant.Default -> CodecZenohDeclFinal.encodeSource(_b.body, w)?.let { return it }
            }
            return null
        }

        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes and the value is `Decoded.Ok`;
        /// a refused frame is `Decoded.Refused` with the kind that refused
        /// it (docs/adr/0099, amendment of 2026-11-16): `NeedMoreBytes` when
        /// the cursor's tail is shorter than the frame (RFC §synth-5-B
        /// L494-519), which the caller resumes after appending bytes, and
        /// another kind for a frame no more bytes make whole.
        fun decode(cursor: SceCursor): Decoded<CodecZenohDeclaration> {
            // Decode fixed prefix (RFC §synth-5-B variant: fields before tag suffix).
            val raw = cursor.peekSlice(1) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
            val header = raw[0].toUByte()
            if (!cursor.advance(1)) return Decoded.Refused(CodecError.NeedMoreBytes)
            // Dispatch on the tag field; each arm decodes its body codec
            // from the cursor. The default arm (when declared) carries
            // the runtime tag value so encode can round-trip it back
            // onto the wire.
            val body: CodecZenohDeclarationVariant = when (((header.toInt() shr 0) and 0x1F)) {
                0 -> {
                    val _arm = com.sce.generated.codec_zenoh_decl_kexpr.CodecZenohDeclKexpr.decode(cursor, (((header.toInt() shr 5) and 0x1).toUByte())).valueOr { _refused -> return _refused }
                    CodecZenohDeclarationVariant.CodecZenohDeclKexpr(_arm)
                }
                1 -> {
                    val _arm = com.sce.generated.codec_zenoh_undecl_kexpr.CodecZenohUndeclKexpr.decode(cursor).valueOr { _refused -> return _refused }
                    CodecZenohDeclarationVariant.CodecZenohUndeclKexpr(_arm)
                }
                2 -> {
                    val _arm = com.sce.generated.codec_zenoh_decl_subscriber.CodecZenohDeclSubscriber.decode(cursor, (((header.toInt() shr 5) and 0x1).toUByte())).valueOr { _refused -> return _refused }
                    CodecZenohDeclarationVariant.CodecZenohDeclSubscriber(_arm)
                }
                3 -> {
                    val _arm = com.sce.generated.codec_zenoh_undecl_subscriber.CodecZenohUndeclSubscriber.decode(cursor, (((header.toInt() shr 7) and 0x1).toUByte())).valueOr { _refused -> return _refused }
                    CodecZenohDeclarationVariant.CodecZenohUndeclSubscriber(_arm)
                }
                4 -> {
                    val _arm = com.sce.generated.codec_zenoh_decl_queryable.CodecZenohDeclQueryable.decode(cursor, (((header.toInt() shr 5) and 0x1).toUByte()), (((header.toInt() shr 7) and 0x1).toUByte())).valueOr { _refused -> return _refused }
                    CodecZenohDeclarationVariant.CodecZenohDeclQueryable(_arm)
                }
                5 -> {
                    val _arm = com.sce.generated.codec_zenoh_undecl_queryable.CodecZenohUndeclQueryable.decode(cursor, (((header.toInt() shr 7) and 0x1).toUByte())).valueOr { _refused -> return _refused }
                    CodecZenohDeclarationVariant.CodecZenohUndeclQueryable(_arm)
                }
                6 -> {
                    val _arm = com.sce.generated.codec_zenoh_decl_token.CodecZenohDeclToken.decode(cursor, (((header.toInt() shr 5) and 0x1).toUByte())).valueOr { _refused -> return _refused }
                    CodecZenohDeclarationVariant.CodecZenohDeclToken(_arm)
                }
                7 -> {
                    val _arm = com.sce.generated.codec_zenoh_undecl_token.CodecZenohUndeclToken.decode(cursor, (((header.toInt() shr 7) and 0x1).toUByte())).valueOr { _refused -> return _refused }
                    CodecZenohDeclarationVariant.CodecZenohUndeclToken(_arm)
                }
                26 -> {
                    val _arm = com.sce.generated.codec_zenoh_decl_final.CodecZenohDeclFinal.decode(cursor).valueOr { _refused -> return _refused }
                    CodecZenohDeclarationVariant.CodecZenohDeclFinal(_arm)
                }
                else -> {
                    val _arm = com.sce.generated.codec_zenoh_decl_final.CodecZenohDeclFinal.decode(cursor).valueOr { _refused -> return _refused }
                    CodecZenohDeclarationVariant.Default(tag = CodecZenohDeclarationUnknownTag.of(((header.toInt() shr 0) and 0x1F).toUByte()) ?: error("a tag no arm names is one CodecZenohDeclarationUnknownTag holds"), body = _arm)
                }
            }
            return Decoded.Ok(
                CodecZenohDeclaration(
                    header = header,
                    body = body
                )
            )
        }
    }
}
