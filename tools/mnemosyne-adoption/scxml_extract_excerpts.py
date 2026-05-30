#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""
R2 — extract a normative excerpt for each W3C SCXML section from the vendored
snapshot, keyed by the same section ids the A1 converter assigns.

A section's excerpt is its *direct* body text: everything between its heading
and the next heading of any level (so a parent section's excerpt is its intro
prose, not its subsections — those are their own sections). Tags are stripped
and whitespace collapsed; the result is the vendored quote anchored to the
section for Mnemosyne's normative_excerpt field (B3 read-path + B2 drift).

Naming is NOT re-derived here: this imports scxml_toc_to_manifest (A1) so the
section-id SSOT stays single. This module only adds body-text extraction.

Output: JSON `{section_id: {text, anchor_url, source_revision}}` (the shape the
apply step feeds to `set-section-normative-excerpt`). Sections whose direct
body is blank (pure container headings) are omitted.

Usage:
    python3 tools/mnemosyne-adoption/scxml_extract_excerpts.py \\
        [--html .../spec-snapshot/scxml-REC-20150901.html] \\
        [--source-revision REC-scxml-20150901] \\
        [--out out/scxml-excerpts.json]
"""

import argparse
import json
import sys
from html.parser import HTMLParser
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import scxml_toc_to_manifest as a1  # noqa: E402

HEADING_TAGS = {"h2", "h3", "h4"}
# Tags whose text content is not part of the readable normative prose.
DROP_TEXT_TAGS = {"script", "style"}


class BodyExtractor(HTMLParser):
    """Capture each heading's anchor plus the body text that follows it, up to
    the next heading. Runs in one pass, document order."""

    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.bodies = {}  # anchor -> body text
        self._cur_anchor = None
        self._in_heading = False
        self._heading_anchor = None
        self._buf = []
        self._drop_depth = 0

    def _flush(self):
        if self._cur_anchor is not None:
            text = " ".join("".join(self._buf).split())
            self.bodies[self._cur_anchor] = text
        self._buf = []

    def handle_starttag(self, tag, attrs):
        if tag in HEADING_TAGS:
            self._flush()
            self._in_heading = True
            self._heading_anchor = dict(attrs).get("id")  # modern form
        elif tag == "a" and self._in_heading and self._heading_anchor is None:
            a = dict(attrs).get("id")  # legacy in-heading anchor
            if a:
                self._heading_anchor = a
        elif tag in DROP_TEXT_TAGS:
            self._drop_depth += 1

    def handle_startendtag(self, tag, attrs):
        if tag == "a" and self._in_heading and self._heading_anchor is None:
            a = dict(attrs).get("id")
            if a:
                self._heading_anchor = a

    def handle_endtag(self, tag):
        if tag in HEADING_TAGS and self._in_heading:
            self._in_heading = False
            self._cur_anchor = self._heading_anchor  # body now accumulates here
        elif tag in DROP_TEXT_TAGS and self._drop_depth > 0:
            self._drop_depth -= 1

    def handle_data(self, data):
        if self._in_heading or self._drop_depth:
            return
        if self._cur_anchor is not None:
            self._buf.append(data)

    def close(self):
        super().close()
        self._flush()


def extract(html, source_revision):
    # A1 owns the heading -> section_id mapping (anchor + naming policy).
    headings = a1.HeadingExtractor()
    headings.feed(html)
    sections = a1.build_sections(headings.headings)

    bodies = BodyExtractor()
    bodies.feed(html)
    bodies.close()

    excerpts = {}
    for s in sections:
        anchor = s["anchor_url"].rsplit("#", 1)[-1]
        text = bodies.bodies.get(anchor, "")
        if not text:
            continue  # pure container heading — no direct prose
        excerpts[s["section_id"]] = {
            "text": text,
            "anchor_url": s["anchor_url"],
            "source_revision": source_revision,
        }
    return excerpts


def main(argv=None):
    ap = argparse.ArgumentParser(description="W3C SCXML section -> normative excerpt")
    ap.add_argument(
        "--html", default=str(HERE / "spec-snapshot" / "scxml-REC-20150901.html")
    )
    ap.add_argument("--source-revision", default="REC-scxml-20150901")
    ap.add_argument("--out", default=None)
    args = ap.parse_args(argv)

    html = Path(args.html).read_text(encoding="utf-8")
    excerpts = extract(html, args.source_revision)
    out_json = json.dumps(excerpts, indent=2, ensure_ascii=False) + "\n"
    if args.out:
        Path(args.out).write_text(out_json, encoding="utf-8")
    else:
        sys.stdout.write(out_json)

    lengths = [len(v["text"]) for v in excerpts.values()]
    sys.stderr.write(
        "excerpts=%d (of sections); text len min/median/max = %d/%d/%d\n"
        % (
            len(excerpts),
            min(lengths) if lengths else 0,
            sorted(lengths)[len(lengths) // 2] if lengths else 0,
            max(lengths) if lengths else 0,
        )
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
