# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""Every wrapping and saturating operation of every width is its definition
(docs/adr/0063, section 1): the exact result ``e``, then the value congruent to
``e`` modulo ``2**W`` (two's complement for a signed width), or
``max(m, min(M, e))``.

The expected values are computed here from the width's bit count alone: a
wrap keeps the low ``W`` bits of ``e`` and reads them as two's complement for a
signed width, which is a different computation from the runtime's offset
modulo the span. The operands are each width's ``m``, ``M``, 0, 1 and -1 and
their neighbours, and ``2**(W/2)`` and ``2**(W/2) + 1``, whose products pass
``2**W`` and stay within ``2**(2W)``. The pinned cases after the sweep spell
the same points by hand."""

import itertools

import pytest

from sce_forge_runtime import algorithm

# (name, width, bits, signed)
WIDTHS = [
    ("I8", algorithm.I8, 8, True),
    ("I16", algorithm.I16, 16, True),
    ("I32", algorithm.I32, 32, True),
    ("I64", algorithm.I64, 64, True),
    ("U8", algorithm.U8, 8, False),
    ("U16", algorithm.U16, 16, False),
    ("U32", algorithm.U32, 32, False),
    ("U64", algorithm.U64, 64, False),
]

BINARY = {
    "add": lambda a, b: a + b,
    "sub": lambda a, b: a - b,
    "mul": lambda a, b: a * b,
}


def bounds(bits, signed):
    if signed:
        return -(2 ** (bits - 1)), 2 ** (bits - 1) - 1
    return 0, 2**bits - 1


def wrapped(e, bits, signed):
    """The low ``bits`` bits of ``e``, read as two's complement when signed."""
    low = e & ((1 << bits) - 1)
    if signed and low >> (bits - 1):
        return low - (1 << bits)
    return low


def saturated(e, bits, signed):
    lo, hi = bounds(bits, signed)
    return max(lo, min(hi, e))


def operands(bits, signed):
    lo, hi = bounds(bits, signed)
    half = 2 ** (bits // 2)
    values = {lo, lo + 1, 0, 1, 2, half, half + 1, hi - 1, hi}
    if signed:
        values |= {-1, -2}
    return sorted(values)


@pytest.mark.parametrize("name, width, bits, signed", WIDTHS, ids=[w[0] for w in WIDTHS])
@pytest.mark.parametrize("op", sorted(BINARY))
def test_a_binary_operation_is_its_definition_on_every_pair(name, width, bits, signed, op):
    exact = BINARY[op]
    wrapping = getattr(width, f"wrapping_{op}")
    saturating = getattr(width, f"saturating_{op}")
    for a, b in itertools.product(operands(bits, signed), repeat=2):
        e = exact(a, b)
        assert wrapping(a, b) == wrapped(e, bits, signed), f"{name}.wrapping_{op}({a}, {b})"
        assert saturating(a, b) == saturated(e, bits, signed), f"{name}.saturating_{op}({a}, {b})"


@pytest.mark.parametrize("name, width, bits, signed", WIDTHS, ids=[w[0] for w in WIDTHS])
def test_a_negation_is_its_definition_on_every_operand(name, width, bits, signed):
    for a in operands(bits, signed):
        assert width.wrapping_neg(a) == wrapped(-a, bits, signed), f"{name}.wrapping_neg({a})"
        assert width.saturating_neg(a) == saturated(-a, bits, signed), f"{name}.saturating_neg({a})"


@pytest.mark.parametrize("name, width, bits, signed", WIDTHS, ids=[w[0] for w in WIDTHS])
def test_every_result_lies_in_its_width(name, width, bits, signed):
    lo, hi = bounds(bits, signed)
    assert (width.lo, width.hi) == (lo, hi)
    xs = operands(bits, signed)
    for a, b in itertools.product(xs, repeat=2):
        for op in BINARY:
            for rule in ("wrapping", "saturating"):
                r = getattr(width, f"{rule}_{op}")(a, b)
                assert lo <= r <= hi, f"{name}.{rule}_{op}({a}, {b}) = {r}"


@pytest.mark.parametrize(
    "call, want",
    [
        (lambda: algorithm.U8.wrapping_add(255, 1), 0),
        (lambda: algorithm.U8.wrapping_sub(0, 1), 255),
        (lambda: algorithm.I8.wrapping_add(127, 1), -128),
        (lambda: algorithm.I8.wrapping_sub(-128, 1), 127),
        (lambda: algorithm.I8.wrapping_add(-128, -1), 127),
        (lambda: algorithm.I8.wrapping_sub(127, -1), -128),
        (lambda: algorithm.I8.wrapping_neg(-128), -128),
        (lambda: algorithm.U8.wrapping_neg(1), 255),
        (lambda: algorithm.U8.wrapping_neg(0), 0),
        (lambda: algorithm.U32.wrapping_sub(5, 2**32 - 1), 6),
        (lambda: algorithm.I64.wrapping_add(2**63 - 1, 1), -(2**63)),
        (lambda: algorithm.I64.wrapping_sub(-(2**63), 1), 2**63 - 1),
        (lambda: algorithm.U64.wrapping_sub(0, 1), 2**64 - 1),
        (lambda: algorithm.I64.wrapping_neg(-(2**63)), -(2**63)),
    ],
)
def test_each_wrap_point_wraps_to_the_other_bound(call, want):
    assert call() == want


@pytest.mark.parametrize(
    "call, want",
    [
        (lambda: algorithm.U8.wrapping_mul(17, 17), 33),
        (lambda: algorithm.I8.wrapping_mul(17, 17), 33),
        (lambda: algorithm.I8.wrapping_mul(-17, 17), -33),
        (lambda: algorithm.U16.wrapping_mul(257, 257), 513),
        (lambda: algorithm.I16.wrapping_mul(257, 257), 513),
        (lambda: algorithm.U32.wrapping_mul(65537, 65537), 131073),
        (lambda: algorithm.I32.wrapping_mul(65537, 65537), 131073),
        (lambda: algorithm.U64.wrapping_mul(2**32 + 1, 2**32 + 1), 2**33 + 1),
        (lambda: algorithm.I64.wrapping_mul(2**32 + 1, 2**32 + 1), 2**33 + 1),
        (lambda: algorithm.U64.wrapping_mul(2**64 - 1, 2**64 - 1), 1),
        (lambda: algorithm.U64.wrapping_mul(2**64 - 1, 2), 2**64 - 2),
        (lambda: algorithm.I8.wrapping_mul(-128, -1), -128),
        (lambda: algorithm.I64.wrapping_mul(-(2**63), -1), -(2**63)),
    ],
)
def test_a_product_past_the_width_keeps_its_residue(call, want):
    assert call() == want


@pytest.mark.parametrize(
    "call, want",
    [
        (lambda: algorithm.U8.saturating_add(255, 1), 255),
        (lambda: algorithm.U8.saturating_sub(0, 1), 0),
        (lambda: algorithm.U8.saturating_sub(3, 5), 0),
        (lambda: algorithm.I8.saturating_add(127, 1), 127),
        (lambda: algorithm.I8.saturating_add(-128, -1), -128),
        (lambda: algorithm.I8.saturating_sub(-128, 1), -128),
        (lambda: algorithm.I8.saturating_sub(127, -1), 127),
        (lambda: algorithm.I8.saturating_sub(0, -128), 127),
        (lambda: algorithm.I8.saturating_sub(-2, -128), 126),
        (lambda: algorithm.I64.saturating_add(2**63 - 1, 1), 2**63 - 1),
        (lambda: algorithm.I64.saturating_sub(-(2**63), 1), -(2**63)),
        (lambda: algorithm.U64.saturating_sub(0, 1), 0),
        (lambda: algorithm.U64.saturating_add(2**64 - 1, 2**64 - 1), 2**64 - 1),
        (lambda: algorithm.U8.saturating_mul(17, 17), 255),
        (lambda: algorithm.I8.saturating_mul(17, 17), 127),
        (lambda: algorithm.I8.saturating_mul(-17, 17), -128),
        (lambda: algorithm.I8.saturating_mul(-17, -17), 127),
        (lambda: algorithm.I8.saturating_mul(-128, -1), 127),
        (lambda: algorithm.I8.saturating_mul(-128, 2), -128),
        (lambda: algorithm.I8.saturating_mul(127, -1), -127),
        (lambda: algorithm.U64.saturating_mul(2**32 + 1, 2**32 + 1), 2**64 - 1),
        (lambda: algorithm.I64.saturating_mul(-(2**63), -1), 2**63 - 1),
        (lambda: algorithm.I64.saturating_mul(2**32 + 1, -(2**32 + 1)), -(2**63)),
        (lambda: algorithm.I8.saturating_neg(-128), 127),
        (lambda: algorithm.I8.saturating_neg(127), -127),
        (lambda: algorithm.I8.saturating_neg(-1), 1),
        (lambda: algorithm.I64.saturating_neg(-(2**63)), 2**63 - 1),
    ],
)
def test_a_result_past_either_bound_stands_at_it(call, want):
    assert call() == want


@pytest.mark.parametrize("width", [algorithm.U8, algorithm.U16, algorithm.U32, algorithm.U64])
def test_an_unsigned_saturating_negation_is_zero_for_every_input(width):
    for a in (0, 1, 2, width.hi - 1, width.hi):
        assert width.saturating_neg(a) == 0


def test_saturation_is_reduced_once_per_operation():
    i8 = algorithm.I8
    assert i8.saturating_add(i8.saturating_add(127, 1), -1) == 126
    assert i8.saturating_add(127, i8.saturating_add(1, -1)) == 127


def test_no_total_operation_raises_where_the_checked_one_does():
    with pytest.raises(algorithm.AlgorithmFailure):
        algorithm.U32.sub(0, 1)
    assert algorithm.U32.wrapping_sub(0, 1) == 2**32 - 1
    assert algorithm.U32.saturating_sub(0, 1) == 0
