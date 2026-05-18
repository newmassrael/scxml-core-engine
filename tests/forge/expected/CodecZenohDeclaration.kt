// SCE-MAP: codec_zenoh_declaration:54

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_zenoh_declaration

import com.sce.forge.runtime.SceCursor
import com.sce.generated.codec_zenoh_decl_keyexpr.*
import com.sce.generated.codec_zenoh_undecl_keyexpr.*
import com.sce.generated.codec_zenoh_decl_subscriber.*
import com.sce.generated.codec_zenoh_undecl_subscriber.*
import com.sce.generated.codec_zenoh_decl_queryable.*
import com.sce.generated.codec_zenoh_undecl_queryable.*
import com.sce.generated.codec_zenoh_decl_token.*
import com.sce.generated.codec_zenoh_undecl_token.*
import com.sce.generated.codec_decl_final.*

// RFC §5.B variant primitive (B1-β): discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body. Arm body types are referenced by FQN
// (defensive — wildcard imports could otherwise surface an ambiguity if
// two imported codecs declare same-named inner classes).
sealed class CodecZenohDeclarationVariant {
    data class CodecZenohDeclKeyexpr(val body: com.sce.generated.codec_zenoh_decl_keyexpr.CodecZenohDeclKeyexpr) : CodecZenohDeclarationVariant()
    data class CodecZenohUndeclKeyexpr(val body: com.sce.generated.codec_zenoh_undecl_keyexpr.CodecZenohUndeclKeyexpr) : CodecZenohDeclarationVariant()
    data class CodecZenohDeclSubscriber(val body: com.sce.generated.codec_zenoh_decl_subscriber.CodecZenohDeclSubscriber) : CodecZenohDeclarationVariant()
    data class CodecZenohUndeclSubscriber(val body: com.sce.generated.codec_zenoh_undecl_subscriber.CodecZenohUndeclSubscriber) : CodecZenohDeclarationVariant()
    data class CodecZenohDeclQueryable(val body: com.sce.generated.codec_zenoh_decl_queryable.CodecZenohDeclQueryable) : CodecZenohDeclarationVariant()
    data class CodecZenohUndeclQueryable(val body: com.sce.generated.codec_zenoh_undecl_queryable.CodecZenohUndeclQueryable) : CodecZenohDeclarationVariant()
    data class CodecZenohDeclToken(val body: com.sce.generated.codec_zenoh_decl_token.CodecZenohDeclToken) : CodecZenohDeclarationVariant()
    data class CodecZenohUndeclToken(val body: com.sce.generated.codec_zenoh_undecl_token.CodecZenohUndeclToken) : CodecZenohDeclarationVariant()
    data class CodecDeclFinal(val body: com.sce.generated.codec_decl_final.CodecDeclFinal) : CodecZenohDeclarationVariant()
    data class Default(val tag: UByte, val body: com.sce.generated.codec_decl_final.CodecDeclFinal) : CodecZenohDeclarationVariant()
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
    var body: CodecZenohDeclarationVariant = CodecZenohDeclarationVariant.CodecDeclFinal(com.sce.generated.codec_decl_final.CodecDeclFinal())
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

    fun encode(): ByteArray {
        // Encode fixed prefix (tag field bytes are part of the prefix).
        // The tag value is read from the struct field, NOT derived from
        // the body discriminant — keeping author-set tag / body in sync
        // is the caller's responsibility (v1 keeps the layout simple).
        val r = mutableListOf<Byte>()
        r.add(header.toByte())
        // Append the active arm body's encoded bytes.
        when (val _b = this.body) {
            is CodecZenohDeclarationVariant.CodecZenohDeclKeyexpr -> r.addAll(_b.body.encode(this.header).toList())
            is CodecZenohDeclarationVariant.CodecZenohUndeclKeyexpr -> r.addAll(_b.body.encode().toList())
            is CodecZenohDeclarationVariant.CodecZenohDeclSubscriber -> r.addAll(_b.body.encode(this.header).toList())
            is CodecZenohDeclarationVariant.CodecZenohUndeclSubscriber -> r.addAll(_b.body.encode(this.header).toList())
            is CodecZenohDeclarationVariant.CodecZenohDeclQueryable -> r.addAll(_b.body.encode(this.header).toList())
            is CodecZenohDeclarationVariant.CodecZenohUndeclQueryable -> r.addAll(_b.body.encode(this.header).toList())
            is CodecZenohDeclarationVariant.CodecZenohDeclToken -> r.addAll(_b.body.encode(this.header).toList())
            is CodecZenohDeclarationVariant.CodecZenohUndeclToken -> r.addAll(_b.body.encode(this.header).toList())
            is CodecZenohDeclarationVariant.CodecDeclFinal -> r.addAll(_b.body.encode().toList())
            is CodecZenohDeclarationVariant.Default -> r.addAll(_b.body.encode().toList())
        }
        return r.toByteArray()
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
                    val _arm = com.sce.generated.codec_zenoh_decl_keyexpr.CodecZenohDeclKeyexpr.decode(cursor, header) ?: return null
                    CodecZenohDeclarationVariant.CodecZenohDeclKeyexpr(_arm)
                }
                1 -> {
                    val _arm = com.sce.generated.codec_zenoh_undecl_keyexpr.CodecZenohUndeclKeyexpr.decode(cursor) ?: return null
                    CodecZenohDeclarationVariant.CodecZenohUndeclKeyexpr(_arm)
                }
                2 -> {
                    val _arm = com.sce.generated.codec_zenoh_decl_subscriber.CodecZenohDeclSubscriber.decode(cursor, header) ?: return null
                    CodecZenohDeclarationVariant.CodecZenohDeclSubscriber(_arm)
                }
                3 -> {
                    val _arm = com.sce.generated.codec_zenoh_undecl_subscriber.CodecZenohUndeclSubscriber.decode(cursor, header) ?: return null
                    CodecZenohDeclarationVariant.CodecZenohUndeclSubscriber(_arm)
                }
                4 -> {
                    val _arm = com.sce.generated.codec_zenoh_decl_queryable.CodecZenohDeclQueryable.decode(cursor, header) ?: return null
                    CodecZenohDeclarationVariant.CodecZenohDeclQueryable(_arm)
                }
                5 -> {
                    val _arm = com.sce.generated.codec_zenoh_undecl_queryable.CodecZenohUndeclQueryable.decode(cursor, header) ?: return null
                    CodecZenohDeclarationVariant.CodecZenohUndeclQueryable(_arm)
                }
                6 -> {
                    val _arm = com.sce.generated.codec_zenoh_decl_token.CodecZenohDeclToken.decode(cursor, header) ?: return null
                    CodecZenohDeclarationVariant.CodecZenohDeclToken(_arm)
                }
                7 -> {
                    val _arm = com.sce.generated.codec_zenoh_undecl_token.CodecZenohUndeclToken.decode(cursor, header) ?: return null
                    CodecZenohDeclarationVariant.CodecZenohUndeclToken(_arm)
                }
                26 -> {
                    val _arm = com.sce.generated.codec_decl_final.CodecDeclFinal.decode(cursor) ?: return null
                    CodecZenohDeclarationVariant.CodecDeclFinal(_arm)
                }
                else -> {
                    val _arm = com.sce.generated.codec_decl_final.CodecDeclFinal.decode(cursor) ?: return null
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
