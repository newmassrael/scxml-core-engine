# ADR 0042 - A shared record list is a standard part, and an application writes only its rules

- Status: Accepted (2026-10-20); step 1 in progress
- Date: 2026-10-20
- Scope: a new standard statechart `sce:std/shared/record_list.scxml` and its schemas under
  `stdlib/shared/`; `sce-build/tests/fixtures/static_datamodel/todo_peer.scxml` and the documents
  it imports; `backends/rust/world`; docs/SCE_ACCEPTED_SUBSET.md (standard library), SCE_MESH.md
  where a child session meets a Mesh peer
- Related: ADR 0034 (a shared list is held by its members and the hub only relays),
  docs/SCE_SHARED_LIST_TRUST.md, ADR 0005 (bounding and declaring)
- Amended by: ADR 0046 (a change is one unit, a register can be emptied and a row retired, a
  refusal is told to the parent; the protocol moves on in this part while `todo_peer` stays at
  revision 1, which replaces the rule of section 4 that both copies change at once)

## Context

`todo_peer.scxml` is one copy of a shared list: rows numbered by a uint32, each a set of fields
numbered by a uint16, each field a multi-value register over an observed-remove set
(`merge/orset_*`, `merge/mv_value_*`), stamped by a hybrid logical clock (`merge/hlc_*`), with a
merkle digest to find lost changes (`merge/merkle_*`), a settle that lets the copies forget
tombstones, a room the host gives each list, and a roster of roles. Which field number means what is
the host's word (`todo_write`: "the numbers of the fields are the host's"). So the protocol is
already generic over the application's fields. It is also the document ADR 0034's trust layer, the
world simulation and the cross-language test (a Kotlin copy with Rust copies) are built around.

It is, however, a test fixture, and it has grown parts that belong to one kind of list: a schedule
read from its values (`todo.agenda`, `event_agenda`, `event_occurrence`) and the host's time zones
(`todo.zone`). An application that needs a shared list of its own records has two choices today,
both wrong for the long term:

1. Copy the generic part (some 800 lines) into its own document beside its rules. The protocol then
   exists twice: a fix to it here (an independent review of ADR 0034 is pending and will produce
   fixes) has to be found and repeated in every copy, and a copy that misses one diverges on the
   wire from the copies that have it.
2. Drive this fixture from its host. The host then reads the list's values and decides what each
   command writes, which puts application rules in host code, outside the statechart that every
   backend generates and every scenario judges.

The document an application should be able to use is the generic list, kept here once, which it
starts as a child session and talks to with events, writing its own rules in its own statechart.

## Decision

**The generic shared record list is a standard statechart of this repository, and an application
composes it: the application's statechart invokes it as a child session, turns each command into
field writes it sends the child, and keeps its own derived view from the changes the child
announces.**

### 1. The standard part

- `stdlib/shared/record_list.scxml` (`sce:std/shared/record_list.scxml`) holds what `todo_peer`
  holds today except the schedule and the zones: join, room, role, edit, write, supersede, keep,
  compact, settle, propose, ack, floor, resync, visit, node, share, and their schemas, moved to
  `stdlib/shared/` with names that say what they are (`record_write`, `record_edit`, ...).
- Every bound is a `sce:limit` (docs/SCE_ACCEPTED_SUBSET.md §2.15, "Bounds"): the number of live
  values, tombstones and digest nodes, the roster, and the bytes of a text field a person may write
  (today 32 in `todo_edit`) and of a text a write may carry (today 96, room for a sealed value).
  The defaults are today's numbers, so a build that names no limit behaves as `todo_peer` does.
- A field's value is an `int64` or a bounded text, as today. An application keeps an enum in a
  numeric field by the enum's declared wire number, which is what the number is for; a record is
  spread over several fields of one row.

### 2. What the list tells the statechart that invokes it

Today a host reads `values` from a saved state; a parent statechart cannot, and an event cannot carry
a list. The list therefore announces each register whose live values changed, to its parent
(`#_parent`), after the change is taken in, whatever its source (the person's own edit, a peer's
write or supersede, a resync, a settle that changes nothing live is not announced):

