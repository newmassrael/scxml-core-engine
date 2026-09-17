#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Mirrors: license-verify.yml
#
# Every third-party library the visualizer ships must be registered, and must
# carry its licence text.
#
# ## The gap this closes, stated as it was found
#
# `license-ssot.sh` guards `sce/sce_licenses.cmake` against `third_party/` —
# the C++ distribution — and looks at nothing under `web/`. So the browser
# visualizer accumulated two libraries that no check could see: d3 (ISC) and
# elkjs (EPL-2.0, fetched from unpkg at runtime). Meanwhile
# `LICENSE-THIRD-PARTY.md` opened with "All dependencies are MIT licensed",
# and nothing in the tree could contradict it.
#
# ⚠ The dangerous half is not the stale sentence. It is that a dependency
# whose licence obliges the DISTRIBUTOR — EPL-2.0 section 4 obliges a
# commercial one to defend and indemnify every other contributor — could be
# added to a shipped page by one edit to a `<script src>`, with no review
# step anywhere that would notice. That is a compliance verdict reachable
# only by a human happening to look.
#
# ## What is asked, and why it is asked this way
#
# ⭐ ASKED, not named. The gate does not carry a list of expected libraries:
# a list is a second copy of the registry and drifts from it. It reads what
# is ON DISK under the vendor directory and requires the registry to account
# for each one. A library added without an entry fails; an entry for a
# library since deleted fails too, in the other direction.

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

VENDOR_DIR="web/visualizer/vendor"
REGISTRY="LICENSE-THIRD-PARTY.md"

[[ -f "$REGISTRY" ]] || sce_gate_fail \
    "$REGISTRY is missing — the registry this gate holds the tree to does not exist"

# No vendor directory is a legitimate state (nothing vendored yet), and is
# not the same as an empty one that a mistake produced.
if [[ ! -d "$VENDOR_DIR" ]]; then
    printf 'web-vendor-licenses: no %s — nothing vendored for the web, nothing to register.\n' \
        "$VENDOR_DIR"
    exit 0
fi

vendored=()
while IFS= read -r dir; do
    [[ -n "$dir" ]] && vendored+=("$(basename "$dir")")
done < <(find "$VENDOR_DIR" -mindepth 1 -maxdepth 1 -type d | sort)

if [[ ${#vendored[@]} -eq 0 ]]; then
    sce_gate_fail \
        "$VENDOR_DIR exists but holds no library directory.
  A vendored library lives in its own directory beside its licence text; a bare
  file dropped here carries none and cannot be registered by this gate."
fi

failures=0
for name in "${vendored[@]}"; do
    dir="$VENDOR_DIR/$name"

    # The licence text must travel with the code. For EPL-2.0 this is not
    # bookkeeping: section 3 forbids removing notices, and a copy shipped
    # without its licence has removed the largest one.
    # ⚠ Double quotes, not single, and not a style choice. `script-read-coverage`
    # tells a gate's inputs from its prose by the quoted region: a value is a
    # quoted region that IS a tracked path, prose is one that merely contains
    # a name among other words. Single quotes are not a region it models, so a
    # message written in them hands it the bare word LICENSE — a tracked file
    # at the repository root — and the self-test then demands this gate declare
    # a file it never opens.
    if [[ ! -f "$dir/LICENSE" ]]; then
        printf "ERROR: %s has no LICENSE file\n" "$dir" >&2
        failures=$((failures + 1))
    fi

    # Provenance: which version, from where, and a hash a reviewer can check
    # the bytes against. Without it "unmodified upstream copy" is a claim
    # nobody can test.
    if [[ ! -f "$dir/README.md" ]]; then
        printf "ERROR: %s has no README.md recording version, upstream source and hash\n" "$dir" >&2
        failures=$((failures + 1))
    fi

    # And the registry must account for it by name.
    if ! grep -qiF "$name" "$REGISTRY"; then
        printf 'ERROR: %s is vendored but %s never mentions it\n' "$dir" "$REGISTRY" >&2
        failures=$((failures + 1))
    fi
done

# The other direction. A registry naming the vendor path of a library that
# is no longer there is as wrong as a library nobody registered — it tells a
# reader SCE ships something it does not.
while IFS= read -r claimed; do
    [[ -z "$claimed" ]] && continue
    if [[ ! -d "$VENDOR_DIR/$claimed" ]]; then
        printf 'ERROR: %s points at %s/%s, which is not in the tree\n' \
            "$REGISTRY" "$VENDOR_DIR" "$claimed" >&2
        failures=$((failures + 1))
    fi
done < <(grep -oE "web/visualizer/vendor/[A-Za-z0-9._-]+" "$REGISTRY" \
    | sed 's|web/visualizer/vendor/||' | sort -u)

# ⚠ The page is what actually ships. A `<script src>` reaching out to a CDN
# puts a third party's code in every viewer's browser while leaving the
# vendor directory — and therefore this gate's other checks — untouched.
# That is exactly how elkjs arrived and stayed unregistered for as long as
# it did.
if remote=$(grep -nE '<script[^>]+src="https?://' web/visualizer/*.html 2>/dev/null); then
    printf 'ERROR: the visualizer loads a script from a remote origin:\n%s\n' "$remote" >&2
    printf '  Vendor it under %s/ and register it, or the code ships unregistered.\n' "$VENDOR_DIR" >&2
    failures=$((failures + 1))
fi

if [[ $failures -gt 0 ]]; then
    sce_gate_fail \
        "$failures problem(s) above. Every library the visualizer ships must sit in
  $VENDOR_DIR/<name>/ beside its licence text and a readme recording version, upstream
  source and hash, and must be registered in $REGISTRY."
fi

printf "web-vendor-licenses: OK — %d vendored librar%s, each with its licence text, a readme and a registry entry.\n" \
    "${#vendored[@]}" "$([[ ${#vendored[@]} -eq 1 ]] && printf 'y' || printf 'ies')"
