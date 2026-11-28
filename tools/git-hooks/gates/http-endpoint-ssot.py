# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `http-endpoint-ssot`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# Where the W3C BasicHTTP fixture listener answers had come to be spelled in
# five places at once — twelve C11 runners, two gate helpers, a CI job, the
# Rust and Go harnesses, two Python suites — and the drift only became
# visible when a second checkout could not run the BasicHTTP suites while
# the first held the port. `tests/w3c/basic_http_test_endpoint.h` owns it
# now; this keeps that ownership a property of the tree.
#
# Trigger is deliberately the catch-all, and that is affordable rather than
# lazy: a re-spelling can appear in any file, and narrowing to the paths we
# happen to remember would miss exactly the one nobody thought of. The gate
# asks `git grep` for the handful of files that name the number at all and
# runs its comment-stripper only on those — 124ms measured, down from 14.7s
# when it stripped every tracked source file.
#
# It gets its own lane rather than riding another's because nothing else
# can catch the defect: a re-spelled port that equals the owner's still
# WORKS, so every suite stays green and the drift surfaces only the day
# someone moves the endpoint. There is no compile error to lean on.
{
    "workflows": ["http-endpoint-ssot.yml"],
    "runner_workflow": True,
    # 0.185s, and the first figure here this gate has ever been able to
    # confirm. The 4 it carried was chosen on 2026-08-24 to cover the
    # process the runner starts around a 124ms scan, because `--measure`
    # answered 0 every time — `scripts/gate` timed with whole-second
    # `SECONDS`, so a gate that finishes in a fifth of a second had no
    # readable cost at all. The runner now keeps a millisecond clock, and
    # six runs on 2026-09-02 read 0.165 0.145 0.154 0.162 0.169 0.185,
    # none of them queued behind the cargo lock. 0.185 is the worse of
    # the last pair. The process the 4 was covering turns out to be
    # ~40ms, not the seconds it was insured against.
    # 8s where the landing queue judges it: the median of its last ten
    # landing runs in the integrator's timings.tsv (2026-11-28), the seat the
    # budget bounds ("The push budget" in gate_registry.py). It read
    # 0.185s warm on a developer's tree.
    "cost_s": 8,
    "summary": "BasicHTTP fixture endpoint has one owner",
    "cache": True,
    # The whole tree, and it stays so: the scan finds its candidates with
    # `git grep` over every tracked source, and in a narrowed unit's copy
    # that search would see only the inputs, so a re-spelling in a file
    # left out would pass.
    "inputs": None,
    "commits": [],
    "peak_gb": 1,
    "parallelism": PARALLELISM_SCRIPT,
}
