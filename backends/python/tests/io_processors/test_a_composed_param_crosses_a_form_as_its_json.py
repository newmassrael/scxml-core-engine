# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""A composed value a `<param>` carries crosses a form as its JSON text.

A statechart sends a list or a record by the name of the place that holds it
(docs/adr/0086, section 5.2). Over the BasicHTTP Event I/O Processor, and to a
host-run `<invoke>`, each pair is the text a form carries (W3C SCXML C.2), so
the value's text is the one JSON text the event carries it as. The renderer
used to unwrap every list as if it were a repeated name, so `late = [5, 6]`
crossed as the two pairs `late=5&late=6`, which no receiver reads back as the
list. A name written twice (W3C test178) is told from a value that is a list by
its type, `RepeatedParam`, as the Kotlin runtime tells them apart; the C++, Go
and Rust runtimes keep the two in separate shapes and write a list or an object
as its JSON text.
"""

from __future__ import annotations

from sce_runtime.scripting.i_script_engine import ScriptValue
from sce_runtime.send_params import RepeatedParam, put_param, wire_params_from_payload


def _payload(*pairs):
    """The payload a send holds `pairs` in, each `(name, value)` in document
    order, as the generated send puts them."""
    marshalled: dict = {}
    repeats: dict = {}
    for name, value in pairs:
        put_param(marshalled, repeats, name, value)
    return marshalled


def test_a_list_param_is_one_pair_its_json_text() -> None:
    marshalled = _payload(("late", [5, 6]))
    assert not isinstance(marshalled["late"], RepeatedParam), "one occurrence is the value"
    assert wire_params_from_payload(marshalled) == {"late": ["[5,6]"]}


def test_a_list_of_records_is_one_pair_its_json_text_at_every_depth() -> None:
    items = [
        {"tags": ["a", 'b"c'], "id": 1, "due": None, "ratio": 1.5},
        {"tags": [], "id": 2, "due": None, "ratio": 0.1},
    ]
    marshalled = _payload(("items", items), ("note", "echo"))
    assert wire_params_from_payload(marshalled) == {
        "items": [
            '[{"due":null,"id":1,"ratio":1.5,"tags":["a","b\\"c"]},'
            '{"due":null,"id":2,"ratio":0.1,"tags":[]}]'
        ],
        "note": ["echo"],
    }


def test_a_params_text_is_the_json_the_event_carries_it_as() -> None:
    value = {"b": [1, None], "a": {"y": True, "x": "t"}}
    marshalled = _payload(("spare", value))
    text = wire_params_from_payload(marshalled)["spare"]
    assert text == [ScriptValue.of(value).to_json_literal()]
    assert ScriptValue.of(marshalled).to_json_literal() == '{"spare":' + text[0] + "}"


def test_a_repeated_name_is_one_pair_per_value_each_value_its_own_text() -> None:
    marshalled = _payload(("n", 1), ("n", [2, 3]), ("n", "four"))
    assert isinstance(marshalled["n"], RepeatedParam)
    assert wire_params_from_payload(marshalled) == {"n": ["1", "[2,3]", "four"]}
    # The event carries the repeated name as the array of its values.
    assert ScriptValue.of(marshalled).to_json_literal() == '{"n":[1,[2,3],"four"]}'


def test_a_payload_that_is_no_mapping_carries_no_pairs() -> None:
    assert wire_params_from_payload("") == {}
    assert wire_params_from_payload("<books/>") == {}
