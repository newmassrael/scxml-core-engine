# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `web-vendor-licenses`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# The same compliance verdict for the browser visualizer, which
# `license-ssot` cannot reach: it reads `third_party/`, and the
# visualizer's libraries live under `web/visualizer/vendor/`.
#
# That gap was not hypothetical. d3 (ISC) shipped vendored and
# unregistered for as long as the visualizer existed, and elkjs
# (EPL-2.0 — a licence that obliges a COMMERCIAL distributor to defend
# and indemnify every other contributor) was pulled from unpkg at
# runtime by one `<script src>` line. Throughout, LICENSE-THIRD-PARTY.md
# opened with "All dependencies are MIT licensed" and nothing in the
# tree could contradict it.
{
    "workflows": ["license-verify.yml"],
    "runner_workflow": True,
    # The files the gate OPENS, not only the ones it is about. It reads
    # each vendored library's `LICENSE` and `README.md` and the registry
    # itself, so a commit that touches only one of those — deleting a
    # licence text, say, which is the violation — has to select this
    # lane. `script-read-coverage` refuses the entry without them.
    "extra": [
        "web/visualizer/vendor/**",
        "web/visualizer/*.html",
        "LICENSE-THIRD-PARTY.md",
    ],
    # 2s where the landing queue judges it: the median of its last ten
    # landing runs in the integrator's timings.tsv (2026-11-28), the seat the
    # budget bounds ("The push budget" in gate_registry.py). It read
    # 0s warm on a developer's tree.
    "cost_s": 2,
    "summary": "visualizer vendor libraries vs LICENSE-THIRD-PARTY.md",
    "cache": True,
    # Read by `check learn` (2026-11-24, bx 340e9f9e): the registry of
    # licences, each vendored library's texts and the pages that load them
    # (`web/visualizer/*.html`). It lists `web/visualizer/` for the pages
    # and `web/visualizer/vendor/` for a library nobody registered; the
    # vendor tree is named whole, which gives its listings too.
    "inputs": [
        "LICENSE-THIRD-PARTY.md",
        "web/visualizer/",
        "web/visualizer/*.html",
        "web/visualizer/vendor/**",
    ],
    "commits": [],
    "peak_gb": 1,
    "parallelism": PARALLELISM_SCRIPT,
}
