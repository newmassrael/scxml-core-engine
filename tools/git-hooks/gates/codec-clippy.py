# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `codec-clippy`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# The generated codecs are committed goldens, not workspace members:
# `cargo metadata` lists eight packages and none of them is a codec, so
# `clippy --workspace` lints the generator and never its output. Both arms
# were CI-only until the registry started asking which workflows lack a
# gate.
{
    "workflows": ["sce-forge-codec-clippy.yml"],
    "runner_workflow": True,
    # The goldens compile against the Rust forge runtime and what cargo
    # builds for it (its path dependencies, the workspace manifests). The
    # workflow named the runtime's directory alone, so an edit of
    # `backends/rust/portable-bytes` selected nothing that lints codecs.
    "builds": ["forge-runtime-rust"],
    "ci_only": "106s linting 95 generated codec goldens. "
               "sce-forge-codec-clippy.yml runs it.",
    "cost_s": 106,
    "summary": "clippy generated codecs, alloc on",
    "cache": True,
    # Keyed on its triggers and the two directories the script lists for
    # `codec_*.rs`. `bx --checks prepush --learn codec-clippy` (pc2,
    # 2026-11-24): 206 reads and 5 listings, every one inside these once
    # the runtime's cargo build is among the triggers.
    "inputs": INPUTS_ARE_TRIGGERS,
    "lists": ["tests/forge/expected/", "tests/forge/expected/views/"],
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
