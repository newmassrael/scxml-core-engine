# ADR 0057 — An element may require a feature, and every world of the features is a valid machine

- Status: Proposed (2026-10-26); not yet implemented. Requested by watching-zenoh on the owner's instruction
  ("kconfig와 sce code gen으로 기능을 껐다 켰다 자유롭게 조작할 수 있어야 해"), and designed under the owner's rule of
  the same day: the most long-term correct structure, textbook in its foundations, cost ignored ("비용 무시하고 가장
  장기적으로 올바르면서, 학술적으로 교과서적이고, 궁극적인 구조로 설계해"). The surface the consumer confirmed on
  2026-10-26 (§3) is unchanged by that rule; the foundations under it are what the rule set
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
  `docs/SCE_CODEGEN_DETERMINISM.md`, watching-zenoh `docs/feature_inventory.md` §8 and `docs/upstream-parity.md` D7

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

The product line's features are declared once, by the product, and every statechart of it reads the same model; a
declaration inside each statechart would be one copy per document. A statechart imports at most one feature model.
A consumer that keeps its own declaration (watching-zenoh's `deploy/wz-config.toml`) generates this document from it,
so the product has one source and SCE reads its own format. Feature names are `[a-z][a-z0-9]*(-[a-z0-9]+)*`; each is
also spelled for every backend (§6), and two names that spell the same in any backend are refused.

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

New diagnostic codes (names provisional): `validation/feature-model-cycle`, `validation/feature-name-collision`,
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
would compile code the model never allowed. Every backend that keeps variability emits the model's edges as refusals:
`#[cfg(all(feature = "a", not(feature = "b")))] compile_error!(...)` in Rust, `#error` in C and C++, and the
Cargo `[features]` fragment that makes Cargo enforce the edges in the first place. The `not` there refuses a build;
it selects no code.

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

## Questions left open

None of the design. The remaining choice is the owner's acceptance of it.
