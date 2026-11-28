# ADR 0151 — A read model is a view: a relational query over a machine's bounded tables, maintained by each change, and a viewer's value is computed only from the rows the viewer may read

- Status: Accepted (2026-11-28) under the owner's standing rule (the most long-term correct, textbook structure, cost
  ignored), by the main session the owner delegated the decision to, after an independent review; amended on
  acceptance as listed in "Amended on acceptance". Amended the same day by the owner, at the owner's question why a
  feature made in SCXML is not proved in TLA+ (first landed in ca877ecb830 and kept here over the review's text): D7
  is judged at the levels of ADR 0115, D9 names the read model of a whole deployment, D11's reference evaluator is
  the one crate the theorem is proved of, and the order of work puts the theorems and the model before the operators
  ("Amended on acceptance", 12 to 15).
- Landed: nothing. No step of "Order of work" is built, and no model or test of it has been run.
- Amended 2026-11-29 (ADR 0156 D17): D7's model per document, the `<sce:noninterference>` statements judged on its
  self-composition, and step 4 of "Order of work" rest on the translation of the `sce-static` data model, which ADR
  0156 D17 decides and its order steps 2a to 2c schedule; no step scheduled it before. Step 4 comes after them, and
  `docs/formal/Compare.tla`, D2's comparison in TLA+, is one of D17's modules.
- Date: 2026-11-27 (proposed); 2026-11-28 (accepted).
- Written under the owner's standing rule (the most long-term correct structure, textbook in its foundations, cost
  ignored) and the owner's scope rule of 2026-11-22: a new facility is built only on the backends a consumer needs,
  Rust by default; its contract is general; what is special to one application stays in the application.
