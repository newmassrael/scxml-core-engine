# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
#
# Cross-language numerical conformance harness entry point (Python half).
#
# The actual test class is generated at test-setup time by conftest.py
# (which runs `sce-codegen generate-conformance --language python`) from
# tools/codegen/templates/forge/python/conformance/harness.py.jinja2 and
# the fixture catalog at tests/forge/conformance/fixtures.json. The
# rendered module lands at target/conformance_generated/python/
# conformance_generated.py; this file is a tiny shim so unittest discovery
# finds the test class under this expected name.

import conftest  # noqa: F401 — runs bootstrap() at import time

# Loaded as a submodule of the synthetic conftest.CONFORMANCE_PACKAGE so its
# own relative-import line resolves the way the Forge codegen expects.
# RFC §5.B B2-test-vector: switched from a single-name import to
# `import *` so per-fixture `<Pascal>TestVectors(unittest.TestCase)`
# classes — re-exported from the harness module — flow through to
# pytest discovery alongside `TestNumericalConformance`. The harness
# module's `__name__` namespace is the auto-derived single source of
# truth; new fixtures with `<sce:test-vector>` rows surface here
# without any per-fixture edit.
from _sce_conformance.conformance_generated import *  # noqa: F401, F403
