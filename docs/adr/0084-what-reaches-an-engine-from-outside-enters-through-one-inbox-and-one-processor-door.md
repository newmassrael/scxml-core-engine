# ADR 0084 — What reaches an engine from outside enters through one inbox, and every host type through one processor door

- Status: Accepted (2026-11-03). The owner asked for the most long-term correct, textbook structure with cost ignored,
  took the recommendation of that day (one processor door shared by both engines, the Mesh request as one row of it,
  one threading rule for every engine) and ordered the work started.
  Step 7 done (2026-11-04): `sce-codegen lower` lowers a host-run `<invoke>` and a Mesh request, and the four
  `static_host_*` scenarios replay on the Interpreter with the standard host registered through the door. The
  replay found two gaps in the door, both closed in step 7 (see D7).
  Step 8 done (2026-11-05), the last: the Interpreter's `processEventMutex_` is gone. Its owner is the thread
  running its macrosteps, and an input from a thread that finds another one running them is posted to its inbox
  (`core/EngineInbox.h`) and applied at the next macrostep boundary. How the rule is held on an engine with no
  loop of a thread of its own is recorded in D8. W3C test 201 showed the boundary drain of D2 left the external queue
  out of arrival order; D2 is amended, the Interpreter follows it, and the generated runtimes owe step 9.
  Step 9 done (2026-11-05): every generated runtime applies what its inbox holds before its owner puts an event of
  its own on the external queue, at the one function that queue's enqueue goes through, and each runtime's inbox
  conformance test holds it to W3C test 201's shape. The generated C++ engine's W3C HTTP harness posts the
  listener's event to the inbox as a host would, instead of raising it on the owner. Recorded in D2 as amended at
  step 9.
- Date: 2026-11-03
- Number: written as 0083 on its branch, and given 0084 when ADR 0083 (a host records what a change made) reached
  `main` first.
- Amends:
  - `SCE_MESH.md` §1 and §19, which say the Interpreter has no Mesh route. It has one for a send (the target factory
    under `SendHelper::MESH_TARGET_SCHEME`, and `IChildMeshRouter` for a child) and gains one for a request.
  - ARCHITECTURE.md, which gains a single-source section "Engine Threading" (§D1-§D3 below) that every runtime's
    thread contract points to instead of stating its own.
  - ADR 0005, decision 5, which keeps the Interpreter refused by name for a Mesh request: from step 7 it is
    lowered and served by the router the host registers (D6, D7).
- Scope:
  - `sce/include/core/` (the shared layer): a host-invoke ledger lifted out of `StaticExecutionEngine.h`, and the
    inbox contract;
  - the C++ Interpreter (`sce/src/runtime/`, `sce/include/runtime/`): `InvokeHandlerFactory`, `ActionExecutorImpl`'s
    send, `StateMachine`'s registration calls and its threading;
  - the generated C++ engine (`sce/include/static/StaticExecutionEngine.h`) and every other runtime (Rust, Kotlin,
    Go, Python, C11, Swift): each gains the inbox and states its thread contract by reference;
  - the Interpreter lowering (`sce-build/src/forge/static_js.rs`), which stops refusing a host-run `<invoke>` and a
    Mesh request;
  - `docs/SCE_ACCEPTED_SUBSET.md`, `SCE_MESH.md`, ARCHITECTURE.md.
