#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""Revision-pinned `mnemosyne-cli` resolution for the closed-loop tests.

These tests drive the real binary, so which revision they drive decides what
they prove. Resolving it from `PATH` made them assert against "whatever is
installed": `~/.cargo/bin` is a shared slot that any `cargo install`
overwrites, so the suite silently followed an unrelated revision. That is not
hypothetical — the slot was overwritten out of band and these tests then
carried a `mnemosyne.toml` fixture declaring `output_path` / `docs` /
`default_doc`, keys Mnemosyne retired with R400. `deny_unknown_fields` rejected
them, and because the suite self-skips when no CLI is on `PATH`, CI reported
green while the fixture rotted.

`MNEMOSYNE_REV` in `.github/workflows/spec-citations.yml` is the single source
of truth for the revision (CI installs it; `pre-push` Stage 8 asserts it). This
module reads the same value, resolves the revision-keyed install path, and
verifies `--version` reports it. A missing pinned build skips — the suite
should not fail on a machine that never installed it — but a binary reporting a
different revision is never used, so a pass here is attributable to the pin.

Honour `MNEMOSYNE_BIN` to relocate the binary; it overrides the *location*,
never the revision assertion.
"""

import os
import re
import subprocess
from pathlib import Path

_REPO_ROOT = Path(__file__).resolve().parents[3]
_WORKFLOW = _REPO_ROOT / ".github" / "workflows" / "spec-citations.yml"


def pinned_rev():
    """The 40-hex `MNEMOSYNE_REV` from the workflow, or None if unreadable."""
    try:
        text = _WORKFLOW.read_text(encoding="utf-8")
    except OSError:
        return None
    m = re.search(r"^\s*MNEMOSYNE_REV:\s*([0-9a-f]{40})\b", text, re.M)
    return m.group(1) if m else None


def pinned_cli():
    """Absolute path to the pinned `mnemosyne-cli`, or None to skip.

    None means "not installed here". A binary that exists but reports another
    revision also yields None rather than being used — running it would produce
    a result the pin cannot vouch for.
    """
    rev = pinned_rev()
    if not rev:
        return None
    short = rev[:8]
    override = os.environ.get("MNEMOSYNE_BIN")
    candidate = (
        Path(override)
        if override
        else Path.home() / ".local" / "share" / "mnemosyne-rev" / short / "bin" / "mnemosyne-cli"
    )
    if not candidate.is_file() or not os.access(candidate, os.X_OK):
        return None
    try:
        out = subprocess.run(
            [str(candidate), "--version"], capture_output=True, text=True, timeout=30
        ).stdout
    except (OSError, subprocess.SubprocessError):
        return None
    return str(candidate) if short in out else None


def skip_reason():
    """unittest skip reason, naming the pin so a skip is diagnosable."""
    rev = pinned_rev()
    if not rev:
        return "MNEMOSYNE_REV not readable from .github/workflows/spec-citations.yml"
    return (
        f"pinned mnemosyne-cli ({rev[:8]}) not installed — "
        f"cargo install --git https://github.com/newmassrael/mnemosyne --rev {rev} "
        f"--locked --root ~/.local/share/mnemosyne-rev/{rev[:8]} mnemosyne-cli"
    )


MNEMOSYNE_CLI = pinned_cli()
