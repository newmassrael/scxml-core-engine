# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `example-codegen`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# Catches codegen breakage in the example documents (the namespace
# migration that broke them shipped green otherwise) and lints every
# document this repository authors. The trigger comes from the
# workflow's `paths:` like every other mirrored gate; it used to be a
# `local` list, and that list named `examples/**` while the gate's own
# sweep also read `integration_resources/*/*.scxml` — so editing an
# authored document there ran the lint never. The self-test now
# compares a gate's sweep against its trigger.
{
    "workflows": ["example-codegen.yml"],
    # The workflow runs `scripts/gate example-codegen` instead of
    # restating this gate's commands, so the check has one spelling
    # for both callers. Every other mirrored gate still carries a
    # second copy of its commands in its workflow; this flag is what
    # makes the difference measurable per gate rather than a claim
    # about the set, and the self-test below holds the delegation.
    "runner_workflow": True,
    "deps": ["codegen-build"],
    # Runs in the host image (tools/ci-images/host, docs/adr/0145), with the
    # generator it builds there: clang-format-19 formats the C++ the examples
    # are generated as, which the lane installed at job time.
    "container": "tools/ci-images/host/Containerfile",
    "needs": ["docker"],
    "ci_only": "99s against a declared 4s — the widest gap in the table, "
               "and the clearest case of a warmed-sweep reading being "
               "mistaken for what a push pays. It generates from every "
               "authored document under examples/ and integration_"
               "resources/, which is work proportional to the corpus. "
               "example-codegen.yml runs it.",
    "cost_s": 99,
    "summary": "example SCXML codegen smoke + authored-document lint",
    "cache": True,
    "inputs": None,
    "commits": [],
    "peak_gb": 1,
    "parallelism": PARALLELISM_SCRIPT,
}
