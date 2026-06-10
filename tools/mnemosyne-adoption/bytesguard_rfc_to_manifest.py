#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""
Convert the EventSchema bytes-guard RFC's headings into a Mnemosyne
bulk-section-create manifest for the `bytesguard` namespace workspace
(docs/sce-ledger/bytesguard).

The RFC (docs/sce-ledger/bytesguard/rfc-eventschema-bytes-guard.md) is an SCE
design document: the cross-backend program that made `bytes`-typed
`_event.data` transition guards lower natively on all six backends. SCE code
comments cite it by filename ("rfc-eventschema-bytes-guard.md §3 B3") and by
its item families ("RFC §3 B2/B5"); this ledger makes the §bytesguard-<n>
form resolve so the set_equality_validator can gate the enrolled modules.

Heading shape is uniform `## §<n> Title` / `### §<n>.<m> Title` (the sigil is
mandatory in this document, unlike the synth RFC), labels are purely numeric,
and dots are digit-flanked — so ids are citation-safe verbatim. Item ids
inside sections (B1..B7, commit rows) are list entries, not headings; cites
keep them as prose after the section token ("§bytesguard-3 B5").

This is *adoption tooling*, not part of the SCE engine. It is deterministic
and uses only the Python standard library.

Usage:
    python3 tools/mnemosyne-adoption/bytesguard_rfc_to_manifest.py \\
        [--md docs/sce-ledger/bytesguard/rfc-eventschema-bytes-guard.md] \\
        [--parent-doc bytesguard] \\
        [--manifest out/bytesguard-manifest.json]
"""

import argparse
import json
import re
import sys
from pathlib import Path

HEADING_RE = re.compile(
    r"^(#{2,3})[ \t]+§(?P<label>[0-9]+(?:\.[0-9]+)*)[ \t]+(?P<title>.*?)[ \t]*$"
)

FENCE_RE = re.compile(r"^[ \t]*(`{3,}|~{3,})")


def parent_label_of(label):
    """Drop the last dotted component: 1.3 -> 1 ; 3.1 -> 3 ; 7 -> None."""
    if "." not in label:
        return None
    return label.rsplit(".", 1)[0]


def extract_sections(md_text):
    """Yield (label, title) for each §-numbered h2/h3 heading outside code
    fences, in document order."""
    in_fence = False
    fence_marker = None
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
        m = HEADING_RE.match(line)
        if m:
            yield m.group("label"), m.group("title").strip()


def to_manifest(sections, parent_doc):
    """import-sections manifest with the document's own hierarchy. Skeleton
    only — the RFC is tracked in-repo, so the ledger exists to resolve cites,
    not to render a vendored quote."""
    entries = []
    for label, title in sections:
        entry = {
            "section_id": f"bytesguard-{label}",
            "parent_doc": parent_doc,
            "title": title or label,
        }
        parent = parent_label_of(label)
        if parent is not None:
            entry["parent_section"] = f"bytesguard-{parent}"
        entries.append(entry)
    return entries


def convert(md_text, parent_doc):
    return to_manifest(extract_sections(md_text), parent_doc)


def self_check(manifest):
    """Emitted-id invariants: unique, citation-safe (numeric labels keep
    digit-flanked dots only by construction), every parent present."""
    ids = [e["section_id"] for e in manifest]
    if len(ids) != len(set(ids)):
        dupes = sorted({i for i in ids if ids.count(i) > 1})
        return f"duplicate section ids: {dupes}"
    id_set = set(ids)
    for entry in manifest:
        parent = entry.get("parent_section")
        if parent is not None and parent not in id_set:
            return f"parent {parent!r} of {entry['section_id']!r} not emitted"
    return None


def main(argv=None):
    here = Path(__file__).resolve().parent
    repo_root = here.parent.parent
    default_md = (
        repo_root / "docs" / "sce-ledger" / "bytesguard" / "rfc-eventschema-bytes-guard.md"
    )
    ap = argparse.ArgumentParser(description="bytes-guard RFC -> Mnemosyne manifest")
    ap.add_argument("--md", default=str(default_md))
    ap.add_argument("--parent-doc", default="bytesguard")
    ap.add_argument("--manifest", default=None, help="manifest JSON output path")
    args = ap.parse_args(argv)

    manifest = convert(Path(args.md).read_text(encoding="utf-8"), args.parent_doc)

    err = self_check(manifest)
    if err:
        sys.stderr.write(f"error: {err}\n")
        return 1

    manifest_json = json.dumps(manifest, indent=2, ensure_ascii=False) + "\n"
    if args.manifest:
        Path(args.manifest).write_text(manifest_json, encoding="utf-8")
    else:
        sys.stdout.write(manifest_json)
    sys.stderr.write("sections=%d\n" % len(manifest))
    return 0


if __name__ == "__main__":
    sys.exit(main())
