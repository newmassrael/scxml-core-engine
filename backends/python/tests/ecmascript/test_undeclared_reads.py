# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""A datamodel variable may be spelled as a Lua keyword (``local``, ``end``),
which the frontend reads as ``_ENV["name"]``; this engine's ReferenceError
check has to look at that name, not at ``_ENV``.

The cases are ``tests/scripting/undeclared_reads.json``, which the Rust and Go
engines' checks read too.
"""

from __future__ import annotations

import json
from pathlib import Path

from sce_runtime.scripting.i_script_engine import ScriptValue
from sce_runtime.scripting.lua_engine import LuaScriptEngine

_REPO_ROOT = Path(__file__).resolve().parents[4]
_TABLE = _REPO_ROOT / "tests" / "scripting" / "undeclared_reads.json"


def test_a_read_is_refused_exactly_when_the_shared_table_says() -> None:
    assert _TABLE.is_file(), f"cannot read the shared table at {_TABLE}"
    cases = json.loads(_TABLE.read_text(encoding="utf-8"))["cases"]
    # A floor: an empty table would pass every assertion below.
    assert len(cases) >= 10, f"the table lost cases: {len(cases)}"
    for i, case in enumerate(cases):
        engine = LuaScriptEngine()
        session = f"s{i}"
        engine.create_session(session)
        for name in case["declared"]:
            engine.set_variable(session, name, ScriptValue.of(f"#{name}"))
        try:
            engine.evaluate_expression(session, case["expr"])
            refused = False
        except Exception:
            refused = True
        assert refused == case["refused"], f"{case['expr']!r} with {case['declared']}"
