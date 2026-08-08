# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Single source of truth for locating the sce-codegen binary from the
# Python harnesses.
#
# Search order is debug first, release second — the same order
# scripts/lib/sce_codegen.sh, cmake/SCEFindCodegen.cmake and the Gradle
# builds use. Debug leads because that is the profile every build path
# in this repository now produces: the generator's cost is process
# start-up and I/O rather than optimisation, so a release build only
# compiles the dependency tree a second time instead of sharing the one
# clippy and the test suite already produced. Release stays in the
# search path so a tree still holding an older release build keeps
# working, and it is looked at second so a stale binary cannot outrank
# a fresh one.
#
# Consumers call require() instead of naming a profile, because naming
# one is what broke: the profile was spelled out independently at ~100
# sites across five languages, and moving it moved only some of them —
# the conformance jobs then looked for a release binary CI no longer
# produced. `codegen_binary_resolution.rs` fails if a profile-specific
# path reappears outside the four ecosystem locators.

import shutil
import subprocess
from pathlib import Path

# .../backends/python/forge-runtime/tests/_sce_codegen.py → repo root
REPO_ROOT = Path(__file__).resolve().parents[4]

BUILD_COMMAND = [
    "cargo",
    "build",
    "--bin",
    "sce-codegen",
    "--features",
    "cli",
    "-p",
    "sce-build",
]


def find() -> Path | None:
    """Return an existing sce-codegen binary, or None."""
    for profile in ("debug", "release"):
        candidate = REPO_ROOT / "target" / profile / "sce-codegen"
        if candidate.exists():
            return candidate
    return None


def require() -> Path:
    """Return the sce-codegen binary, building it when no profile holds
    one.

    Rebuilding from the current sce-build sources whenever the Rust
    toolchain is available (local dev) is what keeps a schema change in
    conformance.rs from being silently ignored by Python tests calling
    an old binary; cargo's incremental build makes it a near-instant
    no-op when nothing moved. In CI the Python job downloads a
    pre-built artifact and has no Rust toolchain, so shutil.which
    returns None and the located binary is used as-is.

    A missing binary is an error rather than a skip: it is a build
    product this function can produce, so skipping would report a green
    run for a harness that never executed.
    """
    if shutil.which("cargo") is not None:
        subprocess.run(BUILD_COMMAND, cwd=str(REPO_ROOT), check=True)
    binary = find()
    if binary is None:
        raise RuntimeError(
            f"sce-codegen binary not found under {REPO_ROOT}/target/"
            "{debug,release}. Build it first: "
            "`cargo build --bin sce-codegen --features cli -p sce-build`"
        )
    return binary
