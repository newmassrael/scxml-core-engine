// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.runtime

/**
 * The significant digits of a positive finite double, and where the decimal
 * point stands: the value is `0.<digits> * 10^point`, and [digits] has no
 * leading or trailing zero.
 */
internal class ShortestDigits(val digits: String, val point: Int)

/**
 * The fewest digits that read back as [positive], the nearest to it when
 * several fewest-digit decimals do (the digits ECMA-262 `Number::toString`
 * writes; see [Json.numberText]).
 *
 * A platform supplies this because `Double.toString` is not it: on the JVM
 * before JDK 19 it sometimes writes more digits than the value needs
 * (JDK-4511638), and this module builds on JDK 17.
 */
internal expect fun shortestDigits(positive: Double): ShortestDigits