- `record_list.register` with the register (`id`, `field`), how many live values it now holds
  (`count`, 0 when every value was written over), and its first live value in a fixed order (the
  order of their dots), then `record_list.value` for each further live value, with its ordinal, so a
  register written twice at once reaches the parent as both values (ADR 0034's "a person chooses").
- A parent that is not interested in a field ignores its events; a list started by a host instead of
  a parent sends nothing to a parent (`#_parent` is not addressable at the top level, so the send is
  written under a condition the list keeps from how it was started: `announce`, which the
  `<invoke>` hands it and a host does not; see "Revisions made in building it").

### 3. The fixture and the world

- `todo_peer.scxml` becomes the list plus its schedule: a statechart that invokes
  `sce:std/shared/record_list.scxml` and keeps `todo.agenda` and `todo.zone` over the values it is
  told. The world simulation, the scenarios and the cross-language test keep driving the same event
  names, forwarded by `todo_peer` to its child, so what they judge does not change.
- If a child session cannot be bound to a Mesh endpoint of its own (a child's `<send target="#server">`
  through `hostForChild`), that is built first, in the Mesh host core of every backend that runs a
  world copy (Rust, and Kotlin through the external actor), because the list must send to its peers
  wherever it runs.

### 4. Order of work

1. The standard part, its schemas, its announcements and its scenarios (copied from `todo_peer`'s,
   renamed), on every backend `todo_peer` runs on today. `todo_peer` is untouched in this step.
2. `todo_peer` rebuilt on the standard part, with the world, scenario and cross-language tests
   passing unchanged; the duplicated body is deleted in the same change. Until this step lands the
   protocol exists twice, and no change to the protocol is made in either copy alone.
3. ADR 0034's trust layer and docs/SCE_SHARED_LIST_TRUST.md name the standard part as the document
   they secure.

## Status of the plan

Step 1 is built and not yet verified: the standard part, its schemas, its announcements and its
scenarios, on the backends `todo_peer` runs on. `todo_peer` is untouched. Steps 2 and 3 are not
started.

## Revisions made in building it

- **The condition a list keeps of how it was started is `announce`.** The `<invoke>` that starts
  the list hands it `<param name="announce" expr="true"/>`; a list a host starts keeps the `false`
  its `<data>` gives it and tells no one. The telling is a transition of its own
  (`record_list.announce`, which a write or supersede that moved a register raises for itself) and
  is taken only under that condition, so nothing is sent to `#_parent` by a list no statechart
  started, whatever raised the event; a parent may raise it too, to ask what a register holds. A
  host call was not taken instead: an action is refused inside `<if>` and `<foreach>` under this
  data model, and a register's further values are told in a `<foreach>`. The send names `#_parent`
  by an expression among its declared `sce:targets` and not as a written target, since a written
  `#_parent` makes a machine one that is only ever invoked (the C11 backend gives it no entry of its
  own and the C++ one a parent type), and this list is started by a host as often as by a parent.
  `record_list_alone` (a list a host started: `announced` stays 0, nothing fails) and
  `record_list_parent` (what a parent is told) pin it.
- **A register is told once for each change taken in, as it stands when the change is whole.** A
  write or supersede that moved a register raises `record_list.announce` for it; the first of a
  change puts the register in `moved` and queues `record_list.tell` behind every event the change
  has already queued, and a register already in `moved` is not put there twice. The telling then
  tells each register in `moved` once and empties it. So a person's edit, a supersede of each value
  it saw and then a write, is told once, as the register stands after the write, where the first
  build told it once for each event. The order of the dots is the wall time, then the counter, then
  the node (`sce:std/shared/register_values.scxml`). `announced` counts what was told;
  `record_list_parent` pins the hub's own change over two values told once.
- **The bounds are named** `merge_entries`, `merge_values`, `merge_digest` (the names the merge
  rules already carry), `merge_text` (the 96 bytes of text a write carries, which
  `sce:std/merge/mv_value.scxml` now names too, so the value a copy keeps moves with the write),
  `shared_typed_text` (the 32 bytes a person types, `record_edit`), `shared_roster`,
  `shared_answers` and `shared_moved` (the registers one change may move before it is told). A text
  bound is a schema field's `sce:max-size`, which `sce:limit` could not name before; the generator
  now reads a `sce:limit` beside it.
- **The list names routes, not peers.** `todo_peer` sends to `#server`, `#alice` and `#bob` and
  decides whom to pass a change on to by fixed node numbers (1 and 2). The standard part has two
  logical targets: a person's copy sends to `#hub`, and the hub (the copy `record_list.join` names
  the hub, whatever its node) sends to `#people`. A deployment binds both as it binds any `<send
  target="#name">` today (`machines.<m>.bindings` in deploy.yaml, SCE_MESH.md): `#hub` to the
  key the hub listens on, and `#people` to one key every person's copy listens on, which the Mesh
  delivers to each of them, so one send reaches them all. Nothing had to be added to the deploy
  surface. The hub therefore passes a change on to its author as well; the author holds it already
  and takes it again as nothing (the sheet's hub does the same). No condition reads a node number:
  the hub's own change is passed on to the people and sent to no hub, and a node of the digest
  walk or a share goes to `#hub` from a person and to `#people` from the hub, where the copy it is
  for reads its `to`. Otherwise the protocol is the one `todo_peer` speaks.
- **A statechart invokes a standard statechart by its library name**
  (`<invoke type="scxml" src="sce:std/shared/record_list.scxml">`), which no part of the generator
  read before: the child is the library's text, carried in memory as a child written in `<content>`
  is and generated beside its parent under the same synthesised name, and the `source-hash` of the
  parent's tree covers it and the documents it imports. `sce-codegen generate` takes a standard
  statechart by that name as well. The Interpreter's lowering rewrites an in-line child where its
  text stands in the parent, and a standard child has none there, so it is refused as an invoke by
  `src` (`generate/unsupported-feature`): `record_list_parent` runs on the six generated backends,
  and the Interpreter replays the list's own scenarios.
- **A scenario names a standard statechart by its library name.** The scenarios of the list say
  `"machine": "sce:std/shared/record_list"`, and every part of the scenario infrastructure takes
  that as it takes a fixture's stem: the regen scripts of Rust, Kotlin, Go and Python generate each
  standard statechart a scenario names from the library, the C++ and C11 build rules
  (`sce_add_state_machine`, `sce_generate_static_integration_c_test`) take a library name as
  `sce_add_algorithm` already did, the Interpreter's scan lowers it by that name, each driver reads
  the machine under the last segment of the name, and the guard that every scenario is replayed
  wherever its machine lowers reads it from the library. No copy of the document and no link to it
  stands in the fixture directory.
- **The list's scenarios are written by hand, and the world holds it to the to-do afternoon in its
  own process.** A scenario derived from every step of the world's afternoon was some 9 MB for the
  three copies, too much to commit and to carry in the wasm32 scenario table. The committed ones are
  short (`record_list_person`, `record_list_hub`, `record_list_alone`, `record_list_parent`, some
  35 KB in all) and pin each behaviour the derived ones did: two writes at once both kept, a
  supersede that drops only what it saw, the walk of the digest that finds a lost change, a settle
  and a floor that forgets tombstones and refuses a write at or before it, a copy that says it is
  stale, the room's and the roster's refusals, and the hub passing every change on. Their expected
  values come from a model of the list that was first held to every one of the 777 steps
  `todo_peer`'s three copies took in the afternoon, not from a backend. The world test still
  derives the afternoon as the standard list's (the schedule's steps left out, every event renamed,
  every send by its route, and one send where the to-do hub sent the same change to each person)
  and replays it on the standard part's Rust machine, writing no file, so the two copies of the
  protocol stay held to one run until step 2.
