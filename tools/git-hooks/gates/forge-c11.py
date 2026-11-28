# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `forge-c11`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# ⚠ The C11 arm existed as a complete standalone CMake project, a harness
# template and ten per-kind fragments, and NOTHING RAN ANY OF IT —
# measured 2026-09-17, there was no gate script and no job in
# forge-conformance.yml. The four sibling arms made the set look
# complete. Its first run was green, so what it had not been surviving
# was change, which is exactly what a lane is for.
#
# ⚠⚠ THE FOLLOW-UP LANDED THE SAME DAY. This entry said `workflows` was
# empty on purpose — "forge-conformance.yml has no C11 job today, so
# claiming one would make `--ci-owed` report a lane that cannot pay" —
# and carried a `no_ci_reason` naming the job as the thing that would
# retire it. `conformance-c` is in that workflow now, so both are gone.
#
# The history is kept rather than deleted silently because a
# `no_ci_reason` is a PROMISE, and this note is the only evidence it was
# kept. An entry that simply stopped carrying one would read the same as
# an entry that never made the promise at all.
#
# ⚠⚠⚠ No `ci_only`: unlike the C++ arm, this one is 17s and a push that
# touches `backends/c/**` or the forge fixtures still selects it. It runs
# in BOTH places on purpose — the push catches it before it leaves, and
# the job catches the pushes that bypassed the hooks.
#
# Slug `forge-c` until 2026-10-20. A lane gate is named
# `<family>-<Language::canonical_name()>`, and this backend's name is
# `c11` (its W3C gate was already `w3c-c11`), so the old slug was the one
# lane a backend's row could not be derived from.
{
    "workflows": ["forge-conformance.yml"],
    "runner_workflow": True,
    # The modules its projects declare every generation through, resolve
    # the generator through, and format generated code by.
    "extra": ["backends/c/**", "cmake/SCECodegenPlan.cmake",
              "cmake/SCEFindCodegen.cmake", "cmake/SCEClangFormat.cmake",
              "tools/ci-images/host/**"],
    # The host image (ADR 0145, family `host`): the toolchains of the arm and
    # the packages the generator links against, the same wherever the gate
    # runs. The runner pulls the image its Containerfile's digest names and
    # runs `scripts/gate forge-c11` inside it, the generator build among its
    # deps ("Gates that run in an image" in tools/git-hooks/gate_registry.py).
    "container": "tools/ci-images/host/Containerfile",
    "needs": ["docker"],
    "deps": ["codegen-build"],
    "ci_only": "453s where the landing queue judges it, which the push budget "
               "cannot hold beside the rest; forge-conformance.yml runs it after landing.",
    # 453s where the landing queue judges it: the median of its last ten
    # landing runs in the integrator's timings.tsv (2026-11-28), the seat the
    # budget bounds ("The push budget" in gate_registry.py). It read
    # 17s warm on a developer's tree.
    "cost_s": 453,
    "summary": "C11 forge conformance build + test",
    "cache": True,
    # Keyed on its triggers: what `bx --checks prepush --learn forge-c11`
    # (2026-11-24) read outside them, the runner's inputs and
    # `codegen-build`'s key was what the submodule preflight read deriving
    # every gate's triggers, which it no longer does, and the schema the
    # generator validates against, which forge-conformance.yml now names.
    "inputs": INPUTS_ARE_TRIGGERS,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
