# ADR 0043 - A child session is a machine of the deploy, and its parent gives it its Mesh endpoint

- Status: Accepted (2026-10-20); steps 1-2 in progress
- Date: 2026-10-20
- Scope: a child session started by `<invoke type="scxml">` that sends to or receives from a Mesh
  peer; `deploy.yaml` (SCE_MESH.md §14); the host route of SCE_MESH.md §19 on every backend; the
  host cores `sce-rust-mesh` and `sce-kotlin-mesh`; the generated invoke sites of the six backends
  and the Interpreter's child sessions; `backends/rust/world` and its external actor protocol; the
  scenarios of docs/SCE_ACCEPTED_SUBSET.md §2.15
- Related: ADR 0042 (a shared record list is a standard part an application invokes), ADR 0034
  (a shared list is held by its members and the hub only relays), ADR 0005 decision 6 (the
  parent gives a child its host), SCE_MESH.md §9.2, §9.6, §10.5, §10.6, §10.9, §10.10, §18, §19,
  §20

## Context

ADR 0042 makes the shared record list a standard statechart that an application invokes as a
child session. The list exchanges its protocol (writes, supersedes, settles, resyncs) with its
peers and, under ADR 0034, with a relay, wherever it runs. A list that runs as a child must
therefore send to a Mesh peer and be sent to by one. No backend lets a child session do either
today. The Mesh router is registered on the engine a host holds, and a generated parent builds its
child's engine itself and gives it a clock, a link to its parent and, when the child declares
`<sce:action>`s, the host its own host answers for it. It gives it no router. A child's send to
`#server` then finds no processor and is `error.execution`, and nothing addressed to the child can
reach it, because the host has no handle on an engine the parent owns.

The evidence, per backend (line numbers as of `c3466693e5`):

- **Rust.** `sce_rust_mesh::endpoint::register` installs the endpoint on the engine it is handed
  (`engine.register_mesh_router` and `engine.register_mesh_rpc_invoker`,
  backends/rust/mesh/src/endpoint.rs:565-590), and the world calls it on the engine of each actor
  and nothing else (backends/rust/world/src/lib.rs:895, and :805 on a restart). The generated
  parent builds its child with `sce_rust_runtime::Engine::new(child_policy)` and calls only
  `set_clock` and `set_completion_callback` on it
  (tools/codegen/templates/rust/invoke_methods.rs.jinja2:427-437; the hybrid start at :283-292);
  the policy takes only the host of its acts (`child_host_call`, :373-375). A child's peer send
  lowers to a host-served send, `engine.perform_host_send` returns `None` when no handler is
  registered for the type (backends/rust/runtime/src/engine.rs:4074-4079), and the send site raises
  `error.execution` (tools/codegen/templates/rust/actions/send.rs.jinja2:109-117).
- **Kotlin.** `registerMeshRouter` writes the router into the engine instance's own
  `hostProcessors` (backends/kotlin/runtime/src/commonMain/kotlin/com/sce/runtime/StateMachineEngine.kt:791,
  :830-832), and `sce-kotlin-mesh`'s `register` calls it on the engine it is given
  (backends/kotlin/mesh/src/commonMain/kotlin/com/sce/mesh/Endpoint.kt:402-412). `startInvoke`
  gives the child `onSendToParent` and the clock and nothing else (StateMachineEngine.kt:4198-4257).
  The send site raises `error.execution` when `performHostSend` finds no processor
  (tools/codegen/templates/kotlin/actions/send.kt.jinja2:88-96). The world's step server registers
  its recording router on the top-level engine and raises every event it is given into that engine
  (backends/kotlin/world/src/main/kotlin/com/sce/world/StepServer.kt:187, :140).
- **Go.** `RegisterMeshRouter` registers on one engine (backends/go/runtime/host_processor.go:183);
  the generated parent builds its child with `sce.NewEngine` and calls `SetClock`,
  `SetCompletionCallback` and `Initialize` (tools/codegen/templates/go/invoke_methods.go.jinja2:322-332,
  and :213-223 for a hybrid candidate).
