#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""
Migrate SCE's W3C spec citations in source *comments* to the Mnemosyne citation
form (``§scxml-<id>``) so that the Mnemosyne ``set_equality_validator``
(validate-code-refs) can check every code citation against the vendored
spec-mirror ledger (docs/spec/scxml). Two source shapes are recognized:

  * prose:      ``W3C SCXML <label>``     (digits follow the prefix)
  * bare-sigil: ``W3C §<label>`` / ``W3C SCXML §<label>``  (a ``§`` sigil was
                written by hand, without the Mnemosyne ``scxml-`` namespace)

Both collapse to the same canonical ``§scxml-<id>``. A bare ``§`` *without* a
W3C marker (``RFC §3``, ``SCE_FORGE.md §3.1``) is an SCE-internal design-doc
reference, not a W3C citation, and is left untouched for the design-ledger
workspace.

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
DEFAULT_MESH_LEDGER = os.path.join(
    REPO_ROOT, "docs", "sce-ledger", "mesh", ".atomic", "workspace.atomic.json"
)
DEFAULT_WIRE_LEDGER = os.path.join(
    REPO_ROOT, "docs", "sce-ledger", "wire", ".atomic", "workspace.atomic.json"
)
_NS_DEFAULT_LEDGER = {"mesh": DEFAULT_MESH_LEDGER, "wire": DEFAULT_WIRE_LEDGER}

# A citation label: a numeric path (digits + dotted digits) or a lettered
# appendix path (single uppercase letter + at least one dotted-digit group).
# Bare single letters and word tokens are intentionally NOT matched.
LABEL_RE = r"(?:[0-9]+(?:\.[0-9]+)*|[A-Z](?:\.[0-9]+)+)"

# The mesh namespace (SCE_MESH.md) has purely numeric section labels; unlike the
# scxml namespace there is no prose "W3C SCXML <n>" marker — the source always
# writes a bare `§<n>` sigil. Matching a bare sigil is far less self-evident than
# a marked one, so the bare path leans on three guards (see _plan_bare): ledger
# membership, a cross-namespace ambiguity check, and a foreign-marker denylist.
MESH_LABEL_RE = r"[0-9]+(?:\.[0-9]+)*"

# The wire namespace (Wire RFC) labels its commit-series waves W0..W5 and the
# half-wave W4.5. The label starts with a letter and keeps its dot verbatim
# (W4.5 -> wire-W4.5; the dot is digit-flanked so the extractor reads it whole).
# A wire label is unique to this namespace (scxml/mesh never start a label with
# 'W'), so the cross-namespace and foreign-marker guards are harmless no-ops for
# it -- "RFC §W4" is the Wire RFC, not the IETF "RFC <digits>" form.
WIRE_LABEL_RE = r"W[0-9]+(?:\.[0-9]+)?"

# An external-standard marker that DIRECTLY precedes the sigil (anchored to the
# end of the line-so-far) names a non-mesh citation the bare-sigil migrator must
# not claim:
#   W3C / W3C SCXML -> a W3C SCXML cite (the marked path's namespace, scxml)
#   RFC <digits>    -> an IETF RFC ("RFC 9562 §5.7" UUID, "RFC 8949 §4.2.1" CBOR)
#   ISO/LGPL/MIT    -> a standard / licence section
# Anchored to `$` (immediately before the §) on purpose: a foreign cite earlier
# on the same line ("W3C §5.10 ... see §16.7") must not disqualify a later mesh
# cite. Other-doc references ("rfc-...-phase-c.md §3", "Phase C P2 §3") and the
# mesh doc's own name ("SCE_MESH.md §16.7") are deliberately NOT markers here:
# the former all cite low §1-§6 numbers caught by the cross-namespace guard, and
# the latter IS the mesh source — treating "SCE_MESH.md" as foreign would skip
# the very citations this path exists to migrate.
FOREIGN_MARKER_RE = re.compile(
    r"(?:\bW3C(?:[ \t]+SCXML)?|\bRFC[ \t]+[0-9]+|\bISO|\bLGPL|\bMIT)[ \t]*$"
)

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


def _leaf(label, namespace):
    """The id leaf (the part after '<ns>-'). wire keeps its wave label verbatim
    (W4.5); scxml/mesh apply the A1 policy (numeric dots / lettered hyphens)."""
    return label if namespace == "wire" else label_to_leaf(label)


def label_to_id(label, namespace="scxml"):
    """'6.2' -> 'scxml-6.2' ; 'C.2' -> 'scxml-C-2' (A1 policy) ; 'W4.5' ->
    'wire-W4.5'. The namespace segment defaults to scxml."""
    return f"{namespace}-" + _leaf(label, namespace)


def _line_offsets(text):
    line_starts = [0]
    for m in re.finditer("\n", text):
        line_starts.append(m.end())

    def lineno(off):
        import bisect

        return bisect.bisect_right(line_starts, off)

    return line_starts, lineno


