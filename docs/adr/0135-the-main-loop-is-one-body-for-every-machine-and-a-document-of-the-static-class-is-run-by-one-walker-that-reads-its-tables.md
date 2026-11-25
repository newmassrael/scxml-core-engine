# ADR 0135 — The main loop is one body for every machine, and a document of the static class is run by one walker that reads its tables

- Status: Accepted (2026-11-26), under the owner's standing rule: the most long-term correct, textbook structure,
  cost ignored. The Rust part is implemented in the order of the last section; its figures are the footprint gate's
  and the consumer's measurement at the landing (section "Acceptance").
- Date: 2026-11-26
- Supersedes: ADR 0129 (a document of the static class printed as its own step function). Its class and its
  differential test are kept, bound to what replaces it (decisions 3 and 4).
- Amends: ADR 0125 (decision 3: the row walk carries the selection of its class as well as its exit and entry
  sets), ADR 0127 (decision 2: a document the row walk serves is selected from its candidate rows, not from the
  selection rows), and ADR 0064 (decision 6 holds again as written: one structure, no build mode, no form an image
  chooses).
- Related: ADR 0066 (a program pays for what its documents use), ADR 0068 (the main loop is one copy), ADR 0074 (the
  engine's per-turn state belongs to its shared core), ADR 0123 (what stays per machine and why, decision 3),
  ADR 0131 (a policy's hook is handed the engine's state apart from the policy).
- Requested by: a downstream bare-metal consumer, 2026-11-26.

## Context

The consumer measured its release image (fat LTO; the crate-level `opt-level` does not change these figures) with
three machines printed as their own step functions (ADR 0129):

| symbol | bytes |
|---|---|
| `Engine<P>::run_turn` | 1,303 to 1,327, one body per policy, three bodies |
| `Turn<EngineHost<P>, StepFunctionForm<P>>` | one instantiation per machine, three bodies |
| a machine's `select_transition` | 1,720 / 1,622 / 832 |
| a machine's `microstep` | 1,077 / 995 / 439 |

The step-function form added 6,970 bytes to the image's transport area where it was meant to save. Each arm inlines
`DocumentTables::first_enabled` and the policy's hooks with constant arguments, so the directly coded form is the
whole selection and the whole microstep written out again for every atomic state of every machine, and the main loop
it was generic over is compiled once for each machine. ADR 0129's acceptance asked for an image of printed machines
smaller than the interpreted one; the measurement refutes it.

The consumer proposed the shape that measurement points to: the form's procedures as methods of `MainLoop`, and
`Turn` one body over `dyn MainLoop`, keeping the generated code's direct hook calls; they estimated about 2,600
of the 4,617 bytes saved. The repository's own decisions already aim there: ADR 0064 runs Appendix D once per
program, ADR 0068 and ADR 0074 make the loop and its bookkeeping the core's, and ADR 0123 decision 3 lists what
stays per machine and why. ADR 0129 is the one decision that made the loop generic again.

## Decision

### 1. One main loop, one body, for every machine and every path

The textbook basis is the one ADR 0064 took: an interpreter written once over the program's data, with the
per-type part reached through an object interface (dynamic dispatch through a vtable, as a C++ virtual call is), so
the code that does not depend on the type is not instantiated per type.

- `engine::Turn` (§scxml-D-mainEventLoop: the macrostep's eventless transitions and internal events to completion,
  the invocations it entered, one external event at a time, the budgets, the error chain, the end of the run) is a
  non-generic struct over `&mut dyn MainLoop`. The program compiles it once. It reaches the document's procedures
  through the tables (`helpers::microstep`, decision 3), never through a type parameter.
- Every path into it goes through one shared, out-of-line entry: `initialize`, `step`, `tick`,
  `process_event` and `process_event_with` reach it through `Engine::run_turn`, and `stop` and the end of a run
  reach `exit_interpreter` (§scxml-D-exitInterpreter), which takes `&mut dyn MainLoop` too.
- The per-policy shells keep only what is the policy's own layout: `run_turn` builds the workspace of the
  document's size and the `EngineHost` and hands both to the shared entry; `new_in_place` writes the fields;
  `queue`, `store`, `core` and `behaviour` are address computations; `bind_queued` binds the typed payload.
  These are ADR 0123 decision 3's list, unchanged.
- ADR 0131's split borrow is kept as it is: `EngineHost` holds the engine, and every hook is dispatched by
  `Engine::with_policy`, the policy and `EngineState` borrowed apart. No hook holds two references to one place,
  and the Miri test of the hook dispatch (`a_hook_borrows_the_policy_and_the_engine_state_apart`) runs unchanged.

### 2. ADR 0129's printed form is retired

Removed from the runtime: `StepFunction`, `Steps`, `StepFunctionForm`, `StatePolicy::STEP_FUNCTION`,
`NO_WALK`, and the `Form` parameter of `Turn`. Removed from `sce-build`: the `step_function` module,
`RustMachineForm` and `StatechartCodegenOptions::form`, the `--step-function` option of `generate` and `check`,
the template's step-function blocks, and the diagnostic `codegen/step-function-outside-class` (its appendix row in
`docs/SCE_ACCEPTED_SUBSET.md`, its row in `SCE_ERROR_CONTRACT.md`, its value in the schema's code list and its
golden; `ALL_DIAGNOSTIC_CODES` holds 568 codes). The schema is `pre-release` (SCE_ERROR_CONTRACT.md §8.1), so the
removal needs no `v` bump. Removed from the tests: the generations of both forms in
`tools/codegen/trees/rust-integration.toml`, `sce_rust_tests::step_function_differential` and its test, and the
runtime test of a printed step function. The two consumer-shaped documents stay in the corpus, under
`tests/integration/static_class/`, and the differential of decision 4 reads them.