- **Python.** `register_mesh_router` registers on one engine
  (backends/python/runtime/sce_runtime/engine.py:1139); the child is
  `Engine(_child_policy, script_engine=...)` and is given no router
  (tools/codegen/templates/python/invoke_methods.py.jinja2:267).
- **C11.** The router lives in the machine's own registry (`_register_mesh_router`,
  tools/codegen/templates/c/state_machine.c.jinja2:5488-5493). `_init_with_parent` zeroes the child
  and takes no registry (:6143), and `_invoked_begin` constructs it with a `NULL` one, "no host
  wiring to install: the host started the parent, not this child" (:6196-6233).
- **C++ (generated).** The engine raises `error.execution` for a peer target when no mesh send
  callback is set (sce/include/static/StaticExecutionEngine.h:1082-1090). The generated
  `TransportRouter` sets that callback only on the sessions it hosts, which are the deploy's
  machines (tools/codegen/templates/mesh/cpp/mesh_transport.h.jinja2:2712); a local child is built
  by its parent and has none. A child placed in another partition (Session F) runs under
  `ChildSessionAdapter`, whose callback accepts `#_parent` and declines every other target
  (sce/include/mesh/ChildSessionAdapter.h:123-128).
- **Interpreter.** A child session is built with the event dispatcher its parent's invoke executor
  holds (sce/src/runtime/InvokeExecutor.cpp:306-311, set from the parent at
  sce/src/runtime/StateMachine.cpp:2014). Its peer send therefore reaches whatever Mesh target
  creator is registered for the parent (sce/src/events/EventTargetFactoryImpl.cpp:115-127), and
  would leave under the parent's name with what comes back delivered to the parent. That is not
  a route for a child: it is the parent sending on the child's behalf.

The host cores also refuse a remote `<invoke type="scxml">` peer (SCE_MESH.md §19, "What a host
core refuses"), so placing the child in a process of its own is not available outside C++.

## Decision

**A child session that talks to Mesh peers is a machine of the deployment with a name of its own.
Its Mesh endpoint is held by its parent's host beside the parent's, lives as long as the parent's,
and is given to each session of the child when the invocation starts, as the parent gives the
child its host. Nothing on the wire changes: a peer cannot tell a child from a machine a host
started.**

### 1. Identity: the deploy names the child

A child that takes part in Mesh is declared under `machines:` like any machine, keyed by its name
on the network, with `invoked_by` in place of `source`:

```yaml
topology:
  phone:
    machines:
      planner:                       # a machine the host starts
        source: planner.scxml
        bindings: { ... }
      alice:                         # the list the planner invokes
        invoked_by:
          machine: planner           # the parent, a machine of the same device
          invoke: list               # the literal id of the parent's <invoke>
        bindings:
          "#server": { transport: zenoh_inbox }
        dedup: { window_size: 512 }  # every per-machine section a host core reads
```

- **The key is the name.** It is the `source` of every envelope the child sends, the name its
  peers bind as `#alice`, the path segment of SCE_MESH.md §18.1 and the key chunk of §20.1, held to
  the same character rule, and unique across the deployment by the existing `DuplicateMachine`
  rule. The deploy author chooses it, as for any machine, so a protocol's own peer vocabulary
  (the list sends to `#server`, `#alice`, `#bob`) is met by naming the children so, and the name
  does not move when the application renames its own machine or its invoke.
- **`invoked_by` names the site, not the document.** One site is one name: a document invoked from
  two sites is two machines, and a hybrid `<invoke>` (`srcexpr`) is one machine whose bindings
  must cover the peer targets of every candidate. `invoked_by.machine` may itself be an
  `invoked_by` machine, which is how a grandchild is named. The child's document is read from the
  parent's `<invoke>` (`src`, a standard library name, or `<content>`), so the entry has no
  `source`; it has no `platform`, `scheduler` or `memory` either, which are its parent's process's.
- **The build checks the site.** With `--deploy`, the build refuses: an `<invoke>` whose child
  (any candidate) has a peer send, under a machine of the deploy, with no `invoked_by` entry for
  it; an `invoked_by` that names no machine of the deploy, or no `<invoke>` of that machine's
  document; two entries for one site; an `<invoke>` named by an entry that has no literal `id`
  (an `idlocation`, or an id the processor generates, names a different invocation on every start
  and cannot name a machine); and an entry under a machine that §14.4 hosts as a server pool, whose
  sessions would share the child's one name. Each refusal is a diagnostic code of its own, entered
  in the acceptance appendix as SCE_ERROR_CONTRACT.md requires. A child that only receives, and
  sends nothing, may be declared: being sent to is taking part.
- **The manifest says it.** `sce-codegen generate` reports, beside `needs_mesh_router`, which
  invoke sites start a child that needs a router, so a host learns from the build which children
  it must answer for (SCE_MESH.md §19, "Manifest").

Without `--deploy`, nothing changes: the child's peer sends lower to host-served sends as today,
and a host that answers no router for the child sees them raise `error.execution` in the child.

### 2. The endpoint: the child's own, beside its parent's, for as long as the parent's

The host core builds one endpoint for each declared child, from a peer table the build emits
nested in its parent's (`Machine.children` in `<machine>_mesh_peers.rs` and `<Machine>MeshPeers.kt`,
each child its invoke id and its own `Machine`). Each endpoint is a full endpoint of the core, with
its own `Router`, transport, environment, admission and sealer:

