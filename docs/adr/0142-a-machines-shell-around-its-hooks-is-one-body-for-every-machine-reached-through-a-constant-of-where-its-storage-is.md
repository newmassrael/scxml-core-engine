# ADR 0142 — A machine's shell around its hooks is one body for every machine, reached through a constant of where its storage is

- Status: Rejected (2026-11-26) by its own measurement (section "Measured"): on the nostd-mcu gate's probe it takes
  44 bytes off each machine and adds 70 to the shared core, 38 and 100 to the plain document's machine and core, and
  104 to 120 bytes to the worst-case stack; for three machines the image is 62 bytes smaller with diagnostics on and
  larger without them. That is not the 300 to 400 bytes "Expected" asked for the unsafe parts it adds, and its
  constants do not build at the workspace's `rust-version` (1.75). Nothing of it landed; the engine keeps ADR 0123's
  shell and ADR 0135's one `Turn` body.
- Would have amended: ADR 0123 decision 3 (what of `EngineHost` stays per machine): `Behaviour::store`, `MainLoop::queue`,
  `MainLoop::core` and `MainLoop::behaviour` are no longer kept, for the reasons below; the rest of its table stands.
  ADR 0070 section 3 (the constructor out of line) keeps its decision; the core's starting values move out of it.
- Related: ADR 0064 and ADR 0068 (the procedures and the main loop once per program), ADR 0071 (a workspace's buffers
  uninitialised until written), ADR 0124 (the data of the event held beside the engine), ADR 0131 (a hook is handed
  the engine's state apart from the policy), ADR 0135 (one `Turn` body), ADR 0141 (one body over a list's raw parts).
- Requested by: a downstream bare-metal consumer, 2026-11-26.

## Context

The consumer's release image holds three generated machines, built with fat LTO. Its explain report of the image
(area transport, field generics) lists the runtime's bodies that are still compiled once per machine:

| body | bytes | bodies | largest |
|---|---|---|---|
| `Engine::run_turn` | 468 | 3 | 156 |
| `Engine::new_in_place` | 353 | 3 | 121 |
| `<EngineHost as Behaviour>::enter_state` | 170 | 3 | 125 |
| `<EngineHost as MainLoop>::queue` | 137 | 2 | 70 |
| `<EngineHost as Behaviour>::execute_transition_content` | 137 | 3 | 80 |
| `<EngineHost as Behaviour>::store` | 114 | 2 | 57 |
| `<EngineHost as Behaviour>::exit_state` | 35 | 2 | 34 |
| `<EngineHost as MainLoop>::behaviour` | 33 | 3 | 11 |

About 1.45 KB. The consumer's transport budget asks for one body per operation for every machine, on the condition
that the instruction count of its receive path does not rise.

## What each body depends on

A body is compiled once per policy `P` because something in it names `P`. There are three kinds of such a thing,
and only the first two are a reason for a body per machine:

- **the layout of `Engine<P>`**: where the queues, the run store, the core and the workspace are, and how long their
  arrays are. Numbers, fixed when the program is compiled.
- **the policy's generated hooks**: its guards, onentry, onexit and transition content, which are the machine's own
  code and stay per machine.
- **nothing**: code that would be the same for every `P`.

| body | what in it is `P`'s | verdict |
|---|---|---|
| `run_turn` | the workspace's frame (its size is the document's); the host of `P`; the `Turn`'s fields: the slices of the workspace (layout), the tables (a constant), the two vtables | the frame and the host stay; the rest is layout and moves |
| `MainLoop::queue` | the offsets of the two queues and their capacities | layout: moves |
| `MainLoop::core` | the offset of the core | layout: moves |
| `Behaviour::store` | the offsets of the run store's three arrays and their lengths | layout: moves, but the procedures reach it through `dyn Behaviour` (see decision 4) |
| `MainLoop::behaviour` | the address of `P`'s `Behaviour` vtable | a coercion: made once per turn |
| `new_in_place` | the policy (moved in); the empty values of `P`'s queue, store, scheduler and done-data types; the core's starting values, whose one number of `P`'s is the default state | the policy and the typed empties stay; the core moves |
| `enter_state`, `execute_transition_content`, `exit_state` | the policy's hook, inlined: the 125, 80 and 34 bytes are the machine's onentry, content and onexit; the 1 and 13 byte bodies of the other machines are their empty hooks | the machine's own code: stays |

