# ADR 0111 — A machine's bound events cross its boundary typed, through a link that lends its slots and a door that can be waited on

- Status: Proposed (2026-11-22). Written under the owner's standing rule (the most long-term correct structure,
  textbook in its foundations, cost ignored; decided, not asked). It is item 4 of the order agreed with a downstream
  consumer on 2026-11-19, which the owner confirmed: the boundary binding and the lend contract, before ADR 0100's
  landing. Accepted once the consumer's review of it ends.
- Date: 2026-11-22
- Amends, once landed (each amended document gains an "Amended by: ADR 0111" line in the landing that moves it):
  - ADR 0084 D2, by an input that is posted and waited for (D5);
  - ADR 0106 #142, by a link a machine imports and the lending of its transmit slots (D1, D3);
  - ADR 0110 D2 item 2, by the JSON reader and writer leaving the C11 core for a separate unit (D2);
  - RFC §synth-5-E's buffer-pool, by a run-time slot count, a headroom, a slot index that crosses an interface and a
    shared book (D4);
  - the `queue` kind, by a run-time capacity under its bound (D4);
  - `docs/SCE_ACCEPTED_SUBSET.md`, the paragraphs of each of the above.
- Scope: the Rust and C11 lanes, which are the two that generate a link and a buffer-pool (the kinds are MCU-class
  elsewhere); the door's wait (D5) on every runtime; the build entry of another repository (D6).
- Related: ADR 0079 (a device core does no I/O), ADR 0082 (an algorithm writes into storage its caller holds), ADR 0086
  (a bound is part of a type), ADR 0102 §1.1 (a codec document is a message type whose value is its record view), ADR
  0108 D3/D4 (a run-time limit under a build bound), ADR 0110 (the C11 core is freestanding).

## Context

A consumer is moving a protocol's whole logic into SCE documents, generated for Rust and for C11 from one source set,
and wants the core to be sans-I/O: a deterministic, single-threaded set of machines that performs no I/O, reads no
clock of its own, and sees only typed events. Bytes are decoded at the boundary before a machine sees them, and a
machine's output is encoded on the way out. Concurrency lives only in the seam, through ADR 0084's inbox and door.

What exists, checked at `7f52c3d78ee`:

- **A link already decodes and encodes.** ADR 0106 #142 generates, for Rust and C11, a link's receive path (`pump`:
  poll the driver, decode each frame with the framer, call `inject` for every `<sce:inbound>` row whose `when`
  holds) and its transmit path (`send`: encode an `<sce:outbound>` event's value into a transmit-pool slot or a
  buffer, hand the bytes to `Link::tx`, return the slot).
- **Nothing couples it to a machine.** No statechart imports a link or a codec (ADR 0110 says so). `inject` hands
  the host a value borrowed for the call; the host must convert it and raise it itself. A machine's `<send>` of an
  outbound event has no processor that reaches the link.
- **The C11 core carries event data as JSON text.** A `<send>`'s params, done data and a child's events are written
  as text and read back (`wire.h`, `event_payload.h`); only the per-event `_raise_<event>_typed` entry fills a
  typed payload directly. ADR 0110 left making them typed to this record.
- **The transmit path copies.** `Link::tx(TxFrame<&[u8]>)` copies the bytes before it returns, so the slot is
  returned at once. A link that sends from the slot itself, as a DMA engine or a zero-copy stack does, cannot be
  generated. The consumer's links lend their slots in place, so its two link documents generate nothing (its W5).
