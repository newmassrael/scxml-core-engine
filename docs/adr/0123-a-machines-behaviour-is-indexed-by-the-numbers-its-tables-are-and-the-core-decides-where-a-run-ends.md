# ADR 0123 — A machine's behaviour is indexed by the numbers its tables are, and the core decides where a run ends

- Status: Accepted (2026-11-24), on the owner's instruction: the long-term, textbook structure, cost ignored.
- Amends: ADR 0064 (decision 3, the behaviour the core calls; decision 5, the engine as a core inside a typed
  layer) and ADR 0068 (decision 3, the main loop's hooks).
- Evidence: `backends/rust/probes/nostd-footprint`, `thumbv7em-none-eabihf`, `opt-level = "s"`, at 3c4678882d4.
- Requested by: a downstream bare-metal consumer, which asked SCE to take out of each machine what the Rust runtime
  still compiles once per machine.

## Context

ADR 0064 runs Appendix D once per program over a document's tables, ADR 0068 does the same for the main loop and the
queues, and ADR 0070 and 0075 took the constructor's temporaries and the unused storage out of each machine. What a
second machine still costs was measured on the probe:

| | bytes |
|---|---|
| one more machine, the probe's two machines | 1,080 |
| one more machine, two machines without `<history>` or `<parallel>` | 1,144 |
| of it, the probe's own driver | 194 |
| of it, the shared `Turn::run` taken out of line | about 52 |

The rest is the second machine's own code:

| function | bytes |
|---|---|
| `Engine::new_in_place` | 124 |
| `Engine::drain_inbox`, and `MainLoop::drain_inbox` | 114 + 14 |
| `Engine::run_turn` | 104 |
| `Engine::step` | 38 |
| `EngineHost` as `Behaviour`: `enter_state` | 74 |
| `EngineHost` as `Behaviour`: `first_enabled_transition` | 72 |
| `EngineHost` as `Behaviour`: `store` | 32 |
| `EngineHost` as `Behaviour`: `exit_state` | 18 |
| `EngineHost` as `Behaviour`: `execute_history_default_content` | 12 |
| `EngineHost` as `MainLoop`: `queue`, `bind_queued`, `behaviour` | 52, 20, 14 |

and its tables in `.rodata`.

Most of the `Behaviour` hooks are conversions. The core calls a hook with a `StateId`, an `EventId` or a
`HistoryId`; `EngineHost` turns each into the machine's enum with `P::STATES[id]`, `P::EVENTS[id]` or
`P::HISTORIES[id]`, and calls the generated policy, which matches on the enum. Each conversion is a bounds-checked
load and a panic site, compiled once for every machine, and none of it is the machine's own: the tables are indexed by
those numbers already, and the enum's discriminant is the number. This is against ADR 0064 decision 1, under which a
function is generic over a machine only where it touches a value of the machine's type.

`enter_state` also decided whether the run had ended. Appendix D's enterStates stops the interpreter when it enters a
`<final>` whose parent is the `<scxml>` element. `EngineHost::enter_state` read the state's row in `P::TABLES` to
answer that, after the onentry, in every machine. The procedure is `helpers::microstep`'s, and the answer is in the
tables the procedure already reads.

## Decision

### 1. The behaviour hooks take the numbers the core speaks

`StatePolicy::first_enabled_transition`, `execute_entry_actions`, `execute_exit_actions`,
`execute_history_default_content` and `bind_current_event` take `StateId`, `EventId` and `HistoryId`, not the
machine's `State`, `Event` and `History`. `execute_transition_content` and `take_transition` already took a
`TransitionId`. The engine converts nothing to call them.

This is the table-driven automaton of ADR 0064 decision 1 carried through: one numbering for the structure and the
behaviour. The tables are indexed by a state's number, and so is the code that answers for the state.

- **The generated code matches on named numbers.** The template writes, beside each enum and from the same loop that
  numbers its variants, one associated constant per variant: `<Machine>State::ID_<variant>`,
  `<Machine>History::ID_<variant>` and `<Machine>Event::ID_<variant>`, with `<Machine>Event::ID_Null` for the
  null event. Each is written as `StateId(Self::<variant> as u16)`, from the discriminant, so a state's two names
  cannot disagree. A hook's `match` reads as before, with `ID_` in front of each variant.
- **The constants are named after their variant, not upper-cased.** A variant holds no `_` except where an escape
  puts one at its edge (`Self_`, `_1x`), so `ID_<variant>` is never another variant's name and never another
  constant's. Upper-casing would fold variants that differ only in case (`ABc` and `Abc`) into one name. The
  constants carry `#[allow(non_upper_case_globals)]` for that reason.
- **The enums stay the machine's names.** `get_current_state`, `get_active_states`, `raise`, `terminal_state` and
  the rest of the typed API are unchanged. Conversions remain only where a typed value enters or leaves that API.
- **A hook that needs the machine's own value converts where it needs it.** `StatePolicy` provides `event_of` and
  `history_of` beside `state_of`. The generated code converts in two places: a descriptor matched by name reads the
  event's name, `Self::get_event_name(Self::event_of(event))`, and `bind_current_event` reads the name of the event
  it binds. A machine with neither has no conversion at all.
- **`Engine::note_payload_reading` takes the `EventId`** that `bind_current_event` is handed, since it only records
  the number in `EngineCore`.
- **The typed event of a queued delivery is made only where a step reads it.** `Engine::bind_queued` used to build an
  `EventWithMetadata` with the machine's event for every event taken off a queue. It now keeps the number and
  converts at the payload door (`lift_event_payload`, `std` only), for `<finalize>` and for autoforward. A
  `no_std` machine with neither converts nothing there either.

### 2. The core decides where a run ends

Appendix D's enterStates is `helpers::microstep::enter_states`, written once for every program. It reads from the
tables whether a state it enters is a `<final>` with no parent, and reports it as `Entered::ended_in`.
`EngineCore::settle`, which already takes the current state from `Entered`, records it as the run's terminal state.
`EngineHost::enter_state` runs the machine's onentry and nothing else.

The record is made once the entry set is entered, where it used to be made inside the walk. No legal configuration
enters a state after a top-level `<final>` in the same set, and nothing between the two reads it.

### 3. Each remaining hook, judged against ADR 0064 decision 1

The core would hold the views (the run store's, the two queues') for the length of a turn where the borrows allow it.
They do not, for this reason: a hook that runs the machine's executable content is handed the whole engine,
`&mut Engine<P>` (`Engine::with_policy`), and that content reads and writes the very storage a view would hold.

| hook | what it hands out | which hook needs the same storage | verdict |
|---|---|---|---|
| `Behaviour::store` | `RunStoreMut` over `run_store`, whose sizes are the document's | a guard or content asking `In()` (`Engine::is_in_state`, `get_active_states` for a script's view, `is_state_in_final_state` on entering a nested `<final>`), in `first_enabled_transition`, `enter_state`, `exit_state`, `execute_transition_content` and `execute_history_default_content` | kept |
| `MainLoop::queue` | `EventQueueMut` over the internal or the external queue, whose depths are the document's | a `<raise>` (`Engine::raise`, the internal queue) and a `<send>` to this session (the external queue), in every hook that runs content; the inbox drain (the external queue) | kept |
| `MainLoop::core` | `&mut EngineCore` | `Engine::raise` reads and counts the error chain (`handling_error_event`, the cascade bound); `is_current_event_external` and `note_payload_reading` read and write it | kept |
| `MainLoop::behaviour` | the same host as `&mut dyn Behaviour` | none: it is the coercion to the second trait object | kept |
| `MainLoop::bind_queued` | binds a queued event through the policy | it pops the typed payload, binds it, runs `<finalize>` over a typed `EventWithMetadata` | per machine, by decision 1 |
| `MainLoop::drain_inbox`, `Engine::drain_inbox` | applies what other threads posted | it applies typed inputs (`InboxInput<P::Event, P::Payload>`) through the typed entry points | per machine, by decision 1 |
| `Engine::new_in_place`, `run_turn`, `step` | the machine's own storage and workspace | — | per machine: they build what is sized by the document (ADR 0068 §5, ADR 0070 §3) |

