# ADR 0070 — An empty event queue is a constant of its storage type

- Status: Accepted (2026-10-30), on the owner's instruction to optimise the footprint structurally
  until no optimisation point is left.
- Amends: ADR 0056's constant construction (`Engine::new_const`, `ConstStatePolicy`) and ADR 0068
  (which left `ConstStatePolicy` with one item).
- Evidence: `backends/rust/probes/nostd-footprint`.

## Context

After ADR 0068, `Engine::new` is the largest piece of code each machine still compiles for itself
that is not the document's own: 240 bytes on the probe, with a 512-byte stack frame. The
disassembly shows why. The constructor took its empty queues from `P::EventQueue::default()`, a
function evaluated at run time. That function built each queue's slot array on the
constructor's own stack, one slot per loop iteration, and then copied it into the engine with
`memcpy`. Wherever the constructor is inlined, as into a consumer's task setup, that frame comes
with it.

`Engine::new_const` took the same queues from a constant, `ConstStatePolicy::EMPTY_EVENT_QUEUE`,
which each const-buildable policy declared. After ADR 0068 that constant was the trait's only
item.

## Decision

### 1. The storage type declares its empty value

`EventQueueStorage` gains `const EMPTY: Self`, and `EventQueue` gives `Self::new()`, a constant
expression on both profiles. Both constructors write that constant into the engine's fields. An
empty slot is zero bytes, so the compiler clears the queues where they live. Every starting
value of an engine is now a constant except the policy, so the two constructors differ only in
how they place the fields.

### 2. `ConstStatePolicy` is removed

With the queues' constant on the storage type, the trait had nothing left to declare. It is
removed, along with the `impl` the Rust template wrote for a const-buildable machine.
`Engine::new_const` has no bound beyond `StatePolicy`, and the caller builds the policy in a
constant expression; a generated policy that can be built that way still offers `new_const`.

### 3. `Engine::new_in_place` is out of line

Once the constructor builds nothing on its stack, it is small, but inlined it was copied into
every place a program builds the machine: on the probe, whose first machine is built at four
entry points, the image grew by 120 bytes. Kept out of line, a machine's constructor is one copy
whatever the number of call sites, and it is what `Engine::new` calls.

## Measured (2026-10-30)

| | one machine | two machines |
|---|---|---|
| diagnostics on | 194,215 -> 194,117 | 195,905 -> 195,831 |
| diagnostics off | 194,009 -> 193,913 | 195,641 -> 195,573 |
| plain documents | 191,070 -> 190,966 | 192,526 -> 192,462 |
| plain, constructor inlined (decision 3 not taken) | 191,190 | 192,578 |

The constructor is 124 to 144 bytes, with no temporary on the stack (it was 240 bytes and a
512-byte frame). The core and the worst-case stack are unchanged.

## Consequences

- A host that names `ConstStatePolicy` no longer compiles. Nothing generated after this ADR
  names it, and the constructors' signatures are otherwise the same: `new_in_place` and `new`
  take the policy, and `new_const` takes it in a constant expression.
- Generated Rust trees for const-buildable machines lose the `impl ConstStatePolicy` block, so a
  consumer regenerates with this pin's generator.
- A queue storage type outside this crate, if one were written, must now declare its empty
  value as a constant.
