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

# The table lives in the tool that CARRIES these fields, not here. A checker
# and a rewriter that keep separate rules drift into the gap between them: this
# test would start watching a field `apply_source_headings.py` will not carry
# (an author told to run a tool that cannot fix the failure), or the tool would
# carry one nothing checks. One table, both directions.
#
# Every field the converter derives from the source document is in it, because
# a field the converter emits is a field the source OWNS. Measured: with only
# `section_id` compared, `mesh-8.3` sat in the store as "Realization Status
# (2026-04-13)" while the heading had read "Realization Status" for long enough
# that no round remembered dropping the date — the ledger and the document told
# different stories and every check was green.
import apply_source_headings  # noqa: E402

CARRIED_FIELDS = tuple(apply_source_headings.CARRIED_FIELDS)


def store_sections(store_path):
    with open(store_path, encoding="utf-8") as fh:
        return json.load(fh)["sections"]


def converter_sections(convert, md_path, parent_doc):
    manifest = convert(md_path.read_text(encoding="utf-8"), parent_doc)
    return {e["section_id"]: e for e in manifest}


class SectionSetLockstep(unittest.TestCase):
    """converter(source) == committed store, per markdown-carrier workspace."""

    def assert_lockstep(self, ws, md, convert, parent_doc):
        store = REPO_ROOT / ws / ".atomic" / "workspace.atomic.json"
        md_path = REPO_ROOT / md
        self.assertTrue(store.exists(), f"missing store {store}")
        self.assertTrue(md_path.exists(), f"missing source doc {md_path}")
        from_doc = converter_sections(convert, md_path, parent_doc)
        from_store = store_sections(store)
        only_doc = sorted(set(from_doc) - set(from_store))
        only_store = sorted(set(from_store) - set(from_doc))
        self.assertEqual(
            (only_doc, only_store),
            ([], []),
            f"{ws}: source doc and committed store disagree.\n"
            f"  in doc, not in store (regenerate via import-sections): {only_doc}\n"
            f"  in store, not in doc (heading removed/renumbered?):    {only_store}",
        )
        # The id set agreeing only says the same headings exist. What each
        # heading SAYS is carried too, and `import-sections` refuses to
        # overwrite an existing section, so a source edit that changes a
        # carried field is applied with the matching `set-section-<field>`
        # command — which nothing demanded until this compared them.
        drift = [
            f"{sid}.{field}: doc={from_doc[sid].get(field)!r} "
            f"store={from_store[sid].get(field)!r}"
            for sid in sorted(set(from_doc) & set(from_store))
            for field in CARRIED_FIELDS
            if from_doc[sid].get(field) != from_store[sid].get(field)
        ]
        self.assertEqual(
            drift,
            [],
            f"{ws}: the store carries a value the source document no longer "
            f"says. Carry them over with:\n"
            f"  python3 tools/mnemosyne-adoption/apply_source_headings.py "
            f"--apply --cli <pinned mnemosyne-cli>\n  " + "\n  ".join(drift),
        )

    def test_every_markdown_carrier_workspace(self):
        # Enumerated from the tool's table, not restated here. A fifth
        # markdown-carrier workspace is then in lockstep the moment it is
        # carryable, instead of the moment someone remembers to add a fifth
        # test method — which is the failure mode this whole round is about.
        # `subTest` so one drifting workspace does not hide the others.
        self.assertGreaterEqual(
            len(apply_source_headings.WORKSPACES),
            4,
            "the workspace table shrank; a lockstep check over nothing passes",
        )
        for ws, md, convert, parent_doc in apply_source_headings.WORKSPACES:
            with self.subTest(workspace=ws):
                self.assert_lockstep(ws, md, convert, parent_doc)


if __name__ == "__main__":
    unittest.main()