- Amends (each amended document gains an "Amended by: ADR 0151" line in the landing that moves it):
  - `SCE_FORGE.md` §3.2 (the kind list), §4 (a section for the kind), §8 (the kind summary) and §10 (the
    application of the admission test, D1);
  - `docs/SCE_ACCEPTED_SUBSET.md` §2.15 (a variable whose value is a view, D9, and the certification of what leaves
    a machine, D10) and the appendix (D12's codes);
  - ADR 0114, "Not in scope": "relational queries over a journal (a kind of its own)" is this record;
  - ADR 0114 D4: its sentences "What a view computed is the checkpoint, so a view needs no other snapshot" and "a
    view is rebuilt by following from position 0 under a new name" read, once amended: what a read model folded into
    its tables is the checkpoint, so it needs no other snapshot; the state of a view over those tables is derived and
    not saved, and is built again from them by one evaluation (D4 here); and a read model whose folding changes is
    rebuilt by following from position 0 under a new name;
  - ADR 0108 D2: a map's order is the order its keys were put, and no operator of a view reads it (D2);
  - ADR 0118 D3: a view's state is one more part of a machine's state bound, a bound of the heap and not of the saved
    form, since it is not saved (D8);
  - ADR 0115 D7 (a view is translated as its relational meaning, so a machine that holds one has a model) and D15
    (the properties of two runs it admits, D7's and D10's noninterference, judged on the self-composed model: the
    record of its own D15 waits for), D7 here;
  - ADR 0152 D1: a value listed `subscribable:` is a point where a value leaves the machine (D10), so one that
    depends on a filtered table is refused there, and `audience: viewer` on a view with a viewer is the one way a
    subscriber is given a filtered table's rows;
  - ADR 0152 D8: `max_viewers` counts the viewers a host holds (D9) with those its subscriptions keep, and is the
    bound of the table of viewers (D8 here).
- Scope: `sce-build` (the kind, its plan, its checks, its certification and its reference evaluator) and the Rust
  lane, native and `wasm32-unknown-unknown`. Every other backend, and the Interpreter, refuses the kind by name (D11).
- Related: ADR 0050 and ADR 0086 (a bound by name, part of a type), ADR 0055 (optional types), ADR 0092 (a field of
  several record shapes), ADR 0101 (one deterministic CBOR form per type), ADR 0108 (a table by key), ADR 0113 (an
  instance), ADR 0114 (the journal and its followers), ADR 0116 (a person, and relations as facts), ADR 0118 (cost
  from the plan), ADR 0133 (totality where the build proves it), ADR 0157 (a bounded FIFO's declared full policy),
  ADR 0152 (a client's subscription to the value this record publishes).

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
  names its reader: "A read model is a session that follows" (D4). Every step of it has landed: the build reads
  `follows:` (`sce-build/src/mesh/deploy.rs`), and the pool's follower is `Pool::follow`
  (`backends/rust/mesh/src/pool/follow.rs`), as amended on landing step 4 (2026-11-27). Its D4 also says that what a
  view computed is the checkpoint and that a view is rebuilt by following from position 0; here a view's state is
  derived and not saved (D4), so that sentence is amended ("Amends"). Its "Not in scope" names this record:
  relational queries over a journal, a kind of its own.
- **Who.** ADR 0113 makes an instance a sender (`backends/rust/mesh/src/peers.rs`, `Instances`); ADR 0116 puts a
  person behind it and decides that groups and relations are facts of an observed stream whose meaning is the
  document's (D5). A request may carry the position of that stream an earlier answer told, and the machine waits
  until it has followed that far; the build reads where a request carries it as `position_from`
  (`sce-build/src/mesh/deploy.rs`). Nothing gives the relations themselves a form a build can read, and nothing
  holds a row back until the relations it was written under have been taken in.
- **Bounds.** A bound is part of a type and may be named, `[name=N]`, moved by `sce-codegen --limit name=N` and
  read as `_x.limits.<name>` (ADR 0050, ADR 0086). ADR 0118 (proposed) makes a bound a term of the named limits.
- **Publication.** A variable declared `sce:direction="out"` is in the machine's snapshot
  (`<Machine>Observe::snapshot`, `tools/codegen/templates/rust/state_machine.rs.jinja2`;
  `docs/SCE_ACCEPTED_SUBSET.md`, "Snapshot"): one value, the same for every reader. Nothing checks what it was
  computed from, or under which guards.

So a read model can be computed today only by recomputing it whole, by code whose reads no one can check, in an order
that depends on history, and published once to everyone. SCE_FORGE §10's admission of a new kind was the answer
given when the need was first stated; this record is that kind, and its viewer filter and the certification of what
else leaves the machine are part of it rather than facilities beside it.

## The textbook structure

- **A relation is a set of tuples, and a query is a function from relations to a relation** (Codd 1970), closed under
  composition: a view may be defined over views. Aggregation extends the algebra with grouping (Klug 1982). Order is
  not a property of a relation but of how a result is presented, and a result cut to its first `n` rows is
  determined only when its order is total, which needs one total comparison for every type a column may have.
- **A view is maintained by its change, not recomputed** (Gupta and Mumick 1995). DBSP (Budiu, Chajed, McSherry,
  Ryzhyk and Tannen, VLDB 2023) gives the general form: tables are streams of Z-sets (each row with an integer
  weight), and the incremental form of a query `Q` is `D . Q . I` (differentiate after the query of the
  integrated input). A linear operator (select, project, map, and the aggregates `count` and `sum`, which keep one
  running value per group) is its own incremental form. A join is bilinear: `Δ(a ⋈ b) = Δa ⋈ Δb + Δa ⋈ b + a ⋈ Δb`,
  where `a` and `b` are the integrated states before the change (`z^-1 I`); read after it, they count `Δa ⋈ Δb`
  twice. A `min`, a `max` and a nest of rows need the integrated rows of each changed group. The incremental form of
  a composition is the composition of the incremental forms (the chain rule). The integrated state is held indexed by
  the key an operator looks up (McSherry et al., "Shared Arrangements", VLDB 2020), so a change of a linear, bilinear
  or aggregate operator costs the rows it reaches and not the tables.
- **A recursive view is maintained at the cost of what its change reaches.** An insertion into a recursive relation
  adds the derivations it makes; a deletion may remove a derivation others rest on, and maintenance then removes what
  rested on it and derives again what still holds by another way, which can be much of the fixed point (DRed: Gupta,
  Mumick and Subrahmanian, "Maintaining views incrementally", SIGMOD 1993). What the change does not reach costs
  nothing.
- **Maintenance may be deferred to the next read** without changing the meaning (Colby, Griffin, Libkin, Mumick and
  Trickey, SIGMOD 1996).
- **A query is restricted to what its reader may read by rewriting it**: each table is first filtered by the reader's
  rights, and the rest of the query runs on what is left (query modification: Stonebraker and Wong, "Access control in
  a relational data base management system by query modification", ACM Annual Conference 1974). The reader is then
  answered as if the database held only the rows it may read (the Truman model: Rizvi, Mendelzon, Sudarshan and Roy,
  "Extending query rewriting techniques for fine-grained access control", SIGMOD 2004).
- **Who may read an object is a relation, and derived relations are rules over relation tuples** (Pang et al.,
  Zanzibar, USENIX ATC 2019; Fong, "Relationship-based access control: protection model and policy language", CODASPY
  2011). A tuple says `object#relation@subject`, a subject being a principal or a set of them (`group#member`), and
  a namespace derives relations by union, intersection and exclusion, by the same object's other relation (computed
  userset) and through a related object (tuple to userset). These are rules of Datalog. Where no rule depends on its
  own exclusion (stratified negation) their meaning is the perfect model: the strata are taken in order, each the
  least fixed point of its rules over the strata below it (Apt, Blair and Walker 1988; Przymusinski 1988). A content
  made after a relation was removed must not be judged by the relations before the removal (Zanzibar's "new enemy"
  problem); Zanzibar answers it with a token of the position the content was written at, and a check waits for the
  relations to reach that position.
- **Noninterference** (Goguen and Meseguer 1982): what a low observer sees is unchanged by what the high part does.
  It is a property of two runs, not of one (a hyperproperty, Clarkson and Schneider 2010; 2-safety, Terauchi and
  Aiken 2005), so it is checked on two runs side by side (self-composition, Barthe, D'Argenio and Rezk 2004).
- **Within a program it is certified from the program's text.** Each value is labelled with the classes of what it
  was computed from, the classes forming a lattice (Denning, "A lattice model of secure information flow",
  Communications of the ACM 1976). A statement is certified when the label of the value it writes, joined with the
  label of the conditions under which it runs (the program-counter label, which carries the implicit flows of guards
  and branches), is at most the label of what it writes to (Denning and Denning, "Certification of programs for secure
  information flow", Communications of the ACM 1977). Volpano, Smith and Irvine state the certification as a type
  system and prove that a typed program is noninterferent ("A sound type system for secure flow analysis", Journal of
  Computer Security 1996); Sabelfeld and Myers survey the field ("Language-based information-flow security", IEEE
  Journal on Selected Areas in Communications 2003). The proof is of what runs that end show, not of when they end or
  how long they take, and a certification is conservative: it may refuse a program that leaks nothing.
- **Floating numbers have a total order and an exact sum.** IEEE 754-2019 (IEEE Standard for Floating-Point
  Arithmetic, IEEE 2019) defines totalOrder (section 5.10), a total order of every floating datum, signed zeros and
  NaNs included. Floating addition is not associative (Goldberg, "What every computer scientist should know about
  floating-point arithmetic", ACM Computing Surveys 1991), so a sum rounded at each addition depends on the order of
  its additions. A sum kept exactly in a fixed-point register as wide as the format's range (the long accumulator:
  Kulisch and Miranker, *Computer Arithmetic in Theory and Practice*, Academic Press 1981) and rounded once is a
  function of the multiset of its terms, and a subtraction keeps it exact.

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
  <sce:import kind="event-schema" src="task_line.scxml" as="TaskLine"/>
  <sce:import kind="event-schema" src="board_row.scxml" as="BoardRow"/>
  <sce:import kind="view" src="access.scxml" as="Access"/>
  <sce:viewer of="person"/>
  <sce:table name="tasks" type="map&lt;uint64, record:Task&gt;[tasks_max=10000]">
    <sce:visible relations="Access" relation="viewer" object="row.project" at="row.relations_at"/>
  </sce:table>
  <sce:table name="projects" type="map&lt;uint64, record:Project&gt;[projects_max=500]">
    <sce:visible relations="Access" relation="viewer" object="key" at="row.relations_at"/>
  </sce:table>
  <sce:table name="people" type="map&lt;uint64, record:Person&gt;[people_max=2000]" public="true"/>
  <sce:query>
    <sce:from table="tasks" as="t"/>
    <sce:join table="projects" as="p" on="p.key == t.row.project"/>
    <sce:join table="people" as="a" on="a.key == t.row.assignee" kind="left"/>
    <sce:where cond="t.row.open"/>
    <sce:group by="p.key">
      <sce:field name="project" expr="p.key"/>
      <sce:field name="title" expr="p.row.title"/>
      <sce:count name="open"/>
      <sce:nest name="next" row="record:TaskLine" order="t.row.due asc" bound="per_project=5">
        <sce:field name="title" expr="t.row.title"/>
        <sce:field name="assignee" expr="a.row.name"/>
      </sce:nest>
    </sce:group>
    <sce:order by="open desc"/>
  </sce:query>
  <sce:result type="record:BoardRow" key="project" bound="board_rows=200"/>
</scxml>
```

The result's key, `project`, is the group's column `p.key`, which is a key the build infers (D2). `assignee` is of
an optional type, since its side of the join is `left`. `next` is of the record D2 generates for a nest: its rows,
of `TaskLine` and at most `per_project`, and its `total`. The relations document `Access` reads one table of
its own, `tuples`, which the machine binds through the import (D9).

The operators are `from`, `join` (inner, or `left`, whose right side is then of an optional type, ADR 0055),
`where`, `group` with `field`, `count`, `sum`, `min`, `max` and `nest`, `select` where there is no group,
and `order`. A `nest` names the record of its rows, its order and its bound. Their expressions are SCE's expression
language (`SCE_FORGE.md` §3.4) over the rows in scope, each one proven total where the document is read (ADR 0133),
since a view has no `error.execution` to raise: a maintenance step that failed halfway would leave the view and its
tables disagreeing. A `sum` of integers is of a type the build proves wide enough from the column's type and the
group's largest count. A `sum` of a floating type is kept exactly in a long accumulator (Kulisch and Miranker 1981)
whose width the build computes from the column's type and the group's largest count; an infinity and a NaN are
counted beside it. It is rounded once, when it is read, to nearest with ties to even: a NaN if a NaN or both
infinities are counted, the infinity if one is, the rounded accumulator otherwise. A removed row is subtracted
exactly, so the sum is a function of the group's rows and has the same bytes maintained and recomputed. A `min`, a
`max` and an order compare by the comparison of D2, which is total for every type.

**Admission** (`SCE_FORGE.md` §10), to be added there as an application when this record is accepted:

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
- **Every type has one comparison, and it is total.** It is a comparison of values, so two values whose deterministic
  forms are equal (ADR 0101) compare equal:
  - an integer by its number; a boolean `false` before `true`;
  - a floating number by IEEE 754-2019 totalOrder: `-0` before `+0`, and every NaN is the one NaN ADR 0101 writes,
    which sorts after the positive infinity;
  - a text by its Unicode code points, which is the order of its UTF-8 bytes, with no collation (a collation is a
    property of a language and a locale, the Unicode Collation Algorithm of Unicode Technical Standard #10, and is not
    in scope); bytes as unsigned bytes; in both, a prefix before what extends it;
  - an enum by the order its document declares its values;
  - a record by its fields, in the order its schema declares them, the first that differs deciding; a field of several
    record shapes (ADR 0092) by its tag, in the order declared, and then by the record;
  - a list by its elements in the same way, a prefix first; a set and a map as the list of their entries in the order
    of their keys;
  - an optional value (ADR 0055, the right side of a `left` join) is absent before every present value, as in the
    order of the sum `none | some(T)`; `asc` therefore puts it first and `desc` last, and an order term may write
    `absent first` or `absent last` to say otherwise (SQL's `NULLS FIRST` and `NULLS LAST`).

  Equality, in a join, a group and a key, is equality under this comparison.
- **The result's order is total.** `<sce:order>` names the columns it orders by, and the build appends the result's
  key as the last one, so two rows are never tied. A `nest`'s order is completed the same way, by the key of the
  rows it holds.
- **A bound cuts by the order and says what it cut.** A result and a `nest` declare a bound; the value is the first
  rows in the order, at most the bound, and the count of all the rows there were (`total`), so a value that was cut
  says so. A result or a nest with a bound that may be reached and no order is refused (`view/bound-without-order`):
  a cut without an order is not determined. An intermediate relation declares no bound and none is cut: its size is a
  term of its inputs' bounds (D8), so only a result and a nest can be cut, and each by its own order.
- **The value is typed.** A view's value is a record the build generates: `rows`, a list of the result's record of
  the declared bound, and `total`, an unsigned integer of the width the build proves from the largest count the
  plan's terms allow (D8), as it proves a sum's. A nest's value is a record of the same form, of its row record and
  its bound. Its bytes are its deterministic CBOR form (ADR 0101).

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
  join's columns, the rows of a group that a `min`, a `max` or a nest reads in their order, one running value per
  group for a `count` and a `sum`, and the result in its order.
- **The work of one change is stated per class of operator**, in rows each operator reads from its state or writes to
  its output:
  - a linear operator (select, project, map, `count` and `sum`) does work in proportion to the delta it takes;
  - a join, to its delta and the rows of the other side's integrated state that match it;
  - a `min`, a `max` and a nest, to the places in the changed group's ordered rows that a row leaves and takes;
  - an order and its cut, to the place a row leaves and takes and the one row that crosses the cut;
  - a recursive relation (D5): an insertion, to the derivations it adds; a deletion, to the derivations that rested
    on what it removed and those derived again by another way (DRed), which can be much of the fixed point, and is
    at most the fixed point's term (D8).

  So outside recursion the work of one change is never the size of the tables, and within it the work is of what the
  change reaches and none of what it does not. The runtime counts the work (`ViewWork`), so a test holds it and a
  deployment's metrics report it (ADR 0109 D16).
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
relation that depends on its own exclusion (`view/relation-cycle-through-exclusion`). Every other document of
relations is stratified, and its meaning is the perfect model: the strata in order, each the least fixed point of its
rules over the strata below it (Apt, Blair and Walker 1988; Przymusinski 1988). It is maintained as DBSP maintains a
recursive query, at the work D4 states for recursion, and its size is bounded by the term of its objects and subjects
(D8). The tuples come from wherever the machine's tables come from (D9): its own facts, or the observed
stream ADR 0116 D5 names. SCE gives the rewrites and their meaning; what a relation means to the application is the
document's.

### D6. The viewer is a property of the plan

- **A view may declare a viewer**, `<sce:viewer of="person"/>` or `of="instance"`: the person the delegation names
  (ADR 0116 D1, its issuer and subject) or the instance (ADR 0113 D1). Its value is then a value per viewer.
- **Every table it reads states who may read a row**: `<sce:visible relations="..." relation="..." object="..."/>`
  names the relation the viewer must stand in to the row's object, or `public="true"` says every viewer may read it
  all. A table with neither is refused (`view/table-without-visibility`): access is lacking unless it is declared.
  `<sce:visible>` may name, with `at`, the row's field that holds the position of the relations' stream its writer
  read (ADR 0116 D5); D9 says when it must.
- **The build puts the filter first.** Over each table, the first operator of the plan is the semi-join of its rows
  with the objects the viewer stands in that relation to, less the rows whose `at` is above the position the
  relations were followed to (D9); no operator reads a table but through it, and the relation itself is read only for
  the viewer it is joined with. This is query modification (Stonebraker and Wong 1974), and what a viewer is answered
  is the Truman model's answer (Rizvi, Mendelzon, Sudarshan and Roy 2004): the query over the rows the viewer may
  read. A plan that would read a table otherwise is not one the build can make.
- **A view reads only what its viewer may**: a view with a viewer reads views of the same viewer, passed on, and views
  with none; a view with no viewer reads only public tables and views with none (`view/viewer-mismatch`).
- **The viewers are a table.** The set of viewers whose values are kept (D9) is one more input of the plan, bounded
  by `max_viewers` (D8), and the filter is a join with it, so the plan is lifted over every viewer at once: one
  change of a row costs work for the viewers that may read it, which is the size of the values it changes, and none
  for the others.
- **Lifted, every relation after a filter carries its viewer.** A join of two relations that carry a viewer joins on
  equal viewers as well as on its own columns, and every group, every order, every nest and the result have the
  viewer as the first column of their key. A plan that joins the rows one viewer may read with the rows another may
  read, or groups or orders them together, is not one the build can make.

### D7. The statement, and how it is checked

For a view with a viewer, let `D` be the contents of all its input tables, the relation tuples among them, and the
position its relations were followed to (D9). For a
viewer `v`, `Vis(v, D)` is the set of objects `v` stands in each named relation to, derived from `D`'s tuples
by D5, together with the rows of each table whose object is in it and whose `at`, where the table declares one, is at or
below the position the relations were followed to (D9), and the whole of each public table.

> **For every viewer `v` and contents `D` and `D'`, if `Vis(v, D) = Vis(v, D')`, the value published for `v`
> has the same bytes over `D` and over `D'`: its rows, their order, every count and `total` included.**

A change of rows outside a viewer's relations is a change from `D` to `D'` with `Vis(v, D) = Vis(v, D')`: a row
of an object `v` stands in no relation to, or a tuple whose change moves none of `v`'s relations. Such a change
leaves `v`'s value byte-identical, and so publishes nothing new to `v` (ADR 0152 D2 sends nothing for a value
whose bytes did not change).

It follows from three facts:

1. **The plan is `Q'` after the filter, lifted over the viewers** (D6): every table is read through the filter,
   every join of two relations that carry a viewer is on equal viewers, and every group, order, nest and key begins
   with the viewer; and `Q'` is a function of its input whose result's order is total (D2). It is checked where the
   document is read, as a rule of the build.
2. **The maintained value equals the plan's function** after every change (D4).
3. **One value has one form of bytes** (ADR 0101).

Each is judged at the levels of ADR 0115, and none rests on a random test alone (amended 2026-11-28 by the owner).

- **Two theorems, for every size** (ADR 0115 D3 and D13), proved in Verus and named in `docs/formal/README.md` with
  what each takes as given:
  - *T1, the plans are noninterfering*: for every plan the build can make, its value over `D` is a function of
    `Vis(v, D)`. It is proved once, by induction on the plan, over the specification functions of the reference
    evaluator (D11): each operator is a function of its inputs, and every input of a plan whose tables are each read
    through its filter and whose joins of viewers are on equal viewers is a function of `Vis`. It is the soundness
    of a typing of information flow, the form in which Volpano, Smith and Irvine proved theirs, here for a language
    with no loops and no state.
  - *T2, maintenance is exact*: each operator of the Rust runtime (D4) takes its integrated state and a delta to the
    delta of its specification (`D . Q . I` of that operator), and a circuit is their composition (the chain rule).
    Each operator carries the contract and its proof (ADR 0115 D13); DBSP's own theory was checked in Lean by its
    authors, and this proves the code that runs.
- **A model per document** (ADR 0115 D7 and D8). `sce-codegen model` translates a view as its relational meaning:
  each operator of the plan is a TLA+ operator over finite sets and functions (a join a set comprehension, a group a
  function of its key, an order a sequence sorted by the comparison of D2). A machine that holds a view therefore has
  a model, the view's value in each observation is the plan's function of the tables there, and D8's replay holds
  every lane's maintained value to that recomputed one at every observation of every scenario.
- **The statements, per document** (ADR 0115 D15). A properties document may declare
  `<sce:noninterference view="..."/>`, D7's statement for one view, and `<sce:noninterference/>`, D10's for the
  machine: what leaves it is the same in two runs whose filtered tables differ. TLC judges each on the
  self-composition of the machine's model (Barthe, D'Argenio and Rezk 2004): two copies take the same events, except
  that each may take a change of its own that moves none of the viewer's relations (for D10, any change of a filtered
  table), and the invariant of the product is that the viewer's two values (for D10, the two copies' observations)
  are equal. These are the properties of two runs ADR 0115 D15 admits, and every other stays refused there. A verdict
  is *holds within* the exploration's bounds; T1 is what says D7's for every size. A configuration that moves the
  filter after the first join, one that drops the viewer from a join's columns, and one that sends the size of a
  filtered table must each make TLC find a pair of runs that breaks its statement, as ADR 0115 D5 asks of every part.
- **The kit** (D11) replays generated cases on every lane against the reference evaluator and checks the statement on
  random contents by self-composition: for random contents and a random viewer, a random change outside the viewer's
  relations (with `Vis` confirmed unmoved by the reference evaluator), and the viewer's bytes before and after equal,
  on the reference evaluator and on every lane. It tests the lanes, and judges nothing the levels above judge.

**What it does not say.** It is of values, not of time: a change outside a viewer's relations costs the server work,
and may move the moment the viewer's next value is computed. Goguen and Meseguer's statement is of what an observer
is shown, and a timing channel is not closed here. Nor does it let a view publish to a viewer anything computed from
rows the viewer may not read (a count of every task, for one): no form of declassification is offered. And it is a
statement of a view's value alone: what else leaves the machine is held by D10.

### D8. Bounds are declared, and tuned, by name

- **Every table's bound, every result's and every nest's** is a bound of its type, a number or `name=N` (ADR 0050,
  ADR 0086), so a deployment moves each one by name at its build (`sce-codegen --limit tasks_max=20000`).
