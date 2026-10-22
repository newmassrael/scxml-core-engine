# ADR 0056 — The Rust microstep's buffers are the document's size, and the runtime names none

- Status: Accepted (2026-10-22) and implemented, under the owner's standing rule: the most long-term correct
  way, cost ignored (items 1 and 2 of the owner's answer to the review of the stack change, 2026-10-22; item 2's
  workspace is decided below)
- Date: 2026-10-22
- Scope: `backends/rust/runtime` (`helpers/hierarchy.rs`, `helpers/microstep.rs`, `policy.rs`, `engine.rs`,
  `helpers/configuration.rs`, `saved_state.rs`, `lib.rs`); `sce-build/src/microstep_bounds.rs` and the Rust half of
  `generator.rs`; `tools/codegen/templates/rust/{state_machine,utility_methods,entry_exit_actions}.rs.jinja2`; the
  committed Rust trees; the MCU probes under `backends/rust/probes` and `scripts/gates/nostd-mcu.sh`
- Related: ADR 0053 (the runtime's two profiles and their aliases), ADR 0050 (a build's bounds are read from the
  document), the Rust stack change of 2026-10-22 (`Engine::new_in_place`, the enabled set sized to 16, the in-place
  conflict filter), `backends/rust/probes/nostd-footprint/footprint.budget`

## Context

