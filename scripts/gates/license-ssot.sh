#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: license-verify.yml
#
# Drift guard for `sce/sce_licenses.cmake`, the SSOT both distribution
# channels read: the find_package install rules and `scripts/package_embed.sh`.
# It catches a vendored third_party dependency added without its LICENSE
# entered into the SSOT, an upstream upgrade that moves an embedded SPDX or
# copyright notice out of the registered grep target, and SSOT entries
# pointing at files renamed or deleted on disk.
#
# There was no local mirror of this until the registry started asking which
# workflows lack one. The verdict is a compliance verdict — an LGPL section 1
# or MIT section 1 violation ships in a release tarball — and it was reachable
# only after a push, on a check that needs nothing but bash, grep, find and
# sed and takes three hundredths of a second.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

./scripts/verify_licenses.sh \
    || sce_gate_fail "license SSOT drift — sce/sce_licenses.cmake no longer describes what the tree ships"
