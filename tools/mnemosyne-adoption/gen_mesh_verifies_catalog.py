#!/usr/bin/env python3
"""Generate the verifies-catalog JSON for the mesh ledger from test declarations.

Mnemosyne's verify axis validates that a `verifies` binding *exists*; the catalog
is the deterministic source it validates that binding's *correctness* against
(`[verifies_catalog]` + `validate-verifies-linkage`). The value of the axis comes
entirely from the catalog and the binding being two independent artifacts: a
binding that claims a test covers a section the test itself does not claim is
what the linkage gate is for. A catalog hand-authored alongside the bindings
would check nothing.

SSOT: each mesh test declares what it verifies, in its own file header:

    // SCE-VERIFIES: mesh-10.5 mesh-10.6

The sibling scxml generator reads W3C's `resources/<id>/metadata.txt specnum`
field — a declaration the SCE tree does not author. Mesh tests have no external
metadata, so the declaration lives with the test, travels with it, and is
reviewed in the same diff that changes what the test covers.

Granularity is the FILE, not the symbol, and bindings must match (the "binding
granularity <= SSOT granularity" rule the scxml generator established). A mesh
test file is one scenario driven end to end — `test_mesh_session_f_eventful.cpp`
is the wire-16 + `<finalize>` lifecycle, `DedupRouterTest.cpp` is the dedup
window — so the file is the unit that verifies something. Declaring per symbol
would claim a precision the tests are not organised around.

The marker deliberately writes `mesh-10.5`, NOT `§mesh-10.5`. A `§` token in a
scanned tree is a citation and would demand its own paired binding through the
code-refs gate; this line is a test's declaration of intent, not a claim that
the code beside it implements the section.

Contract emitted (Mnemosyne `verifies-catalog/v1`, deserialized by
mnemosyne-validate::verifies_linkage::VerifiesCatalog — field name `section_ids`
is load-bearing, `symbol` optional, extra keys ignored):
    {
      "format": "verifies-catalog/v1",
      "entries": [
        {"file": "tests/mesh/DedupRouterTest.cpp",
         "section_ids": ["mesh-10.5"]},
        ...
      ]
    }

A binding (file -> section) is catalog-valid iff `section` is EXACTLY one of the
listed section_ids for that file.

Deterministic and standard-library only.
"""
import argparse
import hashlib
import json
import os
import re
import sys
from glob import glob

REPO_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))

# Test trees the mesh ledger draws verification evidence from. Kept explicit
# rather than globbed from `paths` in mnemosyne.toml: `paths` is the set of
# files whose CITATIONS are gated, which is a different question from which
# files may serve as evidence, and conflating them would silently enrol a
# production tree as its own witness.
TEST_TREES = (
    "tests/mesh/*.cpp",
    "tests/mesh/*.h",
    "sce-build/tests/mesh_*.rs",
    "tests/w3c/dist/*.cpp",
)

MARKER_RE = re.compile(r"^\s*(?://|#)\s*SCE-VERIFIES:\s*(.+?)\s*$")
SECTION_RE = re.compile(r"^mesh-[0-9][0-9A-Za-z.]*$")


def known_sections(repo_root: str) -> set:
    """Section ids present in the mesh ledger store.

    A marker naming a section that does not exist is the same hallucination
    class `severity_missing` rejects for citations, and it must fail here
    rather than produce a catalog entry no binding can ever match.
    """
    store = os.path.join(
        repo_root, "docs", "sce-ledger", "mesh", ".atomic", "workspace.atomic.json")
    with open(store, encoding="utf-8") as fh:
        return set(json.load(fh).get("sections", {}))


def scan_file(path: str, repo_root: str, valid: set):
    """Return (relative_path, [section_ids]) or None when unmarked."""
    rel = os.path.relpath(path, repo_root).replace(os.sep, "/")
    sections = []
    errors = []
    with open(path, encoding="utf-8", errors="replace") as fh:
        for lineno, line in enumerate(fh, 1):
            m = MARKER_RE.match(line)
            if not m:
                continue
            for token in m.group(1).replace(",", " ").split():
                if not SECTION_RE.match(token):
                    errors.append(
                        f"{rel}:{lineno}: '{token}' is not a mesh-<n> section id")
                elif token not in valid:
                    errors.append(
                        f"{rel}:{lineno}: '{token}' is not a section in the mesh ledger")
                elif token in sections:
                    errors.append(f"{rel}:{lineno}: '{token}' declared twice")
                else:
                    sections.append(token)
    if errors:
        return rel, None, errors
    if not sections:
        return rel, None, []
    return rel, sorted(sections), []


def build_entries(repo_root: str):
    valid = known_sections(repo_root)
    entries = []
    errors = []
    for pattern in TEST_TREES:
        for path in sorted(glob(os.path.join(repo_root, pattern))):
            rel, sections, errs = scan_file(path, repo_root, valid)
            errors.extend(errs)
            if sections:
                entries.append({"file": rel, "section_ids": sections})
    entries.sort(key=lambda e: e["file"])
    return entries, errors


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--repo-root", default=REPO_ROOT)
    ap.add_argument("--out", default=os.path.join(
        REPO_ROOT, "docs", "sce-ledger", "mesh", ".atomic", "verifies-catalog.json"))
    ap.add_argument("--check", action="store_true",
                    help="exit non-zero if the on-disk catalog is stale")
    args = ap.parse_args()

    entries, errors = build_entries(args.repo_root)
    if errors:
        for e in errors:
            print(f"SCE-VERIFIES error: {e}", file=sys.stderr)
        return 1

    catalog = {"format": "verifies-catalog/v1", "entries": entries}
    rendered = json.dumps(catalog, indent=2, ensure_ascii=False) + "\n"
    sha256 = hashlib.sha256(rendered.encode("utf-8")).hexdigest()

    if args.check:
        if not os.path.exists(args.out):
            print(f"catalog missing: {args.out}", file=sys.stderr)
            return 1
        with open(args.out, encoding="utf-8") as fh:
            if fh.read() != rendered:
                print(f"catalog STALE: {args.out} — regenerate with "
                      f"tools/mnemosyne-adoption/gen_mesh_verifies_catalog.py",
                      file=sys.stderr)
                return 1
        # Surface the hash so a reviewer can confirm/update the
        # [verifies_catalog].sha256 pin in mnemosyne.toml in one motion.
        print(f"catalog up to date: {len(entries)} entries; sha256={sha256}")
        return 0

    os.makedirs(os.path.dirname(args.out), exist_ok=True)
    with open(args.out, "w", encoding="utf-8") as fh:
        fh.write(rendered)
    print(f"wrote {args.out}: {len(entries)} entries")
    # Re-pin in one motion: paste this into [verifies_catalog].sha256.
    print(f"sha256={sha256}  -> update [verifies_catalog].sha256 in mnemosyne.toml")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
