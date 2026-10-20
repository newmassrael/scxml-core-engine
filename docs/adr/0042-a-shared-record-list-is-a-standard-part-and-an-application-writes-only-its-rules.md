# ADR 0042 - A shared record list is a standard part, and an application writes only its rules

- Status: Accepted (2026-10-20); step 1 in progress
- Date: 2026-10-20
- Scope: a new standard statechart `sce:std/shared/record_list.scxml` and its schemas under
  `stdlib/shared/`; `sce-build/tests/fixtures/static_datamodel/todo_peer.scxml` and the documents
  it imports; `backends/rust/world`; docs/SCE_ACCEPTED_SUBSET.md (standard library), SCE_MESH.md
  where a child session meets a Mesh peer
- Related: ADR 0034 (a shared list is held by its members and the hub only relays),
  docs/SCE_SHARED_LIST_TRUST.md, ADR 0005 (bounding and declaring)

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
  written under a condition the list keeps from how it was started, or the announcement is a host
  call when there is no parent; the implementation chooses one and the scenarios pin it).

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
