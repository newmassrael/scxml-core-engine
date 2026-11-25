# ADR 0132 — A delayed-send queue is sized by what its document can leave waiting

- Status: Accepted (2026-11-25), under the owner's rule for this work: the most long-term correct, textbook
  structure, cost ignored.
- Date: 2026-11-25
- Amends: ADR 0075 (a machine keeps a delayed-send queue only when its document can fill it), and the runtime's
  `MAX_SCHEDULED_EVENTS`, which stops being the capacity of any generated machine.
- Related: ADR 0076 (the internal queue's depth is proven from the document; this decision is its counterpart for the
  delayed-send queue), ADR 0057 (a node present in some builds), ADR 0131 (the engine's state apart from the policy).
- Scope: `sce-build/src/scheduler_bound.rs`, `sce-build/src/bound_reading.rs`, `sce-build/src/generator.rs`,
  `sce-build/src/parser.rs`, `sce-build/src/model.rs`, `tools/codegen/templates/rust/state_machine.rs.jinja2`,
  `backends/rust/runtime` (`engine.rs`, `scheduler_storage.rs`, `policy.rs`, `saved_state.rs`, `lib.rs`),
  `schemas/sce-forge-ext.xsd`, the diagnostic code `codegen/no-std-scheduler-unbounded`.

## Context

A downstream bare-metal consumer generates a `no_std`, allocator-free Rust machine for a session lifecycle: each
atomic state arms a deadline with a delayed `<send>` in its `<onentry>` and cancels it by the same id in its
`<onexit>`, and the compound state around them arms an overall limit the same way. At most two of those deadlines
can wait at once. The engine's `PullScheduler` kept its no_std entries in a `heapless::Vec` of
`MAX_SCHEDULED_EVENTS` = 32 for every machine, about 32 bytes an entry, so each engine paid about a kilobyte of RAM
for slots its document can never fill.

Thirty-two was a guess, and so is any number a document is not held to. A send armed on a full ring stops the
program rather than lose the transition it would take (§scxml-6.2 has the processor deliver what it was asked to
deliver), so the capacity is a claim about every run: an undercount stops a machine in the field, an overcount is RAM
nothing can use. ADR 0076 answered the same question for the internal queue by proving its depth from the document,
and the delayed-send queue is open to the same treatment: every entry is armed by a construct the document writes or
by the engine on its behalf.

## Decision

### 1. What arms an entry, and what takes one away

An entry is armed by:

- each run of a `<send>` that waits (`Action::delays`: a `delay` other than none, or a `delayexpr`, written or
  lowered; §scxml-6.2.4). One entry per run, whatever its target: the hosted profile keeps a routed send, a
  host-served send and a send to `#_internal` in the same queue, and counting them alike is an overcount only for the
  bare-metal `#_internal` case, which raises at once;
- each start of a host-run `<invoke>` that names a deadline (`_sce_deadline_ms`), one at a time per element; the
  engine drops the deadline when the start ends, completed or cancelled (§scxml-6.4).

A send armed under the id of one already waiting is a second entry: `PullScheduler::schedule_event_as` replaces
nothing, and §scxml-6.2 does not make an id a slot. An entry leaves when it comes due and is delivered, when a
`<cancel>` names its id (§scxml-6.3; `PullScheduler::cancel_event` drops every entry of that id), and when a run
starts again (decision 4).

### 2. The bound the generator proves

The entries are partitioned by the site that armed them. A site arms at most one entry each time its block runs,
times the passes of each `<foreach>` around it (the bound of the `list<T>[N]` it walks, read as ADR 0076 reads it).
Its entries are bounded when, between two runs of its block, something takes every entry of the earlier run away:

1. A send in an entry-time block of a state (its `<onentry>`, its initial transition's content, the default content
   of its initial `<history>`), whose id an `<onexit>` of the same state cancels. A state is exited before it is
   entered again, its `<onexit>` runs whenever it is exited, by a transition out of it or out of an ancestor and when
   the run ends (§scxml-D-exitInterpreter), and exit content runs before entry content (§scxml-3.13).
2. A send in an `<onexit>` of a state, whose id an `<onentry>` of the same state cancels.
3. A send in a transition's content, whose id an `<onexit>` of a state the transition targets cancels. A target is
   entered by the transition, and if it is active when the transition is taken again it is exited first, since the
   transition's domain is a proper ancestor of each of its targets (§scxml-D-getTransitionDomain). A `<history>`
   target does not qualify: it enters what it holds, and its parent may stay active.
4. A send in an entry-time block or an `<onexit>` of a state entered at most once in a run. A transition enters only
   states in the configuration together with its targets after it, so a state is entered again only where one of
   its targets (a `<history>` read as its parent) can be active with it: an ancestor, a descendant, or a state of
   another region of a common `<parallel>`.

A `<cancel>` counts only where it surely runs: it names the id as written (`sendid`, equal to the `<send id>`, or
to the id the parser names a send without one by), it is present in every build its send is present in (ADR 0057:
each term of the send's canonical condition holds every feature of some term of the cancel's), and no statement
before it in its block can end the block with an error (§scxml-4.9). What cannot end a block is read conservatively:
a `<raise>`, a host call, a `<cancel>` of an id that cannot fail, and a statement the static lowering wrote and
marked as one that cannot fail. A send whose id is generated as it runs (`idlocation`) is cancelled through a
variable, which this analysis does not follow; only rule 4 bounds it.

The sites of rules 1 and 3 are not added up across the document. An entry of rule 1 waits only while its state is
active: it is armed after the state is entered and is gone before the state leaves. One of rule 3 is armed after the
transition's exits and before its target is entered, and is gone when the target leaves. So within a microstep every
such entry waiting belongs to a state of the configuration before the microstep (while its exits run) or after it
(from its transition content on), and between microsteps to a state of the configuration. They count by the legal
configuration in which the most wait: a compound state's children one at a time, a `<parallel>`'s regions together,
one top-level state (§scxml-3.3). Every other site, and every invocation deadline, counts whatever is active. The
bound is that configuration's sum plus the rest.

Anything else is not proven: a delayed send no rule bounds, one in a `<foreach>` over a list whose bound the document
does not state, one in a `<history>`'s default transition (which runs again while its parent stays active and holds
no history yet), a `<finalize>` that may send (it is kept as text and runs for every event of its child, §scxml-6.5),
and a bound past 65,535. The analysis is `SchedulerBound::of` in `sce-build/src/scheduler_bound.rs`, over the model
as the static lowering leaves it, since only then is it known which statement can fail. What it reads alike with
ADR 0076's analysis (which states can be active together, the places a `<foreach>` walks, a basis in lines of a
comment) is one module, `sce-build/src/bound_reading.rs`.

