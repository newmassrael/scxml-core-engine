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

Eighth run, 2026-10-15: the three `text_slot*` documents, whose cases had been written
by hand — `text_slot_latest`, `text_slot_retitled` and `text_slot_holds`, 1400 cases.
The texts are UTF-8 and the bound is in bytes, so they are drawn at every encoded
width, at exactly the 32-byte bound and just under it, empty, equal, and as the
precomposed and decomposed forms of one letter; a text past the bound is never drawn,
since the schema refuses it where it is written and no document answers it. All six
backends agreed with the model on every case. Characters with no visible form are
written as `\\u` escapes, the rest as themselves.

Ninth run, 2026-10-15: the three `mv_value_*` documents, which keep the values a field
of a shared to-do list holds beside the dots of its writes — `mv_value_holds`,
`mv_value_keep` and `mv_value_drop`, 1300 cases. A write is named by its row, its field
and its dot, so the lists are drawn from a few of each part and a probe differs from a
value of the list in exactly one of them, in only the number and the text it carries
(the same write), or not at all; the texts are the ones the `text_slot` cases use, at
every encoded width and at the 32-byte bound. A list one short of the 512 values the
result holds is drawn now and then. Nine laws are checked on the model before any case
is written.

Tenth run, 2026-10-15: the reference policy of SCE_FORGE.md section 5.6 in the form the
harness can hold, a record of numbers and flags — `policy_scalar_layer` (500 cases) and
`policy_scalar_rule` (300, a stateless validator whose `sequence` the model extends the
way it extends a `cases` array). Masks are drawn over the three bits that name a field,
over bits that name none (up to the widest `uint64`) and as single bits; a limit is drawn
at each edge of the 0 to 365 range the rule holds it to, inside it and anywhere a
`uint16` can be. Nine laws are checked on the model before any case is written, among
them that folding under one mask does not depend on how it is grouped, and one fact the
document's host step rests on: two policies that each pass the rule can fold into one
that does not. Rust, Python, Go, Kotlin, C++ and C11 all agreed with the model on every
case.

Eleventh run, 2026-10-20: `register_values` of sce:std/shared, the values of one
register of a shared record list in the order of their dots, which the list tells the
statechart that invoked it in (docs/adr/0042) — 400 cases over the lists the
`mv_value_*` cases draw, with the register read most of the time one the list holds and
otherwise one drawn afresh. The order is the wall time, then the counter, then the
node, so a list is drawn with registers whose dots share one or two of the three.
"""

from __future__ import annotations

import calendar
import datetime
import itertools
import json
import math
import re
import sys
import unicodedata
from pathlib import Path

import content_line_model

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


# ── Wrapping and saturating operations (SCE_FORGE.md 3.4.2) ───────────────
#
# docs/adr/0063 defines each one as the exact result in Z brought into the
# type: modulo 2^W into [m, M] for wrapping, clamped to [m, M] for saturating.
# Python's integers are exact, so the model is the definition itself and
# shares no overflow test with any backend's runtime.


def ring_range(signed: bool, bits: int):
    """The [m, M] of an integer type."""
    if signed:
        half = 2 ** (bits - 1)
        return -half, half - 1
    return 0, 2**bits - 1


def total_arith(signed: bool, bits: int, guarded: bool):
    """algorithm_total_arith_<type>: op 0..3 wrapping add, sub, mul, neg; 4..6
    saturating add, sub, mul; 7 saturating neg of a signed type; any other op
    0. A `guarded` document takes its operands as int32 and answers 0 for one
    its type does not hold."""
    lo, hi = ring_range(signed, bits)

    def wrap(e: int) -> int:
        return (e - lo) % (hi - lo + 1) + lo

    def saturate(e: int) -> int:
        return max(lo, min(hi, e))

    def model(a: int, b: int, op: int):
        if guarded and not (fits(a, lo, hi) and fits(b, lo, hi)):
            return ("ok", 0)
        exact = {0: a + b, 1: a - b, 2: a * b, 3: -a, 4: a + b, 5: a - b, 6: a * b, 7: -a}
        if op <= 3:
            return ("ok", wrap(exact[op]))
        if op <= 6 or (op == 7 and signed):
            return ("ok", saturate(exact[op]))
        return ("ok", 0)

    return model


def tick_elapsed(now: int, last: int, timeout: int):
    """algorithm_tick_elapsed: the ticks since `last` on a counter that wraps
    at 2^32, at least `timeout`."""
    return ("ok", (now - last) % 2**32 >= timeout)


def tick_until_due(now: int, last: int, period: int):
    """algorithm_tick_until_due: what remains of `period` since `last`, held
    at 0 once it is over."""
    return ("ok", max(0, period - (now - last) % 2**32))


def lcg_table(n: int):
    """algorithm_lcg_table: the (n mod 16)-th state of x' = 1664525 x +
    1013904223 (mod 2^32) from 12345."""
    x = 12345
    for _ in range(n % 16):
        x = (1664525 * x + 1013904223) % 2**32
    return ("ok", x)


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


def total_arith_args(signed: bool, bits: int, guarded: bool):
    """Arguments for one `algorithm_total_arith_<type>`: mostly the type's own
    edges, where a wrap or a saturation starts — m, M, their neighbours, 0,
    ±1, and the square roots of the bounds, whose products cross them — and
    sometimes any value of the type, an op the document does not define, or,
    for a guarded document, an operand its type does not hold."""
    lo, hi = ring_range(signed, bits)
    root = 2 ** ((bits - 1) // 2 if signed else bits // 2)
    edges = {lo, lo + 1, hi - 1, hi, 0, 1, 2, root - 1, root, root + 1}
    if signed:
        edges |= {-1, -2, -root, -root - 1}
    edges = sorted(e for e in edges if fits(e, lo, hi))
    defined = 8 if signed else 7

    def value(rng: SplitMix64) -> int:
        kind = rng.below(10)
        if kind < 5:
            return rng.pick(edges)
        if kind < 7:
            return rng.between(max(lo, -300), min(hi, 300))
        return rng.between(lo, hi)

    def generate(rng: SplitMix64):
        op = rng.between(0, defined - 1) if rng.below(10) else rng.pick([defined, 9, 200, U8_MAX])
        a, b = value(rng), value(rng)
        if guarded and rng.below(8) == 0:
            outside = rng.pick([lo - 1, hi + 1, I32_MIN, I32_MAX])
            if rng.below(2):
                a = outside
            else:
                b = outside
        return [a, b, op]

    return generate


TICK_EDGES = [0, 1, 2, 5, 100, 1000, 2**31 - 1, 2**31, U32_MAX - 1, U32_MAX]


def tick_args(rng: SplitMix64):
    """A counter reading `now`, the `last` one, and a bound: half the time
    `now` is a few ticks either side of `last + bound`, wrapped, which is
    where the answer changes; the rest any readings at all."""
    last = rng.pick(TICK_EDGES) if rng.below(2) else rng.between(0, U32_MAX)
    bound = rng.pick(TICK_EDGES) if rng.below(2) else rng.between(0, U32_MAX)
    if rng.below(2):
        now = (last + bound + rng.between(-3, 3)) % 2**32
    else:
        now = rng.pick(TICK_EDGES) if rng.below(2) else rng.between(0, U32_MAX)
    return [now, last, bound]


def lcg_args(rng: SplitMix64):
    return [rng.between(0, U8_MAX)]


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


def stable_point(cursors: list, last_seen: list, now: int, max_idle: int):
    """sce:std/merge/stable_point — the least cursor among the devices whose
    cursor moved within `max_idle` of `now`, 0 where none did. The idle time is a
    checked int64 difference, taken device by device, so a reading that cannot be
    subtracted fails `overflow` where the loop reaches it."""
    if len(cursors) != len(last_seen) or max_idle < 0:
        return ("fails", "precondition")
    stable, taking = 0, False
    for cursor, seen in zip(cursors, last_seen):
        idle = now - seen
        if not fits(idle, I64_MIN, I64_MAX):
            return ("fails", "overflow")
        if idle <= max_idle:
            if not taking or cursor < stable:
                stable = cursor
            taking = True
    return ("ok", stable)


def utf8_length(data: list):
    """sce:std/text/utf8_length — the code points of a UTF-8 byte string. The
    judge of what is UTF-8 is Python's strict decoder, which holds RFC 3629 as the
    document does (no overlong form, no surrogate, nothing past U+10FFFF) but is
    not the document's own table of ranges, so the two can disagree."""
    try:
        return ("ok", len(bytes(data).decode("utf-8")))
    except UnicodeDecodeError:
        return ("fails", "precondition")


def utf8_within(data: list, limit: int):
    """sce:std/text/utf8_within — at most `limit` code points; not UTF-8 fails."""
    counted = utf8_length(data)
    if counted[0] == "fails":
        return counted
    return ("ok", counted[1] <= limit)


#: The `sce:max-size` of `sce:std/text/text_slot`'s text, in UTF-8 bytes, and the
#: most places `text_slot_retitled` holds in its result.
TEXT_BOUND = 32
SLOT_CAPACITY = 16
UNTITLED = "untitled"


def titled_slot(place: int, version: int, text: str) -> dict:
    return {"slot": place, "version": version, "text": text}


def text_slot_latest(a: dict, b: dict):
    """sce:std/text/text_slot_latest — `a`, moved to `b`'s version and text when
    `b`'s version is greater; equal versions keep `a`, and the place is `a`'s. A
    text is only ever one the schema's bound already holds, so nothing fails."""
    if b["version"] > a["version"]:
        return ("ok", titled_slot(a["slot"], b["version"], b["text"]))
    return ("ok", dict(a))


def text_slot_retitled(slots: list):
    """sce:std/text/text_slot_retitled — each place in order, an empty text titled
    "untitled", the rest carried over. The result holds 16 places at most; a list
    past that is where the bounded backends fail and the heap backends grow, so a
    case never reaches it."""
    assert len(slots) <= SLOT_CAPACITY
    return ("ok", [dict(held, text=held["text"] or UNTITLED) for held in slots])


def text_slot_holds(slots: list, probe: dict):
    """sce:std/text/text_slot_holds — some place has the probe's slot and its
    text, byte for byte; the version is not compared. Two strings of one encoding
    are equal in code points exactly when they are equal in bytes, so the
    precomposed and the decomposed "e with acute" stay two."""
    return ("ok", any(held["slot"] == probe["slot"] and held["text"] == probe["text"] for held in slots))


#: The variants of `enum_task_phase`, by the names its document declares, each with
#: the one `algorithm_task_phase_advance` moves it to; and the most tasks that
#: algorithm holds in its result.
TASK_NEXT_PHASE = {"todo": "in_review", "in_review": "shipped", "shipped": "shipped"}
TASK_CAPACITY = 4


def task_record(task_id: int, phase: str) -> dict:
    return {"taskId": task_id, "phase": phase}


def task_phase_advance(tasks: list):
    """algorithm_task_phase_advance — each task in order, one phase on, a shipped
    task staying shipped. A phase is the name its variant declares, which is what
    the cases write and every backend's harness builds the enum field from. The
    result holds four tasks at most; a list past that is where the bounded backends
    fail and the heap backends grow, so a case never reaches it."""
    assert len(tasks) <= TASK_CAPACITY
    return ("ok", [task_record(held["taskId"], TASK_NEXT_PHASE[held["phase"]]) for held in tasks])


def task_phase_reopened(task: dict):
    """algorithm_task_phase_reopened — a shipped task back to `todo`, any other as
    it was."""
    phase = "todo" if task["phase"] == "shipped" else task["phase"]
    return ("ok", task_record(task["taskId"], phase))


#: The most values `mv_value_keep` and `mv_value_drop` hold in their result.
MV_CAPACITY = 512
#: The fields that name a write: the row and field it is to, and its dot. The number
#: and the text are what the write carries, and are not part of which write it is.
MV_WRITE = ("id", "field", "wallTime", "counter", "nodeId")


def mv_value(row: int, field: int, number: int, text: str, wall: int, counter: int, node: int) -> dict:
    return {
        "id": row, "field": field, "value": number, "text": text,
        "wallTime": wall, "counter": counter, "nodeId": node,
    }


def same_write(held: dict, other: dict) -> bool:
    return all(held[name] == other[name] for name in MV_WRITE)


def mv_value_holds(values: list, probe: dict):
    """sce:std/merge/mv_value_holds — some value is the write the probe names: the
    same row and field and the same dot. The number and the text are not compared."""
    return ("ok", any(same_write(held, probe) for held in values))


def mv_value_keep(values: list, held: dict):
    """sce:std/merge/mv_value_keep — the values, in order, then `held` unless a value
    of the same write is already there. The result holds 128 values at most; a list
    past that is where the bounded backends fail and the heap backends grow, so a
    case never reaches it."""
    if any(same_write(there, held) for there in values):
        return ("ok", [dict(there) for there in values])
    assert len(values) < MV_CAPACITY
    return ("ok", [dict(there) for there in values] + [dict(held)])


def mv_value_drop(values: list, gone: dict):
    """sce:std/merge/mv_value_drop — every value but the one of the write `gone`
    names, in the order the values had."""
    return ("ok", [dict(there) for there in values if not same_write(there, gone)])


