#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# What the pre-push hook hands the gates: which selection the push asks for, and
# where the gates run. Sourced by `pre-push`, and by `test-prepush-gates.sh`,
# which drives both functions with a stand-in bx and a stand-in runner.
#
# ## Which selection: an update to main is the landing queue's judgment
#
# main moves only through the bx landing queue (bx RFC 0024): a GitHub ruleset
# lets the queue's integrator alone update `refs/heads/main`, and the integrator
# judges each candidate by running this hook for the landing push's own line,
# `refs/heads/main <candidate> refs/heads/main <base>`. Measured 2026-11-08: a
# landed item (cdba8e0eb0) was judged on 7 gates while 34 `ci_only` gates it
# reached ran only after it landed, and main went red. So a push that updates
# main asks for the landing selection (`--landing`): every gate the change
# reaches, `ci_only` included, less the gates hosted CI audits in its place:
# those whose platform the fleet does not have, and those whose cost the change
# sets without a bound (`audited_after_landing`). A push of any other ref keeps
# the selection it had, and the refs of the queue itself keep bx-land-refs.sh's
# rules (a genuine item is not gated at all).
#
# The flag is all this file decides. WHICH gates it selects is the registry's
# (`push_keeps` in gate_registry.py), so the hook, the runner and the plan
# cannot select differently.
#
# ## Where: bx when `$BX` names it, else the runner in place
#
# Through bx the push is the set of checks `prepush` (bx.toml), planned by the
# runner from the same selection arguments an in-place run takes.
#
# ## The status is handed through unchanged
#
# The integrator reads the hook's exit status: 0 lands the item, 1 is a gate
# that found the tree wrong, and anything else is judged again and then sent
# back as not measured. bx ends with 2 when a unit was not measured (a machine
# could not run it, or it never finished), and the runner's own 3 is a gate
# that could not run. Mapping either to 0 would land an item nobody judged, so
# `run_gates` adds no fallback, swallows nothing and pipes nothing: its status
# is bx's or the runner's.

# prepush_select_refs  (the gated refs on stdin, git's four words a line)
#   -> sets PREPUSH_SELECT_ARGS: (--landing) when one of the refs updates
#      refs/heads/main with a commit, else empty.
#
# The REMOTE ref decides, since that is what the push moves; a deletion moves
# main to nothing and publishes no commit for a gate to judge.
prepush_select_refs() {
    local _lref lsha rref _rsha
    PREPUSH_SELECT_ARGS=()
    while read -r _lref lsha rref _rsha; do
        [[ -z "${lsha:-}" ]] && continue
        [[ "$lsha" =~ ^0+$ ]] && continue
        if [[ "$rref" == "refs/heads/main" ]]; then
            PREPUSH_SELECT_ARGS=(--landing)
            return 0
        fi
    done
    return 0
}

# run_gates <runner arguments...>  -> the status of bx, or of the runner, as is.
#
# `RUNNER` is the hook's path to scripts/gate, used when `$BX` does not name an
# executable bx. The bx line is the one `gate_registry_contract` reads.
run_gates() {
    if [[ -n "${BX:-}" && -x "${BX}" ]]; then
        "$BX" --checks prepush -- bash scripts/gate --plan "$@"
    else
        "${RUNNER:?run_gates needs RUNNER, the path of scripts/gate}" "$@"
    fi
}
