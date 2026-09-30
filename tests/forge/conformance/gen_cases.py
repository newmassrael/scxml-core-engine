#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""Generated conformance cases, from an independent model (E11).

Each fixture below has a MODEL: a few lines of Python over exact integers that
say what the fixture's document means. The model generates inputs — every edge
of every type the fixture touches, and a seeded spread between them — and
computes each input's answer. The cases are written into
`numerical_reference.json`, beside this file, where the six backends' harnesses
already read them, so no backend needs to know the cases are generated.

A case no backend disagrees on proves little; a case one backend answers
differently is a divergence the hand-written cases did not reach, and the model
says which backend is wrong. That is the whole of the method, and the reason the
model is Python: its integers have no width, so it cannot share a backend's
overflow.

Two properties make the output trustworthy:

* The model is checked against every hand-written case of the same fixture
  before it generates anything. Those cases are held by all six backends, so a
  model that disagrees with one is wrong, not the case.
* The generator is deterministic. It draws from SplitMix64, written out below,
  rather than from `random`, whose stream is an implementation detail; the same
  seed gives the same file on every Python. The `forge-python` gate runs
  `--check`, so a committed file that this script would not write is refused.

Usage, from the repository root:

    python3 tests/forge/conformance/gen_cases.py            # rewrite the file
    python3 tests/forge/conformance/gen_cases.py --check    # fail if it would change

A generated case is one line whose note starts with `fuzz`. Regeneration
replaces exactly those lines and never touches a hand-written one.

To add a fixture: write its model, an argument generator aimed at each
operation's own failure edge (a uniform draw almost never lands where an integer
operation starts to fail), and an entry in FIXTURES with a seed no other entry
uses. The model must reproduce the fixture's hand cases first; the script says
which one it does not.

First run, 2026-09-30: 1200 cases over `algorithm_checked_arith`,
`algorithm_checked_call` and `algorithm_checked_narrow`; Rust, Python, Go,
Kotlin, C++ and C11 all agreed with the model on every one. The integer
contract (SCE_FORGE.md 3.4.1) holds across the six backends for these
operations, and a wrong expectation in one generated case fails each lane
that was tried (Rust, Python, Go, C11), so the cases are exercised.

Second run, 2026-09-30: the eight civil-date fixtures of `stdlib/time`, modelled
on Python's `datetime` (cross-checked by `validate_calendar`, which runs first).
It found `add_months` computing `year * 12` at int32 width before widening the
sum, so a year past 178956970 failed `overflow` although the result year fits —
a defect in the document, not in a backend: Python, Go, C11 and C++ all failed
the same 103 cases, and all six agreed with the model once the document widened
its inputs into int64 locals as `days_from_civil` does. A wrong expectation in a
generated record, boolean or `fails` case fails Python and C11.
"""

from __future__ import annotations

import calendar
import datetime
import json
import re
import sys
from pathlib import Path

REFERENCE = Path("tests/forge/conformance/numerical_reference.json")

I32_MIN, I32_MAX = -(2**31), 2**31 - 1
I64_MIN, I64_MAX = -(2**63), 2**63 - 1
U8_MAX = 2**8 - 1
U32_MAX = 2**32 - 1

#: What a generated case's note starts with. Regeneration finds its own lines
#: by it, so nothing else may start a note this way.
MARK = "fuzz"


class SplitMix64:
    """Sebastiano Vigna's SplitMix64, so a seed means one stream everywhere."""

    def __init__(self, seed: int) -> None:
        self.state = seed & (2**64 - 1)

    def next(self) -> int:
        self.state = (self.state + 0x9E3779B97F4A7C15) & (2**64 - 1)
        z = self.state
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & (2**64 - 1)
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & (2**64 - 1)
        return z ^ (z >> 31)

    def below(self, bound: int) -> int:
        """A value in [0, bound). The slight bias of a modulus is irrelevant
        to a spread of inputs and keeps the stream easy to re-implement."""
        return self.next() % bound

    def between(self, low: int, high: int) -> int:
        """A value in [low, high]."""
        return low + self.below(high - low + 1)

    def pick(self, items):
        return items[self.below(len(items))]


