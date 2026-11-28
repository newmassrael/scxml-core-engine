# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `license-ssot`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# A compliance verdict — an AGPL-3.0 section 4 / MIT section 1 violation in a
# release tarball — that was reachable only after a push, on a check that
# needs nothing but bash and takes three hundredths of a second.
{
    "workflows": ["license-verify.yml"],
    "runner_workflow": True,
    # 2s where the landing queue judges it: the median of its last ten
    # landing runs in the integrator's timings.tsv (2026-11-28), the seat the
    # budget bounds ("The push budget" in gate_registry.py). It read
    # 0s warm on a developer's tree.
    "cost_s": 2,
    "summary": "license SSOT vs the tree it describes",
    # `package_embed.sh` writes `git describe` into the package's VERSION
    # file, which the licence check does not read.
    "cache": True,
    # Read by `check learn` (2026-11-24, bx 340e9f9e): the SSOT, the
    # licence texts it names, and its checker. The checker lists
    # `third_party/` and each dependency's directory and looks at every
    # file at depth two (`find -maxdepth 2 -type f`) to find a dependency
    # the SSOT does not name, which is the violation this gate exists for:
    # `third_party/` and `third_party/*/` are those listings and
    # `third_party/*/*` those files. Two notices sit deeper and are opened
    # by name (lua.h, json.hpp). `LICENSE*` covers the root's texts.
    "inputs": [
        "LICENSE*",
        "sce/sce_licenses.cmake",
        "scripts/verify_licenses.sh",
        "third_party/",
        "third_party/*/",
        "third_party/*/*",
        "third_party/lua/src/lua.h",
        "third_party/nlohmann_json/include/nlohmann/json.hpp",
    ],
    "commits": [],
    "peak_gb": 1,
    "parallelism": PARALLELISM_SCRIPT,
}
