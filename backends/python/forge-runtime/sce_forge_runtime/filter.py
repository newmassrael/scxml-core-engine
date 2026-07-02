# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

"""Three signal-filter classes matching SCE_FORGE.md Section 4.8.

[`MovingAverage`]: sliding window arithmetic mean.
[`LowPass`]: first-order exponential smoothing.
[`Debounce`]: latch only after N consecutive identical samples.
"""

from typing import Generic, TypeVar

T = TypeVar("T")


class MovingAverage:
    """Sliding-window arithmetic mean."""

    def __init__(self, window: int) -> None:
        if window < 1:
            raise ValueError("window must be >= 1")
        self._window = window
        self._buffer: list[float] = [0.0] * window
        self._index = 0
        self._filled = False

    def update(self, value: float) -> float:
        self._buffer[self._index] = value
        self._index = (self._index + 1) % self._window
        if not self._filled and self._index == 0:
            self._filled = True
        count = self._window if self._filled else self._index
        return sum(self._buffer[:count]) / count

    def reset(self) -> None:
        self._buffer = [0.0] * self._window
        self._index = 0
        self._filled = False


class LowPass:
    """First-order exponential low-pass: y[n] = alpha * x[n] + (1 - alpha) * y[n-1].
    On the first sample, y[0] = x[0] (no warm-up bias toward zero).
    """

    def __init__(self, alpha: float) -> None:
        self._alpha = alpha
        self._state = 0.0
        self._initialized = False

    def update(self, value: float) -> float:
        if not self._initialized:
            self._state = value
            self._initialized = True
        else:
            self._state = self._alpha * value + (1.0 - self._alpha) * self._state
        return self._state

    def reset(self) -> None:
        self._state = 0.0
        self._initialized = False


class Debounce(Generic[T]):
    """Output latches to a new value only after `window` consecutive identical
    samples. Until the buffer fills, the most recent input passes through.
    """

    def __init__(self, window: int) -> None:
        if window < 1:
            raise ValueError("window must be >= 1")
        self._window = window
        self._buffer: list[T] = [None] * window  # type: ignore[list-item]
        self._index = 0
        self._filled = False
        self._output: T | None = None

    def update(self, value: T) -> T:
        self._buffer[self._index] = value
        self._index = (self._index + 1) % self._window
        if not self._filled and self._index == 0:
            self._filled = True

        if self._filled:
            if all(self._buffer[i] == self._buffer[0] for i in range(1, self._window)):
                self._output = self._buffer[0]
        else:
            self._output = value
        # mypy/runtime: _output is set on the first call above, so cast is safe.
        return self._output  # type: ignore[return-value]

    def reset(self) -> None:
        self._buffer = [None] * self._window  # type: ignore[list-item]
        self._index = 0
        self._filled = False
        self._output = None
