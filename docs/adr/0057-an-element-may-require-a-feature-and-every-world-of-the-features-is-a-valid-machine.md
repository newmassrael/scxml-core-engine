# ADR 0057 — An element may require a feature, and every world of the features is a valid machine

- Status: Accepted (2026-10-26, by the owner: "approved, proceed"); being implemented in the order below. Requested
  by a downstream consumer on the owner's instruction that features be switched on and off freely through Kconfig and SCE
  code generation, and designed under the owner's rule of the same day: the most long-term correct structure,
  textbook in its foundations, cost ignored. The surface the consumer confirmed on 2026-10-26 (§3) is unchanged by
  that rule; the foundations under it are what the rule set
- Date: 2026-10-26
- Scope: a new forge kind `feature-model` and its schema; presence conditions on the parsed model
  (`sce-build/src/parser.rs`, `model.rs`, `sce_attr.rs`); one derivation function and the `sce-codegen project`
  command; the classification of every validation rule by how it behaves under derivation, and the family-based
  checks that follow; new `DiagnosticCode` variants and their acceptance-doc rows; `schemas/sce-forge-ext.xsd`,
  `schemas/sce-kind-catalog.v1.schema.json`, `schemas/sce-transition-table.v1.schema.json`,
  `schemas/sce-saved-state.v*.schema.json`; the templates of every backend under `tools/codegen/templates/`; the
  `--features` input of `sce-codegen generate`; a conformance law tested in each lane
- Related: ADR 0053 (features only add), ADR 0056 (the microstep's buffers are the document's size), ADR 0039 (the
  scope of an import is an input to the check), ADR 0044/0045 (saved state), `docs/SCE_TRANSITION_TABLE.md`,
  `docs/SCE_CODEGEN_DETERMINISM.md`, and the consumer's feature inventory and upstream-parity notes

## Context

A consumer chooses what a build contains with Cargo features driven from Kconfig. An SCE-generated machine can today
be switched only as a whole: the consumer wraps the `include!` of the generated file in one `#[cfg]`. A state, a
transition, an `<sce:action>` or the trait method it becomes cannot be left out of a build, so a client-only build
still carries the accept path of the unicast session machine and the host methods it calls.

What is asked for is a *software product line* in the established sense (Apel, Batory, Kästner, Saake,
*Feature-Oriented Software Product Lines*, Springer 2013): one artifact from which a family of variants is derived
by a selection of features. The field fixes the vocabulary this ADR uses and the obligations it accepts:

