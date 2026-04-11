# SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
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
from _sce_conformance.conformance_generated import TestNumericalConformance  # noqa: F401
