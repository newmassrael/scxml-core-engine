# ADR 0164 — A recursive type is declared in one document, a value of it is a tree of at most its bound of nodes held in an arena by index, and a function over it recurses on disjoint parts of its argument

- Status: Accepted (2026-11-29) under the owner's standing rule, by the main session the owner delegated the decision
  to, after an independent review; amended on acceptance as listed in "Amended on acceptance". The downstream consumer
  that asked for it reviewed the draft as its user, and the points of its review taken are listed there too. CBMC's
  use for the C11 arena is not approved by this acceptance: it waits for the owner's own words (D12).
- Landed: step 1 of "Order of work" (this record, accepted, and the application of D1 in `SCE_FORGE.md` §10). No other
  step is built, and no proof, model or test of it has been run.
- Amended by: ADR 0156 D17 (2026-11-29): the typed values of `sce-static` are translated by ADR 0156 D17 (its order
  steps 2a to 2c), the step 5b waits for; 5b comes after 2c and lifts `Construct::Inductive`, which D17's refusal table
  names. `Inductive.tla` is read by both checkers as D17's modules are: its folds over `Apalache.tla`, its types
  annotated, and its `RECURSIVE` reference kept to a configuration TLC alone runs.
- Date: 2026-11-28 (proposed); 2026-11-29 (accepted).
- Written under the owner's standing rule (the most long-term correct structure, textbook in its foundations, cost
  ignored) and the owner's scope rule of 2026-11-22: a new facility is built only on the backends a consumer needs,
  Rust by default; its contract is general; what is special to one application stays in the application. At the
  request of a downstream consumer whose protocol logic, moving into SCE documents, holds recursive data in 21 places:
  a configuration document's value tree, a trie of key paths with wildcard chunks, a protocol schema's syntax tree,
  the tree of a message's dissected fields, an allocation policy built of nested combinators, and a hash trie. The
  request is the first reference case and not the scope.
