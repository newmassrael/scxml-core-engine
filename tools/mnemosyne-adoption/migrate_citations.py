#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""
Migrate SCE's prose spec citations (``W3C SCXML <label>``) in source *comments*
to the Mnemosyne citation form (``§scxml-<id>``) so that the Mnemosyne
``set_equality_validator`` (validate-code-refs) can check every code citation
against the vendored spec-mirror ledger (docs/spec/scxml).

This is *adoption tooling*, not part of the SCE engine. It is deterministic and
uses only the Python standard library.

Scope and safety rules (why this is not a one-line sed):

  * Only the citation *label* is rewritten, never surrounding prose. A label is
    eligible only if it is a real section number:
      - numeric:          ``6.2``  ``5.10``  ``5.10.1``  ``3.13``
      - lettered/appendix: ``C.2``  ``B.2``  ``C.1``     ``C.2.1``
    Word-led mentions (``W3C SCXML BasicHTTPEventProcessor``, ``... Platform``,
    ``... specification``) are left untouched -- they are prose, not citations.

  * Section-id normalization is the SAME policy the A1 converter owns
    (scxml_toc_to_manifest.label_to_leaf): numeric labels keep their dots,
    lettered labels turn dots into hyphens. A1 is the SSOT; this module imports
    it so the two can never drift.

  * A label is migrated only if its normalized id EXISTS in the ledger. A number
    that is not a section (``W3C SCXML 1.0`` -- the spec *version*) or a typo'd /
    hallucinated citation has no ledger entry, so it is left as prose AND
    reported. That report is the human-review surface for the prose->§ cutover;
    versions stay prose, genuine citation errors get fixed at the source.

  * Replacements happen only inside *comments* (``//``, ``/* */``, and the
    ``*``-prefixed continuation lines of a block/doc comment). String- and
    char-literal contents are never edited, so runtime strings cannot change.

