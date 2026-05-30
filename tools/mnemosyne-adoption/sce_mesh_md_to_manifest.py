#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""
Convert SCE_MESH.md's heading structure into a Mnemosyne bulk-section-create
manifest (the input to `import-sections`) for the SCE design ledger's `mesh`
namespace workspace (docs/sce-ledger/mesh).

This is the markdown sibling of the A1 converter
(scxml_toc_to_manifest.py, which reads the vendored W3C HTML). SCE_MESH.md is an
*internal* design document tracked in this repo, not an external standard, so:
  * the source is the markdown file itself (no vendored snapshot, no drift CI);
  * there is no published per-section anchor URL, so the manifest is skeleton
    only (section_id, parent_doc, title, parent_section) -- no normative_excerpt.

The ledger exists so code comments that cite the mesh design ("error.communication
raise policy, §16.7") resolve to a real section, letting the Mnemosyne
set_equality_validator gate mesh modules. Citations use the form §mesh-<n>; the
namespace segment (`mesh`, before the first hyphen) is what section_namespace
scoping keys on, so mesh cites never collide with §scxml-<n> (W3C) in the same
file.

Section-id naming policy (SSOT for the mesh namespace is this module):
  * ids are emitted BARE (no § sigil) -- import-sections stores section_id
    literally; the § is only the citation/render form.
  * mesh headings are purely numeric (`## 1.`, `### 3.1`, `#### 9.6.1`); numeric
    labels keep their dots, matching the numeric branch of the A1 policy:
        16.7  -> mesh-16.7   (cite §mesh-16.7)
        9.6.1 -> mesh-9.6.1
  * un-numbered headings (`### Problem`, `### Rationale`) are skipped: they carry
    no section number, so no §mesh-<n> citation can target them.

Fenced code blocks are skipped so that shell/yaml comment lines inside ```...```
(e.g. `# sce_mesh_common is a thin library`) are never mistaken for headings.

Usage:
    python3 tools/mnemosyne-adoption/sce_mesh_md_to_manifest.py \\
        [--md SCE_MESH.md] \\
        [--parent-doc GENERATED.md] \\
        [--manifest out/mesh-manifest.json]

With no --manifest the JSON is written to stdout and a human summary to stderr.
"""

import argparse
import json
import re
import sys
from pathlib import Path

# An ATX heading at level 2-4: "## ", "### ", "#### ". Level 1 (document title)
# and level 5+ are out of scope. Captures (hashes, text).
HEADING_RE = re.compile(r"^(#{2,4})[ \t]+(.+?)[ \t]*$")

# A leading numeric section label, optionally followed by a trailing dot
# ("## 1. Vision" has the dot, "### 3.1 Scheduler" does not). Captures
# (label, title).
LABEL_RE = re.compile(r"^(\d+(?:\.\d+)*)\.?[ \t]+(.*)$", re.DOTALL)

# A fenced code-block delimiter: ``` or ~~~ (optionally indented, optionally with
# an info string). Toggling these is what keeps in-code `#` lines from parsing as
# headings.
FENCE_RE = re.compile(r"^[ \t]*(`{3,}|~{3,})")


def parent_leaf_of_label(label):
    """Drop the last dotted component: 16.7 -> 16 ; 9.6.1 -> 9.6 ; 1 -> None."""
    if "." not in label:
        return None
    return label.rsplit(".", 1)[0]


def extract_headings(md_text):
    """Yield (label, title) for every numbered ATX heading outside code fences,
    in document order. Un-numbered headings are skipped."""
    in_fence = False
    fence_marker = None
    for line in md_text.splitlines():
        fence_m = FENCE_RE.match(line)
        if fence_m:
            marker = fence_m.group(1)[0]  # ` or ~
            if not in_fence:
                in_fence = True
                fence_marker = marker
            elif marker == fence_marker:
                in_fence = False
                fence_marker = None
            continue
        if in_fence:
            continue
        head_m = HEADING_RE.match(line)
        if not head_m:
            continue
        label_m = LABEL_RE.match(head_m.group(2))
        if not label_m:
            continue  # un-numbered heading -> no citation target
        yield label_m.group(1), label_m.group(2).strip()


def build_sections(headings):
    """Map (label, title) pairs to section dicts. parent_section is derived from
    the label (16.7 -> mesh-16), so it never depends on heading nesting depth."""
    sections = []
    seen = set()
    for label, title in headings:
        section_id = f"mesh-{label}"
        if section_id in seen:
            # A duplicate numeric label in the document is a source-doc problem;
            # keep the first, report the rest to stderr (idempotent ids).
            sys.stderr.write(f"warning: duplicate label {label!r}; keeping first\n")
            continue
        seen.add(section_id)
        parent_label = parent_leaf_of_label(label)
        sections.append(
            {
                "section_id": section_id,
                "parent_section": f"mesh-{parent_label}" if parent_label else None,
                "title": title,
            }
        )
    return sections


def to_manifest(sections, parent_doc):
    """import-sections manifest. parent_section is dropped when null; normative
    excerpt is intentionally omitted (skeleton only)."""
    manifest = []
    for s in sections:
        entry = {
            "section_id": s["section_id"],
            "parent_doc": parent_doc,
            "title": s["title"],
        }
        if s["parent_section"]:
            entry["parent_section"] = s["parent_section"]
        manifest.append(entry)
    return manifest


def convert(md_text, parent_doc):
    return to_manifest(build_sections(extract_headings(md_text)), parent_doc)


def main(argv=None):
    here = Path(__file__).resolve().parent
    repo_root = here.parent.parent
    ap = argparse.ArgumentParser(description="SCE_MESH.md -> Mnemosyne manifest")
    ap.add_argument("--md", default=str(repo_root / "SCE_MESH.md"))
    ap.add_argument("--parent-doc", default="GENERATED.md")
    ap.add_argument("--manifest", default=None, help="manifest JSON output path")
    args = ap.parse_args(argv)

    md_text = Path(args.md).read_text(encoding="utf-8")
    manifest = convert(md_text, args.parent_doc)

    # Self-check: every emitted id must be citation-safe (the §mesh- citation
    # extractor keeps a dot only when flanked by digits). mesh ids are numeric so
    # this always holds, but assert it so a future non-numeric heading cannot
    # silently emit an id the validator would truncate.
    for entry in manifest:
        leaf = entry["section_id"][len("mesh-") :]
        for i, ch in enumerate(leaf):
            if ch == "." and not (
                i > 0 and leaf[i - 1].isdigit() and i + 1 < len(leaf) and leaf[i + 1].isdigit()
            ):
                sys.stderr.write(
                    f"error: non-citation-safe id {entry['section_id']!r}\n"
                )
                return 1

    manifest_json = json.dumps(manifest, indent=2, ensure_ascii=False) + "\n"
    if args.manifest:
        Path(args.manifest).write_text(manifest_json, encoding="utf-8")
    else:
        sys.stdout.write(manifest_json)

    top = [s for s in manifest if "parent_section" not in s]
    sys.stderr.write(
        "sections=%d (top-level=%d, nested=%d)\n"
        % (len(manifest), len(top), len(manifest) - len(top))
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
