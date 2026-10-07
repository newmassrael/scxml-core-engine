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

`validate_laws` holds the models to something a case cannot: what the documents
claim about themselves — a set union that commutes, associates and is idempotent,
a change log whose answer does not depend on where it was cut, a mailbox page
that continues from the last id, a clock that only moves forward, a limiter that
admits exactly `burst` permits. Every backend agreeing with a model on a
thousand cases still leaves such a claim unproved, and a model that refutes one
says the claim is false of the algorithm the document wrote. It runs with
`--check`, and refutes each of four deliberately broken models (a clock that
forgets its counter, a log applied deletes first as Tutanota does, a page that
reads one id early, a union that keeps one new entry).

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

Fifth run, 2026-09-30: `hlc_send`, `hlc_receive`, `hlc_within_drift`, `orset_live`,
`orset_contains`, `orset_observed`, `dedup_admit`, `dedup_holds` and the four
`ordering_*` documents — 3900 cases. Lists that a host keeps ascending are also
given unsorted or with a sequence twice, since the documents' loops answer those
the same way on every backend and a lowering that assumed order would not.
`ordering_gap_end` takes the clock difference only for its first held slot, as `&&`
short-circuits; the model does the same, and no backend read it eagerly. All six
agreed with the model on every case, and a wrong expectation in six of the
fixtures fails Go and C++.

