# ADR 0147 — The one door of ADR 0084 is proved as a concurrent object: a promise, a protocol that refines it, every runtime's history linearizable to it, Rust's sequential core by Verus and the atomics by GenMC

- Status: Accepted (2026-11-26, by the owner), with ADR 0146, at the owner's order to review the plan against the
  most long-term correct structure (textbook, general, cost ignored), improve it and adopt it. The plan first put
  to a downstream consumer the same day proved the door on two lanes, Rust and C11. Every runtime has the door,
  and each is held to it by one judge, so this record covers every runtime.
- Date: 2026-11-26
- Amends ADR 0115: its Scope gains ADR 0084's door, the shared concurrency of every generated runtime and of the
  Interpreter. Its levels gain a history read against a promise, for a concurrent object, beside D4's trace read
  against a model, for a sequential one. ADR 0084 gains the line "Amended by: ADR 0147" when step 2 lands, and its
  conformance tests then write a history.
- Scope: `docs/formal` (two modules), `sce-build` (an inbox history checker beside the queue and broadcast-log
  ones), the inbox conformance test of every runtime, `backends/rust/runtime/src/inbox.rs` and the Rust engine's
  drain (a sequential core taken out), the GenMC lane (the bounded inboxes). A generated machine's output does not
  change.
- Related: ADR 0084 (D1, D2 as amended at steps 8 to 10), ADR 0115 (D1, D2, D5, D13), ADR 0146 (the judge), ADR 0062
  (the broadcast-log history), RFC §synth-5-P (GenMC and the queue history).
- Requested by: a downstream consumer, 2026-11-26. Its protocol logic moves into SCE documents, and the door is the
  only concurrency left in the code it runs.

## Context

A machine of SCXML is deterministic and single-threaded. ADR 0084 confines each engine to its owner thread. Every
input from another thread enters through one inbox per session, which the owner drains at a macrostep boundary and
before each event of its own enters its external queue. A tree keeps a directory, so an input that names a session
is posted to that session's inbox. Eight implementations of this exist: the Interpreter, the generated C++ engine,
Rust (`std` and bounded), C11, Kotlin, Go, Python and Swift. ADR 0084's step-9 and step-10 amendments were found
by runs (W3C test 201, `static_invoke_ending_order.scxml`), not by a model. Each runtime has an inbox conformance
test that holds it to those shapes. No statement of what the door promises exists apart from the prose of D2, and
nothing judges an implementation against such a statement.

For a downstream consumer whose protocol logic becomes generated machines, the door is the one place where a
schedule of threads can change an outcome. ADR 0115's levels now hold a machine (D7, D8) and a protocol of the host
core (D2, D13), but not this.

## The textbook structure

- **A concurrent object is specified by a sequential specification, and an implementation is correct when every
  history it produces is linearizable to it** (Herlihy and Wing 1990). A history records each operation's
  invocation and response. A checker decides whether some total order of the operations, consistent with their
  real-time order, is a run of the specification. This repository already judges its queue and its broadcast log
  this way, on every backend (RFC §synth-5-P, layer 2; ADR 0062, section 5).
- **The design is a protocol that refines the promise** (ADR 0115, D1 and D2). Each part of the protocol is shown to
  be needed by a run that takes it out.
- **A sequential core is proved, and the concurrency around it is judged by a model and by its memory model**
  (ADR 0115, D13; IronFleet).
- **The atomics of an implementation are judged under the language's memory model, by a stateless model checker
  over every execution the model allows** (GenMC; RFC §synth-5-P).

## Decision

### D1. The promise: `InboxService.tla`

What a host may rely on, with no mechanism. A tree of sessions, posters on any number of threads, one owner:

- **Taken or refused, when posted.** Whether a post is taken or refused is decided when it is posted. A refused post
  is handed back with its reason and never takes effect. The reasons are three. `unknown`: the tree does not hold
  the session the post names, having never held it or having let it go when its run ended where the runtime closes
  its inbox. `ended`: the session's run has ended and the runtime closes its inbox. `full`: the session's bounded
  inbox holds its capacity, counted as the inputs taken for the session and not yet in its queue. Any other post to
  a session the tree holds is taken. A post is refused as full only when that count is reached, and may still be
  taken then, because a drain frees room before what it took out reaches the queue. That an inbox never holds more
  than its capacity is D2's.
- **At most once, and never lost.** A post the inbox takes is applied exactly once, by the session it names, by the
  owner-thread operation of the same name.
- **Run to completion.** A posted input reaches a session only through that session's external queue, which the
  running macrostep does not read (ADR 0084, D1). The drain before an event of the owner's own runs inside a
  macrostep, and what it applies enters the queue and waits for the boundary.
- **Arrival order.** Each session's external queue takes the posts to that session in the order they linearized,
  against that session's own enqueues: an event of the owner's own enters behind every post taken before it (D2 as
  amended at steps 9 and 10).
- **The end.** What a session's inbox holds when its run ends is applied then, as a direct call made at that moment
  would be (an event counted as unseen, an ending refused by its token). A later post is refused where the runtime
  closes its inbox, and applied at the end of the owner's loop where it never closes.
- **Progress.** Under the owner's weak fairness and a waker that is called, every post that is taken is applied.

### D2. The protocol: `EngineInbox.tla`, refining `InboxService`

ADR 0084's mechanism: a multi-producer, single-consumer queue per session; the drain at the boundary and at the one
chokepoint of the external enqueue, one drain at a time; the directory, which a child enters before its first
macrostep and leaves after its inbox is closed, and which a post's session is looked up in when it is posted; a
bounded capacity, where a full inbox refuses; and the waker. Its parts, each with a run that takes it out and the
promise it breaks (ADR 0115, D5):

