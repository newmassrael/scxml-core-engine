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
import subprocess
import sys

# A1 owns the label -> id normalization policy; import it so we never drift.
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scxml_toc_to_manifest import label_to_leaf  # noqa: E402

# The synth converter owns the synth leaf policy (extractor token rule: a dot
# survives only between digits, 5.B -> 5-B, 5.J.2 -> 5-J-2); import it so the
# migrator and the manifest can never drift.
from synth_rfc_to_manifest import label_to_leaf as synth_label_to_leaf  # noqa: E402

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
DEFAULT_SYNTH_LEDGER = os.path.join(
    REPO_ROOT, "docs", "spec", "synth", ".atomic", "workspace.atomic.json"
)
DEFAULT_BYTESGUARD_LEDGER = os.path.join(
    REPO_ROOT, "docs", "sce-ledger", "bytesguard", ".atomic", "workspace.atomic.json"
)
_NS_DEFAULT_LEDGER = {
    "mesh": DEFAULT_MESH_LEDGER,
    "wire": DEFAULT_WIRE_LEDGER,
    "synth": DEFAULT_SYNTH_LEDGER,
    "bytesguard": DEFAULT_BYTESGUARD_LEDGER,
}

# A citation label: a numeric path (digits + dotted digits) or a lettered
# appendix path (single uppercase letter + at least one dotted-digit group).
# Bare single letters and word tokens are intentionally NOT matched.
LABEL_RE = r"(?:[0-9]+(?:\.[0-9]+)*|[A-Z](?:\.[0-9]+)+)"

# Words that may sit between the marker and the label and still mean "this is a
# section citation": "W3C SCXML Appendix D.2", "W3C SCXML Section 3.6". Without
# them the word occupies the label position and the whole citation matches
# NOTHING — neither migrated nor reported — so a wrong section number passes
# every gate. Measured across three separate words before this became a set:
# `Appendix` hid 376 sites (D.2, which is not a ledger id), `Section` 22 and
# `specification` 14, the first two of them inside directories a ledger already
# enrolls.
#
# The set is deliberately CLOSED, and `HIDDEN_CITE_RE` below reports any other
# word in that position rather than silently accepting or silently ignoring it.
# Growing this list on demand would make gate coverage depend on authoring
# vocabulary, which is what let the class recur three times.
CONNECTIVE_RE = r"(?:Appendix|Section|section|specification)"

# The class detector lives in `_plan_marked` (it needs the runtime `prefix`):
# marker, then a word that is NOT a known connective, then a section-shaped
# label. That shape is either a citation hidden behind prose or prose that reads
# like one; both need a human, so it is reported as its own violation rather than
# guessed either way.

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

# The synth namespace (protocol-synthesis RFC, docs/spec/synth) labels mix
# dotted digits and single-uppercase segments: 5.B, 5.J.2, 6.2.6, 2.1, 7.
#
# A LETTERED label (one with a `.UPPERCASE` segment) is structurally unique
# to this RFC across every document SCE comments cite, so it is claimable
# bare. A NUMERIC label is not: "RFC §<n>" in this codebase names at least
# six different internal documents (the C11-backend RFC's "§2.1 dim
# coverage", the EventSchema RFC's "§3 B2/B3", the sce:template RFC's
# "§6.3 Q3", the NL->IR design RFC's "§8 strict variant membership",
# SCE_FORGE.md, SCE_ERROR_CONTRACT.md ...), and sibling-ledger exclusion
# cannot see unregistered documents. So a numeric label is claimed ONLY
# under a marker that names the document ("SCE Protocol-Synthesis RFC §7"); every
# other numeric sighting is reported for manual review — the reviewer
# resolves it by rewriting the confirmed-synth sites to the §synth-<id>
# form directly (the namespace token itself names the document).
SYNTH_LABEL_RE = r"[0-9]+(?:\.(?:[0-9]+|[A-Z]))*"
SYNTH_DOC_MARKER_RE = re.compile(r"(?i)\bSCE[ \t]+Protocol-Synthesis[ \t]+RFC[ \t]*§?[ \t]*$")

