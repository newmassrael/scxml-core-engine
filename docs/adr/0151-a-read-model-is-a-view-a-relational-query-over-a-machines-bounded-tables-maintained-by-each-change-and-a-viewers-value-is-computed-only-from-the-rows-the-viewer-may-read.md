# ADR 0151 — A read model is a view: a relational query over a machine's bounded tables, maintained by each change, and a viewer's value is computed only from the rows the viewer may read

- Status: Accepted (2026-11-28, by the owner), as amended at its review the same day: D7 is judged at the levels of
  ADR 0115 (a theorem for every size, a model per document, and the kit), D9 names the read model of a whole
  deployment (ADR 0114 D4 as amended), D10's reference evaluator is the one crate the theorem is proved of, and the
  order of work puts the theorem and the model before the operators. The passages its sources were cited for were
  checked at acceptance (Sources). Nothing of it is built.
- Date: 2026-11-27
- Written under the owner's standing rule (the most long-term correct structure, textbook in its foundations, cost
  ignored) and the owner's scope rule of 2026-11-22: a new facility is built only on the backends a consumer needs,
  Rust by default; its contract is general; what is special to one application stays in the application.
- Amends, once accepted (each amended document gains an "Amended by: ADR 0151" line in the landing that moves it):
  - `SCE_FORGE.md` §3.2 (the kind list), §4 (a section for the kind), §8 (the kind summary) and §10 (the
    application of the admission test, D1);
  - `docs/SCE_ACCEPTED_SUBSET.md` §2.15 (a variable whose value is a view, D9) and the appendix (D11's codes);
  - ADR 0114, "Not in scope": "relational queries over a journal (a kind of its own)" is this record;
  - ADR 0108 D2: a map's order is the order its keys were put, and no operator of a view reads it (D2);
  - ADR 0118 D3: a view's state is one more part of a machine's state bound (D8);
  - ADR 0115 D7 (a view is translated as its relational meaning, so a machine that holds one has a model) and D15 (the
    one property of two runs it admits, a view's noninterference, judged on the self-composed model: the record of its
    own D15 waits for), D7 here.
- Scope: `sce-build` (the kind, its plan, its checks and its reference evaluator) and the Rust lane, native and
  `wasm32-unknown-unknown`. Every other backend, and the Interpreter, refuses the kind by name (D10).
- Related: ADR 0050 and ADR 0086 (a bound by name, part of a type), ADR 0101 (one deterministic CBOR form per type),
  ADR 0108 (a table by key), ADR 0113 (an instance), ADR 0114 (the journal and its followers), ADR 0116 (a person,
  and relations as facts), ADR 0118 (cost from the plan), ADR 0133 (totality where the build proves it), ADR 0152
  (a client's subscription to the value this record publishes).

## Context

What exists today, and what each part does not do:

- **Tables.** A `sce-static` statechart holds a table by key as a value, `map<K, V>[B]` and `set<K>[B]` (ADR
  0108), on Rust the `BoundedMap` and `BoundedSet` of `backends/rust/portable-bytes/src/map.rs`. A map is walked
  in the order its keys were put (ADR 0108 D2): a fact of its history, not of its contents.
- **Computation over tables.** An `algorithm` (`SCE_FORGE.md` §4.12) is lent a map by name, walks it with
  `<sce:foreach>` and may build and return one (ADR 0108 D5). A join, a group or an order can therefore be written
  today, as loops, by hand. Each call computes the whole answer again, so its cost is the size of the tables and not
  of what changed. Nothing in it says which rows it reads, so nothing can say what a reader of its answer learns of
  rows it may not see. Its order is whatever its loops met first, which is the maps' put order.
- **Facts.** ADR 0114 keeps a session's facts in a journal written with its state
  (`backends/rust/session-store/src/journal.rs`: `JournalStore::after`, the horizon, `Stream::Observed`), and
  names its reader: "A read model is a session that follows" (D4). Its steps 3 and 4, the build's `follows:` and
  the pool's follower, are not built (`sce-build/src/mesh/deploy.rs` reads no `follows:`). Its "Not in scope"
  names this record: relational queries over a journal, a kind of its own.
- **Who.** ADR 0113 makes an instance a sender (`backends/rust/mesh/src/peers.rs`, `Instances`); ADR 0116 puts a
  person behind it and decides that groups and relations are facts of an observed stream whose meaning is the
  document's (D5). Nothing gives that meaning a form a build can read.
- **Bounds.** A bound is part of a type and may be named, `[name=N]`, moved by `sce-codegen --limit name=N` and
  read as `_x.limits.<name>` (ADR 0050, ADR 0086). ADR 0118 (proposed) makes a bound a term of the named limits.
- **Publication.** A variable declared `sce:direction="out"` is in the machine's snapshot
  (`<Machine>Observe::snapshot`, `tools/codegen/templates/rust/state_machine.rs.jinja2`;
  `docs/SCE_ACCEPTED_SUBSET.md`, "Snapshot"): one value, the same for every reader.

So a read model can be computed today only by recomputing it whole, by code whose reads no one can check, in an order
that depends on history, and published once to everyone. SCE_FORGE §10's admission of a new kind was the answer
given when the need was first stated; this record is that kind, and its viewer filter is part of it rather than a
second facility beside it.

## The textbook structure

- **A relation is a set of tuples, and a query is a function from relations to a relation** (Codd 1970), closed under
  composition: a view may be defined over views. Aggregation extends the algebra with grouping (Klug 1982). Order is
  not a property of a relation but of how a result is presented, and a result cut to its first `n` rows is
  determined only when its order is total.
- **A view is maintained by its change, not recomputed** (Gupta and Mumick 1995). DBSP (Budiu, Chajed, McSherry,
  Ryzhyk and Tannen, VLDB 2023) gives the general form: tables are streams of Z-sets (each row with an integer
  weight), and the incremental form of a query `Q` is `D . Q . I` (differentiate after the query of the
  integrated input). A linear operator (select, project, map) is its own incremental form; a join is bilinear and
  needs the integrated state of both sides, `Δ(a ⋈ b) = Δa ⋈ Δb + Δa ⋈ b + a ⋈ Δb`; an aggregate needs the
  integrated rows of each changed group; and the incremental form of a composition is the composition of the
  incremental forms (the chain rule). The integrated state is held indexed by the key an operator looks up
  (McSherry et al., "Shared Arrangements", VLDB 2020), so a change costs the rows it reaches and not the tables.
- **Maintenance may be deferred to the next read** without changing the meaning (Colby, Griffin, Libkin, Mumick and
  Trickey, SIGMOD 1996).
- **Who may read an object is a relation, and derived relations are rules over relation tuples** (Pang et al.,
  Zanzibar, USENIX ATC 2019; Fong 2011, relationship-based access control). A tuple says `object#relation@subject`,
  a subject being a principal or a set of them (`group#member`), and a namespace derives relations by union,
  intersection and exclusion, by the same object's other relation (computed userset) and through a related object
  (tuple to userset). These are rules of Datalog, whose meaning is the least fixed point when no rule depends on its
  own exclusion (stratified negation; Apt, Blair and Walker 1988).
- **Noninterference** (Goguen and Meseguer 1982): what a low observer sees is unchanged by what the high part does.
  It is a property of two runs, not of one (a hyperproperty, Clarkson and Schneider 2010; 2-safety, Terauchi and
  Aiken 2005), so it is checked on two runs side by side (self-composition, Barthe, D'Argenio and Rezk 2004), and it
  is enforced by a typing of information flow (Denning 1976; Volpano, Smith and Irvine 1996): a value labelled with
  what it was computed from may reach only an observer allowed to read every label.

## Decision

### D1. A view is a document of its own kind

`sce:kind="view"` is a document that imports the record schemas of its tables and its result, declares its input
tables, an optional viewer (D6) and one result, and states one query (D2) or, for a document of relations, rules
(D5). It holds no state of its own between two changes but the integrated state of D4, which is derived, and it is
not inline-eligible (`SCE_FORGE.md` §3.2): it is declared as a document and imported.

```xml
<scxml sce:kind="view" name="board">
  <sce:import kind="event-schema" src="task.scxml" as="Task"/>
  <sce:import kind="event-schema" src="project.scxml" as="Project"/>
  <sce:import kind="event-schema" src="person.scxml" as="Person"/>
  <sce:import kind="event-schema" src="board_row.scxml" as="BoardRow"/>
  <sce:import kind="view" src="access.scxml" as="Access"/>
  <sce:viewer of="person"/>
  <sce:table name="tasks" type="map&lt;uint64, record:Task&gt;[tasks_max=10000]">
    <sce:visible relations="Access" relation="viewer" object="row.project"/>
  </sce:table>
  <sce:table name="projects" type="map&lt;uint64, record:Project&gt;[projects_max=500]">
    <sce:visible relations="Access" relation="viewer" object="key"/>
  </sce:table>
  <sce:table name="people" type="map&lt;uint64, record:Person&gt;[people_max=2000]" public="true"/>
  <sce:query>
    <sce:from table="tasks" as="t"/>
    <sce:join table="projects" as="p" on="p.key == t.row.project"/>
    <sce:join table="people" as="a" on="a.key == t.row.assignee" kind="left"/>
    <sce:where cond="t.row.open"/>
    <sce:group by="p.key">
      <sce:field name="project" expr="p.row.title"/>
      <sce:count name="open"/>
      <sce:nest name="next" order="t.row.due asc" type="list&lt;record:TaskLine&gt;[per_project=5]">
        <sce:field name="title" expr="t.row.title"/>
        <sce:field name="assignee" expr="a.row.name"/>
      </sce:nest>
    </sce:group>
    <sce:order by="open desc"/>
  </sce:query>
  <sce:result type="record:BoardRow" key="project" bound="board_rows=200"/>
</scxml>
```

The operators are `from`, `join` (inner, or `left`, whose right side is then of an optional type, ADR 0055),
`where`, `group` with `field`, `count`, `sum`, `min`, `max` and `nest`, `select` where there is no group,
and `order`. Their expressions are SCE's expression language (`SCE_FORGE.md` §3.4) over the rows in scope, each
one proven total where the document is read (ADR 0133), since a view has no `error.execution` to raise: a
maintenance step that failed halfway would leave the view and its tables disagreeing. A `sum` is of integers, of
a type the build proves wide enough from the column's type and the group's largest count, and a sum, a minimum, a
maximum or an order over a floating type is refused: the sum of floating numbers depends on the order of its
additions, and an incremental sum and a recomputed one would differ in their bytes.

**Admission** (`SCE_FORGE.md` §10, where it is recorded as an application):

| Axis | Pass/Fail | Reason |
|---|---|---|
| Domain alignment | PASS | Deterministic data and executable state: a view is a function of bounded keyed tables, its result a typed value whose bytes are one deterministic CBOR form (ADR 0101), equal on every lane and equal whether maintained or recomputed (D4) |
| Futamura projection compatibility | PASS | The tables' types and bounds, the operators, the keys, the order, the relations and the result's type are build-time; only the rows are runtime. The plan is compiled to one maintenance circuit per view, as DBSP compiles a query, and nothing is interpreted at run time |
| Cross-domain reuse | PASS | GUI and scene state (a list a screen shows, grouped and ordered, per person), game networking and the MMORPG data plane (a player's area of interest is a viewer's relations; a leaderboard is a group and an order), network protocols (a routing or interest table joined with its peers), embedded telemetry and sensor pipelines (aggregates over a table of readings, maintained per reading), build and CI systems (the status of each target joined with its dependencies) |

Passed 3-of-3. Two alternatives were ruled out before admission: an `algorithm` (it has no plan the build can read,
so neither D4 nor D7 can be had of it), and a flag on `bounded-collection` (a slot table addressed by handles is
not a relation).

### D2. Its meaning is relational, and its order is total and its own

- **A table is read as the set of its rows.** A map is the set of its `(key, value)` pairs and a set the set of its
  keys. No operator reads a map's put order (ADR 0108 D2), so the result is a function of the tables' contents and
  not of the history that put them.
- **A key is known for every intermediate relation**, inferred by the rules of functional dependency: a table's key;
  for a join, the pair of its sides' keys, or the left side's key alone where the join is on the right side's key; a
  group's columns. The result names its key, which must be one the build infers, so a result never holds two rows of
  one key (`view/result-key-not-unique` otherwise).
- **The result's order is total.** `<sce:order>` names the columns it orders by, and the build appends the result's
  key as the last one, so two rows are never tied. A `nest`'s order is completed the same way, by the key of the
  rows it holds.
- **A bound cuts by the order and says what it cut.** A result and a `nest` declare a bound; the value is the first
  rows in the order, at most the bound, and the count of all the rows there were (`total`), so a value that was cut
  says so. A result or a nest with a bound that may be reached and no order is refused (`view/bound-without-order`):
  a cut without an order is not determined. An intermediate relation declares no bound and none is cut: its size is a
  term of its inputs' bounds (D8), so only a result and a nest can be cut, and each by its own order.
- **The value is typed.** A view's value is a record the build generates: `rows`, a list of the result's record of
  the declared bound, and `total`, a `uint32`. Its bytes are its deterministic CBOR form (ADR 0101).

### D3. A view may read another view

A `<sce:table>` may name a view instead of a type (`view="Other"`): its rows are the other view's result, keyed by
that view's key. The plans compose, and so do their incremental forms (the chain rule), so a change goes through both
as one circuit. A cycle of views is refused (`view/cycle`); recursion is a property of relations alone (D5). A view
that reads another passes it its own viewer, and one with no viewer reads only views with none (D6).

### D4. A view is maintained by each change, and its state is derived

- **A change is a delta.** Each `<sce:put>`, `<sce:remove>` and `<sce:clear>` on a table a view reads is, to the
  view, the old row with weight -1 and the new with +1 (DBSP's Z-set). The deltas of a macrostep are applied before
  the view is next read, by an expression of the machine or at the snapshot at the end of the macrostep (deferred
  maintenance): wherever a view is read, its value is the query of its tables as they are there.
- **The runtime holds each operator's integrated state, indexed by the key it looks up**: a join's two sides by the
  join's columns, a group's rows by the group and in the nest's order, the result in its order. A change of one row
  reaches the rows of the other side that match it, the group it is in, and the place in the order it leaves and
  takes. The work of one change is therefore the size of the deltas each operator takes and gives, never the size of
  the tables. The runtime counts it, in rows each operator reads from its state or writes to its output
  (`ViewWork`), so a test holds it and a deployment's metrics report it (ADR 0109 D16).
- **The state is derived and is not saved.** A saved state holds the tables, as it does today; a restore, and a
  migration that changes a view (ADR 0109 D11), builds the state again by one evaluation of the tables, whose cost
  is the tables'. A derived state saved beside its source would be a second source of one truth, which ADR 0114
  rejects for the same reason.
- **The operators are written once**, in the Rust forge runtime (`sce-forge-runtime`, module `view`, beside the
  `queue` and `broadcast-log` algorithms), generic over the row types; a view's generated code is the circuit that
  connects them, printed from the plan.

### D5. Who may read a row is a derived relation, declared as rewrites of relation tuples

A view document whose body is `<sce:relation>` elements in place of a query is a document of relations. Its one
input is a table of relation tuples, a `set<record:Tuple>[B]` of a schema with the object, the relation, and the
subject, which is a principal or a set of principals (an object and a relation, `group#member`). Its relations
are declared as Zanzibar's namespace rewrites:

```xml
<sce:relation name="viewer" object="project">
  <sce:direct/>                                   <!-- project#viewer@subject -->
  <sce:computed relation="editor"/>               <!-- every editor is a viewer -->
  <sce:through relation="parent" then="viewer"/>  <!-- the viewers of the parent project -->
</sce:relation>
<sce:relation name="editor" object="project"><sce:direct/></sce:relation>
<sce:relation name="member" object="group"><sce:direct/></sce:relation>
```

Children of a relation are united; `<sce:all>` intersects its children and `<sce:but-not>` excludes. A subject that
is a set is expanded through its relation, so groups nest. The build reads them as rules of Datalog and refuses a
relation that depends on its own exclusion (`view/relation-cycle-through-exclusion`); every other cycle is a least
fixed point, maintained as DBSP maintains a recursive query, and its size is bounded by the term of its objects and
subjects (D8). The tuples come from wherever the machine's tables come from (D9): its own facts, or the observed
stream ADR 0116 D5 names. SCE gives the rewrites and their meaning; what a relation means to the application is the
document's.

### D6. The viewer is a property of the plan

- **A view may declare a viewer**, `<sce:viewer of="person"/>` or `of="instance"`: the person the delegation names
  (ADR 0116 D1, its issuer and subject) or the instance (ADR 0113 D1). Its value is then a value per viewer.
- **Every table it reads states who may read a row**: `<sce:visible relations="..." relation="..." object="..."/>`
  names the relation the viewer must stand in to the row's object, or `public="true"` says every viewer may read it
  all. A table with neither is refused (`view/table-without-visibility`): access is lacking unless it is declared.
- **The build puts the filter first.** Over each table, the first operator of the plan is the semi-join of its rows
  with the objects the viewer stands in that relation to; no operator reads a table but through it, and the relation
  itself is read only for the viewer it is joined with. This is the typing of information flow: every value of the
  result is computed from rows the viewer may read, and a plan that would read one otherwise is not one the build can
  make.
- **A view reads only what its viewer may**: a view with a viewer reads views of the same viewer, passed on, and views
  with none; a view with no viewer reads only public tables and views with none (`view/viewer-mismatch`).
- **The viewers are a table.** The set of viewers whose values are kept (D9) is one more input of the plan, and the
  filter is a join with it, so the plan is lifted over every viewer at once: one change of a row costs work for the
  viewers that may read it, which is the size of the values it changes, and none for the others.

### D7. The statement, and how it is checked

For a view with a viewer, let `D` be the contents of all its input tables, the relation tuples among them. For a
viewer `v`, `Vis(v, D)` is the set of objects `v` stands in each named relation to, derived from `D`'s tuples
by D5, together with the rows of each table whose object is in it, and the whole of each public table.

> **For every viewer `v` and contents `D` and `D'`, if `Vis(v, D) = Vis(v, D')`, the value published for `v`
> has the same bytes over `D` and over `D'`: its rows, their order, every count and `total` included.**

A change of rows outside a viewer's relations is a change from `D` to `D'` with `Vis(v, D) = Vis(v, D')`: a row
of an object `v` stands in no relation to, or a tuple whose change moves none of `v`'s relations. Such a change
leaves `v`'s value byte-identical, and so publishes nothing new to `v` (ADR 0152 D2 sends nothing for a value
whose bytes did not change).

It follows from three facts:

1. **The plan is `Q'` after the filter** (D6), and `Q'` is a function of its input whose result's order is total
   (D2): a rule of the build, where the document is read.
2. **The maintained value equals the plan's function** after every change (D4).
3. **One value has one form of bytes** (ADR 0101).

Each is judged at the levels of ADR 0115, and none rests on a random test alone.

- **Two theorems, for every size** (ADR 0115 D3 and D13), proved in Verus and named in `docs/formal/README.md` with
  what each takes as given:
  - *T1, the plans are noninterfering*: for every plan the build can make, its value over `D` is a function of
    `Vis(v, D)`. It is proved once, by induction on the plan, over the specification functions of the reference
    evaluator (D10): each operator is a function of its inputs, and every input of a plan whose tables are each read
    through its filter is a function of `Vis`. It is the soundness of a typing of information flow, the form in which
    Volpano, Smith and Irvine proved theirs, here for a language with no loops and no state.
  - *T2, maintenance is exact*: each operator of the Rust runtime (D4) takes its integrated state and a delta to the
    delta of its specification (`D . Q . I` of that operator), and a circuit is their composition (the chain rule).
    Each operator carries the contract and its proof (ADR 0115 D13); DBSP's own theory was checked in Lean by its
    authors, and this proves the code that runs.
- **A model per document** (ADR 0115 D7 and D8). `sce-codegen model` translates a view as its relational meaning:
  each operator of the plan is a TLA+ operator over finite sets and functions (a join a set comprehension, a group a
  function of its key, an order a sequence sorted by its total order). A machine that holds a view therefore has a
  model, the view's value in each observation is the plan's function of the tables there, and D8's replay holds every
  lane's maintained value to that recomputed one at every observation of every scenario.
- **The statement, per document** (ADR 0115 D15). A properties document may declare
  `<sce:noninterference view="..."/>`. TLC judges it on the self-composition of the machine's model (Barthe,
  D'Argenio and Rezk 2004): two copies take the same events, except that each may take a change of its own that moves
  none of the viewer's relations, and the invariant of the product is that where the two copies' `Vis` are equal, so
  are the viewer's two values. It is the one property of two runs ADR 0115 D15 admits, and every other stays refused
  there. Its verdict is *holds within* the exploration's bounds; T1 is what says it for every size. A configuration
  that moves the filter after the first join must make TLC find a pair of runs that breaks it, as ADR 0115 D5 asks of
  every part.
- **The kit** (D10) replays generated cases on every lane against the reference evaluator and checks the statement on
  random contents. It tests the lanes, and judges nothing the levels above judge.

**What it does not say.** It is of values, not of time: a change outside a viewer's relations costs the server work,
and may move the moment the viewer's next value is computed. Goguen and Meseguer's statement is of what an observer
is shown, and a timing channel is not closed here. Nor does it let a view publish to a viewer anything computed from
rows the viewer may not read (a count of every task, for one): no form of declassification is offered.

### D8. Bounds are declared, and tuned, by name

Every table's bound, every result's and every nest's is a bound of its type, a number or `name=N` (ADR 0050, ADR
0086), so a deployment moves each one by name at its build (`sce-codegen --limit tasks_max=20000`). An intermediate
relation's size and every operator's state are terms of those names, computed from the plan (ADR 0118 D3): a join's
state is its sides' bounds, its output at most their product or, on a key, the left side's bound; a derived relation's
fixed point at most its objects' bound times its subjects'. The build states each term, part by part, as one more
part of the machine's state bound, so nothing a view holds is unbounded and every bound has a name a build can move.

### D9. A machine holds a view's value as a variable, and publishes it

```xml
<data id="board" sce:view="Board" sce:direction="out">
  <sce:bind table="tasks" expr="tasks"/>
  <sce:bind table="projects" expr="projects"/>
  <sce:bind table="people" expr="people"/>
  <sce:bind table="tuples" view="Access" expr="tuples"/>
</data>
```

- **Its tables are the machine's map and set variables**, bound by name as an algorithm's slots are (ADR 0108 D5),
  of the types the view declares. A variable held to the facts (`sce:from="journal"`, ADR 0114 D1) may be one, so a
  read model is the session ADR 0114 names: it follows journals and observed streams, folds them into its tables, and
  declares views over them.
- **The read model of a whole deployment is one session.** A machine whose views fold journals and streams that share
  no field keeps one session (`sessions: { key_from: none }`, ADR 0114 D4 as amended 2026-11-28), its tables are the
  deployment's, and its viewers, not its sessions, separate what each person reads.
- **Its value is never assigned**: an `<assign>`, a put, an algorithm's `location` naming it is refused
  (`view/assigned`). It is the query of the bound tables wherever it is read.
- **A view without a viewer** is a value the machine's expressions read and its snapshot carries, as any published
  variable is.
- **A view with a viewer** is read by no expression of the machine (the machine reads its tables whole, and a guard on
  one viewer's value would make the machine's behaviour depend on a choice of viewer) and is not in the snapshot. It
  is published per viewer: on Rust `<Machine>Observe::view_<name>(&viewer)` gives the value for one viewer, and
  ADR 0152 gives it to the clients that subscribe. The viewers whose values are kept are those a host or a
  subscription asks for; a viewer asked for the first time costs one evaluation of what that viewer may read, and a
  viewer no longer asked for is dropped from the table of viewers.
- **A view reads only its own session's tables.** Nothing is shared between sessions (SCE_MESH.md §8.1): rows of
  another session reach it as facts it follows, through its own inbox.

### D10. One reference evaluator, and the lanes

- **The reference evaluator is the plan's, and is one crate.** `sce-view-reference` evaluates the plan of a view on
  tables given in the language-neutral value form of the conformance cases (`tests/forge/conformance`), from scratch
  and naively: every join a nested loop, every group a full pass, every order a sort. Its functions are proved to
  compute the specification functions T1 is stated over (D7; ADR 0115 D13), and `sce-build` and the kit evaluate by
  it, so the evaluator a lane is held to is the one the theorem is of. It is the specification a runtime is held to, as
  ADR 0118 D1 holds every backend's bytes to a fold over the encode plan. Its surface is a command,
  `sce-codegen view eval <document> <tables>`.
- **The cases are generated, with their seeds.** `tests/forge/conformance/gen_cases.py` writes random tables and
  random sequences of changes from a seed it records, and the expected values the reference evaluator gives; a lane
  replays the case and compares bytes after every change.
- **Rust is the lane**, native and on `wasm32-unknown-unknown` (`backends/rust/scenarios` already runs machines
  there), with an allocator: the operators keep their state on the heap, up to its bound, as a map with `Heap`
  storage does (ADR 0108 D4). A build of a view without an allocator, every other backend and the Interpreter refuse
  it by name (`view/backend-unsupported`), so a lane that does not support it says so and never answers otherwise.

### D11. Refusals

Each is a `DiagnosticCode` with its appendix row in `docs/SCE_ACCEPTED_SUBSET.md`, added in the step that makes it:
`view/result-key-not-unique`, `view/bound-without-order`, `view/cycle`, `view/relation-cycle-through-exclusion`,
`view/table-without-visibility`, `view/viewer-mismatch`, `view/expression-may-fail`,
`view/aggregate-may-overflow`, `view/floating-aggregate`, `view/assigned` and `view/backend-unsupported`.

## Not in scope

A query a client writes at run time (the plan is the build's, and an ad hoc query is an interpreter, which §10's second
axis refuses); declassification; timing channels; a view over the tables of several sessions; full outer joins, window
functions, text search and approximate aggregates; recursion outside relations; a view on a lane without an allocator;
and any one application's relations or views.

## Consequences

- A read model is a document a build checks: its plan, its bounds, its key and its order are known before it runs.
- One changed row costs the rows it reaches, so a read model of many rows answers a change in time that does not grow
  with them.
- What a viewer is shown is computed from what the viewer may read, by construction and by a check, and a change it
  may not see cannot be told from no change.
- A restore costs one evaluation of the tables, since the state of the views is built again and not saved.
- An author writes relations as rewrites and a query as operators, not loops; what an algorithm computed by hand over
  tables can move to a view, and gains the two properties above.

## Alternatives rejected

- **An `algorithm` per query.** It recomputes its answer whole and has no plan the build reads, so neither D4 nor D7
  can be had of it.
- **Recomputing the view at each macrostep and comparing.** Correct, and its cost is the tables'.
- **SQL text as the surface.** A second language with its own grammar, its own types and its own expressions beside
  SCE's. The plan is the contract; a SQL front end that writes a plan is a later question, not a reason for a second
  meaning.
- **An engine embedded in the runtime** (an SQL database, a dataflow system). Its semantics are its own, its bytes are
  not held to the other lanes, its plan is not the build's, and it runs on neither `wasm32` in a page nor every host.
- **The filter at the edge**, a host taking rows out of the published value for each reader. A count, a sum, a nest's
  `total` and a cut by order are computed before it and already carry what was taken out.
- **A predicate per row evaluated at read** (row-level security). A predicate of any form cannot be shown not to leak
  through what is computed after it (Rice), and it is the planner's to push it below an aggregate. A relation is a
  join the build sees and places.
- **Differential dataflow with partially ordered times.** More than a session needs: a session's changes are totally
  ordered by its macrosteps, and DBSP's streams over that order suffice.
- **Saving the operators' state with the session.** A second source of a truth the tables already hold.

## Order of work

Each step lands with a test that fails without it.

1. This record, accepted, and the application of D1 in `SCE_FORGE.md` §10.
2. The kind's reader, checks and plan, with every refusal of D11 and its appendix row:
   `sce-build/tests/a_view_document_is_read_into_one_plan.rs`. Red without it: `sce:kind="view"` is an unknown
   kind. Each refusal has a case that must be refused by its code, and the case that puts the filter after a join must
   be one the build cannot make.
3. The reference evaluator, `sce-view-reference`, with its specification functions and T1 proved in Verus, and its
   command and the generator of cases: `sce-build/tests/a_view_is_answered_by_its_plan.rs` (laws the evaluator must
   keep: a join is the filtered product, a count is the rows of its group, the order is total, a cut keeps `total`)
   and `sce-build/tests/a_change_outside_a_viewers_relations_leaves_its_value_byte_identical.rs`. A mutation
   casefile, `sce-build/tests/mutations/a_view_reads_a_table_only_through_its_filter.cases`, moves the filter after
   the first join and must turn the second test red, and the proof of T1 must fail on it.
4. The model: `sce-codegen model` of a view, and `<sce:noninterference>` in ADR 0115 D15's properties document,
   judged on the self-composed model. Example documents are entered in `docs/formal/models.json` with a run that holds
   and a configuration that moves the filter after the first join, which TLC must find. Red without it: a machine that
   holds a view has no translation, and `sce-codegen model` refuses it by name.
5. The Rust operators and the generated circuit, each operator with its T2 contract proved, and held to the reference:
   `backends/rust/forge-conformance/tests/forge_view.rs` with
   `a_maintained_view_equals_its_reference_after_every_change` (10 000 rows, two joins, a group and an order, random
   changes from recorded seeds), `a_changed_row_costs_work_in_proportion_to_the_change` (the `ViewWork` of one
   changed task equal at 10 000 and at 20 000 rows, its project of the same size in both; red for a circuit that
   recomputes) and
   `a_change_outside_a_viewers_relations_publishes_the_same_bytes`.
6. The machine: `sce:view`, its bindings, its refusals, the snapshot, `view_<name>` and the rebuild on restore:
   `backends/rust/scenarios/tests/view.rs` (`a_restored_view_equals_the_one_before_it_was_saved`,
   `a_view_read_in_a_macrostep_sees_the_puts_before_it`), run native and on `wasm32-unknown-unknown`, and its
   scenarios replayed against its model (ADR 0115 D8).
7. Relations (D5): the rewrites, nested groups, exclusion, and the refusal of a cycle through it, held to the
   reference with the cases of step 5, translated as a least fixed point by strata, and T1 carried over the strata.
8. The documents named in "Amends".

Step 6 needs nothing of ADR 0114's follower; a read model that follows (D9) needs ADR 0114 step 4 and its amendment of
2026-11-28, and a viewer that is a person needs ADR 0116 step 4.

## Open questions

Decided as recommended, for the owner's acceptance, each with its reason.

1. The kind's name is `view`: the database's name for a named query whose value is its result. `query` names a
   question asked once, and a deployment's `queryables` (ADR 0041) are another thing.
2. Recursion is allowed in relations alone, and only where no relation depends on its own exclusion: nested groups
   need it, and stratification is the condition under which a fixed point is the one meaning.
3. Aggregates and orders over floating types are refused: equal bytes between a maintained value and a recomputed one
   is the contract.
4. A viewer with no relations is given an empty value (`rows` empty, `total` 0), not a refusal: a refusal would
   tell the viewer that it has none, which the empty value also says and no more.
5. The operators' state is not saved (D4).
6. D7 is judged by a theorem for every size and a model per document, not by the kit alone: a random test samples
   what a theorem states of every input, and the model is what ADR 0115 D8 holds every lane to (the owner, at
   acceptance, 2026-11-28).

## Sources

Codd 1970 ("A relational model of data for large shared data banks"); Klug 1982 (aggregate functions in the relational
algebra and calculus); Gupta and Mumick 1995 (maintenance of materialized views, IEEE Data Engineering Bulletin);
Colby, Griffin, Libkin, Mumick and Trickey 1996 (deferred view maintenance); Budiu, Chajed, McSherry, Ryzhyk and
Tannen 2023 (DBSP, VLDB); McSherry, Lattuada, Schwarzkopf and Roscoe 2020 (shared arrangements, VLDB); Apt, Blair and
Walker 1988 (stratified negation); Pang et al. 2019 (Zanzibar, USENIX ATC); Fong 2011 (relationship-based access
control, CODASPY); Goguen and Meseguer 1982 (security policies and security models, IEEE S&P); Denning 1976 (a lattice
model of secure information flow); Volpano, Smith and Irvine 1996 (a sound type system for secure flow analysis);
Terauchi and Aiken 2005 (secure information flow as a safety problem); Barthe, D'Argenio and Rezk 2004 (secure
information flow by self-composition); Clarkson and Schneider 2010 (hyperproperties); Lattuada et al. 2023 (Verus,
OOPSLA). Checked at acceptance (2026-11-28): DBSP defines a query's incremental form as `D . Q . I`, its correctness
proofs rest on the join's bilinearity, and its theory was checked in Lean (Chajed's formalization); in D4's rule for a
join, `a` and `b` are the tables before the change. Zanzibar's rewrites are `_this`, the computed userset and the tuple
to userset, combined by union, intersection and exclusion. Goguen and Meseguer's statement is of what one group of users
can see being unaffected by what another does with its commands. The others are cited from the texts as known.