- A view held for the turn beside a `&mut Engine<P>` handed to a hook is two mutable paths to one place. The borrow
  checker refuses it, and making it compile through a raw pointer would make it undefined behaviour instead. The
  borrow that would have to go is the hook's `&mut Engine<P>`, which is the generated code's API for raising,
  sending, scheduling and asking `In()`. Narrowing it to the parts content may touch is a change of that API, larger
  than this ADR, and would itself be decided by what content needs from the queues and the store.
- `MainLoop::behaviour` is a coercion that trait upcasting replaces: with `trait MainLoop: Behaviour`, a
  `&mut dyn MainLoop` coerces to `&mut dyn Behaviour` without a method. That coercion is stable from Rust 1.86, and
  the workspace's `rust-version` is 1.75. Merging the two traits into one was considered and refused: `Behaviour` is
  public and implemented outside the engine (the TTCN-3 export's macrostep, the differential test's documents), and
  it would carry hooks of the main loop that the microstep's procedures never call. When the floor reaches 1.86, the
  supertrait replaces the method.
- `bind_queued` and `drain_inbox` touch the machine's typed payloads and inputs, so they are per machine under
  decision 1 itself. A numbered form for them is the queues' change measured and reverted in ADR 0064's amendment:
  numbering one side while the code on both sides still speaks the machine's types costs more than it shares
  (156 and 140 bytes). Decision 1 applies inside `bind_queued` instead: the event stays a number until a step reads
  it.

