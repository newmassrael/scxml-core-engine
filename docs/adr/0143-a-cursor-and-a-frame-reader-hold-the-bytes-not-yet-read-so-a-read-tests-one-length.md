# ADR 0143 — A cursor and a frame reader hold the bytes not yet read, so a read tests one length

- Status: Accepted (2026-11-26), under the owner's standing rule: the most long-term correct, textbook structure,
  decided by measurement (section "Measured").
- Amends: nothing decided. ADR 0128 (a frame is read by a total reader that records its first refusal), ADR 0137
  (the refusal of a read is one call, out of line) and ADR 0138 (one ZInt read out of line, fifth amendment) keep
  what they decide; only how a reader states where it stands changes.
- Related: ADR 0130 (one decode of a ZInt for both readers), ADR 0136 (a body's reads out of line).
- Requested by: a downstream bare-metal consumer, 2026-11-26 (its size ledger names the reader's code inlined in the
  decoders as the largest part of the codec area it still pays for).

## Context

`SceCursor` held its input and a position (`buf`, `pos`), and `SceReader` holds a cursor as its place in its input
(ADR 0138, fifth amendment). A read of `N` fixed bytes was `buf.get(pos..).and_then(first_chunk::<N>)`: the
compiler cannot know that `pos` never passes `buf.len()`, so each read tested the position against the length and
then the bytes left against `N`, two comparisons, each with its own branch to the read's refusal. A read of a
length known only when it runs computed `len - pos` and compared again. A decoder's body inlines every one of its
reads, so the consumer's client library held about 3.2 KB of the reader's code in its decoders
(`sce_forge_runtime::reader` in its ledger: `take_array` 967 bytes, `take_slice` 549, `rest` 229 by the line
tables of a reproduction of its build).

## Decision

A cursor holds the whole input and the bytes of it not yet read: `SceCursor { buf, rest }`, `rest` always a suffix of
`buf`. Every read is a test of `rest` alone and a move of its start:

- `SceReader::take_array::<N>` is `rest.split_first_chunk::<N>()`, `take_slice(n)` is `rest.split_at_checked(n)`,
  `peek_u8` is `rest.first()`, `advance(n)` is `rest.get(n..)`; the cursor's `peek_array`, `peek_slice`,
  `peek_u8` and `advance` read `rest` the same way. One comparison and one branch to the refusal, and no index
  the compiler cannot prove, so no panic path.
- A position is counted, not stored: `buf.len() - rest.len()` (`SceCursor::position`, `SceReader::position` and
  `mark`). `rewind(at)` sets `rest` to `buf.get(at..)`; `since(at)` is `buf.get(at..position)`.
- A refusal ends the input by making `rest` empty, as it set the position to the end before.
- `codec::read_zint_short` reads the one or two bytes at the start of `rest` and moves it; the cursor's one ZInt
  read runs `read_zint` over `rest` from its start.

No public declaration changes: the cursor's and the reader's methods keep their names, types and answers, and the
fields were the crate's own.

## Measured

On a reproduction of the consumer's `pico-client-coop-nostd` library (its crates at the pin of 2026-11-26, fat LTO,
one codegen unit, `opt-level = 3`, x86-64), `.text` by the line tables, each byte to the innermost source outside
Rust's standard library:

| | `.text` | `forge-runtime/src/reader.rs` |
|---|---|---|
| main before this decision (ADR 0138 fifth amendment) | 225,299 | 3,203 |
| this decision | 224,967 | 3,015 |

332 bytes smaller, spread over the decoders and the consumer's own readers (`read_control_in_place` -160,
`InitBodyIn::read_into` -96, the decoders of `timestamp`, `wireexpr`, `encoding`, `transport_close` and others
-16 to -22 each). A read of a fixed field on the receive path executes fewer instructions (one comparison where there
were two, and no reload of the position); the consumer measures its receive path at the landing.

## Alternatives considered

- **The read's refusal recorded in place, with no call** (a flag that the input ended inside a read, set with the
  empty `rest` by two stores, `NeedMoreBytes` the frame's refusal unless one came before it). Measured on the same
  reproduction: 121 bytes further down in total, but the bodies of the receive path (`read_push`, `read_declare`)
  grew by 16 to 32 bytes each as the compiler rearranged their refusals, and the reader would hold its refusal in two
  forms. Not taken; ADR 0137's one call stands.
- **The position kept and the invariant `pos <= buf.len()` stated to the compiler** with an unchecked slice. Needs
  `unsafe` in every read for what the slice states by its type. Refused.
- **A pointer pair in place of `buf`** (`start`, `rest`): one word less, but `rewind` and `since` would rebuild a slice
  from a pointer, `unsafe` again. Refused.