Sixth run, 2026-09-30: the three `outbound_*` documents, the sync client's
`sync_failure`, `sync_delete_outcome` and `sync_upload_outcome`, the HTTP
`precondition`, the four `mailbox_*` documents, `changes_since`, `changes_apply`,
`utc_offset_at` and `utc_from_local` — 5300 cases. The models state each document's
meaning (a change log is decided by each item's last row; a mailbox page is the ids
above `after`; a local reading names the earliest instant, else the gap's) and are
held to 280 hand cases before they generate; only where a failure's name depends on
the order the document evaluates in (`utc_from_local`, `outbound_stale`) do they
follow that order. All six backends agreed on every case, and a wrong expectation in
seven of the fixtures fails Go and C++.

Seventh run, 2026-09-30: `algorithm_crc16` and its table form, `algorithm_checked_index`,
`algorithm_bytes_squeeze`, `algorithm_list_param_sum`, the two HLC resources,
`algorithm_weekly_expand` (`week % interval` is reached only for a day the mask names),
`acl_membership`, `acl_granted` and the three `merkle_*` documents — 4600 cases; all
six backends agreed. What the laws found: `merkle_diff` promises that its answer is
never later than the earliest change one side lacks, and that is false once one side
has been pruned — a branch only the pruned side held, and dropped, is listed by
neither digest, so the search descends into the branches both list and answers a
later minute. The algorithm is Actual's and does what its correction intends; the
document's sentence was too strong, so it now says where the bound holds, and two
hand cases pin the same pair of digests pruned and not. A wrong expectation in five
of the fixtures fails Go and C++.
"""

from __future__ import annotations

import calendar
import datetime
import itertools
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


def hlc_send(prev: dict, now: int, node: int):
    """sce:std/merge/hlc_send — the wall time never goes back; when it does not
    advance the counter does, and a counter at its uint32 maximum cannot."""
    wall = now if now > prev["wallTime"] else prev["wallTime"]
    counter = 0
    if wall == prev["wallTime"]:
        counter = prev["counter"] + 1
        if counter > U32_MAX:
            return ("fails", "overflow")
    return ("ok", hlc(wall, counter, node))


def hlc_receive(prev: dict, msg: dict, now: int, node: int):
    """sce:std/merge/hlc_receive — the largest of three wall times, the counter
    continuing whichever of prev and msg share it."""
    seen = max(prev["wallTime"], msg["wallTime"])
    wall = now if now > seen else seen
    counter = 0
    if wall == prev["wallTime"] and wall == msg["wallTime"]:
        counter = max(prev["counter"], msg["counter"]) + 1
    elif wall == prev["wallTime"]:
        counter = prev["counter"] + 1
    elif wall == msg["wallTime"]:
        counter = msg["counter"] + 1
    if counter > U32_MAX:
        return ("fails", "overflow")
    return ("ok", hlc(wall, counter, node))


def hlc_within_drift(wall: int, now: int, max_drift: int):
    """sce:std/merge/hlc_within_drift — the difference of two int64 readings is
    checked, and only taken when the wall time is ahead."""
    if max_drift < 0:
        return ("fails", "precondition")
    if wall <= now:
        return ("ok", True)
    ahead = wall - now
    if ahead > I64_MAX:
        return ("fails", "overflow")
    return ("ok", ahead <= max_drift)


def orset_live(adds: list, tombstones: list):
    """sce:std/merge/orset_live — adds, in order, that no tombstone equals."""
    return ("ok", [add for add in adds if add not in tombstones])


def orset_contains(live: list, element: int):
    return ("ok", any(entry_["element"] == element for entry_ in live))


def orset_observed(live: list, element: int):
    return ("ok", [entry_ for entry_ in live if entry_["element"] == element])


UNDO_PLAN_MAX = 256


def field_version_compare(a: dict, b: dict):
    """sce:std/merge/field_version_compare — the order of the two writes' clock
    stamps and nothing else of them."""
    ka, kb = hlc_key(a), hlc_key(b)
    return ("ok", (ka > kb) - (ka < kb))


def undo_field_plan(versions: list, batch: int, floor: int):
    """sce:std/merge/undo_field_plan — one step for each field the batch wrote.

    A step is planned from the batch's EARLIEST write to its field (by stamp). The
    field's winner is its latest write by stamp, the first of them in list order if
    two tie. 3: that earliest write arrived at or before `floor`. 2: the winner is
    an undo of this batch. 0: the winner is a write of this batch, and the value to
    restore is the one the latest write earlier than the earliest batch write held
    (0 if there is none). 1: anyone else's write is the winner, and its stamp is
    reported. Steps come in the order the earliest writes appear in `versions`."""
    if batch == 0:
        return ("fails", "precondition")
    if sum(1 for write in versions if write["batchId"] == batch) > UNDO_PLAN_MAX:
        return ("fails", "precondition")
    steps = []
    for first in versions:
        if first["batchId"] != batch:
            continue
        field = [
            write
            for write in versions
            if write["entityId"] == first["entityId"] and write["fieldId"] == first["fieldId"]
        ]
        if any(write["batchId"] == batch and hlc_key(write) < hlc_key(first) for write in field):
            continue
        winner = base = None
        for write in field:
            if winner is None or hlc_key(write) > hlc_key(winner):
                winner = write
            if hlc_key(write) < hlc_key(first) and (base is None or hlc_key(write) > hlc_key(base)):
                base = write
        if first["arrival"] <= floor:
            verdict = 3
        elif winner["origin"] == 2 and winner["revertsBatch"] == batch:
            verdict = 2
        elif winner["batchId"] == batch:
            verdict = 0
        else:
            verdict = 1
        steps.append(
            {
                "entityId": first["entityId"],
                "fieldId": first["fieldId"],
                "verdict": verdict,
                "restoreRef": base["valueRef"] if verdict == 0 and base is not None else 0,
                "winnerWallTime": winner["wallTime"] if verdict == 1 else 0,
                "winnerCounter": winner["counter"] if verdict == 1 else 0,
                "winnerNodeId": winner["nodeId"] if verdict == 1 else 0,
            }
        )
    return ("ok", steps)


def undo_set_readds(batch_tombstones: list, live: list):
    """sce:std/merge/undo_set_readds — the elements the batch removed that the set
    does not hold, each once, in the order the batch removed them."""
    if len(batch_tombstones) > UNDO_PLAN_MAX:
        return ("fails", "precondition")
    held = {entry_["element"] for entry_ in live}
    out = []
    for tombstone in batch_tombstones:
        element = tombstone["element"]
        if element not in held and element not in out:
            out.append(element)
    return ("ok", out)


def dedup_admit(window: list, ident: dict, capacity: int):
    """sce:std/mesh/dedup_admit — the window with the id appended, keeping the last
    `capacity`; an id already held leaves it unchanged."""
    if not (capacity > 0 and len(window) <= capacity):
        return ("fails", "precondition")
    held = ident in window
    skip = 0 if held or len(window) < capacity else 1
    out = window[skip:]
    if not held:
        out = out + [ident]
    return ("ok", out)


def dedup_holds(window: list, ident: dict):
    return ("ok", ident in window)


def slot(seq: int, arrived_at: int) -> dict:
    return {"seq": seq, "arrivedAtMs": arrived_at}


def ordering_drain(pending: list, start: int):
    """sce:std/mesh/ordering_drain — one pass in the list's order: each slot at
    the sequence expected next moves it on. A list that is not ascending is
    answered as the document's loop answers it, which is the same on every
    backend."""
    nxt = start
    for held in pending:
        if held["seq"] == nxt:
            if nxt + 1 > U64_MAX:
                return ("fails", "overflow")
            nxt += 1
    return ("ok", nxt)


def ordering_gap_end(pending: list, nxt: int, now: int, timeout_ms: int):
    """sce:std/mesh/ordering_gap_end — only the first held slot counts, and the
    clock difference is only taken for it, as `&&` short-circuits."""
    resume = nxt
    for index, held in enumerate(pending):
        if index == 0 and held["seq"] != nxt:
            waited = now - held["arrivedAtMs"]
            if not fits(waited, I64_MIN, I64_MAX):
                return ("fails", "overflow")
            if waited >= timeout_ms:
                resume = held["seq"]
    return ("ok", resume)


def ordering_hold(pending: list, seq: int, now: int):
    """sce:std/mesh/ordering_hold — the arrival placed before the first held
    slot with a larger sequence; a sequence already held is kept as it was."""
    arrived = slot(seq, now)
    out, placed = [], False
    for held in pending:
        if held["seq"] == seq:
            placed = True
        if not placed and seq < held["seq"]:
            out.append(arrived)
            placed = True
        out.append(held)
    if not placed:
        out.append(arrived)
    return ("ok", out)


def ordering_prune(pending: list, nxt: int):
    return ("ok", [held for held in pending if held["seq"] >= nxt])


class Failure(Exception):
    """A checked operation of the document that has no value: its contract name."""

    def __init__(self, name: str) -> None:
        super().__init__(name)
        self.name = name


def checked(value: int, low: int = I64_MIN, high: int = I64_MAX) -> int:
    """`value` if it fits, else the `overflow` the document's checked operation
    reports — for a model that follows the document's order of evaluation."""
    if not fits(value, low, high):
        raise Failure("overflow")
    return value


def answering(model):
    """A model that raises `Failure` where the document fails, as a model that
    answers `("fails", name)`."""

    def answer(*args):
        try:
            return model(*args)
        except Failure as failure:
            return ("fails", failure.name)

    return answer


def outbound_overflows(depth: int, max_pending: int):
    return ("ok", depth >= max_pending)


def outbound_sends_now(ready: bool, depth: int):
    return ("ok", ready and depth == 0)


def outbound_stale(enqueued_at: int, now: int, max_age: int):
    """sce:std/mesh/outbound_stale — the clock difference is taken only when a
    bound is given, and is checked."""
    if max_age > 0:
        return ("ok", checked(now - enqueued_at) > max_age)
    return ("ok", False)


def http_status_holds(kind: int, status: int, low: int) -> bool:
    return low <= status <= 599 if kind == 4 else status == 0


def sync_failure(kind: int, status: int):
    """sce:std/sync/sync_failure — what to do about the failure that stopped a
    run, the reference's order (each later rule wins)."""
    if not (1 <= kind <= 9 and http_status_holds(kind, status, 300)):
        return ("fails", "precondition")
    action = 2
    if kind in (8, 1, 2):
        action = 1
    if kind == 7:
        action = 4
    if kind == 3:
        action = 0
    if kind == 4 and status == 401:
        action = 3
    if kind == 4 and status in (503, 502):
        action = 1
    return ("ok", action)


def sync_delete_outcome(kind: int, status: int):
    """sce:std/sync/sync_delete_outcome — 0 forget the item, 1 forget the local
    deletion and resynchronise, 2 keep the deletion and stop."""
    if not (1 <= kind <= 9 and http_status_holds(kind, status, 200)):
        return ("fails", "precondition")
    outcome = 2
    if kind == 4 and status < 500:
        outcome = 0 if status < 300 or status in (404, 410) else 1
    return ("ok", outcome)


def sync_upload_outcome(create: bool, status: int, dav_error: bool):
    """sce:std/sync/sync_upload_outcome — 0 stored, 1 discard the local change,
    2 stop the run."""
    if not 200 <= status <= 599:
        return ("fails", "precondition")
    outcome = 2
    if status < 300:
        outcome = 0
    if status in (403, 412) or (status == 409 and dav_error):
        outcome = 1
    if not create and status in (404, 410, 409):
        outcome = 1
    return ("ok", outcome)


def http_precondition(exists: bool, read_only: bool, if_match: int, unmodified: int, none_match: int, modified: int):
    """sce:std/http/precondition — RFC 9110 13.2.2 in its order: If-Match, else
    If-Unmodified-Since; then If-None-Match, else, for GET and HEAD, If-Modified-Since."""
    if not (if_match <= 3 and unmodified <= 2 and none_match <= 3 and modified <= 2):
        return ("fails", "precondition")
    if if_match != 0:
        if not exists or if_match == 3:
            return ("ok", 412)
    elif unmodified == 2:
        return ("ok", 412)
    if none_match != 0:
        if (none_match == 1 and exists) or none_match == 2:
            return ("ok", 304 if read_only else 412)
    elif read_only and modified == 2:
        return ("ok", 304)
    return ("ok", 0)


def slot_entry(ident: int, hi: int, lo: int) -> dict:
    return {"id": ident, "hi": hi, "lo": lo}


def mailbox_ack(queue: list, acked: list):
    """sce:std/sync/mailbox_ack — every message not acknowledged, in order."""
    return ("ok", [s for s in queue if {"hi": s["hi"], "lo": s["lo"]} not in acked])


def mailbox_assign(queue: list, counter: int, hi: int, lo: int):
    """sce:std/sync/mailbox_assign — the id already held for the message, or one
    more than the counter; every queued id must be within the counter."""
    if not 0 <= counter < I64_MAX:
        return ("fails", "precondition")
    ident = counter + 1
    for held in queue:
        if not 1 <= held["id"] <= counter:
            return ("fails", "precondition")
        if held["hi"] == hi and held["lo"] == lo:
            ident = held["id"]
    return ("ok", ident)


def mailbox_insert(queue: list, counter: int, hi: int, lo: int):
    """sce:std/sync/mailbox_insert — unchanged when the message is held, else
    with it appended under `counter + 1`; ids must ascend and stay within counter."""
    if not 0 <= counter < I64_MAX:
        return ("fails", "precondition")
    previous, held_already = 0, False
    for held in queue:
        if not previous < held["id"] <= counter:
            return ("fails", "precondition")
        previous = held["id"]
        held_already = held_already or (held["hi"] == hi and held["lo"] == lo)
    out = list(queue)
    if not held_already:
        out.append(slot_entry(counter + 1, hi, lo))
    return ("ok", out)


def mailbox_page(queue: list, after: int, limit: int):
    """sce:std/sync/mailbox_page — the messages with an id above `after`, in id
    order, at most `limit`; ids must ascend."""
    if not 1 <= limit <= 256:
        return ("fails", "precondition")
    previous = 0
    for held in queue:
        if not held["id"] > previous:
            return ("fails", "precondition")
        previous = held["id"]
    return ("ok", [held for held in queue if held["id"] > after][:limit])


def change_entry(item: int, op: int, token: int) -> dict:
    return {"item": item, "op": op, "token": token}


def log_is_ordered(log: list) -> bool:
    """Non-decreasing tokens and an `op` of 1 to 3, in every row."""
    previous = 0
    for row in log:
        if not (row["token"] >= previous and 1 <= row["op"] <= 3):
            return False
        previous = row["token"]
    return True


def changes_since(log: list, since: int, current: int, low_water: int, limit: int):
    """sce:std/sync/changes_since — of the rows at or after `since`, each item's
    last one, in token order, at most `limit`."""
    if not low_water <= since <= current or not 1 <= limit <= 256 or not log_is_ordered(log):
        return ("fails", "precondition")
    last_of = {}
    for index, row in enumerate(log):
        last_of[row["item"]] = index
    rows = [
        row for index, row in enumerate(log) if row["token"] >= since and last_of[row["item"]] == index
    ]
    return ("ok", rows[:limit])


def changes_apply(state: list, log: list):
    """sce:std/sync/changes_apply — each item decided by its last row alone: an
    add or a modify writes it at the row's token, a delete removes it; an item no
    row names keeps what `state` held; the answer ascends by item."""
    items = [held["item"] for held in state]
    if any(later <= earlier for earlier, later in zip(items, items[1:])) or not log_is_ordered(log):
        return ("fails", "precondition")
    held = {entry_["item"]: entry_["token"] for entry_ in state}
    for row in log:
        if row["op"] == 3:
            held.pop(row["item"], None)
        else:
            held[row["item"]] = row["token"]
    return ("ok", [{"item": item, "token": held[item]} for item in sorted(held)])


def utc_offset_at(starts: list, offsets: list, t: int):
    """sce:std/time/utc_offset_at — the offset of the last transition at or
    before `t`; the table must be non-empty, equal in length, strictly ascending."""
    if not (len(starts) == len(offsets) and len(starts) > 0):
        return ("fails", "precondition")
    if not t >= starts[0]:
        return ("fails", "precondition")
    if any(later <= earlier for earlier, later in zip(starts, starts[1:])):
        return ("fails", "precondition")
    in_force = max(index for index, start in enumerate(starts) if start <= t)
    return ("ok", offsets[in_force])


def utc_from_local(starts: list, offsets: list, local: int):
    """sce:std/time/utc_from_local — the earliest instant whose offset gives the
    reading, else, for a gap, the reading under the offset before it; each
    subtraction is checked, in the order the document takes them."""
    if not (len(starts) == len(offsets) and len(starts) > 0):
        return ("fails", "precondition")
    if not checked(local - offsets[0]) >= starts[0]:
        return ("fails", "precondition")
    count = len(starts)
    for k in range(count):
        if k > 0 and not starts[k] > starts[k - 1]:
            return ("fails", "precondition")
        instant = checked(local - offsets[k])
        if instant >= starts[k] and (k + 1 == count or instant < starts[k + 1]):
            return ("ok", instant)
    for g in range(1, count):
        before = checked(local - offsets[g - 1])
        if before >= starts[g] and checked(local - offsets[g]) < starts[g]:
            return ("ok", before)
    return ("fails", "precondition")


def crc16_ccitt_false(data: list):
    """algorithm_crc16, algorithm_crc16_table — CRC-16/CCITT-FALSE (poly 0x1021,
    init 0xFFFF, no reflection, no final xor), bit by bit."""
    crc = 0xFFFF
    for byte in data:
        crc ^= byte << 8
        for _ in range(8):
            crc = ((crc << 1) ^ 0x1021) & 0xFFFF if crc & 0x8000 else (crc << 1) & 0xFFFF
    return ("ok", crc)


def checked_index(data: list, signed: int, unsigned: int):
    """algorithm_checked_index — `data[i] ^ data[u]`, either index outside the
    buffer failing `out-of-range`."""
    if not (0 <= signed < len(data) and 0 <= unsigned < len(data)):
        return ("fails", "out-of-range")
    return ("ok", data[signed] ^ data[unsigned])


def bytes_squeeze(data: list):
    """algorithm_bytes_squeeze — each run of one byte written once, as `tr -s`."""
    out: list = []
    for byte in data:
        if not out or out[-1] != byte:
            out.append(byte)
    return ("ok", out)


def list_param_sum(values: list):
    """algorithm_list_param_sum — the sum, and the last element once more."""
    return ("ok", sum(values) + (values[-1] if values else 0))


def legacy_hlc_compare(a: dict, b: dict):
    """algorithm_hlc_compare — 1 when `a` is the later stamp, else 0."""
    return ("ok", 1 if hlc_key(a) > hlc_key(b) else 0)


def weekly_expand(dtstart: int, mask: int, interval: int, window_start: int, window_end: int):
    """algorithm_weekly_expand — each day of the window, on or after `dtstart`,
    whose weekday (day 0 is a Thursday) is in `mask` and whose week since `dtstart`
    is a multiple of `interval`. Every operation is checked, and `week % interval`
    is reached only for a day the mask names, as `&&` short-circuits."""
    out = []
    day = window_start if window_start > dtstart else dtstart
    while day < window_end:
        weekday = truncated_remainder(checked(day + 3), 7)
        week = truncated_quotient(checked(day - dtstart), 7)
        if (mask >> weekday) & 1 == 1:
            if interval == 0:
                raise Failure("divide-by-zero")
            if truncated_remainder(week, interval) == 0:
                out.append(day)
        day = checked(day + 1)
    return ("ok", out)


ACL_ALL, ACL_AUTHENTICATED, ACL_UNAUTHENTICATED, ACL_OWNER = 4294967295, 4294967294, 4294967293, 4294967292


def acl_membership(edges: list, user: int):
    """sce:std/acl/acl_membership — every group the user belongs to, directly or
    through others, in the order a breadth-first walk meets them; the user is never
    their own group and 0 is no group."""
    if user == 0:
        return ("ok", [])
    found: list = []
    queue = [user]
    for current in queue:
        for edge in edges:
            group = edge["group"]
            if edge["member"] == current and group != user and group != 0 and group not in found:
                found.append(group)
                queue.append(group)
    return ("ok", found)


def acl_granted(tree: list, acl: list, user: int, groups: list, owner: int):
    """sce:std/acl/acl_granted — of the privileges of `tree`, those an applicable
    entry of `acl` grants, itself or through an aggregate above it, in tree order."""
    for position, privilege in enumerate(tree):
        earlier = tree[:position]
        root_first = privilege["id"] == 0 and privilege["parent"] == 0 if position == 0 else privilege["id"] != 0
        parent_known = position == 0 or any(q["id"] == privilege["parent"] for q in earlier)
        if not root_first or not parent_known or any(q["id"] == privilege["id"] for q in earlier):
            return ("fails", "precondition")
    parent_of = {privilege["id"]: privilege["parent"] for privilege in tree}

    def applies(entry_: dict) -> bool:
        principal = entry_["principal"]
        if principal == ACL_ALL:
            return True
        if principal == ACL_AUTHENTICATED and user != 0:
            return True
        if principal == ACL_UNAUTHENTICATED and user == 0:
            return True
        who = owner if principal == ACL_OWNER else principal
        return who != 0 and user != 0 and who < ACL_OWNER and (who == user or who in groups)

    def granted(privilege_id: int) -> bool:
        at = privilege_id
        while True:
            if any(entry_["privilege"] == at and applies(entry_) for entry_ in acl):
                return True
            if at == 0:
                return False
            at = parent_of[at]

    return ("ok", [privilege["id"] for privilege in tree if granted(privilege["id"])])


def merkle_path(minute: int) -> list:
    """The nodes a change at `minute` lies under: the root, then for each depth 1
    to 16 the node whose prefix is the minute's base-3 digits down to that depth."""
    return [(0, 0)] + [(depth, minute // 3 ** (16 - depth)) for depth in range(1, 17)]


def merkle_node(depth: int, prefix: int, hash_: int) -> dict:
    return {"depth": depth, "prefix": prefix, "hash": hash_}


def merkle_digest_valid(nodes: list) -> bool:
    """Ascending (depth, prefix) with no depth past 16 — a digest the family wrote."""
    keys = [(node["depth"], node["prefix"]) for node in nodes]
    return all(depth <= 16 for depth, _ in keys) and all(a < b for a, b in zip(keys, keys[1:]))


def merkle_insert(nodes: list, minute: int, hash_: int):
    """sce:std/merge/merkle_insert — the change's hash XORed into every node on its
    path, a missing node created with the hash alone."""
    if minute < 0 or not merkle_digest_valid(nodes):
        return ("fails", "precondition")
    table = {(node["depth"], node["prefix"]): node["hash"] for node in nodes}
    for key in merkle_path(minute):
        table[key] = table.get(key, 0) ^ hash_
    return ("ok", [merkle_node(depth, prefix, table[(depth, prefix)]) for depth, prefix in sorted(table)])


def merkle_prune(nodes: list, keep: int):
    """sce:std/merge/merkle_prune — under every kept node only the `keep` children
    with the largest prefixes stay, and nothing under a dropped node."""
    if keep < 1 or not merkle_digest_valid(nodes):
        return ("fails", "precondition")
    kept = []
    for node in nodes:
        stays, depth, prefix = True, node["depth"], node["prefix"]
        while stays and depth >= 1:
            after = sum(
                1
                for other in nodes
                if other["depth"] == depth
                and (depth == 1 or other["prefix"] // 3 == prefix // 3)
                and other["prefix"] > prefix
            )
            stays = after < keep
            prefix = prefix // 3 if depth >= 2 else 0
            depth -= 1
        if stays:
            kept.append(node)
    return ("ok", kept)


def merkle_diff(a: list, b: list):
    """sce:std/merge/merkle_diff — where two digests part: equal roots mean the
    same changes, else descend from the root into the first child, in ascending
    prefix, whose hashes differ, stopping at a child missing on one side; the answer
    is the minute the path so far begins at, in milliseconds."""

    def root(nodes: list) -> int:
        return next((node["hash"] for node in reversed(nodes) if node["depth"] == 0), 0)

    def child_hash(nodes: list, depth: int, prefix: int):
        return next(
            (node["hash"] for node in reversed(nodes) if node["depth"] == depth and node["prefix"] == prefix),
            None,
        )

    if root(a) == root(b):
        return ("ok", {"differs": False, "millis": 0})
    depth, prefix, span = 0, 0, 3**16
    while depth < 16:
        children = sorted(
            {
                node["prefix"]
                for node in a + b
                if node["depth"] == depth + 1 and (depth == 0 or node["prefix"] // 3 == prefix)
            }
        )
        descended = False
        for child in children:
            in_a, in_b = child_hash(a, depth + 1, child), child_hash(b, depth + 1, child)
            if in_a is None or in_b is None:
                break
            if in_a != in_b:
                prefix, descended = child, True
                break
        if not descended:
            break
        depth += 1
        span //= 3
    minute = checked(prefix * span, 0, U64_MAX)
    minute = checked(minute)
    return ("ok", {"differs": True, "millis": checked(minute * 60000)})


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


def entry_pool(rng: SplitMix64) -> list:
    """Distinct entries that share fields with one another, so that a set of them
    holds the same element twice and the same tag on two elements."""
    stamp = stamp_value(rng)
    first = entry(rng.pick(ELEMENT_EDGES), stamp["wallTime"], stamp["counter"], stamp["nodeId"])
    pool = [first]
    for _ in range(rng.between(1, 8)):
        candidate = entry_near(rng, rng.pick(pool))
        if candidate not in pool:
            pool.append(candidate)
    return pool


def entry_sample(rng: SplitMix64, pool: list) -> list:
    return shuffled(rng, pool)[: rng.between(0, len(pool))]


def union_args(rng: SplitMix64):
    pool = entry_pool(rng)
    return [entry_sample(rng, pool), entry_sample(rng, pool)]


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


def clamp(value: int, low: int, high: int) -> int:
    return min(max(value, low), high)


def clock_near(rng: SplitMix64, wall: int) -> int:
    """A clock reading about a stamp's wall time: the same, a step either way, a
    little apart, at an edge, or anywhere — so the wall time advances, ties and
    goes back each often."""
    kind = rng.below(10)
    if kind < 4:
        return clamp(wall + rng.pick([0, 0, 1, -1]), I64_MIN, I64_MAX)
    if kind < 7:
        return clamp(wall + rng.between(-1000, 1000), I64_MIN, I64_MAX)
    if kind < 9:
        return rng.pick(WALL_EDGES)
    return rng.between(I64_MIN, I64_MAX)


def send_args(rng: SplitMix64):
    prev = stamp_value(rng)
    node = rng.pick(NODE_EDGES) if rng.below(3) == 0 else rng.between(0, 5)
    return [prev, clock_near(rng, prev["wallTime"]), node]


def receive_args(rng: SplitMix64):
    prev = stamp_value(rng)
    kind = rng.below(10)
    msg = prev if kind == 0 else (near_stamp(rng, prev) if kind < 8 else stamp_value(rng))
    anchor = prev["wallTime"] if rng.below(2) else msg["wallTime"]
    node = rng.pick(NODE_EDGES) if rng.below(3) == 0 else rng.between(0, 5)
    return [prev, msg, clock_near(rng, anchor), node]


def drift_args(rng: SplitMix64):
    kind = rng.below(10)
    if kind < 6:
        wall = REALISTIC_WALL + rng.between(-100_000, 100_000)
        now = wall - rng.between(-100_000, 100_000)
        ahead = wall - now
        bound = rng.pick([ahead, ahead - 1, ahead + 1, 0, 1, 300_000, 5000, -1])
    elif kind < 9:
        edges = [I64_MIN, I64_MIN + 1, -1, 0, 1, I64_MAX - 1, I64_MAX]
        wall, now = rng.pick(edges), rng.pick(edges)
        bound = rng.pick([0, 1, I64_MAX, I64_MAX - 1, -1])
    else:
        wall, now = rng.between(I64_MIN, I64_MAX), rng.between(I64_MIN, I64_MAX)
        bound = rng.between(-1, I64_MAX)
    return [wall, now, bound]


def live_args(rng: SplitMix64):
    """Adds and the tombstones that remove some of them, sometimes an add twice
    and a tombstone for an add the set never held."""
    pool = entry_pool(rng)
    adds = entry_sample(rng, pool)
    if adds and rng.below(6) == 0:
        adds = adds + [rng.pick(adds)]
    tombstones = entry_sample(rng, pool)
    if rng.below(6) == 0:
        tombstones = tombstones + [entry_near(rng, rng.pick(pool))]
    return [adds, tombstones]


def element_args(rng: SplitMix64):
    """A set's live entries and an element — mostly one it holds, or its
    neighbour, or an edge."""
    pool = entry_pool(rng)
    live = entry_sample(rng, pool)
    kind = rng.below(10)
    if kind < 5 and live:
        element = rng.pick(live)["element"]
    elif kind < 7:
        element = clamp(rng.pick(pool)["element"] + rng.pick([-1, 1]), 0, U64_MAX)
    elif kind < 9:
        element = rng.pick(ELEMENT_EDGES)
    else:
        element = rng.between(0, U64_MAX)
    return [live, element]


UNDO_BATCHES = [1, 2, 5, 2**32, 2**63, U64_MAX - 1, U64_MAX]
UNDO_ENTITIES = [0, 1, 2, 7, 2**32, U64_MAX]
UNDO_FIELDS = [0, 1, 2, 3, U32_MAX]
REF_EDGES = [0, 1, 2, 2**32, 2**63, U64_MAX]


def field_write(entity, field, stamp, arrival, batch, origin, reverts, ref) -> dict:
    return {
        "entityId": entity,
        "fieldId": field,
        "wallTime": stamp["wallTime"],
        "counter": stamp["counter"],
        "nodeId": stamp["nodeId"],
        "arrival": arrival,
        "batchId": batch,
        "origin": origin,
        "revertsBatch": reverts,
        "valueRef": ref,
    }


def history_for(rng: SplitMix64, batch: int):
    """The writes of a few fields: stamps unique within a field (a clock issues
    none twice), some of them the batch's, a person's or another batch's, some an
    undo of the batch or of another, in an order that is sometimes shuffled."""
    other = rng.pick([candidate for candidate in UNDO_BATCHES if candidate != batch])
    fields = []
    for _ in range(rng.between(1, 3)):
        pair = (rng.pick(UNDO_ENTITIES), rng.pick(UNDO_FIELDS))
        if pair not in fields:
            fields.append(pair)
    arrival = rng.between(1, 4) if rng.below(4) else rng.pick([0, 1, 2**32, 2**63, U64_MAX - 40])
    versions = []
    for entity, field in fields:
        first = stamp_value(rng)
        seen = {hlc_key(first)}
        stamps = [first]
        for _ in range(rng.pick([0, 1, 2, 2, 3, 4, 6])):
            stamp = near_stamp(rng, first) if rng.below(3) else stamp_value(rng)
            if hlc_key(stamp) not in seen:
                seen.add(hlc_key(stamp))
                stamps.append(stamp)
        for stamp in stamps:
            roll = rng.below(10)
            owner = batch if roll < 4 else (0 if roll < 7 else other)
            if rng.below(8) == 0:
                origin, reverts = 2, (batch if rng.below(2) else other)
            else:
                origin, reverts = (1 if owner else rng.pick([0, 0, 1])), 0
            ref = rng.pick(REF_EDGES) if rng.below(3) == 0 else rng.between(0, 9)
            versions.append(field_write(entity, field, stamp, min(arrival, U64_MAX), owner, origin, reverts, ref))
            arrival += 1
    if rng.below(2):
        versions = shuffled(rng, versions)
    return versions


def plan_args(rng: SplitMix64):
    """A batch, a history of a few fields and a floor. Mostly an ordinary batch;
    sometimes batch 0, one of exactly 256 writes, one of 257."""
    if rng.below(120) == 0:
        count = rng.pick([256, 257])
        batch = rng.pick(UNDO_BATCHES)
        versions = [
            field_write(1, index, hlc(REALISTIC_WALL + index, 0, 1), index + 1, batch, 1, 0, index)
            for index in range(count)
        ]
        return [versions, batch, rng.pick([0, 1, count + 1])]
    batch = rng.pick(UNDO_BATCHES) if rng.below(40) else 0
    versions = history_for(rng, batch if batch else UNDO_BATCHES[0])
    if batch and rng.below(5):
        # An undo of the batch already written to one of its fields, later than
        # everything the field holds — the state an undo leaves behind.
        written = [w for w in versions if w["batchId"] == batch]
        if written:
            pick = rng.pick(written)
            field = [
                w for w in versions
                if w["entityId"] == pick["entityId"] and w["fieldId"] == pick["fieldId"]
            ]
            latest = max(field, key=hlc_key)
            stamp = after_stamp(hlc(latest["wallTime"], latest["counter"], latest["nodeId"]))
            if stamp is not None:
                versions.append(
                    field_write(
                        pick["entityId"], pick["fieldId"], stamp,
                        min(max(w["arrival"] for w in versions) + 1, U64_MAX),
                        rng.pick([candidate for candidate in UNDO_BATCHES if candidate != batch]),
                        2, batch, rng.between(0, 9),
                    )
                )
                if rng.below(2):
                    versions = shuffled(rng, versions)
    arrivals = [write["arrival"] for write in versions]
    kind = rng.below(10)
    if kind < 4:
        floor = 0
    elif kind == 4:
        floor = U64_MAX
    elif kind < 8:
        floor = rng.pick(arrivals)
    else:
        floor = clamp(rng.pick(arrivals) + rng.pick([-1, 1]), 0, U64_MAX)
    return [versions, batch, floor]


def readds_args(rng: SplitMix64):
    """What a batch removed and what the set holds now — the same element often
    both, an element removed twice, and sometimes 256 or 257 removals."""
    if rng.below(100) == 0:
        count = rng.pick([256, 257])
        removed = [entry(index, REALISTIC_WALL + index, 0, 1) for index in range(count)]
        return [removed, entry_sample(rng, entry_pool(rng))]
    pool = entry_pool(rng)
    removed = entry_sample(rng, pool)
    if removed and rng.below(4) == 0:
        removed = removed + [entry_near(rng, rng.pick(removed))]
    return [removed, entry_sample(rng, pool)]


def version_compare_args(rng: SplitMix64):
    """Two writes whose stamps are the same, differ in a field or two, or are
    unrelated; nothing else about them is related."""
    first = stamp_value(rng)
    kind = rng.below(10)
    second = first if kind == 0 else (near_stamp(rng, first) if kind < 8 else stamp_value(rng))

    def write(stamp):
        return field_write(
            rng.pick(UNDO_ENTITIES),
            rng.pick(UNDO_FIELDS),
            stamp,
            rng.pick(REF_EDGES),
            rng.pick(UNDO_BATCHES + [0]),
            rng.pick([0, 1, 2]),
            rng.pick(UNDO_BATCHES + [0]),
            rng.pick(REF_EDGES),
        )

    return [write(first), write(second)]


ID_EDGES = [0, 1, 2, 2**32, 2**63, U64_MAX - 1, U64_MAX]


def envelope_id(rng: SplitMix64) -> dict:
    """An id whose halves are small or at an edge, so two ids often share one."""
    hi = rng.pick(ID_EDGES) if rng.below(2) else rng.between(0, 3)
    lo = rng.pick(ID_EDGES) if rng.below(2) else rng.between(0, 3)
    return {"hi": hi, "lo": lo}


def id_window(rng: SplitMix64) -> list:
    window = []
    for _ in range(rng.between(0, 9)):
        candidate = envelope_id(rng)
        if candidate not in window:
            window.append(candidate)
    return window


def admit_args(rng: SplitMix64):
    window = id_window(rng)
    ident = rng.pick(window) if window and rng.below(3) == 0 else envelope_id(rng)
    capacity = rng.pick(
        [len(window), len(window), len(window) + 1, len(window) - 1, 0, 1, 2, 3, 8, 256, 1000, U32_MAX]
    )
    return [window, ident, clamp(capacity, 0, U32_MAX)]


def holds_args(rng: SplitMix64):
    window = id_window(rng)
    ident = rng.pick(window) if window and rng.below(2) else envelope_id(rng)
    return [window, ident]


SEQ_BASES = [0, 1, 5, 100, 2**32 - 3, 2**53 - 3, 2**63 - 3, U64_MAX - 6]


def arrival_time(rng: SplitMix64) -> int:
    kind = rng.below(10)
    if kind < 6:
        return REALISTIC_WALL + rng.between(-100_000, 100_000)
    if kind < 9:
        return rng.pick([0, 1, -1, I64_MIN, I64_MIN + 1, I64_MAX - 1, I64_MAX])
    return rng.between(I64_MIN, I64_MAX)


def held_slots(rng: SplitMix64):
    """The base sequence, and slots near it — ascending as a host keeps them, and
    now and then shuffled or holding one twice, which the loops answer the same
    way on every backend."""
    base = rng.pick(SEQ_BASES)
    seqs = sorted(
        {clamp(base + rng.between(0, 9), 0, U64_MAX) for _ in range(rng.between(0, 7))}
    )
    if seqs and rng.below(8) == 0:
        seqs.append(rng.pick(seqs))
    if len(seqs) > 1 and rng.below(6) == 0:
        seqs = shuffled(rng, seqs)
    return base, [slot(seq, arrival_time(rng)) for seq in seqs]


def sequence_near(rng: SplitMix64, base: int, pending: list) -> int:
    known = [held["seq"] for held in pending] + [base]
    return clamp(rng.pick(known) + rng.pick([0, 0, 1, -1, 2]), 0, U64_MAX)


def drain_args(rng: SplitMix64):
    base, pending = held_slots(rng)
    return [pending, sequence_near(rng, base, pending)]


def gap_end_args(rng: SplitMix64):
    base, pending = held_slots(rng)
    nxt = sequence_near(rng, base, pending)
    if pending and rng.below(2):
        waited = rng.pick([0, 1, 49, 50, 51, 5000])
        now = clamp(pending[0]["arrivedAtMs"] + waited, I64_MIN, I64_MAX)
        timeout = clamp(waited + rng.pick([-1, 0, 1]), 0, I64_MAX)
    else:
        now = arrival_time(rng)
        timeout = rng.pick([0, 1, 50, 5000, I64_MAX, -1, I64_MIN])
    return [pending, nxt, now, timeout]


def hold_args(rng: SplitMix64):
    base, pending = held_slots(rng)
    return [pending, sequence_near(rng, base, pending), arrival_time(rng)]


def prune_args(rng: SplitMix64):
    base, pending = held_slots(rng)
    return [pending, sequence_near(rng, base, pending)]


# Mesh queues, the sync client's outcomes, the mailbox, the change log and the
# time-zone table: the documents whose answers are decisions over a few small
# codes or over lists a host keeps in order.


def depth_value(rng: SplitMix64) -> int:
    """A queue depth: small, at an edge of a width, or anywhere in uint32."""
    kind = rng.below(10)
    if kind < 5:
        return rng.between(0, 6)
    if kind < 8:
        return rng.pick([255, 256, 65535, 65536, U32_MAX - 1, U32_MAX])
    return rng.between(0, U32_MAX)


def outbound_overflows_args(rng: SplitMix64):
    depth = depth_value(rng)
    return [depth, clamp(depth + rng.pick([-1, 0, 0, 1]), 0, U32_MAX) if rng.below(2) else depth_value(rng)]


def outbound_sends_now_args(rng: SplitMix64):
    return [rng.below(2) == 0, depth_value(rng)]


def outbound_stale_args(rng: SplitMix64):
    kind = rng.below(10)
    if kind < 6:
        enqueued = arrival_time(rng) if rng.below(3) == 0 else REALISTIC_WALL + rng.between(-100_000, 100_000)
        age = rng.pick([0, 1, 49, 50, 51, 1000, 5000, rng.between(0, 100_000)])
        now = clamp(enqueued + age + rng.pick([-1, 0, 1]), I64_MIN, I64_MAX)
        bound = rng.pick([age, age - 1, age + 1, 0, 1, 50, 5000, -1])
    else:
        enqueued, now = arrival_time(rng), arrival_time(rng)
        bound = rng.pick([0, 1, 50, I64_MAX, -1, I64_MIN])
    return [enqueued, now, bound]


STATUS_EDGES = [
    0, 1, 199, 200, 201, 204, 206, 299, 300, 301, 302, 304, 400, 401, 403, 404, 409, 410, 412,
    429, 499, 500, 501, 502, 503, 504, 599, 600, 601, -1, I32_MIN, I32_MAX,
]


def response_status(rng: SplitMix64) -> int:
    return rng.pick(STATUS_EDGES) if rng.below(2) else rng.between(190, 610)


def failure_args(rng: SplitMix64):
    """A failure kind and the status it carries: a response has one, everything
    else none, and now and then the pairing is wrong."""
    kind = 4 if rng.below(3) == 0 else (rng.between(1, 9) if rng.below(10) else rng.pick([0, 10, 255]))
    if kind == 4:
        status = response_status(rng) if rng.below(4) == 0 else rng.between(300, 599)
    else:
        status = response_status(rng) if rng.below(8) == 0 else 0
    return [kind, status]


def upload_args(rng: SplitMix64):
    return [rng.below(2) == 0, response_status(rng), rng.below(2) == 0]


def http_precondition_args(rng: SplitMix64):
    """The conditional headers of a request, in the codes the document names; an
    occasional code beyond them is a precondition failure."""

    def code(limit: int) -> int:
        return rng.between(0, limit) if rng.below(20) else rng.pick([limit + 1, 4, 255])

    return [rng.below(2) == 0, rng.below(2) == 0, code(3), code(2), code(3), code(2)]


def mailbox_ids(rng: SplitMix64):
    """Ascending slot ids from a base, near int64's end as well as near 1, and
    now and then out of order, repeated or zero — a queue the host did not keep."""
    base = rng.pick([1, 1, 7, 2**53, I64_MAX - 12])
    ids, current = [], base - 1
    for _ in range(rng.between(0, 7)):
        step = rng.pick([1, 1, 1, 2, 5])
        if current + step > I64_MAX - 2:
            break
        current += step
        ids.append(current)
    if len(ids) > 1 and rng.below(8) == 0:
        ids = shuffled(rng, ids)
    if ids and rng.below(10) == 0:
        ids[rng.below(len(ids))] = rng.pick([0, -1, ids[0]])
    return ids


def mailbox_queue(rng: SplitMix64) -> list:
    return [slot_entry(ident, **envelope_id(rng)) for ident in mailbox_ids(rng)]


def mailbox_counter(rng: SplitMix64, queue: list) -> int:
    top = max([held["id"] for held in queue], default=0)
    room = [top] * 5 + [top + 1] * 3 + [min(top + 4, I64_MAX - 1)]
    return rng.pick(room + [top - 1, 0, -1, I64_MAX - 1, I64_MAX])


def message_key(rng: SplitMix64, queue: list) -> dict:
    return rng.pick(queue) if queue and rng.below(2) else envelope_id(rng)


def mailbox_ack_args(rng: SplitMix64):
    queue = mailbox_queue(rng)
    acked = [
        {"hi": held["hi"], "lo": held["lo"]} for held in entry_sample(rng, queue)
    ] + ([envelope_id(rng)] if rng.below(3) == 0 else [])
    return [queue, shuffled(rng, acked)]


def mailbox_assign_args(rng: SplitMix64):
    queue = mailbox_queue(rng)
    key = message_key(rng, queue)
    return [queue, mailbox_counter(rng, queue), key["hi"], key["lo"]]


def mailbox_page_args(rng: SplitMix64):
    queue = mailbox_queue(rng)
    known = [held["id"] for held in queue]
    after = clamp(rng.pick(known + [0, -1, I64_MIN, I64_MAX]) + rng.pick([0, 0, -1, 1]), I64_MIN, I64_MAX)
    limit = rng.pick([0, 257, U32_MAX, 256]) if rng.below(6) == 0 else rng.between(1, 8)
    return [queue, after, limit]


ITEM_EDGES = [0, 1, 2, 2**32, 2**53, 2**63, U64_MAX - 1, U64_MAX]
TOKEN_BASES = [0, 1, 10, 2**53, U64_MAX - 12]


def change_log(rng: SplitMix64, items: list) -> list:
    """Rows in non-decreasing token order over a few items, each row an add, a
    modify or a delete — and now and then one out of order or of an unknown op."""
    rows, token = [], rng.pick(TOKEN_BASES)
    for _ in range(rng.between(0, 8)):
        token = clamp(token + rng.pick([0, 1, 1, 2, 5]), 0, U64_MAX)
        rows.append(change_entry(rng.pick(items), rng.between(1, 3), token))
    if rows and rng.below(10) == 0:
        index = rng.below(len(rows))
        if rng.below(2):
            rows[index]["op"] = rng.pick([0, 4, 255])
        else:
            rows[index]["token"] = rng.pick([0, rows[index]["token"] + 7, U64_MAX])
    return rows


def change_items(rng: SplitMix64) -> list:
    pool = [rng.pick(ITEM_EDGES) if rng.below(4) == 0 else rng.between(1, 6) for _ in range(rng.between(1, 5))]
    return pool


def changes_since_args(rng: SplitMix64):
    log = change_log(rng, change_items(rng))
    tokens = [row["token"] for row in log] or [0]
    since = clamp(rng.pick(tokens) + rng.pick([0, 0, 1, -1]), 0, U64_MAX)
    current = clamp(max(tokens) + rng.pick([1, 1, 2, 2, 3, 0, -1]), 0, U64_MAX)
    low_water = clamp(since - rng.pick([0, 0, 0, 1, 3, 5, -1]), 0, U64_MAX)
    limit = rng.pick([0, 257, U32_MAX, 256]) if rng.below(6) == 0 else rng.between(1, 6)
    return [log, since, current, low_water, limit]


def changes_apply_args(rng: SplitMix64):
    items = change_items(rng)
    held = sorted(set(entry_sample(rng, items)))
    state = [{"item": item, "token": rng.pick(TOKEN_BASES) + rng.between(0, 3)} for item in held]
    if len(state) > 1 and rng.below(10) == 0:
        state = shuffled(rng, state)
    return [state, change_log(rng, items)]


def offset_table(rng: SplitMix64):
    """A time-zone table: ascending transition instants and the offset each
    starts, sometimes too short, empty, unsorted, or of two lengths."""
    base = rng.pick([0, 1_700_000_000, -(2**40), 2**53, I64_MIN + 1, I64_MAX - 100_000])
    starts, at = [], base
    for _ in range(rng.between(1, 5)):
        starts.append(at)
        at += rng.pick([1, 60, 3600, 86_400, 10_000_000])
        if at > I64_MAX:
            break
    offsets = [rng.pick([0, 3600, -18_000, 7200, 32_400, -43_200, 50_400, 64_800]) for _ in starts]
    if rng.below(20) == 0:
        starts, offsets = [], []
    elif len(starts) > 1 and rng.below(12) == 0:
        starts = shuffled(rng, starts)
    if offsets and rng.below(12) == 0:
        offsets = offsets[:-1] if rng.below(2) else offsets + [0]
    return starts, offsets


def utc_offset_args(rng: SplitMix64):
    starts, offsets = offset_table(rng)
    anchor = rng.pick(starts) if starts else 0
    t = clamp(anchor + rng.pick([0, 0, 1, -1, 30, 3600, -3600]), I64_MIN, I64_MAX) if rng.below(5) else arrival_time(rng)
    return [starts, offsets, t]


def byte_list(rng: SplitMix64, longest: int = 40) -> list:
    """Bytes: of a length at an edge of the loops that walk them, and random,
    repeated, all-ones or printable."""
    length = rng.pick([0, 1, 2, 3, 4, 5, 8, 9, 16, 31, 32, 33]) if rng.below(2) else rng.between(0, longest)
    length = min(length, longest)
    style = rng.below(4)
    if style == 0:
        return [rng.between(0, 255) for _ in range(length)]
    if style == 1:
        return [rng.pick([0, 0x80, 0xFF, 1, 0x7F]) for _ in range(length)]
    if style == 2:
        return [rng.between(32, 126) for _ in range(length)]
    return [rng.pick([0x31, 0x32, 0x33]) for _ in range(length)]


def crc_args(rng: SplitMix64):
    return [byte_list(rng)]


def index_args(rng: SplitMix64):
    """Mostly indices inside the buffer, and often enough one at or past its end or
    below its start, where the read fails."""
    data = byte_list(rng, 12)
    count = len(data)
    if count > 0 and rng.below(10) < 7:
        return [data, rng.between(0, count - 1), rng.between(0, count - 1)]
    signed = rng.pick([-1, 0, count - 1, count, count + 1, I32_MIN, I32_MAX]) if rng.below(3) == 0 else rng.between(-2, count + 2)
    unsigned = rng.pick([0, count - 1, count, count + 1, U32_MAX]) if rng.below(3) == 0 else rng.between(0, count + 2)
    return [data, clamp(signed, I32_MIN, I32_MAX), clamp(unsigned, 0, U32_MAX)]


def squeeze_args(rng: SplitMix64):
    """Bytes made of at most sixteen runs, the buffer the algorithm fills."""
    data, previous = [], None
    for _ in range(rng.between(0, 16)):
        byte = rng.between(0, 255) if rng.below(2) else rng.pick([0, 1, 2, 255])
        if byte == previous:
            byte = (byte + 1) & 0xFF
        data += [byte] * rng.pick([1, 1, 2, 3, 5])
        previous = byte
    return [data]


def list_sum_args(rng: SplitMix64):
    longest = 200 if rng.below(10) == 0 else 12
    return [[rng.pick(I32_EDGES) if rng.below(3) == 0 else rng.between(I32_MIN, I32_MAX) for _ in range(rng.between(0, longest))]]


def weekly_args(rng: SplitMix64):
    """A recurrence and a window of at most 366 days at or after day 0: the
    usual week, a window at the end of int64, and a start far enough before it
    that the difference overflows."""
    kind = rng.below(10)
    if kind < 7:
        dtstart = rng.pick([0, 1, 3, 4, 19723, 20000]) + rng.between(0, 30)
        window_start = max(0, dtstart + rng.between(-40, 40))
    elif kind < 9:
        window_start = I64_MAX - rng.between(0, 20)
        dtstart = window_start - rng.pick([0, 1, 7, 400, 2**40])
    else:
        dtstart = I64_MIN + rng.between(0, 5)
        window_start = rng.between(0, 30)
    window_end = min(window_start + rng.pick([0, 1, 7, 8, 14, 31, 100, 365, 366, rng.between(0, 366)]), I64_MAX)
    interval = rng.pick([1, 1, 2, 3, 4, 0, -1, rng.between(1, 8)])
    return [dtstart, rng.between(0, 255), interval, window_start, window_end]


ACL_SMALL = [1, 2, 3, 4, 5, 6]


def acl_tree(rng: SplitMix64) -> list:
    """A privilege tree: the root first as its own parent, each later one under an
    earlier parent — and now and then one that is not: out of order, repeated, with
    a parent nobody declares, or with no root."""
    tree, ids = [{"id": 0, "parent": 0}], [0]
    for _ in range(rng.between(0, 6)):
        fresh = rng.pick([i for i in range(1, 12) if i not in ids] + [U32_MAX - 4])
        if fresh in ids:
            continue
        tree.append({"id": fresh, "parent": rng.pick(ids)})
        ids.append(fresh)
    if rng.below(8) == 0:
        how = rng.below(4)
        if how == 0 and len(tree) > 2:
            tree = shuffled(rng, tree)
        elif how == 1 and len(tree) > 1:
            tree.append(dict(rng.pick(tree[1:])))
        elif how == 2 and len(tree) > 1:
            tree[-1] = {"id": tree[-1]["id"], "parent": 99}
        elif how == 3 and len(tree) > 1:
            tree = tree[1:]
    return tree


def principal_value(rng: SplitMix64) -> int:
    return rng.pick([ACL_ALL, ACL_AUTHENTICATED, ACL_UNAUTHENTICATED, ACL_OWNER, 0] + ACL_SMALL + ACL_SMALL)


def granted_args(rng: SplitMix64):
    tree = acl_tree(rng)
    ids = [privilege["id"] for privilege in tree] + [99]
    acl = [{"principal": principal_value(rng), "privilege": rng.pick(ids)} for _ in range(rng.between(0, 6))]
    user = rng.pick([0] + ACL_SMALL)
    groups = [rng.pick(ACL_SMALL + [0]) for _ in range(rng.between(0, 3))]
    owner = rng.pick([0] + ACL_SMALL + [ACL_OWNER, U32_MAX])
    return [tree, acl, user, groups, owner]


def membership_args(rng: SplitMix64):
    edges = [
        {"member": rng.pick(ACL_SMALL + [0]), "group": rng.pick(ACL_SMALL + [0])}
        for _ in range(rng.between(0, 10))
    ]
    return [edges, rng.pick([0] + ACL_SMALL)]


def merkle_minute(rng: SplitMix64) -> int:
    """A change's minute: a real one, near the start or the 3^16 boundary where the
    leading digit outgrows 2, and far past what a clock reads."""
    kind = rng.below(10)
    if kind < 4:
        return 29_841_720 + rng.between(-100_000, 100_000)
    if kind < 6:
        return rng.pick([0, 1, 2, 3, 8, 26, 27, 28])
    if kind < 8:
        return 3**16 + rng.between(-3, 3)
    if kind < 9:
        return rng.pick([2**40, I64_MAX // 60_000 - 1, I64_MAX // 60_000, I64_MAX // 60_000 + 1])
    return rng.pick([I64_MAX - 1, I64_MAX, 2**62])


def digest_of(changes: list) -> list:
    """The digest of a set of changes, each a (minute, hash) pair: what the host
    builds by inserting each one as it first stores it."""
    nodes: list = []
    for minute, hash_ in changes:
        nodes = answer_of(merkle_insert(nodes, minute, hash_))
    return nodes


def change_hash(rng: SplitMix64) -> int:
    return rng.pick([0, 1, U32_MAX]) if rng.below(5) == 0 else rng.between(0, U32_MAX)


def near_changes(rng: SplitMix64, count: int, base: int | None = None) -> list:
    """Changes at minutes sharing most of their base-3 prefix, so the digest has
    siblings and a path worth descending; the same `base` gives the changes of two
    replicas that part only below their shared prefix."""
    base = merkle_minute(rng) if base is None else base
    return [
        (clamp(base + rng.pick([0, 1, 2, 3, 9, 27, 243, 6561, 100_000]), 0, I64_MAX), change_hash(rng))
        for _ in range(count)
    ]


def any_changes(rng: SplitMix64, count: int) -> list:
    return [(merkle_minute(rng), change_hash(rng)) for _ in range(count)]


def merkle_insert_args(rng: SplitMix64):
    changes = near_changes(rng, rng.between(0, 4)) if rng.below(2) else any_changes(rng, rng.between(0, 3))
    nodes = digest_of(changes)
    if nodes and rng.below(10) == 0:
        nodes = shuffled(rng, nodes) if rng.below(2) else nodes + [merkle_node(17, 0, 1)]
    if changes and rng.below(4) == 0:
        minute, hash_ = rng.pick(changes)
    else:
        minute, hash_ = merkle_minute(rng), change_hash(rng)
    if rng.below(15) == 0:
        minute = -1 - rng.between(0, 5)
    return [nodes, minute, hash_]


def merkle_prune_args(rng: SplitMix64):
    nodes = digest_of(near_changes(rng, rng.between(1, 6)))
    if nodes and rng.below(12) == 0:
        nodes = shuffled(rng, nodes)
    return [nodes, rng.pick([1, 1, 2, 2, 3, 0, 1000]) if rng.below(2) else rng.between(1, 4)]


def merkle_diff_args(rng: SplitMix64):
    """Two replicas that share most of their changes and each hold a few the other
    lacks, one sometimes pruned so a child is missing on one side."""
    base = merkle_minute(rng)
    shared = near_changes(rng, rng.between(0, 4), base)
    only_a = near_changes(rng, rng.between(0, 3), base) if rng.below(4) else any_changes(rng, rng.between(0, 2))
    only_b = near_changes(rng, rng.between(0, 3), base) if rng.below(4) else any_changes(rng, rng.between(0, 2))
    a, b = digest_of(shared + only_a), digest_of(shared + only_b)
    if rng.below(3) == 0:
        b = answer_of(merkle_prune(b, rng.pick([1, 2])))
    if rng.below(6) == 0:
        a, b = b, a
    return [a, b]


def utc_local_args(rng: SplitMix64):
    starts, offsets = offset_table(rng)
    if starts and rng.below(5):
        k = rng.below(len(starts))
        shift = offsets[k] if k < len(offsets) else 0
        local = clamp(starts[k] + shift + rng.pick([0, 0, 1, -1, 30, 59, 60, 3599, 3600, -60]), I64_MIN, I64_MAX)
    else:
        local = arrival_time(rng)
    return [starts, offsets, local]


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
    "hlc_send": (lambda args: hlc_send(*args), send_args, 300, 0xE110_0016),
    "hlc_receive": (lambda args: hlc_receive(*args), receive_args, 400, 0xE110_0017),
    "hlc_within_drift": (lambda args: hlc_within_drift(*args), drift_args, 250, 0xE110_0018),
    "orset_live": (lambda args: orset_live(*args), live_args, 300, 0xE110_0019),
    "orset_contains": (lambda args: orset_contains(*args), element_args, 200, 0xE110_001A),
    "orset_observed": (lambda args: orset_observed(*args), element_args, 250, 0xE110_001B),
    "dedup_admit": (lambda args: dedup_admit(*args), admit_args, 400, 0xE110_001C),
    "dedup_holds": (lambda args: dedup_holds(*args), holds_args, 200, 0xE110_001D),
    "ordering_drain": (lambda args: ordering_drain(*args), drain_args, 300, 0xE110_001E),
    "ordering_gap_end": (lambda args: ordering_gap_end(*args), gap_end_args, 400, 0xE110_001F),
    "ordering_hold": (lambda args: ordering_hold(*args), hold_args, 400, 0xE110_0020),
    "ordering_prune": (lambda args: ordering_prune(*args), prune_args, 250, 0xE110_0021),
    "outbound_overflows": (
        lambda args: outbound_overflows(*args),
        outbound_overflows_args,
        150,
        0xE110_0022,
    ),
    "outbound_sends_now": (
        lambda args: outbound_sends_now(*args),
        outbound_sends_now_args,
        100,
        0xE110_0023,
    ),
    "outbound_stale": (lambda args: answering(outbound_stale)(*args), outbound_stale_args, 300, 0xE110_0024),
    "sync_failure": (lambda args: sync_failure(*args), failure_args, 300, 0xE110_0025),
    "sync_delete_outcome": (lambda args: sync_delete_outcome(*args), failure_args, 300, 0xE110_0026),
    "sync_upload_outcome": (lambda args: sync_upload_outcome(*args), upload_args, 300, 0xE110_0027),
    "precondition": (lambda args: http_precondition(*args), http_precondition_args, 400, 0xE110_0028),
    "mailbox_ack": (lambda args: mailbox_ack(*args), mailbox_ack_args, 300, 0xE110_0029),
    "mailbox_assign": (lambda args: mailbox_assign(*args), mailbox_assign_args, 400, 0xE110_002A),
    "mailbox_insert": (lambda args: mailbox_insert(*args), mailbox_assign_args, 400, 0xE110_002B),
    "mailbox_page": (lambda args: mailbox_page(*args), mailbox_page_args, 400, 0xE110_002C),
    "changes_since": (lambda args: changes_since(*args), changes_since_args, 400, 0xE110_002D),
    "changes_apply": (lambda args: changes_apply(*args), changes_apply_args, 400, 0xE110_002E),
    "utc_offset_at": (lambda args: utc_offset_at(*args), utc_offset_args, 400, 0xE110_002F),
    "utc_from_local": (lambda args: answering(utc_from_local)(*args), utc_local_args, 500, 0xE110_0030),
    "algorithm_crc16": (lambda args: crc16_ccitt_false(*args), crc_args, 300, 0xE110_0031),
    "algorithm_crc16_table": (lambda args: crc16_ccitt_false(*args), crc_args, 300, 0xE110_0032),
    "algorithm_checked_index": (lambda args: checked_index(*args), index_args, 300, 0xE110_0033),
    "algorithm_bytes_squeeze": (lambda args: bytes_squeeze(*args), squeeze_args, 300, 0xE110_0034),
    "algorithm_list_param_sum": (lambda args: list_param_sum(*args), list_sum_args, 200, 0xE110_0035),
    "algorithm_hlc_compare": (lambda args: legacy_hlc_compare(*args), compare_args, 200, 0xE110_0036),
    "algorithm_hlc_tick": (lambda args: hlc_send(*args), send_args, 300, 0xE110_0037),
    "algorithm_weekly_expand": (lambda args: answering(weekly_expand)(*args), weekly_args, 500, 0xE110_0038),
    "acl_membership": (lambda args: acl_membership(*args), membership_args, 300, 0xE110_0039),
    "acl_granted": (lambda args: acl_granted(*args), granted_args, 400, 0xE110_003A),
    "merkle_insert": (lambda args: merkle_insert(*args), merkle_insert_args, 400, 0xE110_003B),
    "merkle_prune": (lambda args: merkle_prune(*args), merkle_prune_args, 300, 0xE110_003C),
    "merkle_diff": (lambda args: answering(merkle_diff)(*args), merkle_diff_args, 500, 0xE110_003D),
    "field_version_compare": (
        lambda args: field_version_compare(*args),
        version_compare_args,
        250,
        0xE110_003E,
    ),
    "undo_field_plan": (lambda args: undo_field_plan(*args), plan_args, 400, 0xE110_003F),
    "undo_set_readds": (lambda args: undo_set_readds(*args), readds_args, 300, 0xE110_0040),
}


LAW_RUNS = 600


class Law:
    """One claim a document makes about itself, held to the model on random inputs.

    The claims are the ones the documents' own prose states — a union that
    commutes, a log whose answer does not depend on where it was cut, a page that
    continues from the last id — and they are what a backend agreeing with the
    model on a thousand cases still does not prove. A model that refutes one says
    the document's claim is false of the algorithm it wrote."""

    def __init__(self, name: str) -> None:
        self.name = name
        self.ran = 0

    def holds(self, condition: bool, inputs) -> None:
        self.ran += 1
        if not condition:
            raise SystemExit(f"law refuted — {self.name}: {json.dumps(inputs)}")

    def was_asked(self, at_least: int = LAW_RUNS // 5) -> None:
        """A law that was asked of too few inputs proves nothing."""
        if self.ran < at_least:
            raise SystemExit(f"law `{self.name}` was asked of only {self.ran} inputs")


def answer_of(result):
    """The value of a model's answer, or `None` where the document fails."""
    return result[1] if result[0] == "ok" else None


def validate_laws() -> None:
    """Hold each document's claims about itself to its model (see [`Law`])."""
    rng = SplitMix64(0xE110_1A75)
    union_laws = [Law(f"orset_union {name}") for name in ("commutes", "associates", "is idempotent", "keeps a")]
    prune_law = Law("orset_live: pruning what every replica observed leaves the set's value")
    order_laws = [Law(f"hlc_compare {name}") for name in ("is antisymmetric", "is transitive", "ties only the same stamp")]
    clock_laws = [Law("hlc_send sorts after prev"), Law("hlc_receive sorts after prev and msg")]
    classify_law = Law("lww_classify does not depend on the order logged")
    apply_laws = [Law("changes_apply: where the log is cut does not matter"), Law("changes_apply is idempotent")]
    ack_laws = [Law("mailbox_ack is idempotent"), Law("mailbox_ack ignores the order acknowledged")]
    page_law = Law("mailbox_page: pages from the last id read the whole queue")
    insert_law = Law("mailbox_insert holds the message once, under the id mailbox_assign gives")
    dedup_law = Law("dedup_admit keeps the window within capacity and the id last")
    hold_law = Law("ordering_hold keeps the held slots ascending and holds the arrival once")
    gcra_law = Law("gcra_admit admits exactly `burst` permits of a fresh limiter at one instant")
    retry_law = Law("sync_retry_at never answers before `previous`")
    utc_law = Law("utc_from_local names an instant that reads as the local time")

    for _ in range(LAW_RUNS):
        pool = entry_pool(rng)
        a, b, c = entry_sample(rng, pool), entry_sample(rng, pool), entry_sample(rng, pool)

        def union(x, y):
            return orset_union(x, y)[1]

        def as_set(entries):
            return sorted(json.dumps(entry_, sort_keys=True) for entry_ in entries)

        union_laws[0].holds(as_set(union(a, b)) == as_set(union(b, a)), [a, b])
        union_laws[1].holds(as_set(union(union(a, b), c)) == as_set(union(a, union(b, c))), [a, b, c])
        union_laws[2].holds(union(a, a) == a, a)
        union_laws[3].holds(union(a, []) == a and as_set(union([], a)) == as_set(a), a)

        adds, tombstones = entry_sample(rng, pool), entry_sample(rng, pool)
        observed = [t for t in tombstones if rng.below(2)]
        live = lambda x, y: orset_live(x, y)[1]
        prune_law.holds(
            as_set(live(live(adds, observed), live(tombstones, observed))) == as_set(live(adds, tombstones)),
            [adds, tombstones, observed],
        )

        first = stamp_value(rng)
        second, third = near_stamp(rng, first), near_stamp(rng, first)
        sign = lambda x, y: hlc_compare(x, y)[1]
        order_laws[0].holds(sign(first, second) == -sign(second, first), [first, second])
        for x, y, z in itertools.permutations([first, second, third]):
            if sign(x, y) <= 0 and sign(y, z) <= 0:
                order_laws[1].holds(sign(x, z) <= 0, [x, y, z])
        order_laws[2].holds((sign(first, second) == 0) == (first == second), [first, second])

        sent = answer_of(hlc_send(first, clock_near(rng, first["wallTime"]), rng.between(0, 5)))
        if sent is not None:
            clock_laws[0].holds(sign(sent, first) == 1, first)
        message = near_stamp(rng, first)
        received = answer_of(
            hlc_receive(first, message, clock_near(rng, first["wallTime"]), rng.between(0, 5))
        )
        if received is not None:
            clock_laws[1].holds(sign(received, first) == 1 and sign(received, message) == 1, [first, message])

        logged = [stamp_value(rng) for _ in range(rng.between(0, 5))]
        stamped = rng.pick(logged + [stamp_value(rng)])
        classify_law.holds(
            lww_classify(logged, stamped) == lww_classify(shuffled(rng, logged), stamped), [logged, stamped]
        )

    for _ in range(LAW_RUNS):
        items = change_items(rng)
        state = changes_apply_args(rng)[0]
        log = change_log(rng, items)
        cut = rng.between(0, len(log))
        whole = answer_of(changes_apply(state, log))
        head = answer_of(changes_apply(state, log[:cut]))
        if whole is not None and head is not None:
            apply_laws[0].holds(answer_of(changes_apply(head, log[cut:])) == whole, [state, log, cut])
            apply_laws[1].holds(answer_of(changes_apply(whole, log)) == whole, [state, log])

        queue = mailbox_queue(rng)
        acked = [{"hi": held["hi"], "lo": held["lo"]} for held in entry_sample(rng, queue)] + [envelope_id(rng)]
        once = answer_of(mailbox_ack(queue, acked))
        ack_laws[0].holds(answer_of(mailbox_ack(once, acked)) == once, [queue, acked])
        ack_laws[1].holds(answer_of(mailbox_ack(queue, shuffled(rng, acked))) == once, [queue, acked])

        limit = rng.between(1, 4)
        if answer_of(mailbox_page(queue, -1, 256)) is not None:
            after, collected = -1, []
            for _ in range(len(queue) + 2):
                page = answer_of(mailbox_page(queue, after, limit))
                if not page:
                    break
                collected += page
                after = page[-1]["id"]
            # A reader that never reaches an empty page has not read the queue: the
            # loop above is bounded so that this is a refutation and not a hang.
            page_law.holds(collected == queue and not page, [queue, limit])

        counter = mailbox_counter(rng, queue)
        key = message_key(rng, queue)
        inserted = answer_of(mailbox_insert(queue, counter, key["hi"], key["lo"]))
        assigned = answer_of(mailbox_assign(queue, counter, key["hi"], key["lo"]))
        keys = [(held["hi"], held["lo"]) for held in queue]
        if inserted is not None and assigned is not None and len(set(keys)) == len(keys):
            # A mailbox holds each message once; a queue with one twice is not one
            # the host kept, and the two documents answer it differently (the
            # first of the copies, the last) without either being wrong.
            mine = [held for held in inserted if held["hi"] == key["hi"] and held["lo"] == key["lo"]]
            insert_law.holds(len(mine) == 1 and mine[0]["id"] == assigned, [queue, counter, key])

        window = id_window(rng)
        ident = rng.pick(window) if window and rng.below(2) else envelope_id(rng)
        capacity = max(1, len(window) + rng.pick([0, 1, 3]))
        admitted = answer_of(dedup_admit(window, ident, capacity))
        if admitted is not None:
            held_before = ident in window
            dedup_law.holds(
                len(admitted) <= capacity
                and (admitted == window if held_before else admitted[-1] == ident),
                [window, ident, capacity],
            )

        seq_base, slots = held_slots(rng)
        ascending = sorted({held["seq"]: held for held in slots}.values(), key=lambda held: held["seq"])
        arrival = sequence_near(rng, seq_base, ascending)
        held_now = answer_of(ordering_hold(ascending, arrival, arrival_time(rng)))
        ordered = [held["seq"] for held in held_now]
        hold_law.holds(
            ordered == sorted(set(ordered)) and ordered.count(arrival) == 1, [ascending, arrival]
        )

        interval = rng.between(1, 10_000)
        burst = rng.between(1, 12)
        now = REALISTIC_WALL + rng.between(-1000, 1000)
        tat, granted = now - rng.between(0, burst * interval * 2), 0
        for _ in range(burst + 3):
            decision = answer_of(gcra_admit(tat, now, 1, interval, burst))
            if decision["admitted"]:
                granted += 1
                tat = decision["tat"]
        gcra_law.holds(granted == burst, [interval, burst, now])

        retry_args = sync_retry_args(rng)
        retried = answer_of(sync_retry_at(*retry_args))
        if retried is not None:
            retry_law.holds(retried >= retry_args[0], retry_args)

        starts, offsets = offset_table(rng)
        if starts and len(starts) == len(offsets):
            instant = starts[0] + rng.between(0, 90_000_000)
            offset = answer_of(utc_offset_at(starts, offsets, instant))
            if offset is not None:
                named = answer_of(answering(utc_from_local)(starts, offsets, instant + offset))
                if named is not None:
                    back = answer_of(utc_offset_at(starts, offsets, named))
                    utc_law.holds(
                        named <= instant and back is not None and named + back == instant + offset,
                        [starts, offsets, instant],
                    )

    insert_order_law = Law("merkle_insert: the order the changes are inserted in does not matter")
    cancel_law = Law("merkle_insert: a change inserted twice cancels itself on every node")
    prune_keeps_law = Law("merkle_prune only drops nodes, keeps the rest in order, and keeps the root")
    diff_bound_law = Law("merkle_diff never answers later than the earliest change one side lacks")
    diff_pruned_law = Law("merkle_diff still tells a pruned digest apart from one that differs")
    membership_law = Law("acl_membership lists each group once and never the user")
    for _ in range(LAW_RUNS):
        # Distinct nonzero hashes: a change whose hash is 0, or two on opposite sides
        # whose hashes are equal, cancel out of an XOR digest, and no digest of this
        # family can see them — a limit of the structure, not of a document.
        hashes = shuffled(rng, [rng.between(1, U32_MAX) for _ in range(12)])
        if len(set(hashes)) < len(hashes):
            continue
        base = merkle_minute(rng)
        minutes = [clamp(base + rng.pick([0, 1, 2, 3, 9, 27, 243, 6561, 100_000]), 0, I64_MAX) for _ in range(9)]
        pairs = list(dict.fromkeys(zip(minutes, hashes)))
        changes = pairs[: rng.between(1, 5)]
        forward = digest_of(changes)
        insert_order_law.holds(forward == digest_of(shuffled(rng, changes)), changes)
        again = answer_of(merkle_insert(forward, changes[0][0], changes[0][1]))
        rest = {(node["depth"], node["prefix"]): node["hash"] for node in digest_of(changes[1:])}
        cancel_law.holds(
            all(node["hash"] == rest.get((node["depth"], node["prefix"]), 0) for node in again)
            and set(rest) <= {(node["depth"], node["prefix"]) for node in again},
            changes,
        )

        pruned = answer_of(merkle_prune(forward, rng.between(1, 3)))
        kept = [node for node in forward if node in pruned]
        prune_keeps_law.holds(
            pruned == kept and any(node["depth"] == 0 for node in pruned) == any(node["depth"] == 0 for node in forward),
            [forward],
        )

        left = pairs[: rng.between(0, len(pairs))]
        right = pairs[rng.between(0, len(pairs)) :]
        differing = set(left) ^ set(right)
        b_digest = digest_of(right)
        pruned_side = rng.below(3) == 0
        if pruned_side:
            b_digest = answer_of(merkle_prune(b_digest, rng.between(1, 2)))
        divergence = answer_of(answering(merkle_diff)(digest_of(left), b_digest))
        if divergence is not None and differing:
            earliest = min(minute for minute, _ in differing)
            if pruned_side:
                # Pruning leaves the root as it was, so the sides are still told apart;
                # how early the answer is is not promised for a branch that only the
                # pruned side held and dropped — see merkle_diff.scxml.
                diff_pruned_law.holds(divergence["differs"], [left, right])
            else:
                diff_bound_law.holds(
                    divergence["differs"] and divergence["millis"] <= earliest * 60_000,
                    [left, right],
                )

        edges = membership_args(rng)[0]
        user = rng.between(1, 6)
        groups = answer_of(acl_membership(edges, user))
        membership_law.holds(len(groups) == len(set(groups)) and user not in groups and 0 not in groups, [edges, user])

    every_law = (
        union_laws + [prune_law] + order_laws + clock_laws + [classify_law] + apply_laws + ack_laws
        + [page_law, insert_law, dedup_law, hold_law, gcra_law, retry_law, utc_law]
        + [insert_order_law, cancel_law, prune_keeps_law, diff_bound_law, diff_pruned_law, membership_law]
    )
    for law in every_law:
        law.was_asked()


def after_stamp(stamp: dict):
    """A stamp later than `stamp`, or None where nothing is above it."""
    if stamp["nodeId"] < U64_MAX:
        return hlc(stamp["wallTime"], stamp["counter"], stamp["nodeId"] + 1)
    if stamp["counter"] < U32_MAX:
        return hlc(stamp["wallTime"], stamp["counter"] + 1, 0)
    if stamp["wallTime"] < I64_MAX:
        return hlc(stamp["wallTime"] + 1, 0, 0)
    return None


def validate_undo_laws() -> None:
    """The claims `undo_field_plan` and `undo_set_readds` make about themselves
    (see [`Law`]); the history comes from the same generator the cases do."""
    rng = SplitMix64(0xE110_20D0)
    again_law = Law("undo_field_plan: once its undoing writes are in, the batch reads as already undone")
    order_law = Law("undo_field_plan does not depend on the order of the history")
    fields_law = Law("undo_field_plan plans each field the batch wrote once and no other")
    value_law = Law("undo_field_plan restores the value the field held before the batch's earliest write")
    safe_law = Law("undo_field_plan never undoes a field someone else wrote after the batch")
    readds_law = Law("undo_set_readds brings each removed element back once and none the set holds")

    def by_field(steps):
        return sorted(steps, key=lambda step: (step["entityId"], step["fieldId"]))

    for _ in range(LAW_RUNS * 2):
        batch = rng.pick(UNDO_BATCHES)
        versions = history_for(rng, batch)
        floor = rng.pick([0, 0, 0, U64_MAX])
        plan = answer_of(undo_field_plan(versions, batch, floor))
        if plan is None:
            continue
        inputs = [versions, batch, floor]

        written = {(w["entityId"], w["fieldId"]) for w in versions if w["batchId"] == batch}
        planned = [(step["entityId"], step["fieldId"]) for step in plan]
        fields_law.holds(len(planned) == len(set(planned)) and set(planned) == written, inputs)

        reordered = answer_of(undo_field_plan(shuffled(rng, versions), batch, floor))
        order_law.holds(by_field(reordered) == by_field(plan), inputs)

        applied = list(versions)
        room = True
        for step in plan:
            field = [
                w for w in versions
                if w["entityId"] == step["entityId"] and w["fieldId"] == step["fieldId"]
            ]
            latest = max(field, key=hlc_key)
            batch_writes = [w for w in field if w["batchId"] == batch]
            earliest = min(batch_writes, key=hlc_key)
            earlier = [w for w in field if hlc_key(w) < hlc_key(earliest)]
            before = max(earlier, key=hlc_key)["valueRef"] if earlier else 0

            if step["verdict"] == 0:
                value_law.holds(step["restoreRef"] == before, [inputs, step])
                safe_law.holds(latest["batchId"] == batch, [inputs, step])
                stamp = after_stamp(hlc(latest["wallTime"], latest["counter"], latest["nodeId"]))
                if stamp is None:
                    room = False
                else:
                    applied.append(
                        field_write(
                            step["entityId"], step["fieldId"], stamp,
                            min(max(w["arrival"] for w in versions) + 1, U64_MAX),
                            batch + 1 if batch < U64_MAX else batch - 1,
                            2, batch, step["restoreRef"],
                        )
                    )
            elif step["verdict"] == 1:
                safe_law.holds(latest["batchId"] != batch, [inputs, step])
            else:
                safe_law.holds(step["verdict"] in (2, 3), [inputs, step])
        if room and any(step["verdict"] == 0 for step in plan):
            again = answer_of(undo_field_plan(applied, batch, floor))
            expected = [dict(step, verdict=2 if step["verdict"] == 0 else step["verdict"],
                             restoreRef=0 if step["verdict"] == 0 else step["restoreRef"])
                        for step in plan]
            again_law.holds(by_field(again) == by_field(expected), [inputs, applied])

    for _ in range(LAW_RUNS):
        removed, live = readds_args(rng)
        if len(removed) > UNDO_PLAN_MAX:
            continue
        back = answer_of(undo_set_readds(removed, live))
        held = {entry_["element"] for entry_ in live}
        wanted = {entry_["element"] for entry_ in removed} - held
        readds_law.holds(len(back) == len(set(back)) and set(back) == wanted, [removed, live])

    for law in (again_law, order_law, fields_law, value_law, safe_law, readds_law):
        law.was_asked()


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


def case_objects(body: str):
    """The top-level objects of a `cases` body, in order, each as the text it is
    written in — from the start of its first line to its closing brace — whether
    it sits on one line or several, so a hand-written case is carried over as it
    was written."""
    found, depth, begin, in_string, escaped = [], 0, 0, False, False
    for index, char in enumerate(body):
        if in_string:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == '"':
                in_string = False
        elif char == '"':
            in_string = True
        elif char == "{":
            if depth == 0:
                begin = body.rfind("\n", 0, index) + 1
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                found.append(body[begin : index + 1])
    return found


def hand_cases(body: str):
    """The hand-written cases of a `cases` body, each as its text, and how many
    generated cases it held — which regeneration drops."""
    hand, generated = [], 0
    for text in case_objects(body):
        if str(json.loads(text).get("note", "")).startswith(MARK):
            generated += 1
        else:
            hand.append(text)
    return hand, generated


def verify_model(fixture: str, model, cases) -> int:
    """Hold the model to every hand-written case of `fixture`."""
    checked = 0
    for text in cases:
        case = json.loads(text)
        args = case["args"]
        answer = model(args)
        if "fails" in case:
            ok = answer == ("fails", case["fails"])
        else:
            ok = answer[0] == "ok" and answer[1] == case["expected"]
        if not ok:
            raise SystemExit(
                f"{fixture}: the model answers {answer} for {args}, but the hand case "
                f"says {' '.join(text.split())} — the model is wrong, since every backend "
                "holds the case"
            )
        checked += 1
    return checked


def regenerate(text: str):
    report = []
    for fixture, (model, draw, count, seed) in FIXTURES.items():
        start, end = section(text, fixture)
        hand, dropped = hand_cases(text[start:end])
        checked = verify_model(fixture, model, hand)
        seen = {identity(json.loads(text)["args"]) for text in hand}
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
        body = "\n" + ",\n".join(hand + made)
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
    validate_laws()
    validate_undo_laws()
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