def fits(value: int, low: int, high: int) -> bool:
    return low <= value <= high


def truncated_quotient(a: int, b: int) -> int:
    """a / b toward zero, for b != 0 — C's `/`, and what the contract says."""
    quotient = abs(a) // abs(b)
    return quotient if (a >= 0) == (b > 0) else -quotient


def truncated_remainder(a: int, b: int) -> int:
    """a % b with the dividend's sign, for b != 0 — C's `%`."""
    remainder = abs(a) % abs(b)
    return remainder if a >= 0 else -remainder


# A model answers with ("ok", value) or ("fails", contract_name).


def arith(a: int, b: int, op: int):
    """algorithm_checked_arith — SCE_FORGE.md 3.4.1, one operation per `op`."""

    def i32(value: int):
        return ("ok", value) if fits(value, I32_MIN, I32_MAX) else ("fails", "overflow")

    if op == 0:
        return i32(a + b)
    if op == 1:
        return i32(a - b)
    if op == 2:
        return i32(a * b)
    if op == 3:
        if b == 0:
            return ("fails", "divide-by-zero")
        if a == I32_MIN and b == -1:
            return ("fails", "overflow")
        return i32(truncated_quotient(a, b))
    if op == 4:
        if b == 0:
            return ("fails", "divide-by-zero")
        # Mathematically 0, and still a failure: the operation the hardware
        # traps on, which the contract makes uniform.
        if a == I32_MIN and b == -1:
            return ("fails", "overflow")
        return i32(truncated_remainder(a, b))
    if op == 5:
        return i32(-a)
    if op == 6:
        # `op - 7` in a uint8: op is 6 here, so it is always below zero.
        return ("fails", "overflow")
    return ("ok", 0)


def checked_call(a: int, b: int, op: int):
    """algorithm_checked_call — arith called three ways, every failure passed on."""
    first = arith(a, b, op)
    if first[0] == "fails":
        return first
    s = arith(a, b, 0)
    if s[0] == "fails":
        return s
    square = arith(s[1], s[1], 2)
    if square[0] == "fails":
        return square
    if square[1] > 100:
        return ("ok", 0)
    return arith(s[1], 1, 0)


def checked_narrow(x: int, y: int):
    """algorithm_checked_narrow — y into a uint8, the int64 sum into an int32."""
    if not fits(y, 0, U8_MAX):
        return ("fails", "overflow")
    wide = x + y
    if not fits(wide, I64_MIN, I64_MAX):
        return ("fails", "overflow")
    if not fits(wide, I32_MIN, I32_MAX):
        return ("fails", "overflow")
    return ("ok", wide)


# ── The civil calendar (sce:std/time) ─────────────────────────────────────
#
# The proleptic Gregorian calendar, the way Python's own library does it, so
# the model shares no method with the documents it judges. Two things keep it
# honest: `calendar` answers leap years and month lengths for any integer year,
# and the ordinal arithmetic below — days before a year, days before a month —
# is the textbook one, checked against `datetime.date` over its whole range by
# `validate_calendar` before a case is written. Outside 1..9999 `datetime`
# cannot answer, and the same arithmetic is what the model stands on.

EPOCH_ORDINAL = datetime.date(1970, 1, 1).toordinal()  # 719163
DAYS_BEFORE_MONTH = [0, 0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334]


def days_in_month_of(year: int, month: int) -> int:
    return calendar.monthrange(year, month)[1]


def ordinal(year: int, month: int, day: int) -> int:
    """Days since 0000-12-31, for a date that exists: 0001-01-01 is 1."""
    y = year - 1
    before_year = y * 365 + y // 4 - y // 100 + y // 400
    leap_day = 1 if month > 2 and calendar.isleap(year) else 0
    return before_year + DAYS_BEFORE_MONTH[month] + leap_day + day


