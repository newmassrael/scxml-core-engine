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
# The population it answers against is `.claude/sce_checkpoints.md`, which a
# person owns. See that file for what this deliberately does not answer.

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

# Compare case-insensitively, and let a key's hyphen match a hyphen, a space,
# an underscore or nothing, so "ATOMIC-B", "Atomic B" and "atomic_b" all name
# the same checkpoint.
haystack=${proposal^^}
for key in "${keys[@]}"; do
    pattern=${key//-/[-_ ]?}
    if [[ $haystack =~ (^|[^A-Z0-9])${pattern}($|[^A-Z0-9]) ]]; then
        printf 'YES\n'
        printf '%s\n' "sce_admits: the proposal names $key" >&2
        exit 0
    fi
done

say_no "sce_admits: the proposal names none of: ${keys[*]}"
