# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""A union in a saved state, in the Python runtime (docs/adr/0092, section 6.1
item 7): one object, its tag beside the fields of its case, read back by its
tag first; and a list of a type that states a lower count, held to it when it is
read back (docs/adr/0086, section 15). Cross-language sibling:
``backends/go/runtime/saved_union_test.go``."""

from __future__ import annotations

import pytest

from sce_runtime.saved_json import parse_json, write_json
from sce_runtime.saved_state import StateRefusal
from sce_runtime.saved_values import (
    saved_read_list_within,
    saved_read_uint8,
    saved_read_union,
    saved_read_union_tag,
    saved_write_uint8,
    saved_write_union,
)


def test_a_union_is_saved_with_its_tag_before_the_fields_of_its_case() -> None:
    circle = saved_write_union("kind", "circle", {"radius": saved_write_uint8(7)})
    assert write_json(circle) == '{"kind":"circle","radius":7}'
    assert write_json(saved_write_union("kind", "dot", None)) == '{"kind":"dot"}'


def test_a_saved_union_is_read_back_by_its_tag_first() -> None:
    saved = parse_json('{"radius":7,"kind":"circle"}')
    assert saved_read_union_tag(saved, "s", "kind") == "circle"
    cases = {
        "dot": lambda: ("dot", None),
        "circle": lambda: ("circle", saved_read_uint8(saved["radius"], "s.radius")),
    }
    assert saved_read_union(saved, "s", "kind", "Shape", cases) == ("circle", 7)
    assert saved_read_union(parse_json('{"kind":"dot"}'), "s", "kind", "Shape", cases) == ("dot", None)


@pytest.mark.parametrize("text, want", [
    ('[1]', "'s' is not an object"),
    ('{"radius":7}', "'s' has no field 'kind'"),
    ('{"kind":3}', "'s.kind' is not a text"),
    ('{"kind":null}', "'s.kind' is absent (null), and its type is not optional"),
    ('{"kind":"oval"}', "'s' (oval) is not a case of Shape"),
])
def test_a_saved_union_whose_tag_names_no_case_is_refused(text: str, want: str) -> None:
    cases = {"dot": lambda: "dot"}
    with pytest.raises(StateRefusal) as refused:
        saved_read_union(parse_json(text), "s", "kind", "Shape", cases)
    assert str(refused.value) == want


def test_a_list_is_read_back_only_within_the_counts_its_type_states() -> None:
    assert saved_read_list_within(parse_json("[1,2]"), "l", 1, 3, saved_read_uint8) == [1, 2]
    with pytest.raises(StateRefusal) as short:
        saved_read_list_within(parse_json("[]"), "l", 1, 3, saved_read_uint8)
    assert str(short.value) == "'l' holds 0 elements, fewer than the 1 its type declares"
    with pytest.raises(StateRefusal) as past:
        saved_read_list_within(parse_json("[1,2,3,4]"), "l", 1, 3, saved_read_uint8)
    assert str(past.value) == "'l' holds 4 elements, past the 3 it is bounded by"