- **The table of viewers is bounded by `max_viewers`**, the bound ADR 0152 D8 names, with no default, written by the
  deployment and moved by name. A viewer asked for past it is refused, with the bound as the reason, and a kept
  viewer is never evicted to make room: a full table of viewers answers full, as a full map does (ADR 0108 D4), and
  its policy is the one ADR 0157 D1 calls `refuse`, under which the one refused loses nothing that was kept.
- **Every other size is a term of those names**, computed from the plan (ADR 0118 D3): a join's state is its sides'
  bounds, its output at most their product or, on a key, the left side's bound; a derived relation's fixed point at
  most its objects' bound times its subjects'; `total`, at most the term of the relation before the cut. Over a view
  with a viewer, every operator after the filter holds at most `max_viewers` times its own term, and the values kept
  are at most `max_viewers` times the result's bound, each with its nests' bounds.
- **The build states each term, part by part, as one more part of the machine's state bound.** Since a view's state
  is not saved (D4), its terms bound the heap the runtime holds and not the saved form. So nothing a view holds is
  unbounded, and every bound has a name a build can move.

### D9. A machine holds a view's value as a variable, and publishes it

```xml
<data id="board" sce:view="Board">
  <sce:bind table="tasks" expr="tasks"/>
  <sce:bind table="projects" expr="projects"/>
  <sce:bind table="people" expr="people"/>
  <sce:bind view="Access" table="tuples" expr="tuples" follows="directory"/>
</data>
```

