# ADR 0055 — A value that may be absent is of an optional type, and absent is no value of any other

- Status: Proposed (2026-10-21), under the owner's standing rule: the most long-term correct way,
  cost ignored; never a sentinel number and never a companion flag that admits impossible states
- Date: 2026-10-21
- Amends: ADR 0044 decisions 1 and 3 (a revision may widen a field from `T` to `optional<T>`, and a
  revision's digest names each field's type at that revision); docs/SCE_ACCEPTED_SUBSET.md "What a
  revision does not make safe"
- Revisits: ADR 0052, "An optional reason" (see "What changed since ADR 0052" below)
- Scope: the value-type grammar (`forge::model::AlgorithmValueType`, `read_algorithm_value_type`,
  `read_type_attr`, the `dataValueType` of `schemas/sce-forge-ext.xsd`), an event schema's fields;
  the expression layer (`forge::types::InferredType`, `forge::expr`'s lexer, typer, `slot_admits`
  and the six emitters), the enum judge (`forge::static_enum`), the statechart's scope
  (`forge::type_ctx::StaticScope`); `forge::schema_revision`; `forge::static_lowering` and its
  saved shape; the payload lift and wire writer of every generated backend and of the
  Interpreter's library (`forge/static_js/sce_static.js`); the forge runtimes' `found`; the saved
  state of the Rust and Kotlin backends and `schemas/sce-saved-state.v{1,2}.schema.json`; the replay
  scenario vocabulary (`sce_build::scenario_replay`) and its seven drivers
- Related: ADR 0005 (`sce-static` closes its gaps by bounding and declaring), ADR 0044 (revisions),
  ADR 0045 (saved shapes), ADR 0047 ("a reading that answers none would need a type"), ADR 0048 and
  ADR 0051 (calls of other kinds), ADR 0050 (bounds), ADR 0053 (the bare-metal Rust profile);
  SCE_FORGE.md §4.6.1 and the content-line codec (whose optional entries already have a host type
  on every backend)

## Context

An application keeps, for an item it tracks, the day the item is due, as an `int64` count of days
since the epoch. The day is often not known when the item is created and is given later. The
application needs the empty day to be:

- distinguishable from every day: every `int64` is a day, so no number is free to mean "none";
- carried in a field of an event schema, held in a statechart variable and in a record's field;
- compared, saved and restored;
- replayed from a log of commands the application wrote at an older revision of the schema, in
  which the field was a plain `int64`.

What the language offers today, read in the code and the documents on 2026-10-21:

- **No type says "may be absent".** `sce:type` is a scalar (`SceType`), an `enum:<alias>`, and,
  for a statechart variable or an algorithm's slot, `list<T>` or `record:<alias>`
  (`AlgorithmValueType`). Every variable declares its initial value and "no zero, empty string or
  first variant stands in" (docs/SCE_ACCEPTED_SUBSET.md §2.15).
- **Three earlier decisions stopped at the missing type.** ADR 0047 refused a `fromNumber` that
  answers "none" because "a reading that answers none would need a type a statechart has no form
  for". ADR 0052 named "no refusal yet" by a variant `none` because "there is no optional value in
  `sce-static`", and rejected a per-field `sce:optional` as "a new type carried on six backends and
  in the saved state, to say what one variant says". ADR 0048 turns a lookup's miss into a failure
  through `found`; a statechart had no type to hold the miss in.
