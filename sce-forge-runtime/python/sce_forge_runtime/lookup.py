# SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

"""Exact-match lookup tables. See SCE_FORGE.md Section 4.9.

A lookup is a finite discrete mapping — unlike interpolation there is no
continuity assumption. `lookup(keys, values, needle)` returns the matching
value iff `needle` equals one of the keys exactly; otherwise `None`.
"""

from typing import Optional, Sequence, TypeVar

K = TypeVar("K")
V = TypeVar("V")


def lookup(keys: Sequence[K], values: Sequence[V], needle: K) -> Optional[V]:
    """Exact-match lookup over parallel key/value sequences.

    Returns `values[i]` at the first index where `keys[i] == needle`, or
    `None` on miss. Keys must be unique (validated at parse time by the
    Forge compiler).
    """
    for i, k in enumerate(keys):
        if k == needle:
            return values[i]
    return None
