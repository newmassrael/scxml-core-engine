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

Third run, 2026-09-30: the sync documents — `hlc_compare`, `lww_classify`,
`orset_union`, `hlc_text`, `gcra_admit` (overflow at every checked product and
sum) and `murmur3_32` (written as the reference writes it, checked against its 24
vectors) — 1800 cases whose arguments are records, lists and bytes, with uint64
node ids up to 2^64 - 1. All six backends agreed with the model on every one; no
document or backend disagreed. What it did find is that the Kotlin lane had not
run them: the harness reads this file by path and Gradle did not know it, so a
change to the cases alone left `jvmTest` UP-TO-DATE. The build now declares it.
A wrong expectation in a generated case of each new fixture fails Kotlin, C++
and Go.

Fourth run, 2026-09-30: the retry and sync-time documents — `retry_next_backoff`
(an integer meets a real as a real, so a wait past 2^53 is rounded to the nearest
double before it is multiplied and compared), `retry_jittered`, `retry_exhausted`
and `sync_retry_at` — 1350 cases. The backends agreed with the model on all of
them, but Rust and C11 failed the `0.9999999999999999` multiplier cases: their
harnesses parse this file with serde_json, whose default float parser can land a
unit in the last place off the nearest double and read that multiplier as 1.0.
Python, Go, Kotlin and C++ parse it correctly. The defect was in the harness,
not in a backend; both `Cargo.toml` files now enable `float_roundtrip`. JSON has
no NaN or infinity, so those multipliers stay with the hand cases.
"""

from __future__ import annotations

import calendar
import datetime
import json
import math
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


# Sync: hybrid logical clocks, the last-writer-wins cell, the observed-remove
# set, a rate limiter and the hash that keys a change log. Their arguments are
# records, lists and bytes, so the generators below build values rather than
# integers, and keep the field order the hand-written cases use.

U16_MAX = 2**16 - 1
U64_MAX = 2**64 - 1
TEXT_WALL_MAX = 253_402_300_799_999  # 9999-12-31T23:59:59.999Z


def hlc(wall: int, counter: int, node: int) -> dict:
    return {"wallTime": wall, "counter": counter, "nodeId": node}


def hlc_key(stamp: dict):
    return (stamp["wallTime"], stamp["counter"], stamp["nodeId"])


def hlc_compare(a: dict, b: dict):
    """sce:std/merge/hlc_compare — lexicographic on (wall time, counter, node),
    which is what a tuple comparison is."""
    ka, kb = hlc_key(a), hlc_key(b)
    return ("ok", (ka > kb) - (ka < kb))


def lww_classify(logged: list, msg: dict):
    """sce:std/merge/lww_classify — 2 a duplicate, 1 when a later stamp is held,
    else 0."""
    held = [hlc_key(stamp) for stamp in logged]
    key = hlc_key(msg)
    if key in held:
        return ("ok", 2)
    return ("ok", 1 if any(other > key for other in held) else 0)


def entry(element: int, wall: int, counter: int, node: int) -> dict:
    return {"element": element, "wallTime": wall, "counter": counter, "nodeId": node}


def orset_union(a: list, b: list):
    """sce:std/merge/orset_union — a, then b's entries that a does not hold."""
    return ("ok", a + [other for other in b if other not in a])


def hlc_text(stamp: dict):
    """sce:std/merge/hlc_text — the 46 ASCII bytes of a stamp's canonical text."""
    wall, counter, node = stamp["wallTime"], stamp["counter"], stamp["nodeId"]
    if not (0 <= wall <= TEXT_WALL_MAX and counter <= U16_MAX):
        return ("fails", "precondition")
    instant = datetime.datetime(1970, 1, 1) + datetime.timedelta(milliseconds=wall)
    text = f"{instant:%Y-%m-%dT%H:%M:%S}.{wall % 1000:03d}Z-{counter:04X}-{node:016x}"
    return ("ok", list(text.encode("ascii")))


def gcra_admit(tat: int, now: int, cost: int, interval: int, burst: int):
    """sce:std/rate/gcra_admit — the generic cell rate algorithm in int64. Every
    product and sum the document writes is checked, so a value that leaves int64
    anywhere in them fails `overflow`, whether or not the answer would have."""
    if not (cost >= 1 and interval >= 1 and burst >= 1 and cost <= burst):
        return ("fails", "precondition")
    start = tat if tat > now else now
    arrival = start + cost * interval
    limit = now + burst * interval
    for value in (cost * interval, arrival, burst * interval, limit):
        if not fits(value, I64_MIN, I64_MAX):
            return ("fails", "overflow")
    if arrival <= limit:
        return ("ok", {"admitted": True, "tat": arrival, "retryAfter": 0})
    wait = arrival - limit
    if not fits(wait, I64_MIN, I64_MAX):
        return ("fails", "overflow")
    return ("ok", {"admitted": False, "tat": tat, "retryAfter": wait})


