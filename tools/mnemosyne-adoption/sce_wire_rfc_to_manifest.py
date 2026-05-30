#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""
Convert the Wire RFC's milestone headings into a Mnemosyne bulk-section-create
manifest for the SCE design ledger's `wire` namespace workspace
(docs/sce-ledger/wire).

The Wire RFC (claudedocs/rfc-sce-diagnostic-wire-unification.md) is an SCE design
document whose §3 "Milestone roadmap" defines a commit-series of waves W0..W5
(plus the half-wave W4.5). SCE code comments cite those waves as §W<n>
("RFC §W4", "§W5 D5"); this ledger makes the §wire-W<n> form resolve so the
set_equality_validator can gate the parsing/runtime modules that carry them.

Unlike the mesh converter (numbered `## N.` headings), the wire waves are
`### W<n>` headings whose label starts with a letter. Two wrinkles this handles:
  * each wave has a duplicate "RFC (legacy section header, retained ...)" heading
    kept in the doc for the design record — first occurrence (the contract /
    LANDED heading) wins, the legacy duplicate is dropped;
  * the wave label keeps its dot verbatim (W4.5 -> wire-W4.5): the dot is
    digit-flanked, so the §-citation extractor reads it whole.

This is *adoption tooling*, not part of the SCE engine. It is deterministic and
uses only the Python standard library.

Usage:
    python3 tools/mnemosyne-adoption/sce_wire_rfc_to_manifest.py \\
        [--md claudedocs/rfc-sce-diagnostic-wire-unification.md] \\
        [--parent-doc GENERATED.md] \\
        [--manifest out/wire-manifest.json]
"""

import argparse
import json
import re
import sys
from pathlib import Path

# A milestone-wave heading: "### W0 ...", "### W4.5 ...". Captures (label, title).
WAVE_RE = re.compile(r"^###[ \t]+(W[0-9]+(?:\.[0-9]+)?)\b[ \t]*(.*?)[ \t]*$")

FENCE_RE = re.compile(r"^[ \t]*(`{3,}|~{3,})")


def extract_waves(md_text):
    """Yield (label, title) for each W<n> wave heading outside code fences, in
    document order, keeping the first occurrence of a label (the contract /
    LANDED heading) and dropping the retained "RFC (legacy ...)" duplicate."""
    in_fence = False
    fence_marker = None
    seen = set()
    for line in md_text.splitlines():
        fence_m = FENCE_RE.match(line)
        if fence_m:
            marker = fence_m.group(1)[0]
            if not in_fence:
                in_fence, fence_marker = True, marker
            elif marker == fence_marker:
                in_fence, fence_marker = False, None
            continue
        if in_fence:
            continue
        m = WAVE_RE.match(line)
        if not m:
            continue
        label, title = m.group(1), m.group(2).strip()
        if label in seen:
            continue  # legacy duplicate heading for the design record
        seen.add(label)
        yield label, title


def to_manifest(waves, parent_doc):
    """import-sections manifest. The waves are flat (no parent_section): they are
    top-level milestones of the RFC, and no §wire cite relies on a hierarchy.
    normative_excerpt is omitted (skeleton only) — like the mesh ledger, the wire
    source is an in-repo design doc, so the ledger exists to resolve cites, not
    to render a vendored quote."""
    return [
        {"section_id": f"wire-{label}", "parent_doc": parent_doc, "title": title or label}
        for label, title in waves
    ]


def convert(md_text, parent_doc):
    return to_manifest(extract_waves(md_text), parent_doc)


def main(argv=None):
    here = Path(__file__).resolve().parent
    repo_root = here.parent.parent
    default_md = repo_root / "claudedocs" / "rfc-sce-diagnostic-wire-unification.md"
    ap = argparse.ArgumentParser(description="Wire RFC waves -> Mnemosyne manifest")
    ap.add_argument("--md", default=str(default_md))
    ap.add_argument("--parent-doc", default="GENERATED.md")
    ap.add_argument("--manifest", default=None, help="manifest JSON output path")
    args = ap.parse_args(argv)

    manifest = convert(Path(args.md).read_text(encoding="utf-8"), args.parent_doc)

    # Self-check: every emitted id must be citation-safe (a dot only when it is
    # flanked by digits, the condition the §-citation extractor keeps it under).
    for entry in manifest:
        leaf = entry["section_id"][len("wire-") :]
        for i, ch in enumerate(leaf):
            if ch == "." and not (
                i > 0 and leaf[i - 1].isdigit() and i + 1 < len(leaf) and leaf[i + 1].isdigit()
            ):
                sys.stderr.write(f"error: non-citation-safe id {entry['section_id']!r}\n")
                return 1

    manifest_json = json.dumps(manifest, indent=2, ensure_ascii=False) + "\n"
    if args.manifest:
        Path(args.manifest).write_text(manifest_json, encoding="utf-8")
    else:
        sys.stdout.write(manifest_json)
    sys.stderr.write("waves=%d\n" % len(manifest))
    return 0


if __name__ == "__main__":
    sys.exit(main())
