#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Rust mod.rs <-> subdirectory drift.
#
# Catches the failure mode that motivated the integration/ tree split: a
# `pub mod X;` in an aggregator mod.rs whose `X/` subdir is missing (rustc
# E0432 "unresolved import"), or an `X/` subdir on disk the aggregator
# forgot to `pub mod` (silently dead code, or a downstream E0432 from a test
# that imports it). Either way `workspace-tests` catches it eventually — but
# only after ~30s of compilation; this structural check fails in under a
# second with a diff that points at the offending file.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

check_mod_tree() {
    local mod_file="$1"
    local parent_dir
    parent_dir="$(dirname "$mod_file")"
    [[ -f "$mod_file" ]] || { printf '  SKIP: %s (missing)\n' "$mod_file" >&2; return 0; }

    # `pub mod X;` lines in mod.rs (ignore commented-out / nested forms —
    # the aggregator convention is a flat top-level list).
    local listed
    listed="$(grep -E '^pub mod [a-zA-Z_][a-zA-Z0-9_]*;' "$mod_file" \
        | sed -E 's/^pub mod ([a-zA-Z_][a-zA-Z0-9_]*);.*/\1/' \
        | sort)"

    # Subdirectories sitting next to mod.rs. Every codegen-output subdir
    # here is a Rust module by convention (carries its own mod.rs). A future
    # non-module sibling dir would need a small exclusion list; flag it
    # loudly rather than ignore silently.
    local on_disk
    on_disk="$(find "$parent_dir" -mindepth 1 -maxdepth 1 -type d -printf '%f\n' | sort)"

    # Symmetric diff: mod.rs entries with no dir, then dirs with no entry.
    local listed_only on_disk_only
    listed_only="$(comm -23 <(echo "$listed") <(echo "$on_disk"))"
    on_disk_only="$(comm -13 <(echo "$listed") <(echo "$on_disk"))"
    if [[ -n "$listed_only" || -n "$on_disk_only" ]]; then
        printf '  FAIL: %s\n' "$mod_file" >&2
        [[ -n "$listed_only" ]] && {
            printf '    pub mod entries with no matching subdir (rustc E0432):\n' >&2
            printf '      %s\n' $listed_only >&2
        }
        [[ -n "$on_disk_only" ]] && {
            printf '    subdirs not registered in mod.rs (orphaned modules):\n' >&2
            printf '      %s\n' $on_disk_only >&2
        }
        return 1
    fi
    return 0
}

drift_count=0
for mod_file in \
    backends/rust/tests/src/generated/mod.rs \
    backends/rust/tests/src/integration/mod.rs; do
    check_mod_tree "$mod_file" || drift_count=$((drift_count + 1))
done
(( drift_count == 0 )) || sce_gate_fail "$drift_count mod.rs drift"