- **Duplicate suppression and ordering are per child** (SCE_MESH.md §10.5, §10.6): the child's
  windows and receive buffers are keyed on the senders it hears from, and its peers key theirs on
  the child's name. The parent's streams and the child's never mix.
- **Sequence streams are per (child, target)** (§10.6.3), counted by the child's endpoint.
- **Retry, buffering and deadlines are the child's bindings'** (§10.10, §9.5): what the deploy
  says under the child's entry, not under its parent's.
- **Admission and sealing are per child.** ADR 0034's host signs and checks a list's operations
  at the endpoint (`Endpoint::set_sealer`, `set_admission`); the list's endpoint is the child's,
  so one device holds as many lists as its application invokes, each with its own trust.

The endpoint lives as long as its parent's endpoint, not as long as one session of the child. A
state that is exited and entered again starts a new child session, and a restored parent starts its
children again from their beginning (docs/SCE_ACCEPTED_SUBSET.md §2.15, "Running invocations");
each of those sessions is the same machine to its peers, which key their duplicate windows and
their ordering on its name (SCE_MESH.md §10.9, invariant 5). An endpoint made afresh for each
session would begin a new sequence stream at 1 under a name its peers already hold one for, and
their receive buffers would drop it, since SCE_MESH.md §10.6.4 drops a sequence number below the
next one expected from that sender. Keeping one endpoint per name for the parent's
lifetime keeps §10.9's rule that one name is one router, and a child adds no case of a stream that
starts over which its parent does not already have.

### 3. Outbound: the parent gives the child its router, as it gives it its host

Each runtime gains one registration beside the router's (SCE_MESH.md §19, "Registration"): a
factory for the routers of the machine's children. The generated parent asks it each time an
invocation starts (on entry, on a restore, and for the candidate a hybrid `<invoke>` chose), with
the invoke's id and the stem of the child's document, as `hostForChild(invokeId, document)` is
asked (ADR 0005 decision 6), and installs the answer on the child's engine **before the child
initializes**, since an `<onentry>` of its initial state may already send. What it installs is
what a host installs on a machine it starts: the child's sends of type `sce:mesh` and its
`<invoke type="sce:mesh-rpc">` go to the answered router, and the answer carries a factory of its
own for the child's children.

- A factory that answers none, or a machine with no factory registered, leaves the child as it is
  today: a peer send raises `error.execution` in the child (§19, "Errors"). It never falls back to
  the parent's router, which would send the child's event under the parent's name.
- A host core registers the factory with the parent's endpoint (`endpoint::register` in Rust,
  `register` in Kotlin, both widened), and answers each invoke id with the endpoint of the child
  declared for it. A host with a router of its own answers with its own.
