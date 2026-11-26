#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: mutation-rounds.yml
#
# The Swift rounds of the mutation corpus: the casefiles whose runner is
# `mutation_swift`, each a Swift package built and its tests run once per case.
# A gate runs in one image (ADR 0145, D4), and those rounds need the Swift
# toolchain, which is the swift image's and not the host image's that
# `mutation-rounds` runs in. So this gate is `mutation-rounds` run in the swift
# image over the Swift share of the same selection: the script is that gate's,
# sourced with the family `swift`, so the two choose by one set of rules and
# neither restates the other ("Two gates over one corpus" in
# scripts/gates/mutation-rounds.sh).

source "$(dirname "${BASH_SOURCE[0]}")/mutation-rounds.sh" swift