- **Its tables are the machine's map and set variables**, bound by name as an algorithm's slots are (ADR 0108 D5),
  of the types the view declares. A variable held to the facts (`sce:from="journal"`, ADR 0114 D1) may be one, so a
  read model is the session ADR 0114 names: it follows journals and observed streams, folds them into its tables, and
  declares views over them.
- **The read model of a whole deployment is one session.** A machine whose views fold journals and streams that share
  no field keeps one session (`sessions: { key_from: none }`, ADR 0114 D4 as amended 2026-11-28), its tables are the
  deployment's, and its viewers, not its sessions, separate what each person reads.
- **The bindings cover every table of the view and of the views it imports (D3), each once.** A table of an imported
  view is named with `view=` and the name the importing view gives the import (`Access` above). A table left
  unbound, bound twice, named by no view, or bound to a variable of another type is refused
  (`view/binding-mismatch`).
- **Its value is never assigned**: an `<assign>`, a put, an algorithm's `location` naming it is refused
  (`view/assigned`). It is the query of the bound tables wherever it is read.
- **A row is shown no earlier than the relations it was written under.** A table of relation tuples folded from a
  stream the session follows names that stream on its binding (`follows=`, a name in the machine's `follows:`), and
  the position the session has taken in of it (ADR 0114 D4) is an input of every filter over those relations. A row
  whose `at` is above that position is visible to no viewer; when the session has followed the stream that far, the
  row enters each value it belongs to, as any change does. Since a row written after a revocation carries a position
  past it, it is never judged by the relations before the revocation (ADR 0116 D5, the same rule for a request). A
  filter over relations bound with `follows=` whose `<sce:visible>` names no `at` is refused
  (`view/visibility-without-position`). Relations that are the session's own facts need none: its macrosteps order
  them with its rows.
- **A view without a viewer** is a value the machine's expressions read and its snapshot carries, as any published
  variable is. It reads only public tables (D6), so nothing in it depends on a filtered table (D10).
- **A view with a viewer** is read by no expression of the machine (the machine reads its tables whole, and a guard on
  one viewer's value would make the machine's behaviour depend on a choice of viewer) and is not in the snapshot: the
  snapshot is one value for every reader, so declaring it `sce:direction="out"` is a flow D10 refuses. It is
  published per viewer, and only to the viewers kept. A viewer is kept while a host holds it or a subscription
  (ADR 0152) asks for it, counted once, up to `max_viewers` (D8). On Rust, `<Machine>Observe::hold_<name>(&viewer)`
  adds the viewer, or answers as a value that `max_viewers` is reached, and gives a hold;
  `<Machine>Observe::view_<name>(&hold)` reads the value for the viewer it holds; releasing the hold drops the viewer
  when nothing else keeps it. A viewer kept for the first time costs one evaluation of what that viewer may read.
- **A view reads only its own session's tables.** Nothing is shared between sessions (SCE_MESH.md §8.1): rows of
  another session reach it as facts it follows, through its own inbox.

### D10. What leaves a machine is certified not to depend on a filtered table

- **The statement.** A filtered table is a variable of the machine bound to a table that a view reads through
  `<sce:visible>` (D6). No value that leaves the machine depends on a row of a filtered table: not explicitly (it was
  computed from one) and not implicitly (it was computed, written or sent under a guard, a branch or a loop that read
  one). A value leaves the machine at:
  - a variable declared `sce:direction="out"`, which the snapshot carries;
  - a value a deployment lists `subscribable:` (ADR 0152 D1);
  - a send of any type, a fact said to the journal (`<send type="sce:journal">`, ADR 0114 D1) included: its event,
    its target, its delay, its data, and whether it is sent at all;
  - an invocation's parameters and a final state's done data;
  - a variable bound as a table a view declares `public="true"`, whose rows every viewer is shown.

  The one way a filtered table's rows leave the machine is a view's value for one viewer (D9), which D7 states and
  checks. The host that runs the machine holds its tables and its saved state, and is not an observer of this
  statement.
- **The labels.** A label is a set of the machine's filtered tables, ordered by inclusion (Denning's lattice of
  subsets); the empty set is public, and every point where a value leaves the machine requires it. A filtered table is
  labelled with itself, and so is every field of an event's data whose value flows into it anywhere in the document:
  a content kept in a filtered table is as secret where it arrives as where it is kept. That an external event
  arrived, and its name, are public, since its sender and the deployment know them. Every other variable and the
  machine's control are labelled with what flows into them. The build infers the least labels that satisfy the rules
  below, the least solution of monotone constraints over a finite lattice.
