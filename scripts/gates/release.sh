#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: release.yml
#
# The release of the commit checked out (ADR 0097, section 7), in the host image
# (ADR 0145): the generator the release materializes every tree with is built
# there, and the formatters it writes them through are the image's, so a release
# is made with the packages the gates judge and not with what a runner carries.
#
# The verdict is the declaration's: `scripts/release.py --check` holds the
# packages tools/codegen/release.toml declares to the tree manifests. When a
# release tag (v<major>.<minor>.<patch>) names HEAD, as on the tag push
# release.yml runs on, the release is made too, into build/release: the
# materialized trees, the archives, the release commit with its tags, and
# release.json, which the workflow's later steps publish. The release commit
# and its tags are written into this repository and pushed by nobody here.
# Without a tag nothing is made, so a run anywhere else, a landing among them,
# reads the declaration and writes nothing.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

python3 scripts/release.py --check \
    || sce_gate_fail "tools/codegen/release.toml does not declare its packages by their trees (scripts/release.py --check says which)"
sce_gate_step "the release declaration holds"

mapfile -t TAGS < <(git tag --points-at HEAD --list 'v*.*.*')
if (( ${#TAGS[@]} == 0 )); then
    sce_gate_step "no release tag names HEAD, so no release is made"
    exit 0
fi
(( ${#TAGS[@]} == 1 )) \
    || sce_gate_fail "${#TAGS[@]} release tags name HEAD (${TAGS[*]}): one commit is released under one version"
TAG="${TAGS[0]}"

python3 scripts/release.py --check --tag "$TAG" \
    || sce_gate_fail "$TAG does not match the workspace's version (scripts/release.py --check --tag $TAG says how)"

CODEGEN="$(sce_gate_codegen)" \
    || sce_gate_cannot_run "could not provide sce-codegen, which materializes every tree of the release"

# An empty directory is what the pipeline writes a release into; a release left
# from an earlier run is not this one.
OUT="$SCE_REPO_ROOT/build/release"
rm -rf "$OUT"
python3 scripts/release.py --tag "$TAG" --out "$OUT" --codegen "$CODEGEN" \
    || sce_gate_fail "the release of $TAG could not be made (scripts/release.py says where it stopped)"
sce_gate_step "released $TAG into build/release"
