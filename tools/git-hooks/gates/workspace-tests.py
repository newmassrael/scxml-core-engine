# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `workspace-tests`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# Mirrors the Rust workspace suite only. `w3c-tests.yml` is deliberately
# NOT listed: it has no `paths:` filter (it runs on every CI push) and
# it drives the C++ ctest suite, which this hook does not mirror at all.
# Mapping it here would make its catch-all the trigger for the hook's
# longest gate and, worse, would classify every path — defeating the
# unclassified-forces-FULL rule that keeps this file honest.
# `extra` covers inputs the workflow filter misses because they are
# `include_str!`-ed into the suite rather than compiled: the acceptance
# doc, the wire schemas, and the forge-AST export schema.
{
    "workflows": ["rust-workspace-tests.yml"],
    # The host image (ADR 0145, family `host`): the C, C++, JVM, Go and
    # Python toolchains the suite's harness asks for, the same wherever the
    # gate runs. The runner pulls the image its Containerfile's digest names
    # and runs `scripts/gate workspace-tests` inside it ("Gates that run in an
    # image" in tools/git-hooks/gate_registry.py).
    "container": "tools/ci-images/host/Containerfile",
    "needs": ["docker"],
    # What the lane sets for the run inside: every harness tool required
    # rather than skipped, and the test artifacts without DWARF, which kept
    # the lane inside the runner's disk.
    "container_env": ["SCE_REQUIRE_TOOLS", "CARGO_PROFILE_DEV_DEBUG",
                      "CARGO_PROFILE_TEST_DEBUG", "CARGO_PROFILE_DEV_STRIP",
                      "CARGO_PROFILE_TEST_STRIP", "CARGO_INCREMENTAL"],
    # Run whole it is the longest unit of a landing after cpp-suite-wasm
    # (1363 s), and its script already runs one shard of itself when
    # SCE_WORKSPACE_SHARD names `k/N`, as the lane's four jobs do. A plan
    # makes one unit per shard of the lane's count (`shard_count`).
    "shard_env": "SCE_WORKSPACE_SHARD",
    # The lane's one difference — `--no-fail-fast`, so a run nobody can
    # iterate on reports every failure — was a reporting choice by its own
    # description, which made it a switch rather than a reason to keep two
    # spellings. The gate reads `SCE_GATE_NO_FAIL_FAST` and the lane sets
    # it.
    "runner_workflow": True,
    # ⚠ `.github/workflows/**` is deliberately NOT here, though the suite
    # reads every file in that directory at run time
    # (`mutation_round_survives_the_next_push`, `ci_supersession_policy`).
    # `extra` is the HOOK's trigger list as well as the CI-coverage one,
    # and adding it classified every workflow path — which broke
    # `unfiltered-workflow-self`: editing a workflow that declares no
    # `paths:` of its own is meant to be unclassified and buy the full run,
    # and a `ci_only` gate cannot take that job because the hook is never
    # offered it. The path would have read as known and selected nothing.
    #
    # The coverage the suite needs is one-directional and belongs on the
    # other side: `rust-workspace-tests.yml` lists `.github/workflows/**`
    # in its `paths:`, so the lane starts for a workflow edit. A workflow
    # filter WIDER than its gate's triggers is what `ci-only-coverage`
    # allows; the reverse is what it refuses.
    # `tests/json_text/**` and `tests/durations/**` are tables the runtime's
    # and sce-build's unit tests read at run time — every engine is held to
    # them — so an edit to a table alone has to start the suite that reads it.
    "extra": [
        # The image the suite runs in: an edit of it is a change the suite
        # judges.
        "tools/ci-images/host/**",
        # The installer of the rev-pinned validator, which the gate runs in
        # the image (`sce_gate_provide_mnemosyne_pin`).
        "scripts/install_mnemosyne_cli.sh",
        # What spreads the suite over the lane's shards
        # (`SCE_WORKSPACE_SHARD`), which the gate script runs.
        "scripts/lib/workspace_shards.py",
        # What `a_red_gate_names_the_smallest_rerun` runs and reads: the
        # reader of a red runner's report, the gate library that hands it the
        # report, the runner that prints its heading, and the mutation
        # harness's one-case selector.
        "scripts/lib/repro.py",
        "scripts/gates/lib.sh",
        "scripts/gate",
        "scripts/mutate",
        "docs/SCE_ACCEPTED_SUBSET.md",
        "schemas/**",
        "apis/**",
        "tests/json_text/**",
        "tests/durations/**",
        "tests/document_stem/**",
        "tests/external_event_budget/**",
        "tests/scripting/**",
        # The shared Mesh-target predicate table the build reads.
        "tests/mesh/**",
        # The server suite (docs/adr/0107, D10), a workspace member outside
        # `backends/rust/**`.
        "tests/server/**",
        # The policy documents `forge_conformance` compiles into its
        # per-language run tests.
        "tests/forge/resources/**",
        # The forge conformance catalog and the document sets beside it,
        # which `generation_can_be_planned` and
        # `configure_plans_are_kept_and_asked_together` plan and generate a
        # harness and a set from.
        "tests/forge/conformance/**",
        # The integration stems `the_emitted_transition_function_equals_the_producer`
        # generates and states the table of.
        "integration_resources/flat_lifecycle/**",
        "integration_resources/nested_lifecycle/**",
        # The conformance documents, which suites here read by path
        # (`w3c_name_system_variable`, `codegen_depfile_content`,
        # `cli_lint_parity`, `a_regeneration_removes_what_it_no_longer_writes`)
        # and parse as a corpus.
        "resources/**",
        # The two CMake routes that generate at build time and the one module
        # they ask the generator's plan through, which
        # `the_conformance_cmake_route_declares_what_the_generator_plans`
        # compiles in and holds to declaring what the generator plans.
        "cmake/SCECodegenPlan.cmake",
        "cmake/SCEStaticW3CTest.cmake",
        "cmake/SCEStaticIntegrationFixture.cmake",
        # The module the plan module includes, which
        # `a_configure_asks_each_plan_once_per_input_state` configures a
        # project through with a real `cmake`.
        "cmake/SCEClangFormat.cmake",
        # The locator the plan module resolves the generator through when
        # no includer did.
        "cmake/SCEFindCodegen.cmake",
        # The test tree's build, whose Mesh rule-3 override and every
        # command it declares from a plan the same suite compiles in and
        # holds to declaring what the generator plans.
        "tests/CMakeLists.txt",
        # The C11 conformance suite's build, which the same suite compiles
        # in and holds to declaring its fixtures after their plans were
        # asked together.
        "backends/c/tests/CMakeLists.txt",
        # The conformance registry, whose fixtures
        # `a_c11_machine_includes_what_it_states_it_requires` generates,
        # and the one reading of it both conformance routes register
        # their fixtures through, which `w3c_registry_drives_cmake`
        # reads.
        "tests/w3c/conformance/fixtures.json",
        "cmake/SCEW3CRegistry.cmake",
        # The C11 ctest fixture's BasicHTTP listener and the script that
        # hands its port to the runners, which
        # `the_c11_http_fixture_hands_its_runners_the_port_it_bound` runs.
        "tests/w3c/http_server_fixture.sh",
        "tests/w3c/standalone_http_server.js",
        "tests/w3c/http_endpoint.js",
        "tests/w3c/basic_http_test_endpoint.h",
        # The C11 runtime's build, whose vocabulary of what a machine
        # requires `a_c11_machine_includes_what_it_states_it_requires`
        # compiles in and holds to what the generator can state.
        "backends/c/runtime/CMakeLists.txt",
        # The host's log seams, which
        # `a_c11_machine_logs_through_the_log_seams` links a machine with
        # and reads the lines of.
        "backends/c/runtime/posix/log.c",
        # The runtime's text unit, the real seams' definitions, which the
        # same test links a machine with (docs/adr/0110, D2).
        "backends/c/runtime/text/real_text.c",
        # The C11 runtime's headers, whose includes
        # `a_c11_machines_core_reaches_only_freestanding_headers` walks from
        # every machine's core (docs/adr/0110, D1): an edit of one alone
        # has to start the suite that judges it.
        "backends/c/runtime/include/**",
        "backends/c/forge-runtime/include/**",
        # The C11 MCU gate's set, its budget, its board and the record that
        # names the set (docs/adr/0110, D3), which
        # `the_c11_mcu_gate_weighs_the_set_its_record_names` compiles in
        # and holds to one another.
        "scripts/gates/c11-mcu.sh",
        "backends/c/mcu/**",
        "docs/adr/0110-a-c11-machine-core-is-freestanding-and-a-gate-weighs-and-runs-it-on-an-mcu.md",
        # The standard library, whose statecharts that save are held to
        # their pinned shapes (`a_standard_statechart_that_saves_keeps_its_shape`,
        # docs/adr/0045): an edit of one alone has to start the suite.
        "stdlib/**",
        # The scenarios of the standard host features, which
        # `every_replay_scenario_keeps_to_its_vocabulary` holds to the
        # members their drivers read: an edit of one alone has to start
        # the suite.
        "tests/host_features/**",
        # The registry of metric names both lanes are held to (docs/adr/0109,
        # D16), which `sce-rust-metrics` reads at run time.
        "tests/metrics/**",
        # The narrow row's runtime header, which the C11 broadcast-log
        # tests read to hold the stamp width the generator judges against
        # to the width the header keeps and each name the generated header
        # calls to a declaration of it.
        "backends/c/forge-runtime/include/sce/forge/broadcast_log_narrow.h",
        # The C++ runtime's two broadcast-log headers, which the C++
        # broadcast-log tests read: each name the generated headers call
        # against a declaration of it, and the stamp width the generator
        # judges against the width the narrow row keeps.
        "backends/cpp/forge-runtime/include/sce/forge/broadcast_log.h",
        "backends/cpp/forge-runtime/include/sce/forge/broadcast_log_narrow.h",
        # The Kotlin replays of the two C ABIs (group key agreement, peer
        # core), which the lane builds with Gradle and the capi crates'
        # tests run under `SCE_REQUIRE_TOOLS`: an edit of one alone has to
        # start the suite that runs it.
        "backends/kotlin/mls-replay/**",
        "backends/kotlin/peer-replay/**",
        # The Kotlin binding of the peer core's C ABI, which the peer
        # core's Kotlin replay and the world's Kotlin device are built on.
        "backends/kotlin/peer/**",
        # The ledgers' configurations and stores, which
        # `every_ledger_binding_names_what_its_file_holds` reads: an edit
        # of a binding alone has to start the suite that holds it to its
        # file. The bound files themselves span the source trees, and an
        # edit of one that renames a bound symbol is read on the next run
        # of this lane, not on its own push.
        "docs/spec/*/mnemosyne.toml",
        "docs/spec/*/.atomic/workspace.atomic.json",
        "docs/sce-ledger/*/mnemosyne.toml",
        "docs/sce-ledger/*/.atomic/workspace.atomic.json",
        # The generated diff's renderer and the locator it resolves the
        # generator through, which
        # `a_change_shows_what_it_does_to_the_generated_code` copies into
        # a repository of its own and runs.
        "scripts/generated_diff.sh",
        "scripts/lib/sce_codegen.sh",
        # The release pipeline's declaration of its packages and its
        # reader, which `a_release_declares_its_packages_by_their_trees`
        # holds to the tree manifests.
        "scripts/release.py",
        "tools/codegen/release.toml",
    ],
    # The crate whose test suite this gate runs. `include-str-coverage`
    # in the self-test reads it: every file the sources under this root
    # `include_str!` is an input the suite asserts on, so some lane that
    # runs the suite has to start when it changes. Declaring the root is
    # the hand-written half; which paths it implies comes from the tree,
    # so an `include_str!` added tomorrow is covered the day it is
    # written rather than the day somebody remembers a list.
    "include_str_roots": ["sce-build"],
    # 1363s, and it was declared 151 until 2026-09-20. This is the densest
    # correctness net in the tree — every contract test under
    # `sce-build/tests/` runs here — and it leaves the push anyway,
    # because the budget is a ceiling on what a developer waits for rather
    # than a ranking of what is valuable.
    #
    # ⚠ NINE TIMES OFF, AND NOTHING HERE COULD SEE IT. `budget_breach`
    # compares a declaration against a measurement only for gates a push
    # SELECTS; this one is `ci_only`, so no push prices it and its figure
    # was watched by nothing. The header says as much about `w3c-kotlin`
    # and calls it the right scope for a PUSH budget, which it is — the
    # residue is that a ci_only cost can rot without limit, and this is
    # what that looks like when it does.
    #
    # ⚠⚠ Where the time actually is, measured the same day: three targets
    # own 790s of it — `diagnostic_fix_is_applicable` 469s over 5 tests,
    # `ecmascript_acceptance_parity` 187s over 4, `mutation_rounds_
    # selection` 134s over 13. 64% of the lane in 22 tests, so "the suite
    # is large" is not what makes it long, and 504 test binaries share the
    # other 36%.
    #
    # ⚠⚠⚠ A fail-fast run reads a THIRD of this and looks like the whole
    # lane: 180 of 504 binaries in 1184s, because the slow targets sort
    # early. `SCE_GATE_NO_FAIL_FAST=1` is what makes a local run comparable
    # to the CI lane's, and three rounds of fixing one failure at a time
    # were spent before that was read off the gate script.
    "ci_only": "1363s measured 2026-09-20, declared 151s until then. "
               "Fully mirrored by rust-workspace-tests.yml, which runs "
               "the same command. The trade is the sharpest of the four: "
               "this is where every sce-build contract test lives, so a "
               "broken contract now reaches main and is answered a round "
               "later.",
    "cost_s": 1363,
    "summary": "cargo test --workspace --features cli,sce-rust-mesh/wss",
    # The gate builds the Gradle prerequisites the suites run (the Kotlin
    # runtime jars and the three JVM programs) before the suites, so a
    # pass no longer depends on what the machine built before. What is
    # left is a test that skips on a missing tool, and a landing runs the
    # suites under SCE_REQUIRE_TOOLS and reads such a miss as could not
    # run. It also reads HEAD's history (`lowering_decision_ledger`),
    # which `commits` carries.
    "cache": True,
    "kept_only_from_a_landing": "a suite test skips with a warning where a "
                            "tool or a built JVM program it runs is "
                            "missing (`ToolLocator::require_or_skip`, "
                            "`require_all_or_warn`)",
    "inputs": None,
    "commits": ["HEAD"],
    # The highest figure a run has reached (PEAK_MEASURED); the one run
    # that reached it was killed there, so the true peak may lie above.
    "peak_gb": 16,
    "parallelism": PARALLELISM_COMPILE,
}
