# ADR 0165 — A graph is a relation over a machine's tables, and its searches are statements of one proved library whose answer depends only on the tables' contents

- Status: Accepted (2026-11-29) under the owner's standing rule, by the main session the owner delegated the decision
  to, after an independent review; amended on acceptance as listed in "Amended on acceptance".
- Landed: nothing. No step of "Order of work" is built, and no proof, model or test of it has been run.
- Amended by: ADR 0156 D17 (2026-11-29): what D9 and step 7 wait for, "a translation of the `sce-static` data model
  into the generated model, and the module of a map", is ADR 0156 D17, its order steps 2a to 2c, and
  `docs/formal/Map.tla`; "neither is a step of any record yet" no longer holds. `docs/formal/Compare.tla` is built by
  whichever of D17's step 2a and this record's step 2 lands first, the other citing it, and step 7 comes after D17's
  step 2c.
- Date: 2026-11-28 (proposed); 2026-11-29 (accepted).
- Written under the owner's standing rule (the most long-term correct structure, textbook in its foundations, cost
  ignored) and the owner's scope rule of 2026-11-22: a facility is built on the backends a consumer needs, Rust by
  default; its contract is general, and what is special to one application stays in the application.
- Requested by: a downstream consumer's census of its hand-written logic, whose catalogue names the gap "a general
  graph: shortest-path trees, reachability, removal of detached nodes" in nine rows. The census is held outside this
  repository; Context says what it found, and D12 checks that nothing here is shaped by that one application.
