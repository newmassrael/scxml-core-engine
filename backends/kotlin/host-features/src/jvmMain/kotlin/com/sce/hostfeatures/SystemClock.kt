// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.hostfeatures

import java.time.Instant
import java.time.ZoneId
import java.time.ZoneOffset

/** The JVM's clock: UTC from the system clock, the monotonic reading from `System.nanoTime`, the zone the JVM runs in. */
class SystemClock : ClockSource {
    private val origin = System.nanoTime()

    override fun read(): Reading =
        Reading(utcMs = System.currentTimeMillis(), monotonicMs = (System.nanoTime() - origin) / 1_000_000)

    override fun zone(): Zone {
        val id = ZoneId.systemDefault()
        val offset = id.rules.getOffset(Instant.now()).totalSeconds
        // A zone that is only an offset (`+09:00`, `Z`) has no identifier to report.
        val identifier = if (id is ZoneOffset) "" else id.id
        return Zone(identifier, offset)
    }
}
