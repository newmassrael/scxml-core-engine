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
"""

from __future__ import annotations

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


#: fixture -> (model, argument generator, how many cases, seed).
#: A seed is per fixture and never reused, so adding a case to one fixture does
#: not move another's.
FIXTURES = {
    "algorithm_checked_arith": (lambda args: arith(*args), arith_args, 600, 0xE110_0001),
    "algorithm_checked_call": (lambda args: checked_call(*args), call_args, 300, 0xE110_0002),
    "algorithm_checked_narrow": (lambda args: checked_narrow(*args), narrow_args, 300, 0xE110_0003),
}


def case_line(args, answer) -> str:
    kind, value = answer
    tail = f'"expected": {value}' if kind == "ok" else f'"fails": "{value}"'
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
        kinds = {"ok": 0, "overflow": 0, "divide-by-zero": 0}
        for line in made:
            for name in kinds:
                if (name == "ok" and '"expected"' in line) or f'"fails": "{name}"' in line:
                    kinds[name] += 1
        report.append((fixture, checked, dropped, len(made), kinds))
    return text, report


def main(argv) -> int:
    check = "--check" in argv
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
