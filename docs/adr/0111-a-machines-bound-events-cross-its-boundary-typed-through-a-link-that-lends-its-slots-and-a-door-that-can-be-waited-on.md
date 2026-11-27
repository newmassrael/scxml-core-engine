# ADR 0111 — A machine's bound events cross its boundary typed, through a link that lends its slots and a door that can be waited on

- Status: Accepted (2026-11-22). Revised twice the same day on the consumer's review (one voice of two of its
  sessions), whose every request is taken below; the consumer has no objection to the second revision. Written under the owner's standing rule (the most long-term correct structure, textbook in its
  foundations, cost ignored; decided, not asked). It is item 4 of the order agreed with a downstream consumer on
  2026-11-19, which the owner confirmed: the boundary binding and the lend contract, before ADR 0100's landing.
  Amended 2026-11-28 on two later requests of the consumer, under the same rule: a machine asks for a link through a
  host-run invocation and receives its handle as the completion's data (D1), and a host ends what it holds by an
  explicit input, a value destroyed without one being counted as a violation of the contract (D3); the order of work
  places them (items 3 and 3a). The steps they join had not started. The amendment was revised the same day on the
  consumer's review of it; the Review table at the end says where each of its twelve points went.
- Date: 2026-11-22
- Amends, once landed (each amended document gains an "Amended by: ADR 0111" line in the landing that moves it):
  - ADR 0084 D1, by an owner that is whoever holds the drive (D5);
  - ADR 0084 D2, by an input that is posted and waited for, and by a reply the machine sends later (D5);
  - ADR 0106 #142, by a link a machine imports, link instances at run time, batches and the lending of slots (D1 to
    D3);
  - ADR 0110 D2 item 2, by the JSON reader and writer leaving the C11 core for a separate unit (D2);
  - RFC §synth-5-E's buffer-pool, by a run-time slot count and slot length, storage the caller holds, a headroom, a
    slot index that crosses an interface, a shared book, and a slot lent to an event (D4);
  - the `queue` kind, by a run-time capacity under its bound (D4);
  - ADR 0084 D4 and D6, by one more reserved type a host serves through a registration of its own, `sce:link-dial`
    (D1, amended 2026-11-28);
  - ADR 0045 and the saved state of `docs/SCE_ACCEPTED_SUBSET.md`, by a link table saved as its generations and
    restored closed (D1, amended 2026-11-28);
  - ADR 0115 D7's link connector (`LinkChannel.tla`), by the dial a machine asks for, the lowered frame limit and the
    host's explicit end, and a module of the door's host handles beside it (D1 and D3, amended 2026-11-28);
  - `docs/SCE_ACCEPTED_SUBSET.md`, the paragraphs of each of the above.
- Scope: the Rust lane (`std` and `no_std`) and the C11 lane, which are the two that generate a link and a
  buffer-pool (the kinds are MCU-class elsewhere); the door's ask and drive (D5) on every runtime; the build entry of
  another repository (D6).