- **The buffer-pool cannot be lent.** The consumer's transmit lend book is hand-written (its W4) because the kind
  cannot state four things:
  - a slot count fixed at run time under a build bound;
  - a headroom in front of the frame, into which the link writes its own prefix (a stream's length) so that prefix
    and frame leave in one write;
  - a slot index that crosses an interface (a trait object, a C ABI) and a typed handle rebuilt from it;
  - access through a shared reference.
  The generated pool has `SLOT_COUNT` and `SLOT_SIZE` constants, takes `&mut self` for every transition, and
  hides the index behind a typestate handle (`Slot<CpuMut>`).
- **A post answers only whether the inbox took it.** The consumer's C ABI must keep one upstream behaviour: a put
  made inside a subscriber callback, on the owner's own thread, has completed its delivery to the same process before
  the put returns (the consumer's measurement R3069). Other calls of that ABI need their effect applied before they
  return, from any thread. ADR 0084 has no way to wait for an input to be applied.
- **Another repository builds by linking `sce_build`.** The consumer's build tool calls the library in process.
  This repository's own build scripts do so too, but the library's API is not a published interface. A consumer needs
  the generator's version and diagnostics on a surface that is stable.

## The textbook structure

- **Parse at the boundary; the core sees only typed values.** Sans-I/O protocol design separates the protocol's state
  machine from its I/O (the h11 and QUIC stacks' "bring your own I/O"; Brett Cannon's and Cory Benfield's sans-I/O
  notes). The parser runs where the bytes arrive, and the machine receives events of a type (Parnas's information
  hiding: the wire format is a secret of the boundary module). A malformed input is a typed refusal at the boundary,
  never a partially read value inside.
- **A message type is declared once.** ADR 0102 §1.1: a codec document defines its value (DFDL's infoset), and its
  record view is derived from it. A second declaration of the same fields beside it would be a second definition.
- **An I/O processor carries a `<send>`.** §scxml-6.2: `type` names the Event I/O Processor and `target` the
  destination; a send that cannot be dispatched places `error.communication` on the internal queue. A link is an
  Event I/O Processor whose wire format is its codecs.
- **Zero-copy transmit is buffer lending.** A network stack lends its buffers to the producer and takes them back at
  completion (BSD `mbuf` with its leading space for headers, Linux `sk_buff` with `skb_reserve` headroom and
  `skb_push`, DPDK `rte_mbuf` headroom). The producer writes the payload after the headroom; each layer below
  prepends its header into the headroom; the buffer leaves in one write.
- **A handle that crosses an interface is a generational index.** An index alone is reused once its slot is freed (the
  ABA problem); an index paired with a generation that changes on each reuse is refused when stale (the slot-map
  pattern; Rust's `generational-arena`, game engines' entity handles).
- **A caller that needs an effect applied asks and waits for a reply.** The actor model's request-reply ("ask")
  returns a future the caller waits on, and the actor itself never blocks (Hewitt; Akka's `ask`, Erlang's
  `gen_server:call`). A wait has a timeout; `gen_server:call` from the server's own process deadlocks, which Erlang
  documents and which a design must rule out.
- **A tool's published interface is its command line.** A build calls a code generator as a program with versioned
  arguments and machine-readable diagnostics (`protoc` and its plugins, `flatc`, `bindgen`'s CLI), wrapped by a
  build system's module (CMake's `FindProtobuf` `protobuf_generate`) and a Cargo build-script helper
  (`prost-build`, `tonic-build` run `protoc`). Linking the generator's library makes every internal type a
  dependency of the consumer.

## Decision

### D1. A machine imports a link, and each of the link's event rows is an event of the machine, typed by its codec

A statechart imports a link as it imports any forge kind:

```xml
<sce:import kind="link" src="links/session_link.scxml" as="session_link"/>
```

Each `<sce:inbound>` row of the link is an external event the machine may receive, and each `<sce:outbound>` row an
event it may send through the link. The event's data is the record view (ADR 0102 §1.1) of the row's codec:

- an `<sce:outbound event encode="C">` row's codec is `C`, as today;
- an `<sce:inbound>` row gains `decode="C"`, the codec whose value its event carries, read from the frame's bytes; a
  row that names none carries the framer's value, as today. `when` still reads the framer's decoded frame, so a row
  is chosen by the frame's header and its event typed by its body. A `decode` codec the link does not import, or
  whose decode takes caller inputs, is refused at generation, as an `encode` codec is.

The machine declares nothing more. The record view is derived, so the machine's receive-side and send-side checks
(`forge::event_schema_check::check`, `check_send_side`) read it as they read an imported event schema, and
`sce:interface="closed"` holds a machine to its links' events as to its schemas. A machine that also imports an event
schema for the same event name is refused, by a diagnostic code of its own: one name, one type.

A row's codec must have a type of the profile for every entry (ADR 0102 §1.1's rule for a message type): a codec whose
value cannot be read from octets alone, or with an entry of no type, is refused by name. A record that holds a list or
bytes is ADR 0086's bounded field, so the record view of a frame with a variable body is a record with a bounded
`bytes` field. Nothing further is decided about records here.

A link is MCU-class (RFC §synth-5-E). Importing one into a machine generated for another lane is refused as a link
document is refused there (`codegen/mcu-class-kind-on-non-mcu-language`).

### D2. A received frame is decoded where it arrives and posted through the door as a typed event

For each link a machine imports, the generator writes a **receive coupling** beside the machine: `pump_into` (Rust)
and `<link>_pump_into` (C11). It runs the link's receive path and, for each event a row raises, builds the event's
owned value from the decoded one and posts it to the machine through ADR 0084's door as a typed external event.
Decoding happens on the thread that runs the pump, before the inbox; the owner only ever takes a typed value.

- **The inbox carries a typed event.** Rust's `InboxInput` gains a typed arm (`InboxInput::external_typed(event,
  payload)`, the payload the machine's `P::Payload`), applied by `raise_external_typed`, the owner operation of
  the same name. C11's `<m>_inbox_input_t` holds the event's payload union, as `_raise_<event>_typed` takes it. A
  bounded inbox's slot is sized by the largest payload of an event the machine can receive, a number the generator
  knows; a payload too large for the slot cannot be declared.
- **A refused frame is a typed refusal at the boundary.** A frame its framer or its row's codec refuses raises no
  event of the row. It is counted by the link and handed to the `refused` callback, as today. A link may declare
  `<sce:refused event="E"/>`, and then each refusal is also posted as `E`, whose data is the standard record
  `sce:std/codec/refusal.scxml` (the codec's refusal reason as its enum, the byte offset where it was found, and the
  row it was read for, or none for the framer). A protocol that must answer a malformed frame (by closing its
  session) does so in its machine, and the refusal reaches it as data, never as a partial value.
- **A post the inbox refuses is the link's backpressure.** Under `drop` the frame is counted and dropped; under
  `block` the pump stops and leaves the rest in the driver until the next pump; under `signal-event` it is counted
  and the link's backpressure event is posted when the inbox has room again. The three are ADR 0106's policies; the
  coupling applies them to the inbox instead of to the host's callback.
- **Every event that has a type is carried typed inside the core.** A `sce-static` machine carries the data of an
  event whose name has a type (an imported event schema, a link row's record view, a child's done data typed by its
  schema) as that typed value through every position: the external and internal queues, a `<send>` to itself, its
  parent or a child it runs, and a child's done data. Text is written only at a boundary whose format is text: the
  BasicHTTP processor, saved state's text form, the Interpreter's ECMAScript data model, the conformance trace. On
  C11, `wire.h` and `event_payload.h` therefore leave the core and become a separate unit (ADR 0110 D2's item 2),
  which a machine with an event that carries untyped data links and states as a requirement (`payload-text`); a
  machine whose every event with data is typed links no JSON reader. The c11-mcu gate holds this with a fixture whose
  events are all typed and whose image is refused if it links the unit.

### D3. A bound `<send>` is encoded into a slot the link lends, and the link takes the slot to send it

A machine sends an outbound row's event through the link it imported:

```xml
<send type="sce:link" target="session_link" event="session.frame.send">
  <param name="seq" expr="next_seq"/> ...
</send>
```

`type="sce:link"` is the Event I/O Processor of an imported link, and `target` names the link's import alias,
written as a literal (`targetexpr` with `sce:link` is refused: the link is fixed at build). The send-side check reads
the row's record view. An event no outbound row of that link declares is refused by name.

The send is executed when the `<send>` is, inside the macrostep, and performs no I/O. It does what a network stack's
producer does with a lent buffer:

1. **Acquire.** The link lends a free slot of its transmit pool, with the link's headroom (D4) left in front.
2. **Encode in place.** The row's codec encodes the event's value into the slot, after the headroom.
3. **Arm.** The slot moves to the pool's `dma-armed-tx` state (RFC §synth-5-E) with the encoded length. The slot now
   belongs to the link; the machine's macrostep goes on.

The driver sends armed slots when the host next drives it (`poll`), outside the macrostep: the link takes each armed
slot, writes its own prefix into the headroom if it has one, hands prefix and frame to the driver as one contiguous
range, and the slot returns to the pool when the driver reports the send done (`tx_complete`, then
`pool_return`). A driver that copies (today's `Link::tx`) reports it done at once; a driver that lends (DMA, a
zero-copy stack) reports it when the hardware has read the bytes. `sce_link_runtime::Link` gains the lending
transmit operation beside the copying one, and the generated link uses the lending one whenever the link names a
transmit pool.

A send that cannot be dispatched places `error.communication` on the internal queue (§scxml-6.2), with
`_event.data.reason` the structured reason of ADR 0106 #111: `pool-exhausted` (no free slot), `encode-refused` (the
value is outside what the codec can write, with the codec's own reason). A failure the driver reports later is not
the send's: it is counted by the link and, when the link declares `<sce:tx-failed event="E"/>`, posted through the
door as `E` with the same structured reason. A link without a transmit pool encodes into a buffer of the largest
encoding and hands it to the copying operation, as today; its send cannot be exhausted.

A link a machine imports and whose backpressure is `block` is refused at generation for its transmit side: a
macrostep cannot wait for a slot. The receive side keeps `block` (D2).

### D4. A pool is lent: a run-time count, a headroom, a generational slot index and a shared book

The buffer-pool kind keeps its seven-state ownership machine (RFC §synth-5-E); what a holder of a slot may do is
unchanged. Four things are added, and the generated Rust and C11 pools implement them.

- **A run-time slot count under the build bound.** `<sce:slot-count>` is the bound `B`: the storage reserved at
  build (ADR 0086). A deployment constructs the pool with a capacity `c`, `1 <= c <= B`
  (`Pool::with_capacity(c)`, a `const fn`; C11 `<pool>_init(c)`), and only the first `c` slots are ever lent.
  A `c` outside that range is refused at construction (`Err`; `false`). `Pool::new()` is `with_capacity(B)`.
  This is U16's rule, the rule ADR 0108 D3/D4 gave a map's run-time limit: the type holds the bound, the value holds
  the limit, and the storage never grows.
- **A headroom.** A link names the headroom its framing needs on its pool reference, `<sce:tx-pool ref="p"
  headroom="2"/>` (none for a datagram, the length prefix's width for a stream). A headroom not smaller than the
  slot size is refused. The encoder writes at the headroom's end; on taking the slot the link writes its prefix,
  at most the headroom, just in front of the frame and hands the two as one range. A prefix longer than the headroom
  is refused when the link is generated where its width is known, and by the take (`None`, counted) where it is not.
- **A slot index that crosses an interface.** A lent slot has a `SlotIndex`: the slot's number and its generation,
  a `u32` each, `Copy`, `repr(C)` in Rust and a plain struct in C11. The generation is incremented each time the
  slot returns to `free`. Generated code holds the typestate handle (`Slot<S>`); the index is what crosses a trait
  object or a C ABI. `pool.reclaim::<S>(index)` (C11: `<pool>_reclaim(index, state)`) rebuilds the handle if the
  slot is in state `S` and its generation is the index's, and refuses otherwise: a stale or forged index is counted
  as an ownership violation (`pool/ownership-violation`) and yields nothing. `Slot::idx()` becomes `Slot::index()`
  and returns the `SlotIndex`.
- **A shared book.** Every transition takes `&self`; the pool's state book is interior-mutable. A pool declares how
  it is shared, `<sce:sharing>owner</sce:sharing>` (the default) or `shared`:
  - `owner`: one thread uses the pool, the owner of the machine that lends it. The book is `Cell`s, the pool is
    `!Sync`, and a transition is a load and a store. The consumer's cooperative engine is this case.
  - `shared`: the owner and a driver context (another thread, an interrupt) both move slots, as a DMA completion
    handler does. The book is atomic: each transition is a compare-and-swap from the state it leaves, so two contexts
    can never both take one slot. On a target with no compare-and-swap (ARMv6-M), the transition runs inside the
    deployment's critical section (`critical-section` in Rust, the seam `_sce_critical_enter` and
    `_sce_critical_exit` in C11), which the generated header declares exactly when the pool is `shared`.
  The hardware-owned edges stay `unsafe` in Rust, since the caller asserts what the hardware did; under `shared`
  they are safe to call from the driver context.

The `queue` kind takes the run-time capacity by the same rule: `CAPACITY` is the bound, and a queue is
constructed with `c <= CAPACITY`, refused otherwise. Its full policy is unchanged (U17 is a record of its own).

### D5. A post may be waited for until it is applied, and the owner's own wait is applied before it returns

ADR 0084 D2 is amended: the door gains a second kind of post, an **ask**, which returns a completion the caller may
wait on. A plain post is unchanged.

- **What completes.** An asked input is applied when the owner has applied it by its owner-thread operation (D2 of
  0084) and the macrostep that took it has run to completion: an event's macrostep, or for a host-invoke ending the
  macrostep that took its `done.invoke` or `error.invoke`. The completion then carries the outcome: `processed`,
  `unseen` (the run had ended, §scxml-3.13), or `refused` (the token check refused an ending). An input the inbox
  refuses is refused at the ask, as at a post, and has no completion.
- **Another thread waits.** The posting thread waits; the owner never does, and no macrostep waits for anything.
  Rust `std`: `EngineInbox::ask(input) -> Result<Completion, InboxRefusal>`, and `Completion::wait(deadline)`
  blocks on it. `no_std` and C11: the completion is a slot of a table the deployment sizes
  (`SCE_MAX_ASKS`, a const parameter of `BoundedInbox`), polled (`is_done`) or signalled through a callback the
  ask names; a full table refuses the ask. An interrupt handler may not ask, only post: it preempts the owner it
  would wait for. The C11 `ask` from interrupt context is refused by the same check that reads the context.
- **A wait is bounded, and an ask can be withdrawn until it is taken.** `wait` takes a deadline and answers
  `timed-out` when it passes; the input stays posted and is still applied. `cancel` withdraws the input if the
  owner has not drained it yet and answers whether it did; once drained, it can only stop the waiting.
- **The owner's own ask is applied before it returns.** An ask made on the owner thread between macrosteps (in a
  callback the host runs on a step's output, which is where a subscriber callback runs in a sans-I/O core) is applied
  at once: the owner applies what its inbox holds first, in order, then the asked input, and runs macrosteps until
  the asked input's macrostep has completed; then the ask returns its completion, already done. This keeps R3069: a
  put made in a subscriber callback has completed its delivery before the put returns. Callbacks run by those
  macrosteps may ask again; the nesting is bounded by `SCE_MAX_ASK_DEPTH` (Rust: a const parameter; default 4),
  and an ask past it is refused (`depth`). An ask made on the owner thread inside a macrostep (from executable
  content, through a host processor's handler) is refused (`inside-macrostep`): applying it there would observe an
  input in the middle of a step, which ADR 0084 D1 forbids, and waiting for it would deadlock, which is the
  `gen_server:call` error. A document that needs that order raises or sends the event itself.
- **Every runtime.** ADR 0084 holds one threading rule for every engine, so every runtime gains the ask with the
  same outcomes. Rust and C11 first, since the consumer needs them; the others in the order 0084's step list used.

### D6. Another repository builds through the `sce-codegen` command line, a CMake module and a build-script helper

The supported way for a build outside this repository to generate code is the `sce-codegen` program:

- **The command line is the interface.** Its subcommands and flags, `--error-format=json` diagnostics
  (SCE_ERROR_CONTRACT.md), `--plan` for the files a run writes, and `--write-deps` for the documents it read.
  `sce-codegen --version` prints the generator's revision and an interface version, which changes when a flag a
  consumer relies on changes. `sce_build`'s library API is not an interface for another repository, and nothing
  promises it.
- **C and C++: the CMake module.** `cmake/SCECodegen.cmake` and `SCECodegenPlan.cmake`, installed with
  `find_package(SCE)`, are that interface. They gain one function for forge documents of every kind,
  `sce_add_forge(TARGET <t> DOCUMENTS <files> LANGUAGE c11|cpp [WITH_IMPORTS])`, declared from the plan as every
  generation is (`sce_add_algorithm` becomes a call of it). A program links what the plan says the generated files
  require (`sce_c_link_runtime`).
- **Rust: a build-script helper crate.** `sce-codegen-build`, a build dependency, finds the program (`SCE_CODEGEN`,
  else `PATH`), refuses one whose interface version it does not know, runs it with `--plan` and `--write-deps`,
  prints `cargo:rerun-if-changed` for every document read, and writes the module declarations of what it generated.
  It is the one implementation of ADR 0097's build-script flow: `sce_build::action::build_script` keeps this
  repository's own build scripts and becomes a client of the same steps, so the two cannot drift.

The consumer's build tool, which links `sce_build`, moves to this entry after it lands; that move is the consumer's.

## Consequences

- **A protocol machine sees only typed events.** Bytes stop at the link, a malformed frame reaches a machine only as a
  typed refusal it declared, and a machine's output leaves through its link with no I/O inside a macrostep.
- **A C11 machine whose events are all typed links no JSON reader**, so the freestanding image ADR 0110 weighs loses
  the text payload code. A machine with untyped event data states the `payload-text` requirement and links it.
- **A generated link sends from its pool in place**, so a DMA or zero-copy driver can be generated, and the consumer's
  hand-written lend book and its two link documents become generated code.
- **The pool's API changes** (`&self`, `with_capacity`, `SlotIndex`, `reclaim`); a consumer that drives a generated
  pool by hand follows `docs/SCE_CODEC_CONSUMER_MIGRATION.md`'s section.
- **A caller can wait for an effect** without the core giving up its single owner, and the owner's own wait cannot
  deadlock: it is applied or refused.
- **Another repository builds from a program, not a library**, so the generator's internals stay free to change.

## Alternatives rejected

1. **Decode on the owner, at the drain.** The inbox would carry bytes, so a bounded inbox would hold the largest frame
   per slot, the owner would parse inside its loop, and a malformed frame would be found after the poster had moved
   on. Decoding where the bytes arrive refuses at the boundary and posts what the machine reads.
2. **A step that calls a codec (the consumer's first request, U4).** It puts the wire format inside the machine, so
   every machine that reads a frame repeats its parse, and a malformed frame is a failure inside a step. The boundary
   owns the format; the machine owns the protocol.
3. **An event schema written beside each codec.** It states every field twice and needs a check that the two agree.
   ADR 0102 §1.1 rejected it for the same reason; the record view is derived.
4. **Encode in the host after the macrostep.** The host would need the codec and the event's value outside the
   machine, and a send's failure (no slot, a value the codec cannot write) could no longer be the send's
   `error.communication`. Encoding into a lent slot inside the send costs no I/O and keeps the failure where
   §scxml-6.2 puts it.
5. **A plain slot index across the interface.** It is reused once its slot returns, and a stale holder would write
   into someone else's frame. The generation makes the stale index refusable.
6. **Always atomic, or always `&mut`.** `&mut` cannot be shared with the link that holds a slot while the session
   acquires another, which is the consumer's batching case. Always atomic costs a compare-and-swap per transition on a
   single-threaded engine that needs none, and on ARMv6-M a critical section. The pool states its sharing.
7. **Making the caller the owner for the call (a synchronous entry for any thread).** It ends the single owner that
   ADR 0084 rests on; two threads would run macrosteps of one machine.
8. **Applying the owner's own ask later, as an internal event.** It would lose R3069's order: the put would return
   before its delivery.
9. **Publishing `sce_build`'s API.** Every internal type would become a consumer's dependency, and a change of the
   generator's structure a breaking change of another repository's build.

## Order of work

1. This record, reviewed by the consumer.
2. The pool (D4), Rust and C11: `with_capacity`, the headroom, `SlotIndex` and `reclaim`, the `owner` and
   `shared` books, with the conformance fixtures of the kind; the queue's run-time capacity.
3. The link's lending transmit path (D3's arm and take, `Link`'s lending operation, the headroom and prefix), on the
   loopback fixtures of `tests/forge/conformance/link_events/`, which gain a lending driver.
4. A machine imports a link (D1): the grammar, the import, the record views as event types, the `decode` attribute,
   `<sce:refused>` and `<sce:tx-failed>`, and the refusals.
5. The receive coupling and the typed inbox input (D2), and `<send type="sce:link">` (D3), Rust and C11, held end to
   end by a machine that talks to itself over a loopback link in both lanes, and the same scenario's trace compared
   between them.
6. Typed carriage inside the C11 core (D2's last item): `wire.h` and `event_payload.h` into the `payload-text`
   unit; the c11-mcu gate gains the all-typed fixture.
7. The ask (D5): Rust and C11, then the other runtimes; ARCHITECTURE.md's threading section states it once.
8. The build entry (D6): `sce-codegen --version`'s interface version, `sce_add_forge`, and the
   `sce-codegen-build` crate, with this repository's build scripts moved onto its steps.

## Open questions

- None for this record. U17 (a full queue's policy) reopens ADR 0062 in a record of its own.
