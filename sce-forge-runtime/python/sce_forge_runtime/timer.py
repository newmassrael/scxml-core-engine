# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

"""Timer HAL — abstract base class that the user implements once per platform.
See SCE_FORGE.md Section 4.10.

Python's runtime model has no embedded constraints (heap, GIL, callable
closures are all natural), so the interface uses a plain `Callable[[], None]`
rather than the C-style function-pointer + context pattern that the C++ and
Rust runtimes use. The behavioural contract is identical: the user supplies a
concrete `Timer` implementation that schedules callbacks against the host
platform's clock and event loop.
"""

from abc import ABC, abstractmethod
from typing import Callable

TimerCallback = Callable[[], None]


class Timer(ABC):
    """Platform timer interface.

    Implementations must be re-entrant safe: a previously scheduled callback
    must not run after `cancel()` returns. Timer object lifetime is owned by
    the user; generated code holds references and never destroys them.
    """

    @abstractmethod
    def start_periodic(self, interval_ms: int, callback: TimerCallback) -> None: ...

    @abstractmethod
    def start_one_shot(self, delay_ms: int, callback: TimerCallback) -> None: ...

    @abstractmethod
    def cancel(self) -> None: ...
