# ADR 0064 — Appendix D runs once per program over a document's tables, and a machine brings only its tables and its behaviour

- Status: Accepted (2026-10-29), on the owner's instruction, under the owner's standing rule: the most
  long-term correct structure, cost ignored. Implemented in the order of the last section.
- Date: 2026-10-29
- Scope: `backends/rust/runtime` (`helpers/microstep.rs`, `helpers/hierarchy.rs`, `helpers/configuration.rs`,
  `policy.rs`, `engine.rs`, `saved_state.rs`); `sce-build` (a new module beside `microstep_bounds.rs`, and the
  Rust half of `generator.rs`); `tools/codegen/templates/rust/*.rs.jinja2`; the committed Rust trees; the probes
  under `backends/rust/probes` and `scripts/gates/nostd-mcu.sh`; `ARCHITECTURE.md` ("Zero Duplication
  Architecture", "Rust AOT Backend"). The same direction for the C11 and C++ backends is stated in decision 8 and
  left to their own changes.
- Related: ADR 0003 (one portable document, compiled per target), ADR 0050 (a build's bounds are read from the
  document), ADR 0053 (the Rust runtime's features only add, and a bare-metal crate asks for none), ADR 0056 (the
  microstep's buffers are the document's size), `backends/rust/probes/nostd-footprint/footprint.budget`
- Requested by: the watching-zenoh parity session (Zenoh/pico preset benchmark), 2026-10-29

## Context

### What was measured

The watching-zenoh session measured a Zephyr image on the T2G kit (ARM thumb, `opt-level = "s"`) that holds two
SCE machines, a session machine and a connect machine. The runtime's structural procedures are linked once for
each machine. The linked symbol sizes it reported, generic arguments removed, both copies together:

| Symbol | Both copies |
|---|---|
| `walk_entry` | 1,502 B |
| `Engine::select_transitions` | 1,448 B |
| `Engine::complete_macrostep` | 828 B |
| `Engine::run_main_event_loop` | 606 B |
| `Engine::microstep` | 600 B |
| `effective_target_states_on`, `compute_exit_set`, `add_transition_to_entry_set`, `enter_entry_set`, `transition_domain`, `entry_set_of_enabled`, `sort_in_entry_order`, `sort_in_exit_order` | about 1,900 B |

The second copy is about 3.5 KB. The image's `.text` is 147,772 B against 145,364 B for single-thread zenoh-pico,
2,408 B over, so the second copy is more than the whole remaining gap. Its acceptance condition: the kit lane's
turnaround (0.19 ms, graded against pico) does not regress, and `.text` falls.

### Why every machine pays for the algorithm again

Every procedure of Appendix D in this runtime is generic over the machine: `helpers::microstep` over
`D: Document` and `R: Run`, and the engine's loop over `P: StatePolicy`. Rust compiles a generic function once for
each set of type arguments, so an image with `n` machines holds `n` copies of the algorithm. The footprint budget
already counts this as per-machine cost ("the runtime's generic code monomorphised for it"), and its own history
shows the cause is the shape, not where the code sits: when the microstep moved from the generated policy into the
runtime (2026-09-24), the bytes moved with it and stayed per machine.

The other backends have the same shape. The C++ `MicrostepAlgorithms` is a template over its `Host`, instantiated
once for each AOT policy. The C11 backend emits the microstep into every machine's source from
`tools/codegen/templates/c/microstep.jinja2`.

### Why the requested `&dyn` view alone does not produce one copy

The request was a build mode in which the procedures read the document through a `&dyn` view or a table of
function pointers. A trait object removes the generic parameter for the methods, but not for the types the trait
names. `Document` carries `State`, `History`, `Configuration`, `EnabledSet`, `HistoryDefaults` and `Walk`. Every
one of them is a different type for each machine: the first two are the machine's own enums, and the other four are
bounded buffers whose capacities are the document's (ADR 0056). `dyn Document<State = S, History = H, ...>` is a
different type for each machine, so a function that takes it is still compiled once per machine. To get one copy,
a procedure must not name any type that belongs to a machine. That means the states and histories must reach it
as numbers, and the buffers must reach it as slices.

### Why it is not a build mode

- ADR 0053 decided that the runtime's features only add and that a bare-metal crate asks for none. A feature that
  replaces how the algorithm is dispatched does not add, and an MCU-only feature is the bare-metal crate asking for
  one.
- Two dispatch modes are two copies of the parameterisation of Appendix D, each needing the conformance corpus,
  the differential test and the stack gate, against the Zero Duplication rule.
- The reason the generic form exists is speed: the compiler can inline the machine's code into the algorithm. That
  is a measurable claim. The acceptance condition below measures it, and if the one structure loses, the fix belongs
  in that structure.

## Decision

### 1. A runtime function is generic over a machine only where it touches a value of the machine's type

The algorithm reads a document's structure and calls the document's behaviour. Neither needs the machine's types:
the structure can be written as numbers, and the behaviour can be reached through an object-safe trait. A
function stays generic over `P` only where it reads or writes a value whose type is the machine's own: a typed
payload, a typed event in a queue, or a typed state handed to or from the host. Every such function is one of two
kinds: a conversion between the machine's types and the numbers, or a call into the generated code. Both are small.

This is the classic split between a table-driven automaton and a directly coded one, as compiler textbooks
describe it for scanners and parsers: per-language tables read by one driver, against code generated for each
language. A table-driven program is smaller, and a directly coded one leaves more room for inlining. SCE's
document is the language and its machines are the automata. The uSCXML project's transformation of SCXML into
ANSI C uses the table-driven form for Appendix D: the document becomes tables, and one step function runs them.

### 2. The document is compiled into tables, and its states are numbers in document order

`sce-build` gains one function from the model to a document's tables, beside `MicrostepBounds::of` and in the
same style. Its output is the single source every template emits from:

| Table | One row per | Columns |
|---|---|---|
| states | state, in document order | parent, kind (atomic, compound, parallel, final), first child and child count, initial transition |
| histories | `<history>` | parent, shallow or deep, default transition, where its record is kept |
| transitions | transition, initial transition and history default transition | source, internal or external, first target and target count, whether it has content |
| targets | target, as written | a state or a history |
| bounds | document | `MAX_CONFIGURATION`, `MAX_ENABLED`, `MAX_DEPTH`, history defaults, walk capacity (ADR 0056) |

- **A state is a `StateId(u16)`, and its number is its position in document order.** `document_order` becomes the
  identity, entry order is ascending order, and exit order is descending order. A `<history>` is a `HistoryId(u16)`
  and a transition a `TransitionId(u16)`. The generator refuses a document with more than 65,535 of any of them,
  and names the bound in the refusal.
- **An initial transition and a history's default transition are rows of the transitions table.** Appendix D
  treats them as transitions. One row type then serves all three, and the entry procedures read one kind of target
  list.
- **The tables are one `static` for each machine** (`&'static DocumentTables`). They contain no pointers into
  generated code, so the C11 and C++ backends can emit the same rows.
- **The machine's enums stay, as its public names.** The generated `State` is `#[repr(u16)]`, and each variant's
  discriminant is its state's number, so `state as u16` costs nothing. The generated `STATES: [State; N]` is the
  inverse. The variants are declared in document order, not in the sorted order they have now. The generated
  `History` follows the same rule.

### 3. The behaviour is one object-safe trait the core calls

What only the generated code can answer, and what only the engine can do, is one trait with no associated types
and no generic methods, reached as `&mut dyn`:

- `first_enabled_transition(StateId, EventId) -> Option<TransitionId>`: guards and event matching stay in the
  generated code.
- `exit_state(StateId, configuration_before_exit: &[StateId])`, `enter_state(StateId, is_default_entry)`,
  `execute_transition_content(TransitionId)`, `execute_history_default_content(HistoryId)`.
- What the main loop asks of the queues and invokes: whether an internal or external event waits, take the next
  one and bind it as `_event` (the typed payload is bound inside the call and never reaches the core), and start
  the pending invokes.

The typed layer implements it once for each machine. `EngineHost<P>` already has this role: it converts the numbers
to the machine's enums and calls the generated policy. A `dyn` trait was chosen over a hand-written table of
function pointers because the two compile to the same indirect call, and the compiler checks the trait's
signatures. When a program holds one machine and links with LTO, the compiler can turn the indirect calls back into
direct ones. The acceptance measurement shows whether it does.

### 4. The microstep's buffers are one workspace of the document's size, in the frame of the call that uses it

ADR 0056 decided that every buffer is the document's size and that the engine does not own a workspace. Both
still hold. What changes is where the buffers are declared:

- The generated policy spells `type Workspace = MicrostepWorkspace<C, E, H, W>`, each const parameter one of the
  bounds of decision 2. Stable Rust allows this because the numbers are literals in the generated code. An array
  of `[T; P::MAX_CONFIGURATION]` in generic code does not compile on stable Rust, which is why ADR 0056 used
  associated types.
- The typed layer declares the workspace as a local of the call that runs a macrostep and hands the core its
  buffers as slices. A slice-backed bounded vector takes a push, and a push past its end stops the program with the
  generator's name in the message, as ADR 0056 decision 3 has it now.
- ADR 0056 §4 rejected an engine-owned workspace for two reasons. It would cost static RAM for the whole run, and
  it would need the engine and the buffers borrowed apart through a hook that borrows the whole engine. A local in
  the calling frame has neither cost. It holds the same buffers the chain of frames holds today, and only for the
  length of the call.
- The buffers hold numbers, not structs. An enabled transition is a `TransitionId` of 2 bytes. `EnabledTransition`
  carries a source, a `&'static` target slice, a `usize` index and two flags, which comes to 16 bytes on a 32-bit
  target by its field layout (computed, not measured). The stack gate measures the change.
- Under `std` the workspace is the same fixed size. The bounds are upper bounds the generator derives and the debug
  build asserts (ADR 0056 decision 3), so a growable buffer does not hold anything more. The microstep's buffers
  then have no `std` and `no_std` variants.

### 5. The engine is a core with no generic parameter, inside a typed layer

`Engine<P>` is divided by the rule of decision 1:

- **`EngineCore`** (no generic parameter) holds what the algorithm reads and writes: whether the machine runs, the
  counters and the external-event budget (ARCHITECTURE.md "External-Event Budget"), the truncation state, and the
  last event or state a diagnostic names, stored as an `EventId` or a `StateId`. It also holds
  `run_main_event_loop`, `complete_macrostep`, the selection and the microstep, over `&DocumentTables`, the
  workspace's slices and `&mut dyn` behaviour. The configuration's storage is the document's size and is held by
  the typed layer. The core reads and writes it through a slice.
- **`Engine<P>`** holds the policy, the typed queues and scheduler, the configuration's storage and the public API.
  The public API keeps its spelling: `get_current_state() -> P::State`, `raise(EventWithMetadata<P::Event,
  P::Payload>)`. Each method converts and calls the core.

`PullScheduler<E, S>` and `EventQueueManager<T, N>` stay generic in this decision because they hold the machine's
typed events. Decision 1 applies to them too. They can hold `EventId`s, with the payloads in a separate store of the
machine's type. That change is the next one this decision leads to, but it is not decided here: it changes what a
saved state holds (ADR 0044, ADR 0045), and it needs its own measurement.

### 6. There is one structure and no build mode

No feature and no `cfg` selects between this structure and the present one. The present generic procedures are
removed in the same series of changes that adds the core, as `StateChain` and `SceTransitionBuf` were removed by
ADR 0056. If the acceptance measurement finds a regression, it is repaired in the core, for example by changing
which hooks the core calls or what the tables precompute. No second dispatch mode is added.

### 7. The instrument measures what a second machine costs, not which symbols carry its name

`footprint.budget` weighs the `.text` of symbols whose names carry the probe machine's crate. After this change the
algorithm's symbols carry no machine name, and the structure moves from code to `.rodata`. That instrument would
report a move as a shrink, which is the mistake its own comment records for 2026-09-24. The gate gains a probe image
that links two different machines, and holds three figures, in both directions as now:

1. **What one more machine costs**: the image with two machines minus the image with one, in `.text` plus
   `.rodata`. This is the figure a consumer multiplies by its machine count.
2. **What the program pays once**: the core's `.text`, weighed by its module path.
3. **The core is not instantiated per machine**: no symbol under the core's module path carries a machine type
   argument. The compiler already guarantees this for the non-generic functions. The check fails if a generic
   parameter is ever added back to one of them.

The stack gate (ADR 0056, `tools/mcu_stack/worst_case_stack.py`) has to account for the core's indirect calls: an
indirect call is charged with the largest frame among the hooks of the machines in the probe, not refused.

### 8. The other backends take the same structure, each in its own change

The tables of decision 2 come from one function in `sce-build`, so all backends can emit the same rows. The C11
backend then has one microstep in its runtime instead of one per machine, and the C++ AOT engine has one
`MicrostepAlgorithms` over the tables instead of one per policy. The C++ Interpreter builds the same tables at load
time from its parsed model, so the Interpreter and the AOT engine share the algorithm through data rather than
through a template. That is the end state of the Zero Duplication rule. Each of those moves changes a different
runtime and needs its own measurement, so this ADR decides only their direction.

## Alternatives considered

- **An opt-in feature or `cfg` that switches the procedures to dynamic dispatch** (the request as written). Rejected
  by ADR 0053 and the Zero Duplication rule, as the Context sets out.
- **A `&dyn Document` view over the present trait.** It does not produce one copy, because the associated types
  differ for each machine. Erasing them is decision 2.
- **Sharing only `walk_entry` and `select_transitions`** (about 2.2 KB of the second copy). That leaves the rest of
  the algorithm per machine and states no rule for which functions may be generic, so the next procedure added to
  the runtime would be generic again.
- **Identical code folding in the linker.** It merges only functions whose machine code is byte for byte the same.
  The two instances read different tables and inline different hooks, so their code is not the same.
- **Polymorphization in the compiler.** It is a nightly-only option and is not available on the stable toolchain
  this crate supports.
- **Precomputing more in the tables**: the configuration as a bitset, the ancestors and descendants of each state
  as bitsets, the exit set of each transition. This is how uSCXML's C transformation works. It is a choice of
  representation and does not affect whether the algorithm is shared. The ancestor tables grow with the square of
  the state count. And the exit set of a transition whose target is a `<history>` depends on what the history
  recorded, so it cannot all be computed at build time. Once the tables exist, this change can be made inside them,
  measured, and accepted or rejected by the gate of decision 7.

## Consequences

- **What a machine costs an image** is its tables, its generated behaviour and its typed layer. The algorithm is
  paid once per program. The watching-zenoh measurement suggests the second copy of the algorithm (about 3.5 KB)
  goes, less the tables and typed layer the second machine adds. That is an expectation. The figures of decision 7
  and the T2G measurement decide it.
- **Speed** is the risk. The hooks become indirect calls, and the generated code can no longer be inlined into
  the algorithm. The tables replace `match` expressions over the enums with indexed reads of a static array, which
  the compiler often emits for those `match` expressions already. Acceptance requires all of these: the kit lane's
  turnaround does not regress, the probe's frames do not grow past `STACK_BYTES`, and the Rust lane passes the W3C
  corpus.
- **Correctness evidence carries over.** The differential test (`tests/microstep_algorithms.rs`, 4,000 random
  documents against the reference recursion) builds tables instead of implementing `Document`, so the random
  documents can be generated as rows. The reference recursion stays the reference.
- **API.** A hand-written `StatePolicy` fails to compile once, as it did under ADR 0056. It now provides the
  tables and the behaviour hooks instead of the structural methods. The generated machines are regenerated. The
  public methods of `Engine<P>` keep their names and types. The generated enums keep their variants, but their
  declaration order and discriminants change, which a consumer that relied on the derived discriminants will see.
- **ADR 0056 is amended, not replaced.** The buffers are still the document's size and the runtime still names no
  capacity. They are now declared once, as the workspace of decision 4, rather than as associated types read across
  the chain of frames. Its §4 still holds.
- **A machine that has `<parallel>`, `<history>` or neither** runs the same core. The present design gets this from
  `NoHistory` being uninhabited. Under this design, a document with no histories has an empty histories table and
  the core never reads it. The cost of the history branches is paid once per program instead of being removed per
  machine.

## Order of the changes

Each step leaves the tree building and the corpus passing:

1. `sce-build`: the tables function, held to `MicrostepBounds::of` and to the present generated structural methods
   for every fixture (the same answers, read from the rows).
2. The runtime core over the tables, beside the present procedures, held to them by the differential test. The core
   is the subject of that test from here on.
3. The Rust templates: tables, `#[repr(u16)]` enums in document order, hooks over the numbers, the workspace type.
   The trees are regenerated, and `Engine<P>` is moved onto the core.
4. The present generic procedures and the structural methods of `StatePolicy` are removed.
5. The gate of decision 7: the two-machine probe, the three figures, and the stack walk with indirect calls.
6. The commit is sent to watching-zenoh, which bumps its pin and measures `.text` and the turnaround on the T2G kit.
   Its measurement is the acceptance of this ADR.
