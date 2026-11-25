# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `ecma262-lowered-cpp`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# The only gate that configures the OTHER value of SCE_SCRIPT_ENGINE.
#
# Every gate above builds the quickjs selection, which is what
# `tests/CMakeLists.txt` said three times in as many words — "no gate
# configures -DSCE_SCRIPT_ENGINE=lua" — and each of those comments named a
# file compiled by every build and run by none. This one compiles a C++
# artifact generated with `--script-engine lua` and runs it, which is the
# measurement `docs/SCE_LUA_TRANSLATION_SEAM.md` ended without.
#
# Its workflow declares a `paths:` filter, so the filter IS the trigger and
# there is no `narrows`/`extra` pair to keep in step with it. `sce/**` is
# deliberately NOT in that filter as a whole: the selection reaches only the
# scripting seam, the shared helpers the generated code calls, and the
# option's own CMakeLists.
{
    "workflows": ["ecma262-lowered-cpp.yml"],
    "builds": ["main-tree"],
    "runner_workflow": True,
    "deps": ["codegen-build"],
    # The scratch tree is thrown away on every run, so there is no warm
    # case for this gate the way there is for one that reuses `build/`:
    # `SCE_SCRIPT_ENGINE` is PUBLIC on `sce_scripting`, the selection is
    # therefore a property of the whole tree, and a developer's `build/` is
    # the quickjs one and must stay that way. What that costs is the sce
    # libraries from cold on every invocation, which is minutes rather than
    # the ~60s an already-built tree takes.
    #
    # 123s measured by `scripts/gate --measure` on the build machine
    # 2026-08-29, with ccache warm. 86 of those seconds are the
    # `sce_build_is_current` ctest FIXTURE, not the suite — the suite itself
    # runs in 0.16s — so the number is dominated by a staleness check the
    # tree makes every ctest case pay.
    #
    # So it is CI's, not the push's. A developer who wants it runs
    # `scripts/gate ecma262-lowered-cpp` by name.
    "ci_only": "it configures and builds a second tree from cold — the "
               "`lua` selection cannot share the quickjs `build/` because "
               "the option is PUBLIC on sce_scripting. "
               "ecma262-lowered-cpp.yml runs it, on a filter naming the "
               "seam's two halves and the two tables the fixture's "
               "population comes from.",
    "cost_s": 123,
    "summary": "ECMA-262 through a Lua-lowered C++ artifact",
    "cache": True,
    # The whole tree: `bx --checks prepush --learn ecma262-lowered-cpp`
    # (2026-11-24) read 946 of its 1931 paths outside its triggers, nearly
    # all of them the root configure's test tree (`tests/**`,
    # `resources/**`, `integration_resources/**`, the LICENSE files,
    # `web/visualizer/test-list.js`), which `main-tree` does not name.
    "inputs": None,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
