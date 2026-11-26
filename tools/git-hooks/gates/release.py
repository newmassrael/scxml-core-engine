# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The registry entry of the gate `release`, read by that slug alone
# ("One file per gate" in tools/git-hooks/gate_table.py).
#
# The release pipeline's work (ADR 0097, section 7), run in the host image
# (ADR 0145). release.yml publishes a release and reached no verdict about a
# change, so until 2026-11-27 it was a workflow no gate mirrored, and it built
# the generator and materialized every tree with the packages it installed on
# its runner at job time. Under ADR 0145 no workflow installs a package and the
# packages a run reads are an image's, which the runner enters through a gate:
# so the work is this gate's, and its verdict is the one the work rests on, that
# tools/codegen/release.toml declares its packages by their trees. On a commit a
# release tag names it also makes the release, which the workflow publishes
# (scripts/gates/release.sh says what it writes, and that a run without a tag
# writes nothing).
{
    "workflows": ["release.yml"],
    "runner_workflow": True,
    # The host image (ADR 0145, family `host`): the generator's build and the
    # formatters the trees are written through. The runner pulls the image its
    # Containerfile's digest names and runs `scripts/gate release` inside it.
    "container": "tools/ci-images/host/Containerfile",
    "needs": ["docker"],
    # The generator every tree of a release is materialized with, built in
    # the image before the gate.
    "deps": ["codegen-build"],
    "narrows": "release.yml runs on a release tag and so declares no paths:; "
               "what the gate reads is the release declaration, its reader and "
               "the tree manifests it holds the declaration to.",
    # The declaration and its reader, the manifests of the trees it names, and
    # the image the release is made in.
    "extra": ["scripts/release.py", "tools/codegen/release.toml",
              "tools/codegen/trees/**", "tools/ci-images/host/**"],
    "ci_only": "release.yml runs it on a release tag, and a push would pull the "
               "host image for a declaration the workspace suite reads on every "
               "push already (a_release_declares_its_packages_by_their_trees).",
    # Not measured as a gate: release.yml's median is 13.9 min
    # (`ci_supersession_policy`), the generator's build and every tree
    # materialized and packed, and a run without a tag is the declaration's
    # read alone, seconds after the image is there. The worse is declared.
    "cost_s": 834,
    "summary": "the release declaration holds, and a tagged commit is released as declared",
    "cache": False,
    "no_cache_reason": "on a commit a release tag names it writes the release "
                       "commit, its tags and the archives, which a kept pass "
                       "would not write again",
    "inputs": None,
    "commits": [],
    "peak_gb": 5,
    "parallelism": PARALLELISM_COMPILE,
}
