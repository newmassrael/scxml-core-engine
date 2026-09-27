# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""A string written into JSON text is written one way on every engine
(ARCHITECTURE.md, "JSON Text (Single Source of Truth)").

The cases live in `tests/json_text/string_escape.json`, the table every
engine's writer is measured against. This backend writes a string two ways —
its own escaper, and `json.dumps` where a whole payload is serialised — and
both are held to the table here.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Dict, List

import pytest

from sce_runtime.scripting.i_script_engine import _json_string

# The repository root, from this file's own location: the shared table is
# named by the same path every other reader uses.
_REPO_ROOT = Path(__file__).resolve().parents[4]
_TABLE = _REPO_ROOT / "tests" / "json_text" / "string_escape.json"


def _load() -> List[Dict[str, Any]]:
    assert _TABLE.is_file(), f"cannot read the shared table at {_TABLE}"
    cases = json.loads(_TABLE.read_text(encoding="utf-8"))["cases"]
    # A floor, not an equality: adding a case must not have to touch this
    # number, but a table that stopped being read must not pass either.
    assert len(cases) >= 8, f"the shared escape table produced only {len(cases)} case(s)"
    return cases


@pytest.mark.parametrize("case", _load(), ids=lambda case: case["name"])
def test_the_escaper_writes_the_one_form(case: Dict[str, Any]) -> None:
    assert _json_string(case["text"]) == f'"{case["escaped"]}"'


@pytest.mark.parametrize("case", _load(), ids=lambda case: case["name"])
def test_a_serialised_payload_writes_its_strings_the_same_way(case: Dict[str, Any]) -> None:
    # The arguments every whole-payload serialisation in this backend passes.
    written = json.dumps({"k": case["text"]}, separators=(",", ":"), ensure_ascii=False)
    assert written == f'{{"k":"{case["escaped"]}"}}'
