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
}
