# ADR 0124 — The data of the event being processed is read through one value, whether a queue owns it or a call lends it

- Status: Accepted (2026-11-25). The structure (D1 to D6) is the owner's decision of that day, under the rule "the
  most long-term correct, textbook structure, cost ignored". The Rust runtime and the Rust generator carry it in this
  landing; the other backends are planned in D4, each a landing of its own. The landing is accepted on D6.
- Date: 2026-11-25
- Scope: `backends/rust/runtime` (`policy.rs`, `engine.rs`), `tools/codegen/templates/rust`, the Rust payload
  channel of `sce-build` (`forge/generator.rs`, `forge/native_action.rs`, `forge/static_lowering.rs`), every
  hand-written `StatePolicy`.
- Replaces: the proposal "ADR 0121, an event may lend its context to the guards and actions that run while it is
  `_event`" (branch `event-context`, never landed). Its problem statement and its window rule are kept; its structure
  is not (Alternatives).
- Related: ADR 0049 (a decoded codec is a view over its wire bytes), ADR 0058 (a codec's source trait and parts,
  implemented by every representation), ADR 0064 and ADR 0068 (Appendix D and the main loop once per program), ADR 0086
  (a typed payload rides with its event, and its door), ADR 0111 (bound events cross a boundary typed), ADR 0123 (the
  hooks take numbers), ADR 0053 and ADR 0064 decision 6 (one dialect of generated code), ADR 0044 and ADR 0045 (the
  saved state).

## Context

A downstream consumer runs a protocol session as a generated Rust machine. Its host decodes each received message at
the boundary into a view over the received bytes (ADR 0049) and delivers the matching event. What the machine's guards,
`<sce:arg>` expressions and acts read of the message was a copy:

- `StatePolicy::Payload` is `'static + Clone` (ADR 0086), so it cannot hold a view, only an owned copy of one.
- The engine copied the payload of each event it took into the policy (`populate_event_payload`, a
  `pending_payload` field), and every reader read it there.

So the host copied the facts into its own state before it delivered the event, and its acts read them back. The only
alternative open to it was to erase the view's lifetime with `unsafe`.

§scxml-5.10 already says what a reader should see: `_event.data` is the data of the event being processed, bound when
the event is taken off a queue and unchanged until the next event is taken. What was missing is a way for that data to
be the host's borrowed value for the span the specification gives it, read through the same interface every reader
already uses for the owned payload.

## Decision

### D1. `_event.data` is read through one value

Every reader of the data of the event being processed — a guard, an `<sce:arg>` expression, a transition's content —
reads it through one value, the machine's `StatePolicy::EventData<'e>`. For an event that carries data it is a borrow of
that data, read through its schema's read interface; for an event that carries none, nothing. Whether the data came
owned through a queue or was lent by a synchronous call, every reader reads it the same way.

The read interface is what ADR 0058 makes of a codec, applied to an event schema: the presentation of the value's fields,
which every representation of the value produces. A generated Rust machine has:

- `<Machine>EventData<'e>`: `None`, and one variant per event that carries a typed payload. It takes `'e` exactly when
  a variant borrows. It is `Copy`.
- For an event whose schema is a schema of entries, `<Machine><Event>Ref<'e>`: one public member per field, a text as
  `&'e str`, a byte string as `&'e [u8]`, an optional one as the optional of that, every other value (a number, a truth
  value, an enum) by copy. It is the schema's parts in the sense of ADR 0058 §4: the owned payload reads itself as one
  (`<Machine><Event>Payload::data()`), and a host that holds a codec view of the message fills one from the view's
  accessors, which reads each field once and copies no byte.
- For an event whose schema holds a composition (a list, a record, a map, docs/adr/0086), the variant is a borrow of
  its payload record, `&'e <Machine><Event>Payload`. A list has no borrowed presentation that is one type: ADR 0058
  presents it through `impl Iterator`, a type per implementor, and a value of one type for every representation is
  what D3 needs. So a host lends such a payload as the record it holds; a later record may give the walks of a
  composition a borrowed form, and the variant changes then, with no hook changing.
