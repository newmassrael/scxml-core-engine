# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The verdicts of the test profile (docs/SCE_TEST_PROFILE.md §5): the four a
document sets with ``<sce:setverdict>`` (P12) and ``error``, which only the
runtime sets (P13), in the order ETSI ES 201 873-1 §24.1 gives them.

Cross-language siblings: ``sce_rust_runtime::test::Verdict`` (Rust),
``SCE::Test::Verdict`` (C++), ``com.sce.runtime.test.Verdict`` (Kotlin),
``testprofile.Verdict`` (Go).
"""

from __future__ import annotations

from enum import IntEnum


class Verdict(IntEnum):
    """A test verdict, declared in the order ``none < pass < inconclusive < fail <
    error``.

    The order is the one ES 201 873-1 §24.1 overwrites by: a verdict only ever
    becomes worse, so the order of the members is the order, and ``NONE`` is
    the verdict of a component that set none. ``str`` of a verdict is its
    spelling in the test log and in TTCN-3 (``inconc`` for ``INCONCLUSIVE``),
    which an enumeration's ``name``, the member's own, cannot be.
    """

    #: ``none``: no verdict was set.
    NONE = 0
    #: ``pass``.
    PASS = 1
    #: ``inconc``.
    INCONCLUSIVE = 2
    #: ``fail``.
    FAIL = 3
    #: ``error``: set only by the runtime (P13).
    ERROR = 4

    def __str__(self) -> str:
        return _SPELLING[self]


_SPELLING = {
    Verdict.NONE: "none",
    Verdict.PASS: "pass",
    Verdict.INCONCLUSIVE: "inconc",
    Verdict.FAIL: "fail",
    Verdict.ERROR: "error",
}


def worse(a: Verdict, b: Verdict) -> Verdict:
    """The worse of ``a`` and ``b``: what ``setverdict`` leaves a local verdict
    at (P12, -4 §9.45), and how the test case verdict folds the local ones
    (P14)."""
    return b if b > a else a