def register_values(values: list, row: int, field: int):
    """sce:std/shared/register_values — the values of the register of `row` and
    `field`, in the order of their dots: the earlier wall time first, then the
    earlier counter, then the lower node. Values of one dot keep the order the list
    had them in, which a list mv_value_keep made never holds twice."""
    held = [dict(there) for there in values if there["id"] == row and there["field"] == field]
    return ("ok", sorted(held, key=lambda there: (there["wallTime"], there["counter"], there["nodeId"])))


#: The range `policy_scalar_rule` holds `limitDays` to, inclusive.
POLICY_LIMIT_MAX = 365


def policy_scalar_layer(base: dict, over: dict, mask: int):
    """tests/forge/resources/policy_scalar_layer.scxml — a policy layered over
    another. `strict` is set when either layer sets it, whatever the mask says;
    the other three fields follow bits 0, 1 and 2 of the mask, from `over` when
    the bit is set and from `base` when it is not. A higher bit names no field."""
    return (
        "ok",
        {
            "strict": over["strict"] or base["strict"],
            "limitDays": over["limitDays"] if mask & 1 else base["limitDays"],
            "burst": over["burst"] if mask & 2 else base["burst"],
            "locked": over["locked"] if mask & 4 else base["locked"],
        },
    )


def policy_scalar_rule(strict: bool, limit_days: int):
    """tests/forge/resources/policy_scalar_rule.scxml — a policy is refused when
    its limit is out of range, and then when it is strict with no limit; the
    range is judged first, so that is the reason a strict policy with a limit out
    of range gives."""
    if not 0 <= limit_days <= POLICY_LIMIT_MAX:
        return ("ok", {"valid": False, "reason": "limit_days_out_of_range"})
    if strict and limit_days == 0:
        return ("ok", {"valid": False, "reason": "plausibility_failed"})
    return ("ok", {"valid": True, "reason": ""})


def entry(element: int, wall: int, counter: int, node: int) -> dict:
    return {"element": element, "wallTime": wall, "counter": counter, "nodeId": node}


def orset_union(a: list, b: list):
    """sce:std/merge/orset_union — a, then b's entries that a does not hold."""
    return ("ok", a + [other for other in b if other not in a])


#: What a cell that holds a number carries in the fields of a formula.
NUMBER = {"op": 0, "aRef": False, "aRow": 0, "aCol": 0, "aVal": 0, "bRef": False, "bRow": 0, "bCol": 0, "bVal": 0}


def cell(row: int, col: int, value: int, stamp: dict, held: dict = NUMBER) -> dict:
    """A cell of a grid: the number `value`, or the formula `held` carries, with
    the stamp of the write that put it there."""
    return {"row": row, "col": col, "value": value, **stamp, **held}


def reads(row: int, col: int) -> tuple:
    """An operand that names the cell at a place."""
    return ("ref", row, col)


def literal(number: int) -> tuple:
    """An operand that is a number."""
    return ("lit", number)


def formula(op: int, a: tuple = ("lit", 0), b: tuple = ("lit", 0)) -> dict:
    """The fields of a formula: operation `op` over the operands a and b. An
    operand the operation does not read is held as the literal 0."""
    held = {"op": op}
    for side, operand in (("a", a), ("b", b)):
        is_ref = operand[0] == "ref"
        held[f"{side}Ref"] = is_ref
        held[f"{side}Row"] = operand[1] if is_ref else 0
        held[f"{side}Col"] = operand[2] if is_ref else 0
        held[f"{side}Val"] = 0 if is_ref else operand[1]
    return held


def lww_write(cells: list, write: dict):
    """sce:std/merge/lww_write — the cell at the write's place is the write when
    its stamp is later, and the cell when it is not; a place no cell holds gains
    one."""
    out, held = [], False
    for stored in cells:
        if (stored["row"], stored["col"]) == (write["row"], write["col"]):
            held = True
            out.append(write if hlc_key(write) > hlc_key(stored) else stored)
        else:
            out.append(stored)
    if not held:
        out.append(write)
    return ("ok", out)


def rga_node(key: int, after: int) -> dict:
    return {"key": key, "after": after}


def rga_insert(nodes: list, node: dict):
    """sce:std/merge/rga_insert — directly after the element `after` names (the
    front for 0), past every element there with a larger id; an element already
    held changes nothing; one whose predecessor is not held goes last."""
    if any(held["key"] == node["key"] for held in nodes):
        return ("ok", list(nodes))
    out, phase = [], 1 if node["after"] == 0 else 0
    for held in nodes:
        if phase == 0:
            out.append(held)
            if held["key"] == node["after"]:
                phase = 1
        elif phase == 1:
            if held["key"] > node["key"]:
                out.append(held)
            else:
                out += [node, held]
                phase = 2
        else:
            out.append(held)
    if phase != 2:
        out.append(node)
    return ("ok", out)


def rga_holds(nodes: list, key: int):
    """sce:std/merge/rga_holds — whether an element has the id."""
    return ("ok", any(held["key"] == key for held in nodes))


def rga_gone(key: int, by: int) -> dict:
    return {"key": key, "by": by}


def rga_remove(gone: list, removal: dict):
    """sce:std/merge/rga_remove — the marks, and the removal unless its id is
    already marked."""
    marked = any(held["key"] == removal["key"] for held in gone)
    return ("ok", list(gone) + ([] if marked else [removal]))


#: The faults a cell of a grid shows instead of a number (sce:std/merge/sheet_settle).
CYCLE, DIVIDE, OVERFLOW, BAD_RANGE, UNKNOWN_OP = 1, 2, 3, 4, 5


def sheet_value(value: int, fault: int) -> dict:
    return {"value": value, "fault": fault}


def sheet_apply(op: int, a: int, b: int):
    """sce:std/merge/sheet_apply — a + b, a - b, a * b and a / b toward zero in
    int64, the exact result judged against the width: one outside it is a fault,
    as is a division by zero, and an operation that is none of the four."""
    if op not in (2, 3, 4, 5):
        return ("ok", sheet_value(0, UNKNOWN_OP))
    if op == 5 and b == 0:
        return ("ok", sheet_value(0, DIVIDE))
    exact = {2: a + b, 3: a - b, 4: a * b}.get(op) if op != 5 else truncated_quotient(a, b)
    if not fits(exact, I64_MIN, I64_MAX):
        return ("ok", sheet_value(0, OVERFLOW))
    return ("ok", sheet_value(exact, 0))


def sheet_first(cells: list) -> dict:
    """The index of the first cell at each place."""
    first: dict = {}
    for at, held in enumerate(cells):
        first.setdefault((held["row"], held["col"]), at)
    return first


def sheet_orders(rows: list, gone: list, cols: list, gone_cols: list) -> tuple:
    """Each row id's and column id's place in its order, counted from 1, and the
    ids removed from either."""
    return (
        {node["key"]: place + 1 for place, node in enumerate(rows)},
        {node["key"]: place + 1 for place, node in enumerate(cols)},
        {mark["key"] for mark in gone},
        {mark["key"] for mark in gone_cols},
    )


def sheet_members(cells: list, formula_cell: dict, orders: tuple):
    """The cells a sum takes, by index, or None where its rectangle is not
    defined: the cells that stand in the rows and columns between the corners'
    places, and in a row and a column that are not removed."""
    row_place, col_place, gone_rows, gone_cols = orders
    if not (formula_cell["aRef"] and formula_cell["bRef"]):
        return None
    rows = [row_place.get(formula_cell["aRow"], 0), row_place.get(formula_cell["bRow"], 0)]
    cols = [col_place.get(formula_cell["aCol"], 0), col_place.get(formula_cell["bCol"], 0)]
    if 0 in rows or 0 in cols:
        return None
    return [
        at
        for at, held in enumerate(cells)
        if held["row"] not in gone_rows
        and held["col"] not in gone_cols
        and min(rows) <= row_place.get(held["row"], 0) <= max(rows)
        and min(cols) <= col_place.get(held["col"], 0) <= max(cols)
    ]


def sheet_formula(held: dict, a: tuple, b: tuple, members):
    """What a cell shows, as (number, fault), given what its operands show — a
    and b, each (number, fault) — and what the cells of its rectangle show, or
    None where there is none. Where several things fault, the smallest code is
    shown."""
    op = held["op"]
    if op == 0:
        return (held["value"], 0)
    if op == 1:
        return a
    if op in (2, 3, 4, 5):
        if a[1] or b[1]:
            return (0, min(fault for fault in (a[1], b[1]) if fault))
        applied = sheet_apply(op, a[0], b[0])[1]
        return (applied["value"], applied["fault"])
    if op == 6:
        if members is None:
            return (0, BAD_RANGE)
        faults = [fault for _, fault in members if fault]
        if faults:
            return (0, min(faults))
        above = sum(number for number, _ in members if number > 0)
        below = sum(number for number, _ in members if number < 0)
        if above > I64_MAX or below < I64_MIN:
            return (0, OVERFLOW)
        return (above + below, 0)
    return (0, UNKNOWN_OP)


def sheet_operand(held: dict, side: str, first: dict, shows):
    """What an operand reads: its literal, or what the first cell at its place
    shows (0 where there is none), or None when that cell is not yet shown."""
    if not held[f"{side}Ref"]:
        return (held[f"{side}Val"], 0)
    holder = first.get((held[f"{side}Row"], held[f"{side}Col"]))
    return (0, 0) if holder is None else shows(holder)


def sheet_settle(cells: list, rows: list, gone: list, cols: list, gone_cols: list):
    """sce:std/merge/sheet_settle — the cells gone over in passes, a cell going on
    the trail, as (index, number, fault), in the first pass that finds everything
    it reads already there; a pass that adds none is the last."""
    orders, first = sheet_orders(rows, gone, cols, gone_cols), sheet_first(cells)
    shown: dict = {}
    trail: list = []
    moving, passes = True, 0
    while moving and passes <= len(cells):
        moving, passes = False, passes + 1
        for at, held in enumerate(cells):
            if at in shown:
                continue
            op = held["op"]
            a = sheet_operand(held, "a", first, shown.get) if op in (1, 2, 3, 4, 5) else (0, 0)
            b = sheet_operand(held, "b", first, shown.get) if op in (2, 3, 4, 5) else (0, 0)
            members = sheet_members(cells, held, orders) if op == 6 else None
            seen = None if members is None else [shown.get(other) for other in members]
            if a is None or b is None or (seen is not None and None in seen):
                continue
            shown[at] = sheet_formula(held, a, b, seen)
            trail += [at, *shown[at]]
            moving = True
    return ("ok", trail)


def sheet_semantics(cells: list, rows: list, gone: list, cols: list, gone_cols: list) -> dict:
    """What each cell shows, by index, as (number, fault), for the cells that do
    not read a cycle — worked out by following each cell's references depth first
    and not by passes, so that it is a second statement of what sheet_settle
    means, held to it by a law."""
    orders, first = sheet_orders(rows, gone, cols, gone_cols), sheet_first(cells)
    memo: dict = {}
    busy: set = set()

    def solve(at: int):
        if at in memo:
            return memo[at]
        if at in busy:
            return None
        busy.add(at)
        held = cells[at]
        op = held["op"]
        a = sheet_operand(held, "a", first, solve) if op in (1, 2, 3, 4, 5) else (0, 0)
        b = sheet_operand(held, "b", first, solve) if op in (2, 3, 4, 5) else (0, 0)
        members = sheet_members(cells, held, orders) if op == 6 else None
        seen = None if members is None else [solve(other) for other in members]
        if a is None or b is None or (seen is not None and None in seen):
            result = None
        else:
            result = sheet_formula(held, a, b, seen)
        busy.discard(at)
        memo[at] = result
        return result

    return {at: answer for at in range(len(cells)) if (answer := solve(at)) is not None}


def sheet_show(cells: list, trail: list):
    """sce:std/merge/sheet_show — one record for each place, in order of row then
    column, from the cell first held there; what the trail says of it (the last
    entry naming its index), or 0 and the fault CYCLE where it names none."""
    said = {trail[at]: (trail[at + 1], trail[at + 2]) for at in range(0, len(trail), 3)}
    first = sheet_first(cells)
    out = []
    for (row, col), at in sorted(first.items()):
        number, fault = said.get(at, (0, CYCLE))
        out.append({"row": row, "col": col, "value": number, "fault": fault})
    return ("ok", out)


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


def value_of(write: dict) -> tuple:
    """The stamp that names the value a write sets: the stamp of the write that
    first carried it, as three numbers. All zero is no value."""
    return (write["valueWallTime"], write["valueCounter"], write["valueNodeId"])


