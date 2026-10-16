// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package com.sce.hostfeatures

/** Both readings of one instant. */
data class Reading(
    /** Milliseconds since 1970-01-01T00:00:00Z, POSIX time. */
    val utcMs: Long,
    /** Milliseconds since an origin fixed for the life of the process. */
    val monotonicMs: Long,
)

/** The time zone a host reads local time in. */
data class Zone(
    /** An IANA identifier, or empty when only an offset is known. */
    val id: String,
    /** Seconds east of UTC in force now. */
    val utcOffsetS: Int,
)

/** Where a host's time comes from. A source that cannot be read answers `null`, which the host reports as `unavailable`. */
interface ClockSource {
    fun read(): Reading?

    fun zone(): Zone?
}

/** A clock a scenario sets: it reads what it was told. A field that is `null` is a clock that cannot be read. */
class ScriptedClock : ClockSource {
    var reading: Reading? = Reading(0, 0)
    var zoneNow: Zone? = Zone("", 0)

    override fun read(): Reading? = reading

    override fun zone(): Zone? = zoneNow
}

internal fun ReferenceHost.clockNow(): Outcome {
    val reading = clock.read() ?: return failure("sce-host-clock-now", "unavailable")
    // No answer goes below one already given.
    val floor = monotonicFloor
    if (floor == null || reading.monotonicMs > floor) monotonicFloor = reading.monotonicMs
    return record("utc_ms" to reading.utcMs.toString(), "monotonic_ms" to monotonicFloor.toString())
}

internal fun ReferenceHost.clockZone(): Outcome {
    val zone = clock.zone() ?: return failure("sce-host-clock-zone", "unavailable")
    return record("zone_id" to zone.id, "utc_offset_s" to zone.utcOffsetS)
}
