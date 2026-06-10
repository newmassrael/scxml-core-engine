#!/usr/bin/env python3
"""Generate the verifies-catalog JSON for the scxml ledger from W3C test metadata.

Mnemosyne's verify-axis validates that a `verifies` binding *exists*; the
catalog is the deterministic source it validates *correctness* against (Mnemosyne
P2, `[verifies_catalog]` + `validate-verifies-linkage`). Per the boundary set in
the field-report response, the metadata-format parser is consumer-domain logic
and lives here, not in Mnemosyne core; Mnemosyne consumes the neutral JSON
contract this emits.

SSOT: each W3C conformance test ships `resources/<id>/metadata.txt` with a
`specnum` field — the spec section the test author declared it targets. That
section granularity is the authoritative granularity; the catalog binds at it and
no finer (the "binding granularity <= SSOT granularity" rule this episode
established).

Contract emitted (Mnemosyne `verifies-catalog/v1`, deserialized by
mnemosyne-validate::verifies_linkage::VerifiesCatalog — field name `section_ids`
is load-bearing, `symbol` optional, extra keys ignored):
    {
      "format": "verifies-catalog/v1",
      "entries": [
        {"file": "tests/w3c/aot_tests/Test387.h", "symbol": "Test387",
         "section_ids": ["scxml-3.10"]},
        ...
      ]
    }

A binding (file, symbol -> section) is catalog-valid iff `section` is EXACTLY one
of the listed section_ids for that (file, symbol). Section-granular: a test
targeting specnum 6.4 lists "scxml-6.4"; a sub-section binding (6.4.1 ...) is the
`FinerThanDeclared` granularity lint, since the metadata does not assert
sub-section precision.

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


def specnum_to_section(specnum: str) -> str:
    """W3C specnum (e.g. '6.4', 'B.2', 'C.1') -> ledger section id.

    Numeric labels keep their dots (3.12 -> scxml-3.12); lettered/appendix
    labels turn dots into hyphens (B.2 -> scxml-B-2). This is the same
    normalization the A1 converter (scxml_toc_to_manifest.label_to_leaf) owns;
    kept in lockstep so the catalog and the section ids cannot drift.
    """
    sn = specnum.strip()
    if sn[:1].isalpha():
        return "scxml-" + sn.replace(".", "-")
    return "scxml-" + sn


def parse_metadata(path: str):
    fields = dict(re.findall(r"^(\w+):\s*(.*)$", open(path).read(), re.M))
    return fields.get("id"), fields.get("specnum")


def build_entries(repo_root: str):
    entries = []
    skipped = []
    for mp in sorted(glob(os.path.join(repo_root, "resources", "*", "metadata.txt"))):
        tid, specnum = parse_metadata(mp)
        if not tid or not specnum:
            continue
        header = f"tests/w3c/aot_tests/Test{tid}.h"
        if not os.path.exists(os.path.join(repo_root, header)):
            skipped.append((tid, "no aot header"))
            continue
        entries.append({
            "file": header,
            "symbol": f"Test{tid}",
            "section_ids": [specnum_to_section(specnum.strip())],
        })
    entries.sort(key=lambda e: int(e["symbol"][4:]))
    return entries, skipped


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--repo-root", default=REPO_ROOT)
    ap.add_argument("--out", default=os.path.join(
        REPO_ROOT, "docs", "spec", "scxml", ".atomic", "verifies-catalog.json"))
    ap.add_argument("--check", action="store_true",
                    help="exit non-zero if the on-disk catalog is stale")
    args = ap.parse_args()

    entries, skipped = build_entries(args.repo_root)
    catalog = {"format": "verifies-catalog/v1", "entries": entries}
    rendered = json.dumps(catalog, indent=2, ensure_ascii=False) + "\n"
    sha256 = hashlib.sha256(rendered.encode("utf-8")).hexdigest()

    if args.check:
        if not os.path.exists(args.out):
            print(f"catalog missing: {args.out}", file=sys.stderr)
            return 1
        if open(args.out).read() != rendered:
            print(f"catalog STALE: {args.out} — regenerate with "
                  f"tools/mnemosyne-adoption/gen_verifies_catalog.py", file=sys.stderr)
            return 1
        # Surface the hash so a reviewer can confirm/update the
        # [verifies_catalog].sha256 pin in mnemosyne.toml in one motion.
        print(f"catalog up to date: {len(entries)} entries; sha256={sha256}")
        return 0

    os.makedirs(os.path.dirname(args.out), exist_ok=True)
    open(args.out, "w").write(rendered)
    print(f"wrote {args.out}: {len(entries)} entries"
          + (f" ({len(skipped)} tests skipped: no aot header)" if skipped else ""))
    # Re-pin in one motion: paste this into [verifies_catalog].sha256.
    print(f"sha256={sha256}  -> update [verifies_catalog].sha256 in mnemosyne.toml")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