# The bytesguard namespace (EventSchema bytes-guard RFC, docs/sce-ledger/
# bytesguard) has purely numeric labels (1, 1.3, 3, 3.1, 6) — every one
# collides with sibling-ledger numbers AND with the other internal documents
# that share the bare "RFC §<n>" convention, so a bytesguard cite is claimed
# ONLY under its document-naming marker ("rfc-eventschema-bytes-guard.md §3",
# backtick-quoted or bare). Unmarked sightings are reported; the reviewer
# resolves confirmed ones to the §bytesguard-<n> form directly.
BYTESGUARD_LABEL_RE = r"[0-9]+(?:\.[0-9]+)*"
BYTESGUARD_DOC_MARKER_RE = re.compile(r"bytes-guard\.md`?[ \t]*$")

# An external-standard marker that DIRECTLY precedes the sigil (anchored to the
# end of the line-so-far) names a non-mesh citation the bare-sigil migrator must
# not claim:
#   W3C / W3C SCXML -> a W3C SCXML cite (the marked path's namespace, scxml)
#   RFC <digits>    -> an IETF RFC ("RFC 9562 §5.7" UUID, "RFC 8949 §4.2.1" CBOR)
#   ISO/LGPL/MIT    -> a standard / licence section
#   ARCHITECTURE    -> ARCHITECTURE.md, whose headings carry no numbers at all,
#                      so "ARCHITECTURE §9.3" can never denote a mesh section.
#                      Measured: without this the mesh path rewrote
#                      "ARCHITECTURE §9.3" on a `StageCopyPolicy` variant into
#                      §mesh-9.3 "Remote Invoke Lifecycle" — a real section, so
#                      every gate stayed green on a citation about the wrong
#                      subject. 9 sibling occurrences live under sce-build/src/
#                      forge and would migrate the same way if that tree is ever
#                      enrolled.
# Anchored to `$` (immediately before the §) on purpose: a foreign cite earlier
# on the same line ("W3C §5.10 ... see §16.7") must not disqualify a later mesh
# cite. Other-doc references ("rfc-...-phase-c.md §3", "Phase C P2 §3") and the
# mesh doc's own name ("SCE_MESH.md §16.7") are deliberately NOT markers here:
# the former all cite low §1-§6 numbers caught by the cross-namespace guard, and
# the latter IS the mesh source — treating "SCE_MESH.md" as foreign would skip
# the very citations this path exists to migrate.
FOREIGN_MARKER_RE = re.compile(
    r"(?:\bW3C(?:[ \t]+SCXML)?|\bRFC[ \t]+[0-9]+|\bISO|\bLGPL|\bMIT"
    r"|\bARCHITECTURE(?:\.md)?)[ \t]*$"
)

# File extensions we know how to tokenize for comments. Rust block comments
# nest; C/C++ ones do not. The mask family mirrors mnemosyne's
# comment_syntax_for dispatch (Slash / Hash / whole-text) so the migrator
# rewrites exactly what validate-code-refs will scan:
#   slash:      C-family line+block comments
#   hash:       `#` line comments (string literals masked)
#   whole-text: no comment grammar (templates, XML) — the validator
#               whole-text-scans these unknown extensions, so the migrator
#               rewrites whole-text too (lockstep over precision).
NESTED_BLOCK = {".rs"}
SLASH_EXTS = {".rs", ".cpp", ".cc", ".cxx", ".h", ".hpp", ".hxx", ".jinja", ".j2",
              ".c", ".go", ".kt", ".kts"}
HASH_EXTS = {".py", ".sh"}
WHOLE_TEXT_EXTS = {".jinja2", ".scxml", ".xsd"}
KNOWN_EXTS = SLASH_EXTS | HASH_EXTS | WHOLE_TEXT_EXTS


