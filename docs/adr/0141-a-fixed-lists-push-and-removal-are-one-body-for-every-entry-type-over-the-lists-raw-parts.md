# ADR 0141 — A fixed list's push and removal are one body for every entry type, over the list's raw parts

- Status: Accepted (2026-11-26), under the owner's standing rule: the most long-term correct, textbook structure,
  decided by measurement. The figures in "Context" are the consumer's reading of its size ledger of 2026-11-25; the
  effect is measured by the consumer at the landing (section "Measured").
- Amends: nothing decided. ADR 0077 section 7 (an entry past a list's capacity is read and not held, and the keep
  rule of the kinds a gate asks for) is unchanged in what it holds and refuses; only where its code is compiled moves.
- Related: ADR 0065 section 6 R6 (the kinds a chain's gates ask for), ADR 0136 (a chain is read by one routine, each
  test of a kind a call by name), ADR 0138 (one function out of line where a generic copy bought nothing).
- Requested by: a downstream bare-metal consumer, 2026-11-26.

## Context

A generated decoder that reads a list field holds each entry it reads through
`codec::push_keeping<L: HeldList, Asked>` (`backends/rust/forge-runtime/src/codec.rs`), which calls the list's
`SceList::is_full`, `try_push`, `as_slice` and `remove_at` (`storage.rs`). On the inline profile every list is a
`heapless::Vec<T, N>`, whose `push` and `remove` are generic over both `T` and `N`, so a program compiles one
copy of each for every pair of entry type and capacity it decodes. In the consumer's set of generated codecs 37 call
`push_keeping`. Its bare-metal client image reads four generated declaration decoders, each holding a `heapless`
push of about 91 bytes, besides the pushes inlined into its own bodies.

Nothing in a push depends on `T` but the entry's size and its drop: the test of the length against the capacity,
the address of the slot, a copy of `size_of::<T>()` bytes and the length moved by one. A removal is the same plus a
copy of the tail down one slot. The one typed part of the keep rule past the capacity is the gates' tests of a kind,
which ADR 0136 makes calls by name.

## Decision

### 1. One body for each operation, generic over nothing

The fixed-capacity list's two operations with a body are each one function of the runtime
(`backends/rust/forge-runtime/src/fixed_list.rs`), not generic and `#[inline(never)]`, over the list as its raw
parts: the address of its first slot, its length, its capacity and the size of an entry.

    unsafe fn push(slots: *mut u8, len: usize, capacity: usize, size: usize, entry: *const u8) -> bool;
    unsafe fn remove(slots: *mut u8, len: usize, size: usize, at: usize, out: *mut u8);

- `push` copies the `size` bytes at `entry` into the slot at `len` and answers `true`, or, where `len` is not
  below `capacity`, copies nothing and answers `false`.
- `remove` moves nothing and answers `false` where `at` is not below `len` (no panic path); otherwise it copies the slot at `at` into
  `out` and the `len - at - 1` slots after it down one slot.
- Neither sets the length: `heapless::Vec`'s length is a private field of a `repr(Rust)` struct, so its only
  address is `set_len`, one store, which the typed impl makes.

This is the split `hashbrown` makes between `RawTableInner`, one body per operation for every key and value type,
over a layout it is handed, and `RawTable<T>`, the typed casts around it.

### 2. The typed impl is the casts around them

`SceList<T> for heapless::Vec<T, N>` moves from `storage.rs` to `fixed_list.rs`, beside the bodies whose
preconditions it meets:

- `try_push` holds the entry in `ManuallyDrop`, calls `push` with `as_mut_ptr`, its length, `N` and
  `size_of::<T>()`, and on `true` calls `set_len(len + 1)`; on `false` it drops the entry it still owns and
  answers `CapacityExceeded`, as before.
- `remove_at` calls `remove` into a `MaybeUninit<T>` of its frame, calls `set_len(len - 1)`, and then drops the
  entry it took out.
- `empty`, `clear`, `as_slice`, `last_mut` and `is_full` are a field access or the vector's own drop of its
  entries, which is typed by nature, and stay as they were.

### 3. The keep rule is one call off the common path

`push_keeping` keeps its signature. Where the list has room it is `is_full` (a compare with `N`) and `try_push`,
one call. Past the capacity it notes `full` and, where the gates ask a kind (`Asked::COUNT > 0`, a constant),
makes one call of `keep_past_capacity<L, Asked>`, `#[inline(never)]`, which holds the search of ADR 0077 section 7
unchanged: still typed, each test of a kind a call by name (ADR 0136). Where no gate asks a kind there is no call and
the entry is dropped in place.

### 4. Why it is sound for every `T`