- **The rules** are certification (Denning and Denning 1977) stated as a type system (Volpano, Smith and Irvine
  1996):
  - an expression is labelled with the join of the labels of what it reads; the outcome of a put, a remove or a clear
    (ADR 0108 D4), with that of the table it acted on; a view without a viewer, with nothing, since it reads only
    public tables (D6);
  - a piece of executable content runs at a program-counter label: the join of the labels of the transition that
    runs it and of every `<if>` condition and every `<foreach>` collection that chose to run it. An assignment, a
    put, a remove, a clear, a raise and a send are certified when the label of what they write, joined with that
    program-counter label, is at most the label of where they write it;
  - an error event that executable content raises is labelled with its program-counter label and with the labels of
    what failed;
  - a statechart's control is a value too. A state's activity, which `In()` reads and a history state records, is
    labelled with the program-counter labels of every transition that enters or exits it. A transition runs at the
    join of its source state's activity, its event's label and its guard's. An internal event is labelled with the
    program-counter label of the content that raised it, and a done event with the activity of the state that
    raised it;
  - an algorithm (ADR 0108 D5) is certified over its own body as the machine is, and a call labels each value it
    returns and each map it writes with the join of the labels of the arguments the body certifies it depends on.
- **A refusal names the flow.** A document whose certification fails is refused at generation
  (`view/filtered-table-flows-out`). The diagnostic names the point where the value leaves, the filtered table, and
  the chain of flows, explicit and implicit, the certification found from the one to the other.