def comment_mask(text, nested):
    """Return a bytearray-like list of booleans, True where text[i] is inside a
    line or block comment (and not inside a string/char literal).

    Rust-aware when `nested` is set (the .rs dispatch): raw strings
    (`r"..."`, `r#"..."#`, `br##"..."##`) terminate only at the matching
    `"#...#` run — a plain-quote state machine flips parity on a raw-string
    body containing an odd number of `"` and silently mis-masks everything
    after it (found on forge/diagnostic.rs, 316 raw strings; comment cites
    downstream were skipped). A `'` is a char literal only when it closes
    within the next few chars (`'x'`, `'\\n'`, `'\\u{...}'`); otherwise it is
    a lifetime (`&'a str`, `'static`) and must not open a string-like state."""
    mask = [False] * len(text)
    i, n = 0, len(text)
    NORMAL, STR, LINE, BLOCK = range(4)
    state = NORMAL
    depth = 0

    def char_literal_end(start):
        """Index one past a char literal opening at text[start] == \"'\", or
        None if this quote is a lifetime, not a char literal."""
        j = start + 1
        if j >= n:
            return None
        if text[j] == "\\":
            j += 2
            if j < n and text[j - 1] == "u" and text[j] == "{":
                close = text.find("}", j)
                if close == -1:
                    return None
                j = close + 1
        elif text[j] == "'":
            return None  # '' is not a char literal
        else:
            j += 1
        if j < n and text[j] == "'":
            return j + 1
        return None

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
            # Rust raw / byte-raw strings: r"..."  r#"..."#  br##"..."##.
            # The body is opaque (no escapes); it ends at `"` + the same
            # number of `#` as the opener.
            if nested and c in "rb":
                j = i
                if text[j] == "b" and j + 1 < n and text[j + 1] == "r":
                    j += 1
                if text[j] == "r":
                    k = j + 1
                    hashes = 0
                    while k < n and text[k] == "#":
                        hashes += 1
                        k += 1
                    if k < n and text[k] == '"':
                        closer = '"' + "#" * hashes
                        end = text.find(closer, k + 1)
                        i = n if end == -1 else end + len(closer)
                        continue
            if c == '"' or (nested and c == "b" and nxt == '"'):
                if c == "b":
                    i += 1  # the opening quote of a byte string
                state = STR
                i += 1
                continue
            if c == "'":
                if nested:
                    end = char_literal_end(i)
                    i = end if end is not None else i + 1
                    continue
                # Non-Rust C family: treat as a char literal with escapes.
                j = i + 1
                while j < n:
                    if text[j] == "\\":
                        j += 2
                        continue
                    if text[j] == "'":
                        j += 1
                        break
                    if text[j] == "\n":
                        break  # unterminated; bail at line end
                    j += 1
                i = j
                continue
            i += 1
        elif state == STR:
            if c == "\\":
                i += 2
                continue
            if c == '"':
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


def hash_comment_mask(text):
    """True where text[i] is inside a `#` line comment (string literals
    masked out, mirroring mnemosyne's Hash comment syntax)."""
    mask = [False] * len(text)
    i, n = 0, len(text)
    NORMAL, STR1, STR2, LINE = range(4)
    state = NORMAL
    while i < n:
        c = text[i]
        if state == NORMAL:
            if c == "#":
                state = LINE
                mask[i] = True
            elif c == "'":
                state = STR1
            elif c == '"':
                state = STR2
            i += 1
        elif state in (STR1, STR2):
            if c == "\\":
                i += 2
                continue
            if (state == STR1 and c == "'") or (state == STR2 and c == '"'):
                state = NORMAL
            i += 1
        else:  # LINE
            if c == "\n":
                state = NORMAL
            else:
                mask[i] = True
            i += 1
    return mask


def mask_for(path, text):
    """Comment mask dispatch, in lockstep with mnemosyne's comment_syntax_for:
    slash-family files get the C tokenizer, hash-family the `#` tokenizer
    (CMakeLists.txt included), and whole-text extensions (templates, XML —
    unknown to the validator, which whole-text-scans them) an all-True mask."""
    if os.path.basename(path) == "CMakeLists.txt":
        return hash_comment_mask(text)
    ext = os.path.splitext(path)[1]
    if ext in WHOLE_TEXT_EXTS:
        return [True] * len(text)
    if ext in HASH_EXTS:
        return hash_comment_mask(text)
    return comment_mask(text, nested=(ext in NESTED_BLOCK))


