#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""
Store <-> source lockstep guards for the markdown-carrier ledger workspaces.

Mnemosyne's own validators treat the atomic store as the sole SSOT (R400) and
never read the markdown the sections were generated from — so an edit to the
tracked source document (or a hand-mutated section set) would otherwise drift
silently: the store keeps validating, the document tells a different story.
These tests close that gap deterministically, per workspace:

  * section-set lockstep — converter(source doc) must emit exactly the
    section ids the committed store carries (mesh / wire / synth /
    bytesguard). A heading added, removed, or renumbered in the source fails
    here until the store is regenerated (or the edit is reverted).

The synth workspace is SCE-authored (the RFC is co-located with the forge
implementation it defines), so it has no [workspace.spec_source] and no
vendored-snapshot provenance pin — the section-set lockstep above is the
only guard it needs.

The scxml workspace is deliberately absent from the section-set check: its
sections come from the vendored W3C TOC via scxml_toc_to_manifest + EPUB
projection, already drift-guarded by check_spec_drift.py and
validate-content-drift.

Run:  python3 -m unittest discover -s tools/mnemosyne-adoption/tests
"""

import json
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
TOOL_DIR = HERE.parent
REPO_ROOT = TOOL_DIR.parent.parent
sys.path.insert(0, str(TOOL_DIR))

import bytesguard_rfc_to_manifest  # noqa: E402
import sce_mesh_md_to_manifest  # noqa: E402
import sce_wire_rfc_to_manifest  # noqa: E402
import synth_rfc_to_manifest  # noqa: E402


def store_section_ids(store_path):
    with open(store_path, encoding="utf-8") as fh:
        return set(json.load(fh)["sections"].keys())


def converter_section_ids(convert, md_path, parent_doc):
    manifest = convert(md_path.read_text(encoding="utf-8"), parent_doc)
    return {e["section_id"] for e in manifest}


class SectionSetLockstep(unittest.TestCase):
    """converter(source) == committed store, per markdown-carrier workspace."""

    def assert_lockstep(self, ws, md, convert, parent_doc):
        store = REPO_ROOT / ws / ".atomic" / "workspace.atomic.json"
        md_path = REPO_ROOT / md
        self.assertTrue(store.exists(), f"missing store {store}")
        self.assertTrue(md_path.exists(), f"missing source doc {md_path}")
        from_doc = converter_section_ids(convert, md_path, parent_doc)
        from_store = store_section_ids(store)
        only_doc = sorted(from_doc - from_store)
        only_store = sorted(from_store - from_doc)
        self.assertEqual(
            (only_doc, only_store),
            ([], []),
            f"{ws}: source doc and committed store disagree.\n"
            f"  in doc, not in store (regenerate via import-sections): {only_doc}\n"
            f"  in store, not in doc (heading removed/renumbered?):    {only_store}",
        )

    def test_mesh(self):
        self.assert_lockstep(
            "docs/sce-ledger/mesh", "SCE_MESH.md", sce_mesh_md_to_manifest.convert, "mesh"
        )

    def test_wire(self):
        self.assert_lockstep(
            "docs/sce-ledger/wire",
            "docs/sce-ledger/wire/rfc-sce-diagnostic-wire-unification.md",
            sce_wire_rfc_to_manifest.convert,
            "wire",
        )

    def test_synth(self):
        self.assert_lockstep(
            "docs/spec/synth",
            "docs/spec/synth/rfc-sce-protocol-synthesis.md",
            synth_rfc_to_manifest.convert,
            "synth",
        )

    def test_bytesguard(self):
        self.assert_lockstep(
            "docs/sce-ledger/bytesguard",
            "docs/sce-ledger/bytesguard/rfc-eventschema-bytes-guard.md",
            bytesguard_rfc_to_manifest.convert,
            "bytesguard",
        )


if __name__ == "__main__":
    unittest.main()