- Related:
  - ADR 0043 (child sessions in Mesh, the child router);
  - ADR 0045 (a saved tree holds its running host invocations; `TreeInvokers`, `completeHostInvokeIn`);
  - `docs/SCE_ACCEPTED_SUBSET.md` §2.15 (the Interpreter runs a `sce-static` document after `sce-codegen lower`);
  - ADR 0079 (a device's peer core does no I/O itself; the host drives it).

## Context

Two gaps were measured on origin/main `3f6ad3413a` (2026-11-03). They are one question asked twice: by which door,
and on which thread, does something that happens outside a machine enter it?

### The Interpreter has no host processor door

- `InvokeHandlerFactory::isSupportedType` (`sce/src/runtime/InvokeExecutor.cpp:952`) accepts `scxml` and the SCXML
  processor URI and nothing else. Any other `type` raises `error.execution` (`:1070-1093`). There is no registration
  through which a host could serve a type.
- `SendHelper::isSupportedSendType` (`sce/include/common/SendHelper.h:512`) accepts the SCXML Event I/O Processor and
  BasicHTTP and nothing else.
- `sce/include/core/HostProcessor.h` (the request, response, event and handler types of a host-served send and
  invoke) sits in the shared layer, but nothing under `sce/src` includes it. Its users are the generated engine,
  `StandardHost.h`, `SavedEngine.h` and the AOT tests. The bookkeeping of a running host invocation (the started
  table keyed by type and id, the token, the deadline, the refusal, the one ending) is written inside
  `StaticExecutionEngine.h` (`startedHostInvokes_` at `:878`, `startHostInvoke` at `:3693`, `endHostInvoke` at
  `:3786`). The Go, C11 and Python runtimes carry their own copies of the same shapes.
- `sce-codegen lower` therefore refuses, by name, a host-run `<invoke>` and an `<invoke type="sce:mesh-rpc">` in a
  `sce-static` document (`static_js.rs`, `unsupported_invoke`): the Interpreter would raise `error.execution` where
  every generated backend starts the invocation.
- A Mesh request on the Interpreter degrades to `error.execution` (SCE_MESH.md §9.5). On the five backends that
  route Mesh through the host, the same request is a host-served invoke of type `sce:mesh-rpc` that the router
  registers for (§19). The Interpreter cannot take that route because it has no host-invoke door at all.

§scxml-6.4.1 and §scxml-6.2.5 leave the set of invocable types and Event I/O Processors to the platform. Every
generated backend already lets the host widen that set. The Interpreter is the one engine where it cannot.

### Each runtime answers the threading question differently, and two of them answer it wrongly

| Runtime | Stated contract | Enforced by | A host-invoke ending from another thread |
|---|---|---|---|
| Rust | `Engine<P>` is `Send`, not `Sync` (`engine.rs:12-16`) | the type system: every entry is `&mut self` | cannot compile |
| Swift | not `Sendable` (`Engine.swift:136-141`) | the compiler (tools 6.0) | cannot compile |
| Go | "NOT safe for concurrent use" (`engine.go:41-43`) | documentation | unguarded |
| Python | "Single-threaded" (`engine.py:243`) | documentation | unguarded |
| C11 | none (one line in `clock.h:22`) | nothing | unguarded |
| Kotlin | macrosteps on one coroutine; `send` is a non-suspending `Channel` (`StateMachineEngine.kt:235-238`) | `macrostepLock` around the coroutine's macrosteps | **mutates `startedHostInvokes` and `scheduledSends` without the lock** (`endHostInvoke`, `:1581`) |
| C++ AOT | "single-threaded" (`StaticExecutionEngine.h:3159`); `SCE_THREAD_SAFE` (default off) guards the queue only | the queue mutex, when built with it | **unguarded token check and table erase**, and SCE's own Mesh deadline already arrives on the scheduler thread (`tests/CMakeLists.txt:5349-5351`) |
| C++ Interpreter | multi-threaded: the scheduler delivers on a thread of its own (`StateMachine.h:546`) | `processEventMutex_` | (no door) |

The failures are already real. Kotlin's lock exists because a host thread's `stop` emptied `scheduledSends` while a
macrostep was sorting it (`StateMachineEngine.kt:1962-1974`, a `NullPointerException` in a hosted run). The generated
C++ engine is built with `SCE_THREAD_SAFE` in its Mesh deadline test because the deadline is raised from another
thread, and the option protects the queue push but not the invocation table the same ending erases. Neither runtime
documents which thread may end a host invocation. The Go and Python hosts are each told to build the serialisation
themselves, and that is the arrangement Kotlin and C++ show breaking.

Every runtime already holds one of the parts the answer needs. A forge SCQ multi-producer, single-consumer queue
(`sce/include/mesh/EventQueueBridge.h`) is used by the shared-memory channel only. Rust has `Hal::wake` and
`Hal::irq_save` (`hal.rs:72-106`), which no queue of the engine uses. Kotlin has its `Channel`. The Mesh child
routers hold what arrives and the parent pulls it before the child's macrostep (`applyMeshCalls`), which is an inbox
whose thread safety is left to each router.

## Decision

### D1. Run-to-completion is the invariant, and an engine is confined to its owner thread

A statechart's macrostep runs to completion: no input is observed in the middle of one (§scxml-C-1, the external
queue; the UML and Harel run-to-completion step). Every engine of SCE holds that invariant by one mechanism.