A bare-metal machine runs Appendix D's microstep on its host's main stack, and the stack of the deepest call chain
(`step` down to an entered state's `<onentry>`) is the number a downstream client measured against a 13 KB budget.
The first measurement (2026-10-22, a consumer's Cortex-M7 client) found three `Engine<P>` frames of 520-550 bytes
each, and they were copies of one thing: a transition buffer sized for 32 enabled transitions, which the runtime
chose once for every document. The first change sized it to the bound the configuration already imposed, 16, and
brought the chain from 1932 to 668 bytes on the probe machine.

That change left two things wrong, each named in the review that followed it:

1. **16 is not the document's number.** It was the runtime's existing cap on a configuration
   (`MAX_HIERARCHY_DEPTH`), itself a global guess. A configuration is a root-to-leaf path with every region of each
   `<parallel>` on it, so it is not bounded by depth: a `<parallel>` of twenty regions holds twenty-one states in a
   tree two levels deep, and panicked at a push past 16 under `no_std`. And for the common machine, three flat
   states, 16 is sixteen times what one active state needs.
2. **The runtime held the constant.** A capacity is a fact about a document, which the generator reads, and the
   runtime was guessing it for every document.

## Decision

### 1. The bounds are read from the document's state tree and written on the policy

`sce_build::microstep_bounds::MicrostepBounds::of` reads the tree from each state's parent and gives four numbers,
each an upper bound that follows from the tree:

| Number | What it is | Rule |
|---|---|---|
| `MAX_CONFIGURATION` | the most states one configuration holds (§scxml-3.4) | `1 + max(children)` for a compound state, `1 + sum(children)` for a `<parallel>`, `1` for an atomic one; the largest over the `<scxml>` element's children |
| `MAX_ENABLED` | the most transitions one selection enables (§scxml-D-selectTransitions) | the same walk counting atomic states only: a selection takes at most one from each |
| `MAX_DEPTH` | the level of the deepest state or `<history>`, the `<scxml>` element's child being 1 | a walk up the hierarchy reaches the `<scxml>` element in that many steps; a `<history>` is a level below its parent |
| history defaults | how many `<history>` default transitions a microstep owes | one for each parent that has a `<history>` |

The exit set, the entry set, a list of effective targets and a history's record are sets of states that are, or are
about to be, in one configuration, so they take the configuration's bound.

The generated policy states them as associated consts (`MAX_CONFIGURATION`, `MAX_ENABLED`, `MAX_DEPTH`) and spells
the buffers as associated types (`Configuration`, `EnabledSet`, `HistoryDefaults`), each a
`helpers::hierarchy::BoundedVec<T, N>` of its number: a `Vec` under `std`, where the capacity is not read, and a
`heapless::Vec` of exactly that capacity without it. The associated types are what stable Rust offers for a
capacity that depends on the implementor: a const generic argument cannot be computed from a generic parameter.

### 2. The runtime names no capacity

`StateChain`, `new_chain`, `push_chain`, `state_chain_from_slice`, `MAX_HIERARCHY_DEPTH`, `SceTransitionBuf` and
`MAX_ENABLED_TRANSITIONS` are gone. `Bounded<T>` (a slice that starts empty, takes a push, loses items and is
consumed into them) is the one trait the procedures of `helpers::microstep` are written against; `Document` carries
the three buffer types and `MAX_DEPTH`, and the walks up the hierarchy are held to the document's own depth.
`EntrySet` is generic over its two buffers, not over the document, so a document that borrows the engine does not
keep the borrow in the set it returns.

### 3. A bound is checked where it is used, and shown to be one

A `no_std` push past a capacity panics with the generator's name on it. A `std` build grows its buffers and would
never say the bound was wrong, so the engine asserts, in a debug build, on every selection and every microstep,
that what it holds is within `MAX_ENABLED` and `MAX_CONFIGURATION`: the conformance corpus is then the evidence that
the bound is an upper bound and not an estimate. `microstep_bounds` is held to a brute-force listing of every legal
configuration of the document, for flat, nested, parallel, wide-parallel and history trees, and the probe machines'
numbers are asserted exactly (`rust_template_sizes_the_microstep_to_the_documents_state_tree`, the `nostd-queue-size`
probe), since a number that is merely small enough would let the generator drift back toward a constant unseen.

### 4. The engine does not own a workspace

The review asked whether the buffers should be one workspace the engine owns, so that stack use does not depend on
the document. They are not, and the measurement is why. With the buffers sized to the document, the frames of the
external-event path from `step` to the entry of a state, on the parallel probe machine (nine states, four of them
atomic, four levels), are 396 bytes, from 1932 before the work of 2026-10-22 and 668 after its first part; a flat
machine's are smaller. A workspace would move the live buffers of that chain, a few hundred bytes, into the engine
permanently: a saving in peak stack bought with the same amount of static RAM for the whole run. It also needs the
engine and the buffers borrowed apart through a policy hook that borrows the whole engine, which stable Rust gives
only with `unsafe` or a runtime checkout. That is a loss in one resource for a gain in another, at a price in
`unsafe`, for a document-independence the sizing already gives. It is not built; a consumer whose measurement shows
the stack, not the RAM, to be the limit, with the frames of this decision as the baseline, reopens it.

## Consequences

- **A document's number is its own.** The probe machine of two parallel regions and a nested `<parallel>` holds nine
  states, four enabled and four levels, and a flat machine one of each; a wide `<parallel>` no longer overflows a
  buffer chosen for depth.
- **The measured frames** (`thumbv7em-none-eabihf`, `opt-level = "s"`, static prologue sizes from the disassembly, the
  parallel probe machine), before the work of 2026-10-22 and now: `select_transitions` 1184 -> 232 bytes,
  `take_event` 536 -> 88, `microstep` 544 -> 160, `enter_states` 224 -> 40, and the main loop's 584 -> 64, with
  `complete_macrostep` (the internal-event path) 128 in a frame of its own.
- **`.text`** per machine, diagnostics on and off: 5236 / 5184 before the stack change, 5334 / 5288 after it, 5340 /
  5288 after this one, inside the band of the budget (it is not re-pinned).
- **API.** Under `std` nothing changes for a consumer that did not name a removed type: `get_active_states` returns
  `Vec<State>` as before. Under `no_std` it returns the document's `Configuration`; `Engine::enter_at` and
  `StatePolicy::set_active_states` take a slice; a hand-written policy states the six items above. A consumer's
  hand-written `StatePolicy` is the one thing that fails to compile, once, with the missing items named.
- **The frames are held, not only measured.** `scripts/gates/nostd-mcu.sh` now computes the worst-case stack from the
  footprint probe's disassembly (`tools/mcu_stack/worst_case_stack.py`: each function's frame from its prologue,
  and the largest sum along the calls of the object) and holds it to `STACK_BYTES` in `footprint.budget`, in both
  directions, in both configurations. It is static and says so: a host callback is taken to need a stated number of
  bytes, the formatting machinery a stated number, a recursion ends only at a bound the budget states (the document's
  depth), and an indirect call or a cycle the budget does not account for fails the run. The probe's profile
  (`opt-level = "z"`, fat LTO) inlines the policy's entry actions into the entry procedure, so its 1776 bytes include
  an 872-byte frame that is the generated policy's, not the runtime's; a mutation that widens the enabled set from
  4 to 40 slots moves it to 2928 and trips the band. GNU's and LLVM's disassemblers give the same figure.
- **A document that is not legal** (a machine whose `initial` names two top-level states, which §scxml-3.11 forbids
  and the semantic check refuses) can hold more than its tree bounds; the check is the guard, and a bare-metal push
  past a bound names the generator.
- **C11 keeps its own sizing** (the document's atomic-state count, bitmaps for the configuration) and is not touched.