- **What it proves, and what not.** A certified machine is noninterferent in Volpano, Smith and Irvine's sense: two
  runs that differ only in the rows of filtered tables show the same values at every point where a value leaves the
  machine. As D7, it is a statement of values and not of time. The certification is conservative: a document that
  leaks nothing may be refused, and it then moves what it computes over filtered rows into a view with a viewer.
- **The kit checks it as well.** By self-composition, as D7 is checked: two runs of a certified machine from one
  state, whose filtered tables differ in rows a random change moved, give the same bytes at every point where a value
  leaves the machine, on every lane.

### D10a. A blob a view's value names reaches a viewer under the viewer's own grant

A row of a view's value may hold a `record:BlobRef` (ADR 0153). The value reaches a viewer only through a published
value (ADR 0152), whose changelog is a holder of grants and translates each reference to the subscriber's own handle
when it delivers (ADR 0152 D2, ADR 0153 D3 and D4, both as accepted on 2026-11-28). A viewer therefore never reads
another holder's handle, and the blob lives while a changelog row or a viewer's grant names it. A view's state itself
holds no grant: it is derived (D4), and what it names is held by the tables it is derived from.

ADR 0118 is Proposed. D8's terms are stated in its vocabulary and are checked by it once it is accepted; until then
`view/` bounds are checked by D8's own names alone, and no saved form is measured, since a view saves none (D4).

### D11. One reference evaluator, and the lanes

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

### D12. Refusals

Each is a `DiagnosticCode` with its appendix row in `docs/SCE_ACCEPTED_SUBSET.md`, added in the step that makes it:
`view/result-key-not-unique`, `view/bound-without-order`, `view/cycle`, `view/relation-cycle-through-exclusion`,
`view/table-without-visibility`, `view/viewer-mismatch`, `view/expression-may-fail`,
`view/aggregate-may-overflow`, `view/binding-mismatch`, `view/visibility-without-position`, `view/assigned`,
`view/filtered-table-flows-out` and `view/backend-unsupported`.

## Not in scope

A query a client writes at run time (the plan is the build's, and an ad hoc query is an interpreter, which §10's second
axis refuses); declassification; timing and termination channels; the secrecy of an external event's arrival (D10); a
view over the tables of several sessions; full outer joins, window functions, text search and approximate aggregates;
recursion outside relations; a view on a lane without an allocator; and any one application's relations or views.

## Consequences

- A read model is a document a build checks: its plan, its bounds, its key, its order and its flows are known before
  it runs.
- Outside recursion, one changed row costs the rows it reaches, so a read model of many rows answers a change in time
  that does not grow with them. Within a recursive relation, a deleted tuple costs what it reaches, which may be much
  of the fixed point, and nothing it does not reach.
- A view's value for a viewer is computed from what the viewer may read, by construction and by a check, and a change
  it may not see cannot be told from no change. Nothing else that leaves the machine depends on a filtered table,
  which the build certifies (D10).
- A row is shown no earlier than the relations it was written under.
- The viewers kept are at most `max_viewers`; one past it is refused, and none kept is evicted.
- A view orders and aggregates every type, floating types included, with the same bytes maintained and recomputed.
- A restore costs one evaluation of the tables, since the state of the views is built again and not saved.
- An author writes relations as rewrites and a query as operators, not loops; what an algorithm computed by hand over
  tables can move to a view, and gains the properties above.

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
- **A predicate per row** (row-level security). Placed before every other operator, as PostgreSQL's security barrier
  places it, it is as safe as D6's filter. It is not chosen because a relation is a table D4 maintains like any
  other, so a changed tuple costs the rows it reaches, where a predicate is evaluated again over every row it covers;
  and because rights that follow relations between objects (a group, a parent project) are the relationship-based
  model (Fong 2011) that ADR 0116 D5 already decided.
- **Refusing only the syntactic exits of a filtered table** (a filtered variable may not be `out`, subscribable or
  bound as public). It misses what is computed from one and what is decided by a guard on one: the explicit and
  implicit flows certification exists for (D10).
- **Refusing floating aggregates and orders.** totalOrder makes a `min`, a `max` and an order deterministic, and a
  long accumulator makes a sum exact and independent of the order of its terms; a refusal would hold a type out of
  views for a reason the arithmetic answers.
- **Differential dataflow with partially ordered times.** More than a session needs: a session's changes are totally
  ordered by its macrosteps, and DBSP's streams over that order suffice.
- **Saving the operators' state with the session.** A second source of a truth the tables already hold.

## Order of work

Each step lands with a test that fails without it.

1. This record, accepted, and the application of D1 in `SCE_FORGE.md` §10.
2. The kind's reader, checks and plan, with every refusal of D12 that the view document alone decides and its
   appendix row: `sce-build/tests/a_view_document_is_read_into_one_plan.rs`. Red without it: `sce:kind="view"` is an
   unknown kind. Each refusal has a case that must be refused by its code; the case that puts the filter after a join,
   and the case that joins two relations of viewers on their own columns alone, must be plans the build cannot make.
