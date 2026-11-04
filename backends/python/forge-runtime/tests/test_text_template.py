# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The fence of an untrusted hole (docs/adr/0087, section 3.1): every marker is
removed from the value, a marker the removal rebuilds too, and the cases are the
ones every runtime's removal is held to (the Rust runtime's text_template
tests)."""

import pytest

from sce_forge_runtime.text_template import remove_markers


@pytest.mark.parametrize(
    "value, kept",
    [
        ("plain words", "plain words"),
        ("a<<<TEXTb", "ab"),
        ("aTEXT>>>b<<<TEXTc", "abc"),
        ("<<<TE<<<TEXTXT", ""),
        ("TEXT>>TEXT>>>>", ""),
        ("é<<<TEXTü", "éü"),
        ("", ""),
    ],
)
def test_every_marker_is_removed_until_none_is_left(value, kept):
    assert remove_markers(value, "<<<TEXT", "TEXT>>>") == kept


def test_the_longer_marker_is_removed_where_both_start():
    assert remove_markers("abcd", "ab", "abc") == "d"


def test_an_empty_marker_is_no_marker():
    assert remove_markers("x>>y", "", ">>") == "xy"
    assert remove_markers("abc", "", "") == "abc"
