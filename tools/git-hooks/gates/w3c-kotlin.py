# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `w3c-kotlin`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
{
    "workflows": ["w3c-tests.yml"],
    "runner_workflow": True,
    "narrows": "same catch-all as `w3c-cpp`, same reason. This arm reads "
               "the Kotlin backend, the Kotlin templates and the "
               "generator.",
    # The last three are what the SCRIPT opens: the refusal list it
    # holds a ceiling over, the fixture registry it walks, and the one
    # document it probes the seam with. Declared because `--ci-owed`
    # reads this list to say which lanes judge a change, and a commit
    # editing only a divergence list was being told this lane was not
    # one of them (`script-read-coverage`).
    "extra": ["backends/kotlin/**", "tools/codegen/templates/**",
              "sce-build/src/**",
              # How both suite modules run their tests and take a row's
              # overlay, applied by each (it was the tests module's own
              # build file until the W3C suite became a module).
              "gradle/sce-kotlin-suite.gradle.kts",
              # The shared Lua the Lua engine copies in at build time, and
              # the shared tables its readers ask.
              "sce/include/scripting/*.lua", "tests/ecmascript/**",
              "tests/ecmascript/kotlin_frontend_refusals.json",
              "tests/w3c/conformance/fixtures.json",
              "resources/150/test150.scxml",
              # The test log every runtime writes of the generated suite,
              # which `ATestSuiteRunsThroughItsRunnerTest` reads at run time.
              "sce-build/tests/fixtures/test_profile/runtime/expected_logs/**",
              # The suite's committed traces, which the same test replays,
              # and the trace's wire schema, whose status `TraceTest` holds
              # the runtime's to, both read at run time.
              "sce-build/tests/fixtures/test_profile/runtime/traces/**",
              "schemas/sce-test-trace.v1.schema.json",
              # The traces and expected logs of the corpus's other suites,
              # which `ACorpusSuiteReplaysToItsExpectedLogsTest` replays and
              # reads at run time.
              "sce-build/tests/fixtures/test_profile/runs/**"],
    "deps": ["codegen-build"],
    # 69s was measured on a push whose Kotlin tree was already current.
    # 152s, pace-normalised, is what it cost on 2026-08-24 when a
    # `tools/codegen/templates/**` edit invalidated every generated Kotlin
    # file — and that is not the unusual case for this row, because that
    # path is one of its own triggers. A gate is worth what it costs when
    # its trigger actually fires, so the higher reading is the one written
    # down. The earlier note said 10s / 34s, both taken with fewer suites
    # in the same run.
    "ci_only": "152s when a template edit fires its own trigger — half "
               "the 300s push budget for one gate, and it breached the "
               "ceiling on the push that measured it. w3c-tests.yml is "
               "unfiltered, so CI runs this arm on every push regardless: "
               "the same trade `w3c-python` took at less than half the "
               "cost. What is given up is that a Kotlin AOT regression "
               "now reaches main and is answered a round later.",
    "cost_s": 152,
    "summary": "W3C conformance, Kotlin/JVM AOT (Rhino + QuickJS + Lua, "
               "each over the machines emitted for the language it reads)",
    "cache": True,
    "inputs": None,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