### 4. Acceptance

This ADR is accepted on all of these:

- `footprint.budget`'s `MACHINE_BYTES`, `MACHINE_BYTES_NO_DIAGNOSTICS` and `MACHINE_BYTES_PLAIN` fall, and are
  set to what the probe measures;
- the stack gate holds (`STACK_BYTES`, and the plain configuration's bound);
- the Rust lane passes the W3C corpus;
- the consumer measures its own image at the landing.

## Measured

Measured on one build machine, `thumbv7em-none-eabihf`, `opt-level = "s"`, rustc 1.100.0, with the gate's own
arithmetic (`.text` plus `.rodata` of the probe staticlib, the image of two machines minus the image of one; the
core by `CORE_SYMBOLS`). The parent's figures are the ones CI read for 3c4678882d4, byte for byte. (The Context's
1,080 and 1,144 are a symbol-by-symbol difference of the same images, which counts what moved between symbols
differently; this table is the gate's.)

| | parent | this change |
|---|---|---|
| one more machine, with the macrostep report | 1,118 | 1,008 |
| one more machine, without it | 1,110 | 1,000 |
| one more machine, two plain documents | 1,172 | 1,076 |
| shared core, with the report / without / plain | 7,878 / 7,770 / 5,504 | 8,004 / 7,896 / 5,630 |
| image of two machines, with the report / without / plain | 211,171 / 210,985 / 208,198 | 211,019 / 210,833 / 208,122 |

Each machine is 96 to 110 bytes smaller and the program pays 126 bytes once, so an image of two machines is smaller
by 152 bytes (76 with plain documents), and each further machine saves its 96 to 110. The image of one plain machine
is 20 bytes larger. The worst-case stack is read by the stack gate at the landing.

## Consequences

- A hand-written `StatePolicy` fails to compile once: its five hooks take numbers.
  `docs/SCE_CODEC_CONSUMER_MIGRATION.md` says what changes in one, line by line. A generated machine is
  regenerated.
- A machine's behaviour no longer converts a number to call a hook, and no hook of `EngineHost` decides anything
  about the run. What remains per machine is what decision 3 names, each with its reason.
- The generated enums gain one associated constant per variant. They are public, as the enums are, and a host may
  match on them where it holds a number.
- `Entered` gains a field. It is built only by `helpers::microstep`, and a caller that reads it is unchanged.

## Alternatives considered

- **Keep typed hooks and convert without a check** (`get_unchecked`, or a transmute from the `u16`). The tables'
  `check` holds every number in range, but the compiler cannot see that, so the conversion would be `unsafe` to save
  what a single numbering removes for free. Refused.
- **Upper-case constants, or constants in a module per machine.** Upper-casing folds variants that differ only in case
  into one name, which a document may write. A module is equivalent to associated constants and puts the names
  apart from the type they number. Refused.
- **Keep the end-of-run check in `EngineHost`, over numbers.** It would still be one copy per machine, of a
  question Appendix D's enterStates answers and `helpers::microstep` already holds the tables for. Refused.

## Order of the changes

1. The runtime's hooks take numbers, the templates write the constants and match on them, and every hand-written
   policy in the runtime's tests follows; the migration note in the same commit.
2. The end of a run is the core's (`Entered::ended_in`, `EngineCore::settle`), with its tests.
3. `bind_queued` converts only where a step reads the typed event.
4. ARCHITECTURE.md's "Rust AOT Backend".
5. The main session regenerates the committed Rust trees, measures the probe and sets the budget, runs the stack gate
   and the Rust lane, and sends the landing to the consumer.