- Amends (each amended document gains an "Amended by: ADR 0164" line: the records that carry such lines gained it on
  acceptance, ADR 0044, ADR 0086, ADR 0092, ADR 0108 and ADR 0115; every other document gains it in the landing that
  moves it, held by step 10):
  - ADR 0044 decisions 1 and 3 and ADR 0112 D5 (a member's lineage and its saved shape are keyed by
    `<document name>/<member>`, since a member declares no event, D1);
  - ADR 0065 section 2 (a codec's encode refuses a value of more nodes than its bound, which `Refuses_C` reads, D11);
  - ADR 0086 section 1.1 (the grammar: an alias that names a member of an inductive document takes one bracket, its
    node bound and an optional depth bound, D3), section 1.5 (a type that contains itself within one inductive
    document is admitted with that bound; through the imports of several documents it stays refused, D1), section 2.1
    (the size row of an inductive type, D4), section 4 (a reader counts nodes, and refuses the one past the bound, D4)
    and section 7 (the types each backend generates: an arena, D5);
  - ADR 0087 section 1.1 (the JSON Schema of an inductive type recurses by `$ref` and states its node bound beside the
    `$ref` at the use; a position that requires its type's GBNF grammar refuses an inductive type, D4), section 2.1
    item 4 (`depth(R)` of an inductive type, D4) and section 2.3 (a refusal code, `tooManyNodes`, numbered 16, D4);
  - ADR 0092 section 1.1 item 3 (a union that holds itself through its cases' records is admitted when the union and
    the records are members of one inductive document, D1) and section 1.1 item 1 (a case of a union member may hold a
    record member of its document, D9);
  - ADR 0101 section 1.1 (a CBOR entry may be of an inductive type), section 3.2 (a later revision adds no field that
    holds a member of its own document, so the skip of an undeclared key stays bounded, D1), section 3.3 (a value of
    more nodes than its bound is `CborOutOfRange`) and section 4.1 (its size row), D4;
  - ADR 0108 D1 (an inductive type is no key, D9; a bounded list whose element is a key is a key, D10), D4 (a put that
    replaces a present key may answer `full` when the value's nodes pass their bound, and a document checks
    `nodes(...)` beside `len(...)`, D6) and D7 (an inductive member that holds a map stands in no wire position, as a
    record that holds one does not, D9);
  - ADR 0115 D7 (the model holds an inductive value in its canonical preorder form and computes over it by folds, D11)
    and D10 (an algorithm's signature may hold `<sce:measure>`, an element apart from a loop's `<sce:decreases>`, D8);
  - ADR 0133 D1 (a whole store from a bound at most the place's is total; a remove and a clear inside a value are
    total; a store into a smaller bound is checked, D6);
  - ADR 0156 D2 (`docs/formal/Inductive.tla` joins the one spec-bearing surface of the semantics, D11) and D4 (the
    generated `TypeOK` of an inductive variable is `Canonical`, D11);
  - `SCE_WIRE_CONTRACTS.md` (the wire, saved and codec forms of an inductive value, its JSON Schema, and
    `Inductive.tla` in the semantics surface, as ADR 0086 section 13, ADR 0092 section 11 and ADR 0101 section 7
    registered theirs);
  - `schemas/sce-forge-ext.xsd` (`kindType` gains `inductive`; `<sce:import>`, whose attributes are `src`, `kind`,
    `as` and `requires`, gains `member`; the elements `<sce:record>`, `<sce:union>`, `<sce:function>` and
    `<sce:measure>`; an `outcome` on an `<sce:assign>` into an `inout` value, D7),
    `schemas/sce-saved-state.v1.schema.json` (an inductive variable's saved shape) and
    `sce-build/src/forge/kind_catalog.rs` (the kind);
  - `stdlib/lift/refusal_code.scxml` (`tooManyNodes` = 16, `sce:since` its enum's next revision) and
    `stdlib/collection/put_outcome.scxml` (its comment: a replace may answer `full` when a value's nodes pass their
    bound);
  - ADR 0151 D2 (the comparison of an inductive value, D4);
  - `SCE_FORGE.md` §3.2 (the kind list), §4 (a section for the kind), §4.12 (an algorithm's local functions, its
    structural recursion and its views of an inductive value, D7 and D8), §8 (the kind summary) and §10 (the
    application of the admission test, D1);
  - `docs/SCE_ACCEPTED_SUBSET.md` §2.15 (a variable of an inductive type, D6) and the appendix (D13's codes);
  - `docs/spec/synth/rfc-sce-protocol-synthesis.md` §synth-5-H ("Recursive and tree data kinds", deferred there),
    which this record decides, and §synth-5-A (a descent is a `<sce:while>` that `algorithm/while-unbounded` does not
    refuse, D7).
- Scope: `sce-build` (the kind, the type, its checks, sizes and depth, the check of a recursion, the model's
  translation); the Rust lane, native, `no_std` and `wasm32-unknown-unknown` (`sce-portable-bytes`, the machine and
  algorithm lowerings, the streaming lift beside `backends/rust/runtime/src/json.rs` and `event_payload.rs`); the C11
  lane (`sce/forge/arena.h`, its lowerings, and the stack of containers that replaces the depth bitmask of
  `sce/forge/wire.h`). Every other backend, the Interpreter and the TTCN-3 export refuse an inductive type by name
  (D13).
- Related: ADR 0050 and ADR 0086 (a bound by name, part of a type), ADR 0055 (optional types, and narrowing), ADR 0077
  (a decode refuses before it stores), ADR 0082 (an algorithm writes into storage its caller holds), ADR 0084 (the
  door, where state shared between execution contexts is kept), ADR 0087 (a value of a schema asked of a model), ADR
  0092 (unions), ADR 0101 (one deterministic CBOR form per type), ADR 0108 (maps), ADR 0115 (the levels of formal
  judgement), ADR 0149 (an automaton over a text), ADR 0151 (views and their total comparison), ADR 0154 (a map's
  storage lent by its host), ADR 0156 (models and proof obligations generated from documents). Proposed records it
  meets: ADR 0118 (cost from the plan), whose vocabulary D13 adopts on its acceptance and does not amend before; ADR
  0163 (a text read by one grammar), which refuses a recursive rule (`grammar/recursive-rule`) and a recursive JSON
  Schema import (`import/json-schema-recursive`) because no type contains itself, and expects a depth bound, so a
  grammar codec or a JSON Schema import whose root type is inductive waits for an amendment of ADR 0163; ADR 0165 (a
  graph as a relation), which leaves recursive data apart; ADR 0166 (documents generic in their types), whose
  parameters an inductive document may take with regular recursion only, every member named inside its document at the
  parameters it is declared with, and polymorphic recursion refused (Mycroft, "Polymorphic type schemes and recursive
  definitions", International Symposium on Programming, LNCS 167, 1984).

## Context

What exists today, read from the tree at `662e4d060cc`:

- **The type grammar is a finite tree of constructors.** `sce-build/src/forge/value_type.rs` reads one grammar for
  every position that declares a value (ADR 0086 section 1, with ADR 0092's `union:<alias>` and ADR 0108's
  `map<K, T>[B]` and `set<K>[B]`), as `TypeOf`, whose `Record { alias }` and `Union { alias }` name another document.
  Its module comment states the two shapes that stay refused: `optional<optional<T>>`, and "a type that contains
  itself, which has no largest value". Every fold over a type (the sizes `J` and `C`, the depth, the JSON Schema and
  the GBNF grammar) is a structural recursion over the type that stops at a record met again on its path
  (`SizeError::ContainsItself`, `GrammarError::ContainsItself`).
- **A record names another only by importing it, so a recursive type is an import cycle.** An event schema names a
  record by `<sce:import kind="event-schema">` (ADR 0086 section 1.1 item 8), and a union names its cases' records the
  same way (ADR 0092 section 1.1, `UnionModel` in `sce-build/src/forge/model.rs`, whose `UnionCase::record` is an
  import alias). A record that holds itself, directly or through a union's case, is refused where the imports are
  walked (`validation/cross-kind-circular-dependency`, ADR 0086 section 1.5 and ADR 0092 section 9) or where a
  directory is sorted (`manifest/circular-dependency`, `sce-build/src/forge/manifest.rs`).
- **An algorithm does not recurse.** `AlgorithmStmt::Call` (`sce-build/src/forge/model.rs`) calls an algorithm the
  document imports; its comment says "v1 forbids recursion", and the Interpreter's lowering refuses an algorithm that
  imports itself however far round (`sce-build/src/forge/static_js.rs`, `algorithm_functions`). The loops are
  `<sce:foreach>` over a bounded collection and `<sce:while>` (SCE_FORGE.md §4.12; its `max-iter` is §synth-5-A's).
- **The pattern of an arena exists, for a map.** A map keeps its entries in `B` slots with a free list, names a slot
  by its position one past its place so that 0 is no slot, and lets no handle, slot or position leave the value (ADR
  0108 D2 and D8; Rust `backends/rust/portable-bytes/src/map.rs`, its storage a type parameter `Inline`, `Heap` or
  `Linear`; C11 `backends/c/forge-runtime/include/sce/forge/map.h`). A key is a type of exact equality; a list is not
  one (`NotAKeyWhy::Collection`: "a list, a map or a set is no key").
- **The model holds integers and booleans.** `sce-codegen model` (`sce-build/src/formal/document_model.rs`) writes a
  document's module over Appendix D for the integers and the booleans a document's `<data>` start as, and refuses a
  `sce-static` document as a whole by name (`Construct::Datamodel`). ADR 0156 D3 orders the breadth that is missing; a
  typed record, list, map or union has no translation yet.
- **The deferred record.** `docs/spec/synth/rfc-sce-protocol-synthesis.md` §synth-5-H names the need ("KeyExpr tries
  are n-ary trees") with a sketch (`type="self"` or `type="ref<OtherKind>"` on a codec's field) and defers it;
  §synth-5-F offers, until then, a flat trie built at deployment time and placed in read-only memory, which serves a
  set of keys fixed by the deployment and no set that changes at run time.

So a tree can be written today only by hand: as a map from a number the author draws to a node that names its children
by such numbers. Nothing then tells the build that the value is a tree: that each node but the root has one parent,
that a walk ends, how many nodes a walk visits, or what the value's bytes are as a value rather than as the author's
numbering. Two such maps that hold one tree under two numberings are two values on the wire and in a comparison.

## The textbook structure

- **A recursive data type is the least fixed point of the equation its declaration states.** A declaration whose right
  side combines products, sums and the type's own name denotes the initial algebra of the functor the right side is,
  whose elements are the finite terms built by its constructors (Goguen, Thatcher, Wagner and Wright, "Initial algebra
  semantics and continuous algebras", Journal of the ACM 1977; Hoare, "Recursive data structures", International
  Journal of Computer and Information Sciences 1975, which gives such types to a programming language as values with
  no pointer visible). Mutually recursive types are one fixed point of a tuple of equations, declared together: an ML
  `datatype ... and ...`, a Coq `Inductive ... with ...` block (Pierce, *Types and Programming Languages*, MIT Press
  2002, chapter 20; Harper, *Practical Foundations for Programming Languages*, 2nd edition, Cambridge University Press
  2016, chapter 15). Mutual recursion of types is confined to one block, or to one module, in each of them: Haskell
  needs a boot file to let two modules' types name each other.
- **A definition is admissible when the type occurs only strictly positively** in its constructors' arguments, never
  to the left of a function arrow (Coquand and Paulin, "Inductively defined types", COLOG-88, LNCS 417, 1990).
  Products, sums, finite sequences, optional values and finite maps over a fixed key type are strictly positive
  functors. A type is inhabited by a finite term exactly when some constructor's arguments are all inhabited, which is
  a least fixed point computed over the declarations; the fewest nodes of a finite term rooted at each type is
  computed by the same iteration over a grammar, Knuth's generalization of Dijkstra's algorithm ("A generalization of
  Dijkstra's algorithm", Information Processing Letters 6(1), 1977).
- **Recursion over such a type terminates when every recursive call is on a strict subterm of the argument**
  (structural recursion: Giménez, "Codifying guarded definitions with recursive schemes", TYPES 1994, LNCS 996, 1995,
  the guard condition of Coq; Abel and Altenkirch, "A predicative analysis of structural recursion", Journal of
  Functional Programming 2002, the analysis behind Agda's checker, which descends from the foetus checker and now
  judges size-change matrices). It is the special case of well-founded recursion whose measure is the size of the
  argument (Nordström, "Terminating general recursion", BIT 1988), the measure Verus asks of a recursive specification
  function by its `decreases` clause (Lattuada et al., "Verus: Verifying Rust Programs using Linear Ghost Types",
  OOPSLA 2023). A recursion that visits each node once is a fold, a catamorphism (Meijer, Fokkinga and Paterson,
  "Functional programming with bananas, lenses, envelopes and barbed wire", FPCA 1991), whose cost is the number of
  nodes times the cost of one step. A recursion that also reads the sub-terms it does not call on is a paramorphism
  (Meertens, "Paramorphisms", Formal Aspects of Computing 4(5), 1992); one that carries values from the root down is a
  downward accumulation (Gibbons, "Upwards and downwards accumulations on trees", Mathematics of Program Construction
  1992, LNCS 669, 1993); mutually recursive functions over mutually recursive types are a mutumorphism (Fokkinga,
  "Tupling and mutumorphisms", The Squiggolist 1(4), 1990).
- **A recursion is run without a call stack by making its continuation data**: each frame a record of what remains to
  do, held on an explicit stack (a transformation to continuation-passing style and then defunctionalization, which
  makes a first-order recursion an iteration over a stack of frames: Danvy and Nielsen, "Defunctionalization at work",
  PPDP 2001, after Reynolds, "Definitional interpreters for higher-order programming languages", ACM Annual Conference
  1972). The stack is then storage of a size the program states, not the platform's.
- **Objects linked by pointers are represented without pointers by indices into arrays**, with a free list of the
  unused slots (Cormen, Leiserson, Rivest and Stein, *Introduction to Algorithms*, 3rd edition, MIT Press 2009,
  section 10.3, "Implementing pointers and objects"; Knuth, *The Art of Computer Programming*, volume 1, 3rd edition,
  Addison-Wesley 1997, section 2.2.3, the list of available space). A tree of any fan-out is held in two links per
  node, its first child and its next sibling (Knuth, section 2.3.2, the binary tree representation of a forest; CLRS
  section 10.4, the left-child, right-sibling representation), so a node reserves nothing for children it does not
  have. Where every object of one bound may be of any of several kinds, one pool of slots of the largest kind holds
  every value a pool per kind holds, in less storage, since each kind's pool would have to hold the whole bound; pools
  per size class pay where each class has a bound of its own (Wilson, Johnstone, Neely and Boles, "Dynamic storage
  allocation: a survey and critical review", IWMM 1995). Allocating the objects of one value in one region freed as a
  whole is arena or region allocation (Hanson, "Fast allocation and deallocation of memory based on object lifetimes",
  Software: Practice and Experience 1990; Tofte and Talpin, "Region-based memory management", Information and
  Computation 1997). A tree whose every node has one owner needs no collector: freeing a subtree frees exactly what no
  other node reaches, which is the discipline of linear and unique ownership (Wadler, "Linear types can change the
  world!", IFIP TC 2 Working Conference on Programming Concepts and Methods, 1990).
- **A representation is proved correct by an abstraction function** from the concrete state to the abstract value and
  an invariant on the concrete state, each operation shown to keep the invariant and to commute with the function
  (Hoare, "Proof of correctness of data representations", Acta Informatica 1972). Two concrete states of one abstract
  value are then indistinguishable to every operation, so the bytes of a value are the abstract value's.
- **A tree has a canonical sequential form.** Its nodes listed in preorder, each with its children in their order,
  determine the tree, and the preorder numbering is a function of the tree (Knuth, volume 1, sections 2.3.1 and 2.3.3,
  the sequential representation). In that numbering every node's descendants follow it, so a fold is a single pass
  from the last node to the first.
- **The largest value of a bounded tree type is an optimization with optimal substructure**: the largest encoding of a
  term of at most `k` nodes rooted at a constructor is the constructor's own bytes and the best division of the
  remaining `k - 1` nodes among its children, which dynamic programming computes exactly (Bellman, *Dynamic
  Programming*, Princeton University Press 1957).
- **A recursive type has a finite description in the encodings SCE uses**: CDDL rules may be recursive (RFC 8610
  Appendix C), CBOR nests arrays and maps to any depth (RFC 8949 section 3), a JSON Schema refers to itself by `$ref`
  (JSON Schema 2020-12, Core section 8.2.3.1), and ASN.1 admits a type that refers to itself through `SEQUENCE OF` and
  `CHOICE` (ITU-T X.680).

## Decision

**A recursive type is a member of an inductive group: one document of the new kind `inductive`, which declares records
and unions that may name each other and the types it imports. A use of a member outside its document states its node
bound, the most nodes a value holds, and may state a depth bound, and a value is a finite tree within both. Its bytes
and its equality are the tree's, and every lane holds it in one arena of that many slots, a child named by its slot's
index and a sequence of children threaded through the slots, with no pointer. A function over it is a local function
of one algorithm document and recurses only on disjoint proper parts of one argument, so it ends, visits each node at
most once for each cycle of functions, and runs on an explicit stack the build sizes. The model holds the tree in its
canonical preorder form and computes over it by folds. Rust and C11 serve it; every other lane refuses it by name.**

### D1. A recursive type is declared with the types it recurses through, in one document of the kind `inductive`

`sce:kind="inductive"` is a document of type declarations that form one recursive scope: `<sce:record name>` elements,
each with a `<datamodel>` of fields written as an event schema's are (ADR 0086 section 1), and `<sce:union name>`
elements, each with a tag and cases written as a union document's are (ADR 0092 section 1.1). Inside the document a
member is named `record:<name>` or `union:<name>`, with no import; anything else it names (an enum, an event schema, a
union, another inductive document's member) it imports, and the import graph stays acyclic. It is not inline-eligible
(`SCE_FORGE.md` §3.2): it is declared as a document and imported, one member at a time,
`<sce:import kind="inductive" src="..." member="<member>" as="<alias>"/>`.

```xml
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="inductive" name="config_value">
  <sce:import kind="enum" src="config_kind.scxml" as="ConfigKind"/>
  <sce:import kind="event-schema" src="config_flag.scxml" as="Flag"/>
  <sce:import kind="event-schema" src="config_number.scxml" as="Number"/>
  <sce:import kind="event-schema" src="config_text.scxml" as="Text"/>
  <sce:union name="Value">
    <datamodel>
      <data id="kind" sce:type="enum:ConfigKind">
        <sce:case variant="none"/>
        <sce:case variant="flag" record="Flag"/>
        <sce:case variant="number" record="Number"/>
        <sce:case variant="text" record="Text"/>
        <sce:case variant="array" record="Array"/>
        <sce:case variant="object" record="Object"/>
      </data>
    </datamodel>
  </sce:union>
  <sce:record name="Array">
    <datamodel>
      <data id="items" sce:type="list&lt;union:Value&gt;[config_fanout=64]"/>
    </datamodel>
  </sce:record>
  <sce:record name="Object">
    <datamodel>
      <data id="members" sce:type="list&lt;record:Member&gt;[config_fanout=64]"/>
    </datamodel>
  </sce:record>
  <sce:record name="Member">
    <datamodel>
      <data id="key" sce:type="string[config_key=64]"/>
      <data id="value" sce:type="union:Value"/>
    </datamodel>
  </sce:record>
</scxml>
```

A machine that holds a configuration imports `Value` and states its bound where it uses it:

```xml
<sce:import kind="inductive" src="config_value.scxml" member="Value" as="Config"/>
<data id="config" sce:type="union:Config[config_nodes=512]"/>
```

A trie of key paths, each chunk a text, with a child for a wildcard chunk beside the children by literal chunk:

```xml
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="inductive" name="key_trie">
  <sce:import kind="event-schema" src="route.scxml" as="Route"/>
  <sce:record name="Node">
    <datamodel>
      <data id="route" sce:type="optional&lt;record:Route&gt;"/>
      <data id="literal" sce:type="map&lt;string[chunk=64], record:Node&gt;[fanout=32]"/>
      <data id="wild" sce:type="optional&lt;record:Node&gt;"/>
    </datamodel>
  </sce:record>
</scxml>
```

- **Why one document, and why a kind.** A mutually recursive definition is one fixed point and is declared as one
  block, as ML's `datatype ... and ...` and Coq's `Inductive ... with ...` are, and Haskell lets two modules' types
  name each other only through a boot file; so are its revisions, since a change to one member changes the type of
  every member. A recursive group is also one generated unit with one node type (D5), which a group spread over
  several documents would have no document to own. Admitting cycles of imports between event-schema and union
  documents instead would make the import graph's topological order, which the manifest and the generator read
  (`manifest.rs`), a partial one, would let one member's revision move apart from the others', and would leave the
  group with no lineage, since its members declare no event (below). The document is therefore a kind of its own, and
  not a flag on event-schema or union documents.
- **A member's grammar is its kind's.** A record member's fields are an event schema's fields, any type of the
  grammar; a union member's tag is a closed enum the document imports and its cases are one to one with the enum's
  variants, each holding a record (a record member of this document or an imported event schema) or nothing, with
  every rule of ADR 0092 section 1.1 (`validation/union-cases`, `validation/union-tag-collision`). Nothing in a member
  is written in a way a record or a union is not.
- **Revisions and lineage.** A member's lineage, the key its revisions and its saved shape are named by, is
  `<document name>/<member>`, where an event schema's is its event's name (ADR 0044 decisions 1 and 3, ADR 0112 D5),
  since a member declares no event. The document declares `sce:revision`; a record member's fields carry `sce:since`
  up to it and grow as an event schema's fields do (ADR 0086 section 10); a union member's revision stays its tag
  enum's (ADR 0092 section 8.1); and the document's revision is at least each of its members'. A later revision does
  not add to a member a field whose type holds a member of the same document: an older reader steps over a key it does
  not declare by a skip bounded in depth (16 levels in a CBOR codec, ADR 0101 section 3.2; `depth(R)` in a lift, ADR
  0087 section 2.1 item 4), and a sub-tree of up to `B` nodes passes either, so such a field would make an older
  reader refuse what a newer one writes, against ADR 0044 decision 2; a recursion that grows is a new document. Two
  builds' members are revisions of one when their lineages are equal and each field and case is a revision of its
  namesake; anything else parts them, and `schema_revision::divergence` names the member, as
  `mesh/event-schema-mismatch` names a field.

**Admission** (`SCE_FORGE.md` §10), added there as an application on this record's acceptance:

| Axis | Pass/Fail | Reason |
|---|---|---|
| Domain alignment | PASS | Deterministic data layout: a value is a finite term of bounded size whose bytes are one deterministic CBOR form (ADR 0101), equal on every lane whatever slots hold it, and its storage is an arena of slots the build sizes from a named bound |
| Futamura projection compatibility | PASS | The members, their fields, the bounds, the slot layout, the size of the largest value and the recursion of every function over the type are build-time; only the shape and contents of a value are runtime. A function over a tree is compiled to one loop over an explicit stack; nothing is interpreted at run time |
| Cross-domain reuse | PASS | Network protocols (a trie of key paths matched against patterns with wildcards; the syntax tree of a schema language; the dissected fields of a message), GUI and scene state (a widget tree, a scene graph without sharing), robotics control (a behaviour tree), embedded telemetry and configuration (a configuration document's value tree), build and CI systems (the expression tree of a build rule's condition), game networking (a quadtree of an area of interest) |

Passed 3-of-3. Four alternatives were ruled out before admission: a self reference on a codec's field (§synth-5-H's
sketch), which serves only self-recursion and only inside a codec; cycles of imports between event-schema and union
documents (above); a flag on `bounded-collection`, whose slots are addressed by handles a document holds, where a
member's nodes are reached only through their parent; and a kind per shape (a `tree` kind, a `trie` kind), each a
single shape of the one construct this record admits.

### D2. A member denotes the least fixed point, and the build checks that it has finite values

The members of a document state a system of equations: each record member is the product of its fields' types, each
union member the sum of its cases, and a member's name in a field stands for its type. Its meaning is the least
solution, the initial algebra: a value of a member is a **finite term**, a tree whose nodes are member values. Every
constructor of the grammar is a strictly positive functor (a product, a sum, a bounded list, an optional, a bounded
map or set over a key type that holds no member), and the grammar has no function type, so every document is
admissible in Coquand and Paulin's sense by construction, and no check of positivity is needed or made.

- **A node** is a value of a member that stands where a member's type is written: the root, a field, a list's element,
  an optional's value, a map's value. A union member's case holds its record in its own node, as ADR 0092 section 7's
  sum holds its case's record, so a value of `Value` in the case `array` is one node, and each element of its `items`
  one more. Every other type a member holds (a number, a text, a list of numbers, an imported record) is part of its
  node.
- **Every member is on a cycle of the document.** A member that names no member through which it is named again is no
  recursive type, and is an event schema or a union of its own: `inductive/member-not-recursive`, naming the member.
  One fact has one spelling.
- **Every member has a finite value.** The build computes the least fixed point of "a record member is inhabited when
  every field is (a list whose lower count is 0, an optional and a map are always inhabited), a union member when one
  case is". A member it leaves out (`Node` whose one field is `record:Node`) has no finite term, and is refused as
  `inductive/uninhabited`, naming the member and the field that cannot be filled. The same fixed point gives each
  member's **least value**, the fewest nodes a term rooted at it holds, and its **least depth**, the fewest levels; it
  is Knuth's generalization of Dijkstra's algorithm to a grammar, each member a nonterminal (Knuth 1977).
- **Names.** A member's name is unique in its document and differs from every alias the document imports:
  `inductive/member-name-taken`, naming both declarations.

### D3. A use of a member states its node bound, and every other constructor composes with it

The grammar of ADR 0086 section 1.1 gains no constructor. An alias that names a member of an inductive document takes
exactly one bracket, its **node bound**, and within it, after a comma, an optional **depth bound**:

```text
T ::= ... | record:<alias>[B] | union:<alias>[B]
        | record:<alias>[B, depth D] | union:<alias>[B, depth D]
-- <alias> names a member of an inductive document
```

`B` is written as every bound is, a number or `name=N` (ADR 0050, ADR 0086 section 1.2), moved by
`sce-codegen --limit name=N` and read as `_x.limits.<name>`. A value of `union:Config[config_nodes=512]` is a term of
`Value` of at most 512 nodes. The bracket is required where a member is used outside its document
(`validation/attribute-rule-violated` without it, ADR 0086 section 1.4's refusal of a type with no bound), and written
nowhere inside it, since a member named inside its document is a part of the same value, counted against the bound of
the value it stands in (`validation/attribute-rule-violated` on such a bracket, ADR 0086 section 11's refusal of a
bracket on a type that takes none). A bound below the least value of its member leaves the type with no value, and is
refused as `inductive/bound-below-least-value`, naming the use, the bound and the least value, as a list's lower count
past its bound is (ADR 0086 section 15.2). It is judged again after a build moves the bound by name.

The node bound is the one every value has, because a value's storage and the cost of every walk of it are functions of
its nodes, and a depth bound with a fan-out of `F` admits `F^depth` nodes. The fan-out of one node is its own lists'
and maps' bounds (`config_fanout` above), each a bound of its own type, and a node is held to both.

The depth bound `D`, written as every bound is (`union:Config[config_nodes=512, depth config_depth=16]`), adds what
the node bound cannot say: that no path from the root holds more than `D` nodes. A value of the type is then a term of
at most `B` nodes and at most `D` levels; a reader refuses a term past `D` as `tooDeep` (D4), a write that would leave
one raises `error.execution` as a write past `B` does (D6), and every recursion over it needs at most `D + 1` frames
for each cycle where `B + 1` would be needed without it (D8). It is what a stack on a microcontroller is sized by,
where `B` frames would be too many and the trees are shallow. A depth bound below the least value's depth, or above
`B`, is refused as `inductive/bound-below-least-value` and `validation/attribute-rule-violated` respectively, judged
again after a build moves either by name.

Every constructor takes a member type and a member takes every type, as ADR 0086's orthogonality asks:

| Where | Admitted | What it means |
|---|---|---|
| a member's field, list element, optional, map value | a member of the same document, no bracket | a child of the node, counted against the value's bound |
| a member's field | a member of another inductive document, with its bracket | a value of its own bound held inside the node, as an imported record is |
| a member's field | any type of the grammar holding no member | part of the node |
| anywhere a type stands outside the document | `record:<alias>[B]`, `union:<alias>[B]` | a value of at most `B` nodes: a variable, a field of an event schema, a list's element, an optional's value, a map's value, an algorithm's parameter, local and return, a CBOR codec's entry (D4) |
| a map's or a set's key | no member type | D9 |

### D4. A value is its term: its bytes, its equality, its order and its sizes

- **One form per value.** A value is written as its term, nested as its type is: a record member as a record is, a
  union member as a union is, each child at its place (ADR 0086 section 3.1 for the JSON wire and the saved state, ADR
  0092 section 2 for a union's tag beside its fields; ADR 0101 section 2 for the deterministic CBOR of a codec and of
  a signed value, a union the map of one entry keyed by its case). No form writes an index or a slot. Two lanes
  holding one term in two arrangements of their slots write one string of bytes, because the bytes are a function of
  the term (D12, T1b).
- **Equality and order.** Two values are equal when their terms are, and a term holds its maps of children in ADR 0108
  D2's order, the order their keys were put: that order is part of the term, and so of its saved form, its snapshot,
  its canonical form in the model (D11) and its walks. Where ADR 0151 D2's total comparison is read, a value compares
  as the record or the union its root node is, each child compared by this rule at its place, and a map of children
  compared as every map is there, by its entries in the order of their keys; a term is finite, so the comparison ends.
  Two terms that differ only in the order their maps' keys were put are two values and two states of the model, and
  ADR 0151 D2 calls them equal, as it does two such maps. A statechart does not compare two inductive values (`===` is
  refused, as for every composed value, ADR 0086 section 6.1).
- **Depth.** `depth(union:U[B])` (and of `record:R[B]`) is the deepest JSON nesting of a term of at most `B` nodes,
  and of at most `D` levels where the type states a depth bound: the longest path over the members, each node's own
  containers counted at its place, computed by the build in time of the order of `M · B` for `M` members. It is the
  `depth(R)` a lift's `tooDeep` check reads (ADR 0087 section 2.1 item 4), whose fold today refuses a type that holds
  itself. A term's depth is not bounded by anything less: a chain of `B` nodes is `B` levels deep.
- **A reader counts nodes and levels, and refuses before it stores.** A lift of an inductive type is a streaming
  reader with an explicit stack sized by `depth(...)`, not the runtime's `json::parse`
  (`backends/rust/runtime/src/json.rs`, which builds a whole `Value` by descending into it), and `lift_too_deep`
  (`backends/rust/runtime/src/event_payload.rs`) reads the same depth. C11's wire writer replaces its depth bitmask
  (`SCE_FORGE_WIRE_MAX_DEPTH`, 31 levels, one bit per level, `backends/c/forge-runtime/include/sce/forge/wire.h`),
  which a term about ten nodes deep passes, by a stack of containers sized from the type. Every reader (the wire, the
  saved state, a codec) counts each node it begins and each level it opens, and refuses the node past `B`, or past
  `D`, before any of it is stored (ADR 0077 section 2): `tooManyNodes` (code 16, after `tooFew` at 14 and the 15 ADR
  0086 records for the harness's `notOffered`; `sce:since` the next revision of `refusal_code.scxml`'s enum) or
  `tooDeep` in the refusal record of a delivery or a lift, with `bound` the bound broken; `CborOutOfRange` for a CBOR
  codec's decode, the class ADR 0101 section 3.3 gives a list past its bound.
- **Sizes, exact, where a size is read.** `J(T)` and `C(T)` of `union:U[B]` and `record:R[B]` are the largest
  encodings of a term of at most `B` nodes (and `D` levels), computed by dynamic programming over the node count. For
  each member `m` and each `k` from 1 to `B`, `S_m(k)` is the largest encoding of a term rooted at `m` of at most `k`
  nodes (none below `m`'s least value): the bytes of `m`'s node that do not depend on its children, at their largest,
  and the best division of the remaining `k - 1` nodes among its child positions, each position's best for `j` nodes
  being: a member field, `S(j)`; an optional, its absent form for `j = 0` and `S(j)` otherwise; a list or a map of `n`
  children, its head `h(n)`, each entry's key at its largest for a map, and the best division of `j` among `n`
  children, for every `n` up to its bound and at least its lower count; a union member, the largest over its cases;
  with a depth bound, a level index beside `k`. The division is a max-plus convolution, so the computation takes time
  of the order of `M · P · F · B^2` (members, child positions, fan-out). It is made once per member, up to the largest
  bound used where a size is read (a wire field, a codec's entry, a signed value, a C11 event slot), which ADR 0086
  section 2.3 holds tight, and not at all for a type used only in variables and algorithms, whose storage is its arena
  (D5) and not an encoding. It is exact, so a maximal value encodes to exactly `C(T)` (ADR 0101 section 4.2) and
  `J(T)` (ADR 0086 section 2.3), and a C11 event slot, sized by `J` (ADR 0086 section 8), is no larger than a value
  needs. A size past `2^64 - 1` bytes is refused as every such size is (ADR 0101 section 4.4).
- **Grammars.** The JSON Schema of a member is a `$defs` entry per member of its document, keyed `<document>/<member>`
  and escaped as a JSON Pointer token (`/` as `~1`), its children by `$ref`. The use's node bound is written as the
  annotation `x-sce-max-nodes` (and `x-sce-max-depth` for a depth bound) beside the `$ref` at the use, which JSON
  Schema 2020-12 lets stand beside it (Core section 8.2.3.1), and not in the shared `$defs`, which every use of any
  bound reads; as the `x-sce-max-bytes` of a text, a reader enforces it and a validator of JSON Schema does not, and
  ADR 0087 section 1.1's sentence that a text in the schema's language is one the reader accepts is amended to hold
  with that annotation read. A GBNF grammar cannot state a node count but by unfolding the type into rules for each
  count, a number of rules of the order of `B^3`. A recursive GBNF grammar with the lift's count of nodes after it was
  weighed and is not taken: the grammar is what a model's decoding is constrained by (ADR 0087), so it would let a
  model write a text the lift then refuses as `tooManyNodes`, which is the one thing ADR 0087 section 1.1 asks a
  grammar never to do. A position that requires its type's GBNF grammar refuses an inductive type as
  `inductive/no-exact-grammar`, naming the position.

### D5. A value is held in one arena of its bound, a child named by the index of its slot

- **The representation.** A value of `union:U[B]` is an arena: `B` slots, each holding one node or free; the index of
  its root; the number of its nodes; and a free list threaded through the free slots (CLRS section 10.3; Knuth section
  2.2.3). A node holds its member (where the document has more than one), its case (for a union member), every field
  that holds no member, and its child positions. A field or an optional of a member holds its child's index (0 when
  absent). A list or a map of members, a **sequence**, holds the index of its first child and its count, and its
  children are threaded through the arena: each child holds the indices of its previous and next sibling, so a
  sequence keeps its order and a child leaves it in constant time (the first-child, next-sibling representation, Knuth
  section 2.3.2 and CLRS section 10.4, linked both ways). A child of a map holds its own key. Where the document has a
  map of members, one index per value of `2B` cells, keyed by the parent's slot, the position in the parent and the
  key, finds a child by its key as a map's index finds an entry (ADR 0108 D8), under one seed per value, which the
  value draws where ADR 0108 D8 has a map draw its own: from the host where a machine builds it, and from the value it
  is drawn from where an algorithm builds or copies it. A list's and a map's bound stays the limit of its count,
  judged where a child is added. So no node reserves a cell for a child it does not have, and a value of `B` nodes
  holds at most `B - 1` children whatever its fan-out.
- **One index width per document.** An index is the smallest unsigned integer of 8, 16 or 32 bits that holds the
  largest node bound any use of the document's members names in the build, one width for the document and the build,
  so the document has one node type whatever bound a use states. A slot is named one past its place, so 0 names no
  slot, as a C11 map names its slots (`map.h`).
- **One pool, of the largest node.** Every slot is sized for the largest of the document's nodes without its
  sequences, with its two sibling links and the largest key a map of members holds, and every member's node is held in
  the one pool. The bound is one bound over every member: any division of `B` nodes among the members is a value of
  the type, so a pool per member would have to hold `B` of its own, and the pools together take more than one pool of
  the largest node takes whenever a document has two members; pools per size class pay where each class has a bound of
  its own (Wilson, Johnstone, Neely and Boles 1995), which a type states nowhere. A document whose members differ much
  in size states the difference where it is, by holding a large part of a node in a member of its own.
- **The invariant** `Wf`: the root's slot holds a node; every child index of a node names a slot that holds a node;
  every node but the root is the child of exactly one position of one node and the root of none; each sequence's links
  agree in both directions, run from its first child, and number its count; each map child's key is in the index under
  its parent and position, and the index holds nothing else; following children from the root reaches every node and
  never the same node twice; the number of nodes is the slots that hold one, at most `B`, and no path from the root
  holds more than `D` nodes where the type states `D`; and the free list holds every other slot once. So a value is a
  tree: no node is shared, no node is lost, and nothing cycles.
- **The abstraction function** `Term(a)` reads the term from the root. Every operation is stated on terms and shown to
  commute with `Term` (Hoare 1972; D12, T1a), so what a value means does not depend on which slots hold which nodes,
  and no operation's result can show it.
- **No index leaves the value**, as no slot of a map does (ADR 0108 D2). A document reads and writes through places
  (D6, D7). A host reads through a view that borrows the value and exposes typed accessors per member and none of its
  indices, and builds a value through a builder that answers `Full` when the term would pass `B` or `D`. On C11 the
  accessors are generated functions over the bound-erased view below and an opaque node reference, `sce_arena_ref_t`,
  whose field is no part of the interface; where ADR 0108 D8's C11 map view shows a host its slot positions, this one
  shows none.
- **Freeing is exact.** A node leaves the value only with the subtree below it, which no other node reaches (`Wf`),
  and its slots go back to the free list in the same operation. No collector runs, and a value never holds an
  unreachable node.
- **One free list, proved once.** The slot pool and its free list are written once, in `sce-portable-bytes` on Rust
  and beside `map.h` on C11, and shared by the arena and the map (`BoundedMap`), so one free list is proved (D12, T1a)
  and two cannot drift.
- **One shape on Rust and C11.** Rust holds a value as `sce_portable_bytes::arena::Arena<N, B, S>`, `N` the node type
  the generator writes for the document (an `enum` with a variant per member, or the member's `struct` where there is
  one), and `S` the storage, chosen by type as a map's is (ADR 0108 D4 "Storage"): `Inline`, the slots in the value,
  which never allocates, and `Heap`, grown up to `B` where the runtime has an allocator; a lent storage follows ADR
  0154 on its acceptance, by the same reason. C11 holds it as the type `SCE_ARENA_DECLARE` declares in
  `backends/c/forge-runtime/include/sce/forge/arena.h` for the document's node type and the bound, reserved whole
  where the machine holds it (ADR 0082), the node type a `struct` of a member tag and a `union` of the members'
  layouts, and declares beside it `sce_arena_view_t` (the slots, the count, the capacity and the root), a view of a
  value of any bound, through which an algorithm takes one, as `list_view.h` declares a list's. The structure is the
  same on both lanes: the slots, an index of one width, the sibling links, the free list and `Wf`. Neither lane's
  memory layout is a contract; the bytes the two lanes must agree on are D4's forms, which are of the term.

### D6. A statechart holds a value as a variable, reads it by places and writes it in place

- **A variable.** `<data id="config" sce:type="union:Config[config_nodes=512]"/>` holds a value of the type, built
  whole as a record or a union variable is (`docs/SCE_ACCEPTED_SUBSET.md` §2.15, ADR 0092 section 6.1 item 5): a union
  member with no default value (ADR 0092, Amendment of 2026-11-07), a record member from one `<sce:set>` per field. A
  child field takes a value of its member: another variable, a place inside a value, an event's field, what an
  algorithm returns. It is a value: what it takes is the term, whatever holds it.
- **A place** is a path from a variable through its nodes: a field (`config.members`), a list's element at an index or
  a `<foreach>` item, a map's value at a key, each step through a union member taken where its case is proven, by ADR
  0092 section 6.1 item 2's rule and its code. A place reads a sub-value, and `nodes(x)`, a `uint32`, reads how many
  nodes a value or a place holds. Neither copies.
- **A walk.** `<foreach>` over a list or a map inside a value walks it as a list's or a map's loop does (ADR 0108 D5),
  its item a sub-value; a body that writes the walked value takes the copy ADR 0108 D6 item 3 says it takes, and only
  such a body. The copy is of up to `B` nodes, held in a scratch arena of `B` slots, on C11 a scratch role of the
  machine (ADR 0086 section 8.5), counted in D14.
- **A write in place.** `<assign location="p" expr="e"/>` replaces the sub-value at the place `p`, and the list and
  map statements (`<sce:append>`, `<sce:clear>`, `<sce:put>`, `<sce:remove>`) act on a sequence inside a value through
  a nested place (ADR 0108 D12), the list's or the map's own bound judged as it is. Its meaning is stated on terms:
  the result is the old term with the sub-term at `p` replaced by the value `e` had before the write. A write
  **succeeds exactly when the term it leaves has at most `B` nodes and `D` levels**, and otherwise leaves the value as
  it was and raises `error.execution`, as an append to a full list does. Success is a property of the result, not of
  how the write is carried out. Every write is lowered to one routine (`Arena::replace` and its C11 twin), which reads
  the source before it frees anything and takes one of three ways, so that it never answers full for a result that
  fits (D12, T1a):
  - the source lies inside the sub-tree at `p` (`<assign location="t.left" expr="t.left.right"/>`): the source is
    moved to `p`, and the rest of the old sub-tree at `p` is freed;
  - the source contains `p` (`<assign location="t.left.x" expr="t.left"/>`): the source less the sub-tree at `p` is
    copied into free slots, and the old sub-tree at `p` is moved into the copy, at the copy's place of `p`, so its
    nodes serve the new term and none of them is copied;
  - the source and `p` are disjoint, or the source is another value: the old sub-tree at `p` is freed, then the source
    is copied.
- **A put into a map of members.** A `<sce:put>` whose map value is a member names its `outcome` as any put does (ADR
  0108 D4), and may answer `full` for either bound, the map's count or the value's nodes. A put that replaces a
  present key may answer `full` too, since the new value may hold more nodes than the one it replaces: ADR 0108 D4's
  sentence that a replace never needs a slot holds of a map's own slots and not of a value's nodes, and is amended so.
  A document that checks its limits before it puts checks `nodes(...)` beside `len(...)`, and
  `sce:std/collection/put_outcome.scxml`'s comment says that `full` may also mean a value's nodes.
- **Totality** (ADR 0133 D1, amended by three items). A store of a whole variable from a value whose type's node bound
  and depth bound are at most the variable's is total, as a widening is (ADR 0133 D1 items 4 and 5); a `<sce:remove>`
  and a `<sce:clear>` inside a value are total, since they only free nodes; a store from a larger bound into a smaller
  one is checked, and raises `error.execution` past the bound, as a list's is (ADR 0086). Every other write that adds
  nodes stays a statement that can fail (ADR 0133 D1 item 6: "a list or a map an algorithm returns, a write of a map
  at a key: each stays checked"), and a read of a place stays a read that can miss where it reads an index or a key
  (item 3). A document that must not fail tests `nodes(...)` against `_x.limits.<name>` before it writes, as ADR 0108
  D4's document tests `len(faces)`.
- **Saved state, snapshot and replay.** A variable is saved, shown in the snapshot and written in a scenario as its
  term in D4's JSON form; the saved shape names the member by its lineage, `<document name>/<member>` (D1), and its
  bounds, so a change of a member moves the shape as a change of a record does (ADR 0045). A restore reads the term
  with D4's reader and refuses one past the bound.
- **An event's field** may be of an inductive type, read and sent as D4 says; a member that holds a map stands in no
  wire position (D9).

### D7. An algorithm reads a value through a view and builds one into storage its caller holds

- **A parameter is a view** of its argument, as every parameter is (SCE_FORGE.md §4.12): of an inductive type it
  writes no bracket and holds the argument's bound, and is lowered to a borrow of the argument's arena and the index
  of the sub-value's root (Rust `ArenaRef<'_, N, M>`, C11 the bound-erased `sce_arena_view_t` and a node reference,
  D5), so a call copies nothing. A place read from a view is a view.
- **A return is built into the caller's storage**, as an algorithm's list is (ADR 0082):
  `<sce:return type="union:Config[out_nodes=512]"/>` takes a destination of that bound from its caller and the body
  builds the term into it from the leaves up, each node after its children, so the destination is written forward only
  and never holds a node the result does not. An algorithm that returns an inductive value declares `may-fail`, as one
  that returns a list does, and a build that would pass the destination's bound fails the call, which leaves the
  caller's variable as it was (ADR 0108 D5's rule for a map built for a return).
- **A built sub-value is used exactly once.** A value a local function of the same document returns into the
  destination (D8), or a node the body builds there, is placed as one child, or returned, exactly once on every path:
  a second use would make two parents of one node and a value with none would hold the destination's slots for
  nothing. A body that does otherwise is refused as `algorithm/built-value-not-used-once`, naming the value and the
  path. A view of a parameter placed into the destination is copied there, a cost of its nodes.
- **A local of an inductive type** states its bracket and holds an arena of its own, in the frame of a function that
  does not recurse (D8).
- **A descent is a loop.** A local that is a view of an optional member (absent once the walk has left the tree) may
  be walked down by a `<sce:while>`. The loop is a **descent** when on every path through its body the local is set to
  a strict child of itself or the loop exits; it then runs at most the node count of the value it started in (at most
  `D` passes where the type states a depth bound), and the build counts it so, without a `max-iter`, and does not
  refuse it as `algorithm/while-unbounded` (§synth-5-A, amended). A walk down one path of a trie is written so. A
  descent's local is not a path in D8's sense: a recursive call's argument is a path written from the measure
  parameter's name.
- **A value is written in place by an algorithm that cannot fail.** As ADR 0108 D6 item 2 admits a map, an algorithm
  that does not declare `may-fail` may take a parameter of an inductive type `inout="true"`: it writes the caller's
  value in place, and the caller sees every write when the call returns. Every write in it that adds nodes names an
  `outcome` of `sce:std/collection/put_outcome.scxml` (a put's, or the `outcome` such an algorithm's `<sce:assign>`
  takes into an `inout` value), since it can find the value full, so the algorithm cannot fail and never leaves a
  write half done; a write is made at a place written from the parameter's name or at a descent's local, which in an
  `inout` value is a cursor of it and not a view. An `inout` slot on a `may-fail` algorithm is refused as ADR 0108 D6
  item 2 refuses one (`algorithm/inout-on-may-fail`), and a statechart calls the algorithm by `<sce:call>` as that
  item says. A trie takes a key path of run-time length so: a descent walks the path's chunks and puts a child where
  one is absent, in time of the path's length and not of the trie's size, where a statechart's places, of a length
  fixed in its text, could only rebuild the value whole.

### D8. A function recurses only on disjoint proper parts of one argument, and runs on an explicit stack

- **The recursive scope is one document.** An algorithm document may declare, beside its entry, local functions,
  `<sce:function name>`, each with a `<sce:signature>` and a `<sce:body>` as the entry has, callable from the entry
  and from each other and from nowhere else. The entry and its local functions may call each other in cycles; a cycle
  of calls through imports stays an import cycle and stays refused (`manifest/circular-dependency`). A function on a
  cycle of calls declares its **measure**, `<sce:measure param="<param>"/>` in its signature (an element apart from
  the `<sce:decreases>` ADR 0115 D10 gives a loop, which holds an expression; one name in two shapes would be read two
  ways, ADR 0115 D10 amended), a parameter of a member type; every function of one cycle measures a member of one
  inductive document.
- **The rule.** A call from a function on a cycle to a function of the same cycle is **structural** when the argument
  it passes at the callee's measure is a path written from the caller's measure parameter's name, of one step or more,
  each step a field of a member, an element of a list or a value of a map taken as the item of a `<sce:foreach>` (or
  at an index), through union members whose case is proven, ending at a member value: a node strictly below the
  caller's argument. Any other argument is `algorithm/recursion-not-structural`, naming the call and the measure. Two
  more conditions make the calls of one activation take **disjoint** subtrees, each at most once:
  - every `<sce:foreach>` that encloses the call inside the function contributes exactly one element step to the path,
    its own item, and no `<sce:while>` encloses it, so each run of the call site takes another node;
  - no two call sites that can both run in one activation take paths one of which is a prefix of the other, two sites
    being unable to both run only when they lie in different branches of one `<sce:if>`. In the comparison an element
    step, a loop's item or an index, matches every element step of the same list or map.

  Either broken is `algorithm/recursion-overlaps`, naming both sites, or the site and the loop. The other parameters
  of a structural call are free: they carry what the walk computes on its way down.
- **What it proves.** Each activation's calls take disjoint proper subtrees of its own argument, so every node of the
  first argument is the argument of at most one activation of the cycle, by induction on depth. A call into a cycle
  from outside it, with `v` at the measure, therefore makes at most `nodes(v)` activations of that cycle, at most `B`,
  and is at most `height(v) + 1` frames deep (at most `D + 1` where the type states a depth bound): it ends. The
  bounds are per cycle. A body on one cycle may call another cycle on the same value (a cycle that computes a
  sub-tree's size, called at every node), and a body's cost then includes the cycles it calls: the work of such a call
  is at most `B` activations of the inner cycle for each of at most `B` activations of the outer one, a term of `B`
  (of the order of `B` times the height), which D14 states. In the terms of the recursion schemes, a structural
  function is a paramorphism (it reads the descendants it does not call on; Meertens 1992), with downward accumulation
  for the values its free parameters carry down (Gibbons 1993), and a mutumorphism where two functions recurse through
  each other (Fokkinga 1990). The measure is the size of the argument, which falls at every call, so it is also
  Nordström's well-founded recursion and Verus's `decreases` (D12, T2).
- **No recursion is native.** A function cycle is compiled on every lane to one loop over an explicit stack of frames,
  each frame the locals of one activation and the point it resumes at, the continuation-passing transformation
  followed by defunctionalization (Danvy and Nielsen 2001, after Reynolds 1972), as the build already runs Appendix
  D's recursions over a stack of the document's capacity (`ARCHITECTURE.md`). A stack holds at most `B + 1` frames
  (`D + 1` with a depth bound) for each cycle live on it, and its capacity is the sum over the cycles a call can have
  live together; it is known where the entry is called, from its argument's bounds: storage the caller holds, as an
  algorithm's output is (ADR 0082), taken on the heap up to that capacity where the Rust runtime has an allocator, and
  from the caller's storage on Rust without one and on C11, where a machine that calls holds it as a scratch role (ADR
  0086 section 8.5). There the stack, and an inductive return's destination (D7), pass through every algorithm between
  the caller that holds them and the recursion: whether an algorithm takes a recursion's stack, and its capacity, is
  decided over its calls as a whole, as ADR 0108 D8 decides an algorithm's seed. A recursion therefore needs no stack
  of the platform's beyond one frame of its loop, which the small stack of a freestanding C11 core on a
  microcontroller (ADR 0110) can give whatever the depth of the tree.
- **A function on a cycle holds no arena of its own**: a local of an inductive type with a bracket is refused there as
  `algorithm/arena-in-recursive-function`, since every activation would hold one and the frames would hold up to `B`
  of them. It holds views, and builds into the entry's destination (D7).
- **A pattern over a trie is a fold with a state.** A match of a key pattern with wildcard chunks against a trie
  visits a node once, carrying the set of positions of the pattern's automaton that the path to the node leaves live
  (Thompson, "Regular expression search algorithm", Communications of the ACM 1968; ADR 0149's automaton over a text):
  at a node, the set is advanced by the chunk of each child, and the walk calls on a child only where the set is not
  empty. It is structural by the rule above, and its cost is the trie's nodes times the pattern's positions, where a
  recursion on the pair of a node and a position of the pattern, which the rule refuses, can visit one node once for
  each path through the pattern.

```xml
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="algorithm" name="config_has_none">
  <sce:import kind="inductive" src="config_value.scxml" member="Value" as="Config"/>
  <sce:import kind="enum" src="config_kind.scxml" as="ConfigKind"/>
  <sce:signature>
    <sce:param name="v" type="union:Config"/>
    <sce:return type="bool"/>
  </sce:signature>
  <sce:body>
    <sce:return expr="hasNone(v)"/>
  </sce:body>
  <sce:function name="hasNone">
    <sce:signature>
      <sce:param name="v" type="union:Config"/>
      <sce:measure param="v"/>
      <sce:return type="bool"/>
    </sce:signature>
    <sce:body>
      <sce:if cond="v.kind === ConfigKind.none"><sce:return expr="true"/></sce:if>
      <sce:if cond="v.kind === ConfigKind.array">
        <sce:foreach in="v.items" item="c">
          <sce:if cond="hasNone(c)"><sce:return expr="true"/></sce:if>
        </sce:foreach>
      </sce:if>
      <sce:if cond="v.kind === ConfigKind.object">
        <sce:foreach in="v.members" item="m">
          <sce:if cond="hasNone(m.value)"><sce:return expr="true"/></sce:if>
        </sce:foreach>
      </sce:if>
      <sce:return expr="false"/>
    </sce:body>
  </sce:function>
</scxml>
```

`hasNone(m.value)` passes a grandchild of `v` (the `Member` node is the child): its path is `members`, the loop's
item, `value`, one element step for its one loop, and no prefix of the other site's path `items`, the loop's item. A
second `hasNone(c)` in the first loop's body is `algorithm/recursion-overlaps`, and a call `hasNone(v)` is
`algorithm/recursion-not-structural`: `v` is no node below itself.

### D9. How unions and maps compose with it

**Unions (ADR 0092).**
- A union member is a union in every rule of ADR 0092: its tag, its one-to-one cases, its wire form, its narrowing,
  its whole construction, its growth. What changes is the scope of a case's `record`: a record member of its document,
  or an imported event schema, or nothing. ADR 0092 section 1.3's rejection of cases declared inline stands: a case
  still names a record, which is a member any position of the grammar can hold and hand to an algorithm.
- ADR 0092 section 1.1 item 3's sentence that a union that holds itself is a type that contains itself
  (`validation/cross-kind-circular-dependency`) stays true across documents; within an inductive document it is the
  recursion this record admits.
- A union member's node holds its case's record inline (D2), so a recursive sum costs one node per value, as a sum
  holds its case in the languages ADR 0092 section 7 lowers it to.
- A union that is not a member may hold a member type with its bracket in a case's record, as any record may:
  `union:Answer` with a case whose record holds `union:Config[config_nodes=512]`.

**Maps (ADR 0108).**
- **A map's value may be a member**, inside the document (`literal` above: children by key, each a node, its key held
  in the child, D5) or outside it with a bracket (`map<string[32], union:Config[config_nodes=64]>[16]`: each entry a
  value of its own bound, held in its own arena, which Inline storage reserves for each of the map's slots and Heap
  storage grows as the map is filled). A map of children keeps ADR 0108 D2's order, the order its keys were put. That
  order is part of the term, so of its saved form, its snapshot, its canonical form in the model (D11) and its walks.
  Where ADR 0151 D2's comparison is read, a map of children is compared as every map is, by its entries in key order.
  Two terms differing only in put order are two values and two model states, and ADR 0151 calls them equal.
- **A member type is no key** (`validation/map-key-type`, with the reason `NotAKeyWhy::Inductive`): a key's every
  value is bounded, which a member's is only through a bracket, and a key held inside a node of the same document
  would put a node in two places of one term. Nothing asks for a tree as a key; a set of trees is a map keyed by what
  tells them apart.
- **A key path is a key** by D10, which amends ADR 0108 D1 apart from the rest of this decision.
- **A trie keyed by its paths.** Where a document reads a table only by whole paths (does this path hold a value, put
  one at it, remove it) or may walk the whole table per question within its bounds, the table is
  `map<list<string[chunk=64]>[depth=16], record:Route>[routes=1024]`, a map keyed by the chunk path, and needs nothing
  of this record but D10. Where it asks for the entries under a prefix, or matches a pattern with wildcard chunks
  against every path, in time that must not grow with the table, it is the trie of D1, walked by D8's fold and written
  by D7's descent in an `inout` value. Both are written in SCE; the document chooses by what it asks.
- **Wire positions.** A member that holds a map, at any depth, stands in no wire position
  (`validation/map-in-wire-position`, ADR 0108 D7), as a record that holds one does not; a trie is held in a variable
  and walked by algorithms, and sent, where it must be, as a value of a type without a map (a list of path and route
  records).
- **ADR 0108 D6 and D12 hold within a value.** A read never copies a sub-value; a loop copies only a value its body
  writes; a nested place inside a value is written in place.

### D10. A bounded list whose element is a key is a key

ADR 0108 D1 is amended: `list<K>[B]`, `K` a key, is a key; `list<string[chunk=64]>[depth=16]`, a key path, is the one
this record needs. It is a facility of its own, beside the rest of this record, with its own test (Order of work, step
8).

- **Equality and order.** Two such keys are equal when their lengths are equal and their elements, in order, are
  equal. Where an order is read, it is ADR 0151 D2's for a list: element by element, a prefix before what extends it.
- **What the index reads.** The hash and the comparison of ADR 0108 D8 read a key's canonical form, which need only be
  injective and self-delimiting, since the hash is never observable: a list key is its length, then each element's
  form in order. Under ADR 0108's amendment of 2026-11-28 (a key is a sequence of atoms, read by one function of its
  type; on its landing branch, not on `main` at `662e4d060cc`), that is its length as one integer atom and then each
  element's atoms; before that amendment lands, the canonical bytes of ADR 0108 D8, its length in eight little-endian
  bytes and then each element's bytes. Either is the same equality.
- **No overlap with a byte string.** On Rust a list key is held in a type of its own, `ListKey<K, N>`, whose `MapKey`
  gives the length and the elements; `BoundedList<u8, N>`'s impl stays the one byte string a `bytes[N]` key is
  (`backends/rust/portable-bytes/src/map.rs`), so no two impls overlap, and `list<uint8>[N]` and `bytes[N]` stay two
  key types.
- **What stays no key.** A set and a map, whose equality is of sets, which ADR 0108 D1's reason against a collection
  is true of; a list whose element is no key; a member type (D9).
- **Lanes.** Rust and C11, as the rest of this record; every other lane refuses such a key by name until it lowers it.

### D11. The model holds a value in its canonical preorder form and computes over it by folds

- **The canonical form.** The model holds a value of `union:U[B]` as
  `[n |-> k, at |-> [i \in 1..B |-> IF i <= k THEN node ELSE Free]]`: its `k` nodes numbered in preorder from 1, the
  root first, each node a record of its member, its case, its fields that hold no member, and its children's numbers;
  the slots past `k` the one value `Free`. `Canonical(x)` states it: `x` is well formed (`Wf` read on the numbering),
  the root is 1, each child's number is its parent's plus one plus the sizes of the sub-trees of its earlier siblings
  (children in the order their positions are declared, a list's in its order, a map's in ADR 0108 D2's), and every
  slot past `n` is `Free`. `Wf` alone admits other numberings of one term; `Canonical` admits one, so TLC's equality
  of states is equality of values. That it does is the theorem `CanonicalIsOneToOne`, proved for every `B` in TLAPS
  (approved by ADR 0156 D13) and checked by TLC at small `B` (D12); until its proof lands, `docs/formal/README.md`
  names it bounded (ADR 0115 D3).
- **Every operation is a fold, never a recursive operator.** In preorder every node's descendants have greater
  numbers, so a computation from the leaves up is one pass over `B .. 1`, and one from the root down one pass over
  `1 .. B`. The module `docs/formal/Inductive.tla`, beside `AppendixD.tla` and in its words, writes once, over the
  canonical form: `Canonical`, `Nodes`, a place's sub-value, and the replacement of a sub-value (the new term numbered
  again by one pass that computes sub-tree sizes and one that assigns numbers). A function of D8 carries parameters
  down, returns from inside a loop and passes one child's result into a sibling's call, so it is not one pass from the
  leaves up: each D8 cycle is translated as `ApaFoldSeqLeft` over the preorder `1 .. B` with an accumulator that holds
  the explicit stack of frames, each step running the frames until the next call, which is the defunctionalization
  every lane runs (D8). TLC checks each translation against a `RECURSIVE` reference operator of the same function on
  fixtures at small `B`, which TLC accepts. No translation uses `ApaFoldSet`, whose order is not specified. `Free` and
  each member's node are the variants of one type (Apalache's `Variants.tla`), so a slot has one type Apalache's type
  checker gives it; Apalache has supported neither recursive operators nor recursive functions since its release
  0.23.1, and the release the judge pins is 0.63.0.
- **What a document's model gains.** A variable of an inductive type has `Canonical` as its `TypeOK`, with `n` at most
  its node bound and every path at most its depth bound, the obligation ADR 0156 D4 generates for every bounded
  structure; a write is the replacement operator, with its `error.execution` when the result passes a bound; a descent
  (D7) is bounded by `Nodes`; a build into a destination that answers full is the call's `error.execution`; a codec's
  refusal of a value past its bound is the disjunct `Nodes(v) > B` of `Refuses_C`, written from
  `codec_totality::refusals` (ADR 0065 section 2, ADR 0115 D7); and a lift's refusal record holds `tooManyNodes` (D4).
- **The model's node bound is the exploration's.** A canonical form of `B` slots for every state is more than TLC
  explores at a bound a deployment builds with (512 nodes, say). The node bound and the depth bound of each inductive
  variable are constants of the module, which an exploration (ADR 0115 D15) sets, at most the build's, and every
  verdict names the bounds it was judged at, as *holds within* them: a smaller bound answers `full` where the build's
  would not, so a verdict at it is of that machine, and is read for the build's by the small-scope hypothesis
  (Jackson, *Software Abstractions*, MIT Press 2006), stated as an assumption with the verdict, never as a proof of
  the larger bound. An exploration names the values an input of an inductive type may take as it names any input's, or
  a node count from which the values are drawn. A lane's run is read against an instance of the model at the build's
  bounds (below), which a single trace's states keep small.
- **Refused by name until it is translated.** Today `sce-codegen model` refuses every `sce-static` document whole
  (`Construct::Datamodel("sce-static")`). Step 2 adds `Construct::Inductive`, naming each inductive variable, so a
  document that holds one is refused by both today, and by `Construct::Inductive` alone once ADR 0156 D3 translates
  the typed values of `sce-static`, until this decision's translation lands (step 5b). A document is never given a
  model of part of itself (ADR 0115 D7).
- **Runs are compared as terms.** A lane's trace writes a value as its term (D4's JSON form), and the model reads it
  into the canonical form, so ADR 0115 D8's acceptance of a run does not depend on how the lane's arena placed its
  nodes.
- **One surface.** `Inductive.tla` is registered with `AppendixD.tla` in the one spec-bearing surface of the semantics
  ADR 0156 D2 names, in `SCE_WIRE_CONTRACTS.md`.

### D12. The formal level of each claim

Each claim is judged at the level of ADR 0115 that can judge it for every size where one can, and the reason is given;
none rests on a random test alone.

| Claim | Level | How | Why this level |
|---|---|---|---|
| T1a. The arena's generic operations, over every storage `S` (build, read a place, the three ways of `replace`, the list and map statements at a place, the sibling links, copy, free), and the slot pool and free list it shares with `BoundedMap`, keep `Wf` and commute with `Term`; `replace` succeeds exactly when the result is within the bounds and leaves the value as it was otherwise | theorem, every `B` (ADR 0115 D13), proved once | Verus, over `backends/rust/portable-bytes/src/arena.rs` and the shared pool, named in `docs/formal/proofs.json` with their mutants; `BoundedMap`'s use of the shared pool is proved in the same step, since `proofs.json` names no proof of it today | an invariant of a data structure and a refinement (Hoare 1972) over every size; the model holds terms and not arenas, so no model judges the arena, and a random test samples what the theorem states of every arena |
| T1b. A document's encode is a function of `Term`, and its decode the inverse on every encoding within the bounds, refusing the node past `B` and the level past `D` before it stores it | theorem per generated document (ADR 0115 D10, ADR 0156 D11) | the generator writes each document's encode and decode with a Verus specification over the term; Verus checks each output | the codec of a document is generated, not written once, so it is proved where it is generated, as T2 |
| T2. Each generated recursion's loop simulates its functions' recursion: it ends within `nodes(v)` activations of each cycle and never pushes its stack past its capacity | theorem per generated algorithm (ADR 0115 D10, ADR 0156 D4) | the generator writes each recursion's Rust with a Verus specification function over the term, `decreases` the argument's size, and the loop's invariant relating the stack and the term; Verus checks each output. It checks the defunctionalization, not what the body means, which is the document's | D8's rule is a decision procedure of the build; checking each output keeps the generator and the rule outside the trusted base (Pnueli, Siegel and Singerman 1998; ADR 0156 D11). It waits for ADR 0115 D10's generated Verus form, which ADR 0156 records as not started |
| `CanonicalIsOneToOne`, and each operator of `Inductive.tla` the operation on terms | theorem, every `B`, for the first; model at small bounds for both | TLAPS (approved by ADR 0156 D13) for the theorem, named bounded in `docs/formal/README.md` until it is proved; TLC checks the theorem and each operator against a term-level definition for every term of at most 5 nodes of the fixture documents | the module is a definition the model generator reads, and the theorem is what makes a state of the model a value; exhaustive checking at small `B` catches an operator written wrong |
| Each D8 translation is its function | model at small bounds | TLC, the fold against a `RECURSIVE` reference operator, on fixtures | the reference is the function as written; TLC accepts it where Apalache does not |
| A lane's value is the model's at every observation | runs (ADR 0115 D8 and its converse) | traces and replayed behaviours, values compared as terms (D11) | as for every document with a model |
| `J` and `C` are the largest encodings, exactly | an argument for every `B` (optimal substructure) and tests | the build's tests enumerate every term of at most 7 nodes of the fixture documents, and compare closed forms where CBOR's heads widen (a star of 24 leaves and one of 256, a chain of 300), with the computed sizes; the conformance corpus encodes a maximal value on each lane | the recurrence is Bellman's; the enumeration and the closed forms hold the code to it, the second where seven nodes never reach |
| The C11 arena meets T1a's contract | bounded at `B <= 8` (the small-scope hypothesis, Jackson, *Software Abstractions*, 2006), named as bounded in `docs/formal/README.md` (ADR 0115 D3); and differential tests | CBMC harnesses of the same contract, once the owner approves CBMC for this use; until then, differential tests alone: the conformance corpus holds C11's bytes and refusals to Rust's | no deductive checker of C is pinned |
| An author's properties over inductive variables | model, per document (ADR 0115 D15) | TLC over the generated module, at the exploration's bounds (D11) | as every property |

CBMC's use for the C11 arena is approved only by the owner's own words, recorded here. Until then step 9 holds C11 by
differential tests alone. ADR 0156 D13 approves CBMC, in the owner's words, for the harness of a C11 seam, a thin one
whose bounded proof D7 of that record argues complete; ADR 0115 makes each tool the owner's to approve, and an
acceptance the owner delegated does not extend an approval. The main session is asking the owner for it.

### D13. Lanes

Rust and C11 serve it, because the consumer that asked generates its sans-I/O core for both from one set of documents,
and the scope rule builds a facility on the backends a consumer needs:

- **Rust**, native, `no_std` with `Inline` storage, and `wasm32-unknown-unknown`: the arena, the machine's variables
  and places, the algorithms' views, outputs and recursions, the wire, the saved state and the codec, the lift a
  streaming reader beside `json.rs`;
- **C11**: the same, in the freestanding core of ADR 0110, every arena and every stack reserved where the machine or
  the caller holds it, nothing allocated; its wire writer holds a stack of containers sized from the type in place of
  `wire.h`'s depth bitmask (D4), and a host reads a value through generated accessors over `sce_arena_view_t` (D5).

Go, Kotlin, Python, C++, Swift and the Interpreter refuse a document that holds an inductive type by name where they
render it (`generate/unsupported-feature`, naming the type and the position), as ADR 0086 section 7.3 and ADR 0092
section 7 have a lane refuse a composition it does not lower, and the TTCN-3 export refuses it by name as it refuses a
construct it does not export. Refusing the Interpreter has a cost that is stated here: ADR 0115 D15 has the
Interpreter, each lane and the model agree on every step of a run drawn over an exploration, and for a document that
holds an inductive value that agreement is of two parties, the lanes and the model. A lane is added by an amendment
when a consumer needs it, against the same corpus.

### D14. Refusals and bounds

New `DiagnosticCode`s, each with its appendix row in `docs/SCE_ACCEPTED_SUBSET.md`
(`acceptance_doc_covers_every_code`), its row in SCE_ERROR_CONTRACT.md's catalog and
`schemas/sce-diagnostic.v1.schema.json`, added in the step that makes it:

| Code | Refuses | Decision |
|---|---|---|
| `inductive/member-not-recursive` | a member on no cycle of its document | D2 |
| `inductive/uninhabited` | a member with no finite value, naming the field that cannot be filled | D2 |
| `inductive/member-name-taken` | two members of one name, or a member named as an import's alias | D2 |
| `inductive/bound-below-least-value` | a node bound below its member's least value, or a depth bound below its least depth, as written or after a build moves it | D3 |
| `inductive/no-exact-grammar` | an inductive type where a position requires its GBNF grammar | D4 |
| `algorithm/recursion-not-structural` | a call on a cycle whose argument at the callee's measure is no path below the caller's measure; a function on a cycle with no `<sce:measure>`, or measures of two documents on one cycle | D8 |
| `algorithm/recursion-overlaps` | two call sites that can run in one activation on paths one of which is a prefix of the other; a call site under a loop that contributes no element step, or under a `<sce:while>` | D8 |
| `algorithm/arena-in-recursive-function` | a local of an inductive type with a bracket in a function on a cycle | D8 |
| `algorithm/built-value-not-used-once` | a built sub-value placed or returned twice, or left unused, on some path | D7 |

Existing codes keep their classes: a member used without its bracket or with one inside its document, and a depth
bound above the node bound, `validation/attribute-rule-violated`; an `inout` inductive parameter of a `may-fail`
algorithm, `algorithm/inout-on-may-fail`; a member type as a map's key, `validation/map-key-type`; a member holding a
map in a wire position, `validation/map-in-wire-position`; a case's record that names no member and no import,
`validation/invalid-attribute`; a read of a case's field unproven, `expression/union-read-unproven`; an import cycle,
`validation/cross-kind-circular-dependency` and `manifest/circular-dependency`; a lane that does not lower it,
`generate/unsupported-feature`. A run's refusals are `tooManyNodes`, `tooDeep` and `CborOutOfRange` (D4) and a write's
`error.execution` (D6), not diagnostics.

**Bounds, by name** (ADR 0050, ADR 0086 section 1.2): the node bound of every use, `[name=N]`, and its depth bound
where it states one, each moved by `--limit`, read as `_x.limits.<name>`, and declared by a machine's and a schema
unit's constants as every bound is (ADR 0050 section 3); the bounds of every list and map inside a member, each its
own.

Every other size is a term of those names, stated in ADR 0118 D3's vocabulary once that record is accepted:
- an arena's storage: `B` times one slot, the largest of its document's nodes without their sequences, with two
  sibling links and the largest key a map of members holds (D5), and the index's `2B` cells where the document has a
  map of members. A child sequence reserves nothing per node: the trie of D1 at 1024 nodes, a fan-out of 32 and chunks
  of 64 bytes takes about 100 KB, where children reserved at their full fan-out in each node would take about 3 MB;
- a member of another document held in a node with its own bracket: that value's arena inside each slot, so its
  storage is multiplied by `B`, which a document that holds one states;
- a recursion's stack: at most `B + 1` frames (`D + 1` with a depth bound) of the cycle's largest frame for each cycle
  live on it, summed over the cycles a call can have live together; its activations, at most `B` for each cycle per
  call into it; a body's work, one body's cost and the work of the cycles it calls, at most of the order of `B` times
  the height for a cycle called at every node of another (D8);
- a walk that writes the value it walks: a scratch arena of `B` slots, a scratch role of a C11 machine (ADR 0086
  section 8.5) (D6);
- `J` and `C`: the values D4's computation gives for the build's bounds, made where a size is read, which are not a
  closed expression of `B` in general and are stated as the build's evaluation of that computation; a C11 event slot
  is sized by `J` (ADR 0086 section 8).

## Not in scope

A graph with sharing or with cycles (a node of two parents, a back edge), which is a map from a number to a node or a
relation of ADR 0151 (ADR 0165, Proposed), and not a term; sharing of nodes between two values (the path copying of a
persistent structure), since a value here is a tree owned once and a snapshot of it is a copy or a journal's (ADR
0114), and a snapshot of a value shared by readers in other execution contexts (read-copy-update) is the seam's,
behind the door of ADR 0084, and not the value's; infinite and coinductive values; an inductive document generic in
its types beyond ADR 0166's regular recursion, and polymorphic recursion; a grammar codec or a JSON Schema import
whose root type is inductive, which waits for an amendment of ADR 0163; a member type as a key; recursion other than
D8's, and recursion through imports; a recursive scope that spans documents; an inductive type on Go, Kotlin, Python,
C++, Swift, the Interpreter and the TTCN-3 export; a GBNF grammar of an inductive type; and any one application's
documents.

## Consequences

- A recursive type is a type of the language: declared once, held by any position, written to every form SCE writes,
  sized by the build, compared, saved, replayed and modelled as a value.
- A value never holds more than its bound of nodes, nor more levels than its depth bound, and never an unreachable
  node, on any lane, and a write past a bound leaves it as it was. Its storage is its bound times one slot of its
  largest node without its child sequences (D14), reserved or grown as a map's is, and a node of a large fan-out
  reserves nothing for the children it does not have.
- No index, slot or pointer is part of a value's meaning or bytes; two lanes, or one lane after a restore, that hold
  one term in different slots are indistinguishable.
- A function over a tree ends, makes at most one activation per node for each cycle of functions, costs its nodes
  times one body and the cycles that body calls, and needs no native stack beyond one frame, whatever the tree's
  depth; its explicit stack holds at most `B + 1` frames, or `D + 1` with a depth bound, for each cycle live on it. A
  function the rule refuses is written as a fold carrying a state, or as a descent, or not over a tree.
- A table read only by whole paths is a map keyed by the path, with no tree (D10). A trie takes a path of run-time
  length in time of the path, by a descent in an algorithm that writes it `inout` (D7).
- A document that holds an inductive value has a model once the model generator translates `sce-static`'s typed values
  and this record's step lands, and none before; it is refused by name in the meantime.
- Two lanes serve it; five and the Interpreter refuse it by name until a consumer needs them, and for a document that
  holds an inductive value ADR 0115 D15's agreement is between the lanes and the model.

## Alternatives rejected

- **Pointers to nodes on the heap** (Rust `Box`, C `malloc`). No bound on the nodes, an allocation per node, no form
  for C11's freestanding core, no value a model can hold, and an address that a value's bytes and equality must then
  be kept from.
- **Cycles of imports between event-schema and union documents.** A mutual definition is one fixed point and one
  revision; the import graph's topological order is read by the manifest and the generator (D1).
- **A self reference on a codec's field** (`type="self"`, §synth-5-H's sketch). Self-recursion only, inside a codec
  only: a value of the language is not a codec's frame (ADR 0101 section 1).
- **A flat trie built at deployment time** (§synth-5-F). It serves a set of keys the deployment fixes, and stays the
  right answer there; it is not a value a machine changes at run time.
- **A bound on depth alone, or a fixed-depth unrolling of the type** (§synth-5-H's stopgap). A depth with a fan-out
  admits a number of nodes exponential in it, so neither storage nor a walk's cost follows from it; an unrolling
  writes the type once per level. A depth bound is kept beside the node bound, where it sizes a stack (D3).
- **The arena as the wire form** (a list of nodes and their indices). One value would have as many forms as
  numberings, against ADR 0101's one form per type, unless the numbering were made canonical, which is the term's form
  again with the indices written out; and the representation would become a contract.
- **Sharing of equal subtrees** (hash-consing, a directed acyclic graph). It makes a value's node count and its
  storage two numbers, a write in place unsound without a copy on write, and freeing a question of reachability, which
  the tree's single ownership answers for nothing.
- **General recursion with a measure the author writes**, its decrease proved by a solver at build. It admits more
  functions and states no bound on their activations; the cost of a recursion that calls twice on one child is
  exponential in the depth, and ADR 0118 needs a term.
- **Size-change termination** (Lee, Jones and Ben-Amram, "The size-change principle for program termination", POPL
  2001). Decidable and more general than D8's rule, and Agda's termination checker, which descends from foetus (Abel
  and Altenkirch 2002), now judges size-change matrices; but it is as silent on cost as the previous, and its
  generality buys functions that are written better as a fold with a state (D8's trie match).
- **Native recursion with a stack bound computed from the frame.** The platform's stack is not the program's to size
  on every lane, and a deep tree on a microcontroller would overflow it where an explicit stack of the build's size
  does not.
- **A tree written by hand as a map of numbered nodes** (ADR 0108, today). Possible, and nothing tells the build that
  it is a tree: no invariant, no structural recursion, no canonical bytes, no model.
- **The `bounded-collection` kind.** Its slots are addressed by handles a document or a host holds; a member's nodes
  are reached only through their parent, and a handle held apart from the tree would be an index leaving the value
  (D5).
- **A preorder sequential layout as the arena** (the nodes kept in preorder, each with its degree, Knuth section
  2.3.3). It is the model's form (D11), where it makes equality of states equality of values; in a lane it finds no
  child by its key in constant time and makes every write that moves a sub-tree renumber what follows it.
- **Children reserved in each node at their full fan-out** (an index per child position, a map's slots and index cells
  per node). A node of fan-out `F` then reserves `F` cells, and a value `B · F`, where at most `B - 1` children exist;
  a map of children in each node needs a seed of its own, which nothing inside a node has. The first-child,
  next-sibling threading of D5 reserves two links per node.
- **A pool of slots per member, each sized for its member.** The node bound is one bound over every member, and any
  division of it among them is a value, so each member's pool would have to hold `B` slots and the pools together take
  more than one pool of the largest node (D5). A pool per size class pays where each class has a bound of its own; a
  type states none. A downstream consumer's review asked for it, and it is declined for this reason (Amended on
  acceptance).

## Order of work

Each step lands with a test that fails without it. The theorems and the model come before the lanes, as ADR 0151's
order puts them, and no lane waits on anything but the steps before it and, for the replay of its runs against the
model alone, step 5b.

1. This record, accepted, and the application of D1 in `SCE_FORGE.md` §10, which land together.
2. The kind's reader and checks (`sce-build/src/forge/kind_catalog.rs`, and `schemas/sce-forge-ext.xsd`: `kindType`'s
   `inductive`, `<sce:import>`'s `member`, `<sce:record>`, `<sce:union>`, `<sce:function>`, `<sce:measure>`, the
   `outcome` of an `<sce:assign>` into an `inout` value), the type's bracket with its depth bound and their refusals,
   every code of D14 a document alone decides with its appendix row, and the model generator's `Construct::Inductive`:
   `sce-build/tests/an_inductive_document_is_read_as_one_recursive_scope.rs`. Red without it: `sce:kind="inductive"`
   is an unknown kind, `record:Node[8]` is refused as a bracket on a record, and `sce-codegen model` names no
   inductive variable among a fixture's refusals. Each refusal has a case that must be refused by its code.
3. The sizes, the depth and the JSON Schema: `sce-build/tests/an_inductive_type_is_sized_by_its_node_bound.rs`, which
   enumerates every term of at most 7 nodes of three fixture documents (a self-recursive record, a recursive union
   with a list of children, two members recursing through each other) and holds `J`, `C` and `depth` to the largest of
   them, and holds the computed sizes to closed forms where CBOR's heads widen (a star of 24 leaves, a star of 256, a
   chain of 300). Red without it: the folds refuse the type as one that holds itself.
4. The Rust arena with T1a proved: `backends/rust/portable-bytes/src/arena.rs` and the slot pool and free list it
   shares with `BoundedMap`, each in `docs/formal/proofs.json` with mutants that must each fail its proof (a freed
   sub-tree's slots not returned; a replace that frees the target before reading a source that contains it; a sibling
   link set one way only; a map child left out of the index), and `backends/rust/portable-bytes/tests/arena.rs` with
   `a_replace_answers_full_exactly_when_the_result_passes_the_bound`, `two_arenas_of_one_term_write_the_same_bytes`
   and `a_child_removed_from_a_sequence_keeps_its_siblings_in_order`.
5. The model, in two parts.
   - **5a**, before every lane: `docs/formal/Inductive.tla` (`Canonical`, `Nodes`, places, the replacement, the fold
     of a D8 cycle), `CanonicalIsOneToOne` proved in TLAPS and checked by TLC at small bounds, each operator against
     its term-level definition, each fixture's D8 translation against a `RECURSIVE` reference, entered in
     `docs/formal/models.json` with a configuration per operator taken out that TLC must find, and registered in
     `SCE_WIRE_CONTRACTS.md`. Red without it: the configuration that takes `Canonical`'s preorder clause out must make
     TLC find two states of one value.
   - **5b**, when ADR 0156 D3 reaches the typed values of `sce-static`: the generator's translation (D11). Red without
     it: `sce-codegen model` refuses a fixture with an inductive variable as `Construct::Inductive`.
6. The Rust machine: variables, places, `nodes`, walks, writes in place by their three ways, puts into maps of
   members, the wire, the lift as a streaming reader (`backends/rust/runtime/src/event_payload.rs`, not `json.rs`'s
   `parse`), the saved state (`schemas/sce-saved-state.v1.schema.json`), the snapshot and the host's view:
   `backends/rust/scenarios/tests/inductive.rs` (`a_restored_tree_equals_the_tree_saved`,
   `a_write_past_the_bound_raises_and_leaves_the_value`,
   `a_subtree_moved_under_its_own_ancestor_fits_where_the_result_fits`,
   `a_subtree_written_under_its_own_descendant_keeps_its_nodes`, `a_lift_refuses_a_tree_only_past_its_depth`,
   `a_replacing_put_answers_full_when_the_nodes_pass_the_bound`), run native and on `wasm32-unknown-unknown`, with
   generated cases of the conformance corpus (`tests/forge/conformance/gen_cases.py`, from a recorded seed) for the
   forms and the refusals; its scenarios are replayed against the model once 5b has landed.
7. Algorithms: views, outputs, local functions, `<sce:measure>`, D8's rule, descents, `inout` values, the explicit
   stack passed through calls, T1b and T2. T1b and T2 wait for ADR 0115 D10's first generated Verus form, which ADR
   0156 records as not started; the rest does not. Tests:
   `sce-build/tests/a_recursion_takes_disjoint_parts_of_its_measure.rs`, with a case for each refusal of D7 and D8 and
   the trie match of D8 admitted; the mutation casefile
   `sce-build/tests/mutations/a_recursion_takes_disjoint_parts.cases`, whose case that drops the prefix check must
   turn the overlapping case red; and `backends/rust/forge-conformance/tests/forge_inductive.rs` with
   `a_trie_match_runs_at_most_one_activation_per_node` (activations counted on a trie of 10 000 nodes),
   `a_recursion_deeper_than_the_native_stack_runs` (a chain of 100 000 nodes on a thread of 64 KiB of stack, red for a
   native recursion; its sizes are not read, so step 3's computation is not made for it) and
   `an_inout_trie_takes_a_path_in_time_of_its_length`.
8. The key path (D10): `sce-build/tests/a_list_of_keys_is_a_key.rs` and the map's index on Rust and C11 fed a
   `ListKey`, in the form ADR 0108's amendment of 2026-11-28 gives a key where it has landed. Red without it:
   `validation/map-key-type` on `map<list<string[8]>[4], uint8>[4]`.
9. C11: `arena.h` with `sce_arena_view_t` and the generated accessors, the lowerings, the stack and the destination
   lent through calls (D8), the scratch arena of a walk that writes (D6), and the wire writer's stack of containers in
   `sce/forge/wire.h` in place of the depth bitmask; `backends/c/tests/unit/forge_arena_test.c`, and the corpus of
   step 6 and the algorithms of step 7 replayed on C11 with Rust's bytes. C11 is held by these differential tests
   alone until the owner approves CBMC for it; the CBMC harnesses of D12 land then.
10. The documents named in "Amends", among them `stdlib/lift/refusal_code.scxml` and
    `stdlib/collection/put_outcome.scxml`, each with its "Amended by: ADR 0164" line, which `sce-build`'s
    `forge::inductive::documents` holds them to, as `forge::journal::documents` holds ADR 0114's. Red without it: a
    document named in "Amends" with no such line.

## Open questions

Each was decided on acceptance, with its reason.

1. **One document per recursive group, rather than cycles of imports.** A mutual definition is one fixed point, grows
   as one and is generated as one unit; the import graph stays acyclic for the manifest and the generator (D1).
2. **Every value states a node bound, and may state a depth bound beside it**, rather than a depth or a fan-out per
   level alone: storage and every cost are functions of nodes, and a depth bound sizes what a node count sizes too
   loosely, a stack (D3).
3. **The wire form is the term**, never the arena: one form per value (ADR 0101), and the representation stays free
   (D4, D5).
4. **A value is a tree, with no sharing**: equality, size and freeing are then the term's, and nothing needs a
   collector (D5, Alternatives rejected).
5. **Recursion is structural on one measure and takes disjoint parts**, rather than general measures or size-change
   termination: it is decided by the build, gives at most `B` activations of each cycle, and every function the
   consumer's rows name is a fold, a fold with a state, or a descent (D8).
6. **No recursion is native**: an explicit stack of the build's size on every lane, as Appendix D's recursions already
   run (D8).
7. **A member type is no key; a list of keys is one**, by a decision of its own (D9, D10).
8. **A GBNF grammar is refused**, a recursive one with the lift's check after it having been weighed and found to let
   a model write what the lift refuses; the JSON Schema states the bounds beside the `$ref` at the use (D4).
9. **The kind is named `inductive`**, what fixes its meaning, the least fixed point, as Coq's keyword names it; `tree`
   names the shape of one value and `data-tree` (§synth-5-H) would read as a kind of data rather than of type
   declarations. The import names a member by `member`, not `type`, which reads as the kind of the import.
10. **Rust and C11**, by the scope rule. CBMC's use for the C11 arena waits for the owner's own words; until then C11
    is held by differential tests alone (D12).
11. **A later revision adds no field that holds a member of its own document**, of the review's two answers (that, or
    a skip of an undeclared key by D4's counting reader): an older reader does not know the type of a key it does not
    declare, so it cannot count that key's nodes, and both its skips, the CBOR codec's and the lift's, are bounded in
    depth (D1).
12. **A run-time path is written into a trie by an `inout` value**, of the review's two answers (that, or the rebuild
    of the whole value as the decided cost): a statechart's places are of a length its text fixes, so without it every
    insertion of a path of run-time length would cost the whole value and a destination of `B` slots; ADR 0108 D6 item
    2 already admits the same form for a map, with the same reason that an algorithm that cannot fail leaves no write
    half done (D7).
13. **The measure is `<sce:measure param>`**, of the review's two answers (that, or one `<sce:decreases>` in two
    shapes): a name read two ways, an expression in a loop and a parameter in a signature, is the ambiguity one
    spelling per fact avoids (D8).
14. **`CanonicalIsOneToOne` is proved in TLAPS**, of the review's two answers (that, or named bounded): it is what
    makes a state of every inductive model a value, so it is worth its upkeep for every size, and it is named bounded
    only until the proof lands (D11).
15. **Child sequences are threaded through the arena** (first child, next sibling), rather than reserved per node, and
    **one pool of the largest node** holds every member, rather than a pool per member: the first answers both
    reviews, the second answers a downstream consumer's review by the textbook, since one bound over every member
    makes per-member pools larger (D5, Alternatives rejected).
16. **The model's node bound is the exploration's**, and every verdict names it: a deployment's bound is beyond what
    TLC explores, and a verdict at a smaller one is of a smaller machine, read for the deployment's under the
    small-scope hypothesis and never as a proof of it; a lane's runs are read at the build's bounds (D11).

## Amended on acceptance

Each amendment of the independent review is listed by the review's number, and was taken as the review recommends, its
choices decided as "Open questions" states; amendment 5 waits on the owner as written there. The points of a
downstream consumer's review, which read the draft as its user, follow as C1 to C5.

1. D4, D9 and D11: a map of children keeps its put order as part of the term, its saved form, snapshot, canonical form
   and walks; ADR 0151 D2 compares it by key order, and calls equal two terms that differ in put order alone.
2. D6, D12 and steps 4 and 6: a write is stated on terms, and `replace` takes three ways (a source inside the place
   moved, a source containing it copied less the place and the old sub-tree moved in, a disjoint one freed then
   copied); a mutant and a test hold the second.
3. D1: a member's lineage is `<document name>/<member>`; `sce:since` up to the document's revision; a union member's
   revision stays its tag enum's; a later revision adds no field holding a member of its own document (Open questions
   11); Amends gains ADR 0044, ADR 0112 D5 and ADR 0101 section 3.2.
4. D4, D13, Scope and steps 6 and 9: `depth(union:U[B])` is computed; a lift of an inductive type is a streaming
   reader sized by it, not `json.rs`'s `parse`; C11's wire writer holds a stack of containers in place of `wire.h`'s
   depth bitmask.
5. D12, Status and step 9: CBMC for the C11 arena waits for the owner's own words, which the main session is asking
   for; until then C11 is held by differential tests alone, and the C11 row is named bounded at `B <= 8`.
6. D5, D14, Consequences and Alternatives rejected: child sequences are threaded first child, next sibling, linked
   both ways; a map child holds its key; one index of `2B` cells and one seed per value; storage is `B` times one slot
   without sequences, and the index.
7. D5 and D7: one index width per document and build; C11's bound-erased `sce_arena_view_t`.
8. D4, D12 and steps 3 and 7: sizes are computed once per member up to the largest bound used where a size is read,
   and not for variables and algorithms alone; closed-form fixtures where CBOR's heads widen.
9. D8, D14 and Consequences: bounds are per cycle; a body's cost includes the cycles it calls; a stack's capacity is
   summed over the cycles live together.
10. D11 and D12: each D8 cycle is translated as `ApaFoldSeqLeft` with the frames in its accumulator, checked by TLC
    against a `RECURSIVE` reference; never `ApaFoldSet`; `Free` and the members are variants of one type.
11. D11 and D12: `Canonical` is defined and is `TypeOK`; `CanonicalIsOneToOne` is proved in TLAPS and named bounded
    until it is (Open questions 14).
12. D12 and step 7: T1 is split into T1a (the arena, proved once, with `BoundedMap`'s shared pool) and T1b (each
    document's codec, as T2); T2 checks the defunctionalization and waits for ADR 0115 D10's generated form.
13. D11: the model reads a codec's refusal past the bound in `Refuses_C`, `tooManyNodes`, a descent bounded by `Nodes`
    and a build's full as `error.execution`; `Inductive.tla` joins ADR 0156 D2's surface.
14. D11 and Order of work: step 2 adds `Construct::Inductive`; step 5 splits into 5a (the module, before the lanes)
    and 5b (the generator's translation, when ADR 0156 D3 reaches typed values); only step 6's replay waits for 5b.
15. D6 and Amends: ADR 0133 D1 gains three items: a whole store from a bound at most the place's is total; a remove
    and a clear inside a value are total; a store into a smaller bound is checked.
16. D6 and Amends: a replacing put may answer `full` on a value's nodes; ADR 0108 D4 is amended; documents check
    `nodes(...)` beside `len(...)`; `put_outcome.scxml`'s comment says so.
17. D7, D9 and Consequences: an algorithm that cannot fail writes an inductive value `inout`, at a descent's cursor,
    each node-adding write naming an outcome (Open questions 12).
18. D7, D8 and Amends: a descent's local is a view of an optional member, set to a strict child or exiting on every
    path; it is exempt from `algorithm/while-unbounded` (§synth-5-A amended); a view local is no path of D8.
19. D8: the stack and a destination pass through intermediate calls, decided over an algorithm's calls as a whole, as
    ADR 0108 D8 decides a seed.
20. D6, D14 and step 9: a walk that writes copies into a scratch arena of `B` slots, a scratch role on C11.
21. D10 (new) and step 8: a list of keys is a key by a decision of its own; equality by length and elements, order ADR
    0151 D2's; its form injective and self-delimiting, aligned with ADR 0108's amendment of 2026-11-28; `ListKey`
    beside `BoundedList<u8, N>`'s impl.
22. Related and Not in scope: ADR 0163, ADR 0165 and ADR 0166 are named, with what waits on ADR 0163 and the regular
    recursion ADR 0166 admits.
23. D8 and Amends: the measure is `<sce:measure param>`, apart from ADR 0115 D10's `<sce:decreases>` (Open questions
    13).
24. Amends, D4 and step 10: `SCE_WIRE_CONTRACTS.md`, `sce-forge-ext.xsd`, the saved-state schema, `kind_catalog.rs`,
    ADR 0065 section 2, ADR 0087 section 2.1 item 4 and `refusal_code.scxml` are named; ADR 0118 moves to Related;
    `tooManyNodes` is 16; step 10's test is `forge::inductive::documents`.
25. The textbook structure and Sources: RFC 8610 Appendix C; TAPL chapter 20 and PFPL chapter 15; continuation-passing
    then defunctionalization; paramorphism, downward accumulation and mutumorphism; Knuth 1977 for the least value;
    foetus and size-change matrices behind Agda's checker.
26. D4: a member's `$defs` key is `<document>/<member>` escaped as a JSON Pointer; the bounds stand beside the `$ref`
    at the use; a recursive GBNF with the lift's check was weighed and is refused (Open questions 8).
27. D5, D13 and D14: C11's host reads through generated accessors and an opaque reference; refusing the Interpreter
    leaves ADR 0115 D15's agreement to two parties; a member held with its own bracket multiplies storage; a C11 event
    slot is sized by `J`.
28. D1, D5 and Alternatives rejected: the separate kind is kept, with its reason; `bounded-collection` and a preorder
    sequential layout are rejected; one slot pool and free list is shared with `BoundedMap`; the import names its
    member by `member`.

From a downstream consumer's review:

- C1. Child sequences threaded first child, next sibling (Knuth section 2.3.2): taken, as amendment 6. Slots sized per
  member rather than by the largest member: declined, by the textbook, since one node bound over every member makes a
  pool per member hold `B` slots of its own; one pool of the largest node, with one free list proved once, is kept
  (D5, Alternatives rejected, Open questions 15).
- C2. An optional depth bound beside the node bound, which caps a recursion's stack at `D + 1` frames and which a
  reader refuses past as `tooDeep`: taken (D3, D4, D6, D8, D14).
- C3. The model's node bound is an exploration constant, which every verdict names: taken, with a lane's runs read at
  the build's bounds (D11, Open questions 16).
- C4. The C11 arena, CBMC, `no_std` with `Inline` storage and `Heap` grown up to `B` were accepted as drafted; CBMC
  stays subject to amendment 5.
- C5. A snapshot of a value shared across execution contexts (read-copy-update) is the seam's, behind the door, and
  not the value's: stated in Not in scope.

## Sources

Goguen, Thatcher, Wagner and Wright 1977 ("Initial algebra semantics and continuous algebras", Journal of the ACM
24(1)); Hoare 1975 ("Recursive data structures", International Journal of Computer and Information Sciences 4(2));
Hoare 1972 ("Proof of correctness of data representations", Acta Informatica 1(4)); Coquand and Paulin 1990
("Inductively defined types", COLOG-88, LNCS 417); Giménez 1995 ("Codifying guarded definitions with recursive
schemes", TYPES 1994, LNCS 996); Abel and Altenkirch 2002 ("A predicative analysis of structural recursion", Journal
of Functional Programming 12(1)); Nordström 1988 ("Terminating general recursion", BIT 28(3)); Meijer, Fokkinga and
Paterson 1991 ("Functional programming with bananas, lenses, envelopes and barbed wire", FPCA, LNCS 523); Reynolds
1972 ("Definitional interpreters for higher-order programming languages", ACM Annual Conference); Danvy and Nielsen
2001 ("Defunctionalization at work", PPDP); Lee, Jones and Ben-Amram 2001 ("The size-change principle for program
termination", POPL); Thompson 1968 ("Regular expression search algorithm", Communications of the ACM 11(6)); Wadler
1990 ("Linear types can change the world!", IFIP TC 2 Working Conference on Programming Concepts and Methods); Hanson
1990 ("Fast allocation and deallocation of memory based on object lifetimes", Software: Practice and Experience
20(1)); Tofte and Talpin 1997 ("Region-based memory management", Information and Computation 132(2)); Bellman 1957
(*Dynamic Programming*, Princeton University Press); Pnueli, Siegel and Singerman 1998 ("Translation validation",
TACAS); Lattuada et al. 2023 ("Verus: Verifying Rust Programs using Linear Ghost Types", OOPSLA); Pierce 2002 (*Types
and Programming Languages*, MIT Press, chapter 20); Harper 2016 (*Practical Foundations for Programming Languages*,
2nd edition, Cambridge University Press, chapter 15); Cormen, Leiserson, Rivest and Stein 2009 (*Introduction to
Algorithms*, 3rd edition, MIT Press, sections 10.3 and 10.4); Knuth 1997 (*The Art of Computer Programming*, volume 1,
3rd edition, Addison-Wesley, sections 2.2.3, 2.3.1, 2.3.2 and 2.3.3); Meertens 1992 ("Paramorphisms", Formal Aspects
of Computing 4(5)); Gibbons 1993 ("Upwards and downwards accumulations on trees", Mathematics of Program Construction
1992, LNCS 669); Fokkinga 1990 ("Tupling and mutumorphisms", The Squiggolist 1(4)); Knuth 1977 ("A generalization of
Dijkstra's algorithm", Information Processing Letters 6(1)); Mycroft 1984 ("Polymorphic type schemes and recursive
definitions", International Symposium on Programming, LNCS 167); Wilson, Johnstone, Neely and Boles 1995 ("Dynamic
storage allocation: a survey and critical review", IWMM, LNCS 986); Jackson 2006 (*Software Abstractions*, MIT Press);
RFC 8610 (CDDL, Appendix C), RFC 8949 (CBOR), JSON Schema 2020-12 (Core, section 8.2.3.1, `$ref`), ITU-T X.680
(ASN.1).

What was checked, and what was not:

- **Checked, in this repository, for this record:** the passages of ADR 0044 (its header and decisions 1 to 3 as ADR
  0112 D5 amends them), ADR 0086 (sections 1, 1.5, 2.1, 15), ADR 0087 (section 2.1 item 4 and section 2.3's codes),
  ADR 0092 (sections 1, 2, 5 to 9), ADR 0101 (sections 1 to 4), ADR 0108 (D1 to D8, and its amendment of 2026-11-28 on
  its landing branch), ADR 0115 (the levels, D7 to D10), ADR 0133 (D1), ADR 0151 (whole), ADR 0152 (whole), ADR 0154
  (D1), ADR 0156 (the Context, D1 to D5, D13, D14), the Proposed ADR 0163, ADR 0165 and ADR 0166 on their branches
  (their statements this record meets), and SCE_FORGE.md §3.2, §4.12 and §10 that the decisions amend or follow; and
  the code the Context and the decisions cite (`value_type.rs`'s grammar, its refusal of a type that holds itself and
  of a list as a key; `model.rs`'s `UnionModel` and `AlgorithmStmt::Call`; `static_js.rs`'s refusal of an algorithm
  that imports itself; `map.h`'s slots, free list and host view, and `map.rs`'s `MapKey` impls; `document_model.rs`'s
  refusal of `sce-static`; `json.rs`'s `parse`, `event_payload.rs`'s `lift_too_deep` and `wire.h`'s
  `SCE_FORGE_WIRE_MAX_DEPTH`; `refusal_code.scxml`'s numbers; `sce-forge-ext.xsd`'s `<sce:import>`;
  `docs/formal/proofs.json`'s files), and §synth-5-A, §synth-5-F and §synth-5-H of the protocol-synthesis RFC. The
  independent review confirmed Apalache's removal of recursive operators in its release 0.23.1.
- **Not checked against the texts:** every work of the literature above is cited from the texts as known and was not
  read again for this record. The passages the decisions rest on are to be checked in the step that builds each: the
  initial-algebra reading and strict positivity (D2, step 2), the guard condition and the predicative analysis of
  structural recursion (D8, step 7), defunctionalization (D8, step 7), the abstraction-function method (D5, step 4),
  the sequential representation of trees in preorder (D11, step 5), section 8.2.3.1 of JSON Schema's Core and RFC
  8610's Appendix C (D4, step 3), the recursion schemes (D8, step 7), Knuth 1977 (D2, step 2), and the survey of
  storage allocation (D5, step 4).
