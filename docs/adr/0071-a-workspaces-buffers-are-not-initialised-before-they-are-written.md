# ADR 0071 — A workspace's buffers are not initialised before they are written

- Status: Accepted (2026-10-30), on the owner's instruction to optimise the footprint structurally
  until no optimisation point is left.
- Amends: ADR 0056 (the microstep's buffers, sized by the document, in a workspace a turn lends)
  and ADR 0068 (one workspace per turn, in `Engine::run_turn`).
- Evidence: `backends/rust/probes/nostd-footprint`; a consumer's stack bound, whose path runs
  through `run_turn`.

## Context

`Workspace::new` filled the enabled set, the walk's stack and the history defaults with
placeholders. The placeholder walk step, `WalkStep::Descendants(Target::state(StateId(0)))`, is
not zero bytes in Rust's enum layout. So the compiler built the filled walk in a temporary on
`run_turn`'s stack, one slot per loop iteration, and copied it into the workspace. Every machine's
`run_turn` carried that loop and copy, and its frame held the walk twice.

None of those slots is read before it is written. A `SliceVec` reads only the prefix it pushed.

## Decision

1. `SliceVec<'a, T>` holds `&'a mut [MaybeUninit<T>]`, the way `heapless::Vec` holds its
   storage. Its invariant is `len <= slots.len()`, with `slots[..len]` initialised. `push`
   writes a slot; `pop`, `Deref` and `DerefMut` read only the prefix. Every `unsafe` of the
   microstep module is in these methods and rests on that invariant. The items are `Copy`, so
   nothing forgotten is owed a drop.
2. The `Workspace` keeps the enabled set, the walk and the history defaults as `MaybeUninit`
   slots, left uninitialised by `Workspace::new`. The words of the sets are still zeroed, because
   a `StateSet` reads them as words.
3. `Buffers::EMPTY` is the empty workspace as a constant, and `run_turn` starts from it. A turn
   writes the zeroed words and nothing else.
4. A walk that lends its empty stack's slots to another walk (`SliceVec::slots_mut`) also sets
   its length to zero. Whatever the borrower leaves in a slot is then never read as an item, in a
   release build as in a debug one.
5. `WalkSlots` names `[MaybeUninit<WalkStep>]`, the slots a walk keeps its steps in, in the
   signatures that lend them.

## Measured (2026-10-30)

| | one machine | two machines |
|---|---|---|
| diagnostics on | 194,117 -> 194,005 | 195,831 -> 195,697 |
| diagnostics off | 193,913 -> 193,801 | 195,573 -> 195,439 |
| plain documents | 190,966 -> 190,874 | 192,462 -> 192,334 |

Worst-case stack: 1,120 -> 1,032 bytes with the features (`run_turn` 328 -> 240), 928 -> 880
without (240 -> 192). The core is 62 bytes smaller in the feature configurations and 58 in the
plain one.

## Consequences

- `SliceVec::new`, `Scratch::walk`, `Scratch::history_defaults`, `effective_target_states_on`,
  and the `<history>` procedure that computes effective targets take `MaybeUninit` slots. A
  caller outside the crate that lends its own slots passes `MaybeUninit` arrays.
- `{:?}` of a `SliceVec` lists the items held, not the slots.
