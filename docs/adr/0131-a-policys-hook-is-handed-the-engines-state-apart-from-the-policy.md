# ADR 0131 — A policy's hook is handed the engine's state apart from the policy

- Status: Accepted (2026-11-25). A soundness defect a downstream consumer found by running a generated Rust machine
  under Miri, confirmed in the runtime. The structure is the owner's decision of that day: the textbook split borrow by
  decomposition of the struct, no `unsafe`, no workaround.
- Date: 2026-11-25
- Scope: `backends/rust/runtime` (`engine.rs`, `policy.rs`, `helpers/invoke_processing.rs`, `lib.rs`),
  `tools/codegen/templates/rust`, every hand-written `StatePolicy`, `scripts/gates/forge-rust.sh`.
- Related: ADR 0064 and ADR 0068 (Appendix D and the main loop once per program, over `EngineHost`), ADR 0123 (what
  stays per machine and why, decision 3), ADR 0124 (the data of the event is held beside the engine, never in what a
  hook is handed), ADR 0045 (a host-run invocation names the session that starts it).

## Context

`Engine<P>` held its policy as one field among the queues, the run store, the scheduler and the host's registrations.
Every hook of the policy that runs executable content takes the policy as `&mut self` and the engine as
`engine: &mut Engine<Self>`, and the engine dispatched each through one helper:

```rust
fn with_policy<R>(&mut self, f: impl FnOnce(&mut P, &mut Self) -> R) -> R {
    let policy_ptr: *mut P = &mut self.policy as *mut P;
    unsafe { f(&mut *policy_ptr, self) }
}
```

For the length of every hook a `&mut P` into `self.policy` and a `&mut Self` whose memory contains that same policy
were live together. That is undefined behaviour: two live unique references to overlapping memory. Miri reports it on
the first entry of every machine (`trying to retag ... for Unique permission ... tag does not exist`, path
`Engine::initialize` -> `enter_state` -> `with_policy`); `cargo test` passes it, because the optimiser has so far
not used the licence the reference rules give it.

The helper's safety argument rested on two premises, and both were false:

1. That the borrows are sound because the fields the policy's code writes and the fields it reaches through the engine
   do not overlap. The aliasing rules are about references, not about which bytes a function happens to touch: a
   `&mut Engine<P>` asserts unique access to all of the engine, the policy included, from the moment it is used, and
   doing so invalidates the `&mut P` derived from it.
2. That the engine exposes no route back to the policy. `Engine::policy(&self) -> &P` is public, so a hook could read
   the policy through the engine it was handed while it held the policy mutably. The engine's own code did the same
   (below).

## Decision

### D1. The engine is its policy and its state

```rust
pub struct Engine<P: StatePolicy> {
    policy: P,
    state: EngineState<P>,
}
```

`EngineState<P>`, exported beside `Engine`, holds every field the engine held but the policy, with the same `cfg`s and
documentation. Both constructors build the two in place from one list of starting values, which the compiler checks
against both structs, as it checked the one before.

### D2. A hook takes the state, and the dispatch is safe

Every hook of `StatePolicy` that took `engine: &mut Engine<Self>` takes `engine: &mut EngineState<Self>`. The dispatch
is one route, as before:

```rust
fn with_policy<R>(&mut self, f: impl FnOnce(&mut P, &mut EngineState<P>) -> R) -> R {
    f(&mut self.policy, &mut self.state)
}
```

Two borrows of two fields, which the compiler checks. The `unsafe` block and its safety comment are gone; the engine's
only `unsafe` blocks are its constructors', which write the fields into an uninitialised slot.

### D3. A method lives where what it reads lives

A method that reads only the state is `EngineState`'s, written once: the queues and `raise`, the external raises,
the scheduler and its passthroughs, the clock, the configuration and history queries, the diagnostics, the host's
registrations, the host-run invocations' completions and cancellation, the inbox drain. A method that needs the policy
is `Engine`'s: the constructors, `initialize` and `enter_at`, `step`, `tick`, `process_event` and its spellings, the
turn and the main event loop that dispatch hooks, `policy()`, the inbox handle (which names the session), the
adoption of a tree's inboxes and invokers (which hands them to the children the policy holds), the routing of a
host-run invocation's ending to a child, and `time_until_next_scheduled_ms` (which asks the policy for its children's
deadlines). Nothing is written twice.

