# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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

The same widths carry the operations that do not fail (docs/adr/0063), which
an author names where they are written (``wrappingSub(now, start)``) and the
generator lowers to ``U32.wrapping_sub(a, b)`` in any forge expression, of an
algorithm (``may-fail`` or not), another kind or a ``sce-static`` machine.
Each computes the exact result and brings it into the
width by its rule: the value congruent to it modulo ``2**W`` (two's complement
for a signed width), or the nearer bound when it lies past one. Neither raises.
"""

from __future__ import annotations

import dataclasses
import enum
import math
import struct


class AlgorithmError(enum.Enum):
    """Why a may-fail algorithm has no value to return."""

    OVERFLOW = "overflow"
    DIVIDE_BY_ZERO = "divide-by-zero"
    # A buffer append past its declared capacity, or a string past the bound its
    # variable declares (``bounded``). This backend's buffers grow past their
    # capacity (SCE_FORGE.md §4.12), so a buffer never reports one; the failure
    # has one name on every backend.
    CAPACITY_EXCEEDED = "capacity-exceeded"
    # A `<sce:require>` precondition that does not hold: an input outside the
    # algorithm's domain.
    PRECONDITION = "precondition"
    # An index below 0 or not below its collection's length.
    OUT_OF_RANGE = "out-of-range"

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
    """The checked, wrapping and saturating operations of one SCE integer
    width, the type that holds ``[lo, hi]``."""

    def __init__(self, lo: int, hi: int) -> None:
        self.lo = lo
        self.hi = hi

    def _fit(self, v: int) -> int:
        if v < self.lo or v > self.hi:
            raise AlgorithmFailure(AlgorithmError.OVERFLOW)
        return v

    def _wrap(self, e: int) -> int:
        """The value of the width congruent to ``e`` modulo ``2**W``. Python's
        ``%`` by a positive modulus is never negative, so the offset from
        ``lo`` lands in ``[0, 2**W)`` whatever the sign of ``e``."""
        return (e - self.lo) % (self.hi - self.lo + 1) + self.lo

    def _saturate(self, e: int) -> int:
        """``e``, or the bound of the width it lies past."""
        return max(self.lo, min(self.hi, e))

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

    def wrapping_add(self, a: int, b: int) -> int:
        return self._wrap(a + b)

    def wrapping_sub(self, a: int, b: int) -> int:
        return self._wrap(a - b)

    def wrapping_mul(self, a: int, b: int) -> int:
        return self._wrap(a * b)

    def wrapping_neg(self, a: int) -> int:
        """``MIN`` for a signed ``MIN``, and ``2**W - a`` for an unsigned
        ``a`` other than 0."""
        return self._wrap(-a)

    def saturating_add(self, a: int, b: int) -> int:
        return self._saturate(a + b)

    def saturating_sub(self, a: int, b: int) -> int:
        return self._saturate(a - b)

    def saturating_mul(self, a: int, b: int) -> int:
        return self._saturate(a * b)

    def saturating_neg(self, a: int) -> int:
        """``MAX`` for a signed ``MIN``. Of an unsigned width it is 0 for every
        input, which the build refuses where it is written (docs/adr/0063,
        section 2); the method follows the definition there all the same."""
        return self._saturate(-a)

    def narrow(self, v: int) -> int:
        """A value stored where this width is declared: the same value, or an
        overflow when the width cannot hold it — never a wrapped one."""
        return self._fit(v)


I8 = _Width(-(2**7), 2**7 - 1)
I16 = _Width(-(2**15), 2**15 - 1)
I32 = _Width(-(2**31), 2**31 - 1)
I64 = _Width(-(2**63), 2**63 - 1)
U8 = _Width(0, 2**8 - 1)
U16 = _Width(0, 2**16 - 1)
U32 = _Width(0, 2**32 - 1)
U64 = _Width(0, 2**64 - 1)


def at(xs, i: int):
    """The element of ``xs`` at ``i``, or an out-of-range failure when ``i``
    is below 0 or not below ``len(xs)``. Python reads a negative index from
    the end, which no other backend does, so it is refused before it reaches
    the subscript."""
    if i < 0 or i >= len(xs):
        raise AlgorithmFailure(AlgorithmError.OUT_OF_RANGE)
    return xs[i]


def found(value):
    """The value a reading found, or an out-of-range failure when it found none
    (``None``): the variant an enum's ``from_underlying`` found for a number,
    which is what ``<alias>.fromNumber(n)`` answers (SCE_FORGE.md §4.12), and
    the value a lookup whose miss is an error found for a key. A number outside
    the enum and a key outside the table fail as an index outside a collection
    does."""
    if value is None:
        raise AlgorithmFailure(AlgorithmError.OUT_OF_RANGE)
    return value


def otherwise(value, default):
    """``value`` when it is present, and what ``default()`` computes when it is
    absent (``None``): ``x ?? d`` of a value that may be absent
    (docs/adr/0055). The generator hands ``d`` as a ``lambda``, so it is
    computed only when ``value`` is absent and a failure of it (an
    :class:`AlgorithmFailure`) is raised only then; ``value`` is read once,
    as the argument it is. Presence is ``is not None`` and never a truth
    test: a present ``0``, ``''`` or ``False`` is the value."""
    if value is not None:
        return value
    return default()


def copied(record):
    """``record``, a payload of an event schema (a dataclass of fixed-width,
    text and enum fields), as a copy of its own, and an absent value (``None``)
    as it is: what a record put into an optional record is, so that a field
    written through the one is not written through the other
    (docs/adr/0055). A record is a value in every language of this contract, and
    Python's payload class is a mutable object a name would otherwise share."""
    if record is None:
        return None
    return dataclasses.replace(record)


def to_f32(value: float) -> float:
    """``value`` rounded to the nearest binary32, as the double that holds it
    exactly — what a 32-bit real is on the backends that have one. Python's
    ``float`` is a binary64, so an operation on 32-bit reals is rounded to a
    single where it is made (the generator writes the call), and a value past
    the single's range is the infinity it is there, where ``struct`` refuses it.
    A NaN stays one, an infinity stays infinite."""
    try:
        return struct.unpack("<f", struct.pack("<f", value))[0]
    except OverflowError:
        return math.inf if value > 0 else -math.inf


def bounded(value, capacity: int):
    """``value`` when it holds no more than ``capacity`` bytes — the bound a
    string variable of a ``sce-static`` machine declares, counted in UTF-8 bytes,
    and the bound a byte string declares, counted in bytes — and a capacity
    failure otherwise. A Python string is counted in characters, which no other
    backend counts, so it is encoded to be counted in bytes as they count it."""
    size = len(value.encode("utf-8")) if isinstance(value, str) else len(value)
    if size > capacity:
        raise AlgorithmFailure(AlgorithmError.CAPACITY_EXCEEDED)
    return value
