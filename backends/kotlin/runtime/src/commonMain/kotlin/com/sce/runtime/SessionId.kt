// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.runtime

import kotlin.uuid.ExperimentalUuidApi
import kotlin.uuid.Uuid

/**
 * The id of a root session, the value of the system variable `_sessionid`
 * (docs/adr/0045, "Questions the owner answered" 2).
 *
 * `_sessionid` is bound when a session is loaded and kept until it ends, and it
 * is an XML Nmtoken: another session addresses this one as `#_scxml_<id>`, and
 * every child's id is its parent's and the id the child was started under,
 * `<parent>.<invokeid>`. So a root's id is never empty. The host may give it
 * when it creates the machine ([refusalOf] tells it when what it gives is not an
 * Nmtoken, rather than having it replaced), and where it gives none the runtime
 * draws one ([drawn]), a UUID version 4 in lowercase hexadecimal with hyphens,
 * the one spelling every backend draws, so no reader can tell which backend
 * drew it. A restore binds the saved id instead of either, since the session
 * carries on.
 *
 * Cross-language sibling: `sce_rust_runtime::helpers::session_id`.
 */
object SessionId {
    /**
     * Why [id] is not an id a session may have, or `null` when it is one: an XML
     * 1.0 Nmtoken, one name character or more (production `Nmtoken`, fifth
     * edition).
     */
    fun refusalOf(id: String): String? {
        if (id.isEmpty()) {
            return "the session id is empty, and a session id is one name character or more " +
                "(an XML Nmtoken)"
        }
        var at = 0
        while (at < id.length) {
            val point = id.codePointAt(at)
            if (!isNameChar(point)) {
                return "the session id holds U+${point.toString(16).uppercase().padStart(4, '0')}, which " +
                    "is not a name character: a session id is an XML Nmtoken (letters, digits, '.', '-', " +
                    "'_', ':')"
            }
            at += if (point > 0xFFFF) 2 else 1
        }
        return null
    }

    /** Whether [id] is an XML 1.0 Nmtoken. */
    fun isNmtoken(id: String): Boolean = refusalOf(id) == null

    /**
     * A session id drawn from the platform's randomness, a UUID version 4
     * (RFC 9562, section 5.4) in lowercase hexadecimal in groups of 8, 4, 4, 4
     * and 12, for a session whose host gave none.
     */
    @OptIn(ExperimentalUuidApi::class)
    fun drawn(): String = Uuid.random().toString()

    /** XML 1.0 `NameStartChar`. */
    private fun isNameStartChar(c: Int): Boolean =
        c == ':'.code || c in 'A'.code..'Z'.code || c == '_'.code || c in 'a'.code..'z'.code ||
            c in 0xC0..0xD6 || c in 0xD8..0xF6 || c in 0xF8..0x2FF || c in 0x370..0x37D ||
            c in 0x37F..0x1FFF || c in 0x200C..0x200D || c in 0x2070..0x218F || c in 0x2C00..0x2FEF ||
            c in 0x3001..0xD7FF || c in 0xF900..0xFDCF || c in 0xFDF0..0xFFFD || c in 0x10000..0xEFFFF

    /** XML 1.0 `NameChar`. */
    private fun isNameChar(c: Int): Boolean =
        isNameStartChar(c) || c == '-'.code || c == '.'.code || c in '0'.code..'9'.code || c == 0xB7 ||
            c in 0x300..0x36F || c in 0x203F..0x2040
}

/** The Unicode code point at [index], reading a surrogate pair as one (common code has no `codePointAt`). */
private fun String.codePointAt(index: Int): Int {
    val high = this[index]
    if (high.isHighSurrogate() && index + 1 < length && this[index + 1].isLowSurrogate()) {
        return 0x10000 + ((high.code - 0xD800) shl 10) + (this[index + 1].code - 0xDC00)
    }
    return high.code
}
