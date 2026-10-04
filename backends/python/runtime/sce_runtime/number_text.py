# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""A 64-bit float as ECMAScript's ``Number::toString`` spells it, radix 10.

The one spelling SCE writes on every engine (ARCHITECTURE.md, "JSON Number
Text (Single Source of Truth)"): the fewest digits that read back as the same
double, in decimal notation when ``1e-6 <= |x| < 1e21`` and as ``d[.ddd]e[+-]n``
otherwise, no fraction on a whole value and ``0`` for either zero.
``tests/json_text/real_text.json`` holds the cases.

Python's own ``repr`` has the right digits and the wrong layout: it keeps a
``.0`` on a whole value, pads a negative exponent to two digits (``1e-07``) and
leaves decimal notation at ``1e16``. So the digits are taken from ``repr`` and
laid out here.
"""

from __future__ import annotations

from decimal import Decimal


def number_text(value: float) -> str:
    """``value`` as ``String(value)`` spells it in ECMAScript.

    A value that is not finite is spelled ``NaN``, ``Infinity`` or
    ``-Infinity``; a JSON writer, which has no spelling for those, tests for
    them before it calls this.
    """
    if value != value:
        return "NaN"
    if value == float("inf"):
        return "Infinity"
    if value == float("-inf"):
        return "-Infinity"
    if value == 0:
        return "0"
    # `repr` is the shortest round-trip digits; `normalize` drops the zeros
    # a whole value's `.0` leaves, so the digits are exactly the significant
    # ones and `exponent` says where they stand.
    _, digit_tuple, exponent = Decimal(repr(abs(value))).normalize().as_tuple()
    digits = "".join(str(d) for d in digit_tuple)
    k = len(digits)
    # The value is `0.<digits> * 10^point`; the specification's `k` and `n`.
    point = k + int(exponent)
    sign = "-" if value < 0 else ""
    if k <= point <= 21:
        return sign + digits + "0" * (point - k)
    if 0 < point <= 21:
        return sign + digits[:point] + "." + digits[point:]
    if -6 < point <= 0:
        return sign + "0." + "0" * (-point) + digits
    power = point - 1
    mantissa = digits[0] + ("." + digits[1:] if k > 1 else "")
    return f"{sign}{mantissa}e{'-' if power < 0 else '+'}{abs(power)}"
