# ADR 0005 — `sce-static`: each remaining gap is closed by bounding a value or declaring a set

- Status: Accepted
- Date: 2026-10-06
- Scope: `sce-build` static lowering (`forge/static_lowering.rs`), the seven engines that
  run a `datamodel="sce-static"` document (Rust, Kotlin, Go, C++, Python, C11 and the
  Interpreter), `docs/SCE_ACCEPTED_SUBSET.md` §2.15
- Related: `docs/SCE_ACCEPTED_SUBSET.md` (§2.13 hybrid `<invoke>`, §2.15), `SCE_MESH.md`
  §9.5, `ARCHITECTURE.md`, `docs/adr/0003-one-portable-document-compiled-per-target.md`

## Context

A `sce-static` machine has no script engine. Everything it does is native code written
by the generator, so the generator has to know, at build time, how much room a value
takes and which routes the machine can ever use. The constructs the model still
refuses are the ones where one of those two things is missing:

| Refused today | What is missing |
|---|---|
| a record with a `string` field | a bound for the field |
| `bytes` (variable, record field, payload field) | a bound, and a spelling on the wire |
| `<send targetexpr>` / `<send typeexpr>` | the set of values the expression can take |
| a `<send>` to another processor | the same set, plus a lowering of the processor's own type |
| a mesh `<invoke>` | a statement of which targets the mesh exists for |
| an `<invoke type="scxml">` of a child that declares `<sce:action>`s | a way for the parent to give the child its host |

Two of these were measured on 2026-10-06 to be worse than "not yet written":

- A record with a `string` field is accepted by `sce-codegen check` for Rust, Kotlin, Go,
  C++ and Python, and refused only by C11. Rust writes `#[derive(Clone, Copy)]` over a
  `String` field, which does not compile (E0204). No committed document declares such a
  record, so nothing depended on it.
- A parent that invokes a child declaring `<sce:action>`s was accepted by every language
  but C11, and every one of them wrote a call of the child's constructor with its host
  left out. That is refused by every language since `450da98d63`.

In both, the refusal lived inside one target, so the others passed a document whose
output did not build. The decisions below put each refusal where every target asks it
once, and say what closes each gap, so the refusals can be lifted one engine at a time
against a fixture rather than by judgement.

## Principle

**Nothing a `sce-static` machine does is decided by text evaluated at run time. A value
is bounded where it is declared, a choice is made among a set the document declares, and
a capability a target does not have is refused by name — by contract where the contract
says the target will never have it, as "no lowering yet" only where it will.**

One document has one answer on every engine that runs it. A construct that only some
engines can hold is therefore refused on all of them until all can, or refused by
contract on the ones that never will.

## Decisions

### 1. A record's `string` field is bounded by its schema's `sce:max-size`, which is required

An event-schema field `<data id="label" sce:type="string" sce:max-size="8"/>` held by a
record variable (or a list of records) is a string of at most 8 UTF-8 bytes, held and
checked as a string variable of `sce:capacity="8"` is: a value past the bound is the
evaluation that failed (§scxml-5.7.1) — `error.execution`, and the field is left as it
was. The bound applies to `<sce:set>`, an `<assign location="last.label">`, a payload
field read into the record, a saved state being restored, and a `<foreach>` item copied
out of a list.

A record that has a string field the schema does not bound is refused at the `<data>` as
`scxml/static-datamodel-rule`, naming the field and the attribute to add. There is no
default bound.

Rejected:

- *A default bound, as a forge `bytes` slot has (`BYTES_DEFAULT_MAX`, 256).* The `sce-static`
  model writes no value the author did not: no zero, no empty string, no first variant
  (§2.15, "`<data>` without `expr`"). A default would be a limit nobody wrote that
  fails a value at run time, which is the failure this model exists to name at build time.
- *Unbounded where the engine can hold one (Rust, Kotlin, Go, C++, Python), bounded only
  in C11.* The same document would accept a value on five engines and refuse it on the
  sixth. Measured 2026-10-06, the five do not even hold the unbounded form: Rust does
  not compile it.
- *A new attribute (`sce:capacity` on a schema field).* `sce:max-size` already means "the
  most bytes a `string` or `bytes` entry holds" in forge codecs and in a typed request
  (`docs/SCE_ACCEPTED_SUBSET.md`, "A typed request is held to its record"), and the
  schema parser already accepts it on a field. `sce:capacity` on a schema `<data>` is
  refused today as not read by SCE, and stays so.

Consequence for Rust: a record with a string field is `Clone`, not `Copy`; the machine's
own copies of a record are written as clones. A record without one stays `Copy`.

### 2. `bytes` is a bounded byte string held as a string is, and rides the wire as Latin-1 text

A `bytes` variable declares `sce:capacity` (required, in bytes), a record's or a
payload's `bytes` field declares `sce:max-size` (required), exactly as a string does.
On the wire, in a `<param>`, in a saved state and in a snapshot it is its byte-exact
Latin-1 text — the contract the typed payload already states (`SCE_ACCEPTED_SUBSET.md`,
"`bytes` on the wire"). A literal is a text of characters up to U+00FF no longer than the
bound, as a typed request's already is. Assignment, comparison with `===` / `!==`, and
the string operations that have a byte meaning (length, join under the bound) are
lowered; an operation that has none is refused by name.

