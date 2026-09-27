# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""A ``<send>`` delay is read as one CSS2 time on every engine
(ARCHITECTURE.md, "Durations (Single Source of Truth)").

The cases live in ``tests/durations/css2_time.json``, the table every engine's
delay reader is measured against.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Dict, List

import pytest

from sce_runtime import parse_delay_ms

# The repository root, from this file's own location: the shared table is
# named by the same path every other reader uses.
_REPO_ROOT = Path(__file__).resolve().parents[4]
_TABLE = _REPO_ROOT / "tests" / "durations" / "css2_time.json"


def _load() -> List[Dict[str, Any]]:
    assert _TABLE.is_file(), f"cannot read the shared table at {_TABLE}"
    cases = json.loads(_TABLE.read_text(encoding="utf-8"))["cases"]
    # A floor, not an equality: adding a case must not have to touch this
    # number, but a table that stopped being read must not pass either.
    assert len(cases) >= 20, f"the shared duration table produced only {len(cases)} case(s)"
    return cases


@pytest.mark.parametrize("case", _load(), ids=lambda case: case["name"])
def test_a_delay_is_read_as_the_one_css2_time(case: Dict[str, Any]) -> None:
    assert parse_delay_ms(case["text"]) == case["ms"]
