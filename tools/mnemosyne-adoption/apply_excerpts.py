#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""
R2 apply driver — feed an excerpts JSON (from scxml_extract_excerpts.py) into a
Mnemosyne workspace via `set-section-normative-excerpt`, one call per section.

Separated from the extractor (which is pure/deterministic/testable): this is the
side-effecting CLI driver. Each call uses --no-regenerate; GENERATED.md is
regenerated once at the end so the cascade runs a single render, not 191.

normative_excerpt is frozen after first set, so this is run once per workspace
on the skeleton; re-running on already-set sections will be rejected by the
setter (expected — surfaced, not swallowed).

Usage (run from the workspace root, where mnemosyne.toml lives):
    python3 .../apply_excerpts.py --excerpts out/scxml-excerpts.json
"""

import argparse
import json
import subprocess
import sys
import tempfile
from pathlib import Path


def main(argv=None):
    ap = argparse.ArgumentParser(description="apply normative excerpts via mnemosyne-cli")
    ap.add_argument("--excerpts", required=True)
    ap.add_argument("--cli", default="mnemosyne-cli")
    ap.add_argument("--workspace", default=".", help="workspace root (mnemosyne.toml dir)")
    args = ap.parse_args(argv)

    excerpts = json.loads(Path(args.excerpts).read_text(encoding="utf-8"))
    ok, failed = 0, []

    for section_id, e in excerpts.items():
        with tempfile.NamedTemporaryFile("w", suffix=".txt", delete=False, encoding="utf-8") as tf:
            tf.write(e["text"])
            text_path = tf.name
        try:
            proc = subprocess.run(
                [
                    args.cli, "set-section-normative-excerpt",
                    "--section", f"§{section_id}",
                    "--text-file", text_path,
                    "--anchor-url", e["anchor_url"],
                    "--source-revision", e["source_revision"],
                    "--no-regenerate",
                ],
                cwd=args.workspace, capture_output=True, text=True,
            )
        finally:
            Path(text_path).unlink(missing_ok=True)
        if proc.returncode == 0:
            ok += 1
        else:
            failed.append((section_id, proc.stderr.strip().splitlines()[-1] if proc.stderr.strip() else "?"))

    # Single final render so GENERATED.md reflects every excerpt.
    regen = subprocess.run(
        [args.cli, "generate-docs"], cwd=args.workspace, capture_output=True, text=True
    )

    sys.stderr.write(f"applied={ok} failed={len(failed)} regenerate_exit={regen.returncode}\n")
    for sid, msg in failed[:20]:
        sys.stderr.write(f"  FAIL {sid}: {msg}\n")
    return 0 if not failed and regen.returncode == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
