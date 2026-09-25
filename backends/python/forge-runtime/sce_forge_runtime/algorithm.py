# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The integer arithmetic contract's runtime half — SCE_FORGE.md §3.4.1.

An algorithm that declares ``<sce:return may-fail="true">`` raises
:class:`AlgorithmFailure` in place of a value, and the generator lowers each
of its integer ``+ - * / %`` and unary ``-`` to a method of the declared
width (``I32.add(a, b)``). Python's integers do not overflow, so without this
an ``int32`` sum past 2**31 - 1 would simply be a larger number — the one
backend that agrees with no other. A method computes the operation exactly,
then holds the result to its width: an overflow (a signed ``MIN / -1`` or
``MIN % -1`` included, which is one) and a division by zero raise.

The exception is Python's failure channel: it leaves the algorithm and reaches
the caller by itself, so the generated body needs nothing after a statement.
"""

from __future__ import annotations

import enum


class AlgorithmError(enum.Enum):
    """Why a may-fail algorithm has no value to return."""

    OVERFLOW = "overflow"
    DIVIDE_BY_ZERO = "divide-by-zero"
    # A buffer append past its declared capacity. This backend's buffers grow
    # past their capacity (SCE_FORGE.md §4.12), so it never reports one; the
    # case exists because the failure has one name on every backend.
    CAPACITY_EXCEEDED = "capacity-exceeded"

    @property
    def contract_name(self) -> str:
        """The failure's name in the contract — the spelling every backend shares."""
        return self.value


class AlgorithmFailure(Exception):
    """Raised by a may-fail algorithm in place of a value."""

    def __init__(self, error: AlgorithmError) -> None:
        super().__init__(error.contract_name)
        self.error = error


class _Width:
    """The checked operations of one SCE integer width."""

    def __init__(self, lo: int, hi: int) -> None:
        self.lo = lo
        self.hi = hi

    def _fit(self, v: int) -> int:
        if v < self.lo or v > self.hi:
            raise AlgorithmFailure(AlgorithmError.OVERFLOW)
        return v

    def add(self, a: int, b: int) -> int:
        return self._fit(a + b)

    def sub(self, a: int, b: int) -> int:
        return self._fit(a - b)

    def mul(self, a: int, b: int) -> int:
        return self._fit(a * b)

    def div(self, a: int, b: int) -> int:
        """Truncated toward zero, as every backend divides — Python's ``//``
        rounds toward -inf, so the quotient is taken from the magnitudes."""
        if b == 0:
            raise AlgorithmFailure(AlgorithmError.DIVIDE_BY_ZERO)
        q = abs(a) // abs(b)
        return self._fit(q if (a < 0) == (b < 0) else -q)

    def rem(self, a: int, b: int) -> int:
        """The sign of the dividend. A remainder whose quotient overflows has
        no value either: ``MIN % -1`` fails as ``MIN / -1`` does."""
        q = self.div(a, b)
        return a - q * b

    def neg(self, a: int) -> int:
        return self._fit(-a)


I8 = _Width(-(2**7), 2**7 - 1)
I16 = _Width(-(2**15), 2**15 - 1)
I32 = _Width(-(2**31), 2**31 - 1)
I64 = _Width(-(2**63), 2**63 - 1)
U8 = _Width(0, 2**8 - 1)
U16 = _Width(0, 2**16 - 1)
U32 = _Width(0, 2**32 - 1)
U64 = _Width(0, 2**64 - 1)