For the consumer's session the bound is 2: the limit of the compound state, and one deadline of whichever child is
active.

### 3. A capacity the document declares, and the refusal

A document whose delayed sends the proof does not bound declares the capacity on its `<scxml>`:
`sce:scheduler-capacity="N"`, spelled and read as `sce:capacity` is for the external queue (a whole number of at
least one; anything else is `validation/attribute-rule-violated`). It is carried by the forge AST, the XSD and the
pseudocode page (`scheduler:`). A proven capacity is used even where one is declared, as ADR 0076 uses a proven
depth: a smaller declared one stops the program, a larger one is capacity nothing can fill.

A bare-metal build (`generate -l rust --no-std`) of a document whose queue has neither a proven nor a declared
capacity is refused with `codegen/no-std-scheduler-unbounded`, located on the send the proof could not bound and
naming why. There is no fallback to a constant. The hosted build of the same document keeps its growing queue.

### 4. The runtime

- `PullScheduler<S, const N: usize = MAX_SCHEDULED_EVENTS>`: the no_std ring is `heapless::Vec<_, N>`, and
  `PullScheduler::CAPACITY` is `N`. The default is the capacity only of a policy written by hand that names none.
  The scans of the queue (the next entry due, the entry a pop takes) walk the entries as a slice, out of line, so
  every capacity shares one copy of them, as ADR 0075 had every machine of one send-id type share one.
