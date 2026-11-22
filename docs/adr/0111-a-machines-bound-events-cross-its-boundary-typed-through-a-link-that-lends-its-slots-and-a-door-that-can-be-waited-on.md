# ADR 0111 — A machine's bound events cross its boundary typed, through a link that lends its slots and a door that can be waited on

- Status: Accepted (2026-11-22). Revised twice the same day on the consumer's review (one voice of two of its
  sessions), whose every request is taken below; the consumer has no objection to the second revision. Written under the owner's standing rule (the most long-term correct structure, textbook in its
  foundations, cost ignored; decided, not asked). It is item 4 of the order agreed with a downstream consumer on
  2026-11-19, which the owner confirmed: the boundary binding and the lend contract, before ADR 0100's landing.
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
  - `docs/SCE_ACCEPTED_SUBSET.md`, the paragraphs of each of the above.
- Scope: the Rust lane (`std` and `no_std`) and the C11 lane, which are the two that generate a link and a
  buffer-pool (the kinds are MCU-class elsewhere); the door's ask and drive (D5) on every runtime; the build entry of
  another repository (D6).
- Related: ADR 0079 (a device core does no I/O), ADR 0082 (an algorithm writes into storage its caller holds), ADR 0086
  (a bound is part of a type), ADR 0090 (an event's origin is the reply address its processor writes), ADR 0100 §1.4
  (a codec's flag inputs), ADR 0102 §1.1 (a codec document is a message type whose value is its record view), ADR 0108
  D3/D4 (a run-time limit under a build bound; an empty map is all zero), ADR 0110 (the C11 core is freestanding).

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

## Decision

### D1. A machine imports a link kind; link instances are values at run time; each row is an event typed by its codec

A statechart imports a link as it imports any forge kind:

```xml
<sce:import kind="link" src="links/session_link.scxml" as="session_link"/>
```

The import names a **link kind**, fixed at build. Its **instances** exist at run time: the host opens one when its
driver dials or accepts, and the coupling (D2) gives it a `link` handle, a generational index (D4's form) of the
built-in type `link:<alias>`. A machine holds handles in its datamodel (a map from a peer to its links, ADR 0108),
compares them, and names one where it sends (D3) or closes (D3). A stale handle (its instance closed) is refused
where it is used, as D4's slot index is.

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
  link whose peer stops reading is ever closed; a link may declare `<sce:closed event="E"/>`, posted when the instance is closed by either
  side, after which its handle is stale.
- **A run-time frame limit.** An instance's slot length and its frame limit (the negotiated batch size, the path's
  MTU) are given when the host opens it, each at most the pool's build bounds (D4).

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
  stays the pool's own (C11: `<pool>_init_over(bytes, len, c, l)`). A deployment that sizes its arena by a negotiated
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
  or a C ABI. `pool.reclaim::<S>(index)` (C11: `<pool>_reclaim(index, state)`) rebuilds the handle if the slot is in
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

## Order of work

1. This record.
2. The pool (D4), Rust and C11: run-time count and length, `over`, the all-zero empty pool, the headroom,
   `SlotIndex` and `reclaim`, the `owner` and `shared` books, a slot lent to events, with the kind's conformance
   fixtures; the queue's run-time capacity.
3. The link's transmit side (D3): stages, open, append and arm with their outcomes, the take order, the lending
   `Link` operation, the prefix, `tx-room`, `tx-idle`, `link-close` in both forms and `closed`, on the loopback fixtures of
   `tests/forge/conformance/link_events/`, which gain a lending driver and a batching link.
4. A machine imports a link (D1): the grammar, link handles, record views as event types, `decode`, `each`, row
   inputs, and the refusals.
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