def undo_field_plan(versions: list, batch: int, floor: int):
    """sce:std/merge/undo_field_plan — one step for each field the batch wrote.

    A step is planned from the batch's EARLIEST write to its field (by stamp). The
    field's winner is its latest write by stamp, the first of them in list order if
    two tie. 3: that earliest write arrived at or before `floor`. 2: the winner is
    an undo of this batch. 0: the winner is a write of this batch, and the value to
    restore is the one the latest write earlier than the earliest batch write held:
    that write's value stamp, copied (all zero if there is none). 1: anyone else's
    write is the winner, and its stamp is reported. Steps come in the order the
    earliest writes appear in `versions`."""
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
        restore = value_of(base) if verdict == 0 and base is not None else (0, 0, 0)
        steps.append(
            {
                "entityId": first["entityId"],
                "fieldId": first["fieldId"],
                "verdict": verdict,
                "restoreWallTime": restore[0],
                "restoreCounter": restore[1],
                "restoreNodeId": restore[2],
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


def mailbox_resume(restored: int, held: list, cursors: list):
    """sce:std/sync/mailbox_resume — the greatest id the restored copy or a device
    has seen, which must be an id (not below 0) and leave room for one more."""
    seen = [restored] + held + cursors
    if min(seen) < 0 or max(seen) >= I64_MAX:
        return ("fails", "precondition")
    return ("ok", max(seen))


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


#: The digits of a merkle key, and so the depth of a digest: thirty base-3 digits
#: write every minute of a signed 64-bit millisecond clock.
MERKLE_DEPTH = 30
#: The minutes the key can write: 3^30. Every one below it has a leading digit of
#: 0, 1 or 2, which is what a walk from the root reaches.
MERKLE_MINUTES = 3**MERKLE_DEPTH
#: The last minute a signed 64-bit millisecond clock holds, 153722867280912, and
#: the last the digest accepts; it is below MERKLE_MINUTES, and its milliseconds
#: are the last a merkle_diff answer can name.
MERKLE_LAST_MINUTE = I64_MAX // 60_000
#: The year-2051 limit of the reference's 16-digit key, and the first minute past it.
MERKLE_OLD_MINUTES = 3**16


def merkle_path(minute: int) -> list:
    """The nodes a change at `minute` lies under: the root, then for each depth 1
    to 30 the node whose prefix is the minute's base-3 digits down to that depth."""
    return [(0, 0)] + [
        (depth, minute // 3 ** (MERKLE_DEPTH - depth)) for depth in range(1, MERKLE_DEPTH + 1)
    ]


def merkle_node(depth: int, prefix: int, hash_: int) -> dict:
    return {"depth": depth, "prefix": prefix, "hash": hash_}


def merkle_digest_valid(nodes: list) -> bool:
    """Ascending (depth, prefix) with no depth past 30 — a digest the family wrote."""
    keys = [(node["depth"], node["prefix"]) for node in nodes]
    return all(depth <= MERKLE_DEPTH for depth, _ in keys) and all(a < b for a, b in zip(keys, keys[1:]))


def merkle_insert(nodes: list, minute: int, hash_: int):
    """sce:std/merge/merkle_insert — the change's hash XORed into every node on its
    path, a missing node created with the hash alone; a minute no signed 64-bit
    millisecond clock holds is refused."""
    if not 0 <= minute <= MERKLE_LAST_MINUTE or not merkle_digest_valid(nodes):
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
    depth, prefix, span = 0, 0, MERKLE_MINUTES
    while depth < MERKLE_DEPTH:
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


def merkle_change_hash(change: dict, kind: int):
    """sce:std/merge/merkle_change_hash — a 32-bit mix of one change of an
    observed-remove set: the element, the wall time and the node each as a low and
    a high 32-bit half, the counter and the kind, one step at a time, then the
    avalanche of MurmurHash3's fmix32. A negative wall time is refused."""
    if change["wallTime"] < 0:
        return ("fails", "precondition")
    word = 2**32
    parts = [
        change["element"] % word,
        change["element"] // word,
        change["wallTime"] % word,
        change["wallTime"] // word,
        change["counter"],
        change["nodeId"] % word,
        change["nodeId"] // word,
        kind + 1,
    ]
    mixed = 0
    for part in parts:
        mixed = (mixed * 1_000_003 + part) % word
    mixed ^= mixed >> 16
    mixed = (mixed * 0x85EBCA6B) % word
    mixed ^= mixed >> 13
    mixed = (mixed * 0xC2B2AE35) % word
    mixed ^= mixed >> 16
    return ("ok", mixed)


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


#: Cursors a device may hold: small arrival numbers, where the least is told
#: from the rest, and the edges of a uint64.
CURSOR_EDGES = [0, 1, 2, 3, 5, 9, U32_MAX, U32_MAX + 1, 2**63 - 1, 2**63, 2**64 - 2, 2**64 - 1]


def stable_point_args(rng: SplitMix64):
    """A group of devices, the wall time it is read at, and how long a device may
    be idle. Most readings sit within a few milliseconds of the bound, where `<=`
    is told from `<`; some sit at the ends of an int64, where the idle time is
    the difference that does not fit."""
    count = rng.pick([0, 1, 2, 2, 3, 3, 4, 5, 6])
    if rng.below(8) == 0:
        now = rng.pick([I64_MIN, I64_MIN + 1, -1, 0, 1, I64_MAX - 1, I64_MAX])
        max_idle = rng.pick([0, 1, 50, I64_MAX])
        edge_seen = [I64_MIN, I64_MIN + 1, -1, 0, 1, I64_MAX - 1, I64_MAX]
        last_seen = [rng.pick(edge_seen) for _ in range(count)]
    else:
        now = rng.between(0, 1_000_000)
        max_idle = rng.pick([0, 1, 50, 50, 1000, I64_MAX])
        last_seen = []
        for _ in range(count):
            # Idle for a little less than, exactly, or a little more than the bound,
            # or not at all, or read ahead of `now`.
            idle = rng.pick([-5, 0, 0, 1, max(max_idle - 1, 0), max_idle, max_idle + 1, 3 * max_idle + 7])
            last_seen.append(now - idle if idle <= I64_MAX else now)
    cursors = [rng.pick(CURSOR_EDGES) if rng.below(3) == 0 else rng.between(0, 20) for _ in range(count)]
    kind = rng.below(14)
    if kind == 0 and last_seen:
        last_seen = last_seen[:-1]
    elif kind == 1:
        max_idle = rng.pick([-1, I64_MIN])
    return [cursors, last_seen, now, max_idle]


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


#: Row and column ids a grid hands out, a number's worth of values, and the ids of
#: a replicated array: the edges of the widths they are carried in.
PLACE_EDGES = [0, 1, 2, 255, 2**31, U32_MAX, 2**32, 2**53, 2**63, U64_MAX]
VALUE_EDGES = [I64_MIN, I64_MIN + 1, -1, 0, 1, 2**53, I64_MAX - 1, I64_MAX]
KEY_EDGES = [1, 2, 255, 256, 257, 2**32, 2**53, 2**63, U64_MAX - 1, U64_MAX]


def place_id(rng: SplitMix64) -> int:
    return rng.pick(PLACE_EDGES) if rng.below(2) == 0 else rng.between(0, U64_MAX)


def cell_value(rng: SplitMix64) -> int:
    return rng.pick(VALUE_EDGES) if rng.below(3) == 0 else rng.between(-1000, 1000)


def stamp_of(held: dict) -> dict:
    return hlc(held["wallTime"], held["counter"], held["nodeId"])


def lww_cells(rng: SplitMix64) -> list:
    """Cells at distinct places whose stamps share fields with one another, so
    that two writes tie on the wall time and are told apart by the counter, or
    tie on both and are told apart by the node."""
    places: list = []
    for _ in range(rng.pick([0, 1, 1, 2, 2, 3, 4, 5])):
        place = (place_id(rng), place_id(rng))
        if place not in places:
            places.append(place)
    base = stamp_value(rng)
    cells = []
    for row, col in places:
        number, held = sheet_held(rng, places)
        cells.append(cell(row, col, number, near_stamp(rng, base) if rng.below(3) else stamp_value(rng), held))
    return cells


def lww_write_args(rng: SplitMix64):
    """Writes carry what a cell holds, a number or a formula, whole: the write
    that wins a cell wins all of it."""
    cells = lww_cells(rng)
    if cells and rng.below(3):
        held = rng.pick(cells)
        place = (held["row"], held["col"])
        stamp = stamp_of(held) if rng.below(8) == 0 else near_stamp(rng, stamp_of(held))
    else:
        place, stamp = (place_id(rng), place_id(rng)), stamp_value(rng)
    number, payload = sheet_held(rng, [(held["row"], held["col"]) for held in cells] + [place])
    return [cells, cell(place[0], place[1], number, stamp, payload)]


def rga_key(rng: SplitMix64) -> int:
    return rng.pick(KEY_EDGES) if rng.below(2) == 0 else rng.between(1, U64_MAX)


def rga_array(rng: SplitMix64, count: int) -> list:
    """An array built the way a replica builds one: by inserting, each element
    after one already there or at the front."""
    nodes: list = []
    for _ in range(count):
        key = rga_key(rng)
        if any(held["key"] == key for held in nodes):
            continue
        after = 0 if not nodes or rng.below(4) == 0 else rng.pick(nodes)["key"]
        nodes = rga_insert(nodes, rga_node(key, after))[1]
    return nodes


def rga_insert_args(rng: SplitMix64):
    nodes = rga_array(rng, rng.pick([0, 1, 2, 3, 4, 6, 8]))
    held = [node["key"] for node in nodes]
    key = rng.pick(held) if held and rng.below(6) == 0 else rga_key(rng)
    kind = rng.below(10)
    if kind < 2 or not held:
        after = 0
    elif kind < 8:
        after = rng.pick(held)
    else:
        after = rga_key(rng)
    return [nodes, rga_node(key, after)]


def rga_holds_args(rng: SplitMix64):
    nodes = rga_array(rng, rng.pick([0, 1, 2, 3, 4, 6, 8]))
    held = [node["key"] for node in nodes]
    kind = rng.below(6)
    if kind < 3 and held:
        key = rng.pick(held)
    elif kind == 3:
        key = 0
    else:
        key = rga_key(rng)
    return [nodes, key]


def rga_remove_args(rng: SplitMix64):
    gone: list = []
    for _ in range(rng.pick([0, 1, 2, 3, 4, 6])):
        key = rga_key(rng)
        if all(held["key"] != key for held in gone):
            gone.append(rga_gone(key, rng.pick(NODE_EDGES)))
    if gone and rng.below(3) == 0:
        key = rng.pick(gone)["key"]
    else:
        key = rga_key(rng)
    return [gone, rga_gone(key, rng.pick(NODE_EDGES))]


#: Row and column ids a grid under test is built from, besides the small ones, and
#: ids no order of the grid holds, for a reference that names nothing.
SHEET_IDS = [1, 2, 3, 255, 256, 2**32, 2**63, U64_MAX]
SHEET_ABSENT = [0, 41, U64_MAX - 1]
#: Numbers a formula reads, at the edges where an int64 operation starts to fail.
SHEET_EDGES = [I64_MIN, I64_MIN + 1, -3037000500, -2**32, -1, 0, 1, 2, 2**31, 2**32, 3037000499, 3037000500, I64_MAX - 1, I64_MAX]


def sheet_number(rng: SplitMix64) -> int:
    return rng.pick(SHEET_EDGES) if rng.below(2) == 0 else rng.between(-20, 20)


def sheet_apply_args(rng: SplitMix64):
    op = rng.pick([2, 2, 3, 3, 4, 4, 5, 5, 0, 1, 6, 255])
    a = sheet_number(rng)
    if rng.below(6) == 0:
        # The other operand just where the result leaves int64, for this operation.
        nearest = {2: I64_MAX - a, 3: a + I64_MAX, 4: I64_MAX // a if a else 1, 5: rng.pick([0, -1, 1])}.get(op, 0)
        b = clamp(nearest + rng.pick([-1, 0, 1]), I64_MIN, I64_MAX)
    else:
        b = sheet_number(rng)
    return [op, a, b]


def sheet_ids(rng: SplitMix64, count: int) -> list:
    """A few distinct ids, small or at the edges of their width."""
    ids: list = []
    for _ in range(count):
        key = rng.pick(SHEET_IDS) if rng.below(2) == 0 else rng.between(1, 40)
        if key not in ids:
            ids.append(key)
    return ids


def sheet_order(rng: SplitMix64, ids: list) -> list:
    """An array of these ids, built as a replica builds one."""
    nodes: list = []
    for key in ids:
        after = 0 if not nodes or rng.below(3) == 0 else rng.pick(nodes)["key"]
        nodes = rga_insert(nodes, rga_node(key, after))[1]
    return nodes


def sheet_operand_of(rng: SplitMix64, places: list) -> tuple:
    """A reference to one of the places, or a number."""
    if places and rng.below(10) < 7:
        return reads(*rng.pick(places))
    return literal(sheet_number(rng))


def sheet_held(rng: SplitMix64, places: list):
    """What a cell holds: its number and its formula fields. The number is the
    one a number cell shows; a formula keeps it too, which nothing reads."""
    kind = rng.below(10)
    if kind < 4:
        return cell_value(rng), NUMBER
    op = rng.pick([1, 1, 2, 2, 3, 4, 5, 5, 6, 6, 6, 7, 255])
    if op == 6:
        corners = [reads(*rng.pick(places)) if places and rng.below(10) else literal(0) for _ in range(2)]
        return cell_value(rng), formula(op, *corners)
    return cell_value(rng), formula(op, sheet_operand_of(rng, places), sheet_operand_of(rng, places))


def sheet_grid(rng: SplitMix64):
    """A grid: its cells, the orders of its rows and columns, and the removals.
    The places are among the rows and columns the orders hold, and one more that
    they do not, so that a cell may stand outside them."""
    row_ids = sheet_ids(rng, rng.pick([0, 1, 2, 2, 3, 3, 4]))
    col_ids = sheet_ids(rng, rng.pick([0, 1, 2, 2, 3]))
    rows, cols = sheet_order(rng, row_ids), sheet_order(rng, col_ids)
    gone = [rga_gone(key, rng.pick(NODE_EDGES)) for key in row_ids if rng.below(5) == 0]
    gone_cols = [rga_gone(key, rng.pick(NODE_EDGES)) for key in col_ids if rng.below(5) == 0]
    candidates = [
        (row, col)
        for row in row_ids + [rng.pick(SHEET_ABSENT)]
        for col in col_ids + [rng.pick(SHEET_ABSENT)]
    ]
    places = shuffled(rng, candidates)[: min(rng.pick([0, 1, 2, 3, 4, 5, 6, 8]), len(candidates))]
    base = stamp_value(rng)
    cells = []
    for row, col in places:
        number, held = sheet_held(rng, candidates)
        cells.append(cell(row, col, number, near_stamp(rng, base) if rng.below(3) else stamp_value(rng), held))
    return cells, rows, gone, cols, gone_cols


def sheet_settle_args(rng: SplitMix64):
    return list(sheet_grid(rng))


def sheet_show_args(rng: SplitMix64):
    """Cells and a trail of them in no order that matters: most of what the cells
    show, some of it left out, and sometimes an entry that names no cell."""
    cells, rows, gone, cols, gone_cols = sheet_grid(rng)
    shown = sheet_semantics(cells, rows, gone, cols, gone_cols)
    entries = [[at, *answer] for at, answer in shown.items() if rng.below(8) != 0]
    if rng.below(8) == 0:
        entries.append([len(cells) + rng.between(0, 3), cell_value(rng), 0])
    return [cells, [number for entry in shuffled(rng, entries) for number in entry]]


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


#: Where UTF-8's rules change their mind: the first and last scalar value of
#: each encoded width, either side of the surrogates, and the last one.
SCALAR_EDGES = [
    0x00, 0x41, 0x7F, 0x80, 0x7FF, 0x800, 0xD7FF, 0xE000, 0xFEFF, 0xFFFF,
    0x10000, 0x1F468, 0x10FFFF,
]
#: The bytes at which the grammar narrows or ends a range, and one either side.
BYTE_EDGES = [
    0x00, 0x7F, 0x80, 0x8F, 0x90, 0x9F, 0xA0, 0xBF, 0xC0, 0xC1, 0xC2, 0xDF,
    0xE0, 0xE1, 0xEC, 0xED, 0xEE, 0xEF, 0xF0, 0xF1, 0xF3, 0xF4, 0xF5, 0xFF,
]


def draw_scalar(rng: SplitMix64) -> int:
    """A Unicode scalar value, at an edge or anywhere in a width; never a surrogate."""
    if rng.below(3) == 0:
        return rng.pick(SCALAR_EDGES)
    low, high = rng.pick([(0, 0x7F), (0x80, 0x7FF), (0x800, 0xD7FF), (0xE000, 0xFFFF), (0x10000, 0x10FFFF)])
    return rng.between(low, high)


def utf8_text(rng: SplitMix64, longest: int) -> list:
    """The UTF-8 bytes of a drawn string of at most `longest` characters."""
    return list("".join(chr(draw_scalar(rng)) for _ in range(rng.between(0, longest))).encode("utf-8"))


def utf8_args(rng: SplitMix64) -> list:
    """A byte string that is mostly valid text, and often one fault away from it:
    a byte replaced, dropped or put in at a place the grammar looks at, or the
    string cut mid-character; the rest is bytes drawn from the grammar's edges."""
    style = rng.below(6)
    if style == 0:
        return [[rng.pick(BYTE_EDGES) if rng.below(2) else rng.between(0, 255) for _ in range(rng.between(0, 6))]]
    data = utf8_text(rng, rng.pick([1, 2, 4, 8, 20]))
    if style <= 2 or not data:
        return [data]
    at = rng.below(len(data))
    if style == 3:
        data[at] = rng.pick(BYTE_EDGES)
    elif style == 4:
        del data[at:]
    else:
        data.insert(at, rng.pick(BYTE_EDGES))
    return [data]


def utf8_within_args(rng: SplitMix64) -> list:
    """The same strings with a limit at, just under and just over their length."""
    data = utf8_args(rng)[0]
    counted = answer_of(utf8_length(data))
    if counted is None or rng.below(4) == 0:
        limit = rng.pick([0, 1, 2, 5, 100, U32_MAX])
    else:
        limit = max(0, counted + rng.pick([-1, 0, 0, 1]))
    return [data, limit]


#: Characters of each encoded width (in UTF-8 bytes). No NUL: the C backend ends
#: a text there.
TEXT_WIDTHS = {
    1: ["a", "Z", "0", " ", "\x7f", "-"],
    2: ["é", "ß", "́", "߿", "\u0080"],
    3: ["€", "한", "ࠀ", "￿", "﻿"],
    4: ["\U0001f468", "\U00010000", "\U0010ffff", "\U0001f600"],
}
#: Texts that differ by one character or one encoding: the precomposed and the
#: decomposed "e with acute", which a byte comparison tells apart as the document
#: says it does, a case, and the title the document gives an empty text.
TEXT_PAIRS = ["é", "é", "e", "E", "É", "café", "café", UNTITLED, "Untitled", "untitle"]
VERSION_EDGES = [I64_MIN, I64_MIN + 1, -1, 0, 1, 2**31, 2**53, I64_MAX - 1, I64_MAX]


def utf8_size(text: str) -> int:
    return len(text.encode("utf-8"))


def text_of_size(rng: SplitMix64, size: int) -> str:
    """A text of exactly `size` UTF-8 bytes, its characters of mixed widths."""
    parts, used = [], 0
    while used < size:
        width = rng.pick([w for w in TEXT_WIDTHS if used + w <= size])
        parts.append(rng.pick(TEXT_WIDTHS[width]))
        used += width
    return "".join(parts)


def slot_text(rng: SplitMix64) -> str:
    """A text the schema's bound holds: empty, a title or near-title, exactly the
    bound, a byte or two under it, or any length between. A text past the bound
    is never drawn: the schema refuses it where it is written, so it is no
    document's input, and every document here reads and writes the one bound."""
    style = rng.below(10)
    if style == 0:
        return ""
    if style <= 2:
        return rng.pick(TEXT_PAIRS)
    if style <= 4:
        return text_of_size(rng, TEXT_BOUND)
    if style <= 6:
        return text_of_size(rng, TEXT_BOUND - rng.between(1, 3))
    return text_of_size(rng, rng.between(1, TEXT_BOUND))


def slot_place(rng: SplitMix64) -> int:
    return rng.pick([0, 1, 2, 3, U32_MAX - 1, U32_MAX]) if rng.below(3) else rng.between(0, 6)


def slot_version(rng: SplitMix64) -> int:
    return rng.pick(VERSION_EDGES) if rng.below(3) == 0 else rng.between(-3, 12)


def near_text(rng: SplitMix64, base: str) -> str:
    """`base`, or a text that agrees with it up to its end: a prefix, one byte
    more, the last character changed, or a text drawn afresh. A comparison is
    decided at the first byte that differs, and a bounded buffer is where the last
    one sits."""
    kind = rng.below(7)
    if kind <= 1:
        return base
    if kind == 2 and base:
        return base[: rng.below(len(base))]
    if kind == 3 and utf8_size(base) < TEXT_BOUND:
        return base + "x"
    if kind == 4 and base:
        changed = base[:-1] + ("!" if base[-1] != "!" else "?")
        if utf8_size(changed) <= TEXT_BOUND:
            return changed
    return slot_text(rng)


def slot_value(rng: SplitMix64) -> dict:
    return titled_slot(slot_place(rng), slot_version(rng), slot_text(rng))


def text_slot_latest_args(rng: SplitMix64):
    """Two writes of a place: the later one on either side, the same version with
    another text, an adjacent version, the ends of an int64, and now and then a
    different place, which `a` keeps."""
    a = slot_value(rng)
    version = a["version"]
    kind = rng.below(8)
    if kind == 0:
        other = version
    elif kind == 1 and version < I64_MAX:
        other = version + 1
    elif kind == 2 and version > I64_MIN:
        other = version - 1
    else:
        other = slot_version(rng)
    place = a["slot"] if rng.below(4) else slot_place(rng)
    return [a, titled_slot(place, other, near_text(rng, a["text"]))]


def slot_list(rng: SplitMix64) -> list:
    """Up to `SLOT_CAPACITY` places, with places and near-equal texts repeated, as
    a replica's list may hold them."""
    held: list = []
    for _ in range(rng.pick([0, 1, 1, 2, 2, 3, 4, 5, 8, 15, 16, 16])):
        if held and rng.below(5) == 0:
            base = rng.pick(held)
            place = base["slot"] if rng.below(2) else slot_place(rng)
            held.append(titled_slot(place, slot_version(rng), near_text(rng, base["text"])))
        else:
            held.append(slot_value(rng))
    return held


def task_value(rng: SplitMix64) -> dict:
    """A task: an id at either end of a uint32 or between, and any phase."""
    task_id = rng.pick([0, 1, 2, 3, 42, rng.below(U32_MAX + 1), U32_MAX - 1, U32_MAX])
    return task_record(task_id, rng.pick(list(TASK_NEXT_PHASE)))


def task_phase_advance_args(rng: SplitMix64):
    """Up to the four tasks the result holds, an id repeated now and then."""
    tasks: list = []
    for _ in range(rng.pick([0, 1, 2, 3, 4, 4])):
        if tasks and rng.below(5) == 0:
            tasks.append(task_record(rng.pick(tasks)["taskId"], rng.pick(list(TASK_NEXT_PHASE))))
        else:
            tasks.append(task_value(rng))
    return [tasks]


def task_phase_reopened_args(rng: SplitMix64):
    return [task_value(rng)]


def text_slot_retitled_args(rng: SplitMix64):
    held = slot_list(rng)
    if held and rng.below(3) == 0:
        at = rng.below(len(held))
        held[at] = dict(held[at], text="")
    return [held]


def text_slot_holds_args(rng: SplitMix64):
    """A list and a probe: one of its places under another version, the place with
    another text, the text under another place, or a place it holds whole."""
    held = slot_list(rng)
    if not held or rng.below(4) == 0:
        return [held, slot_value(rng)]
    base = rng.pick(held)
    kind = rng.below(5)
    if kind <= 1:
        probe = titled_slot(base["slot"], slot_version(rng), base["text"])
    elif kind == 2:
        probe = titled_slot(base["slot"], base["version"], near_text(rng, base["text"]))
    elif kind == 3:
        probe = titled_slot(slot_place(rng), base["version"], base["text"])
    else:
        probe = dict(base)
    return [held, probe]


#: The stamps a drawn value is known by come from a few of each part, so that two
#: writes agree in all but one part of their dot, and in all of it, as often as the
#: writes of two copies of one row do.
MV_WALLS = [0, 1727000000400, 1727000060000, 1727000120000, I64_MAX]
MV_COUNTERS = [0, 1, 2, U32_MAX]
MV_NODES = [1, 2, 7, 2**53]
MV_NUMBERS = [0, 1, -1, I64_MIN, I64_MAX]


def mv_fresh(rng: SplitMix64) -> dict:
    """A value of a row and a field some other value may share, under a dot some
    other value may share a part of."""
    number = rng.pick(MV_NUMBERS) if rng.below(3) == 0 else rng.between(-3, 12)
    return mv_value(
        rng.pick([0, 1, 1, 2, 2**53]), rng.pick([1, 2, 9]), number, slot_text(rng),
        rng.pick(MV_WALLS), rng.pick(MV_COUNTERS), rng.pick(MV_NODES),
    )


def mv_nearby(rng: SplitMix64, base: dict) -> dict:
    """A value that differs from `base` in one part of what names its write, in only
    what the write carries (so it is the same write), or not at all."""
    kind = rng.below(9)
    if kind == 0:
        return dict(base, id=rng.pick([x for x in (0, 1, 2) if x != base["id"]]))
    if kind == 1:
        return dict(base, field=rng.pick([x for x in (1, 2, 9) if x != base["field"]]))
    if kind == 2:
        return dict(base, wallTime=rng.pick([x for x in MV_WALLS if x != base["wallTime"]]))
    if kind == 3:
        return dict(base, counter=rng.pick([x for x in MV_COUNTERS if x != base["counter"]]))
    if kind == 4:
        return dict(base, nodeId=rng.pick([x for x in MV_NODES if x != base["nodeId"]]))
    if kind <= 6:
        return dict(base, value=rng.between(-3, 12), text=near_text(rng, base["text"]))
    if kind == 7:
        return dict(base)
    return mv_fresh(rng)


def mv_list(rng: SplitMix64, size: int) -> list:
    """Up to `size` values, none of them the write of another, with rows, fields and
    parts of dots repeated, as the values of a register that was written twice at
    once are."""
    held: list = []
    for _ in range(size * 4):
        if len(held) == size:
            break
        one = mv_nearby(rng, rng.pick(held)) if held and rng.below(2) else mv_fresh(rng)
        if not any(same_write(there, one) for there in held):
            held.append(one)
    return held


def mv_probe(rng: SplitMix64, held: list) -> dict:
    """A value to ask about, take in or drop: one the list holds, one that differs
    from it in a part of its name, or one afresh."""
    if not held or rng.below(4) == 0:
        return mv_fresh(rng)
    return mv_nearby(rng, rng.pick(held))


def mv_value_holds_args(rng: SplitMix64):
    held = mv_list(rng, rng.pick([0, 1, 2, 3, 3, 4, 6, 8]))
    return [held, mv_probe(rng, held)]


def mv_value_keep_args(rng: SplitMix64):
    """As the holds' cases are, and now and then a list one short of what the result
    holds, which the write that is not there fills."""
    if rng.below(200) == 0:
        full = [mv_value(index, 1, index, slot_text(rng), 1727000000400, 0, 7) for index in range(MV_CAPACITY - 1)]
        return [full, mv_value(MV_CAPACITY, 1, 0, slot_text(rng), 1727000000400, 0, 7)]
    held = mv_list(rng, rng.pick([0, 1, 2, 3, 3, 4, 6, 8]))
    return [held, mv_probe(rng, held)]


def mv_value_drop_args(rng: SplitMix64):
    held = mv_list(rng, rng.pick([0, 1, 2, 3, 3, 4, 6, 8]))
    return [held, mv_probe(rng, held)]


def register_values_args(rng: SplitMix64):
    """Values whose registers share rows, fields and parts of dots, as the values of a
    register written at once by several people do, and the register to read: one a
    value of the list is in, most of the time, or one drawn afresh, which the list
    may hold nothing of."""
    held = mv_list(rng, rng.pick([0, 1, 2, 3, 4, 6, 8, 8]))
    if held and rng.below(4) != 0:
        one = rng.pick(held)
        return [held, one["id"], one["field"]]
    return [held, rng.pick([0, 1, 2, 2**53]), rng.pick([1, 2, 9])]


POLICY_LIMITS = [0, 1, 7, 30, 364, POLICY_LIMIT_MAX, POLICY_LIMIT_MAX + 1, U16_MAX]
POLICY_BURSTS = [0, 1, 10, 50, U32_MAX]


def policy_limit(rng: SplitMix64) -> int:
    """A limit at an edge of the range, inside it, or anywhere a uint16 can be."""
    kind = rng.below(10)
    if kind < 4:
        return rng.pick(POLICY_LIMITS + [2, 366, U16_MAX - 1])
    if kind < 8:
        return rng.between(0, POLICY_LIMIT_MAX)
    return rng.between(0, U16_MAX)


def policy_scalar_settings(rng: SplitMix64) -> dict:
    return {
        "strict": rng.below(2) == 1,
        "limitDays": policy_limit(rng),
        "burst": rng.pick(POLICY_BURSTS) if rng.below(2) else rng.between(0, U32_MAX),
        "locked": rng.below(2) == 1,
    }


def policy_mask(rng: SplitMix64) -> int:
    """A mask over the three bits that name a field, one that sets bits no field
    has, and the widest `uint64`."""
    kind = rng.below(10)
    if kind < 6:
        return rng.between(0, 7)
    if kind == 6:
        return rng.pick([8, 16, 255, 2**32, 2**63])
    if kind == 7:
        return U64_MAX
    if kind == 8:
        return rng.between(0, U64_MAX)
    return 1 << rng.below(3)


def policy_layer_args(rng: SplitMix64):
    """Two policies, sometimes the same one, and a mask."""
    base = policy_scalar_settings(rng)
    over = dict(base) if rng.below(12) == 0 else policy_scalar_settings(rng)
    return [base, over, policy_mask(rng)]


def policy_rule_args(rng: SplitMix64):
    return [rng.below(2) == 1, policy_limit(rng)]


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
ARRIVAL_EDGES = [0, 1, 2, 2**32, 2**63, U64_MAX]


def field_write(entity, field, stamp, arrival, batch, origin, reverts, value) -> dict:
    """One write. `value` is the stamp that names the value it sets (the stamp of
    the write that first carried it), spelled out as the record spells it."""
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
        "valueWallTime": value["wallTime"],
        "valueCounter": value["counter"],
        "valueNodeId": value["nodeId"],
    }


def value_stamp(rng: SplitMix64, own: dict) -> dict:
    """The stamp naming the value a write sets: none (all zero), the write's own
    (a value it introduces), or one drawn apart from it (an undo writing an earlier
    value back). The drawn ones sit at the edges of each field, so a copy that
    crosses two fields — the wall time into the counter — changes the answer."""
    roll = rng.below(10)
    if roll < 2:
        return hlc(0, 0, 0)
    if roll < 6:
        return dict(own)
    return stamp_value(rng)


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
            value = value_stamp(rng, stamp)
            versions.append(field_write(entity, field, stamp, min(arrival, U64_MAX), owner, origin, reverts, value))
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
        stamps = [hlc(REALISTIC_WALL + index, 0, 1) for index in range(count)]
        versions = [
            field_write(1, index, stamp, index + 1, batch, 1, 0, stamp)
            for index, stamp in enumerate(stamps)
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
                        2, batch, value_stamp(rng, stamp),
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
            rng.pick(ARRIVAL_EDGES),
            rng.pick(UNDO_BATCHES + [0]),
            rng.pick([0, 1, 2]),
            rng.pick(UNDO_BATCHES + [0]),
            value_stamp(rng, stamp),
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


#: Ids a copy or a device may report: small ones, where the greatest is told from
#: the rest, a 53-bit edge, and the edges of an int64 where the room for one more ends.
RESUME_EDGES = [0, 1, 2, 3, 7, 9, 2**32, 2**53, I64_MAX - 2, I64_MAX - 1]


def mailbox_resume_args(rng: SplitMix64):
    """A restored counter, the ids the copy holds and the cursors the devices
    report. A copy older than a device (a counter below a cursor) is the case the
    document exists for, so the counter is drawn below the others as often as above.
    Now and then one value is not an id (below 0) or leaves no room (at the end of
    an int64)."""
    def draw():
        return rng.pick(RESUME_EDGES) if rng.below(3) == 0 else rng.between(0, 20)

    restored = draw()
    held = [draw() for _ in range(rng.pick([0, 0, 1, 2, 3, 5]))]
    cursors = [draw() for _ in range(rng.pick([0, 1, 1, 2, 3, 4]))]
    kind = rng.below(14)
    if kind == 0:
        restored = rng.pick([-1, I64_MIN, I64_MAX])
    elif kind == 1 and held:
        held[rng.below(len(held))] = rng.pick([-1, I64_MIN, I64_MAX])
    elif kind == 2 and cursors:
        cursors[rng.below(len(cursors))] = rng.pick([-1, I64_MIN, I64_MAX])
    return [restored, held, cursors]


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
    """A change's minute, one the key can write: a real one, near the start or the
    3^16 boundary the reference's 16-digit key ends at, anywhere out to the last
    minute a signed 64-bit millisecond clock holds, and at the key's own edges."""
    kind = rng.below(10)
    if kind < 4:
        return 29_841_720 + rng.between(-100_000, 100_000)
    if kind < 5:
        return rng.pick([0, 1, 2, 3, 8, 26, 27, 28])
    if kind < 7:
        return MERKLE_OLD_MINUTES + rng.between(-3, 3)
    if kind < 9:
        return rng.pick(
            [
                2**40,
                3**20,
                3**25,
                3**29 - 1,
                3**29,
                2 * 3**29,
                MERKLE_LAST_MINUTE - 1,
                MERKLE_LAST_MINUTE,
            ]
        )
    return rng.between(MERKLE_OLD_MINUTES, MERKLE_LAST_MINUTE)


def merkle_late_minute(rng: SplitMix64) -> int:
    """A minute from 3^16 on, where the reference's key is a 17th digit and the
    clock reads 2051-11-05T13:21Z and after: either side of the boundary, at a
    year's turn far past it, or anywhere out to the last minute a clock holds."""
    kind = rng.below(4)
    if kind == 0:
        return MERKLE_OLD_MINUTES + rng.between(-2, 2)
    if kind == 1:
        year = 525_960
        return MERKLE_OLD_MINUTES + year * rng.pick([1, 10, 100, 1000, 10_000, 1_000_000, 100_000_000])
    if kind == 2:
        return rng.pick([2**40, 3**20, 3**25, 3**29 - 1, 3**29, 2 * 3**29, MERKLE_LAST_MINUTE - 1, MERKLE_LAST_MINUTE])
    return rng.between(MERKLE_OLD_MINUTES, MERKLE_LAST_MINUTE)


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
        (clamp(base + rng.pick([0, 1, 2, 3, 9, 27, 243, 6561, 100_000]), 0, MERKLE_LAST_MINUTE), change_hash(rng))
        for _ in range(count)
    ]


def any_changes(rng: SplitMix64, count: int) -> list:
    return [(merkle_minute(rng), change_hash(rng)) for _ in range(count)]


def merkle_insert_args(rng: SplitMix64):
    changes = near_changes(rng, rng.between(0, 4)) if rng.below(2) else any_changes(rng, rng.between(0, 3))
    nodes = digest_of(changes)
    if nodes and rng.below(10) == 0:
        nodes = shuffled(rng, nodes) if rng.below(2) else nodes + [merkle_node(MERKLE_DEPTH + 1, 0, 1)]
    if changes and rng.below(4) == 0:
        minute, hash_ = rng.pick(changes)
    else:
        minute, hash_ = merkle_minute(rng), change_hash(rng)
    if rng.below(15) == 0:
        minute = -1 - rng.between(0, 5)
    elif rng.below(15) == 0:
        # A minute no signed 64-bit millisecond clock holds, the first past it and
        # past what thirty digits write among them.
        minute = rng.pick([MERKLE_LAST_MINUTE + 1, MERKLE_MINUTES - 1, MERKLE_MINUTES, MERKLE_MINUTES + 1, 2**62, I64_MAX])
    return [nodes, minute, hash_]


def merkle_insert_late_args(rng: SplitMix64):
    """An insert at a minute from 3^16 on into a digest of changes around it: the
    path the reference's key has no digit for, with siblings at every depth below
    the shared prefix."""
    base = merkle_late_minute(rng)
    changes = near_changes(rng, rng.between(0, 4), base)
    nodes = digest_of(changes)
    if changes and rng.below(4) == 0:
        minute, hash_ = rng.pick(changes)
    else:
        minute, hash_ = clamp(base + rng.pick([0, 1, 2, 5, 27, 729, 100_000]), 0, MERKLE_LAST_MINUTE), change_hash(rng)
    return [nodes, minute, hash_]


def merkle_prune_args(rng: SplitMix64):
    nodes = digest_of(near_changes(rng, rng.between(1, 6)))
    if nodes and rng.below(12) == 0:
        nodes = shuffled(rng, nodes)
    return [nodes, rng.pick([1, 1, 2, 2, 3, 0, 1000]) if rng.below(2) else rng.between(1, 4)]


def merkle_prune_late_args(rng: SplitMix64):
    """A prune of the digest of changes at minutes from 3^16 on."""
    nodes = digest_of(near_changes(rng, rng.between(1, 6), merkle_late_minute(rng)))
    return [nodes, rng.pick([1, 1, 2, 2, 3]) if rng.below(2) else rng.between(1, 4)]


def merkle_diff_late_args(rng: SplitMix64):
    """Two replicas whose changes are at minutes from 3^16 on, parting below a long
    shared prefix; now and then the one change each side lacks sits on either side
    of the boundary itself, 3^16 - 1 and 3^16, the last minute of the reference's
    key and the first it has no digit for."""
    base = merkle_late_minute(rng)
    shared = near_changes(rng, rng.between(0, 4), base)
    if rng.below(4) == 0:
        only_a = [(MERKLE_OLD_MINUTES - 1, change_hash(rng))]
        only_b = [(MERKLE_OLD_MINUTES, change_hash(rng))]
        shared = []
    else:
        only_a = near_changes(rng, rng.between(0, 3), base)
        only_b = near_changes(rng, rng.between(0, 3), base)
    a, b = digest_of(shared + only_a), digest_of(shared + only_b)
    if rng.below(3) == 0:
        b = answer_of(merkle_prune(b, rng.pick([1, 2])))
    if rng.below(6) == 0:
        a, b = b, a
    return [a, b]


#: The to-do list's field numbers and ids at the places the old key packed them
#: badly (`id * 16 + field` ran one register into another from field 16) and where
#: the new one's halves meet (`id * 65536 + field`, a uint16 and a uint32).
CHANGE_FIELD_EDGES = [0, 1, 2, 3, 9, 15, 16, 17, 31, 32, 100, 255, 256, 257, 1000, 4095, 4096, 65534, 65535]
CHANGE_ID_EDGES = [0, 1, 7, 15, 16, 255, 65535, 65536, 2**21 - 1, 2**21, 2**32 - 1]


def change_wall(rng: SplitMix64) -> int:
    """A wall time for a change: a real one, either side of 3^16 minutes, at a
    multiple of 2^32 milliseconds (where a low half alone cannot tell two apart),
    or at an edge of the type, negative ones included."""
    kind = rng.below(8)
    if kind < 2:
        return REALISTIC_WALL + rng.between(-1000, 1000)
    if kind < 4:
        return MERKLE_OLD_MINUTES * 60_000 + rng.between(-60_000, 60_000)
    if kind < 6:
        return clamp(rng.between(1, 2**31) * 2**32 + rng.pick([0, 0, 1, 59_999]), 0, I64_MAX)
    if kind < 7:
        return rng.pick(WALL_EDGES)
    return rng.between(0, I64_MAX)


def merkle_change_hash_args(rng: SplitMix64):
    kind = rng.below(10)
    if kind < 6:
        element = rng.pick(CHANGE_ID_EDGES) * 65_536 + rng.pick(CHANGE_FIELD_EDGES)
    elif kind < 8:
        element = rng.pick(ELEMENT_EDGES)
    else:
        element = rng.between(0, U64_MAX)
    counter = rng.pick(COUNTER_EDGES) if rng.below(3) == 0 else rng.between(0, 5)
    node = rng.pick(NODE_EDGES) if rng.below(3) == 0 else rng.between(0, 5)
    return [entry(element, change_wall(rng), counter, node), rng.pick([0, 1, 0, 1, 0, 1, 2, 255])]


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
    "stable_point": (lambda args: stable_point(*args), stable_point_args, 500, 0xE110_0041),
    "utf8_length": (lambda args: utf8_length(*args), utf8_args, 600, 0xE110_0042),
    "utf8_within": (lambda args: utf8_within(*args), utf8_within_args, 400, 0xE110_0043),
    "mailbox_resume": (lambda args: mailbox_resume(*args), mailbox_resume_args, 500, 0xE110_0044),
    "lww_write": (lambda args: lww_write(*args), lww_write_args, 400, 0xE110_0045),
    "rga_insert": (lambda args: rga_insert(*args), rga_insert_args, 500, 0xE110_0046),
    "rga_holds": (lambda args: rga_holds(*args), rga_holds_args, 250, 0xE110_0047),
    "rga_remove": (lambda args: rga_remove(*args), rga_remove_args, 250, 0xE110_0048),
    "text_slot_latest": (lambda args: text_slot_latest(*args), text_slot_latest_args, 500, 0xE110_0049),
    "text_slot_retitled": (lambda args: text_slot_retitled(*args), text_slot_retitled_args, 400, 0xE110_004A),
    "text_slot_holds": (lambda args: text_slot_holds(*args), text_slot_holds_args, 500, 0xE110_004B),
    "algorithm_task_phase_advance": (
        lambda args: task_phase_advance(*args),
        task_phase_advance_args,
        200,
        0xE110_0058,
    ),
    "algorithm_task_phase_reopened": (
        lambda args: task_phase_reopened(*args),
        task_phase_reopened_args,
        60,
        0xE110_0059,
    ),
    "sheet_apply": (lambda args: sheet_apply(*args), sheet_apply_args, 500, 0xE110_004C),
    "sheet_settle": (lambda args: sheet_settle(*args), sheet_settle_args, 600, 0xE110_004D),
    "sheet_show": (lambda args: sheet_show(*args), sheet_show_args, 300, 0xE110_004E),
    "mv_value_holds": (lambda args: mv_value_holds(*args), mv_value_holds_args, 400, 0xE110_004F),
    "mv_value_keep": (lambda args: mv_value_keep(*args), mv_value_keep_args, 500, 0xE110_0050),
    "mv_value_drop": (lambda args: mv_value_drop(*args), mv_value_drop_args, 400, 0xE110_0051),
    "register_values": (lambda args: register_values(*args), register_values_args, 400, 0xE110_0058),
    "policy_scalar_layer": (lambda args: policy_scalar_layer(*args), policy_layer_args, 500, 0xE110_0052),
    # A validator's cases are a `sequence`, and this one holds no state, so each
    # generated case stands alone.
    "PolicyScalarRule::validate": (lambda args: policy_scalar_rule(*args), policy_rule_args, 300, 0xE110_0053),
    "merkle_change_hash": (
        lambda args: merkle_change_hash(*args),
        merkle_change_hash_args,
        500,
        0xE110_0054,
    ),
    **{
        f"algorithm_total_arith_{'i' if signed else 'u'}{bits}": (
            (lambda model: lambda args: model(*args))(total_arith(signed, bits, bits < 32 and signed)),
            total_arith_args(signed, bits, bits < 32 and signed),
            400,
            seed,
        )
        for signed, bits, seed in [
            (False, 8, 0xE110_0063),
            (False, 16, 0xE110_0064),
            (False, 32, 0xE110_0065),
            (False, 64, 0xE110_0066),
            (True, 8, 0xE110_0067),
            (True, 16, 0xE110_0068),
            (True, 32, 0xE110_0069),
            (True, 64, 0xE110_006A),
        ]
    },
    "algorithm_tick_elapsed": (lambda args: tick_elapsed(*args), tick_args, 300, 0xE110_006B),
    "algorithm_tick_until_due": (lambda args: tick_until_due(*args), tick_args, 300, 0xE110_006C),
    "algorithm_lcg_table": (lambda args: lcg_table(*args), lcg_args, 100, 0xE110_006D),
}

#: Further streams of cases for a fixture above, each with a seed of its own, so a
#: case family added later does not move the cases of another. The digest's
#: late streams hold it to minutes from 3^16 on, which the reference's key could
#: not write and the first form of this document did not walk.
EXTRA_STREAMS = {
    "merkle_insert": [(merkle_insert_late_args, 200, 0xE110_0055)],
    "merkle_prune": [(merkle_prune_late_args, 120, 0xE110_0056)],
    "merkle_diff": [(merkle_diff_late_args, 250, 0xE110_0057)],
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
    write_laws = [
        Law("lww_write: the order the writes arrive in does not matter"),
        Law("lww_write is idempotent"),
    ]
    rga_laws = [
        Law("rga_insert: any delivery order that follows the predecessors gives one array"),
        Law("rga_insert is idempotent"),
        Law("rga_insert holds exactly the ids inserted"),
    ]
    remove_laws = [
        Law("rga_remove: the order the removals arrive in does not matter"),
        Law("rga_remove is idempotent"),
    ]
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

        # Writes whose stamps all differ, as the stamps of distinct writes do, to a
        # few places so that several of them meet.
        stamp_base = stamp_value(rng)
        writes, stamps = [], set()
        for _ in range(rng.between(1, 7)):
            stamp = near_stamp(rng, stamp_base) if rng.below(3) else stamp_value(rng)
            if hlc_key(stamp) not in stamps:
                stamps.add(hlc_key(stamp))
                writes.append(cell(rng.pick([0, 1, 2]), rng.pick([0, 1]), cell_value(rng), stamp))

        def written(sequence):
            held: list = []
            for write in sequence:
                held = lww_write(held, write)[1]
            return sorted(json.dumps(held_cell, sort_keys=True) for held_cell in held)

        write_laws[0].holds(written(writes) == written(shuffled(rng, writes)), writes)
        write_laws[1].holds(written(writes + shuffled(rng, writes)) == written(writes), writes)

        # Insertions of an array, each after the front or after an earlier one, and
        # larger than the one it follows, which is the id a replica issues.
        inserts, used = [], set()
        id_offset = rng.pick([0, 0, 2**32, 2**53, 2**63])
        for _ in range(rng.between(1, 8)):
            parent = rng.pick(inserts) if inserts and rng.below(4) else None
            after = parent["key"] if parent else 0
            key = rng.between(after + 1, after + 6) if parent else id_offset + rng.between(1, 6)
            if key not in used:
                used.add(key)
                inserts.append(rga_node(key, after))

        def delivered(operations):
            """An order the insertions can arrive in: each after the one it follows."""
            pending, placed, order = shuffled(rng, operations), {0}, []
            while pending:
                op = rng.pick([candidate for candidate in pending if candidate["after"] in placed])
                pending.remove(op)
                placed.add(op["key"])
                order.append(op)
            return order

        def arranged(order, start=()):
            nodes = list(start)
            for op in order:
                nodes = rga_insert(nodes, op)[1]
            return nodes

        first_array = arranged(delivered(inserts))
        rga_laws[0].holds(first_array == arranged(delivered(inserts)), inserts)
        rga_laws[1].holds(arranged(shuffled(rng, inserts), first_array) == first_array, inserts)
        rga_laws[2].holds(sorted(op["key"] for op in first_array) == sorted(used), inserts)

        removals = [rga_gone(rng.pick([1, 2, 3, 4]), rng.pick(NODE_EDGES)) for _ in range(rng.between(0, 6))]

        def marked(sequence):
            marks: list = []
            for removal in sequence:
                marks = rga_remove(marks, removal)[1]
            return marks

        keys_of = lambda marks: sorted(mark["key"] for mark in marks)  # noqa: E731
        remove_laws[0].holds(
            keys_of(marked(removals)) == keys_of(marked(shuffled(rng, removals)))
            and len(marked(removals)) == len({removal["key"] for removal in removals}),
            removals,
        )
        remove_laws[1].holds(marked(marked(removals) + removals) == marked(removals), removals)

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
    diff_minute_law = Law("merkle_diff answers a minute boundary, and 0 for digests that do not differ")
    for _ in range(LAW_RUNS):
        # Distinct nonzero hashes: a change whose hash is 0, or two on opposite sides
        # whose hashes are equal, cancel out of an XOR digest, and no digest of this
        # family can see them — a limit of the structure, not of a document.
        hashes = shuffled(rng, [rng.between(1, U32_MAX) for _ in range(12)])
        if len(set(hashes)) < len(hashes):
            continue
        base = merkle_late_minute(rng) if rng.below(2) else merkle_minute(rng)
        minutes = [
            clamp(base + rng.pick([0, 1, 2, 3, 9, 27, 243, 6561, 100_000]), 0, MERKLE_LAST_MINUTE)
            for _ in range(9)
        ]
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
        if divergence is not None:
            # merkle_divergence names the minute two replicas must exchange from, at a
            # minute boundary, and 0 when they do not differ.
            diff_minute_law.holds(
                divergence["millis"] % 60_000 == 0 and (divergence["differs"] or divergence["millis"] == 0),
                [left, right],
            )
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

    # What merkle_change_hash says of itself: no part of a change goes unmixed. Two
    # changes that differ in one 32-bit half of one part never hash alike, because
    # a step adds the part to a multiple of an odd number and the steps after it
    # are one-to-one on 32 bits; a hash that kept only the low half of a wall
    # time, an element or a node would fail it at once.
    word_law = Law("merkle_change_hash: changes that differ in one 32-bit half of one part hash apart")
    kind_law = Law("merkle_change_hash: an add and the tombstone that names it hash apart")
    same_law = Law("merkle_change_hash: two changes do not share the difference their add and tombstone make")
    for _ in range(LAW_RUNS):
        change = merkle_change_hash_args(rng)[0]
        if change["wallTime"] < 0:
            continue
        other = dict(change)
        field_name = rng.pick(["element", "wallTime", "counter", "nodeId"])
        high = field_name != "counter" and rng.below(2) == 1
        limit = I64_MAX if field_name == "wallTime" else (U32_MAX if field_name == "counter" else U64_MAX)
        shift = 2**32 if high else 1
        moved = change[field_name] + shift * rng.between(1, 1000)
        if moved > limit:
            moved = change[field_name] - shift * rng.between(1, 1000)
        if not 0 <= moved <= limit or (not high and moved // 2**32 != change[field_name] // 2**32):
            continue
        other[field_name] = moved
        kind = rng.pick([0, 1])
        word_law.holds(
            answer_of(merkle_change_hash(change, kind)) != answer_of(merkle_change_hash(other, kind)),
            [change, other, kind],
        )
        kind_law.holds(
            answer_of(merkle_change_hash(change, 0)) != answer_of(merkle_change_hash(change, 1)),
            [change],
        )
        # The difference an add and its tombstone make to a digest is its own: two
        # registers do not give one difference, or two copies lacking one of each
        # would XOR to the same root as copies that lack nothing.
        if other["wallTime"] >= 0:
            same_law.holds(
                answer_of(merkle_change_hash(change, 0)) ^ answer_of(merkle_change_hash(change, 1))
                != answer_of(merkle_change_hash(other, 0)) ^ answer_of(merkle_change_hash(other, 1)),
                [change, other],
            )
    word_law.was_asked()
    kind_law.was_asked()
    same_law.was_asked()

    stable_order_law = Law("stable_point does not depend on the order of the devices")
    stable_idle_law = Law("stable_point is not held back by a device idle past maxIdle")
    stable_join_law = Law("stable_point is never raised by a device that joins while one takes part")
    stable_least_law = Law("stable_point is the least cursor of the devices taking part, 0 with none")
    for _ in range(LAW_RUNS):
        cursors, last_seen, now, max_idle = stable_point_args(rng)
        answer = answer_of(stable_point(cursors, last_seen, now, max_idle))
        inputs = [cursors, last_seen, now, max_idle]
        order = shuffled(rng, list(range(len(cursors))))
        if len(cursors) == len(last_seen):
            shuffled_answer = stable_point(
                [cursors[i] for i in order], [last_seen[i] for i in order], now, max_idle
            )
            stable_order_law.holds(shuffled_answer == stable_point(cursors, last_seen, now, max_idle), inputs)
        if answer is None:
            continue
        taking = [c for c, seen in zip(cursors, last_seen) if now - seen <= max_idle]
        stable_least_law.holds(answer == (min(taking) if taking else 0), inputs)
        gone = now - max_idle - 1
        if fits(gone, I64_MIN, I64_MAX) and fits(now - gone, I64_MIN, I64_MAX):
            stable_idle_law.holds(
                answer_of(stable_point(cursors + [rng.pick(CURSOR_EDGES)], last_seen + [gone], now, max_idle))
                == answer,
                inputs,
            )
        if taking:
            joined = answer_of(stable_point(cursors + [rng.pick(CURSOR_EDGES)], last_seen + [now], now, max_idle))
            stable_join_law.holds(joined is not None and joined <= answer, inputs)

    # The code-point count, held to what a count of code points must satisfy
    # whatever the bytes are. The strings come from characters, not from bytes, so
    # the count a law expects is the number of characters that were encoded.
    utf8_sum_law = Law("utf8_length of two texts joined is the sum of their lengths")
    utf8_chars_law = Law("utf8_length of an encoded text is the number of its characters")
    utf8_cut_law = Law("a text cut inside its last multi-byte character is not UTF-8")
    utf8_within_law = Law("utf8_within is utf8_length being at most the limit")
    utf8_bytes_law = Law("a code-point count is never more than the bytes, nor less than a quarter of them")
    for _ in range(LAW_RUNS):
        chars = [draw_scalar(rng) for _ in range(rng.between(0, 12))]
        other = [draw_scalar(rng) for _ in range(rng.between(0, 12))]
        text = list("".join(map(chr, chars)).encode("utf-8"))
        tail = list("".join(map(chr, other)).encode("utf-8"))
        inputs = [text, tail]
        utf8_chars_law.holds(answer_of(utf8_length(text)) == len(chars), inputs)
        utf8_sum_law.holds(
            answer_of(utf8_length(text + tail)) == len(chars) + len(other), inputs
        )
        counted = len(chars)
        limit = rng.pick([0, 1, counted, counted + 1, max(counted - 1, 0), U32_MAX])
        utf8_within_law.holds(answer_of(utf8_within(text, limit)) == (counted <= limit), [text, limit])
        utf8_bytes_law.holds(counted <= len(text) <= 4 * counted, inputs)
        if chars and chars[-1] > 0x7F:
            utf8_cut_law.holds(utf8_length(text[:-1]) == ("fails", "precondition"), inputs)

    for law in (utf8_sum_law, utf8_chars_law, utf8_cut_law, utf8_within_law, utf8_bytes_law):
        law.was_asked()

    # The counter a server resumes from, held to what issuing ids needs of it: it
    # is never below an id that was given out, it names one of them, a later look
    # at more devices cannot lower it, resuming from it changes nothing, and the id
    # mailbox_assign gives next is above every one.
    resume_floor_law = Law("mailbox_resume is not below any id it was given")
    resume_named_law = Law("mailbox_resume is one of the ids it was given")
    resume_order_law = Law("mailbox_resume does not depend on the order of the ids")
    resume_more_law = Law("mailbox_resume is never lowered by a device that reports more")
    resume_again_law = Law("mailbox_resume of its own answer is that answer")
    resume_next_law = Law("the id mailbox_assign gives after mailbox_resume is above every id seen")
    for _ in range(LAW_RUNS):
        restored, held, cursors = mailbox_resume_args(rng)
        inputs = [restored, held, cursors]
        answer = answer_of(mailbox_resume(restored, held, cursors))
        reordered = mailbox_resume(restored, shuffled(rng, held), shuffled(rng, cursors))
        resume_order_law.holds(reordered == mailbox_resume(restored, held, cursors), inputs)
        if answer is None:
            continue
        seen = [restored] + held + cursors
        resume_floor_law.holds(all(answer >= value for value in seen), inputs)
        resume_named_law.holds(answer in seen, inputs)
        resume_again_law.holds(answer_of(mailbox_resume(answer, held, cursors)) == answer, inputs)
        extra = rng.pick(RESUME_EDGES)
        more = answer_of(mailbox_resume(restored, held, cursors + [extra]))
        resume_more_law.holds(more is not None and more >= answer, inputs + [extra])
        key = envelope_id(rng)
        issued = answer_of(mailbox_assign([], answer, key["hi"], key["lo"]))
        resume_next_law.holds(issued is not None and issued > max(seen), inputs)

    for law in (
        resume_floor_law,
        resume_named_law,
        resume_order_law,
        resume_more_law,
        resume_again_law,
        resume_next_law,
    ):
        law.was_asked()

    # A remove, as the documents of an observed-remove set spell it: it tombstones
    # the live entries of the element it observes (orset_observed), and nothing a
    # node adds later. The four documents together must give the set its meaning.
    remove_law = Law("an orset remove takes out the element it observed")
    remove_only_law = Law("an orset remove leaves every other element as it was")
    add_wins_law = Law("an orset add concurrent with a remove survives it")
    contains_law = Law("orset_contains is orset_observed having something to observe")
    for _ in range(LAW_RUNS):
        pool = entry_pool(rng)
        adds, tombstones = entry_sample(rng, pool), entry_sample(rng, pool)
        element = rng.pick([held["element"] for held in pool])
        live = orset_live(adds, tombstones)[1]
        observed = orset_observed(live, element)[1]
        contains_law.holds(orset_contains(live, element)[1] == bool(observed), [adds, tombstones, element])

        after = orset_live(adds, orset_union(tombstones, observed)[1])[1]
        inputs = [adds, tombstones, element]
        remove_law.holds(not orset_contains(after, element)[1], inputs)
        remove_only_law.holds(
            [held for held in after if held["element"] != element]
            == [held for held in live if held["element"] != element],
            inputs,
        )
        # An add the remove could not have seen: of the same element, held neither as
        # an add nor as a tombstone, so it is nowhere in `live` when the remove is made.
        concurrent = entry_near(rng, rng.pick(pool))
        concurrent = dict(concurrent, element=element)
        if concurrent not in adds and concurrent not in tombstones:
            survivors = orset_live(adds + [concurrent], orset_union(tombstones, observed)[1])[1]
            add_wins_law.holds(orset_contains(survivors, element)[1], [adds, tombstones, element, concurrent])

    # hlc_text says its text is fixed-width, so comparing two texts byte by byte
    # orders them as hlc_compare does; a change log keys its rows by the text, and
    # a digest hashes it. Two stamps in the range the text can name, sharing fields
    # as often as the cases do.
    text_order_law = Law("hlc_text orders byte by byte as hlc_compare orders the stamps")

    def nameable(stamp):
        return dict(
            stamp,
            wallTime=min(max(stamp["wallTime"], 0), TEXT_WALL_MAX) if abs(stamp["wallTime"]) < 2**62
            else rng.between(0, TEXT_WALL_MAX),
            counter=stamp["counter"] if stamp["counter"] <= U16_MAX else rng.pick([0, 1, U16_MAX]),
        )

    for _ in range(LAW_RUNS):
        first = nameable(stamp_value(rng))
        second = nameable(near_stamp(rng, first))
        one, other = answer_of(hlc_text(first)), answer_of(hlc_text(second))
        text_order_law.holds(
            one is not None and other is not None and (one > other) - (one < other) == hlc_compare(first, second)[1],
            [first, second],
        )

    # The titled places of a shared list. `text_slot_latest` says the later version
    # wins and an equal one keeps `a`, which makes it a join once no two versions
    # tie; `text_slot_retitled` says only an empty text changes, and that to a text
    # that is not empty; `text_slot_holds` says it asks for the place and the text
    # and nothing else.
    latest_laws = {
        name: Law(f"text_slot_latest {name}")
        for name in (
            "commutes for one place of two versions",
            "associates for three versions",
            "is idempotent",
            "keeps a on equal versions",
            "takes the greater version and that write's text",
            "keeps a's place",
        )
    }
    retitled_laws = {
        name: Law(f"text_slot_retitled {name}")
        for name in (
            "is idempotent",
            "keeps the places, their order, slots and versions",
            "changes only an empty text",
            "leaves no text empty",
        )
    }
    holds_laws = {
        name: Law(f"text_slot_holds {name}")
        for name in (
            "holds every place it holds, whatever version asks",
            "does not depend on the order of the places",
            "keeps holding what it held when a place is added",
            "holds nothing of an empty list",
            "asks the retitled list for \"untitled\" exactly where a place was empty or already titled so",
        )
    }
    for _ in range(LAW_RUNS):
        first, second = text_slot_latest_args(rng)
        picked = answer_of(text_slot_latest(first, second))
        latest_laws["is idempotent"].holds(answer_of(text_slot_latest(first, first)) == first, first)
        latest_laws["keeps a on equal versions"].holds(
            answer_of(text_slot_latest(first, dict(second, version=first["version"]))) == first, [first, second]
        )
        winner = second if second["version"] > first["version"] else first
        latest_laws["takes the greater version and that write's text"].holds(
            picked["version"] == max(first["version"], second["version"]) and picked["text"] == winner["text"],
            [first, second],
        )
        latest_laws["keeps a's place"].holds(picked["slot"] == first["slot"], [first, second])

        same = dict(second, slot=first["slot"])
        if same["version"] != first["version"]:
            latest_laws["commutes for one place of two versions"].holds(
                answer_of(text_slot_latest(first, same)) == answer_of(text_slot_latest(same, first)),
                [first, same],
            )
            third = dict(slot_value(rng), slot=first["slot"], version=slot_version(rng))
            if third["version"] not in (first["version"], same["version"]):
                join = lambda x, y: answer_of(text_slot_latest(x, y))
                latest_laws["associates for three versions"].holds(
                    join(join(first, same), third) == join(first, join(same, third)), [first, same, third]
                )

        held = text_slot_retitled_args(rng)[0]
        shown = answer_of(text_slot_retitled(held))
        retitled_laws["is idempotent"].holds(answer_of(text_slot_retitled(shown)) == shown, held)
        retitled_laws["keeps the places, their order, slots and versions"].holds(
            [(s["slot"], s["version"]) for s in shown] == [(s["slot"], s["version"]) for s in held], held
        )
        retitled_laws["changes only an empty text"].holds(
            all(after["text"] == before["text"] for before, after in zip(held, shown) if before["text"]), held
        )
        retitled_laws["leaves no text empty"].holds(all(s["text"] for s in shown), held)

        listed, probe = text_slot_holds_args(rng)
        asked = answer_of(text_slot_holds(listed, probe))
        if listed:
            member = rng.pick(listed)
            holds_laws["holds every place it holds, whatever version asks"].holds(
                answer_of(text_slot_holds(listed, dict(member, version=slot_version(rng)))), [listed, member]
            )
        holds_laws["does not depend on the order of the places"].holds(
            answer_of(text_slot_holds(shuffled(rng, listed), probe)) == asked, [listed, probe]
        )
        if len(listed) < SLOT_CAPACITY:
            holds_laws["keeps holding what it held when a place is added"].holds(
                not asked or answer_of(text_slot_holds(listed + [slot_value(rng)], probe)), [listed, probe]
            )
        holds_laws["holds nothing of an empty list"].holds(not answer_of(text_slot_holds([], probe)), probe)
        if len(listed) <= SLOT_CAPACITY:
            titled = answer_of(text_slot_holds(answer_of(text_slot_retitled(listed)), dict(probe, text=UNTITLED)))
            expected = any(s["slot"] == probe["slot"] and s["text"] in ("", UNTITLED) for s in listed)
            holds_laws[
                "asks the retitled list for \"untitled\" exactly where a place was empty or already titled so"
            ].holds(titled == expected, [listed, probe])

    # The values a field of a shared to-do list holds beside the dots of its writes.
    # `mv_value_keep` takes a write in once however often it arrives, and
    # `mv_value_drop` forgets exactly the write a tombstone names, so a copy that is
    # sent the same writes and tombstones in another order holds the same values.
    mv_laws = {
        name: Law(f"mv_value {name}")
        for name in (
            "keep holds what it kept",
            "keep is idempotent",
            "keep adds the value once and leaves the others in their order",
            "keep adds nothing for a write it holds",
            "drop forgets what it dropped",
            "drop leaves the others in their order",
            "drop of a write that is not held changes nothing",
            "keep and drop of different writes commute as sets",
            "holds nothing of an empty list",
        )
    }
    for _ in range(LAW_RUNS):
        listed, probe = mv_value_holds_args(rng)
        kept = answer_of(mv_value_keep(listed, probe))
        had = answer_of(mv_value_holds(listed, probe))
        mv_laws["keep holds what it kept"].holds(answer_of(mv_value_holds(kept, probe)), [listed, probe])
        mv_laws["keep is idempotent"].holds(answer_of(mv_value_keep(kept, probe)) == kept, [listed, probe])
        mv_laws["keep adds the value once and leaves the others in their order"].holds(
            had or (kept == listed + [probe]), [listed, probe]
        )
        mv_laws["keep adds nothing for a write it holds"].holds(not had or kept == listed, [listed, probe])
        dropped = answer_of(mv_value_drop(listed, probe))
        mv_laws["drop forgets what it dropped"].holds(not answer_of(mv_value_holds(dropped, probe)), [listed, probe])
        mv_laws["drop leaves the others in their order"].holds(
            dropped == [there for there in listed if not same_write(there, probe)], [listed, probe]
        )
        mv_laws["drop of a write that is not held changes nothing"].holds(had or dropped == listed, [listed, probe])
        other = mv_probe(rng, listed)
        if not same_write(other, probe):
            one_way = answer_of(mv_value_drop(answer_of(mv_value_keep(listed, probe)), other))
            other_way = answer_of(mv_value_keep(answer_of(mv_value_drop(listed, other)), probe))
            mv_laws["keep and drop of different writes commute as sets"].holds(
                sorted(map(json.dumps, one_way)) == sorted(map(json.dumps, other_way)), [listed, probe, other]
            )
        mv_laws["holds nothing of an empty list"].holds(not answer_of(mv_value_holds([], probe)), probe)

    every_law = (
        union_laws + [prune_law] + order_laws + clock_laws + [classify_law] + apply_laws + ack_laws
        + [page_law, insert_law, dedup_law, hold_law, gcra_law, retry_law, utc_law]
        + [insert_order_law, cancel_law, prune_keeps_law, diff_bound_law, diff_pruned_law, membership_law]
        + [stable_order_law, stable_idle_law, stable_join_law, stable_least_law]
        + [remove_law, remove_only_law, add_wins_law, contains_law, text_order_law, diff_minute_law]
        + write_laws + rga_laws + remove_laws
        + list(latest_laws.values()) + list(retitled_laws.values()) + list(holds_laws.values())
        + list(mv_laws.values())
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
            before = value_of(max(earlier, key=hlc_key)) if earlier else (0, 0, 0)
            restore = hlc(step["restoreWallTime"], step["restoreCounter"], step["restoreNodeId"])

            if step["verdict"] == 0:
                value_law.holds(value_of(
                    {"valueWallTime": restore["wallTime"], "valueCounter": restore["counter"],
                     "valueNodeId": restore["nodeId"]}) == before, [inputs, step])
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
                            2, batch, restore,
                        )
                    )
            elif step["verdict"] == 1:
                safe_law.holds(latest["batchId"] != batch, [inputs, step])
            else:
                safe_law.holds(step["verdict"] in (2, 3), [inputs, step])
        if room and any(step["verdict"] == 0 for step in plan):
            again = answer_of(undo_field_plan(applied, batch, floor))
            expected = [
                dict(
                    step,
                    verdict=2 if step["verdict"] == 0 else step["verdict"],
                    **(
                        {"restoreWallTime": 0, "restoreCounter": 0, "restoreNodeId": 0}
                        if step["verdict"] == 0
                        else {}
                    ),
                )
                for step in plan
            ]
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


def validate_sheet_laws() -> None:
    """The claims `sheet_settle`, `sheet_show` and `sheet_apply` make about
    themselves (see [`Law`]); the grids come from the same generator the cases
    do."""
    rng = SplitMix64(0xE110_5EE7)
    followed_law = Law("sheet_settle: the passes agree with following the references depth first")
    order_law = Law("sheet_show does not depend on the order the cells are held in")
    places_law = Law("sheet_show shows each place once, in order of row then column")
    reads_law = Law("a formula that reads a cell shows what the cell shows")
    writes_law = Law("sheet_show: the order the writes arrive in does not matter")
    cycle_law = Law("two acyclic formulas that read each other show a cycle on every replica")
    commute_law = Law("sheet_apply: a + b and a * b commute")
    back_law = Law("sheet_apply: a + b - b is a when nothing faults")
    divide_law = Law("sheet_apply: a / b truncates toward zero")
    fault_law = Law("sheet_apply: a fault leaves the value 0")

    def shown_of(cells: list, rows: list, gone: list, cols: list, gone_cols: list) -> list:
        return sheet_show(cells, sheet_settle(cells, rows, gone, cols, gone_cols)[1])[1]

    def replica(writes: list) -> list:
        held: list = []
        for write in writes:
            held = lww_write(held, write)[1]
        return held

    for _ in range(LAW_RUNS * 2):
        grid = sheet_grid(rng)
        cells, rows, gone, cols, gone_cols = grid
        trail = sheet_settle(*grid)[1]
        passed = {trail[at]: (trail[at + 1], trail[at + 2]) for at in range(0, len(trail), 3)}
        followed_law.holds(
            len(trail) == 3 * len(passed) and passed == sheet_semantics(*grid), list(grid)
        )

        shown = sheet_show(cells, trail)[1]
        order_law.holds(shown_of(shuffled(rng, cells), rows, gone, cols, gone_cols) == shown, list(grid))
        places = [(record["row"], record["col"]) for record in shown]
        places_law.holds(
            places == sorted(set(places)) and set(places) == {(held["row"], held["col"]) for held in cells},
            list(grid),
        )
        by_place = {(record["row"], record["col"]): (record["value"], record["fault"]) for record in shown}
        for held in cells:
            if held["op"] == 1 and held["aRef"]:
                reads_law.holds(
                    by_place[(held["row"], held["col"])]
                    == by_place.get((held["aRow"], held["aCol"]), (0, 0)),
                    list(grid),
                )

        # The cells as the writes that made them, stamped one after another in no
        # order that matters, and concurrent writes to some of the same places.
        where = [(held["row"], held["col"]) for held in cells]
        writes = []
        for held in cells:
            writes.append(
                cell(held["row"], held["col"], held["value"], hlc(100 + len(writes), 0, 1 + len(writes) % 3),
                     {key: held[key] for key in NUMBER})
            )
        for _ in range(rng.between(0, 4) if cells else 0):
            source = rng.pick(cells)
            row, col = rng.pick(where)
            writes.append(
                cell(row, col, source["value"], hlc(100 + len(writes), 0, 1 + len(writes) % 3),
                     {key: source[key] for key in NUMBER})
            )
        writes_law.holds(
            shown_of(replica(writes), rows, gone, cols, gone_cols)
            == shown_of(replica(shuffled(rng, writes)), rows, gone, cols, gone_cols),
            [writes, rows, gone, cols, gone_cols],
        )

        one = (rng.pick(SHEET_IDS), rng.pick(SHEET_IDS))
        other = (rng.pick([id_ for id_ in SHEET_IDS if id_ != one[0]]), rng.pick(SHEET_IDS))
        mine = cell(*one, 0, hlc(100, 0, 1), formula(1, reads(*other)))
        theirs = cell(*other, 0, hlc(100, 0, 2), formula(1, reads(*one)))
        both = [shown_of(replica(order), [], [], [], []) for order in ([mine, theirs], [theirs, mine])]
        cycle_law.holds(
            both[0] == both[1] and all(record["fault"] == CYCLE for record in both[0]) and len(both[0]) == 2,
            [mine, theirs],
        )

        x, y = sheet_number(rng), sheet_number(rng)
        for op in (2, 4):
            commute_law.holds(sheet_apply(op, x, y) == sheet_apply(op, y, x), [op, x, y])
        total = sheet_apply(2, x, y)[1]
        if total["fault"] == 0:
            back = sheet_apply(3, total["value"], y)[1]
            back_law.holds(back == sheet_value(x, 0), [x, y])
        quotient = sheet_apply(5, x, y)[1]
        if quotient["fault"] == 0:
            left = x - quotient["value"] * y
            divide_law.holds(abs(left) < abs(y) and (left == 0 or (left > 0) == (x > 0)), [x, y])
        for op in (2, 3, 4, 5):
            result = sheet_apply(op, x, y)[1]
            fault_law.holds(result["fault"] == 0 or result["value"] == 0, [op, x, y])

    for law in (followed_law, order_law, places_law, reads_law, writes_law, cycle_law,
                commute_law, back_law, divide_law, fault_law):
        law.was_asked()


def validate_policy_laws() -> None:
    """The claims the policy documents (SCE_FORGE.md §5.6) make about
    themselves (see [`Law`]); the policies come from the generator the cases do."""
    rng = SplitMix64(0xE110_90C1)
    same_law = Law("policy_scalar_layer: a policy layered over itself is itself")
    keeps_law = Law("policy_scalar_layer: a field whose bit is clear stays the base's")
    takes_law = Law("policy_scalar_layer: a field whose bit is set is the over's")
    tighten_law = Law("policy_scalar_layer: a layer never loosens strict")
    names_law = Law("policy_scalar_layer: a bit that names no field changes nothing")
    fold_law = Law("policy_scalar_layer: folding under one mask does not depend on where it is grouped")
    strict_law = Law("policy_scalar_layer: strict does not depend on the mask")
    rule_law = Law("policy_scalar_rule: valid is exactly in range, and a limit when strict")
    order_law = Law("policy_scalar_rule: an out-of-range limit is reported as the range")

    for _ in range(LAW_RUNS):
        base, over, mask = policy_layer_args(rng)
        third = policy_scalar_settings(rng)
        folded = answer_of(policy_scalar_layer(base, over, mask))
        inputs = [base, over, mask]

        same_law.holds(answer_of(policy_scalar_layer(base, base, mask)) == base, inputs)
        for bit, field in enumerate(("limitDays", "burst", "locked")):
            wanted = over[field] if mask >> bit & 1 else base[field]
            (takes_law if mask >> bit & 1 else keeps_law).holds(folded[field] == wanted, inputs)
        tighten_law.holds(folded["strict"] >= base["strict"] and folded["strict"] >= over["strict"], inputs)
        names_law.holds(answer_of(policy_scalar_layer(base, over, mask & 7)) == folded, inputs)
        strict_law.holds(
            all(answer_of(policy_scalar_layer(base, over, other))["strict"] == folded["strict"]
                for other in (0, 7, U64_MAX)),
            inputs,
        )
        left = answer_of(policy_scalar_layer(answer_of(policy_scalar_layer(base, over, mask)), third, mask))
        right = answer_of(policy_scalar_layer(base, answer_of(policy_scalar_layer(over, third, mask)), mask))
        fold_law.holds(left == right, [base, over, third, mask])

        strict, limit = rng.below(2) == 1, policy_limit(rng)
        verdict = answer_of(policy_scalar_rule(strict, limit))
        in_range = 0 <= limit <= POLICY_LIMIT_MAX
        rule_law.holds(verdict["valid"] == (in_range and (not strict or limit > 0)), [strict, limit])
        order_law.holds(
            in_range or verdict == {"valid": False, "reason": "limit_days_out_of_range"}, [strict, limit]
        )

    for law in (same_law, keeps_law, takes_law, tighten_law, names_law, fold_law, strict_law, rule_law, order_law):
        law.was_asked()

    # What the pattern's host step is for, held as a fact and not left to prose: two
    # policies that each pass the rule can fold into one that does not, a strict
    # base with a limit under a layer that sets the limit to zero, so a fold is
    # validated after it is made and not only before.
    strict_base = {"strict": True, "limitDays": 7, "burst": 50, "locked": False}
    lax_layer = {"strict": False, "limitDays": 0, "burst": 50, "locked": False}
    fold = answer_of(policy_scalar_layer(strict_base, lax_layer, 1))
    if not (
        answer_of(policy_scalar_rule(strict_base["strict"], strict_base["limitDays"]))["valid"]
        and answer_of(policy_scalar_rule(lax_layer["strict"], lax_layer["limitDays"]))["valid"]
        and not answer_of(policy_scalar_rule(fold["strict"], fold["limitDays"]))["valid"]
    ):
        raise SystemExit("two policies that pass the rule no longer fold into one that fails it")


def render(value) -> str:
    """A value the way the hand-written cases write it: a record as
    `{ "year": 1970, ... }`, a list as `[1, 2]`, a boolean as `true`, a number
    as itself."""
    if isinstance(value, dict):
        members = ", ".join(f'"{key}": {render(member)}' for key, member in value.items())
        return "{ " + members + " }"
    if isinstance(value, list):
        return "[" + ", ".join(render(member) for member in value) + "]"
    return written(json.dumps(value, ensure_ascii=False))


def written(encoded: str) -> str:
    """JSON text with each character that has no visible form (a control, a
    format character such as a byte-order mark, an unassigned code point or a
    noncharacter) spelled as a `\\u` escape, and every other character as itself,
    so the file reads the same in an editor as in a JSON reader."""

    def spelled(match: re.Match) -> str:
        char = match.group()
        if unicodedata.category(char) not in ("Cc", "Cf", "Cn", "Co", "Cs"):
            return char
        code = ord(char)
        if code < 0x10000:
            return f"\\u{code:04x}"
        code -= 0x10000
        return f"\\u{0xD800 + (code >> 10):04x}\\u{0xDC00 + (code & 0x3FF):04x}"

    return re.sub(r"[^\x00-\x7e]", spelled, encoded)


def identity(args) -> str:
    """What makes two cases the same input: a record or a list is not hashable."""
    return json.dumps(args, sort_keys=True)


def case_line(args, answer) -> str:
    kind, value = answer
    tail = f'"expected": {render(value)}' if kind == "ok" else f'"fails": "{value}"'
    return f'        {{ "args": {render(args)}, {tail}, "note": "{MARK}" }}'


def section(text: str, fixture: str):
    """(start, end) of `fixture`'s `cases` array contents (a validator's is its
    `sequence`): from just after `[` to just before the `]` that closes it."""
    anchor = text.index(f'"fixture": "{fixture}"')
    arrays = [text.find(f'"{key}": [', anchor) for key in ("cases", "sequence")]
    first = min(at for at in arrays if at >= 0)
    opening = text.index("[", first) + 1
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
        made = []
        for stream_draw, stream_count, stream_seed in [(draw, count, seed)] + EXTRA_STREAMS.get(fixture, []):
            rng = SplitMix64(stream_seed)
            guard, wanted = 0, len(made) + stream_count
            while len(made) < wanted:
                guard += 1
                if guard > stream_count * 50:
                    raise SystemExit(f"{fixture}: cannot draw {stream_count} distinct inputs")
                args = stream_draw(rng)
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
    validate_sheet_laws()
    validate_policy_laws()
    original = REFERENCE.read_text(encoding="utf-8")
    json.loads(original)
    text, report = regenerate(original)
    # The content-line codec's cases are not arguments and answers but a
    # component and its bytes, so their model writes the whole fixture (see the
    # module's own page) rather than the `fuzz` lines of a list.
    text, content_line_report = content_line_model.regenerate(text)
    json.loads(text)
    for fixture, checked, dropped, made, kinds in report:
        print(
            f"{fixture}: model agrees with {checked} hand cases; "
            f"{made} generated (was {dropped}): {kinds}"
        )
    print(content_line_report)
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
