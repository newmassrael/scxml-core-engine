# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `nl-ir-closure`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# The NL→IR closure ledger, measured against the tree it describes.
#
# `docs/SCE_NL_IR_CLOSURE.md` is a denominator — the closed list of what
# remains on SCE's side of the specification→IR path, written before and
# independently of whatever closes its rows. This gate is what keeps it
# from becoming prose: it measures every row and fails when the recorded
# status disagrees with the tree IN EITHER DIRECTION. A row recorded
# closed that the tree contradicts is a false green; a row recorded open
# that the tree has closed is a stale denominator.
#
# ⚠ It does NOT fail for unfinished work. The `closure:` line on stdout
# carries that answer, and the exit status carries whether the record is
# honest. Collapsing the two would make the gate unrunnable in the push
# path for as long as any row is open, which is the whole of its useful
# life.
#
# Catch-all trigger, for the endpoint-SSOT lane's reason: a row can close
# in a template, a header, a CMake list, a Rust module or the visualizer,
# and a `paths:` list narrowed to what anyone remembers misses the one
# nobody thought of. Affordable — grep, one short Python pass, the two
# syntax-only compiles row S3 asks the AOT registrar with, and three runs
# of `sce-codegen requirement-closure` over manifests row G4 writes. The
# runs behind that number are in the measurement-date map.
#
# ⚠ WARM, and for row G4 warm now means more than a filled page cache: the
# binary must be current. The shared locator rebuilds one built from other
# sources — deliberately, because asking a stale binary answers about a
# tree that no longer exists — so a push whose own change touched
# `sce-build/src/` pays that build here if nothing else has paid it yet.
# Measured 2026-09-15: 1.33 1.31 1.33s with the binary current, and 12.9s
# on the run that rebuilt it. The warm figure is the one declared, on the
# basis this table's header already gives: a push happens on a tree the
# developer has just built, and a cold build is paid once.
{
    "workflows": ["nl-ir-closure.yml"],
    "runner_workflow": True,
    # Rows G2-G4 run `sce-codegen` through `sce_gate_codegen`. Without
    # this edge the gate sorted ahead of the build on its 1.3s and paid
    # for it: 288s on 2026-09-24, the push after `sce-build`'s dev
    # profile changed. The self-test now derives the edge from the script.
    "deps": ["codegen-build"],
    # 117s where the landing queue judges it: the median of its last ten
    # landing runs in the integrator's timings.tsv (2026-11-28), the seat the
    # budget bounds ("The push budget" in gate_registry.py). It read
    # 1.3s warm on a developer's tree.
    "cost_s": 117,
    "summary": "the NL→IR closure ledger vs the tree it describes",
    "cache": True,
    "inputs": None,
    "commits": [],
    "peak_gb": 1,
    "parallelism": PARALLELISM_SCRIPT,
}
