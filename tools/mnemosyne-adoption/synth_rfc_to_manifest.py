#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""
Convert the protocol-synthesis RFC's headings into a Mnemosyne
bulk-section-create manifest for the `synth` namespace workspace
(docs/spec/synth).

The RFC ("SCE Forge Extensions for Wire Protocol Synthesis") is authored in
this repo at docs/spec/synth/rfc-sce-protocol-synthesis.md, co-located with the
forge implementation it defines (no [workspace.spec_source]; the tracked RFC is
the SSOT, and downstream consumers vendor it from here). SCE code comments cite
its sections as
"RFC §5.B" / "§6.2.6" / "§5.J.2"; this ledger makes the §synth-<id> form
resolve so the set_equality_validator can gate the enrolled modules.

Heading shapes this handles (all outside code fences):
  * h2 numbered:   "## §5 Proposed extensions"            -> synth-5
  * h2 appendix:   "## Appendix B — Worked example: ..."  -> synth-B
  * h3 numbered:   "### 5.B Codec DSL extensions"         -> synth-5-B
                   "### §2.1 Permanent non-goals"          -> synth-2.1
                   (the leading § sigil is optional; unnumbered h3 headings
                   like "### Already present ..." are prose, skipped)
  * h4 numbered:   "#### §6.2.6 Generated source drift detection" -> synth-6.2.6
  * bold items:    "**5.J.2 Statechart Rust `no_std` variant.**"  -> synth-5-J-2
                   (the §5.J proposal typesets its five subsections as bold
                   lead-ins, not headings; the RFC's own prose cites them as
                   §5.J.<n>, so they are sections by the document's own
                   convention. Only the 3-part digit.LETTER.digit shape is
                   claimed — ordered-list bold items ("1. **Stub trap
                   symbols**") never match.)

Section-id policy is the §-citation extractor's token rule: a dot survives
only between two digits, every other dot becomes a hyphen (5.B -> 5-B,
5.J.2 -> 5-J-2, 6.2.6 stays). This mirrors the scxml ledger's lettered-label
hyphenation (G.7 -> scxml-G-7) for the same reason: the extractor reads
"§synth-5.B" as id "synth-5" + prose ".B", so a dotted lettered id could
never be cited whole.

This is *adoption tooling*, not part of the SCE engine. It is deterministic
and uses only the Python standard library.

Usage:
    python3 tools/mnemosyne-adoption/synth_rfc_to_manifest.py \\
        [--md docs/spec/synth/rfc-sce-protocol-synthesis.md] \\
        [--parent-doc synth] \\
        [--manifest out/synth-manifest.json]
"""

import argparse
import json
import re
import sys
from pathlib import Path

# Numbered heading: "## §5 Title" / "### 5.B Title" / "#### §6.2.6 Title".
# The sigil is optional (the RFC mixes "### §2.1" and "### 1.1"). A label is
# digits followed by dotted digit-or-single-uppercase segments; unnumbered
# headings do not match and are skipped as prose.
HEADING_RE = re.compile(
    r"^(#{2,4})[ \t]+§?(?P<label>[0-9]+(?:\.(?:[0-9]+|[A-Z]))*)[ \t]+(?P<title>.*?)[ \t]*$"
)

# Appendix heading: "## Appendix B — Worked example: VLE ZInt u64".
APPENDIX_RE = re.compile(
    r"^##[ \t]+Appendix[ \t]+(?P<letter>[A-Z])[ \t]*(?:—|–|-)?[ \t]*(?P<title>.*?)[ \t]*$"
)

# Bold subsection lead-in: "**5.J.2 Statechart Rust `no_std` variant.** ...".
# Only the digit.LETTER.digit 3-part shape (the §5.J family's typesetting);
# the title is the bold span's text, trailing sentence period stripped.
BOLD_ITEM_RE = re.compile(
    r"^\*\*(?P<label>[0-9]+\.[A-Z]\.[0-9]+)[ \t]+(?P<title>[^*]+?)\.?\*\*"
)

FENCE_RE = re.compile(r"^[ \t]*(`{3,}|~{3,})")


def label_to_leaf(label):
    """Extractor-aligned id leaf: a dot survives only between two digits.
    5.B -> 5-B ; 5.J.2 -> 5-J-2 ; 6.2.6 -> 6.2.6 ; 7 -> 7."""
    out = []
    for i, ch in enumerate(label):
        if ch == ".":
            digit_flanked = (
                i > 0
                and label[i - 1].isdigit()
                and i + 1 < len(label)
                and label[i + 1].isdigit()
            )
            out.append("." if digit_flanked else "-")
        else:
            out.append(ch)
    return "".join(out)


def parent_label_of(label):
    """Drop the last dotted component: 5.B -> 5 ; 5.J.2 -> 5.J ; 6.2.6 -> 6.2 ;
    7 -> None. Appendix letters have no parent (handled by the caller)."""
    if "." not in label:
        return None
    return label.rsplit(".", 1)[0]


def extract_sections(md_text):
    """Yield (label_or_letter, title, is_appendix) for each numbered heading,
    appendix heading, and §5.J-family bold subsection item, outside code
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
            yield m.group("label"), m.group("title").strip(), False
            continue
        m = APPENDIX_RE.match(line)
        if m:
            yield m.group("letter"), m.group("title").strip(), True
            continue
        m = BOLD_ITEM_RE.match(line)
        if m:
            yield m.group("label"), m.group("title").strip(), False


def to_manifest(sections, parent_doc):
    """import-sections manifest with the document's own hierarchy
    (parent_section = the label minus its last dotted component). Skeleton
    only — like the mesh/wire ledgers, the snapshot markdown is in-repo and
    human-readable, so the ledger exists to resolve cites, not to render a
    vendored quote."""
    entries = []
    for label, title, is_appendix in sections:
        leaf = label if is_appendix else label_to_leaf(label)
        parent = None if is_appendix else parent_label_of(label)
        entry = {
            "section_id": f"synth-{leaf}",
            "parent_doc": parent_doc,
            "title": title or leaf,
        }
        if parent is not None:
            entry["parent_section"] = f"synth-{label_to_leaf(parent)}"
        entries.append(entry)
    return entries


def convert(md_text, parent_doc):
    return to_manifest(extract_sections(md_text), parent_doc)


def self_check(manifest):
    """Emitted-id invariants: unique, citation-safe (a dot only between
    digits), and every parent_section present in the emitted set."""
    ids = [e["section_id"] for e in manifest]
    if len(ids) != len(set(ids)):
        dupes = sorted({i for i in ids if ids.count(i) > 1})
        return f"duplicate section ids: {dupes}"
    id_set = set(ids)
    for entry in manifest:
        leaf = entry["section_id"][len("synth-") :]
        for i, ch in enumerate(leaf):
            if ch == "." and not (
                i > 0 and leaf[i - 1].isdigit() and i + 1 < len(leaf) and leaf[i + 1].isdigit()
            ):
                return f"non-citation-safe id {entry['section_id']!r}"
        parent = entry.get("parent_section")
        if parent is not None and parent not in id_set:
            return f"parent {parent!r} of {entry['section_id']!r} not emitted"
    return None


def main(argv=None):
    here = Path(__file__).resolve().parent
    repo_root = here.parent.parent
    default_md = repo_root / "docs" / "spec" / "synth" / "rfc-sce-protocol-synthesis.md"
    ap = argparse.ArgumentParser(description="protocol-synthesis RFC -> Mnemosyne manifest")
    ap.add_argument("--md", default=str(default_md))
    ap.add_argument("--parent-doc", default="synth")
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
