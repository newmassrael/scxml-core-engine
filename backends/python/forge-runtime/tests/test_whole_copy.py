# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""A composed value a ``sce-static`` machine takes whole into a place is a copy
that shares no list with the value it was taken from (docs/adr/0086, section
6.1): a list is written into in place, by an append or a clear, so a list
variable that held the payload's own list would write into the payload, which
may be a host's. Every list at every depth is a new one; a value nothing writes
into comes back equal."""

import dataclasses
import enum

from sce_forge_runtime import algorithm


class _Level(enum.IntEnum):
    LOW = 0
    HIGH = 1


@dataclasses.dataclass(frozen=True)
class _Item:
    id: int
    tags: list


def test_a_list_taken_whole_is_a_new_list_of_equal_elements():
    held = [5, 6]
    taken = algorithm.whole_copy(held)
    assert taken == [5, 6]
    taken.append(7)
    assert held == [5, 6], "an append to the copy reaches the list it was taken from"


def test_every_list_at_every_depth_is_a_new_one():
    held = [[1, 2], [3]]
    taken = algorithm.whole_copy(held)
    assert taken == held
    taken[0].clear()
    assert held == [[1, 2], [3]], "a clear of an inner list reaches the value it was taken from"
    items = [_Item(1, ["a"]), _Item(2, [])]
    copied = algorithm.whole_copy(items)
    assert copied == items
    copied[0].tags.append("b")
    assert items[0].tags == ["a"], "a list a record holds is shared with the copy"


def test_a_value_nothing_writes_into_comes_back_equal():
    for held in (7, "text", b"\x00\xff", _Level.HIGH, None, 1.5):
        assert algorithm.whole_copy(held) == held
    # An enum member is the member itself, which a test of identity reads.
    assert algorithm.whole_copy(_Level.LOW) is _Level.LOW