- An **event loop** has one **owner thread**: the thread that drives it. Every state of the sessions that loop runs
  belongs to it: the configuration, the queues, the data model, the scheduled sends, the running invocations, the
  child sessions it drives. In most runtimes a session tree (a root and every child it started, ADR 0045) is one loop,
  because the parent drives its children; there the tree has one owner. A runtime whose child sessions run loops of
  their own (Kotlin's coroutine mode) has one owner per such loop, and the rule applies to each.
- Every operation that changes that state is an **owner-thread operation**. The existing synchronous calls
  (`raiseExternal`, `completeHostInvoke`, `failHostInvoke`, `completeHostInvokeIn`, `applyMeshCalls`, `tick` and their
  spellings) keep their signatures and their synchronous outcome, and become owner-thread operations by contract.
- The language enforces confinement where it can. Rust (`!Sync`, `&mut self`) and Swift (non-`Sendable`) already do,
  and keep doing so. Kotlin records the owner when the loop starts and checks it on each owner-thread operation; a
  call from another thread fails with an error naming the inbox. C++ (both engines) checks it in debug builds. Go,
  Python and C11 document it and point to D2.

The alternatives are rejected below. A lock per engine ("monitor") makes the host block on a running macrostep, and a
reentrant one lets a handler's own completion mutate the tree in the middle of the step. Leaving serialisation to the
host is the arrangement measured above to fail.

### D2. One inbox per session tree carries everything that comes from another thread

- Each event loop has one **inbox**: a multi-producer, single-consumer queue. It is the only part of an engine that
  is safe to use from any thread. An input addressed to a session (a host-invoke ending that names a session of the
  tree) is applied by the loop that runs that session: a loop that reaches a child running a loop of its own forwards
  the input to that child's inbox, and a direct owner-thread call that does so answers whether the child's inbox took
  it. A host obtains a handle to it (`inbox()` and its spellings). The handle is
  `Send + Sync` in Rust, `Sendable` in Swift, and documented as thread-safe elsewhere.
- What may be posted is closed and typed. It is the inputs that today have a cross-thread caller:
  - an external event (what `raiseExternal` takes);
  - a host-invoke ending (`HostInvokeEnding`, the struct the shared layer already has, naming the session in the tree);
  - a Mesh delivery for a session (what a router holds today for `applyMeshCalls`).
- The owner drains the inbox at each macrostep boundary, before the external queue is consulted, in the order the
  posts linearised. It applies each input **by the owner-thread operation of the same name**: one body per operation.
  A posted ending and a direct one therefore cannot differ in what they do. An ending posted after its invocation was
  cancelled or restarted is refused by the token check as a direct one is (§scxml-6.4: what a cancelled process sends
  is ignored).
  Amended at step 8: the boundary is not the only drain point. The external queue takes events in the order they
  arrived, so before the owner puts an event of its own on it (a `<send>` to itself, a host-run invocation's
  `done.invoke`), it applies what the inbox holds, and its event joins the queue behind those. Otherwise a post that
  caused the owner's later send is taken after it: W3C test 201 sends BasicHTTP `event1` to itself, the listener's
  thread posts it while the send waits for the answer, and the `timeout` the document sends next must come second.
  The rule holds for every engine. An internal raise does not drain: the internal queue is the running macrostep's
  own, and arrival order is the external queue's.
  Amended at step 9: an event of the owner's own is any event the owner puts on its own external queue: a `<send>`
  to itself, a host-run invocation's `done.invoke` or report, a delayed send or timer released on the owner (`tick`,
  `advance_time` and their spellings), a child the same owner runs reporting to its parent, and a host's direct
  owner-thread raise. An event the owner routes elsewhere (a peer, another session) is not put on its queue and
  does not drain. Each runtime applies its inbox at one chokepoint, the one function its external enqueue goes
  through, by the operation the boundary drain uses. Applying a posted event reaches that function again and pushes
  the event without draining: one drain applies at a time, and a drain asked for inside it applies nothing, since
  the outer one takes what it held in order. A bounded inbox (C11, `no_std` Rust) drains the same way, as an owner
  operation, and the external queue's own bound and refusal do not change. A host's call that hands its event
  straight to the machine rather than queue it (`processEvent` on the C++ engines) is an arrival too: it applies
  what the inbox holds first, and when that applied anything, its event joins the external queue behind it and the
  main event loop takes them in order (on the Interpreter, in auto mode; an interactive host steps the queue
  itself); when it applied nothing, the event is processed directly, as before.
  The chokepoints:
  - the Interpreter: `StateMachine::applyHeldBeforeExternalEnqueue`, which the raiser asks before an external raise
    (`EventRaiserImpl::ForeignEntry`'s `beforeExternalEnqueue`), as do a host-run invocation's `done.invoke` and an
    event handed over inside a running macrostep (step 8);
  - the generated C++ engine: `StaticExecutionEngine::raiseExternal(const EventWithMetadata &)`, just before its
    push onto the external queue, while a run is going (what is posted before a run is kept for it), with
    `drainingInbox_` holding one drain at a time, and `processEvent` draining first as the Interpreter's does
    (`StaticExecutionEngine.h`);
  - Rust: `Engine::enqueue_external` (`engine.rs`), in `std` and `no_std` alike, with `draining_inbox`. While the
    policy is lent to generated code (`with_policy`), a posted ending for a child session would reach the child that
    code is using, so in `std` it waits in `held_child_endings` and the next drain made with the policy free (the
    macrostep boundary at the latest) applies it first; an ending for the machine's own session is applied at once,
    so its own external queue keeps arrival order;
  - C11: `_raise_external`, the generated machine's only push onto its external queue (`state_machine.c.jinja2`),
    with `draining_inbox`; an interrupt still only posts;
  - Kotlin: `StateMachineEngine.enqueueExternal`, in the blocking and the coroutine mode (whose inbox stays the
    `Channel`), with `drainingInbox`;
  - Go: `Engine.enqueueExternal` (`inbox.go`), while the engine runs, with `drainingInbox`;
  - Python: `Engine._enqueue_external`, while the run goes on, with `_draining_inbox`;
  - Swift: `Engine.enqueueExternal`, with `drainingInbox`; the `Sendable` `EngineInbox` only posts.
  Restoring a saved queue pushes its events back without draining on every runtime: they arrived before anything the
  inbox holds now. The W3C HTTP harnesses of C++ and Kotlin post the listener's event to the inbox, as a host would,
  where they used to raise it on the owner.
  Left open: the rule orders one machine's own external queue. A tree's sessions share the root's inbox, and a
  child's queue does not drain it, so an ending posted for a child session can still be taken after a `<send>` its
  parent makes to that child in the same macrostep (Rust holds such endings for the boundary, which is the same
  outcome). Ordering a child's queue against the root's inbox is a follow-up for every engine.
- A post returns whether the inbox took it. A bounded inbox (C11 and `no_std` Rust, whose capacity the deployment
  declares) refuses a post when full and says so; nothing is dropped silently. A hosted runtime's inbox is unbounded.
- A post wakes the owner through a waker the host installs (`wakeWith` and its spellings; a runtime with its own loop,
  such as Kotlin's coroutine mode or the Interpreter's worker, wakes itself). Amended at step 4: Rust's generated
  machines fix their HAL, so `Hal::wake` alone is no waker a host installs. A `std` post calls the HAL's wake and then
  the waker installed with `wake_with`, and a bare-metal deployment names its waking HAL as a parameter of the
  `BoundedInbox` it declares.
- What the inbox holds when a run ends (a top-level final, or the host's stop) is applied then, by the same operations,
  as a direct call would be at that moment: an event joins the external queue of a machine that will not dequeue it and
  is counted as unseen (§scxml-3.13), and an ending finds its invocation over and is refused by the token check. No
  runtime drops a held input. What differs is a post that arrives after the end (decided at step 4):
  - A runtime that can close without waiting for a post in flight closes the inbox at the end and refuses later posts:
    C++, Kotlin, Go, Python and Swift.
  - A runtime whose owner cannot wait for a post in flight (a bare-metal owner sharing one core with its posters, at a
    strict priority) never closes: C11 and Rust, whose `std` profile keeps the rule its bare-metal profile needs. A
    later post is taken and applied where the owner next reaches the end of its loop, as the direct call would be then.
    Its outcome is the closing runtimes' outcome; only the post's answer differs, taken rather than refused.
- The C++ implementation is the forge LSCQ (`sce/forge/queue_segmented.h`): a list of the SCQ rings
  `EventQueueBridge.h` wraps, lock-free on both sides like them, and unbounded but for the heap, which a hosted inbox
  must be (amended at step 3: the bounded SCQ alone would have refused a post when full, a refusal no other hosted
  runtime makes). Its segments come from a heap allocator beside the engine (`core/EngineInbox.h`), since the forge
  runtime holds no allocator of its own (SCE_FORGE.md §2.1, C2). C11 uses the same algorithm
  (`queue.h`, `queue_irq.h` for a post from an interrupt). Kotlin's coroutine mode keeps its `Channel` as the inbox and
  widens what it carries. The other runtimes use their standard library's concurrent queue.

The timers of a runtime whose clock is pulled on the owner thread (`tick`, `advance_time`) need no inbox. A timer that
fires on another thread (the generated C++ engine's Mesh deadline, the Interpreter's scheduler thread) posts.

### D3. ARCHITECTURE.md holds the rule once

ARCHITECTURE.md gains "Engine Threading (Single Source of Truth)", stating D1 and D2. Each runtime's own doc comment
says which owner-thread operations it has and what its inbox handle is called, and points there. It does not restate
the rule. A conformance test per runtime drives one tree from one owner while other threads post external events and
host-invoke endings to it, and holds the event order and the invocation table to a single-threaded replay of the same
posts.

### D4. The Interpreter serves every host-declared type through the door the generated engines have

- `StateMachine` gains the registrations the generated engines have: an event processor per type
  (`registerEventProcessor(type, HostSendHandler)`) and an invoker per type (`registerInvoker(type,
  HostInvokeHandler)`), shared by every session of the tree as `TreeInvokers` is (ADR 0045). A type under the reserved
  `sce:` prefix is refused at registration (`SendHelper::isReservedHostType`, held to
  `reserved_type_cases.json`), as everywhere.
- `InvokeHandlerFactory` stops being a closed set. It answers the built-in SCXML processor, then a registered invoker,
  then nothing. Nothing is `error.execution`, as today and as §scxml-6.4.1 requires for a type the platform does not
  serve. The `<send>` processor set widens the same way.
- What a host-served `<send>` and `<invoke>` hand the host is the request `HostProcessor.h` defines, field for field,
  computed by the Interpreter's own evaluation of the element. A request the Interpreter builds and one the generated
  engine builds for the same element are compared in a shared fixture, as the reserved-type and deadline-value rules
  already are.

### D5. The bookkeeping of a running host invocation is written once, in the shared layer

The started table, the token, the record made before the handler is asked, the refusal, the immediate completion, the
deadline, the one ending, and what a save writes of a running invocation move out of `StaticExecutionEngine.h` into a
host-invoke ledger in `sce/include/core/`. Both C++ engines call it. The ledger does not own a clock. It asks its
engine's scheduler, through a narrow interface, to arm or drop a deadline. The generated engine's behaviour does not
change: the move is a refactor first, judged by the existing AOT host-invoke and save/restore tests unchanged.

### D6. A Mesh request on the Interpreter is a row of that door

On the Interpreter, `<invoke type="sce:mesh-rpc">` is served by the invoker the host's router registers through a
dedicated call (`registerMeshRpcInvoker`, the name the Kotlin runtime uses), as on the five backends of §19. It is not a
mechanism of its own. The request the router receives has the shape §19 gives: `_mesh_event` and `_mesh_deadline_ms`
as the first params, and the target as `src` or the evaluated `srcexpr`. The same shared fixture holds it to the
generated backends' lowering (`host_processor_analyzer::lower_mesh`). The deadline stays the router's. With no router
registered, the request raises `error.execution`, which is the §9.5 degradation, unchanged.

The Interpreter's Mesh send keeps the route it has (the target factory under the Mesh scheme, `IChildMeshRouter` for a
child). §1 and §19 of SCE_MESH.md are corrected to say so.

### D7. The lowering stops refusing what the Interpreter now serves

`sce-codegen lower` lets a host-run `<invoke>` and a Mesh request through, lowering their `<param>` and `namelist`
expressions as it lowers a `<send>`'s. The `sce-static` scenarios that use a host-run invocation
(`tests/host_features/scenarios/static_host_{clock,http,kv,notify}.json`) join the Interpreter replay, with the
standard host (`StandardHost.h`) registered on the Interpreter as it is on the generated engine. The bounded-string
`namelist` refusal stays, since it is about what the Interpreter cannot carry, not about the door.

Amended at step 7. The lowering also rewrites a host-run invoke's `srcexpr` and `<content expr>` and a Mesh
request's `srcexpr` in place, to the strings they compute, because the Interpreter evaluates each of them itself.
The replay found two places where the step-6 door did not yet match the generated engine, and both are fixed in the
engine, not in the scenarios:
- A request typed by `sce:request` promised its host the whole record, so on the generated engine a field whose
  value cannot be made starts nothing. The Interpreter did not keep `sce:request`, so it reported the failed field,
  left it out and still started the invocation. Its parser now keeps the attribute, and both engines refuse at the
  same `<param>`s, which one shared predicate names (`typedRequestRequiresParam`, `core/TypedHostInvokeRequest.h`).
- The Interpreter's deadline adapter read the ledger's moment off the steady clock and turned it into a delay with
  a second read of that clock, while the scheduler in MANUAL mode fires by its logical clock. A deadline is now a
  moment of the scheduler's own clock (`IEventScheduler::now`), which the ledger and the adapter both read.

### D8. The Interpreter's own threading becomes the rule's

The Interpreter's scheduler thread posts to the inbox instead of entering under `processEventMutex_`. The thread that
runs the macrosteps is the owner. When nothing enters the Interpreter except through its owner and its inbox, the
mutex is removed. This is the last step, after the Interpreter's existing W3C and integration suites have run on the
inbox, because it changes the engine every other test depends on.

Amended at step 8. The Interpreter differs from every other engine in one way the rule had not named: no thread
loops on it. A host calls `start` and then may only wait, and the scheduler delivers on threads of its own, so
between two macrosteps no thread is its owner, and a post nobody applies would never be applied. The rule is held
there as follows (`StateMachine.h`, `StateMachine.cpp`):
- **The owner is the thread running the macrosteps, for as long as it runs them.** It is recorded in the inbox
  (`claimOwner`, `releaseOwner`, and `ownedHere`, which this step adds to `core/EngineInbox.h`). A call that finds
  no thread running the machine claims it and runs its macrostep and the main event loop after it, a drive, and
  releases it when the loop is done; the loop drains the inbox at each macrostep boundary, before the external
  queue is consulted.
- **A post wakes the machine by running it.** This is the D2 "runtime with its own loop wakes itself" for an engine
  whose loop has no thread: a thread that posts and then finds the machine free drives it, and a thread that ends
  its drive releases the machine and then drives what was posted meanwhile. A sequentially consistent fence on each
  side of that handover makes one of the two see the other, so no post is left unapplied.
- **The inbox is the engine's own, and every door is safe from any thread.** `processEvent` and the raiser's
  dispatch, `completeHostInvoke`, `failHostInvoke` and `endHostInvokeIn` (whose session takes the ending through
  its own door, D2), and a deadline the scheduler fires are each taken inside the macrostep the calling thread runs,
  as a drive of the calling thread, or as a post. So the Interpreter hands a host no inbox handle and has no
  owner-thread refusal: no call a host makes can be made from the wrong thread. A posted call answers what a post
  answers (`processEvent` a result that says it was posted, an ending whether the inbox took it); the owner-only
  steps (`runMainEventLoop`, the drain) check the owner in debug builds.
- **A raise from another thread is a post.** The raiser (`EventRaiserImpl`) holds the Interpreter's queues and is
  thread-safe, and a raise it would queue while another thread runs the machine (a delayed `<send>`, a child's
  report or completion, a Mesh router's or an HTTP listener's delivery, a host-served send's reply) used to land on
  the queue the running macrostep was reading. The machine installs its inbox on its raiser
  (`EventRaiserImpl::ForeignEntry`), and such a raise is posted.
- **A call that finds posts still held applies them first.** A thread's earlier call may have been posted and its
  next one find the machine free; that call drains the inbox before its own input, and in auto mode its event joins
  the queue behind what it drained, so no thread sees its inputs taken out of order.
- **The owner applies held posts before its own external events** (the D2 drain bullet as amended at step 8). The
  raiser asks the machine before it puts an external event on the queue (`EventRaiserImpl::ForeignEntry`'s
  `beforeExternalEnqueue`), and a host-run invocation's `done.invoke` and an event handed over inside a running
  macrostep do the same; the machine drains when the calling thread runs it. One drain applies at a time, so applying
  a post that reaches the raiser again does not drain inside the drain.
- **`start`, `stop` and the destructor wait.** They cannot be posted, and each must have happened when it returns,
  so a drive another thread is running is waited for. They are the only waits left; no input waits for a macrostep.
- **Clock.** In MANUAL mode `advanceTime` releases what came due on the calling thread, and that thread is the
  owner of each delivery it makes: a drive of its own when no thread runs the machine, inside the drive when called
  from one, a post only when another thread runs it. The clock is pulled on the owner, as D2's timers rule says.

What closes with the run is the generated engine's: the inbox closes at `stop` and at a top-level final, after
what it holds is applied as a direct call would be then (an event counted as unseen, an ending refused by the token
check, since the exit cancelled its invocation), and a later post is refused and counted.

## Consequences

- A host on any runtime ends a host invocation, posts an event or delivers a Mesh call from any thread by one handle,
  and cannot corrupt an engine by doing so. The defect class measured in Kotlin and the generated C++ engine is closed
  by construction rather than by a lock added where it was found.
- The Interpreter runs every document a generated backend runs with a host-run invocation or a Mesh request, given the
  same host registrations. The dev tools built on it (the visualizer, the scenario replay) stop refusing them.
- One copy of the host-invoke lifecycle in C++ instead of two. The Go, C11 and Python copies stay per language,
  because they are different languages; they are held to the shared fixtures.
- Each runtime gains one public type (the inbox handle) and one documented rule. The existing synchronous calls keep
  their signatures. A host that called them from the owner thread changes nothing. A host that called them from another
  thread was already wrong and now gets an error (Kotlin, debug C++) or a compile error (Rust, Swift, already).
- Cost: every runtime is touched, the Interpreter most, and D8 changes the Interpreter's concurrency model.

## Alternatives rejected

- **A lock per engine.** Correct for mutual exclusion but not for run-to-completion: a host thread blocks for a whole
  macrostep, and a handler that completes from inside the step either deadlocks (non-reentrant lock) or mutates the
  tree mid-step (reentrant lock). Kotlin's `macrostepLock` is this alternative, partly applied, and the unguarded
  ending beside it is how partial application fails.
- **Documenting that the host must serialise.** Today's Go, Python and C11 contract. It asks every host to rebuild the
  same queue, and SCE's own code (the C++ Mesh deadline) does not obey it.
- **A host door for the Interpreter of its own.** A second copy of the lifecycle that D5 makes shared, against Zero
  Duplication.
- **A Mesh route for the Interpreter of its own.** §19 already made a Mesh request a host-served invoke. A second
  mechanism would be a second meaning of the same element.
- **Keeping the Interpreter's refusals.** Leaves the one engine the dev tools run unable to run documents the
  generated engines accept, which the lowering of §2.15 set out to end.

## Order of work

1. This ADR, and ARCHITECTURE.md "Engine Threading" (D1-D3). Documentation only.
2. Kotlin: owner check and inbox, with host-invoke endings and external events through it (D1, D2). This is the
   measured defect, so it is repaid first, with a JVM test that ends invocations from other threads during macrosteps.
3. The generated C++ engine: inbox on the SCQ queue, the Mesh deadline posted instead of raised, debug owner check.
4. Rust, Swift, Go, Python, C11: inbox handle and the per-runtime conformance test of D3. Rust and Swift change no
   existing signature.
5. The host-invoke ledger in `sce/include/core/` (D5), as a refactor of the generated engine with its tests unchanged.
6. The Interpreter's processor door for sends and invokes on the ledger, and the Mesh request row (D4, D6), with the
   shared request fixture.
7. The lowering (D7) and the `sce-static` host scenarios on the Interpreter replay.
8. The Interpreter's threading (D8), and SCE_MESH.md §1 and §19.
9. Done (2026-11-05). Owed by the D2 drain bullet as amended at step 8: the generated runtimes (C++, Rust, Kotlin,
   Go, Python, C11, Swift) drained only at the macrostep boundary, so an owner's own external event could overtake a
   post that arrived before it. Each applies what its inbox holds before it puts an event of its own on the external
   queue, at one chokepoint (D2 as amended at step 9), with a test of the W3C 201 shape. The generated C++ engine's
   W3C HTTP harness worked around exactly this by raising the listener's event on the owner instead of posting it
   (`tests/w3c/aot_tests/HttpAotTest.h`); it now posts it, natively from the listener's thread and under WASM from
   the owner inside the send.

Each step lands with its tests and is pushed under the repository's push rule. A step that finds the rule wrong for a
runtime stops and amends this ADR rather than working around it.

## Open questions

- Decided in step 4 (the owner, 2026-11-04): the owner of a tree whose host never runs a loop of its own (a C11
  machine driven from a superloop and an interrupt) is the superloop, and the interrupt posts. C11's inbox is bounded
  and its capacity is declared per deployment (`-DSCE_MAX_INBOX=N`, the override every C11 bound takes), with the
  queue's row (`queue.h`'s SCQ, or `queue_irq.h` on a target with no read-modify-write atomic) chosen by the atomics
  the deployment states. A deployment that declares no capacity has no inbox and declares no post.
- Whether a Mesh router's held calls become inbox posts in step 3 and 4, retiring `takeCalls`, or keep the pull they
  have and only gain the thread-safety requirement in writing. The first is the rule applied fully. It is decided when
  step 3 measures the C++ router.
