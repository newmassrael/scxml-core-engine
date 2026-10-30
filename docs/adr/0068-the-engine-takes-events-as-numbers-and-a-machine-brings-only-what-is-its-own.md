# ADR 0068 — The engine takes events as numbers, and a machine brings only what is its own

- Status: Accepted (2026-10-30), on the owner's instruction to optimise the footprint structurally
  until no optimisation point is left.
- Amends: ADR 0064 (decision 5, the main loop over `MainLoop`).
- Evidence: the watching-zenoh documents measured after ADR 0067, and
  `backends/rust/probes/nostd-footprint`.

## Context

After ADR 0064 a machine's own code holds no Appendix D, and the main loop is shared. After ADRs
0066 and 0067, the consumer's two documents are still 753 bytes over 49e40c9a72. What each
machine still carries is the engine around the loop. It is compiled once per machine because
it names the machine's event type. On the consumer's session document (`thumbv7em`,
`opt-level = "s"`):

| hook | bytes | what it does |
|---|---|---|
| `take_internal_event` | 260 | pop a typed event, bind it, compare its name with `error.` |
| `end_run` | 164 | count unseen external events, exit the interpreter |
| `process_next_external_event` | 82 | pop a typed event, reset the macrostep budget, bind |
| `end_internal_event`, `end_external_event` | 118 | count a discard, name the event |
| `has_*`, `null_event`, `state` | 70 | read the engine's fields |

None of it is the document's. Each line converts the machine's type to a number, or reads a
field. It is per machine because the queues hold `EventWithMetadata<P::Event, P::Payload>` at
the machine's depth, so every function that touches a queue names `P`.

## Decision

### 1. A queue holds numbers

An entry is `QueuedEvent { event: EventId, metadata }`, 4 bytes under `no_std`. The storage is
`EventQueue<Payload, N>` in the machine's policy, at the depth it has today. The engine drives
it through `EventQueueMut<'_>`, which has neither `N` nor a machine type. That is the pattern
of `RunStore` and `RunStoreMut`.
- Under `no_std` the view is a ring over the storage slice.
- Under `std` it borrows a `VecDeque<QueuedEvent>`.

A typed payload rides in a second deque of the same depth, which only the machine's own code
touches. A machine whose payload is `()` stores none: every push and pop of a payload is behind
`size_of::<P::Payload>() != 0`, which the compiler settles.

The typed entry points (`raise`, `raise_external*`, `process_event*`) keep their signatures. They
are the API, and a host links only the ones it calls.

### 2. What the loop reads and writes is a core without a machine type

`EngineCore` holds:
- `LoopState`;
- the flags the loop sets as it takes events;
- the counts a host reads, each event they name as its `EventId`.

The typed accessors (`last_discarded_event` and the rest) turn the number back into the
machine's event, as `get_current_state` already does.

### 3. The main loop's hooks are what only a machine can do

`MainLoop` keeps:
- handing out its queues and its `EngineCore`;
- binding `_event` through its policy, with the typed payload queued with it;
- under `std`, refusing a host-run invocation's late reply;
- ending an event's binding;
- dropping its queued payloads when a run ends;
- starting its invocations.

Taking an event off a queue, resetting the macrostep budget, counting a discard, an unhandled
error or an unseen event, the truncated-chain and truncated-macrostep reports, pointing the
current state and the end of a run are `Turn`'s or `EngineCore`'s. Each is written once: the
two doors that record an unseen event, and the two places that cut an event chain, call one
`EngineCore` method each.

### 4. Which events are errors, and which is the null event, is table data

`DocumentTables` gains `null_event` and `error_events`, the ascending `EventId`s whose names
begin with `error.`. The template writes them from the same loop that numbers the event enum.
The internal drain and `Engine::raise` both compare numbers, and the runtime has no name test
for `error.` at all. The prefix rule lives in the template alone, and the template test holds
it (`rust_template_names_the_error_events_by_their_prefix`).

### 5. One turn enters the initial configuration and runs the loop

