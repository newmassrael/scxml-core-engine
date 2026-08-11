#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: sce-forge-codec-no-alloc.yml
#
# The complement of `codec-clippy`: the same generated codecs compiled
# `--no-default-features`, proving the borrowed zero-copy path is reachable on
# a heap-free MCU target. Scalar codecs carry `&'a [u8]` / `&'a str` views and
# list-bearing ones use bounded `heapless::Vec`, so every `Vec` / `String` /
# `encode_to_vec` site has to stay behind `#[cfg(feature = "alloc")]` for this
# to hold.
#
# Same reason it needed a local mirror as its sibling: the codec goldens are
# not workspace members, so no other gate compiles them at all. The round that
# motivated the check was a downstream codec crate failing
# `--no-default-features` — a consumer finding the defect first.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

./scripts/check_noalloc_codec.sh \
    || sce_gate_fail "generated codecs no longer compile without alloc — an alloc-only item escaped its cfg gate"