def civil_of(n: int):
    """The (year, month, day) of ordinal `n`, by finding its year and then its
    month — a search over `ordinal`, not the era arithmetic the documents use."""
    year = n * 400 // 146097 + 1
    while ordinal(year + 1, 1, 1) <= n:
        year += 1
    while ordinal(year, 1, 1) > n:
        year -= 1
    month = 12
    while ordinal(year, month, 1) > n:
        month -= 1
    return year, month, n - ordinal(year, month, 1) + 1


def exists(year: int, month: int, day: int) -> bool:
    return 1 <= month <= 12 and 1 <= day <= days_in_month_of(year, month)


def is_leap_year(year: int):
    return ("ok", calendar.isleap(year))


def days_in_month(year: int, month: int):
    if not 1 <= month <= 12:
        return ("fails", "precondition")
    return ("ok", days_in_month_of(year, month))


def days_from_civil(year: int, month: int, day: int):
    if not exists(year, month, day):
        return ("fails", "precondition")
    return ("ok", ordinal(year, month, day) - EPOCH_ORDINAL)


def civil_from_days(days: int):
    year, month, day = civil_of(days + EPOCH_ORDINAL)
    if not fits(year, I32_MIN, I32_MAX):
        return ("fails", "overflow")
    return ("ok", {"year": year, "month": month, "day": day})


def weekday_from_days(days: int):
    # 1970-01-01 was a Thursday, and 0 is Sunday; Python's `%` floors, so a
    # negative count needs no folding.
    return ("ok", (days + 4) % 7)


def add_months(year: int, month: int, day: int, n: int):
    if not exists(year, month, day):
        return ("fails", "precondition")
    new_year, index = divmod(year * 12 + (month - 1) + n, 12)
    if not fits(new_year, I32_MIN, I32_MAX):
        return ("fails", "overflow")
    new_month = index + 1
    return (
        "ok",
        {
            "year": new_year,
            "month": new_month,
            "day": min(day, days_in_month_of(new_year, new_month)),
        },
    )


def second_of_day(seconds: int):
    return ("ok", seconds % 86400)