- The names follow §19's spelling per language and are fixed by the first backend that builds it;
  the registration is one call, with the router and the request invoker, not two.
- A child resumed by a restore (ADR 0045) is asked for when the host registers the factory on the
  restored machine, before it drives it: the restore builds the tree before any registration can
  exist. The invokers of the host's own types are not routers and are not given per child: one
  registry serves the whole tree, and each request names its session (ADR 0045, section 3). Only
  the Mesh request invoker stays the child's router's, for the reason above.

### 4. Inbound: what arrives for a child reaches the child

- **The transport routes by name.** What a peer sends to `#alice` is addressed to the machine
  `alice`: a `zenoh_inbox` put on `sce-mesh/1/to/alice/<sender>`, a WebSocket the child dialed or
  that was dialed to the child's own base (SCE_MESH.md §18.1, §20.1). The host's adapter hands it to
  `alice`'s endpoint, as it hands a top-level machine's to that machine's. Nothing is addressed to
  the parent and forwarded.
- **The generated parent applies it.** What the child's endpoint releases for its engine (an event,
  or the end of a Mesh request) waits in the router the child was given. The generated parent
  applies it to the child each time it drives the child, where it already drains the child's
  events to the parent, before the child's macrostep, in the order the endpoint released it, as a
  host applies a top-level endpoint's calls after a step (`endpoint::apply_to`). The event carries
  the `_event` a top-level machine's Mesh event carries (SCE_MESH.md §10.7: external,
  `mesh://<source>`, the SCXML processor as origin type, the send id and invoke id the envelope
  carried), built by the one function that builds it for a top-level machine.
- **A host drives the parent when a child has news.** A host's loop asks whether any endpoint of
  the parent's tree has calls waiting, not only the parent's, and drives the parent when one does.

Events from the parent (`<send target="#_list">`, `autoforward`) and events from peers land on the
child's one external queue in the order they reach it. Nothing is promised between the two, as
SCE_MESH.md §10.1 promises nothing between sources.

### 5. A child is on the network while its session runs

- **Start attaches, end detaches.** The answer the factory gives is the child's endpoint attached
  to that session. When the session ends (it reached a top-level `<final>`, or its parent left the
  invoking state and cancelled it), the generated parent detaches it, after the child has stopped:
  a send the child makes while it is being stopped is its own act and is sent (what §scxml-6.4 cuts
  off from a cancelled child is its parent, W3C test 252, and a peer is not its parent). A grandchild is detached
  before its parent, since stopping a child cancels its children.
- **Presence follows the session.** Attaching is the child's links becoming ready to its peers and
  detaching is them ceasing to be, in each binding's own terms: a `zenoh_inbox` endpoint declares
  and undeclares its presence token (§20.3), a WebSocket the child dialed is opened and closed, and
  a WebSocket base the child accepts on answers and stops answering (§18.3). A peer therefore
  learns that the session ended as it learns a machine went away (§16.7 row 1), and holds what it
  sends meanwhile under its §10.10 buffer. Re-entering an invoking state is seen by the peers as
  the child leaving and coming back, which is what happened.
- **What the endpoint holds outlives the session.** An envelope the child handed its endpoint was
  sent (SCE_MESH.md §10.8): it is retried or given up under the binding's rules whether or not a
  session is attached, and a §16.7 row raised while none is goes to the host, not to a later
  session.
- **What waited for the session goes with it.** Calls queued for the engine and not applied when
  the session detaches are discarded and reported to the host. An envelope in flight to the child
  that arrives after it detached is admitted to its sender's stream and discarded, as SCE_MESH.md
  §9.4 drops an event for a session whose parent left the invoking state; the host is told.
- **A request the session made ends with it.** A `<invoke type="sce:mesh-rpc">` the child started
  is cancelled when the child's state exits, which stopping the child does; detaching cancels any
  the endpoint still holds for the session, so a late reply finds no entry and is dropped
  (§9.5, `<cancel>`).

### 6. The world and the scenarios