`initialize` runs the same `Turn` as `step`, with the entry into the initial configuration
first. The microstep workspace is built once for the turn, not once for the entry and once for
the loop, and the entry is the core's (`Turn::enter_initial_configuration`), not a generic
method of each machine.

The function that builds it, `Engine::run_turn`, is never inlined. At ADR 0064 both workspaces
were locals of functions that inline into `initialize` and `step`, and from there into the
consumer's function that calls them. The consumer reported that function's frame 4,256 bytes
larger (`spawn`, which initialises) and its stepping function's 1,320 bytes larger. A
workspace is the turn's (ADR 0056 §4), so it belongs in a frame that lives exactly as long as
the turn.

### 6. The current state is a number, and the default state is table data

`EngineCore` holds the current state as a `StateId`, and the truncated-macrostep count and the
state it stopped in beside it. `DocumentTables` gains `default_state`, the state the engine
names before it initialises, which the template writes as the variant's number from the same
computation as the state type's `Default`. So:
- the core is built in a constant expression from the tables alone
  (`EngineCore::new(P::TABLES.default_state)`), and `ConstStatePolicy` loses `INITIAL_STATE`;
- `StatePolicy::initial_state` is a provided method reading the tables, so no machine compiles
  one, and the two can no longer disagree;
- `get_current_state` and `last_truncated_macrostep_state` turn the number back into the
  machine's state, as the event accessors do.

The count of payloads the datamodel could not read follows decision 2. It moves into
`EngineCore` with its event as an `EventId`, and the typed `note_payload_reading` only converts.

### 7. The entry set names where the current state is read from

Computing the entry set already dereferences each transition's targets. `EntrySet::last_target`
keeps the first effective target of the last transition that has one. The current state is read
down from it once the set is entered. Before, `enter_states` dereferenced the same targets a
second time. What a `<history>` recorded does not change while a set is entered, so both passes
give the same answer.

## Measured (2026-10-30)

| | `.text` | `.rodata` | against 49e40c9a72 |
|---|---|---|---|
| the consumer's two documents after ADR 0067 | 185,218 | 7,715 | +753 |
| after decisions 1–4 | 184,748 | 7,365 | −67 |
| after decisions 1–7, on a second harness | 184,614 | 7,276 | −525 |

The second harness was built again for the last row, and it measures 49e40c9a72 at 185,302 and
7,113. Each row is compared with 49e40c9a72 on its own harness. The consumer's two documents
are below the tree before Appendix D ran over tables. Neither document has a delayed send, so
the scheduler, which is still per machine, does not enter these figures.

The probe's two machines without either feature, with the probe driving only `step` and
`process_event`:

| | image | one more machine |
|---|---|---|
| after ADR 0067 | 192,040 | 1,594 |
| after decisions 1–4 | 191,812 | 1,244 |
| after decision 5 | 191,574 | |
| after decisions 6 and 7 | 191,476 | |

The probe now drives `tick` too, and `footprint.budget` holds the figures with it. The worst-case
stack is 1,392 → 1,120 bytes with the features and 1,128 → 928 without. `initialize` takes 16
bytes of its own frame and `step` 24.

Decision 1 alone, measured before decision 3, made the probe 218 bytes larger: a shared ring
with every machine still converting to and from it. It pays only with the hooks it served gone.

## Consequences

- What a machine brings is its tables, its guards and actions (`first_enabled_transition`,
  `enter_state`), its constructor, `initialize` and `step`, and the workspace it lends the loop.
- The bounded ring is tested on the host (`event_queue::tests`), where the `std` queue is the
  one the engine uses, so a wrap-around fault cannot reach an MCU unseen.
- The remaining per-machine code is the constructor, the entry points, and `run_turn`, which
  builds the machine's sized workspace.
- The delayed-send scheduler is still per machine. `PullScheduler<P::Event, P::ScheduledSendId>`
  is a public type, so turning it to numbers is a decision of its own. In the probe, `tick` is
  about 650 bytes per machine that has a delayed send.
- A root's own frame is held to `STACK_ROOT_FRAME_BYTES` (`--root-frame-max` of the stack
  tool). A workspace that inlines back into `initialize` or `step` fails the gate rather than
  reaching a consumer's frame.