3. The reference evaluator, `sce-view-reference`, with its specification functions and T1 proved in Verus (D7), its
   command and the generator of cases: `sce-build/tests/a_view_is_answered_by_its_plan.rs` (laws the evaluator must
   keep: a join is the filtered product, a count is the rows of its group, the order is the comparison of D2 for every
   type, signed zeros, NaN and absent values included, a cut keeps `total`, a floating sum is the exact sum rounded
   once) and `sce-build/tests/a_change_outside_a_viewers_relations_leaves_its_value_byte_identical.rs` (D7 by
   self-composition on the reference). A mutation casefile,
   `sce-build/tests/mutations/a_view_reads_a_table_only_through_its_filter.cases`, has two cases that must each turn
   the second test red and make the proof of T1 fail: one moves the filter after the first join, and one drops the
   viewer from a join's columns.
4. The model (D7): `sce-codegen model` of a view and of a machine that holds one, and both forms of
   `<sce:noninterference>` in ADR 0115 D15's properties document, judged on the self-composed model. Example
   documents are entered in `docs/formal/models.json` with a run that holds and the three configurations of D7, each
   of which TLC must find. Red without it: a machine that holds a view has no translation, and `sce-codegen model`
   refuses it by name.
5. The Rust operators and the generated circuit, each operator with its T2 contract proved, and held to the
   reference: `backends/rust/forge-conformance/tests/forge_view.rs` with
   `a_maintained_view_equals_its_reference_after_every_change` (10 000 rows, two joins, a group and an order, random
   changes from recorded seeds), `a_changed_row_costs_work_in_proportion_to_the_change` (the `ViewWork` of one
   changed task equal at 10 000 and at 20 000 rows, its project of the same size in both; red for a circuit that
   recomputes), `a_floating_sum_has_the_same_bytes_maintained_and_recomputed` (random insertions and deletions of
   values whose rounded running sum would differ) and `a_change_outside_a_viewers_relations_publishes_the_same_bytes`.
6. The machine: `sce:view`, its bindings, its refusals, the snapshot, the holds and `view_<name>`, `max_viewers`,
   the position of the relations, and the rebuild on restore: `backends/rust/scenarios/tests/view.rs`
   (`a_restored_view_equals_the_one_before_it_was_saved`, `a_view_read_in_a_macrostep_sees_the_puts_before_it`,
   `a_viewer_past_max_viewers_is_refused_and_no_kept_viewer_is_evicted`,
   `a_row_written_after_a_revocation_is_not_shown_before_the_revocation_is_taken_in`), run native and on
   `wasm32-unknown-unknown`, and its scenarios replayed against its model (ADR 0115 D8).
7. The certification (D10): `sce-build/tests/a_value_that_leaves_a_machine_does_not_depend_on_a_filtered_table.rs`.
   Red without it: a document that sends the size of a filtered table is generated. Each flow is a case that must be
   refused by `view/filtered-table-flows-out` with the chain it names: explicit; through a guard, an `<if>` and a
   `<foreach>`; through a state's activity read by `In()`; through a raised event; through an algorithm's return;
   through a put's outcome; from an event's data that is also put in a filtered table; and a filtered table bound as a
   public one. A machine whose filtered tables are read only by views with a viewer must be certified. A mutation
   casefile, `sce-build/tests/mutations/a_machine_certifies_the_flows_of_its_guards.cases`, drops the guard's label
   from a transition's program-counter label and must turn the guard's case red. The kit's check by
   self-composition: `backends/rust/scenarios/tests/view.rs`,
   `a_change_of_a_filtered_row_moves_nothing_that_leaves_the_machine`.
8. Relations (D5): the rewrites, nested groups, exclusion, and the refusal of a cycle through it, held to the
   reference with the cases of step 5, translated as a least fixed point by strata with T1 carried over the strata, and `a_deleted_relation_tuple_costs_no_work_outside_what_it_reaches` in
   `forge_view.rs` (the `ViewWork` of one deleted membership equal with and without a second, unrelated tree of
   groups of 10 000 tuples).
9. The documents named in "Amends".

Step 4 needs ADR 0156's order steps 2a to 2c (D17, amended 2026-11-29): without them a machine that holds a view
has a typed datamodel the model generator refuses whole.
Step 6 needs nothing of ADR 0114's follower; a read model that follows (D9) uses ADR 0114 step 4, which has landed,
and a viewer that is a person needs ADR 0116 step 4.

## Open questions

Each was decided on acceptance, with its reason.

1. The kind's name is `view`: the database's name for a named query whose value is its result (`CREATE VIEW`,
   ISO/IEC 9075-2:2016, SQL Part 2: Foundation); keeping it maintained is how it is run, not what it means (Gupta and
   Mumick 1995). `query` names a question asked once, and a deployment's `queryables` (ADR 0041) are another thing.
   The informal "view" of ADR 0114 D4 is amended ("Amends").
2. Recursion is allowed in relations alone, and only where no relation depends on its own exclusion: nested groups
   need it, and stratification is the condition under which the perfect model is the one meaning (Apt, Blair and
   Walker 1988; Przymusinski 1988).
3. Floating types are allowed. A `min`, a `max` and an order compare by IEEE 754-2019 totalOrder (D2), which is
   total and deterministic. A floating `sum` is kept exactly in a long accumulator, which a subtraction keeps exact
   under deletion, and is rounded once when it is read (D1), so it is deterministic and independent of the order of
   its terms (Kulisch and Miranker 1981); a sum rounded at each addition would not be (Goldberg 1991).
4. A viewer with no relations is given an empty value (`rows` empty, `total` 0), not a refusal: a refusal would
   tell the viewer that it has none, which the empty value also says and no more. It is the Truman model's answer
   (Rizvi, Mendelzon, Sudarshan and Roy 2004).
5. The operators' state is not saved (D4), and ADR 0114 D4 is amended to say so ("Amends").
6. D7 and D10 are judged by theorems for every size and a model per document, not by the kit alone: a random test
   samples what a theorem states of every input, and the model is what ADR 0115 D8 holds every lane to (the owner,
   2026-11-28).

## Amended on acceptance

Each amendment is the independent review's, by its number there, decided by the main session as stated.

1. D10, new: what leaves a machine is certified not to depend on a filtered table, explicitly or through a guard or a
   branch, by a security type system (Denning and Denning 1977; Volpano, Smith and Irvine 1996; Sabelfeld and Myers
   2003), refused as `view/filtered-table-flows-out`; a view with a viewer is not `out` (D9); the Consequences
   line says "a view's value"; former D10 and D11 are D11 and D12 (review 1, decided as certification).