- **The Rust world** hosts a child as a node of its network under its declared name: `World::add`
  takes the parent's router and the routers of its declared children, links, partitions and
  failures act on a child's name as on any actor's, and the world drives the parent when a child's
  endpoint has news. The oracles that read the network see a child as a machine.
- **The external actor** (Kotlin step server) speaks protocol version 2: `raise` takes an optional
  `invoke`, the path of invoke ids of the running child the event is for, and each entry of a step
  reply's `sent` carries the `invoke` and the `start` of the child that sent it, absent for the
  machine itself. The world keeps the child's endpoint, as it keeps every endpoint; the process
  registers a factory on its parent that records the child's sends and queues what the world
  raises for it. The end of a Mesh request is still not carried (external.rs, "Not carried yet").
- **Scenarios** grow two members (docs/SCE_ACCEPTED_SUBSET.md §2.15). A step's `invoke` delivers
  its `event` to the running child of that invoke, through the child's router, as a peer's event;
  a path joins nested invoke ids with `/`, which an XML id cannot contain. A `sent` entry's
  `invoke` and `start` say which child sent it, told as `child_hosts` tells a child; an entry
  without them is the machine's own. Every driver registers a recording factory beside its
  recording router; a driver that cannot (an engine with no factory, or a driver that did not wire
  one) fails a step that states either member and never passes it unjudged, as for `child_hosts`.
  The world writes both members when it records a parent whose child talks to its peers.

### 7. What each backend carries

| Backend | Router factory | Applies a child's inbound | Host core with child endpoints | World |
|---|---|---|---|---|
| Rust | runtime registration | generated parent | `sce-rust-mesh` | actor |
| Kotlin | runtime registration, asked by the runtime's `startInvoke` | the runtime, for the child `startInvoke` started | `sce-kotlin-mesh` | external actor, protocol 2 |
| Go | runtime registration | generated parent | the host's own | none |
| Python | runtime registration | generated parent | the host's own | none |
| C11 | the machine's registry, passed to `_invoked_begin` and `_init_with_parent` | generated parent | the host's own | none |
| C++ (generated) | engine callback the parent's `TransportRouter` answers | the child's `TransportRouter`, whose session slot the start fills and the end clears | the generated `TransportRouter` of the declared child | none |
| Interpreter | beside the creator registered under `sce:mesh` | the child's own dispatcher | the host's own | none |

Every backend carries the factory, because every scenario driver needs it to record a child and to
give it a peer's event, and because which session a send leaves from is a question about the
session (§scxml-6.4), not about a transport. ARCHITECTURE.md Principle 8 keeps transports out of
the parity obligation, and this adds none. The Interpreter's child is built with a dispatcher of
its own whose Mesh creator is what the factory answered, and with none when it answers none, in
place of the parent's dispatcher it shares today.

A C++ child declared with `invoked_by` that a `partitions:` entry places in another partition would
need the worker's `ChildSessionAdapter` to carry peer sends and a router on the worker's side; it is
refused by name at build time until that is built, as §9.6 refused each transport whose dispatch had
not landed (`TransportUnwired`).

## What this does not decide

- **A stream that starts over when a process does.** A host core rebuilt after a crash numbers its
  sequence streams from 1 under a name its peers hold a stream for. SCE_MESH.md §10.9 invariant 5
  keeps the `(source, routing_id)` keying that would answer it closed until a topology needs it, and
  this decision does not open it: a child adds no restart its parent's process does not have.
- **A child in a pool.** A machine §14.4 hosts as several sessions under one name cannot give each
  session's child a name of its own; the build refuses the combination rather than choose one.
- **Signals, fields and subscriptions.** A child carries what a host core carries (§18, §20): sends
  and requests. A deploy that asks more of a child's binding is refused as it is for any machine.

## Alternatives considered

- **The child sends as its parent.** One endpoint and one name for the parent and every child.
  Rejected: a peer cannot tell which session sent an event or address one of them; two sessions'
  sequence streams interleave under one `source`, so a gap or a reordering in one is charged to
  the other; what arrives for the child lands in the parent, which cannot route it; and ADR 0034's
  per-list admission and sealing would have to be one for all lists. The Interpreter's shared
  dispatcher is this design by accident, and it is corrected.
