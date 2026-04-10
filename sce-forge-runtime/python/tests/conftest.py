# SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
#
# pytest / unittest bootstrap for the cross-language numerical conformance
# harness. Invokes the sce-codegen CLI on the fixture SCXML files in
# tests/forge/resources/ and writes the generated Python modules into a
# per-session temp directory, which is then inserted into sys.path.
#
# This mirrors the Rust build.rs pattern: codegen is invoked at test-setup
# time from the SCXML source of truth, so there are no committed Python
# goldens on the conformance path. The runtime package directory is also
# added to sys.path so the generated fixtures can resolve their
# `from sce_forge_runtime.<kind> import ...` imports.

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
RUNTIME_SRC = REPO_ROOT / "sce-forge-runtime" / "python"
RESOURCE_DIR = REPO_ROOT / "tests" / "forge" / "resources"
SCE_CODEGEN = REPO_ROOT / "target" / "release" / "sce-codegen"

FIXTURES = (
    "filter_moving_average",
    "filter_debounce",
    "interpolation_1d_linear",
    "interpolation_2d_bilinear",
    "observer_coolant",
    "transform_temperature",
    "transform_bitwise",
    "transform_multi_output",
    "condition_threshold",
    "condition_range",
    "condition_programming",
    "procedure_linear",
    "procedure_diamond",
    "procedure_startup_check",
)


def _ensure_codegen() -> None:
    if not SCE_CODEGEN.exists():
        raise RuntimeError(
            f"sce-codegen binary not found at {SCE_CODEGEN}. "
            "Build it first: "
            "`cargo build --bin sce-codegen --features cli --release -p sce-build`"
        )


def _generate_fixtures(out_dir: Path) -> None:
    out_dir.mkdir(parents=True, exist_ok=True)
    for fixture in FIXTURES:
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


def _conformance_out_dir() -> Path:
    # Keep the generated fixtures under the cargo target directory so every
    # language's conformance harness shares one convention for build outputs.
    # pytest tmp_path_factory would also work but would require pytest; we
    # keep the test runnable under stdlib unittest, so a stable path is used.
    out = REPO_ROOT / "target" / "conformance_generated" / "python"
    return out


def pytest_configure(config):  # pragma: no cover - pytest hook
    bootstrap()


def bootstrap() -> Path:
    _ensure_codegen()
    out_dir = _conformance_out_dir()
    _generate_fixtures(out_dir)
    # Runtime first so fixtures can resolve `from sce_forge_runtime...` imports,
    # then the generated directory so `import filter_moving_average` works.
    if str(RUNTIME_SRC) not in sys.path:
        sys.path.insert(0, str(RUNTIME_SRC))
    if str(out_dir) not in sys.path:
        sys.path.insert(0, str(out_dir))
    return out_dir


# Allow `python -m unittest` invocation to work without pytest by running
# bootstrap at import time. pytest will also trigger bootstrap via
# pytest_configure above, but importing conftest is idempotent.
bootstrap()