Usage::

    migrate_citations.py PATH [PATH ...]              # dry-run: print the plan
    migrate_citations.py PATH [PATH ...] --apply      # rewrite files in place
    migrate_citations.py PATH [PATH ...] --json        # machine-readable report

    --ledger PATH    atomic store to validate ids against
                     (default: docs/spec/scxml/.atomic/workspace.atomic.json,
                      resolved relative to this file's repo)
    --prefix TEXT    citation prefix to match (default: "W3C SCXML")
"""

import argparse
import json
import os
import re
import sys

# A1 owns the label -> id normalization policy; import it so we never drift.
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scxml_toc_to_manifest import label_to_leaf  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
REPO_ROOT = os.path.abspath(os.path.join(HERE, "..", ".."))
DEFAULT_LEDGER = os.path.join(
    REPO_ROOT, "docs", "spec", "scxml", ".atomic", "workspace.atomic.json"
)

# A citation label: a numeric path (digits + dotted digits) or a lettered
# appendix path (single uppercase letter + at least one dotted-digit group).
# Bare single letters and word tokens are intentionally NOT matched.
LABEL_RE = r"(?:[0-9]+(?:\.[0-9]+)*|[A-Z](?:\.[0-9]+)+)"

# File extensions we know how to tokenize for comments. Rust block comments
# nest; C/C++ ones do not.
NESTED_BLOCK = {".rs"}
KNOWN_EXTS = {".rs", ".cpp", ".cc", ".cxx", ".h", ".hpp", ".hxx", ".jinja", ".j2"}


def comment_mask(text, nested):
    """Return a bytearray-like list of booleans, True where text[i] is inside a
    line or block comment (and not inside a string/char literal)."""
    mask = [False] * len(text)
    i, n = 0, len(text)
    NORMAL, STR, CHR, LINE, BLOCK = range(5)
    state = NORMAL
    depth = 0
    while i < n:
        c = text[i]
        nxt = text[i + 1] if i + 1 < n else ""
        if state == NORMAL:
            if c == "/" and nxt == "/":
                state = LINE
                mask[i] = mask[i + 1] = True
                i += 2
                continue
            if c == "/" and nxt == "*":
                state = BLOCK
                depth = 1
                mask[i] = mask[i + 1] = True
                i += 2
                continue
            if c == '"':
                state = STR
            elif c == "'":
                state = CHR
            i += 1
        elif state == STR:
            if c == "\\":
                i += 2
                continue
            if c == '"':
                state = NORMAL
            i += 1
        elif state == CHR:
            if c == "\\":
                i += 2
                continue
            if c == "'":
                state = NORMAL
            i += 1
        elif state == LINE:
            if c == "\n":
                state = NORMAL
            else:
                mask[i] = True
            i += 1
        elif state == BLOCK:
            if c == "/" and nxt == "*" and nested:
                depth += 1
                mask[i] = mask[i + 1] = True
                i += 2
                continue
            if c == "*" and nxt == "/":
                depth -= 1
                mask[i] = mask[i + 1] = True
                i += 2
                if depth == 0:
                    state = NORMAL
                continue
            mask[i] = True
            i += 1
    return mask


def label_to_id(label):
    """'6.2' -> 'scxml-6.2' ; 'C.2' -> 'scxml-C-2' (A1 policy)."""
    return "scxml-" + label_to_leaf(label)


def plan_file(path, ledger_ids, prefix):
    """Return (new_text, migrations, skipped) for one file without writing.

    migrations: list of dicts {line, label, id, col}
    skipped:    list of dicts {line, label, id, reason}
    """
    ext = os.path.splitext(path)[1]
    with open(path, "r", encoding="utf-8") as fh:
        text = fh.read()
    mask = comment_mask(text, nested=(ext in NESTED_BLOCK))

    pattern = re.compile(re.escape(prefix) + r"[ \t]+(" + LABEL_RE + r")")
    migrations, skipped = [], []

    # Build line-start offsets for line-number reporting.
    line_starts = [0]
    for m in re.finditer("\n", text):
        line_starts.append(m.end())

    def lineno(off):
        # binary-free: line_starts is sorted; find rightmost <= off
        import bisect

        return bisect.bisect_right(line_starts, off)

    out = []
    last = 0
    for m in pattern.finditer(text):
        if not mask[m.start()]:
            continue  # outside a comment -> never touch
        label = m.group(1)
        sid = label_to_id(label)
        ln = lineno(m.start())
        if sid in ledger_ids:
            out.append(text[last : m.start()])
            out.append("§" + sid)  # §scxml-...
            last = m.end()
            migrations.append({"line": ln, "label": label, "id": sid})
        else:
            skipped.append(
                {
                    "line": ln,
                    "label": label,
                    "id": sid,
                    "reason": "id not in ledger (version-like or hallucinated)",
                }
            )
    out.append(text[last:])
    return "".join(out), migrations, skipped


def load_ledger_ids(ledger_path):
    with open(ledger_path, "r", encoding="utf-8") as fh:
        store = json.load(fh)
    return set(store["sections"].keys())


def iter_source_files(paths):
    for p in paths:
        if os.path.isfile(p):
            yield p
        elif os.path.isdir(p):
            for root, dirs, files in os.walk(p):
                dirs[:] = [
                    d for d in dirs if not d.startswith(".") and d not in ("target", "node_modules")
                ]
                for f in sorted(files):
                    if os.path.splitext(f)[1] in KNOWN_EXTS:
                        yield os.path.join(root, f)


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("paths", nargs="+", help="files or directories to migrate")
    ap.add_argument("--ledger", default=DEFAULT_LEDGER)
    ap.add_argument("--prefix", default="W3C SCXML")
    ap.add_argument("--apply", action="store_true", help="write changes in place")
    ap.add_argument("--json", action="store_true", help="machine-readable report")
    args = ap.parse_args(argv)

    ledger_ids = load_ledger_ids(args.ledger)

    report = {"migrations": [], "skipped": [], "files_changed": 0}
    for path in iter_source_files(args.paths):
        new_text, migs, skips = plan_file(path, ledger_ids, args.prefix)
        rel = os.path.relpath(path, REPO_ROOT)
        for d in migs:
            report["migrations"].append({"file": rel, **d})
        for d in skips:
            report["skipped"].append({"file": rel, **d})
        if migs:
            report["files_changed"] += 1
            if args.apply:
                with open(path, "w", encoding="utf-8") as fh:
                    fh.write(new_text)

    if args.json:
        json.dump(report, sys.stdout, indent=2, sort_keys=True)
        sys.stdout.write("\n")
    else:
        verb = "Rewrote" if args.apply else "Would rewrite"
        for d in report["migrations"]:
            print(f"  {d['file']}:{d['line']}  W3C SCXML {d['label']} -> §{d['id']}")
        if report["skipped"]:
            print("\n-- left as prose (id absent from ledger; review) --")
            for d in report["skipped"]:
                print(f"  {d['file']}:{d['line']}  W3C SCXML {d['label']}  (-> {d['id']}?)")
        print(
            f"\n{verb} {len(report['migrations'])} citation(s) in "
            f"{report['files_changed']} file(s); {len(report['skipped'])} left as prose."
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