def days_from_epoch_seconds(seconds: int):
    return ("ok", seconds // 86400)


def validate_calendar() -> None:
    """Hold the model's calendar to `datetime.date` over its whole range, so a
    mistake in the arithmetic above is found here and not in a backend."""
    first = datetime.date.min.toordinal()
    last = datetime.date.max.toordinal()
    # Every day of every year would take seconds; every 61st day touches each
    # month and weekday position many times, and the year ends are explicit.
    samples = set(range(first, last + 1, 61))
    for year in range(1, 10000):
        samples.update(
            (
                ordinal(year, 1, 1), ordinal(year, 2, 28), ordinal(year, 3, 1),
                ordinal(year, 12, 31),
            )
        )
    for n in samples:
        date = datetime.date.fromordinal(n)
        if civil_of(n) != (date.year, date.month, date.day):
            raise SystemExit(f"the model's calendar disagrees with datetime at ordinal {n}")
        if ordinal(date.year, date.month, date.day) != n:
            raise SystemExit(f"the model's ordinal disagrees with datetime at {date}")
        days = n - EPOCH_ORDINAL
        if weekday_from_days(days)[1] != (date.weekday() + 1) % 7:
            raise SystemExit(f"the model's weekday disagrees with datetime at {date}")
        if days_in_month_of(date.year, date.month) != calendar.monthrange(date.year, date.month)[1]:
            raise SystemExit(f"the model's month length disagrees at {date}")


# The edges of each type the fixtures touch: where an operation starts to
# fail, on both sides of it, and the values a width's arithmetic is most often
# wrong at (zero, one, minus one, the square-root of 2^31, the sign bit).
I32_EDGES = sorted(
    {
        I32_MIN, I32_MIN + 1, I32_MIN + 2, -65537, -65536, -46341, -46340, -46339,
        -256, -255, -128, -127, -8, -7, -3, -2, -1, 0, 1, 2, 3, 7, 8, 127, 128,
        255, 256, 46339, 46340, 46341, 65535, 65536, I32_MAX - 2, I32_MAX - 1, I32_MAX,
    }
)
I64_EDGES = sorted(
    {
        I64_MIN, I64_MIN + 1, I32_MIN - 1, I32_MIN, I32_MIN + 1, -1, 0, 1,
        I32_MAX - 1, I32_MAX, I32_MAX + 1, I64_MAX - 1, I64_MAX,
    }
)
U8_EDGES = [0, 1, 2, 3, 4, 5, 6, 7, 8, 127, 128, 254, 255]
U32_EDGES = [0, 1, 255, 256, 257, 65535, 65536, 2**31, U32_MAX - 1, U32_MAX]


def i32_value(rng: SplitMix64) -> int:
    """An int32: an edge, a value near zero, or anywhere in the range."""
    kind = rng.below(10)
    if kind < 5:
        return rng.pick(I32_EDGES)
    if kind < 8:
        return rng.between(-300, 300)
    return rng.between(I32_MIN, I32_MAX)


def clamp32(value: int) -> int:
    return max(I32_MIN, min(I32_MAX, value))


def near(rng: SplitMix64, target: int, spread: int = 2) -> int:
    """`target`, give or take a few: the two sides of an edge, both drawn."""
    return clamp32(target + rng.between(-spread, spread))


def op_value(rng: SplitMix64) -> int:
    """An `op`: mostly the seven the document defines, sometimes any uint8."""
    if rng.below(10) < 8:
        return rng.between(0, 6)
    return rng.pick(U8_EDGES) if rng.below(2) else rng.between(0, U8_MAX)


def arith_args(rng: SplitMix64):
    """One operation at a time, and half the time aimed at that operation's
    own edge — a uniform draw almost never lands where an int32 operation
    starts to fail, which is where the backends part."""
    op = op_value(rng)
    aimed = rng.below(2) == 0
    a, b = i32_value(rng), i32_value(rng)
    if aimed and op in (0, 1):
        # a + b and a - b fail when the exact result leaves int32: pick b so
        # the result lands within two of either end.
        end = I32_MAX if rng.below(2) else I32_MIN
        a = i32_value(rng)
        b = near(rng, end - a if op == 0 else a - end)
    elif aimed and op == 2:
        # a * b fails past 2^31: b about the quotient of the limit by a.
        a = rng.pick([v for v in I32_EDGES if v not in (0,)])
        limit = I32_MAX if rng.below(2) else I32_MIN
        b = near(rng, truncated_quotient(limit, a), 2)
    elif aimed and op in (3, 4):
        kind = rng.below(4)
        if kind == 0:
            b = 0
        elif kind == 1:
            a, b = I32_MIN, -1
        elif kind == 2:
            b = rng.pick([-1, 1, 2, -2, 3, -3, 7, -7])
            a = i32_value(rng)
        else:
            b = i32_value(rng) or 1
    elif aimed and op == 5:
        a = near(rng, I32_MIN if rng.below(2) else I32_MAX, 2)
    return [a, b, op]


def call_args(rng: SplitMix64):
    """The call fails in three places, so the inputs aim at three edges: the
    callee's own (the `op`), the sum a + b, and the square of that sum — about
    100 where the result changes, and about 46340 where it overflows."""
    op = op_value(rng)
    kind = rng.below(5)
    if kind == 0:
        # s = a + b about the +-100 branch edge (s * s = 100 at +-10).
        s = rng.between(-13, 13)
    elif kind == 1:
        s = near(rng, 46340 if rng.below(2) else -46340, 2)
    elif kind == 2:
        s = i32_value(rng)
    else:
        s = None
    if s is None:
        a, b = i32_value(rng), i32_value(rng)
    else:
        a = i32_value(rng) if rng.below(2) else rng.between(-30, 30)
        b = s - a
        if not fits(b, I32_MIN, I32_MAX):
            a, b = s, 0
    return [a, b, op]


def narrow_args(rng: SplitMix64):
    """The two stores have two edges each: y against a uint8, and the sum
    against int32 (and, far out, int64)."""
    kind = rng.below(10)
    y = rng.between(0, 300) if rng.below(10) < 7 else (
        rng.pick(U32_EDGES) if rng.below(2) else rng.between(0, U32_MAX)
    )
    if kind < 4:
        # The sum about an end of int32, whatever y the local allows.
        yy = min(y, U8_MAX)
        end = I32_MAX if rng.below(2) else I32_MIN
        x = end - yy + rng.between(-2, 2)
        if rng.below(3) == 0:
            y = yy
    elif kind < 6:
        x = rng.pick(I64_EDGES)
    elif kind < 8:
        x = rng.between(I32_MIN - 300, I32_MAX + 300)
    else:
        x = rng.between(I64_MIN, I64_MAX)
    return [x, y]


YEAR_EDGES = sorted(
    {
        I32_MIN, I32_MIN + 1, -1000, -401, -400, -101, -100, -5, -4, -1, 0, 1, 3, 4,
        99, 100, 101, 399, 400, 401, 1582, 1600, 1699, 1700, 1800, 1899, 1900, 1901,
        1969, 1970, 1971, 1999, 2000, 2001, 2023, 2024, 2025, 2026, 2027, 2028, 2100,
        2399, 2400, 9999, 10000, 12345, 65535, 65536, 1_000_000, I32_MAX - 1, I32_MAX,
    }
)


def year_value(rng: SplitMix64) -> int:
    """A year: an edge (century rules, the epoch, the int32 ends), a year a
    calendar is used in, a wider one, or anywhere in int32."""
    kind = rng.below(20)
    if kind < 10:
        return rng.pick(YEAR_EDGES)
    if kind < 15:
        return rng.between(-2000, 4000)
    if kind < 17:
        return rng.between(-100_000, 100_000)
    return rng.between(I32_MIN, I32_MAX)


def date_args(rng: SplitMix64):
    """A year, a month and a day, most often a date that exists and often one
    step from not existing: the day past a month's end, day 0, month 0 and 13."""
    year = year_value(rng)
    kind = rng.below(20)
    if kind < 12:
        month = rng.between(1, 12)
        length = days_in_month_of(year, month)
        day = rng.pick([1, 2, length - 1, length, rng.between(1, length)])
    elif kind < 17:
        month = rng.between(1, 12)
        length = days_in_month_of(year, month)
        day = rng.pick([0, length + 1, 29, 30, 31, 32, U8_MAX])
    elif kind < 19:
        month = rng.pick([0, 13, 14, 255, rng.between(0, U8_MAX)])
        day = rng.between(0, 31)
    else:
        month, day = rng.between(0, U8_MAX), rng.between(0, U8_MAX)
    return year, month, day


def leap_args(rng: SplitMix64):
    return [year_value(rng)]


def month_length_args(rng: SplitMix64):
    month = rng.between(1, 12) if rng.below(10) < 8 else rng.pick([0, 13, 14, 255])
    return [year_value(rng), month]


def civil_args(rng: SplitMix64):
    return list(date_args(rng))


def day_count_edges():
    """Day counts where the calendar changes shape: each century-rule year's
    first and last day, the epoch, and the two ends of int32 years."""
    days = {0, 1, -1, 146097, -146097, 719162, -719162, 719163, -719163, 719468, -719468}
    for year in (-400, -101, -100, -4, -1, 0, 1, 4, 100, 400, 1600, 1900, 1970, 2000, 2100):
        first = ordinal(year, 1, 1) - EPOCH_ORDINAL
        days.update((first - 1, first, first + 1))
    for year in (I32_MIN, I32_MAX):
        first = ordinal(year, 1, 1) - EPOCH_ORDINAL
        last = ordinal(year, 12, 31) - EPOCH_ORDINAL
        days.update((first - 1, first, last, last + 1))
    days.update((I64_MIN, I64_MIN + 1, I64_MAX - 1, I64_MAX, 2**53, -(2**53)))
    return sorted(days)


DAY_COUNT_EDGES = day_count_edges()


def day_count_args(rng: SplitMix64):
    kind = rng.below(10)
    if kind < 4:
        days = rng.pick(DAY_COUNT_EDGES)
    elif kind < 7:
        days = rng.between(-1_000_000, 1_000_000)
    elif kind < 9:
        days = rng.between(-1_000_000_000_000, 1_000_000_000_000)
    else:
        days = rng.between(I64_MIN, I64_MAX)
    return [days]


def add_months_args(rng: SplitMix64):
    year, month, day = date_args(rng)
    kind = rng.below(10)
    if kind < 4:
        n = rng.pick([0, 1, -1, 11, 12, 13, -11, -12, -13, 120, -120, 1200, -1200])
    elif kind < 6:
        n = rng.between(-300, 300)
    elif kind < 8:
        # Close to the int32 ends of the year, where the sum leaves the range.
        year = I32_MAX - rng.between(0, 2) if rng.below(2) else I32_MIN + rng.between(0, 2)
        n = rng.between(0, 30) if year > 0 else -rng.between(0, 30)
        month = rng.between(1, 12)
        day = rng.between(1, days_in_month_of(year, month))
    else:
        n = rng.between(I32_MIN, I32_MAX)
    return [year, month, day, n]


def seconds_args(rng: SplitMix64):
    kind = rng.below(10)
    if kind < 4:
        seconds = rng.pick(
            [0, 1, -1, 86399, 86400, 86401, -86399, -86400, -86401, 2**31, -(2**31), 2**32,
             I64_MIN, I64_MIN + 1, I64_MAX - 1, I64_MAX]
        )
    elif kind < 7:
        seconds = rng.between(-10_000_000, 10_000_000)
    elif kind < 9:
        seconds = rng.between(-(10**12), 10**12)
    else:
        seconds = rng.between(I64_MIN, I64_MAX)
    return [seconds]


#: fixture -> (model, argument generator, how many cases, seed).
#: A seed is per fixture and never reused, so adding a case to one fixture does
#: not move another's.
FIXTURES = {
    "algorithm_checked_arith": (lambda args: arith(*args), arith_args, 600, 0xE110_0001),
    "algorithm_checked_call": (lambda args: checked_call(*args), call_args, 300, 0xE110_0002),
    "algorithm_checked_narrow": (lambda args: checked_narrow(*args), narrow_args, 300, 0xE110_0003),
    "is_leap_year": (lambda args: is_leap_year(*args), leap_args, 150, 0xE110_0004),
    "days_in_month": (lambda args: days_in_month(*args), month_length_args, 200, 0xE110_0005),
    "days_from_civil": (lambda args: days_from_civil(*args), civil_args, 400, 0xE110_0006),
    "civil_from_days": (lambda args: civil_from_days(*args), day_count_args, 400, 0xE110_0007),
    "weekday_from_days": (lambda args: weekday_from_days(*args), day_count_args, 200, 0xE110_0008),
    "add_months": (lambda args: add_months(*args), add_months_args, 400, 0xE110_0009),
    "second_of_day": (lambda args: second_of_day(*args), seconds_args, 150, 0xE110_000A),
    "days_from_epoch_seconds": (
        lambda args: days_from_epoch_seconds(*args),
        seconds_args,
        150,
        0xE110_000B,
    ),
}


def render(value) -> str:
    """A value the way the hand-written cases write it: a record as
    `{ "year": 1970, ... }`, a boolean as `true`, a number as itself."""
    if isinstance(value, dict):
        members = ", ".join(f'"{key}": {render(member)}' for key, member in value.items())
        return "{ " + members + " }"
    return json.dumps(value)


def case_line(args, answer) -> str:
    kind, value = answer
    tail = f'"expected": {render(value)}' if kind == "ok" else f'"fails": "{value}"'
    return f'        {{ "args": {json.dumps(args)}, {tail}, "note": "{MARK}" }}'


def section(text: str, fixture: str):
    """(start, end) of `fixture`'s `cases` array contents: from just after `[`
    to just before the `]` that closes it."""
    anchor = text.index(f'"fixture": "{fixture}"')
    opening = text.index('"cases": [', anchor) + len('"cases": [')
    closing = text.index("\n      ]", opening)
    return opening, closing


HAND_CASE = re.compile(
    r'^\s*\{ "args": (\[.*?\]), (?:"expected": (.+?)|"fails": "(.+?)")(?:, "note": .*)? \},?$'
)


def hand_cases(body: str):
    """The hand-written lines of a `cases` body, and how many generated lines
    it held — which regeneration drops."""
    hand, generated = [], 0
    for line in body.splitlines():
        if not line.strip():
            continue
        if f'"note": "{MARK}' in line:
            generated += 1
            continue
        hand.append(line)
    return hand, generated


def verify_model(fixture: str, model, lines) -> int:
    """Hold the model to every hand-written case of `fixture`."""
    checked = 0
    for line in lines:
        match = HAND_CASE.match(line)
        if not match:
            raise SystemExit(f"{fixture}: cannot read the hand case {line.strip()!r}")
        args = json.loads(match.group(1))
        answer = model(args)
        expected = match.group(2)
        fails = match.group(3)
        if fails is not None:
            ok = answer == ("fails", fails)
        else:
            ok = answer[0] == "ok" and answer[1] == json.loads(expected)
        if not ok:
            raise SystemExit(
                f"{fixture}: the model answers {answer} for {args}, but the hand case "
                f"says {line.strip()} — the model is wrong, since every backend holds the case"
            )
        checked += 1
    return checked


def regenerate(text: str):
    report = []
    for fixture, (model, draw, count, seed) in FIXTURES.items():
        start, end = section(text, fixture)
        hand, dropped = hand_cases(text[start:end])
        checked = verify_model(fixture, model, hand)
        seen = {tuple(json.loads(HAND_CASE.match(line).group(1))) for line in hand}
        rng = SplitMix64(seed)
        made, guard = [], 0
        while len(made) < count:
            guard += 1
            if guard > count * 50:
                raise SystemExit(f"{fixture}: cannot draw {count} distinct inputs")
            args = draw(rng)
            key = tuple(args)
            if key in seen:
                continue
            seen.add(key)
            made.append(case_line(args, model(args)))
        lines = [line.rstrip(",") for line in hand] + made
        body = "\n" + ",\n".join(lines)
        text = text[:start] + body + text[end:]
        kinds: dict[str, int] = {}
        for line in made:
            fails = re.search(r'"fails": "([^"]+)"', line)
            name = fails.group(1) if fails else "ok"
            kinds[name] = kinds.get(name, 0) + 1
        report.append((fixture, checked, dropped, len(made), dict(sorted(kinds.items()))))
    return text, report


def main(argv) -> int:
    check = "--check" in argv
    validate_calendar()
    original = REFERENCE.read_text(encoding="utf-8")
    json.loads(original)
    text, report = regenerate(original)
    json.loads(text)
    for fixture, checked, dropped, made, kinds in report:
        print(
            f"{fixture}: model agrees with {checked} hand cases; "
            f"{made} generated (was {dropped}): {kinds}"
        )
    if text == original:
        print("numerical_reference.json: unchanged")
        return 0
    if check:
        print(
            "numerical_reference.json: out of date — run "
            "tests/forge/conformance/gen_cases.py"
        )
        return 1
    REFERENCE.write_text(text, encoding="utf-8")
    print("numerical_reference.json: rewritten")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
