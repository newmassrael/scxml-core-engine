#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""Review aid for MIS-AIMED citations — the class no gate can catch.

Three gates already cover citations: the form gate forces `§scxml-` form in
enrolled directories, the ledger-existence gate rejects a section number that
does not exist, and `validate-code-refs` rejects an unbound citation. All three
ask "does this id resolve". None asks "is this the RIGHT id" — a citation whose
section exists and is bound is accepted even when it names a section about
something else entirely.

That class is real and was found by hand five times in one round, including a
clean off-by-one pair:

    /// Execute `<onentry>` actions for `state` (§scxml-3.7)   # 3.7 is <final>
    /// Execute `<onexit>`  actions for `state` (§scxml-3.8)   # 3.8 is <onentry>

This script surfaces candidates: a citation to a section whose title is a single
element (`<onentry>`), on a line that names a DIFFERENT element which itself has
a section. It is ADVISORY, not a gate, and deliberately not wired into pre-push
or CI — its output requires judgement and would otherwise become a gate that
fails on correct code, which trains people to bypass gates.

Known-benign shape, ~2 hits at the time of writing: the other element is a CHILD
of the cited one, so the citation is right and the mention is context —
`§scxml-5.5` (`<donedata>`) on a line about its `<content>` payload,
`§scxml-6.4` (`<invoke>`) on a line about its inline `<content>`. The ledger's
section tree does not encode SCXML element containment, so this cannot be
suppressed from data; a reviewer decides.

Usage:
    python3 tools/mnemosyne-adoption/review_citation_aim.py            # all hits
    python3 tools/mnemosyne-adoption/review_citation_aim.py --strong   # subject-position only

`--strong` keeps only lines where the other element is the grammatical SUBJECT
("Execute `<onexit>` …", "inline `<script>` …"), which is where every confirmed
mis-aim landed. Exit status is always 0: this reports, it does not gate.
"""

import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO_ROOT = HERE.parent.parent
LEDGER = REPO_ROOT / "docs" / "spec" / "scxml" / ".atomic" / "workspace.atomic.json"

SOURCE_EXT = (
    ".rs", ".c", ".h", ".cpp", ".hpp", ".cc", ".go", ".kt", ".py", ".jinja", ".jinja2",
)

# Verbs/adjectives that put the following element in subject position. Every
# confirmed mis-aim matched one of these; child-element mentions did not.
SUBJECT_LEAD = r"(?:Execute|Emit|inline|iterates|root|Parse|Validate|Record)"


def element_titled_sections():
    """{section_id: element} for sections whose title is exactly `<element>`."""
    sections = json.loads(LEDGER.read_text())["sections"]
    out = {}
    for sid, sec in sections.items():
        title = (sec.get("title") or "").strip()
        m = re.fullmatch(r"<([a-z]+)>", title)
        if m:
            out[sid] = m.group(1)
    return out


def tracked_sources():
    out = subprocess.run(
        ["git", "-C", str(REPO_ROOT), "ls-files"], capture_output=True, text=True
    )
    for rel in out.stdout.split():
        if rel.endswith(SOURCE_EXT) and not rel.startswith("docs/"):
            yield rel


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        "--strong",
        action="store_true",
        help="only lines where the other element is in subject position",
    )
    args = ap.parse_args(argv)

    title_el = element_titled_sections()
    elements = set(title_el.values())
    # element -> every section titled with it (an element can title more than one)
    el_sections = {}
    for sid, el in title_el.items():
        el_sections.setdefault(el, []).append(sid)

    hits = []
    for rel in tracked_sources():
        try:
            lines = (REPO_ROOT / rel).read_text(errors="ignore").split("\n")
        except OSError:
            continue
        for n, line in enumerate(lines, 1):
            for m in re.finditer(r"§scxml-([0-9A-Za-z.\-]+)", line):
                sid = "scxml-" + m.group(1)
                cited = title_el.get(sid)
                if cited is None:
                    continue
                for other in re.findall(r"<([a-z]+)>", line):
                    if other == cited or other not in elements:
                        continue
                    subject = bool(
                        re.search(SUBJECT_LEAD + r"\s+`?<" + other + r">", line)
                    )
                    if args.strong and not subject:
                        continue
                    hits.append(
                        {
                            "file": rel,
                            "line": n,
                            "cited": sid,
                            "cited_element": cited,
                            "other_element": other,
                            "other_sections": el_sections.get(other, []),
                            "subject_position": subject,
                            "text": line.strip()[:100],
                        }
                    )

    for h in hits:
        mark = "!!" if h["subject_position"] else "  "
        print(f"{mark} {h['file']}:{h['line']}")
        print(
            f"     cites §{h['cited']} = <{h['cited_element']}>, "
            f"line names <{h['other_element']}> "
            f"(its section: {', '.join(h['other_sections']) or 'none'})"
        )
        print(f"     {h['text']}")
    print(
        f"\n{len(hits)} candidate(s); "
        f"{sum(1 for h in hits if h['subject_position'])} in subject position. "
        "Advisory only — a child-element mention is normally correct."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
