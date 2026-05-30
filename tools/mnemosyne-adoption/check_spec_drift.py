#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""
B1 — drift check for the vendored W3C SCXML spec snapshot.

Two independent modes, both keyed off spec-snapshot/PROVENANCE.json (the SSOT
for the snapshot's identity):

  integrity (offline, deterministic)
      The vendored HTML file's sha256 must equal PROVENANCE.fetched_sha256.
      Catches a snapshot edited without its provenance updated (or vice
      versa). Safe to run as a normal push/PR gate — no network.

  upstream (online)
      Re-fetch PROVENANCE.url, recompute sha256, compare against
      PROVENANCE.fetched_sha256. A mismatch means the upstream Recommendation
      changed byte-for-byte since the snapshot was taken. Network-dependent, so
      it runs only on schedule / manual dispatch, never as a build gate.

The recomputed digest is emitted in PROVENANCE's locked format (^[0-9a-f]{64}$)
so it can be handed to Mnemosyne's B2 rev-diff scan without reformatting.

Exit codes: 0 = in sync, 1 = drift detected, 2 = infrastructure error
(malformed provenance, missing file, fetch failure). The 1-vs-2 split lets CI
distinguish a real spec change from a transient network/setup problem.

Usage:
    python3 tools/mnemosyne-adoption/check_spec_drift.py --mode integrity
    python3 tools/mnemosyne-adoption/check_spec_drift.py --mode upstream
"""

import argparse
import hashlib
import json
import re
import sys
import urllib.error
import urllib.request
from pathlib import Path

SHA256_RE = re.compile(r"^[0-9a-f]{64}$")

EXIT_OK = 0
EXIT_DRIFT = 1
EXIT_INFRA = 2


class ProvenanceError(Exception):
    """Malformed or unusable PROVENANCE.json (infrastructure error)."""


def sha256_hex(data):
    return hashlib.sha256(data).hexdigest()


def load_provenance(provenance_path):
    """Return (url, expected_sha256, snapshot_path) or raise ProvenanceError."""
    try:
        doc = json.loads(Path(provenance_path).read_text(encoding="utf-8"))
    except (OSError, ValueError) as exc:
        raise ProvenanceError(f"cannot read {provenance_path}: {exc}") from exc

    expected = doc.get("fetched_sha256")
    if not isinstance(expected, str) or not SHA256_RE.match(expected):
        raise ProvenanceError(
            f"fetched_sha256 must match ^[0-9a-f]{{64}}$, got {expected!r}"
        )
    url = doc.get("url")
    if not isinstance(url, str) or not url.startswith(("http://", "https://")):
        raise ProvenanceError(f"url must be absolute http(s), got {url!r}")
    file_rel = doc.get("file")
    if not isinstance(file_rel, str) or not file_rel:
        raise ProvenanceError(f"file must be a non-empty path, got {file_rel!r}")

    snapshot_path = Path(provenance_path).resolve().parent / file_rel
    return url, expected, snapshot_path


def compare(expected_sha256, actual_bytes):
    """Pure comparison: (in_sync: bool, actual_sha256: str)."""
    actual = sha256_hex(actual_bytes)
    return actual == expected_sha256, actual


def verify_integrity(provenance_path):
    url, expected, snapshot_path = load_provenance(provenance_path)
    try:
        data = snapshot_path.read_bytes()
    except OSError as exc:
        return EXIT_INFRA, f"snapshot unreadable: {exc}"
    in_sync, actual = compare(expected, data)
    if in_sync:
        return EXIT_OK, f"integrity OK: {snapshot_path.name} matches provenance ({actual})"
    return EXIT_DRIFT, (
        f"integrity DRIFT: {snapshot_path.name} sha256={actual} "
        f"but PROVENANCE.fetched_sha256={expected} — snapshot and provenance "
        f"are out of sync"
    )


def fetch(url, timeout):
    req = urllib.request.Request(url, headers={"User-Agent": "sce-spec-drift/1.0"})
    with urllib.request.urlopen(req, timeout=timeout) as resp:  # noqa: S310 (fixed https spec URL)
        return resp.read()


def check_upstream(provenance_path, timeout):
    url, expected, _ = load_provenance(provenance_path)
    try:
        data = fetch(url, timeout)
    except (urllib.error.URLError, OSError) as exc:
        return EXIT_INFRA, f"upstream fetch failed ({url}): {exc}"
    in_sync, actual = compare(expected, data)
    if in_sync:
        return EXIT_OK, f"upstream in sync: {url} sha256={actual}"
    return EXIT_DRIFT, (
        f"upstream DRIFT: {url} now sha256={actual} but snapshot is "
        f"{expected} — the W3C Recommendation changed; refresh the snapshot, "
        f"update PROVENANCE, and re-run the A1 converter"
    )


def main(argv=None):
    here = Path(__file__).resolve().parent
    ap = argparse.ArgumentParser(description="W3C SCXML spec snapshot drift check")
    ap.add_argument("--mode", choices=["integrity", "upstream"], default="integrity")
    ap.add_argument(
        "--provenance", default=str(here / "spec-snapshot" / "PROVENANCE.json")
    )
    ap.add_argument("--timeout", type=float, default=30.0)
    args = ap.parse_args(argv)

    try:
        if args.mode == "integrity":
            code, msg = verify_integrity(args.provenance)
        else:
            code, msg = check_upstream(args.provenance, args.timeout)
    except ProvenanceError as exc:
        code, msg = EXIT_INFRA, f"provenance error: {exc}"

    stream = sys.stdout if code == EXIT_OK else sys.stderr
    stream.write(msg + "\n")
    return code


if __name__ == "__main__":
    sys.exit(main())