def murmur3_32(data: list, seed: int):
    """sce:std/hash/murmur3_32 — MurmurHash3 x86_32, written as the reference
    writes it (Appleby, MurmurHash3.cpp), not as the document's uint64 masks do."""
    mask = 0xFFFFFFFF

    def rotl(value: int, shift: int) -> int:
        return ((value << shift) | (value >> (32 - shift))) & mask

    def scramble(k: int) -> int:
        return (rotl((k * 0xCC9E2D51) & mask, 15) * 0x1B873593) & mask

    h = seed
    blocks = len(data) // 4 * 4
    for offset in range(0, blocks, 4):
        k = int.from_bytes(bytes(data[offset : offset + 4]), "little")
        h = (rotl(h ^ scramble(k), 13) * 5 + 0xE6546B64) & mask
    tail = data[blocks:]
    if tail:
        h ^= scramble(int.from_bytes(bytes(tail), "little"))
    h ^= len(data)
    h = ((h ^ (h >> 16)) * 0x85EBCA6B) & mask
    h = ((h ^ (h >> 13)) * 0xC2B2AE35) & mask
    return ("ok", h ^ (h >> 16))


def retry_next_backoff(prev_ms: int, multiplier: float, max_ms: int):
    """sce:std/mesh/retry_next_backoff. An integer meets a real as a real
    (SCE_FORGE.md 3.4: "an integer as a real"), so both integers are converted
    to float64 — to the nearest, ties to even — before the product and the
    comparison; Python's own `float < int` would compare exactly instead."""
    if not (multiplier >= 1.0 and prev_ms > 0 and max_ms >= prev_ms):
        return ("fails", "precondition")
    grown = float(prev_ms) * multiplier
    if grown < float(max_ms):
        return ("ok", math.floor(grown))
    return ("ok", max_ms)


def retry_jittered(base_ms: int, jitter_pct: int, draw: int):
    """sce:std/mesh/retry_jittered — each step is checked at int64, so a band
    whose `2 * delta + 1` or whose top leaves int64 fails `overflow`."""
    if not (base_ms > 0 and 0 <= jitter_pct <= 100 and draw >= 0):
        return ("fails", "precondition")
    delta = base_ms // 100 * jitter_pct + base_ms % 100 * jitter_pct // 100
    width = 2 * delta + 1
    if not fits(width, I64_MIN, I64_MAX):
        return ("fails", "overflow")
    waited = base_ms - delta + draw % width
    if not fits(waited, I64_MIN, I64_MAX):
        return ("fails", "overflow")
    return ("ok", 1 if waited < 1 else waited)


def retry_exhausted(attempts: int, max_retries: int, retryable: bool):
    """sce:std/mesh/retry_exhausted."""
    if max_retries == 0:
        return ("fails", "precondition")
    return ("ok", (not retryable) or attempts > max_retries)


def sync_retry_at(previous: int, kind: int, status: int, now: int, retry_after: int):
    """sce:std/sync/sync_retry_at — the later of `previous` and the time this
    failure asks for; a 503 asks for its Retry-After, a 502 for fifteen minutes."""
    if not 1 <= kind <= 9:
        return ("fails", "precondition")
    if kind == 4:
        if not 300 <= status <= 599:
            return ("fails", "precondition")
    elif status != 0:
        return ("fails", "precondition")
    if not (previous >= 0 and now >= 0 and retry_after >= 0):
        return ("fails", "precondition")
    asked = 0
    if kind == 4 and status == 503 and retry_after > 0:
        asked = now + retry_after
    if kind == 4 and status == 502:
        asked = now + 900
    if not fits(asked, I64_MIN, I64_MAX):
        return ("fails", "overflow")
    return ("ok", max(asked, previous))


def epoch_ms(year, month, day, hour=0, minute=0, second=0, milli=0) -> int:
    instant = datetime.datetime(year, month, day, hour, minute, second)
    return (instant - datetime.datetime(1970, 1, 1)) // datetime.timedelta(milliseconds=1) + milli


