# ADR 0045 — A saved state holds its running children, and a restore resumes them where they were

- Status: Proposed (2026-10-20)
- Date: 2026-10-20
- Scope: docs/SCE_ACCEPTED_SUBSET.md §2.15 ("Saving and restoring", "Running invocations",
  "Child sessions", the scenario format); `schemas/sce-saved-state.v1.schema.json` and a new
  `schemas/sce-saved-state.v2.schema.json`; SCE_WIRE_CONTRACTS.md (the saved-state row);
  `forge::static_lowering::saved_shape` and the decision which machines save; the save and restore
  of the Rust runtime (`sce_rust_runtime::saved_state`, `<Machine>Persist`) and the Kotlin runtime
  (`com.sce.runtime.SavedState`, `StateMachineEngine`); the invoke templates of every backend; a
  saved state for the Go, Python, C++ and C11 backends; the Mesh host cores (`sce-rust-mesh`,
  `sce-kotlin-mesh`) and SCE_MESH.md §19; the shared instances under
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
durable. What the host created and the document cannot name stays the host's.**

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
  per session and never by a restore; a root keeps what it has today (a `sce-static` root with no
  script engine has none, and its events' `origin` is the empty text, as in every shared instance).

**The ids inside a tree are the same on every backend.** A saved state is one text whatever backend
wrote it (docs/SCE_ACCEPTED_SUBSET.md:4960-4964), and a tree holds ids that today are not: Rust
generates an invoke id from the address of the parent's policy
(tools/codegen/templates/rust/invoke_macros.rs.jinja2:49-50) and a session id from a timestamp and a
process-wide counter (backends/rust/runtime/src/helpers/unique_id_generator.rs:115-127). Every
backend therefore derives a child's session id and a generated invoke id from the tree alone: the
parent's session id, the state, the `<invoke>`'s id and the start, in one spelling the format fixes.
Ids are unique within a tree by construction, and a tree is one process's.

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
   (`hostinvoketoken`, `sendseq`, `invokestarts`) carried on.
3. **Each running child, in document order, depth first.** The parent builds the child's engine as
   it does when the invocation starts (the parent's link for `#_parent`, the same clock, the child's
   host obtained from the parent's host as ADR 0005 decision 6 obtains it, and under ADR 0043 the
   child's router from the factory), and then RESTORES it in place of initializing it, by steps 2 and
   3 applied to the child's session: no `<onentry>`, no `<data>`, no `<param>` or `namelist`
   evaluated, no `idlocation` written, nothing sent. The child is registered in the parent under its
   `invokeid` and `start`. The waiting sends of every session are armed against the one wall clock
   the restore was given, so a child's delayed send comes due when it would have.
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
  buffer the parent drains before and after it drives the child
  (tools/codegen/templates/rust/invoke_methods.rs.jinja2:6-21). The tree is saved only at a boundary
  of its root at which every such buffer is empty, which the generated parent establishes by
  draining after every drive of a child; a save that finds one not empty is refused with that reason,
  a defect of the generator and never a timing the host can meet, and a test holds that it is never
  reached.
- **`done.invoke` is not raised again.** A child that had ended is absent and its completion is in
  the parent's queue or already taken; a child that is resumed did not end, and raises its
  `done.invoke` once, when it ends after the restore. Nothing a child did at its start (its
  `<onentry>`, its sends, its host's first acts) happens a second time.
- **A save of a tree is a save of every session at once.** It is refused when any session's last
  macrostep was `truncated`, or on Kotlin when the root is driven by its own coroutine, as a single
  machine's is today.

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
| The endpoint section of a checkpoint | per machine name of the deployment (the root's and each `invoked_by` child's, ADR 0043 section 1): the outbound sequence counter per target (§10.6.3), the duplicate window per sender (§10.5), the ordering state per source, `next_expected_seq` and the envelopes buffered past it (§10.6.4), the envelopes not yet released to a transport or held for retry (§10.10, ADR 0043 section 5), with their envelope ids, sequence numbers and the wall-clock moment each comes due, and what the endpoint released for a session that the session has not yet taken | the Mesh host core (`sce-rust-mesh`, `sce-kotlin-mesh`), or a host that runs its own router | with the saved state, in one checkpoint |
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
**What arrived after the checkpoint is reported, not silently lost.** SCE_MESH.md §10.4 asks a
transport for at-least-once delivery, "successful receipt or an `error.communication` event". A
transport meets that up to the receiving process; an envelope the endpoint admitted after the last
durable checkpoint is lost with the process, and its sender, whose transport saw it received, is not
told. A restore cannot know whether that happened, so it says that it may have: a restored endpoint
raises, in each restored session that holds state for a peer source, one `error.communication` with
`reason` `"RESTORED"` and the `source` in `_event.data` (a new row of SCE_MESH.md §16.7, after the
`ORDERING_GAP` row it resembles), and a document that keeps a shared state asks that peer again (the
list's resync). A checkpoint the host core took after it stopped admitting (an orderly shutdown,
the links closed first) is marked `clean`, and its restore raises nothing. Whether Mesh should
instead never lose such an envelope is open question 1.

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

### 5. The JSON schema: a version step

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

### 6. Every backend saves

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

### 7. Fixtures, scenarios and tests

- **A tree fixture.** `static_invoke_tree.scxml` with a child and a grandchild, and
  `saved/static_invoke_tree_working.json`: the child has taken one event (its progress is the
  point), holds a delayed send to `#_parent`, and has a host-run invocation running; the parent holds
  a delayed send to the child and an event from the child in its queue; the child's queue holds an
  event from the parent; the grandchild runs under a hybrid candidate whose `srcexpr` variable was
  changed after it started; one invoke has no literal id and stores its generated id in an
  `idlocation`; one invoke has been started twice. Every backend saves that text byte for byte and
  restores from it, and every backend's restored tree then runs the same scenario steps.
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
  none for a `clean` one; a world test restores a one-member list from a checkpoint with a write it had not shared and shows
  the write reach a second member that joins afterwards.

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

## Consequences

- A child holds its progress across a restore: a list with one member keeps its records, and an
  offline write survives the process that made it.
- Every session of a restored tree keeps its session id, its starts and its children's invoke ids.
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

## What this does not decide

- **Keeping an envelope that arrived after the last checkpoint.** Section 4 reports that one may
  have been lost (`"RESTORED"`); keeping it is open question 1.
- **Host-run work and host actions after a checkpoint.** A tree restored from a checkpoint older than
  its last drive repeats that drive's `<sce:action>`s; that is the host's, as it is for a single
  machine today.
- **Migration across a non-additive change of shape**, which §2.15 leaves to a document that can
  declare one.

## Order of work

Each step lands with a test that fails without it.

1. **Documentation and the schema.** §2.15 rewritten for trees (the sentences cited in the Context
   replaced, format 2, the two new pending acts, the scenario member `restore`);
   `sce-saved-state.v2.schema.json` and the v1 header; SCE_WIRE_CONTRACTS.md's row; the v1 schema's
   `invokes` description corrected on the way (it says a document with a mesh-rpc call has no saved
   state, while §2.15 and `invokes_what_a_restore_cannot_start`, static_lowering.rs:3557-3583, save
   it and interrupt it). *Exit:* `saved_state_schema.rs` validates a hand-written tree and refuses
   one changed thing.
2. **The generator.** Which documents save (the child rule), the two acts, `invokestarts` and the
   session id in the model the templates read, and the ids of a child session and of a generated
   invoke derived from the tree on every backend (the spelling keeps the state's id and a dot in
   front, the form W3C test 224 reads from the `idlocation`). *Exit:* `a_machine_waiting_on_another_session_has_no_save_api`
   extended: a delayed send to `#_parent` or a child now saves, one to a session outside the tree
   still does not, and a parent of a child that cannot save has no API.
3. **Rust.** `SavedState` as a session tree, format 2 written, formats 1 and 2 read, the restore of
   section 3, the transit check, the tree fixture. *Exit:* the Rust tests of section 7.
4. **Kotlin.** The same over `SavedState.kt` and `StateMachineEngine`. *Exit:* Kotlin saves the tree
   fixture byte for byte as Rust does, Kotlin restores the text Rust saved, and Rust the text Kotlin
   saved.
5. **Go, Python, C++, C11**, one backend per step, each reading and writing the whole format and held
   to every shared instance. *Exit:* per backend, every instance restored, the tree fixture saved byte
   for byte, the scenario `restore` steps run by its driver.
6. **The host cores** (after ADR 0043 steps 3 and 5): the checkpoint document and its row in
   SCE_WIRE_CONTRACTS.md, holding envelopes back, the endpoint section saved and restored, the
   `"RESTORED"` row of SCE_MESH.md §16.7 and its raise on every backend's Mesh error path. *Exit:*
   the host core tests of section 7.
7. **The world and ADR 0042 step 2:** a parent restored with its list child in the Rust world and the
   cross-language world. *Exit:* the one-member list test of section 7.

The committed generated trees of every language that saves move with steps 2 to 5 (the invoke and
save templates are in the template hash of every language) and are regenerated in those steps.

## Open questions for the owner

1. **Should a host core that persists also keep what it receives?** Holding envelopes back makes a
   restored tree's SENT view exact. Its RECEIVED view may miss an envelope admitted after the last
   durable checkpoint, which section 4 reports as `"RESTORED"` so that §10.4's "receipt or an
   `error.communication` event" holds. Never missing it would mean the receiving endpoint
   acknowledges an envelope only once a durable checkpoint holds it, and its sender retries until
   then: delivery that survives a crash end to end, which needs an acknowledgement on the wire (ADR
   0043 changes nothing on the wire) and costs every peer a round trip per envelope. The repository's
   principles accept both a reported loss and a kept envelope; which one Mesh promises is a product
   choice. The list does not need it (resync recovers); a document that cannot ask again would.
