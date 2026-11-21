// SCE-MAP: codec_variant_peek_basic:29 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.codec_variant_peek_basic

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.MutableListSink
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.codec_peek_arm_a.*
import com.sce.generated.codec_peek_arm_b.*

// RFC §synth-5-B variant primitive: discriminated-union body for the
// codec's tag-field suffix. Each arm wraps an imported codec's decoded
// value; the optional Default arm preserves the runtime tag value
// alongside its catch-all body. Arm body types are referenced by FQN
// (defensive — wildcard imports could otherwise surface an ambiguity if
// two imported codecs declare same-named inner classes).
sealed class CodecVariantPeekBasicVariant {
    data class CodecPeekArmA(val body: com.sce.generated.codec_peek_arm_a.CodecPeekArmASource) : CodecVariantPeekBasicVariant()
    data class CodecPeekArmB(val body: com.sce.generated.codec_peek_arm_b.CodecPeekArmBSource) : CodecVariantPeekBasicVariant()
}

/// The elements a value of [CodecVariantPeekBasic] presents, which its encode reads
/// (docs/adr/0081 §2): any object that presents them is a source of it, which
/// [CodecVariantPeekBasic.encodeSource] writes. The encode derives every other element
/// of the frame.
interface CodecVariantPeekBasicSource {
    val body: CodecVariantPeekBasicVariant
}

// Default-valued primary constructor: the generated procedure_l2 code
// holds codec instances as owned members and initializes them with
// `CodecVariantPeekBasic()` before any encode()/decode() call. Each default
// is a value of that field's own type, which decode() then fills in on
// success — the carrier's zero for a number, and for an enum the first
// variant its document declares, since a closed set does not hold a
// value it never declared.
data class CodecVariantPeekBasic(
    // RFC variant-default-uniformity (Kotlin): pick the declared
    // default arm (`<sce:arm default="true"/>`) instead of the first
    // alternative so a freshly-constructed envelope round-trips byte-
    // exactly through `encode() -> decode()`. Paired with the inner
    // codec's `<sce:flag value=>`-baked default fields above.
    override var body: CodecVariantPeekBasicVariant = CodecVariantPeekBasicVariant.CodecPeekArmA(com.sce.generated.codec_peek_arm_a.CodecPeekArmA())
) : CodecVariantPeekBasicSource {
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
        /// Write any source of [CodecVariantPeekBasic] into `w`: each element read once off
        /// `src` and written where the codec's encode plan lays it out
        /// (docs/adr/0081). It refuses no value: it answers `null`, or
        /// [CodecError.BufferOverflow] from a bounded sink with no room left.
        fun encodeSource(src: CodecVariantPeekBasicSource, w: SceSink): CodecError? {
            val _r_body = src.body
            when (val _b = _r_body) {
                is CodecVariantPeekBasicVariant.CodecPeekArmA -> CodecPeekArmA.encodeSource(_b.body, w)?.let { return it }
                is CodecVariantPeekBasicVariant.CodecPeekArmB -> CodecPeekArmB.encodeSource(_b.body, w)?.let { return it }
            }
            return null
        }

        /// Decode the next frame from `cursor`. On success the cursor
        /// advances past the consumed bytes and the value is `Decoded.Ok`;
        /// a refused frame is `Decoded.Refused` with the kind that refused
        /// it (docs/adr/0099, amendment of 2026-11-16): `NeedMoreBytes` when
        /// the cursor's tail is shorter than the frame (RFC
        /// §synth-5-B L494-519), which the caller resumes after appending
        /// bytes, and another kind for a frame no more bytes make whole.
        fun decode(cursor: SceCursor): Decoded<CodecVariantPeekBasic> {
            // RFC §synth-5-B peek-byte / streaming-
            // prefix: streaming prefix decode (variable-length fields
            // supported via per-field present_if/tlv-chain/embed/repeat
            // helpers). Peek-byte mode additionally peeks the cursor's
            // next byte for variant tag without advancing — arm body
            // decoder reads it as own header.
            val _peekRaw = cursor.peekSlice(1) ?: return Decoded.Refused(CodecError.NeedMoreBytes)
        val _peek: UByte = _peekRaw[0].toUByte()
            // Dispatch on the tag field; each arm decodes its body codec
            // from the cursor. The default arm (when declared) carries
            // the runtime tag value so encode can round-trip it back
            // onto the wire.
            val body: CodecVariantPeekBasicVariant = when (((_peek.toInt() shr 0) and 0x01)) {
                0 -> {
                    val _arm = com.sce.generated.codec_peek_arm_a.CodecPeekArmA.decode(cursor).valueOr { _refused -> return _refused }
                    CodecVariantPeekBasicVariant.CodecPeekArmA(_arm)
                }
                1 -> {
                    val _arm = com.sce.generated.codec_peek_arm_b.CodecPeekArmB.decode(cursor).valueOr { _refused -> return _refused }
                    CodecVariantPeekBasicVariant.CodecPeekArmB(_arm)
                }
                else -> {
                    // codec/variant-arm-unreachable proves every tag names an
                    // arm, so no frame reaches here.
                    error("codec/variant-arm-unreachable admitted a tag no arm names")
                }
            }
            return Decoded.Ok(
                CodecVariantPeekBasic(
                    body = body
                )
            )
        }
    }
}