#: Wall times where the text changes shape: a day, a leap day, a century rule,
#: 2^31 seconds, the last millisecond of year 9999 — each and its neighbours.
WALL_TEXT_EDGES = sorted(
    {
        edge + step
        for edge in (
            epoch_ms(1970, 1, 1),
            epoch_ms(1999, 12, 31, 23, 59, 59, 999),
            epoch_ms(2000, 2, 29),
            epoch_ms(2000, 3, 1),
            epoch_ms(2024, 2, 29, 12, 34, 56, 789),
            epoch_ms(2038, 1, 19, 3, 14, 7),
            epoch_ms(2100, 2, 28, 23, 59, 59, 999),
            epoch_ms(2100, 3, 1),
            epoch_ms(2026, 9, 27, 10),
            epoch_ms(9999, 12, 31, 23, 59, 59, 999),
        )
        for step in (-1, 0, 1)
    }
)
WALL_EDGES = sorted(
    {I64_MIN, I64_MIN + 1, -86_400_000, -1, 0, 1, 999, 1000, 86_399_999, 86_400_000}
    | set(WALL_TEXT_EDGES)
    | {2**53, I64_MAX - 1, I64_MAX}
)
COUNTER_EDGES = [0, 1, 2, 255, 256, U16_MAX - 1, U16_MAX, U16_MAX + 1, U32_MAX - 1, U32_MAX]
NODE_EDGES = [0, 1, 2, 255, 2**32 - 1, 2**32, 2**53, 2**63 - 1, 2**63, U64_MAX - 1, U64_MAX]
ELEMENT_EDGES = [0, 1, 2, 7, 2**63, U64_MAX]
REALISTIC_WALL = 1_790_503_200_000


def stamp_value(rng: SplitMix64) -> dict:
    """A stamp: its wall time near a real clock or at an edge, its counter and
    node at an edge or small, so two stamps often tie on a field."""
    kind = rng.below(10)
    if kind < 4:
        wall = REALISTIC_WALL + rng.between(-1000, 1000)
    elif kind < 8:
        wall = rng.pick(WALL_EDGES)
    else:
        wall = rng.between(I64_MIN, I64_MAX)
    counter = rng.pick(COUNTER_EDGES) if rng.below(3) == 0 else rng.between(0, 5)
    node = rng.pick(NODE_EDGES) if rng.below(3) == 0 else rng.between(0, 5)
    return hlc(wall, counter, node)


def bump(rng: SplitMix64, value: int, low: int, high: int, edges) -> int:
    """`value` moved a little, to an edge, or anywhere in [low, high]."""
    kind = rng.below(4)
    if kind == 0:
        return min(value + 1, high)
    if kind == 1:
        return max(value - 1, low)
    if kind == 2:
        return rng.pick([edge for edge in edges if low <= edge <= high])
    return rng.between(low, high)


def near_stamp(rng: SplitMix64, base: dict) -> dict:
    """A stamp that differs from `base` in a random subset of its fields, which
    is how two stamps come to tie on the first and differ on a later one."""
    wall, counter, node = base["wallTime"], base["counter"], base["nodeId"]
    changes = rng.below(8)
    if changes & 1:
        wall = bump(rng, wall, I64_MIN, I64_MAX, WALL_EDGES)
    if changes & 2:
        counter = bump(rng, counter, 0, U32_MAX, COUNTER_EDGES)
    if changes & 4:
        node = bump(rng, node, 0, U64_MAX, NODE_EDGES)
    return hlc(wall, counter, node)


def compare_args(rng: SplitMix64):
    a = stamp_value(rng)
    kind = rng.below(10)
    b = a if kind == 0 else (near_stamp(rng, a) if kind < 8 else stamp_value(rng))
    return [a, b]


def shuffled(rng: SplitMix64, items: list) -> list:
    """Fisher-Yates over the generator's own stream."""
    items = list(items)
    for index in range(len(items) - 1, 0, -1):
        other = rng.below(index + 1)
        items[index], items[other] = items[other], items[index]
    return items


def lww_args(rng: SplitMix64):
    msg = stamp_value(rng)
    logged = []
    for _ in range(rng.pick([0, 1, 1, 2, 2, 3, 3, 4, 5, 6])):
        other = near_stamp(rng, msg) if rng.below(4) else stamp_value(rng)
        if other not in logged and other != msg:
            logged.append(other)
    kind = rng.below(5)
    if kind == 0:
        logged.insert(rng.below(len(logged) + 1), msg)
    return [shuffled(rng, logged), msg]


