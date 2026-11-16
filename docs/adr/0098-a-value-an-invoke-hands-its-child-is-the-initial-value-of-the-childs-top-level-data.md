# ADR 0098 — A value an invoke hands its child is the initial value of the child's top-level data

- Status: Accepted (2026-11-12). Decided under the owner's standing rule of 2026-11-09: the most long-term correct
  structure, textbook in its foundations, cost ignored, decided without asking. It closes the parameter half of what
  docs/adr/0096 left owed ("Order of landing" put each child's parameters into the child's start, but not in what
  order or in place of what).
- Date: 2026-11-12
- Amends:
  - docs/adr/0096, "Decision" 2, whose "before an invoke's parameters are applied to the child" names when the
    parameters arrive but not what they replace; "Decision" 2 below replaces that clause;
  - docs/SCE_INTEGRATION_FIXTURE_LAYOUT.md, whose open debt "the C11 `_init` split" is closed by "Decision" 4;
  - the comment in `tools/codegen/templates/invoke_methods.jinja2` that says a `<data>`'s own expression "is
    ignored", which is what it will be once this lands and is not what it is now.
- Scope: how a parent's `<param>` and `namelist` values reach an invoked SCXML child in all seven backends and in the
  C++ Interpreter: the parent's half in each `invoke_methods` template, the child's half in each script-engine and
  state-machine template (`tools/codegen/templates/{scriptengine_helpers.jinja2,rust,go,kotlin,python,swift,c}/…`),
  the Interpreter's invoke and datamodel initialization, and the value transfer between two script sessions where a
  backend has one (C11's `lua_xfer_top_to_child`).

## Context

§scxml-6.4 states what happens to the values an `<invoke>` hands an SCXML child:

> If the 'name' of a `<param>` element in the `<invoke>` matches the 'id' of a `<data>` element in the top-level data
> declarations of the invoked session, the SCXML Processor MUST use the value of the `<param>` element as the initial
> value of the corresponding `<data>` element. (The top-level data declarations are those that are contained in the
> `<datamodel>` element that is a child of `<scxml>`.) (Note that this means that any value specified in the `<data>`
> element is ignored.) [...] If the names do not match, the Processor MUST NOT add the value of the `<param>` element
> or namelist key/value pair to the invoked session's data model.

Three requirements follow: the handed value takes the place of the `<data>`'s own value, so that value is never
evaluated; only top-level `<data>` is seeded; a name that matches no top-level `<data>` is not added. A survey of
the seven backends on 2026-11-12 found none that meets all three:

| Backend | The `<data>`'s own value | Which `<data>` can be seeded | When the values arrive |
|---|---|---|---|
| Rust | evaluated, then overwritten (`set_variable`, its error ignored) | top-level, filtered by the parent at generation | the child's start, before its top-level `<script>` |
| C++ | evaluated, then overwritten (a failure logged) | top-level, filtered by the parent | the same |
| Go | evaluated, then overwritten | any name the child's session holds, early-bound state `<data>` included (`HasVariable`) | the same |
| Kotlin | evaluated, then overwritten (an exception swallowed) | the same as Go (`hasVariable`) | the same |
| Python | evaluated, then overwritten | top-level, filtered by the parent | the same |
| Swift | **not evaluated**: the value is declared in its place | any `<data>`, state-local early- and late-bound included | at the `<data>`'s own declaration |
| C11 | evaluated by the child's `_init*`, then the parent writes a global | top-level, filtered by the parent | **after the child's start**: after its initial `<onentry>`, and for a child started by `_init` or `_init_with_clock` after its whole first macrostep |

The C++ Interpreter, the eighth channel the canonical fixtures run on, was not surveyed; its step begins by reading
it.

Evaluating and then overwriting is observable whenever the `<data>`'s own value is not a plain literal: an expression
that fails raises `error.execution` in a child whose datamodel the specification says never read it, and an
expression with an effect has it. Seeding by the child's runtime "does this variable exist" reaches state-local
`<data>`, which the specification excludes. C11's late arrival lets the child's `<onentry>`, its eventless transitions
and its first internal events run on defaults. C11 also carries only booleans, numbers and strings across sessions; an
object or array arrives as nil. The W3C fixtures that seed a child (226, 240, 241, 243, 244, 245, 276) pass a literal
over a `<data>` whose own value is `0` or absent and read it from a transition, so none of them tells these orders
apart, and the canonical integration fixtures read from transition bodies on purpose, to step around C11.

## Decision

### 1. The child's start owns the seeding

The values an invoke hands a child are delivered to the child before its start and are consumed by that start; the
parent evaluates every `<param>` and `namelist` entry (and raises its errors, §scxml-6.4), and hands every name it
evaluated. The child alone decides which names it takes, from its own top-level `<datamodel>`, known when it is
generated. A parent that drops names it knows the child does not declare does what the child would do and is
duplication; the filtering lives in the child.

### 2. A handed value is declared in place of the `<data>`'s own

When the child's start declares its top-level `<data>`, in document order, a `<data>` whose `id` a handed value names
is declared with that value: its `expr`, `src` or inline content is not evaluated, raises nothing and has no effect.
Every other top-level `<data>` takes its own value as before. State-local `<data>`, early- or late-bound, never takes
a handed value. A handed value that names no top-level `<data>` is not added to the child's datamodel. The order of
the start becomes: establish the session (docs/adr/0096); declare the top-level `<data>`, seeded; declare the
early-bound state `<data>`; run the top-level `<script>`; enter the initial configuration. So the top-level
`<script>`, every `<onentry>` and every condition of the first macrostep read the handed value.

A handed value that the child's session cannot hold is the error of that `<data>`'s initialization (§scxml-5.3): the
child raises `error.execution` and the `<data>` is left as a `<data>` that failed, as one whose own expression failed
would be.

### 3. A value crosses sessions whole

A handed value is a datamodel value (null, boolean, number, string, array or object, as §scxml-B-2 lets an ECMAScript
datamodel hold) and arrives in the child equal to what the parent evaluated. A backend that copies between two
script sessions copies the whole value, structure included.

### 4. C11 seeds inside the child's start

C11's `_init_with_parent` and every `_init*` an invoke starts a child through take the handed values before the
child's datamodel is declared, so `invoke_seed_child` after the child's entry is removed, and with it the open debt
"the C11 `_init` split" in docs/SCE_INTEGRATION_FIXTURE_LAYOUT.md.

### 5. Held by one canonical fixture on every backend

A canonical integration fixture holds the three requirements and the order on every channel
(docs/SCE_INTEGRATION_FIXTURE_LAYOUT.md: the C++ Interpreter, C++ AOT, Rust, Go, Kotlin, Python and C11, and Swift,
which runs the integration stems too): a child whose
top-level `<data>` the parent seeds has an own expression that would raise `error.execution`; another has an own
expression with an observable effect; a state-local `<data>` shares a seeded name and keeps its own value; a name the
child does not declare is absent; an object value arrives whole; the child's top-level `<script>` and its initial
`<onentry>` read the seeded values. The child reports what it saw to its parent. A mutation casefile per backend
holds the order (a mutant that evaluates the `<data>`'s own value, or seeds after entry, is killed).

## Consequences

- Parents stop filtering handed names, so a parent's generated invoke no longer depends on the child's datamodel; the
  child's generated start gains its top-level `<data>` ids, which it already knows.
- A child's `error.execution` count changes where the old order evaluated a failing `<data>` it now skips. That is the
  defect fixed, not a regression.
- Swift stops seeding state-local `<data>`; Go and Kotlin stop seeding early-bound state `<data>`.
- C11's children see their parameters from their first `<onentry>`, and objects and arrays reach them.

## Considered alternatives

- **Evaluate, then overwrite** (five backends today): violates "any value specified in the `<data>` element is
  ignored" whenever the value can fail or has an effect.
- **The parent filters, the child trusts** (Rust, C++, Python, C11 today): correct only while every parent is
  generated with its child's datamodel in view, and duplicates in each parent what the child knows.
- **Seed any variable the child's session holds** (Go, Kotlin): reaches state-local `<data>`, which §scxml-6.4
  excludes by naming the top-level declarations.

## Order of landing

A canonical fixture is a commitment of every channel, and one whose semantics only some backends implement does not
belong in `integration_resources/` (docs/SCE_INTEGRATION_FIXTURE_LAYOUT.md). So each backend's landing carries its
templates or engine, its regenerated trees, a test of its own over the cases of "Decision" 5 and its casefile, and the
fixture lands last, with the runners of every channel:

1. Swift (closest: drop state-local seeding, take names by the child's own top-level ids);
2. Rust;
3. C++, the AOT templates and the Interpreter;
4. Go;
5. Kotlin;
6. Python;
7. C11 (seeding inside the child's start, whole values across Lua states);
8. the canonical fixture and its runners on every channel, and the layout document's open debt closed.

Swift, Rust and C++ landed one at a time (2026-11-12 and 13). Go, Kotlin, Python and C11 land together with step 8,
on one branch, each backend's own commits first.

## Decided while landing steps 4 to 8 (2026-11-16)

### The canonical fixture judges in its parent, and judges the child's errors last

`integration_resources/a_child_takes_a_handed_value_in_place_of_its_data/` has four children, one phase each: six
`<param>`s (an own expression that raises, one that acts, a state's early-bound `<data>` sharing a handed name, a name
nobody declares, an object, and a plain value judged last); a `namelist` of three (an own expression that raises, an
undeclared name, an object); a late-bound child whose state's `<data>` shares a handed name; and a child that does
not address its parent, whose first macrostep is an eventless transition reading the handed value. The children
report what they saw; the parent keeps each report in a `<data>` of its own and judges it with eventless
transitions, landing a failure in a `<final>` named for the sentence that broke. A driver asserts `pass` and may read
the kept values. The parent judges the child's error count after every other value, so a driver over an engine that
refuses to hold the one value judged after it (`refusable`) reaches `failChildRaised` only with every other value in
place: that is how the C++ (Interpreter and AOT), Rust, Go and Kotlin drivers hold §scxml-5.3's answer for a value
the session cannot hold, through one document. The child of the first phase writes into the object it was handed
after it reports, and the parent judges that its own `record` is untouched: a child handed the parent's object
rather than a copy (Rhino before the Kotlin step) fails "Decision" 3.

The documents each backend's step carried beside its driver were folded into this one. Two questions only one
channel can ask stay outside it: the bounds of C11's copy between two Lua states
(`backends/c/tests/integration_resources/a_handed_value_past_its_bounds_is_its_datas_error/`, whose target lowers
the bounds), and a `datamodel="sce-static"` child (below).

### The typed `InvokeParams` of a static child

"Decision" 1 gives the choice of names to the child. A `datamodel="sce-static"` child is the one exception, and it is
not a duplication of the child's rule: a static child's variables are typed, so the parent has to know the variable
each value lands in to type the value at all. The parent builds the child's `InvokeParams` (C11: writes the child's
variables between `_invoked_begin` and `_invoked_enter`), and a `<param>` or `namelist` name the child declares no
top-level `<data>` for is refused when the parent is generated (`static_datamodel.rs`, "the child declares no
top-level `<data id=…>`, so the value would be dropped"), which is "Decision" 2's "not added", decided before the run
rather than at it. A script-engine child is handed every name its parent evaluated and takes what its own top-level
`<data>` declares, so no parent reads a script-engine child's declarations any more, and the list the build carried
for that (`child_datamodel_vars`) is gone from the model and the forge AST.

"Decision" 2 holds for a static child as well: a handed value is its variable's initial value, so a variable
declared after it and initialized from it reads the handed value. `tests/integration/a_static_child_declares_its_data_with_a_handed_value.scxml`
holds it (`worker` declares `copied = start` after a handed `start`). Rust's `accept_params`, Go's `AcceptParams`
and Python's `accept_params` declare the variables again in document order, each handed one with its value and every
other one with its own initial value (which can neither fail nor act, so the second declaration is not observable),
and are driven on that fixture. **Open, registered here:** C++ (`acceptParams` in `state_machine.jinja2`), Kotlin
(`acceptParams` in `kotlin/state_machine.kt.jinja2`), Swift (`acceptParams` in `swift/state_machine.swift.jinja2`)
still write a handed value over the variable after every variable was declared, and C11 writes it between
`_invoked_begin` and `_invoked_enter`, after `_invoked_begin` declared them all. Each needs the same document-order
declaration and a driver of the static fixture; C++, Kotlin and Swift declare their variables as members whose
initial values read the members declared before them, so assigning each member in document order, handed or own, is
the whole change there, and C11 needs `_invoked_enter` (or a step between) to declare again the variables after the
first handed one.

### A test that saw no error where the session could not hold a value

Swift's `aHandedValueTheSessionCannotHoldIsTheDatasInitializationError` saw no error.execution for a handed text
that is not JSON. The test was right and the code was not: `JSON.parse` answers nil for text that is not JSON, the
same as for JSON `null`, so the value was declared absent and raised nothing. 2a5f046d44 reads the seed with the
whole-document reading, which says whether the text was JSON, and fails the declaration when it was not.
