# ADR 0125 — What a document fixes of a microstep is computed when it is built, and walked as rows of its tables

- Status: Accepted (2026-11-25), on the owner's approval of its scope, under the owner's standing rule: the most
  long-term correct, textbook structure, cost ignored. Its figures are the gate's measurement at the landing
  (section "Acceptance").
- Amends: ADR 0064 (decision 2, the tables; the alternative "precomputing more in the tables", which it left to be
  decided inside the tables and measured) and ADR 0066 (a program pays for what its documents use, carried from the
  two features to how a microstep's sets are formed).
- Related: ADR 0053 (the runtime's features only add), ADR 0056 (the microstep's buffers are the document's size),
  ADR 0067 (a row is as small as its numbers), ADR 0068, ADR 0123 (the core decides where a run ends).
- Requested by: a downstream bare-metal consumer, 2026-11-25.

## Context

The consumer measured its release C library, which holds two machines whose documents have no `<parallel>`, no
`<history>`, no `cond` and no eventless transition. `sce_rust_runtime` is 12,446 bytes of it, and about 9.5 KB of
that is the shared Appendix D procedures that compute a microstep's exit and entry sets at run time:

| symbol | bytes |
|---|---|
| `Turn::run` (the main loop, with what is inlined into it) | 2,012 |
| `walk_entry` | 1,686 |
| `enter_states` | 1,122 |
| `compute_entry_set` | 994 |
| `select_transitions` | 566 |
| `exit_states` | 536 |
| `transition_domain` | 492 |
| `effective_target_states_on` | 434 |
| `compute_states_to_exit` | 420 |
| `compute_exit_set` | 299 |

For those two documents every one of those sets is decided by the document alone. A transition's domain, the states
it enters and which of them are entered by default can be computed once, when the document is built, and the
runtime would read the answers instead of computing them again on every microstep. ADR 0064 named this as an
alternative ("the exit set of each transition ... a choice of representation, made inside the tables later, measured
and accepted by the gate") and set it aside only until the tables existed. They do.

## Decision

### 1. Appendix D is partially evaluated on what the document fixes

The textbook name is partial evaluation: an interpreter specialised on its static input leaves a residual program
that does only the dynamic part (Jones, Gomard and Sestoft, *Partial Evaluation and Automatic Program Generation*;
the first Futamura projection). Here the interpreter is Appendix D's microstep, its static input is the document,
and its dynamic input is the configuration. For a document in the class of decision 2, the generator runs
Appendix D's own procedures for each transition, and writes what they answer as rows of the document's tables: the
states the transition enters, in entry order, each with whether it is entered by default. What the transition exits
is the dynamic part, and the row says where it is read from.

Appendix D's meaning is unchanged. The rows are what the procedures would compute, the order of every exit, every
content and every entry is the order the procedures take, and the tests of decision 6 hold the two to each other.

### 2. The class: no `<history>`, no `<parallel>`, no transition of more than one target

The class is read off Appendix D, not assumed. A document is in it when it has no `<history>`, no `<parallel>`,
and no transition row (written, a compound state's initial transition, or the document's own) with more than one
target. For such a document:

1. **What a transition enters depends on the transition alone.** getEffectiveTargetStates differs from the targets
   only for a `<history>`, so without one the effective targets are the targets. getTransitionDomain reads the
   source, the targets and the type, so the domain is fixed. computeEntrySet reads the configuration only through a
   history's record (addDescendantStatesToEnter) and through the regions already on the set (a `<parallel>`'s
   defaults); without either, statesToEnter and statesForDefaultEntry are functions of the transition, and
   defaultHistoryContent is empty.
2. **What it enters is one path down the tree.** With one target `t` and domain `d`, addAncestorStatesToEnter adds
   the proper ancestors of `t` below `d`, and addDescendantStatesToEnter adds `t` and, for a compound `t`, its
   initial transition's one target `y`, which §scxml-3.3 requires to be a descendant of `t`, with the ancestors
   between them, and so on down. Every non-atomic state is compound, so this ends at an atomic state. In entry
   order, which is document order, the set is a path: its first state is the child of `d` on the way to `t`, each
   further state a child of the one before, and the last atomic.
3. **Every configuration is one path down the tree.** By induction over the run: the initial configuration is the
   path the document's own initial transition enters (its domain is the `<scxml>` element). A microstep from a
   path takes one transition at most (one atomic state, so selection finds one transition at most), whose source
   is on the path and whose domain is the source or one of its ancestors, so on the path too, or the `<scxml>`
   element. computeExitSet removes every active state below the domain, which leaves the path down to the domain,
   and the entry adds a path that starts at a child of the domain.
4. **What it exits is every active state below its domain, and the domain is the parent of the path's first
   state.** computeExitSet is the active descendants of the domain, in exit order. A subtree is a range of ids, so
   the set is the configuration read over that range, last first; this needs nothing of decision 2's third point,
   it holds of any configuration.
5. **The current state is the path's last state.** The engine's current state is read down the configuration from
   the transition's first target. After the microstep, the states below the domain are exactly the path, so the
   descent ends at the path's last state.

A compound state's initial transition is never a microstep's transition: what it enters is part of the path of
whatever enters its state. Its row is empty, and the runtime's check holds it so.

`cond` guards and eventless transitions change which transition is selected, not what a selected transition exits or
enters: selectTransitions and selectEventlessTransitions decide, and the sets are a function of the transition they
return. Documents that use them stay in the class, and the selection stays the shared procedure.

A document in the class whose sets are not each one path, which only a compound state whose initial transition does
not target one of its descendants can make, gets no rows (`DocumentTables::entry_paths` answers `None`) and is
walked by the general procedures; §scxml-3.3 makes such a document an error, so the corpus test of decision 6
expects none.

**The exit part: one procedure, rows per transition.** A transition whose source is compound exits the active
descendants of its domain, which depend on the active leaf. Two representations were weighed:

- rows per (transition, active leaf): finite, but a transition from a compound state needs one exit list for each
  atomic state below its domain, so the rows grow with nesting, and the configuration already holds the answer;
- the exit set as the active states of the domain's subtree, read off the configuration last first, from the
  domain alone.

The second is chosen. The configuration is a bit vector over document order (ADR 0064, amendment), so the read is
the one exitInterpreter already makes, over a range: exitInterpreter is the case whose domain is the `<scxml>`
element, and the two share one procedure. A chain needs no set algebra, the row holds no exit list, and the domain
need not be stored, since it is the parent of the path's first state. The entry part is static and is the row.

### 3. One core, and the tables carry how a microstep's sets are formed

`helpers::microstep` gains `WalkProcedures`, a pair of function pointers with private fields and two values:

- `GENERAL_WALK`: exitStates and enterStates as they are (`exit_states`, `enter_states`), computing the sets for
  any document;
- `ROW_WALK`: `exit_states_from_rows` and `enter_states_from_rows`, reading them from the rows.

`DocumentTables` gains `walk: &'static WalkProcedures`, `entry_rows` and `entry_paths`. The generator writes
`walk: &ROW_WALK` and the rows for a document in the class, and `walk: &GENERAL_WALK` and no rows for any other.

A pair rather than one pointer for the whole microstep, because Appendix D has two procedures a document can fix,
and calls one of them alone: the microstep is exitStates, executeTransitionContent, enterStates, and the entry into
the initial configuration is enterStates of the document's initial transition. `microstep` and the new
`enter_initial_configuration` call through `tables.walk`, and nothing else does; executeTransitionContent, the
selection, removeConflictingTransitions, isInFinalState and the main loop are shared and unchanged. What a single
state's exit and entry do is one function for both walks (`exit_one`, `enter_one`): the hook, then the run store,
and on entry the end of a run on a top-level `<final>` as ADR 0123 decided. The row walk's exit reads the
configuration over a range with `exit_active`, which exitInterpreter calls over the whole document.

As in ADR 0066, the general walk's procedures are referenced only from `GENERAL_WALK`, and `GENERAL_WALK` only from
the tables of a document outside the class. A program none of whose documents carries it links none of it. There is
no build flag, no `cfg` and no second dispatch mode chosen by the build (ADR 0064 decision 6, ADR 0053): each
document decides through its tables.

### 4. The rows are as small as their numbers

- **`EntryRow`**, one for each transition, 2 bytes: the position of its path in `entry_paths`, or `EntryRow::NONE`
  for a transition with no target and for a compound state's initial transition.
- **`EntryStep`**, 2 bytes: a state's 14-bit id and, in the top bit, whether it is entered by default (ADR 0067).
  A path is its length (`EntryStep::count`) followed by its states.
- **Paths are shared.** The generator writes each distinct path once, and every transition that enters it names it,
  which is every transition to one target from under one domain. A position is a 14-bit field, so a document whose
  paths need more than 16,383 entries is refused and the refusal names the pool, as for the pool of targets.
- **`DocumentTables::check`** refuses the row walk for a document outside the class, rows whose count is not the
  transitions', rows the general walk would not read, and a path the row walk cannot take: one that is not a path
  down the tree, does not hold the transition's target, does not end at an atomic state, enters by default a state
  above the target or a state that is not compound below it, does not enter a compound target by default, or whose
  first state's parent is not the source or an ancestor of it. A compound state's initial transition must have none.
- **The workspace.** The row walk builds no set of states and walks no stack. The generator writes the workspace of
  a document in the class as `Workspace<0, ENABLED, 0, 0>`: the enabled set alone. The selection lends the rest of
  the scratch only to removeConflictingTransitions, which no such document has.

### 5. What each program links of the measured list

| symbol | every document in the class | a document outside it |
|---|---|---|
| `Turn::run`, `select_transitions` | linked | linked |
| `walk_entry`, `enter_states`, `compute_entry_set`, `exit_states`, `compute_states_to_exit` | not linked | linked |
| `transition_domain`, `effective_target_states_on`, `compute_exit_set` | not linked | linked (and by `PARALLEL`'s removeConflictingTransitions) |
| `exit_states_from_rows`, `enter_states_from_rows`, `exit_active` | linked | linked when another document is in the class |

Of the consumer's measured 9.5 KB, the rows of the second and third lines are 5,983 bytes that a program of
documents in the class no longer links. It links the row walk instead, a few hundred bytes, and pays in `.rodata`
2 bytes for each transition and 2 for each entry of a distinct path. `Turn::run` keeps the main loop; what of the
general walk was inlined into it goes with the walk. These are expectations: the gate's figures decide them.

### 6. The proof

- **The differential test** (`backends/rust/runtime/tests/microstep_algorithms.rs`) draws random documents of the
  class, builds their rows from the reference recursion's computeEntrySet, and runs each from its initial
  configuration through random transitions with the general walk and the row walk side by side. Every exit,
  content and entry, in order, every configuration and every `Entered` must be the same; the exits must be the
  reference's exit set in exit order and the entries its entry set in entry order; every configuration must be one
  path down the tree (decision 2, third point); and exitInterpreter must exit what is left alike. The row walk is
  lent an empty workspace. The cases of where a run ends run on both walks, and the check's refusals have a case
  each.
- **The corpus** (`sce-build/tests/a_documents_tables_are_its_tree_in_document_order.rs`): every generatable
  document in the class has an entry path for each transition, each is what the runtime's general
  `compute_entry_set` and `compute_exit_set` answer for that transition, and the tables of the row walk pass the
  runtime's check.
- **The W3C corpus** on the Rust lane: every fixture in the class is generated with `ROW_WALK` and runs on it.
- **The footprint gate** (`scripts/gates/nostd-mcu.sh`): the `plain-documents` configuration, whose two documents
  are in the class, must link no procedure of the general walk, as it links no feature procedure (ADR 0066); the
  other two configurations, whose first document has a `<parallel>` and a `<history>`, must link it; every
  configuration links the row walk. The stack tool follows a call through `tables.walk` into the walks'
  procedures. The figures are re-pinned by measurement.

### 7. The other backends emit the same rows, each in its own landing

`sce_build::document_tables::DocumentTables::entry_paths` is a function of the tables alone, as ADR 0064 decision 8
asks of everything every backend emits. The C11 runtime's one microstep (ADR 0110) gains the same two rows and a
pointer in its tables struct, and the C++ AOT engine's `MicrostepAlgorithms` a row walk beside the general one,
reached through the tables in the same way. Each changes a different runtime and needs its own measurement, so each
is its own landing.

## Consequences

- A program whose documents are all in the class does not carry Appendix D's set computations; one with a document
  outside it carries both walks, the row walk being small.
- A document in the class pays 2 bytes per transition and 2 bytes per entry of each distinct path in `.rodata`,
  and its turn's workspace shrinks to the enabled set.
- `DocumentTables` gains three fields, so a hand-written table fails to compile once; it writes
  `walk: &GENERAL_WALK, entry_rows: &[], entry_paths: &[]`, which serves any document. The generated machines are
  regenerated.
- `microstep::enter_states` and `exit_states` are the general walk's and keep their names; the engine enters the
  initial configuration through `enter_initial_configuration`.
- The general walk stays the reference for every document; the row walk is held to it, never the reverse.

## Alternatives considered

- **Exit rows per (transition, active leaf).** Decision 2 weighs it: rows that grow with nesting for an answer the
  configuration already holds. Refused.
- **A row walk chosen by a feature or `cfg`.** Two dispatch modes chosen by the build, which ADR 0064 decision 6 and
  ADR 0053 refuse; the document decides.
- **Rows for documents with a `<parallel>`.** Its entry set depends on which regions the set already enters, and
  several transitions enter at once; the sets are not one path. Precomputing per selected set is the
  representation decision 2 refused, multiplied. Refused; they keep the general walk.
- **Rows for documents with a `<history>`.** What a transition to a history enters is its record, known only at run
  time, which ADR 0064's alternatives already noted. Refused.
- **Storing the domain in the row.** It is the parent of the path's first state. Refused as a second copy of one
  fact.
- **Generating each machine's exits and entries as code.** That is the per-machine copy ADR 0064 removed.

## Measured

On one build machine, `thumbv7em-none-eabihf`, `opt-level = "s"`, rustc 1.100.0, with the gate's own arithmetic
(`.text` plus `.rodata` of the probe staticlib, the image of two machines minus the image of one; the core by
`CORE_SYMBOLS`), before (the parent, ADR 0124 landed) and after in one run:

| | before | after |
|---|---|---|
| two plain documents: image of one / of two machines | 206,872 / 207,804 | 204,234 / 205,166 |
| two plain documents: one more machine, core | 932, 5,584 | 932, 2,906 |
| a `<parallel>` and `<history>` document beside a plain one: image of two machines | 210,705 | 211,369 |
| the same: one more machine, core (with the report / without) | 868 / 860, 7,890 / 7,782 | 1,462 / 1,454, 8,504 / 8,396 |

- A program whose documents all fix their sets links no general walk: `walk_entry`, `transition_domain`,
  `effective_target_states_on`, `compute_exit_set` and the general `enter_states` / `exit_states` are absent from
  the plain image, which holds `enter_states_from_rows`, `exit_states_from_rows` and `exit_active` instead. The
  image is 2,638 bytes smaller and the core 2,678.
- A program that holds both kinds of document links both walks, by decision 3: the second machine of the mixed
  configuration is the probe's plain one, so it brings the row walk (594 bytes) into a program that already had the
  general walk. That is what such a program pays; the gate holds it.
- The worst-case stack is read by the nostd-mcu gate; the build machines carry no `thumbv6m` standard library, so
  its stack step was not run there.

## Order of the changes

1. This ADR.
2. The runtime: `WalkProcedures`, `GENERAL_WALK`, `ROW_WALK`, the rows and their check; the shared per-state exit
   and entry; the engine enters the initial configuration through the tables' walk; the differential test and every
   hand-written table.
3. `sce-build`: the class test and the entry paths beside `DocumentTables::of`; the Rust template writes the walk,
   the rows and the workspace; the corpus test.
4. The footprint gate's expectation and the stack tool's mapping; ARCHITECTURE.md's "Rust AOT Backend".
5. The main session regenerates the committed trees, runs the Rust lanes and the gate, and re-pins the figures.

## Acceptance

Accepted on all of these: the differential and corpus tests pass; the Rust lane passes the W3C corpus; the
`plain-documents` image links no procedure of the general walk, and its shared core, image and worst-case stack are
re-pinned at what the probe measures; the consumer measures its own image at the landing.