def entry_near(rng: SplitMix64, base: dict) -> dict:
    """An entry sharing some of `base`'s fields: the same element added twice,
    or the same tag on another element, are the cases a union must tell apart."""
    element, wall, counter, node = base["element"], base["wallTime"], base["counter"], base["nodeId"]
    changes = rng.below(16)
    if changes & 1:
        element = bump(rng, element, 0, U64_MAX, ELEMENT_EDGES)
    if changes & 2:
        wall = bump(rng, wall, I64_MIN, I64_MAX, WALL_EDGES)
    if changes & 4:
        counter = bump(rng, counter, 0, U32_MAX, COUNTER_EDGES)
    if changes & 8:
        node = bump(rng, node, 0, U64_MAX, NODE_EDGES)
    return entry(element, wall, counter, node)


def union_args(rng: SplitMix64):
    stamp = stamp_value(rng)
    first = entry(rng.pick(ELEMENT_EDGES), stamp["wallTime"], stamp["counter"], stamp["nodeId"])
    pool = [first]
    for _ in range(rng.between(1, 8)):
        candidate = entry_near(rng, rng.pick(pool))
        if candidate not in pool:
            pool.append(candidate)

    def sample():
        return shuffled(rng, pool)[: rng.between(0, len(pool))]

    return [sample(), sample()]


def text_args(rng: SplitMix64):
    kind = rng.below(20)
    if kind < 6:
        wall = rng.pick(WALL_TEXT_EDGES)
    elif kind < 13:
        wall = rng.between(0, TEXT_WALL_MAX)
    elif kind < 15:
        wall = REALISTIC_WALL + rng.between(-10**9, 10**9)
    elif kind < 17:
        wall = rng.pick([-1, -2, I64_MIN, TEXT_WALL_MAX + 1, TEXT_WALL_MAX + 1000, I64_MAX])
    else:
        wall = rng.between(I64_MIN, I64_MAX)
    counter = rng.pick(COUNTER_EDGES) if rng.below(2) == 0 else rng.between(0, U16_MAX)
    node = rng.pick(NODE_EDGES) if rng.below(2) == 0 else rng.between(0, U64_MAX)
    return [hlc(wall, counter, node)]


