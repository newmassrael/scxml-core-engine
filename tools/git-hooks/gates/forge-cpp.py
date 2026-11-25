# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `forge-cpp`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
{
    "workflows": ["forge-conformance.yml"],
    # Two reasons were recorded here for keeping a second spelling and
    # neither survived. The log artifact does not need the lane's own
    # commands — `scripts/gate forge-cpp | tee` keeps it. The build
    # configuration was real: the lane built RelWithDebInfo under Ninja
    # while the gate left CMAKE_BUILD_TYPE unset, so a local pass said
    # nothing about the binary CI judged. The gate now carries the build
    # type (and Ninja where installed), which cost 6s -> 22s and is what
    # the mirror is for.
    "runner_workflow": True,
    # The modules its projects declare every generation through, resolve
    # the generator through, and format generated C++ by, and the one that
    # says which warning classes its test programs turn off.
    #
    # And the JSON library its test programs include, vendored in the tree
    # (backends/cpp/forge-runtime/tests/conformance/CMakeLists.txt,
    # NLOHMANN_JSON_INCLUDE_DIR): `bx --checks prepush --learn forge-cpp`
    # (2026-11-24) read every header of it, and no trigger named it.
    "extra": ["backends/cpp/**", "sce/**", "cmake/SCECodegenPlan.cmake",
              "cmake/SCEFindCodegen.cmake", "cmake/SCEClangFormat.cmake",
              "cmake/SCETestWarnings.cmake",
              "third_party/nlohmann_json/include/**"],
    "deps": ["codegen-build"],
    "ci_only": "59s, the last move needed to fit the ceiling. "
               "forge-conformance.yml runs it beside the other arms.",
    "cost_s": 59,
    "summary": "C++ forge conformance build + test",
    "cache": True,
    # Keyed on its triggers: what `bx --checks prepush --learn forge-cpp`
    # (2026-11-24) read outside them, the runner's inputs and
    # `codegen-build`'s key was what the submodule preflight read deriving
    # every gate's triggers, which it no longer does, and the schema the
    # generator validates against, which forge-conformance.yml now names.
    "inputs": INPUTS_ARE_TRIGGERS,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
