# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""A one-shot test timer (docs/SCE_TEST_PROFILE.md §4.7, P7): its status, its
expiry instant on the logical clock, and its snapshot (ES 201 873-4 §8.3.2.4).

Cross-language siblings: ``sce_rust_runtime::test::Timer`` (Rust),
``SCE::Test::Timer`` (C++), ``com.sce.runtime.test.Timer`` (Kotlin),
``testprofile.Timer`` (Go).
"""

from __future__ import annotations

from enum import Enum
from typing import Optional

from .port import Duration, Instant


class TimerStatus(Enum):
    """``STATUS`` of a timer binding (-4 §8.3.2.4)."""

    #: A timer that is not running.
    IDLE = "idle"
    #: A timer running, to expire at its instant.
    RUNNING = "running"
    #: A timer whose instant has come and that no timeout branch has taken yet.
    TIMEOUT = "timeout"


class Timer:
    """One timer binding of a component. A new timer is idle."""

    __slots__ = ("_status", "_expires_at", "_snapshot_timed_out")

    def __init__(self) -> None:
        self._status = TimerStatus.IDLE
        self._expires_at: Instant = 0
        self._snapshot_timed_out = False

    @property
    def status(self) -> TimerStatus:
        """The timer's status."""
        return self._status

    def start(self, now: Instant, duration: Duration) -> None:
        """``start`` (-4 §9.48): running, to expire ``duration`` after ``now``.
        Starting a running timer restarts it (-1 §23.2)."""
        self._status = TimerStatus.RUNNING
        self._expires_at = now + duration

    def stop(self) -> None:
        """``stop`` (-4 §9.52): idle. A timer that is not running is left as it
        is."""
        if self._status is TimerStatus.RUNNING:
            self._status = TimerStatus.IDLE

    def update(self, now: Instant) -> None:
        """The update phase (§3.2 step 2): a running timer whose instant has
        come times out (-4 §8.6.1.2 a)."""
        if self._status is TimerStatus.RUNNING and self._expires_at <= now:
            self._status = TimerStatus.TIMEOUT

    def expiry(self) -> Optional[Instant]:
        """The instant the timer will time out at while it runs (§3.3), and
        ``None`` otherwise."""
        if self._status is not TimerStatus.RUNNING:
            return None
        return self._expires_at

    def take_snapshot(self) -> None:
        """Take ``SNAP-STATUS`` in a round's snapshot step."""
        self._snapshot_timed_out = self._status is TimerStatus.TIMEOUT

    @property
    def snapshot_timed_out(self) -> bool:
        """Whether the last snapshot saw the timer timed out."""
        return self._snapshot_timed_out

    def timeout_taken(self) -> None:
        """A timeout branch taken on the snapshot (-4 §9.55, P4 step 1)."""
        self._status = TimerStatus.IDLE
        self._snapshot_timed_out = False

    def __repr__(self) -> str:
        return f"Timer(status={self._status.value}, expires_at={self._expires_at})"
