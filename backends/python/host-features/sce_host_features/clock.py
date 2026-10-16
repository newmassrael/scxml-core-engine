# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The clock (docs/SCE_HOST_FEATURES.md, section 2)."""

from __future__ import annotations

import os
import time
from dataclasses import dataclass
from typing import TYPE_CHECKING, Optional, Protocol

from .records import Outcome, failure, record

if TYPE_CHECKING:
    from .host import ReferenceHost


@dataclass
class Reading:
    """Both readings of one instant."""

    #: Milliseconds since 1970-01-01T00:00:00Z, POSIX time.
    utc_ms: int
    #: Milliseconds since an origin fixed for the life of the process.
    monotonic_ms: int


@dataclass
class Zone:
    """The time zone a host reads local time in."""

    #: An IANA identifier, or empty when only an offset is known.
    id: str
    #: Seconds east of UTC in force now.
    utc_offset_s: int


class ClockSource(Protocol):
    """Where a host's time comes from. A source that cannot be read answers
    `None`, which the host reports as `unavailable`."""

    def read(self) -> Optional[Reading]: ...

    def zone(self) -> Optional[Zone]: ...


class SystemClock:
    """The operating system's clock."""

    def __init__(self) -> None:
        self._origin = time.monotonic_ns()

    def read(self) -> Optional[Reading]:
        return Reading(
            utc_ms=time.time_ns() // 1_000_000,
            monotonic_ms=(time.monotonic_ns() - self._origin) // 1_000_000,
        )

    def zone(self) -> Optional[Zone]:
        """The offset in force now, and the identifier the TZ variable names when
        it names a zone, which the C library has no other way to report."""
        identifier = os.environ.get("TZ", "").lstrip(":")
        if "/" not in identifier and identifier != "UTC":
            identifier = ""
        return Zone(id=identifier, utc_offset_s=time.localtime().tm_gmtoff or 0)


class ScriptedClock:
    """A clock a scenario sets: it reads what it was told. A field that is `None`
    is a clock that cannot be read."""

    def __init__(self) -> None:
        self.reading: Optional[Reading] = Reading(0, 0)
        self.zone_now: Optional[Zone] = Zone("", 0)

    def read(self) -> Optional[Reading]:
        return self.reading

    def zone(self) -> Optional[Zone]:
        return self.zone_now


def now(host: "ReferenceHost") -> Outcome:
    reading = host.clock.read()
    if reading is None:
        return failure("sce-host-clock-now", "unavailable")
    # No answer goes below one already given.
    if host.monotonic_floor is None or reading.monotonic_ms > host.monotonic_floor:
        host.monotonic_floor = reading.monotonic_ms
    return record({"utc_ms": str(reading.utc_ms), "monotonic_ms": str(host.monotonic_floor)})


def zone(host: "ReferenceHost") -> Outcome:
    answered = host.clock.zone()
    if answered is None:
        return failure("sce-host-clock-zone", "unavailable")
    return record({"zone_id": answered.id, "utc_offset_s": answered.utc_offset_s})