- **The parent relays the protocol in its own document.** The child sends to `#_parent`, the parent
  forwards to `#server`, and the parent forwards every peer event to `#_list`. Rejected: every
  application writes the list's protocol a second time in its own vocabulary, which is the
  duplication ADR 0042 exists to remove, and the child sees every peer's event with its parent as
  the origin.
- **A name derived from the parent and the invoke.** `<parent>__sce_synth_invoke__<id>` (the name
  SCE_MESH.md §9.6.6 gives a synthesised machine) or the §9.2 session id `<parent>:<invoke_id>`.
  Rejected: the name on the network would change when the application renames a machine or an
  invoke, and every peer's binding is keyed on that name; a protocol that
  addresses `#server` and `#alice` could not be met; and `:` is not a character a path segment or
  a key chunk of §18.1 and §20.1 carries. The synthesised name stays the name of the generated code.
- **A second address on the envelope.** The parent's name plus the child's session (CBOR key 18,
  `child_session_id`), with the parent's endpoint dispatching by it, as the C++ router dispatches by
  session index. Rejected: a document writes `#name`, and the name is what every rule keys on
  (bindings, readiness, duplicate windows, ordering, the link a host authenticated);
  a second dimension would have to be added to each, on every core and every binding.
- **A typed operation on the generated host, one per invoke** (`mesh_for_list()`), as
  `actions_for_<invoke>` is. Rejected: on every backend the Mesh router is registered on the engine
  and is not part of the host interface the build generates; a typed operation would give Mesh a
  second door, and the answer depends on the deploy, not on the document's host contract.
- **Registration after the child starts.** The host finds the child's engine and registers a router
  on it. Rejected for ADR 0005 decision 6's reason: an `<onentry>` of the child's initial state sends
  before any call the host can make, and the host holds no handle on an engine its parent owns.
- **A fresh endpoint for each session, with streams keyed on `(source, routing_id)`.** Rejected: it
  opens §10.9's multi-router gate on every core to serve a case that one endpoint per name does not
  have, and it keeps in a session's lifetime the state that belongs to the name's.
- **Presence for the endpoint's lifetime, discarding while no session is attached.** Rejected: a
  peer would be told the child is there while nothing would take what it sends, and every send in
  that window would be a loss the sender is not told of, which readiness (§10.10) exists to prevent.
- **Every talking child in a partition of its own** (Session F remote invoke). Rejected as the
  answer: it exists for C++ only, the host cores refuse it, and it puts a process and a wire hop
  under every list. It stays a C++ deployment choice once the worker carries peer sends.

## Consequences

- A child is a Mesh participant with every rule a top-level machine has, under a name its deploy
  chose, and its peers cannot tell the difference. ADR 0042's step 2 can rebuild `todo_peer` on the
  standard list with the list's traffic leaving from the list.
- `deploy.yaml` gains `invoked_by`, and a deploy whose parent invokes a talking child must declare
  it; an `<invoke>` of a talking child under a deployed machine needs a literal `id`.
- Every runtime gains one registration, every generated invoke site asks it and applies what it
  answers, and the scenario format and the external actor protocol gain the child's members. The
  scenarios of a parent whose child talks state the child's sends, so seven drivers change.
- A peer sees a child leave and come back each time its invoking state is re-entered, and holds
  what it sends meanwhile; a deployment that re-enters often pays reconnections.
- The C++ partition placement of a talking child is refused until the worker carries it.
- No envelope field, wire value or binding key changes.

## Order of work

Each step lands with a test that fails without it.

1. **Documentation and the schema.** SCE_MESH.md: a section on a child session in Mesh (identity,
   endpoint, lifetime, presence), `invoked_by` in §14, the factory in §19, a line under §10.9
   invariant 5. docs/SCE_ACCEPTED_SUBSET.md §2.15: the scenario members and the child's router.
   *Exit:* the deploy parser accepts `invoked_by` and refuses each case of section 1, with the
   diagnostic codes in the acceptance appendix (`acceptance_doc_covers_every_code`).