What is kept: the class (decision 3) and the idea of a differential test that holds a specialised execution of the
class to the general one (decision 4).

### 3. One walker for the static class, chosen by the tables

The class is ADR 0125's: no `<parallel>`, no `<history>`, no transition of more than one target. Its
configuration is one path down the tree, a selection takes one transition at most, and what a transition exits and
enters is fixed when the document is built. (ADR 0129 also left out `<invoke>`; the walker does not start or
cancel an invocation, the loop and the hooks do, so the class does not need to.)

**The walker is `ROW_WALK`, and it is one copy in the program.** ADR 0125's `WalkProcedures` gains a third
procedure, the selection, beside exitStates and enterStates. `GENERAL_WALK` holds the general selection
(`select_transitions_on_tree`: every atomic state of the configuration and its ancestors, the selection rows of
ADR 0127, removeConflictingTransitions); `ROW_WALK` holds the class's (`select_transition_from_rows`), with
`exit_states_from_rows` and `enter_states_from_rows` as before. `helpers::microstep::select_transitions` calls the
document's through `tables.walk`, as `microstep` and `enter_initial_configuration` call the other two. Every
procedure takes `&mut dyn Behaviour`: no machine instantiates any of them, and a program none of whose documents is
outside the class links none of the general selection or walk (ADR 0066).

**What the walker reads is the document's data alone.**

- The atomic state: the configuration's last state, since every other active state is one of its ancestors and a
  state is numbered after its ancestors.