- `<Machine>Payload::data()` reads an owned payload as `<Machine>EventData`.

This extends what ADR 0086 and ADR 0111 generate (the payload struct per event, the payload sum) and adds no parallel
type system: the parts are read off the payload struct, and the payload sum is still what a queue holds.

### D2. A queued event owns its data, and a call lends it only for the window of its event

- A queued event owns its data: an asynchronous boundary must own what it carries. The queues hold the owned payload
  beside each event as before (ADR 0068).
- `Engine::process_event_with(event, event_data)` processes `event` now with its data lent for that call. It counts the
  external events queued ahead of `event` (after applying what other threads posted, which goes ahead of it as
  `raise_external` puts it), queues `event`, and runs the turn. The data is lent from the moment the engine takes
  `event` off the external queue to the moment it takes the next event, internal or external (§scxml-5.10):
  - the guards that select `event`'s transitions, the exits, the transitions' content and the entries of its
    microstep, and the eventless microsteps after it, are handed the lent data;
  - an external event queued ahead of `event`, an internal event raised while it is processed, and the entry into the
    initial configuration are handed their own data, as every queued event is;
  - an event refused where it is taken opens nothing: `_event` stays what it was, and a lend for it is spent;
  - lent data meets the door a wire delivery meets (docs/adr/0086, section 4.2): data that enters a machine has one
    contract whichever route it takes, or D1's readers could tell the two forms apart. `StatePolicy::admit_event_data`
    holds it to what `lift_event_payload` holds the same data to from the wire: each text and byte string of a schema
    of entries to its bound, refused in the lift's words, code, pointer and bound
    (`event_payload::lent_within`); every other field is held by its type in the parts. A refusal follows the queued
    path exactly: `error.execution` carrying it is taken in the event's place where the document declares one, and
    otherwise the delivery is dropped and counted as a discarded external event. A `no_std` build has no wire and no
    door that refuses, on either route; there lent data past its bound is taken as a delivery that carries no data, as
    such a build takes any delivery whose typed payload is not there. A record lent for a schema that holds a
    composition is held by the types it is made of;
  - when the call's budget of external events (ARCHITECTURE.md "External-Event Budget") ends the turn before `event`
    is taken, `event` stays queued and is taken later as a delivery that carries no data, refused at its door as any
    such delivery is (docs/adr/0086, section 4.2).
- The owned payload of a queued event is handed to the readers as the same `EventData`, borrowed from where the engine
  holds it while the event is `_event` (D3).
- A generated machine pairs each event with its data in one call, `<Machine>Inject::process_<event>_with`, beside
  `raise_<event>`, so no event can be lent another's data. On a `std` build whose document has a reader that keeps
  its own copy of `_event.data` from the wire, a script engine or an autoforwarded child, that entry also writes the
  data's JSON as `raise_<event>` does (`Engine::process_event_with_wire`), so those readers read what the typed
  readers read.

### D3. The data reaches every hook as an argument, and is never stored in the policy

`StatePolicy` gains two items, and the hooks that run a guard or executable content one parameter:

- `type EventData<'e>: Copy`, which a policy whose events carry no data spells `()`;
- `fn event_data<'a>(held: &'a CurrentEventData<'_, Self>) -> Self::EventData<'a>`, the policy's reading of what the
  engine holds: `CurrentEventData::Queued(payload)`, a payload a queue owned, or `CurrentEventData::Lent(data)`, data
  lent for one call. The generated reading is `payload.data()` and `*data`; the one of a policy with no data is `{}`.
  It is one item rather than two (a reading of a payload and a shortening of a lent value) because a lent value is a
  `Self::EventData<'d>`, a projection the compiler holds invariant in `'d`, and only the policy's concrete type
  knows it may be read for a shorter `'a`;
- `first_enabled_transition`, `execute_transition_content`, `execute_entry_actions`, `execute_exit_actions` and
  `execute_history_default_content` take `event_data: Self::EventData<'_>`.

