# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The value a hybrid ``<invoke srcexpr>`` computes is reduced to the stem of the
document it names (docs/SCE_ACCEPTED_SUBSET.md §2.13), and every engine reduces
it the same way.

The cases live in ``tests/document_stem/document_stem.json``, the table every
engine's reader, and the build's, is measured against.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Dict, List

import pytest

from sce_runtime import document_stem

# The repository root, from this file's own location: the shared table is
# named by the same path every other reader uses.
_REPO_ROOT = Path(__file__).resolve().parents[4]
_TABLE = _REPO_ROOT / "tests" / "document_stem" / "document_stem.json"


def _load() -> List[Dict[str, Any]]:
    assert _TABLE.is_file(), f"cannot read the shared table at {_TABLE}"
    cases = json.loads(_TABLE.read_text(encoding="utf-8"))["cases"]
    # A floor, not an equality: adding a case must not have to touch this
    # number, but a table that stopped being read must not pass either.
    assert len(cases) >= 15, f"the shared document-stem table produced only {len(cases)} case(s)"
    return cases


@pytest.mark.parametrize("case", _load(), ids=lambda case: case["name"])
def test_a_value_is_reduced_to_the_one_stem_every_engine_reduces_it_to(
    case: Dict[str, Any],
) -> None:
    assert document_stem(case["value"]) == case["stem"]
