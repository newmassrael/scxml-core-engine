# ADR 0048 — A statechart calls the forge kinds that are functions as it calls an algorithm

- Status: Accepted (2026-10-21) and implemented
- Date: 2026-10-21
- Scope: the imports a `sce-static` statechart reads (`forge::static_imports`), the callee it
  registers (`forge::type_ctx::StaticCallee`, `CalleeKind`), how a call's value is read from what
  the function answers (`forge::types::CallReading`, `forge::expr`'s `lower_call_readings`), how
  each generated backend spells the call (`forge::static_lowering::generated_callee`,
  `forge::generator::StatelessFunction`, `validator_check_call`, `c11_lookup_found`), the
  Interpreter's refusal (`forge::static_js`), the Rust validator template; the forge runtimes'
  `found` (ADR 0046); docs/SCE_ACCEPTED_SUBSET.md §2.15 ("Calling another kind")
- Related: ADR 0046 (a reading that may find nothing fails as `out-of-range`), ADR 0005
  (`sce-static` closes its gaps by bounding and declaring), SCE_FORGE.md §4.12 (a forge kind calls
  an imported transform, condition, lookup, interpolation or algorithm by its alias)

## Context

A forge kind calls the stateless documents it imports by their alias: a transform's expression may
read `scale(raw)` where `scale` is an imported lookup, and the call is spelled as the function the
lookup's own generation emits. A `sce-static` statechart could do that for one kind only. Read in
the code on 2026-10-21: `static_imports::resolve` read the imports of `kind="algorithm"` and no
other, so a statechart that imported a validator, a lookup or a transform and called it from a
guard or an assignment was refused as calling a name nothing provides. To call one, an author had
to write an algorithm whose body restated the lookup's table or the validator's ranges: a second
copy of a rule a document already states, which the two copies are then free to disagree about.

Five kinds are functions of their inputs: a transform computes its outputs from its inputs, a
condition its truth value, a lookup the value its table pairs with a key, an interpolation the
value between its breakpoints, and a validator whether its inputs are valid. The other kinds keep
state from one call to the next (a filter, an observer, a timer, a codec's instance, a procedure, a
queue, a collection) or declare a type (an enum, an event schema), and are not called.

What is to be decided: which of those a statechart calls, with what signature, how each generated
backend spells the call, and what a document is refused for.

## Decision

**A statechart imports a transform, condition, lookup, interpolation or validator as it imports
an algorithm, and calls it as `Alias(args)`.** The import is `<sce:import kind="<kind>"
src="..." as="Alias"/>`, read where the document is parsed, beside its algorithms
(`static_imports::resolve`). The call is judged by the signature a forge kind importing the same
document calls it by (`discover_stateless_signature`): its inputs in the order the document
declares them, and its one value — a transform's output, a condition's truth value, a lookup's or
an interpolation's output, and for a validator whether the inputs are valid, a `bool`. A callee
imported and never called is refused, as an unused algorithm already is.

**Each backend calls the function the document's own generation emits, through the import a
forge kind writes for it** (`forge_import_identity`, the stateless form: the module, never a type
of it, so no name of the document stands beside the machine's own). The function's name is read
once (`StatelessFunction`) and spelled by the symbol helpers each kind's renderer defines it with,
so a forge kind, a statechart and the definition cannot disagree; `discover_primary_function` and
`forge_qualified_call`, which spelled it for forge kinds alone, are folded into it.

**A validator is built for the call and asked once; the call's value is its validity.** Every
backend's validator is an instance with a `validate` method that answers a `ValidationResult`; a
validator that keeps nothing from one check to the next is the same whichever instance asks, so the
statechart builds one at the call (`validator_check_call`: `<module>::<Type>::new().validate`,
`<Type>().validate`, `<module>.New<Type>().Validate`, …, and C's free `<name>_validate`), and the
expression layer reads `valid` (`Valid` on Go) off what it answers (`CallReading::Validity`).

**A lookup whose miss is an error may fail, and a miss is `out-of-range`.** The lookup answers the
value or that its table holds none, in the backend's own optional shape — the shape an enum's
`fromNumber` answers in (ADR 0046) — so it is read the same way: through the runtime's `found`
where the call's failure is received (`CallReading::Found`), a failed assignment writing nothing
and raising `error.execution` (§scxml-4.9). The helper ADR 0046 introduced as `declared` is named
`found` for that reason. C's lookup answers through a pointer, which no expression holds: the
machine declares `<function>_found`, a reading that answers the result and `_take` every C call
that may fail answers (`c11_lookup_found`, sharing the result struct and `_take` of a `may-fail`
algorithm), and calls it in the lookup's place. A lookup with a default answers that default.

**What a statechart cannot call as the document declares it is refused where it is imported**,
`scxml/static-datamodel-rule` naming the import and why:

- a transform of more than one output (no one call is the value of two), and one that reads
  `previous()`, which keeps a value from one activation to the next that a call keeps nowhere;
- a lookup whose output is a string, which a lookup answers as a type of its own (an enum its
  generation declares) that no statechart variable holds;
- an interpolation whose `sce:out-of-bounds` is not `clamp`: every backend's generated function
  clamps, whatever the document declares, so a call would not do what the document says;
- a validator with a rate-of-change rule or a stateful import, which keeps what it read, and which
  a validator built for each call would forget;
- a slot that is not a number or a truth value: these kinds take and answer numbers on every
  backend alike, and a string, bytes or an enum would need the crossing an algorithm's slots have
  (§2.15, "State an algorithm takes and gives back") and these kinds do not declare.

**The Interpreter refuses such a call by name.** Its library carries algorithms, lowered to the
functions the lowered document installs (`static_js_algorithm`); it has no lowering of these kinds
yet, and says so: "a call of an imported lookup has no ecmascript lowering yet". The scenario that
holds the decision (`scenarios/static_forge_calls.json`) is replayed by the six generated backends
and listed as not yet lowered by the Interpreter's replay.

**The Rust validator is `Default` and lets its comparisons stand.** A crate that holds a validator
a statechart calls is checked by clippy under `-D warnings`, which refuses a public type with a
`new()` and no `Default`, and would rewrite a range check into `contains` and a plausibility rule's
negation into its complement. Both rewrites read a NaN otherwise than the other five backends do,
so the comparisons stay as every backend writes them, under a per-item `allow`.

## Consequences

- A rule a lookup, validator or interpolation states once is called from a guard or an assignment
  as it is, on six backends, and the machine is generated with the document's own unit beside it.
- The regen scripts derive the documents a machine calls from every callable kind it imports, not
  from its algorithms alone; Rust's lists them as it lists its algorithms.
- A forge kind that calls a lookup whose miss is an error still reads the lookup's optional answer
  as the value (the signature `discover_stateless_signature` gives it has no `may-fail`): the
  statechart's reading does not repair that, and a fix belongs to the forge kinds' own import pass.
- The generated interpolation ignores `sce:out-of-bounds`. Until it honours `extrapolate` and
  `error`, a statechart calls a clamping interpolation only, and the refusal says why.

## Alternatives considered

- **A per-machine adapter on each backend for a validator and for a lookup that may miss.** It
  would have put six new functions in every machine that calls one, each written per backend, for
  what is an expression: a call with its value read off it. The expression layer already reads an
  enum's `fromNumber` through a runtime helper; the same reading serves the lookup, and a member
  read serves the validator, with C's lookup the one backend that needs a declaration.
- **A free `is_valid` function in every validator's generation.** It would have changed every
  committed validator output to add a second surface that says what `validate(...).valid` says.
- **A multi-output transform called as `Alias.output(args)`.** Left for when a document needs it:
  a transform of one output is the value of the call, and two outputs are refused by name meanwhile.