2. **The runtime door on every backend and the Interpreter.** The registration, the generated
   parent asking it before the child initializes and applying inbound when it drives the child,
   detaching at the end, and the refusal unchanged when it answers none. *Exit:* a new fixture pair
   (a parent and a child that sends to a peer and takes a peer's event, with a hybrid candidate and
   a grandchild) and its scenario, replayed by the seven drivers with a recording factory; the
   `error.execution` of a child given no router held in each engine's own tests; the Interpreter's
   child no longer sends under its parent's name.
3. **The Rust host core.** Child endpoints from the nested peer table, `register` widened, attach
   and detach, presence edges for `wss` and `zenoh_inbox`, discarding and cancelling at detach, the
   tree-wide question a host loop asks. *Exit:* `sce-rust-mesh` tests where a child and its parent
   exchange with one peer over each binding, a child re-entered keeps its sequence stream, and an
   envelope for a detached child reaches neither session.
4. **The world.** Child nodes in the Rust world, the record of a parent with its children's
   members, protocol 2 for the external actor and the Kotlin step server. *Exit:* a world test where
   two parents' list children converge through a hub while the parents re-enter their invoking
   states, and the committed scenarios of that run replayed by the seven drivers.
5. **The Kotlin host core.** Child endpoints in `sce-kotlin-mesh` as in step 3, `wss` only (§20.4).
   *Exit:* the cross-language world test with a Kotlin parent whose list child talks to Rust copies.
6. **The generated C++ router.** A `TransportRouter` for a declared child whose session the parent
   fills at start and clears at end. *Exit:* a C++ fixture where a local child exchanges with a peer
   over one in-tree transport. The partition placement stays refused (section 7).
7. **ADR 0042 step 2** on this ground: `todo_peer` rebuilt on the standard list, the world and
   cross-language tests passing with the list's traffic leaving from the list.

The committed generated trees move with steps 2, 3 and 6 (the invoke templates and the peer table
templates are in the template hash of every language), and are regenerated in those steps.

## Revisions made in building it

Steps 1 and 2, as built.

- **The deploy key** is `invoked_by: {machine, invoke}`, as section 1 proposed, written in place of
  `source:`; a machine entry has exactly one of the two. `source:` became optional in the deploy
  model, and every reader of it reads it through one accessor that skips a child entry.
- **The refusals** are eight codes of stage `mesh-deploy`: `mesh/deploy-child-entry-shape`,
  `-unknown-parent`, `-duplicate-site`, `-pooled-parent`, `-partitioned`, `-unknown-invoke`,
  `-generated-id` and `-undeclared`. The first five are read off the deployment; the last three
  need the parent's document and run as a deploy mutation stage of their own when a machine is
  generated against the deployment. Section 1 did not list an entry with neither or both of
  `source:` and `invoked_by:`, a parent chain that comes back on itself, or a child a partition
  places (section 7 refused the last in words); each is refused by one of the eight.
- **The manifest** field is `child_mesh_routers`: one entry per `<invoke type="scxml">` site whose
  child talks to its peers, or starts one that does, with the invoke's id, the stem of each document
  the site may start, and the child's own sites, nested.
- **The factory's names** are `register_child_mesh_routers` (Rust, Python),
  `registerChildMeshRouters` (Kotlin, the generated C++ engine), `RegisterChildMeshRouters` (Go),
  `sce_host_registry_register_child_mesh_routers` or the machine's `_register_child_mesh_routers`
  (C11) and `StateMachine::registerChildMeshRouters` (Interpreter, registered on the machine beside
  its native action host rather than beside the Mesh creator, since the invoke handler asks a
  parent machine for what a child is given). What it answers is a `ChildMeshRouter` (Rust, Kotlin,
  Go, Python), `sce_child_mesh_router_t` (C11) or `IChildMeshRouter` (C++, in `SCE::Static` for the
  generated engine and in `SCE` for the Interpreter). It is asked with the invoke's id and the
  document stem, empty for an inline child.
