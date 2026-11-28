# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `ecma262-lowered-kotlin`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# The Kotlin twin of the row above, and it exists because the argument that
# row makes applies unchanged to a second backend that had crossed the same
# seam. Kotlin had `ScriptSource.lua`, a lowered arm on `LuaScriptEngine`
# and a frontend that answers the shared table — and no lane that ran an
# ARTIFACT through any of it.
#
# ⚠ It found a defect on its first run: the generated Kotlin evaluates a
# transition's `cond` twice, so a guard with a side effect records the
# wrong arm's answer. Declared in
# `tests/ecmascript/kotlin_lowered_artifact_defects.json`, which this gate
# holds in both directions.
{
    "workflows": ["ecma262-lowered-kotlin.yml"],
    "runner_workflow": True,
    "deps": ["codegen-build"],
    # Local: at 187s where the landing queue judges it (the `cost_s`
    # comment) it fits the ceiling as the judgment's length on the build
    # machines (docs/adr/0157). Unlike the C++ twin this gate
    # configures no second tree: it regenerates two machines into the
    # module's own Gradle build and runs 98 cases against them.
    #
    # ⚠ THE BASIS IS NOT THE WARM ONE THIS TABLE'S HEADER DESCRIBES, and
    # the departure is a faithful reading of that header rather than an
    # exception to it. "Warm" is defended up there as "a push happens on a
    # tree the developer has just built" — and NOTHING but this gate builds
    # this module. Its generation, its two machines and its test sources
    # are reached by no other target, so a warm reading here means "this
    # gate ran before", not "the developer just built the tree". The state
    # a push actually finds it in is the one its own `paths:` filter
    # selects for: a change under `tools/codegen/templates/**` or
    # `sce-build/src/**` invalidates the generation and the compile.
    #
    # So the number is measured from THAT state, worse of two runs, the
    # way the header takes the worse of two:
    #
    #   ./gradlew :sce-kotlin-lowered-ecma262:clean
    #   GRADLE_OPTS="-Dorg.gradle.caching=false" \
    #       scripts/gate --measure ecma262-lowered-kotlin
    #
    # 2026-08-30 on the build machine: 11s and 4s from clean; 1s and 1s
    # warm. 11 is declared. Over-declaring is the safe direction — the
    # ceiling check only refuses a gate that grew PAST its declaration —
    # and it keeps this row above `COST_NOISE_S`, so the gate contributes
    # a ratio to the pace estimate instead of being invisible to it.
    #
    # ⚠⚠ The line this replaced claimed "2s … INCLUDING a clean". Two
    # clean runs measured 4s and 11s, so that number was a warm reading
    # wearing a cold justification. Re-derive with the two commands above
    # rather than trusting this paragraph.
    #
    # ⚠ The suite itself always RUNS: the gate passes `--rerun`, because
    # its verdict is read from the run's output and Gradle answers an
    # unchanged test task UP-TO-DATE without producing any. So this is not
    # a cached green being timed.
    # 187s where the landing queue judges it: the median of its last ten
    # landing runs in the integrator's timings.tsv (2026-11-28), the seat the
    # budget bounds ("The push budget" in gate_registry.py). It read
    # 11s warm on a developer's tree.
    "cost_s": 187,
    "summary": "ECMA-262 through a Lua-lowered Kotlin artifact",
    "cache": True,
    "inputs": None,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
