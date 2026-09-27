// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.runtime

/**
 * The run-time judgements a `<send>` makes about the values its own arguments
 * evaluated to (§scxml-6.2, §scxml-C-1).
 *
 * Port of the C++ `SendHelper` (`sce/include/common/SendHelper.h`), whose Rust
 * and Go ports are `helpers::send` and `runtime/send.go`. A generated send
 * reads these rather than spelling the rules inline, so a `typeexpr` or a
 * `targetexpr` is judged by the same rule on every backend.
 */
object SendHelper {
    /**
     * Whether this platform delivers through [sendType] (§scxml-6.2): the
     * SCXML Event I/O Processor, named or defaulted, and the Basic HTTP one.
     * A type outside the set is the same error as a type that could not be
     * evaluated.
     */
    fun isSupportedSendType(sendType: String): Boolean =
        sendType.isEmpty() ||
            sendType == IoProcessors.SCXML_PROCESSOR ||
            sendType == IoProcessors.BASIC_HTTP_PROCESSOR

    /**
     * Whether [target] is one this processor cannot address (§scxml-6.2,
     * W3C test194): a target that opens with `!`.
     */
    fun isInvalidTarget(target: String): Boolean = target.startsWith("!")

    /**
     * Whether [target] names no session at all (§scxml-C-1, W3C tests 496 and
     * 521): the empty string, or the text of an undefined value. Such a send
     * raises error.communication.
     */
    fun isUnreachableTarget(target: String): Boolean = target.isEmpty() || target == "undefined"

    /**
     * A `<send>` delay, read as the CSS2 time §scxml-6.2 names, in
     * milliseconds — or `null` when the text is not a time, a bare number
     * included, so the caller raises the argument error rather than choosing
     * a wait.
     *
     * The grammar is ARCHITECTURE.md's "Durations (Single Source of Truth)":
     * surrounding ASCII whitespace aside, a non-negative number (digits with
     * an optional fraction of at least one digit, or a leading `.` and
     * digits; no sign, no exponent) followed directly by `ms` or `s`, either
     * case. The milliseconds are computed in exact decimal and truncated,
     * never through a `Double`, and never exceed [Long.MAX_VALUE].
     * `tests/durations/css2_time.json` holds the cases every engine is
     * measured against.
     */
    fun parseDelayMs(text: String): Long? {
        val s = text.trim { it == ' ' || it == '\t' || it == '\n' || it == '\r' || it == '\u000C' || it == '\u000B' }
        val number: String
        val scale: Long
        when {
            s.length >= 2 && s.endsWith("ms", ignoreCase = true) -> { number = s.dropLast(2); scale = 1L }
            s.length >= 1 && s.endsWith("s", ignoreCase = true) -> { number = s.dropLast(1); scale = 1000L }
            else -> return null
        }
        val point = number.indexOf('.')
        val whole = if (point < 0) number else number.substring(0, point)
        val fraction = if (point < 0) null else number.substring(point + 1)
        val allDigits = { part: String -> part.all { it in '0'..'9' } }
        // A number is digits, digits "." digits, or "." digits: the fraction is
        // never empty, and there is at least one digit somewhere.
        if (!allDigits(whole) || (fraction != null && (fraction.isEmpty() || !allDigits(fraction)))) return null
        if (whole.isEmpty() && fraction == null) return null
        var ms = 0L
        for (c in whole) {
            val digit = (c - '0').toLong()
            if (ms > (Long.MAX_VALUE - digit) / 10) return null
            ms = ms * 10 + digit
        }
        if (ms > Long.MAX_VALUE / scale) return null
        ms *= scale
        // Only the fraction digits that name whole milliseconds count; the
        // rest truncate.
        var place = scale / 10
        for (c in fraction.orEmpty()) {
            if (place == 0L) break
            val add = (c - '0').toLong() * place
            if (ms > Long.MAX_VALUE - add) return null
            ms += add
            place /= 10
        }
        return ms
    }
}
