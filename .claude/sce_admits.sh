#!/usr/bin/env bash
# successor_check for this repository's AI loop.
#
# The loop plugin calls this as an ARGV with the proposed next checkpoint
# appended as the last argument, and reads ONE capitalised word of stdout as
# YES or NO. A reply holding no such word is a silence, and a silence is not an
# admission — so this script prints exactly one word on stdout and puts every
# explanation on stderr.
#
# It replaces `claude -p`, which on run 341 (2026-09-12) answered nothing three
# times and closed a run that had finished its milestone. The slot's contract
# says every non-YES answer is a refusal, so a non-deterministic instrument in
# it does not merely degrade the loop: it stops it.
#
# The population is TWO halves, and they are different kinds of thing.
#
#   hand-written   `.claude/sce_checkpoints.md` — the RFC's atomics, a person's
#                  edit, and no predicate knows whether one is finished.
#   derived        the OPEN rows of `docs/SCE_NL_IR_CLOSURE.md`, read at call
#                  time. A row that closes stops being admissible on its own.
#
# The second half exists because the first could not answer for it. That
# ledger's fourteen rows were measured off the tree rather than taken from the
# RFC, so most of them never had a key here, and a loop that proposed one was
# refused: run 378 closed `no_successor` and run 383 spent 193 iterations
# holding a checkpoint nothing would admit. Copying the rows into the key list
# would have made two definitions of one population plus a mapping to keep — the
# drift generator this repository keeps meeting — so the rows are READ instead.
#
# ⭐ It also pays the hole the register names in its own words: *"deriving
# doneness would either need the loop to mark this file (forbidden) or a per-key
# predicate against the tree, which is worth building only once a key is
# re-proposed in practice."* That predicate exists — `scripts/gates/
# nl-ir-closure.sh` measures every row against the tree and the ledger's Status
# column is held to it — so for the derived half the answer comes from the tree
# and not from a list anybody has to remember to strike through.
#
# ⚠ The two halves fail differently ON PURPOSE. An unreadable REGISTER is fatal:
# it is the primary population and an instrument that cannot answer must not be
# the reason a run wanders. An unreadable LEDGER is not: the register still
# names a real population, so this says so on stderr and answers from that half
# alone. Refusing everything because one of two populations is missing would be
# the worse failure.

set -uo pipefail

here=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
register="$here/sce_checkpoints.md"

say_no() {
    printf 'NO\n'
    printf '%s\n' "$1" >&2
    exit 0
}

if [[ ! -r $register ]]; then
    # Refusing here rather than admitting is the whole point: an instrument
    # that cannot answer must not be the reason a run wanders.
    say_no "sce_admits: cannot read the register at $register"
fi

# The proposal is the last argument. Earlier arguments, if the driver sends
# any, are context this check does not need.
if (($# == 0)); then
    say_no "sce_admits: no proposal was passed"
fi
proposal=${*: -1}

if [[ -z ${proposal//[[:space:]]/} ]]; then
    say_no "sce_admits: the proposal was empty"
fi

# Keys are the indented entries under "## Keys": one leading token per line.
mapfile -t keys < <(
    awk '
        /^## Keys/        { inkeys = 1; next }
        inkeys && /^## /  { exit }
        inkeys && /^    [A-Z][A-Z0-9]*(-[A-Z0-9]+)*[[:space:]]/ { print $1 }
    ' "$register"
)

if ((${#keys[@]} == 0)); then
    # A register that lists nothing would admit nothing, which is correct, but
    # it is far more likely that its format drifted. Say which it was.
    say_no "sce_admits: the register lists no keys — check the '## Keys' block format"
fi

# The derived half: the OPEN rows of the closure ledger, read at call time so a
# row that closes stops being admissible without anybody striking it out.
#
# The row shape is `| <id> | open | … |`; `closed` rows are deliberately NOT
# collected, which is the doneness the hand-written half cannot answer. A
# missing ledger is not fatal — see the header.
ledger="$here/../docs/SCE_NL_IR_CLOSURE.md"
rows=()
ledger_note="the ledger at $ledger was unreadable, so only the register answered"
if [[ -r $ledger ]]; then
    mapfile -t rows < <(
        awk -F'|' '
            /^\| *[A-Z][0-9]+ *\| *open *\|/ {
                gsub(/[[:space:]]/, "", $2); print $2
            }
        ' "$ledger"
    )
    ledger_note="the ledger lists ${#rows[@]} open row(s): ${rows[*]}"
fi

# Compare case-insensitively, and let a key's hyphen match a hyphen, a space,
# an underscore or nothing, so "ATOMIC-B", "Atomic B" and "atomic_b" all name
# the same checkpoint.
haystack=${proposal^^}
for key in "${keys[@]}"; do
    pattern=${key//-/[-_ ]?}
    if [[ $haystack =~ (^|[^A-Z0-9])${pattern}($|[^A-Z0-9]) ]]; then
        printf 'YES\n'
        printf '%s\n' "sce_admits: the proposal names $key (register)" >&2
        exit 0
    fi
done

# Then the derived half. A row id is short (`C3`, `S5`), so it is matched on the
# same token boundary the keys use rather than as a substring — `C3` in `ABC3D`
# is not a proposal about row C3.
for row in "${rows[@]}"; do
    if [[ $haystack =~ (^|[^A-Z0-9])${row}($|[^A-Z0-9]) ]]; then
        printf 'YES\n'
        printf '%s\n' "sce_admits: the proposal names open ledger row $row" >&2
        exit 0
    fi
done

say_no "$(printf 'sce_admits: the proposal names neither a register key nor an open ledger row.\n  register keys: %s\n  %s' \
    "${keys[*]}" "$ledger_note")"