The engine holds the data in the per-turn `EngineHost<'e, 'd, P>`, in a field disjoint from its `&'e mut Engine<P>`:
`current`, moved in when an event is taken (`MainLoop::bind_queued`) and kept until the next one is, and `lend`, what
`process_event_with` lends until its event is taken. It cannot be held in the engine: a hook is handed
`&mut Engine<P>`, so data borrowed out of the engine could not be lent to the same call, and holding it in the policy
would need a lifetime on the policy or `unsafe` to erase one. Held beside the engine, each hook call borrows `current`
and the engine apart, and the borrow checker proves that nothing a hook keeps outlives the window. No `unsafe` is added.

`populate_event_payload` and the policy's `pending_payload` are gone; `lift_event_payload` fills the payload the
engine holds (`payload: &mut Self::Payload`) rather than a field of the policy, and `admit_event_data`, provided
(admitting everything), is the door of lent data (D2), which the generator writes for a schema whose texts or byte
strings have a bound. One hook signature serves every
machine: there is no second trait, no second set of hooks, no build option and no type parameter on `EngineHost` or
`run_turn`. `Turn` and `helpers::microstep` stay non-generic and never see the data.

### D4. The document stays portable, and every backend gives it the same meaning

What an act reads is written in the document: an act that needs the whole message is handed it by
`<sce:arg expr="_event.data">` (the schema's read interface, as the parts are), one that needs a field by
`<sce:arg expr="_event.data.<field>">`. Nothing outside the document says what an act reads, and no option of a build
changes what a document means.

Every backend gives D1 to D3 the same meaning and lends with its own idiom, each a landing of its own with a driver of
its lane:

- **C11**: `<m>_process_<event>_with(sm, const <m>_<event>_payload_t *)`, the pointer held in the turn and handed to
  each hook, the readers reading the schema struct through it; outside the window the slot is the owned payload of the
  event taken.
- **C++**: `processEventWith(Event, const <Event>Payload&)` on the generated machine, the reference held in the turn.
- **Go, Kotlin, Python, Swift**: a reference to the schema's value, handed to each hook as an argument and dropped when
  the call returns; the window rule of D2 is the same.
- **The Interpreter**: `processEventWith` binds the data the call lends as `_event.data` for the window, read by native
  actions through its registry and by the data model as any event's data.

### D5. Nothing reads `_event.data` after its event's microstep, so a window closed at the call's return loses nothing

§scxml-5.10 binds `_event` until the next event is taken, which may be in a later host call, while a borrow ends when
`process_event_with` returns. The question is whether anything reads the typed data between the two. It does not, in
the code as it stands:

1. **Every reader of the typed data runs inside its own event's microstep.**
   - A native typed guard is written only for a transition whose event names an event schema
     (`forge::generator::select_native_typed_guards`), and `first_enabled_transition` asks it only when the event
     being selected is that event (`event_match` in `rust/process_transition.rs.jinja2`). An eventless selection
     passes the null event, which matches only transitions with no event, and those have no schema.
   - A `sce-static` document reads `_event.data` only in the guard and the content of a transition on such an event:
     `static_lowering::lower` renames `_event.data` to the payload for a transition alone, and lowers `<onentry>`,
     `<onexit>`, initial content, `<donedata>` and `<invoke>` with the renames that do not name it.
   - An argument-bearing `<sce:action>` is refused outside a `<transition>` (`native_action::check_placement`):
     `<onentry>`, `<onexit>` and initial content run with no triggering event.
   So a reader runs between `Turn::take_event`'s selection of the event and the end of its `take_microstep`, both
   inside the turn that took the event.
2. **What runs after a macrostep completes and before the next event is taken reads none of it.**
   - Eventless transitions are exhausted at macrostep completion, and have no typed reader (item 1).
   - `execute_pending_invokes` evaluates an `<invoke>`'s parameters with no typed `_event.data` (item 1); a script
     engine evaluates its own copy (item 3).
   - `tick` takes delayed events off the scheduler, each a new event bound when it is taken; `tick_children` drives
     other sessions.
   - `<finalize>` is handed the child's event, payload included, before that event is bound
     (`execute_finalize_for_child_event`): its own data, not the window's.
   - The saved state (ADR 0044, ADR 0045) holds the configuration, the data model, the delayed sends and the queued
     external events with their data, and no `_event`.
   - The diagnostics name events by number (`last_discarded_event`, `last_unhandled_error`,
     `last_undecodable_payload`), and the one host accessor of the current event, `is_current_event_external`, is a
     flag. No accessor of its data exists.
3. **A reader that does read `_event` across calls holds its own copy.** A script engine binds `_event` from the wire
   in `bind_current_event` and keeps it until the next binding; D2 writes the wire for a lent event of a machine that
   has one.

So the textbook answer is the first one the question allows: no reader can observe the typed data outside its event's
microstep, and closing the window when the call returns loses nothing. The same holds for a queued event's owned
payload, which the turn drops when it returns: a turn starts with no data held (`EngineHost::new`), and no reader runs
before the event it reads is taken. If a later construct reads the typed data outside its event's microstep (a typed
`<invoke>` parameter, say), it is that construct's record to decide whether it is handed an owned copy (read through
the parts into the payload struct) or the value is defined as no longer readable; this ADR's window does not change.