`Engine<P>` implements `Deref<Target = EngineState<P>>` and `DerefMut`. The relation is the one of `Vec<T>` to `[T]`:
the engine owns its state and adds the policy, and what the state does is done on the engine that owns it. A host that
holds an engine calls `engine.raise_external(..)`, `engine.set_clock(..)` or `engine.get_active_states()` as it did,
and none of those calls reaches the policy, so holding `&mut Engine<P>` grants no second path to it.

### D4. A path a hook reaches that needed the policy hands it over

The split makes a hook-reachable method that reads the policy a compile error, since a hook holds only the state.
Each such path was itself the defect of D2 in a second place, and each is fixed by handing the policy over beside the
state, as `Engine::send_to_target` already did:

- `perform_host_invoke`, which the generated invoke site calls from `execute_pending_invokes`, read the policy's
  `own_session_id()` through the engine (in `start_host_invoke`) to name the session that starts the invocation
  (ADR 0045). It is now the associated function `Engine::perform_host_invoke(policy, engine, request)`, and the start
  takes the session's id from its caller; a restore's re-armed starts are begun by the engine, which holds the policy.
- `Engine::send_to_target(policy, engine, ..)` took the policy apart already, but its `engine` was the whole engine,
  which contains that policy: the same two live references. Its `engine` is now the state.

## Consequences

- Every hand-written `StatePolicy` fails to compile once, at each hook it implements that takes the engine: the
  parameter's type is `&mut EngineState<Self>`. The body is unchanged, since every method a hook may call is the
  state's. `docs/SCE_CODEC_CONSUMER_MIGRATION.md` says what to write.
- Generated Rust is regenerated, and nothing a host calls changes but `perform_host_invoke`, which only a generated
  invoke site calls and which is now `Engine::perform_host_invoke(self, engine, request)` there.
- Through `Deref`, a method of `EngineState` is found on an `Engine` after the methods of `Engine` and of the traits
  implemented for `Engine`. A generated extension trait for `Engine` (`<Machine>Inject`, whose methods are
  `raise_<event>`) whose method has the name of a method of `EngineState` is the one a call on an `Engine` reaches; an
  event whose name spells one (an event called `external` gives `raise_external`) is a call the host spells through
  `EngineState` (`EngineState::raise_external(&mut engine, ..)`). An inherent method of `Engine` is found first, as it
  always was.
- A path such as `Engine::raise(&mut engine, ..)` names a method that is now `EngineState`'s; it is spelled
  `EngineState::raise(&mut engine, ..)`, which the deref coercion of the argument accepts.
- The footprint is a layout change and no new code: the engine is the same fields, two of them now grouped, and every
  method body is the one it was. The grouping can add padding between the policy and the state that the compiler could
  have reordered away before.
- Miri runs a hand-written machine whose hooks write the policy, raise through the state and read the configuration
  in one call (`backends/rust/runtime/tests/a_hook_borrows_the_policy_and_the_engine_state_apart.rs`), in the
  `forge-rust` gate beside the queue runtime's Miri steps and with the same pinned nightly, so a return of the aliasing
  is refused by the gate and not only by a consumer.

## Measured

2026-11-25, `thumbv7em-none-eabihf`, `opt-level = "s"`, rustc 1.100.0, the nostd-mcu gate's
figures (`backends/rust/probes/nostd-footprint/footprint.budget`): the split changes no figure.

| | one more machine | shared core | worst-case stack |
|---|---|---|---|
| diagnostics on | 1,462 -> 1,462 B | 8,504 -> 8,504 B | 984 -> 984 B |
| diagnostics off | 1,454 -> 1,454 B | 8,396 -> 8,396 B | 984 -> 984 B |
| two machines without `<history>` or `<parallel>` | 932 -> 932 B | 2,906 -> 2,906 B | 560 -> 560 B |

Under Miri (the pinned nightly of `scripts/lib/queue_sanitizer.sh`),
`a_hook_borrows_the_policy_and_the_engine_state_apart` (4 tests) and `hello_world` (18 tests) run
with no undefined behaviour reported; before the split, the path `Engine::initialize` ->
`enter_state` -> `with_policy` was reported by a consumer's Miri run as a retag of a tag that no
longer exists.
