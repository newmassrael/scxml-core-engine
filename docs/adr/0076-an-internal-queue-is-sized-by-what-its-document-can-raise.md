# ADR 0076 — An internal queue is sized by what its document can raise

- Status: Accepted (2026-10-31), on the owner's instruction to optimise the footprint structurally
  until no optimisation point is left, and a consumer's request (watching-zenoh).
- Amends: the per-document event-queue capacity (`<scxml sce:capacity>`, SCE Protocol-Synthesis
  RFC §synth-5-L item C3), ADR 0068 (the queues hold event numbers).
- Evidence: watching-zenoh's T2G client image, where each queue of a document that declares no
  capacity is 272 bytes (64 slots of 4 bytes, the runtime default `MAX_EVENT_QUEUE_DEPTH`, and
  16 bytes of head, length and payload manager).

## Context

A machine without an allocator keeps two queues of one type, `P::EventQueue`, whose depth is the
document's `sce:capacity`, else the deploy file's, else 64. A push past the depth stops the program
(`helpers::event_queue::queue_full`): Appendix D processes every queued event, so the engine stops
rather than lose one. The depth is therefore a claim that the queue never holds more, and today
nobody proves it. The default 64 is a guess, and a declared depth is the author's guess.

The two queues are not alike. The external queue holds what the host and other sessions deliver,
whose rate no document can state. The internal queue holds only what this machine raises during a
macrostep (§scxml-C-1), and every source of those events is in the document or in what the engine
does for it:

- `<raise>`, and a `<send>` with no delay to `#_internal`;
- `error.*` events the engine raises for a statement that fails (§scxml-3.12.2), at most one per
  executable-content block run, because an error ends the block (§scxml-4.9);
- `done.state.*` events for a compound state whose `<final>` child is entered, and for a
  `<parallel>` whose regions have all completed (§scxml-3.7).

So the internal queue's depth can be derived from the document, and the external queue's cannot.

## Decision

### 1. Two queue types

`StatePolicy::EventQueue` is replaced by `type InternalQueue` and `type ExternalQueue`, each an
`EventQueue<Payload, N>` with its own depth. The external depth is what it is today: the document's
`sce:capacity`, else the deploy file's, else `MAX_EVENT_QUEUE_DEPTH`. The internal depth is the
bound of decision 2 when the document is in a class decision 2 proves, and the external rule
otherwise.

### 2. The bound the generator proves

The analyzer computes, for each executable-content block (an `<onentry>`, an `<onexit>`, a
transition's content, an initial or history default transition's content), how many internal
events one run of it can enqueue:

- each `<raise>`, and each `<send>` to `#_internal` without a delay, counts one;
- an `<if>` counts its largest branch; a `<foreach>` whose body counts more than zero is
  unbounded;
- a statement that can fail counts the errors it raises before the block ends: one for an
  `<assign>`, `<log>`, `<cancel>` or `<sce:append>` that fails, one per failing `<param>` of a
  `<send>` (the message still goes), one per failing condition of an `<if>` (the chain goes on);
- an `<sce:action>` with arguments counts one when the document names `error.execution`, and
  does not end its block, so each counts.

`R`, the most one microstep can enqueue, is the sum over all blocks (each block runs at most once
in a microstep, because the exit set, the transition set and the entry set are sets), plus:

- one `done.state.*` per compound state with a `<final>` child, and one plus its own `<final>`
  children per `<parallel>`;
- one error per `<param>` or content of a `<donedata>` that can fail;
- for the event that starts a microstep: each guard that can fail, times the most transitions a
  selection enables, and one error for a typed payload that does not decode.

These are read off the generated code's raise sites, and the analysis
(`sce-build/src/internal_queue_bound.rs`) lists each with the construct that causes it. A source
it cannot bound makes the bound not proven: a script engine, an `<invoke>`, a delayed `<send>`
whose delivery can reach the internal queue, an internal `<send>` whose event name is computed,
and a guard that can fail on an eventless transition. The bound is then:

- `0` when `R` is `0`: nothing in the document can reach the internal queue.
- `R × (1 + D)` when `R` is finite, no transition's event descriptor matches an event the internal
  queue can hold (so a dequeued internal event takes no microstep and raises nothing), every
  eventless transition has a target and a condition that does not read `_event`, and the graph
  of eventless transitions is acyclic with longest path `D` (so at most `1 + D` microsteps run
  between the event that starts a macrostep and the first internal dequeue). The graph has an
  edge from `t` to `u` when `u`'s source can be in the configuration with a target of `t`: an
  ancestor, a descendant, or a state of another region of a common `<parallel>`, so two regions
  that each take an eventless transition are a cycle.
- not proven otherwise, or when the bound exceeds 65,535: the internal depth follows the external
  rule, and the generated policy says so.

A proven bound is used even where the author declared a capacity, because a smaller declared
depth stops the program and a larger one is capacity nothing can fill.

### 3. A depth of zero

`EventQueue<Payload, 0>` is a queue of no slots: its push stops the program, which the bound
proves never happens. A machine whose internal bound is `0` keeps no internal slots at all.

`heapless::Deque` refuses a depth of zero when it is compiled, so the payloads a queue keeps
beside its events are a ring of the runtime's own, which takes any depth and drops what it holds
only when a payload has a drop to run, so a queue of a payload with none (`()`, a number) has no
drop code.

## Measured (2026-10-31)

- Of the 486 committed Rust machines, 111 have a proven internal depth (74 of them `0`); the rest
  are not proven, 242 of them because they run a script engine.
- watching-zenoh's `session_fsm_unicast.scxml` and `peer_connection.scxml` are both proven `0`
  (nothing raises): each engine's internal queue loses its 64 or 2 slots. The external queue keeps
  the host's capacity.
- The footprint probe's machines raise events and are not proven; its six images are byte for
  byte what they were before this decision, so the code costs nothing where it saves nothing.

## Consequences

- A document that raises nothing internally pays nothing for its internal queue.
- The bound is a reason as well as a number: the generated policy states which case of decision 2
  gave it, so a change to the document that moves it is visible in the generated diff.
- Hand-written `StatePolicy` impls name two queue types where they named one.
- The external queue's depth stays the host's statement, written as `sce:capacity`.
