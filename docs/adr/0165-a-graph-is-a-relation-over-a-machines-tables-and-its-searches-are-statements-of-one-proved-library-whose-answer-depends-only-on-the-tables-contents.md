# ADR 0165 — A graph is a relation over a machine's tables, and its searches are statements of one proved library whose answer depends only on the tables' contents

- Status: Proposed (2026-11-28), for the owner's acceptance after an independent review.
- Landed: nothing. No step of "Order of work" is built, and no proof, model or test of it has been run.
- Date: 2026-11-28
- Written under the owner's standing rule (the most long-term correct structure, textbook in its foundations, cost
  ignored) and the owner's scope rule of 2026-11-22: a facility is built on the backends a consumer needs, Rust by
  default; its contract is general, and what is special to one application stays in the application.
- Requested by: a downstream consumer's census of its hand-written logic, whose catalogue names the gap "a general
  graph: shortest-path trees, reachability, removal of detached nodes" in nine rows. The census is held outside this
  repository; Context says what it found, and D12 checks that nothing here is shaped by that one application.
- Amends (each amended document gains an "Amended by: ADR 0165" line in the landing that moves it):
  - `docs/SCE_ACCEPTED_SUBSET.md` §2.15 (a graph declared over a machine's tables, D1, and the five statements that
    read it, D3) and the appendix (D11's codes);
  - `SCE_FORGE.md` §4.12 (the declaration and the statements in an algorithm's body, D3);
  - `schemas/sce-forge-ext.xsd` (the elements of D1 and D3, an addition to a `pre-release` surface of
    `SCE_WIRE_CONTRACTS.md`);
  - ADR 0151 D2: its comparison is the one a graph's ties are broken by, built once for both records (D2, D10);
  - ADR 0151 D7 and D11: a stratum's least fixed point is stated over the one specification of a least fixed point
    this record writes, in Verus and in TLA+ (D10);
  - ADR 0156 D2: two more modules of the semantics, `Fixpoint.tla` and `Graph.tla` (D9);
  - ADR 0118 D3, once it is accepted: the working storage of a machine's graph statements is one more part of its
    state bound (D6);
  - `docs/formal/README.md` and `docs/formal/proofs.json` (the theorems of D8).
- Scope: `sce-build` (the declaration, the statements, their checks, the Rust lowering and the model's translation),
  the Rust forge runtime `sce-forge-runtime` (a module `graph`: the specification, the proved core and the adapter
  that feeds it), `sce-portable-bytes` (the comparison of ADR 0151 D2, where that record has not built it first),
  `stdlib/graph` (one outcome enum), `tests/forge/conformance` (the cases) and `docs/formal`. The Rust lane serves it,
  native, on `wasm32-unknown-unknown` and with no allocator. Every other backend, and the Interpreter, refuses it by
  name (D7).
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
  the groups found so far, so its work grows with the square of what it finds times the edges, where a breadth-first
  search is linear. Its answer is in the order the walk meets the groups, cut at 256. `acl_granted.scxml` walks up a
  privilege tree from each privilege. Nothing else in the tree searches a graph, and each search is a loop of its own,
  with its own cost and its own order.
- **Recursion.** ADR 0151 D5 (accepted 2026-11-28, not built) derives relations by Zanzibar's rewrites over a table of
  relation tuples, read as Datalog with stratified negation and maintained by each change (DBSP, DRed), on Rust alone,
  its value per viewer. Nested groups are the transitive closure of `member`: a reachability.
- **One comparison.** ADR 0151 D2 (accepted, not built) gives every type one total comparison: integers by number,
  text by code point, records field by field, IEEE 754 totalOrder for floating numbers, an absent value first.
- **Proof.** Verus is pinned, and `docs/formal/proofs.json` declares the Rust files it proves, each with the changes
  it must refuse (ADR 0115 D13: eight files of the host core and the runtime's drain). `<sce:require>` is in the
  algorithm grammar; `<sce:ensure>` is not, and ADR 0115 D10, an algorithm's contract proved by Verus, is not started
  (ADR 0156, Context).
- **Models.** `sce-codegen model` (`sce-build/src/formal/document_model.rs`) writes a document's TLA+ module over
  `docs/formal/AppendixD.tla`, and refuses by name what it cannot translate (`Construct`, among them
  `Construct::Action`, an element of executable content with no translation). A map has no module yet (ADR 0156 D2).
- **The forge runtime** (`backends/rust/forge-runtime/src`) holds the `queue` and `broadcast-log` algorithms, written
  once and generic over their element; it has no graph.
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
  every root in `V` and the head of every arc whose tail it holds: a monotone operator on a finite lattice of sets has
  a least fixed point (Tarski 1955), reached by iterating it from the empty set in at most `|V|` steps. It is the
  transitive-closure program of Datalog, whose meaning is that least fixed point and whose naive and semi-naive
  evaluations compute it (Abiteboul, Hull and Vianu, *Foundations of Databases*, Addison-Wesley 1995, chapters 12 and
  13).
- **Breadth-first search** computes the number of arcs of a shortest path from the roots to every vertex, and a
  breadth-first tree of predecessors, in `O(V + E)` (CLRS 22.2).
- **Shortest paths.** With weights that are never negative, Dijkstra's algorithm settles the vertices in the order of
  their distance (Dijkstra, "A note on two problems in connexion with graphs", *Numerische Mathematik* 1, 1959), in
  `O((V + E) log V)` with a binary heap whose keys decrease (CLRS 6.5 and 24.3). Its proof rests on two facts:
  extending a path never decreases its weight, and the subgraph of the predecessors it records is a tree of shortest
  paths (the path-relaxation and predecessor-subgraph properties, CLRS 24.5). Where weights may be negative,
  Bellman-Ford relaxes every arc `|V| - 1` times, in `O(V E)`, and reports a cycle of negative weight reached from the
  source (Bellman 1958; Ford 1956; CLRS 24.1).
- **Reachability and shortest distance are one problem over two semirings** (the algebraic path problem: Carré, "An
  algebra for network routing problems", *Journal of the Institute of Mathematics and its Applications* 1971;
  Backhouse and Carré 1975; Mohri, "Semiring frameworks and algorithms for shortest-distance problems", *Journal of
  Automata, Languages and Combinatorics* 2002): over the booleans with `(or, and)` the closure is reachability, over
  the naturals with `(min, +)` it is the shortest distance. Datalog over semirings states both as the least fixed
  point of one program (Abo Khamis, Ngo, Pichler, Suciu and Wang, "Convergence of Datalog over (pre-)semirings", PODS
  2022). What Dijkstra's argument asks of the labels is less than a semiring: a total order, an extension that never
  decreases a label, and an arc whose label is above the label of no arc. A pair of a weight and a count of arcs,
  ordered lexicographically, keeps all three.
- **A shortest path is not unique, and which one an algorithm returns is a fact of its run.** The tree Dijkstra's
  algorithm records depends on the order of the adjacency lists and on how its heap breaks ties, and the textbook says
  nothing of which tree it is (CLRS 24.3). An answer that is a function of the graph breaks every tie by an order
  fixed before the run.
- **A topological order** of a directed graph exists exactly when it has no cycle (CLRS 22.4). Kahn's algorithm takes
  a vertex whose in-degree is zero, removes its arcs, and repeats; it stops with the vertices that lie on a cycle or
  are reached from one left over (Kahn, "Topological sorting of large networks", *Communications of the ACM* 1962;
  Knuth, *The Art of Computer Programming* 1, section 2.2.3, Algorithm T).
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
  head both remain.
- **The answer depends only on the contents.** Each result is a function of the set of keys of `nodes` and the set of
  entries of `edges`: two machines whose tables hold the same entries, put in any order, under any seed and in any
  storage, write results of the same bytes (ADR 0101). A map or a set a statement writes is written in the ascending
  order of its keys by the comparison, so its walk, its saved form and its snapshot are functions of the graph too.

### D3. Five statements read a graph and write named variables whole

| Statement | Writes | What |
|---|---|---|
| `<sce:reach graph roots reached/>` | `reached`, a `set<K>` | `Reach(G, R)` |
| `<sce:breadth-first graph roots hops? parent? first? via?/>` | `hops`, a `map<K, uint32>`; `parent` and `first`, `map<K, K>`; `via`, a `map<K, K_E>` | the count of arcs of a shortest path, and the canonical breadth-first tree |
| `<sce:shortest-paths graph roots distance? parent? first? via?/>` | `distance`, a `map<K, uint64>`; `parent`, `first`, `via` as above | the least weight of a path, and the canonical shortest-path tree |
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
  results (`graph/result-target`). `<sce:prune>` writes the graph's tables, which must be a statechart's variables, an
  algorithm's locals or its `inout` slots: a removal needs no room, so a lent table may be pruned.
- **Refused where the document is read**, each by its code: a statement that names no graph of the document
  (`graph/unknown-graph`); roots of another type than `K` or `set<K>` (`graph/roots-type-mismatch`); a result of
  another type than the table says (`graph/result-type-mismatch`); a result whose bound the build cannot prove at
  least that of `nodes` (`graph/result-bound-below-nodes`); a result that is one of the graph's tables, or the same
  variable twice (`graph/result-aliases-input`); a statement that names no result (`graph/no-result`);
  `<sce:shortest-paths>` over a graph with no `weight` (`graph/unweighted`), since breadth-first search is the one
  spelling of a search whose arcs weigh alike.
- **The outcome enum is a standard document**, `sce:std/graph/order_outcome.scxml`, imported, generated, saved and
  shown as every enum is, as a map's outcomes are `sce:std/collection/put_outcome.scxml` and `remove_outcome.scxml`
  (ADR 0108 D4).

### D4. Where the library lives

- **The code is written once, in the Rust forge runtime**: `sce-forge-runtime`, module `graph`, beside the `queue` and
  `broadcast-log` algorithms and the operators of ADR 0151 D4. Each statement of D3 is lowered to a call of it,
  generic over the vertices' and the edges' key types and over the tables' storage, as a generated algorithm is
  generic over a map's storage (ADR 0108 D4, amended 2026-11-21).
- **It is proved once, there** (D8): its specification, its core and its adapter are files Verus reads, declared in
  `docs/formal/proofs.json`.
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
  of the numbers, which is the order of the keys, so no result is sorted after the search. It decides nothing: it
  ranks by the comparison and copies.
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
  tables' order would save it and answer by their history. The runtime counts the comparisons, the arcs scanned and
  the heap's moves of each call (`GraphWork`), so a test holds the bound (Order of work, step 4) and a deployment's
  metrics may report it.
- **Nothing is kept between calls.** The numbers and the adjacency arrays are built at each call from the tables as
  they are there, so no index is kept beside a table, no write of a table pays for one, and nothing can disagree with
  the tables it was built from.

### D6. Bounds, by name

- **No new bound.** The bound of `nodes`, `B_V`, and of `edges`, `B_E`, are the bounds of their types, numbers or
  names (ADR 0050, ADR 0086), and a deployment moves them with `--limit`.
- **A result's bound** is at least `B_V`: the same name, or numbers the build compares. A cycle has at most `B_V`
  vertices, so a `cycle` list's bound is at least `B_V` too.
- **A label fits its type, proven by the build**: a distance is a `uint64`, and the build proves
  `(B_V - 1) x max(W) <= 2^64 - 1`, `W` the type of `weight`; a count of arcs, a rank and `removed` are `uint32`, and
  the build proves `B_V <= 2^32`. Otherwise the statement is refused (`graph/label-may-overflow`). A weight of at most
  32 bits passes under every bound of at most `2^32`.
- **The working storage** of a call is the numbers of the keys (`B_V` keys and their numbers), the arcs and their
  keys' ranks (`B_E`), the adjacency offsets (`B_V + 1`), the labels, parents and heap (`B_V`): a term
  `a x B_V + b x B_E` bytes, with `a` and `b` the sizes of the key types and the labels. A statechart holds one
  working storage, sized for the largest of its graphs, since no two statements run at once; an algorithm holds its
  own in the owned storage of its `nodes` map (ADR 0154 D3). On a lane with an allocator it grows up to its term, as a
  `Heap` map does; with none it is reserved, as an `Inline` map is (ADR 0108 D4). It is derived and never saved: a
  restore builds nothing, since each call builds it again.
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
  joins, a scenario that holds a graph statement is compared with the document's generated model (D9) and with the
  cases, not with the Interpreter.

### D8. What is proved, and at which level

Each claim is judged at the level of ADR 0115 that can say it. No claim of what an answer is rests on a random test
alone; the one claim a test alone judges is the cost (C8), since no level of ADR 0115 states a cost. The library is a
function that ends, of every graph of every size: Verus's level (ADR 0115 D13, the level of a function). A protocol's
levels (TLC on a hand model, Apalache, TLAPS) judge what runs and interleaves, and the library does neither.

| Claim | Level | Why that level |
|---|---|---|
| C0. ADR 0151 D2's comparison is a total order on every type a key may have | Verus, once per way of making a type (an integer, a boolean, a text and a byte string, an enum, a record, a union, a list, an optional value), each from the parts it is made of | it is a statement of every value, and a record's or a list's comparison is total because its parts' are; ADR 0151's ties rest on it too, so it is proved once for both |
| C1. For every graph of numbers, the core's answer is the specification's: `Reach`, the labels, the parents, `via`, `first`, the ranks, the witness cycle and the removal of D2 | a Verus theorem for every size, the core's functions carrying their contracts (ADR 0115 D13) | a statement of every graph; the core reads integers and arrays, which Verus reads; a model covers small graphs and a test samples |
| C2. The specification says what D2 says: the parents form a tree whose walks are least walks; breadth-first search is the search whose weights are zero; `Reach` is the least fixed point of `Fixpoint` and the domain of the labels; removal leaves the subgraph induced on `Reach`; Kahn's order is a topological order of the vertices taken, the least in the comparison's order; the vertices left lie on a cycle or are reached from one; the witness is a cycle | Verus lemmas over the specification functions | a check of the definition against its intent, so that C1 proves what the record means |
| C3. The adapter numbers the vertices in the order of their keys and makes an arc of exactly the entries whose two ends are vertices | a Verus theorem over the adapter, from C0, with the walks and lookups of a map taken as given: each entry once, the value put at a key (ADR 0108 D8), named as an assumption in `docs/formal/README.md` | the map's own code carries no Verus contract, and the assumption is the map's documented contract, which its kit tests |
| C4. A result depends only on the tables' contents | follows from C0 to C3; and a property test on the Rust lane, over random orders of the puts, seeds and storages | the test covers the assumption C3 names |
| C5. No statement fails | the build's proof for its expressions and its bounds (ADR 0133, D6); and in C1 and C3 Verus's checks of every operation's overflow and every index | the build proves the inputs, and Verus that the code given them neither overflows nor indexes out of range |
| C6. The TLA+ operators of D9 mean the Verus specification | TLC over the generated cases (D9): agreement on every case, not a proof | the specification is written once for each tool, and no tool translates between them; the cases hold the two writings together |
| C7. What a document says of its machine, under its graph statements | its generated model, judged by TLC (ADR 0115 D7 and D15), its runs replayed against it on the Rust lane (D8 there) | as for every document |
| C8. A call's work is within the bound of D5 | a property test of `GraphWork` on random graphs of growing size | ADR 0118 is Proposed, and no level of a theorem of cost exists |
| C9. The independent model of the cases (Order of work, step 4) computes what D2 says | judgement (ADR 0115 D14), with the laws of C2 checked on its answers before a case is written | no tool reads an intent; a model written apart from the proof is a second reading of it |

- **The proof's files.** `backends/rust/forge-runtime/src/graph/spec.rs` (the specification functions and the lemmas
  of C2), `core.rs` (C1) and `adapter.rs` (C3), each a file Verus reads alone with its contracts present only under
  `cfg(verus_only)`, so the same text is the plain Rust the crate compiles (ADR 0115 D13), each declared in
  `docs/formal/proofs.json` with the count of functions it verifies and the changes it must refuse (Order of work,
  step 3). The least fixed point is `sce-forge-runtime`'s module `fixpoint`, of specification functions only (D10).
- **The specification cannot be edited to make a proof pass** (ADR 0115 D11): the specification functions and the
  contracts are changed by a commit that changes nothing else and names its reason.

### D9. The model of a document that holds a graph

- **Two modules of the semantics** (ADR 0156 D2), beside `docs/formal/AppendixD.tla`: `docs/formal/Fixpoint.tla`, the
  least fixed point of a monotone operator on the subsets of a finite set, reached by iterating it from the empty set;
  and `docs/formal/Graph.tla`, the operators of D2 over finite sets and functions (`Arcs`, `Reach` as `Fixpoint`'s
  least fixed point, `Label`, `Parent`, `Via`, `First`, `Rank`, `Left`, `Cycle`, `Prune`), each citing the sentence of
  D2 it states. The comparison of ADR 0151 D2 is read from the module that states it for a view's order.
- **A statement is read as its specification.** `sce-codegen model` translates a graph statement as the assignment of
  each operator's value to the variable it names, a map written in the order of its keys as the map's own module holds
  a map. It does not translate the core: a caller is reasoned about through the contract of what it calls (Hoare
  1971), and C1 proves the core meets it.
- **An exploration names the keys.** A vertex is a key of the document's own type, so an exploration names the values
  it may take (ADR 0115 D7). The comparison orders them, so a document that holds a graph statement is not data
  independent in its vertices' type in ADR 0156 D5's sense, which asks for equality alone; its verdicts are *holds
  within* the keys named.
- **Held to the proof's specification.** `docs/formal/Graph_cases.tla` is generated from the conformance cases (Order
  of work, step 4) and states, for each case, that each operator's value over its graph is the case's answer; TLC
  checks it (C6). A part taken out of `Graph.tla` (ties broken by the greatest key, the count of arcs dropped from a
  label) must make a case fail, by the rule of the part taken out (ADR 0115 D5).
- **Until it is built**, `sce-codegen model` refuses a document that holds `<sce:graph>` or a statement of D3 by name,
  through its refusal of executable content with no translation (`Construct::Action`,
  `sce-build/src/formal/document_model.rs`), naming the element. It needs the module of a map first, which ADR 0156 D2
  lists among the constructs that have none.

### D10. Its relation to ADR 0151's recursive relations: one meaning, two evaluations

- **One meaning.** A relation of ADR 0151 D5 that follows itself (nested groups, the `member` of a `member`) and
  `<sce:reach>` are both the least fixed point of a Datalog program over a machine's tables. The least fixed point is
  specified once: in Verus, as `sce-forge-runtime`'s module `fixpoint`, of specification functions only, which the
  reference evaluator of ADR 0151 D11 imports without any code the runtime runs, and in TLA+, as
  `docs/formal/Fixpoint.tla`. `Reach` here is stated as its least fixed point, and ADR 0151's strata are stated over
  the same two modules: its D7 translates a stratum by `Fixpoint.tla` and its D11's reference evaluator states a
  stratum's specification by `fixpoint` ("Amends").
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
- **One comparison.** ADR 0151 D2's comparison, with C0, is built once, in `sce-portable-bytes`, by whichever of the
  two records reaches its step first (Order of work, step 2), and both read it.

### D11. Refusals, and what each construct is on every backend

Each refusal is a `DiagnosticCode` with its row in the appendix of `docs/SCE_ACCEPTED_SUBSET.md`, added in the step
that makes it: `graph/table-not-map`, `graph/endpoint-type-mismatch`, `graph/weight-not-unsigned-integer`,
`graph/expression-may-fail`, `graph/duplicate-graph`, `graph/unknown-graph`, `graph/roots-type-mismatch`,
`graph/result-type-mismatch`, `graph/result-bound-below-nodes`, `graph/result-target`, `graph/result-aliases-input`,
`graph/no-result`, `graph/unweighted` and `graph/label-may-overflow`. A backend that does not serve the statements
refuses them with the existing `generate/unsupported-feature` (D7), so no code is added for it.

| Construct | The build checks | Bounds | Rust | Every other backend and the Interpreter | The model |
|---|---|---|---|---|---|
| `<sce:graph>` | its tables, the types of its ends and weight, their totality, its name (D1) | `B_V` and `B_E`, its tables' | lowered to the adapter's view of the two tables | `generate/unsupported-feature`, naming it | a set of arcs (`Arcs`) once D9 lands; refused by name (`Construct::Action`) until then |
| `<sce:reach>`, `<sce:breadth-first>`, `<sce:shortest-paths>` | its graph, roots and results, their types, bounds and widths (D3, D6) | each result's at least `B_V`; the working storage's term in `B_V` and `B_E` | a call of the core | as above | the assignment of `Reach`, `Label`, `Parent`, `Via`, `First`; refused by name until then |
| `<sce:topological-order>` | as above, and its outcome's enum | as above | a call of the core | as above | `Rank`, `Left`, `Cycle`; refused by name until then |
| `<sce:prune>` | its graph's tables are writable places of the document (D3) | none beyond its tables' | a call of the core, then removals from the two tables | as above | `Prune`; refused by name until then |
| `sce:std/graph/order_outcome.scxml` | as every enum | none | as every enum | as every enum: an enum is served everywhere, and a backend that refuses the statements never reaches it | as every enum |

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
- A machine reserves working storage of a term in `B_V` and `B_E` on a lane with no allocator (D6), and holds none
  between calls.
- A graph statement runs on Rust alone; every other backend and the Interpreter refuse it by name, and a document that
  must run there writes its loop as it does today.
- The specification is written once for each tool that judges it, Verus and TLA+, and the two are held together by the
  cases (C6); the cases' independent model is a third reading of D2, kept apart on purpose (C9).
- A document that holds a graph statement has no model until the map's module and D9 land, and is refused by name
  until then.
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
- **Floating weights, summed exactly.** A label is a sum along a walk and a minimum over walks; an exact floating sum
  needs an accumulator as wide as the exponent range for every vertex (Kulisch and Miranker 1981), and a sum rounded
  at each arc depends on the order of the arcs (Goldberg 1991). Integer weights are exact and enough (Open question
  2).
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
2. The comparison of ADR 0151 D2 and its proof C0, unless ADR 0151 has built them first:
   `backends/rust/portable-bytes/tests/every_key_compares_by_one_total_order.rs` (signed integers across zero, `-0`
   before `+0`, a NaN after the positive infinity, a text's prefix before what extends it, a record decided by its
   first field that differs, an absent value first) and its proof's entry in `docs/formal/proofs.json`, which must
   refuse a record compared by its last field first. Red without it: the comparison does not exist.
3. The specification, the least fixed point and the core, with C1 and C2: `graph/spec.rs`, `fixpoint.rs` and
   `graph/core.rs` declared in `docs/formal/proofs.json`, each refusing the changes that break what it states: the
   count of arcs dropped from a label (a cycle of weight zero then leaves a vertex whose walk of parents never reaches
   a root), a tie kept by the first arc met, Kahn's algorithm taking the first vertex freed rather than the least, and
   an arc kept whose end is no vertex. And `backends/rust/forge-runtime/tests/graph_core.rs`:
   `a_cycle_of_weight_zero_still_gives_a_tree`, `equal_labels_are_broken_by_the_least_number`,
   `the_vertices_a_topological_order_leaves_lie_on_or_after_a_cycle`. Red without it: the module does not exist, and
   the judge refuses a manifest whose files it cannot find.
4. The adapter, `GraphWork` and the cases, with C3, C4 and C8. `tests/forge/conformance/gen_cases.py` writes graph
   cases from recorded seeds (parallel arcs, loops, weights of zero, ends that are no vertex, ties of every kind, keys
   of an integer, a record and a text), each with the answer of an independent model in the same file, which computes
   reachability by iterating to a fixed point and labels by relaxing every arc until none changes, with C2's laws
   checked on its answers before a case is written; `--check` holds the cases to it.
   `backends/rust/forge-conformance/tests/forge_graph.rs`: `every_graph_statement_answers_its_case`;
   `a_graph_answer_has_the_same_bytes_whatever_order_its_tables_were_put_in` (red for an adapter that numbers the
   vertices in put order); `a_graph_call_does_work_within_its_bound` (the work at 10 000 and at 20 000 vertices and
   arcs within the ratio the bound allows; red for a search that scans every arc once per vertex);
   `acl_membership_is_the_reach_of_its_edges_as_a_set`. Red without it: the adapter does not exist, and no case is
   written.
5. The build: the declaration, the statements, their checks and every refusal of D11 with its appendix row, the Rust
   lowering, `GRAPH_BACKENDS`, the elements in `schemas/sce-forge-ext.xsd` and `sce:std/graph/order_outcome.scxml`.
   `sce-build/tests/a_graph_is_declared_over_a_machines_tables.rs` (each refusal by its code, and a document holding
   each statement generated for Rust) and
   `sce-build/tests/a_graph_statement_is_refused_by_name_where_no_backend_lowers_it.rs` (C11, C++, Go, Kotlin, Python,
   Swift and the Interpreter's lowering each refusing with `generate/unsupported-feature`, naming the element). A
   mutation casefile, `sce-build/tests/mutations/a_graph_result_holds_the_bound_of_its_nodes.cases`, drops the check
   of `graph/result-bound-below-nodes` and must turn that refusal's case red. Red without it: `<sce:graph>` is an
   unknown element.
6. The machine: `backends/rust/scenarios/tests/graph.rs` over a fixture that keeps a router's table:
   `a_machine_prunes_what_its_roots_no_longer_reach`, `equal_cost_paths_are_chosen_by_the_least_key`,
   `a_graph_statement_reads_the_puts_before_it_in_its_macrostep` and
   `a_restored_machine_answers_as_it_did_before_it_was_saved`, run native, on `wasm32-unknown-unknown`, and in a
   `no_std` build with no allocator as `scripts/check_noalloc_codec.sh` builds the `sce:std/text` documents. Red
   without it: the fixture is refused where it is generated.
7. The model (D9): `docs/formal/Fixpoint.tla` and `docs/formal/Graph.tla`, the translation of each statement, and
   `docs/formal/Graph_cases.tla`, judged by `sce-codegen formal models` (ADR 0146); step 6's fixture entered in
   `docs/formal/models.json` with a property that holds (the route kept to each router is the least-key shortest path)
   and a part taken out (ties broken by the greatest key) that TLC must find violating; ADR 0151 D7's and D11's strata
   stated over `Fixpoint`. It needs the map's module (ADR 0156 D2). Red without it: `sce-codegen model` refuses the
   fixture by `Construct::Action`.
8. The documents named in "Amends".

The proof and the cases (steps 3 and 4) come before the statements (step 5), so that a statement is lowered to a core
already proved, as ADR 0151 puts its theorems before its operators.

## Open questions

Each is decided, with its reason.

1. **Bellman-Ford is not in this record.** Over weights that are never negative it computes the answer Dijkstra's
   algorithm does, at `O(V E)`; a weight of an unsigned type cannot be negative, so Dijkstra's precondition is a fact
   of the type and not of a run. Negative weights serve difference constraints (CLRS 24.4), which no consumer has
   asked for. If one does, a statement of its own adds Bellman-Ford with an outcome `negative-cycle` and the cycle it
   found; a statement's name keeps its meaning once published.
2. **A weight is an unsigned integer, never a floating number.** The naturals under `(min, +)` are a semiring and
   every sum is exact, so a label is one number on every lane. A floating sum along a walk depends on the order of its
   additions (Goldberg 1991), and keeping it exact would cost a long accumulator per vertex (Alternatives). An
   application whose costs are real scales them to fixed point, and folds into the integer its own way of choosing
   between equal paths, such as the term by which two implementations of one protocol choose the same hop: that choice
   is the application's, and stays in it.
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
   4's test holds the two equal as sets, so the two cannot come to mean different things.
7. **The statements stand in an algorithm's body as well as a statechart's.** A schema compiler's closure of imports
   is a function of tables an algorithm is lent; its results are locals (D3), since a lent slot may hold fewer entries
   than its bound.
8. **`first` and `via` are results.** A router forwards by the next hop and the link, and computing them from the
   parents in a document is a loop per vertex up its walk; the core computes them in `O(V)` in the order it settles
   the vertices.
9. **The topological order is a statement, though the request named four.** The census's schema rows check for a cycle
   among imports and types, and build and CI systems order targets by it (D12); with its witness, a cycle is named and
   not only detected.
10. **Several roots are one search.** A search from a set of roots is the search from a vertex joined to each, so a
    router's tree per root is one statement per root, and a set of roots is one statement.

## Sources

Cormen, Leiserson, Rivest and Stein 2009 (*Introduction to Algorithms*, 3rd ed., MIT Press: Appendix B.4; sections
6.5, 22.1, 22.2, 22.4, 24.1, 24.3, 24.4 and 24.5); Dijkstra 1959 ("A note on two problems in connexion with graphs",
*Numerische Mathematik* 1); Bellman 1958 ("On a routing problem", *Quarterly of Applied Mathematics*); Ford 1956
(*Network Flow Theory*, RAND paper P-923); Kahn 1962 ("Topological sorting of large networks", *Communications of the
ACM*); Knuth (*The Art of Computer Programming*, volume 1, section 2.2.3); Bang-Jensen and Gutin 2009 (*Digraphs:
Theory, Algorithms and Applications*, 2nd ed., Springer); Codd 1970 ("A relational model of data for large shared data
banks", *Communications of the ACM*); Tarski 1955 ("A lattice-theoretical fixpoint theorem and its applications",
*Pacific Journal of Mathematics*); Abiteboul, Hull and Vianu 1995 (*Foundations of Databases*, Addison-Wesley,
chapters 12 and 13); Carré 1971 ("An algebra for network routing problems", *Journal of the Institute of Mathematics
and its Applications*); Backhouse and Carré 1975 ("Regular algebra applied to path-finding problems", *Journal of the
Institute of Mathematics and its Applications*); Mohri 2002 ("Semiring frameworks and algorithms for shortest-distance
problems", *Journal of Automata, Languages and Combinatorics*); Abo Khamis, Ngo, Pichler, Suciu and Wang 2022
("Convergence of Datalog over (pre-)semirings", PODS); Ross and Sagiv 1992 ("Monotonic aggregation in deductive
databases", PODS); Gupta, Mumick and Subrahmanian 1993 ("Maintaining views incrementally", SIGMOD); Even and Shiloach
1981 ("An on-line edge-deletion problem", *Journal of the ACM*); Ramalingam and Reps 1996 ("An incremental algorithm
for a generalization of the shortest-path problem", *Journal of Algorithms*); McCarthy 1960 ("Recursive functions of
symbolic expressions and their computation by machine, part I", *Communications of the ACM*); Jones, Hosking and Moss
2011 (*The Garbage Collection Handbook*, CRC Press); Hoare 1969 ("An axiomatic basis for computer programming",
*Communications of the ACM*); Hoare 1971 ("Procedures and parameters: an axiomatic approach", in *Symposium on
Semantics of Algorithmic Languages*, Lecture Notes in Mathematics 188, Springer); Meyer 1992 ("Applying design by
contract", *IEEE Computer*); Lattuada et al. 2024 (Verus, SOSP); Nordhoff and Lammich 2012 ("Dijkstra's shortest path
algorithm", *Archive of Formal Proofs*); Goldberg 1991 ("What every computer scientist should know about
floating-point arithmetic", *ACM Computing Surveys*); Kulisch and Miranker 1981 (*Computer Arithmetic in Theory and
Practice*, Academic Press).

They are cited from the texts as known, and none was read again for this record. The passages D2, D5 and D10 rest on
are to be checked against the texts in the step that builds each, before the step lands: CLRS's proof of Dijkstra's
algorithm and its path-relaxation and predecessor-subgraph properties (24.3, 24.5), on which the claim that a
lexicographic label keeps the proof rests; Kahn's statement of what his algorithm leaves when the graph has a cycle;
the least fixed point of a monotone operator and its iteration (Tarski; Abiteboul, Hull and Vianu, chapter 12); Ross
and Sagiv's account of a minimum through recursion; and the venue and year of Hoare's paper on procedures, cited here
as known. The census of the downstream consumer was read for this record, its catalogue of gaps and the nine rows it
files under graphs; it is held outside this repository and is not cited by place. The tree was read at `662e4d060cc`
for every file this record names.