ADR 0123 decision 3 kept `store`, `queue` and `core` per machine because a view of them held for a turn would be a
second mutable path to the storage a hook is handed. That stays true and is not what this ADR changes. What ADR 0131
made possible is that the engine is two fields, the policy and the state, and a hook is handed both as two borrows of
one `&mut Engine<P>`: every path into the engine during a turn can be made from one address, so a view can be made
from that address when it is read, and dropped before a hook runs, without a second path that the next reference
would invalidate.

## Decision

### 1. One constant per policy of where its storage is

`Engine<P>::SHAPE` is a `&'static MachineShape`, made when the program is compiled, as a vtable is: the document's
tables, the offset of the core in the engine, and for each queue, the run store and the workspace, where it is and
how long its arrays are. It is read once per turn and never by the procedures.

- Each storage type states its own layout through a trait only the runtime can implement: `EventQueueStorage`,
  `RunStorage` and `Buffers` gain a sealed supertrait (`QueueLayout`, `StoreLayout`, `WorkspaceLayout`, each in a
  `sealed` module of the type's own module, as `SchedulerStorage` already is). The engine turns the numbers into
  references, so a type that could state a wrong layout would be undefined behaviour in safe code; sealing is what
  makes the numbers trusted. The runtime's own types are the only implementors today, and generated code names them.
- `EventQueue`, `RunStore` and `Workspace` are `#[repr(C)]`, their fields in the order the engine reads them, so
  where each array is follows from the lengths: a queue's shape is its offset and its capacity, a store's its offset
  and three lengths, a workspace's four lengths. Each type checks, when its shape is made, that the offsets its lengths
  give are the ones the compiler laid it out at (`field_offset!`, a constant expression over `MaybeUninit` and
  `offset_from`, since `core::mem::offset_of` is younger than the workspace's `rust-version`); a layout that
  disagreed would not compile.
- The engine's own offsets (of the core, the two queues and the store in `Engine<P>`) are read the same way.

On a 32-bit target the constant is 14 words; the `MainLoop` vtable of each machine loses three entries.

### 2. The engine is reached through one address for the length of a turn

- `EngineHost` holds the engine as `NonNull<Engine<P>>`, made from the turn's `&mut Engine<P>`, and makes every
  reference into it from that address: a hook's `&mut Engine<P>` (split, as before, into the policy and the state),
  and the parts.
- `EngineParts` is the engine's core, the events of its two queues and its run store's arrays as addresses, made once
  per turn from the engine's address and `SHAPE` by one body (`EngineParts::new`, `#[inline(never)]`).
- `TurnMachine` is what `Turn` holds: the host as `dyn MainLoop` and as `dyn Behaviour`, and the address of the
  host's parts. Its accessors take `&mut self`: `core()` and `queue()` make a view from the parts, `main_loop()`
  and `behaviour()` make the trait objects. So at most one of them is alive at a time, and a hook, which is reached
  only through a trait object, never runs while a view of the storage it is handed is alive. `Behaviour::store`
  borrows the host, so the procedures' store view and a hook exclude each other the same way.

This is the textbook split of a container into a raw part over addresses and a typed part that owns the borrow
(`RawVec` and `Vec`, `RawTable` and `HashMap`), and the one ADR 0141 made for a fixed list.

Why it is sound under the aliasing rules (Stacked Borrows and Tree Borrows alike): every address is derived from one
raw pointer made from the `&mut Engine<P>` the turn was handed, which stays protected for the call and below which
nothing else is derived; a reference made from it is used, and then not used again once the next one is made. A
reference made from a second, separately derived path would be invalidated by the next one made from the first, and
there is none. The host's own fields (`current`, `lend`) are apart from the engine.

### 3. Each body

- **`run_turn`** is the machine's frame and one call: it reserves the workspace of the document's size
  (`MaybeUninit`, written by nothing), builds the host (whose parts are one call of `EngineParts::new`) and calls
  `Turn::run`, which lends the workspace from the numbers of `SHAPE` (`microstep::lend_workspace`, the same lend
  `Buffers::split` now makes) and runs the turn. The workspace stays in this frame and no other (ADR 0068 section 5,
  ADR 0071).
- **`MainLoop::queue`** and **`MainLoop::core`** are gone: the loop reads both through `TurnMachine`.
- **`MainLoop::behaviour`** is gone: `TurnMachine::new` coerces the host to both trait objects once per turn. This
  is the coercion trait upcasting would make, which is stable from Rust 1.86 and past the workspace's 1.75.
- **`Behaviour::store`** stays a hook, because the procedures are written over `&mut dyn Behaviour`, a public trait
  that the TTCN-3 export's macrostep and the differential tests implement too. Its body for every machine is a call of
  one body, `StoreParts::view` (`#[inline(never)]`), over the parts the host holds.
- **`new_in_place`** writes the core with one body for every machine (`EngineCore::init`, out of line, handed the
  default state). What stays is the policy, moved into the slot, and the empty values of the machine's own storage
  types: each is its type's constant written where it lives (ADR 0070), whose length is the document's.
- **`enter_state`, `execute_transition_content`, `exit_state`** stay as they are. Their bodies are the policy's
  hooks inlined; the wrapper around each is the split of the engine into the policy and the state, which is no code,
  and the read of the event's data, which is `P`'s type. Taking the hook out of line would add a call and leave the
  bytes where they are. A one-byte body is a machine's empty hook; folding those is a linker's identical-code folding,
  not a change of the code.

### 4. The receive path

Per call, each operation the loop and the procedures make on the storage costs fewer instructions than it did: `core`
and `queue` are loads from the turn's machine and the parts where they were an indirect call into a body that
computed the addresses; `behaviour` is a load where it was an indirect call; `store` is a branch into one body that
copies six words where the machine's body computed them. Per turn, two things are added: making the parts (one call,
about fifteen loads, adds and stores) and lending the workspace from its lengths (a fill of the sets' words, which
was there before, and four offsets). A turn that takes one external event makes about a dozen `core` reads, half a
dozen `queue` views and several `behaviour` and `store` calls, so the count is expected to fall. Section
"What would show it wrong" says what the measurement must show.

### 5. Stack

`run_turn`'s frame holds the host's parts (fifteen words on a 32-bit target) beside the workspace, and `Turn::run`'s
frame the turn's machine in place of one trait object. The worst-case stack moves by about the parts' size; the main
session re-pins `STACK_BYTES` with the gate's measurement.

### 6. What checks it

- Miri, in `scripts/gates/forge-rust.sh`'s step with the pinned nightly: the integration test
  `backends/rust/runtime/tests/a_turn_reaches_the_engine_through_one_address.rs` (a machine whose hooks queue on both
  queues and read the configuration between the turn's views, events a host queued before a turn, and a stop outside
  a turn, which reaches the engine through a host made for it alone), and the unit tests whose names end in
  `through_its_parts` (a queue, two queues placed in one value, a run store, a workspace lent from uninitialised
  bytes, and a workspace split). The hook dispatch test of ADR 0131 runs every path of this ADR as well.
- The compile-time checks of each storage type's layout (decision 1).
- `footprint.budget`: `CORE_SYMBOLS` names `engine::EngineParts`; the rest of the new shared code is in paths it
  already names (`helpers::run_store`, `helpers::event_queue`, `helpers::microstep`, `engine::Turn`,
  `engine::EngineCore`). No indirect call is added: the stack analysis follows the same `MainLoop` and `Behaviour`
  hooks from the same `Turn` body.

## Expected

From the consumer's figures, three machines, `.text` by symbol:

| body | before | expected after | per machine |
|---|---|---|---|
| `run_turn` | 468 | about 200 | 156 to about 65 |
| `new_in_place` | 353 | about 250 to 320 | the core's starting values out (10 to 40 each) |
| `MainLoop::queue` | 137 | 0 | gone |
| `Behaviour::store` | 114 | about 8 | a branch |
| `MainLoop::behaviour` | 33 | 0 | gone |
| `enter_state`, `execute_transition_content`, `exit_state` | 342 | 342 | the machine's own code |

About 600 to 700 bytes out of the machines, against once in the program `EngineParts::new`, `StoreParts::view`,
`EngineCore::init` and the lend in `Turn::run` (about 200 bytes), and `.rodata` of 14 words a machine less three
vtable entries (about 44 bytes a machine). Net for three machines: about 300 to 400 bytes smaller; each machine past
the third, about 250 bytes.

## Consequences

- `EventQueueStorage`, `RunStorage` and `Buffers` are sealed. A crate that implemented one for a type of its own
  no longer compiles; none in this repository does, and a generated policy names the runtime's types.
- `EventQueue`, `RunStore` and `Workspace` are `repr(C)`. A queue whose payload is aligned past two bytes may carry
  up to its alignment less two bytes of padding before its payload ring that the compiler could have reordered away.
- No generated code, template, golden or committed tree changes: every name a generated machine spells is unchanged.
- `Buffers::split` writes the sets' words as zeros each time it is called, as the turn's lend does; a procedure that
  takes a set clears it anyway, so no result changes.
- The `MainLoop` trait is the main loop's hooks into the machine's code and typed values, and nothing else.

## Alternatives considered

- **Trait upcasting** in place of `MainLoop::behaviour`. Stable from Rust 1.86; the workspace's floor is 1.75.
  The coercion made once per turn is the same thing without it. When the floor reaches 1.86, `trait MainLoop:
  Behaviour` can replace the second pointer.
- **One `Behaviour` implementor for every machine, dispatching to the machine's hooks through function pointers in the
  constant.** Removes `store` from the machines too, but every hook becomes two indirect calls (the trait object's,
  then the constant's), on the receive path. Refused: it raises the instruction count the condition holds.
- **The procedures handed the store apart from `dyn Behaviour`.** Removes `store` from the machines with no extra
  call, but the store must then be borrowed apart from what a hook is handed, so a hook that asks `In()` would read
  the configuration through a borrow the procedures hand it: a change of `Behaviour`, of every hook of `StatePolicy`
  that reads the configuration, of the templates and of the public microstep API that the TTCN-3 export and the tests
  implement. Refused for about 50 bytes a machine that a branch already takes down to four.
- **Every part's offset in the constant, without `repr(C)`.** 24 words a machine on a 32-bit target where 14 do.
  Refused: the layouts are fixed and checked when compiled at no cost.
- **The engine's starting state as one constant image copied in.** The image is the size of the state, queues
  included, for every machine, in `.rodata`. Refused.
- **Views held for the turn.** Undefined behaviour across a hook (ADR 0123 decision 3). Refused again.

## What would show it wrong

- The consumer's receive path executing more instructions than at the parent. Then the per-turn making of the parts
  costs more than the calls it saves on that path, and the parts are made where the engine is built instead, once,
  which is sound only for an engine that does not move after it is built; that would need its own decision.
- `MACHINE_BYTES` not falling on the nostd-mcu gate's probe, or `.rodata` growing by more than the constant.
- Miri reporting a retag of a tag that no longer exists on any path of the integration test.

## Measured

On the nostd-mcu gate's build (`backends/rust/probes/nostd-footprint`), the branch rebased on main 1c8b51842e3,
against the pins of `footprint.budget` at that commit:

| figure | main | this decision | change |
|---|---|---|---|
| one more machine, diagnostics on | 1696 | 1652 | -44 |
| one more machine, diagnostics off | 1688 | 1652 | -36 |
| shared core, diagnostics on | 8758 | 8828 | +70 |
| shared core, diagnostics off | 8654 | 8740 | +86 |
| worst-case stack | 1104 | 1208 | +104 |
| one more machine, plain documents | 908 | 946 | +38 |
| shared core, plain documents | 2664 | 2764 | +100 |
| worst-case stack, plain documents | 544 | 664 | +120 |

The per-turn making of the parts (`EngineParts::new`, `StoreParts::view`) and `EngineCore::init` cost about as
much in the shared core as the per-machine bodies they remove, and the parts held in the turn's frame raise the
stack. The second condition of "What would show it wrong" (`MACHINE_BYTES` not falling as expected) holds.

Two defects were found and not repaired, because the decision is rejected: `field_offset!` in an associated
constant borrows a `MaybeUninit` of a type with interior mutability, which Rust 1.75 refuses in a constant
(E0658, issue 80384), so the `rust-version` gate fails; and the Miri step the branch added stopped on an unused
`unsafe` block in a test before it ran. A later attempt at one body per operation starts from the measured split
above, not from the estimate in "Expected".