- Related: ADR 0079 (a device core does no I/O), ADR 0082 (an algorithm writes into storage its caller holds), ADR 0086
  (a bound is part of a type), ADR 0090 (an event's origin is the reply address its processor writes), ADR 0100 §1.4
  (a codec's flag inputs), ADR 0102 §1.1 (a codec document is a message type whose value is its record view), ADR 0108
  D3/D4 (a run-time limit under a build bound; an empty map is all zero), ADR 0110 (the C11 core is freestanding).
  Added with the amendment of 2026-11-28: ADR 0045 (a saved state holds its running host invocations), ADR 0098 (a
  parent evaluates what an invoke hands), ADR 0099 (no type has a zero), ADR 0115 (the levels of a formal check).

## Context

A consumer is moving a protocol's whole logic into SCE documents, generated for Rust and for C11 from one source set,
and wants the core to be sans-I/O: a deterministic set of machines that performs no I/O, reads no clock of its own,
and sees only typed events. Bytes are decoded at the boundary before a machine sees them, and a machine's output is
encoded on the way out. Concurrency lives only in the seam, through ADR 0084's inbox and door.

What exists, checked at `7f52c3d78ee`:

- **A link already decodes and encodes.** ADR 0106 #142 generates, for Rust and C11, a link's receive path (`pump`:
  poll the driver, decode each frame with the framer, call `inject` for every `<sce:inbound>` row whose `when`
  holds) and its transmit path (`send`: encode an `<sce:outbound>` event's value into a transmit-pool slot or a
  buffer, hand the bytes to `Link::tx`, return the slot).
- **Nothing couples it to a machine.** No statechart imports a link or a codec (ADR 0110 says so). `inject` hands
  the host a value borrowed for the call; the host must convert it and raise it itself. A machine's `<send>` of an
  outbound event has no processor that reaches the link. A link is one value of the generated type; a protocol runs
  several links at once, opened and closed at run time.
- **The C11 core carries event data as JSON text.** A `<send>`'s params, done data and a child's events are written
  as text and read back (`wire.h`, `event_payload.h`); only the per-event `_raise_<event>_typed` entry fills a
  typed payload directly. ADR 0110 left making them typed to this record.
- **The transmit path copies one message per slot.** `Link::tx(TxFrame<&[u8]>)` copies the bytes before it returns,
  so the slot is returned at once. A link that sends from the slot itself, as a DMA engine or a zero-copy stack does,
  cannot be generated, and neither can a batch: the consumer's protocol (zenoh) writes one frame header with one
  sequence number and then appends network messages to the same slot until it is full or flushed, and a message that
  does not fit gives its sequence number back. The consumer's links lend their slots in place, so its two link
  documents generate nothing (its W5).
- **The buffer-pool cannot be lent.** The consumer's transmit lend book is hand-written (its W4) because the kind
  cannot state: a slot count and a slot length fixed at run time under build bounds (the batch size is negotiated
  per link); storage the caller holds; a headroom in front of the frame, into which the link writes its own prefix (a
  stream's length) so that prefix and frame leave in one write; a slot index that crosses an interface (a trait
  object, a C ABI) and a typed handle rebuilt from it; access through a shared reference. The generated pool has
  `SLOT_COUNT` and `SLOT_SIZE` constants, takes `&mut self` for every transition, and hides the index behind a
  typestate handle (`Slot<CpuMut>`).
- **A post answers only whether the inbox took it.** The consumer's C ABI must keep upstream behaviours that need an
  effect applied before a call returns:
  - a put made inside a subscriber callback has completed its delivery to the same process before the put returns
    (the consumer's measurement R3069);
  - zenoh-c runs that delivery on the calling thread, so a program may hold a lock across `z_put` that its own
    callback takes;
  - `z_close` returns when the Close message has left, the links are shut and every callback has finished, which is
    several steps after the call;
  - zenoh-pico's single-threaded build never reads while it puts, and its multi-threaded build without a read task
    lets whichever thread calls `zp_read` run the session.
  ADR 0084 fixes one owner thread per loop and has no way to wait for an input to be applied.
- **A refusal must carry what the protocol decides on.** zenoh 1.10.2 drops the rest of a batch and removes the link
  without a Close on an established link's decode error, ignores a sequence number outside its window, closes with
  `INVALID` on a decode error during establishment and with `GENERIC` on an extension handler's error. The
  decision needs the link, the frame's header and which codec of the frame refused.
- **Another repository builds by linking `sce_build`.** The consumer's build tool calls the library in process, with
  options the command line does not expose, and commits the generated code with a drift check. The library's API is
  not a published interface.

## The textbook structure

- **Parse at the boundary; the core sees only typed values.** Sans-I/O protocol design separates the protocol's state
  machine from its I/O (the h11 and QUIC stacks' "bring your own I/O"). The parser runs where the bytes arrive, and the
  machine receives events of a type (Parnas's information hiding: the wire format is a secret of the boundary
  module). A malformed input is a typed refusal at the boundary, never a partially read value inside.
- **A message type is declared once.** ADR 0102 §1.1: a codec document defines its value (DFDL's infoset), and its
  record view is derived from it. A second declaration of the same fields beside it would be a second definition.
- **An I/O processor carries a `<send>`.** §scxml-6.2: `type` names the Event I/O Processor and `target` the
  destination; a send that cannot be dispatched places `error.communication` on the internal queue. A link is an
  Event I/O Processor whose wire format is its codecs.
- **Zero-copy transmit is buffer lending, and batching is a buffer held open.** A network stack lends its buffers to
  the producer and takes them back at completion (BSD `mbuf` with its leading space for headers, Linux `sk_buff`
  with `skb_reserve` headroom and `skb_push`, DPDK `rte_mbuf`). A batching transmitter keeps one buffer open per
  queue, appends while it fits, and closes it on full or on a flush (Nagle; zenoh's transmission pipeline, one stage
  per priority, taken in strict priority order).
- **A handle that crosses an interface is a generational index.** An index alone is reused once its slot is freed (the
  ABA problem); an index paired with a generation that changes on each reuse is refused when stale (the slot-map
  pattern).
- **A caller that needs an effect applied asks and waits for a reply, and the server may reply later.** The actor
  model's request-reply returns a future the caller waits on, and the actor never blocks (Akka's `ask`, Erlang's
  `gen_server:call`). A server may answer after further work by replying to the caller's address
  (`gen_server:reply`). A wait has a timeout; `gen_server:call` from the server's own process deadlocks, which a
  design must rule out.
- **Whoever finds the structure free runs everyone's requests.** Flat combining (Hendler, Incze, Shavit and Tzafrir,
  SPAA 2010): threads publish requests; the one that takes the lock becomes the combiner and applies them in order,
  and each requester reads its own answer. The structure stays sequential, and no thread is dedicated to it.
- **A tool's published interface is its command line.** A build calls a code generator as a program with versioned
  arguments and machine-readable diagnostics (`protoc` and its plugins, `flatc`), wrapped by a build system's
  module (CMake's `protobuf_generate`) and a Cargo build-script helper (`prost-build`). Linking the generator's
  library makes every internal type a dependency of the consumer.
- **A name that grants a resource is minted by the system, never written by its holder** (added 2026-11-28). A
  capability is an unforgeable reference the system gives (Dennis and Van Horn, 1966; Miller, "Robust Composition",
  2006). A value handed through a channel moves to the receiver only when the channel takes it, and is given back
  otherwise (Rust's `std::sync::mpsc::SendError` holds the value it could not send). A resource a process owns, a
  file descriptor or a socket, ends with that process: it is no part of what a checkpoint restores without the kernel's
  help (Linux's TCP repair mode, which CRIU uses, is that help), and one that loses it is told (an Erlang monitor
  delivers `'DOWN'`, with `noconnection` when the node was lost). A session carried over a new connection is the
  protocol's act, a handshake (TLS 1.3 resumes a session by one, RFC 8446 §2.2).
- **A counted resource ends by an explicit act** (added 2026-11-28). A linear value is used exactly once (Wadler,
  "Linear types can change the world!", 1990), and a session-typed channel ends by an explicit close (Honda,
  Vasconcelos and Kubo, ESOP 1998). Rust's types are affine: a value may be dropped unsaid, or never (`mem::forget`
  is safe, Rust RFC 1066), so a destructor can be a safety net and not the end a protocol counts. A model of a
  protocol has a step for an end only where the end is an act. Freeing a region frees every object in it at once
  (Tofte and Talpin, "Region-based memory management", 1997), which is how a scope's end ends what it holds. And an
  abstraction a program does not use costs it nothing when the compiler can see that it is not used (Stroustrup's
  zero-overhead principle), which a report through a type parameter or a seam gets and a branch on a null hook
  does not.

The works cited in the two bullets added on 2026-11-28 were cited from memory and were not checked against their
sources when the amendment was written; the section and RFC numbers (RFC 8446 §2.2, Rust RFC 1066) and the year of
Tofte and Talpin's paper in particular.

## Decision

### D1. A machine imports a link kind; link instances are values at run time; each row is an event typed by its codec

A statechart imports a link as it imports any forge kind:

```xml
<sce:import kind="link" src="links/session_link.scxml" as="session_link"/>
```

The import names a **link kind**, fixed at build. Its **instances** exist at run time: the host opens one when its
driver dials or accepts, or when the machine asks it to (below), and the coupling (D2) gives it a `link` handle, a
generational index (D4's form) of the built-in type `link:<alias>`. A machine holds handles in its datamodel (a map
from a peer to its links, ADR 0108), compares them, and names one where it sends (D3) or closes (D3). A stale handle
(its instance closed) is refused where it is used, as D4's slot index is.

Each `<sce:inbound>` row of the link is an external event the machine may receive, and each `<sce:outbound>` row a
message it may send through the link. The event's data is the record view (ADR 0102 §1.1) of the row's codec:

- an `<sce:outbound event encode="C">` row's codec is `C`, as today;
- an `<sce:inbound>` row gains `decode="C"`, the codec whose value its event carries; a row that names none
  carries the framer's value, as today. `when` still reads the framer's decoded frame, so a row is chosen by the
  frame's header and its event typed by its body;
- a row's codec may take ADR 0100 §1.4 flag inputs, bound from the framer's decoded header by the row
  (`<sce:input name="flags" expr="decoded.flags"/>`), which is how a body whose layout depends on the header's flags
  is decoded (zenoh's per-message header flags). A codec that decodes the whole frame, header included, takes none.
  An input the row does not bind, or a codec the link does not import, is refused at generation, naming the row.

Every inbound event's `_event.origin` is the handle of the link instance it came from (ADR 0090: the origin is the
reply address its processor writes), and its data includes the framer's header fields when the framer read them
(`_event.data.header`, the framer's record view).

The machine declares nothing more. The record view is derived, so the machine's receive-side and send-side checks
(`forge::event_schema_check::check`, `check_send_side`) read it as they read an imported event schema, and
`sce:interface="closed"` holds a machine to its links' events as to its schemas. A machine that also imports an event
schema for the same event name is refused, by a diagnostic code of its own: one name, one type.

A link is MCU-class (RFC §synth-5-E): the Rust lane, in its `std` and `no_std` profiles alike, and the C11 lane
may import one, so a machine generated for a hosted Rust build and for a firmware image is one document. Importing a
link into a machine generated for another lane is refused as a link document is refused there
(`codegen/mcu-class-kind-on-non-mcu-language`).

(Amended 2026-11-28, on the consumer's request to move a link's dial plan and its deadline into the document; revised
the same day on its review, the table at the end.) An instance enters the machine's **link table**, the book its
handles index (one per imported link kind, of the bound the link declares), in one of two ways, and both give the same
handle:

- **The host opens it on its own**, when its driver accepts, or when the host dials on its own account a link the
  document does not ask for. A link may declare `<sce:opened event="E"/>`; `E` is then posted with the instance's
  record (below) before any event of the instance. A link that declares none makes the handle known first as the
  `_event.origin` of the instance's first event.
- **The machine asks for a dial**, through a host-run invocation (§scxml-6.4.1), whose lifecycle is the one every
  host-run invocation has: the ledger of ADR 0084 D5, an ending posted through the door of ADR 0084 D2, and
  `docs/SCE_ACCEPTED_SUBSET.md`, "Types the host runs".

```xml
<invoke type="sce:link-dial" id="dial" sce:link="tcp_link" sce:request="Dial">
  <param name="locator" expr="next_locator"/>
  <param name="_sce_deadline_ms" expr="dial_ms"/>
</invoke>
```

- **One invocation dials one locator; the plan is the document's.** The host makes one attempt to reach the locator
  it is handed, by the link kind `sce:link` names, and answers with an instance or a failure. Which locators, in
  what order, how often each is tried again, the backoff between tries, which failure takes a locator out of the
  plan, and the deadline of the whole open are the document's: its states, its variables and its own timers (a
  delayed `<send>`). They are where a protocol's profiles differ (one stays on a locator and tries it again, another
  walks the list round after round and drops a locator whose failure cannot be retried), so none of them is the
  host's. Two cases follow with nothing more. A list that mixes schemes is walked by invocations of different link
  kinds, one per locator, each with its own kind's framing and headroom (D4). A peer that goes on dialing the
  locators it has not reached while it already holds links, and reports that it is partly connected, does so by
  further invocations, which may run in parallel regions.
- **The type is SCE's; the service is the host's.** `sce:link-dial` is reserved, as `sce:mesh-rpc` is (ADR 0084
  D6), and the host serves it through a registration of its own per link kind (Rust `register_link_dialer::<L>`, C11
  `<machine>_register_link_dialer_<alias>`), not through `--host-invoker`, whose types are the host's to name and
  whose completion is data the host writes. The request is a host-run request: its `<param>`s, typed by
  `sce:request` as any typed host-run request is, are evaluated by the machine where the invocation starts, as ADR
  0098 Decision 1 has a parent evaluate what it hands, and a value that cannot be evaluated or does not fit its field is
  `error.execution` and starts nothing. The host's part is the I/O of the one attempt.
- **The deadline is the dial's.** `_sce_deadline_ms`, the engine's as for every host-run invocation, measures the dial
  alone: from the start of the invocation to its completion, which the host makes when the transport is connected and
  the instance open, before any message of the protocol has crossed it. The protocol's handshake over the new link is
  the machine's, under its own timers. A deadline over the dial and the handshake together, on one clock, is the
  document's: a delayed `<send>` armed before the first dial and cancelled when the handshake completes.
- **The handle is minted by the coupling, never written by the host.** The host completes the invocation with the
  instance it opened through the generated coupling of that kind (Rust `complete_link_dial(token,
  LinkInstance<L>)`, C11 `<machine>_complete_link_dial_<alias>(m, token, instance)`), not with data. The token check
  every host-run completion passes accepts it; then, in the same owner operation, the instance enters the link table,
  its handle is minted, and `done.invoke.<id>` is queued with the instance's record as its `_event.data`. A handle is
  therefore a capability: a host cannot make one from bytes, and the entry of one kind takes no instance of another
  (in Rust the type, in C11 the pointer type, differs).
- **Its type is decided at generation.** `sce:link` names an imported link alias, and the completion's data is the
  record `sce:std/link/opened.scxml` over that kind:
  - `link`, of type `link:<alias>`;
  - `slot_length` and `frame_limit`, the instance's run-time limits (D3), each a `uint32`;
  - `streamed`, a `bool`: the transport is a byte stream, so the link writes its length prefix into the headroom
    (D4); an instance that is not streamed keeps its frames' boundaries and writes none;
  - `reliable`, a `bool`: the transport delivers what it accepts, in order and without loss, or reports the link
    lost. The two are independent: a WebSocket carries datagrams and is reliable. Both are the instance's, reported
    by the host for the transport it opened, since one link kind may be carried by transports that differ in either;
  - `result`, the record `sce:result` names, when the invocation names one (what the host learned while dialing).

  The same record, without `result`, is the data of a host's own open's `opened` event. An assignment of
  `_event.data.link` to a location of any type but `link:<alias>` or `optional<link:<alias>>` is refused where it
  is written (`scxml/static-datamodel-rule`). Refused at parse, by a code of its own (`validation/link-dial-invoke`):
  no `id` (the done event and the generated entries are named after it); no `sce:link`, or one that names no
  imported link; `typeexpr`; a `namelist` or `<content>`; `sce:link` on an invocation of another type. A machine
  generated for a lane that cannot import a link is refused as an import of one is (above).
- **A failure is a record too.** The host fails the dial with a record, not with free data (Rust
  `fail_link_dial(token, DialFailed)`, C11 `<machine>_fail_link_dial_<alias>(m, token, &failed)`), and the data of
  `error.invoke.<id>` is `sce:std/link/dial_failed.scxml`:
  - `cause`, of the closed enum `unresolved` (the locator's name did not resolve), `unreachable` (no route, or no
    answer), `refused` (the peer refused the connection), `unsupported` (the host does not carry the locator's
    scheme or one of its options), `invalid-locator` (the locator cannot be read), `resources` (the host lacked what
    a dial needs: a descriptor, memory, a slot), `deadline` (the engine's: `_sce_deadline_ms` passed) and `other`;
  - `retriable`, a `bool`: whether the same locator may succeed if tried again. It is `false` for
    `invalid-locator` and `unsupported`, `true` for `deadline`, and the host's judgement for the rest.

  The deadline's `error.invoke.<id>` carries this record (`cause` `deadline`, `retriable` `true`) in place of
  the string `"deadline"` an untyped host-run invocation carries, so the machine reads one type on every failure.
- **A failure, a deadline or a cancel leaves no link.** A failed dial and a deadline are `error.invoke.<id>`, as
  above; leaving the invoking state cancels the start, and the host is told to stop. In each case no instance entered
  the table and nothing of a link is posted. A completion the token check refuses (after a cancel, a deadline or a
  restart, or a second one) mints nothing and hands the instance back to the host (Rust: `Err(Refused(instance))`;
  C11: `false`, the instance still the host's), which closes it with its driver. The instance moves to the machine
  only with an accepted completion, at one point.
- **The handle comes before the instance's events.** The coupling pumps no instance until it is in the table, and its
  first event is queued behind its `done.invoke.<id>` (or its `opened` event) through the external queue's one
  chokepoint (ADR 0084 D2, as amended at step 9), whichever thread pumps (D2). A machine never sees an origin it was
  not given.
- **An unclaimed handle.** If the macrostep that takes `done.invoke.<id>` selects no transition for it, the event is
  discarded, as any event no transition takes is, and the only copy of the handle with it: the engine closes the
  instance as `<sce:link-close armed="discard"/>` would (it has armed nothing) and posts its `closed` with the reason
  `unclaimed` (D3), reported as `link/unclaimed` (the seam of D3). A transition that takes the event and keeps no
  handle loses it as a program loses a file descriptor it does not store: the instance stays open until its peer or
  its host closes it or the run ends (D3), and a document that must not lose one states it as a property (ADR 0115
  D15).
- **The same handle as a host's own open.** A handle from a dial the machine asked for and one from the host's own
  open are of one type, `link:<alias>`, index one table, are refused when stale the same way, and are closed by
  `<sce:link-close>` and reported by `closed` the same way (D3). A machine may hold both in one map.
- **The variable that holds it.** A handle has no zero (ADR 0099), so a variable that holds one before a link is open
  is `optional<link:<alias>>`, and a table of them is a map (`map<PeerId, link:session_link>[8]`, ADR 0108).
  `<assign location="peer" expr="_event.data.link"/>` copies the handle, not the instance: a handle is a value, as an
  integer is (ADR 0108's value semantics), and two locations that hold it name one instance.
- **A saved state holds the table's generations, not its instances.** An instance is a transport's connection, owned
  by the process that runs its driver, and it does not survive that process (the textbook structure, above). A save
  writes, per link kind, each table entry's generation and whether it was open; a handle in a location is saved as
  the value it is. A restore opens nothing: every entry the save held open is restored closed with its generation
  advanced, so every saved handle that named it is stale (`stale-link` where it is used, D3), and no later instance
  can be taken for it, which a table restored from generation zero would allow (the ABA case of D4). For each such
  entry the restore posts the link's `closed` with the reason `restored`, behind the saved external queue, so that an
  instance's last event is its `closed`, as in a run. An event of the saved queue that borrowed an rx slot (D2) is
  saved as its value, since the slot is not saved. A dial still running at the save is a running host-run invocation
  and starts again as each does (ADR 0045: `restarted` set, a new token, the deadline it had left), so the host
  dials again. A protocol that resumes its peer's session over a new link does so by its own handshake; a handle
  carried over a restore would let it read a new transport as the old one. The code that writes and reads the
  generations is generated only into what saves: C11's `<machine>_saved.c` (the separate unit of ADR 0110 D2) and
  Rust's save and restore entries, so a machine whose deployment does not save links none of it. The generations
  themselves are kept in every build, since they are what makes a handle stale.
- **In the model.** In `LinkChannel.tla` (ADR 0115 D7) a dial is an invocation the machine starts and the
  environment completes, fails with a `dial_failed` record or lets the deadline take, or the machine cancels by
  leaving its state; the completion is the one step that adds the instance and posts `done.invoke`, and a completion
  after the start has ended is a step that adds nothing. A restore is the step of the restart fault (ADR 0115 D4)
  that closes every open instance with `restored`.

### D2. A received batch is decoded where it arrives, split into its messages, and handed to the machine typed

For each link a machine imports, the generator writes a **receive coupling** beside the machine (`pump_into` in
Rust, `<link>_pump_into` in C11). It runs the link's receive path for one instance and turns each frame into the
events its rows raise.

- **A batch is split into its messages.** A frame may carry several messages: a datagram or a stream frame carries
  several transport messages, and a transport frame several network messages. An inbound row may name a bounded
  list of its codec's value (`each="messages"`), and then raises one event per element, in order, each with the
  frame's header. The framer and the list's element codec read the batch; the machine sees messages.
- **Who applies the events.** When the thread that pumps holds the drive (D5), the coupling hands the machine one
  event at a time, and takes the next only after the previous one's macrostep has completed and its outputs have
  been handed out (D5): a single-threaded session reads one message, runs it, and only then reads the next, so a put
  made from the callback of message 1 leaves before message 2 is read (zenoh-pico's order). When another thread holds
  the drive, the coupling posts each event through ADR 0084's door, typed. Either way decoding happens on the pumping
  thread, before the machine; the owner only ever takes a typed value.
- **A received event holds its frame's slot, not a copy.** A link with an `<sce:rx-pool>` receives into a pool slot,
  and its events borrow from it: the inbox and the external queue hold the slot's index (D4) and the decoded view
  over it, so an inbox entry is a handle and a header, not a frame. The slot is returned when the last event of its
  frame has been taken and every output borrowing from it has been released (D4). What the machine keeps beyond that step it assigns to its own variables,
  which copy (ADR 0108's value semantics). A link without an rx-pool copies each event's value into the inbox entry,
  whose size is then the largest payload of an event the machine receives.
- **A refusal is a typed record, posted where it happened.** A frame or message its framer or its row's codec
  refuses raises no event of the row. A link may declare `<sce:refused event="E"/>`; each refusal is then posted as
  `E` with `_event.origin` the link instance and the record `sce:std/link/refusal.scxml`:
  - `path`, an enum the generator writes from the link's codec set: the framer, each row's codec, and each entry
    within them that can refuse (an extension's entry and a body's are two values), so the machine distinguishes an
    extension's failure from a body's (zenoh's `GENERIC` and `INVALID`), which a byte offset cannot;
  - `reason`, the codec's refusal reason, and `offset`, where it was found;
  - `header`, the framer's header when the framer read it, absent otherwise;
  - `no-row` as a `path`: a frame that no row's `when` matches, with its header;
  - `dropped`, the count of the batch's remaining messages: a refusal ends the batch, the messages after it are not
    decoded, and the refusal is posted after the events of the messages before it.
- **A refusal is never lost.** Each link instance keeps a refusal latch: a refusal that finds the inbox full is held
  there and posted when the inbox has room, before any later event of that instance; refusals that arrive while one
  is held are counted into it. A protocol that closes a link on a malformed frame therefore always learns of it,
  whatever the inbox's backpressure.
- **An event the inbox refuses is the link's backpressure.** Under `drop` the event is counted and dropped; under
  `block` the pump stops and leaves the rest in the driver until the next pump; under `signal-event` it is counted
  and the link's backpressure event is posted when the inbox has room again. These are ADR 0106's policies, applied
  to the inbox.
- **Every event that has a type is carried typed inside the core.** A `sce-static` machine carries the data of an
  event whose name has a type (an imported event schema, a link row's record view, a child's done data typed by its
  schema) as that typed value through every position: the external and internal queues, a `<send>` to itself, its
  parent or a child it runs, and a child's done data. Text is written only at a boundary whose format is text: the
  BasicHTTP processor, saved state's text form, the Interpreter's ECMAScript data model, the conformance trace. On
  C11, `wire.h` and `event_payload.h` therefore leave the core and become a separate unit (ADR 0110 D2's item 2),
  which a machine with an event that carries untyped data links and states as a requirement (`payload-text`); a
  machine whose every event with data is typed links no JSON reader. The c11-mcu gate holds this with a fixture whose
  events are all typed and whose image is refused if it links the unit.

### D3. A machine builds its batches in slots its links lend, with an outcome it reads at once

The machine owns its batches. A link declares its transmit **stages**, `<sce:stages count="8"/>`, each an ordered
queue of batches (zenoh's priorities); a link instance holds at most one open batch per stage. Three operations,
executable content that answers in a location as ADR 0108's `<sce:put>` does:

```xml
<sce:link-open   link="peer_link" stage="prio" event="frame.header" outcome="o"> <param .../> </sce:link-open>
<sce:link-append link="peer_link" stage="prio" event="push.message" outcome="o"> <param .../> </sce:link-append>
<sce:link-arm    link="peer_link" stage="prio" outcome="o"/>
```

- **open** lends a free slot of the link's transmit pool to the stage, with the link's headroom (D4) in front, and
  encodes the envelope row's codec (the frame header, with the sequence number the machine gives) at its start;
- **append** encodes a message row's codec at the end of the open batch. A message that does not fit is rolled back
  (the batch's length is what it was) and answers `slot-full`, so the machine gives its sequence number back, arms
  the batch, and opens another;
- **arm** closes the stage's batch and queues the slot for the driver (the pool's `dma-armed-tx`, RFC §synth-5-E).
  An express message is an append followed at once by an arm.

`link` is an expression of the link's handle type (D1); `stage` an expression of `uint8` below the declared
count. Each operation writes its outcome, of the standard enum `sce:std/link/tx_outcome.scxml`, into `outcome`:
`ok`; `slot-full`; `pool-exhausted` (no free slot); `past-limit` (the encoding passes the instance's frame limit);
`slot-short` (the instance's slot length cannot hold the headroom and one byte); `encode-refused` (a value the
codec cannot write, its reason beside it in the outcome's record); `no-open-batch` (append or arm on a stage with
none); `batch-open` (open on a stage that has one); `stale-link` (a closed instance). An operation that does not
answer `ok` changes nothing. None performs I/O, and none raises an event: the outcome is data the machine branches
on in the same transition, which is what a batch's sequence numbers need.

- **Congestion is the machine's, per message.** A message whose policy is to drop treats `pool-exhausted` as its
  drop; one whose policy is to block keeps the message and waits for room. A link may declare
  `<sce:tx-room event="E"/>`, posted for an instance once a slot returns to its pool after an operation of it
  answered `pool-exhausted`. No macrostep waits: a machine that blocks runs its own deadline timer and its own
  congestion close, as the protocol states them. The link's `<sce:backpressure>` governs its receive side only
  (D2), so ADR 0106's generation-time refusal of `block` on a coupled transmit side is not made.
- **An idle link.** A link may declare `<sce:tx-idle event="E"/>`, posted once for an instance when the driver has
  taken every slot it had armed and the instance holds an open batch. The machine arms the batch then, or, while its
  producer is still appending, starts its batching time limit: a batch leaves at once when the link is idle and waits
  for the limit only while it is being filled (zenoh's pipeline: a consumer with nothing to take takes the open batch).
  Without it the only ground to arm is a timer, so a message sent alone waits the whole limit, or the limit is zero
  and batching is lost.
- **The take order.** The driver takes armed slots in strict priority of their stage (the lower stage number first)
  and in arming order within a stage, so a conduit's sequence numbers leave in order.
- **The driver runs outside the macrostep.** Arming a slot wakes the host through the inbox's waker (ADR 0084 D2), and
  the host polls the driver after every macrostep that armed one. The driver takes each armed slot, writes the
  link's prefix into the headroom if it has one, and hands prefix and batch as one contiguous range; the slot
  returns to the pool when the driver reports the send done (`tx_complete`, then `pool_return`). A copying driver
  (today's `Link::tx`) reports it done at once; a lending one (DMA, a zero-copy stack) when the hardware has read
  the bytes. `sce_link_runtime::Link` gains the lending transmit operation beside the copying one.
- **The host holds no batch.** Every batch of a coupled link is a machine's; the host opens none and joins none.
- **One message, one frame.** `<send type="sce:link" target="peer_link" event="E">` is open, append and arm in one,
  for a link whose outbound row encodes a whole frame. Its failure places `error.communication` on the internal queue
  (§scxml-6.2), with `_event.sendid` and `_event.data.reason` the outcome's value (ADR 0106 #111). `target` is an
  expression of the link's handle type; `targetexpr` is how a run-time instance is named.
- **A machine closes a link.** `<sce:link-close link="peer_link"/>` asks the host to close the instance once what it
  armed has been taken; `<sce:link-close link="peer_link" armed="discard"/>` closes it at once, returning its armed
  slots unsent, which is how a protocol drops a link on congestion or on a decode error without a Close, and how a
  link whose peer stops reading is ever closed; a link may declare `<sce:closed event="E"/>`, posted when the instance
  is closed by either side, after which its handle is stale. Its data, and the host's part in a close, are below.
- **A run-time frame limit, lowered by the protocol** (amended 2026-11-28). An instance's slot length and its frame
  limit are given when the host opens it, on its own or by a dial (D1): the transport's own, such as the path's MTU or
  the largest frame the link carries, each at most the pool's build bounds (D4), and the `opened` record states them.
  The batch size a protocol negotiates is known only after its handshake (the smaller of the sizes the two sides
  advertise), so the machine lowers the instance's limit to it:

  ```xml
  <sce:link-limit link="peer_link" frame-limit="negotiated" outcome="o"/>
  ```

  The limit only falls. A value above the instance's current limit answers `not-lower`; zero answers
  `limit-short`; a stage holding an open batch already longer than the value answers `batch-open`; a closed instance
  answers `stale-link`; each changes nothing. `ok` applies the value to every operation after it. The outcome is of
  `sce:std/link/tx_outcome.scxml`, which gains `not-lower` and `limit-short`. In `LinkChannel.tla` it is the
  machine's step `LowerLimit(i, n)`, enabled only for an `n` below the instance's limit, so the limit never rises in
  any behaviour.
- **What the host holds, it ends by saying so** (amended 2026-11-28, on the consumer's request: an end made only by a
  destructor or a reference count is no step of the model). An end that happens when a value is destroyed is not an
  event: the model of ADR 0115 has no step for it, its moment is the allocator's, a cycle of references never comes to
  it, and in Rust it may not come at all (the textbook structure, above). So every handle the host holds that the
  machine counts ends by an explicit input of the host, posted through ADR 0084's door and a step of the model.
  `Drop` stays, as a safety net that keeps the core sound, and what it does is observed as a violation of the
  contract.
  - **A link instance.** The host ends its instance by `close(instance, cause)` (C11 `<link>_close(instance,
    cause)`), `cause` `peer` (the driver saw the peer close, or lost the transport) or `host` (the host's own
    decision, its shutdown among them). Where the machine has asked to close it (`<sce:link-close>`), the host's close
    completes that request and the reason is the machine's. The `closed` event's data is the record
    `sce:std/link/closed.scxml`: `reason`, one of `asked` (the machine's close, its armed slots taken), `discarded`
    (the machine's `armed="discard"`), `peer`, `host`, `unclaimed` (D1), `restored` (D1) and `dropped` (below);
    and `unsent`, the count of armed slots returned unsent. Its `_event.origin` is the instance's handle, as for
    every event of the instance (D1).
  - **A host handle.** A document names the kinds of handle the host holds of what the machine keeps for it (a
    session, a subscription the machine delivers to, a query whose replies it hands out), each with the events that
    end it:

    ```xml
    <sce:host-handle kind="session" capacity="4" released="session.released"/>
    <sce:host-handle kind="subscriber" capacity="64" scope="session"
                     released="subscriber.released" delivery-ended="subscriber.delivery-ended"/>
    ```

    The door keeps a table of `capacity` entries per kind (the bound, ADR 0086), whose index is D4's generational
    form and whose type in the datamodel is `handle:<kind>`. The host takes an entry before it tells the machine of
    the object (Rust `inbox.host_handle::<Session>()`, or `host_handle_in::<Subscriber>(&session)` for a kind with a
    scope; C11 `<machine>_host_handle_take_<kind>`; each answering `full` when the table is), passes its index in the
    event that declares the object, and ends it by `release`, which posts the `released` event, and, for a kind that
    names `delivery-ended`, by `end_delivery`, which posts that event: the host's sink of the machine's outputs for
    the handle is gone, while the handle may still be held. Each is posted at most once, and `release` ends the
    delivery too. Their data is the record `sce:std/host/handle_end.scxml`: `handle`; `how`, `explicit` or
    `dropped`; and `scoped`, the count of handles that ended with it (below). The machine ends a handle from its side
    by `<sce:handle-end handle="h"/>`; the host's later `release` then answers `ended` and posts nothing. An entry is
    free again, its generation advanced, only when both holders have let it go: the machine by taking its
    `released` event or by `<sce:handle-end>`, the host by its `release` or by its value's end. An output addressed
    to a handle whose delivery has ended is not run by the host's adapter and is reported as
    `handle/after-delivery-end`; that is the race of two holders, not a violation.
  - **A scope ends at once.** A kind may name another kind as its `scope`, and each of its handles is then taken in
    a handle of that kind. Ending a scope's handle (the host's `release`, the machine's `<sce:handle-end>`, or the
    safety net below) ends every handle taken in it, in the same step, by one input: the scope's own `released`
    event, whose `scoped` counts them; no event is posted for each, and each of their host values answers `ended`
    from then on. A session's close is therefore one input that ends every subscription, query and publisher of it,
    as freeing a region frees every object in it (the textbook structure, above). Scopes form a tree; a cycle, or a
    scope that names no declared kind, is refused at parse (`validation/host-handle-scope`).
  - **What waits for the last run is the adapter's.** An end is a post (below), so it returns before the machine has
    taken it, and possibly while the host's adapter is still running an output for that handle. An API whose
    contract is that its undeclare returns only after the last callback has run waits in its adapter: it sets the
    entry's delivery-ended bit, after which no new run starts (the word below), waits for the runs in progress to
    finish, posts the end, and returns. The machine waits for nothing.
  - **The table lies in storage the caller may hold.** As a pool (D4), a table may own its `capacity` entries or be
    made over a span its caller lends, under the Lent contract (ADR 0146, which states it in full for ADR 0154, in
    draft): a count `c <= capacity` given at run time, the span's length checked once at construction, all zero
    when empty, the span outliving the table. An entry is one atomic word, whose layout is generated and published:
    the generation, the entry's state (free, held, ended by the machine, ended by the host), the delivery-ended bit,
    and bits left to the holder. An adapter keeps its call latch (its count of runs in progress) in those bits, so
    the check that the delivery has not ended and the latch that admits a run are one compare-and-swap of one word,
    and an end sets its bit in that word, after which no compare-and-swap admits a run. A target without an atomic
    of that width runs the transition inside the deployment's critical section, as a `shared` pool does (D4).
  - **An output's hold on an rx slot** (D4) ends when the host's run of that output returns, by the adapter's
    explicit release; an output the host keeps beyond its run copies what it keeps.
  - **A value of SCE's dropped without its end is a violation, counted and still told.** The violation is exactly
    this: a `LinkInstance` or a `HostHandle`, the values SCE's runtime hands a host, dropped without its end. An
    adapter whose own API's contract makes the drop of a handle its end (an API whose drop of a subscriber is its
    undeclare) calls `release` in that drop, and that is an explicit end, not a violation: the end is the API's act,
    made at the point its contract names. A value of SCE's dropped without its end does what the end would, and
    marks it: the instance's `closed` with the reason `dropped`, the handle's `released` (and its
    `delivery-ended`, if that was not posted) with `how` `dropped`, a scope's ending its scope as `release` would.
    The machine still learns of the end and the core stays sound, but the end is not a clean one, and it is reported
    as `link/dropped` or `handle/dropped` (the seam below). C11 has no destructor, so its ends are the explicit
    calls alone. A value the host forgets without dropping it (`mem::forget`, a leaked reference) is never seen: its
    entry stays held until the run ends, when the engine ends every instance and handle still held, posting nothing to
    a run that has ended, and a host value used after that answers `ended`. That is the one gap the safety net
    leaves, and it holds an entry of its own kind until the run's end, never another holder's.
  - **A violation is reported through a seam, which costs nothing when it does nothing.** The counters, the report
    to the trace port (ADR 0115 D4, D8) and a debug build's log are not in the runtime; they are an observer of the
    host's contract that the deployment gives, and every report named in this decision (`link/dropped`,
    `handle/dropped`, `link/unclaimed`, `handle/after-delivery-end`) goes to it. In Rust the runtime's values take
    the observer as a type parameter (`LinkInstance<L, O: ContractObserver = Unobserved>`); `Unobserved` is a type
    of size zero whose methods are empty, so monomorphisation leaves no code and no field of it, and `Counting`
    (the counters) and `Tracing` (the trace port) are the two the runtime provides. In C11 it is the seam
    `_sce_contract_violation(kind, index)`, declared by the runtime and defined by the deployment as every C11 seam
    is (ADR 0110 D2); an empty definition is removed by link-time optimisation. One program is built either way.
    The safety net itself, the posted end, is no part of the seam: the core's soundness does not wait on an
    observer.
  - **An end is never lost.** An end that finds a bounded inbox full is held in its table entry, as a refusal is held
    in its link's latch (D2), and posted when the inbox has room, before any later input of that instance or handle.
    An end is always a post, never an ask (D5), so it may be made wherever a post may: on any thread, from an
    interrupt, and inside a macrostep, where a callback ends a handle.
  - **In the model.** An end is a step of the environment, written where the environment is (a sketch; the modules
    are written with step 3):

    ```tla
    \* LinkChannel.tla (ADR 0115 D7): the host's end of instance i
    HostClose(i, c) == /\ c \in {"peer", "host"} /\ inst[i] = "open"
                       /\ inst' = [inst EXCEPT ![i] = "closed"]
                       /\ Post(Closed(i, IF asked[i] # "none" THEN asked[i] ELSE c))
    HostDrop(i)     == /\ ~HostEndsExplicitly /\ inst[i] = "open"
                       /\ inst' = [inst EXCEPT ![i] = "closed"]
                       /\ Post(Closed(i, "dropped"))
    ```

    `HostHandle.tla`, beside the connectors, states the door's handle table the same way. Its steps are
    `Take(k, s)`, `Release(h)`, `EndDelivery(h)`, `MachineEnd(h)`, `EndScope(s)` (every handle of the scope `s`
    in one step, with `s` itself), `StartRun(h)`, `FinishRun(h)` and `HostDropHandle(h)`, and an entry is free
    again only after both holders' steps. `StartRun(h)` is enabled only while `h`'s delivery has not ended, so the
    invariant `NoRunAfterDeliveryEnded` (no run of an output for `h` begins after `EndDelivery(h)`, `Release(h)`
    or the end of its scope) holds of every behaviour, and an API's promise that nothing is delivered after its
    undeclare returns refines onto it. `HostEndsExplicitly` is a named assumption of the host (ADR 0115 D6): under
    it no behaviour has a `dropped` end, so a run of the code whose trace reports one (level 5, ADR 0115 D8) is
    refused, TLC naming the step it cannot explain, which is where a test sees the violation; the run that takes the
    assumption out (ADR 0115 D5) shows which properties stand on it. In a document's model (level 4, ADR 0115 D7) the
    end events are inputs of its alphabet, and the exploration's environment gives a handle's ends only after it was
    taken and at most once each, which is `HostHandle.tla`'s promise.

### D4. A pool is lent: run-time counts and lengths, caller-held storage, a headroom, a generational index, a shared book

The buffer-pool kind keeps its seven-state ownership machine (RFC §synth-5-E); what a holder of a slot may do is
unchanged. The generated Rust and C11 pools gain:

- **A run-time slot count and slot length under the build bounds.** `<sce:slot-count>` and `<sce:slot-size>` are
  the bounds `B` and `S`: the storage reserved at build (ADR 0086). A pool is constructed with a count `c`,
  `1 <= c <= B`, and each lending names a length `l <= S` (a link instance's negotiated slot length), so the lent
  slot is `l` bytes long. A value outside its range is refused at construction or at the lending (`Err`;
  `false`). This is U16's rule, the rule ADR 0108 D3/D4 gave a map's run-time limit: the type holds the bound, the
  value holds the limit, and the storage never grows.
- **Storage the caller holds.** A pool may own its storage, as today, or be made over storage the caller holds
  (ADR 0082) and sized at run time: `Pool::over(bytes, c, l)` takes a span of at least `c * l` bytes (aligned as the
  pool declares) and the count and length it is cut into, each within the build bounds, and the book of `B` entries
  stays the pool's own (C11: `<pool>_init_over(&pool, bytes, len, c, l)` on a `<pool>_over_t`). A deployment that sizes its arena by a negotiated
  length lends exactly what it negotiated, and the span is refused at construction when it is short or misaligned.
- **An empty pool is all zero.** The constructor is a `const fn` (C11: a static initializer), and a pool with no slot
  out is all zero bytes (state `free` is 0, generation 0), so a `static` pool lies in `.bss`, as ADR 0108 made an
  empty map.
- **A headroom.** A link names the headroom its framing needs on its pool reference, `<sce:tx-pool ref="p"
  headroom="2"/>` (none for a datagram, the length prefix's width for a stream). The envelope is encoded at the
  headroom's end; on taking the slot the link writes its prefix, at most the headroom, just in front and hands the
  two as one range. A prefix wider than the headroom is refused when the link is generated where its width is known,
  and by the take (counted, the slot returned) where it is not.
- **A slot index that crosses an interface.** A lent slot has a `SlotIndex`: the slot's number and its generation, a
  `u32` each, `Copy`, `repr(C)` in Rust and a plain struct in C11. The generation is incremented each time the
  slot returns to `free`. Generated code holds the typestate handle (`Slot<S>`); the index crosses a trait object
  or a C ABI. `pool.reclaim::<S>(index)` (C11: `<pool>_over_reclaim(&pool, index, state)`) rebuilds the handle if the slot is in
  state `S` and its generation is the index's, and refuses otherwise: a stale or forged index is counted as an
  ownership violation (`pool/ownership-violation`) and yields nothing. `Slot::idx()` becomes `Slot::index()`.
- **A shared book.** Every transition takes `&self`; the pool's state book is interior-mutable. A pool declares how
  it is shared, `<sce:sharing>owner</sce:sharing>` (the default) or `shared`:
  - `owner`: whoever holds the drive (D5) uses the pool. The book is `Cell`s and the pool is `!Sync`; a
    transition is a load and a store.
  - `shared`: the drive and a driver context (another thread, an interrupt) both move slots, as a DMA completion
    handler does. Each transition is a compare-and-swap from the state it leaves. On a target with no
    compare-and-swap (ARMv6-M) it runs inside the deployment's critical section (`critical-section` in Rust, the
    seams `_sce_critical_enter` and `_sce_critical_exit` in C11, declared exactly when a pool is `shared`).
  The hardware-owned edges stay `unsafe` in Rust, since the caller asserts what the hardware did.
- **A slot lent to an event and its outputs.** A receive slot whose frame raised events is lent to them (D2). Its
  hold count counts the events that hold it and the outputs that borrow from it (a subscriber's delivery handed out
  after the step, D5), and the slot returns when the count falls to zero: when the last event's macrostep has
  completed and every output that borrows it has been released by the host that ran it.
- **The storage type decides what a pool keeps.** By the rule ADR 0154 gave a map (`MapStorage::Generation` is
  `u32` in lent storage and `()` in a map's own), the book, the generation, the run-time count and length, the
  headroom, the index and the lent holds belong to a pool over storage the caller holds, and the pool over its own
  storage keeps the layout and the code it had before the book: one state byte per slot beside the slots, a handle
  that is the slot's number, a lending that names no length, and transitions that take `&mut`. No document declares
  it; one document gives both types. In Rust, `Storage` gives `Owned` the state array and `Generation = ()` and every
  `SlotStore` (`Over<'a>`) the book and `Generation = u32`; `Slot<S, G>` carries `G`, defaulting to the handle of
  the pool over its own storage, and `SlotIndex`, `into_index`, `reclaim` and the rest are `Pool<Over<'_>>`'s. A
  program that owns storage whose slots must cross an interface by index makes `Pool::over` over a `&'static mut`
  buffer. In C11 each form is a header of its own: `<pool>.h` gives the pool over its own storage, the static one,
  with the includes, macros and functions it had before the book, and the lending form, `<pool>_over_t` with
  `<pool>_init_over` and `<pool>_over_*`, is `<pool>_over.h`, which includes `<pool>.h`. C11: the lending form is
  `<pool>_over.h`, and only a program that lends includes it; a unit that keeps every `static inline` it reads (a
  footprint gate built with `-fkeep-inline-functions`) then holds nothing of a form it does not use.
  `<sce:sharing>shared` alone changes the pool over its own storage: a driver context moves its slots too, which one
  state per slot cannot be shared with, so its state is the atomic book (Rust: `Owned` is a `SlotStore`; C11: the
  static pool is a `<pool>_over_t` over the header's storage behind the calls it had before the book, so a shared
  pool's `<pool>.h` includes `<pool>_over.h`, the one pool header that does). A link's send takes its transmit pool
  as its type lends: Rust through the pool's `TxPool` (`&mut` the pool over its own storage, `&` the lending type),
  C11 by two sends, `<link>_send_<row>` in `<link>.h`, which includes `<pool>.h`, and `<link>_send_<row>_over` in
  `<link>_over.h`, which includes `<link>.h` and `<pool>_over.h`.

The `queue` kind takes the run-time capacity by the same rule: `CAPACITY` is the bound, and a queue is constructed
with `c <= CAPACITY`, refused otherwise. Its full policy is unchanged (U17 is a record of its own).

### D5. The owner is whoever holds the drive; a post may be asked and answered, now or by a later reply

ADR 0084 D1 and D2 are amended.

- **The drive.** A loop's state belongs to whoever holds its **drive**: a token taken outside a macrostep and given
  back outside one. Run-to-completion is unchanged: one holder at a time, and the drive never moves inside a step.
  ADR 0084's owner thread is the case of a host that never gives it away. A host may also leave it free, and then any
  thread that calls into the loop takes it, applies its own input and what the inbox holds, runs the loop until it is
  quiet, hands out the outputs, and gives it back (flat combining). No thread need be dedicated to the loop, which is
  zenoh-pico's single-threaded and its read-task-less multi-threaded builds, and the consumer's cooperative engine.
  Rust holds the drive as a `&mut` of the engine obtained from the loop's lock; C11 as a flag taken by
  compare-and-swap (or with interrupts masked, ADR 0084's row 0); the other runtimes as their lock.
- **Outputs are handed to whoever caused them.** A macrostep's outputs (what it sends to host processors, a
  subscriber's delivery among them) are handed out by the drive's holder when the step completes. The output of an
  asked input's macrostep is handed to the asker, on the asker's thread: an ask's completion carries those outputs,
  and the asker runs them before its call returns. zenoh-c's delivery on the calling thread, and a lock the program
  holds across the call, are kept. A host may hand other outputs to other threads (zenoh-c runs a link's callbacks
  concurrently with another link's), and an ask made from such a thread is an ask of another thread.
- **An ask.** The door gains a second kind of post, which returns a completion. An asked input is answered when the
  macrostep that took it has completed, with `processed`, `unseen` (the run had ended, §scxml-3.13) or `refused`
  (the token check refused an ending), and the outputs of that step. An input the inbox refuses is refused at the ask
  and has no completion.
  - An asked input may borrow the asker's data until its completion, since the asker waits for it: the inbox entry
    holds a reference to the caller's payload, not a copy, so a put of a large payload from another thread costs no
    inbox entry of the largest payload's size. A plain post copies, since its poster does not wait.
  - If the drive is free, the asker takes it and the answer is there when the ask returns.
  - If another holds it, the asker waits (Rust `std`: `Completion::wait(deadline)`; `no_std` and C11: a slot of a
    table the deployment sizes, `SCE_MAX_ASKS`, polled or signalled by a callback). A wait takes a deadline and answers
    `timed-out`, the input staying posted; `cancel` withdraws it if the holder has not taken it yet. An interrupt
    handler may not ask, only post: it preempts the holder it would wait for.
  - If the asker holds the drive, between macrosteps (in a callback it is running for a step's output), its ask is
    applied before it returns: the input is applied, and the loop runs until that input's macrostep has completed,
    whose outputs it runs. R3069 holds: a put made in a subscriber callback has completed its delivery before the put
    returns. Inputs other threads posted before the ask are applied first, in their order; events of the holder's own
    receive coupling are not posted (D2), so none of them is applied in between, which keeps zenoh-pico's order of a
    put inside a read. The holder may hand the nested step's outputs to its callback only after that callback returns
    (the consumer's engine defers them so); the completion says the step ran, not that its outputs were run.
  - Nesting is bounded by `SCE_MAX_ASK_DEPTH`, which the deployment sizes; an ask past it is refused (`depth`) and
    applies nothing, its input not posted.
  - An ask made inside a macrostep (from executable content, through a host processor's handler) is refused
    (`inside-macrostep`): applying it there would observe an input in the middle of a step, which ADR 0084 D1
    forbids, and waiting for it would deadlock, which is `gen_server:call`'s error. A document that needs that order
    raises or sends the event itself.
- **A later reply.** An inbound event of an ask carries the asker's reply address as its `_event.origin` (ADR 0090).
  An event schema, or a link row, may declare `reply="explicit"`: its ask is then answered not when its macrostep
  completes but when the machine sends `<send type="sce:reply" target="_event.origin">` to that address, from any
  later step (`gen_server:reply`). `z_close` is such an ask: the machine arms the Close, closes the links, waits
  for their `closed` events and for its callbacks to finish, and then replies. A machine that ends with an explicit
  ask unanswered answers it `unseen`. A close asked from a callback does not wait for that callback.
- **Every runtime.** ADR 0084 holds one threading rule for every engine, so every runtime gains the drive and the ask
  with the same outcomes. Rust and C11 first, since the consumer needs them; the others in the order 0084's step list
  used. ARCHITECTURE.md's threading section states the rule once.

### D6. Another repository builds through the `sce-codegen` command line, a CMake module and a build-script helper

The supported way for a build outside this repository to generate code is the `sce-codegen` program:

- **The command line is the interface, and it covers every option.** Each option of the library's compile entries
  that a consumer uses is a flag: the views and the owned origin (`ForgeCompileOptions`), a deployment's node
  storage, the `no_std` profile, imports. A `validate` subcommand checks a document against the grammar without
  generating. `--error-format=json` diagnostics (SCE_ERROR_CONTRACT.md), `--plan` for the files a run writes, and
  `--write-deps` for the documents it read. `sce-codegen --version` prints the generator's revision and an
  interface version, which changes when a flag a consumer relies on changes.
- **Output forms.** `--emit module` (today's form) and `--emit include`, a form an `include!` can take: no inner
  attributes and no inner doc comments, and no generated tests. A consumer that commits generated code writes into its
  own tree (`--output-dir`) and checks it with `--check`, which answers whether a run would change any file and
  writes nothing. ADR 0097 is this repository's choice; a consumer may commit its outputs.
- **C and C++: the CMake module.** `cmake/SCECodegen.cmake` and `SCECodegenPlan.cmake`, installed with
  `find_package(SCE)`, are that interface, and gain one function for forge documents of every kind,
  `sce_add_forge(TARGET <t> DOCUMENTS <files> LANGUAGE c11|cpp [WITH_IMPORTS])`, declared from the plan as every
  generation is (`sce_add_algorithm` becomes a call of it).
- **Rust: a build-script helper crate.** `sce-codegen-build`, a build dependency, finds the program (`SCE_CODEGEN`,
  else `PATH`), refuses one whose interface version it does not know, runs it with `--plan` and `--write-deps`,
  and prints `cargo:rerun-if-changed` for every document read. Writing module declarations is an option, off when
  the consumer writes its own. It is the one implementation of ADR 0097's build-script flow:
  `sce_build::action::build_script` becomes a client of the same steps, so the two cannot drift.
- **`sce_build`'s library API is not an interface for another repository**, and nothing promises it. The consumer's
  build tool moves to this entry after it lands; that move is the consumer's.

## Consequences

- **A protocol machine sees only typed events,** split into messages, with the link it came from and the header it
  came under; a malformed frame reaches it only as a typed refusal that names where the frame failed, and is never
  lost to a full inbox.
- **A machine builds its own batches,** with the outcome of each step in hand, so a batch's sequence numbers, its
  priorities and its congestion policy are the protocol's logic in the document.
- **No I/O happens inside a macrostep.** The driver takes armed slots when the host polls it, and returns them when
  the hardware is done.
- **A C11 machine whose events are all typed links no JSON reader,** and a received frame is held in its rx slot, not
  copied into the inbox.
- **The pool's API changes** (`&self`, run-time count and length, `over`, `SlotIndex`, `reclaim`); a consumer that
  drives a generated pool by hand follows `docs/SCE_CODEC_CONSUMER_MIGRATION.md`'s section.
- **The owner is a role, not a thread.** A caller can wait for an effect, run the outputs it caused on its own
  thread, and be answered after several steps, without the core giving up run-to-completion or a single holder.
- **Another repository builds from a program, not a library,** so the generator's internals stay free to change.
- **A dial plan and its deadline are the document's** (amended 2026-11-28), and the handle the machine receives is the
  one a host's own open gives. A failed, cancelled or late dial leaves no link, and a restore leaves every saved
  handle stale and says so by `closed`.
- **Every end the machine counts is a step of the model** (amended 2026-11-28), made by the host explicitly. A value
  dropped without one still ends, counted as the host's violation and visible in a run's trace.

## Alternatives rejected

1. **Decode on the owner, at the drain.** The inbox would carry bytes, the owner would parse inside its loop, and a
   malformed frame would be found after the poster had moved on.
2. **A step that calls a codec (the consumer's first request, U4).** It puts the wire format inside the machine.
3. **An event schema written beside each codec.** It states every field twice; ADR 0102 §1.1 rejected it.
4. **A send that encodes one message per slot and reports failure as `error.communication` only.** A batch's
   sequence number must be given back in the same transition that learns the message did not fit; an event on the
   internal queue arrives after the transition has moved on. The outcome in a location answers at once.
5. **The host holding the batch.** It would hold protocol state (which frame is open, its sequence number) outside
   the document, which is what the consumer is moving into it.
6. **A link fixed at build.** A protocol runs several links at once, opened and closed at run time; a close is per
   link.
7. **A plain slot index across the interface.** It is reused once its slot returns; the generation refuses a stale
   holder.
8. **Always atomic, or always `&mut`.** `&mut` cannot be shared with the link that holds an armed slot while the
   session opens another. Always atomic costs a compare-and-swap per transition on a loop that needs none.
9. **A dedicated owner thread.** zenoh-pico and the consumer's cooperative engine have none; the drive passes to
   whoever calls, which flat combining makes safe.
10. **Running an ask's outputs on the drive's holder.** A program that holds a lock across `z_put` that its callback
    takes would deadlock until the deadline.
11. **Applying the owner's own ask later, as an internal event.** It would lose R3069's order.
12. **Publishing `sce_build`'s API.** Every internal type would become a consumer's dependency.
13. **A host-declared invoke type whose `sce:result` carries the handle** (2026-11-28). The host would write a
    capability as data, so it could make one up or complete with another kind's; an instance the coupling opened and
    the table minted at the completion cannot be either.
14. **A `<send>` to the host asking for a link, answered by an event** (2026-11-28). It has no cancel when the state
    is left, no deadline, no single ending and no restart on a restore; a host-run invocation has each, in the one
    ledger of ADR 0084 D5, and a second lifecycle beside it would be a copy.
15. **A handle that survives a restore, its host attaching a new connection to it** (2026-11-28). The protocol state
    the old connection carried (sequence numbers, open batches, what was negotiated) would be read against a
    transport that never carried it; a resumed session is the protocol's handshake.
16. **A link or a handle ended by its value's destruction alone** (2026-11-28). The model has no step for it, its
    moment is the allocator's, a cycle of references never reaches it, and `mem::forget` skips it.
17. **A destructor that refuses: a drop bomb, a `Drop` that panics unless the value was ended** (2026-11-28). A
    panic in a `Drop` that runs while the thread unwinds aborts the process, and the last reference of a C ABI's
    value is counted down in a program that cannot be told why; the safety net keeps the core sound and the
    violation observed.
18. **Compiling the reports out by a build switch (a Cargo feature or `cfg`, a preprocessor macro)** (2026-11-28,
    the consumer's review, 11(b)). The goal, no cost when nothing observes, is taken; the means is not. A switch that
    compiles code out is a second program that nothing tests unless it is built both ways, which ADR 0110 D2 refuses
    ("It never becomes a preprocessor switch"). The observer seam reaches the same cost with one program.
19. **One invocation that walks a list of locators by the host's plan** (2026-11-28, the amendment's first text). A
    protocol's profiles differ in the order, the retries and what a failure takes out, which is protocol logic outside
    the document, and a list that mixes schemes cannot share one link kind's framing.

## Order of work

1. This record.
2. The pool (D4), Rust and C11: run-time count and length, `over`, the all-zero empty pool, the headroom,
   `SlotIndex` and `reclaim`, the `owner` and `shared` books, a slot lent to events, with the kind's conformance
   fixtures; the queue's run-time capacity. Landed with this step: the pool and its conformance contract,
   `tests/forge/conformance/buffer_pool/pool_contract.json`, which the Rust and C11 arms both run. The hold count of a
   slot lent to events and `take_prefixed` are the pool's; the receive coupling (step 5) and the take (step 3) drive
   them. The queue's run-time capacity has not landed. Amended (2026-11-28): the first landing made every pool lend,
   and a consumer measured what an MCU pool that lends nothing then paid (C11 at `-Os`: a pool's text from 820 to 2914
   bytes, its state from a byte per slot to a 24-byte line; Rust: a 4-byte handle grew to 8 on a 32-bit target, and
   every lending checked a run-time length and stored a book line). The storage type now decides it, as a map's does
   (ADR 0154): the pool over its own storage keeps the layout and the code it had before the book, and only the
   lending type, `Over` and `<pool>_over_t`, keeps the book and the generation. The contract runs against each pool's
   lending type, and each arm holds the pool over its own storage to that layout.
   Amended again (2026-11-28): the C11 header still held both forms, each `static inline`, and a footprint gate
   that keeps every inline function a unit reads measured a pool's text at 3434 bytes where the header before the
   book gave 820. The lending form is now a header of its own, `<pool>_over.h` (and a link's sends into it,
   `<link>_over.h`), which only a program that lends includes.
3. The link's transmit side (D3): stages, open, append and arm with their outcomes, the take order, the lending
   `Link` operation, the prefix, `tx-room`, `tx-idle`, `link-close` in both forms and `closed`, on the loopback fixtures
   of `tests/forge/conformance/link_events/`, which gain a lending driver and a batching link. With it (amended
   2026-11-28), the host's explicit end (D3): `close` and its causes, the `closed` record and its reasons, the host
   handle kinds (`<sce:host-handle>`, their table over lent storage and its one-word entry, `released` and
   `delivery-ended`, `<sce:handle-end>`, scopes and their one-step end), an output's hold released when its run
   returns, `Drop` of SCE's values reported as `link/dropped` and `handle/dropped` through the contract observer
   (Rust's type parameter, C11's seam), an end held while the inbox is full, `<sce:link-limit>`, and the steps of
   `LinkChannel.tla` and `HostHandle.tla` with `NoRunAfterDeliveryEnded`; each with a fixture that ends explicitly
   and one that drops, whose trace the model refuses.
3a. A machine asks for a dial (D1, amended 2026-11-28), before step 4, whose fixtures open their links by it; the link
   table and its handle type are taken into this step from step 4. `sce:link-dial`, one locator per invocation, its
   registration per link kind, the deadline of the dial alone, the minted handle and the `opened` record with
   `streamed` and `reliable`, `<sce:opened>`, the `dial_failed` record, the completion the token check refuses
   handing the instance back, the unclaimed handle, the saved table's generations (in the saved unit only) and the
   `closed` a restore posts, and the dial's steps in `LinkChannel.tla`; Rust and C11, on the loopback fixtures, with
   a dial that completes, fails retriably and not, passes its deadline, is cancelled, is completed after its cancel,
   and is running when the machine is saved, and a document that walks two locators of two link kinds.
4. A machine imports a link (D1): the grammar, link handles (their table from step 3a), record views as event types,
   `decode`, `each`, row inputs, and the refusals.
5. The receive coupling (D2): one event at a time under the drive, typed posts otherwise, rx slots lent to events,
   the refusal record and its latch; `<send type="sce:link">`; held end to end by a machine that talks to itself over
   a loopback link in Rust and C11, with the same scenario's trace compared between them.
6. Typed carriage inside the C11 core (D2's last item): `wire.h` and `event_payload.h` into the `payload-text`
   unit; the c11-mcu gate gains the all-typed fixture.
7. The drive and the ask (D5): Rust and C11, then the other runtimes; ARCHITECTURE.md.
8. The build entry (D6): the flags, `validate`, `--emit include`, `--check`, the interface version,
   `sce_add_forge`, and the `sce-codegen-build` crate, with this repository's build scripts moved onto its steps.

## Open questions

- None for this record. U17 (a full queue's policy) reopens ADR 0062 in a record of its own (order item 4b); U7 (a
  pool of statechart instances chosen by a run-time key) is item 4c.

## Review of the amendment of 2026-11-28

The consumer reviewed the amendment the day it was written and made twelve points. Each is answered here and the
text above carries the answer.

| # | Point | Answer | Where |
|---|---|---|---|
| 1 | Who owns the dial plan | Taken: one invocation dials one locator; the order, the retries, the backoff and the deadline of the whole open are the document's, and the sentence that gave them to the host is gone | D1, "One invocation dials one locator; the plan is the document's"; Alternatives rejected, 19 |
| 2 | A list of mixed schemes; partial connectivity | Taken, as a consequence of 1: one invocation per locator, of that locator's link kind, and further invocations while links are held | D1, the same bullet |
| 3 | A typed failure | Taken: `sce:std/link/dial_failed.scxml`, `cause` (a closed enum) and `retriable`; the deadline carries it too | D1, "A failure is a record too" |
| 4 | What `_sce_deadline_ms` measures | Taken: the dial alone; the handshake and a deadline over both are the machine's | D1, "The deadline is the dial's" |
| 5 | Lowering `frame_limit` | Taken: `<sce:link-limit>`, which only lowers, to a value at most the one `opened` stated | D3, "A run-time frame limit, lowered by the protocol"; Order of work, 3 |
| 6 | `streamed` and `reliable` in `opened` | Taken, as properties of the instance | D1, "Its type is decided at generation" |
| 7 | One name for two operations | Taken: the invocation is `sce:link-dial`; `<sce:link-open>` stays the batch's open | D1; Amends; Order of work, 3a |
| 8 | The scope of the `Drop` violation | Taken: only SCE's `LinkInstance` and `HostHandle` dropped without their end; an adapter whose API makes the drop its end calls `release` in it, an explicit end | D3, "A value of SCE's dropped without its end is a violation" |
| 9 | A session's close ends its handles | Taken: a kind names its `scope`, and the scope's end ends every handle in it in one step | D3, "A scope ends at once"; "In the model" (`EndScope`) |
| 10 | Waiting for the last callback | Taken: the wait is the adapter's; `HostHandle.tla` holds `NoRunAfterDeliveryEnded` | D3, "What waits for the last run is the adapter's"; "In the model" |
| 11(a) | The table's entries in the caller's storage, one word | Taken: the Lent contract, and one atomic word holding the delivery-ended bit and the holder's latch | D3, "The table lies in storage the caller may hold" |
| 11(b) | No cost for the reports | The goal taken, the means changed: an observer seam (a Rust type parameter whose empty case is of size zero; a C11 seam with an empty definition), not a build switch, which ADR 0110 D2 refuses | D3, "A violation is reported through a seam"; Alternatives rejected, 18 |
| 11(c) | The generations' save code only where a machine saves | Taken: generated only into the saved unit | D1, "A saved state holds the table's generations" |
| 12 | The consumer's own release of a link by `Drop` | The consumer's: it moves to the explicit `close(instance, peer \| host)`; nothing in this record | (none) |