A transition on an event whose payload carries a `bytes` field is lowered like any typed
payload: the field is read into the variable or record field that holds it under the
same bound.

Rejected:

- *Base64 or hex on the wire.* The Latin-1 spelling is already the contract on the
  Interpreter-visible surface; a second spelling would make the same event two different
  texts depending on the engine that wrote it.
- *A default capacity.* For the reason given in decision 1.

### 3. `targetexpr` and `typeexpr` choose among a declared set

A `<send>` that writes `targetexpr` or `typeexpr` declares the values it can take, as a
hybrid `<invoke>` declares `sce:candidates`:

```xml
<send event="go" targetexpr="route" sce:targets="#_parent #_internal"/>
<send event="go" typeexpr="kind"    sce:types="http scxml"/>
```

The expression is a string expression, lowered natively. The machine compares its value
to the declared set when the send runs and sends by the matching entry. A value in none
of them is `error.communication` for a target and `error.execution` for a type, the two
errors the specification gives a send whose route cannot be used, and nothing is sent.
Three refusals guard the declaration at build time, as they do for `sce:candidates`: an
expression with no declared set (there is no finite list to lower), two entries naming
one route, and an attribute written and left empty. A declared entry that neither the
machine's own processors nor the host's declared types (`--host-processor`) serve is
refused where it is declared, by name.

Rejected:

- *Evaluating the expression as script text.* `sce-static` has no script engine; that is
  the model.
- *Passing the value to the host unvalidated.* The host would receive a route the build
  never saw, and `computed_routes` in the manifest (`SCE_ERROR_CONTRACT.md` §10) would
  list a route no document declares.

### 4. A `<send>` to another processor is lowered through the processor the runtime already has

`type="BasicHTTPEventProcessor"` is served by every language's runtime. The static
lowering writes the `<param>`s of such a send as the form-encoded text the processor
sends (each value as the text a form carries), and the processor is the runtime's, not
generated code. A type the host serves is lowered already (`--host-processor`); a type
neither the runtime nor the host serves stays refused by name at the `<send>`.

### 5. A mesh `<invoke>` is lowered for C++ and refused by contract for the others

`SCE_MESH.md` ("Language coverage") states that the mesh runtime targets C++ only, and
that a mesh construct addressed to another backend is refused at build time rather than
dropped. The static lowering follows it: `<invoke type="sce:mesh-rpc">` is lowered by the
C++ target (and so by the Interpreter, which is C++), and refused by Rust, Kotlin, Go,
Python and C11 as a contract — the message says the mesh is C++ only, not that a
lowering is missing — until `SCE_MESH.md` says otherwise.

### 6. The parent gives a child its host through a factory on the parent's own host

A child that declares `<sce:action>`s takes its host when it is built, because its first
`<onentry>` can already perform an act (a host installed afterwards arrives one act too
late). So the host has to exist when the invoke starts, and the parent has to be the one
that obtains it. The parent's own host interface gains, for each `<invoke type="scxml">`
whose child declares acts, one operation that answers the child's host:

```
fn actions_for_worker(&mut self) -> WorkerActions      // Rust; the others spell it alike
```

The machine calls it each time the invocation starts — on entry of the invoking state,
on a restore that starts the child again from its beginning (§2.15, "Running
invocations"), and for each candidate of a hybrid `<invoke>` that declares acts — and
builds the child with what it returns. In C11, whose child is a value the parent holds,
the parent's act table carries the function and the child's table is what it returns.

Rejected:

- *A registration call before the machine starts* (`set_worker_actions(...)`). An invoke
  can start from the initial state's `<onentry>`, before any call the host makes after
  construction: one act too late, the reason the host is a constructor parameter.
- *All children's hosts in the parent's constructor.* A state that invokes on every entry
  starts a fresh child each time, and a hybrid `<invoke>` has one child per candidate;
  one fixed instance cannot be both, and it would carry state across invocations.
- *The child's acts merged into the parent's interface.* The two documents' act names are
  independent and may collide, and the child's acts are the child document's own
  contract, which a merged interface would let the parent's host answer differently from
  the document that declares them.

## Order of work and what lifts each refusal

Each refusal is a method on the target (`StaticTarget::lowers_…`) that defaults to
refusing; an engine flips it in the same commit that lands its lowering and a fixture
that every engine that has flipped it runs. A refusal is removed from the shared check
only when the last engine has flipped it, and replaced by decision 5's contract refusal
where it applies.

1. A record's `string` field (decision 1), then `bytes` (decision 2), which reuses the
   bound machinery. Fixtures `static_record_string`, `static_bytes`.
2. The child's host (decision 6). Fixture: a child declaring an act, invoked, restored,
   and invoked again.
3. `targetexpr` / `typeexpr` (decision 3), then BasicHTTP `<param>`s (decision 4).
4. The mesh `<invoke>` (decision 5): the C++ lowering and the contract refusal wording.

## Consequences

- The accepted subset (`SCE_ACCEPTED_SUBSET.md` §2.15) records a construct as accepted
  only when every engine that is not refused by contract runs it; "refused until its
  spelling is written" is a state with a named method that lifts it, never a prose
  promise inside one target.
- A document written for the current subset does not change meaning: each decision adds
  a declaration (`sce:max-size`, `sce:targets`, `sce:types`) to something that is
  refused today.
- Decision 1 makes a Rust record with a string field a non-`Copy` type, which is a
  property of the generated API of that record alone.
