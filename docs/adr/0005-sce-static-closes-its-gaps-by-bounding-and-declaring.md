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
| a mesh `<invoke>` | a lowering in C++, whose generated router takes the request itself (the other five reach the host's router already) |
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
a capability a target does not have yet is refused by name, as "no lowering yet", with a
method that lifts the refusal.**

One document has one answer on every engine that runs it. A construct that only some
engines can hold is therefore refused on all of them until all can. The mesh is the one
exception, because it is a service an engine has a route to or does not: the six
generated languages lower a mesh request through the route they serve it by, and the
Interpreter, which has no mesh route, refuses it by name (decision 5).

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
"`bytes` on the wire"). A value a request or a payload carries is a text of characters up
to U+00FF no longer than the bound, as a typed request's already is. A literal in the
document is narrower, and stays so: printable ASCII with no backslash, the one literal
whose bytes every engine spells alike (`decode_bytes_literal`, RFC bytesguard-3), of at
most the bound — widened when that helper is, for every engine at once. Assignment,
comparison with `===` / `!==`, and the string operations that have a byte meaning
(length, join under the bound) are lowered; an operation that has none is refused by
name.

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
<send event="go" typeexpr="kind"    sce:types="http://www.w3.org/TR/scxml/#SCXMLEventProcessor http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor"/>
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

As landed, a `typeexpr` entry is a processor every generated language and the Interpreter
deliver through, written by its URI: the SCXML Event I/O Processor and BasicHTTP. A type
the host serves is still chosen by a written `type`, which the host's declaration claims
after the parse; letting a computed type select one is a separate step, because the host
registry is a per-build input and the set a document declares would then depend on it.
A `targetexpr` and a `typeexpr` are not computed together: a target chosen among `#_`
locations is no address for the HTTP processor the type may name.

A computed type has no template of its own in any language. The walk expands the `<send>`
into an `<if>` whose branches are one `<send>` of a written `type` for each declared
entry, taken when the value equals it, and whose `<else>` is a `<send>` of a type nothing
delivers through. Every engine already answers that with `error.execution`, so the
choice is made once and each engine delivers by the arm it has for a written type; the
refusals a language holds for a written type apply to each declared entry.

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

As landed for all six generated languages: the machine hands its engine the text each
`<param>` spells, by the one rule every engine writes (a string as itself, an integer as
its decimal digits, a bool as `true` or `false`, a real as its ECMAScript `String()`), and
the engine's transport renders the form from it. Rust, Kotlin, Go and Python already did,
and nothing had ever read what they produced: no fixture sent BasicHTTP from a
`sce-static` machine. `static_send_http` is that fixture, held on each engine by a test
that records the request where the engine hands it to its transport (Rust, Kotlin, Go,
Python and C++ have a callback for that; C11 has none, and its test is the platform
instead, see below), so no listener is involved. A scenario states what a machine's fields
hold and not what it sent, so the fixture has none; the Interpreter creates its HTTP
client through a factory and runs the document's own `<send>`, and is held by the W3C
`harness: http` fixtures as before. C++ and C11 refused the construct by name until now,
and both refusals had outlived their reason. C++'s was written before the machine filled
the text map. C11's rested on a belief that it had no spelling of a typed value as wire
text and no client to make the POST, and it has both: its host-served arm writes each pair
through the wire helper (`sce_forge_wire_text`) into the request `_perform_basic_http`
takes, and the client is the surface of `sce/http_client.h`. Both are removed with no new
template code. That surface keeps its early `sce_test_http_` names and its header still
calls itself test support, which is a naming debt and not a limit: the script-engine
machines already depend on it, and a platform supplies it (`sce_c_runtime_posix` for
POSIX). C11's test is that platform: it provides the client itself, keeps the request
instead of making it, and is linked without the POSIX runtime, so its client is the only
definition of those symbols.

### 5. A mesh `<invoke>` is lowered through the route its backend serves it by

Every backend serves `<invoke type="sce:mesh-rpc">` by exactly one of two routes
(`SCE_MESH.md` §9.5, "Backend coverage"). C++ generates its own `TransportRouter`, and the
request lowers into that router's `performMeshInvoke`. The other five hand the request to
a router the host registers: the build rewrites the invoke into a host-served `<invoke>`
of that type before the static lowering sees it, and a `sce-static` machine already
lowers the `<param>`s of one (`SCE_ACCEPTED_SUBSET.md` §2.12, "Types the host runs").

This decision first read the mesh as a runtime the five lacked, to be lowered one
backend at a time as each gained it, and read the Interpreter as C++ and so as lowering
it with C++. Both were wrong, and the tree says so: the five serve a static mesh request
today with no change, which `a_static_datamodel_holds_every_variable_to_a_type` holds
for each of them, and the Interpreter is an engine of its own that has no mesh route to
lower into. What was left was C++, the one backend where the request reaches the
lowering as itself, and which refused it by name.