2. D6 and D7 fact 1: a lifted plan joins relations of viewers on equal viewers and keys every group, order, nest and
   result by the viewer first; step 3's casefile gains a case that drops the viewer from a join (review 2).
3. D6 cites query modification (Stonebraker and Wong 1974) and the Truman model (Rizvi et al. 2004), not a type
   system; "Alternatives rejected" drops Rice's theorem as the reason against a row predicate (review 3).
4. D6 and D9: a row whose `at` is above the position the relations were followed to is not visible, and a filter
   over followed relations without `at` is refused as `view/visibility-without-position` (review 4, ADR 0116 D5).
5. D8 and D9: the table of viewers is bounded by ADR 0152 D8's `max_viewers`, refused when reached and never
   evicted; the per-viewer term is stated; a host keeps a viewer by a hold (review 5).
6. D4 and Consequences: the work of a change is stated per class of operator, recursion by DRed (Gupta, Mumick and
   Subrahmanian 1993); step 7 tests the work under recursion (review 6).
7. D2: one total comparison per type, text by code point with no collation, an absent value first under `asc`, and
   the `absent first` and `absent last` terms (review 7).
8. D1's example: the key is `p.key`, the nest names its row record and bound and carries `total`, `Access`'s
   `tuples` is bound through the import with `view/binding-mismatch` for a binding that is not exact, and
   `total`'s width is proven (review 8).
9. The textbook section: DBSP's join reads the states before the change, `count` and `sum` are linear, and a
   stratified program means its perfect model (Apt, Blair and Walker 1988; Przymusinski 1988) (review 9).
10. Context: ADR 0114 has landed; "Amends" names ADR 0114 D4, and ADR 0152 D1 and D8 (review 10 and its cross-record
    findings).
11. Open question 3: floating `min`, `max` and order by IEEE 754-2019 totalOrder, a floating `sum` in a long
    accumulator (Kulisch and Miranker 1981); `view/floating-aggregate` is withdrawn (the review's answer to open
    question 3, decided by the main session).
12. D7, by the owner (2026-11-28, first landed in ca877ecb830): its facts are judged by T1 and T2 in Verus, by the
    document's generated model and by `<sce:noninterference>` on its self-composition, and the kit tests the lanes;
    D10's statement has the same form for the machine; "Amends" names ADR 0115 D7 and D15.
13. D9, by the owner: the read model of a whole deployment is one session, ADR 0114 D4 as amended the same day
    (`sessions: { key_from: none }`).
14. D11, by the owner: the reference evaluator is the crate `sce-view-reference`, whose functions carry the
    specification T1 is proved over, and `sce-build` and the kit evaluate by it.
15. Order of work and open question 6, by the owner: the theorems and the model come before the operators (steps 3
    and 4), and every later step is numbered one on.

## Sources

Codd 1970 ("A relational model of data for large shared data banks", Communications of the ACM); Klug 1982 (aggregate
functions in the relational algebra and calculus); Gupta and Mumick 1995 (maintenance of materialized views, IEEE Data
Engineering Bulletin); Gupta, Mumick and Subrahmanian 1993 ("Maintaining views incrementally", SIGMOD); Colby,
Griffin, Libkin, Mumick and Trickey 1996 (deferred view maintenance, SIGMOD); Budiu, Chajed, McSherry, Ryzhyk and
Tannen 2023 (DBSP, VLDB); McSherry, Lattuada, Schwarzkopf and Roscoe 2020 (shared arrangements, VLDB); Stonebraker and
Wong 1974 ("Access control in a relational data base management system by query modification", ACM Annual
Conference); Rizvi, Mendelzon, Sudarshan and Roy 2004 ("Extending query rewriting techniques for fine-grained access
control", SIGMOD); Apt, Blair and Walker 1988 ("Towards a theory of declarative knowledge", in Minker (ed.),
*Foundations of Deductive Databases and Logic Programming*, Morgan Kaufmann); Przymusinski 1988 ("On the declarative
semantics of deductive databases and logic programs", in the same volume); Pang et al. 2019 ("Zanzibar: Google's
consistent, global authorization system", USENIX ATC); Fong 2011 ("Relationship-based access control: protection model
and policy language", CODASPY); Goguen and Meseguer 1982 ("Security policies and security models", IEEE Symposium on
Security and Privacy); Denning 1976 ("A lattice model of secure information flow", Communications of the ACM); Denning
and Denning 1977 ("Certification of programs for secure information flow", Communications of the ACM); Volpano, Smith
and Irvine 1996 ("A sound type system for secure flow analysis", Journal of Computer Security); Sabelfeld and Myers
2003 ("Language-based information-flow security", IEEE Journal on Selected Areas in Communications); Terauchi and
Aiken 2005 ("Secure information flow as a safety problem", SAS); Lattuada et al. 2023 ("Verus: Verifying Rust
Programs using Linear Ghost Types", OOPSLA); Barthe, D'Argenio and Rezk 2004 ("Secure information
flow by self-composition", CSFW); Clarkson and Schneider 2010 ("Hyperproperties", Journal of Computer Security); IEEE
754-2019 (IEEE Standard for Floating-Point Arithmetic, IEEE 2019); Goldberg 1991 ("What every computer scientist should
know about floating-point arithmetic", ACM Computing Surveys); Kulisch and Miranker 1981 (*Computer Arithmetic in
Theory and Practice*, Academic Press); ISO/IEC 9075-2:2016 (SQL Part 2: Foundation; `CREATE VIEW`, `NULLS FIRST` and
`NULLS LAST`); Unicode Technical Standard #10 (the Unicode Collation Algorithm, the Unicode Consortium). They are cited
from the texts as known and were not read again for this record or its acceptance; the passages D4, D5, D7 and D10
rest on (DBSP's incremental join and aggregate, DRed, Zanzibar's rewrites and its answer to the new enemy problem,
Goguen and Meseguer's statement, Volpano, Smith and Irvine's rules and theorem) are to be checked against the texts in
the step that builds each. Three were checked at the owner's amendment (2026-11-28): DBSP defines a query's
incremental form as `D . Q . I`, its correctness proofs rest on the join's bilinearity, and its theory was checked in
Lean (Chajed's formalization); Zanzibar's rewrites are `_this`, the computed userset and the tuple to userset,
combined by union, intersection and exclusion; Goguen and Meseguer's statement is of what one group of users can see
being unaffected by what another does with its commands.
