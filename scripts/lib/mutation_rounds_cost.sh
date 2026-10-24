# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# The cost model a mutation-rounds job is scheduled by, and the arithmetic
# that turns it into a slice and a promise. Sourced by
# `scripts/gates/mutation-rounds.sh`; side-effect free, so that
# `mutation_corpus_fits_its_lane` can ask the same functions the gate asks.
#
# A round is a rebuild and a test run PER CASE, so a job's cost is
# `setup + cases x per-case`, and both terms are properties of the RUNNER.
#
# What is written down is the COST, not the count. The count used to be a
# constant — three ctest cases a job — chosen against a cost measured once, on
# 2026-08-25, and kept in a test rather than beside it. The tree then grew and
# nothing re-measured: on runs 39782655402 and 39948952108 (2026-10-22/23) a
# ctest job spent up to 98 minutes before its baseline and up to 176 on one
# case, so three cases were 626 minutes against 330, and every C++ casefile of
# more than one case ended `cancelled` while the arithmetic that admitted it
# still passed. A count cannot notice that its premise moved. So the cost is the
# input, the slice is derived from it and from the ceiling the workflow
# declares, and the gate compares every job's own duration with what this model
# promised it (`mutation_rounds_overran`), so a model that has fallen behind the
# tree turns a job red, by name, while the job still finishes.

# `setup per-case`, in minutes, by runner; the worst measured, rounded UP:
#
#   ctest  100 180   setup 98 (`a_static_cpp_a_64_bit_integer_crosses_as_its_
#                    digits_ctest`, 2026-10-23); one case 176 (`a_static_cpp_a_
#                    machine_hands_its_state_to_an_algorithm_ctest`,
#                    2026-10-22). A template mutation regenerates every machine
#                    the target compiles.
#   *       15  13   `mutation_rounds_selection`, 21 cargo cases in 4h16m:
#                    about 12.2 each, because its cases drive whole gate
#                    scripts. `go`, `pytest` and `gradle` are carried by this
#                    row; the honest cost of an unmeasured runner is the
#                    worst measured one.
mutation_rounds_cost() {
    case "$1" in
        ctest) printf '100 180\n' ;;
        *) printf '15 13\n' ;;
    esac
}

# The share of the ceiling a job is scheduled into. The rest is where a job
# that ran past its model is reported before the platform cancels it: a
# schedule that fills the ceiling leaves a drift no room to be reported in, and
# the model is only ever as current as its last measurement.
MUTATION_ROUNDS_BUDGET_PERCENT=85

# The `timeout-minutes:` the rounds job of workflow `$1` declares, read from
# the `rounds:` job on: the selection job above it declares its own. Prints
# nothing for a workflow it cannot read, which the gate refuses by name.
mutation_rounds_ceiling() {
    [[ -r "$1" ]] || return 0
    awk '$0 == "  rounds:" { inside = 1 }
         inside && $1 == "timeout-minutes:" { print $2; exit }' "$1"
}

# The minutes of ceiling `$1` a job is scheduled into.
mutation_rounds_budget() {
    printf '%s\n' "$(( $1 * MUTATION_ROUNDS_BUDGET_PERCENT / 100 ))"
}

# How many `$1` cases one job may hold under ceiling `$2`: what the budget
# leaves after setup, divided by one case. Zero means no slice fits, which the
# gate refuses before it schedules anything.
mutation_rounds_cases_per_job() {
    local setup per_case budget
    read -r setup per_case <<<"$(mutation_rounds_cost "$1")"
    budget="$(mutation_rounds_budget "$2")"
    if (( budget <= setup )); then
        printf '0\n'
        return
    fi
    printf '%s\n' "$(( (budget - setup) / per_case ))"
}

# The minutes the model promises a job of a `$1` casefile holding `$2` cases,
# run as one of `$3` shards: setup and the largest slice. `scripts/mutate
# --shard` spreads the remainder over the leading shards, so the largest is the
# ceiling of the division.
mutation_rounds_promise() {
    local setup per_case shards="${3:-1}"
    read -r setup per_case <<<"$(mutation_rounds_cost "$1")"
    (( shards >= 1 )) || shards=1
    printf '%s\n' "$(( setup + ($2 + shards - 1) / shards * per_case ))"
}

# Whether a job that started at epoch second `$1` and ends at `$2` ran past
# `$3` promised minutes. Prints the minutes it took, rounded UP so that a job
# is never read as inside a promise it overran by seconds; returns 0 when it
# overran and 1 when it did not.
mutation_rounds_overran() {
    local took="$(( ($2 - $1 + 59) / 60 ))"
    printf '%s\n' "$took"
    (( took > $3 ))
}
