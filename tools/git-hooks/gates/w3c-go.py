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
              "sce-build/tests/fixtures/test_profile/runs/**",
              # The recorded runs of the Go lane, which
              # `backends/go/tests/run_trace` holds its runs to at run time
              # (docs/adr/0115, D4 and D8).
              "docs/formal/traces/**",
              # The turnstile of the properties judge's fixture, whose machine
              # the Go integration tree holds, and its scenarios with the Go
              # lane's recorded runs of them, which
              # `backends/go/tests/run_trace` replays and holds its runs to at
              # run time (docs/adr/0115, D15).
              "sce-build/tests/fixtures/formal_properties/**"],
    "deps": ["codegen-build"],
    # 10s, timed on a warm tree on 2026-09-02 and no longer a normalised
    # figure. Six `--measure` runs read 9 10 9 10 10 10 — none of them
    # queued behind the cargo package-cache lock, which is what makes
    # this the cleanest of the three retirements. 10 is the worse of the
    # last pair. Two pushes that day read 15s and 8s, straddling it.
    # 143s where the landing queue judges it: the median of its last ten
    # landing runs in the integrator's timings.tsv (2026-11-28), the seat the
    # budget bounds ("The push budget" in gate_registry.py). It read
    # 10s warm on a developer's tree.
    "cost_s": 143,
    "summary": "W3C conformance, Go AOT",
    # It runs in the host image wherever it runs (ADR 0145): the runner runs
    # `scripts/gate` inside the image its Containerfile addresses, so the
    # hosted lane, a build machine and a workstation judge it in one
    # environment, and no lane installs a package for it. A machine takes its
    # unit only where docker runs (`needs`).
    "container": "tools/ci-images/host/Containerfile",
    "needs": ["docker"],
    "cache": True,
    "inputs": None,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
