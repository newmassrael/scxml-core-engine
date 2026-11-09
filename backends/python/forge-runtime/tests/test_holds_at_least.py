# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""A list a ``sce-static`` machine filled by appends is sent into a field that
states a lower count only once it holds that count (docs/adr/0086, section
15.6): a variable states none, so the send holds it where the list is written,
and a short list fails the param as a precondition, with nothing written."""

import pytest

from sce_forge_runtime import algorithm


def test_a_list_that_holds_the_count_is_written():
    assert algorithm.holds_at_least(1, 1, lambda: ["one"]) == ["one"]
    assert algorithm.holds_at_least(3, 2, lambda: "written") == "written"
    assert algorithm.holds_at_least(0, 0, lambda: []) == []


def test_a_short_list_fails_as_a_precondition_and_writes_nothing():
    written = []
    with pytest.raises(algorithm.AlgorithmFailure) as failure:
        algorithm.holds_at_least(0, 1, lambda: written.append("written"))
    assert failure.value.error is algorithm.AlgorithmError.PRECONDITION
    assert written == [], "the list was written though it was short"