I64_SCALES = [1, 2, 3, 1000, 131072, 2**31, 2**32, 2**53, 2**62, I64_MAX // 2, I64_MAX]


def gcra_args(rng: SplitMix64):
    """Mostly the limiter a service would run (millisecond clocks, intervals of
    a few thousand, small costs), and often a product at int64's edge."""
    kind = rng.below(10)
    if kind < 5:
        now = REALISTIC_WALL + rng.between(-100_000, 100_000)
        interval = rng.between(1, 100_000)
        burst = rng.between(1, 50)
        cost = rng.between(1, burst)
        tat = now + rng.between(-burst * interval, burst * interval * 2)
    elif kind < 7:
        now = rng.pick([0, 1, -1, REALISTIC_WALL, I64_MAX - 1, I64_MAX, I64_MIN, I64_MIN + 1])
        interval = rng.pick(I64_SCALES)
        burst = rng.pick([1, 2, 3, 100, U32_MAX - 1, U32_MAX])
        cost = rng.between(1, burst)
        tat = rng.pick([I64_MIN, 0, now, I64_MAX - 1, I64_MAX])
    elif kind < 9:
        # cost * interval and burst * interval straddling 2^63.
        interval = rng.between(1, 2**32)
        burst = rng.between(1, U32_MAX)
        limit_interval = I64_MAX // burst
        interval = max(1, limit_interval + rng.between(-2, 2)) if rng.below(2) else interval
        cost = rng.between(1, burst)
        now = rng.pick([0, REALISTIC_WALL, -REALISTIC_WALL, I64_MAX // 2])
        tat = rng.pick([0, now, I64_MAX // 2, I64_MIN // 2])
    else:
        now = rng.between(I64_MIN, I64_MAX)
        tat = rng.between(I64_MIN, I64_MAX)
        interval = rng.between(-2, 2**40)
        burst = rng.between(0, 10)
        cost = rng.between(0, 12)
    return [tat, now, cost, interval, burst]


DATA_LENGTH_EDGES = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 11, 12, 13, 15, 16, 17, 31, 32, 33, 63, 64, 65]
SEED_EDGES = [0, 1, 2, 0x9747B28C, 2**31, U32_MAX - 1, U32_MAX]


def murmur_args(rng: SplitMix64):
    length = rng.pick(DATA_LENGTH_EDGES) if rng.below(3) else rng.between(0, 200)
    style = rng.below(4)
    if style == 0:
        data = [rng.between(0, 255) for _ in range(length)]
    elif style == 1:
        data = [rng.pick([0, 0x80, 0xFF, 0x7F, 1]) for _ in range(length)]
    elif style == 2:
        data = list(("hlc-" + str(rng.below(10**12))).encode("ascii"))[:length] + [97] * max(
            0, length - 16
        )
    else:
        data = [rng.between(32, 126) for _ in range(length)]
    seed = rng.pick(SEED_EDGES) if rng.below(2) else rng.between(0, U32_MAX)
    return [data, seed]


MULTIPLIER_EDGES = [
    1.0, 1.0000000000000002, 1.5, 2.0, 3.0, 10.0, 1e3, 1e15, 1e18, 1e19, 1e300,
    1.7976931348623157e308,
]


def multiplier_value(rng: SplitMix64) -> float:
    """A multiplier of at least 1.0 — a real a deploy file holds — or, now and
    then, one below it, which is a precondition. JSON has no NaN or infinity, so
    those are left to the hand cases."""
    kind = rng.below(10)
    if kind < 3:
        return rng.pick(MULTIPLIER_EDGES)
    if kind < 7:
        return 1.0 + rng.below(400) / 100
    if kind < 9:
        return rng.pick([0.5, 0.9999999999999999, 0.0, -1.0, 0.1])
    return 1.0 + rng.below(10**6) / 10**6


def backoff_args(rng: SplitMix64):
    """Wait, multiplier and cap; often a product close to the cap, where the
    product is compared as a real, and past 2^53, where an integer stops being
    exact as one."""
    kind = rng.below(10)
    if kind < 4:
        prev = rng.between(1, 10_000)
        cap = rng.between(prev, prev * 50)
    elif kind < 6:
        prev = rng.between(1, 2**20)
        cap = prev + rng.between(0, 2**21)
    elif kind < 8:
        prev = rng.pick([2**53 - 1, 2**53, 2**53 + 1, 2**53 + 2, 2**62, I64_MAX - 1, I64_MAX])
        cap = rng.pick([prev, prev, I64_MAX, I64_MAX - 1, 2**53 + 1, 2**62 + 1])
        cap = max(cap, prev)
    elif kind < 9:
        prev = rng.pick([0, -1, 1, 2, I64_MIN])
        cap = rng.pick([0, 1, 10, -5, I64_MAX])
    else:
        prev = rng.between(1, I64_MAX)
        cap = rng.between(1, I64_MAX)
    multiplier = multiplier_value(rng)
    if kind < 6 and rng.below(3) == 0 and prev > 0:
        # A multiplier that puts the product within one unit of the cap.
        multiplier = max(1.0, cap / prev * (1 + rng.pick([-(2**-52), 0.0, 2**-52])))
    return [prev, multiplier, cap]


def jitter_args(rng: SplitMix64):
    kind = rng.below(10)
    if kind < 5:
        base = rng.between(1, 100_000)
        pct = rng.pick([0, 1, 10, 25, 50, 99, 100, rng.between(0, 100)])
        draw = rng.between(0, 4 * base)
    elif kind < 7:
        base = rng.pick([1, 2, 99, 100, 101, 199, 200, 1000])
        pct = rng.between(0, 100)
        draw = rng.pick([0, 1, 2 * base, 2 * base + 1, I64_MAX, I64_MAX - 1, rng.between(0, I64_MAX)])
    elif kind < 9:
        # The band's width and top near int64's end.
        base = rng.pick([I64_MAX, I64_MAX - 1, I64_MAX // 2, I64_MAX // 2 + 1, 2**62, 2**62 + 1])
        pct = rng.pick([0, 1, 50, 99, 100])
        draw = rng.pick([0, 1, I64_MAX, rng.between(0, I64_MAX)])
    else:
        base = rng.pick([0, -1, 1, I64_MIN, rng.between(-100, 100)])
        pct = rng.pick([-1, 0, 101, 100, I64_MIN, I64_MAX])
        draw = rng.pick([-1, 0, 1, I64_MIN, 5])
    return [base, pct, draw]


def exhausted_args(rng: SplitMix64):
    edges = [0, 1, 2, 3, 4, 255, 65535, U32_MAX - 1, U32_MAX]
    attempts = rng.pick(edges) if rng.below(2) else rng.between(0, 12)
    limit = rng.pick(edges) if rng.below(2) else rng.between(0, 8)
    if rng.below(3) == 0:
        attempts = min(max(limit + rng.pick([-1, 0, 1]), 0), U32_MAX)
    return [attempts, limit, rng.below(2) == 0]


FAILURE_STATUSES = [300, 301, 400, 401, 403, 404, 408, 409, 429, 500, 501, 502, 503, 504, 599]


def sync_retry_args(rng: SplitMix64):
    """A failure of each kind — kind 4 is an HTTP status, every other has none —
    mostly a 502 or 503, at a clock near a real one or at int64's end."""
    kind = rng.pick([1, 2, 3, 4, 4, 4, 4, 4, 4, 5, 6, 7, 8, 9]) if rng.below(10) else rng.between(0, 12)
    if kind == 4:
        status = rng.pick([502, 503, 503, 502]) if rng.below(3) else rng.pick(FAILURE_STATUSES)
    else:
        status = 0
    if rng.below(12) == 0:
        status = rng.pick([0, 299, 600, -1, 503]) if kind == 4 else rng.pick([1, 503, -1])
    clock = rng.below(10)
    if clock < 6:
        now = 1_790_503_200 + rng.between(-100_000, 100_000)
        previous = rng.pick([0, 0, 1, now - 1, now, now + rng.between(-5000, 5000), now + 900, now + 3600])
        retry_after = rng.pick([0, 1, 60, 3600, 86_400, rng.between(0, 100_000)])
    elif clock < 9:
        now = rng.pick([0, 1, I64_MAX - 900, I64_MAX - 899, I64_MAX - 1, I64_MAX])
        previous = rng.pick([0, 1, I64_MAX - 1, I64_MAX])
        retry_after = rng.pick([0, 1, 899, 900, 901, I64_MAX - 1, I64_MAX])
    else:
        now = rng.pick([-1, 0, 5, I64_MIN])
        previous = rng.pick([-1, 0, 5, I64_MIN])
        retry_after = rng.pick([-1, 0, 5, I64_MIN])
    return [previous, kind, status, now, retry_after]


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
    "hlc_compare": (lambda args: hlc_compare(*args), compare_args, 250, 0xE110_000C),
    "lww_classify": (lambda args: lww_classify(*args), lww_args, 250, 0xE110_000D),
    "orset_union": (lambda args: orset_union(*args), union_args, 300, 0xE110_000E),
    "hlc_text": (lambda args: hlc_text(*args), text_args, 300, 0xE110_000F),
    "gcra_admit": (lambda args: gcra_admit(*args), gcra_args, 400, 0xE110_0010),
    "murmur3_32": (lambda args: murmur3_32(*args), murmur_args, 300, 0xE110_0011),
    "retry_next_backoff": (lambda args: retry_next_backoff(*args), backoff_args, 400, 0xE110_0012),
    "retry_jittered": (lambda args: retry_jittered(*args), jitter_args, 400, 0xE110_0013),
    "retry_exhausted": (lambda args: retry_exhausted(*args), exhausted_args, 150, 0xE110_0014),
    "sync_retry_at": (lambda args: sync_retry_at(*args), sync_retry_args, 400, 0xE110_0015),
}


def render(value) -> str:
    """A value the way the hand-written cases write it: a record as
    `{ "year": 1970, ... }`, a list as `[1, 2]`, a boolean as `true`, a number
    as itself."""
    if isinstance(value, dict):
        members = ", ".join(f'"{key}": {render(member)}' for key, member in value.items())
        return "{ " + members + " }"
    if isinstance(value, list):
        return "[" + ", ".join(render(member) for member in value) + "]"
    return json.dumps(value)


def identity(args) -> str:
    """What makes two cases the same input: a record or a list is not hashable."""
    return json.dumps(args, sort_keys=True)


def case_line(args, answer) -> str:
    kind, value = answer
    tail = f'"expected": {render(value)}' if kind == "ok" else f'"fails": "{value}"'
    return f'        {{ "args": {render(args)}, {tail}, "note": "{MARK}" }}'


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
        seen = {identity(json.loads(HAND_CASE.match(line).group(1))) for line in hand}
        rng = SplitMix64(seed)
        made, guard = [], 0
        while len(made) < count:
            guard += 1
            if guard > count * 50:
                raise SystemExit(f"{fixture}: cannot draw {count} distinct inputs")
            args = draw(rng)
            key = identity(args)
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
