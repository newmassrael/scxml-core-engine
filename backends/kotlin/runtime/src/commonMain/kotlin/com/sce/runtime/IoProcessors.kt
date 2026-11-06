// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.runtime

/**
 * One entry of `_ioprocessors`: the key it is filed under, and the address
 * external entities use to reach this session through that processor.
 */
data class IoProcessorDescriptor(val name: String, val location: String)

/**
 * An event's origin and sender as a saved state holds them back: what
 * [IoProcessors.readStoredOrigin] reads a format-1 or format-2 `origin` and
 * `origintype` as, and what a format-3 record holds as written (docs/adr/0090,
 * section 7).
 */
data class StoredOrigin(val origin: EventOrigin?, val sender: String?)

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
     * What every [scxmlLocation] begins with, before the session id. Alone it
     * names no session ([sessionIdFromScxmlLocation] reads it as none).
     */
    const val SCXML_LOCATION_PREFIX = "sce://scxml/"

    /**
     * Address that reaches this session over the SCXML Event I/O Processor.
     *
     * §scxml-C-1 leaves the transport platform-specific, so the address is an
     * SCE-scheme URI naming the session. The session id is percent-encoded
     * because it is not constrained to URI-safe characters.
     */
    fun scxmlLocation(sessionId: String): String = SCXML_LOCATION_PREFIX + percentEncode(sessionId)

    /**
     * Session id an SCXML Event I/O Processor location names, or `""`.
     *
     * The inverse of [scxmlLocation], kept beside it so the two spellings of
     * one address cannot drift apart. §scxml-C-1 requires the location a
     * session publishes to be usable as a `<send>` target, which only holds if
     * something can read a session back out of it.
     */
    fun sessionIdFromScxmlLocation(uri: String): String {
        if (uri.length <= SCXML_LOCATION_PREFIX.length || !uri.startsWith(SCXML_LOCATION_PREFIX)) return ""
        return percentDecode(uri.substring(SCXML_LOCATION_PREFIX.length))
    }

    /**
     * The origin the SCXML Event I/O Processor writes for an event the session
     * [senderSessionId] sends (docs/adr/0090, section 2).
     *
     * The origin of an event raised in the receiving session matches the
     * location the sending session publishes, so the pair is this
     * processor's type and [scxmlLocation] of the sender — the one function that
     * also writes the session's `_ioprocessors` entry ([build]). The sender's id
     * itself travels beside the pair, as [EventMetadata.sender], for the lookups
     * that find a session by it.
     */
    fun scxmlOrigin(senderSessionId: String): EventOrigin =
        EventOrigin(SCXML_PROCESSOR, scxmlLocation(senderSessionId))

    /**
     * [scxmlOrigin] of a sender that may be absent: a machine that holds no
     * session id has no address to publish, so an event it sends carries no
     * origin rather than the location of an empty id.
     */
    fun scxmlOriginOf(senderSessionId: String?): EventOrigin? =
        if (senderSessionId.isNullOrEmpty()) null else scxmlOrigin(senderSessionId)

    /**
     * The `origin` text a format-1 saved state writes for an event's [origin]
     * and [sender] (docs/adr/0090, section 7): the sending session's id for an
     * event a session sent, the address otherwise, and empty for none — the
     * rule format 2 wrote by. The `origintype` beside it is the pair's
     * processor. Format 3 holds the pair itself.
     */
    fun storedOriginText(origin: EventOrigin?, sender: String?): String =
        if (sender.isNullOrEmpty()) origin?.address ?: "" else sender

    /**
     * The origin and sender a stored [origin] and [originType] are read as
     * (docs/adr/0090, section 7).
     *
     * An empty [origin] is none whatever type was stored beside it: an empty
     * address answers nothing, and format 2 wrote this processor's type beside
     * it for events no processor stamped. An empty [originType] beside an
     * address is this processor's, which is what format 2 meant by it. An SCXML
     * location (which one backend stored in place of the id) is that session's
     * pair, with the id it names as the sender; a value without a scheme read
     * with this processor's type is a session's id, read the same way. Anything
     * else is the pair as stored, with no sender. So neither field of a pair
     * read back is ever empty.
     */
    fun readStoredOrigin(origin: String, originType: String): StoredOrigin {
        if (origin.isEmpty()) return StoredOrigin(null, null)
        val located = sessionIdFromScxmlLocation(origin)
        if (located.isNotEmpty()) return StoredOrigin(scxmlOrigin(located), located)
        val processor = originType.ifEmpty { SCXML_PROCESSOR }
        val bareId = !origin.contains("://")
        if (bareId && processor == SCXML_PROCESSOR) return StoredOrigin(scxmlOrigin(origin), origin)
        return StoredOrigin(EventOrigin(processor, origin), null)
    }

    /**
     * The sender a stored waiting send's [origin] names (docs/adr/0090,
     * section 7): a waiting send is always a session's, so a bare id is that id
     * and an SCXML location names it. `null` for an empty text.
     */
    fun readStoredSender(origin: String): String? {
        val located = sessionIdFromScxmlLocation(origin)
        return located.ifEmpty { origin }.ifEmpty { null }
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
