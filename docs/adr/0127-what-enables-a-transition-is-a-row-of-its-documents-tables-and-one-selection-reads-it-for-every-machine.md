# ADR 0127 — What enables a transition is a row of its document's tables, and one selection reads it for every machine

- Status: Accepted (2026-11-25), under the owner's rule that the most long-term correct structure is chosen and its
  cost is not a reason against it. Its figures are read off a downstream consumer's release image; the footprint
  gate measures the landing.
- Amends: ADR 0123 (a machine's behaviour is indexed by the numbers its tables are), carried from where a run ends to
  which transition an event enables.
- Related: ADR 0064 (Appendix D runs once over a document's tables), ADR 0067 (a row is as small as its numbers),
  ADR 0074 (the engine's per-turn state belongs to its shared core), ADR 0125 (what a document fixes of a microstep is
  walked as rows), ADR 0131 (a policy's hook is handed the engine's state apart from the policy; `guard_holds` is one
  of those hooks).
- Requested by: a downstream bare-metal consumer, 2026-11-25.

## Context

The consumer runs two generated machines in one no_std image and reported about 1.5 KB of `sce_rust_runtime` code
copied once for each machine's policy: `first_enabled_transition`, `run_turn`, `new_in_place`, `queue` and the rest
of the engine's hooks. Its release image (x86_64, size profile; byte counts from the symbol table) holds:

| symbol | per machine (two machines) | one copy |
|---|---|---|
| `Behaviour::first_enabled_transition` | 177, 316 | |
| `Engine::run_turn` | 158, 158 | |
| `Engine::new_in_place` | 135, 121 | |
| `MainLoop::queue` | 73, 67 | |
| `Behaviour::store` | 57, 57 | |
| `Behaviour::execute_transition_content` | 48, 54 | |
| `MainLoop::bind_queued` | 46, 46 | |
| `Behaviour::enter_state`, `exit_state` | 1 and 1, 100 and 36 | |
| `MainLoop::behaviour`, `core`, the rest | 20, 20 | |
| `Turn::run` (the main loop) | | 1,632 |
| `select_transitions` | | 544 |
| `exit_states_from_rows`, `enter_states_from_rows` | | 656, 616 |
| `Turn::take_microstep`, `take_event`, `exit_interpreter` | | 390, 130, 256 |
| `EventQueueMut::push`, `pop` | | 88, 103 |

So the work that does not depend on the policy, which is the turn loop, the selection walk over the configuration,
the exit and entry walks and the queue operations, is already one copy in the image (ADR 0064, ADR 0068, ADR 0074).
What is still copied for each machine is of two kinds:

1. **The shells that hand the shared core the machine's typed storage** (`store`, `queue`, `core`, `behaviour`,
   `bind_queued`, `run_turn`, `new_in_place`): about 200 to 290 bytes for each machine. Each is the field layout of
   `Engine<P>` (queues and a workspace sized by the document) and nothing else. Their bodies are already a handful of
   address computations. A copy that serves every machine needs the layout erased, by offsets read at run time or by
   views held across the hooks. ADR 0123 decision 3 explains why the hooks cannot hold views. Neither is smaller than
   the shells it would replace.
2. **The document's own selection**, `first_enabled_transition`: a `match` over states, then the event numbers each
   transition answers, then its guard. This is not a copy of shared code. It is the document's data compiled as
   instructions and jump tables (177 and 316 bytes, plus their tables in `.rodata`). It is the part that ADR 0125's
   direction moves into the tables.

## Decision

### 1. The rows

For a generated machine, `DocumentTables` gains a `selection` of three tables:

- `first_written`: for each state, in state order, the first written transition's number, plus one entry past the
  last state. The written transitions are the first rows, grouped by source in state order (`position_in_source`), so
  a state's own transitions are the range between its entry and the next. Two bytes for each state.
- `triggers`: for each transition, the position in `trigger_events` of the list of events it answers, or `NONE`
  for a transition no selection returns: a compound state's initial transition, a history's default, the document's
  initial transition, or a written transition that answers no event of the machine. Two bytes for each transition.
- `trigger_events`: the lists, each its length followed by its events in ascending order. An eventless transition's
  list is the machine's "no event" alone, and no other list holds it. Identical lists are held once. Two bytes for
  each entry.

The runtime spells them `SelectionRows`, `TriggerRow` and `TriggerEvent`, and `DocumentTables::check` holds them to
these rules (`TablesFault::Selection`, `TablesFault::Triggers`). The Rust table generator writes them when it
writes the machine (decision on the open questions, 1).

### 2. One selection

`select_transitions` asks `DocumentTables::first_enabled(state, event, machine)`. It walks the state's range in the
order the transitions are written, which is document order (§scxml-3.13), reads each transition's list, and asks the
machine `Behaviour::guard_holds(transition)` only for a transition that answers the event, until one holds. The
eventless selection and the selection for a named event stay separate passes: the main loop asks the first with the
null event, whose only list is the eventless transitions'. A guard is still the only part that is the machine's own.
`StatePolicy::guard_holds` is generated as a `match` over the guarded transitions, and its default holds for a
document with no guard. The policy's `first_enabled_transition` stays for hand-written tables, whose `selection` is
`None`; a generated policy no longer writes it, and its default stops a policy of tables without rows that does not
write it either, with a message that says so. This is the same split as `GENERAL_WALK` and `ROW_WALK` (ADR 0125
decision 3): each document's tables decide, not the build.

The rows are the selection of both forms of ADR 0129. A document printed as its own step function keeps them in its
tables: its arms name which states a selection asks, and each is answered by this same `first_enabled`, through
`Steps::enabled`, so a printed policy writes no `first_enabled_transition` either and its guards run where the
shared selection would run them. `first_enabled` is generic over the machine it asks (`M: Behaviour + ?Sized`): the
shared selection passes `&mut dyn Behaviour` and is one copy for the program, and a step function passes the engine
itself, whose guard calls are then direct.

### 3. What it costs and saves

The turn makes the same walk over the configuration. For each state it visits, it reads a range and compares the
event with short lists instead of jumping through a `match`. It makes a virtual call only for a transition that
answers the event, which for a document with no guard reaches a hook that returns `true`. Expected on the consumer's
image: both `first_enabled_transition` copies (493 bytes and their jump tables) become rows of about 2 bytes for each
state, transition and list entry. One `first_enabled` of 100 to 150 bytes is added once for the program. The
footprint gate decides the figures.

## Decisions on the open questions

1. **The lists are made from the events the machine numbers, when the machine is generated.** They are not built from
   the analyzer's `Transition::prefix_matching_events`: `build_prefix_matching` runs in the analyzer, and events can
   be added to `model.events` after it (a test component's snapshot lowering adds the events its branches receive),
   which a wildcard transition matches at run time but would not match in a row. The Rust table generator
   (`rust_tables`, beside `tables.written`) lists the named events in the order the event enum numbers them
   (`generator::rust_numbered_events`, the order the template's `sort` filter gave before, so no machine's numbers
   moved), and the template now declares the enum, its `ID_<variant>` numbers, `EVENTS`, the name tables and
   `error_events` from that same list. For each written transition it asks every member, by the name the machine's
   `get_event_name` answers for it (the wildcard member's is `*`), whether the transition's descriptor matches, with
   the function a machine that matched at run time called:
   `sce_rust_runtime::helpers::event_matching::matches_event_descriptor` (§scxml-3.12.1), which `sce-build` links.
   So there is one definition of the match, and the rows are made from the set that is numbered. An eventless
   transition's list is the null member alone. `document_tables::DocumentTables::selection` computes the rows; the
   corpus test `a_transitions_enabling_events_are_the_rows_its_event_enum_numbers` holds them, for every generatable
   document, written transition and member, to the comparison the generated `first_enabled_transition` made (an
   enum compare for a single literal, the descriptor match by name for any other, the null member alone for an
   eventless transition).
2. **The rows hold no guard bit.** `StatePolicy::guard_holds(transition, event_data, engine)`, whose `engine` is the
   engine's state (`&mut EngineState<Self>`, ADR 0131) as every hook's, is generated by the template as a `match`
   over exactly the transitions that took the guard branch before (`{% if trans.cond or trans.native_guard %}`, a
   test component's snapshot branch included), each arm the same
   `guard_expr` macro, so the template stays the only judge of which transitions are guarded; every other transition
   holds, and the trait's default holds for a document with no guard. A guard that cannot be evaluated is still
   false and raises `error.execution` through `safe_evaluate_guard`, and `bind_current_event` still binds the event
   before the first guard of a selection runs (§scxml-5.10): the engine calls it before `select_transitions`, as
   before.
3. **Selection order is §scxml-3.13's.** A state's transitions are tried in document order: the written rows are
   grouped by source in state order and within a source in the order they are written (`position_in_source`), so a
   state's range is that order, and `check` refuses a range that holds another state's transition. The eventless and
   the named selections stay separate passes: the eventless list is the null member alone, and no named list holds
   it.

## Order of the changes

1. The runtime: the three tables, `DocumentTables::first_enabled` and its check, `Behaviour::guard_holds`,
   `StatePolicy::guard_holds`, and `selection: None` in every hand-written table; a differential test that runs the
   rows against a hand-written `first_enabled_transition` on random documents
   (`what_enables_a_transition_is_a_row_of_its_tables`).
2. `sce-build`: the rows in `rust_tables`; the template writes `selection` and `guard_holds` in place of
   `first_enabled_transition`; a corpus test that each generated document's rows answer what its descriptors match
   (`a_transitions_enabling_events_are_the_rows_its_event_enum_numbers`).
3. The main session regenerates the Rust trees, runs the Rust W3C lane and the footprint gate, and pins the figures.

## Alternatives considered

- **Erase the engine's layout so that its shells are one copy.** This needs offsets or views across hooks that
  borrow the whole engine (ADR 0123 decision 3). The shells are already a few address computations each. Refused as
  costing more than it saves.
- **Mark the shells `#[inline(never)]` or `#[inline]`.** Inlining does not change which copies exist; it only
  changes where they sit.
- **Build the rows from the analyzer's `prefix_matching_events`.** Refused: that list is made before every event is
  in the model, so a row could miss an event the machine numbers and a run-time match would take (decision on the
  open questions, 1).
- **Lower each small machine of ADR 0125's class to static per-state code without the shared `Turn` interpreter.**
  This is not an alternative to this ADR but a second form of the same machine, decided in ADR 0129: a document with
  no `<parallel>`, no `<history>` and no `<invoke>` may be printed as its own step function, chosen per image, the
  two forms tied by a differential test. Measured on the downstream image, which links two machines and not fifteen,
  the shared interpreter costs 816 to 3,200 bytes for each machine there. The rows of this ADR serve the interpreted
  form, which stays the form of every document outside that class and of an image whose interpreter many machines
  share (ADR 0064, ADR 0074).
