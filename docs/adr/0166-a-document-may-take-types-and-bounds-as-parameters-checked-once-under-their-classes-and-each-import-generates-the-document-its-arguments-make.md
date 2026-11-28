# ADR 0166 — A document may take types and bounds as parameters, checked once under their classes, and each import generates the document its arguments make

- Status: Proposed (2026-11-28), for the owner's acceptance after an independent review.
- Built: nothing. No step of "Order of work" is built, and no test of it has been run.
- Date: 2026-11-28
- Written under the owner's standing rule (the most long-term correct structure, textbook in its foundations, cost
  ignored) and the owner's scope rule of 2026-11-22: a facility is built on the backends a consumer needs, Rust by
  default; its contract is general; what is special to one application stays in the application. The request came from
  a downstream consumer with one piece of logic that is generic over an application's types; this record decides the
  general facility and names nothing of that consumer.
- Amends (each amended document gains an "Amended by: ADR 0166" line in the landing that moves it):
  - RFC §synth-5-G (`docs/spec/synth/rfc-sce-protocol-synthesis.md`, "Parametric kinds"), deferred there: this record
    decides it, and answers its question Q10 (a parameter is declared by an element and written with a sigil, D1);
  - `SCE_FORGE.md`: a new §3.7 "Type parameters" (D1 to D10); §3.3 (`$P` wherever a type is written); §4.12 (an
    algorithm's `<sce:algorithm-param>`, D2, and `===` on a value of class `key`, D2); §5 and §5.4 (an import passes
    arguments, and how an instance is resolved, D3);
  - ADR 0086 §1 (the one type grammar gains `$P` for a type parameter and `[$N]` for a bound parameter, D2) and §5.1
    (an instance fits only an instance of the same generic at the same arguments, D3);
  - ADR 0092 §1 (a union's case may be a parameter of class `record`, or an instance of a parametric schema, D2);
  - ADR 0101 §1 (a CBOR entry's type may hold a parameter, and its plan is made per instance, D1);
  - ADR 0044 and ADR 0112 D5 (an instance is named in a saved state by its family's event and its arguments, and a
    revision adds a field whose type holds a parameter only where a reader takes its default without a literal, D3);
  - ADR 0050 §1 and §2 (a bound parameter is read as `_x.limits.$N`, and its argument may be a named limit of the
    importer, D8);
  - ADR 0133 D1 (an operation on a parameter of a numeric class is total exactly where it is proven at every member of
    the class, D4);
  - ADR 0115 D7 and D10, and ADR 0156 D3 to D5 (a model, a contract's proof and a proof obligation are of an instance;
    a parameter's declaration supplies the condition of data independence for the code of its generic, D7);
  - `docs/SCE_ACCEPTED_SUBSET.md` §2.15 (a variable of an instance's type, and an import with arguments) and the
    appendix (D10's codes); `SCE_ERROR_CONTRACT.md` (D10's codes);
  - `schemas/sce-forge-ext.xsd`, `apis/forge-ast.v1.schema.json`, `schemas/sce-sourcemap.v1.schema.json` and
    `schemas/sce-manifest.v1.schema.json` (D1, D3, D6), each a surface `SCE_WIRE_CONTRACTS.md` registers, extended
    without removing anything.
- Scope: `sce-build`: the grammar, the reader, the check once (D4), the elaboration of each instance and its judgement
  again (D5), the import closure, the names of the units and the command line's arguments (D6). An instance reaches
  every backend and the Interpreter as a written document of its kind (D6), so no runtime and no template changes; the
  lanes are held to instances on Rust first and then on every lane by the numerical conformance harness (Order of
  work, step 5).
- Related: ADR 0005 (bound and declare), ADR 0050 and ADR 0086 (a bound by name, part of its type), ADR 0055 (optional
  values), ADR 0081 (a codec's plan, printed by each backend), ADR 0092 (unions), ADR 0101 (one deterministic CBOR
  form per type), ADR 0108 and ADR 0154 (maps, their keys and their storage), ADR 0109 D11 (a migration of a saved
  state), ADR 0118 (cost from the plan; Proposed), ADR 0133 (totality where the build proves it), ADR 0151 (a view,
  whose slots are of the same grammar), ADR 0156 (a system as documents, its models and obligations generated).

## Context

What exists today, read from the tree at `662e4d060cc`:

- **One type grammar, nominal where it names a document.** `sce-build/src/forge/value_type.rs` holds the grammar every
  position declares a value by (ADR 0086 §1): the fixed-width numbers, `bool`, `enum:<alias>`, `string[B]`,
  `bytes[B]`, `optional<T>`, `record:<alias>`, `union:<alias>`, `list<T>[B]`, `map<K, T>[B]` and `set<K>[B]`. A
  record, a union and an enum are named by the alias of an import, and a record type is its schema document: two
  schemas that agree field for field in two files are two types (`TypeIdentity::same_record`, `fits`). Nothing in the
  grammar stands for a type the document does not fix.
- **An import names one document and passes it values, not types.** `ForgeImport` (`sce-build/src/forge/model.rs`)
  holds an import's `src`, `kind` and `alias`, and two kinds of argument already: `<sce:flag-bind>`, which hands an
  imported codec the values of its flag inputs, and `<sce:variant-dispatch>`. No import passes a type.
- **A use at another type is met in three ways today, and each is a defect.**
  - *A type fixed by hand.* `stdlib/merge/orset_entry.scxml` makes the element of the observed-remove set a `uint64`,
    so `orset_union`, `orset_live`, `orset_observed` and `orset_contains` are a set of 64-bit numbers and of nothing
    else; `stdlib/merge/lww_cell.scxml` makes a register's value an `int64`.
  - *A type erased by hand.* `stdlib/shared/record_list.scxml` keeps, for each field of each row, a number or a text,
    and "reads neither": the application's types are erased into two representations the list carries for every field.
    That document is a statechart, which this record leaves out (D1).
  - *A document copied.* RFC §synth-5-G names the case (one VLE logic for `u16`, `u32` and `u64`) and defers it:
    "write three near-identical files; clean up once generics land". Its Q10 asks the XSD shape of a parameter, and
    RFC §synth-5-O asks that provenance survive "any future parametric-kind instantiation".
- **A textual template exists, and judges nothing before it is expanded.** `<sce:template>` with `<sce:use>`
  (`sce-build/src/template.rs`, mirrored by the Interpreter's `sce/src/parsing/TemplateExpander.cpp`) substitutes
  `{$name}` tokens in the XML text, a value being an opaque string: a template is judged only in each document it is
  expanded into, and its errors are reported there.
- **The runtimes are generic; the documents above them are not.** The Rust runtime's map is
  `BoundedMap<K, V, const N: usize, S: MapStorage>` (`backends/rust/portable-bytes/src/map.rs`, ADR 0108 and ADR
  0154), and the queue of `backends/rust/forge-runtime/src/queue` is written once over its element. A document that
  uses them names one type.
- **Bounds are per build, by name.** A bound is part of its type and may be named, `[name=N]`, moved by
  `--limit name=N` (a global argument of `sce-codegen`, `sce-build/src/cli/mod.rs`), and read as `_x.limits.<name>`
  (ADR 0050). A name has one number in a build (ADR 0050 §1), so two uses of one document that need two numbers in one
  build need two documents.
- **Models and proofs are per document.** `sce-codegen model` (`sce-build/src/formal/document_model.rs`) translates a
  statechart and refuses by name what it does not translate (`Construct`), a `sce-static` machine among it
  (`Construct::Datamodel`); an algorithm's contract is to be proved by Verus on the Rust that runs (ADR 0115 D10); ADR
  0156 D5 decides that a document which only copies and compares a type may be explored on a few of its values (Wolper
  1986), and nothing computes that condition yet.

So an application's types reach a standard algorithm today only by being the type the algorithm fixed, by being erased
into a representation it carries, or through a copy of it; and the one facility that substitutes into a document, the
textual template, judges nothing until it is expanded.

## The textbook structure

- **Parametric polymorphism.** A term may be abstracted over a type and applied to one (Girard 1972, the system F of
  his thesis, University of Paris VII; Reynolds 1974, "Towards a theory of type structure", Colloque sur la
  Programmation, LNCS 19), and such a term acts on every type alike: Strachey's parametric polymorphism as against his
  ad hoc polymorphism (Strachey 1967, "Fundamental concepts in programming languages", lecture notes, published in
  Higher-Order and Symbolic Computation 13, 2000).
- **Parametricity.** Reynolds' abstraction theorem: a term of a polymorphic type maps related arguments to related
  results, for every relation between two instantiations of its type variable (Reynolds 1983, "Types, abstraction and
  parametric polymorphism", Information Processing 83, IFIP, North-Holland). Wadler draws from it a theorem of every
  polymorphic type, "for free": a function of type `forall a. list a -> list a` commutes with every map of the
  elements (Wadler 1989, "Theorems for free!", FPCA 1989). The theorem rests on one premise: the term does nothing
  with a value of the variable's type but what the type grants. A language's checker is what makes the premise true.
- **A generic is checked once.** A polymorphic term is checked with its type variables abstract, and the substitution
  lemma carries the judgement to every instance: if a term has a type under a context that holds the variable `X`,
  then for every type `S` the term with `S` for `X` has that type with `S` for `X` (Pierce 2002, *Types and
  Programming Languages*, MIT Press, chapter 23; chapter 26 for bounded variables). A language that checks only its
  instances, as C++ checks a template's body at each instantiation, reports a generic's errors in the instance, at a
  place its author did not write.
- **A parameter that needs operations is bounded.** By a subtype bound (bounded quantification: Cardelli and Wegner
  1985, "On understanding types, data abstraction, and polymorphism", ACM Computing Surveys 17(4); Pierce 2002,
  chapter 26) or by a class whose instances supply the operations (type classes: Wadler and Blott 1989, "How to make
  ad-hoc polymorphism less ad hoc", POPL 1989). A class is *coherent* when a type has one instance of it, so the
  meaning of a program does not depend on which instance a search found; Wadler and Blott translate a class's
  operations to a dictionary passed as an argument. ML's modules pass them explicitly, as a structure given to a
  functor (MacQueen 1984, "Modules for Standard ML", ACM Conference on LISP and Functional Programming), and the two
  are one mechanism that differs in how an instance is found (Dreyer, Harper, Chakravarty and Keller 2007, "Modular
  type classes", POPL 2007). An operation whose precondition is weaker and whose postcondition is stronger than those
  required may stand for it (behavioural subtyping: Liskov and Wing 1994, "A behavioral notion of subtyping", ACM
  TOPLAS 16(6)).
- **A finite class is checked at each of its members.** A term of the intersection of finitely many types is checked
  at each of them (Reynolds 1996, "Design of the programming language Forsythe", in O'Hearn and Tennent (eds.),
  *ALGOL-like Languages*, Birkhauser; Pierce 1991, *Programming with intersection types and bounded polymorphism*, PhD
  thesis, Carnegie Mellon University).
- **Three ways to run a generic.** *Erasure*: one body for every instance, its types erased to one representation and
  recovered by casts (Bracha, Odersky, Stoutamire and Wadler 1998, "Making the future safe for the past: adding
  genericity to the Java programming language", OOPSLA 1998). *Type passing*: the run time carries the type and
  specialises when it is first used (Kennedy and Syme 2001, "Design and implementation of generics for the .NET Common
  Language Runtime", PLDI 2001). *Monomorphisation*: the compiler writes one copy per instance, each over concrete
  types (C++'s templates, Stroustrup 1988, "Parameterized types for C++", USENIX C++ Conference; MLton, Weeks 2006,
  "Whole-program compilation in MLton", ACM SIGPLAN Workshop on ML; Rust). A class's dictionaries are removed the same
  way, by specialising each use to its instance (Jones 1994, "Dictionary-free overloading by partial evaluation", ACM
  SIGPLAN Workshop on Partial Evaluation and Semantics-Based Program Manipulation). Monomorphisation leaves no type to
  the run time and costs a copy per instance, and it ends only where no generic reaches itself at another argument:
  polymorphic recursion (Mycroft 1984, "Polymorphic type schemes and recursive definitions", International Symposium
  on Programming, LNCS 167) makes the set of instances infinite.
- **A parameterised module applied twice to the same arguments** is one type (applicative functors: Leroy 1995,
  "Applicative functors and fully transparent higher-order modules", POPL 1995) or two (generative, as Standard ML's
  functors are). Rust's generics and C++'s templates are applicative: `Vec<u8>` is one type wherever it is written.
- **An instance is named by its generic and its arguments**, decodably, in every symbol that carries it (the Itanium
  C++ ABI's mangling of template arguments; Rust's v0 symbol mangling, RFC 2603, 2019).
- **A generator is trusted by checking each output** (Pnueli, Siegel and Singerman 1998, "Translation validation",
  TACAS 1998), as ADR 0156 D11 decides for this repository.
- **A program that only copies and compares the values of a type is data independent in it**, and a few values stand
  for all (Wolper 1986, "Expressing interesting properties of programs in propositional temporal logic", POPL 1986).
  Parametricity is the same fact stated of a type, and ADR 0156 D5 decides when a verdict may use it.

## Decision

### D1. A document may declare parameters, and four kinds take them

A document declares its parameters as the first children of its `<scxml>` root, before its imports. The record of the
observed-remove set, generic in its element:

```xml
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0"
       sce:kind="event-schema" name="orset_entry" sce:event-name="merge.orset.entry">
  <sce:type-param name="E" of="key"/>
  <datamodel>
    <data id="element" sce:type="$E" sce:direction="in"/>
    <data id="wallTime" sce:type="int64" sce:direction="in"/>
    <data id="counter" sce:type="uint32" sce:direction="in"/>
    <data id="nodeId" sce:type="uint64" sce:direction="in"/>
  </datamodel>
</scxml>
```

and the union of two such sets, generic in the element and in the bound of its answer, which hands its own parameter
on to the record it imports:

```xml
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0"
       sce:kind="algorithm" name="orset_union">
  <sce:type-param name="E" of="key entry"/>
  <sce:bound-param name="N"/>
  <sce:import kind="event-schema" src="orset_entry.scxml" as="Entry">
    <sce:type-arg name="E" type="$E"/>
  </sce:import>
  <sce:signature>
    <sce:param name="a" type="list&lt;record:Entry&gt;"/>
    <sce:param name="b" type="list&lt;record:Entry&gt;"/>
    <sce:return type="list&lt;record:Entry&gt;[$N]" may-fail="true"/>
  </sce:signature>
  <sce:body>
    <!-- the body of stdlib/merge/orset_union.scxml, with [$N] for [merge_entries=1024]: it copies
         entries and compares them field by field, the element by the equality its class key grants -->
  </sce:body>
</scxml>
```

- **Three elements declare.** `<sce:type-param name of>` declares a type, `<sce:bound-param name min>` a bound, and
  `<sce:algorithm-param name>` an algorithm of a declared signature (D2); each name is unique among the document's
  parameters. Every use writes the parameter with the sigil `$`: a type as `$E` wherever the grammar takes a type, a
  bound as `[$N]` or `[J..$N]` wherever it takes a bound, and an algorithm by its name where an imported algorithm's
  alias is called. The sigil keeps a parameter apart from a scalar keyword, an import's alias and a limit name at
  every use, and the template expander substitutes only `{$name}` (`sce-build/src/template.rs`), so `$E` is no
  template token. The declaration is an element, as RFC §synth-5-G's Q10 preferred for the XSD (Open question 7).
- **The kinds that take parameters** are those whose every slot is of the one type grammar (ADR 0086 §1) and whose
  document is a unit of values that keeps no state between two calls: `algorithm`, `event-schema` (a record type),
  `union`, and a `codec` of `sce:encoding="cbor"`, whose entries are values of the language and whose plan is made
  over their types (ADR 0101 §1 and §5). `<sce:algorithm-param>` is an algorithm's alone. Every other kind refuses a
  parameter by name (`type-param/kind-not-parametric`, naming the four). The reasons, for the kinds nearest the line:
  - a kind whose slots are of the older scalar attribute of SCE_FORGE §3.3 and not of the one grammar (`transform`,
    `condition` and `lookup` among them);
  - a kind that keeps state between calls or whose element is a document it names (`queue`, `broadcast-log`,
    `bounded-collection`, `worker`, `link`, `buffer-pool`, `timer`, `observer`): its runtime is written once over its
    element already, and a document of it names one;
  - a codec of another encoding, whose layout is written field by field and is the document's own;
  - a `statechart`, whose types reach the events it takes and sends. An event's name names one schema on the network
    (ADR 0044), so two instances of one statechart would need their events named per instance, which is a decision of
    its own (Not in scope).

  `view` (ADR 0151) meets the criterion, and takes parameters by these elements when its kind is built (Open question
  9).
- **A parameter is used.** A parameter that no slot, no type and no call of its document names is refused
  (`type-param/parameter-unused`): an instance's identity would depend on an argument that changes nothing in its
  values, and two instances would be two types of the same values (Open question 6).
- **No kind is added**, so SCE_FORGE §10's admission test does not apply: parameters are a construct of documents
  across four kinds, of the type grammar and of the import. That test's second axis is nevertheless the reason for D5:
  every argument is known at build, and each instance is generated with its arguments in place, so nothing is
  dispatched on a type at run time.

### D2. What a parameter ranges over

**A type parameter** ranges over the bounded types of ADR 0086's grammar that hold the classes it names in `of=`, a
list separated by spaces; with no `of` it ranges over every type of the grammar. The classes are a closed list. Each
is a structural predicate of the grammar that the build decides for every type, so a type holds a class in exactly one
way (its instance is derived from the type, never written) and every class is coherent by construction:

| Class | Holds of | Grants in the generic | Implies |
|---|---|---|---|
| (none) | every bounded type | to be held, passed, returned and copied whole, where the position admits every type (D4) | |
| `present` | a type that is not an `optional` at its top | `optional<$T>` | |
| `wire` | a type that holds no map and no set at any depth | to stand in an event schema's field, in a CBOR entry, and inside another type of an algorithm's slot (ADR 0108 D7) | |
| `key` | a key of ADR 0108 D1 that a body compares with `===`: an integer, `bool`, an enum, a bounded text, a record whose every field is one of these | `===` and `!==`, with the equality ADR 0108 D1 gives a key; to be a map's key and a set's element | `present`, `wire` |
| `entry` | a fixed-width number, `bool`, an enum or a bounded text: what one field of a record an algorithm takes holds (SCE_FORGE §4.12) | to be a field of a record an algorithm names | `present`, `wire` |
| `record` | a record type, an instance of a parametric schema included | to be a union's case (ADR 0092 §1) | `present`, `wire` |
| `integer` | the eight integer types | the integer arithmetic, comparisons, bit operations and literals of the expression language; `_x.types.$W.bits`, `.min` and `.max` | `key`, `entry` |
| `unsigned` | `uint8` to `uint64` | as `integer` | `integer` |
| `signed` | `int8` to `int64` | as `integer` | `integer` |
| `float` | `float32` and `float64` | the arithmetic, comparisons and literals of a float; `_x.types.$W.bits` | `entry` |

- **Nothing else is granted.** A value of a parameter's type is no operand, no condition, no number, no text and no
  record whose fields can be read, but through a class it holds. So a generic can neither inspect what it was not
  granted nor branch on it, which is the premise of the abstraction theorem (Reynolds 1983) and makes C2 of D9 hold of
  every instance.
- **A key's equality is the map's.** `===` at a parameter of class `key` is the equality ADR 0108 D1 defines, by which
  a map tells its keys apart: a text and a byte string by their bytes, a record field by field. Where an instance's
  argument is a record, the instance writes the comparison as the conjunction of its fields' comparisons in the
  schema's order, at every depth, which is the same relation in the form a body already writes (SCE_FORGE §4.12
  compares a record's fields and not a record), so no lane is given a new operator. A key ADR 0108 admits whose
  comparison a body does not write (a byte string outside the comparisons SCE_FORGE §4.12 lists) is not of the class:
  the class grants only what every one of its instances can write, and it is widened in the step that teaches a body
  the comparison, with no rule of its own.
- **Why `union` and `enum` are not classes.** A generic gains no operation from knowing that its argument is a union
  or an enum: it cannot name the cases or the variants of a document it does not know. An enum argument is admitted by
  `key`, which grants what an enum can do in a body that does not know its variants (be compared and be a key). A
  class that grants nothing and that no position needs would narrow the arguments for no reason (Open question 2).
- **Why `record` is.** A union's case is a record or none (ADR 0092 §1), so a generic union whose case is a parameter
  needs its argument to be a record; that is the one position of the language that admits only records, and the class
  grants nothing else.
- **The numeric classes are finite**, and what depends on a width (a literal, an overflow, a shift) is decided at each
  member (D4). `_x.types.$W.bits`, `.min` and `.max` read the width and the least and greatest value of the argument,
  written as the literals of the instance before anything reads the tree, as `_x.limits.<name>` is (ADR 0050 §2). In
  the generic, `bits` is a `uint32`, and `min` and `max` are of the type `$W`; each has its value at each member when
  D4 checks there.

**A bound parameter**, `<sce:bound-param name="N" min="K"/>` (`min` 1 when it is absent), ranges over the bounds of
ADR 0086 at or above `min`: positive integers that fit in 32 bits (ADR 0050 §3). It stands where a bound stands,
`[$N]`, and as the greater count of `[J..$N]` where `J` is at most `min`. An expression reads it as `_x.limits.$N`, a
`uint32` whose value the generic does not know, so it lands only where every `uint32` lands. A bound parameter exists
because a name has one number in a build (ADR 0050 §1): two uses of one document that need two bounds in one build
need the bound to be an argument (D8).

**An algorithm parameter**, `<sce:algorithm-param name="before">` holding a `<sce:signature>` over the document's type
parameters and the types it imports, with its `may-fail` and, where it carries one, its contract (`<sce:require>` and
`<sce:ensure>`, ADR 0115 D10), ranges over the algorithms whose signature fits it once the type arguments are in place
(D3). The generic calls it by its name, as it calls an imported algorithm, by an expression or `<sce:call>`. It is how
a generic is given an operation no class grants (an order on its records, a read of a field, a hash): the operation is
named at the import, as a structure is given to a functor (MacQueen 1984), so which algorithm runs is written where
the instance is made, and the instance calls it directly (D5). Classes an author declares, with instances the build
would find, are not offered (Open question 3).

### D3. An import passes the arguments, and an instance is its generic and its arguments

```xml
<sce:import kind="algorithm" src="sce:std/merge/orset_union.scxml" as="tagsUnion">
  <sce:type-arg name="E" type="string[tag_text=32]"/>
  <sce:bound-arg name="N" bound="tags_max=256"/>
</sce:import>
<sce:import kind="event-schema" src="sce:std/merge/orset_entry.scxml" as="TagEntry">
  <sce:type-arg name="E" type="string[tag_text=32]"/>
</sce:import>
<sce:import kind="algorithm" src="sort_by.scxml" as="sortTasks">
  <sce:type-arg name="R" type="record:Task"/>
  <sce:bound-arg name="N" bound="tasks_max=512"/>
  <sce:algorithm-arg name="before" ref="dueFirst"/>
</sce:import>
```

(The first two imports are of a `sce:std/merge` made generic as in D1, which this record does not do: Open question
12.)

- **Every parameter has exactly one argument at every import.** `<sce:type-arg name type>` gives a type of the
  grammar, written in the importer's names: its aliases, its named limits, and its own parameters where the importer
  is generic. `<sce:bound-arg name bound>` gives a bound of ADR 0086, a number or a named limit of the importer, or a
  bound parameter of a generic importer. `<sce:algorithm-arg name ref>` gives the alias of an algorithm the importer
  imports, or an algorithm parameter of a generic importer. A parameter left without an argument is
  `type-param/argument-missing`; an argument that names no parameter, or any argument on an import of a document that
  has none, is `type-param/argument-unknown`; an argument that does not hold its parameter's classes, a bound below
  its `min`, and an algorithm whose signature does not fit are `type-param/argument-outside-class`, naming the class
  and the first place in the argument where it fails, as `fits` names a `Misfit` (ADR 0086 §5.1). An argument is never
  inferred and has no default (Open question 8).
- **Arguments are given at an import, never in a type.** `record:Entry<uint64>` is no type of the grammar: the grammar
  names every nominal type by an alias (ADR 0086 §1), the import is the one place a document says which document a
  name means, and the alias then names the instance in every type and expression of the importer (Open question 1).
- **An algorithm argument fits** its parameter when, with the type arguments in place, each of its parameters is of
  the declared parameter's type (the identity a record argument is held to, SCE_FORGE §4.12), its return fits the
  declared return, and it declares `may-fail` only where the declared signature does. Where the declared signature
  carries a contract, the argument's precondition must follow from the declared one and its postcondition imply the
  declared one (Liskov and Wing 1994): an obligation of the instance, discharged where the argument's contract is
  proved (D7).
- **An instance is its generic and its arguments.** Its identity is the generic document's identity and its canonical
  arguments: each type argument spelled by the grammar, with each record, union and enum by its document's identity (a
  record by its event, as ADR 0112 D5 names one) and never by the alias it was written with; each bound by the number
  the build resolved it to; each algorithm argument by the identity of the document it names. Two imports with equal
  canonical arguments name one type and one unit, in one document or in two, as Rust's and C++'s generics do (Leroy
  1995's applicative reading): `record:TagEntry` above is the record that the algorithm's `record:Entry` names at the
  same argument, so the importer hands `tagsUnion` its lists. A bound argument that is a named limit gives the
  identity its number and the importer's constants its name (D8).
- **An instance fits only itself.** `fits` (ADR 0086 §5.1) admits an instance where the same generic at equal
  canonical arguments is expected. Instances are invariant in their arguments, as a record type is its document:
  `orset_entry` at `string[16]` does not fit `orset_entry` at `string[32]`, though a `string[16]` fits a `string[32]`
  (Open question 4).
- **A slot that is a parameter at its top writes the argument as the slot writes a type.** An algorithm's parameter is
  a view of its argument and writes no outermost bound (SCE_FORGE §4.12), so a parameter declared `$T` is, in the
  instance at `string[32]`, a `string` parameter; every other slot writes the argument whole.
- **An instance in a saved state** is named as ADR 0112 D5 names a record: by its family's event (the `sce:event-name`
  of the generic, which names the family and no event) followed by its canonical arguments,
  `merge.orset.entry<string[32]>`, with the generic's revisions and lineage. A revision of the generic is a revision
  of every instance (ADR 0044). A revision may add a field whose type holds a parameter only where a reader takes the
  field's default without a literal: an `optional<$T>`, read absent, or a list, read empty; no literal is of every
  type, so any other such field is refused (`type-param/added-field-without-default`). A variable that held an
  instance at other arguments held another type, and its change of type is a migration's (ADR 0109 D11), as any change
  of a variable's type is.
- **An instance of a parametric schema is a record, and no event's schema.** An event's name names one schema on the
  network (ADR 0044), and a family is many schemas, so a statechart that takes or sends the family's event with an
  instance as its schema is refused (`type-param/instance-as-event`); a schema of its own holds the instance as a
  field.

### D4. What the build checks once, on the generic

A generic document is checked where it is read, with its parameters abstract, by every check a written document of its
kind is checked by, under these rules. A refusal is reported at the generic's line, and needs no instance.

- **Typing.** Every expression and statement is typed with a parameter's type as a type of its own, equal only to
  itself. An operation on a value of it is one a class it holds grants (D2), or it is refused
  (`type-param/class-not-held`), naming the operation, the parameter and the class that grants it, with the fix that
  adds the class to `of=`.
- **Positions.** Every type that holds a parameter is held to the rules of the position it is written in, as if the
  parameter were any type of its classes; where a rule admits fewer types than the classes allow, the generic is
  refused with the class the rule needs (`type-param/class-not-held`). The rules this reaches today: `optional<$T>`
  needs `present` (ADR 0086 refuses an optional of an optional); a map's key and a set's element need `key` (ADR 0108
  D1); an event schema's field, a CBOR entry, and a type inside another type of an algorithm's slot need `wire` (ADR
  0108 D7: no map stands there), and so does the return of an algorithm that does not declare `may-fail` (SCE_FORGE
  §4.12: a map return declares it); a field of a record an algorithm names needs `entry` (SCE_FORGE §4.12: such a
  record holds entries, and a list of numbers or `bool`s on the backends that lend one); a union's case needs `record`
  (ADR 0092 §1). A rule of positions added later is held the same way, and C1's property test (D9) is what finds one
  this list missed. A type that contains itself cannot be written through a parameter: an argument is fixed before its
  instance is named, so no instance holds its own identity.
- **Numeric classes, at each member.** Where a parameter holds `integer`, `unsigned`, `signed` or `float`, every check
  that reads a width runs once for each member of the class: that a literal is of the type, that an operation is total
  where ADR 0133 asks it to be proven, that a shift's count is below `_x.types.$W.bits`, and every other. A refusal
  names the member it fails at. A document accepted for `$W` of `unsigned` is therefore accepted at `uint8`, `uint16`,
  `uint32` and `uint64`, and ADR 0133 D1 holds of each: an operation is proven total exactly where it is proven at
  every member.
- **Bound parameters.** `_x.limits.$N` is a `uint32` of unknown value, and a lesser count `J` in `[J..$N]` is held to
  `J <= min`.
- **Imports inside a generic.** An argument that is the importer's own parameter holds the classes the imported
  parameter needs where the importer's declaration implies them (the column "Implies" of D2), or
  `type-param/argument-outside-class` is reported at the importing generic; an algorithm parameter handed on is held
  to the inner signature as an algorithm argument is (D3).
- **What the check once proves.** For every argument that holds its parameter's classes, the instance passes every
  rule of the language the generic passed: the substitution lemma (Pierce 2002, chapters 23 and 26), for a language
  whose operations on a parameter are those its classes grant and whose numeric classes are finite. D9 says how the
  claim is held. What depends on a number the arguments bring is the instance's (D5).

### D5. Each instance is generated as the document its arguments make, and judged again

- **Elaboration.** Once the imports of a set are read and each generic is checked (D4), the build makes each instance
  by substituting its canonical arguments into the generic's tree: each `$T` by its type (D3 for a slot at its top),
  each `[$N]` by its bound, each call of an algorithm parameter by a call of its argument, each comparison of D2's
  `key` in the form D2 gives it, and `_x.types.$W.*` and `_x.limits.$N` by the instance's literals. The instance is a
  written document of the generic's kind, named as D6 says. Each node keeps the generic's position and the chain of
  imports that made the instance, so a diagnostic, a line of the sourcemap (RFC §synth-5-O) and a trace name the
  generic's line and the arguments.
- **The meaning of a generic is the meaning of its instances.** No generic runs. A document with parameters has no
  code, no model and no behaviour of its own, only the family of its instances, and each instance means what the
  document written with its types would mean. Nothing is left to prove of the elaboration but that it is the
  substitution, which D9 holds (C5).
- **The instances are finitely many, and the elaboration ends.** Imports are acyclic (an import cycle is refused where
  the imports are walked, `sce-build/src/forge/cross_kind_check.rs`). Every argument is written in the importer's
  names, and an instance's own imports are its generic's, which lie below the generic in the import graph. By
  induction on the height of a document in that graph, the instances made from it are finitely many. No generic can
  reach itself at another argument, which is the polymorphic recursion (Mycroft 1984) that would make them infinite.
- **Each instance is judged again, by every check a written document is judged by** (Pnueli, Siegel and Singerman
  1998; ADR 0156 D11). Two kinds of refusal can come of it, and only two, each named on the instance with the import
  that made it:
  - *a number*: what the arguments' sizes make of the instance, its largest encodings (ADR 0086 §2), a C11 slot (ADR
    0086 §8), its cost (ADR 0118), a size past what a lane holds. These refuse the instance as they would refuse a
    written document of the same numbers; the generic is not at fault;
  - *a lane*: a construct the instance holds that a backend does not lower (a record that holds a list on a backend
    `ALGORITHM_LISTED_RECORD_BACKENDS` does not list, a map on one `ALGORITHM_MAP_BACKENDS` does not list), refused on
    that backend alone by its existing code (`generate/unsupported-feature`).

  A refusal by a rule of the language that D4 covers is a defect of the check once, not of the document: the instance
  is refused with its own refusal and with `type-param/instance-breaks-its-generic`, naming the generic, the arguments
  and the rule; nothing of it is generated; and C1's property test (D9) holds that it is never reported.

### D6. One unit per instance, named by its generic and its arguments, on every lane and the Interpreter

- **One unit per instance in a set.** `sce-build/src/import_closure.rs` follows an import with arguments to its
  instance, and a set generated together (`generate --with-imports`, `orchestrate`) generates each distinct instance
  once, where the generic's unit would be placed.
- **The unit's name** is the generic's name followed by its canonical arguments, each made of identifier characters by
  the one mangler of RFC §synth-5-O (`sce-build/src/forge/symbol_mangling.rs`, `__` between segments), a record
  argument by its event: `orset_union__string_32__256`. Two instances of one set that mangle to one name are refused
  (`type-param/instance-name-collision`), and a name past a lane's identifier limit is held to the mangler's existing
  rule (`platform.strict_c99_identifiers`). The manifest lists each instance with its generic, its canonical arguments
  and its unit, and the sourcemap maps each generated line to the generic's line and the instance.
- **No type is erased.** Every item an instance's unit declares is over concrete types. On Rust it is the code a
  written document of those types would generate: no generic parameter on an item it declares, no `dyn`, no `Any`, no
  boxed value, and no tag of a type at run time; a runtime type it uses (`BoundedMap<K, V, N, S>`) is applied to
  concrete types where it is named. Every other lane holds an instance's values in the types it holds a written
  document's in. A lane's own generics are not used for a generic document (Alternatives rejected): one elaboration
  serves every lane, and the code that runs is the code its proofs and models are of (D7).
- **A generic alone generates nothing.** `sce-codegen generate` and `lower-algorithm` of a generic document with a
  parameter left without an argument are refused (`type-param/uninstantiated`). `--type-arg NAME=TYPE`,
  `--bound-arg NAME=BOUND` and `--algorithm-arg NAME=DOCUMENT`, read as the import's elements are and global as
  `--limit` is (ADR 0050), name the instance a host calls with no document that imports it. `sce-codegen check` of a
  generic alone runs D4.
- **Every backend and the Interpreter serve an instance, because what reaches them is a written document.** Rust, C11,
  C++, Go, Kotlin, Python and Swift print it through the templates they print its kind through, and the Interpreter
  runs a `sce-static` document as `static_js` lowers it after the elaboration, its algorithms among it. A lane refuses
  an instance exactly where it would refuse the written document (D5), by name. Under the owner's scope rule the first
  lane held to instances is Rust, the one a consumer needs; the others are held in the same step (Order of work, step
  5), since holding them costs fixtures of the conformance harness and no code.
- **The forge-AST and the TTCN-3 export.** `apis/forge-ast.v1.schema.json` carries a generic with its parameters, and
  each instance as the document it is; the test profile's TTCN-3 export (ADR 0091) names an instance's types as types
  of their own.

### D7. Models, contracts and proof obligations are of an instance

- **The model generator reads instances only.** `sce-codegen model` (ADR 0115 D7, ADR 0156 D3) is given a statechart,
  which takes no parameters (D1), and every instance a statechart holds is elaborated before the model is written
  (D5): the generator meets written documents only, and has no translation to add for this record. A statechart that
  holds an instance is refused today as every `sce-static` machine is (`Construct::Datamodel`,
  `formal/no-translation`), and is translated by the step of ADR 0156 D3 that translates `sce-static`, with nothing
  further here. A generic document handed to `sce-codegen model` or to the formal judge directly is refused by name
  (`Construct::TypeParameters`, `formal/no-translation`): it has no model, only its instances do. ADR 0119's second
  encoding reads the same instances.
- **An algorithm's contract is proved per instance** (ADR 0115 D10): the proof is of the code that runs, and the code
  that runs is the instance's. A contract written in the generic over `$T` is the instance's contract at its
  arguments, and an algorithm argument's obligation (D3) is discharged in the instance, where both contracts are
  proved.
- **The proof obligations ADR 0156 D4 generates** (`TypeOK`, an invariant per bounded structure) are of the instance's
  types and bounds, which are concrete.
- **Data independence by declaration.** In a generic, a parameter whose classes are among `present`, `wire`, `entry`
  and `record`, or that has none, is copied and nothing else, and one with `key` is copied and compared for equality:
  Wolper's condition, which D4 guarantees for the generic's own code at every instance. ADR 0156 D5's analysis of a
  machine's types reads the condition from the declaration for the part of the machine an instance contributes, and
  decides as before for the rest; no verdict uses it before ADR 0156 D5 states the threshold its theorem gives.

### D8. Bounds by name

- A bound parameter's argument is a number, a named limit of the importer (`tags_max=256`), or a bound parameter of a
  generic importer. A named limit stays the importer's: it is among the importer's `named_limits` (ADR 0050 §1), moved
  by `--limit tags_max=N`, and declared as the importer's typed constant (ADR 0050 §3). The instance holds its number
  (D3), and the instance's unit declares the bound under the name it came with, as a schema's unit declares a field's;
  a bound passed as a plain number has no name.
- A named limit written inside a generic (`[merge_entries=1024]`) stays one name with one number in the build (ADR
  0050 §1), shared by every instance. That is the line between the two: a name is a deployment's knob, and a bound
  that must differ between two instances in one build is a parameter.
- The facility adds no resource at run time (nothing is erased, dispatched or kept for a type, D6), and so no bound of
  its own. Its cost is at the build and in code: one unit per instance, which the manifest lists (D6) and ADR 0118's
  cost reads as it reads a written document's.

### D9. What each claim is judged by

| Claim | Level | Why that level |
|---|---|---|
| C1. An instance of an accepted generic, at arguments of its classes, passes every rule of the language its generic passed (D4) | Every instance judged again (D5), and a property test of the build, `every_instance_of_an_accepted_generic_is_accepted` | The claim is the substitution lemma of SCE's static semantics, and that semantics is not yet an object a prover reads (ADR 0156 D2 decides to write it), so no theorem can be stated over it yet. The code does not rest on the claim: every instance is judged again and refused if a rule fails (ADR 0156 D11). The test holds the check once to the lemma, so that the judgement again never refuses what a generic was promised |
| C2. An instance commutes with a renaming of a parameter it only copies (any map of its values) or copies and compares (an injective map) | A property test on every lane, by self-composition (below) | The abstraction theorem holds of the language because D4 grants nothing else; what could break it is a lane whose lowering reads a representation (an order of values where the document kept an order of arrival), which only running the lanes shows |
| C3. The instances of a set are finitely many, and the elaboration ends | The argument of D5, by induction on the height of the import graph, and a test that an import cycle through an argument is refused | The argument is short and complete; the test holds the premise it rests on, an acyclic graph |
| C4. Equal canonical arguments name one type and one unit, and unequal ones two | A property test of the build | A function of the canonical text, decided by comparing it |
| C5. An instance means what the document written with its types means | By definition (D5), and a test that a generic `orset_union` instantiated at `uint64` is, node for node, the tree of `stdlib/merge/orset_union.scxml` as it is written today | The meaning of a generic is defined as its instances'; the test shows that the elaboration writes that document |
| The behaviour of an instance: its contract, its model, its properties | The levels of its kind (ADR 0115 D10 by Verus, D7's generated model, D15's properties), of the instance | An instance is a written document to every level (D7) |

The self-composition of C2: for an instance at two arguments `A` and `B` of a parameter's classes and a renaming `f`
from the values of `A` to those of `B` (injective where the class is `key`), each lane runs the instance at `A` on
inputs `x` and at `B` on `f(x)`, and the answers are related by `f`, byte for byte once `f` is applied: the free
theorem of the instance (Wadler 1989), as a test.

No TLA+ model of the facility is written. The elaboration is a function of documents computed at build, with no state
over time, no concurrency and no environment, and TLC judges behaviours. A theorem of C1 in a prover is the right
level once ADR 0156 D2's semantics exists as an object a prover reads, and is not in this record's order of work
because what it would be stated over is not written (Open question 11).

### D10. Refusals

Each is a `DiagnosticCode` with its row in the appendix of `docs/SCE_ACCEPTED_SUBSET.md` and in
`SCE_ERROR_CONTRACT.md`, added in the step that makes it:

| Code | Stage | When |
|---|---|---|
| `type-param/kind-not-parametric` | Validation | a parameter declared on a kind that takes none (D1), naming the four that do |
| `type-param/unknown-parameter` | Validation | a `$P` that names no parameter of the document, or a parameter of another sort (a bound where a type is written, or the reverse) |
| `type-param/parameter-unused` | Validation | a parameter that no slot, type or call names (D1) |
| `type-param/class-not-held` | Validation | an operation or a position that needs a class the parameter does not hold (D4), with the fix that adds it |
| `type-param/argument-missing` | Import | an import of a generic that leaves a parameter without an argument (D3) |
| `type-param/argument-unknown` | Import | an argument that names no parameter, or any argument on an import of a document that has none (D3) |
| `type-param/argument-outside-class` | Import | an argument outside its parameter's classes, a bound below its `min`, or an algorithm whose signature does not fit (D3, D4) |
| `type-param/added-field-without-default` | Validation | a revision that adds a field whose type holds a parameter and is neither an optional nor a list (D3) |
| `type-param/instance-as-event` | Validation | an instance of a parametric schema taken or sent as an event's schema (D3) |
| `type-param/uninstantiated` | Cli | `generate` or `lower-algorithm` of a generic with a parameter left without an argument (D6) |
| `type-param/instance-name-collision` | Generate | two instances of one set mangled to one unit name (D6) |
| `type-param/instance-breaks-its-generic` | Validation | an instance refused by a rule the check once covers: a defect of the check, reported with the instance's own refusal (D5) |

An unknown class in `of=` is `validation/invalid-attribute`, its candidates the classes of D2. A generic handed to the
model is `formal/no-translation` with `Construct::TypeParameters` (D7). A number or a lane that refuses an instance
does so by the code it has today (D5).

## Not in scope

- A statechart's parameters. Its types reach the events it takes and sends, and an event's name names one schema on
  the network (ADR 0044), so an instance of a statechart needs a rule that names its events per instance. That rule is
  a record of its own, and would use D2 to D10 unchanged; the first document it would serve is
  `stdlib/shared/record_list.scxml`, which erases its rows' types today.
- Parameters of the kinds D1 refuses, and of `view` until ADR 0151 builds its kind.
- A parameter that is itself generic (higher kinds), an argument that is a generic not yet applied (higher rank),
  polymorphic recursion, and computation on types beyond `_x.types.$W.bits`, `.min`, `.max` and `_x.limits.$N`.
- Classes an author declares, with instances the build finds (D2 closes the list, and an operation no class grants is
  an algorithm argument); bounds on a record's fields (row polymorphism; records are nominal, ADR 0086 §5.1); variance
  between instances; default arguments; arguments inferred at a call.
- A type-directed encoding of an application's own, a format defined by a rule per type constructor (polytypic
  programming: Jansson and Jeuring 1997, "PolyP: a polytypic programming language extension", POPL 1997; Hinze 2000,
  "A new approach to generic functional programming", POPL 2000). A parametric document cannot inspect its argument,
  which is what parametricity says; a format over every type is a codec encoding, as ADR 0101 is for CBOR, and a new
  encoding is a record of its own. Within this record a generic reaches a value's bytes through a CBOR codec
  instantiated at its type (D1).
- Converting the standard documents that fix a type today (Open question 12).
- Run-time type information, erasure and boxing, on every lane.

## Consequences

- An algorithm, a record, a union or a CBOR codec is written once and used at an application's types, with the bounds
  each use needs; what is fixed by hand today becomes an argument.
- A generic's errors are reported at the generic, at its author's line, once. An importer sees only the refusals of
  its arguments, a number of its instance, or a lane.
- Every lane and the Interpreter run instances with no change of their own, and no generated code carries an erased
  type, a dispatch on a type or a description of a type at run time.
- Each instance is a unit of code: a program grows with the instances it uses, as with every monomorphising compiler,
  and the manifest says which.
- A model, a proof and a trace are of instances, which are written documents to them; a generic supplies the condition
  of data independence for its own code by its declaration.
- A bound that must differ between two uses in one build is a bound parameter; a named limit stays one number per
  build.

## Alternatives rejected

- **A copy of a document per type** (today's way, and RFC §synth-5-G's "write three near-identical files"). Each copy
  is checked and corrected alone, and a defect corrected in one stays in the others.
- **The application's types erased into a universal representation** (`record_list`'s number or text, a `bytes` field
  holding an encoding, an `any` type). What the build checks becomes what the run time checks, a value's bounds stop
  being part of its type (ADR 0086), and every reader decodes what a type would have stated.
- **`<sce:template>` extended to types.** A template substitutes text and is judged only where it is expanded: its
  errors land in the importer's document, and nothing is said of the template for every use. Parameters are typed and
  checked once (D4).
- **Each lane's own generics.** C11 has none; Go's generics share one body among the types of one shape and pass a
  dictionary; Kotlin's are erased on the JVM, and Swift's pass a type's metadata at run time where the compiler does
  not specialise. Each would make the code that runs differ in shape across lanes and from the code a proof is of. One
  elaboration, before every lane, gives every lane the same instance and leaves no type to the run time.
- **Erasure, or a type passed at run time** (Bracha et al. 1998; Kennedy and Syme 2001). Each leaves a type, or a
  cast, to the run time, which the bounded lanes cannot afford and which SCE's static checks exist to remove.
- **Checking only the instances.** Correct for the code generated, but it reports a generic's errors to its importers,
  at their arguments, and says nothing of a generic before it is used.
- **Classes an author declares, with instances the build finds.** Coherence then needs one instance per type across
  every document a build reads (an orphan rule), and which operation runs is chosen by a search the importer's text
  does not show. With the classes closed and derived (D2) every class is coherent by construction, and an algorithm
  argument (D3) names the operation at the import; the two differ only in how an instance is found (Dreyer et al.
  2007).
- **`union` and `enum` as classes.** They would narrow the arguments and grant nothing (D2).
- **Instances generative**, a new type at each import. Two documents that import one instance at one argument could
  not hand each other its values, which is the use (D3).

## Order of work

Each step lands with a test that fails without it.

1. This record, accepted; `SCE_FORGE.md` §3.7 written from D1 to D10, and RFC §synth-5-G and its Q10 marked decided by
   it.
2. The grammar and the reader: `$P` in `sce-build/src/forge/value_type.rs` (a constructor for a type parameter, and a
   bound that may be a parameter), the three declarations, the three arguments on `ForgeImport`
   (`sce-build/src/forge/model.rs`, `sce-build/src/forge/parser.rs`), `schemas/sce-forge-ext.xsd` and the forge-AST:
   `sce-build/tests/a_document_declares_its_parameters_and_an_import_passes_their_arguments.rs`. Red without it:
   `<sce:type-param>` is an element the XSD refuses, and `$E` an unknown type. It holds `kind-not-parametric`,
   `unknown-parameter`, `parameter-unused`, `argument-missing` and `argument-unknown`, each with a case.
3. The check once (D4), with its refusals:
   `sce-build/tests/a_generic_document_is_checked_once_under_the_classes_of_its_parameters.rs`, an accepted case for
   each class and a refused case for each row of D10 that a generic or its import decides (`class-not-held` at each
   position and operation of D4, `argument-outside-class` for a class, a `min` and a signature,
   `added-field-without-default`, `instance-as-event`), and a numeric case refused at `uint8` alone (a literal `0x1FF`
   held in a `$W` of `unsigned`), naming the member. A mutation casefile,
   `sce-build/tests/mutations/a_generic_reads_its_parameter_only_through_its_classes.cases`, holds two cases, each of
   which must turn the test red: one grants `===` to a parameter with no class, and one checks a numeric class at its
   widest member alone.
4. The elaboration and the judgement again (D5): `sce-build/tests/an_instance_is_the_document_its_arguments_make.rs`
   (C5: a fixture `orset_union` with `E` of `key entry`, instantiated at `uint64` and `merge_entries=1024`, is node
   for node the tree of `stdlib/merge/orset_union.scxml`; C4: two imports at equal arguments name one type and one
   unit, and imports at `string[16]` and at `string[32]` name two that do not fit each other; an instance refused for
   a number names the generic's line and the import that made it; C3: an import cycle through an argument is refused)
   and `sce-build/tests/every_instance_of_an_accepted_generic_is_accepted.rs` (C1: generic documents generated from a
   seed it records, over every position and operation of D2 and D4, each at random arguments of its classes; every
   instance passes every check, and `instance-breaks-its-generic` is never reported). Red without it: an import with
   arguments makes no instance. The casefile of step 3 gains a case that drops `present` from what `optional<$T>`
   needs, which must turn the property test red.
5. Generation and the lanes (D6): `sce-build/tests/an_instance_is_generated_once_per_set_and_names_no_parameter.rs`
   (one unit for an instance two documents of a set import; the Rust of an instance declares no generic parameter and
   names no `dyn` and no `Any`; `generate` of a generic without `--type-arg` refused as `uninstantiated`;
   `instance-name-collision`; the manifest and the sourcemap name the generic and the arguments). Red without it: an
   import with arguments generates no unit. Instance fixtures in `tests/forge/conformance/fixtures.json` (the
   `orset_union` fixture at a text, an algorithm with an algorithm argument instantiated at two comparators, a union
   whose case is of class `record`, a CBOR codec of an instance, an algorithm of a `$W` of `unsigned` at `uint16` and
   at `uint64`), run by the numerical conformance harness on Rust, then on every backend and the Interpreter. Red
   without it: the fixtures name instances no lane has a unit for.
6. Parametricity on the lanes (C2): `tests/forge/conformance/gen_cases.py` writes, for the `orset_union` fixture at
   `uint64` and at a text and an injective renaming between them, the cases of both, and the harness holds the answers
   to be related by the renaming on every lane. A mutation casefile,
   `sce-build/tests/mutations/an_instance_keeps_the_order_its_document_gives.cases`, makes the Rust lowering of the
   instance walk its entries in the order of their elements' values, which must turn the renaming cases red.
7. The documents named in "Amends".

Steps 2 to 4 change `sce-build` alone, and step 5 adds fixtures and no code to a lane. No step needs ADR 0156 D3: a
machine that holds an instance is modelled when `sce-static` is (D7).

## Open questions

Each is decided here, with its reason, for the owner's acceptance.

1. **Where arguments are given.** At an import, never applied in a type: the alias is the one place a document says
   which document a name means (ADR 0086 §1), and it then names the instance in every type and expression.
2. **Which nominal sorts are classes.** `record` is, for the union's case, the one position that admits only records;
   `union` and `enum` are not, since they grant nothing and no position needs them, and an enum argument is admitted
   by `key`.
3. **Operations beyond the classes.** An algorithm argument named at the import, not a class an author declares: the
   closed classes are coherent by construction, the operation that runs is written where the instance is made, and
   with every use specialised (D5) the two mechanisms compile to the same code (Wadler and Blott 1989; Jones 1994;
   Dreyer et al. 2007).
4. **Variance.** None: an instance is its generic and its arguments as a record type is its document, and the use (D3)
   needs identity only; variance would make whether an instance fits depend on more than its identity.
5. **Statecharts.** Not in this record: an instance of a statechart needs its events named per instance, since an
   event's name names one schema on the network (ADR 0044), and that is a decision about the names of events, not
   about parameters.
6. **A parameter no slot names.** Refused: a distinction no value carries is no type's to make here, and an instance's
   identity would otherwise depend on an argument that changes nothing.
7. **The spelling.** A declaration is an element (`<sce:type-param>`, `<sce:bound-param>`, `<sce:algorithm-param>`),
   which the XSD validates, as RFC §synth-5-G's Q10 preferred; a use is `$P`, which no scalar keyword, alias or limit
   name can be, and which the template expander does not read.
8. **Defaults and inference.** Neither: an import's instance is the text of the import, which a default would make
   depend on text the importer does not show, and an argument inferred at a call would make each call site's instance
   depend on its operands.
9. **`view`.** It meets D1's criterion, its slots being of the grammar and its plan printed per document (ADR 0151
   D4), so per instance; it takes parameters by this record's elements when ADR 0151 builds its kind, with no rule of
   its own.
10. **Monomorphisation over one body with a dictionary.** Monomorphisation: no erased type, the bounded lanes, and the
    proof of the code that runs (ADR 0115 D10) each ask for it. Its cost, a unit per instance, is listed by the
    manifest.
11. **The level of C1.** A property test and the judgement of every instance now; a theorem once ADR 0156 D2's
    semantics is an object a prover reads. The code does not rest on C1 meanwhile, since an instance that breaks a
    rule is refused and never generated (D5).
12. **The standard library.** Its documents that fix a type (`orset_entry` and the algorithms over it, `lww_cell`) are
    converted in records of their own: a conversion changes the names of units that a host calls today (D6), which is
    a decision about that family and its hosts, not about the facility.

## Sources

Read for this record, in this repository: ADR 0151 and ADR 0152 whole, as the house style; `SCE_FORGE.md` §3.2, §3.3,
§4.12 (algorithm, records, lists of records, unions, maps and sets) and §10; ADR 0115's table of levels and its D3,
D7, D10 and D15; ADR 0156's Context, textbook structure and its D3, D4, D5, D11, D12 and D14; ADR 0050's decision; ADR
0101 §1 and §5; ADR 0112 D5; ADR 0108 D1; RFC §synth-5-G and Q10 in `docs/spec/synth/rfc-sce-protocol-synthesis.md`;
and the code named in the Context (`value_type.rs`'s grammar, `TypeEnv`, `TypeIdentity` and `fits`; `ForgeImport` and
`parse_imports`; `template.rs`; `document_model.rs`'s `Construct`; `symbol_mangling.rs`; `import_closure.rs`; the
standard documents `orset_entry`, `orset_union`, `lww_cell` and `record_list`). ADR 0086, ADR 0092, ADR 0044, ADR 0133
and ADR 0154 were read where the decisions cite them, not whole.

The literature: Girard 1972 (thesis, University of Paris VII); Reynolds 1974 ("Towards a theory of type structure",
Colloque sur la Programmation, LNCS 19); Strachey 1967 ("Fundamental concepts in programming languages", Higher-Order
and Symbolic Computation 13, 2000); Reynolds 1983 ("Types, abstraction and parametric polymorphism", Information
Processing 83, IFIP); Wadler 1989 ("Theorems for free!", FPCA); Pierce 2002 (*Types and Programming Languages*, MIT
Press, chapters 23 and 26); Cardelli and Wegner 1985 ("On understanding types, data abstraction, and polymorphism",
ACM Computing Surveys 17(4)); Wadler and Blott 1989 ("How to make ad-hoc polymorphism less ad hoc", POPL); MacQueen
1984 ("Modules for Standard ML", ACM Conference on LISP and Functional Programming); Dreyer, Harper, Chakravarty and
Keller 2007 ("Modular type classes", POPL); Liskov and Wing 1994 ("A behavioral notion of subtyping", ACM TOPLAS
16(6)); Reynolds 1996 ("Design of the programming language Forsythe", in O'Hearn and Tennent (eds.), *ALGOL-like
Languages*, Birkhauser); Pierce 1991 (*Programming with intersection types and bounded polymorphism*, PhD thesis,
Carnegie Mellon University); Bracha, Odersky, Stoutamire and Wadler 1998 ("Making the future safe for the past",
OOPSLA); Kennedy and Syme 2001 ("Design and implementation of generics for the .NET Common Language Runtime", PLDI);
Stroustrup 1988 ("Parameterized types for C++", USENIX C++ Conference); Weeks 2006 ("Whole-program compilation in
MLton", ACM SIGPLAN Workshop on ML); Jones 1994 ("Dictionary-free overloading by partial evaluation", ACM SIGPLAN
Workshop on Partial Evaluation and Semantics-Based Program Manipulation); Mycroft 1984 ("Polymorphic type schemes and
recursive definitions", International Symposium on Programming, LNCS 167); Leroy 1995 ("Applicative functors and fully
transparent higher-order modules", POPL); Pnueli, Siegel and Singerman 1998 ("Translation validation", TACAS); Wolper
1986 ("Expressing interesting properties of programs in propositional temporal logic", POPL); Jansson and Jeuring 1997
("PolyP: a polytypic programming language extension", POPL); Hinze 2000 ("A new approach to generic functional
programming", POPL); the Itanium C++ ABI (its mangling of template arguments) and Rust's RFC 2603 (v0 symbol
mangling), cited for practice and not as a foundation.

They are cited from the texts as known; none was read again for this record. The passages the decisions rest on are to
be checked against the texts in the step that builds each: the statement of the abstraction theorem and its premise
(Reynolds 1983) and the free theorem C2 tests (Wadler 1989), for step 6; the substitution lemma as Pierce states it
for System F and for bounded quantification, for steps 3 and 4; coherence and the dictionary translation (Wadler and
Blott 1989) and the specialisation that removes dictionaries and its termination (Jones 1994), for step 4; polymorphic
recursion (Mycroft 1984), for C3; the applicative reading of functors (Leroy 1995), for C4; and the checking of an
intersection at each of its members (Reynolds 1996, Pierce 1991), for D4's numeric classes.

