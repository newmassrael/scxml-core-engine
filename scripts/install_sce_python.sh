#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Install the Python environment this repository's Python suites run in.
#
# A virtual environment holding the Python runtime's dependencies, read from
# `backends/python/runtime/pyproject.toml` (the one place they are pinned), and
# the packages the suites themselves import (SUITE_PACKAGES below).
#
# WHY AN ENVIRONMENT OF ITS OWN (2026-11-08). The runtime imports `lupa.lua54`,
# which lupa ships from 2.0 on (`sce_runtime/scripting/lua_engine.py`). A
# distribution's python3-lupa is older: the build machines carry Ubuntu's
# 1.14.1, which has no `lua54`, so under the system python3 every test that
# imports the runtime failed at collection, after a precondition that asked only
# for `import lupa` had passed. The environment makes the Lua binding a property
# of the repository instead of the machine.
#
# The unit is the REVISION: `SCE_PYTHON_REV` below is the environment's version,
# and the install path derives from it. bx decides that a machine has the tool
# by running that path, not by comparing what built it, so an environment whose
# contents changed at the same path would never be built again. Raise the
# revision with every change to what the environment holds: the runtime's
# dependencies in pyproject.toml, or SUITE_PACKAGES. A lupa floor raised in
# pyproject.toml without it is caught by the gates, which read the same floor
# (`sce_gate_python_requires_lupa` in scripts/gates/lib.sh).
#
# NOT THE RUNTIME ITSELF. The `sce_runtime` package is not installed here: the
# suites reach the tree's copy (PYTHONPATH, or the conftest of an emitted suite),
# and a copy in site-packages would answer in its place. `w3c-python` runs an
# emitted suite with no PYTHONPATH to show that its conftest finds the runtime by
# itself, and an installed copy would let that run pass without it.
#
# What it needs and does not install: python3 with its venv module (Debian and
# Ubuntu: python3-venv), and access to PyPI.
#
# The build machines get it from `[platform.linux.tools.sce-python]` in bx.toml,
# which runs this script with SCE_PYTHON_ROOT set to bx's store entry
# (`bx-provision <alias> --apply` builds it where it is missing). The gates that
# run the suites (`sce_gate_python` in scripts/gates/lib.sh) look for the
# environment's python at the path derived here, and run on `python3` where it
# is not there.
set -euo pipefail

# The environment's revision; see the header.
SCE_PYTHON_REV="1"

# What the suites import besides the runtime's dependencies. These are not in
# pyproject.toml because that file is the runtime package's metadata, what
# `pip install backends/python/runtime` gives a user of the package, and these
# are the repository's: pytest runs the AOT suites (6.0 or later for the
# `--no-header` the gate passes), PyYAML and jsonschema read a pack in
# tools/authoring and tools/targets, and pypdfium2 is the authoring core's PDF
# text pack, whose cases skip without it. Floors only where a suite needs one;
# the environment keeps what was current when it was built until the revision
# moves.
SUITE_PACKAGES=("pytest>=6.0" "pyyaml" "jsonschema" "pypdfium2")

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PYPROJECT="$REPO_ROOT/backends/python/runtime/pyproject.toml"
SCE_PYTHON_ROOT="${SCE_PYTHON_ROOT:-$HOME/.local/share/sce-python/$SCE_PYTHON_REV}"
PYTHON="$SCE_PYTHON_ROOT/bin/python"

# No cache under the home: in bx's store everything an install leaves outside
# its prefix is to go, and a cache would be the one thing that stayed.
PIP_INSTALL=("$PYTHON" -m pip install --no-cache-dir --disable-pip-version-check)

echo "building sce-python $SCE_PYTHON_REV in $SCE_PYTHON_ROOT"
python3 -m venv "$SCE_PYTHON_ROOT"

# `tomllib` joined the standard library in Python 3.11; an older Python (Ubuntu
# 22.04 has 3.10) reads the file with tomli, the same parser before it joined.
# On 3.11 and later the marker installs nothing.
"${PIP_INSTALL[@]}" 'tomli>=1.1; python_version < "3.11"'
RUNTIME_DEPENDENCIES="$("$PYTHON" - "$PYPROJECT" <<'PY'
import sys

if sys.version_info >= (3, 11):
    import tomllib
else:
    import tomli as tomllib

with open(sys.argv[1], "rb") as f:
    for requirement in tomllib.load(f)["project"]["dependencies"]:
        print(requirement)
PY
)"
if [[ -z "$RUNTIME_DEPENDENCIES" ]]; then
    echo "install_sce_python: $PYPROJECT declares no dependencies, and the runtime needs lupa" >&2
    exit 1
fi
mapfile -t RUNTIME_REQUIREMENTS <<< "$RUNTIME_DEPENDENCIES"

"${PIP_INSTALL[@]}" "${RUNTIME_REQUIREMENTS[@]}" "${SUITE_PACKAGES[@]}"

# The postcondition, asserted rather than assumed: `lupa.lua54` is the module the
# runtime imports, and a pip that resolved some other lupa would leave the
# suites where they started.
"$PYTHON" -c 'import lupa.lua54, pytest, yaml, jsonschema, pypdfium2'
"$PYTHON" --version
echo "sce-python $SCE_PYTHON_REV installed at $PYTHON"
