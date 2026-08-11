#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: embed-vendor-smoke.yml
#
# emit_embed_manifest.sh fail-fast regression case.
#
# Reproduces the May 2026 embed-vendor-smoke CI failure where
# emit_embed_manifest.sh silently consumed clang's degraded AST when
# JSEngine.h's transitive quickjs.h was unresolvable on the runner. The
# result: a manifest that was a function of the parse environment (locally
# clean, CI broken) instead of the source tree, and a
# verify_embed_manifest.sh diff with ghost duplicate symbols carrying
# placeholder types like `int` for `std::vector<std::string>`.
#
# The fix moved emit to fail-fast on any clang parse error. This case plants
# a synthetic header that includes a nonexistent file, runs emit, and
# asserts non-zero exit + diagnostic present + no manifest written. It
# catches the silent-degrade pattern's reintroduction before any developer
# pays the CI round-trip.
#
# Distinct from `embed-vendor`, which covers the orthogonal drift path: a
# committed manifest that no longer matches the current header surface.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

bash scripts/test_emit_manifest_fail_fast.sh \
    || sce_gate_fail "emit_embed_manifest fail-fast case"
