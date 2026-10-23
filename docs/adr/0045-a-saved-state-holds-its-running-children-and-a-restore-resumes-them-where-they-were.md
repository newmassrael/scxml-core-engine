# ADR 0045 — A saved state holds its running children, and a restore resumes them where they were

- Status: Accepted (2026-10-20); steps 1 and 2 done (two ids of step 2 land with steps 3 to 5,
  see its "Where it stands"), step 3 in progress (see its "Where it stands")
- Date: 2026-10-20
- Scope: docs/SCE_ACCEPTED_SUBSET.md §2.15 ("Saving and restoring", "Running invocations",
  "Child sessions", the scenario format); `schemas/sce-saved-state.v1.schema.json` and a new
  `schemas/sce-saved-state.v2.schema.json`; SCE_WIRE_CONTRACTS.md (the saved-state row);
  `forge::static_lowering::saved_shape` and the decision which machines save; the save and restore
  of the Rust runtime (`sce_rust_runtime::saved_state`, `<Machine>Persist`) and the Kotlin runtime
  (`com.sce.runtime.SavedState`, `StateMachineEngine`); the invoke templates of every backend; a
  saved state for the Go, Python, C++ and C11 backends; the Mesh host cores (`sce-rust-mesh`,
  `sce-kotlin-mesh`) and SCE_MESH.md §5, §10, §13, §14, §16.7 and §19 (the checkpoint, durable
  receipt, `deploy.yaml`'s `persistence:` and `receipt:`, pattern 22); the shared instances under
  `sce-build/tests/fixtures/static_datamodel/saved/` and the scenario drivers
- Related: ADR 0042 (a shared record list is a standard part an application invokes; branch
  `shared-record-list`), ADR 0043 (a child session that talks to Mesh peers has its own endpoint;
  branch `child-mesh-adr`), ADR 0044 (the revisions of a schema are one schema in a saved state;
  branch `schema-revision-reach`), ADR 0034 (a shared list is held by its members),
  ADR 0005 decision 6 (the parent gives a child its host)

## Context

A host whose process can be killed saves a `sce-static` machine at a macrostep boundary and
restores it into a new process (docs/SCE_ACCEPTED_SUBSET.md:4947-4964). The saved state holds every
variable, the configuration, the history, the waiting sends, the host-run invocations and the
external queue, but of a child session an `<invoke type="scxml">` started it holds only the id:

- **The format names the child and nothing of it.** `invokes` is "the `<invoke>`s whose child is
  running, by the id the document gives each", and "a restore starts each again, from the beginning
  of its child, under the same id. [...] a child's PROGRESS is lost: a machine saved after its child
  took one of two events and restored needs that event again. The child's own delayed `<send>`s are
  part of that progress, and start over with it" (docs/SCE_ACCEPTED_SUBSET.md:5009-5021). The schema
  says the same (`schemas/sce-saved-state.v1.schema.json:55-60`: "A child session is not saved").
- **A hybrid child loses the candidate too.** "The candidate that was running is not recorded: a
  restore runs the start again, which reads `srcexpr` from the restored variables and hands every
  candidate the arguments' current values" (docs/SCE_ACCEPTED_SUBSET.md:5034-5045).
- **Rust.** `SavedState::invokes` is a `Vec<String>` ("A child is not saved",
  backends/rust/runtime/src/saved_state.rs:110-115); `save` fills it from `running_invokes()`
  (:1063-1069), and `enter` judges each id and, last of all, calls `engine.restart_invokes`
  (:1441-1505), which defers the start of each child as entering its state would
  (backends/rust/runtime/src/engine.rs:1374-1394, policy.rs:539-561). The child itself is a typed
  engine the parent's policy owns, `child_<id>: Option<Box<Engine<ChildPolicy>>>`
  (tools/codegen/templates/rust/state_machine.rs.jinja2:679, :687 for a hybrid candidate), so the
  parent could save it: the child's own `<Machine>Persist` exists whenever the child's document
  saves.
- **Kotlin.** `runningInvokes()` lists the ids whose child is not final
  (backends/kotlin/runtime/src/commonMain/kotlin/com/sce/runtime/StateMachineEngine.kt:2726-2727),
  and `enterSaved` calls `restartInvoke` for each and then `startDeferredInvokes()` (:2797-2799),
  which the template answers by deferring the start again
  (tools/codegen/templates/kotlin/state_machine.kt.jinja2:388). The child is a
  `StateMachineEngine` in `activeInvokes[id].child`, again a machine that can save itself.
- **The tests hold the loss as the rule.** `a_child_that_had_taken_an_event_is_started_over_not_resumed`
  (backends/rust/tests/tests/a_saved_child_session_is_restarted_after_a_restore.rs:102-118) and
  `aChildThatHadTakenAnEventIsStartedOverNotResumed`
  (backends/kotlin/tests/src/test/kotlin/com/sce/integration/ASavedChildSessionIsRestartedAfterARestoreTest.kt:106-127),
  over the shared instance `saved/static_invoke_working.json`, whose whole account of the child is
  `"invokes":["worker"]`.
- **The child's host is asked again because the child starts again.** A parent obtains a child's
  host "each time the invocation starts — on entry of the invoking state, and on a restore, which
  starts the child again from its beginning" (docs/SCE_ACCEPTED_SUBSET.md:3664-3665).
- **Two sends cannot be saved because the session they wait for is not in the state.** A document
  with a delayed `<send>` to the parent, to an invocation or to a child session is generated without
  `save` / `restore` (docs/SCE_ACCEPTED_SUBSET.md:5181-5198; `saved_shape` returns `None`,
  sce-build/src/forge/static_lowering.rs:3360-3378).
- **Only two of the six generated backends save.** Go, Python, C++ and C11 have no saved state
  (docs/SCE_ACCEPTED_SUBSET.md:3926-3935, :4044-4045, :3675), and the Interpreter refuses a
  `sce-static` document at its root (:5211-5227).

What makes this a defect now is ADR 0042. The shared record list is a standard statechart an
application invokes as a child session, and "today a host reads `values` from a saved state"
(docs/adr/0042, line 62, on branch `shared-record-list`). Under §2.15 a parent restored from its
saved state starts that list again from its beginning, with no records. Its peers may give them back
by resync, but where no peer holds a copy (a list with one member, or what a member wrote offline
and had not yet shared) they are gone for good. ADR 0043 left this as its first open question
(docs/adr/0043, lines 382-393, on branch `child-mesh-adr`), and it holds for any child that lives
long and holds state worth keeping, not only the list.

ADR 0043 also makes a talking child a machine of the deployment with an endpoint of its own, which
holds sequence counters, duplicate windows, ordering buffers and envelopes not yet sent (ADR 0043
section 2; SCE_MESH.md §10.5, §10.6.3, §10.6.4, §10.10), and it leaves open "a stream that starts
over when a process does" (ADR 0043, "What this does not decide"). A restore that resumes the child
but not its endpoint would resume a list whose next envelope its peers drop as one they have already
passed (SCE_MESH.md §10.6.4, rule 3).

The owner chose: a parent's saved state holds its running children, recursively, and a restore
resumes them where they were. This ADR decides how.

## Decision

**A saved state is a tree of sessions. Each running child is saved inside its parent, as the whole
saved state of its own document, under the invoke that started it and the number of that start; a
restore judges the whole tree first and then rebuilds every session where it stood, starting none of
them. What the network changes is not part of a session, and the Mesh host core saves it beside the
tree in one checkpoint, holding every envelope back until the checkpoint that produced it is
durable. What the host created and the document cannot name stays the host's. Each binding chooses
in deploy.yaml between the receipt its transport reports, where a restore reports what may have been
lost, and durable receipt, where nothing the receiver was sent is lost.**

### 1. What a saved state holds for each running child

A saved state is one root SESSION; a session holds what a saved state holds today, and in place of
the list of ids, each running child as an entry:

| Member of a child entry | What it is |
|---|---|
| `id` | the id the document gives the `<invoke>`, as `invokes` holds it today: the key a restore judges against the document (the invoke exists, its state is in the saved configuration, no id twice) |
| `start` | which start of that `<invoke>` this is, counted from 1 within its parent session: the number `child_hosts` and ADR 0043's `sent` already tell a child by (docs/SCE_ACCEPTED_SUBSET.md, the scenario table) |
| `invokeid` | the id the session was started under, as the parent handed it to the child and as `_event.invokeid` and an `idlocation` variable hold it; carried rather than computed again, because a resumed child keeps the id its parent's variables and queued events already name |
| `document` | which document the session runs: the child's name for a static invoke, the candidate's for a hybrid one |
| `session` | the child's saved state, of the same grammar as the root's, its own children included |

A session also carries:

- **`invokestarts`**: for every `<invoke>` of the document started at least once, how many times it
  has been started, so that a start after a restore is numbered on. It is carried for the reason
  `sendseq` and `hostinvoketoken` are: a restored parent that counted from 0 again would give a new
  start the number of one a host or a peer already saw. A running child's `start` is the count of its
  invoke (a state re-entered cancels the earlier child before it starts the next one), and a restore
  refuses any other.
- **`sessionid`**: the session's own id. Events held in the tree name sessions by it (the `origin`
  of a child's event queued in its parent, the `origin` a parent's event carries in its child's
  queue), so every session of a restored tree keeps the id it had. A session id is generated once
  per session and never by a restore. A child's is `<parent>.<invokeid>` (below). A root's is bound
  when the machine is created, as §scxml-5.10 binds `_sessionid` at load, and is never empty: the
  host gives it, or the runtime draws one (the owner's answer, "Questions the owner answered" 2).

**The ids inside a tree are the same on every backend.** A saved state is one text whatever backend
wrote it (docs/SCE_ACCEPTED_SUBSET.md:4960-4964), and a tree holds ids that today are not: Rust
generates an invoke id from the address of the parent's policy
(tools/codegen/templates/rust/invoke_macros.rs.jinja2:49-50) and a session id from a timestamp and a
process-wide counter (backends/rust/runtime/src/helpers/unique_id_generator.rs:115-127). Every
backend therefore derives a child's session id and a generated invoke id from the tree alone: the
parent's session id, the state, the `<invoke>`'s id and the start, in one spelling the format fixes.
Ids are unique within a tree by construction, and a tree is one process's. The one id a tree does
not derive is its root's, which is drawn or given once when the root is created and then saved and
restored with the tree like every other id, so the derivation always starts from the same text.

**Running** keeps its meaning: started and not ended. A child that has ended is absent from its
parent's list, its `done.invoke` in the parent's external queue or already taken, exactly as today
(docs/SCE_ACCEPTED_SUBSET.md:5023-5032).

**Hybrid children.** The candidate that was running is recorded (`document`) and resumed, whatever
`srcexpr` reads now: `srcexpr` and the arguments are evaluated when an invocation starts
(§scxml-6.4), and a resumed child did not start. The child's variables hold what it was handed and
what it did since. A restore refuses a candidate the hybrid `<invoke>` does not list.

**Children of children.** A child's `session` holds its own running children by the same rule, to
any depth. Nothing about a grandchild is special: it is a child of a session that happens not to be
the root.

**Which documents save.** A document saves when its own state can be held (today's rule,
`saved_shape`) AND every document it can start through an `<invoke type="scxml">` saves, every
candidate of a hybrid `<invoke>` included, recursively. A parent with a child that cannot be saved
is generated without `save` / `restore`, so a host finds out when it compiles rather than when a
restore drops part of the tree (the reason §2.15 already gives, docs/SCE_ACCEPTED_SUBSET.md:5185-5188).
An `<invoke>` whose child runs on another device (`remote_mesh_target`,
sce-build/src/forge/static_lowering.rs:3571-3583) is not a session this tree holds, and still
keeps its parent from saving.

**Sends between sessions of the tree are now held.** A delayed `<send>` to `#_parent`, or to
`#_<invokeid>` of a child of the sending session, waits on a session the tree holds, so it no longer
removes the save API. `pending` gains two acts, `parent` (from a child to its parent) and `child`
(from a parent to the child of one invoke id), saved and armed as `raise` is, in the sending
session's scheduler, and delivered when due to the session the restored tree holds under that link;
a target that is not running when it comes due is answered as it is for a send that was never saved.
A delayed send to any other session (`#_scxml_<sessionid>` of a session outside the tree, a Mesh
peer) still removes the save API.

### 2. The shape of a tree

There is no one hash of a tree. **Each session carries the `shape` of its own document**, computed
as today (`saved_shape`), and the restoring build judges each session against the shape of the
document it would run there: the root against its own, a child against the document its parent's
`<invoke>` names (a static child's one document; a hybrid child's candidate named by `document`).
The parent's shape still names each `<invoke>` by its id and the state that holds it, and does not
name the child's shape, so changing a child does not change its parent's shape and a refusal can
say which session it is.

**ADR 0044 composes per session.** Each session carries its own `revisions` (ADR 0044 decision 3),
and the restoring build upgrades each session by its own document's `SavedShape` before reading it,
as it upgrades a root today: a child saved at revision r of a record schema now at revision R reads
the defaults of the fields added after r; a child saved at a later revision than the build's is
refused. A schema that two sessions of the tree import is judged in each session by that session's
own document, since each session's variables are its own.

**In format 2 the shape names an enum by its document, not by its alias.** ADR 0044 keys an enum's
`revisions` entry and the Mesh check by the enum's document (`schema_revision::document_key`:
`enum:` and the path of the enum's file relative to the directory of the machine's own document,
`.` and `..` folded and components joined by `/`, or the folded `sce:std/...` name of a standard
one). The format-1 shape still spells every enum-typed variable and record field `enum:<alias>`,
the name the importing document gave the import, because changing that spelling inside format 1
would refuse every state already saved with an enum. Format 2 is the step at which it is put right:
its shape spells an enum-typed variable and record field by the same `document_key` text that
`revisions` uses, so renaming an import alias no longer refuses a restore of an enum that did not
change, and a session's shape and its `revisions` name one enum one way. Moving the enum's file
moves its key and still refuses, since it is another document to every reader (ADR 0044).