### D6. Measure-ready

The footprint and stack gates hold the landing (`scripts/gates/nostd-mcu.sh`, `footprint.budget`). What it adds to a
machine is the turn's two fields, one comparison where an external event is taken, and a reading of the data at each
hook call, which is nothing for a machine whose `EventData` is `()`.

## Alternatives

- **Two forms of `_event.data`, a typed payload and an opaque host context (the proposal's D5).** Two sources of truth
  for one datum, which a host could fill differently; and a guard could not read the context, since the generator
  lowers no guard over a type it cannot see. Refused: D1 makes one value the only form.
- **An opaque host-chosen type the document cannot describe (the proposal's `Context<'e>`).** Generated code could only
  pass it through, so no guard or argument of the document could read it, and each other backend needed a plan unrelated
  to the document. Refused: the lent value is the schema's read interface, which the document names and every backend
  derives.
- **A build option that changes generated signatures (the proposal's `--event-context`).** Two dialects of generated
  code from one document, against ADR 0064 decision 6 and ADR 0053: a library compiled against one dialect does not link
  the other, and every test would have to run twice. Refused: every machine has the signature of D3.
- **A second trait with a parallel set of hooks (the proposal's `EventContextPolicy` and its `_in` hooks).** Two hooks
  per behaviour, kept in step by delegation, and a policy that implements the wrong one compiles. Refused: `StatePolicy`
  gains what it needs, and a policy with no data spells `()`.
- **`EngineHost` and `run_turn` generic over a hook carrier (the proposal's `H: Hooks<P>`).** A second instance of the
  turn and of every `Behaviour` hook per machine that lends, against the direction of ADR 0064, ADR 0068 and ADR 0123,
  which take per-machine code out. Refused: the lend is a value of the turn, not a type parameter.
- **A borrowed variant of `Payload`.** `Payload` would need a lifetime, and with it `Engine`, the queues, the inbox and
  the saved state; a queued event outlives the call that queued it, so it could not be queued at all. Refused by D2:
  the queue owns, the call lends.
- **The data in the policy, or in the engine.** The policy outlives every event, so it would need a lifetime or
  `unsafe`; the engine is lent mutably to every hook, so data it held could not be lent to the same call. Refused by D3.
- **A trait object per schema as the read interface (`&'e dyn <Event>Data`).** One type, but a dynamic call per read and
  a vtable per representation, and a list through a trait object needs an internal iteration every lowering would have
  to learn. Refused for schemas of entries, whose parts are one `Copy` type and read each field once; a composition is
  read through its record (D1).

## Consequences

- A host lends the message it received for the call that delivers its event: no fact is copied into its own state to
  be read back, no byte string is copied once more, and no `unsafe` is written on either side.
- A hand-written `StatePolicy` fails to compile once: two new items and one parameter on five hooks.
  `docs/SCE_CODEC_CONSUMER_MIGRATION.md` says what changes. A generated machine is regenerated.
- The content of a transition binds `ev` as a borrow of the data its hook is handed, an argument and no field of the
  machine, where it bound an owned clone of a field: the copy per transition is gone.
- A test component's branch still holds the message it took from the round's snapshot in the machine, now
  `taken_message`: that message is no event's data, it is what the branch took (docs/SCE_TEST_PROFILE.md §12.3).
- A payload handed whole to an imported algorithm (`sce-static`, a call's adapter) is read through its parts, and the
  adapter takes them by reference: a record handed to an algorithm holds numbers, truth values, bounded strings and
  enums alone, which the check of the call holds on every target (a record that holds a byte string is refused there,
  before and after this record), so each field crosses as the value it is.
- A lent event the budget does not reach is delivered later without its data. A host that lends after queueing more
  external events than one call takes should not; one that does sees the door refuse the delivery, as it refuses any
  delivery of a typed event without its data.

## Verification

- `backends/rust/runtime/tests/an_event_lends_its_data_while_it_is_processed.rs`: a hand-written policy whose guard,
  exits, content and entries record the data they are handed. The lent event's microstep sees the bytes the host lent,
  from a buffer dropped right after the call; the same event queued with an owned payload is seen the same way; the
  initial entry, the internal event the content raised and an event with no data see none; an event queued ahead is
  handed its own payload; a lent event the budget does not reach is taken later with none; a machine not running
  refuses the call and counts it.
- `backends/rust/tests/tests/event_schema_native.rs`: a generated guard over a byte string reads a lent view as it
  reads a queued payload, on a match and a miss.
- `backends/rust/tests/tests/native_action.rs`: a generated act's arguments are the same whether its event's data was
  lent or queued.
- `backends/rust/tests/tests/lent_payload.rs`, over `sce-build/tests/fixtures/lent_payload`: a text past its bound is
  refused whole on both routes, in the same words, and the document answers the `error.execution` alike; at its
  bound it is taken on both; a byte string past its bound is refused on both routes alike, and within it is read alike;
  a payload is handed whole to an imported algorithm from the queue and lent. The runtime's window test
  also holds a lent value its door refuses in a document with no `error.execution`: dropped and counted.
- `sce-build/tests/every_rust_machine_is_handed_its_event_data_through_one_hook_signature.rs`: a machine with no
  typed data, one whose guard reads it and one whose act reads it have the same five hook signatures and the policy's
  reading, and none names a second trait, a stored payload or a populate hook.

## Measured

Before, from `backends/rust/probes/nostd-footprint/footprint.budget` at the parent, `thumbv7em-none-eabihf`,
`opt-level = "s"`, rustc 1.100.0:

| | before | after |
|---|---|---|
| one more machine, with the macrostep report (`MACHINE_BYTES`) | 862 | to be measured |
| one more machine, without it (`MACHINE_BYTES_NO_DIAGNOSTICS`) | 854 | to be measured |
| one more machine, two plain documents (`MACHINE_BYTES_PLAIN`) | 926 | to be measured |
| shared core, with the report / without / plain | 7,890 / 7,782 / 5,584 | to be measured |
| worst-case stack / plain / a root's own frame | 952 / 760 / 64 | to be measured |

The probe's machines carry no typed payload, so their `EventData` is `()`: what they pay is the turn's `lend` and
`current` and the count where an external event is taken.

## Order of the changes

1. This record.
2. The runtime: `StatePolicy::EventData`, `event_data` and the hooks' parameter, `CurrentEventData`, `EngineHost`'s
   `current` and `lend`, `Engine::process_event_with`, the lift into the held payload, every hand-written policy in
   the runtime's tests, the window's test, and the migration note.
3. The generator: `<Machine>EventData`, the parts, `process_<event>_with`, the guards, `<sce:arg>` and content
   reading the hook's argument, the template's signatures, and the generated tests.
4. ARCHITECTURE.md's "Rust AOT Backend".
5. The main session regenerates the committed Rust trees, measures the probe and sets the budget, runs the stack gate
   and the Rust lane, and sends the landing to the consumer.