def plan_file(path, ledger_ids, prefix="W3C SCXML", namespace="scxml", exclude_ledger_ids=None):
    """Return (new_text, migrations, skipped) for one file without writing.

    migrations: list of dicts {line, label, id}
    skipped:    list of dicts {line, label, id, reason}

    namespace="scxml" runs the *marked* path (prose "W3C SCXML <n>" / bare
    "W3C §<n>"), unchanged. namespace="mesh" runs the *bare-sigil* path, where
    exclude_ledger_ids is the sibling (scxml) ledger used by the cross-namespace
    ambiguity guard in _plan_bare.
    """
    ext = os.path.splitext(path)[1]
    with open(path, "r", encoding="utf-8") as fh:
        text = fh.read()
    mask = comment_mask(text, nested=(ext in NESTED_BLOCK))
    line_starts, lineno = _line_offsets(text)

    if namespace == "scxml":
        return _plan_marked(text, mask, ledger_ids, prefix, lineno)
    return _plan_bare(text, mask, ledger_ids, exclude_ledger_ids or set(), namespace, lineno, line_starts)


def _plan_marked(text, mask, ledger_ids, prefix, lineno):
    """The scxml path: a W3C marker (prose or sigil) is required before the
    label. See plan_file's module docstring for the two shapes."""
    # A citation may be a slash chain ("3.8/3.9" = sections 3.8 and 3.9). Each
    # member is rewritten independently and rejoined with " / " (the spaces let
    # the extractor see two separate §ids; "§a/§b" without them would read as a
    # single id with a stray slash). "I/O" never matches: "I" alone is not a
    # LABEL_RE label (it needs a .digit), so the chain cannot start.
    chain = LABEL_RE + r"(?:/" + LABEL_RE + r")*"
    #   prose:      W3C SCXML 5.10            (digits directly after the prefix)
    #   bare-sigil: W3C §5.5 / W3C SCXML §3.3 (a § sigil already present)
    # The sigil branch is tried first so "W3C SCXML §3.3" is read as sigil, not
    # as a prose miss. A bare "§3" with no W3C marker (e.g. "RFC §3",
    # "SCE_FORGE.md §3.1") is never matched here -> SCE-internal design-doc refs
    # are handled by the bare-sigil path under their own namespace.
    sig_re = r"(?:W3C[ \t]+SCXML|W3C)[ \t]*§[ \t]*(?P<sigchain>" + chain + r")"
    prose_re = re.escape(prefix) + r"[ \t]+(?P<prosechain>" + chain + r")"
    pattern = re.compile(sig_re + r"|" + prose_re)
    migrations, skipped = [], []
    out, last = [], 0
    for m in pattern.finditer(text):
        if not mask[m.start()]:
            continue  # outside a comment -> never touch
        is_sig = m.group("sigchain") is not None
        chain_text = m.group("sigchain") if is_sig else m.group("prosechain")
        ln = lineno(m.start())
        # A quoted W3C citation ("W3C SCXML §5.8") is a runtime *string value*
        # shown inside a comment, not a section citation. Rewriting it would
        # desync the comment from the literal the code emits, so leave it
        # verbatim and report it for human review.
        if is_sig and m.start() > 0 and text[m.start() - 1] == '"':
            for lbl in chain_text.split("/"):
                skipped.append(
                    {
                        "line": ln,
                        "label": lbl,
                        "id": label_to_id(lbl),
                        "reason": "quoted spec-string value (runtime literal); left verbatim",
                    }
                )
            continue
        labels = chain_text.split("/")
        ids = [label_to_id(lbl) for lbl in labels]
        if all(s in ledger_ids for s in ids):
            out.append(text[last : m.start()])
            out.append(" / ".join("§" + s for s in ids))  # §scxml-a / §scxml-b
            last = m.end()
            for lbl, s in zip(labels, ids):
                migrations.append({"line": ln, "label": lbl, "id": s})
        else:
            # Whole chain left as prose; report only the member(s) not in the
            # ledger (version-like or hallucinated).
            for lbl, s in zip(labels, ids):
                if s not in ledger_ids:
                    skipped.append(
                        {
                            "line": ln,
                            "label": lbl,
                            "id": s,
                            "reason": "id not in ledger (version-like or hallucinated)",
                        }
                    )
    out.append(text[last:])
    return "".join(out), migrations, skipped