def _leaf(label, namespace):
    """The id leaf (the part after '<ns>-'). wire keeps its wave label verbatim
    (W4.5); synth applies the extractor token rule (dots survive only between
    digits, owned by synth_rfc_to_manifest); scxml/mesh apply the A1 policy
    (numeric dots / lettered hyphens)."""
    if namespace == "wire":
        return label
    if namespace == "synth":
        return synth_label_to_leaf(label)
    return label_to_leaf(label)


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
    with open(path, "r", encoding="utf-8") as fh:
        text = fh.read()
    mask = mask_for(path, text)
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
    # `(?!/)` after the chain refuses a trailing slash that no label follows.
    # "3.13/Appendix D" would otherwise migrate its head to "§scxml-3.13" and
    # leave "/Appendix D" glued to it, and the validator reads that whole run as
    # ONE id ("scxml-3.13/Appendix") -> a section_missing on a section nobody
    # cited. Emitting a token that parses as a different citation than the author
    # wrote is worse than not migrating: refusing leaves the prose in place, so
    # the citation-form gate reports it and a human separates the chain.
    chain = LABEL_RE + r"(?:/" + LABEL_RE + r")*" + r"(?!/)"
    #   prose:      W3C SCXML 5.10            (digits directly after the prefix)
    #   bare-sigil: W3C §5.5 / W3C SCXML §3.3 (a § sigil already present)
    # The sigil branch is tried first so "W3C SCXML §3.3" is read as sigil, not
    # as a prose miss. A bare "§3" with no W3C marker (e.g. "RFC §3",
    # "SCE_FORGE.md §3.1") is never matched here -> SCE-internal design-doc refs
    # are handled by the bare-sigil path under their own namespace.
    sig_re = r"(?:W3C[ \t]+SCXML|W3C)[ \t]*§[ \t]*(?P<sigchain>" + chain + r")"
    # `Appendix ` may sit between the marker and the label ("W3C SCXML Appendix
    # D.2"). Without this the word occupies the label position, so the whole
    # citation matched nothing at all — neither migrated nor reported — and a
    # fabricated appendix subsection passed every gate silently. Measured: the
    # Kotlin engine carried seven "Appendix D.2" cites while `D.2` is not a
    # ledger section, and `--check-ledger` saw none of them. The group is
    # non-capturing and inside the match span, so a migration replaces the word
    # along with the label — the §id already names the appendix.
    prose_re = (
        re.escape(prefix)
        + r"[ \t]+(?:"
        + CONNECTIVE_RE
        + r"[ \t]+)?(?P<prosechain>"
        + chain
        + r")"
    )
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

    # Class detector: a citation hidden behind a word in the label position.
    # `CONNECTIVE_RE` above lists the words that legitimately precede a label;
    # anything else there means the marker is followed by prose that a
    # section-shaped number then follows, and the main pattern skipped the whole
    # thing silently. Report it so a human normalises the comment, rather than
    # extending the connective list on demand — that is how this class recurred
    # three times (Appendix / Section / specification).
    hidden_re = re.compile(
        re.escape(prefix)
        + r"[ \t]+(?!"
        + CONNECTIVE_RE
        + r"[ \t])(?P<word>[A-Za-z][A-Za-z-]*)[ \t]+(?P<hidden>"
        + LABEL_RE
        + r")(?![0-9])"
    )
    for m in hidden_re.finditer(text):
        if not mask[m.start()]:
            continue
        lbl = m.group("hidden")
        # A BARE integer after a word is a W3C IRP *test* number, not a section
        # ("W3C SCXML test 530"). Spec sections are 1-7 plus lettered appendices,
        # every one of which carries a dot. Same rule the two check paths apply,
        # applied here so the detector cannot invent citations out of test ids.
        if "." not in lbl or lbl == "1.0":
            continue
        skipped.append(
            {
                "line": lineno(m.start()),
                "label": m.group("hidden"),
                "id": label_to_id(m.group("hidden")),
                "reason": (
                    f"citation hidden behind the word {m.group('word')!r}; the "
                    f"label is not in the position any gate inspects — write "
                    f"'{prefix} {m.group('hidden')}' or the §-form"
                ),
            }
        )
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
    label_re = {
        "wire": WIRE_LABEL_RE,
        "synth": SYNTH_LABEL_RE,
        "bytesguard": BYTESGUARD_LABEL_RE,
    }.get(namespace, MESH_LABEL_RE)
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
        # synth document-naming marker ("SCE Protocol-Synthesis RFC §8 Q8"): names
        # the protocol-synthesis RFC explicitly, so a numeric label under it
        # is claimable. Ledger membership still gates either way.
        synth_doc_marked = namespace == "synth" and SYNTH_DOC_MARKER_RE.search(line_prefix)
        bguard_doc_marked = namespace == "bytesguard" and BYTESGUARD_DOC_MARKER_RE.search(
            line_prefix
        )
        reasons = []
        for lbl, sid in zip(labels, ids):
            leaf = _leaf(lbl, namespace)
            lettered = namespace == "synth" and any(ch.isupper() for ch in lbl)
            if sid not in target_ids:
                reasons.append((lbl, sid, f"id not in {namespace} ledger (non-section)"))
            elif (namespace == "synth" and not lettered and not synth_doc_marked) or (
                namespace == "bytesguard" and not bguard_doc_marked
            ):
                reasons.append(
                    (
                        lbl,
                        sid,
                        "numeric label without document-naming marker; manual review",
                    )
                )
            elif not synth_doc_marked and not bguard_doc_marked and any(
                (ns + "-" + leaf) in exclude_ids for ns in ("scxml", "mesh")
            ):
                reasons.append((lbl, sid, "ambiguous: also a sibling-ledger section; manual review"))
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