- **The expression layer already parses `null` and types it nowhere.** `null` lexes to
  `ExprKind::NullLit`, typed `InferredType::Null`, and `slot_admits` admits `Null` in every slot as
  it admits `Unknown` (`forge/expr.rs`). The lexer refuses `??` and `?.` by name ("nullish
  coalescing", "optional chaining").
- **The codecs already have absence, with one host type per backend.** A CBOR or content-line
  entry that is not `sce:required` is optional, generated as `Option<T>` (Rust), `T?` (Kotlin),
  `std::optional<T>` (C++), a pointer (Go), `Optional[T]` (Python) and a `<name>_present` member
  beside the value (C11); the conformance oracle spells an absent entry as JSON `null`. A positional
  field gated by `sce:present-if` is wrapped the same way, the wrap written out four times in
  `forge/generator.rs`.
- **A missing key already has a meaning on the event wire.** ADR 0044's lift reads a field a
  delivery lacks as an older sender's delivery: the field's `sce:default` when a later revision
  added it, and a refusal when the first revision declares it.
- **A changed type is not a revision** (ADR 0044 decision 1(a); docs/SCE_ACCEPTED_SUBSET.md "What a
  revision does not make safe"): a field re-typed between two revisions is refused on the network
  and in a saved state.

What is to be decided: the type, its expressions, its wire, its lowering on the six generated
backends and the Interpreter, and how a schema whose field becomes optional evolves.

## Decision

### 1. The type: `optional<T>`

`sce:type="optional<T>"` (in XML `optional&lt;T&gt;`, as `list&lt;T&gt;` is written) is a value
that is either absent or a value of `T`. Absent is a value of `optional<T>` and of no other type.

**What `T` may be.**

| `T` | Where | Why |
|---|---|---|
| a fixed-width number (`int8` … `uint64`, `float32`, `float64`), `bool` | everywhere the type is admitted | the case that has no spare value to give up |
| `string`, `bytes` | everywhere, with the bound `T` declares (`sce:capacity` on a variable, `sce:max-size` on a field) | an empty text is a text; absence is not an empty text |
| `enum:<alias>` of a closed enum | everywhere | a variant is a value; absence is not a variant (see "What changed since ADR 0052") |
| `record:<alias>` | a statechart variable and an algorithm's parameter, return and local; not a field of an event schema | a schema has no record fields; a variable that holds "no record yet" is the case ADR 0052 met |

**Nesting.** Refused, each where the `sce:type` is written (`validation/invalid-attribute`, naming
what may stand inside):

- `optional<optional<T>>`: Kotlin's `T??`, Python's `None`, ECMAScript's `null` and JSON's `null`
  collapse the two absences into one, so four of the carriers cannot hold it, and nothing asks
  for it.
- `optional<list<T>>`: a list is a variable and never a value (§2.15, "List variables"), and its
  resting state, empty, is its own.
- `list<optional<T>>`: a list holds fixed-width numbers, `bool`s or records, so that a bound in
  elements is a bound in bytes; a list of values that may be absent is a list of records with an
  optional field.
- `optional<T>` for an open enum: refused as the open enum itself is in a statechart.

**Where it is admitted.** A `sce-static` statechart's variable, a field of an event schema (and so
a record's field and a payload's field), an algorithm's parameter, return and local. Every other
`sce:type` position — a codec entry, a transform's, lookup's, interpolation's, condition's or
validator's slot, a field that declares `sce:quantity` — refuses it by name (`validation/invalid-attribute`, "an optional
value is a type of an event schema, a statechart and an algorithm"), because those positions are
scalar by contract (the reason `AlgorithmValueType` exists apart from `SceType`). A codec keeps its
own spelling of absence, `sce:required` (decision 7).

**In the model.** `optional` is a constructor of the value type, as `list` is: `AlgorithmValueType`
gains `Optional { elem }` over an element type of its own (a scalar `SceType` or a record alias),
serialised untagged as `{"optional": "int64"}` so that the forge-AST of every document that holds
no optional value is byte-identical. An event schema's field, typed today by a bare `SceType` it
shares with every kind's `ForgeField`, is typed by the value type restricted to a scalar or an
optional scalar. The presence is never a boolean beside the scalar: every site that reads a field's
or a variable's type matches the constructor, so the compiler names each site that has not said
what it does with an optional value, as `ListElemType` keeps a record element from being skipped.
`InferredType` gains `Optional(OptionalElem)`, an element kept `Copy` as `ListElem` is.

**No new diagnostic code.** Every refusal below reuses a code that already says the same thing in
a neighbouring place; none needs an appendix entry in docs/SCE_ACCEPTED_SUBSET.md.

### 2. The expression surface

The forge expression language is a typed subset of ECMAScript, and it already spells absence the
way ECMAScript and JSON do. So:

| What | Written | Type | Can it fail? |
|---|---|---|---|
| absent | `null` | the `optional<T>` of the place it lands in | no |
| a present value | any value of `T` where an `optional<T>` is expected | `optional<T>` | no |
| is it present | `x !== null`, `x === null` | `bool` | no |
| the value, or a fallback | `x ?? d` (`d` of `T`, or of `optional<T>`) | `T` (or `optional<T>`) | only if `d` can |
| the value | `x.value` | `T` | yes, when `x` is absent |
| equality | `x === y`, `x !== y` | `bool` | no |

**`null` is typed by its place, and only an optional place takes it.** It stands where an
`optional<T>` is expected: an `<assign>` to, or `<data expr>` of, an optional variable, a
`<sce:set>` of an optional field, an argument of an optional parameter, an algorithm's optional
return, a branch of a conditional whose other branch is of `optional<T>` or of `T`, and an operand
of `===` / `!==` beside an optional value. Anywhere else it is `expression/type-mismatch` ("`null`
stands only where an optional value is expected"), and `slot_admits` stops admitting `Null` in
every slot. Every variable still declares its initial value: an optional one starts at `null` or at
a value, written in `expr` (or in its record's `<sce:set>`), and nothing stands in.

**A value of `T` is a value of `optional<T>`.** Where an optional is expected, a value of `T` is
admitted by the rule that admits it where `T` is (an integer of any width checked into the width,
a text held to the bound), and is present. The other direction is never implicit: an optional
value where a `T` is expected is `expression/type-mismatch`, naming `.value` and `??`. ADR 0047
refused an implicit conversion of an enum to its number because "it would make `n = status` mean
the number in one place and a refusal in another"; an implicit read of an optional would make
`due + 1` succeed in one run and fail in another with nothing written that says it might.

**Reading the value is a read that may find nothing.** `x.value` is the value of a present `x`. Of
an absent `x` it is the failure every reading that finds nothing already is: `out-of-range`, as an
index outside a list is and as `fromNumber` of a number no variant declares is (ADR 0047), since an
optional is a collection of at most one value. So in a statechart the statement fails — nothing is
written, `error.execution` is raised, the block ends (§scxml-4.9) — and a guard it stands in is
false; in a `<data expr>` it is refused at build time, as any initial value that can fail is; in an
algorithm only a body that declares `may-fail` may read it. It is lowered through the one helper
each runtime already has for this reading, `found` (decision 4), so no new failure channel exists.
`.value` on a record holds the read: `item.due.value`, `pick.value.title` for an
`optional<record:Item>`; a record field named `value` is still that field, since a member is
judged by the type of what it is read from.

**A read the same expression has already tested cannot fail.** Within one expression, `x !==
null && x.value < today`, `x === null || x.value < today` and `x !== null ? x.value : today` read
`x.value` only where the test has found `x` present, and an expression writes nothing in between
(no expression assigns; a call hands a copy and nothing reaches the machine's state). So a `.value`
on the guarded side of `&&`, `||` or `?:`, whose test is `x !== null` / `x === null` of the same
name — a variable, a parameter, a local, a field of a record variable, of a loop's record item or
of the payload, not a call — is typed as a read that cannot fail: it adds no `may-fail` to an
algorithm, it is admitted in a `<data expr>`, and it is lowered without the failure plumbing. The
test is read from the expression's tree, not from the statements around it; a test in an `<if>`
does not reach the `<assign>` inside it (an `<assign>` before the read could change the variable),
and narrowing across statements is not decided here.

**`??` is the total read.** `x ?? d` is the value of a present `x` and `d` of an absent one; `d` is
evaluated only when `x` is absent (ECMAScript's rule), so a failure of `d` is a failure only then.
`x` is evaluated once. The lexer stops refusing `??`; `?.` stays refused: for an optional record,
`x !== null ? x.value.f : d` says what `x?.f ?? d` would.

**Equality is total.** `x === y` of two values of `optional<T>` (or one of `T`) is true when both
are absent or both are present and their values are equal by `T`'s `===`; `null === null` is true
and an absent value equals no value of `T`. Ordering (`<`, `<=`, …), arithmetic, `len`, `+` on
text and a truth test of an `optional<bool>` are `expression/type-mismatch` on the optional
operand: what an absent value is less than, or what an absent text joined to another is, has no
answer the language would not have to invent. A test of a flag that may be absent is written
`flag === true`.

**Where values flow.**

- *Conditionals.* `c ? a : b` whose branches are of `T` and `optional<T>`, or `null` and `T`, is
  of `optional<T>`; two `null`s have no type unless the place gives one (and Go, which needs one,
  refuses it as it refuses any value of no type).
- *`<foreach>`.* A list holds no optional element; a record item's optional field is read as any
  optional value (`v.due !== null`), and is not written, as nothing a loop binds is.
- *`_event.data`.* A payload's optional field is read as an optional value
  (`_event.data.due`), and `<assign location="shown" expr="_event.data"/>` takes a payload whose
  fields are those of the record, each of the same type: an optional field of the record takes
  the payload's optional field, and is not filled from a field of `T` (the schemas are one
  document, as §2.15 already requires for a record).
- *Algorithms (`<sce:signature>`).* A parameter, a return and a local may be of `optional<T>`; an
  argument of `T` is lifted, `null` is admitted, and an algorithm that answers "none" says so in
  its return type (`<sce:return type="optional&lt;int64&gt;"/>`). A record whose schema has an
  optional field crosses an algorithm's call field by field as every record does (the adapters of
  "State an algorithm takes and gives back"), an optional enum field crossing by its variant's
  name when present.
- *Calls of other kinds (ADR 0048, ADR 0051).* A transform, condition, lookup, interpolation or
  validator takes and answers values, never an optional: a slot of `optional<T>` is refused where
  the callee is imported, and an argument that is optional is `expression/type-mismatch`, naming
  `.value` and `??`. A lookup whose miss answers `optional<T>` (the reading ADR 0048 turns into a
  failure) is not decided here; this type is what it would answer.
- *`<param>`, `<donedata>`, a typed request.* An optional value is carried as decision 3 writes it.
  A carrier that is text and not JSON — a BasicHTTP form, the `params` of a host-run `<invoke>`
  with no `sce:request` — has no spelling for absence, so an optional value there is refused at
  the `<param>` (`scxml/static-datamodel-rule`, naming `??`). A `<send>` to a field of `T` of an
  optional value is refused by the rule that holds a send to the field that reads it (§2.15); a
  value of `T` to a field of `optional<T>` is admitted.
- *A child session's `<param>`.* A child's variable of `optional<T>` is not handed a value yet:
  the generated `InvokeParams` already spell "not handed" as `None` / `null` /
  `std::optional`, and an optional variable would need a second absence inside the first. Refused
  by name at the `<param>` until a step decides the shape (decision 8, step 9).
- *`<log>`.* An absent value is logged as `null`, on every engine.

### 3. The wire

**On the event wire (`_event.data`), an absent value is JSON `null` and a present one is the
JSON `T` already writes** (a 64-bit integer as the text of its digits, an enum as its variant's
declared name). A missing key keeps the one meaning ADR 0044 gave it — the sender's revision does
not have the field — and `null` is the one spelling of absence. So:

| The delivery | A field of `optional<T>` of the first revision | One a later revision added (`sce:since`) |
|---|---|---|
| carries a value of `T` | present | present |
| carries `null` | absent | absent |
| lacks the key | refused, `error.execution`, nothing written (every first-revision field is carried) | its `sce:default`: absent when the field declares none, the value when it does |
| carries anything else | refused, as a value of another type is | refused |

A field of `T` that receives `null` is refused, as it is today. A writer always writes the key: an
absent value is `"due": null`, never a key left out, so a reader never confuses a sender that knows
the field with one that does not.

**`sce:default` on an optional field added by a later revision.** It may be left out, and then a
delivery that lacks the field reads absent: a sender that predates the field said nothing about
it, which is what absent means. Written, it is a value of `T` (`sce:default="0"`), never a spelling
of absence: `sce:default="null"` of an `optional<string>` is the four-letter text `null`, and of an
`optional<int64>` it is no integer and is refused as an unfit default is. The defaults object every
lift is handed (`forge::schema_revision::wire_of`) holds `null` for a field whose default is absent,
and every engine reads it with the reader it reads a delivered value with.

**The lifts** (`PayloadFields::decode_with_defaults`, `EventPayload.decode(data, defaults)`,
`sce.LiftPayloadWithDefaults`, `event_payload.lift(data, fields, defaults)`,
`EventPayloadFields::decode(data, defaults, out)`, `sce_payload_decode_with_defaults`, and the
Interpreter's `field(data, name, type, fallback)`) tell three cases apart for an optional field: a
value, `null`, and a missing key; the Interpreter's `field` tells "no default" (its fourth argument
`undefined`) from "the default is absent" (`null`).

**The saved state** holds an absent value as `null` and a present one as `T` writes it, in a
variable, a record's field and a list of records alike; `null` is added to the `value` of
`schemas/sce-saved-state.v1.schema.json` and `v2`, additively, with `format` unchanged. A Mesh
envelope carries `_event.data` as JSON with no schema identity (ADR 0044 decision 2), so it carries
`null` unchanged.

**The replay scenario vocabulary** writes absence as `null` in `data` and in an expected
`variables` value (a variable, or a record's field); `sce_build::scenario_replay` admits `null`
only where the machine's variable or field is optional, and every driver reads `null` against the
host type of decision 4.

**The forge codecs.** A CBOR or content-line codec keeps `sce:required` (decision 7), whose
generated host type is already the optional of decision 4 on five backends; its oracle already
writes an absent entry as `null`. A positional codec's field has no absence of its own: a layout
that may leave a field out says by what (`sce:present-if` over a flag or a chain), and a value of
`optional<T>` is refused there. An event schema's own unit (the payload type a host builds) holds
an optional field in the host type of decision 4, and the typed host API (`raise<Event>`) takes it
as that type.

### 4. Each backend's lowering

One host type per backend, the one its codecs' optional entries already use where they agree, and
one helper for each concern:

| | The type | Absent | Present | Is present | `x.value` | `x ?? d` | `===` |
|---|---|---|---|---|---|---|---|
| Rust | `Option<T>` | `None` | `Some(v)` | `is_some()` | `found(x)?` (a borrow or clone for a text) | `match x { Some(v) => v, None => d }` | `==` |
| Kotlin | `T?` | `null` | `v` | `!= null` | `SceChecked.found(x)` taken | `x ?: d` | `==` |
| C++ | `std::optional<T>` | `std::nullopt` | `v` | `has_value()` | `Checked::found(x)` taken | a runtime helper handed `d` as a lambda | `==` (C++17 defines it over `optional`) |
| Go | `*T` | `nil` | a pointer to a fresh copy | `!= nil` | `scealgorithm.Found(…)` over the pointer | a runtime helper handed `d` as a function literal | one runtime helper, by value |
| Python | `Optional[T]` | `None` | `v` | `is not None` | `found(x)` | a runtime helper handed `d` as a `lambda` | `==` |
| C11 | `sce_optional_<elem>_t`, `{ bool present; T value; }` | `SCE_OPTIONAL_NONE` | `SCE_OPTIONAL_SOME(v)` | `.present` | `<elem>_optional_take(&sce_failure_, x)` | decided in its step, under the rule below | `<elem>_optional_eq` |
| Interpreter | the JSON value or `null` | `null` | `v` | `!== null` | `SceStatic.found(x)`, which throws | `SceStatic.otherwise(x, function () { return d; })` | `===` |

Whatever the spelling, `x ?? d` evaluates `x` once and `d` only when `x` is absent, and a failure
of `d` is the statement's failure as any failure inside an expression is; the scenario of step 3
holds every backend to both (a `d` that would overflow, read beside a present `x`).

**What is shared.**

- *The read* is the runtime helper ADR 0047 named `found` "for what it reads, a value that may not
  have been found" — `sce_forge_runtime::algorithm::found`, `SceChecked.found`,
  `SCE::Forge::Checked::found`, `scealgorithm.Found`, `sce_algorithm.found` — so the failing read
  of an optional, of an enum's number and of a lookup's key is one function per runtime, with one
  failure (`out-of-range`). The Interpreter's library gains `SceStatic.found`, thrown as
  `SceStatic.variant` throws. A guarded read (decision 2) is the same expression without the
  helper.
- *The type's spelling* is one function of the generator per backend (`LangCtx::optional_of`),
  which the statechart, the algorithm, the schema unit and the codecs' optional and gated wraps
  all call. The four copies of the wrap in `forge/generator.rs` (a gated field, a gated repeat, a
  gated chain, a gated embed) and the spellings of the CBOR and content-line generators are folded
  into it where they agree, so an optional is spelled one way per backend.
- *The wire* is one reader and one writer of `null` per runtime, in the module that already reads
  and writes a delivered value, called by every lift, the saved state and the typed host API.
- *The C11 type* is declared by one macro of a new runtime header (`sce/forge/optional.h`) per
  element type — the struct, its two constructors, its `_take` and its `_eq` — beside the type it
  wraps: in the runtime for the scalars, beside `sce_static_string_<N>_t` for a text of bound `N`,
  under the enum document's guard for an enum (as `c11_enum_declaration` is), and beside the
  machine's record type for a record, so a program that includes a machine and an algorithm holding
  the same type declares it once.

**Go: a pointer, weighed against a struct with a presence member.** A generic `Optional[T]` struct
of two unexported members would copy by value, compare with `==`, never allocate and never alias.
A pointer is what Go's own encoders and the SCE codecs already spell an optional value with
(`nil` the one absence, no member that can disagree with another), and is what a Go host already
reads from every SCE codec. The pointer is chosen, and what the struct would have given is held by
rule: the machine never writes through a pointer it holds (it replaces it, as it replaces a slice),
a host is handed a copy, and equality is the runtime's helper over the values, never `==` over the
addresses.

**C11: the one representation that is a flag and a value.** C has no optional type, and a pointer
needs storage outside the value that a C machine without an allocator does not have. A tagged union
is the same two parts. So C11 holds `{ bool present; T value; }`, as C++'s `std::optional` does
under its interface, and closes what a flag beside a value would let through: the struct is one
value (an assignment, a parameter, a return carry both parts together), it is built only by the
two constructors (absent zeroes `value`, so two absent values are the same bytes), and generated
code reads `value` only through `_take`, `_eq` and the guarded read. The document, the wire and
the saved state never see the flag. The codecs' `<name>_present` members are left as they are: a
codec's struct is its own API, and a value of a codec's entry never flows as one value.

**The Interpreter** holds absence as `null` — what ECMAScript, the lowered document and JSON all
mean by it — and its library reads and writes it. `??` is lowered to a library call rather than left
to the engine, so the lowered document does not depend on which ECMAScript edition the engine reads.

**The bare-metal Rust profile** (ADR 0053) needs nothing: `Option` is `core`'s, an
`Option<heapless::String<N>>` is a bounded text, and the lift that reads a payload into a buffer of
its bound reads `null` where it reads a value.

### 5. Evolution

**A revision may widen a field from `T` to `optional<T>`, and that is the only change of type a
revision may make.** Every value of `T` is a value of `optional<T>` and is written the same on the
wire and in a saved state, so everything written before the widening reads after it. The field
declares the revision that widened it:

```xml
<data id="due" sce:type="optional&lt;int64&gt;" sce:direction="in" sce:optional-since="2"/>
```

- `sce:optional-since` is an integer past the revision the field was added at (1 for a field of
  the first revision, its `sce:since` otherwise) and at most the schema's `sce:revision`; it is
  written only on an `optional<T>` field, and only on an event schema (elsewhere
  `validation/sce-attribute-unread`). Out of range, it is `validation/attribute-rule-violated`. A
  field that was optional from the revision that added it declares none.
- At a revision before `sce:optional-since` the field is of `T`; from it on, of `optional<T>`.
- Narrowing (`optional<T>` to `T`) and every other change of type remain refused: a delivery or a
  saved state of absence would have nothing to be read as.

**The rule of ADR 0044 decision 1, amended.** Two schemas of one event are revisions of one schema
when (a) every field both declare is declared alike — the same type, bound, quantity, `since`,
default and `optional-since` — or differs only in that one declares `T` and the other
`optional<T>` with an `optional-since` past the first's `sce:revision`; and (b) is unchanged. The
rule stays symmetric. `forge::schema_revision::divergence` answers a widening the other side's
revision cannot hold, or two builds that disagree about which revision widened a field, as a
`Redeclared` field, with the two types and revisions in the reason of `mesh/event-schema-mismatch`.

**On the network.** A delivery from an older sender carries `T` and is read as present. A newer
sender's `null` reaching a receiver built before the widening is refused there, as a value its
field cannot hold (`error.execution`): the receiver's type has no member for it, as an older
receiver has none for a variant added after it (ADR 0044 decision 6). A deployment that widens a
field therefore upgrades the machines that receive it before the ones that send absence, and the
build names the machines that exchange the event.

**A log written at an older revision** is a sequence of deliveries, and is replayed through the
lift above: an entry written before the widening carries the field's value and reads present, an
entry written before the field existed reads its `sce:default` (absent when it declares none). The
language does not reinterpret what an entry holds: a number an application once wrote to mean
"none" is replayed as the day it is, and turning it into absence is the application's own
migration of its log.

**A saved state (ADR 0044 decision 3, ADR 0045), amended.** The shape names a record schema's
first-revision fields with their type at revision 1, so a schema that widened a first-revision
field keeps the shape it had, and a state saved before the widening restores. The digest of each
revision names each field with its type at that revision. A state saved at a revision before the
widening restores into the widened build with each record's value of the field present; one that
holds `null` in such a field is refused, since its revision does not admit absence (as a record
holding a field its revision does not declare is refused). The upgrade is in the one function each
runtime judges a saved state in (`saved_state::upgrade`, `SavedState.upgrade`). A shape line spells
an optional variable `variable due optional<int64> -`, and an optional field `field item.due
optional<int64>`, in both formats.

**A statechart variable is not a schema.** A variable re-typed from `T` to `optional<T>` is a
change of the statechart, which has no revisions, and its shape moves as it does for any re-typed
variable (ADR 0045: the restore is refused and the host keeps the text). A value that must survive
an upgrade of its type is kept in a record of a schema, which the rule above carries.

### 6. What changed since ADR 0052

ADR 0052 rejected "an optional reason" because a per-field `sce:optional` "would be a new type
carried on six backends and in the saved state, to say what one variant says". Three things changed:

- **A case a variant cannot serve.** `none` worked because an enum's variants are the author's to
  add. A day, a count, an amount have no value to spare: every `int64` is a day, so the only
  spellings of "no day" the language had were a sentinel number or a second field, both of which
  the owner's rule refuses.
- **It is a type, not a per-field attribute.** What ADR 0052 rejected was absence on a field. What
  is decided here is a type a variable, a field, a parameter, a return, a payload and a saved state
  all hold, so a value that may be absent flows from the wire to a variable to an algorithm without
  a second spelling at each step.
- **The cost ADR 0052 named is accepted** under the owner's rule.

By the rule this ADR applies, `record_refusal.none` is now a variant that stands for no refusal
rather than a refusal: the list never sends it, and a parent's variable starts at it because it had
no other start. A parent can now hold its last refusal in a variable of
`optional<record:Refused>`, starting at `null`. Whether `none` is retired while the list is a
pre-release part (ADR 0052 decision 3's condition still holds) is a question for the owner
(below).

### 7. The codecs keep `sce:required`, for now

A CBOR or content-line entry is optional unless it is `sce:required="true"`: the reverse of the
rule decided here, where a bare `T` is always present. Its host type is already this ADR's
(decision 4 folds the spellings into one function), so a value read from a codec and held by a
statechart has one type on each backend. Making codec entries say `optional<T>` and a bare `T`
required would flip the meaning of every codec entry written without `sce:required`, the standard
Mesh envelope among them; it is a decision of its own, asked of the owner below, and not taken
here. Until it is, `optional<…>` on a codec entry is refused naming `sce:required`.

### 8. How it lands

Each step lands alone, refuses by name what it does not yet lower (`generate/unsupported-feature`,
"an optional value has no <language> lowering yet", the pattern every model feature has landed
by), and carries the tests that hold it. Expected values in every scenario are derived from the
documents, not observed from a backend.

1. **The type in the model and the grammar.** `AlgorithmValueType::Optional`; a schema field typed
   by the value type; `read_algorithm_value_type` / `read_type_attr`; the `dataValueType` of
   `schemas/sce-forge-ext.xsd`; the element and nesting rules and the positions that refuse it;
   the forge-AST shape (`apis/forge-ast.v1.schema.json`, additive); every backend and the
   Interpreter's lowering refusing a document that holds one. `null` refused in every slot that is
   not optional, and `slot_admits` no longer admitting `Null`. Tests: a new
   `sce-build/tests/an_optional_value_is_declared_by_its_type.rs` — each admitted element on each
   position, each nesting refused, each other kind refused, the serialised shape, the per-backend
   refusal; and `null` assigned to an `int64` variable refused (a test that turns red if
   `slot_admits` admits `Null` again). Docs: §2.15 "Optional variables", the EventSchema kind.
2. **The expression surface.** `InferredType::Optional`; the lift of `T`; `=== null`; `.value`
   typed as a read through `found` (`CallReading::Found`); `??` (the lexer's refusal removed);
   conditionals; the guarded read; the refusals of ordering, arithmetic, `len`, text joins, truth
   tests, other kinds' slots, text carriers and child params; the enum judge holding an
   `optional<enum>`. Tests: `sce-build/tests/an_optional_value_is_tested_read_and_compared.rs`, one
   case per row of decision 2's table and per refusal, and the guarded read admitted in a
   `<data expr>` where the unguarded one is refused. No backend lowers yet.
3. **The shared scenario and the drivers' vocabulary.** `scenarios/static_optional.json` (a variable,
   a record's field, a payload's field and a list of records holding an optional `int64`, an
   `optional<string>` and an `optional<enum:…>`: set, cleared, tested, read, read absent with
   `error.execution` counted, `??`, equality, a delivery with a value, with `null`, without the key);
   `scenario_replay` admitting `null` where the machine holds an optional; each scanning driver
   listing the scenario as not yet lowered, with the reason, until its backend's step. Tests:
   `every_replay_scenario_keeps_to_the_vocabulary_its_drivers_replay` reading `null` in the new
   scenario and refusing it in a scenario whose variable is not optional.
4. **One backend at a time, in any order (seven steps, each its own branch):** Rust, Kotlin, Go,
   Python, C++, C11, the Interpreter. Each adds its runtime's `null` reader and writer and lift
   case, its spelling in `LangCtx::optional_of`, the lowering of every row of decision 2, the typed
   host API's optional arguments and readers, lifts its refusal, takes itself off the scenario's
   list, and replays `static_optional.json` (Rust `static_scenarios`, Kotlin `StaticScenarioTest`,
   Go `static_scenarios_test.go`, Python `test_static_scenarios.py`, C++
   `AStaticDatamodelRunsGeneratedCppTest`, the Interpreter
   `AStaticDatamodelRunsLoweredUnderTheInterpreterTest`, and for C11 a hand-written
   `backends/c/tests/integration/test_static_optional.c`, as the other C scenarios are). C11 adds
   `sce/forge/optional.h`, and the Go step the runtime's equality and copy helpers. The committed
   trees of its backend that its template change reaches are regenerated with it.
5. **The saved state (Rust, Kotlin).** `null` in the two saved-state schemas; the shape's spelling;
   the readers and writers; `saved/static_optional.json` and `saved/format1/static_optional.json`
   (one absent and one present value of each element), validated by
   `sce-build/tests/saved_state_schema.rs` and restored by each backend's saved-state tests.
6. **Widening across revisions.** `sce:optional-since` in the grammar, the forge roster of known
   names, the reader and the rendering; the amended `divergence` and its `Redeclared` reason; the
   per-revision type in the shape and the digests; the upgrade refusing `null` saved at an earlier
   revision; the absent default. Tests: `forge::schema_revision`'s unit tests (widened alike, widened
   at two revisions, narrowed, widened past the other's revision); a new
   `scenarios/static_optional_widened.json` on the seven engines (an entry of revision 1 read
   present, one of revision 2 read absent, one lacking a first-revision field refused, an added
   optional field with no default read absent); a revision-1 saved instance restored into the
   revision-2 build and one holding `null` refused; the Mesh check over two machines built from the
   two revisions. Docs: ADR 0044's amendments in docs/SCE_ACCEPTED_SUBSET.md "An event schema that
   grows" and "What a revision does not make safe".
7. **Algorithms.** Optional parameters, returns and locals; the adapters' copy of an optional field
   and of an optional enum crossing by name; `.value` in a body making it `may-fail` unless
   guarded. Tests: `scenarios/static_state_algorithm_optional.json` (an algorithm that answers the
   earliest due day of a list of records, absent for an empty list or one with none due) on every
   engine; the refusal of an unguarded `.value` in a body that does not declare `may-fail`.
8. **`optional<record:X>`.** A variable and an algorithm slot of an optional record: started at
   `null`, taken whole from a record, a payload, a list's element; `r.value.f`; a field write to an
   absent record failing as a read does. Tests: `scenarios/static_optional_record.json` on every
   engine, and its saved instance.
9. **The edges left refused by name**: a child session's optional variable as a `<param>` (with
   the shape of `InvokeParams` it needs), a host action's optional argument, a host-run `<invoke>`'s
   typed request and result with optional fields. Each with the scenario or test of its edge.
10. **The owner's answers**, each its own step: retiring `record_refusal.none` (ADR 0052) and
    moving `record_list_parent`, its scenario and its saved shape pin
    (`sce-build/tests/fixtures/standard_saved_shapes.json`); codec entries spelled by `optional<T>`.

Steps 1 and 2 come first and in that order; 3 after 2; the seven of step 4 after 3 and
independently of one another; 5 after the Rust and Kotlin steps of 4; 6 after 5; 7, 8 and 9 after
the backends they lower on.

## Alternatives considered

- **A sentinel value** (`-9223372036854775808`, `0`, a far-future day). It is a day: arithmetic on
  it answers a day, a comparison orders it among the days, and nothing in the type tells a reader
  to test it first. The owner's rule refuses it.
- **A companion `bool`** (`hasDue` beside `due`). Two fields that must agree, a `due` that holds a
  day while `hasDue` is false, and a wire, a saved state and a scenario that can carry the
  disagreement. The owner's rule refuses it. (The C11 representation is not this: decision 4.)
- **A sum type, an enum whose variants carry values.** It would express this and more, and it has
  no native carrier on Go, Python or C, where it would be a hand-built tagged structure, and none
  on the wire. Every backend and the wire have a native "absent or a value"; the general feature
  is not asked for. If it is ever added, this type is what it must agree with.
- **A date type.** It answers a different question: a date may be absent too, and a day count is
  already the `int64` an application computes with (`sce:std/time`).
- **A per-field `sce:optional="true"`** (ADR 0052's framing). Absence on a field only: a variable
  that copies the field, a parameter it is handed to, a return that answers it would each need the
  attribute again, which is a type spelled as an attribute in every place.
- **Absence as a missing key on the wire.** A missing key already means "the sender's revision
  does not have this field" (ADR 0044); a field added later with a present default would then read
  two meanings from one delivery.
- **Every type admitting `null`.** A value that may be absent everywhere is a test owed
  everywhere; a bare `T` stays present.
- **An implicit read** (`due + 1` failing when `due` is absent). ADR 0047's reason against an
  implicit conversion holds: one expression would mean a value in one run and a failure in
  another with nothing written that says so.
- **A read spelled as a builtin** (`value(x)`, `present(x)`). `value` is a name documents already
  give their variables and fields (the shared list's `record_value` among them); a presence
  builtin would be a second spelling of `x !== null`.
- **Narrowing across statements** (`<if cond="due !== null">` making `due.value` total inside).
  Sound only when nothing in between writes the variable, which an `<assign>` before the read can;
  the analysis is a feature of its own, and a statechart receives the failure of an unguarded
  read as `error.execution`. Left for a later decision.
- **A struct for Go, `Optional[T]`.** Weighed in decision 4: value semantics, at the cost of a
  second Go spelling of optional beside every codec's pointer.
- **A pointer for C11.** It needs storage elsewhere, which a C machine without an allocator does
  not have for a value it holds.
- **Admitting `optional<T>` in a codec entry beside `sce:required`.** Two spellings of one thing in
  one document kind; decision 7.

## Consequences

- A value that may be absent is declared as such, read only where the document says what absence
  means, and carried on the wire, in a saved state and in a scenario as `null`.
- An application widens a field of a released schema from `T` to `optional<T>` in a new revision
  and keeps its logs and its users' saved states; it upgrades the machines that receive the event
  before the ones that send absence.
- `null` is no longer admitted in a slot that is not optional, which a document that relied on it
  (if any does) finds out at build time.
- Every committed tree of a machine that holds an optional value is new; the trees of the codecs
  whose optional wrap is folded into `LangCtx::optional_of` must regenerate unchanged, which is
  what that step's regeneration shows.

## Questions for the owner

1. **C11's representation** (decision 4): a struct of a flag and a value, built and read only by
   the shared constructors and readers, is the one form without an allocator. Accepted?
2. **Go's representation** (decision 4): a pointer, as every Go codec already spells an optional
   entry, or a generic value struct, which would then be adopted by the codecs too?
3. **A first-revision optional field whose key a delivery lacks** (decision 3) is refused, so that
   a sender that forgot the field is told apart from one that wrote absence. Or should a missing key
   read absent for an optional field?
4. **`record_refusal.none`** (decision 6): retire it while the shared list is pre-release, so a
   parent holds `optional<record:Refused>` starting at `null`?
5. **Codec entries** (decision 7): move CBOR and content-line codecs to `optional<T>`, with a bare
   type required, through a release in which an entry that says neither is refused?
6. **A statechart variable widened from `T` to `optional<T>`** (decision 5) moves the shape and is
   refused on restore, as any re-typed variable is. Is that the rule, or should a statechart
   declare revisions of its own variables (a decision of its own)?