def _plan_bare(text, mask, target_ids, exclude_ids, namespace, lineno, line_starts):
    """The mesh path: a bare `§<n>` (no marker) is claimed for this namespace
    only when three guards all pass:
      1. ledger membership — `<ns>-<n>` is a real section in this ledger;
      2. cross-namespace   — `scxml-<n>` is NOT also a section. A number in both
         the mesh and scxml ledgers (e.g. §6.4: the W3C <invoke> section AND the
         mesh Custom-Transport profile) is ambiguous from the number alone, so
         it is reported for manual review rather than auto-claimed;
      3. foreign marker    — the text immediately before the sigil is not a
         W3C / RFC <digits> / ISO / licence marker (an external-standard cite).
    A reported (skipped) citation stays bare, which the namespace-scoped gate
    skips anyway — so leaving it is safe; the report is the manual-review surface.
    """
    label_re = WIRE_LABEL_RE if namespace == "wire" else MESH_LABEL_RE
    chain = label_re + r"(?:/" + label_re + r")*"
    pattern = re.compile(r"§[ \t]*(?P<barechain>" + chain + r")")
    migrations, skipped = [], []
    out, last = [], 0
    for m in pattern.finditer(text):
        if not mask[m.start()]:
            continue
        ln = lineno(m.start())
        labels = m.group("barechain").split("/")
        ids = [label_to_id(lbl, namespace) for lbl in labels]
        # Quoted runtime string value shown in a comment -> leave verbatim.
        if m.start() > 0 and text[m.start() - 1] == '"':
            for lbl, sid in zip(labels, ids):
                skipped.append(
                    {
                        "line": ln,
                        "label": lbl,
                        "id": sid,
                        "reason": "quoted string value (runtime literal); left verbatim",
                    }
                )
            continue
        # A hyphen-glued suffix ("§16.5-L3500", a line back-reference) would fuse
        # into the §id on render ("§mesh-16.5-L3500" reads as one bad id, since
        # '-' is a section-id char). Refuse it; the source must space-separate the
        # suffix ("§16.5 L3500") first.
        tail = text[m.end() : m.end() + 2]
        if len(tail) == 2 and tail[0] == "-" and tail[1].isalnum():
            for lbl, sid in zip(labels, ids):
                skipped.append(
                    {
                        "line": ln,
                        "label": lbl,
                        "id": sid,
                        "reason": "hyphen-glued suffix would corrupt the §id; space-separate it",
                    }
                )
            continue
        # Same-line context before the sigil that marks another namespace.
        line_prefix = text[line_starts[ln - 1] : m.start()]
        if FOREIGN_MARKER_RE.search(line_prefix):
            for lbl, sid in zip(labels, ids):
                skipped.append(
                    {
                        "line": ln,
                        "label": lbl,
                        "id": sid,
                        "reason": "foreign-standard marker before sigil (W3C/RFC/ISO/licence)",
                    }
                )
            continue
        reasons = []
        for lbl, sid in zip(labels, ids):
            leaf = _leaf(lbl, namespace)
            if sid not in target_ids:
                reasons.append((lbl, sid, f"id not in {namespace} ledger (non-section)"))
            elif ("scxml-" + leaf) in exclude_ids:
                reasons.append((lbl, sid, "ambiguous: also a W3C section; manual review"))
        if not reasons:
            out.append(text[last : m.start()])
            out.append(" / ".join("§" + s for s in ids))
            last = m.end()
            for lbl, sid in zip(labels, ids):
                migrations.append({"line": ln, "label": lbl, "id": sid})
        else:
            for lbl, sid, reason in reasons:
                skipped.append({"line": ln, "label": lbl, "id": sid, "reason": reason})
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
    ap.add_argument(
        "--namespace",
        default="scxml",
        choices=["scxml", "mesh", "wire"],
        help="scxml: W3C-marked cites (default); mesh: bare §<n> SCE_MESH.md "
        "cites; wire: bare §W<n> Wire RFC wave cites",
    )
    ap.add_argument(
        "--ledger",
        default=None,
        help="target atomic store (default: the namespace's workspace ledger)",
    )
    ap.add_argument(
        "--exclude-ledger",
        default=None,
        help="sibling ledger for the mesh cross-namespace guard "
        "(default: the scxml ledger)",
    )
    ap.add_argument("--prefix", default="W3C SCXML")
    ap.add_argument("--apply", action="store_true", help="write changes in place")
    ap.add_argument("--json", action="store_true", help="machine-readable report")
    args = ap.parse_args(argv)

    ledger_path = args.ledger or _NS_DEFAULT_LEDGER.get(args.namespace, DEFAULT_LEDGER)
    ledger_ids = load_ledger_ids(ledger_path)
    # The cross-namespace guard only applies to mesh (whose numeric labels can
    # collide with W3C section numbers). wire labels (W<n>) are unique, so no
    # sibling ledger is loaded.
    exclude_ids = set()
    if args.namespace == "mesh":
        exclude_ids = load_ledger_ids(args.exclude_ledger or DEFAULT_LEDGER)

    report = {"migrations": [], "skipped": [], "files_changed": 0}
    for path in iter_source_files(args.paths):
        new_text, migs, skips = plan_file(
            path, ledger_ids, args.prefix, args.namespace, exclude_ids
        )
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
            print(f"  {d['file']}:{d['line']}  §{d['label']} -> §{d['id']}")
        if report["skipped"]:
            print("\n-- left unchanged (review) --")
            for d in report["skipped"]:
                print(f"  {d['file']}:{d['line']}  §{d['label']}  ({d['reason']})")
        print(
            f"\n{verb} {len(report['migrations'])} citation(s) in "
            f"{report['files_changed']} file(s); {len(report['skipped'])} left unchanged."
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
