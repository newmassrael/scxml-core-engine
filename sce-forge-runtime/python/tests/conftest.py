# SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
#
# pytest / unittest bootstrap for the cross-language numerical conformance
# harness. Two invocations of the sce-codegen CLI:
#
#   1. `generate-conformance --language python` renders the test harness
#      itself (test_numerical_conformance.py body) from the shared template
#      tools/codegen/templates/forge/python/conformance/harness.py.jinja2
#      and the fixture catalog tests/forge/conformance/fixtures.json. The
#      rendered file lives under target/conformance_generated/python/ and is
#      imported directly from disk — no committed Python test scaffolding
#      exists on this conformance path.
#
#   2. `generate` is invoked per fixture to compile each SCXML into a Python
#      module under the same output directory. sys.path is extended so both
#      the runtime package and the generated fixtures resolve at import time.
#
# The set of fixtures to generate comes from the manifest; adding a fixture
# means adding one entry to fixtures.json and one entry to
# numerical_reference.json — never touching this file.

from __future__ import annotations

import fcntl
import shutil
import subprocess
import sys
import types
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
RUNTIME_SRC = REPO_ROOT / "sce-forge-runtime" / "python"
RESOURCE_DIR = REPO_ROOT / "tests" / "forge" / "resources"
MANIFEST = REPO_ROOT / "tests" / "forge" / "conformance" / "fixtures.json"
SCE_CODEGEN = REPO_ROOT / "target" / "release" / "sce-codegen"

# Synthetic package name under which generated fixtures and the harness are
# loaded. Cross-file Forge fixtures emit canonical relative imports
# (`from . import transform_temperature`), which only resolve when the
# importing module belongs to a parent package. The output directory itself
# (`target/conformance_generated/python/`) is named for human navigation, not
# as a Python identifier, so we install a synthetic in-memory package whose
# `__path__` points at it. The leading underscore documents that the package
# is internal to the conformance harness and not part of any public API.
CONFORMANCE_PACKAGE = "_sce_conformance"


def _ensure_codegen() -> None:
    """Rebuild sce-codegen from the current sce-build sources when the Rust
    toolchain is available (local dev). Cargo's incremental build makes this
    a near-instant no-op when nothing has changed; the value is eliminating
    the stale-binary foot-gun where a schema change in conformance.rs would
    otherwise be silently ignored by Python tests calling the old binary.

    In CI, the Python job downloads a pre-built artifact from the build-
    codegen job and has no Rust toolchain — shutil.which returns None and
    we fall through to the binary-exists check below."""
    if shutil.which("cargo") is not None:
        subprocess.run(
            [
                "cargo",
                "build",
                "--bin",
                "sce-codegen",
                "--features",
                "cli",
                "--release",
                "-p",
                "sce-build",
            ],
            cwd=str(REPO_ROOT),
            check=True,
        )
    if not SCE_CODEGEN.exists():
        raise RuntimeError(
            f"sce-codegen binary not found at {SCE_CODEGEN}. "
            "Build it first: "
            "`cargo build --bin sce-codegen --features cli --release -p sce-build`"
        )


def _load_fixture_names() -> list[str]:
    """Pull the fixture list from sce-codegen itself so this script never
    has to know the manifest schema. The Rust binary owns the schema (see
    sce-build/src/conformance.rs)."""
    result = subprocess.run(
        [str(SCE_CODEGEN), "list-fixtures", "--manifest", str(MANIFEST), "--format", "plain"],
        check=True,
        capture_output=True,
        text=True,
    )
    return [line.strip() for line in result.stdout.splitlines() if line.strip()]


def _generate_fixtures(out_dir: Path) -> None:
    out_dir.mkdir(parents=True, exist_ok=True)
    for fixture in _load_fixture_names():
        scxml = RESOURCE_DIR / f"{fixture}.scxml"
        subprocess.run(
            [
                str(SCE_CODEGEN),
                "generate",
                str(scxml),
                "--language",
                "python",
                "--output-dir",
                str(out_dir),
            ],
            check=True,
            capture_output=True,
            text=True,
        )


def _generate_harness(out_dir: Path) -> None:
    """Render the test_numerical_conformance.py body from the shared template."""
    subprocess.run(
        [
            str(SCE_CODEGEN),
            "generate-conformance",
            "--language",
            "python",
            "--manifest",
            str(MANIFEST),
            "--output-dir",
            str(out_dir),
        ],
        check=True,
        capture_output=True,
        text=True,
    )


def _conformance_out_dir() -> Path:
    return REPO_ROOT / "target" / "conformance_generated" / "python"


def pytest_configure(config):  # pragma: no cover - pytest hook
    bootstrap()


def bootstrap() -> Path:
    """Generate fixtures + harness, race-safe under pytest-xdist.

    Multiple xdist workers import this conftest concurrently and would
    otherwise race on the shared output directory — sce-codegen writes
    multiple files in sequence, so a worker can observe truncated output
    while another is still writing. We serialize bootstraps with an
    advisory POSIX file lock; the second-and-later workers block until
    the first finishes, then re-enter and find every output already in
    place (sce-codegen is idempotent for unchanged inputs)."""
    _ensure_codegen()
    out_dir = _conformance_out_dir()
    out_dir.mkdir(parents=True, exist_ok=True)
    lock_path = out_dir / ".bootstrap.lock"
    with open(lock_path, "w") as lock_file:
        fcntl.flock(lock_file.fileno(), fcntl.LOCK_EX)
        try:
            _generate_fixtures(out_dir)
            _generate_harness(out_dir)
        finally:
            fcntl.flock(lock_file.fileno(), fcntl.LOCK_UN)
    # Runtime first so fixtures can resolve `from sce_forge_runtime...` imports.
    if str(RUNTIME_SRC) not in sys.path:
        sys.path.insert(0, str(RUNTIME_SRC))
    # Install a synthetic parent package whose submodule search path is the
    # generated output directory. With this in place, every fixture file is
    # importable as `_sce_conformance.<name>` and the canonical relative
    # imports the Forge codegen emits (`from . import transform_temperature`)
    # resolve through this parent. We deliberately avoid adding `out_dir`
    # itself to `sys.path` — top-level imports of fixture names would mask
    # the package boundary and make the relative imports inert.
    if CONFORMANCE_PACKAGE not in sys.modules:
        pkg = types.ModuleType(CONFORMANCE_PACKAGE)
        pkg.__path__ = [str(out_dir)]
        sys.modules[CONFORMANCE_PACKAGE] = pkg
    return out_dir


bootstrap()