def _tracked_files():
    """Repo-relative paths git tracks, or None when that cannot be determined.

    None means "do not filter": outside a git checkout (a vendored copy, a
    release tarball) every file present is the content under review, so falling
    back to no filter is the honest default rather than silently checking
    nothing.
    """
    try:
        out = subprocess.run(
            ["git", "-C", REPO_ROOT, "ls-files"],
            capture_output=True,
            text=True,
            timeout=60,
        )
    except (OSError, subprocess.SubprocessError):
        return None
    if out.returncode != 0:
        return None
    return {line for line in out.stdout.splitlines() if line}


def load_ledger_ids(ledger_path):
    with open(ledger_path, "r", encoding="utf-8") as fh:
        store = json.load(fh)
    return set(store["sections"].keys())


def paths_from_toml(toml_path):
    """Extract the set_equality_validator `paths` array from a mnemosyne.toml.

    Returns each enrolled path resolved against REPO_ROOT (the toml's
    [workspace] root is the repo root). Reading the array here keeps the
    --check form gate in lockstep with what validate-code-refs covers: a
    dir enrolled in the toml is automatically form-gated, no second list to
    drift. Hand-rolled (no tomllib on the py3.10 CI image); the array is a
    flat list of quoted strings, with `#` comments stripped per line."""
    with open(toml_path, "r", encoding="utf-8") as fh:
        text = fh.read()
    # Anchor the table header to a line start (re.MULTILINE) so a `#`-
    # commented mention of [plugins.set_equality_validator] earlier in the
    # file is not mistaken for the real table.
    sec = re.search(
        r"^\[plugins\.set_equality_validator\](.*?)(?:^\[|\Z)",
        text,
        re.DOTALL | re.MULTILINE,
    )
    body = sec.group(1) if sec else text
    arr = re.search(r"paths\s*=\s*\[(.*?)\]", body, re.DOTALL)
    if not arr:
        raise SystemExit(f"no set_equality_validator.paths in {toml_path}")
    entries = re.sub(r"#[^\n]*", "", arr.group(1))
    return [os.path.join(REPO_ROOT, p) for p in re.findall(r'"([^"]+)"', entries)]