- **What arrives for a child** is applied by the generated parent with `apply_mesh_calls` (Rust,
  Python), `applyMeshCalls` (Kotlin, C++), `ApplyMeshCalls` (Go) and the machine's
  `_apply_mesh_calls` (C11), and the session's end is `detach_mesh` / `detachMesh` / `DetachMesh` /
  `_detach_mesh`. Rust's `sce-rust-mesh` endpoint now hands its own machine's calls through the same
  `Engine::apply_mesh_call`, and builds a peer's `_event` with the runtime's `mesh_event_metadata`,
  so the top-level machine and a child are told through one body. Kotlin, Go and Python carry the
  same function (`meshEventMetadata`, `MeshEventMetadata`, `mesh_event_metadata`). The generated
  C++ engine's call is the `SCE::Common::ForwardedEvent` the engine already raises by name.
- **A cancelled child on the generated C++ engine is not exited before its router is told.**
  Section 5 has the parent detach the router after the child has stopped, so what the child sends
  while it is stopped is sent. Rust, Go, Python and Kotlin do that (the Rust and Go hybrid
  candidates are now stopped on cancel too, which they were not). The generated C++ parent has never
  exited a cancelled child: the child's `#_parent` send reaches the parent at once, where
  §scxml-6.4 has the parent take nothing from it (W3C test 252), so exiting it would break that. It
  drops the child as before and tells the routers of the child's sessions, a grandchild first,
  through `detachMesh`, which asks the policy's `detachChildMeshSessions` before the child's own.
  The Interpreter stops a cancelled child when the parent's invoke handler lets the session go,
  which follows the cancel at once, and its router is told after that, so its own children are
  told first.
- **The Interpreter's child takes what arrives in its own event loop.** Section 7's "the child's
  own dispatcher": the child is built with an `EventDispatcherImpl` of its own over the parent's
  scheduler, whose target factory is the parent's with every scheme but the Mesh one
  (`EventTargetFactoryImpl::forChildSession`), and whose Mesh creator is the router's
  `createTarget`. What arrives is raised by the router on the child's own raiser, which it is given
  by `attach` before the child starts. A parent dispatcher of a type the runtime does not build is
  shared as before, with a warning; no host in the repository has one.
- **What each backend does not carry yet.** C11 gives a router to a child of
  `datamodel="sce-static"` only, the only child its generated parent starts through
  `_invoked_begin`, and the build gives the parent the `sce:mesh` processor when a child needs a
  router. A Kotlin child run on a coroutine of its own is given its router and sends through it,
  but its parent does not drive it, so nothing is applied to it. The generated C++ child has no
  `sce:mesh-rpc` invoker: a request it makes raises `error.execution` as before.
- **The scenario** delivers a step's `invoke` event with plain external metadata (the `_event` of a
  step's own event, data only), not the SCE_MESH.md §10.7 metadata of a peer, which needs a source
  no scenario names. A driver that cannot record a child, or finds no child of the path running,
  fails the step.
- **Event coverage of a child's receivers is not judged.** The deploy's coverage and schema
  cross-checks read a machine's `source:`; a child entry has none and is skipped by them until the
  peer table of step 3 reads the child's document from its parent.

## Open questions for the owner

1. **A restored parent loses what its list held.** docs/SCE_ACCEPTED_SUBSET.md §2.15 decided that a
   restore starts a running child again from its beginning, so a child's progress is lost. For a
   list that is every record the copy held, recovered only from peers by resync, and lost for good
   where no peer holds it (a list with one member, or what a member wrote offline and had not yet
   shared). ADR 0042 step 2 restores
   `todo_peer` in its world tests. The repository's principles do not choose between the two ways
   out: the parent's saved state carries its running children's saved states (a change to §2.15 and
   to every backend's save and restore), or the standard list keeps its records through its host and
   is restored from there (a host concern each application meets). This needs the owner's choice
   before step 7.