- **Two shapes per document, as long as format 1 is read.** The generator emits the format-1 shape
  (alias spelling, and ADR 0044's revised format-1 shape) for judging a format-1 text, and the
  format-2 shape (document-key spelling, and its revised form) for judging and writing a format-2
  one. A format-1 text is read exactly as today; nothing in it is re-spelled. The two texts are
  the same line for line but for the enum-typed lines, header included, so a document that holds
  no enum has one shape in both formats; and since the format-1 text names no document, moving an
  enum's file moves the format-2 shape and the key and leaves the format-1 shape as it was.
- **One text on every backend.** The key is computed once, by the generator, from paths the
  restoring build reads, and emitted into every backend's shape constant; no backend computes or
  re-spells it. So every backend hashes the identical key text, writes the identical `revisions`
  keys and saves the identical bytes, and the shared format-2 instances hold this the same way they
  hold every other part of the format. The key holds no absolute path, so it is also one text in
  every checkout.

**A child whose document changed shape refuses the restore of the whole tree**, with a reason that
names the session by its path (`invokes[0].session.invokes[1]`, `list` start 3, shape `...`), and
leaves everything as it was, as a root of another shape is refused today
(docs/SCE_ACCEPTED_SUBSET.md:5125-5130). It is not restarted from its beginning in place of being
resumed: that is the loss this ADR removes, and a restore that did it whenever a document changed
would do it silently on the day an application updated. The host keeps the text, and a document that
needs to change shape across an update waits for the migration §2.15 already names as the author's
until a document can declare one.

Because of that, a standard statechart that holds state (ADR 0042's list) is a promise about its
shape: it changes only as ADR 0044 allows (fields added to a record schema with a `since` and a
default), and a test pins the shape of every standard statechart that saves, so that an edit which
would refuse its users' saved states is seen in the repository and not on their devices.

### 3. Restore

A restore is one act over the tree, in this order:

1. **Judge everything before building anything.** The format, then every session depth first: its
   shape and revisions, configuration, variables, history, waiting sends, host-run invocations,
   external queue (each as today), and its running children (each `id` an `<invoke>` of the document
   whose state the configuration stands in, named once; `start` the count in `invokestarts`; a
   `document` the `<invoke>` can start; a session of that document's shape). A refusal anywhere
   builds nothing and names the path.
2. **The root, as today.** The host's clock installed, the configuration entered without
   `<onentry>` and without `<data>`, the variables, the history, the external queue in order, the
   waiting sends armed against the wall clock, the host-run invocations re-armed, the counters
   (`hostinvoketoken`, `sendseq`, `invokestarts`) carried on, and the saved `sessionid` bound as
   the root's id in place of one drawn or given: the session continues, so a peer's
   `#_scxml_<id>` address and every child's `<root>.<invokeid>` stay what they were.
3. **Each running child, in document order, depth first.** The parent builds the child's engine as
   it does when the invocation starts (the parent's link for `#_parent`, the same clock, the child's
   host obtained from the parent's host as ADR 0005 decision 6 obtains it, and under ADR 0043 the
   child's router from the factory), and then RESTORES it in place of initializing it, by steps 2 and
   3 applied to the child's session: no `<onentry>`, no `<data>`, no `<param>` or `namelist`
   evaluated, no `idlocation` written, nothing sent. The child is registered in the parent under its
   `invokeid` and `start`. The waiting sends of every session are armed against the one wall clock
   the restore was given, so a child's delayed send comes due when it would have.
   The router is the one thing the restore cannot ask for as it builds the child: the factory is a
   registration on the parent's engine, which the restore is what hands to the host, so no factory
   exists yet. A resumed child is therefore owed its router, and the factory is asked for it when the
   host registers the factory, before the host drives the root (Rust: `owe_child_mesh`,
   `StatePolicy::start_owed_child_meshes`). That keeps ADR 0043's order, the router installed
   before the child does anything, since a resumed child runs nothing until its parent drives it; a
   grandchild is asked of the router its parent is given, in turn, and a factory registered again
   asks for none a second time. A host that drives the root before it registers a factory runs the
   resumed children with no router, as a start with no factory does, until it registers one.
4. **Nothing runs.** The restore takes no step. The first drive of the root is the drive the saved
   tree would have made next: each session's host-run invocations start again at the top of that
   session's first macrostep (`restarted`, as today), the parent drives its children where it always
   does, and a machine that owes a start or holds an interruption answers `0` to the wake-up query,
   as today.

What this means for the events between sessions:

- **Queued events stay where they were queued.** An event a child sent to `#_parent` and the parent
  had not taken is in the parent's `external`, with its `invokeid` and `origin`; an event the parent
  sent to a child (`<send target="#_<invokeid>">`, `autoforward`) that the child had not taken is in
  the child's `external`. Both carry the session ids the restored tree keeps, so `_event.origin` and
  `_event.invokeid` read as they did.
- **Nothing is in flight between two sessions at a boundary.** Today's restart had to place a
  child's start-up sends behind the saved queues (backends/rust/runtime/src/saved_state.rs:1501-1504);
  a resumed child sends nothing as it is restored, so there is no such ordering to keep. What a save
  must exclude instead is an event in transit: Rust carries a child's `#_parent` sends through a
  buffer the parent drains after it drives the child (the `settle_<slot>` each generated Rust
  parent has per child slot, tools/codegen/templates/rust/invoke_methods.rs.jinja2). The tree is
  saved only at a boundary of its root at which every such buffer is empty, which the generated
  parent establishes by draining within every drive of a child: its start, its tick, and each
  delivery the parent makes to it (a `<send>` to `#_<invokeid>` made at once, a delayed one coming
  due, one to the child's `#_scxml_<sessionid>`). A save that finds one not empty is refused with
  that reason, a defect of the generator and never a timing the host can meet, and a test holds
  that it is never reached
  (backends/rust/tests/tests/nothing_is_in_transit_when_a_tree_is_saved.rs).
- **What that moves: a child's reply to a delivery reaches its parent within the delivery.** Before
  this decision a child that answered an event its parent sent it at once left the answer in its
  buffer until the parent's next `tick` of its children, so a host that drove the parent by `step`
  alone never saw it, and one that ticked saw it a tick later, behind what the host had raised in
  between. Now the answer, and the child's `done.invoke` if the event ended it, are on the parent's
  external queue when the `<send>` returns, ahead of any event the parent sends itself later in the
  same block, and the macrostep that sent takes them before it ends. The standard leaves open the
  order in which events of two sessions reach a queue, so both orders are its; the W3C tests whose
  parent addresses a child by its invoke id and waits for what follows (test192 and test253 for
  an answer, test347 for the child's ending) are the ones a change in that order reaches, and each
  holds either way: it waits in a state that takes only that event, and a `done.invoke` is raised
  behind every answer the child sent before it ended. A host that drove with
  `step` and `tick` sees the same events, earlier; one that drove with `step` alone now sees
  answers it did not see before.
- **`done.invoke` is not raised again.** A child that had ended is absent and its completion is in
  the parent's queue or already taken; a child that is resumed did not end, and raises its
  `done.invoke` once, when it ends after the restore. Nothing a child did at its start (its
  `<onentry>`, its sends, its host's first acts) happens a second time.
- **A save of a tree is a save of every session at once.** It is refused when any session's last
  macrostep was `truncated`, or on Kotlin when the root is driven by its own coroutine, as a single
  machine's is today.

**A child session's host-run invocations are served by the host of the tree.** A child's `<invoke>`
of a type the host serves (its own, or a standard host feature's) is a host-run invocation like the
root's, and the tree fixture holds one running. Decided 2026-10-21, over a factory of child
invokers modelled on ADR 0043's child routers:

- **One registry for the tree.** The set of invocable types is the processor's (§scxml-6.4.1), not
  a session's, so the invokers a host registers on the machine it built serve every session of its
  tree. A parent hands its registry to each child as it starts it or resumes it (before the child
  runs, so a resumed child's re-armed invocations, which start at the top of its first macrostep,
  find it), and a registration made later serves the children already running. Invokers carry no
  identity, unlike a Mesh endpoint, which is why ADR 0043 needed a router per child and this does
  not; the Mesh request invoker stays per session, since a child's is its router's and a parent's
  would send under the parent's name.
- **Each request names its session.** `HostInvokeRequest` carries `session_id`, the id of the
  session that started the invocation. A host that serves a type to part of the tree only refuses
  the other sessions' requests with the response's refusal (the child's `error.execution` for an
  invocation that could not start); there is no second registration.
- **An ending is routed by session id.** The host ends an invocation through the machine it holds,
  naming the session and the request's token (`complete_host_invoke_in`, `fail_host_invoke_in`);
  each generated parent hands it to the child that is that session or holds it (a session below a
  child has the child's id and a dot in front of the rest, the ids being tree paths since answer 2).
  A session the tree does not hold, or a start it is no longer running, is refused with the reason
  and raises nothing. The child takes the event when its parent next drives it.
- **The build declares the types once for the tree.** A child written in place is generated with its
  parent's `--host-invoker` declaration, and with the standard types it names.
- **Backends.** Rust implements it with step 3 and Kotlin with step 4, Go, Python and C++ with their
  step 5, and C11 with its own.

**An `<invoke idlocation>` of a child session is written under `sce-static`.** Each start is started
under `<state>.<start>.<id>` (step 2), which the child's events carry as `_event.invokeid` and a saved
tree holds as the child's `invokeid`, so a stored id now has readers: the document, and the restore,
which keeps the variable as saved and writes nothing. The variable is a declared string whose bound
exceeds the longest such id (twenty digits of start count); a host-run invocation or a Mesh request,
started under the id the build wrote, still refuses it. Every backend that saves writes it, C11
among them.

**A state written in format 1 is still read.** It holds no child's state, and the build that wrote
it promised to start each listed child from its beginning; the restoring build does exactly that for
such a text (the only case in which a child restarts), because the state never held more. A host can
see which it has from `format`.

### 4. What is the saved state's, what is the endpoint's, and what is the host's

The rule: **a value belongs to the saved state when only a session's own steps change it; it belongs
to the endpoint when the network changes it; it belongs to the host when the host created it and no
document can name it.**

| Held by | What | Written by | When |
|---|---|---|---|
| The saved state | every session of the tree: variables (a list's records among them), configuration, history, queues, waiting sends, running invocations with their requests, counters, session ids, children | the generated machine and its runtime | at a macrostep boundary of the root |
| The endpoint section of a checkpoint | per machine name of the deployment (the root's and each `invoked_by` child's, ADR 0043 section 1): the outbound sequence counter per target (§10.6.3), the duplicate window per sender (§10.5), the ordering state per source, `next_expected_seq` and the envelopes buffered past it (§10.6.4), the envelopes not yet released to a transport or held for retry (§10.10, ADR 0043 section 5), with their envelope ids, sequence numbers and the wall-clock moment each comes due, the envelopes a durable binding still holds unacknowledged and, per durable source, the acknowledged mark (section 5), and what the endpoint released for a session that the session has not yet taken | the Mesh host core (`sce-rust-mesh`, `sce-kotlin-mesh`), or a host that runs its own router | with the saved state, in one checkpoint |
| The host | keys and every part of ADR 0034's trust state (device keys, the MLS group state, the membership and operation logs the host keeps), credentials, the store itself, and what a host keeps for its own host-run work (§2.15 already leaves that to it) | the host | as the host's own storage requires; never rolled back with a checkpoint |

Why the endpoint is not in the saved state: a session's state changes only when the session steps,
at boundaries the generated machine controls, while an endpoint's changes whenever the network
moves (an envelope admitted, a gap timed out, a retry due), on threads the machine does not own, and
an endpoint lives as long as its name and not as long as one session (ADR 0043 section 2). Putting it
in the machine's save would make the generated machine read state it does not own at a moment it
cannot fix. Why it is not the host's alone: the host cores are SCE's code, and every host that uses
one would otherwise have to reach inside it.

**One checkpoint, written once.** The host core takes a checkpoint between two drives of a root:
the root's saved state (the tree) and the endpoint section of every machine name in that tree, as
one document (`sce-mesh-checkpoint`, a new wire surface of the host cores, registered in
SCE_WIRE_CONTRACTS.md as `pre-release`), which the host writes in one durable write. Two documents
written at two moments could disagree about an envelope (released by the endpoint and recorded in
its duplicate window, but in neither session's queue), and that envelope would be lost without
anyone being told.

**An envelope leaves only after the checkpoint that produced it is durable.** A host core that
persists a root holds the envelopes a drive produced in the endpoint's outbound buffer, takes the
checkpoint, and releases them to the transports when the host reports that the checkpoint is
durable; it may take one checkpoint for several drives, which delays their envelopes by as much. A
restore resumes from the last durable checkpoint and releases the envelopes it holds as unsent, with
their saved ids and sequence numbers; a peer that already received one drops it as a duplicate
(§10.5) or as a sequence it has passed (§10.6.4). So a peer never holds an envelope that the restored
tree does not know it sent, and a restored endpoint never stamps a sequence number a peer has
already seen. This closes, for a host that persists, the case ADR 0043 left open ("a stream that
starts over when a process does") without the `(source, routing_id)` keying SCE_MESH.md §10.9
invariant 5 keeps closed. A host core that does not persist sends at once, as today, and has no
checkpoint to restore from; it does not offer a checkpoint without holding envelopes back, because
a checkpoint taken after its envelopes left restores a tree whose next envelopes its peers drop.

**Which machines are checkpointed is declared in the deploy.** A top-level machine whose host core
checkpoints it says so in deploy.yaml, `persistence: checkpoint` beside its `bindings:` (absent: not
checkpointed, today's behaviour); its `invoked_by` children are checkpointed with it, since they are
in its tree. The store is still the host's (it hands the core where and how to write durably); the
declaration is the deployment's, because section 5's durable receipt is a promise between two
machines that the build can only check if it sees that both of them checkpoint.

**What arrived after the checkpoint is reported, not silently lost.** SCE_MESH.md §10.4 asks a
transport for at-least-once delivery, "successful receipt or an `error.communication` event". A
transport meets that up to the receiving process; an envelope the endpoint admitted after the last
durable checkpoint is lost with the process, and its sender, whose transport saw it received, is not
told. On a binding with the default receipt (section 5) a restore cannot know whether that happened,
so it says that it may have: a restored endpoint raises, in each restored session that holds state
for a peer source, one `error.communication` with `reason` `"RESTORED"` and the `source` in
`_event.data` (row 17 of SCE_MESH.md §16.7, after the `ORDERING_GAP` row it resembles), and a document
that keeps a shared state asks that peer again (the list's resync). A checkpoint the host core took
after it stopped admitting (an orderly shutdown, the links closed first) is marked `clean`, and its
restore raises nothing. A source whose every route to the restored machine has durable receipt is not
reported either: nothing it sent can have been lost (section 5).

The `routing_id` (§10.9 invariant 1) is the new router's, generated at construction as always.
Presence is not saved: attaching a restored session declares it again (ADR 0043 section 5). A Mesh
request in flight is interrupted, as §2.15 already decides for every session
(docs/SCE_ACCEPTED_SUBSET.md:5162-5179).

**Trust state moves forward only.** Keys and MLS state are never in a checkpoint (both documents are
plain JSON a host may store anywhere) and are never rolled back: MLS forbids reusing an epoch's
secrets, and a revoked member must stay revoked. A restored tree therefore runs under trust state at
least as new as the checkpoint, which is the combination that works: the list reads what peers seal
now and resyncs what it missed. A checkpoint holds a list's records in the clear, as the saved state
already does, so a host stores it encrypted at rest under its own key; SCE does not do that for it.

### 5. Two receipt guarantees, chosen per binding

**The owner's choice (2026-10-20).** Both guarantees are SCE's, and a deployment chooses one per
binding in deploy.yaml:

- **Receipt as the transport reports it** (the default, no setting): today's SCE_MESH.md §10.4
  contract, "successful receipt or an `error.communication` event", with a restore reporting a
  possible loss as `"RESTORED"` (section 4).
- **Durable receipt** (opt-in per binding): the receiver acknowledges an envelope on the wire only
  once a durable checkpoint holds it, and the sender keeps the envelope, and sends it again, until
  that acknowledgement arrives. Nothing the receiver was sent is lost across a crash and a restore of
  either side.

**Why both.** SCE's domains need both. Deadline-bound and loss-tolerant traffic (a brake command
that is stale in milliseconds, a sensor sample the next one replaces, a game state update) must not
pay a durable write and a round trip for every envelope. Traffic with no recovery of its own (a
command that happens once, a write no peer can give back) must not have to build its own
acknowledgement protocol in its documents, which every application would then write differently. A
protocol that recovers by itself, such as the shared list's resync, may use either. That it is a
deployment choice and not a document's follows SCE_MESH.md §5: QoS is a deployment concern, declared
per binding in deploy.yaml only, and the same document runs under either.

**The key.** Per binding, `receipt: transport | durable`, default `transport`. It sits beside
`ordering:` and `codec:`, the binding keys the Mesh core realises itself, and not under `qos:`, which
holds transport-native settings passed through to the transport. As with `ordering:`, the binding
key says whether, and an optional per-machine block says how:

```yaml
topology:
  phone:
    machines:
      alice:
        invoked_by: { machine: planner, invoke: list }
        receipt:                      # optional; both fields required when present
          window: 256                 # envelopes held unacknowledged per target, >= 1
          retry_ms: 1000              # resend a held envelope after this long, >= 1
        bindings:
          "#server":
            transport: zenoh_inbox
            receipt: durable          # default: transport
      planner:
        source: planner.scxml
        persistence: checkpoint       # section 4; required at both ends of a durable binding
```

The defaults when the block is absent are 256 and 1000, constants of `sce-build/src/mesh/deploy.rs`
as the ordering defaults are. A durable binding requires `persistence: checkpoint` on the machine
that sends over it and on the machine it reaches (for an `invoked_by` child, on the top-level
machine whose tree holds it): a receiver that never checkpoints could never acknowledge, and a sender
that never checkpoints would forget what it holds the first time its process died.

**Native durability does not stand in for it.** No in-tree transport acknowledges an envelope
against the receiver's checkpoint. DDS durability (`TRANSIENT_LOCAL`, `PERSISTENT`) and Zenoh
storages keep samples for a reader that joins late, in the writer or the infrastructure; TCP and a
WebSocket acknowledge what the receiving kernel has. Those are different promises and stay where they
are (the transport's own settings), composing with `receipt: durable` independently: a binding may
carry both. The transport registry (SCE_MESH.md §10.4.2) gains two fields mirroring the ordering pair:
`supplies_durable_receipt` (the transport acknowledges against the receiver's checkpoint itself, so
the Mesh core does not; `false` for every in-tree transport) and `receipt_representable` (the
transport can carry an acknowledgement from the receiver back to that one sender; `false` for CAN,
whose broadcast has no per-sender return path, as it has no per-receiver sequence).

**On the wire: one new pattern, and a sequence number on every durable envelope.** Nothing else
changes.

- A durable binding stamps `sequence_no` (CBOR key 14, §10.6.3) on every envelope, whatever
  `ordering:` says. The key and the per-target counter exist; only the rule for when it is stamped
  widens. The counter is the one section 4 saves.
- The acknowledgement is a new pattern, `DurableReceipt = 22` (receiver to sender; 22 is the first
  unused value, SCE_MESH.md §13 "Adding a variant requires a new wire value, never reuse"): `id` a
  fresh UUID v7, `source` the receiver's name, `type` the empty text, `datacontenttype` `None`, `data`
  empty, `routing_id` as on every envelope, and `sequence_no` the acknowledged mark: every envelope
  of the stream from the addressee to `source` numbered at most this is held by a durable checkpoint
  of `source`. It is cumulative, so a lost acknowledgement is repaired by the next one, and it is
  idempotent, so it needs no acknowledgement, no sequence number of its own and no duplicate
  suppression. A receiver knows which streams to acknowledge from the deploy, as it knows which are
  ordered (§10.6.3: "topology guarantees"), so no envelope carries a flag asking for it.
- The envelope schema stays a CBOR map with no new key; a reader that does not know pattern 22 is a
  peer the build did not deploy on a durable binding. SCE_MESH.md §13's key table and pattern enum and
  §9.6.2's list of assigned values gain the entry.

**How it composes.**

- **With the checkpoint rule.** On the sender the two holds follow one another: an envelope is
  produced by a drive, held until the checkpoint that holds the drive is durable (section 4), released
  to the transport, and then held again until a `DurableReceipt` covers it. The first hold means a
  receiver never holds an envelope its sender's restore does not know it sent; the second means a
  sender never forgets an envelope its receiver's restore does not know it received. Every held
  envelope is in the sender's endpoint section, with its id and sequence number, so a sender restored
  sends again exactly what was not yet acknowledged.
- **On the receiver.** For each durable source the endpoint keeps `held`, the highest sequence
  number below which it has admitted every envelope, and `durable`, the value `held` had at the last
  durable checkpoint, which the endpoint section saves. When the host reports a checkpoint durable,
  the endpoint sends a `DurableReceipt` to each durable source whose mark moved. An acknowledgement
  reports a checkpoint that is already durable, so it does not wait for one. A restored receiver goes
  back to the `durable` it saved; what it admitted after it is still held by its senders (never
  acknowledged) and is sent again.
- **With duplicate suppression.** On a durable stream a duplicate is told by its sequence number:
  one at most `held` is dropped and answered with the current acknowledgement, so a sender whose
  acknowledgement was lost is answered. This is exact and unbounded, where the §10.5 window (which
  still runs) is a ring a long retry can outlast.
- **With ordering.** With `ordering: required` the §10.6.4 buffer releases a durable stream in
  sequence order, but does not fast-forward past a gap at `gap_timeout`: on a durable stream a gap is
  an envelope its sender still holds and will send again, not a loss, so `ORDERING_GAP` is never
  raised for it. With `ordering: none` envelopes are released as they arrive (§10.1) and `held` counts
  the contiguous prefix.
- **With retry and readiness.** The sender sends a held envelope again (same id, sequence number and
  data) after `retry_ms` without an acknowledgement covering it, and at once at each readiness edge
  of the link (§10.10, ADR 0043 section 5). The held set is bounded by `window`: an admit while
  `window` envelopes are held is refused with `error.communication` `BACKPRESSURE_DROP` (row 9), so a
  send that was never accepted is told and nothing accepted is dropped. The §10.10 age bound
  (`max_age_ms`, `OUTBOUND_STALE_DROP`) does not apply to a durable binding's envelopes: dropping one
  for age would break the promise, and traffic whose envelopes go stale is what the default receipt is
  for. A peer that is gone is reported as it is today (`PEER_PARTITIONED`), and its envelopes stay
  held for it.
- **With a restore report.** A source whose every route to a restored machine is durable gets no
  `"RESTORED"` (section 4).

**Which cores carry it.** The two host cores, `sce-rust-mesh` (over `wss` and `zenoh_inbox`) and
`sce-kotlin-mesh` (over `wss`), with the checkpoint they gain in this ADR. A durable binding the
machine's backend has no core for is refused at build time by name, as ADR 0043 refuses what a core
does not carry: the generated C++ `TransportRouter` until its own step lands, and a transport whose
registry entry is not `receipt_representable`. A Go, Python or C11 host that runs its own router
(SCE_MESH.md §19, "the host's own") is told by the manifest which of its bindings are durable, as it
is told it needs a router at all, and carries the contract itself.

**What the build refuses** (each a diagnostic code of its own, entered in the acceptance appendix
when the parser that raises it lands, order of work step 6): a durable binding whose sending or
receiving machine is not `persistence: checkpoint`; a durable binding on a transport that is not
`receipt_representable`; a durable binding the machine's backend has no core for; a `receipt:`
machine block with no durable binding, or with a field below 1 (the `outbound_buffer:` precedent);
`persistence: checkpoint` on a machine whose document has no saved state (section 1, "Which
documents save").

### 6. The JSON schema: a version step

The saved state becomes **format 2**, `schemas/sce-saved-state.v2.schema.json`, `pre-release`. A
root is `{ "format": 2 }` and a session; a session is today's members with `invokes` as an array of
child entries in document order and with `invokestarts` and `sessionid`; a child entry is `id`,
`start`, `invokeid`, `document` and `session`; `pending` gains `parent` and `child`. The child's
`session` is an object of the same grammar, not a text, so the shared instances stay one canonical
text with no escaping inside escaping and the schema validates the tree to its leaves.

Every backend WRITES format 2, and READS formats 1 and 2. `sce-saved-state.v1.schema.json` stays, its
header naming it a format that is read and no longer written, so the instances older builds wrote
are still validated. SCE_WIRE_CONTRACTS.md's row names both files and both producers' status
constants for each.

Why a step and not an addition: the policy lets a `pre-release` surface change without one
(SCE_WIRE_CONTRACTS.md:149-153) and asks it to grow additively where it can (ADR 0044, "format: 2"
alternative), but a saved state is the one surface whose consumer is a LATER build reading an
EARLIER text, often on a device, which cannot pin a commit as the policy tells consumers to. The
change of `invokes` from ids to sessions is a change of grammar, and `format` is what a reader checks
first: a build that predates trees refuses a format-2 text by its format, instead of reading part
of it, and a build that has trees knows a format-1 text promises no more than it holds. An additive
member beside `invokes` would have named every running child twice.

### 7. Every backend saves

The owner asked for the design that lasts longest. A tree saved by one backend restores on another,
as a single machine does today (docs/SCE_ACCEPTED_SUBSET.md:4960-4964), and ADR 0042's list runs on
every lane; a format that four of six generated backends cannot read is a cross-backend promise
with a hole in it. So **this ADR makes Go, Python, C++ and C11 save**, the whole format at once
(single machines and trees), held to the same shared instances byte for byte:

- **Go and Python** as Rust and Kotlin do: a `Save` / `save` and a `Restore` / `restore` on the
  generated machine over a runtime module that reads and writes the format, with the wall-clock forms.
- **C++** on the generated static engine, the runtime half in `sce/include/static`.
- **C11** with no allocation: the generator states the largest text a machine's tree can save when
  every value and queue is bounded (`sce-static` bounds them through `sce:limit` and the queue
  capacities), the host hands a buffer of that size, and a reader that walks the text in place; a
  value the generator cannot bound makes the save take the host's buffer and refuse a text that
  does not fit, naming the session.

**The Interpreter does not save.** A saved state is a property of `sce-static`, whose typed
variables are what its shape is computed from; the Interpreter refuses `sce-static` at the root and
runs the lowered ECMAScript document, which has no shape (docs/SCE_ACCEPTED_SUBSET.md:5211-5227). Its
scenario driver skips a scenario that saves by name, counted, as the `visualizer-layout` lane skips
the ones it cannot run, and never passes it.

### 8. Fixtures, scenarios and tests

- **A tree fixture.** `static_invoke_tree.scxml` with a child and a grandchild, and
  `saved/static_invoke_tree_working.json`: the child has taken one event (its progress is the
  point), holds a delayed send to `#_parent`, and has a host-run invocation running; the parent holds
  a delayed send to the child and an event from the child in its queue; the child's queue holds an
  event from the parent; the grandchild runs under a hybrid candidate whose `srcexpr` variable was
  changed after it started; one invoke has no literal id and stores its generated id in an
  `idlocation`; one invoke has been started twice. The root's id is given to the machine when it
  is created, as a host may give it, so that the text is one on every backend and no test reads a
  drawn id's value. Every backend saves that text byte for byte and restores from it, and every
  backend's restored tree then runs the same scenario steps.
- **The old format.** Today's shared instances move to `saved/format1/` unchanged and every backend
  restores them; the format-1 `static_invoke_working.json` restores with its child started from the
  beginning, and that is the only test in which a child restarts.
- **Refusals.** Each refusal of section 3 step 1 has a test on each backend that starts from a valid
  tree and changes one thing, among them a grandchild of another shape (the reason names its path),
  a `start` that is not its invoke's count, a candidate the hybrid `<invoke>` does not list, and a
  child of a later schema revision.
- **The scenario format** gains a step member `restore`: the driver saves the machine, discards it,
  and restores it from the text before the step's event or time, so a scenario states what the
  restored tree does next. The six generated drivers run it; the Interpreter's skips it, counted.
- **The existing loss tests are turned around**: `a_child_that_had_taken_an_event_is_started_over_not_resumed`
  and its Kotlin twin become tests that the child resumes with its event taken, and the hybrid
  restore tests (`a_static_hybrid_invoke_is_started_again_by_a_restore.rs`,
  `AStaticHybridInvokeIsStartedAgainByARestoreTest.kt`) become tests that the running candidate is
  resumed with the arguments it was handed.
- **The schema.** `sce-build/tests/saved_state_schema.rs` validates every format-1 instance against
  v1 and every format-2 one against v2, and the negative case changes one thing in a tree.
- **The host cores.** A `sce-rust-mesh` test kills a host core between a drive and the durable
  checkpoint and shows that no peer received an envelope of that drive; another restores after the
  checkpoint and shows that the peer drops the re-released envelope once and takes the next one;
  a third restores a checkpoint and shows one `"RESTORED"` per peer source in each session, and
  none for a `clean` one; a world test restores a one-member list from a checkpoint with a write it
  had not shared and shows the write reach a second member that joins afterwards.
- **Durable receipt.** Over each binding a core carries it on: a receiver killed after it admitted an
  envelope and before its checkpoint was durable is restored and still receives the envelope, once,
  because its sender held it; a sender killed while it held an unacknowledged envelope is restored and
  sends it again, and the receiver takes it once and acknowledges it again; an acknowledgement lost on
  the link is repaired by the next; an ordered durable stream with a gap waits for the resend and
  raises no `ORDERING_GAP`; an admit past `window` raises `BACKPRESSURE_DROP`; and the same scenario
  over a default binding loses the envelope and raises `"RESTORED"`. The deploy parser's refusals of
  section 5 each have a test, and the §8.2 matrix test renders the two registry fields.

## Alternatives not taken

- **The list keeps its records through its host.** The host would persist the list's records and
  hand them back to a restarted list. Rejected: the list's state is not only its records (the clock,
  the tombstones, the settle position, the roster, the digest), so the host would have to hold the
  list's whole state in its own terms, outside the statechart every backend generates and every
  scenario judges, which is what ADR 0042 moved out of host code; every application would meet it
  again for every other long-lived child; and the host has no handle on an engine its parent owns
  (ADR 0043, "Registration after the child starts").
- **Restarting children (today's rule).** Rejected: it is the loss, permanent where no peer holds a
  copy, and it makes the length of a child's life something an author has to design around.
- **The host saves each child separately.** One document per session, written by the host.
  Rejected: the host holds no child's engine; the documents are written at different moments, so an
  event between a parent and a child can be in neither or in both; a restore has to rebuild children
  through their parents in any case (the link, the host, the router); and N writes cannot be one
  durable act.
- **A `children` member beside `invokes` in format 1.** Additive, but every running child would be
  named twice, and a reader that predates it would ignore it and restart the children, silently
  dropping what the text holds. Rejected for format 2.
- **The child's saved state as a text inside its parent's.** Rejected: escaped JSON inside JSON is
  not one canonical text across six writers, and the schema could not see into it.
- **One hash over the whole tree.** Rejected: it cannot say which session changed, and ADR 0044's
  lineage is judged per session.
- **Restarting a child whose document changed shape and resuming the rest.** Rejected: it is a
  partial restore, the loss §2.15 refuses for a root, happening on the day an application updates.
- **The endpoint state inside the saved state.** Rejected: the generated machine would read state
  the network changes on other threads at a moment it cannot fix, and an endpoint outlives sessions.
- **A checkpoint without holding envelopes back.** Rejected: a restored endpoint then stamps
  sequence numbers its peers have passed (dropped, SCE_MESH.md §10.6.4) or sends again what its peers
  hold under new ids; either way a peer's view and the restored tree disagree without anyone told.
- **Keys and trust state in the checkpoint.** Rejected: both documents are plain JSON a host may store
  anywhere, ADR 0034 keeps keys in the host, and trust state must never move backward.
- **Saving on Rust and Kotlin only.** Rejected for the owner's stance: the cross-backend promise of
  §2.15 would hold for two lanes of six, and the standard list runs on all of them.
- **Durable receipt for every binding.** Rejected by the owner: deadline-bound and loss-tolerant
  traffic would pay a durable write and a round trip per envelope for a promise it does not want.
- **Only the reported loss (`"RESTORED"`), with no durable receipt.** Rejected by the owner: traffic
  that cannot recover by itself would have to carry an acknowledgement protocol in its own documents,
  once per application and differently each time.
- **The choice in the document** (an attribute on `<send>`). Rejected: SCE_MESH.md §5 removed QoS
  from documents, so that one document runs in several deployments.
- **A flag on each envelope asking for an acknowledgement.** Rejected: the receiver already learns
  from the deploy which of its streams are ordered, and a durable stream is a property of the same
  binding; a flag would be a second statement of it that two builds could disagree on.
- **An acknowledgement per envelope, by its id.** Rejected: the cumulative mark over `sequence_no` is
  one envelope for any number of envelopes, repairs a lost acknowledgement with the next, and gives an
  exact duplicate test that a bounded id window does not.
- **Transport-native durability as the receipt** (DDS `PERSISTENT`, a Zenoh storage). Rejected: it
  keeps samples for readers that join late and knows nothing of the receiver's checkpoint, so an
  envelope a crashed receiver had admitted is not sent to it again.

## Consequences

- A child holds its progress across a restore: a list with one member keeps its records, and an
  offline write survives the process that made it.
- Every session of a restored tree keeps its session id, its starts and its children's invoke ids.
- Every root has a session id that is an address (answer 2): a `sce-static` root no longer has the
  empty id, and the four backends that draw one today draw it in the one spelling. A host that
  restores one saved state twice at once has two live sessions of one id, and owns that duplicate:
  a restore binds the saved id because the session continues, and SCE cannot see a second copy.
- §2.15's sentences on lost progress, on a hybrid candidate read again and on the child's host asked
  because the child starts again are replaced; the host of a child is still asked on a restore,
  since the one it answered is gone with the process, and now answers a child that resumes.
- Two kinds of delayed send that removed the save API (to `#_parent`, to a child) no longer do.
- A parent whose child cannot save cannot save; today it could, by losing the child.
- An application that changes a child's document beyond ADR 0044's additions loses the ability to
  restore states saved before, as a change of its own document does today; the standard statecharts
  that save are pinned so that SCE does not do this to an application.
- A host core that persists delays every envelope by one durable write (or by its checkpoint
  interval), and a peer sees nothing a restore takes back.
- Every saved-state text changes (format 2), every shared instance is rewritten, and every committed
  generated tree that saves moves with the templates.
- Go, Python, C++ and C11 gain a saved state, which the docs' "is not saved" sentences lose.
- deploy.yaml gains `persistence: checkpoint` per machine, `receipt:` per binding and a `receipt:`
  block per machine; the transport registry gains two fields; the envelope gains one pattern value,
  22, and no key. A durable binding costs its sender a held copy of every unacknowledged envelope and
  its receiver one acknowledgement per checkpoint per source.
- §16.7 gains row 17, `"RESTORED"`, and `ORDERING_GAP` is no longer raised on a durable stream.

## What this does not decide

- **Host-run work and host actions after a checkpoint.** A tree restored from a checkpoint older than
  its last drive repeats that drive's `<sce:action>`s; that is the host's, as it is for a single
  machine today.
- **Migration across a non-additive change of shape**, which §2.15 leaves to a document that can
  declare one.

## Order of work

Each step lands with a test that fails without it.

1. **Documentation and the schema.** §2.15 states format 2 (trees, the two new pending acts, the
   scenario member `restore`) as the format the backends move to, and says plainly which format each
   backend writes until its step lands; `sce-saved-state.v2.schema.json` and the v1 header (a format
   read and no longer written once step 4 lands); SCE_WIRE_CONTRACTS.md's row and the v2 file in
   `wire_surface_stability.rs`; the v1 schema's `invokes` description corrected on the way (it says a
   document with a mesh-rpc call has no saved state, while §2.15 and
   `invokes_what_a_restore_cannot_start`, static_lowering.rs:3557-3583, save it and interrupt it).
   SCE_MESH.md states the checkpoint, the `"RESTORED"` row, `persistence:`, `receipt:`, pattern 22
   and the two registry fields, each marked as not yet parsed or carried, so that a deploy naming
   `receipt:` or `persistence:` is still refused as an unknown key and no build promises a receipt
   nobody keeps. The checkpoint's schema and the deploy diagnostics land with their producer and their
   parser in step 6, because a schema no artifact is validated against is a document and not a
   contract (SCE_WIRE_CONTRACTS.md policy 5). *Exit:* `saved_state_schema.rs` validates a hand-written
   tree against v2 and refuses one changed thing, and every format-1 instance still validates
   against v1.
2. **The generator.** Which documents save (the child rule), the two acts, `invokestarts` and the
   session id in the model the templates read, and the ids of a child session and of a generated
   invoke derived from the tree on every backend (the spelling keeps the state's id and a dot in
   front, the form W3C test 224 reads from the `idlocation`), and the format-2 shape: an enum-typed
   variable or record field spelled by ADR 0044's `document_key` instead of `enum:<alias>`, emitted
   beside the format-1 shape and its revised forms, the key text computed once by the generator and
   identical in every backend's constant. *Exit:* `a_machine_waiting_on_another_session_has_no_save_api`
   extended: a delayed send to `#_parent` or a child now saves, one to a session outside the tree
   still does not, and a parent of a child that cannot save has no API; a lowering test shows that
   renaming an enum import's alias leaves the format-2 shape as it was and moves the format-1 shape,
   and that moving the enum's file moves the format-2 shape and the enum's key (the format-1 shape
   names no document and stays); and a cross-backend test shows the six backends' generated
   format-2 shape constants and enum keys equal as text.

   *Where it stands.* Done: the child rule (`a_tree_can_hold`), the two sends that no longer
   remove the API (a format-1 save refuses with a reason while one waits), a per-invoke start
   count in every generated parent, the generated invoke id `<state>.<start>.<invoke>` on all six
   backends, child session ids `<parent>.<invokeid>` on Rust, C++, Go, Python and Kotlin (a
   Kotlin child is given its id where its parent builds it, `adoptChild`, so a script session it
   opens to take an argument is named by it, and a start refuses a child given no id), a test that
   pins the shape of every standard statechart that saves, and the format-2 shape: the generator
   hashes the format-1 and format-2 texts of `saved_shape` in one walk (`SavedShape`'s
   `format2_first` and `format2_revised`), Rust's and Kotlin's `SavedShape` constants carry both,
   and Go, Python, C++ and C11, which do not save yet, declare the format-2 shapes and enum keys as
   constants of the machine (`static_format2_shape`). Step 2 is done but for two ids that land
   later: the C11 child session id, whose buffer sizes are decided with C11's saved state (step 5),
   and the id of a `sce-static` root, empty today on Rust and Kotlin and drawn by the process in
   another spelling on Go, Python, C++ and C11, which is decided ("Questions the owner answered" 2)
   and lands with each backend's save: Rust in step 3, Kotlin in step 4, the other four in step 5.
3. **Rust.** `SavedState` as a session tree, format 2 written, formats 1 and 2 read, the restore of
   section 3, the transit check, the tree fixture, and the root's session id (drawn or given at
   creation, saved, bound again by a restore). *Exit:* the Rust tests of section 8.

   *Where it stands.* Done: `SavedState` holds `sessionid`, `invokestarts` and its running
   children as `SavedInvokes::Resumed` (format 2) or `Restarted` (format 1), and writes the
   format it is of; the runtime reads both, and a refusal inside a child names its session's path
   (`StateRefusal::within`). `save` writes format 2 with the format-2 shape, refuses a machine
   with no id and a save whose children are not the ones the machine runs, and holds a delayed
   send to the parent or a child as `parent` / `child`. A restore judges the root, then each child
   by its own document through `<Machine>Persist::judge` (`StatePolicy::judge_child`, to any depth)
   before anything is built, binds the saved root id, carries the counts on, and resumes each child
   as its start builds it (`do_resume_child`, sharing `build_child_policy` and `track_child` with
   the start) in place of initializing it; a format-1 state still starts its children again. The
   root's id is drawn when the policy is built (`new`; `helpers::session_id`, `getrandom`) or
   given where it is built (`new_with_session_id`), refused there when it is not an Nmtoken; on
   `wasm32-unknown-unknown` and `no_std`, which have no source of randomness, `new` is not
   compiled, so the host gives the id as a required argument, and a `no_std` policy holds the
   `&'static str` it was given (8 bytes of RAM on `thumbv7em-none-eabihf`; the nostd-mcu footprint
   budget weighs the constructor's check); a parent builds each child under its tree id
   (`for_child`), and nothing names a session by a counter any more. The shared texts moved to
   `saved/format1/` and `saved/` holds the format-2 texts Rust writes, each root given
   `fixture-root`; the loss tests are turned around (`a_saved_child_session_is_resumed_after_a_restore.rs`,
   `a_static_hybrid_invoke_is_started_again_by_a_restore.rs`, the child's host and params tests),
   with format 1 kept as the one case a child restarts. A child's replies to a drive of it, a
   delivery included, reach its parent within the drive (section 3), and
   `nothing_is_in_transit_when_a_tree_is_saved.rs` holds that a save after any host call never
   meets the transit refusal. A resumed child's Mesh router is asked of the factory when the host
   registers it (step 3 of section 3), held by
   `a_child_session_is_given_its_mesh_router_by_its_parent.rs`. A child session's host-run
   invocation is served by the tree's invokers and ended by session id (section 3,
   `a_child_sessions_host_invocation_is_served_by_the_trees_host.rs`), and a child session's
   `<invoke idlocation>` is written under `sce-static` (section 3). The tree fixture
   `static_invoke_tree.scxml` holds every item of section 8 (a grandchild under a hybrid candidate
   whose `srcexpr` variable changed, a delayed send each way, an event queued each way, a
   host-run invocation running in the child, an `idlocation`, a start counted twice, and a
   record of a schema at its third revision in the child); `a_saved_tree_holds_every_session_where_it_stood.rs`
   holds what each session stood at, its round trip, and the refusals of a grandchild of another
   shape and of a child of a later revision, each naming the session's path; the scenario format
   has its `restore` step, run by the Rust replay (`scenarios/static_invoke_tree.json`) and
   skipped, counted, by the Interpreter. Open: the shared text `saved/static_invoke_tree_working.json`
   itself, which Rust writes first: the test that pins it writes it under `UPDATE_EXPECT=1`, and
   the text is committed from that run once read.
4. **Kotlin.** The same over `SavedState.kt` and `StateMachineEngine`, the root's id included.
   *Exit:* Kotlin saves the tree fixture byte for byte as Rust does, Kotlin restores the text Rust
   saved, and Rust the text Kotlin saved.

   *Where it stands.* Done: `SavedState` holds `sessionId`, `invokeStarts` and its running
   children as `SavedInvokes.Resumed` (format 2) or `Restarted` (format 1), writes the format it
   is of with that format's shape, and reads both; the engine binds the root's id (given as the
   last constructor parameter of a generated `sce-static` machine, or drawn), saves each running
   child whole, judges every child down to the last grandchild before it builds one
   (`buildSavedChild`, `judgeAsSession`) and resumes each, owing the factory of children's routers
   its router until one is registered; `parent` and `child` delayed sends are acts of the save;
   the tree's host invokers are one registry the children share, a request names its session
   (`HostInvokeRequest.sessionId`) and an ending is routed by it (`completeHostInvokeIn`,
   `failHostInvokeIn`); an `<invoke idlocation>` of a child session is written. The tree fixture
   `saved/static_invoke_tree_working.json` is saved byte for byte, and restored, by
   `ASavedTreeHoldsEverySessionWhereItStoodTest`, whose run is Rust's; the cross-language world
   test (`a_kotlin_copy_sends_and_holds_what_a_rust_copy_does`) holds a Kotlin copy's save to a
   Rust copy's, each given its actor's name as the root's id. The scenario format's `restore`
   step runs on Kotlin (`staticInvokeTreeRestoresBetweenItsSteps`). Two things moved with it
   that the Rust backend already did: an event the host raises by name is an external event of
   the SCXML processor (`origintype` is set), and `_event.invokeid` of a child's event is the id
   the start was made under only for an `<invoke>` that keeps it in an `idlocation`, the
   document's own id otherwise (W3C tests 338 and 228). Open: a send to a session by its id
   (`#_scxml_<id>`) still removes the save API. The Kotlin parent has no buffer a child's event
   waits in, so it has no `nothing in transit` refusal to reach;
   `ASavedTreeHoldsEverySessionWhereItStoodTest` holds that a save after any raise or tick
   restores and saves again byte for byte.
5. **Go, Python, C++, C11**, one backend per step, each reading and writing the whole format, the
   root's id in the one spelling of answer 2 in place of the one its process draws today, and held
   to every shared instance. *Exit:* per backend, every instance restored, the tree fixture saved byte
   for byte, the scenario `restore` steps run by its driver.

   *Where it stands: Go.* Done: the runtime holds the format (`saved_state.go`, over an ordered JSON
   reader that keeps a number's spelling, `saved_json.go`) and reads and writes both formats; the
   25 shared texts read and write back byte for byte, and a refusal inside a child names its
   session's path. `SaveSession`, `JudgeSession` and `EnterSession` (`engine_saved.go`) do what the
   Rust `save`, `judge` and `enter` do, over a `SavedPolicy` a generated policy adds to
   `StatePolicy`; a generated `<Machine>SaveAt`, `Judge` and `RestoreAt` read and write the
   variables, the histories and each running child, and a record or an enum writes its own saved
   form (`ToSaved`, `<Type>FromSaved`). A root draws its session id as a UUID version 4 (or is given
   one, `SetSessionID`, held to an XML Nmtoken) and a restore binds the saved one. A child is
   resumed in place of started, its router owed to the factory until the host registers it
   (`OweChildMesh`), and a format-1 child is started again at the next drive. The tree's host
   invokers are one registry its sessions share, a request names its session
   (`HostInvokeRequest.SessionID`) and an ending is routed by it (`CompleteHostInvokeIn`,
   `FailHostInvokeIn`); a restored host-run invocation starts again at the first drive with the
   deadline it had left, and one cancelled before that is never started. What a child sends its
   parent while the parent delivers to it reaches the parent within the delivery
   (`SettleChildren`), so a save at any boundary finds nothing in transit; and an event a child
   sends its parent is an external event (it was typed internal, a defect of the Go drain the
   shared tree text found). An `<invoke idlocation>` of a child session is written. Held by
   `static_invoke_tree` (the tree fixture saved byte for byte, restored, refused by path, and
   its `restore` scenario run by `static_scenarios_test.go`), the child resume, hybrid restart,
   send id, host invocation, delayed host send and Mesh request tests, each a port of Rust's.
   Open: the mutation round over the new machines, and a send to a session by its id.

   *Where it stands: Python.* Done: the runtime holds the format (`saved_state.py`, over
   an ordered JSON reader that keeps a number's spelling, `saved_json.py`, and the
   per-type value readers and writers of `saved_values.py`) and reads and writes both
   formats; the 25 shared texts read and write back byte for byte, and a refusal inside
   a child names its session's path. `save_session`, `judge_session` and
   `enter_session` (`saved_engine.py`) do what Go's `SaveSession`, `JudgeSession` and
   `EnterSession` do, over the hooks a generated policy adds (`running_invokes`,
   `invoke_starts`, `invoke_owner`, `host_invoke_owner`, `judge_child`,
   `resume_children`, `restart_invokes`); a generated module's `save_at`, `judge`,
   `restore_at` and `restore` read and write the variables, the histories and each
   running child, and a record or an enum writes its own saved form (`to_saved`,
   `from_saved`). A root's session id is a UUID version 4 in the one spelling (the
   engine draws it, or `create_engine(session_id=...)` gives it, held to an XML
   Nmtoken) and a restore builds the engine under the saved one. A child is resumed in
   place of started, its router owed to the factory until the host registers it
   (`owe_child_mesh`), and a format-1 child is started again at the next drive. The
   tree's host invokers are one registry its sessions share (`TreeInvokers`), a request
   names its session (`HostInvokeRequest.session_id`) and an ending is routed by it
   (`complete_host_invoke_in`, `fail_host_invoke_in`); a restored host-run invocation
   starts again at the first drive, in the main loop and at the top of `advance_time`,
   with the deadline it had left, and one cancelled before that is never started. An
   `<invoke idlocation>` of a child session is written. Held by
   `test_a_state_another_backend_saved_is_restored`, the tree tests (the shared text
   restored and saved again, refused by path), the scenario `restore` step of
   `static_invoke_tree`, and the child resume, hybrid restart, send id, host
   invocation, delayed host send and Mesh request tests, each a port of Rust's.
   Python drives a child synchronously: an event the parent forwards (or delivers
   with a `<send>`) is taken by the child within the delivery, and its replies are on
   the parent's queue before the call returns, so a tree saved at a boundary never
   finds an event in transit, and the state in which a child's own queue still holds an
   event its parent sent is reached by a restore of the text and not by a run; the tree
   test holds that state by the shared text. A machine records the document-order-first
   atomic state as its current leaf (`Engine.current_state`), as `enter_at` already
   stated, so for a `<parallel>` the text differs from one Rust writes in that member
   alone. Open: the mutation round over the new machines, and a send to a session by its
   id.

   *Where it stands: C++.* Done: the header-only runtime holds the format (`saved/SavedState.h`,
   over the ordered JSON reader and writer of `saved/SavedJson.h`, the per-type value readers
   and writers of `saved/SavedValues.h` and the session id of `saved/SessionId.h`) and reads and
   writes both formats. `SCE::Saved::EngineAccess` (`saved/SavedEngine.h`), a friend of
   `StaticExecutionEngine`, does what Go's `SaveSession`, `JudgeSession` and `EnterSession` do
   over the hooks a generated policy adds (`runningInvokes`, `invokeStarts`, `invokeOwner`,
   `hostInvokeOwner`, `judgeChild`, `resumeChildren`, `restartInvokes`, and the tables of the
   document's histories); a generated machine's `saveAt`, `judge` and `restoreAt` read and write
   the variables, the histories and each running child, and a record or an enum writes its own
   saved form (`<Type>ToSaved`, `<Type>FromSaved`, beside the type, so a record stays plain
   data). A session draws its id as the other backends do, `bindSessionId` gives it, and a
   restore binds the saved one. The tree's host invokers are one registry its sessions share
   (`TreeInvokers`), a request names its session (`HostInvokeRequest::sessionId`) and an ending
   is routed by it (`completeHostInvokeIn`, `failHostInvokeIn`, answering `Delivered`,
   `NoSuchSession` or `NotRunning`); a restored host-run invocation starts again in the first
   macrostep the host drives, with the deadline it had left, and one cancelled before that is
   never started. A child is resumed in place of started, its router owed to the factory until
   the host registers it (`oweChildMesh`), and a format-1 child is started again at the next
   drive. An `<invoke idlocation>` of a child session is written. A document that saves names a
   `<send>` that names no id by the id the parser numbered (`__send_<n>`), so the id a waiting
   send is saved under is the text every backend writes; the scheduler neither replaces nor
   indexes such an id, which no author can name, so two waiting sends of one element both wait.
   A document with a Mesh request has no saved state in C++: the request goes out by the router
   the build generates, which is not a host-run invocation a saved state could hold, so a
   restore could not tell the document it was interrupted. Held by
   `AStateAnotherBackendSavedIsRestoredTest`, `ASavedTreeHoldsEverySessionWhereItStoodTest`,
   `ASavedHostInvocationIsStartedAgainAfterARestoreTest`, and the scenario `restore` step of
   `static_invoke_tree`. Open: the child resume, hybrid restart, send id and delayed host send
   tests as ports of Rust's, and the mutation round over the new machines.

   *Where it stands: C11.* Done, with no allocation: the runtime is header-only (`saved_json.h`,
   `saved_values.h`, `saved_state.h`). A writer counts the bytes that do not fit, so a save run
   with no buffer measures the text and one run with a buffer of that size writes it; a reader is
   a set of spans over the text, and each reader of a value takes no output to only judge. A
   record reads a field a later revision added from the default its schema declares, and an enum
   holder is checked from a table. A restore reads and judges the whole text, then builds the
   machine and applies it (variables, histories, the current state, the queues, the waiting
   sends, the host-run invocations, the children last), so a refused restore leaves the caller's
   storage as it was. A child is resumed in place of started, by the macro its start is built
   with, and a session is known by its parent's id and the id its start was made under
   (`<state>.<start>.<invoke>`, the same text on every backend even where the author wrote an
   `id`, which stays the id the machine's events from the child carry). The tree's host invokers
   are one registry its sessions share; a request names its session and is answered through
   `_complete_host_invoke_in`; a restored host-run invocation starts again at the first drive
   with the deadline it had left. An `<invoke idlocation>` of a child session is written.
   Deviations from the other backends, each recorded in SCE_ACCEPTED_SUBSET.md: a root draws
   `<name>_session_<n>` and has no id before it is created; the save takes the exact size by a
   second call in place of a bound the generator states; a child is driven in the macrostep that
   started it, so an event forwarded to it does not wait in its queue; an event a child sends its
   parent carries no origin or origin type. Held by `c11_unit_saved_state`,
   `c11_integration_static_saved` (every shared instance read and written back),
   `c11_integration_static_host_invoke_saved`, `c11_integration_static_saved_tree`,
   `c11_integration_static_child_saved` (the child resume and the format-1 restart),
   `c11_integration_static_host_send_saved` (a waiting host send), `c11_integration_static_mesh_request_saved`
   (a Mesh request not sent again), and the scenario `restore` step of `static_invoke_tree`.
   Open: the mutation round over the new machines.
6. **The host cores** (after ADR 0043 steps 3 and 5): the checkpoint document, its schema and its
   row in SCE_WIRE_CONTRACTS.md with a producer status constant in `sce-rust-mesh` and
   `sce-kotlin-mesh`; holding envelopes back; the endpoint section saved and restored; the
   `"RESTORED"` raise; durable receipt (pattern 22, the two holds, the receiver's marks, retry and
   window); the deploy parser's `persistence:`, `receipt:` and their refusals as diagnostic codes in
   the acceptance appendix; the two registry fields and the §8.2 matrix. *Exit:* the host core and
   durable receipt tests of section 8.
7. **The world and ADR 0042 step 2:** a parent restored with its list child in the Rust world and the
   cross-language world. *Exit:* the one-member list test of section 8.

The committed generated trees of every language that saves move with steps 2 to 5 (the invoke and
save templates are in the template hash of every language) and are regenerated in those steps.

## Questions the owner answered

1. **Should a host core that persists also keep what it receives?** Asked in the proposal: holding
   envelopes back makes a restored tree's sent view exact, and its received view may miss an envelope
   admitted after the last durable checkpoint, which the principles allowed either to report or to
   keep. Answered 2026-10-20: both, chosen per binding in deploy.yaml, the reported loss by default
   and durable receipt by opt-in (section 5, with the owner's reasons).
2. **What is the id of a root session?** Left open by step 2: a `sce-static` root had the empty id
   on Rust and Kotlin and an id its process drew, in a spelling of its own, on Go, Python, C++ and
   C11, so a tree's ids could not start from one text. Answered 2026-10-21 ("the most long-term
   correct way, cost ignored"):
   - **Drawn by default, never empty.** Every backend binds a system-generated, unique id when the
     machine is created, as §scxml-5.10 binds `_sessionid` at load and keeps it until the session
     ends. The empty text is not allowed: it is not an XML NMTOKEN, which §scxml-5.10 makes the
     id, and it is no address (`#_scxml_<id>`). One spelling on every backend, a UUID version 4 in
     lowercase hexadecimal with hyphens (36 characters), so that no reader can tell which backend
     drew it.
   - **Or given by the host.** A host may give the root's id when it creates the machine (its Mesh
     machine or device identity, say). The id given is held to the XML 1.0 `Nmtoken` production
     (name characters only: letters, digits, `.`, `-`, `_`, `:` and the other characters
     `NameChar` admits, at least one); one that is not is refused when the machine is created,
     with the reason, and never replaced by another. A target with no source of randomness the
     runtime can read (a `no_std` build, `wasm32-unknown-unknown`) has no drawn default: its
     constructor takes the id from the host, as a required argument its API states, as such a
     target already takes the wall clock from the host (§2.15).
   - **Saved and restored.** The root's id is the `sessionid` of the format-2 root, and a restore
     binds the saved id in place of drawing or taking one (section 3, step 2). A host that restores
     one saved state twice at once owns the duplicate it made (Consequences).
   - **Tests do not read a drawn id.** The shared cross-backend fixtures (the tree fixture, the
     byte-for-byte save tests, the scenarios) give the root's id explicitly, so their saved text is
     one on every backend; a test may hold a drawn id to the spelling, never to a value.
   - **Children** keep `<parent>.<invokeid>`, derived from the tree as section 1 states.

No question is open.