def iter_source_files(paths):
    for p in paths:
        if os.path.isfile(p):
            yield p
        elif os.path.isdir(p):
            for root, dirs, files in os.walk(p):
                dirs[:] = [
                    d
                    for d in dirs
                    if not d.startswith(".")
                    and d not in ("target", "node_modules", "build")
                ]
                for f in sorted(files):
                    if os.path.splitext(f)[1] in KNOWN_EXTS or f == "CMakeLists.txt":
                        yield os.path.join(root, f)


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        "paths", nargs="*", help="files or directories to migrate"
    )
    ap.add_argument(
        "--namespace",
        default="scxml",
        choices=["scxml", "mesh", "wire", "synth", "bytesguard"],
        help="scxml: W3C-marked cites (default); mesh: bare §<n> SCE_MESH.md "
        "cites; wire: bare §W<n> Wire RFC wave cites; synth: protocol-"
        'synthesis RFC cites ("RFC §5.B" marked + bare §5.J.2 / §6.2.6); '
        "bytesguard: rfc-eventschema-bytes-guard.md-marked cites",
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
    ap.add_argument(
        "--check",
        action="store_true",
        help="read-only gate: exit 1 if any enrolled comment carries a "
        "free-text 'W3C SCXML <n>' section cite (it must be §scxml- form so "
        "validate-code-refs can gate it). The spec version 1.0 is "
        "allowlisted; string literals are out of scope (comment_only).",
    )
    ap.add_argument(
        "--check-ledger",
        action="store_true",
        help="read-only gate for trees kept in PROSE: exit 1 only if a "
        "section-SHAPED label names a section absent from the ledger (a "
        "fabricated cite). Unlike --check it does not demand §-form, so it "
        "suits tools/codegen/templates/, whose comments are emitted verbatim "
        "into generated code and must stay readable to consumers.",
    )
    ap.add_argument(
        "--from-toml",
        default=None,
        help="check exactly the set_equality_validator paths enrolled in "
        "this mnemosyne.toml (keeps the form gate in lockstep with the "
        "validator's coverage instead of a hand-maintained dir list)",
    )
    args = ap.parse_args(argv)

    if args.from_toml:
        args.paths = list(args.paths) + paths_from_toml(args.from_toml)
    if not args.paths:
        ap.error("no paths given (pass paths or --from-toml)")

    ledger_path = args.ledger or _NS_DEFAULT_LEDGER.get(args.namespace, DEFAULT_LEDGER)
    ledger_ids = load_ledger_ids(ledger_path)
    # The cross-namespace guard applies where numeric labels can collide:
    # mesh excludes against the W3C scxml ledger; synth (whose §6.2.6 / §3 /
    # §7 numbers exist in BOTH siblings) excludes against scxml AND mesh.
    # wire labels (W<n>) are unique, so no sibling ledger is loaded.
    exclude_ids = set()
    if args.namespace == "mesh":
        exclude_ids = load_ledger_ids(args.exclude_ledger or DEFAULT_LEDGER)
    elif args.namespace == "synth":
        if args.exclude_ledger:
            exclude_ids = load_ledger_ids(args.exclude_ledger)
        else:
            exclude_ids = load_ledger_ids(DEFAULT_LEDGER) | load_ledger_ids(
                DEFAULT_MESH_LEDGER
            )

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

    if args.check_ledger:
        # Hallucination gate for trees that deliberately stay in PROSE.
        #
        # `--check` bundles two demands: cites must be in §<ns>- form, AND every
        # section-shaped label must exist in the ledger. The form half cannot
        # apply to `tools/codegen/templates/`: a template comment has two
        # audiences, its own documentation and the text it EMITS into generated
        # code, and the same string serves both. Forcing §-form there rewrites
        # the citations that ship inside every generated file, and it puts
        # §ids into committed generated trees that the Rust scan set excludes —
        # measured, that made Mnemosyne R840 report 7 sections as reachable only
        # from an excluded tree even though the authoring templates were scanned
        # and bound.
        #
        # This mode keeps only the half that matters for those trees: a label
        # shaped like a section but absent from the ledger is a fabricated
        # citation and fails. Prose stays prose; a wrong section number cannot
        # survive. `label_to_id` already normalises lettered forms, and the same
        # allowlist logic as `--check` exempts the spec version and bare W3C
        # test numbers.
        version_allowlist = {"1.0"}
        # Tracked content only. A citation is a claim the REPOSITORY makes, and
        # untracked files make none: the hits there are gitignored generated
        # artifacts left over from a build that predates a template fix. Scanning
        # them turns a citation gate into a build-freshness gate, failing on
        # whatever a developer happens to have on disk for a reason unrelated to
        # the commit. A fabricated number in a generated file always also exists
        # in its authoring template, which IS tracked and IS scanned — so
        # restricting scope loses no detection.
        tracked = _tracked_files()

        def in_scope(rel):
            # Only a path INSIDE this repo is subject to its tracking. `relpath`
            # yields a "../"-prefixed value for anything outside (a vendored
            # checkout, a caller-supplied directory, the tests' temp dirs), and
            # those are the content under review wherever they came from.
            if tracked is None or rel.startswith(".."):
                return True
            return rel in tracked

        def is_violation(d):
            if not in_scope(d["file"]):
                return False
            # A hidden citation is a violation whatever its label resolves to:
            # the number may be perfectly valid and still be invisible to every
            # gate, which is the defect. Ledger membership is irrelevant here.
            if d["reason"].startswith("citation hidden behind"):
                return True
            if d["reason"].startswith("quoted spec-string"):
                return False
            return d["label"] not in version_allowlist and "." in d["label"]

        bad = [d for d in report["skipped"] if is_violation(d)]
        if bad:
            print(
                "ERROR: citation(s) naming a section absent from the ledger. A "
                "section number that does not exist is a false claim about the "
                "spec, whether or not the cite is in §-form:",
                file=sys.stderr,
            )
            for d in sorted(bad, key=lambda d: (d["file"], d["line"])):
                print(
                    f"  {d['file']}:{d['line']}  {args.prefix} {d['label']} "
                    f"-> §{d['id']}: {d['reason']}",
                    file=sys.stderr,
                )
            print(
                "\nResolve each against the ledger (title/body), then correct "
                "the number. Do NOT migrate it to §-form to silence this.",
                file=sys.stderr,
            )
            return 1
        print(
            "ledger-existence check: OK — every section-shaped citation "
            "resolves to a ledger section."
        )
        return 0

    if args.check and args.namespace != "scxml":
        # Non-scxml form gate: a *claimable* free-text cite (one --apply
        # would rewrite) is the violation; skipped cites are legitimately
        # bare (sibling-namespace numbers, other documents' ids, quoted
        # runtime strings) and stay for their own ledgers' rounds.
        if report["migrations"]:
            print(
                f"ERROR: free-text {args.namespace} RFC section cite(s) in "
                f"enrolled comments. Citations must use the §{args.namespace}- "
                "form so validate-code-refs can gate them against the ledger:",
                file=sys.stderr,
            )
            for d in sorted(report["migrations"], key=lambda d: (d["file"], d["line"])):
                print(f"  {d['file']}:{d['line']}  §{d['label']} -> §{d['id']}", file=sys.stderr)
            print(
                "\nMigrate with: python3 tools/mnemosyne-adoption/"
                f"migrate_citations.py <path> --namespace {args.namespace} --apply",
                file=sys.stderr,
            )
            return 1
        print(
            f"citation-form check: OK — no claimable free-text {args.namespace} "
            "section cites in enrolled comments."
        )
        return 0

    if args.check:
        # The spec is "SCXML 1.0"; "W3C SCXML 1.0" is the version, not a
        # section reference, so it legitimately stays prose.
        version_allowlist = {"1.0"}
        # migrations  = free-text cites that map to a real section (must be
        #               §scxml- form).
        # skipped     = free-text whose id is not in the ledger. Three sub-
        #               cases, only the third is a violation:
        #                 - the version string ("1.0", allowlisted);
        #                 - a bare integer ("403") — a W3C IRP *test* number,
        #                   not a spec section (sections are 1-7 + lettered
        #                   appendices, all in the ledger), so it stays prose;
        #                 - a section-SHAPED label not in the ledger ("5.11",
        #                   "3.13.2", "G.99") — a hallucinated / wrong section
        #                   cite that must be rewritten to §scxml- so that
        #                   validate-code-refs surfaces it as section_missing.
        #               (String-literal cites carry a different reason and are
        #               out of scope — the validator is comment_only.)
        violations = list(report["migrations"])
        for d in report["skipped"]:
            # A hidden citation ("W3C SCXML Section 3.6") is a violation whatever
            # its label resolves to — the number is not in the position the
            # validator inspects, so the cite is ungated even though it is right.
            if d["reason"].startswith("citation hidden behind"):
                violations.append(d)
                continue
            if d["reason"].startswith("quoted spec-string"):
                continue
            if d["label"] in version_allowlist:
                continue
            if "." not in d["label"]:
                continue
            violations.append(d)
        if violations:
            print(
                "ERROR: free-text 'W3C SCXML <n>' section cite(s) in enrolled "
                "comments. Citations must use the §scxml- form so "
                "validate-code-refs can gate them against the ledger:",
                file=sys.stderr,
            )
            for d in sorted(violations, key=lambda d: (d["file"], d["line"])):
                print(
                    f"  {d['file']}:{d['line']}  W3C SCXML {d['label']} "
                    f"-> §{d['id']}",
                    file=sys.stderr,
                )
            print(
                "\nMigrate with: python3 "
                "tools/mnemosyne-adoption/migrate_citations.py <path> --apply",
                file=sys.stderr,
            )
            return 1
        print(
            "citation-form check: OK — no free-text 'W3C SCXML <n>' section "
            "cites in enrolled comments."
        )
        return 0

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
