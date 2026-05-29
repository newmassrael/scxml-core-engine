#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""
Convert the W3C SCXML Recommendation's heading structure into a Mnemosyne
bulk-section-create manifest (the input to the future `A2` primitive) plus an
anchor map for later normative-excerpt assembly.

This is *adoption tooling*, not part of the SCE engine. It is deterministic
(same vendored HTML in -> same manifest out, byte for byte) and uses only the
Python standard library so it adds no dependency to any engine crate. The live
drift-check CI job (B1) re-fetches the upstream URL and compares sha256 against
spec-snapshot/PROVENANCE.json; this converter only ever reads the vendored
snapshot.

Section-id naming policy (this is SCE's own choice; the SSOT for it is this
module):
  * numeric labels keep their dots:        5.10      -> §scxml-5.10
  * lettered/appendix labels use hyphens:  G.1       -> §scxml-G-1
  * unnumbered appendix content (e.g. the Appendix D algorithm helpers, which
    carry no spec number) synthesize an id from the appendix letter plus the
    spec anchor:                           #interpret -> §scxml-D-interpret

The policy is designed so every id is "citation-safe" under Mnemosyne's code
citation extractor. That extractor's grammar is owned by Mnemosyne, not
mirrored here: rather than re-encode the rule, the policy's compatibility with
it is proven by the closed-loop integration test (tests/), which feeds
representative ids through the real `mnemosyne-cli validate-code-refs`.

Usage:
    python3 tools/mnemosyne-adoption/scxml_toc_to_manifest.py \\
        [--html tools/mnemosyne-adoption/spec-snapshot/scxml-REC-20150901.html] \\
        [--parent-doc docs/GENERATED.md] \\
        [--source-revision REC-scxml-20150901] \\
        [--manifest out/scxml-manifest.json] \\
        [--anchor-map out/scxml-anchor-map.json]

