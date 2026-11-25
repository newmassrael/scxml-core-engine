# ADR 0127 — What enables a transition is a row of its document's tables, and one selection reads it for every machine

- Status: Proposed (2026-11-25). Its figures are read off a downstream consumer's release image; the implementation is
  its own landing, measured by the footprint gate.
- Amends: ADR 0123 (a machine's behaviour is indexed by the numbers its tables are), carried from where a run ends to
  which transition an event enables.
- Related: ADR 0064 (Appendix D runs once over a document's tables), ADR 0067 (a row is as small as its numbers),
  ADR 0074 (the engine's per-turn state belongs to its shared core), ADR 0125 (what a document fixes of a microstep is
  walked as rows).
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

## Decision (proposed)

### 1. The rows

For a generated machine, `DocumentTables` gains a `selection` of three tables:

- `first_written`: for each state, in state order, the first written transition's number, plus one entry past the
  last state. The written transitions are the first rows, grouped by source in state order (`position_in_source`), so
  a state's own transitions are the range between its entry and the next. Two bytes for each state.
- `triggers`: for each transition, the position in `trigger_events` of the list of events it answers, or `NONE`
  for a transition no selection returns: a compound state's initial transition, a history's default, the document's
  initial transition, or a written transition that answers no event of the machine. Two bytes for each transition.
- `trigger_events`: the lists, each its length followed by its events in ascending order. An eventless transition's
  list is the machine's "no event". Identical lists are held once. Two bytes for each entry.

The generator writes them from `Transition::prefix_matching_events`, the one statement of which document events a
descriptor matches (`event_descriptor::attribute_matches`, §scxml-3.12.1). The C11 and Kotlin backends already
dispatch from that list.

### 2. One selection

`select_transitions` asks `DocumentTables::first_enabled(state, event, machine)`. It walks the state's range, reads
each transition's list, and asks the machine `Behaviour::guard_holds(transition)` only for a transition that answers
the event. A guard is still the only part that is the machine's own. `StatePolicy::guard_holds` is generated as a
`match` over the guarded transitions, and its default holds for a document with no guard. The policy's
`first_enabled_transition` stays for hand-written tables, whose `selection` is `None`. This is the same split as
`GENERAL_WALK` and `ROW_WALK` (ADR 0125 decision 3): each document's tables decide, not the build.

### 3. What it costs and saves

The turn makes the same walk over the configuration. For each state it visits, it reads a range and compares the
event with short lists instead of jumping through a `match`. It makes a virtual call only for a transition that
answers the event, which for a document with no guard reaches a hook that returns `true`. Expected on the consumer's
image: both `first_enabled_transition` copies (493 bytes and their jump tables) become rows of about 2 bytes for each
state, transition and list entry. One `first_enabled` of 100 to 150 bytes is added once for the program. The
footprint gate decides the figures.

## Open questions, to settle in the landing

- **The event set must be final when the lists are made.** `build_prefix_matching` runs in the analyzer, and
  `internal_queue_bound` inserts into `model.events` afterwards. Any event added after the lists were made is a
  member that a wildcard transition matches at run time today but would not match in a row. The landing must either
  build the lists from the event set the template numbers or show that no later insertion can be matched.
- **A guard that only one backend lowers** (`Transition::native_guard`, a test component's snapshot branch) is set
  per backend. Asking the machine's `guard_holds` for every candidate keeps the template the only judge of which
  transitions are guarded, so the rows hold no guard bit.

## Order of the changes

1. The runtime: the three tables, `DocumentTables::first_enabled` and its check, `Behaviour::guard_holds`,
   `StatePolicy::guard_holds`, and `selection: None` in every hand-written table; a differential test that runs the
   rows against a hand-written `first_enabled_transition` on random documents.
2. `sce-build`: the rows in `rust_tables`; the template writes `selection` and `guard_holds` in place of
   `first_enabled_transition`; a corpus test that each generated document's rows answer what its descriptors match.
3. The main session regenerates the Rust trees, runs the Rust W3C lane and the footprint gate, and pins the figures.

## Alternatives considered

- **Erase the engine's layout so that its shells are one copy.** This needs offsets or views across hooks that
  borrow the whole engine (ADR 0123 decision 3). The shells are already a few address computations each. Refused as
  costing more than it saves.
- **Mark the shells `#[inline(never)]` or `#[inline]`.** Inlining does not change which copies exist; it only
  changes where they sit.
