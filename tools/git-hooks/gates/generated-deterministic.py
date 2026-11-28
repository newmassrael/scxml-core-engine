# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `generated-deterministic`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# What replaces `regen-reproduces` once nothing generated is committed
# (ADR 0097, section 5): every action of every declared tree run here
# from an empty cache and held against the fleet's entry of the same key.
# Declared and skipped until bx's fleet cache exists: the script passes
# and says so when `SCE_FLEET_CACHE` names no instance, and the half that
# needs no fleet (two runs on one machine) is a test of the workspace
# suite, `a_declared_tree_reproduces_its_committed_tree`.
{
    "local": ["tools/codegen/**", "stdlib/**"],
    "builds": ["sce-codegen"],
    "no_ci_reason": "it compares against the fleet's action cache, which no "
                    "hosted runner reaches (a site's REAPI instance, read "
                    "through `bx-fleet site-tunnel`), and which does not "
                    "exist until bx's fleet cache lands (ADR 0097, "
                    "amendment 16). Without one the gate is skipped.",
    # Unmeasured: the skip is a shell start. A run that names a fleet
    # builds the generator twice (the comparer in a target directory of
    # its own) and runs every declared tree's actions, about what
    # `regen-reproduces` costs.
    # 1s where the landing queue judges it: the median of its last ten
    # landing runs in the integrator's timings.tsv (2026-11-28), the seat the
    # budget bounds ("The push budget" in gate_registry.py). It read
    # 1s warm on a developer's tree.
    "cost_s": 1,
    "summary": "every action agrees with the fleet's entry of its key",
    "cache": False,
    "no_cache_reason": "its verdict is about the fleet's entries as they "
                       "are when it runs, which move without any commit",
    "inputs": None,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
