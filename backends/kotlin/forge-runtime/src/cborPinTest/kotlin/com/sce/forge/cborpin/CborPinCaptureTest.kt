// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The Kotlin driver of the CBOR pin (tests/forge/cbor_pin/pin.py, docs/adr/0101
// step 0).
//
// Compiled into jvmTest only when scripts/capture_cbor_pin.sh asks for it by
// setting SCE_CBOR_PIN_INPUTS (build.gradle.kts, `generateCborPinFixtures`), beside
// the Kotlin the generator wrote for the pinned codecs and every document they
// import. Each line of the file SCE_CBOR_PIN_INPUTS names, `<codec>/<case> <hex>`,
// is decoded by that codec and the value it read encoded again; the file
// SCE_CBOR_PIN_OUTPUT names gets `<id> ok <hex>` with what the encode wrote, or
// `<id> refused <words>` naming the CodecError the decode or the encode answered.

package com.sce.forge.cborpin

import com.sce.forge.runtime.CodecError
import com.sce.forge.runtime.Decoded
import com.sce.forge.runtime.SceCursor
import com.sce.forge.runtime.SceSink
import com.sce.generated.broadcast_log_conformance_text_event.BroadcastLogConformanceTextEvent
import com.sce.generated.codec_cbor_map.CodecCborMap
import com.sce.generated.codec_cbor_open_enum.CodecCborOpenEnum
import com.sce.generated.codec_cbor_record_list.CodecCborRecordList
import com.sce.generated.codec_cbor_record_list_bytes.CodecCborRecordListBytes
import com.sce.generated.codec_cbor_record_list_least.CodecCborRecordListLeast
import com.sce.generated.codec_cbor_signed_enum.CodecCborSignedEnum
import com.sce.generated.envelope.Envelope
import java.io.ByteArrayOutputStream
import java.io.File
import kotlin.test.Test

/** A sink that grows, over one array rather than a list of boxed bytes. */
private class GrowingSink : SceSink {
    val out = ByteArrayOutputStream()

    override fun writeBytes(bytes: ByteArray, off: Int, len: Int): CodecError? {
        out.write(bytes, off, len)
        return null
    }

    override fun position(): Int = out.size()
}

private const val HEX_DIGITS = "0123456789abcdef"

private fun hex(bytes: ByteArray): String {
    val text = StringBuilder(bytes.size * 2)
    for (b in bytes) {
        val v = b.toInt() and 0xff
        text.append(HEX_DIGITS[v ushr 4])
        text.append(HEX_DIGITS[v and 0x0f])
    }
    return text.toString()
}

private fun unhex(text: String): ByteArray {
    require(text.length % 2 == 0) { "an odd count of hexadecimal digits" }
    return ByteArray(text.length / 2) { i ->
        val high = Character.digit(text[2 * i], 16)
        val low = Character.digit(text[2 * i + 1], 16)
        require(high >= 0 && low >= 0) { "not a hexadecimal digit near ${2 * i}" }
        ((high shl 4) or low).toByte()
    }
}

/** One decode of [input] and one encode of what it read, as the line's verdict. */
private fun <T> roundTrip(
    input: ByteArray,
    decode: (SceCursor) -> Decoded<T>,
    encode: (T, SceSink) -> CodecError?,
): String {
    val value = decode(SceCursor(input)).valueOr { return "refused decode ${it.error}" }
    val sink = GrowingSink()
    val refused = encode(value, sink)
    if (refused != null) return "refused encode $refused"
    return "ok " + hex(sink.out.toByteArray())
}

/** Each pinned codec by its document's name. */
private val CODECS: Map<String, (ByteArray) -> String> = mapOf(
    "codec_cbor_map" to { input: ByteArray ->
        roundTrip(input, { c -> CodecCborMap.decode(c) }) { v, w -> v.encode(w) }
    },
    "broadcast_log_conformance_text_event" to { input: ByteArray ->
        roundTrip(input, { c -> BroadcastLogConformanceTextEvent.decode(c) }) { v, w -> v.encode(w) }
    },
    "codec_cbor_open_enum" to { input: ByteArray ->
        roundTrip(input, { c -> CodecCborOpenEnum.decode(c) }) { v, w -> v.encode(w) }
    },
    "codec_cbor_signed_enum" to { input: ByteArray ->
        roundTrip(input, { c -> CodecCborSignedEnum.decode(c) }) { v, w -> v.encode(w) }
    },
    "codec_cbor_record_list" to { input: ByteArray ->
        roundTrip(input, { c -> CodecCborRecordList.decode(c) }) { v, w -> v.encode(w) }
    },
    "codec_cbor_record_list_bytes" to { input: ByteArray ->
        roundTrip(input, { c -> CodecCborRecordListBytes.decode(c) }) { v, w -> v.encode(w) }
    },
    "codec_cbor_record_list_least" to { input: ByteArray ->
        roundTrip(input, { c -> CodecCborRecordListLeast.decode(c) }) { v, w -> v.encode(w) }
    },
    "envelope" to { input: ByteArray ->
        roundTrip(input, { c -> Envelope.decode(c) }) { v, w -> v.encode(w) }
    },
)

class CborPinCaptureTest {
    @Test
    fun eachPinnedCaseIsDecodedAndEncodedAgain() {
        val inputs = checkNotNull(System.getenv("SCE_CBOR_PIN_INPUTS")) {
            "SCE_CBOR_PIN_INPUTS names the cases to capture (scripts/capture_cbor_pin.sh)"
        }
        val output = checkNotNull(System.getenv("SCE_CBOR_PIN_OUTPUT")) {
            "SCE_CBOR_PIN_OUTPUT names where the capture is written"
        }
        File(output).bufferedWriter().use { written ->
            File(inputs).forEachLine { line ->
                if (line.isNotBlank()) {
                    val space = line.indexOf(' ')
                    check(space > 0) { "not a case line: ${line.take(80)}" }
                    val id = line.substring(0, space)
                    val run = checkNotNull(CODECS[id.substringBefore('/')]) { "$id names no pinned codec" }
                    written.write(id)
                    written.write(" ")
                    written.write(run(unhex(line.substring(space + 1).trim())))
                    written.write("\n")
                }
            }
        }
    }
}
