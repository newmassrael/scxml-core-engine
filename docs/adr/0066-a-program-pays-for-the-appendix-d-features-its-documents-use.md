# ADR 0066 — A program pays for the Appendix D features its documents use

- Status: Accepted (2026-10-30), on the owner's instruction after the ADR 0064 acceptance
  measurement: "구조로 더 줄이기" — repair it structurally, as ADR 0064 decision 6 requires.
- Amends: ADR 0064 (decisions 2 and 6).
- Evidence:
  - the watching-zenoh T2G measurement of 6f72d5975e;
  - `backends/rust/probes/nostd-footprint`, built at `opt-level = "s"`.

## Context

ADR 0064 compiles Appendix D once per program, over a document's tables. Its acceptance is the
consumer's image, and that image did not get smaller. On the watching-zenoh kit (two machines,
`opt-level = "s"`, only the SCE pin changed), `.text` grew by 2,444 bytes:

| | before (49e40c9a72) | after (6f72d5975e) |
|---|---|---|
| runtime code with no machine type | 376 | 6,502 |
| connect machine (`PeerConnectionPolicy`) | 3,586 | 1,286 |
| session machine (`SessionFsmUnicastPolicy`) | 3,946 | 1,724 |

Each machine's own share fell by about 2.3 KB. The shared core costs 6.1 KB, so two machines do
not pay for it. The probe, rebuilt at the consumer's `opt-level`, shows the same shape: two
machines are 1,894 bytes over 49e40c9a72 there.

Neither of the consumer's documents has a `<history>` or a `<parallel>` (`peer_connection.scxml`:
10 states, 17 transitions; `session_fsm_unicast.scxml`: 12 states, 30 transitions). A large part
of the core is code those documents cannot reach:

- Effective targets dereference a `<history>` to what it recorded, or to its default.
- The entry walk, for a `<history>`, enters the recorded states or the default targets, and owes
  the default's content to the parent.
- exitStates records every history of every state it exits.
- The entry walk defaults the regions of a `<parallel>` that nothing entered.
- removeConflictingTransitions exists because a `<parallel>` lets several transitions be enabled at
  once. With no `<parallel>`, a configuration has one atomic state and a selection at most one
  transition.
- isInFinalState of a `<parallel>` asks every region in turn.

