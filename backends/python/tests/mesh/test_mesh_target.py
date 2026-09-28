# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""Which ``<send target>`` values name a Mesh peer, on the Python runtime.

The cases live in ``tests/mesh/mesh_target_cases.json``, the table the C++
core's ``SendHelper::isMeshTarget``, the build and every runtime read.
"""

from __future__ import annotations

import json
from pathlib import Path

from sce_runtime import is_mesh_target, mesh_peer

_REPO_ROOT = Path(__file__).resolve().parents[4]
_TABLE = _REPO_ROOT / "tests" / "mesh" / "mesh_target_cases.json"


def test_a_mesh_peer_is_named_as_the_shared_table_names_it() -> None:
    assert _TABLE.is_file(), f"cannot read the shared table at {_TABLE}"
    cases = json.loads(_TABLE.read_text(encoding="utf-8"))["cases"]
    # A floor: an empty table would pass every assertion below.
    assert len(cases) >= 10, f"the table lost cases: {len(cases)}"
    for case in cases:
        target, peer = case["target"], case["peer"]
        assert mesh_peer(target) == peer, repr(target)
        assert is_mesh_target(target) == (peer is not None), repr(target)