- An entry is moved as its bytes and then not used: the impl holds it in `ManuallyDrop`, so after `push` the list
  is its one owner; on a refusal nothing was copied and the impl drops it once. An entry given up by the keep rule
  is dropped once, by `remove_at`, after the list's length counts only what it holds, so a `Drop` that panics
  leaves no entry counted twice. No path drops an entry twice or leaks one.
- Every slot is `index * size` bytes from the first, inside the buffer of `N * size` bytes, which a type's size
  keeps at most `isize::MAX`: no product overflows. A size is a multiple of its alignment, so every slot is aligned
  as the first is; an over-aligned `T` needs nothing more, and the copies are of bytes and ask no alignment.
- A zero-sized `T` is a copy of no bytes from and to aligned non-null addresses, and the length counts it.
- `ptr::copy` and `ptr::copy_nonoverlapping` are untyped: padding, uninitialised bytes and the provenance of a
  reference an entry holds (a list on the `Lent` profile holds the frame's slices) move with it.
- The slot pointer is taken from `as_mut_ptr` after the length is read and is not used after `set_len`, so no
  reference to the vector is made while it is live.

Each `unsafe` block states its argument in place. The module's unit tests cover a push, a push past the capacity
with and without a keep rule (an entry given up and one not held), a keep rule that finds no entry to give up,
`remove_at` at the first, a middle and the last slot, a removal past the end, a list of `Drop` entries dropped
whole, zero-sized entries with and without drop glue, an entry aligned to 64 and an entry that borrows. Each entry
with drop glue counts its drops, so a double drop or a leak is a failed assertion; `scripts/gates/forge-rust.sh` runs
them under Miri (`cargo miri test -p sce-forge-runtime --lib fixed_list::`), where a misaligned access, a read
without provenance or a use of a moved-out slot is reported as undefined behaviour.

## Consequences

- No byte decoded or refused changes: an entry is held, refused or kept past the capacity exactly as before. The
  generated code calls the same functions, so no golden and no committed tree of generated code changes.
- No public declaration changes: the bodies and `keep_past_capacity` are the crate's own, and the impl's methods
  keep their signatures.
- The heap containers (`alloc::vec::Vec`, `CappedVec`) keep their present form: they are not on the bare-metal
  image, and their push is the allocator's growth, which no raw body shares.
- A program that holds lists of several entry types or capacities compiles one push and one removal for all of
  them; each push site is the length, the call, a store of the length and, on the refusal, the entry's drop.
- Each entry pushed costs one call and a copy whose length is an argument (on a target with no copy loop of its own,
  a call of `memcpy`), where the typed push copied a constant size in place: a few instructions per entry on the
  receive path, against a body per entry type in the image.

## Alternatives considered

- **Keep `heapless`'s push and removal.** A body for every pair of entry type and capacity. Refused: nothing in them
  is typed but the size.
- **A body generic over `T` and not `N`.** Still one per entry type, and the size is the only thing the type gives.
  Refused.
- **The parts as one struct handed to the body.** The Rust ABI passes an aggregate wider than two words by
  reference, which is a store of each part at every site. Refused: the parts are scalars in registers.
- **A list type of the runtime's own, whose length the body writes.** The inline profile's list type is named in
  every generated tree and in every deployment's file (`heapless::Vec<T, N>`); replacing it is a change to the
  generated code and to the consumer's surface for one store saved per push. Refused.
- **The keep rule type-erased too, its tests of a kind handed as function pointers.** An indirect call per test,
  which a stack bound cannot follow (ADR 0136). Refused: the rule is reached by one call, off the common path.

## What would show it wrong

At a small `T`, the site of a call (the parts in registers, the length stored after it and the refused entry's
drop) may not be smaller than the typed push it replaces, and a large share of pushes may make the call and the
copy of a runtime length visible on the receive path. The consumer's ledger before and after the landing decides,
read by owner (`sce_forge_runtime`, `heapless`) and by the four decoders' bodies; a result against this ADR
amends it by measurement.

## Measured

To be measured by the consumer at the landing, on the same image and by the same attribution. Expected direction:
the `heapless` push in each of the four declaration decoders falls to a call site, one `fixed_list::push` and one
`fixed_list::remove` are added to the runtime's bytes, and the receive path rises by a few instructions per entry
held.

## Order of the changes

1. This ADR.
2. The runtime: `fixed_list.rs` with the two bodies, the `heapless::Vec` impl and its unit tests; `push_keeping`'s
   keep rule out of line; the Miri line of `scripts/gates/forge-rust.sh`.
3. The main session runs the runtime's tests, the Miri step, the Rust golden tests and the conformance lane, and the
   workspace clippy; no golden is regenerated.
4. The consumer measures its image at the landing.

## Acceptance

Accepted on all of these: the runtime's unit tests pass, and pass under Miri; the Rust goldens are unchanged and the
forge conformance lane passes; the workspace clippy passes with `-D warnings`; the consumer measures its image at the
landing.
