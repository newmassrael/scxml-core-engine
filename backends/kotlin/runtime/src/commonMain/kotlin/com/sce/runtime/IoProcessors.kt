// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.runtime

/**
 * One entry of `_ioprocessors`: the key it is filed under, and the address
 * external entities use to reach this session through that processor.
 */
data class IoProcessorDescriptor(val name: String, val location: String)

/**
 * `_ioprocessors` entry set (§scxml-C-1-1, §scxml-C-2-3).
 *
 * Port of the C++ `IOProcessorHelper` (`sce/include/common/IOProcessorHelper.h`).
 * Deciding the entries here rather than inside each script engine is what keeps
 * a machine reading the same entry names and the same addresses whichever
 * backend runs it — before this existed, QuickJS published a single `scxml`
 * key whose location was the raw session id.
 */
object IoProcessors {
    /** Entry name §scxml-C-1-1 requires for the SCXML Event I/O Processor. */
    const val SCXML_PROCESSOR = "http://www.w3.org/TR/scxml/#SCXMLEventProcessor"

    /** Entry name §scxml-C-2-3 requires for the Basic HTTP Event I/O Processor. */
    const val BASIC_HTTP_PROCESSOR = "http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor"

    /** Alias the SCXML processor is indexed under by SCXML documents. */
    const val SCXML_ALIAS = "scxml"

    /** Alias the Basic HTTP processor is indexed under by SCXML documents. */
    const val BASIC_HTTP_ALIAS = "basichttp"

    private const val UNRESERVED = "-_.~"

    /**
     * Address that reaches this session over the SCXML Event I/O Processor.
     *
     * §scxml-C-1 leaves the transport platform-specific, so the address is an
     * SCE-scheme URI naming the session. The session id is percent-encoded
     * because it is not constrained to URI-safe characters.
     */
    fun scxmlLocation(sessionId: String): String = "sce://scxml/" + percentEncode(sessionId)

    /**
     * Session id an SCXML Event I/O Processor location names, or `""`.
     *
     * The inverse of [scxmlLocation], kept beside it so the two spellings of
     * one address cannot drift apart. §scxml-C-1 requires the location a
     * session publishes to be usable as a `<send>` target, which only holds if
     * something can read a session back out of it.
     */
    fun sessionIdFromScxmlLocation(uri: String): String {
        val prefix = "sce://scxml/"
        if (uri.length <= prefix.length || !uri.startsWith(prefix)) return ""
        return percentDecode(uri.substring(prefix.length))
    }

    /**
     * The `_event.origin` a receiver should see for an event sent by
     * [originSessionId].
     *
     * §scxml-C-1 requires the origin of a delivered event to match the
     * 'location' the sending session published, which is what makes it an
     * address the receiver can answer. The engine carries the sender's BARE
     * session id internally — [EventMetadata.origin] — because its
     * session-keyed lookups (`<finalize>` dispatch, cancelled-invoke
     * filtering) match on the id. Converting where the event is raised would
     * make one value serve two consumers that need different spellings. So the
     * conversion belongs at the boundary where the value becomes visible to the
     * document, and this is that conversion — the same rule, and the same
     * shape, as the C++ `IOProcessorHelper::publishedOrigin` both engines
     * already share.
     *
     * A remote invoke is the case that makes this more than a rename: its child
     * session is stamped with a URI rather than an id, and wrapping a URI in
     * [scxmlLocation] would produce an address naming nothing. An argument that
     * already carries a scheme is therefore passed through — it is already an
     * address.
     */
    fun publishedOrigin(originSessionId: String): String = when {
        originSessionId.isEmpty() -> ""
        originSessionId.contains("://") -> originSessionId
        else -> scxmlLocation(originSessionId)
    }

    /**
     * Entry set for a session.
     *
     * Every processor is filed twice: under the specification's entry name and
     * under the short alias SCXML documents index with. Both keys carry the
     * same location, so the choice of spelling never changes where an event goes.
     *
     * §scxml-C-2-3's entry appears only when [basicHttpAccessUri] is non-empty.
     * Support for that processor is optional and per-deployment, so a session
     * with no inbound endpoint advertises no address rather than one nothing
     * answers on.
     */
    fun build(sessionId: String, basicHttpAccessUri: String = ""): List<IoProcessorDescriptor> {
        val scxmlUri = scxmlLocation(sessionId)
        val descriptors = mutableListOf(
            IoProcessorDescriptor(SCXML_PROCESSOR, scxmlUri),
            IoProcessorDescriptor(SCXML_ALIAS, scxmlUri),
        )
        if (basicHttpAccessUri.isNotEmpty()) {
            descriptors += IoProcessorDescriptor(BASIC_HTTP_PROCESSOR, basicHttpAccessUri)
            descriptors += IoProcessorDescriptor(BASIC_HTTP_ALIAS, basicHttpAccessUri)
        }
        return descriptors
    }

    /** RFC 3986 percent-encoding: unreserved is `A-Za-z0-9-._~`. */
    private fun percentEncode(value: String): String {
        val encoded = StringBuilder(value.length)
        for (byte in value.encodeToByteArray()) {
            val ch = byte.toInt().toChar()
            if (ch.isLetterOrDigit() && ch.code < 0x80 || UNRESERVED.indexOf(ch) >= 0) {
                encoded.append(ch)
            } else {
                encoded.append('%')
                encoded.append(((byte.toInt() shr 4) and 0xF).toString(16).uppercase())
                encoded.append((byte.toInt() and 0xF).toString(16).uppercase())
            }
        }
        return encoded.toString()
    }

    /**
     * Reverses [percentEncode]. A malformed escape is left verbatim rather than
     * dropped: the input is an address a document supplied, and silently
     * rewriting it would turn a bad target into a different valid one.
     */
    private fun percentDecode(value: String): String {
        val bytes = ArrayList<Byte>(value.length)
        var i = 0
        while (i < value.length) {
            val hi = if (value[i] == '%' && i + 2 < value.length) hexNibble(value[i + 1]) else -1
            val lo = if (hi >= 0) hexNibble(value[i + 2]) else -1
            if (hi >= 0 && lo >= 0) {
                bytes += ((hi shl 4) or lo).toByte()
                i += 3
            } else {
                for (b in value[i].toString().encodeToByteArray()) bytes += b
                i += 1
            }
        }
        return bytes.toByteArray().decodeToString()
    }

    private fun hexNibble(c: Char): Int = when (c) {
        in '0'..'9' -> c - '0'
        in 'a'..'f' -> c - 'a' + 10
        in 'A'..'F' -> c - 'A' + 10
        else -> -1
    }
}