- **A child session cannot reach a Mesh peer today**, on Rust or on Kotlin: a Mesh router is
  registered on the engine a host holds (`sce_rust_mesh::endpoint::register`,
  `StateMachineEngine.registerMeshRouter`), the generated parent builds a child's engine and gives it
  only its clock and its parent link, and a child's send to `#hub` finds no processor and is
  `error.execution`. What `hostForChild` answers is the host of a child's `<sce:action>`s, not a
  router. Section 3's second point is therefore the first work of step 2, and is the subject of an
  ADR of its own; this step leaves the list's sends as they are.

## Alternatives considered

- **Each application copies the generic part.** Rejected for the reason in Context: two copies of a
  protocol diverge, and the independent review has to be applied to every copy.
- **The host drives the list and holds the rules.** Rejected: the rules leave the generated,
  scenario-judged code (Context, choice 2).
- **A generic list parameterised by the application's field schema at build time** (the list
  generated once per application with typed fields). More type-safe at the boundary, but it needs a
  template mechanism for statecharts this repository does not have, and the wire would differ per
  application, which the trust layer and the relay would then have to learn. Kept as a possible
  later step; the numbered-field form is what the protocol and the trust layer already speak.

## Consequences

- An application writes its rules once, in its own statechart, and every backend generates them.
- The protocol is reviewed, fixed and tested in one document.
- A parent that mirrors registers keeps a second, derived copy of what the list holds; it is a view,
  rebuilt from the announcements, and is never sent anywhere.
- The step-2 refactor touches the world simulation and the cross-language test, which other work in
  this repository depends on; it lands as one change with those tests passing unchanged.
