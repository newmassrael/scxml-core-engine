# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The members of a JSON object written into `_event.data` come in one order
on every engine (ARCHITECTURE.md, "JSON Object Key Order").

The cases live in `tests/json_text/object_key_order.json`, the table every
engine's writer is measured against. This backend has no parameter builder
yet — it writes a payload through `ScriptValue.to_json_literal` — so each case
is collected into the object a `<send>` would carry, by the rule the table
states, and that writer is held to the table here.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Dict, List

import pytest

from sce_runtime.scripting.i_script_engine import ScriptValue

# The repository root, from this file's own location: the shared table is
# named by the same path every other reader uses.
_REPO_ROOT = Path(__file__).resolve().parents[4]
_TABLE = _REPO_ROOT / "tests" / "json_text" / "object_key_order.json"


def _load() -> List[Dict[str, Any]]:
    assert _TABLE.is_file(), f"cannot read the shared table at {_TABLE}"
    cases = json.loads(_TABLE.read_text(encoding="utf-8"))["cases"]
    # A floor, not an equality: adding a case must not have to touch this
    # number, but a table that stopped being read must not pass either.
    assert len(cases) >= 8, f"the shared key-order table produced only {len(cases)} case(s)"
    return cases


def _evaluated(params: List[List[Any]]) -> Dict[str, Any]:
    """The object a `<send>` carries for these params: one occurrence of a
    name is the value itself, more than one an array of them in declaration
    order (W3C SCXML test178 — an object cannot hold a name twice)."""
    seen: Dict[str, List[Any]] = {}
    for name, value in params:
        seen.setdefault(name, []).append(value)
    return {name: values[0] if len(values) == 1 else values for name, values in seen.items()}


@pytest.mark.parametrize("case", _load(), ids=lambda case: case["name"])
def test_the_members_of_an_object_are_written_in_the_one_order_every_engine_writes(
    case: Dict[str, Any],
) -> None:
    written = ScriptValue.of(_evaluated(case["params"])).to_json_literal()
    assert written == case["data"]