| Part | Taken out | Breaks |
|---|---|---|
| the drain before an event of the owner's own | step 9 undone | arrival order (W3C test 201's shape) |
| the address resolved when posted | step 10 undone | arrival order of a child's queue |
| one drain at a time | a drain inside a drain applies | arrival order |
| the inbox applied when a run ends | what it holds is left | never lost |
| the directory entered before the first macrostep | entered after it | refused wrongly, or lost |
| the waker | a post wakes nobody | progress |
| the directory looked up when posted | a post naming a session the tree does not hold is taken, into no inbox | never lost, and the refusal of an unknown session |
| a full inbox refuses | it takes past its capacity | no promise: the bound (`Bounded`), which is what an inbox's memory is promised |

Its bounds are those of a configuration (posters, sessions, a session the tree does not hold, capacity, inputs). The
inductive invariant of at most once and arrival order is the one ADR 0115 D3 proves for every size once Apalache is
approved.

### D3. Every runtime's history is linearizable to the promise

An **inbox history** is a wire surface registered as the queue history is
(`tests/forge/conformance/inbox_history.schema.json`). It says what the run was given: the sessions the tree holds
(`sessions`), whether the runtime closes a session's inbox when its run ends (`closes`), and the capacity of a
bounded inbox (`capacity`, left out where the inbox has no bound). It records, per thread, the operations in program
order, each with the two readings of one shared clock it was invoked and returned at (`invoked`, `returned`):

- a poster's `post(session, input)`, whose `outcome` is `taken`, or `refused` with its `reason`: `full`, `ended` or
  `unknown`;
- the owner's `enqueue(session, item, source)`, recorded at the instant `item` entered the session's external queue.
  Its `source` is `posted` for an input a post took, applied from the inbox, and `item` is that input. It is `own`
  for an event of the owner's own, and `item` is the owner's name for it;
- the owner's `end(session)`, the end of that session's run.

A drain is recorded as the `enqueue` of each input it applies. No macrostep boundary is recorded: run to completion
is the external queue's (D1), and the history says where each item entered that queue.

`sce-codegen check-inbox-history` decides whether the history is linearizable to D1's sequential reading. It shares
the search of the queue and broadcast-log checkers, and only the specification differs. Each runtime's inbox
conformance test writes a history of its stress run. The test is judged by the checker on every lane: the
Interpreter, the generated C++ engine, Rust (`std` and bounded), C11, Kotlin, Go, Python and Swift. A runtime that
cannot run threads in its tests (bare-metal C11 and Rust) posts from interrupts in its emulated target and writes the
same history. A corpus of histories, each named by what it must be judged (linearizable or not), holds the checker
itself, as `sce-build/tests/fixtures/queue_histories/` holds the queue's.

### D4. Rust's sequential core is proved by Verus

The decisions of the Rust drain are taken out of `Engine` and `inbox.rs` into a value with a transition function,
as ADR 0115 D13 asks of the state a lock guards. The core holds what is held per session, whether a drain is under
way, the directory and whether the run ended. Verus proves that its steps refine D2's owner-side actions: an input
applied once and in order, only to the external queue, none left at the end. `EngineInbox` and `BoundedInbox` keep the
channel and the ring. The core decides, and the shell moves inputs.

### D5. The atomics are judged by GenMC

The bounded inboxes (C11's ring and directory slots under their sequence stamp, Rust's `BoundedInbox`) and the
C++ LSCQ inbox are judged under the C11 memory model by the GenMC lane that already judges the forge queues
(`.github/workflows/genmc-queue.yml`): each with a test of two posters and the owner, and one per refusal (full,
session gone, slot restarted). Rust's `std` inbox rests on the standard library's channel and `Mutex`. They are
assumed, and the assumption is named in the module (ADR 0115, D6).

## Consequences

- What the door promises is written once, in a module, and every implementation is judged against it by one checker.
- The step-9 and step-10 faults become runs a model must find when their part is taken out.
- Each runtime's conformance test keeps its shapes and also writes a history. A history the checker cannot
  linearize is a defect of the runtime, the checker or the promise, and which of them is settled before any is
  edited.

## Alternatives rejected

- **Prove the door on the lanes a consumer uses first.** The door is one rule with eight implementations. One judge
  costs the same for eight as for two, and a lane left out is one whose conformance stays a set of shapes.
- **Read each runtime's run as a TLA+ trace (ADR 0115, D4).** A trace orders events as they were logged, and logging
  from several threads either orders them by a lock, which changes the schedule, or leaves the order unknown. A
  history with invocation and response times is the textbook record of a concurrent object.
- **Verus over the channel and the ring.** Verus reads one thread. The memory model is GenMC's to judge.

## Order of work

After ADR 0146's steps 1 to 3. Each step lands with a test that fails without it.

1. `InboxService.tla` and `EngineInbox.tla`, with the parts of D2 and their runs, judged by `sce-codegen formal
   models`.
2. The inbox history, its schema and checker, its corpus. Then the Rust and C11 conformance tests write one,
   then the rest.
3. Rust's sequential core (D4), with its proof and mutants in `proofs.json`.
4. GenMC tests of the bounded inboxes and the LSCQ inbox (D5).

## Sources

Herlihy and Wing 1990 (*Linearizability: a correctness condition for concurrent objects*); Hawblitzel et al., SOSP
2015 (IronFleet); Kokologiannakis et al., PLDI 2019 (GenMC); ADR 0084 and RFC §synth-5-P of this repository.
