#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""Carry a markdown-carrier ledger's source headings into its atomic store.

The markdown-carrier workspaces (mesh / wire / synth / bytesguard) derive their
sections from a tracked source document: a converter reads the headings and
emits a manifest, and `mnemosyne-cli import-sections` creates them. Creation is
the only thing that command does — it refuses an existing section with
"already exists with DIVERGENT content", by design, so nothing silently
overwrites a curated store.

That leaves a gap on the *edit* path. When a heading's text changes, the store
keeps the old text and no procedure carries the new one over. Measured
2026-08-11: `mesh-8.3` sat in the store as "Realization Status (2026-04-13)"
while `SCE_MESH.md` had read "Realization Status" for long enough that no round
remembered dropping the date. Every validator was green — the store is the SSOT
they check, and it was internally consistent. It just disagreed with the
document it was generated from.

This is the edit path. For each workspace it converts the source, compares the
fields the converter derives, and applies each difference with the matching
`set-section-<field>` command.

    python3 tools/mnemosyne-adoption/apply_source_headings.py            # report
    python3 tools/mnemosyne-adoption/apply_source_headings.py --apply    # carry

`CARRIED_FIELDS` and `WORKSPACES` live here rather than in the test that guards
them: the checker and the rewriter must read one table, or the gate starts
watching a field this tool will not carry (or the reverse — carrying a field
nothing checks). `test_store_source_lockstep.py` imports both from here.
"""

import argparse
import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO_ROOT = HERE.parent.parent
sys.path.insert(0, str(HERE))

import bytesguard_rfc_to_manifest  # noqa: E402
import sce_mesh_md_to_manifest  # noqa: E402
import sce_wire_rfc_to_manifest  # noqa: E402
import synth_rfc_to_manifest  # noqa: E402

# The fields a converter derives from the source document, mapped to the CLI
# command that carries each one. A field the converter emits is a field the
# source OWNS, so this map is also the definition of what "in lockstep" means.
CARRIED_FIELDS = {
    "title": "set-section-title",
    "parent_doc": "set-section-parent-doc",
    "parent_section": "set-section-parent-section",
}

# (workspace dir, source document, converter, parent_doc argument)
WORKSPACES = [
    (
        "docs/sce-ledger/mesh",
        "SCE_MESH.md",
        sce_mesh_md_to_manifest.convert,
        "mesh",
    ),
    (
        "docs/sce-ledger/wire",
        "docs/sce-ledger/wire/rfc-sce-diagnostic-wire-unification.md",
        sce_wire_rfc_to_manifest.convert,
        "wire",
    ),
    (
        "docs/spec/synth",
        "docs/spec/synth/rfc-sce-protocol-synthesis.md",
        synth_rfc_to_manifest.convert,
        "synth",
    ),
    (
        "docs/sce-ledger/bytesguard",
        "docs/sce-ledger/bytesguard/rfc-eventschema-bytes-guard.md",
        bytesguard_rfc_to_manifest.convert,
        "bytesguard",
    ),
]


def store_sections(ws):
    path = REPO_ROOT / ws / ".atomic" / "workspace.atomic.json"
    with open(path, encoding="utf-8") as fh:
        return json.load(fh)["sections"]


def source_sections(md, convert, parent_doc):
    text = (REPO_ROOT / md).read_text(encoding="utf-8")
    return {e["section_id"]: e for e in convert(text, parent_doc)}


def divergences(ws, md, convert, parent_doc):
    """Carried fields where the source and the store disagree.

    Only sections present in BOTH are compared. A heading added or removed is
    the `import-sections` / `remove-section` path, not this one, and the
    section-set half of the lockstep test already reports it — reporting it
    twice here would suggest this tool can fix it, which it cannot.
    """
    src = source_sections(md, convert, parent_doc)
    store = store_sections(ws)
    out = []
    for sid in sorted(set(src) & set(store)):
        for field in CARRIED_FIELDS:
            want, have = src[sid].get(field), store[sid].get(field)
            if want != have:
                out.append((sid, field, want, have))
    return out


def carry(ws, sid, field, want, cli):
    """Apply one field with its `set-section-<field>` command."""
    cmd = [cli, CARRIED_FIELDS[field], "--section", f"§{sid}"]
    if field == "parent_section":
        # A parent is named as a section reference, and "no parent" is its own
        # flag rather than an empty value — a top-level heading has to be
        # expressible.
        cmd += ["--no-parent"] if want is None else ["--parent", f"§{want}"]
    else:
        cmd += [f"--{field.replace('_', '-')}", want]
    out = subprocess.run(
        cmd, cwd=REPO_ROOT / ws, capture_output=True, text=True
    )
    if out.returncode != 0:
        raise SystemExit(
            f"{ws}: {' '.join(cmd)} failed:\n{out.stderr or out.stdout}"
        )


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--apply", action="store_true", help="carry the differences")
    ap.add_argument(
        "--cli",
        default="mnemosyne-cli",
        help="mnemosyne-cli to invoke (default: PATH). The gate passes its "
        "revision-pinned binary so this tool cannot apply edits with a "
        "different revision than the one that validates them.",
    )
    args = ap.parse_args(argv)

    total = 0
    for ws, md, convert, parent_doc in WORKSPACES:
        diffs = divergences(ws, md, convert, parent_doc)
        total += len(diffs)
        for sid, field, want, have in diffs:
            print(f"  {ws}  {sid}.{field}: store={have!r} -> source={want!r}")
            if args.apply:
                carry(ws, sid, field, want, args.cli)

    if total == 0:
        print("every carried heading field matches its source document.")
        return 0
    if args.apply:
        print(f"carried {total} field(s) into the store(s).")
        return 0
    print(f"\n{total} field(s) diverge. Re-run with --apply to carry them.")
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
