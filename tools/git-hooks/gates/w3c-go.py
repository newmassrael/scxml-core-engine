# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `w3c-go`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
{
    "workflows": ["w3c-tests.yml"],
    "runner_workflow": True,
    "narrows": "same catch-all as `w3c-cpp`, same reason. This arm reads "
               "the Go backend, the Go templates and the generator.",
    # The shared Lua is copied under backends/go; the shared tables its
    # readers ask are not.
    "extra": ["backends/go/**", "tools/codegen/templates/**",
              "sce-build/src/**", "tests/ecmascript/**",
              # The test log's wire schema, which the runtime's
              # `TestEveryKindIsOneTheSchemaNames` reads at run time.
              "schemas/sce-test-log.v1.schema.json",
              # The test log every runtime writes of the generated suite,
              # which the Go suite runner's test of `answers_the_job`
              # reads at run time.
              "sce-build/tests/fixtures/test_profile/runtime/expected_logs/**",
              # The suite's committed traces, which the same test replays,
              # and the trace's wire schema, whose status the runtime's
              # `TestSchemaFileDeclaresStatus` holds its own to, both read
              # at run time.
              "sce-build/tests/fixtures/test_profile/runtime/traces/**",
              "schemas/sce-test-trace.v1.schema.json",
              # The traces and expected logs of the corpus's other suites,
              # which `a_corpus_suite_replays_to_its_expected_logs_test.go`
              # replays and reads at run time.
              "sce-build/tests/fixtures/test_profile/runs/**"],
    "deps": ["codegen-build"],
    # 10s, timed on a warm tree on 2026-09-02 and no longer a normalised
    # figure. Six `--measure` runs read 9 10 9 10 10 10 — none of them
    # queued behind the cargo package-cache lock, which is what makes
    # this the cleanest of the three retirements. 10 is the worse of the
    # last pair. Two pushes that day read 15s and 8s, straddling it.
    "cost_s": 10,
    "summary": "W3C conformance, Go AOT",
    "cache": True,
    "inputs": None,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