- The candidate rows (`DocumentTables::candidates`, `CandidateRows`), the selection of decision 3 fused over the
  ancestor chain: for each atomic state and each event the document numbers (the null event among them), the
  transitions §scxml-D-selectTransitions would try, in its order (the atomic state's own transitions in document
  order, §scxml-3.13, then each ancestor's, outward). The selection asks the machine `guard_holds` for each in turn
  and takes the first that holds; with the null event it is §scxml-D-selectEventlessTransitions. A guard that
  cannot be evaluated is false and raises `error.execution` in the policy's hook (§scxml-5.9), as for every document.
- The entry rows and paths of ADR 0125 (the step rows: what each transition enters, in entry order, with whether each
  state is entered by default; what it exits is every active state below its domain, the parent of the path's first
  state), and the state rows' parent and subtree.

**The candidate rows are a compressed sparse row table, not a dense one.** A dense table indexed by (state, event)
grows as the product of the two, though a state answers a few of a document's events. The rows are the standard
storage of a sparse relation (compressed sparse row: an offset per row, then the row's non-empty cells):

- `first_cell`: for each state, by `StateId`, the position of its first cell, and one entry past the last state.
  A state that is not atomic has no cell.
- `cells`: `CandidateCell`, 4 bytes, an event and the position of its list of candidates; a state's cells are in
  ascending order of the event, and the selection finds one by binary search.
- `candidates`: the lists, each its length (`Candidate::count`) followed by its transitions; identical lists are
  held once, as every pool of the tables is (ADR 0067).

The tables cost `2 (S + 1) + 4 C + 2 P` bytes for `S` states, `C` non-empty cells and `P` pool entries, in place
of ADR 0127's selection rows, which a document the row walk serves no longer carries.

**The tables decide, not the build.** `sce_build::document_tables::DocumentTables::candidates` computes the cells
from ADR 0127's rows (`DocumentTables::selection`), so the two selections are made from one list of the events
each transition answers, and the Rust template writes them for every document that fixes its sets (ADR 0125's test,
`DocumentTables::entry_paths`). Such a document's tables carry `walk: &ROW_WALK`, the entry rows, the candidate
rows and no selection rows; every other document's carry `walk: &GENERAL_WALK`, the selection rows and no
candidate rows. `DocumentTables::check` holds this (`TablesFault::Candidates`) and refuses a cell that is not a
cell of the class's selection (`TablesFault::CandidateCell`): a cell of a state that is not atomic, cells not in
ascending order of the event or past the null event, an empty list or one past the pool, and a candidate that is not
a written transition of the state or of one of its ancestors, or not in the selection's order.

**Appendix D is kept whole for the class**, because everything else is the one loop and the one set of hooks:
eventless transitions to completion and the internal queue before the external one are `Turn`'s; `done.state.<id>`
is raised by the policy's entry hook on entering a `<final>` child of a compound state, and a top-level `<final>`
ends the run by `Entered::ended_in` (ADR 0123); the order of `<onexit>`, transition content and `<onentry>` is
`microstep`'s, and the errors of §scxml-5.9 are the hooks'.

### 4. A differential test binds the walker to the general interpreter

- **The corpus** (`sce-build/tests/the_class_walker_runs_as_the_general_interpreter_does.rs`): every generatable
  document of the corpus that fixes its sets, the two consumer-shaped documents of `tests/integration/static_class`
  among them, is tabled twice from one model, as the generator tables it (the row walk with its entry and candidate
  rows) and as the general interpreter would read it (the general walk with the selection rows). Seeded random
  sequences of the document's events drive both through `helpers::microstep`, by a loop that takes eventless
  transitions to completion and the internal queue before the external one, with a machine that answers each guard
  from the seed, raises `done.state.<id>` on entering a `<final>` child of a compound state where the document
  numbers it, and keeps a journal of every hook call. After every step the configurations and `Entered` must be
  equal, and at the end the journals: the guards asked, the exits, the contents and the entries, in order.
- **Random documents of the class** (`backends/rust/runtime/tests/the_class_walker_selects_as_the_general_selection_does.rs`):
  for every atomic state and every event, the walker's selection and the general selection answer the same
  transition and ask the same guards in the same order, and the check refuses each fault of the candidate rows.

### 5. The footprint gate

`backends/rust/probes/nostd-footprint/footprint.budget` names the procedures as they are: `CORE_SYMBOLS` loses
`engine::Tables`, `GENERAL_WALK_PROCEDURES` gains `select_transitions_on_tree` and `ROW_WALK_PROCEDURES`
`select_transition_from_rows`, so `plain-documents` must link no general selection as it links no general walk.
The figures are re-pinned by measurement.

## Consequences

- An image links `Turn`, `exit_interpreter` and each walker it uses once, whatever number of machines it holds;
  a machine brings its tables, its policy's hooks and the shells of decision 1.
- A document the row walk serves carries candidate rows in place of selection rows; `first_enabled` is the general
  selection's, and `Behaviour::first_enabled_transition` stays for tables written by hand, which the general walk
  serves.
- `DocumentTables` gains `candidates`, so a hand-written table fails to compile once; it writes
  `candidates: None`. Every generated machine is generated again (the template hash moves).
- ADR 0129's form and its diagnostic are gone; a build that passed `--step-function` is refused by the argument
  parser as any unknown option is.

## Alternatives considered

- **Keep the printed form and fix its inlining.** The arms are what a directly coded automaton is; their cost is the
  per-machine copy ADR 0064 removed, and the measurement shows it does not pay at three machines. Refused.
- **The form's procedures as methods of `MainLoop`, generated per machine with direct hook calls** (the consumer's
  proposal). It makes the loop one body, which decision 1 takes, but keeps a per-machine copy of the selection and
  the microstep. The walker of decision 3 reads data the generator already computes, with one copy in the program.
- **A dense (state, event) table.** Constant-time lookup, at a size that is the product of the document's states and
  events. Refused for the compressed rows, whose lookup is a binary search over the few events a state answers.
- **A walker with its own main loop for the class.** A second copy of §scxml-D-mainEventLoop, which ADR 0068 removed.
  Refused: the loop is one, and the class differs only in what reads the document.
- **Choosing the walker by a build option.** ADR 0064 decision 6 and ADR 0053: each document's tables decide.

## Order of the changes

1. This ADR; ADR 0129 marked superseded; ADR 0064, 0125 and 0127 marked amended.
2. The step function differential and its generations removed; the two consumer-shaped documents moved to
   `tests/integration/static_class/`.
3. `sce-build`: `--step-function`, `RustMachineForm`, the `step_function` module, the template's blocks and
   `codegen/step-function-outside-class` removed.
4. The runtime: `Turn` one body over `dyn MainLoop`; `StepFunction`, `Steps`, `StepFunctionForm`,
   `STEP_FUNCTION` and `NO_WALK` removed.
5. The walker: `CandidateRows` and its check, the selection in `WalkProcedures`, the generator's candidate rows,
   the two differential tests, and the footprint gate's names.
6. The main session generates every Rust machine again, runs the Rust lanes, clippy and the footprint gate, and
   re-pins the figures.

## Acceptance

Accepted on all of these: the two differential tests pass; the Rust lanes pass; the footprint gate's images link one
`Turn` and no core symbol carries a machine's type; `plain-documents` links no procedure of the general selection
or walk; and the consumer's image of three machines links one `Turn` body and one `run_turn` shell per machine of
the size of its workspace's construction.