With no --manifest/--anchor-map the JSON is written to stdout and a human
summary to stderr.
"""

import argparse
import json
import re
import sys
from html.parser import HTMLParser
from pathlib import Path

W3C_SCXML_URL = "https://www.w3.org/TR/scxml/"

# A leading section label: numeric (1, 3.2, 3.12.1) OR a single appendix letter
# optionally followed by dot-numbers (A, A.1, C.2). Anything else (e.g.
# "procedure interpret(...)", "Datatypes") has no label.
LABEL_RE = re.compile(r"^((?:\d+(?:\.\d+)*)|(?:[A-Z](?:\.\d+)*))\s+(.*)$", re.DOTALL)

HEADING_TAGS = {"h2", "h3", "h4"}


class HeadingExtractor(HTMLParser):
    """Collect (level, anchor, text) for every h2/h3/h4 in document order.

    Anchors come from either the modern form (`<h3 id="X">`) or the legacy form
    (`<h3><a id="X" name="X" />...`); the first id seen for a heading wins.
    convert_charrefs (default) turns `&lt;scxml&gt;` back into `<scxml>`.
    """

    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.headings = []
        self._level = None
        self._buf = []
        self._anchor = None

    def handle_starttag(self, tag, attrs):
        if tag in HEADING_TAGS:
            self._level = int(tag[1])
            self._buf = []
            self._anchor = dict(attrs).get("id")  # modern form
        elif tag == "a" and self._level is not None and self._anchor is None:
            anchor = dict(attrs).get("id")  # legacy in-heading anchor
            if anchor:
                self._anchor = anchor

    def handle_startendtag(self, tag, attrs):
        # Self-closing legacy anchor: <a id="X" name="X" />
        self.handle_starttag(tag, attrs)

    def handle_data(self, data):
        if self._level is not None:
            self._buf.append(data)

    def handle_endtag(self, tag):
        if tag in HEADING_TAGS and self._level is not None:
            text = " ".join("".join(self._buf).split())
            self.headings.append((self._level, self._anchor, text))
            self._level = None
            self._buf = []
            self._anchor = None


def label_to_leaf(label):
    """5.10 -> 5.10 ; G.1 -> G-1 ; A -> A. Dots kept only between digits."""
    if label[0].isdigit():
        return label  # numeric: dots stay between digits
    return label.replace(".", "-")  # lettered: dots become hyphens


def parent_leaf_of_label(label):
    """Drop the last dotted component: 5.10 -> 5 ; 3.12.1 -> 3.12 ; A.1 -> A ; A -> None."""
    if "." not in label:
        return None
    return label.rsplit(".", 1)[0]


def build_sections(headings):
    """Map extracted headings to section dicts in document order. Each:
    section_id, parent_section, title, anchor_url."""
    sections = []
    # stack[level] = section_id of the most recent heading at that level.
    stack = {2: None, 3: None, 4: None}
    current_appendix = None  # letter of the appendix we are currently inside

    for level, anchor, raw in headings:
        if not anchor or not raw:
            continue

        label_m = LABEL_RE.match(raw)
        label = label_m.group(1) if label_m else None
        title = label_m.group(2).strip() if label_m else raw

        # Track which appendix we are inside (for unnumbered appendix content).
        if label and level == 2 and label[0].isalpha():
            current_appendix = label  # e.g. "A", "D", "G"

        if label is not None:
            leaf = label_to_leaf(label)
            parent_label = parent_leaf_of_label(label)
            parent_leaf = label_to_leaf(parent_label) if parent_label else None
        else:
            # Unnumbered content. Only meaningful inside an appendix (the
            # Appendix D algorithm helpers). Front matter (Abstract, Status,
            # Table of Contents, ...) has no label and no appendix -> skip.
            if current_appendix is None:
                stack[level] = None
                continue
            leaf = f"{current_appendix}-{anchor}"
            parent_leaf = None  # filled from the heading stack below

        section_id = f"scxml-{leaf}"

        # Parent: prefer the label-derived parent; otherwise the nearest
        # enclosing heading on the stack (covers the unnumbered appendix funcs).
        parent_section = None
        if label is not None and parent_leaf is not None:
            parent_section = f"scxml-{parent_leaf}"
        else:
            for lvl in range(level - 1, 1, -1):
                if stack.get(lvl):
                    parent_section = stack[lvl]
                    break
            # Unnumbered appendix subsections marked <h2> (e.g. Appendix D's
            # #InformalSemantics, #Algorithm) have no enclosing heading on the
            # stack; root them at the appendix letter itself.
            if parent_section is None and label is None and current_appendix:
                root = f"scxml-{current_appendix}"
                if section_id != root:
                    parent_section = root

        sections.append(
            {
                "section_id": f"§{section_id}",
                "parent_section": f"§{parent_section}" if parent_section else None,
                "title": title,
                "anchor_url": f"{W3C_SCXML_URL}#{anchor}",
            }
        )
        stack[level] = section_id
        for deeper in range(level + 1, 5):
            stack[deeper] = None

    return sections


def to_manifest(sections, parent_doc):
    """A2 bulk-section-create manifest. normative_excerpt is intentionally
    omitted (skeleton only); excerpts are assembled per-section later using the
    anchor map. parent_section is dropped when null."""
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


def to_anchor_map(sections, source_revision):
    return {
        s["section_id"]: {
            "anchor_url": s["anchor_url"],
            "source_revision": source_revision,
        }
        for s in sections
    }


def convert(html, parent_doc, source_revision):
    parser = HeadingExtractor()
    parser.feed(html)
    sections = build_sections(parser.headings)
    return to_manifest(sections, parent_doc), to_anchor_map(sections, source_revision)


def main(argv=None):
    here = Path(__file__).resolve().parent
    ap = argparse.ArgumentParser(description="W3C SCXML TOC -> Mnemosyne manifest")
    ap.add_argument(
        "--html", default=str(here / "spec-snapshot" / "scxml-REC-20150901.html")
    )
    ap.add_argument("--parent-doc", default="docs/GENERATED.md")
    ap.add_argument("--source-revision", default="REC-scxml-20150901")
    ap.add_argument("--manifest", default=None, help="manifest JSON output path")
    ap.add_argument("--anchor-map", default=None, help="anchor-map JSON output path")
    args = ap.parse_args(argv)

    html = Path(args.html).read_text(encoding="utf-8")
    manifest, anchor_map = convert(html, args.parent_doc, args.source_revision)

    manifest_json = json.dumps(manifest, indent=2, ensure_ascii=False) + "\n"
    anchor_json = json.dumps(anchor_map, indent=2, ensure_ascii=False) + "\n"

    if args.manifest:
        Path(args.manifest).write_text(manifest_json, encoding="utf-8")
    if args.anchor_map:
        Path(args.anchor_map).write_text(anchor_json, encoding="utf-8")
    if not args.manifest and not args.anchor_map:
        sys.stdout.write(manifest_json)

    appendix = [s for s in manifest if re.match(r"§scxml-[A-Z]", s["section_id"])]
    sys.stderr.write(
        "sections=%d (body=%d, appendix=%d)\n"
        % (len(manifest), len(manifest) - len(appendix), len(appendix))
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