- Amends (each amended document gains an "Amended by: ADR 0165" line in the landing that moves it, where it carries
  such lines):
  - `docs/SCE_ACCEPTED_SUBSET.md` §2.15 (a graph declared over a machine's tables, D1, and the five statements that
    read it, D3) and the appendix (D11's codes);
  - `SCE_FORGE.md` §3.3 (the type list gains `uint128`, D6) and §4.12 (the declaration and the statements in an
    algorithm's body, D3);
  - `schemas/sce-forge-ext.xsd` (the elements of D1 and D3, an addition to a `pre-release` surface of
    `SCE_WIRE_CONTRACTS.md`);
  - ADR 0101 section 1: `uint128` joins the one type grammar with one deterministic form (D6);
  - ADR 0108 D6, items 2 and 3: a graph statement is a writer of every result it names, and `<sce:prune>` of its
    graph's two tables, so a `<foreach>` over one of them copies it and an `inout` argument is never one of them (D3);
  - ADR 0115 D11 and D13, and ADR 0146 D5: an entry of `docs/formal/proofs.json` names the crate root its file is read
    from and the specification files it includes (`includes`), and every included file is held by D11's isolation as a
    proved file's contracts are (D8);
  - ADR 0151 D2, which gains the sentence: "This comparison is specified once, here; ADR 0165 breaks a graph's ties by
    it, and it is built once, in `sce-portable-bytes`, by whichever of the two records reaches its step first" (D10);
  - ADR 0151 D4: the writes of a graph statement are deltas of the tables they write (D3), and the module of the
    operators is named `relational`, since `view` is already a module of `sce-forge-runtime` (D4);
  - ADR 0151 D7 and D11: a stratum's least fixed point is stated over the one specification of a least fixed point
    this record writes, in Verus and in TLA+ (D10);
  - ADR 0151 D10: a graph statement's results and `<sce:prune>`'s writes are certified as every writing statement is
    (D3);
  - ADR 0156 D2: three more modules of the semantics, `Compare.tla`, `Fixpoint.tla` and `Graph.tla` (D9);
  - ADR 0118 D3, once it is accepted: the working storage of a machine's graph statements is one more part of its
    state bound (D6);
  - `SCE_WIRE_CONTRACTS.md`: the three modules join the semantics surface of ADR 0156 D2 when that surface is
    registered;
  - `docs/formal/README.md` and `docs/formal/proofs.json` (the theorems of D8).
- Scope: `sce-build` (the declaration, the statements, their checks, the Rust lowering and the model's translation),
  the Rust forge runtime `sce-forge-runtime` (a module `graph`: the proved core and the adapter that feeds it), a
  specification-only directory `backends/rust/formal-spec/` (the specification functions the proofs include, D8),
  `sce-portable-bytes` (the comparison of ADR 0151 D2, where that record has not built it first), `stdlib/graph` (one
  outcome enum), `tests/forge/conformance` (the cases) and `docs/formal`. The Rust lane serves it, native, on
  `wasm32-unknown-unknown` and with no allocator. Every other backend, and the Interpreter, refuses it by name (D7).
- Depends on, for D9 alone: a translation of the `sce-static` data model into the generated model, and the module of a
  map of ADR 0156 D2; neither is a step of any record yet.
- Related: ADR 0005 (a `sce-static` document closes its gaps by bounding), ADR 0050 and ADR 0086 (a bound by name,
  part of a type), ADR 0101 (one deterministic form per value), ADR 0108 (a table by key), ADR 0115 (the levels of
  formal judgement), ADR 0118 (cost from the plan), ADR 0133 (a statement is total where the build proves it), ADR
  0151 (a view, its order, and recursive relations), ADR 0154 (a map's lent storage), ADR 0156 (a system of documents,
  its models and proof obligations generated from them).

## Context

What exists today, read from the tree at `662e4d060cc`:

- **Tables.** A `sce-static` statechart holds a table by key as a value, `map<K, V>[B]` or `set<K>[B]` (ADR 0108), on
  Rust `BoundedMap` and `BoundedSet` of `backends/rust/portable-bytes/src/map.rs`: an array of slots, an order linked
  through them that is the order the keys were first put (ADR 0108 D2), and a hash index under a seed, in one of five
  storages (`Inline`, `Heap`, `Linear`, and the lent `Lent` and `LentIndexed` of ADR 0154). A walk, the saved form and
  the snapshot follow the put order: a fact of the map's history, not of its contents.
- **Computation over tables.** An `algorithm` (`SCE_FORGE.md` §4.12, "Maps and sets") is lent a map by name, walks it
  with `<sce:foreach>` in put order, reads it with `len`, `has` and `m[k]`, and builds maps of its own. Rust and C11
  lower it (`ALGORITHM_MAP_BACKENDS`, `sce-build/src/forge/codegen_matrix.rs`). An algorithm is of concrete types: one
  generic over the record it is lent was rejected (ADR 0108 D6, item 4: C11 has no generics). A list it builds is a
  buffer appended to and read back by index (§4.12, "Read back"); nothing writes a list at an index, so neither a
  binary heap nor an adjacency index can be written in the language.
- **Graph logic written by hand.** `stdlib/acl/acl_membership.scxml` walks the groups of a principal breadth first
  over a `list<record:Edge>`: for each group it takes, it scans every edge, and for each edge that matches it scans
  the groups found so far, so its work is `O(G E)` for `G` groups found and `E` edges, where a breadth-first search is
  `O(V + E)`. Its answer is in the order the walk meets the groups, and it fails past 256 groups (its return declares
  `may-fail`). `acl_granted.scxml` walks up a privilege tree from each privilege. Nothing else in the tree searches a
  graph, and each search is a loop of its own, with its own cost and its own order.
- **Recursion.** ADR 0151 D5 (accepted 2026-11-28, not built) derives relations by Zanzibar's rewrites over a table of
  relation tuples, read as Datalog with stratified negation and maintained by each change (DBSP, DRed), on Rust alone,
  its value per viewer. Nested groups are the transitive closure of `member`: a reachability.
- **One comparison.** ADR 0151 D2 (accepted, not built) gives every type one total comparison: integers by number,
  text by code point, records field by field, IEEE 754 totalOrder for floating numbers, an absent value first.
- **Proof.** Verus is pinned, and `docs/formal/proofs.json` declares the Rust files it proves, each with the changes
  it must refuse (ADR 0115 D13: eight files, seven of the host core and the runtime's drain), each read by Verus
  alone: `run_verus` (`sce-build/src/formal/proofs.rs`) runs `verus --crate-type=lib` on one file per entry.
  `<sce:require>` is in the algorithm grammar; `<sce:ensure>` is not, and ADR 0115 D10, an algorithm's contract proved
  by Verus, is not started (ADR 0156, Context).
- **Models.** `sce-codegen model` (`sce-build/src/formal/document_model.rs`) writes a document's TLA+ module over
  `docs/formal/AppendixD.tla` for documents whose expressions are integers and booleans, and refuses by name what it
  cannot translate (`Construct`). A `sce-static` document is refused whole, before any of its elements is read
  (`whole_document`, `Construct::Datamodel("sce-static")`), so no document that holds a map has a model, and a map has
  no module yet (ADR 0156 D2). An element of executable content with no translation is refused as `Construct::Action`,
  naming it.
- **The forge runtime** (`backends/rust/forge-runtime/src`) holds the `queue` and `broadcast-log` algorithms, written
  once and generic over their element, and `view.rs`, the list of a codec view (ADR 0049), whose name ADR 0151 D4 gave
  to its operators' module as well; it has no graph.
- **What a consumer counted.** A downstream consumer's census counts nine rows of graph logic: a link-state router's
  shortest-path trees, one per root, with a walk of predecessors to next hops and distances, a test of whether one
  node reaches another, and the removal of nodes no longer attached (seven rows); and a schema compiler's transitive
  closure of the files a file imports and of the types a root message reaches, each with a check for a cycle (two
  rows). The router computes its trees with a general library's Bellman-Ford over weights that are never negative, and
  breaks equal-cost ties by a term of its own added to each weight, so that two implementations of its protocol choose
  the same hop.

So a graph can be searched today only by a loop written for each use, at a cost no bound states, in an order that
depends on the history of its tables, with nothing that says what it computes, and with no model.

## The textbook structure

- **A graph is a set of vertices and a binary relation on it**, `G = (V, E)` with `E` a subset of `V x V`, and a
  weighted graph adds a weight `w(e)` to each arc (Cormen, Leiserson, Rivest and Stein, *Introduction to Algorithms*,
  3rd ed., MIT Press 2009, Appendix B.4 and section 22.1). A relation is a table (Codd 1970), so a database holds a
  graph as a table of vertices and a table of edges, and a vertex is named by its key and never by an address. The arc
  `(u, v)` has `u` as its tail and `v` as its head (Bang-Jensen and Gutin, *Digraphs*, Springer 2009).
- **Reachability is a least fixed point.** The set of vertices reached from roots `R` is the least set `X` that holds
  every root in `V` and the head of every arc whose tail it holds: a monotone operator on a complete lattice has a
  least fixed point (Tarski 1955), and on a finite lattice of sets it is reached by iterating the operator from the
  empty set, here in at most `|V|` steps (Kleene's iteration; Davey and Priestley, *Introduction to Lattices and
  Order*, 2nd ed., Cambridge University Press 2002, chapter 8). It is the transitive-closure program of Datalog, whose
  meaning is that least fixed point and whose naive and semi-naive evaluations compute it (Abiteboul, Hull and Vianu,
  *Foundations of Databases*, Addison-Wesley 1995, chapters 12 and 13).
- **Breadth-first search** computes the number of arcs of a shortest path from the roots to every vertex, and a
  breadth-first tree of predecessors, in `O(V + E)` (Moore, "The shortest path through a maze", 1959; CLRS 22.2).
- **Shortest paths.** With weights that are never negative, Dijkstra's algorithm settles the vertices in the order of
  their distance (Dijkstra, "A note on two problems in connexion with graphs", *Numerische Mathematik* 1, 1959), in
  `O((V + E) log V)` with a binary heap whose keys decrease (CLRS 6.5 and 24.3). Its correctness is CLRS's Theorem
  24.6, and Corollary 24.7 that the subgraph of the predecessors it records is a tree of shortest paths. Where weights
  may be negative, Bellman-Ford relaxes every arc `|V| - 1` times, in `O(V E)`, and reports a cycle of negative weight
  reached from the source (Bellman 1958; Ford 1956; CLRS 24.1).
- **Reachability and shortest distance are one problem over two semirings** (the algebraic path problem: Carré, "An
  algebra for network routing problems", *Journal of the Institute of Mathematics and its Applications* 1971;
  Backhouse and Carré 1975; Mohri, "Semiring frameworks and algorithms for shortest-distance problems", *Journal of
  Automata, Languages and Combinatorics* 2002): over the booleans with `(or, and)` the closure is reachability, over
  the naturals with infinity adjoined, under `(min, +)`, it is the shortest distance. Datalog over semirings states
  both as the least fixed point of one program (Abo Khamis, Ngo, Pichler, Suciu and Wang, "Convergence of Datalog over
  (pre-)semirings", PODS 2022). What Dijkstra's argument asks of the labels is less than a semiring: a total order;
  isotonicity, that a label below another stays below it when both are extended by one arc, which is what makes a
  subpath of a least path least; and an extension that never decreases a label (Knuth, "A generalization of Dijkstra's
  algorithm", *Information Processing Letters* 6, 1977; Sobrinho 2002 and 2005). A lexicographic pair of naturals
  under componentwise addition keeps all three (Gurney and Griffin, "Lexicographic products in metarouting", ICNP
  2007).
- **A shortest path is not unique, and which one an algorithm returns is a fact of its run.** The tree Dijkstra's
  algorithm records depends on the order of the adjacency lists and on how its heap breaks ties, and the textbook says
  nothing of which tree it is (CLRS 24.3). An answer that is a function of the graph breaks every tie by an order
  fixed before the run.
- **A topological order** of a directed graph exists exactly when it has no cycle (CLRS 22.4, which finds one by
  depth-first search). Kahn's algorithm takes a vertex whose in-degree is zero, removes its arcs, and repeats; it
  stops with the vertices that lie on a cycle or are reached from one left over (Kahn, "Topological sorting of large
  networks", *Communications of the ACM* 1962; CLRS Exercise 22.4-5; Knuth, *The Art of Computer Programming* 1,
  section 2.2.3, Algorithm T).
- **Removing what the roots cannot reach is mark and sweep**: mark by reachability from the roots, sweep the rest
  (McCarthy, "Recursive functions of symbolic expressions and their computation by machine, part I", *Communications
  of the ACM* 1960; Jones, Hosking and Moss, *The Garbage Collection Handbook*, CRC 2011).
- **A function's contract is proved of the code that runs** (Hoare 1969; Lattuada et al., Verus, SOSP 2024), and **a
  caller is reasoned about through the contract of what it calls, not its body** (Hoare, "Procedures and parameters:
  an axiomatic approach", 1971; Meyer 1992). Dijkstra's algorithm over a priority queue has been proved by machine
  (Nordhoff and Lammich, "Dijkstra's shortest path algorithm", *Archive of Formal Proofs* 2012).
- **Keeping a search's answer current under changes is another family**, with its own cost per change and structures
  kept between changes: a breadth-first tree under deletions (Even and Shiloach, *Journal of the ACM* 1981), shortest
  paths under changes (Ramalingam and Reps, *Journal of Algorithms* 1996), and a recursive Datalog view by DRed
  (Gupta, Mumick and Subrahmanian, SIGMOD 1993).

## Decision

### D1. A graph is a relation over two of a machine's tables, declared by name

There is no graph kind and no graph value. A graph is a name for a relation the build reads off two tables the
document already holds:

```xml
<datamodel>
  <data id="self" sce:type="record:NodeId"/>
  <data id="routers" sce:type="map&lt;record:NodeId, record:Router&gt;[routers_max=512]"/>
  <data id="links" sce:type="map&lt;uint64, record:Link&gt;[links_max=4096]"/>
  <data id="cost" sce:type="map&lt;record:NodeId, uint64&gt;[routers_max=512]"/>
  <data id="hop" sce:type="map&lt;record:NodeId, record:NodeId&gt;[routers_max=512]"/>
  <data id="through" sce:type="map&lt;record:NodeId, uint64&gt;[routers_max=512]"/>
</datamodel>
<sce:graph id="net" nodes="routers" edges="links" tail="row.a" head="row.b" weight="row.weight"/>
...
<transition event="topology.changed">
  <sce:prune graph="net" roots="self"/>
  <sce:shortest-paths graph="net" roots="self" distance="cost" first="hop" via="through"/>
</transition>
```

- **`nodes`** names a map or a set: a variable of the statechart, or a map or set slot or local of an algorithm. Its
  keys are the vertices, of the key type `K`.
- **`edges`** names a map or a set of the same kind of place. Each of its entries is one arc. `tail` and `head` are
  expressions over the entry, of type `K`, reading its key as `key` and, for a map, its value as `row`, the names ADR
  0151 D1 gives an entry. `weight`, which may be left out, is an expression over the entry of an unsigned integer
  type.
- **Directed, or not.** Without `symmetric` the graph is directed. `symmetric="true"` reads each entry as two arcs,
  one each way, of the same key and weight, which is an undirected edge (CLRS Appendix B.4); a loop stays one arc. On
  such a graph every edge between two vertices is a cycle of two arcs, so a topological order of it takes only the
  vertices that have no edge.
- **One table may be both.** `nodes` and `edges` may name the same map: each entry is then a vertex and the arc its
  `tail` and `head` give, as a map of parent pointers is. D2 says what a removal does to it.
- **What it means.** The graph is the vertices and the arcs whose tail and head are both vertices: an entry with an
  end that is not a key of `nodes` is no arc (the subgraph induced on `nodes`). Two entries may give arcs of the same
  tail and head, and an arc may have its tail for its head: the graph is a multigraph with loops, its arcs told apart
  by the keys of their entries.
- **It holds no value.** It has no storage, no saved form, no place in the snapshot, and no expression reads it. Each
  statement of D3 reads its two tables where it runs, as they are there, after every write before it in the macrostep.
- **Where it stands.** In a statechart, as a child of `<scxml>` after its `<datamodel>`; in an algorithm, in its
  `<sce:body>` before the first statement that names it.
- **How the build checks it**, where the document is read: `nodes` and `edges` name maps or sets of the document
  (`graph/table-not-map`); `tail` and `head` are of the key type of `nodes` (`graph/endpoint-type-mismatch`); `weight`
  is of an unsigned integer type (`graph/weight-not-unsigned-integer`, D2 and Open question 2 say why); each of the
  three is proven total where it is read (ADR 0133 D1), since a statement that failed halfway would leave a result
  half written (`graph/expression-may-fail`). Its `id` is a name of the document, unique among its graphs
  (`graph/duplicate-graph`).

### D2. One meaning: walks from the roots, labels ordered whole, and every tie broken by the least key

The statements of D3 are each a function of one definition, stated here once and written in Verus and in TLA+ from
this text (D8, D9).

- **Roots.** A statement's `roots` is an expression whose value is a vertex or a `set<K>`. Its roots are the vertices
  among them; a root that is no key of `nodes` is no root.
- **A walk** from the roots to a vertex `v` is a sequence of arcs, each one's head the next one's tail, from a root to
  `v`; a root has the empty walk. `Reach(G, R)` is the set of vertices that have one: the least fixed point of D10.
- **A label.** The label of a walk is the pair of the sum of its arcs' weights and the number of its arcs, compared by
  the sum and then by the number. Every pair of naturals is a label, so every set of labels has a least one, and the
  label of a vertex reached, `label(v)`, is the least label of a walk to it. An arc adds at least `(0, 1)` to a label,
  so a least walk visits no vertex twice and is a path; with zero weights the count of arcs keeps a cycle of weight
  zero from ever being least. Breadth-first search reads every weight as zero, so its label is the count of arcs
  alone.
- **A parent.** For a vertex reached that is not a root, its arc in the tree is, among the arcs into it whose tail is
  reached and whose tail's label plus the arc's own is its label, the arc of the least tail, and among arcs of that
  tail the arc of the least key of its entry. Its `parent` is that arc's tail, its `via` that arc's key, and its
  `first` the vertex after the root on the walk of parents from the root to it, which is itself where its parent is a
  root. A parent's label is below its child's, so the walk of parents ends at a root, the parents form a tree, and
  each vertex's walk in it is a least walk: a canonical shortest-path tree, which depends on the graph alone.
- **The comparison** that says which key is least is ADR 0151 D2's: one total comparison for every type, here of the
  vertices' key type and of the edges' key type, under which two values of one deterministic form (ADR 0101) compare
  equal. Nothing reads a key's place in its table's order, a slot, a seed, a hash or a lane.
- **A topological order.** Of the vertices (or, with `roots`, of those reached), the order in which Kahn's algorithm
  takes them when it always takes the least vertex that has no arc from a vertex not yet taken. The vertices it never
  takes are exactly those that lie on a cycle or are reached from one. Of them, the witness cycle starts at the least,
  steps each time to the least of its predecessors among them (each has one), and is the first repeated stretch of
  that walk, written from its least vertex in the direction of its arcs.
- **Removal.** Pruning from roots leaves in `nodes` the vertices reached, and in `edges` the entries whose tail and
  head both remain. With no root among the vertices nothing is reached, and a removal empties both tables. Where
  `nodes` and `edges` are one table, an entry is kept exactly when its key is reached, and its arc counts only while
  its head is kept too (D1).
- **The answer depends only on the contents.** Each result is a function of the set of keys of `nodes` and the set of
  entries of `edges`: two machines whose tables hold the same entries, put in any order, under any seed and in any
  storage, write results of the same bytes (ADR 0101). A map or a set a statement writes is written in the ascending
  order of its keys by the comparison, so its walk, its saved form and its snapshot are functions of the graph too.

### D3. Five statements read a graph and write named variables whole

| Statement | Writes | What |
|---|---|---|
| `<sce:reach graph roots reached/>` | `reached`, a `set<K>` | `Reach(G, R)` |
| `<sce:breadth-first graph roots hops? parent? first? via?/>` | `hops`, a `map<K, uint32>`; `parent` and `first`, `map<K, K>`; `via`, a `map<K, K_E>` | the count of arcs of a shortest path, and the canonical breadth-first tree |
| `<sce:shortest-paths graph roots distance? parent? first? via?/>` | `distance`, a `map<K, uint64>` or `map<K, uint128>`, by the width D6 chooses; `parent`, `first`, `via` as above | the least weight of a path, and the canonical shortest-path tree |
| `<sce:topological-order graph roots? rank? cycle? outcome?/>` | `rank`, a `map<K, uint32>`; `cycle`, a `list<K>`; `outcome`, a value of `sce:std/graph/order_outcome.scxml` (`acyclic`, `cyclic`) | each vertex taken and its place in the order, the witness cycle, and whether every vertex was taken |
| `<sce:prune graph roots removed?/>` | the graph's two tables, in place; `removed`, a `uint32` | the removal of D2 and the count of vertices it removed |

`K_E` is the key type of `edges`: its key for a map, its element for a set. Roots, the parent tree and the labels are
those of D2; a root has a label of zero and no parent, first or via.

- **A result is written whole.** When the statement ends, each variable it names is emptied and every entry is put
  into it in the ascending order of its keys, a list in its own order; until then nothing is written, so a reader of
  the variable never sees half a result. `<sce:prune>` removes entries from its tables in place, which keeps the order
  of what remains (ADR 0108 D2).
- **It cannot fail, and the build proves it** (ADR 0133 D1): its graph's expressions and its roots are total, each
  result's bound is at least the bound of `nodes` (D6), its labels' widths are proven (D6), and every result is
  written into storage that holds its bound. The statement is classified once as one that cannot fail
  (`Action::native_fails` false, ADR 0133 D2), holds no error path, and raises no `error.execution`.
- **A result is a variable that holds its bound**: a statechart's variable, or an algorithm's local. An algorithm's
  parameter, and an `inout` slot whose storage a host may lend smaller than its bound (ADR 0154 D1), are refused as
  results (`graph/result-target`): a lent slot's limit is known only at run time (ADR 0154 D1), so the build cannot
  prove a result fits it. An algorithm that must leave a result in such a slot writes it into a local and copies it
  with `<sce:put>`s that name their outcomes, or returns the map (ADR 0108 D5). `<sce:prune>` writes the graph's
  tables, which must be a statechart's variables, an algorithm's locals or its `inout` slots: a removal needs no room,
  so a lent table may be pruned.
- **Refused where the document is read**, each by its code: a statement that names no graph of the document
  (`graph/unknown-graph`); roots of another type than `K` or `set<K>` (`graph/roots-type-mismatch`); a result of
  another type than the table says (`graph/result-type-mismatch`); a result whose bound the build cannot prove at
  least that of `nodes` (`graph/result-bound-below-nodes`); a result that is one of the graph's tables, a variable its
  roots or its graph's `tail`, `head` or `weight` read, or the same variable twice (`graph/result-aliases-input`); a
  statement that names no result (`graph/no-result`); `<sce:shortest-paths>` over a graph with no `weight`
  (`graph/unweighted`), since breadth-first search is the one spelling of a search whose arcs weigh alike.
- **The outcome enum is a standard document**, `sce:std/graph/order_outcome.scxml`, imported, generated, saved and
  shown as every enum is, as a map's outcomes are `sce:std/collection/put_outcome.scxml` and `remove_outcome.scxml`
  (ADR 0108 D4).
- **Where a statement stands.** Wherever executable content stands, a `<foreach>` included: a tree per root, as the
  roots change, is a `<sce:shortest-paths>` inside a `<foreach>` over the roots (Open question 10). A statement is a
  writer of every result it names, and `<sce:prune>` of its graph's two tables (ADR 0108 D6, items 2 and 3, as
  amended): a `<foreach>` over a map a statement in its body writes copies the map, and a map a statement writes is
  never an `inout` argument of the same block's call.
- **What leaves a machine, and what a view sees.** Under ADR 0151 D10, each result is labelled with the join of the
  labels of `nodes`, `edges`, the roots, what `tail`, `head` and `weight` read, and the program-counter label, and
  `<sce:prune>` writes both tables at that join; so a public `nodes` pruned through a filtered `edges` is refused
  (`view/filtered-table-flows-out`). Under ADR 0151 D4, each result's emptying and puts and each removal a prune makes
  are deltas of the tables they write, so a view over them is maintained as it is over any write.

### D4. Where the library lives

- **The code is written once, in the Rust forge runtime**: `sce-forge-runtime`, module `graph`, beside the `queue` and
  `broadcast-log` algorithms and the operators of ADR 0151 D4, whose module this record names `relational`, since
  `view` is already the list of a codec view there (`backends/rust/forge-runtime/src/view.rs`, ADR 0049). Each
  statement of D3 is lowered to a call of it, generic over the vertices' and the edges' key types and over the tables'
  storage, as a generated algorithm is generic over a map's storage (ADR 0108 D4, amended 2026-11-21).
- **It is proved once** (D8): its core and its adapter are files Verus reads, declared in `docs/formal/proofs.json`,
  each including the specification it is proved against from the specification-only directory
  `backends/rust/formal-spec/`.
- **Its surface is the dialect's**: a graph is declared and searched by elements of `schemas/sce-forge-ext.xsd`, as a
  map is written by `<sce:put>`, `<sce:remove>` and `<sce:clear>`. The standard library holds what a document imports
  by name: `sce:std/graph/order_outcome.scxml`.
- **It is not a set of algorithm documents of `sce:std`, as `sce:std/acl` is.** Three reasons, each sufficient:
  1. An algorithm document is of concrete types, and one generic over what it is lent was rejected (ADR 0108 D6, item
     4). A graph's vertex is of the application's type, a record of two halves in one and a text in another; a library
     of one vertex type would make every other application copy its keys into that type and back, a second table to
     keep in step.
  2. The algorithm language holds a list as a buffer appended to and read back, never written at an index. Neither a
     binary heap nor the arrays of an adjacency index can be written in it, so the bounds the textbook states (D5)
     cannot be met, as `acl_membership` shows.
  3. An algorithm's contract is an expression of its own values (ADR 0115 D10, and `<sce:ensure>` is not yet in the
     grammar). It has no quantifier over walks, so "the least walk to every vertex" cannot be stated of a document,
     let alone proved of it. Verus states it over the core (D8).

  A document generic over its types is a record of its own (Not in scope); if one lands, the statements of D3 keep
  their meaning and may be served by it on a backend the runtime does not reach.

### D5. A proved core over vertex numbers, and an adapter that numbers vertices in the order of their keys

- **The adapter** turns the tables into the textbook's graph: the vertices numbered `0` to `n - 1` in the ascending
  order of their keys, so a number's order is its key's order (CLRS's `V = {1, ..., n}`); each entry of `edges` whose
  two ends are vertices an arc of their numbers, its weight and the rank of its key among the edges' keys; and the
  arcs held as adjacency arrays, the arcs of each tail together (CLRS 22.1). It writes each result back in the order
  of the numbers, which is the order of the keys, so no result is sorted after the search. It holds a reference or a
  slot position for each key, never a copy of it (D6). It decides nothing: it ranks by the comparison and copies.
- **The core** reads only numbers. It is breadth-first search with a queue of numbers (`<sce:reach>`,
  `<sce:breadth-first>`, the mark of `<sce:prune>`); Dijkstra's algorithm with a binary heap of numbers keyed by the
  label and then the number, whose keys decrease (`<sce:shortest-paths>`); and Kahn's algorithm with a binary heap of
  the numbers whose in-degree is zero (`<sce:topological-order>`). A tie between labels is broken by the number, and
  the arc a vertex keeps is the least by its tail's number and then its key's rank, when the tail is settled: every
  tail whose label plus its arc's is a vertex's label is settled before that vertex, since its label is lower, so the
  arc kept is the least of D2 whatever the order of the arcs.
- **The cost of one statement.** Ranking the keys is `O((V + E) log (V + E))` comparisons; the search is `O(V + E)`
  for reachability, breadth-first search and removal, and `O((V + E) log V)` for shortest paths and the topological
  order. Ranking is the price of an answer that depends only on the contents (D2); a search whose ties followed the
  tables' order would save it and answer by their history. The core returns, with its answer, the counts of its work
  (`GraphWork`: the arcs scanned, the comparisons and the heap's moves), and the bound is a postcondition Verus proves
  of them (C8): at most `|E|` arcs scanned by a search, twice that on a symmetric graph, and heap operations within
  `c (V + E) ceil(log2 V)`. A property test ties the counts to time (Order of work, step 4), and a deployment's
  metrics may report them.
- **Nothing is kept between calls.** The numbers and the adjacency arrays are built at each call from the tables as
  they are there, so no index is kept beside a table, no write of a table pays for one, and nothing can disagree with
  the tables it was built from.

### D6. Bounds, by name

- **No new bound.** The bound of `nodes`, `B_V`, and of `edges`, `B_E`, are the bounds of their types, numbers or
  names (ADR 0050, ADR 0086), and a deployment moves them with `--limit`.
- **A result's bound** is at least `B_V`: the same name, or numbers the build compares. A cycle has at most `B_V`
  vertices, so a `cycle` list's bound is at least `B_V` too.
- **Numbers fit their types, proven by the build.** A vertex's number, an arc's index and a rank are 32-bit numbers,
  and `usize` is 32 bits on `wasm32-unknown-unknown`, so the build proves `B_V <= 2^32 - 1` and `B_E <= 2^32 - 1`; a
  count of arcs, a rank and `removed`, which may be `B_V`, are `uint32`. Otherwise the statement is refused
  (`graph/label-may-overflow`).
- **A distance's width is chosen from the bounds.** Let `max(W)` be the largest value the build's interval analysis
  proves of the `weight` expression (`int_ranges`, ADR 0133 D1), at most its type's largest. A distance is a `uint64`
  where `(B_V - 1) x max(W) <= 2^64 - 1`, a `uint128` where `(B_V - 1) x max(W) <= 2^128 - 1`, and the statement is
  refused past that (`graph/label-may-overflow`). Under `B_V <= 2^32 - 1` a weight of at most 32 bits always takes a
  `uint64`, and one of 64 bits takes a `uint128` unless `B_V` is 2 at most. A `distance` result whose value type is
  narrower than the width chosen is refused (`graph/result-type-mismatch`).
- **`uint128` is a type of the language.** It joins the one type grammar (ADR 0101 section 1; `SCE_FORGE.md` §3.3) as
  an unsigned integer of 128 bits, whose one deterministic form is the CBOR unsigned integer below `2^64` and the
  unsigned bignum (tag 2) of the shortest byte string at or above it (RFC 8949, sections 3.4.3 and 4.2). An expression
  reads, compares and copies one, and its arithmetic is the contract of `SCE_FORGE.md` §3.4.1 at width 128. It is
  served on Rust as `u128`, native and in the `wasm32-unknown-unknown` build; every other backend and the Interpreter
  refuses a document that holds one by name (`generate/unsupported-feature`), as they refuse the statements (D7).
- **Real costs.** An application whose costs are real numbers scales them to an exact integer form at its seam. A
  floating sum along a path can differ from the exact sum only within rounding, and where the two would choose
  different paths the exact integer answer is the correct one. A term that multiplies a weight by a factor near one,
  to choose among equal paths, is reproduced exactly by one integer per arc, the weight times the scaled factor, whose
  sums take a `uint128` where they need more than 64 bits.
- **The working storage** of a call is the positions of the keys and their numbers (`B_V`), the arcs and their keys'
  ranks (`B_E`), the adjacency offsets (`B_V + 1`), and the labels, parents and heap (`B_V`): a term
  `a x B_V + b x B_E` bytes, with `a` and `b` the sizes of a position, a number and a label. It holds references or
  slot positions of keys, never copies of them.
- **Where it is kept is chosen by the type of `nodes`, never by the build's allocator** (ADR 0108 D4, amended
  2026-11-21: a feature of the build would move every map of it to the heap). Where `<S as MapStorage>::Owned` of
  `nodes` is `Heap`, the working storage grows on the heap up to its term; where it is `Inline` or `Linear`, it is
  reserved in place, as such a map's slots are. A statechart's variables keep `DefaultMapStorage` (ADR 0154), so a
  statechart holds one working storage, sized for the largest of its graphs, since no two statements run at once.
- **A graph over a lent table is given its working storage by its host.** The Rust signature of an algorithm whose
  graph reads a slot takes a `GraphScratch` its caller lends: a statechart lends its own working storage; a host lends
  one it holds, or one made over a run of memory it lends, `GraphScratch::over(..)`, sized by the limits of the tables
  it is made for and refused, where it is made, when the run is too short, as ADR 0154 D1 refuses a run past a map's
  bound. A call is therefore never short of room, and a table lent in `Lent` or `LentIndexed` storage never has
  `B_V`-sized arrays reserved beside it, which would undo what lending it saves (ADR 0154 D1).
- **It is derived and never saved**: a restore builds nothing, since each call builds it again.
- **ADR 0118 D3** counts the working storage as one more part of the machine's state bound once that record is
  accepted; until then the build states it in these two names alone.

So nothing a graph statement holds grows past a bound a build can move by name, and no statement fails for want of
room.

### D7. One answer on every lane that serves it, and every other refuses by name

- **The answer is the graph's** (D2), so every run and every lane that serves the statements writes the same bytes for
  the same contents.
- **Rust serves it**, native, on `wasm32-unknown-unknown`, and with no allocator (D6).
- **Every other backend refuses it by name.** `GRAPH_BACKENDS` (`sce-build/src/forge/codegen_matrix.rs`) lists the
  backends that lower `<sce:graph>` and the statements of D3: Rust. A backend it does not list refuses a document that
  holds one with `generate/unsupported-feature`, naming the element, where it renders the document and after the
  document was judged as it is on every backend, as `ALGORITHM_MAP_BACKENDS` refuses a map; C11, C++, Go, Kotlin,
  Python and Swift do so. The Interpreter's lowering refuses it the same way, so the Interpreter never answers a graph
  otherwise.
- **A backend joins** in the change that gives it the library, held to the cases of D8 and to the model (ADR 0115 D8),
  and never by a second meaning.
- **The Interpreter is the reference** each scenario's runs are compared with (ADR 0108, "Order of work"). Until it
  joins, a scenario that holds a graph statement is compared with the cases alone, and until the document has a model
  (D9) what its properties say of it (C7) is *not judged*.

### D8. What is proved, and at which level

Each claim is judged at the level of ADR 0115 that can say it, and none rests on a random test alone. The library is a
function that ends, of every graph of every size: Verus's level (ADR 0115 D13, the level of a function). A protocol's
levels (TLC on a hand model, Apalache, TLAPS) judge what runs and interleaves, and the library does neither.

| Claim | Level | Why that level |
|---|---|---|
| C0. ADR 0151 D2's comparison is a total order on every type a key may have, and two keys compare equal exactly when they are one key under ADR 0108 D1 | Verus: once for each scalar type, and once for one generic lexicographic combinator from which the comparison of a record, a union, a list and an optional value is instantiated in the order the schema declares | it is a statement of every value; a generated comparison would be code Verus does not read, and a combinator proved once covers every record; ADR 0151's ties rest on it too, so it is proved once for both, and ADR 0108 D1 already excludes a floating key |
| C1. For every graph of numbers, the core's answer is the specification's: `Reach`, the labels, the parents, `via`, `first`, the ranks, the witness cycle and the removal of D2 | a Verus theorem for every size, the core's functions carrying their contracts (ADR 0115 D13) | a statement of every graph; the core reads integers and arrays, which Verus reads; a model covers small graphs and a test samples |
| C2. The specification says what D2 says: the label is isotone and never decreases under an arc, so a subpath of a least walk is least; the parents form a tree whose walks are least walks; breadth-first search is the search whose weights are zero; `Reach` is the least fixed point of `fixpoint` and the domain of the labels; removal leaves the subgraph induced on `Reach`; Kahn's order is a topological order of the vertices taken, the least in the comparison's order; the vertices left lie on a cycle or are reached from one; the witness is a cycle | Verus lemmas over the specification functions | a check of the definition against its intent, so that C1 proves what the record means |
| C3. The adapter numbers the vertices in the order of their keys and makes an arc of exactly the entries whose two ends are vertices (both ways on a symmetric graph) | a Verus theorem over the adapter, from C0, with the walks and lookups of a map taken as given: each entry once, the value put at a key (ADR 0108 D8), named as an assumption in `docs/formal/README.md` | the map's own code carries no Verus contract, and the assumption is the map's documented contract, which its kit tests |
| C4. A result depends only on the tables' contents | follows from C0 to C3; and a property test on the Rust lane, over random orders of the puts, seeds and storages | the test covers the assumption C3 names |
| C5. No statement fails | the build's proof for its expressions and its bounds (ADR 0133, D6); and in C1 and C3 Verus's checks of every operation's overflow and every index | the build proves the inputs, and Verus that the code given them neither overflows nor indexes out of range |
| C6. The TLA+ operators of D9 mean the Verus specification | TLC: over the generated cases under a named size bound, over every graph of a small scope enumerated whole (D9), and over C2's laws checked of `Graph.tla` by the same enumeration; both writings are declarative, sentence for sentence with D2, and neither states an algorithm | the specification is written once for each tool and no tool translates between them; the cases and the enumeration hold the two writings together, and a declarative writing can be read against D2 line by line |
| C7. What a document says of its machine, under its graph statements | its generated model, judged by TLC (ADR 0115 D7 and D15), its runs replayed against it on the Rust lane (D8 there); *not judged* until the model exists (D9) | as for every document |
| C8. A call's work is within the bound of D5 | Verus postconditions on the counts the core returns (`GraphWork`: at most `|E|` arcs scanned by a search, heap operations within `c (V + E) ceil(log2 V)`); and a property test that ties the counts to time on random graphs of growing size | the counts are values of the core, so their bound is a theorem; the test shows the counts measure the time |
| C9. The independent model of the cases (Order of work, step 4) computes what D2 says | judgement (ADR 0115 D14), with the laws of C2 checked on its answers before a case is written | no tool reads an intent; a model written apart from the proof is a second reading of it |

- **The proof's files.** The specification is a directory of its own, `backends/rust/formal-spec/`: `fixpoint.rs` (the
  least fixed point, D10), `compare.rs` (the comparison's specification, C0) and `graph.rs` (the specification
  functions of D2 and the lemmas of C2), each of specification functions only, included by path under
  `cfg(verus_only)` by the files that are proved against it, and depended on by no crate, so a dependency points from
  each user to the specification and never from a build tool to the runtime. The core,
  `backends/rust/forge-runtime/src/graph/core.rs` (C1, C8), and the adapter, `adapter.rs` (C3), carry their contracts
  under `cfg(verus_only)`, so the same text is the plain Rust the crate compiles (ADR 0115 D13).
- **What a manifest entry names.** `run_verus` reads one file alone (Context), and the core's contracts are stated
  over the specification's functions. So an entry of `docs/formal/proofs.json` names the crate root its file is read
  from and the specification files it includes (`includes`, ADR 0146 D5 as amended), with the count of functions it
  verifies and the changes it must refuse (Order of work, step 3).
- **The specification cannot be edited to make a proof pass** (ADR 0115 D11): an included specification file is held
  by the isolation as a proved file's contracts are, so a commit that changes it changes nothing else and names its
  reason.

### D9. The model of a document that holds a graph

- **What it depends on.** A graph is over maps, and its keys are records, texts or numbers a comparison orders, so a
  document that holds one has a model only once the `sce-static` data model (its records, bounded texts, enums and
  maps) is translated into the generated model and a map has its module (ADR 0156 D2). Neither is a step of any record
  yet; this record names them as what D9 waits for, and builds nothing of them.
- **Today** `sce-codegen model` refuses such a document whole, by `Construct::Datamodel("sce-static")`
  (`whole_document`, `sce-build/src/formal/document_model.rs`), before any `<sce:graph>` is reached. Once the data
  model is translated and before this record's model lands, a graph statement, as an element of executable content
  with no translation, is refused by `Construct::Action`, naming it.
- **Three modules of the semantics** (ADR 0156 D2), beside `docs/formal/AppendixD.tla`: `docs/formal/Compare.tla`, the
  comparison of ADR 0151 D2, with its laws checked by TLC over small domains (Order of work, step 2);
  `docs/formal/Fixpoint.tla`, the least fixed point of a monotone operator on the subsets of a finite set; and
  `docs/formal/Graph.tla`, the operators of D2 over finite sets and functions (`Arcs`, `Reach` as `Fixpoint`'s least
  fixed point, `Label`, `Parent`, `Via`, `First`, `Rank`, `Left`, `Cycle`, `Prune`). Each is declarative, written
  sentence for sentence with D2 and citing the sentence it states, and none states an algorithm.
- **A statement is read as its specification.** `sce-codegen model` translates a graph statement as the assignment of
  each operator's value to the variable it names, a map written in the order of its keys as the map's own module holds
  a map. It does not translate the core: a caller is reasoned about through the contract of what it calls (Hoare
  1971), and C1 proves the core meets it.
- **An exploration names the keys.** A vertex is a key of the document's own type, so an exploration names the values
  it may take (ADR 0115 D7). The comparison orders them, so a document that holds a graph statement is not data
  independent in its vertices' type in ADR 0156 D5's sense, which asks for equality alone; its verdicts are *holds
  within* the keys named.
- **Held to the proof's specification** (C6). `docs/formal/Graph_cases.tla` is generated from the conformance cases
  under a size bound it names (Order of work, step 4), and from an enumeration of every graph of a small scope: at
  most four vertices, weights 0 to 2, at most two parallel arcs, and every order of the keys (the small-scope
  hypothesis: Jackson, *Software Abstractions*, MIT Press 2006). It states, for each, that each operator's value over
  its graph is the answer of the case or of the specification's own laws; TLC checks it. A second configuration checks
  C2's laws of `Graph.tla` over the same enumeration. A part taken out of `Graph.tla` (ties broken by the greatest
  key, the count of arcs dropped from a label) must make a case fail, by the rule of the part taken out (ADR 0115 D5).
- **A property is not written in the operators it judges.** A property of a document that holds a graph is written in
  `<fixture>.properties.tla` (ADR 0115 D7), since the expression language has no quantifier, and as Bellman's
  conditions of optimality rather than by `Graph.tla`'s own operators, so it cannot hold by being the same text as
  what it checks: the root's distance is zero; for every arc whose two ends are kept, the head's distance is at most
  the tail's plus the weight; every vertex reached that is no root has an arc into it at which that bound is met; and
  the arc its `via` names is the least of those by D2's rule, its tail's key first and then its own.

### D10. Its relation to ADR 0151's recursive relations: one meaning, two evaluations

- **One meaning.** A relation of ADR 0151 D5 that follows itself (nested groups, the `member` of a `member`) and
  `<sce:reach>` are both the least fixed point of a Datalog program over a machine's tables. The least fixed point is
  specified once: in Verus, as `backends/rust/formal-spec/fixpoint.rs`, a file of specification functions only that
  the proofs of both records include by path and no crate depends on (D8), and in TLA+, as `docs/formal/Fixpoint.tla`.
  `Reach` here is stated as its least fixed point, and ADR 0151's strata are stated over the same two modules: its D7
  translates a stratum by `Fixpoint.tla` and its D11's reference evaluator states a stratum's specification by
  `fixpoint` ("Amends").
- **Two evaluations, by two contracts of cost, neither calling the other.**
  - A relation of a view is maintained by each change, at the cost of what the change reaches (ADR 0151 D4, DRed), its
    state derived, kept between changes, per viewer, and its value published.
  - A graph statement is evaluated where it runs, from the tables as they are, at the cost of D5, keeps nothing
    between calls, and writes a machine's variables.
- **Which one a document writes.** A value that must stay current at every change and be published per viewer is a
  view; a decision a transition makes from the tables as they are is a statement. The build refuses neither where the
  other would serve, since their meanings agree by construction: both are the least fixed point of one module.
- **What neither is.** Shortest paths are no relation of ADR 0151 D5: a least label is a minimum through recursion, an
  aggregate its stratified rules do not hold (monotonic aggregation: Ross and Sagiv, PODS 1992). A search kept current
  by each change is no statement of this record (Not in scope).
- **One comparison, one fixed point.** ADR 0151 D2's comparison is specified once, in ADR 0151 D2, and built once,
  with C0, in `sce-portable-bytes`, by whichever of the two records reaches its step first (Order of work, step 2);
  both read it. The same rule holds of `fixpoint.rs` and `Fixpoint.tla`: written once, by whichever record reaches its
  step first, and ADR 0151's step 3 (its theorem T1) depends on them. *Specified* once is the claim, not *evaluated*
  once: ADR 0151's reference evaluator compares values in the cases' neutral form, and C9's independent model has its
  own comparison; the cases hold them together.

### D11. Refusals, and what each construct is on every backend

Each refusal is a `DiagnosticCode` with its row in the appendix of `docs/SCE_ACCEPTED_SUBSET.md`, added in the step
that makes it: `graph/table-not-map`, `graph/endpoint-type-mismatch`, `graph/weight-not-unsigned-integer`,
`graph/expression-may-fail`, `graph/duplicate-graph`, `graph/unknown-graph`, `graph/roots-type-mismatch`,
`graph/result-type-mismatch`, `graph/result-bound-below-nodes`, `graph/result-target`, `graph/result-aliases-input`,
`graph/no-result`, `graph/unweighted` and `graph/label-may-overflow`. A backend that does not serve the statements, or
`uint128`, refuses them with the existing `generate/unsupported-feature` (D7), so no code is added for it. A run of
memory lent for a graph's working storage that is too short is refused where `GraphScratch::over` makes it, as a value
of the runtime and not a diagnostic (D6), since the build cannot see a slot's limit.

| Construct | The build checks | Bounds | Rust | Every other backend and the Interpreter | The model |
|---|---|---|---|---|---|
| `<sce:graph>` | its tables, the types of its ends and weight, their totality, its name (D1) | `B_V` and `B_E`, its tables' | lowered to the adapter's view of the two tables | `generate/unsupported-feature`, naming it | a set of arcs (`Arcs`) once D9 lands; until then refused with the whole document (`Construct::Datamodel("sce-static")`), and by name (`Construct::Action`) once the data model is translated |
| `<sce:reach>`, `<sce:breadth-first>`, `<sce:shortest-paths>` | its graph, roots and results, their types, bounds and widths (D3, D6) | each result's at least `B_V`; the working storage's term in `B_V` and `B_E` | a call of the core | as above | the assignment of `Reach`, `Label`, `Parent`, `Via`, `First`; refused as the row above until then |
| `<sce:topological-order>` | as above, and its outcome's enum | as above | a call of the core | as above | `Rank`, `Left`, `Cycle`; refused as above until then |
| `<sce:prune>` | its graph's tables are writable places of the document (D3) | none beyond its tables' | a call of the core, then removals from the two tables | as above | `Prune`; refused as above until then |
| `sce:std/graph/order_outcome.scxml` | as every enum | none | as every enum | as every enum: an enum is served everywhere, and a backend that refuses the statements never reaches it | as every enum |
| `uint128` (D6) | as every integer type, its arithmetic by §3.4.1 at width 128 | none | `u128`, native and on `wasm32-unknown-unknown` | `generate/unsupported-feature`, naming the type | an integer, once the data model is translated |

### D12. No kind is added, and the contract is checked for generality by the admission test's axes

A graph is a relation over tables that exist and the statements are of the dialect (D1, D3), so SCE_FORGE §10 admits
nothing here. Its three axes still check what the owner's scope rule asks, that the contract is general and nothing in
it is one application's:

| Axis | Holds | Reason |
|---|---|---|
| Domain alignment | yes | Deterministic data: a function of bounded keyed tables whose answer is one form of bytes (ADR 0101) on every lane that serves it, independent of history (D2) |
| Futamura projection compatibility | yes | The graph's tables, the expressions of its ends and weight, the statements, their results and every bound are known where the document is built; only the rows are known at run time, and a statement is one call of compiled code |
| Cross-domain reuse | yes | Network protocols (link-state routing: shortest-path trees and next hops), build and CI systems (the order of dependent targets, and the cycle that blocks it), game networking and robotics control (paths over a map of places), MMORPG data plane and GUI state (what a root object still reaches, and the removal of what it does not), schema and language tooling (the files an import reaches, the types a root reaches, a cycle among them) |

The one application's choices stay in it: how it weighs a link, the term by which it makes two of its implementations
choose the same equal-cost hop (folded into its integer weights, Open question 2), and what it does with what a
statement found.

## Not in scope

A search kept current by each change (a breadth-first tree under deletions, shortest paths under changes: a record of
its own, which would reuse D2's meaning and ADR 0151's maintenance); shortest paths between every pair (a statement
per root serves a router that keeps a tree per root); negative weights and Bellman-Ford (Open question 1); floating
weights (Open question 2); minimum spanning trees, strongly connected components, flows and matchings, which nothing
in the tree or the census asks for; a graph over a view's value or over the tables of several sessions; a document
generic over its types; a value made of references between its parts (recursive data, a gap of its own); any one
application's way of weighing an arc or of choosing between equal paths; and every backend other than Rust, and the
Interpreter, until a consumer needs one (D7).

## Consequences

- A document searches a graph by one statement whose meaning, cost and bounds the build knows, where today it writes a
  loop whose cost and order are its own.
- The answer depends only on the tables' contents: two machines whose tables hold the same entries write the same
  results, whatever order the entries were put in, under any seed, in any storage, on every lane that serves the
  statements.
- Ranking the keys costs a logarithmic factor even where the search itself is linear (D5); that is the price of an
  answer independent of history.
- A machine whose tables hold their slots in place reserves working storage of a term in `B_V` and `B_E`, chosen by
  the tables' type and not by the build's allocator, a host lends it for a lent table (D6), and nothing is held
  between calls.
- The language gains an unsigned integer of 128 bits, served on Rust and refused by name elsewhere, so that a sum of
  64-bit weights along a long path is exact (D6).
- A graph statement runs on Rust alone; every other backend and the Interpreter refuse it by name, and a document that
  must run there writes its loop as it does today.
- The specification is written once for each tool that judges it, Verus and TLA+, and the two are held together by the
  cases (C6); the cases' independent model is a third reading of D2, kept apart on purpose (C9).
- A document that holds a graph statement has no model until the `sce-static` data model is translated, the map's
  module exists and D9 lands; it is refused by name until then, and what its properties say is *not judged* (C7).
- `sce:std/acl/acl_membership` keeps its loop and its order, and a test holds it to `<sce:reach>` as a set (Open
  question 6).

## Alternatives rejected

- **A graph kind, or a graph value of vertices that point at each other.** Its identity would be an address, which has
  no deterministic form, no saved form that two lanes share, and no answer that is a function of contents; and it
  would hold a second copy of what the tables already hold. A relation over two tables has the tables' own saved form
  and is changed by the writes the document already makes.
- **The library as algorithm documents of `sce:std`.** D4: concrete types, no heap in the language, and no contract
  that can state a least walk.
- **A language of generic documents first, and the library written in it.** It would answer more than graphs, and is a
  record of its own; nothing here waits for it, and D4 says how the statements would be served by it if it lands.
- **Ties broken by the order the tables were put in, or by the first arc met.** The answer would depend on the history
  of the tables, and two machines with the same tables would route differently.
- **Ties broken by a hash of the keys.** The hash of a map is keyed by its seed (ADR 0108 D8), so the answer would
  change with the seed; a fixed hash would make the order arbitrary where the comparison already gives one.
- **Results written in the order the search settles the vertices.** It is a function of the graph too, but a second
  rule of order beside the comparison's; one rule for every result is simpler to read and to state.
- **An adjacency index kept beside the edges table.** Every put would pay for it, a restore would have to build it,
  and a second structure could disagree with the table it indexes; built at each call it costs no more than the
  ranking does (D5).
- **Floating weights.** A label is a left fold from the root, `label(v) = fl(label(u) + w)`, so the order of its
  additions is fixed, and rounding is monotone, so weights that are never negative are isotone and never decrease a
  label: Dijkstra's argument would hold, and a long accumulator (Kulisch and Miranker 1981) answers another problem.
  They are refused for three other reasons: TLC has no IEEE floating type, so the model of D9 could not state a label;
  whether the pinned Verus states IEEE addition is to be checked, and C1 cannot rest on it until it is; and a NaN, a
  negative or an infinite weight is data of a run that a statement would have to refuse, so it could no longer be one
  that cannot fail (D3). Integer weights are exact and enough (Open question 2).
- **A weight that is a record of unsigned integers, summed by component and compared lexicographically.** A secondary
  criterion of a route would be a second component. The one real use in hand is a term that multiplies a weight, which
  a lexicographic pair does not reproduce and one integer per arc reproduces exactly (D6); an abstraction is added
  when a second real use needs it (the owner's rules of development), and none does.
- **Bellman-Ford as the one algorithm.** It computes the same answer over weights that are never negative, in `O(V E)`
  where Dijkstra's algorithm takes `O((V + E) log V)`.
- **A general graph library embedded in the runtime.** Its ties, its weights and its allocation are its own, it is
  proved of nothing, and its answers are held to no other lane.
- **Shortest paths as a recursive view with a minimum.** A minimum through recursion is outside ADR 0151's stratified
  rules (D10), and a view keeps its state between changes where a router asks for a tree when its table changes.
- **A search as a host operation.** It decides, and a seam decides nothing (ADR 0156 D7); a decision belongs to a
  document or to a proved library of the runtime.
- **Removal as a view.** A view never writes its tables (ADR 0151 D9); `<sce:prune>` is a write the document asks for.

## Order of work

Each step lands with a test that fails without it.

1. This record, accepted.
2. The comparison of ADR 0151 D2, its proof C0 and `docs/formal/Compare.tla`, unless ADR 0151 has built them first:
   the comparison of each scalar type and one generic lexicographic combinator from which a record's, a union's, a
   list's and an optional value's comparison is instantiated in schema order; `backends/rust/formal-spec/compare.rs`;
   `backends/rust/portable-bytes/tests/every_key_compares_by_one_total_order.rs` (signed integers across zero, a
   text's prefix before what extends it, a record decided by its first field that differs, an absent value first, and
   `two_keys_compare_equal_exactly_when_they_are_one_key` under ADR 0108 D1); its proof's entry in
   `docs/formal/proofs.json`, which must refuse a combinator that compares its last component first; and
   `Compare.tla`'s laws (a total order; equal exactly when one value) checked by TLC over small domains. Red without
   it: the comparison does not exist.
3. The specification, the least fixed point and the core, with C1, C2 and C8's postconditions:
   `backends/rust/formal-spec/fixpoint.rs` and `graph.rs`, and `backends/rust/forge-runtime/src/graph/core.rs`
   declared in `docs/formal/proofs.json` with the crate root it is read from and the specification files it includes.
   Each must refuse the changes that break what it states: the count of arcs dropped from a label (a cycle of weight
   zero then leaves a vertex whose walk of parents never reaches a root), a tie kept by the first arc met, Kahn's
   algorithm taking the first vertex freed rather than the least, an arc kept whose end is no vertex, a search that
   scans every arc once per vertex (C8's postcondition fails), and in `fixpoint.rs` an iteration stopped one step
   short (C2's lemma that it reaches the least fixed point fails). The judge's own test,
   `sce-build/tests/a_proof_holds_the_specification_it_includes.rs`: an entry with `includes` is read with them, and a
   commit that edits an included specification file together with code is refused by the isolation of ADR 0115 D11.
   And `backends/rust/forge-runtime/tests/graph_core.rs`: `a_cycle_of_weight_zero_still_gives_a_tree`,
   `equal_labels_are_broken_by_the_least_number`, `the_vertices_a_topological_order_leaves_lie_on_or_after_a_cycle`.
   Red without it: the module does not exist, and the judge refuses a manifest whose files it cannot find.
4. The adapter, `GraphWork` and the cases, with C3, C4 and C8's tie to time. `tests/forge/conformance/gen_cases.py`
   writes graph cases from recorded seeds under a size bound it names (parallel arcs, loops, symmetric graphs, one
   table that is both, weights of zero, ends that are no vertex, no root among the vertices, ties of every kind, keys
   of an integer, a record and a text), each with the answer of an independent model in the same file, which computes
   reachability by iterating to a fixed point and labels by relaxing every arc until none changes, with C2's laws
   checked on its answers before a case is written; `--check` holds the cases to it.
   `backends/rust/forge-conformance/tests/forge_graph.rs`, calling the module `sce_forge_runtime::graph` directly:
   `every_graph_call_answers_its_case`; `a_graph_answer_has_the_same_bytes_whatever_order_its_tables_were_put_in` (red
   for an adapter that numbers the vertices in put order); `a_graph_call_does_work_within_its_bound` (the counts and
   the time at 10 000 and at 20 000 vertices and arcs within the ratio the bound allows). Red without it: the adapter
   does not exist, and no case is written.
5. The build: the declaration, the statements, their checks and every refusal of D11 with its appendix row, the Rust
   lowering, `GRAPH_BACKENDS`, the elements in `schemas/sce-forge-ext.xsd`, `sce:std/graph/order_outcome.scxml`, and
   `uint128` with its one form. `sce-build/tests/a_graph_is_declared_over_a_machines_tables.rs`: each refusal by its
   code; a document holding each statement generated for Rust, whose generated code holds no error path for it and
   whose internal queue bound is unchanged by it (ADR 0133 D2); a `<foreach>` over a map a statement in its body
   writes, which copies the map; and a statement's result named as an `inout` argument of the same block, refused.
   `sce-build/tests/a_graph_statement_is_refused_by_name_where_no_backend_lowers_it.rs`: C11, C++, Go, Kotlin, Python,
   Swift and the Interpreter's lowering each refusing a graph statement and a `uint128` with
   `generate/unsupported-feature`, naming it. `sce-build/tests/a_uint128_has_one_deterministic_form.rs`: below `2^64`
   an unsigned integer, at and above it a bignum of the shortest bytes. In `forge_graph.rs`, over generated documents:
   `every_graph_statement_answers_its_case`, and `acl_membership_is_the_reach_of_its_edges_as_a_set`, whose `edges`
   list of `record:Edge` becomes a set of arcs and a `nodes` table of every member and group but 0, whose answer is
   `Reach` less the user and less 0, with no group for the user 0, over graphs of at most 256 groups, since
   `acl_membership` fails past 256. A mutation casefile,
   `sce-build/tests/mutations/a_graph_result_holds_the_bound_of_its_nodes.cases`, drops the check of
   `graph/result-bound-below-nodes` and must turn that refusal's case red. With ADR 0151's step 7 (its D10), or with
   this step if that lands first:
   `sce-build/tests/a_value_that_leaves_a_machine_does_not_depend_on_a_filtered_table.rs` gains the case of a public
   `nodes` pruned through a filtered `edges`, refused by `view/filtered-table-flows-out`. Red without it:
   `<sce:graph>` is an unknown element.
6. The machine: `backends/rust/scenarios/tests/graph.rs` over a fixture that keeps a router's table:
   `a_machine_prunes_what_its_roots_no_longer_reach`, `a_prune_from_no_root_empties_both_tables`,
   `a_map_of_parents_is_pruned_by_its_keys`, `an_undirected_link_is_read_both_ways`,
   `equal_cost_paths_are_chosen_by_the_least_key`, `a_graph_statement_reads_the_puts_before_it_in_its_macrostep`,
   `a_tree_per_root_is_written_in_a_foreach_over_the_roots` and
   `a_restored_machine_answers_as_it_did_before_it_was_saved`, run native, on `wasm32-unknown-unknown`, and in a
   `no_std` build with no allocator as `scripts/check_noalloc_codec.sh` builds the `sce:std/text` documents. With ADR
   0151's step 6, or with this step if that lands first:
   `a_view_over_a_graph_result_equals_its_recomputation_after_the_statement`. Red without it: the fixture is refused
   where it is generated.
7. The model (D9), once the `sce-static` data model is translated and the map has its module (ADR 0156 D2), neither of
   which is a step of any record yet: `docs/formal/Fixpoint.tla` and `docs/formal/Graph.tla`, the translation of each
   statement, and `docs/formal/Graph_cases.tla` with the cases under its named bound, the enumeration of the small
   scope and the configuration of C2's laws, judged by `sce-codegen formal models` (ADR 0146); step 6's fixture
   entered in `docs/formal/models.json` with its properties in `<fixture>.properties.tla` as Bellman's conditions of
   optimality (D9), which hold, and a part taken out (ties broken by the greatest key) that TLC must find violating
   them; ADR 0151 D7's and D11's strata stated over `Fixpoint`. Red without it: `sce-codegen model` refuses the
   fixture by `Construct::Datamodel("sce-static")` today, and by `Construct::Action`, naming the statement, once the
   data model is translated.
8. The documents named in "Amends", each with its "Amended by: ADR 0165" line where it carries such lines.

The proof and the cases (steps 3 and 4) come before the statements (step 5), so that a statement is lowered to a core
already proved, as ADR 0151 puts its theorems before its operators.

## Open questions

Each is decided, with its reason.

1. **Bellman-Ford is not in this record.** Over weights that are never negative it computes the answer Dijkstra's
   algorithm does, at `O(V E)`; a weight of an unsigned type cannot be negative, so Dijkstra's precondition is a fact
   of the type and not of a run. Negative weights serve difference constraints (CLRS 24.4), which no consumer has
   asked for. If one does, a statement of its own adds Bellman-Ford with an outcome `negative-cycle` and the cycle it
   found; a statement's name keeps its meaning once published.
2. **A weight is an unsigned integer, never a floating number, and never a record.** The naturals with infinity
   adjoined under `(min, +)` are a semiring and every sum is exact, so a label is one number on every lane. Floating
   weights would keep Dijkstra's argument, since a label is a left fold whose rounding is monotone, but TLC has no
   IEEE type, the pinned Verus is not known to state IEEE addition, and a NaN, a negative or an infinite weight would
   be run-time data a statement must refuse (Alternatives). An application whose costs are real numbers scales them to
   an exact integer form at its seam; a floating sum along a path can differ from the exact sum only within rounding,
   and where the two would choose different paths the exact integer answer is the correct one. Its own way of choosing
   between equal paths, such as a term that multiplies each weight by a factor near one so that two implementations of
   one protocol choose the same hop, is folded into one integer per arc, which reproduces it exactly; the sums then
   take the `uint128` of D6 where they need more than 64 bits. That choice is the application's, and stays in it. A
   record of weights compared lexicographically was considered and is not added (Alternatives).
3. **Results are written in the ascending order of their keys.** A map's order is its walk and its saved form (ADR
   0108 D2), so the order a result is put in is part of its bytes; one rule for every result, the comparison's, makes
   every result a function of the graph (D2), and the adapter's numbering gives it for free (D5).
4. **An entry with an end that is no vertex is no arc, and is not refused.** A machine's tables change one put at a
   time, and a link may name a router a transition has just removed; refusing would make a passing state an error. The
   textbook graph is induced on its vertices, and `<sce:prune>` removes such entries when the document asks.
5. **A tie is broken by the least key.** The comparison's ascending order is the one order every type has (ADR 0151
   D2); an application that wants another choice orders its keys or its weights so.
6. **`sce:std/acl/acl_membership` stays.** It runs on every backend, is held to a reference implementation's answers
   in their order of meeting (`tools/reference-oracles/`), and is bounded by its return; a statement serves Rust alone
   and answers in key order. Its set of groups is `<sce:reach>` of its edges from the user, less the user, and step
   5's test holds the two equal as sets over graphs of at most 256 groups, so the two cannot come to mean different
   things.
7. **The statements stand in an algorithm's body as well as a statechart's.** A schema compiler's closure of imports
   is a function of tables an algorithm is lent; its results are locals (D3), since a lent slot may hold fewer entries
   than its bound, and an algorithm that must leave one in such a slot copies it there with `<sce:put>`s that name
   their outcomes, or returns it.
8. **`first` and `via` are results.** A router forwards by the next hop and the link, and computing them from the
   parents in a document is a loop per vertex up its walk; the core computes them in `O(V)` in the order it settles
   the vertices.
9. **The topological order is a statement.** It serves the census's two schema rows, which check for a cycle among
   imports and among the types a root reaches; Kahn's algorithm with its witness is the textbook form of that check,
   so a cycle is named and not only detected, and build and CI systems order targets by it (D12).
10. **Several roots are one search, and a tree per root is a loop.** A search from a set of roots is the search from a
    vertex joined to each, so a set of roots is one statement. A router that keeps a tree per root, as its roots
    change, writes a `<sce:shortest-paths>` inside a `<foreach>` over the roots, each iteration writing a local result
    that the body puts into a map of trees keyed by root (ADR 0108 D12); a statement in a `<foreach>` is allowed as
    any executable content is (D3).
11. **An undirected graph is declared, not refused.** Link-state topologies and many of D12's domains are undirected;
    `symmetric="true"` reads each entry as two arcs, the textbook reading of an undirected edge (CLRS Appendix B.4),
    so an application need not keep each edge twice.
12. **One table may be vertices and arcs at once.** A map of parent pointers is a graph whose every entry is both;
    refusing it would make its holder copy the table into a second one. A removal keeps an entry exactly when its key
    is reached (D2), the one rule that writes the table once.

## Amended on acceptance

Each amendment is the independent review's, by its number there, applied as the review recommends and decided by the
main session; amendment 5 carries the main session's later decision on the downstream consumer's answer.

1. D9, D11, Context, D7 and step 7: today a `sce-static` document is refused whole by
   `Construct::Datamodel("sce-static")`, and `Construct::Action` only once its data model is translated; D9 depends on
   that translation and on the map's module, neither stepped; C7 is *not judged* until then.
2. D8, D10 and step 3: the specification is a specification-only directory, `backends/rust/formal-spec/`, included by
   path and depended on by no crate; a manifest entry names its crate root and `includes`, held by ADR 0115 D11's
   isolation; "Amends" gains ADR 0115 D11 and D13 and ADR 0146 D5; step 3 gains the judge's test.
3. D3, "Amends" and steps 5 and 6: a graph statement is certified under ADR 0151 D10, its writes are deltas under ADR
   0151 D4, and it is a writer under ADR 0108 D6 items 2 and 3; the certification case and the view's recomputation
   test are added.
4. The textbook structure and C2: Dijkstra's conditions are a total order, isotonicity and an extension that never
   decreases a label (Knuth 1977; Sobrinho 2002 and 2005), kept by a lexicographic pair (Gurney and Griffin 2007); C2
   gains the lemma.
5. Open question 2, D6 and "Alternatives rejected": floating weights are refused for TLC's want of an IEEE type,
   Verus's statement of IEEE addition unchecked, and run-time NaN, negative and infinite weights, not for the order of
   additions; the lexicographic record of weights is not added, since the consumer's tie term multiplies a weight and
   one integer per arc reproduces it; D6 chooses a `uint64` or a `uint128` distance from the bounds, `uint128` joining
   the type grammar and served on Rust; real costs are scaled to exact integers at the application's seam.
6. D6: the working storage is chosen by the type of `nodes`, not the build's allocator; a graph over a lent table
   takes a `GraphScratch` its host lends, refused where it is made when too short; keys are held by reference or slot
   position. The refusal `graph/lent-without-scratch` the review offered is not added: the build cannot see a slot's
   storage, so the refusal is the runtime's, where the scratch is made.
7. D6: `B_V <= 2^32 - 1` and `B_E <= 2^32 - 1`, since `removed` may be `B_V` and `usize` is 32 bits on `wasm32`; a
   64-bit weight takes a `uint128` distance unless `B_V` is 2 at most.
8. D9 and step 2: the comparison's module is `docs/formal/Compare.tla`, built in step 2 with its laws checked by TLC.
9. D9 and step 7: a fixture's properties are written in `<fixture>.properties.tla` as Bellman's conditions of
   optimality, not by `Graph.tla`'s operators.
10. C6 and D9: both writings of the specification are declarative, sentence for sentence with D2; `Graph_cases.tla`
    adds an exhaustive enumeration of a small scope (Jackson 2006), and a configuration checks C2's laws of
    `Graph.tla`.
11. D5, C8 and step 3: `GraphWork`'s bounds are Verus postconditions on the counts the core returns; the property test
    ties them to time; step 3 gains the mutant of a search that scans every arc once per vertex.
12. C0 and step 2: a record's comparison is one generic lexicographic combinator proved once and instantiated in
    schema order; C0 gains "equal exactly when one key under ADR 0108 D1", with its test.
13. Steps 4 and 5 and Open question 6: step 4's case test calls the runtime module; the statement-level case test and
    the `acl_membership` test move to step 5, the latter with the mapping of a list of edges, less the user and 0,
    over at most 256 groups.
14. D1, D2, D3, Open questions 10 to 12 and step 6: one table may be vertices and arcs at once; a prune from no root
    empties both tables; `graph/result-aliases-input` covers what the roots and the expressions read; a statement may
    stand in a `<foreach>`; `symmetric="true"` declares an undirected graph.
15. D4 and "Amends": ADR 0151 D4's module is named `relational`, since `view` is taken
    (`backends/rust/forge-runtime/src/view.rs`).
16. D10: the fixed point is written once by whichever record reaches its step first, as the comparison is, and ADR
    0151's step 3 depends on it.
17. The textbook structure and Sources: Tarski gives existence and Kleene's iteration reaches it (Davey and Priestley
    2002); `(min, +)` is over the naturals with infinity; CLRS 22.4 sorts by depth-first search and Kahn's is Exercise
    22.4-5; Dijkstra's correctness is Theorem 24.6 with Corollary 24.7; Moore 1959 is added.
18. Context: `acl_membership` is `O(G E)` and fails past 256 groups; `proofs.json` holds seven files of the host core
    and the runtime's drain.
19. "Amends" and D10: ADR 0151 D2 gains a quoted sentence; the comparison is *specified* once, and the evaluators are
    held together by the cases.
20. "Amends": `Compare.tla`, `Fixpoint.tla` and `Graph.tla` join the semantics surface of ADR 0156 D2 in
    `SCE_WIRE_CONTRACTS.md` when it is registered.
21. Steps 3 and 5: the `fixpoint.rs` mutant of an iteration one step short; a graph statement's code holds no error
    path and leaves the internal queue bound unchanged.
22. D3 and Open question 7: the refusal of results in `inout` slots is kept, with the pattern of a local copied by
    `<sce:put>`s that name their outcomes, or a map returned.
23. Open question 9: the topological order is kept, as what serves the census's two schema rows.

## Sources

Cormen, Leiserson, Rivest and Stein 2009 (*Introduction to Algorithms*, 3rd ed., MIT Press: Appendix B.4; sections
6.5, 22.1, 22.2, 22.4 and Exercise 22.4-5, 24.1, 24.3 with Theorem 24.6 and Corollary 24.7, 24.4 and 24.5); Dijkstra
1959 ("A note on two problems in connexion with graphs", *Numerische Mathematik* 1); Moore 1959 ("The shortest path
through a maze", *Proceedings of an International Symposium on the Theory of Switching*, Harvard University Press);
Bellman 1958 ("On a routing problem", *Quarterly of Applied Mathematics*); Ford 1956 (*Network Flow Theory*, RAND
paper P-923); Kahn 1962 ("Topological sorting of large networks", *Communications of the ACM*); Knuth (*The Art of
Computer Programming*, volume 1, section 2.2.3); Knuth 1977 ("A generalization of Dijkstra's algorithm", *Information
Processing Letters* 6); Sobrinho 2002 ("Algebra and algorithms for QoS path computation and hop-by-hop routing in the
Internet", *IEEE/ACM Transactions on Networking*) and 2005 ("An algebraic theory of dynamic network routing",
*IEEE/ACM Transactions on Networking*); Gurney and Griffin 2007 ("Lexicographic products in metarouting", ICNP);
Bang-Jensen and Gutin 2009 (*Digraphs: Theory, Algorithms and Applications*, 2nd ed., Springer); Codd 1970 ("A
relational model of data for large shared data banks", *Communications of the ACM*); Tarski 1955 ("A
lattice-theoretical fixpoint theorem and its applications", *Pacific Journal of Mathematics*); Davey and Priestley
2002 (*Introduction to Lattices and Order*, 2nd ed., Cambridge University Press); Abiteboul, Hull and Vianu 1995
(*Foundations of Databases*, Addison-Wesley, chapters 12 and 13); Carré 1971 ("An algebra for network routing
problems", *Journal of the Institute of Mathematics and its Applications*); Backhouse and Carré 1975 ("Regular algebra
applied to path-finding problems", *Journal of the Institute of Mathematics and its Applications*); Mohri 2002
("Semiring frameworks and algorithms for shortest-distance problems", *Journal of Automata, Languages and
Combinatorics*); Abo Khamis, Ngo, Pichler, Suciu and Wang 2022 ("Convergence of Datalog over (pre-)semirings", PODS);
Ross and Sagiv 1992 ("Monotonic aggregation in deductive databases", PODS); Gupta, Mumick and Subrahmanian 1993
("Maintaining views incrementally", SIGMOD); Even and Shiloach 1981 ("An on-line edge-deletion problem", *Journal of
the ACM*); Ramalingam and Reps 1996 ("An incremental algorithm for a generalization of the shortest-path problem",
*Journal of Algorithms*); McCarthy 1960 ("Recursive functions of symbolic expressions and their computation by
machine, part I", *Communications of the ACM*); Jones, Hosking and Moss 2011 (*The Garbage Collection Handbook*, CRC
Press); Hoare 1969 ("An axiomatic basis for computer programming", *Communications of the ACM*); Hoare 1971
("Procedures and parameters: an axiomatic approach", in *Symposium on Semantics of Algorithmic Languages*, Lecture
Notes in Mathematics 188, Springer); Meyer 1992 ("Applying design by contract", *IEEE Computer*); Lattuada et al. 2024
(Verus, SOSP); Nordhoff and Lammich 2012 ("Dijkstra's shortest path algorithm", *Archive of Formal Proofs*); Jackson
2006 (*Software Abstractions: Logic, Language, and Analysis*, MIT Press); Kulisch and Miranker 1981 (*Computer
Arithmetic in Theory and Practice*, Academic Press); RFC 8949 (*Concise Binary Object Representation*, 2020: sections
3.4.3 and 4.2).

The record's author cited them from the texts as known and read none again. The independent review reports the CLRS
sections, Dijkstra 1959, Bellman 1958, Ford 1956, Kahn 1962, Knuth's Algorithm T, Carré 1971, Backhouse and Carré
1975, Mohri 2002, Abo Khamis et al. 2022, Ross and Sagiv 1992, Gupta, Mumick and Subrahmanian 1993, Even and Shiloach
1981, Ramalingam and Reps 1996, Hoare 1971 and Nordhoff and Lammich 2012 checked, and corrected what amendment 17
lists; Knuth 1977, Sobrinho 2002 and 2005 and Gurney and Griffin 2007 are cited as the review gave them. The passages
D2, D5 and D10 rest on are to be read against the texts in the step that builds each, before the step lands: CLRS's
Theorem 24.6 and Corollary 24.7, and Knuth 1977's and Gurney and Griffin's conditions, on which the claim that a
lexicographic label keeps Dijkstra's proof rests; Kahn's statement of what his algorithm leaves when the graph has a
cycle; the least fixed point and Kleene's iteration (Tarski; Davey and Priestley, chapter 8; Abiteboul, Hull and
Vianu, chapter 12); Ross and Sagiv's account of a minimum through recursion; and RFC 8949's preferred serialization of
a bignum. Whether the pinned Verus states IEEE addition was not checked (amendment 5). The census of the downstream
consumer was read for this record, its catalogue of gaps and the nine rows it files under graphs; it is held outside
this repository and is not cited by place, and its answer on the form of its tie term reached this record through the
main session. The tree was read at `662e4d060cc` for every file this record names.
