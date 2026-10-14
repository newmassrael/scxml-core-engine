#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: targets.yml
#
# A document and its binding are split into a logical interface every target shares and a
# profile only one platform needs; the standalone target builds a C++ library from the first
# alone. Two things are worth a gate. The interface carries nothing of a platform (the schema
# refuses an address, a record field, a number for a symbol), and the library built from it
# fails the same cases `verify` fails -- a library that passed where `verify` failed would read
# the interface differently from the core, and nothing downstream could tell.
#
# The second needs the product's generator and a C++ compiler, and a case that cannot run
# skips, and a skip reads as a pass. So this gate refuses to run without either: they are
# built or installed one step up, and absence here is a broken step, not a reason to judge less.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

python3 -c 'import yaml, jsonschema' \
    || sce_gate_fail "targets needs PyYAML and jsonschema"

codegen="${SCE_CODEGEN:-target/debug/sce-codegen}"
[ -x "$codegen" ] \
    || sce_gate_fail "targets needs the product's generator at $codegen (cargo build --bin sce-codegen --features cli -p sce-build)"

command -v g++ >/dev/null 2>&1 || command -v c++ >/dev/null 2>&1 \
    || sce_gate_fail "targets needs a C++ compiler to build the library it judges"

( cd tools/targets && PYTHONPATH=.:../authoring python3 -m unittest discover -s tests -t . ) \
    || sce_gate_fail "targets regression (a platform in the interface, or a library that judges unlike verify)"
