# ADR 0144 — A generated policy reads a state, a history and an event back from its number by a match the compiler folds to the identity

- Status: Accepted (2026-11-26), under the owner's standing rule: the most long-term correct, textbook structure,
  decided by measurement (section "Measured").
- Amends: nothing decided. ADR 0064 (a state, a history and an event are the numbers the tables know them by) and
  ADR 0130 section 4 (each enum's discriminant is that number) stand; this decision reads them back by that fact.
- Related: ADR 0123 (the hooks are handed numbers), ADR 0131 (the engine's state apart from the policy).
- Requested by: a downstream bare-metal consumer, 2026-11-26 (instructions per received message).

## Context

The engine keeps the current state as a number (`EngineCore::current_state`), and `Engine::get_current_state` answers
`P::state_of(number)`. The trait's default `state_of` reads `STATES[number]`, the table of variants in number order.
Every generated enum already has each variant's discriminant equal to its number (ADR 0130 section 4), so the table
is the identity, but the compiler cannot know that from a read of a table: each `get_current_state` was a bound test,
a load from the table and, at a caller that compares the state with a variant, a test of the loaded byte.

The consumer's link and session machines are asked their state several times in every turn of its receive path.
On its receive harness the reads of `STATES` came to about 16 instructions a message.

## Decision

A generated policy writes `state_of`, `history_of` and `event_of` as a match of each number to its variant, with
`helpers::tables::row_missing` for a number past the last, as the trait's defaults stop there. The compiler reads a
match whose every arm answers the variant whose discriminant is that arm's number as the identity with one bound,
so `get_current_state() == S::Established` is a comparison of the number with a constant, and a `match` on the state
is a switch on the number.

The trait's defaults stay for a policy written by hand. `helpers::tables::row_missing` is public and hidden from the
documentation, so generated code stops where the runtime's own reads stop.

## Measured

On the consumer's receive harness (`count_sub` under callgrind, 6,000 messages against 1,000, a reproduction of its
library with this repository's main and the consumer's two receive-path machines written as this decision writes
them), together with the end of a frame of ADR 0143's amendment of this day: 4,258.4 instructions a message to
4,237.8 (-20.6). The lines of `policy.rs` left the message's path, and `get_current_state` is 3 instructions where it
was 6, besides the 14 of the table reads. The nostd footprint probe's figures did not change.

## Alternatives considered

- **Keep the typed state in the engine beside its number**, updated where the number changes: one load to read it,
  but two records of one fact, each path that moves the state (a turn, a start, a restore) bound to keep them
  together. Refused.
- **Convert the number with `transmute`** under the discriminant's guarantee: the same code, by `unsafe`. Refused.