When Appendix D was generated per machine, the compiler removed these paths from a machine that
could not take them: `NoHistory` is uninhabited, so every history branch fell away. Compiled once
for every program, the core holds them all, whether or not any document uses them. ADR 0064 named
this cost in its Consequences ("the cost of the history branches is paid once per program instead
of being removed per machine") and did not weigh it. The consumer's image is where it showed.

## Decision

### 1. A feature's procedures are reached through the tables of a document that uses it

The parts of Appendix D that only a `<history>` reaches are one set of procedures, and the parts
that only a `<parallel>` reaches are another:

- **`HistoryProcedures`**:
  - dereference a `<history>` target for the effective targets;
  - dereference it for the entry walk, recording owed default content;
  - read what a `<history>` recorded, from a state on;
  - record the histories of the states a microstep exits.
- **`ParallelProcedures`**:
  - default the untaken regions of a `<parallel>` the entry walk enters;
  - remove conflicting transitions from an enabled set;
  - decide isInFinalState for a `<parallel>`.

The runtime defines one constant of each, `HISTORY` and `PARALLEL`, holding the procedures as
function pointers. Their fields are private, so these are the only values the types have.
`DocumentTables` gains:

- `history: Option<&'static HistoryProcedures>`
- `parallel: Option<&'static ParallelProcedures>`

The generator writes `Some(&sce_rust_runtime::HISTORY)` only for a document that has a
`<history>`, and `Some(&sce_rust_runtime::PARALLEL)` only for one that has a `<parallel>`. The
core calls a feature's procedures through these fields, and nowhere else.

They are constants rather than statics because a document's tables are a constant, and a constant
may refer to a static only from Rust 1.83 on, past the runtime's `rust-version` (1.75). A reference
to a constant in a constant is promoted to one constant allocation.

So a feature's code is referenced only from the tables of documents that use it. In a program none
of whose documents does, no table takes the reference, nothing is emitted for the constant, and the
linker drops every function only it reaches.

Without a `<parallel>`, a configuration has one atomic state, so selection enables at most one
transition. The core then skips removeConflictingTransitions instead of calling it on that one
transition. This is the reachability by which `NoHistory` used to remove those
paths, moved from the type system to the link. The core is still one copy, and there is still no
build mode (ADR 0064 decision 6, ADR 0053): what decides is the documents the program holds, not a
flag.

### 2. A table that needs a feature carries it

`DocumentTables::check` refuses:

- rows that name a `<history>` while `history` is `None`;
- a `<parallel>` state while `parallel` is `None`.

A hand-written table states the feature it uses, as a generated one does.

### 3. The call through a feature is a call through a pointer, and the stack instrument follows it

The core reaches a feature through a function pointer read from the tables. The stack tool
resolves such a call the way it resolves a trait object's (`--dyn`). The private fields of the
procedure sets make `HISTORY` and `PARALLEL` their only values, so a call reaches procedures of
those two and nothing else. Of them, a function's call reaches the ones its own source calls
lead to. Resolving every pointer call to every procedure would be sound too, but it charges
paths that cannot run and makes cycles that do not exist: a `Behaviour` hook asks
isInFinalState, whose procedure would then seem to reach a procedure that reaches the hook
again.

### 4. The instrument weighs a program without the features

The footprint probe links its two machines, one with `<parallel>` and `<history>` and one with
neither. It gains a configuration that links two machines with neither feature, the consumer's
shape. The gate holds the shared core's size in that configuration as well, so a change that pulls
feature code back into every program is a red.

## Consequences

- A program of documents without `<history>` or `<parallel>` no longer carries their code. A
  program with one document that uses them carries that feature once, as before.
- The core's feature steps become calls through a pointer, a few cycles each, and only in documents
  that use the feature.
- The differential test (`tests/microstep_algorithms.rs`) builds its tables with the procedures its
  documents need, so the core is held to the reference recursion with the procedures in place.
- This is not the whole repair. The per-machine shells (the queues, the event binding, the typed
  entry points) are the other part of the consumer's figure, and making the engine's body speak
  event numbers is an ADR of its own.

## Measured (2026-10-30)

`thumbv7em-none-eabihf`, `opt-level = "s"`, rustc 1.99.1, the probe's figures
(`backends/rust/probes/nostd-footprint/footprint.budget`):

| | before | after |
|---|---|---|
| shared core, two machines without either feature | 6,048 B | 4,528 B, no feature procedure linked |
| shared core, a machine with `<parallel>` and `<history>` | 6,048 B | 6,678 B |
| worst-case stack, with the features / without | 1,368 B / — | 1,392 B / 1,152 B |

A program without the features no longer carries 1,520 bytes of them. A program with them pays
630 bytes more, because a procedure reached through a pointer is a function of its own and is
not inlined into its caller. That is the cost of one copy serving every program.

It is not enough for the consumer. The same two featureless documents at 49e40c9a72 measure
188,064 and 190,137 bytes. After this change they measure 190,566 and 192,184, still 2,047
bytes more at two machines. Each machine's own engine (`Engine<P>` and its hooks: queues,
event binding, typed entry points) is 1.0–1.5 KB, where its copy of the algorithm used to be
1.8–3.8 KB, and the 4.5 KB core is paid once on top. Removing the per-machine engine is the
next decision (Consequences, last item).

The stack instrument resolves a call through a feature's pointer the way it resolves a trait
object's (decision 3). Each core function's call reaches the hooks and procedures that its own
source calls lead to, since an inlining moves code only along those calls. A core function
whose indirect call no rule states fails the run, instead of passing as a host callback
(`--dyn-complete`).

## Order of the changes

1. The runtime: the two procedure sets, the `DocumentTables` fields and the check, with the
   differential test and the hand-written tables carrying them.
2. The generator writes the fields; the committed trees are regenerated.
3. The probe's configuration of two machines with neither feature, its figures and the stack tool's
   mapping in the gate.
4. Measure at `opt-level = "s"`, and send the commit to the consumer when two machines come out
   below 49e40c9a72.