- the **problem space** is a *feature model* whose semantics is a propositional formula over the features, and the
  valid configurations are its models (Kang et al., FODA, CMU/SEI-90-TR-21, 1990; Batory, "Feature Models, Grammars,
  and Propositional Formulas", SPLC 2005);
- the **solution space** maps features to the artifact by *presence conditions* annotated on its elements, with a
  *derivation* that removes the elements whose condition is false (Czarnecki and Antkiewicz, "Mapping Features to
  Models: A Template Approach Based on Superimposed Variants", GPCE 2005);
- annotations are *disciplined*: they enclose whole syntactic units, never fragments of one (Liebig, Kästner, Apel,
  "Analyzing the Discipline of Preprocessor Annotations in 30 Million Lines of C Code", AOSD 2011);
- the family is correct when **every** derived variant is well formed, which is to be decided *family-based*, over
  presence conditions, not by building variants one by one (Czarnecki and Pietroszek, "Verifying Feature-Based Model
  Templates Against Well-Formedness OCL Constraints", GPCE 2006; Kästner et al., "Type Checking Annotation-Based
  Product Lines", TOSEM 21(3), 2012; Thüm et al., "A Classification and Survey of Analysis Strategies for Software
  Product Lines", ACM Computing Surveys 47(1), 2014);
- a variability-aware operation is correct when it *commutes with derivation* (Erwig and Walkingshaw, "The Choice
  Calculus", TOSEM 21(1), 2011; Midtgaard et al., "Systematic Derivation of Correct Variability-Aware Program
  Analyses", Science of Computer Programming, 2015), and a behavioural analysis of the family is done on a *featured
  transition system* (Classen et al., "Model Checking Lots of Systems", ICSE 2010).

One fact of the target narrows the textbook's general feature model, and the whole design rests on it. Cargo builds
one copy of a crate with the union of the features every dependent asks for (ADR 0053). A configuration a consumer
never chose is therefore built whenever two dependents choose two others. The configuration space SCE may promise is
only one that is **closed under union**, and the presence conditions it may honour are only those that are
**monotone** in the configuration.

## Decision

### 1. The formal model

Let `F` be a finite set of features and `≤` the *requires* order on `F`: `a ≤ b` reads "a requires b", a partial
order (a cycle is two names for one feature and is refused).

- **Configurations.** `Conf = { c ⊆ F | a ∈ c ∧ a ≤ b ⇒ b ∈ c }`, the subsets closed under *requires*. `Conf` is
  closed under union and intersection; it is the finite distributive lattice of down-sets of `(F, ≥)` (Birkhoff,
  "Rings of Sets", Duke Math. J. 3, 1937), with bottom `∅` and top `F`. Every configuration Cargo can produce from
  configurations in `Conf` is again in `Conf`, and Cargo itself enforces `≤` when each `a ≤ b` is the feature edge
  `a = ["b"]`.
- **What the model does not contain, and why.** Excludes, alternative groups and "at least one of" groups (FODA's
  remaining constructs) define spaces that are not closed under union; a crate cannot keep them, so SCE does not
  model them. A consumer's Kconfig `choice` is mapped by the consumer onto features that only add.
- **Presence conditions.** `PC ::= NAME | all(PC, ...) | any(PC, ...)` with the propositional meaning; `true` is the
  condition of an unannotated element. Every condition is negation-free, hence monotone: `c ⊆ c'` and `c ⊨ p` give
  `c' ⊨ p`.
- **Effective condition.** `pc*(e) = pc(e) ∧ pc*(parent(e))`, so a disciplined annotation removes a whole subtree.
- **Derivation.** `π_c(D)` is `D` with every element `e` such that `c ⊭ pc*(e)` removed, the order of the remaining
  elements kept. A gated document *denotes* the function `⟦D⟧ : Conf → Document, c ↦ π_c(D)`.
- **Monotonicity.** `c ⊆ c' ⇒ π_c(D) ⊑ π_c'(D)` (sub-document). Hence `π_F(D)`, the top, contains every variant and is
  `D` itself; every bound computed from `D` (ADR 0056's microstep buffers, ADR 0050's send bounds) is an upper bound
  for every variant.

`not(P)` is excluded by this model, not left open: with it, `π` is not monotone and Cargo's union selects a variant
from which an element vanished because some other crate added a feature. "Other behaviour when `X` is off" is
written additively: a transition under `sce:requires="X"` placed before an ungated transition on the same event wins
by document order (§scxml-3.13) wherever `X` is on, and the ungated one is taken elsewhere.

### 2. The problem space is one document of its own

A feature model is a forge document of the new kind `feature-model`, imported where it is used:

```xml
<!-- session_features.scxml -->
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="feature-model" name="session_features">
  <sce:feature name="session-unicast"/>
  <sce:feature name="session-unicast-accept" requires="session-unicast"/>
</scxml>

<!-- a statechart of the product line -->
<sce:import kind="feature-model" src="session_features.scxml" as="Features"/>
```

The document's root is `<scxml>` with `version="1.0"`, `sce:kind="feature-model"` and `name`, and its only children
are `<sce:feature>` elements. `<sce:feature>` takes `name` (required, unique in the model) and `requires` (optional:
a space-separated list of names declared in the same model, the list convention of `<transition target>`, each
entry one edge `name ≤ entry`). Features are written in any order; the order of `F` used elsewhere (§5, §6) is derived,
not authored.

The product line's features are declared once, by the product, and every statechart of it reads the same model; a
declaration inside each statechart would be one copy per document. A statechart imports at most one feature model.
A consumer that keeps its own declaration generates this document from it, so the product has one source and SCE
reads its own format. The consumer keeps one model per crate that compiles generated machines, generated from that
crate's `Cargo.toml` `[features]`: a Rust `cfg` reads the features of the crate that compiles the module, so that
crate's features are the product line's.

Feature names are `[a-z][a-z0-9]*(-[a-z0-9]+)*`. Each is also spelled for every backend (§6): Cargo takes it as
written, and the C, C++ and Swift macro `SCE_FEATURE_<NAME>` upper-cases it and writes each hyphen as an underscore.
With no upper-case letter and no underscore in the grammar, that spelling loses nothing, so two names never spell
alike in any backend and no refusal for it is needed. The document's own `name` is a document name, not a feature
name: it identifies the model in a manifest and in diagnostics and is spelled into no generated code, so the feature
grammar does not bind it.

The model emits nothing on any backend. The check of its *requires* edges travels in each gated statechart's own
output (§6), so the code that relies on an edge carries the check of it, and a build cannot include the code and
leave the check out. Refusals: `validation/feature-name-invalid`, `validation/feature-declared-twice`,
`validation/feature-requirement-undeclared`, `validation/feature-model-cycle`.

### 3. The solution space: annotations on whole elements

`sce:requires="P"` on a W3C element, `requires="P"` on an `sce:` element. It is an attribute, never a child element:
the children of `<transition>`, `<onentry>` and `<onexit>` are executable content, and a child that does not execute
would change the content walk of every backend. An attribute is read through `sce_attr`, and one on an element that
does not take it is refused by `validation/sce-attribute-unread`.

An element takes a condition when removing it leaves a document of the accepted subset, which makes the annotation
disciplined by construction:

| Element | Takes `requires` | Its references (§5) |
|---|---|---|
| `<state>`, `<parallel>`, `<final>` | yes | targets, `initial`, history defaults that name it |
| `<transition>` | yes | its targets; the derived event variant (§4) |
| executable content (`<raise>`, `<send>`, `<assign>`, `<log>`, `<if>`, `<foreach>`, `<sce:action>`, ...) | yes | the data, events and actions it names |
| `<onentry>`, `<onexit>`, `<invoke>` | yes | what its content names; `done.invoke`/`error.invoke` transitions |
| `<data>` under `datamodel="sce-static"` | yes | every expression that reads or writes it |
| `<data>` under any other data model | no: its readers are not statically known | — |
| `<sce:import>` (other than `feature-model`) | yes | every use of the imported name |
| `<initial>`, `<history>`, `<scxml>`, `<datamodel>`, `<elseif>`/`<else>` | no: required structure | — |

A condition below an element that already carries it adds nothing and is reported as redundant
(`validation/feature-condition-redundant`), so each condition is written once, where the variation is.

### 4. Derived items take the disjunction of their uses

A generated item the document does not write once but derives from its uses exists exactly in the variants that use
it: its condition is the disjunction of the effective conditions of its uses, and `true` if one use is ungated
(presence-condition inference, Kästner et al. 2012). This covers the event enum variant (from the transitions whose
`event` descriptors name it and the `<raise>`/`<send>` sites that deliver it to the machine), the host `Actions`
trait method (from the `<sce:action>` sites that call it), the typed records and enums an import brings in, and the
host processors and invokers a document asks for. A host that sends an event, or implements a method, of a variant it
is not built for fails to compile; that is the projection's own answer, since `π_c(D)` has no such event or method.

### 5. Every variant is well formed, decided over the family

The obligation is `∀ c ∈ Conf. WF(π_c(D))`, where `WF` is acceptance by the accepted subset. It is decided without
deriving variants, by classifying every validation rule by how it behaves under derivation. Each rule declares one
class in a registry, and a test refuses a rule (and a `DiagnosticCode`) that declares none:

1. **Preserved by removal.** A rule that holds in every sub-document of a document it holds in (unique ids, the
   shape of an attribute, a type annotation). Checked once, on `D`; monotonicity carries it to every variant.
2. **Reference.** A rule that an element's referent is present. It holds in every variant iff
   `pc*(referrer) ⇒ pc*(referent)` holds over `Conf`. Because both conditions are monotone and `Conf` is the
   down-set lattice, the implication is decided exactly: it holds iff `pc*(referent)` is true at `↓M` for every prime
   implicant `M` of `pc*(referrer)`, where `↓M` is the closure of `M` under *requires* (any `c ⊨ pc*(referrer)`
   contains some `M`, hence `↓M`, and a monotone condition true at `↓M` is true at every superset; `↓M` is itself a
   configuration). A failure carries `↓M` as its witness: the least configuration in which the variant dangles.
3. **Existence.** A rule that something is present at all (a document keeps a state, an `<invoke>` has its
   completion handled, the exhaustiveness and reachability analyses). Such a rule is lifted to presence
   conditions: the analysis runs once on the *featured* structure and answers, for each fact, the set of
   configurations in which it holds, represented as a reduced ordered BDD over `F` in the model's topological order.
   Reachability and every analysis on the transition graph run on the featured transition system the document
   denotes (Classen et al. 2010). A failure carries the least configuration it holds in.
4. **Product-only.** A rule that no lifting exists for yet is checked on each *distinct* variant. Configurations with
   equal derivations are one variant, so the work is the number of distinct variants, not `|Conf|`. Moving a rule
   from this class to class 3 is an improvement, never a change of verdict.

New diagnostic codes of the statechart side (names provisional until each lands):
`validation/feature-undeclared`, `validation/feature-gate-unsupported`, `validation/feature-condition-redundant`,
`validation/feature-gate-escapes` (class 2, with the witness), `validation/feature-variant-ill-formed` (classes 3
and 4, with the witness and the inner diagnostic it carries). Each lands in the acceptance-doc appendix.

### 6. Compilation commutes with derivation

For each backend `B`, generation is a variability-aware operation and is correct iff the square commutes:

```text
select_c( gen_B(D) )  ≅  gen_B( π_c(D) )      for every c ∈ Conf
```

`≅` is equality of the generated items (the same declarations exist) and of behaviour (the same observations on
every scenario). A backend keeps the variability where its language selects at compile time and derives where it
cannot:

- **Rust**: `#[cfg(P')]` on every gated item (state and event enum variants, match arms, table entries, trait
  methods, record fields), `P'` being `P` with each name written `feature = "NAME"`. Fixed-length tables take a length
  that follows the configuration. One generated file serves every configuration.
- **C and C++**: `#if` over `SCE_FEATURE_<NAME>`, the name in upper snake case.
- **Swift**: `#if` over the compilation condition `SCE_FEATURE_<NAME>`.
- **Go, Kotlin, Python and any backend without compile-time selection**: `sce-codegen generate --features a,b`
  derives `π_c(D)` and lowers it. A gated document generated for such a backend without `--features` is refused, so
  no configuration is chosen silently; a `--features` set outside `Conf` is refused with the missing requirement.

**The emitted condition is canonical.** It is the unique minimal form of the monotone function `pc*` on `Conf`: the
antichain of least configurations that satisfy it, each written by its features that no other feature in it
requires (the rest are implied by *requires*), sorted. Equal conditions are spelled equally, so generation stays deterministic
(`SCE_CODEGEN_DETERMINISM.md`) and a reformulation of an annotation that does not change its meaning does not change
the output.

**The model guards itself.** Because the canonical form leaves implied features out, a build that broke *requires*
would compile code the model never allowed. Every backend that keeps variability writes, into each gated statechart's
own output, the model's edges as refusals: `#[cfg(all(feature = "a", not(feature = "b")))] compile_error!(...)` in
Rust, `#error` in C and C++. The `not` there refuses a build; it selects no code. Cargo enforces the edges in the
first place when the crate's `[features]` declares each `a = ["b"]`; a consumer that generates the model from its
`[features]` (§2) has that by construction, and the refusals catch a crate that does not.

**One derivation.** `π` is implemented once, in `sce-build`. `sce-codegen project --features ...` writes `π_c(D)` as
a plain document; the interpreter, the deriving backends and the conformance oracle all take that document, so no
engine re-implements derivation.

### 7. Wire surfaces carry the family

- The transition table (`SCE_TRANSITION_TABLE.md`) becomes a featured transition table: each row carries its
  canonical condition, and a consumer derives a variant by the same rule.
- The forge AST and the sourcemap carry each element's own and effective condition.
- A saved state records the configuration it was taken in, as the sorted feature set. A restore into a different
  configuration is refused with a typed error naming the features that differ: a smaller one lacks states or data
  the saved state holds, and a larger one has data and invocations the saved run never initialised or started.
  Carrying a saved state across configurations is a migration, outside this ADR.

### 8. Conformance

The law of §6 is tested in every lane on fixtures that cover each class of §5:

- **Behaviour**: for each configuration of the matrix, the gated output built with that configuration and
  `gen_B(π_c(D))` run the same scenarios, and their observations are equal.
- **Items**: the inventory of generated declarations under `c` equals the inventory of `gen_B(π_c(D))`, so an item
  that must vanish in a variant is shown to vanish.
- **The matrix** is every distinct variant. A fixture whose distinct variants exceed a declared bound is tested on a
  `t`-wise covering array of `Conf` (Johansen, Haugen, Fleurey, SPLC 2012), `t` being the widest conjunction in any
  condition, plus `∅`, `F` and every principal down-set `↓{f}`. The bound and the arrays are written in the fixture
  and are part of its review.

### 9. Lifting the generator (the design of step 3)

A backend that keeps the variability must emit, for each configuration `c`, exactly what it emits for `π_c(D)`.
Two designs were weighed for making the law of §6 hold, and the second is the one built.

*Lifting each site of each template.* An item an element writes carries the element's presence; a value computed
from the document (the default initial state, whether a state is compound, which events are externally drivable, a
transition's index among those its state keeps) is read per variant and written once per distinct value under a
selector. Correct, and the wrong cost: the Rust templates hold hundreds of sites that read the document, each
would need deciding, every new site would need the same, and a site missed is silently wrong for some build.

*Merging what the backend already writes.* The generator derives each distinct variant (§5's enumeration), prepares
it as the document was prepared, and generates it as any document is generated — the rules that compute every value
stay the generator's own, written once, and no template changes. The source for every build is the **choice** of
those sources (Erwig and Walkingshaw, the choice calculus, TOSEM 2011) with everything the variants share factored
out, which is a semistructured merge (Apel et al., ESEC/FSE 2011): each source is read as Rust, each list of the
syntax — the items of a file, the members of an `impl`, the statements of a block, the arms of a `match`, the
fields and variants of a type — is aligned member by member across the variants, a member all write alike is
written once, a member some write is written under the selector of those that write it, and members that differ
only inside a list they share are merged inside it. Where nothing is shared the members are alternatives, each
under its own selector. The text written is the variants' own, cut and joined at the members' bounds, so comments
and the `SCE-MAP` markers stay where the variant wrote them. A difference at a place no `#[cfg]` may stand — the
value a block ends with — is refused (`codegen/feature-lifting-mismatch`), as is any output the check below does
not accept.

The merge is **checked, not argued**. For every build the product line tells apart (not one per variant: a
predicate the output writes may tell apart builds that build one variant), the output's `#[cfg]` is resolved on its
syntax tree and compared token for token with the source that build's variant generates alone (translation
validation, Pnueli, Siegel and Singerman, TACAS 1998). A merge that is wrong for any build refuses the generation,
naming the first build and the item that differs. The check is exhaustive over the builds, so the law of §6 is a
property of every output this backend writes, whatever a future template does.

A variant's **selector** is the exact predicate over the build's features that picks it: the conjunction, over the
document's gates, of each gate's condition or its negation as the variant has it. Selectors partition every
configuration the model admits, so exactly one alternative of each value is compiled. The selector written before
a member is a small one that holds in the variants that write it and in none of the others (Espresso's
expand and irredundant moves over the variants' valuations of the document's distinct conditions, Brayton et al.
1984; a heuristic, not a minimum), so a member written for one element reads as that element's own condition. A selector may negate: it is a choice
the generator writes, not a condition an author does, and the monotone grammar of §1 binds only the latter.

**A variant inherits its document's names.** A state's document-order rank and the id a state written without one
is given are assigned over the document, and a variant parsed through a selection keeps the ones `D` gives its
states, matched by the position each starts at, which `π` keeps. Numbered over the variant instead, a state after a
removed one would be ranked, and an unnamed one named, differently in each variant, and the lifted output would have
no one spelling for it. Kept from `D`, the ranks of a variant are not dense; they are order-isomorphic to dense ones,
and only their order is read.

Bounds read from the document (ADR 0056's microstep buffers, ADR 0050's send bounds) are those of `D`, which hold
every variant (§1).

### 10. Lifting the C, C++ and Swift emissions (the design of step 4)

Status of this section: Accepted (2026-11-03, by the owner's word to continue, given to the proposal of the same
day); being implemented in the order below.

§9 holds for every backend that keeps the variability: derive each distinct variant, generate it as any document is
generated, merge the sources, and check the merge build by build against what each variant generates alone. What
§9 wrote for Rust alone is the *reader* (which lists of the syntax a selector may stand in), the *writer* (how a
selector is spelled before or around a member) and the *resolver* (how a build reads a selector). The merge is split
along that line: one language-neutral core (the alignment of members across versions, a shared member written once,
the rest under the selector of the variants that write them, the merge inside a list the versions share, the
selectors of §9 and their Espresso cover), and for each backend a reader, a writer and a resolver.

**A reader reads the grammar that decides where a selector may stand, and that grammar exactly.** It is never a
grammar that approximates the language. A reader that cannot read a variant's source refuses the generation
(`codegen/feature-lifting-mismatch`): it does not guess.

- **Rust** reads Rust with `syn`, as §9 does: `#[cfg]` stands before an item, a statement, an arm, a field, a
  variant, a field value or an element.
- **C and C++** read the *preprocessor's* grammar (ISO C translation phases 1 to 3 and §6.10; the same phases in
  ISO C++): logical lines after line splicing, preprocessing tokens, comments, and the directive lines. A `#if` is a
  directive, so this grammar decides exactly where one may stand: at the start of a logical line that is not inside
  a comment, a raw string literal or the continuation of a directive. The language grammar is not the right reader,
  and none can be used: SCE's own C and C++ are written with macros the language grammar does not know without
  their definitions (`SCE_SM_FN static …`, `SCE_C_UNUSED`, `#line` inside a class body). A parser that does not
  expand them reads 99 syntax errors in one generated C11 file of 1,986 lines and 8 in one C++ `.inl` of 337 lines
  (tree-sitter-c 0.24.2 and tree-sitter-cpp 0.23.4, measured 2026-11-03 on `session.scxml` with `session-unicast`).
  A member is a run of whole logical lines whose preprocessing tokens are balanced in `()`, `[]` and `{}`, so
  every selected group encloses whole token trees: the discipline of Liebig et al. stated on the tokens, which is as
  far as a grammar that does not expand macros can state it. A directive line is a member of its own and is never
  cut; a difference inside a `#define` (one variant's mask names a state another's does not) selects the whole
  directive. The conditional groups a variant writes for itself (`#ifdef __cplusplus`, include guards,
  `SCE_OWNERSHIP_CHECKED`) are lists: their groups are merged inside, and they are written as the variants write
  them.
- **Swift** reads Swift with tree-sitter-swift, whose `#if` positions are Swift's own (a declaration, a member, a
  statement, a `case` of a `switch`; not an element of an array literal, which Swift refuses too). All 215 generated
  Swift machines committed in the tree parse with no error node (measured 2026-11-03). A source that does not is
  refused, and a test holds every committed generated Swift file to parsing with no error node, so a template that
  writes what this reader cannot read fails there and not in a consumer's build.

**Writers.** Rust writes `#[cfg(P)]` before the member. C and C++ write `#if P` and `#endif` on lines of their own
around the member, and the alternatives of one slot as one `#if` / `#elif` chain, since exactly one of them is
compiled in each build (§9: the selectors partition the builds). Swift writes the same with `#if`, `#elseif`,
`#else` and `#endif`. `P` is the selector of §9 spelled for the backend: `Selector::c` for C and C++
(`defined(SCE_FEATURE_A) && !defined(SCE_FEATURE_B)`), a Swift compilation condition for Swift
(`SCE_FEATURE_A && !SCE_FEATURE_B`). A build selects with `-DSCE_FEATURE_<NAME>` in C and C++, and with
`-D SCE_FEATURE_<NAME>` (SwiftPM `.define`) in Swift.

**Resolvers and the check.** For every build the product line tells apart, the resolver evaluates exactly the
conditionals whose condition is written over `SCE_FEATURE_` names in the grammar of the selectors, and keeps every
other directive as written. The check compares the result token for token with the variant's own source resolved
the same way: preprocessing tokens for C and C++, Swift tokens for Swift, comments aside. The merge is not argued
correct for these backends any more than for Rust.

**The model guards itself** in every output that keeps the variability (§6), in the file a build of the statechart
cannot leave out: the header of C11 and C++, the source of Swift. For each edge `a ≤ b`:
`#if defined(SCE_FEATURE_A) && !defined(SCE_FEATURE_B)` / `#error "…"` / `#endif` in C and C++,
`#if SCE_FEATURE_A && !SCE_FEATURE_B` / `#error("…")` / `#endif` in Swift. The message is `Refusal::message`.

**The sourcemap is the document's.** `sce_sourcemap.json` maps a generated symbol to the SCXML element and lines it
was written from; it names no generated line. `generate` writes it from the document as written, so the map of the
output for every build holds every symbol of every variant, each where a variant's own map puts it (a variant's
positions are the document's, §2). The condition under which a symbol is present is a field of §7 still to add.
(Corrected 2026-11-03 while building C11: this paragraph had the maps differ in every row after the first gated
element, which is true of no backend.)

**A `#line` stays with its line.** A `#line` directive names the line after it, so the C reader makes the two one
member and no selector comes between them. A selector's lines move the lines after them within a `#line` scope, as
they move `__LINE__`: only a scope's first line is the one its directive names, and that one is kept.

**A build of `sce-build` without the readers.** The Rust and the C and C++ readers are Rust. The Swift reader is
native code (the Swift grammar is C, as libxml2 is behind the `xsd` feature) and sits behind a default feature of
`sce-build`. A build without it (`sce-build-wasm`) refuses a gated document for Swift without `--features`, as every
backend refused one before it kept the variability (`codegen/feature-selection-required`).

**Conformance** is §8's law in each lane. One fixture set (`tests/product_line/`) is read by every lane, so no lane
keeps a copy: the Rust crate of step 3 moves its two documents there. Each lane builds the output for every build
once for each configuration of the matrix and the variant of that configuration alone, drives both through every
run of up to four events, and compares the observations (configuration, host calls, discarded events, end). The
C11 and C++ lanes build under CMake with `-D`; the Swift lane under SwiftPM with `.define`.

**Order:** the split of the core with Rust as its first instance, changing no output (the Rust lane is the judge);
C11; C++; Swift. The Swift backend writes no `<sce:action>` today, which the fixture uses: the Swift lane waits for
that backend gap to be closed, as its own change, rather than holding to a weaker fixture.

## Consequences

- A consumer gates an element where it is written, and the build proves for every configuration, not a sample, that
  no variant dangles; a failure names the least configuration that shows it.
- The configuration space SCE serves is exactly what Cargo can build; a constraint Cargo cannot keep is not modelled
  rather than modelled and broken.
- The feature model is one document per product line; a consumer that keeps its own declaration generates it.
- Every validation rule acquires a declared class, and the registry makes a new rule choose one.
- Every backend gains variability: four keep it in one output, the rest derive it with `--features`.
- The transition table, the forge AST, the sourcemap and the saved state gain fields; while their status is
  `pre-release` (`SCE_WIRE_CONTRACTS.md`) the additions are admitted.

## Alternatives considered

- **A child element `<sce:requires>`** (the consumer's first sketch): refused, §3.
- **`not(P)` and FODA's exclusive groups**: refused, §1. They are correct for a product that derives one variant per
  build and wrong for one that Cargo unifies.
- **Presets named in a document**: a preset is a configuration, not a feature; it belongs to the consumer's build
  (a Cargo feature that enables atomic ones), and a document names atomic features only.
- **A feature list declared inside each statechart**: one copy per document of what is one model, §2.
- **Checking variants one by one**: exponential in `|F|` and silent about the configurations not built; kept only
  as class 4 of §5, for the rules that have no lifting yet.
- **Attribute-level variation** (a `cond` or `target` that differs per feature, the choice calculus's alternatives):
  undisciplined, and not additive; written instead as two transitions with two conditions.

## Order of work

1. This ADR, accepted by the owner. The attribute shape (§3) and the feature-model document (§2) are fixed then; a
   consumer may start authoring both.
2. The `feature-model` kind, conditions on the model, `π` and `sce-codegen project`, the rule registry with every
   existing rule classified, the class 1 and class 2 checks, the diagnostics and their acceptance-doc rows, and
   `--features` for the deriving backends.
3. The Rust emission of §6 with the canonical form and the self-guard, and the Rust lane's conformance matrix. A
   Rust consumer raises its pin from here.
4. The featured lifting of the class 3 analyses; the C, C++ and Swift emissions and their matrices; the wire
   surfaces of §7.

### What step 2 landed (2026-10-26)

- **The feature model** (`sce-build/src/feature_model.rs`) and **presence conditions**
  (`sce-build/src/presence_condition.rs`): the requires order, configurations enumerated once each, the canonical
  antichain form, and the exact implication with its least-configuration witness. The `feature-model` kind is a
  [`KindClass::Declaration`](../../sce-build/src/forge/codegen_matrix.rs): generating one writes no file on any backend.
  A forge document is named by its file, as every kind is, so a model's name in diagnostics is its file's stem.
- **Conditions on a statechart** (`sce-build/src/product_line.rs`), read inside the parse's ledger; the class 2 check
  of §5 for `target` and `initial`; and the derivation `π`, which blanks what it removes so every position of a
  variant is the source's (after preprocessor expansion), and so needs no position map.
- **Selection**: `SCXMLParser::with_features`, `generate --features`, `check --features` and `sce-codegen project`.
  Every backend refuses a gated document without `--features` (`codegen/feature-selection-required`) until a
  backend keeps the variability in its output, which is step 3 for Rust.
- **The family check**: `check` of a gated document without `--features` holds each distinct variant to
  [`judge_statechart`](../../sce-build/src/lib.rs), the sequence the document itself is held to
  (`check_every_variant`, `validation/feature-variant-ill-formed`).
- **The rule registry**: `DiagnosticCode::derivation_class`, exhaustive. The product line's own sixteen codes are
  `Family`; every other rule is `ProductOnly` at landing, judged on each distinct variant, which is sound for any
  rule. No rule is yet argued into class 1, 2 or 3: each such move is its own change, it reduces the work, and it
  changes no verdict.

### What step 3 landed (2026-10-27)

- **One Rust machine for every build.** `sce-codegen generate` of a gated document with `-l rust` and no
  `--features` derives and prepares each distinct variant as the document was prepared (`StatechartPreparation`, which
  `check` shares), generates each with the document's bounds, merges the sources (`variant_merge`) and checks the
  result build by build (`lift_validation`, `variant_lift`). `check` reaches the same verdict. The other backends
  still refuse a gated document without `--features`.
- **Names are the document's.** A variant keeps the document-order ranks and the ids generated for unnamed states
  that the document gives, so every variant spells a state alike.
- **In-line children** must be the same in every variant: a run writes each as a file of its own, and a child a
  variant removes or changes is refused as `codegen/feature-lifting-mismatch`.
- **Conformance** (`backends/rust/feature-conformance`): `build.rs` generates the machine for every build and the
  machine of each feature set alone; the tests build the crate under each set — rustc resolves the `#[cfg]` itself —
  and drive both through every run of up to four events, asserting equal observations (configuration, host calls,
  discarded events, end). The default set runs in the workspace sweep and the gate `workspace-tests` builds the
  other two.
- **The model guards itself** (§6). The machine for every build ends with a `compile_error!` for each edge of the
  model (`feature_selector::Refusal`), under `all(feature = "a", not(feature = "b"))`; a set of features is refused
  by some edge exactly when it is no configuration, since a set closed under each direct requirement is closed
  under *requires*. No build the model admits holds a refusal, so the check of the lifted machine finds each gone.
- **The manifest states the model.** `FeatureModel::cargo_features` is the `[features]` table the model states,
  each feature implying what it requires directly. The conformance crate's `build.rs` refuses a manifest whose
  table is not that one and prints the table to write, so the edge has one source, the model, and Cargo keeps it.
- Still to do: the same for the C, C++ and Swift emissions (step 4).

### What step 4 has landed so far (2026-11-03)

- **The core and the Rust dialect** (§10). `variant_merge` is the merge no language decides: it aligns members,
  writes shared ones once and merges inside a list the versions share, comparing members by the id of their token
  trees (hash-consed, so two trees have one id exactly when they are the same tokens). A
  [`Dialect`](../../sce-build/src/variant_dialect/mod.rs) is what a language decides: its reader, writer, resolver
  and guard. [`variant_dialect::rust`](../../sce-build/src/variant_dialect/rust.rs) is the first, the reader with
  `syn`, `#[cfg]` as the writer, `lift_validation` as the resolver, `compile_error!` as the guard. The output did not
  change: the lifted machine and sourcemap of the conformance crate's `session.scxml` are the same bytes as before
  the split.
- **The C11 dialect** ([`variant_dialect::preprocessor`](../../sce-build/src/variant_dialect/preprocessor.rs)), which
  C++ will share. `generate` and `check` of a gated document with `-l c11` and no `--features` write one source and
  one header for every build (`Language::keeps_variability`); the header ends with an `#error` for each edge of the
  model (`Refusal::c`). The lifted C of `session.scxml` compiles with `-Wall -Werror` under each of the three feature
  sets, and a set that breaks the edge stops at its `#error`. The enum of events writes its last member without a
  comma, so a variant that adds an event writes that member twice, each under its builds: correct, and left so,
  since the template's hash moves every committed tree.
- **The C11 lane.** The fixtures are `tests/product_line/`, which the Rust crate reads too. `backends/c/tests`
  generates the machine for every build (symbol prefix `lifted`) and each feature set's own (`alone`), and builds
  one program per feature set with that set's `-D`: `c11_feature_conformance_{none,unicast,accept}` drive both
  machines through every run of up to four events and compare what a host observes, and
  `c11_feature_conformance_refuses_a_broken_edge` holds a build that breaks the edge to the header's `#error`. Two
  machines of one name in one program needed the host's table nested under the prefix like every other symbol
  (`native_action::render_with_options`); it had kept the bare name.
- **The C++ dialect and lane** (2026-11-04). C++ is the C11 dialect read with C++'s tokens: `generate` and `check`
  with `-l cpp` write the header and the `.inl` of method bodies for every build, the header ending with the same
  `#error`s. C++ adds `::`, `.*`, `->*` and `<=>` to C's punctuators, and reads `<::` as `<` and `::` where the
  next character is neither `:` nor `>` ([lex.pptoken]): before that rule `shared_ptr<::SCE::IScriptEngine>` opened
  a `[` nothing closed, and the namespace that held it could not be selected. `tests/CMakeLists.txt` builds
  `cpp_feature_conformance_{none,unicast,accept}` and `cpp_feature_conformance_refuses_a_broken_edge` as the C11
  lane does, the two machines kept apart by their namespace prefixes. Two changes of the core came with it: a slot's
  leading text is taken from a version in which the slot follows the one written before it, so a comment over a
  member only some variants write is written once rather than again before the next member; and a refusal names the
  deepest list that could not be merged, not the member that holds it.
- **The saved state records its configuration** (§7, 2026-11-04). A variant's model knows the configuration it is
  (`SCXMLModel::configuration`), the generator's `SavedShape` carries it, and a saved state writes it as `features`,
  sorted, in the root and each session whose document is of a product line (both formats, an optional member added
  additively). The Rust runtime judges it before the shape (`saved_state::check_features`) and names the features
  on only where the state was saved and those on only in the restoring build. The machine for every build states
  each feature under its own `#[cfg]` in one array, so each build records its own set. `tests/product_line/session.scxml`
  is now `sce-static`, and the Rust conformance crate holds the machine for every build to each build's own on the
  saved text and on the refusal. The Kotlin, Go, Python, C++ and C11 runtimes then took the member with the same
  text and refusals (2026-11-04); the C++ and C11 lanes hold their machine for every build to each build's own on
  it as the Rust crate does.
- Still to do: Swift; the featured transition table; conditions in the forge AST and the sourcemap.

## Questions left open

None of the design.
