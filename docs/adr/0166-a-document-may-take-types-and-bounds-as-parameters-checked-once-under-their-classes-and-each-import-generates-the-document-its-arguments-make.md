# ADR 0166 — A document may take types and bounds as parameters, checked once under their classes, and each import generates the document its arguments make

- Status: Accepted (2026-11-29) under the owner's standing rule, by the main session the owner delegated the decision
  to, after an independent review; amended on acceptance as listed in "Amended on acceptance".
- Built: nothing. No step of "Order of work" is built, and no test of it has been run.
- Date: 2026-11-28 (proposed); 2026-11-29 (accepted)
- Written under the owner's standing rule (the most long-term correct structure, textbook in its foundations, cost
  ignored) and the owner's scope rule of 2026-11-22: a facility is built on the backends a consumer needs, Rust by
  default; its contract is general; what is special to one application stays in the application. The request came from
  a downstream consumer with one piece of logic that is generic over an application's types; this record decides the
  general facility and names nothing of that consumer.
- Amends (ADR 0044, ADR 0086, ADR 0092 and ADR 0115, which carry "Amended by" lines, gain one on acceptance; every
  other amended document gains one in the landing that moves it):
  - RFC §synth-5-G (`docs/spec/synth/rfc-sce-protocol-synthesis.md`, "Parametric kinds"), deferred there: this record
    decides it, and answers its question Q10 (a parameter is declared by an element and written with a sigil, D1);
  - `SCE_FORGE.md`: a new §3.7 "Type parameters" (D1 to D10); §3.3 (`$P` wherever a type is written); §3.4
    (`_x.types.$W.bits`, `.min` and `.max`, which this record introduces, D2); §4.12 (an algorithm's
    `<sce:algorithm-param>`, D2, and `===` on a value of class `key`, D2); §5 and §5.4 (an import passes arguments,
    and how an instance is resolved, D3);
  - ADR 0086 §1 (the one type grammar gains `$P` for a type parameter and `[$N]` for a bound parameter, D2) and §5.1
    (an instance fits only an instance of the same generic at the same arguments, D3);
  - ADR 0092 §1 (a union's case may be a parameter of class `record`, or an instance of a parametric schema, D2);
  - ADR 0101 §1 (a CBOR entry's type may hold a parameter, and its plan is made per instance, D1);
  - ADR 0044 and ADR 0112 D5 (an instance is named in a saved state by its family's event and its arguments, each
    spelled as D5 spells a type so that a rename moves no shape; a revision adds a field whose type holds a parameter
    only where a reader takes its default without a literal; a revision of a generic keeps its parameters, D3);
  - ADR 0050 §1 and §2 (a bound parameter is read as `_x.limits.$N`, and its argument may be a named limit of the
    importer, D8);
  - ADR 0133 D1 (an operation on a parameter of a numeric class is total exactly where it is proven at every member of
    the class, D4);
  - ADR 0115 D7 and D10, and ADR 0156 D3 to D5 (a model, a contract's proof and a proof obligation are of an instance;
    the condition of data independence of a parameter is computed from the generic's tree, D7);
  - `docs/SCE_ACCEPTED_SUBSET.md` §2.15 (a variable of an instance's type, and an import with arguments) and the
    appendix (D10's codes); `SCE_ERROR_CONTRACT.md` (D10's codes);
  - `schemas/sce-forge-ext.xsd`, `apis/forge-ast.v1.schema.json`, `schemas/sce-sourcemap.v1.schema.json` and
    `schemas/sce-manifest.v1.schema.json` (D1, D3, D6), each a surface `SCE_WIRE_CONTRACTS.md` registers, extended
    without removing anything.
- Scope: `sce-build`: the grammar, the reader, the check once (D4), the elaboration of each instance and its judgement
  again (D5), the import closure, the names of the units and the command line's arguments (D6). An instance reaches
  every backend and the Interpreter as a written document of its kind (D6), so no runtime and no template changes. The
  numerical conformance harness changes: its fixture schema carries a generic and its arguments, each renderer reads
  an instance's fields, and the Interpreter's runner lowers an instance (Order of work, step 5); the lanes are held to
  instances on Rust first and then on every lane.
- Related: ADR 0005 (bound and declare), ADR 0050 and ADR 0086 (a bound by name, part of its type), ADR 0055 (optional
  values), ADR 0081 (a codec's plan, printed by each backend), ADR 0092 (unions), ADR 0101 (one deterministic CBOR
  form per type), ADR 0108 and ADR 0154 (maps, their keys and their storage), ADR 0109 D11 (a migration of a saved
  state), ADR 0118 (cost from the plan; Proposed), ADR 0133 (totality where the build proves it), ADR 0151 (a view,
  whose slots are of the same grammar), ADR 0156 (a system as documents, its models and obligations generated); ADR
  0168 (a polytypic, datatype-generic encoding: a format a document declares by one rule per type constructor, a
  record of its own drafted after ADR 0163 step 6). A parametric document cannot inspect its argument, so a serializer
  directed by its value's type is ADR 0168's and not this record's; this record supplies to it the parameters and
  their closed classes, derived and decided by entailment (an encoding of ADR 0168 reaches a generic as a class
  derived by that record's rules, as `cbor` is, and D7 counts its use as an operation on the value), the elaboration
  of each instance and its judgement again, the identity and the unit names of instances, and the generation on every
  lane with no erased type.

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
  Programming Languages*, MIT Press, chapter 23; chapter 26 for a variable bounded by a subtype). A variable bounded
  by a predicate on types, as a class is, is a qualified type: the judgement carries the predicate, and an instance is
  admitted where its argument satisfies it, a predicate holding where the declared ones entail it (Jones 1994a,
  *Qualified Types: Theory and Practice*, Cambridge University Press). A language that checks only its instances, as
  C++ checks a template's body at each instantiation, reports a generic's errors in the instance, at a place its
  author did not write.
- **A parameter that needs operations is bounded.** By a subtype bound (bounded quantification: Cardelli and Wegner
  1985, "On understanding types, data abstraction, and polymorphism", ACM Computing Surveys 17(4); Pierce 2002,
  chapter 26) or by a class whose instances supply the operations (type classes: Wadler and Blott 1989, "How to make
  ad-hoc polymorphism less ad hoc", POPL 1989). A class is *coherent* when the meaning of a program does not depend on
  which derivation of its typing is taken; one instance of each class per type, everywhere, is how a language buys
  that property; Wadler and Blott translate a class's operations to a dictionary passed as an argument. ML's modules
  pass them explicitly, as a structure given to a functor (MacQueen 1984, "Modules for Standard ML", ACM Conference on
  LISP and Functional Programming), and the two are one mechanism that differs in how an instance is found (Dreyer,
  Harper, Chakravarty and Keller 2007, "Modular type classes", POPL 2007). An operation whose precondition is weaker
  and whose postcondition is stronger than those required may stand for it (behavioural subtyping: Liskov and Wing
  1994, "A behavioral notion of subtyping", ACM TOPLAS 16(6)); the match of a component's specification to the one
  required is the plug-in match, where the required precondition implies the component's and the required precondition
  with the component's postcondition implies the required postcondition (Zaremski and Wing 1997, "Specification
  matching of software components", ACM TOSEM 6(4)). Equality as a class of types is Standard ML's equality types
  (Milner, Tofte, Harper and MacQueen 1997, *The Definition of Standard ML (Revised)*, MIT Press) and Go's
  `comparable` (*The Go Programming Language Specification*, since Go 1.18, 2022).
- **A finite class is checked at each of its members.** A term of the intersection of finitely many types is checked
  at each of them (Reynolds 1988, "Preliminary design of the programming language Forsythe", Carnegie Mellon
  University, CMU-CS-88-159, reprinted in O'Hearn and Tennent (eds.), *ALGOL-like Languages*, vol. 1, Birkhauser 1997;
  Pierce 1991, *Programming with intersection types and bounded polymorphism*, PhD thesis, Carnegie Mellon
  University).
- **Three ways to run a generic.** *Erasure*: one body for every instance, its types erased to one representation and
  recovered by casts (Bracha, Odersky, Stoutamire and Wadler 1998, "Making the future safe for the past: adding
  genericity to the Java programming language", OOPSLA 1998). *Type passing*: the run time carries the type and
  specialises when it is first used (Kennedy and Syme 2001, "Design and implementation of generics for the .NET Common
  Language Runtime", PLDI 2001). *Monomorphisation*: the compiler writes one copy per instance, each over concrete
  types (C++'s templates, Stroustrup 1988, "Parameterized types for C++", USENIX C++ Conference; MLton, Weeks 2006,
  "Whole-program compilation in MLton", ACM SIGPLAN Workshop on ML; Rust). A class's dictionaries are removed the same
  way, by specialising each use to its instance (Jones 1994b, "Dictionary-free overloading by partial evaluation", ACM
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
- **A program that only moves the values of a type is data independent in it**, and a few values stand for all (Wolper
  1986, "Expressing interesting properties of programs in propositional temporal logic", POPL 1986); a program that
  also compares them for equality is data independent under the later extension of the theorem (Jonsson and Parrow
  1993, "Deciding bisimulation equivalences for a class of non-finite-state programs", Information and Computation
  107(2); Lazic 1999, *A semantic study of data independence with applications to model checking*, DPhil thesis,
  University of Oxford; Lazic and Nowak 2000, "A unifying approach to data-independence", CONCUR 2000). Parametricity
  is the same fact stated of a type, and ADR 0156 D5 decides when a verdict may use it.
- **The whole of this record has a precedent in Ada's generics**: an instantiation is explicit; a formal type belongs
  to a category the language defines (ISO/IEC 8652:2012, *Ada Reference Manual*, §12.5), and an operation beyond the
  category is a formal subprogram given at the instantiation (§12.6); the body is checked once, its legality rules
  assuming the worst of what a formal may be (§12.3(11) and its annotation in the *Annotated Ada Reference Manual*);
  and GNAT, the common compiler, makes each instance a copy. SPARK, Ada's proved subset, proves a generic per instance
  (AdaCore, *SPARK User's Guide*), as D7 does.

## Decision

### D1. A document may declare parameters, and four kinds take them

A document declares its parameters as the first children of its `<scxml>` root, before its imports. The record of the
observed-remove set, generic in its element:

```xml
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0"
       sce:kind="event-schema" name="orset_entry" sce:event-name="merge.orset_entry">
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

  A `view` (ADR 0151) holds its operators' integrated state between two changes (its D4), so it does not meet the
  criterion; a view's parameters are left to that kind's own record (Open question 9).
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
| (none) | every bounded type | what the table of rules (D4) admits of every type where it stands; the table is the statement of it, and this record does not repeat it | |
| `present` | a type that is not an `optional` at its top | `optional<$T>` | |
| `wire` | a type that holds no map and no set at any depth | to stand in an event schema's field, in a CBOR entry, and inside another type of an algorithm's slot (ADR 0108 D7) | |
| `key` | a key of ADR 0108 D1 that a body compares with `===`: an integer, `bool`, a value of one enum, a bounded text, and a flat record, one whose every field is one of these | `===` and `!==`, with the equality ADR 0108 D1 gives a key; to be a map's key and a set's element | `present`, `wire` |
| `entry` | a fixed-width number, `bool`, an enum or a bounded text: what one field of a record an algorithm takes holds (SCE_FORGE §4.12) | to be a field of a record an algorithm names | `present`, `wire` |
| `scalar` | a fixed-width number, `bool` or an enum | to be held in a local initialised or assigned whole, and to be the element of a list an algorithm holds (SCE_FORGE §4.12 refuses a text element there, and builds a record local by `<sce:set>` alone) | `entry`, `fixed` |
| `fixed` | a fixed-width number, `bool`, an enum, a record or a union: a type that writes no bound at its top | a value of an algorithm's parameter declared `$T`, a view (D3), stands where a value of `$T` stands | `present`, `wire` |
| `record` | a record type, an instance of a parametric schema included | to be a union's case (ADR 0092 §1), and the element of a list of records an algorithm holds | `fixed` |
| `cbor` | a type that holds no map and no set at any depth, which therefore has its one deterministic CBOR form (ADR 0101 §1) | to be imported a CBOR codec at, or at a type that holds it: an operation that reads the value's bytes, which D7 counts | `wire` |
| `integer` | the eight integer types | the integer arithmetic, comparisons, bit operations and literals of the expression language; `_x.types.$W.bits`, `.min` and `.max` | `key`, `scalar` |
| `unsigned` | `uint8` to `uint64` | as `integer` | `integer` |
| `signed` | `int8` to `int64` | as `integer` | `integer` |
| `float` | `float32` and `float64` | the arithmetic, comparisons and literals of a float; `_x.types.$W.bits` | `scalar` |

- **Nothing else is granted.** A value of a parameter's type is no operand, no condition, no number, no text and no
  record whose fields can be read, but through a class it holds, an algorithm parameter whose signature names the
  parameter's type (below), or a CBOR codec imported under `cbor`. Each of these is an operation that reaches the
  value, so the classes alone do not say what a generic does with a parameter: D7 computes it from the generic's tree,
  and the premise of the abstraction theorem (Reynolds 1983), on which C2 of D9 rests, holds of a parameter exactly
  where that computation finds it only moved, or moved and compared for equality.
- **A key's equality is the map's.** `===` at a parameter of class `key` is the equality ADR 0108 D1 defines, by which
  a map tells its keys apart: a text and a byte string by their bytes, a record field by field. Where an instance's
  argument is a record, which `key` admits only flat, the instance writes the comparison as the conjunction of its
  fields' comparisons in the schema's order, which is the same relation in the form a body already writes (SCE_FORGE
  §4.12 compares a record's fields and not a record), so no lane is given a new operator. A key ADR 0108 admits whose
  comparison a body does not write (a byte string outside the comparisons SCE_FORGE §4.12 lists) is not of the class:
  the class grants only what every one of its instances can write, and it is widened in the step that teaches a body
  the comparison, with no rule of its own.
- **Why `union` and `enum` are not classes.** A generic gains no operation from knowing that its argument is a union
  or an enum: it cannot name the cases or the variants of a document it does not know. An enum argument is admitted by
  `key`, which grants what an enum can do in a body that does not know its variants (be compared and be a key). A
  class that grants nothing and that no position needs would narrow the arguments for no reason (Open question 2).
- **Why `record` is.** A union's case is a record or none (ADR 0092 §1), so a generic union whose case is a parameter
  needs its argument to be a record; that is the one position of the language that admits only records, beside a list
  of records an algorithm holds, and the class grants nothing else. `scalar`, `fixed` and `cbor` were added on
  acceptance, each because a rule of the language needs it (D4).
- **The numeric classes are finite**, and the whole judgement of D4, its typing included, runs at each member, since
  what an operation means depends on the width it is emitted at (ADR 0133 D1). `_x.types.$W.bits`, `.min` and `.max`
  read the width and the least and greatest value of the argument, written as the literals of the instance before
  anything reads the tree, as `_x.limits.<name>` is (ADR 0050 §2). In the generic, `bits` is a `uint32`, and `min` and
  `max` are of the type `$W`; each has its value at each member when D4 checks there.

**A bound parameter**, `<sce:bound-param name="N" min="K"/>` (`min` 1 when it is absent), ranges over the bounds of
ADR 0086 at or above `min`: positive integers that fit in 32 bits (ADR 0050 §3). It stands where a bound stands,
`[$N]`, and as the greater count of `[J..$N]` where `J` is at most `min`. An expression reads it as `_x.limits.$N`, a
`uint32` whose value the generic knows only to be in `min` to 2^32-1, so it lands only where every such value lands. A
bound parameter exists because a name has one number in a build (ADR 0050 §1): two uses of one document that need two
bounds in one build need the bound to be an argument (D8).

**An algorithm parameter**, `<sce:algorithm-param name="before">` holding a `<sce:signature>` over the document's type
parameters and the types it imports, with its `may-fail`, ranges over the algorithms whose signature fits it once the
type arguments are in place (D3). It carries no contract: a contract on `<sce:algorithm-param>` is refused
(`type-param/algorithm-param-contract`) until ADR 0115 D10 gives a signature one. Today `<sce:require>` is a statement
of a `may-fail` algorithm's body (SCE_FORGE §4.12) and `<sce:ensure>` is not yet of the kind; when both are, an
argument fits a declared contract by the plug-in match (Zaremski and Wing 1997): the declared precondition implies the
argument's, and the declared precondition with the argument's postcondition implies the declared postcondition. A call
of an algorithm parameter whose signature names a type parameter is an operation on that parameter (D7). The generic
calls it by its name, as it calls an imported algorithm, by an expression or `<sce:call>`. It is how a generic is
given an operation no class grants (an order on its records, a read of a field, a hash): the operation is named at the
import, as a structure is given to a functor (MacQueen 1984), so which algorithm runs is written where the instance is
made, and the instance calls it directly (D5). Classes an author declares, with instances the build would find, are
not offered (Open question 3).

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
  arguments: each type argument spelled by the grammar, with each record, union and enum by its document's identity,
  the identity the build's types are told apart by (SCE_FORGE §4.12, `TypeIdentity::same_record`), and never by the
  alias it was written with (a saved state spells an argument otherwise, below); each bound by the number the build
  resolved it to; each algorithm argument by the identity of the document it names. Two imports with equal canonical
  arguments name one type and one unit, in one document or in two, as Rust's and C++'s generics do (Leroy 1995's
  applicative reading): `record:TagEntry` above is the record that the algorithm's `record:Entry` names at the same
  argument, so the importer hands `tagsUnion` its lists. A bound argument that is a named limit gives the identity its
  number alone: `string[tag_text=32]` and `string[label_text=32]` are one argument and name one instance, and each
  name stays its importer's constant (D8).
- **An instance fits only itself.** `fits` (ADR 0086 §5.1) admits an instance where the same generic at equal
  canonical arguments is expected. Instances are invariant in their arguments, as a record type is its document:
  `orset_entry` at `string[16]` does not fit `orset_entry` at `string[32]`, though a `string[16]` fits a `string[32]`
  (Open question 4).
- **An algorithm's parameter declared `$T` is a view of `$T`.** An algorithm's parameter is a view of its argument and
  writes no outermost bound (SCE_FORGE §4.12). In the generic, a value of a parameter declared `$T` at its top is of
  the type "view of `$T`": it flows wherever a view flows (handed whole to a parameter of `$T` of another algorithm,
  compared where `key` holds, read where a class grants), and stands in a place of `$T` (a local, a return, a field,
  an element) only where the parameter holds `fixed`, whose every member writes no bound at its top, so that the view
  is the value; anywhere else it is refused (`type-param/class-not-held`). In the instance the parameter is written as
  the argument without its outermost bound, a `string` parameter at `string[32]`, and every other slot writes the
  argument whole. So `id(x: $T) -> $T` is refused for a `$T` without `fixed`, where at `string[32]` it would return a
  `string` view as a `string[32]`, which SCE_FORGE §4.12 refuses to cut.
- **An instance in a saved state** is named as ADR 0112 D5 names a record: by its family's event (the `sce:event-name`
  of the generic, which names the family and no event) followed by its arguments, `merge.orset_entry<string[32]>`,
  with the generic's revisions and lineage. Each argument is spelled there as D5 spells a type in a shape, so that a
  file renamed moves no shape: a record by its event and its first revision's fields, an enum by its first revision's
  variants, a union by its tag and its cases' records so spelled, and every other type and a bound by the grammar. The
  instance's line is D5's line of the family's first revision with each parameter replaced by its argument so spelled,
  under that name. A revision of the generic is a revision of every instance (ADR 0044). A revision may add a field
  whose type holds a parameter only where a reader takes the field's default without a literal: an `optional<$T>`,
  read absent, or a list, read empty; or, for a parameter of a numeric class, a literal default that D4 checks at
  every member. No other literal is of every type, so any other such field is refused
  (`type-param/added-field-without-default`). A revision may not add, remove or re-class a parameter
  (`type-param/revision-changes-parameters`): a generic with other parameters is a new family, under an event of its
  own. A variable that held an instance at other arguments held another type, and its change of type is a migration's
  (ADR 0109 D11), as any change of a variable's type is.
- **The arguments of one document's imports name no cycle.** Within one document, the imports whose arguments name
  another import's alias (a type argument `record:B`, an algorithm argument `ref="z"`) form a graph, and it is
  acyclic: `A` instantiated at `record:B` beside `B` instantiated at `record:A` would be a type that holds itself, and
  `x` given `z` beside `z` given `x` an identity of infinite depth and a recursion between instances. Such a document
  is refused (`type-param/argument-cycle`), naming the aliases on the cycle. The document import graph stays acyclic
  in both cases, which is why the rule is its own.
- **An instance of a parametric schema is a record, and no event's schema.** An event's name names one schema on the
  network (ADR 0044), and a family is many schemas, so a statechart that takes or sends the family's event with an
  instance as its schema is refused (`type-param/instance-as-event`); a schema of its own holds the instance as a
  field.

### D4. What the build checks once, on the generic

A generic document is checked where it is read, with its parameters abstract, by every check a written document of its
kind is checked by, under these rules. A refusal is reported at the generic's line, and needs no instance.

- **Typing.** Every expression and statement is typed with a parameter of no numeric class as a type of its own, equal
  only to itself (a numeric parameter is typed at each member, below). An operation on a value of it is one a class it
  holds grants (D2), or it is refused (`type-param/class-not-held`), naming the operation, the parameter and the class
  that grants it, with the fix that adds the class to `of=`.
- **Positions: assume the worst.** As Ada judges a generic body (ISO/IEC 8652:2012 §12.3(11)), a rule is met at a
  parameter only where it would be met at every type the parameter may be. Every rule of the language that admits a
  type by a condition states the condition, in one table of rules the build reads; at a parameter the rule admits it
  only where the parameter's declared classes entail the condition (entailment of predicates, Jones 1994a), and
  refuses it otherwise with `type-param/class-not-held`, naming the rule and the class that would entail it, or, where
  no class does, the rule alone. A class joins D2 when a rule needs one that none of D2's entails; `scalar`, `fixed`
  and `cbor` joined on acceptance this way. The rules known today: `optional<$T>` needs `present` (ADR 0086 refuses an
  optional of an optional); a map's key and a set's element need `key` (ADR 0108 D1); an event schema's field, a CBOR
  entry, and a type inside another type of an algorithm's slot need `wire` (ADR 0108 D7: no map stands there), and so
  does the return of an algorithm that does not declare `may-fail` (SCE_FORGE §4.12: a map return declares it); a
  field of a record an algorithm names needs `entry` (SCE_FORGE §4.12: such a record holds entries, and a list of
  numbers or `bool`s on the backends that lend one); the element of a list an algorithm holds needs `scalar` or
  `record` (SCE_FORGE §4.12 refuses a text element, and a list of records is a rule of its own); a local initialised
  or assigned whole needs `scalar` (SCE_FORGE §4.12 builds a record local by `<sce:set>` alone, with no copy in its
  `init` and no assignment of the whole); a value of a parameter declared `$T` in a place of `$T` needs `fixed` (D3);
  a union's case needs `record` (ADR 0092 §1); and a CBOR codec whose entry holds a parameter declares
  `sce:encode-may-fail`, since whether its encode can refuse is decided per type and per backend (ADR 0101 §5.2) and
  no class entails that it cannot. A type that contains itself cannot be written through a parameter: an argument is
  fixed before its instance is named, and the arguments of one document's imports name no cycle (D3), so no instance
  holds its own identity.
- **Numeric classes, at each member.** Where a parameter holds `integer`, `unsigned`, `signed` or `float`, the whole
  judgement, its typing included, runs once for each member of the class, since SCE_FORGE §3.4 lets a number stand at
  another width and ADR 0133 D1 judges each operation at the width it is emitted at: that a literal is of the type,
  that an operation is total where ADR 0133 asks it to be proven, that a shift's count is below `_x.types.$W.bits`,
  and every other. With several numeric parameters it runs at each combination of their members, and its cost is that
  product (at most 8 per parameter, so 64 for two). A refusal names the member, or the combination, it fails at. A
  document accepted for `$W` of `unsigned` is therefore accepted at `uint8`, `uint16`, `uint32` and `uint64`, and ADR
  0133 D1 holds of each: an operation is proven total exactly where it is proven at every member.
- **Bound parameters.** `_x.limits.$N` is a `uint32` in the range `min` to 2^32-1, and a lesser count `J` in `[J..$N]`
  is held to `J <= min`.
- **Imports inside a generic.** An argument that is the importer's own parameter holds the classes the imported
  parameter needs where the importer's declaration implies them (the column "Implies" of D2), or
  `type-param/argument-outside-class` is reported at the importing generic; an algorithm parameter handed on is held
  to the inner signature as an algorithm argument is (D3).
- **What the check once is designed to give.** Its design target is that, for every argument that holds its
  parameter's classes, the instance passes every rule of the language the generic passed: the substitution lemma for
  qualified types (Pierce 2002, chapter 23; Jones 1994a), for a language whose rules are stated as conditions a
  parameter's classes entail and whose numeric classes are finite. It is a target held by C1's test (D9), not a proof:
  the guarantee on the code generated is D5's judgement of every instance again, and what the check once buys is that
  an author meets a generic's errors at the generic, and that D7's computation and C2 have their premise. What depends
  on a number the arguments bring is the instance's (D5).

### D5. Each instance is generated as the document its arguments make, and judged again

- **Elaboration.** Once the imports of a set are read and each generic is checked (D4), the build makes each instance
  by substituting its canonical arguments into the generic's tree: each `$T` by its type (D3 for an algorithm's
  parameter at its top), each `[$N]` by its bound, each call of an algorithm parameter by a call of its argument, each
  comparison of D2's `key` in the form D2 gives it, and `_x.types.$W.*` and `_x.limits.$N` by the instance's literals.
  The instance is a written document of the generic's kind, named as D6 says. Each node keeps the generic's position
  and the chain of imports that made the instance, so a diagnostic, a line of the sourcemap (RFC §synth-5-O) and a
  trace name the generic's line and the arguments.
- **The meaning of a generic is the meaning of its instances.** No generic runs. A document with parameters has no
  code, no model and no behaviour of its own, only the family of its instances, and each instance means what the
  document written with its types would mean. Nothing is left to prove of the elaboration but that it is the
  substitution, which D9 holds (C5).
- **The instances are finitely many, and the elaboration ends.** Two orders hold, and the induction runs over both.
  The import graph of documents is acyclic (an import cycle is refused where the imports are walked,
  `sce-build/src/forge/cross_kind_check.rs`), and within one document the graph of which import's arguments name which
  alias is acyclic (`type-param/argument-cycle`, D3). Every argument is written in the importer's names, and an
  instance's own imports are its generic's, which lie below the generic in the import graph. So by induction on a
  document's height in the import graph first, and, among the imports of one document, on the order of the argument
  graph second, the instances made from it are finitely many and each identity is finite. No generic can reach itself
  at another argument, which is the polymorphic recursion (Mycroft 1984) that would make them infinite.
- **Each instance is judged again, by every check a written document is judged by** (Pnueli, Siegel and Singerman
  1998; ADR 0156 D11). Three kinds of refusal can come of an instance, and only three, each named on the instance with
  the import that made it:
  - *a number*: what the arguments' sizes make of the instance, its largest encodings (ADR 0086 §2), a C11 slot (ADR
    0086 §8), its cost (ADR 0118), a size past what a lane holds. These refuse the instance as they would refuse a
    written document of the same numbers; the generic is not at fault;
  - *a lane*: a construct the instance holds that a backend does not lower (a record that holds a list on a backend
    `ALGORITHM_LISTED_RECORD_BACKENDS` does not list, a map on one `ALGORITHM_MAP_BACKENDS` does not list), refused on
    that backend alone by its existing code (`generate/unsupported-feature`);
  - *a proof*: an obligation proved per instance fails at one (a contract of ADR 0115 D10 proved by Verus on the
    instance's Rust, and, once signature contracts exist, an algorithm argument's plug-in match of D2). The formal
    judge reports it as the verdict of that instance, `violated` or `not judged` with `formal/verdict` (ADR 0146),
    naming the generic and the arguments; the instance is generated, since a verdict is the judge's and not a refusal
    of the build, and a gate that requires the proof fails on it.

  A refusal by a rule of the language that D4 covers is a defect of the check once, not of the document: the instance
  is refused with its own refusal and with `type-param/instance-breaks-its-generic`, naming the generic, the arguments
  and the rule; nothing of it is generated; and C1's property test (D9) holds that it is never reported.

### D6. One unit per instance, named by its generic and its arguments, on every lane and the Interpreter

- **One unit per instance in a set.** `sce-build/src/import_closure.rs` follows an import with arguments to its
  instance, and a set generated together (`generate --with-imports`, `orchestrate`) generates each distinct instance
  once, where the generic's unit would be placed.
- **The unit's name** is the generic's name followed by an injective encoding of its canonical arguments (D3), each
  record, union and enum by its document's identity: the grammar's punctuation (`<`, `>`, `[`, `]`, `,`, `:`, `.`)
  written as escape letters and each name as a segment prefixed by its length, as the Itanium C++ ABI and Rust's v0
  mangling encode a template's arguments. The encoding is written once, beside `mangle` in
  `sce-build/src/forge/symbol_mangling.rs`, whose `mangle(machine, state path, artifact)` escapes only `__` and is not
  injective over this grammar. Two names are therefore equal only where the canonical arguments are, and a collision
  can come only from shortening.
- **Identifier limits.** On C11, ISO/IEC 9899:2011 §5.2.4.1 keeps 63 significant initial characters of an internal
  identifier or a macro name and 31 of an external one. The units of the four kinds declare no external identifier: an
  algorithm's, a union's and a CBOR codec's functions are `static inline` in their headers, and a schema's unit
  declares types and macros, whose tags have no linkage. So an instance's identifiers are held to 63 characters. Under
  `platform.strict_c99_identifiers`, a name past that is written as a prefix and a digest of the canonical argument
  text so that it fits, and the manifest lists the name with the instance; without the flag the mangler's existing
  warning stands. Two names are judged one where their significant initial characters are (C11 §6.4.2.1p6), and two
  instances of one set so judged one are refused (`type-param/instance-name-collision`).
- **The manifest and the sourcemap.** The manifest lists each instance with its generic, its canonical arguments and
  its unit, and the sourcemap maps each generated line to the generic's line and the instance.
- **No type is erased.** Every item an instance's unit declares is over concrete types. On Rust it is the code a
  written document of those types would generate: no generic parameter on an item it declares, no `dyn`, no `Any`, no
  boxed value, and no tag of a type at run time; a runtime type it uses (`BoundedMap<K, V, N, S>`) is applied to
  concrete types where it is named. Every other lane holds an instance's values in the types it holds a written
  document's in. A lane's own generics are not used for a generic document (Alternatives rejected): one elaboration
  serves every lane, and the code that runs is the code its proofs and models are of (D7).
- **A generic alone generates nothing.** `sce-codegen generate` and `lower-algorithm` of a generic document with a
  parameter left without an argument are refused (`type-param/uninstantiated`). `--type-arg NAME=TYPE`,
  `--bound-arg NAME=BOUND` and `--algorithm-arg NAME=DOCUMENT`, read as the import's elements are, name the instance a
  host calls with no document that imports it. They bind the parameters of the root document alone: a parameter's name
  belongs to one document, unlike a limit's (ADR 0050), so they are refused beside `--with-imports` and with
  `orchestrate` (`type-param/argument-flag-in-a-set`). `sce-codegen check` of a generic alone runs D4.
- **Every backend and the Interpreter serve an instance, because what reaches them is a written document.** Rust, C11,
  C++, Go, Kotlin, Python and Swift print it through the templates they print its kind through, and the Interpreter
  runs a `sce-static` document as `static_js` lowers it after the elaboration, its algorithms among it. A lane refuses
  an instance exactly where it would refuse the written document (D5), by name. Under the owner's scope rule the first
  lane held to instances is Rust, the one a consumer needs; the others are held in the same step (Order of work, step
  5), which changes the conformance harness (its fixture schema, its renderers and the Interpreter's runner) and no
  lane.
- **The forge-AST and the TTCN-3 export.** `apis/forge-ast.v1.schema.json` carries a generic with its parameters, and
  each instance as the document it is; the test profile's TTCN-3 export (ADR 0091) names an instance's types as types
  of their own.

### D7. Models, contracts and proof obligations are of an instance

- **The model generator reads instances only.** `sce-codegen model` (ADR 0115 D7, ADR 0156 D3) is given a statechart,
  which takes no parameters (D1): a statechart that declares one is refused where it is validated
  (`type-param/kind-not-parametric`), before any model. The generator translates the algorithm bodies a statechart
  calls (ADR 0115 D7), and for an instance it reads the document the one elaboration of D5 produced; it never resolves
  an import's `src` itself, so it meets written documents only and has no translation to add for this record. A
  statechart that holds an instance is refused today as every `sce-static` machine is (`Construct::Datamodel`,
  `formal/no-translation`), and is translated by the step of ADR 0156 D3 that translates `sce-static`, with nothing
  further here. A generic alone never reaches the generator, so it has no refusal of its own there. ADR 0119's second
  encoding reads the same instances.
- **An algorithm's contract is proved per instance** (ADR 0115 D10): the proof is of the code that runs, and the code
  that runs is the instance's. A contract written in the generic over `$T` is the instance's contract at its
  arguments, and, once ADR 0115 D10 gives a signature a contract, an algorithm argument's plug-in match (D2) is
  discharged in the instance, where both contracts are proved; a proof that fails at one instance is D5's third kind
  of refusal.
- **The proof obligations ADR 0156 D4 generates** (`TypeOK`, an invariant per bounded structure) are of the instance's
  types and bounds, which are concrete.
- **Data independence, computed from the generic's tree.** The classes do not bound what a generic does with a
  parameter (D2): an algorithm parameter whose signature names `$T` may compare or compute on it, and a CBOR codec
  imported at it under `cbor` reads its bytes, and judging the instances again cannot see either. So the condition is
  computed, once, from the generic's tree: every operation that reaches a value of `$T`, its own, those of the
  instances it imports at an argument that holds `$T`, every call of an algorithm parameter whose signature names
  `$T`, and every CBOR codec imported at it. A parameter that every such operation only moves meets Wolper's condition
  (Wolper 1986), and one that they only move and compare for equality meets its extension (Jonsson and Parrow 1993;
  Lazic 1999; Lazic and Nowak 2000); any other operation, `cbor` and an algorithm parameter's call among them, meets
  neither. ADR 0156 D5's analysis of a machine's types reads this computed condition for the part of the machine an
  instance contributes, and decides as before for the rest; no verdict uses it before ADR 0156 D5 states the threshold
  its theorem gives.

### D8. Bounds by name

- A bound parameter's argument is a number, a named limit of the importer (`tags_max=256`), or a bound parameter of a
  generic importer. A named limit stays the importer's: it is among the importer's `named_limits` (ADR 0050 §1), moved
  by `--limit tags_max=N`, and declared as the importer's typed constant (ADR 0050 §3). The instance holds its number
  (D3), and its unit declares the number alone: two importers that name one number differently (`string[tag_text=32]`,
  `string[label_text=32]`) share one instance, and each name stays its importer's constant.
- A named limit written inside a generic (`[merge_entries=1024]`) stays one name with one number in the build (ADR
  0050 §1), shared by every instance. That is the line between the two: a name is a deployment's knob, and a bound
  that must differ between two instances in one build is a parameter.
- The facility adds no resource at run time (nothing is erased, dispatched or kept for a type, D6), and so no bound of
  its own. Its cost is at the build and in code: one unit per instance, which the manifest lists (D6) and ADR 0118's
  cost reads as it reads a written document's.

### D9. What each claim is judged by

| Claim | Level | Why that level |
|---|---|---|
| C1. An instance of an accepted generic, at arguments of its classes, passes every rule of the language its generic passed: the design target of D4 | Every instance judged again (D5), and a property test of the build, `every_instance_of_an_accepted_generic_is_accepted`, whose generator ranges over every statement and expression form of SCE_FORGE §4.12 applied to a value of each parameter, not over D4's list of rules | The claim is the substitution lemma of SCE's static semantics, and that semantics is not yet an object a prover reads (ADR 0156 D2 decides to write it), so no theorem can be stated over it yet. The code does not rest on the claim: every instance is judged again and refused if a rule fails (ADR 0156 D11). The test holds the check once to its target, so that the judgement again never refuses what a generic was promised; a generator drawn from D4's own list could not find a rule the list lacks, which is why it ranges over the forms of the language |
| C2. An instance commutes with a renaming of a parameter that D7's computation finds it only moves (any map of its values) or only moves and compares for equality (an injective map) | A property test on every lane, by self-composition (below) | The abstraction theorem holds of such a parameter because nothing else reaches its values; of a parameter an algorithm argument or a CBOR codec reaches, it says nothing, and C2 claims nothing. What could break it is a lane whose lowering reads a representation (an order of values where the document kept an order of arrival), which only running the lanes shows |
| C3. The instances of a set are finitely many, each identity is finite, and the elaboration ends | The argument of D5, by induction on a document's height in the import graph first and on the order of one document's argument graph second, and a test that two imports whose arguments name each other are refused (`type-param/argument-cycle`) | The argument is short and complete; the test holds the premise that is new here (an import cycle of documents is refused already, by `cross_kind_check.rs`, and so tests nothing of this record) |
| C4. Two imports name one unit exactly when their canonical arguments are equal, and an instance fits another exactly when they are | A property test of the build over random pairs of arguments, each direction of each equivalence a property | A function of the canonical text, decided by comparing it; a test of single cases would not go red for an encoding that merges two arguments it did not meet |
| C5. An instance means what the document written with its types means | By definition (D5); a test that a generic `orset_union` instantiated at `uint64` is, node for node, the tree of `stdlib/merge/orset_union.scxml` as it is written today; and a differential property test over C1's generics, each instance's tree against an oracle that substitutes the arguments into the generic's XML text and reads the result as an ordinary document | The meaning of a generic is defined as its instances'; the tests show that the elaboration writes that document, at one hand-written case and at every generated one |
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
| `type-param/class-not-held` | Validation | an operation or a rule whose condition the parameter's classes do not entail (D4), naming the rule and, where one exists, the class whose addition is the fix |
| `type-param/algorithm-param-contract` | Validation | a contract on an `<sce:algorithm-param>` before ADR 0115 D10 gives a signature one (D2) |
| `type-param/argument-missing` | Import | an import of a generic that leaves a parameter without an argument (D3) |
| `type-param/argument-unknown` | Import | an argument that names no parameter, or any argument on an import of a document that has none (D3) |
| `type-param/argument-cycle` | Import | imports of one document whose arguments name one another's aliases in a cycle (D3), naming the aliases |
| `type-param/argument-outside-class` | Import | an argument outside its parameter's classes, a bound below its `min`, or an algorithm whose signature does not fit (D3, D4) |
| `type-param/added-field-without-default` | Validation | a revision that adds a field whose type holds a parameter and is neither an optional nor a list (D3) |
| `type-param/revision-changes-parameters` | Validation | a revision of a generic that adds, removes or re-classes a parameter (D3) |
| `type-param/instance-as-event` | Validation | an instance of a parametric schema taken or sent as an event's schema (D3) |
| `type-param/uninstantiated` | Cli | `generate` or `lower-algorithm` of a generic with a parameter left without an argument (D6) |
| `type-param/argument-flag-in-a-set` | Cli | `--type-arg`, `--bound-arg` or `--algorithm-arg` beside `--with-imports` or with `orchestrate` (D6) |
| `type-param/instance-name-collision` | Generate | two instances of one set whose unit names agree in their significant initial characters after shortening (D6) |
| `type-param/instance-breaks-its-generic` | Validation | an instance refused by a rule the check once covers: a defect of the check, reported with the instance's own refusal (D5) |

An unknown class in `of=` is `validation/invalid-attribute`, its candidates the classes of D2. A number or a lane that
refuses an instance does so by the code it has today, and a proof by the formal judge's verdict (D5).

## Not in scope

- A statechart's parameters. Its types reach the events it takes and sends, and an event's name names one schema on
  the network (ADR 0044), so an instance of a statechart needs a rule that names its events per instance. That rule is
  a record of its own, a statechart's parameters and the names of its instances' events, not yet numbered; it would
  use D2 to D10 unchanged, and the first document it would serve is `stdlib/shared/record_list.scxml`, which erases
  its rows' types today.
- Parameters of the kinds D1 refuses, `view` among them, whose parameters are that kind's own record's to decide.
- A parameter that is itself generic (higher kinds), an argument that is a generic not yet applied (higher rank),
  polymorphic recursion, and computation on types beyond `_x.types.$W.bits`, `.min`, `.max` and `_x.limits.$N`.
- Classes an author declares, with instances the build finds (D2 closes the list, and an operation no class grants is
  an algorithm argument); bounds on a record's fields (row polymorphism; records are nominal, ADR 0086 §5.1); variance
  between instances; default arguments; arguments inferred at a call.
- A type-directed encoding of an application's own, a format defined by a rule per type constructor (polytypic
  programming: Jansson and Jeuring 1997, "PolyP: a polytypic programming language extension", POPL 1997; Hinze 2000,
  "A new approach to generic functional programming", POPL 2000). A parametric document cannot inspect its argument,
  which is what parametricity says; a format over every type is a codec encoding, as ADR 0101 is for CBOR, and a
  format declared by one rule per type constructor is ADR 0168, on the mechanism this record supplies (Related).
  Within this record a generic reaches a value's bytes through a CBOR codec instantiated at its type, under the class
  `cbor` (D2), which D7 counts as an operation on the value.
- Converting the standard documents that fix a type today (Open question 12).
- Run-time type information, erasure and boxing, on every lane.

## Consequences

- An algorithm, a record, a union or a CBOR codec is written once and used at an application's types, with the bounds
  each use needs; what is fixed by hand today becomes an argument.
- The guarantee on the code generated is D5's: every instance is judged again by every check, and none that breaks a
  rule is generated. The check once (D4) buys that a generic's errors are reported at the generic, at its author's
  line, once, and gives D7's computation and C2 their premise. An importer sees the refusals of its arguments, a
  number of its instance, a lane, or a proof.
- Every lane and the Interpreter run instances with no change of their own, and no generated code carries an erased
  type, a dispatch on a type or a description of a type at run time.
- Each instance is a unit of code: a program grows with the instances it uses, as with every monomorphising compiler,
  and the manifest says which. On a microcontroller the flash the instances take is measured as a deployment measures
  the size of every unit of its code.
- A model, a proof and a trace are of instances, which are written documents to them; the condition of data
  independence of a generic's parameter is computed from its tree, counting algorithm arguments and CBOR codecs as
  operations.
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
   it. A documentation step, exempt from the rule of a red test: it changes no behaviour a test could observe, and the
   behaviour it describes is held by steps 2 to 6.
2. The grammar and the reader: `$P` in `sce-build/src/forge/value_type.rs` (a constructor for a type parameter, and a
   bound that may be a parameter), the three declarations, the three arguments on `ForgeImport`
   (`sce-build/src/forge/model.rs`, `sce-build/src/forge/parser.rs`), `schemas/sce-forge-ext.xsd` and the forge-AST:
   `sce-build/tests/a_document_declares_its_parameters_and_an_import_passes_their_arguments.rs`. Red without it:
   `<sce:type-param>` is an element the XSD refuses, and `$E` an unknown type. It holds `kind-not-parametric`,
   `unknown-parameter`, `parameter-unused`, `argument-missing`, `argument-unknown`, `algorithm-param-contract` and
   `argument-flag-in-a-set`, each with a case.
3. The check once (D4), with its refusals:
   `sce-build/tests/a_generic_document_is_checked_once_under_the_classes_of_its_parameters.rs`, the table of rules D4
   reads, an accepted case for each class, and a refused case for each row of D10 that a generic or its import
   decides: `class-not-held` at each rule of the table (among them a `list<$E>` an algorithm holds with `E` of `key`
   alone, refused since a text is a key and no list element; a local of `$T` initialised whole without `scalar`;
   `id(x: $T) -> $T` without `fixed`; a CBOR codec at a parameter without `sce:encode-may-fail`),
   `argument-outside-class` for a class, a `min` and a signature, `added-field-without-default` (and a numeric
   parameter's literal default accepted at every member), `revision-changes-parameters` and `instance-as-event`; and a
   numeric case refused at `uint8` alone (a literal `0x1FF` held in a `$W` of `unsigned`), naming the member, and one
   refused at one combination of two numeric parameters. A mutation casefile,
   `sce-build/tests/mutations/a_generic_reads_its_parameter_only_through_its_classes.cases`, holds three cases, each
   of which must turn the test red: one grants `===` to a parameter with no class, one checks a numeric class at its
   widest member alone, and one takes the rule of a list's element out of the table of rules.
4. The elaboration and the judgement again (D5): `sce-build/tests/an_instance_is_the_document_its_arguments_make.rs`
   (C5: a fixture `orset_union` with `E` of `key entry`, instantiated at `uint64` and `merge_entries=1024`, is node
   for node the tree of `stdlib/merge/orset_union.scxml`; an instance refused for a number names the generic's line
   and the import that made it; C3: two imports whose type arguments name each other, and two whose algorithm
   arguments do, are refused as `argument-cycle`),
   `sce-build/tests/every_instance_of_an_accepted_generic_is_accepted.rs` (C1: generic documents generated from a seed
   it records, over every statement and expression form of SCE_FORGE §4.12 applied to a value of each parameter, each
   at random arguments of its classes; every instance passes every check, and `instance-breaks-its-generic` is never
   reported; C5: each instance's tree equals the oracle's, the arguments substituted into the generic's XML text and
   read as an ordinary document; C4: over random pairs of arguments, two imports name one unit exactly when their
   canonical arguments are equal, and an instance fits another exactly when they are) and
   `sce-build/tests/a_parameter_is_data_independent_only_where_every_operation_on_it_moves_it.rs` (D7: a parameter
   only moved, one compared by `key`, one an algorithm argument's call compares, one a CBOR codec reads, each given
   its condition). Red without it: an import with arguments makes no instance. The casefile of step 3 gains a case
   that drops `present` from what `optional<$T>` needs, which must turn the property test red.
5. Generation and the lanes (D6): `sce-build/tests/an_instance_is_generated_once_per_set_and_names_no_parameter.rs`
   (one unit for an instance two documents of a set import; the Rust of an instance declares no generic parameter and
   names no `dyn` and no `Any`; `generate` of a generic without `--type-arg` refused as `uninstantiated`;
   `instance-name-collision`; the manifest and the sourcemap name the generic and the arguments). Red without it: an
   import with arguments generates no unit. Instance fixtures in `tests/forge/conformance/fixtures.json` (the
   `orset_union` fixture at a text, an algorithm with an algorithm argument instantiated at two comparators, a union
   whose case is of class `record`, a CBOR codec of an instance, an algorithm of a `$W` of `unsigned` at `uint16` and
   at `uint64`), run by the numerical conformance harness on Rust, then on every backend and the Interpreter. The
   harness changes for them, and no lane does: `tests/forge/conformance/fixtures.schema.json` carries a fixture's
   generic and its arguments; each lane's renderer, which reads a fixture's document for its records' fields, reads
   the instance's; and `tests/integration/AnAlgorithmRunsLoweredUnderTheInterpreterTest.cpp`, which calls
   `sce-codegen lower-algorithm` on the document a fixture names, passes it the fixture's `--type-arg`s. Red without
   it: a fixture that names arguments fails `fixtures.schema.json`, no lane's harness has a unit for it, and the
   Interpreter's runner is refused as `uninstantiated`.
6. Parametricity on the lanes (C2): `tests/forge/conformance/gen_cases.py` writes, for the `orset_union` fixture at
   `uint64` and at a text and an injective renaming between them that preserves neither the numeric order of the
   `uint64` values nor the byte order of the texts (else the mutation below would survive), the cases of both, and the
   harness holds the answers to be related by the renaming on every lane. A mutation casefile,
   `sce-build/tests/mutations/an_instance_keeps_the_order_its_document_gives.cases`, makes the Rust lowering of the
   instance walk its entries in the order of their elements' values, which must turn the renaming cases red.
7. The documents named in "Amends". A documentation step, exempt as step 1 is; the appendix row of each code of D10
   lands with its code, in steps 2 to 5, where `acceptance_doc_covers_every_code` turns red for a code without its
   row.

Steps 2 to 4 change `sce-build` alone, and step 5 changes the conformance harness and no lane. No step needs ADR 0156
D3: a machine that holds an instance is modelled when `sce-static` is (D7).

## Open questions

Each was decided on acceptance, with its reason.

1. **Where arguments are given.** At an import, never applied in a type: the alias is the one place a document says
   which document a name means (ADR 0086 §1), and it then names the instance in every type and expression.
2. **Which nominal sorts are classes.** `record` is, for the union's case and a list of records, the positions that
   admit only records; `union` and `enum` are not, since they grant nothing and no position needs them, and an enum
   argument is admitted by `key` (and `scalar`). `scalar`, `fixed` and `cbor` joined on acceptance, each because a
   rule of the language or D7 needs it (D2, D4).
3. **Operations beyond the classes.** An algorithm argument named at the import, not a class an author declares: the
   closed classes are coherent by construction, the operation that runs is written where the instance is made, and
   with every use specialised (D5) the two mechanisms compile to the same code (Wadler and Blott 1989; Jones 1994b;
   Dreyer et al. 2007), the choice Ada made with its formal subprograms (ISO/IEC 8652 §12.6).
4. **Variance.** None: an instance is its generic and its arguments as a record type is its document, and the use (D3)
   needs identity only; variance would make whether an instance fits depend on more than its identity.
5. **Statecharts.** Not in this record: an instance of a statechart needs its events named per instance, since an
   event's name names one schema on the network (ADR 0044), and that is a decision about the names of events, not
   about parameters. It is the follow-up record of a statechart's parameters and its instances' events (not yet
   numbered), whose first document is `stdlib/shared/record_list.scxml`. The logic that prompted this record is a
   serializer directed by its value's type, which is not one of the four kinds' parametric logic but a polytypic
   encoding: ADR 0168 answers it, on the mechanism this record supplies (Related).
6. **A parameter no slot names.** Refused: a distinction no value carries is no type's to make here, and an instance's
   identity would otherwise depend on an argument that changes nothing.
7. **The spelling.** A declaration is an element (`<sce:type-param>`, `<sce:bound-param>`, `<sce:algorithm-param>`),
   which the XSD validates, as RFC §synth-5-G's Q10 preferred; a use is `$P`, which no scalar keyword, alias or limit
   name can be, and which the template expander does not read.
8. **Defaults and inference.** Neither: an import's instance is the text of the import, which a default would make
   depend on text the importer does not show, and an argument inferred at a call would make each call site's instance
   depend on its operands.
9. **`view`.** It does not meet D1's criterion: ADR 0151 D4 holds each operator's integrated state between two
   changes. A view's parameters are left to that kind's own record, which may use this record's elements.
10. **Monomorphisation over one body with a dictionary.** Monomorphisation: no erased type, the bounded lanes, and the
    proof of the code that runs (ADR 0115 D10) each ask for it. Its cost, a unit per instance, is listed by the
    manifest.
11. **The level of C1.** A property test and the judgement of every instance now; a theorem once ADR 0156 D2's
    semantics is an object a prover reads. The code does not rest on C1 meanwhile, since an instance that breaks a
    rule is refused and never generated (D5).
12. **The standard library.** Its documents that fix a type (`orset_entry` and the algorithms over it, `lww_cell`) are
    converted in records of their own: a conversion changes the names of units that a host calls today (D6), which is
    a decision about that family and its hosts, not about the facility.

## Amended on acceptance

Each amendment is the independent review's, by its number there, unless it is marked as the downstream consumer's; the
main session accepted every one, and each was applied as the review recommends, its choices taken where the review
offers one.

1. D4: a rule is met at a parameter only where the declared classes entail its condition (Ada's "assume the worst",
   ISO/IEC 8652 §12.3(11); entailment of Jones 1994a), every rule stated in one table; classes join D2 as rules need
   them (`scalar` for a list's element and a local copied whole, the CBOR codec's `sce:encode-may-fail`); C1's
   generator ranges over every form of SCE_FORGE §4.12, and the sentence that C1 finds a rule D4 missed is gone
   (review 1).
2. D3 and D2: an algorithm's parameter declared `$T` is a view of `$T`, standing in a place of `$T` only under the new
   class `fixed`; the review's recommended form, over keeping a bound on an instance's parameter (review 2).
3. D2, D7, D9 and Not in scope: the operations that reach a parameter (an algorithm parameter whose signature names
   it, a CBOR codec imported at it) are counted; data independence is computed from the generic's tree; a CBOR codec
   at a parameter needs the explicit class `cbor`, which D7 counts, over refusing it, so that a generic keeps a
   value's bytes and its declaration says so; C2 is narrowed to what the computation finds moved or compared (review
   3).
4. D3, D4, D5 and D9: the arguments of one document's imports name no cycle (`type-param/argument-cycle`); C3's
   induction runs over a document's height and then this order, and its test is this case (review 4).
5. D5 and D7: a third kind of refusal, a proof that fails at one instance, reported by the formal judge's verdict
   (review 5).
6. D2, D7 and D10: a contract on `<sce:algorithm-param>` is refused until ADR 0115 D10 gives a signature one, the fit
   stated as the plug-in match (Zaremski and Wing 1997); `<sce:require>` is said to be a body's statement and
   `<sce:ensure>` not yet of the kind (review 6).
7. D2 and D4: a numeric parameter's whole judgement, typing included, runs at each member and at each combination of
   several, its cost stated; `_x.limits.$N` is in `min` to 2^32-1 (review 7).
8. D4, D9 and Consequences: what the check once proves is called its design target, held by C1 and not proven; the
   guarantee on the code is D5's (review 8).
9. D3: an argument's identity and the unit's name use the document's identity; the saved state alone spells a record
   by its event (review 9).
10. D3 and D10: every argument is spelled in a saved state as ADR 0112 D5 spells a type, rename-stably, and the
    instance's line is stated; a revision keeps its generic's parameters (`type-param/revision-changes-parameters`);
    the examples name `merge.orset_entry` (review 10).
11. D3 and D8: an instance's unit declares a bound's number alone, and the names stay their importers' constants
    (review 11).
12. D6: the unit's name is an injective, length-prefixed encoding of the canonical arguments beside `mangle`; C11's
    limits are stated (63 internal, 31 external) and the four kinds' C11 identifiers are internal or without linkage;
    past the limit, under `strict_c99_identifiers`, a prefix and a digest, listed in the manifest, collisions judged
    on the significant prefix (review 12).
13. D6 and D10: `--type-arg`, `--bound-arg` and `--algorithm-arg` bind the root document alone, refused in a set
    (`type-param/argument-flag-in-a-set`) (review 13).
14. D1, Not in scope and Open question 9: a `view` keeps operator state and does not meet D1's criterion; its
    parameters are its own record's (review 14).
15. Scope, D6 and step 5: the conformance harness changes (its fixture schema, its renderers and the Interpreter's
    runner), with the test that goes red (review 15).
16. Steps 1, 4 and 7: C4 and C5 are property tests, C5 against an oracle that substitutes into the XML; steps 1 and 7
    state their documentation exemption (review 16).
17. The textbook structure and Sources: Wolper's condition is moving data, equality its extension (Jonsson and Parrow
    1993; Lazic 1999; Lazic and Nowak 2000); Pierce's chapter 26 is bounded quantification by subtyping, classes as
    predicates Jones 1994a; Standard ML's equality types, Go's `comparable`, Ada's generics and SPARK are cited, Ada
    as the precedent of the whole (review 17, and its opening line).
18. Amends: SCE_FORGE §3.4, for `_x.types.$W.bits`, `.min` and `.max` (review 18).
19. The Forsythe citation is CMU-CS-88-159 (1988), reprinted in 1997 (review 19).
20. D7 and D10: `Construct::TypeParameters` is dropped, since no generic reaches the model generator (review 20).
21. D7: the model generator translates the algorithm bodies a statechart calls and reads the instances of the one
    elaboration, never resolving `src` itself (review 21).
22. Step 6: the renaming preserves neither numeric order nor byte order (review 22).
23. D3: a numeric parameter's added field may take a literal default checked at every member (review 23).
24. The textbook structure: coherence is the property, one instance per type the means (review 24).
25. Open question 5 and Not in scope: the follow-up record of a statechart's parameters is named, and the logic that
    prompted this record is said to be a polytypic encoding, not one of the four kinds' parametric logic (review 25).
26. The downstream consumer's review, point 1: the serializer directed by its value's type that prompted this record
    is answered by ADR 0168, a polytypic encoding drafted after ADR 0163 step 6; Related and Not in scope name it and
    say what this record supplies to it. It agrees with the independent review's 25 and the textbook (a parametric
    document cannot inspect its argument), so nothing was decided between them.
27. The downstream consumer's review, point 2: it accepts monomorphisation, every lane with C11, models of instances
    alone and the cost per instance in the manifest; Consequences adds that a microcontroller's flash for instances is
    measured as every unit's is.

## Sources

Read for this record, in this repository: ADR 0151 and ADR 0152 whole, as the house style; `SCE_FORGE.md` §3.2, §3.3,
§4.12 (algorithm, preconditions, records, lists of records, bounded lists, unions, maps and sets) and §10; ADR 0115's
table of levels and its D3, D7, D10 and D15; ADR 0156's Context, textbook structure and its D3, D4, D5, D11, D12 and
D14; ADR 0050's decision; ADR 0101 §1 and §5; ADR 0112 D5; ADR 0108 D1; RFC §synth-5-G and Q10 in
`docs/spec/synth/rfc-sce-protocol-synthesis.md`; and the code named in the Context (`value_type.rs`'s grammar,
`TypeEnv`, `TypeIdentity` and `fits`; `ForgeImport` and `parse_imports`; `template.rs`; `document_model.rs`'s
`Construct`; `symbol_mangling.rs`; `import_closure.rs`; the standard documents `orset_entry`, `orset_union`,
`lww_cell` and `record_list`). ADR 0086, ADR 0092, ADR 0044, ADR 0133 and ADR 0154 were read where the decisions cite
them, not whole. Checked on acceptance against the tree: `stdlib/merge/orset_entry.scxml` declares
`merge.orset_entry`; `symbol_mangling.rs`'s `mangle(machine, state_path, artifact)` escapes `__` alone and
`exceeds_c99_external_limit` tests 31 characters; `<sce:require>` is a statement of a `may-fail` body (SCE_FORGE
§4.12) and no `<sce:ensure>` exists; `tests/integration/AnAlgorithmRunsLoweredUnderTheInterpreterTest.cpp` calls
`sce-codegen lower-algorithm` on a fixture's document; SCE_FORGE §4.12 refuses a `string` or `bytes` element of a list
an algorithm holds; the C11 templates of an algorithm, a union and a CBOR codec (`tools/codegen/templates/forge/c/`)
declare their functions `static inline`, and a schema's declares types; ADR 0151 D4 holds each operator's integrated
state.

The literature: Girard 1972 (thesis, University of Paris VII); Reynolds 1974 ("Towards a theory of type structure",
Colloque sur la Programmation, LNCS 19); Strachey 1967 ("Fundamental concepts in programming languages", Higher-Order
and Symbolic Computation 13, 2000); Reynolds 1983 ("Types, abstraction and parametric polymorphism", Information
Processing 83, IFIP); Wadler 1989 ("Theorems for free!", FPCA); Pierce 2002 (*Types and Programming Languages*, MIT
Press, chapter 23, universal types, and chapter 26, bounded quantification by subtyping); Jones 1994a (*Qualified
Types: Theory and Practice*, Cambridge University Press); Cardelli and Wegner 1985 ("On understanding types, data
abstraction, and polymorphism", ACM Computing Surveys 17(4)); Wadler and Blott 1989 ("How to make ad-hoc polymorphism
less ad hoc", POPL); MacQueen 1984 ("Modules for Standard ML", ACM Conference on LISP and Functional Programming);
Dreyer, Harper, Chakravarty and Keller 2007 ("Modular type classes", POPL); Liskov and Wing 1994 ("A behavioral notion
of subtyping", ACM TOPLAS 16(6)); Zaremski and Wing 1997 ("Specification matching of software components", ACM TOSEM
6(4)); Milner, Tofte, Harper and MacQueen 1997 (*The Definition of Standard ML (Revised)*, MIT Press, its equality
types); *The Go Programming Language Specification* (`comparable`, since Go 1.18, 2022); ISO/IEC 8652:2012 (*Ada
Reference Manual*, §12: §12.3(11), §12.5 and §12.6) with the *Annotated Ada Reference Manual*; AdaCore, *SPARK User's
Guide* (the analysis of a generic per instance); Reynolds 1988 ("Preliminary design of the programming language
Forsythe", Carnegie Mellon University, CMU-CS-88-159; reprinted in O'Hearn and Tennent (eds.), *ALGOL-like Languages*,
vol. 1, Birkhauser 1997); Pierce 1991 (*Programming with intersection types and bounded polymorphism*, PhD thesis,
Carnegie Mellon University); Bracha, Odersky, Stoutamire and Wadler 1998 ("Making the future safe for the past",
OOPSLA); Kennedy and Syme 2001 ("Design and implementation of generics for the .NET Common Language Runtime", PLDI);
Stroustrup 1988 ("Parameterized types for C++", USENIX C++ Conference); Weeks 2006 ("Whole-program compilation in
MLton", ACM SIGPLAN Workshop on ML); Jones 1994b ("Dictionary-free overloading by partial evaluation", ACM SIGPLAN
Workshop on Partial Evaluation and Semantics-Based Program Manipulation); Mycroft 1984 ("Polymorphic type schemes and
recursive definitions", International Symposium on Programming, LNCS 167); Leroy 1995 ("Applicative functors and fully
transparent higher-order modules", POPL); Pnueli, Siegel and Singerman 1998 ("Translation validation", TACAS); Wolper
1986 ("Expressing interesting properties of programs in propositional temporal logic", POPL); Jonsson and Parrow 1993
("Deciding bisimulation equivalences for a class of non-finite-state programs", Information and Computation 107(2));
Lazic 1999 (*A semantic study of data independence with applications to model checking*, DPhil thesis, University of
Oxford); Lazic and Nowak 2000 ("A unifying approach to data-independence", CONCUR); Jansson and Jeuring 1997 ("PolyP:
a polytypic programming language extension", POPL); Hinze 2000 ("A new approach to generic functional programming",
POPL); ISO/IEC 9899:2011 (C11, §5.2.4.1 and §6.4.2.1); the Itanium C++ ABI (its mangling of template arguments) and
Rust's RFC 2603 (v0 symbol mangling), cited for practice and not as a foundation.

They are cited from the texts as known; none was read again for this record or its acceptance, and the corrections
made on acceptance (the venue of Forsythe, what Pierce's chapter 26 is, Wolper's condition and its extension to
equality) are the independent review's, not a reading of the texts here. The passages the decisions rest on are to be
checked against the texts in the step that builds each: the statement of the abstraction theorem and its premise
(Reynolds 1983) and the free theorem C2 tests (Wadler 1989), for step 6; the substitution lemma as Pierce states it
and the entailment of qualified types (Jones 1994a), for steps 3 and 4; Ada's rule that a generic body assumes the
worst (§12.3(11) and its annotation), for step 3; coherence and the dictionary translation (Wadler and Blott 1989) and
the specialisation that removes dictionaries and its termination (Jones 1994b), for step 4; polymorphic recursion
(Mycroft 1984), for C3; the applicative reading of functors (Leroy 1995), for C4; the plug-in match (Zaremski and Wing
1997), when ADR 0115 D10 gives a signature a contract; data independence with equality (Jonsson and Parrow 1993; Lazic
1999; Lazic and Nowak 2000), before ADR 0156 D5 uses D7's condition; and the checking of an intersection at each of
its members (Reynolds 1988, Pierce 1991), for D4's numeric classes.

