# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""The pairs a ``<send>`` or a host-run ``<invoke>`` evaluated, held as the
event's data and rendered as the text a form carries.

A ``<param>`` name may repeat, and every value must be delivered (W3C test178):
one occurrence is the value, and more than one is the values in document
order. A value may itself be a list (a list variable, or a composed value sent
by its name, docs/adr/0086, section 5.2), so the two are told apart by type: the
values of a repeated name are a ``RepeatedParam``, and any other value is one
value, whatever it holds. The Kotlin runtime's ``RepeatedParam`` is the same
rule (``StateMachineEngine.putParam``).

The rendering is a form's (W3C SCXML C.2): a repeated name is one pair per
value, and a value is the text ``ScriptValue.to_wire_string`` gives it, so a
list or a record crosses as the one JSON text the event carries it as, never
as one pair per element.
"""

from __future__ import annotations

from typing import Any, Dict, List

from .scripting.i_script_engine import ScriptValue


class RepeatedParam(list):
    """The values of a ``<param>`` name written more than once, in document
    order. A ``list``, so the event's data writes it as the array W3C test178
    reads, and every reader of the data takes it as one."""


def put_param(marshalled: dict, repeats: dict, name: str, value: Any) -> None:
    """Hold ``value`` under ``name`` in ``marshalled``: the value at its first
    occurrence, and the values in document order from its second, as a
    ``RepeatedParam``. ``repeats`` counts the occurrences so far, so a name a
    namelist filled first is taken over by its first ``<param>``."""
    count = repeats.get(name, 0) + 1
    repeats[name] = count
    if count == 1:
        marshalled[name] = value
    elif count == 2:
        marshalled[name] = RepeatedParam([marshalled[name], value])
    else:
        marshalled[name].append(value)


def wire_params_from_payload(marshalled: Any) -> Dict[str, List[str]]:
    """``marshalled`` as W3C SCXML C.2 form parameters: one text per value of
    a repeated name, in document order, and one text for any other value. A
    payload that is not a name/value mapping (a ``<content>`` body, or the
    empty string a ``<send>`` with no arguments has) carries none."""
    if not isinstance(marshalled, dict):
        return {}
    result: Dict[str, List[str]] = {}
    for name, value in marshalled.items():
        values = list(value) if isinstance(value, RepeatedParam) else [value]
        result[name] = [ScriptValue.of(item).to_wire_string() for item in values]
    return result