As landed, C++ computes each `<param>` of a mesh request from the machine's own fields
when the invocation starts, as it does a host-served `<invoke>`'s, and hands the router
the text of each (a string as itself, an integer as its decimal digits, a bool as `true`
or `false`, a real as its ECMAScript `String()`) beside the typed value. A value that
failed is the evaluation that failed (§scxml-5.7.1): `error.execution` is raised, the pair
is left out, and the request still goes. The peer is the `src` the invoke writes; naming
it by `srcexpr` is decision 7. An `idlocation` is refused here as on every `<invoke>` of
this model.
`tests/mesh/test_mesh_static_invoke_request.cpp` records the request where the router
hands it to its link, with no transport, and links no script engine: the link is the
proof that no param reached one.

The Interpreter stays refused, by name, as "a mesh `<invoke>`". Giving it a mesh route
is a product decision about the Interpreter and not about `sce-static`, and a lowering
written for it here would be the only code in that engine that a `sce-static` document
alone could reach.

Rejected:

- *Refusing the five by contract, with a message that says the mesh is C++ only.* It
  would write a limit the design does not have, and it would refuse documents the five
  already serve.
- *Lowering the request in C++ through the script-engine arm.* A `sce-static` machine has
  none, and the arm's evaluation is the text this model exists not to run.

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
The Interpreter has no interface to generate, so the operation is one virtual on the
host it already takes, `INativeActionHost::hostForChild(invokeId, document)`, which the
engine asks as the invocation starts and whose answer it installs on the child before the
child runs; `document` is the stem of the document the child came from, which names the
candidate of a hybrid `<invoke>` as the suffix of `actions_for_<invoke>_<stem>` does, and
the default answers none.

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

### 7. A mesh request names its peer by a string the machine computes, among the bindings the deployment declares

`<invoke type="sce:mesh-rpc" srcexpr="peer">` picks its peer when the invocation starts.
`SCE_MESH.md` §9.5 already says what it can pick among: the bindings the deployment
declares, and nothing it discovers. A name that matches none of them is a setup fault of
the pre-envelope tier, `error.execution`, and so is one that is not a
`#<machine_name>`. That is the declared set this model asks for of a computed choice, and
the build already reads it, so the document declares nothing more (`sce:targets` is
written on a `<send>` because no deployment lists the routes it can take).

As landed, the attribute is a string expression, held to a string and lowered natively.
C++ keeps its own route, so it is the one backend whose machine reads the peer itself:
the string is computed from the machine's fields when the invocation starts, shaped
against `#<machine_name>`, stashed for the `<cancel>` the state's exit pairs with it, and
handed to the router's `performMeshInvoke`, whose lookup is the deployment's bindings. A
value that cannot be computed (a checked operation that fails) is an attribute that cannot
be evaluated: `error.execution` once and nothing is sent, as it is for a hybrid
`<invoke srcexpr>`. The other five hand the request to the host's router as a host-served
`<invoke>`, and every one of them already lowers the `srcexpr` of such an invoke
(`StaticTarget::lowers_host_src_expr`), so the document needs nothing from them. The
Interpreter stays refused by name, as in decision 5. An `idlocation` stays refused.
`tests/mesh/test_mesh_static_invoke_srcexpr.cpp` holds the peer the field names, a
well-formed name no binding carries, a name of the wrong shape, a peer that cannot be
computed, the peer read again at each start, and the reply reaching the machine.

Rejected:

- *Declaring the peers on the invoke* (`sce:peers`). The deployment is the list, the build
  reads it, and a second one in the document could differ from it: a name in one and not
  the other would be a fault only the machine's run could find.
- *Checking the name against the deployment when the build runs.* The value is computed
  when the request is made, which the build cannot see; the router's answer is the check,
  and it is the one a document under a script engine already gets.

## Order of work and what lifts each refusal

Each refusal is a method on the target (`StaticTarget::lowers_…`) that defaults to
refusing; an engine flips it in the same commit that lands its lowering and a fixture
that every engine that has flipped it runs. A refusal is removed from the shared check
only when the last engine has flipped it.

1. A record's `string` field (decision 1), then `bytes` (decision 2), which reuses the
   bound machinery. Fixtures `static_record_string`, `static_bytes`.
2. The child's host (decision 6). Fixture: a child declaring an act, invoked, restored,
   and invoked again.
3. `targetexpr` / `typeexpr` (decision 3), then BasicHTTP `<param>`s (decision 4).
4. The mesh `<invoke>` (decision 5): the C++ lowering. The other five serve the request
   through the host's router and need none; the Interpreter has no mesh route. Then its
   peer by `srcexpr` (decision 7), the same split.

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
