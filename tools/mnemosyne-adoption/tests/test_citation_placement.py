#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""
A §-token has to sit where the binder can read it.

The ledger enforces two different things about a `§scxml-` citation. EXISTENCE
— the id names a real section — is checked over every tracked file. BINDING —
the citation is tied to the symbol the ledger records as implementing that
section — is checked only inside `[code_refs] paths`, and only through the
comment syntax the validator knows for that file's extension.

For Python that syntax is `#` and nothing else: `comment_only` masks string
literals, and a docstring is a string. So a §-token written in a docstring is
read by nobody on the binding side. It is not rejected, it is not counted, and
`citation_unbound` stays silent — the token looks like a machine-checked claim
and is prose with a sigil on it.

The repository already has the rule this test enforces, written for C++ and
Rust after the same thing was measured there twice: a citation in a doc comment
resolves to no symbol (`binding_unbacked ... <no-cite>`) and the fix is to move
it into a line comment inside the body that implements the clause. Python's
docstring is that same position, so it takes the same rule.

Measured 2026-08-12, when the existence sweep first learned to read docstrings:
16 §-tokens lived in Python docstrings, and 10 of them named a (file, section)
pair no `#` comment in that file cited — ten claims the binding axis had never
seen, including six W3C Appendix D algorithm functions in `engine.py` and the
Event I/O Processor clause in `io_processors.py`.

Run:  python3 -m unittest discover -s tools/mnemosyne-adoption/tests
"""

import os
import re
import subprocess
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
TOOL_DIR = os.path.dirname(HERE)
REPO_ROOT = os.path.abspath(os.path.join(TOOL_DIR, "..", ".."))
sys.path.insert(0, TOOL_DIR)

from migrate_citations import (  # noqa: E402
    MIGRATED_TOKEN_RE,
    docstring_spans,
    hash_comment_mask,
)

CONFIG = os.path.join(REPO_ROOT, "docs", "spec", "scxml", "mnemosyne.toml")


def enrolled_prefixes():
    """`[code_refs] paths` from the scxml ledger config.

    Read rather than restated: a test that keeps its own copy of the scan set
    stops describing the gate the moment either one moves.
    """
    with open(CONFIG, encoding="utf-8") as fh:
        toml = fh.read()
    block = toml.split("\npaths = [", 1)[1].split("\n]", 1)[0]
    out = []
    for line in block.splitlines():
        line = line.split("#", 1)[0].strip()
        if line.startswith('"'):
            out.append(line.split('"')[1])
    assert len(out) > 10, f"parsed {len(out)} scan path(s); the parse broke, not the config"
    return out


def enrolled_python_files():
    listing = subprocess.run(
        ["git", "-C", REPO_ROOT, "ls-files"], capture_output=True, text=True, check=True
    ).stdout.splitlines()
    prefixes = enrolled_prefixes()
    return [
        rel
        for rel in listing
        if rel.endswith(".py")
        and any(rel == p or rel.startswith(p.rstrip("/") + "/") for p in prefixes)
    ]


def citations_by_position(rel):
    """(docstring_ids, hash_comment_ids) for one enrolled Python file."""
    with open(os.path.join(REPO_ROOT, rel), encoding="utf-8") as fh:
        text = fh.read()
    if "§" not in text:
        return set(), set()
    spans = docstring_spans(text) or []
    in_hash = hash_comment_mask(text)
    docstring, commented = set(), set()
    for m in MIGRATED_TOKEN_RE.finditer(text):
        i = m.start()
        sid = f"{m.group('ns')}-{m.group('leaf')}"
        if in_hash[i]:
            commented.add(sid)
        elif any(start <= i < end for start, end in spans):
            docstring.add(sid)
    return docstring, commented


class DocstringCitationsCannotBind(unittest.TestCase):
    """No enrolled Python file may claim a section only from a docstring."""

    def test_every_enrolled_python_citation_is_where_the_binder_reads(self):
        orphans = []
        scanned, citing = 0, 0
        for rel in enrolled_python_files():
            scanned += 1
            docstring, commented = citations_by_position(rel)
            if docstring or commented:
                citing += 1
            for sid in sorted(docstring - commented):
                orphans.append(f"{rel}  §{sid}")

        # A pass over a scan set that quietly emptied reads exactly like a pass
        # over a clean one. Measured 2026-08-12: the scxml ledger enrolls the
        # Python runtime's 11 source files, 4 of which cite the spec. The
        # floors sit below those so a module can be split or a citation
        # retired, and above zero so an empty sweep cannot report success.
        self.assertGreaterEqual(
            scanned, 8, f"only {scanned} enrolled .py file(s) found; the scan broke"
        )
        self.assertGreaterEqual(
            citing, 3, f"only {citing} enrolled .py file(s) cite the spec; the scan broke"
        )

        self.assertEqual(
            orphans,
            [],
            "§-token(s) cited only from a Python docstring, where the binding "
            "axis cannot read them — the claim looks machine-checked and is "
            "not:\n  "
            + "\n  ".join(orphans)
            + "\nMove the token into a `#` comment inside the body that "
            "implements the clause and bind it, exactly as the C++ and Rust "
            "doc-comment cases were resolved; leave the docstring saying what "
            "it says in prose.",
        )


class TheGapThisClosesIsReal(unittest.TestCase):
    """The rule above is worth enforcing only if the binder really is blind.

    Asserted against this repository's own Python comment mask, which is kept
    in lockstep with the validator's Hash syntax by `mask_for`'s dispatch — so
    if upstream ever teaches `comment_only` about docstrings, the lockstep
    comment is what has to move, and this states what it would be moving.
    """

    def test_the_python_comment_mask_does_not_cover_a_docstring(self):
        text = '"""§scxml-C-1 is claimed here."""\n# §scxml-C-2 is claimed here\n'
        mask = hash_comment_mask(text)
        positions = [m.start() for m in re.finditer("§", text)]
        self.assertEqual(len(positions), 2, text)
        self.assertFalse(mask[positions[0]], "docstring token unexpectedly masked in")
        self.assertTrue(mask[positions[1]], "`#` comment token unexpectedly masked out")


if __name__ == "__main__":
    unittest.main()
