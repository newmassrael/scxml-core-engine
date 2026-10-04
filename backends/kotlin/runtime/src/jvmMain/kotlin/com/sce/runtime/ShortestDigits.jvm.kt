// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.runtime

import java.math.BigDecimal
import java.math.MathContext
import java.math.RoundingMode

/**
 * The value's exact decimal expansion rounded to one digit, then two, and so
 * on up to seventeen, taking the first length at which a rounding reads back
 * as the same double. The rounding to nearest is tried first; the two
 * neighbours of it are tried too, because a double at a power of two has a
 * gap below it half the size of the gap above, and the nearest decimal can
 * fall outside the interval that reads back while a farther one does not.
 */
internal actual fun shortestDigits(positive: Double): ShortestDigits {
    val exact = BigDecimal(positive)
    val modes = arrayOf(RoundingMode.HALF_EVEN, RoundingMode.FLOOR, RoundingMode.CEILING)
    for (precision in 1..17) {
        for (mode in modes) {
            val rounded = exact.round(MathContext(precision, mode))
            // `toString().toDouble()` is the correctly rounded read that
            // `Double.parseDouble` is.
            if (rounded.toString().toDouble() == positive) {
                val trimmed = rounded.stripTrailingZeros()
                val digits = trimmed.unscaledValue().toString()
                return ShortestDigits(digits, digits.length - trimmed.scale())
            }
        }
    }
    // Seventeen digits always read back, so this is not reached.
    error("no seventeen-digit decimal reads back as $positive")
}