- `StatePolicy::SCHEDULER_BOUND: Option<usize>` (default `None`) is the proven capacity. A build with debug
  assertions keeps the most entries the queue has held at once (`Engine::scheduled_high_water`) and holds it to the
  bound as each turn closes, on both profiles, as ADR 0076 holds each raise to the internal depth. The W3C harness
  states it once more at the end of each run.
- A run starts with nothing waiting: `initialize` empties the queue. What an ended run left is that run's own and is
  not delivered to the next; rule 4 relies on it, and every rule relies on it for a host that starts a run again
  without stopping the last.
- A saved state that lists more delayed sends and invocation deadlines than the proven capacity is refused: it is not
  a state the saved machine could have been in.
- A push past a declared capacity still stops the program, with a message naming the capacity.

### 5. The generated policy

The Rust template writes, for a machine whose queue has a capacity, `pub const SCHEDULER_CAPACITY: usize = N;` and
`type Scheduler = PullScheduler<Self::ScheduledSendId, { SCHEDULER_CAPACITY }>`, with
`const SCHEDULER_BOUND: Option<usize> = Some(SCHEDULER_CAPACITY)` when the capacity is proven, and the basis of the
capacity as comment lines: each site, how many of its events wait at once, and why no more do. A document that cancels
but can leave nothing waiting keeps the zero-size `NoDelayedSends`. A hosted machine with no capacity keeps
`PullScheduler<Self::ScheduledSendId>` and says why.

## Consequences

- The consumer's session engine holds two delayed-send slots where it held 32, about a kilobyte of RAM less for each
  engine, and the number is a proof stated in its generated policy rather than a guess.
- A bare-metal document whose delayed sends cannot be bounded has to say how many may wait. The two no_std probe
  fixtures that arm a send in a state entered again without a cancel on its exit
  (`sce-build/tests/fixtures/no_std/parallel_history_probe.scxml`, `plain_footprint_probe.scxml`) declare 32, the
  capacity their images were measured with.
- `Engine::schedule_event` and its siblings stay public: a host that arms a delayed send by hand on a generated
  machine with a proven capacity arms what no document accounts for, and a test build says so as the turn closes.
- Every hand-written `StatePolicy` keeps compiling: `SCHEDULER_BOUND` has a default and `PullScheduler`'s capacity
  has one.

## Verification

- `sce-build/src/scheduler_bound.rs` tests: onentry and onexit pairs, a compound limit with its children's deadlines
  (2), the regions of a `<parallel>` (added), re-entry after a timer fired or was cancelled, a send never cancelled
  in a state entered again (not proven), a state entered once, a state below a target, a cancel after a statement
  that may fail, a cancel of another id, `idlocation`, two sends under one id, transition content and its target's
  exit, an `<onexit>` send and its state's entry, `<foreach>` over a bounded and an unbounded list, a `<history>`
  default, feature conditions, a host-run deadline and a `<finalize>`.
- `sce-build/tests/c3_no_std_template.rs`: the constant, the queue type and `SCHEDULER_BOUND` in the generated
  policy, the declared capacity, a proven one over a declared one, and the refusal.
- `backends/rust/runtime/tests/a_delayed_send_queue_holds_what_its_document_proves.rs`: a queue of the proven
  capacity, the record of the most it held, the turn that closes past it, and a new run.
- `backends/rust/probes/nostd-queue-size`: the bare-metal ring is sized by the capacity its type names.
- Every Rust machine the test suites run, the W3C conformance corpus among them, is held to its proven capacity in a
  build with debug assertions.
