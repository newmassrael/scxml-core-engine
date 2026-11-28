# ADR 0164 — A recursive type is declared in one document, a value of it is a tree of at most its bound of nodes held in an arena by index, and a function over it recurses on disjoint parts of its argument

- Status: Proposed (2026-11-28), for the owner's acceptance after an independent review.
- Landed: nothing. No step of "Order of work" is built, and no proof, model or test of it has been run.
- Date: 2026-11-28
- Written under the owner's standing rule (the most long-term correct structure, textbook in its foundations, cost
  ignored) and the owner's scope rule of 2026-11-22: a new facility is built only on the backends a consumer needs,
  Rust by default; its contract is general; what is special to one application stays in the application. At the
  request of a downstream consumer whose protocol logic, moving into SCE documents, holds recursive data in 21 places:
  a configuration document's value tree, a trie of key paths with wildcard chunks, a protocol schema's syntax tree,
  the tree of a message's dissected fields, an allocation policy built of nested combinators, and a hash trie. The
  request is the first reference case and not the scope.
- Amends (each amended document gains an "Amended by: ADR 0164" line in the landing that moves it):
  - ADR 0086 section 1.1 (the grammar: an alias that names a member of an inductive group takes one bracket, its node
    bound, D3), section 1.5 (a type that contains itself within one inductive document is admitted with that bound;
    through the imports of several documents it stays refused, D1), section 2.1 (the size row of an inductive type,
    D4), section 4 (a reader counts nodes, and refuses the one past the bound, D4) and section 7 (the types each
    backend generates: an arena, D5);
  - ADR 0087 section 1.1 (the JSON Schema of an inductive type recurses by `$ref` and states its node bound as an
    annotation; a position that requires its type's GBNF grammar refuses an inductive type, D4) and section 2.3 (a
    refusal code, `tooManyNodes`, D4);
  - ADR 0092 section 1.1 item 3 (a union that holds itself through its cases' records is admitted when the union and
    the records are members of one inductive document, D1) and section 1.1 item 1 (a case of a union member may hold a
    record member of its document, D9);
  - ADR 0101 section 1.1 (a CBOR entry may be of an inductive type), section 3.3 (a value of more nodes than its bound
    is `CborOutOfRange`) and section 4.1 (its size row), D4;
  - ADR 0108 D1 (a bounded list whose element is a key is a key; an inductive type is no key, D9) and D7 (an inductive
    member that holds a map stands in no wire position, as a record that holds one does not, D9);
  - ADR 0115 D7 (the model holds an inductive value as its canonical arena and computes over it by folds, D10) and ADR
    0156 D4 (the generated `TypeOK` of an inductive variable is its canonical arena's well-formedness, D10);
  - ADR 0118 D3 (an arena's storage, a recursion's stack and its activations are terms of the node bound, D8 and D13),
    while ADR 0118 is Proposed;
  - ADR 0151 D2 (the comparison of an inductive value, D4);
  - `SCE_FORGE.md` §3.2 (the kind list), §4 (a section for the kind), §4.12 (an algorithm's local functions, its
    structural recursion and its views of an inductive value, D7 and D8), §8 (the kind summary) and §10 (the
    application of the admission test, D1);
  - `docs/SCE_ACCEPTED_SUBSET.md` §2.15 (a variable of an inductive type, D6) and the appendix (D13's codes);
  - `docs/spec/synth/rfc-sce-protocol-synthesis.md` §synth-5-H ("Recursive and tree data kinds", deferred there),
    which this record decides.
- Scope: `sce-build` (the kind, the type, its checks and sizes, the check of a recursion, the model's translation);
  the Rust lane, native, `no_std` and `wasm32-unknown-unknown` (`sce-portable-bytes`, the machine and algorithm
  lowerings); the C11 lane (`sce/forge/arena.h`, its lowerings). Every other backend, the Interpreter and the TTCN-3
  export refuse an inductive type by name (D12).
- Related: ADR 0044 (the revisions of a schema), ADR 0050 and ADR 0086 (a bound by name, part of a type), ADR 0055
  (optional types, and narrowing), ADR 0077 (a decode refuses before it stores), ADR 0082 (an algorithm writes into
  storage its caller holds), ADR 0087 (a value of a schema asked of a model), ADR 0092 (unions), ADR 0101 (one
  deterministic CBOR form per type), ADR 0108 (maps), ADR 0115 (the levels of formal judgement), ADR 0118 (cost from
  the plan), ADR 0133 (totality where the build proves it), ADR 0149 (an automaton over a text), ADR 0151 (views and
  their total comparison), ADR 0154 (a map's storage lent by its host), ADR 0156 (models and proof obligations
  generated from documents).

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
  2002, chapters 20 and 21; Harper, *Practical Foundations for Programming Languages*, Cambridge University Press
  2016, the chapter on inductive types).
- **A definition is admissible when the type occurs only strictly positively** in its constructors' arguments, never
  to the left of a function arrow (Coquand and Paulin, "Inductively defined types", COLOG-88, LNCS 417, 1990).
  Products, sums, finite sequences, optional values and finite maps over a fixed key type are strictly positive
  functors. A type is inhabited by a finite term exactly when some constructor's arguments are all inhabited, which is
  a least fixed point computed over the declarations.
- **Recursion over such a type terminates when every recursive call is on a strict subterm of the argument**
  (structural recursion: Giménez, "Codifying guarded definitions with recursive schemes", TYPES 1994, LNCS 996, 1995,
  the guard condition of Coq; Abel and Altenkirch, "A predicative analysis of structural recursion", Journal of
  Functional Programming 2002, the checker of Agda). It is the special case of well-founded recursion whose measure is
  the size of the argument (Nordström, "Terminating general recursion", BIT 1988), the measure Verus asks of a
  recursive specification function by its `decreases` clause (Lattuada et al., "Verus: Verifying Rust Programs using
  Linear Ghost Types", OOPSLA 2023). A recursion that visits each node once is a fold, a catamorphism (Meijer,
  Fokkinga and Paterson, "Functional programming with bananas, lenses, envelopes and barbed wire", FPCA 1991), whose
  cost is the number of nodes times the cost of one step.
- **A recursion is run without a call stack by making its continuation data**: each frame a record of what remains to
  do, held on an explicit stack (defunctionalization: Reynolds, "Definitional interpreters for higher-order
  programming languages", ACM Annual Conference 1972; Danvy and Nielsen, "Defunctionalization at work", PPDP 2001).
  The stack is then storage of a size the program states, not the platform's.
- **Objects linked by pointers are represented without pointers by indices into arrays**, with a free list of the
  unused slots (Cormen, Leiserson, Rivest and Stein, *Introduction to Algorithms*, 3rd edition, MIT Press 2009,
  section 10.3, "Implementing pointers and objects"; Knuth, *The Art of Computer Programming*, volume 1, 3rd edition,
  Addison-Wesley 1997, section 2.2.3, the list of available space). Allocating the objects of one value in one region
  freed as a whole is arena or region allocation (Hanson, "Fast allocation and deallocation of memory based on object
  lifetimes", Software: Practice and Experience 1990; Tofte and Talpin, "Region-based memory management", Information
  and Computation 1997). A tree whose every node has one owner needs no collector: freeing a subtree frees exactly
  what no other node reaches, which is the discipline of linear and unique ownership (Wadler, "Linear types can change
  the world!", IFIP TC 2 Working Conference on Programming Concepts and Methods, 1990).
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
  section 3), CBOR nests arrays and maps to any depth (RFC 8949 section 3), a JSON Schema refers to itself by `$ref`
  (JSON Schema 2020-12, Core section 8.2.3), and ASN.1 admits a type that refers to itself through `SEQUENCE OF` and
  `CHOICE` (ITU-T X.680).

## Decision

**A recursive type is a member of an inductive group: one document of the new kind `inductive`, which declares records
and unions that may name each other and the types it imports. A use of a member outside its document states one bound,
the most nodes a value holds, and a value is a finite tree of at most that many nodes. Its bytes, its equality and its
order are the tree's, and every lane holds it in an arena of that many slots, a child named by its slot's index, with
no pointer. A function over it is a local function of one algorithm document and recurses only on disjoint proper
parts of one argument, so it ends, visits each node at most once, and runs on an explicit stack the build sizes. The
model holds the tree in its canonical preorder form and computes over it by folds. Rust and C11 serve it; every other
lane refuses it by name.**

### D1. A recursive type is declared with the types it recurses through, in one document of the kind `inductive`

`sce:kind="inductive"` is a document of type declarations that form one recursive scope: `<sce:record name>` elements,
each with a `<datamodel>` of fields written as an event schema's are (ADR 0086 section 1), and `<sce:union name>`
elements, each with a tag and cases written as a union document's are (ADR 0092 section 1.1). Inside the document a
member is named `record:<name>` or `union:<name>`, with no import; anything else it names (an enum, an event schema, a
union, another inductive document's member) it imports, and the import graph stays acyclic. It is not inline-eligible
(`SCE_FORGE.md` §3.2): it is declared as a document and imported, one member at a time,
`<sce:import kind="inductive" src="..." type="<member>" as="<alias>"/>`.

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
<sce:import kind="inductive" src="config_value.scxml" type="Value" as="Config"/>
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

- **Why one document.** A mutually recursive definition is one fixed point and is declared as one block, as ML's
  `datatype ... and ...` and Coq's `Inductive ... with ...` are; so are its revisions, since a change to one member
  changes the type of every member. Admitting cycles of imports between event-schema and union documents instead would
  make the import graph's topological order, which the manifest and the generator read (`manifest.rs`), a partial one,
  and would let one member's revision move apart from the others'.
- **A member's grammar is its kind's.** A record member's fields are an event schema's fields, any type of the
  grammar; a union member's tag is a closed enum the document imports and its cases are one to one with the enum's
  variants, each holding a record (a record member of this document or an imported event schema) or nothing, with
  every rule of ADR 0092 section 1.1 (`validation/union-cases`, `validation/union-tag-collision`). Nothing in a member
  is written in a way a record or a union is not.
- **Revisions.** The document declares one revision, which the unit of every member carries: a record member grows as
  an event schema does (ADR 0086 section 10, ADR 0044) and a union member as a union does (ADR 0092 section 8). Two
  builds' members are revisions of one when their documents are revisions of one and each member is a revision of its
  namesake, member by member; anything else parts them, and `schema_revision::divergence` names the member, as
  `mesh/event-schema-mismatch` names a field.

**Admission** (`SCE_FORGE.md` §10), to be added there as an application when this record is accepted:

| Axis | Pass/Fail | Reason |
|---|---|---|
| Domain alignment | PASS | Deterministic data layout: a value is a finite term of bounded size whose bytes are one deterministic CBOR form (ADR 0101), equal on every lane whatever slots hold it, and its storage is an arena of slots the build sizes from a named bound |
| Futamura projection compatibility | PASS | The members, their fields, the bounds, the slot layout, the size of the largest value and the recursion of every function over the type are build-time; only the shape and contents of a value are runtime. A function over a tree is compiled to one loop over an explicit stack; nothing is interpreted at run time |
| Cross-domain reuse | PASS | Network protocols (a trie of key paths matched against patterns with wildcards; the syntax tree of a schema language; the dissected fields of a message), GUI and scene state (a widget tree, a scene graph without sharing), robotics control (a behaviour tree), embedded telemetry and configuration (a configuration document's value tree), build and CI systems (the expression tree of a build rule's condition), game networking (a quadtree of an area of interest) |

Passed 3-of-3. Three alternatives were ruled out before admission: a self reference on a codec's field (§synth-5-H's
sketch), which serves only self-recursion and only inside a codec; cycles of imports between event-schema and union
documents (above); and a kind per shape (a `tree` kind, a `trie` kind), each a single shape of the one construct this
record admits.

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
  member's **least value**, the fewest nodes a term rooted at it holds.
- **Names.** A member's name is unique in its document and differs from every alias the document imports:
  `inductive/member-name-taken`, naming both declarations.

### D3. A use of a member states its node bound, and every other constructor composes with it

The grammar of ADR 0086 section 1.1 gains no constructor. An alias that names a member of an inductive document takes
exactly one bracket, its **node bound**:

```text
T ::= ... | record:<alias>[B] | union:<alias>[B]     -- <alias> names a member of an inductive document
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

The bound counts nodes, not depth: a value's storage and the cost of every walk of it are functions of its nodes, and
a depth bound with a fan-out of `F` admits `F^depth` nodes. The fan-out of one node is its own lists' and maps' bounds
(`config_fanout` above), each a bound of its own type, and a node is held to both.

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
  the term (D11, T1).
- **Equality and order.** Two values are equal when their terms are. Where a total comparison is read (ADR 0151 D2), a
  value compares as the record or the union its root node is, each child compared by this rule at its place; a term is
  finite, so the comparison ends. A statechart does not compare two inductive values (`===` is refused, as for every
  composed value, ADR 0086 section 6.1).
- **A reader counts nodes and refuses the node past the bound before it stores it.** Every reader of the wire, the
  saved state and a codec reads a term with an explicit stack of at most `B` frames and counts each node it begins;
  the node past `B` is refused before any of it is stored (ADR 0077 section 2): `tooManyNodes` in the refusal record
  of a delivery or a lift, a code of ADR 0087 section 2.3 numbered after the table's last, with `bound` the node
  bound; `CborOutOfRange` for a CBOR codec's decode, the class ADR 0101 section 3.3 gives a list past its bound. A
  term's depth is at most its nodes, so a reader needs no other depth limit for it.
- **Sizes, exact.** `J(T)` and `C(T)` of `union:U[B]` and `record:R[B]` are the largest encodings of a term of at most
  `B` nodes, computed by dynamic programming over the node count. For each member `m` and each `k` from 1 to `B`,
  `S_m(k)` is the largest encoding of a term rooted at `m` of at most `k` nodes (none below `m`'s least value): the
  bytes of `m`'s node that do not depend on its children, at their largest, and the best division of the remaining
  `k - 1` nodes among its child positions, each position's best for `j` nodes being: a member field, `S(j)`; an
  optional, its absent form for `j = 0` and `S(j)` otherwise; a list or a map of `n` children, its head `h(n)`, each
  entry's key at its largest for a map, and the best division of `j` among `n` children, for every `n` up to its bound
  and at least its lower count; a union member, the largest over its cases. The division is a max-plus convolution, so
  the computation takes time of the order of `F · B^2` for each child position of fan-out `F`, once per build; it is
  exact, so a maximal value encodes to exactly `C(T)` (ADR 0101 section 4.2) and `J(T)` (ADR 0086 section 2.3), and a
  C11 slot sized by `J` (ADR 0086 section 8) is no larger than a value needs. A size past `2^64 - 1` bytes is refused
  as every such size is (ADR 0101 section 4.4).
- **Grammars.** The JSON Schema of a member is a `$defs` entry per member of its document, its children by `$ref`, and
  the use's node bound as the annotation `x-sce-max-nodes`, beside the `x-sce-max-bytes` of a text, which a reader
  enforces and a validator of JSON Schema does not; ADR 0087 section 1.1's sentence that a text in the schema's
  language is one the reader accepts is amended to hold with that annotation read. A GBNF grammar cannot state a node
  count but by unfolding the type into rules for each count, a number of rules of the order of `B^3`; a position that
  requires its type's GBNF grammar (ADR 0087 section 1.1) refuses an inductive type as `inductive/no-exact-grammar`,
  naming the position.

### D5. A value is held in an arena of its bound, a child named by the index of its slot

- **The representation.** A value of `union:U[B]` is an arena: `B` slots, each holding one node or free; the index of
  its root; the number of its nodes; and a free list threaded through the free slots (CLRS section 10.3; Knuth section
  2.2.3). A node holds its member (where the document has more than one), its case (for a union member), every field
  that holds no member, and, for each child position, the index of its child's slot: a field, an optional (0 when
  absent), each element of a list, each value of a map. An index is the smallest unsigned integer of 8, 16 or 32 bits
  that holds `B`; a slot is named one past its place, so 0 names no slot, as a C11 map names its slots (`map.h`).
- **The invariant** `Wf`: the root's slot holds a node; every child index of a node names a slot that holds a node;
  every node but the root is the child of exactly one position of one node and the root of none; following children
  from the root reaches every node and never the same node twice; the number of nodes is the slots that hold one, at
  most `B`; and the free list holds every other slot once. So a value is a tree: no node is shared, no node is lost,
  and nothing cycles.
- **The abstraction function** `Term(a)` reads the term from the root. Every operation is stated on terms and shown to
  commute with `Term` (Hoare 1972; D11, T1), so what a value means does not depend on which slots hold which nodes,
  and no operation's result can show it.
- **No index leaves the value**, as no slot of a map does (ADR 0108 D2). A document reads and writes through places
  (D6, D7). A host reads through a view that borrows the value and exposes typed accessors per member and none of its
  indices, and builds a value through a builder that answers `Full` when the term would pass `B`.
- **Freeing is exact.** A node leaves the value only with the subtree below it, which no other node reaches (`Wf`),
  and its slots go back to the free list in the same operation. No collector runs, and a value never holds an
  unreachable node.
- **One shape on Rust and C11.** Rust holds it as `sce_portable_bytes::arena::Arena<N, B, S>`, `N` the node type the
  generator writes for the document (an `enum` with a variant per member, or the member's `struct` where there is
  one), and `S` the storage, chosen by type as a map's is (ADR 0108 D4 "Storage"): `Inline`, the slots in the value,
  which never allocates, and `Heap`, grown up to `B` where the runtime has an allocator; a lent storage follows ADR
  0154 on its acceptance, by the same reason, with no index cells, since an arena has no hash index. C11 holds it as
  the type `SCE_ARENA_DECLARE` declares in `backends/c/forge-runtime/include/sce/forge/arena.h` for the document's
  node type and the bound, reserved whole where the machine holds it (ADR 0082), the node type a `struct` of a member
  tag and a `union` of the members' layouts. The structure is the same on both lanes: the slots, an index of one
  width, the free list and `Wf`. Neither lane's memory layout is a contract; the bytes the two lanes must agree on are
  D4's forms, which are of the term.

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
  such a body.
- **A write in place.** `<assign location="<place>">` replaces the sub-value at a place, and the list and map
  statements (`<sce:append>`, `<sce:clear>`, `<sce:put>`, `<sce:remove>`) act on a list or a map inside a value
  through a nested place (ADR 0108 D12), the list's or the map's own bound judged as it is. A write **succeeds exactly
  when the value it leaves has at most `B` nodes**, and otherwise leaves the value as it was and raises
  `error.execution`, as an append to a full list does. Success is a property of the result, not of how the write is
  carried out: the one routine every write is lowered to (`Arena::replace` and its C11 twin) frees the nodes of the
  replaced sub-value before it takes slots for the new one, and moves rather than copies a new sub-value that lies
  inside the replaced one, so it never answers full for a result that fits (D11, T1). A `<sce:put>` whose map value is
  a member names its `outcome` as any put does (ADR 0108 D4), and answers `full` for either bound, the map's or the
  value's nodes, both being storage exhausted.
- **Totality.** A write that adds nodes stays a statement that can fail (ADR 0133 D1 item 6: "a list or a map an
  algorithm returns, a write of a map at a key: each stays checked"); a read of a place stays a read that can miss
  where it reads an index or a key (item 3). The build proves nothing more, and a document that must not fail tests
  `nodes(...)` against `_x.limits.<name>` before it writes, as ADR 0108 D4's document tests `len(faces)`.
- **Saved state, snapshot and replay.** A variable is saved, shown in the snapshot and written in a scenario as its
  term in D4's JSON form; the saved shape names the document, the member and the bound, so a change of a member moves
  the shape as a change of a record does (ADR 0045). A restore reads the term with D4's reader and refuses one past
  the bound.
- **An event's field** may be of an inductive type, read and sent as D4 says; a member that holds a map stands in no
  wire position (D9).

### D7. An algorithm reads a value through a view and builds one into storage its caller holds

- **A parameter is a view** of its argument, as every parameter is (SCE_FORGE.md §4.12): of an inductive type it
  writes no bracket and holds the argument's bound, and is lowered to a borrow of the argument's arena and the index
  of the sub-value's root (Rust `ArenaRef<'_, N, M>`, C11 a `const` pointer to the arena and an index), so a call
  copies nothing. A place read from a view is a view.
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
- **A descent is a loop.** A `<sce:while>` whose condition reads a view local and whose body sets that local to a
  child of itself, and to nothing else, runs at most the view's node count of passes: the build counts it so without a
  `max-iter`, by the rule D8 applies to a call. A walk down one path of a trie is written so.

### D8. A function recurses only on disjoint proper parts of one argument, and runs on an explicit stack

- **The recursive scope is one document.** An algorithm document may declare, beside its entry, local functions,
  `<sce:function name>`, each with a `<sce:signature>` and a `<sce:body>` as the entry has, callable from the entry
  and from each other and from nowhere else. The entry and its local functions may call each other in cycles; a cycle
  of calls through imports stays an import cycle and stays refused (`manifest/circular-dependency`). A function on a
  cycle of calls declares its **measure**, `<sce:decreases by="<param>"/>` in its signature, a parameter of a member
  type; every function of one cycle measures a member of one inductive document.
- **The rule.** A call from a function on a cycle to a function of the same cycle is **structural** when the argument
  it passes at the callee's measure is a path from the caller's measure parameter of one step or more, each step a
  field of a member, an element of a list or a value of a map taken as the item of a `<sce:foreach>` (or at an index),
  through union members whose case is proven, ending at a member value: a node strictly below the caller's argument.
  Any other argument is `algorithm/recursion-not-structural`, naming the call and the measure. Two more conditions
  make the calls of one activation take **disjoint** subtrees, each at most once:
  - every `<sce:foreach>` that encloses the call inside the function contributes exactly one element step to the path,
    its own item, and no `<sce:while>` encloses it, so each run of the call site takes another node;
  - no two call sites that can both run in one activation take paths one of which is a prefix of the other, two sites
    being unable to both run only when they lie in different branches of one `<sce:if>`. In the comparison an element
    step, a loop's item or an index, matches every element step of the same list or map.

  Either broken is `algorithm/recursion-overlaps`, naming both sites, or the site and the loop. The other parameters
  of a structural call are free: they carry what the walk computes on its way down.
- **What it proves.** Each activation's calls take disjoint proper subtrees of its own argument, so every node of the
  first argument is the argument of at most one activation of the whole run, by induction on depth. A call into a
  cycle from outside it, with `v` at the measure, therefore makes at most `nodes(v)` activations, at most `B` of its
  bound, and is at most `height(v) + 1` frames deep: it ends, and its cost is the nodes times one body's cost. It is a
  fold in the sense of Meijer, Fokkinga and Paterson, generalized to read a node's descendants where it does not call
  on them. The measure is the size of the argument, which falls at every call, so it is also Nordström's well-founded
  recursion and Verus's `decreases` (D11, T2).
- **No recursion is native.** A function cycle is compiled on every lane to one loop over an explicit stack of frames,
  each frame the locals of one activation and the point it resumes at (Reynolds 1972; Danvy and Nielsen 2001), as the
  build already runs Appendix D's recursions over a stack of the document's capacity (`ARCHITECTURE.md`). The stack's
  capacity is the node bound of the argument the entry is called with, known where it is called: storage the caller
  holds, as an algorithm's output is (ADR 0082), taken on the heap up to the bound where the Rust runtime has an
  allocator, and from the caller's storage on Rust without one and on C11, where a machine that calls holds it as a
  scratch role (ADR 0086 section 8.5). A recursion therefore needs no stack of the platform's beyond one frame of its
  loop, which the small stack of a freestanding C11 core on a microcontroller (ADR 0110) can give whatever the depth
  of the tree.
- **A function on a cycle holds no arena of its own**: a local of an inductive type with a bracket is refused there as
  `algorithm/arena-in-recursive-function`, since every activation would hold one and the frames would hold `B` of
  them. It holds views, and builds into the entry's destination (D7).
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
  <sce:import kind="inductive" src="config_value.scxml" type="Value" as="Config"/>
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
      <sce:decreases by="v"/>
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
- **A map's value may be a member**, inside the document (`literal` above: children by key, each a node) or outside it
  with a bracket (`map<string[32], union:Config[config_nodes=64]>[16]`: each entry a value of its own bound, held in
  its own arena, which Inline storage reserves for each of the map's slots and Heap storage grows as the map is
  filled). A map of children keeps ADR 0108 D2's order, the order its keys were put, which is part of the term and so
  of its bytes, equality and order.
- **A member type is no key** (`validation/map-key-type`, with the reason `NotAKeyWhy::Inductive`): a key's every
  value is bounded, which a member's is only through a bracket, and a key held inside a node of the same document
  would put a node in two places of one term. Nothing asks for a tree as a key; a set of trees is a map keyed by what
  tells them apart.
- **A bounded list whose element is a key is a key** (ADR 0108 D1 amended): `list<string[chunk=64]>[depth=16]` is
  compared by its length and then its elements in order, and its canonical bytes for the index are its length as eight
  little-endian bytes and then each element's, as a record key writes its fields (`map.h`'s feed). D1's reason against
  a list, that a collection has no equality a key has, is true of a set and a map, whose equality is of sets, and not
  of a list, whose equality is exact and elementwise. A list of lists of keys is a key by the same rule; a list whose
  element is no key is none.
- **A trie keyed by its paths.** Where a document reads a table only by whole paths (does this path hold a value, put
  one at it, remove it) or may walk the whole table per question within its bounds, the table is
  `map<list<string[chunk=64]>[depth=16], record:Route>[routes=1024]`, a map keyed by the chunk path, and needs nothing
  of this record but the key rule above. Where it asks for the entries under a prefix, or matches a pattern with
  wildcard chunks against every path, in time that must not grow with the table, it is the trie of D1, walked by D8's
  fold. Both are written in SCE; the document chooses by what it asks.
- **Wire positions.** A member that holds a map, at any depth, stands in no wire position
  (`validation/map-in-wire-position`, ADR 0108 D7), as a record that holds one does not; a trie is held in a variable
  and walked by algorithms, and sent, where it must be, as a value of a type without a map (a list of path and route
  records).
- **ADR 0108 D6 and D12 hold within a value.** A read never copies a sub-value; a loop copies only a value its body
  writes; a nested place inside a value is written in place.

### D10. The model holds a value in its canonical preorder form and computes over it by folds

- **The canonical form.** The model holds a value of `union:U[B]` as
  `[n |-> k, at |-> [i \in 1..B |-> IF i <= k THEN node ELSE Free]]`: its `k` nodes numbered in preorder from 1, the
  root first, each node a record of its member, its case, its fields that hold no member, and its children's numbers;
  the slots past `k` the one value `Free`. The preorder numbering is a function of the term (children in the order
  their positions are declared, a list's in its order, a map's in ADR 0108 D2's), and a term is read back from it, so
  two equal terms have one form and TLC's equality of states is equality of values (Knuth, section 2.3.3; D11).
- **Every operation is a fold, never a recursive operator.** In preorder every node's descendants have greater
  numbers, so a computation over a tree from its leaves up is one pass over `B .. 1`, and one from the root down one
  pass over `1 .. B`. The module `docs/formal/Inductive.tla`, beside `AppendixD.tla` and in its words, writes once,
  over the canonical form: `Wf`, `Nodes`, a place's sub-value, the replacement of a sub-value (the new term numbered
  again by one pass that computes subtree sizes and one that assigns numbers), and the fold a structural function of
  D8 is. A function a document declares is translated to that fold, its body as the step. A fold is evaluated by TLC
  and is the form Apalache asks for in place of a recursive operator, over a value of integers and records of one
  variant type, which its type checker types.
- **What a document's model gains.** A variable of an inductive type in the module's `TypeOK` is `Wf` of its canonical
  form with `n` at most its bound, which is the obligation ADR 0156 D4 generates for every bounded structure; a write
  is the replacement operator and its `error.execution` when the result passes the bound. An exploration (ADR 0115
  D15) names the values an input of an inductive type may take as it names any input's, or a node count below the
  bound from which the values are drawn.
- **Refused by name until it is translated.** Today the model generator refuses a `sce-static` document as a whole
  (`Construct::Datamodel`). When the typed values of `sce-static` are translated (ADR 0156 D3's order), a document
  that holds an inductive value is refused by a construct of its own, `Construct::Inductive`, naming the variable,
  until the translation of this decision lands (Order of work, step 5); a document is never given a model of part of
  itself (ADR 0115 D7).
- **Runs are compared as terms.** A lane's trace writes a value as its term (D4's JSON form), and the model reads it
  into the canonical form, so ADR 0115 D8's acceptance of a run does not depend on how the lane's arena placed its
  nodes.

### D11. The formal level of each claim

Each claim is judged at the level of ADR 0115 that can judge it for every size where one can, and the reason is given;
none rests on a random test alone.

| Claim | Level | How | Why this level |
|---|---|---|---|
| T1. Every operation of the Rust arena (build, read a place, replace, the list and map statements at a place, copy, encode, decode, free) keeps `Wf` and commutes with `Term`; `replace` succeeds exactly when the result has at most `B` nodes and leaves the value as it was otherwise; encode is a function of `Term` and decode its inverse on every encoding of at most `B` nodes | theorem, every `B` (ADR 0115 D13) | Verus, over `backends/rust/portable-bytes/src/arena.rs`, named in `docs/formal/proofs.json` with its mutants | an invariant of a data structure and a refinement (Hoare 1972) over every size; the model holds terms and not arenas, so no model can judge the arena, and a random test samples what the theorem states of every arena |
| T2. Each generated recursion ends within `nodes(v)` activations and never pushes its stack past its capacity, and computes what its document's function computes over the term | theorem per generated algorithm (ADR 0115 D10, ADR 0156 D4) | the generator writes each recursion's Rust with a Verus specification function over the term, `decreases` the argument's size, and the loop's invariant relating stack and term; Verus checks each output | D8's rule is a decision procedure of the build; checking each output keeps the generator and the rule outside the trusted base (Pnueli, Siegel and Singerman 1998; ADR 0156 D11) |
| The canonical form is one-to-one with terms, and each operator of `Inductive.tla` is the operation on terms | proof in this record (D10, by the preorder numbering), and a model check at small bounds | TLC checks `CanonicalIsOneToOne` and each operator against a term-level definition for every term of at most 5 nodes of the fixture documents | the module is a definition the model generator reads; its lemma is short enough to argue for every `B`, and exhaustive at small `B` catches an operator written wrong |
| A lane's value is the model's at every observation | runs (ADR 0115 D8 and its converse) | traces and replayed behaviours, values compared as terms (D10) | as for every document with a model |
| `J` and `C` are the largest encodings, exactly | an argument for every `B` (optimal substructure) and exhaustive tests | the build's tests enumerate every term of at most 7 nodes of the fixture documents and compare the largest encoding with the computed size; the conformance corpus encodes a maximal value on each lane | the recurrence is Bellman's; the enumeration holds the code to it |
| The C11 arena meets T1's contract | bounded model checking and differential tests | CBMC harnesses of the same contract at `B` up to 8; the conformance corpus holds C11's bytes and refusals to Rust's | no deductive checker of C is pinned; CBMC is the one ADR 0156 D13 approves for C11, approved there for a seam's harness, and this record's acceptance extends that approval to the C11 arena |
| An author's properties over inductive variables | model, per document (ADR 0115 D15) | TLC over the generated module | as every property |

### D12. Lanes

Rust and C11 serve it, because the consumer that asked generates its sans-I/O core for both from one set of documents,
and the scope rule builds a facility on the backends a consumer needs:

- **Rust**, native, `no_std` with `Inline` storage, and `wasm32-unknown-unknown`: the arena, the machine's variables
  and places, the algorithms' views, outputs and recursions, the wire, the saved state and the codec;
- **C11**: the same, in the freestanding core of ADR 0110, every arena and every stack reserved where the machine or
  the caller holds it, nothing allocated.

Go, Kotlin, Python, C++, Swift and the Interpreter refuse a document that holds an inductive type by name where they
render it (`generate/unsupported-feature`, naming the type and the position), as ADR 0086 section 7.3 and ADR 0092
section 7 have a lane refuse a composition it does not lower, and the TTCN-3 export refuses it by name as it refuses a
construct it does not export. A lane is added by an amendment when a consumer needs it, against the same corpus.

### D13. Refusals and bounds

New `DiagnosticCode`s, each with its appendix row in `docs/SCE_ACCEPTED_SUBSET.md`
(`acceptance_doc_covers_every_code`), its row in SCE_ERROR_CONTRACT.md's catalog and
`schemas/sce-diagnostic.v1.schema.json`, added in the step that makes it:

| Code | Refuses | Decision |
|---|---|---|
| `inductive/member-not-recursive` | a member on no cycle of its document | D2 |
| `inductive/uninhabited` | a member with no finite value, naming the field that cannot be filled | D2 |
| `inductive/member-name-taken` | two members of one name, or a member named as an import's alias | D2 |
| `inductive/bound-below-least-value` | a node bound below its member's least value, as written or after a build moves it | D3 |
| `inductive/no-exact-grammar` | an inductive type where a position requires its GBNF grammar | D4 |
| `algorithm/recursion-not-structural` | a call on a cycle whose argument at the callee's measure is no path below the caller's measure; a function on a cycle with no measure, or measures of two documents on one cycle | D8 |
| `algorithm/recursion-overlaps` | two call sites that can run in one activation on paths one of which is a prefix of the other; a call site under a loop that contributes no element step, or under a `<sce:while>` | D8 |
| `algorithm/arena-in-recursive-function` | a local of an inductive type with a bracket in a function on a cycle | D8 |
| `algorithm/built-value-not-used-once` | a built sub-value placed or returned twice, or left unused, on some path | D7 |

Existing codes keep their classes: a member used without its bracket or with one inside its document,
`validation/attribute-rule-violated`; a member type as a map's key, `validation/map-key-type`; a member holding a map
in a wire position, `validation/map-in-wire-position`; a case's record that names no member and no import,
`validation/invalid-attribute`; a read of a case's field unproven, `expression/union-read-unproven`; an import cycle,
`validation/cross-kind-circular-dependency` and `manifest/circular-dependency`; a lane that does not lower it,
`generate/unsupported-feature`. A run's refusals are `tooManyNodes` and `CborOutOfRange` (D4) and a write's
`error.execution` (D6), not diagnostics.

**Bounds, by name** (ADR 0050, ADR 0086 section 1.2): the node bound of every use, `[name=N]`, moved by `--limit`,
read as `_x.limits.<name>`, and declared by a machine's and a schema unit's constants as every bound is (ADR 0050
section 3); the bounds of every list and map inside a member, each its own. Every other size is a term of those names
(ADR 0118 D3, while it is Proposed, in its vocabulary): an arena's storage, `B` times the slot of its document's node
type; a recursion's stack, at most `B + 1` frames of its largest frame, and its activations, at most `B`; a walk's
work, at most `B` times one body's; `J` and `C`, the values D4's computation gives for the build's `B`, which are not
a closed expression of `B` in general and are stated as the build's evaluation of that computation at it.

## Not in scope

A graph with sharing or with cycles (a node of two parents, a back edge), which is a map from a number to a node or a
relation of ADR 0151, and not a term; sharing of nodes between two values (the path copying of a persistent
structure), since a value here is a tree owned once and a snapshot of it is a copy or a journal's (ADR 0114); infinite
and coinductive values; types generic over their element (§synth-5-G); a member type as a key; recursion other than
D8's, and recursion through imports; a recursive scope that spans documents; an inductive type on Go, Kotlin, Python,
C++, Swift, the Interpreter and the TTCN-3 export; a GBNF grammar of an inductive type; and any one application's
documents.

## Consequences

- A recursive type is a type of the language: declared once, held by any position, written to every form SCE writes,
  sized by the build, compared, saved, replayed and modelled as a value.
- A value never holds more than its bound of nodes and never an unreachable one, on any lane, and a write past the
  bound leaves it as it was. Its storage is its bound times its slot, reserved or grown as a map's is.
- No index, slot or pointer is part of a value's meaning or bytes; two lanes, or one lane after a restore, that hold
  one term in different slots are indistinguishable.
- A function over a tree ends, visits each node at most once, costs its nodes times one step, and needs no native
  stack beyond one frame, whatever the tree's depth. A function the rule refuses is written as a fold carrying a
  state, or as a descent, or not over a tree.
- A table read only by whole paths is a map keyed by the path, with no tree.
- A document that holds an inductive value has a model once the model generator translates `sce-static`'s typed values
  and this record's step lands, and none before; it is refused by name in the meantime.
- Two lanes serve it; five and the Interpreter refuse it by name until a consumer needs them.

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
- **A bound on depth, or a fixed-depth unrolling of the type** (§synth-5-H's stopgap). A depth with a fan-out admits a
  number of nodes exponential in it, so neither storage nor a walk's cost follows from it; an unrolling writes the
  type once per level.
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
  2001). Decidable and more general than D8's rule, and as silent on cost as the previous; its generality buys
  functions that are written better as a fold with a state (D8's trie match).
- **Native recursion with a stack bound computed from the frame.** The platform's stack is not the program's to size
  on every lane, and a deep tree on a microcontroller would overflow it where an explicit stack of the build's size
  does not.
- **A tree written by hand as a map of numbered nodes** (ADR 0108, today). Possible, and nothing tells the build that
  it is a tree: no invariant, no structural recursion, no canonical bytes, no model.

## Order of work

Each step lands with a test that fails without it. The theorems and the model come before the lanes, as ADR 0151's
order puts them.

1. This record, accepted, and the application of D1 in `SCE_FORGE.md` §10.
2. The kind's reader and checks, the type's bracket and its refusals, and every code of D13 that a document alone
   decides, with its appendix row: `sce-build/tests/an_inductive_document_is_read_as_one_recursive_scope.rs`. Red
   without it: `sce:kind="inductive"` is an unknown kind, and `record:Node[8]` is refused as a bracket on a record.
   Each refusal has a case that must be refused by its code.
3. The sizes and the forms' grammar: `sce-build/tests/an_inductive_type_is_sized_by_its_node_bound.rs`, which
   enumerates every term of at most 7 nodes of three fixture documents (a self-recursive record, a recursive union
   with a list of children, two members recursing through each other) and holds `J` and `C` to the largest of them.
   Red without it: the fold refuses the type as one that holds itself.
4. The Rust arena with T1 proved: `backends/rust/portable-bytes/src/arena.rs`, its entry in `docs/formal/proofs.json`
   with mutants that each fail its proof (a freed subtree's slots not returned, a replace that copies before it frees
   and so answers full for a result that fits, a decode that stores the node past the bound), and
   `backends/rust/portable-bytes/tests/arena.rs` with
   `a_replace_answers_full_exactly_when_the_result_passes_the_bound`, `two_arenas_of_one_term_write_the_same_bytes`
   and `a_decode_refuses_the_node_past_the_bound_before_storing_it`.
5. The model: `docs/formal/Inductive.tla`, its lemma checked as D11 says and entered in `docs/formal/models.json` with
   a configuration per operator taken out that TLC must find, and the translation of D10 in `sce-codegen model`,
   landed after the typed values of `sce-static` are translated (ADR 0156 D3). Red without it: `sce-codegen model`
   refuses a fixture document with an inductive variable as `Construct::Inductive`.
6. The Rust machine: variables, places, `nodes`, walks, writes in place, the wire, the saved state, the snapshot and
   the host's view: `backends/rust/scenarios/tests/inductive.rs` (`a_restored_tree_equals_the_tree_saved`,
   `a_write_past_the_bound_raises_and_leaves_the_value`,
   `a_subtree_moved_under_its_own_ancestor_fits_where_the_result_fits`), run native and on `wasm32-unknown-unknown`,
   its scenarios replayed against the model, and generated cases of the conformance corpus
   (`tests/forge/conformance/gen_cases.py`, from a recorded seed) for the forms and the refusals.
7. Algorithms: views, outputs, local functions, D8's rule, the explicit stack and T2:
   `sce-build/tests/a_recursion_takes_disjoint_parts_of_its_measure.rs`, with a case for each refusal of D7 and D8 and
   the trie match of D8 admitted; the mutation casefile
   `sce-build/tests/mutations/a_recursion_takes_disjoint_parts.cases`, whose case that drops the prefix check must
   turn the overlapping case red; and `backends/rust/forge-conformance/tests/forge_inductive.rs` with
   `a_trie_match_runs_at_most_one_activation_per_node` (activations counted on a trie of 10 000 nodes) and
   `a_recursion_deeper_than_the_native_stack_runs` (a chain of 100 000 nodes on a thread of 64 KiB of stack, red for a
   native recursion).
8. The map keyed by a list (D9): `sce-build/tests/a_list_of_keys_is_a_key.rs` and the map's index on Rust and C11 fed
   a list key as `map.h` feeds a record. Red without it: `validation/map-key-type` on
   `map<list<string[8]>[4], uint8>[4]`. Every other lane refuses such a key by name until it lowers it.
9. C11: `arena.h`, the lowerings, the stack lent by the caller, the CBMC harnesses of D11 once CBMC is pinned (ADR
   0156 D13), and `backends/c/tests/unit/forge_arena_test.c`; the corpus of step 6 and the algorithms of step 7
   replayed on C11 with Rust's bytes.
10. The documents named in "Amends".

## Open questions

Each is decided here, with its reason, for the acceptance to confirm or overturn.

1. **One document per recursive group, rather than cycles of imports.** A mutual definition is one fixed point and
   grows as one; the import graph stays acyclic for the manifest and the generator (D1).
2. **The bound counts nodes**, rather than depth or a fan-out per level: storage and every cost are functions of nodes
   (D3).
3. **The wire form is the term**, never the arena: one form per value (ADR 0101), and the representation stays free
   (D4, D5).
4. **A value is a tree, with no sharing**: equality, size and freeing are then the term's, and nothing needs a
   collector (D5, Alternatives rejected).
5. **Recursion is structural on one measure and takes disjoint parts**, rather than general measures or size-change
   termination: it is decided by the build, gives at most `B` activations, and every function the consumer's rows name
   is a fold, a fold with a state, or a descent (D8).
6. **No recursion is native**: an explicit stack of the build's size on every lane, as Appendix D's recursions already
   run (D8).
7. **A member type is no key; a list of keys is one** (D9).
8. **A GBNF grammar is refused**, the JSON Schema states the node bound as an annotation (D4).
9. **The kind is named `inductive`**, what fixes its meaning, the least fixed point, as Coq's keyword names it; `tree`
   names the shape of one value and `data-tree` (§synth-5-H) would read as a kind of data rather than of type
   declarations.
10. **Rust and C11**, by the scope rule; C11's arena is held by CBMC, whose approval for that use the acceptance of
    this record gives (D11).

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
and Programming Languages*, MIT Press, chapters 20 and 21); Harper 2016 (*Practical Foundations for Programming
Languages*, 2nd edition, Cambridge University Press); Cormen, Leiserson, Rivest and Stein 2009 (*Introduction to
Algorithms*, 3rd edition, MIT Press, section 10.3); Knuth 1997 (*The Art of Computer Programming*, volume 1, 3rd
edition, Addison-Wesley, sections 2.2.3, 2.3.1 and 2.3.3); RFC 8610 (CDDL), RFC 8949 (CBOR), JSON Schema 2020-12
(Core, `$ref`), ITU-T X.680 (ASN.1).

What was checked, and what was not:

- **Checked, in this repository, for this record:** the passages of ADR 0086 (sections 1, 1.5, 2.1), ADR 0092
  (sections 1, 2, 5 to 9), ADR 0101 (sections 1 to 4), ADR 0108 (D1 to D8), ADR 0115 (the levels, D7 to D10), ADR 0133
  (D1), ADR 0151 (whole), ADR 0152 (whole), ADR 0154 (D1), ADR 0156 (the Context, D1 to D5, D13, D14) and SCE_FORGE.md
  §3.2, §4.12 and §10 that the decisions amend or follow; and the code the Context cites (`value_type.rs`'s grammar,
  its refusal of a type that holds itself and of a list as a key; `model.rs`'s `UnionModel` and `AlgorithmStmt::Call`;
  `static_js.rs`'s refusal of an algorithm that imports itself; `map.h`'s slots and free list; `document_model.rs`'s
  refusal of `sce-static`), and §synth-5-F and §synth-5-H of the protocol-synthesis RFC.
- **Not checked against the texts:** every work of the literature above is cited from the texts as known and was not
  read again for this record. The passages the decisions rest on are to be checked in the step that builds each: the
  initial-algebra reading and strict positivity (D2, step 2), the guard condition and the predicative analysis of
  structural recursion (D8, step 7), defunctionalization (D8, step 7), the abstraction-function method (D5, step 4),
  the sequential representation of trees in preorder (D10, step 5), and section 8.2.3 of JSON Schema's Core (D4, step
  3).
