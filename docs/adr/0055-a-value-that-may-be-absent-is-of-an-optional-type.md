# ADR 0055 — A value that may be absent is of an optional type, and absent is no value of any other

- Status: Accepted (2026-10-21). The owner decided every question the proposal left open ("The
  owner's answers", below), under the standing rule: the most long-term correct structure, cost
  ignored; never a sentinel number and never a companion flag that admits impossible states. Steps
  1 to 14 are implemented. Section 8 and step 12 amended the same day by that
  rule: a codec
  document declares its format, and format 1 is refused for good (step 12 is implemented).
- Date: 2026-10-21
- Amends: ADR 0044 decisions 1 and 3 (a revision may widen a field from `T` to `optional<T>`);
  ADR 0045 section 2 (a saved shape spells an optional value by the type its present value is, so
  a widening keeps the shape); docs/SCE_ACCEPTED_SUBSET.md "What a revision does not make safe"
- Supersedes: ADR 0052 decision 2 ("no refusal" is the variant `none`); see section 7
- Scope: the value-type grammar (`forge::model::AlgorithmValueType`, `read_algorithm_value_type`,
  `read_type_attr`, the `dataValueType` of `schemas/sce-forge-ext.xsd`), an event schema's fields,
  the CBOR and content-line codecs' entries; the expression layer (`forge::types::InferredType`,
  `forge::expr`'s lexer, typer, `slot_admits` and the six emitters), the enum judge
  (`forge::static_enum`), the statechart's scope and its judge (`forge::type_ctx::StaticScope`,
  `forge::static_datamodel`); `forge::schema_revision`; `forge::static_lowering` and its saved
  shape; the payload lift and wire writer of every generated backend and of the Interpreter's
  library (`forge/static_js/sce_static.js`); a new optional type in the Go and C11 forge runtimes;
  the saved state of the Rust and Kotlin backends and `schemas/sce-saved-state.v{1,2}.schema.json`;
  the replay scenario vocabulary (`sce_build::scenario_replay`) and its seven drivers;
  `stdlib/shared/record_refusal.scxml` and the parents of the shared list
- Related: ADR 0005 (`sce-static` closes its gaps by bounding and declaring), ADR 0044 (revisions),
  ADR 0045 (saved shapes), ADR 0047 ("a reading that answers none would need a type"), ADR 0048 and
  ADR 0051 (calls of other kinds), ADR 0050 (bounds), ADR 0052 (a refused command is named by an
  enum), ADR 0053 (the bare-metal Rust profile); SCE_FORGE.md §4.6.1 and the content-line codec

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
  `ExprKind::NullLit`, typed `InferredType::Null`, and `slot_admits` admitted `Null` in every slot
  as it admits `Unknown` (`forge/expr.rs`), so `<data sce:type="int64" expr="null"/>` was judged
  fit. The lexer refuses `??` and `?.` by name ("nullish coalescing", "optional chaining").
- **The codecs already have absence, spelled the other way round.** A CBOR or content-line entry
  that is not `sce:required="true"` is optional, generated as `Option<T>` (Rust), `T?` (Kotlin),
  `std::optional<T>` (C++), a pointer (Go), `Optional[T]` (Python) and a `<name>_present` member
  beside the value (C11); the conformance oracle spells an absent entry as JSON `null`. A positional
  field gated by `sce:present-if` is wrapped the same way, the wrap written out four times in
  `forge/generator.rs`.
- **A missing key already has a meaning on the event wire.** ADR 0044's lift reads a field a
  delivery lacks as an older sender's delivery: the field's `sce:default` when a later revision
  added it, and a refusal when the first revision declares it.
- **A changed type is not a revision** (ADR 0044 decision 1(a)), and a re-typed variable or field
  moves the saved shape (ADR 0045), so a restore of a state saved before it is refused.

What is to be decided: the type, its expressions, its wire, its lowering on the six generated
backends and the Interpreter, how a schema and a saved state whose value becomes optional evolve,
and what becomes of the two places that spell absence another way (the codecs' `sce:required`,
ADR 0052's `none`).

## Decision

### 1. The type: `optional<T>`

`sce:type="optional<T>"` (in XML `optional&lt;T&gt;`, as `list&lt;T&gt;` is written) is a value
that is either absent or a value of `T`. Absent is a value of `optional<T>` and of no other type.
A bare `T` is always present, wherever a type is written.

**What `T` may be.**

| `T` | Where | Why |
|---|---|---|
| a fixed-width number (`int8` … `uint64`, `float32`, `float64`), `bool` | everywhere the type is admitted | the case that has no spare value to give up |
| `string`, `bytes` | everywhere, with the bound `T` declares (`sce:capacity` on a variable, `sce:max-size` on a field) | an empty text is a text; absence is not an empty text |
| `enum:<alias>` of a closed enum | everywhere | a variant is a value; absence is not a variant (section 7) |
| `record:<alias>` | a statechart variable and an algorithm's parameter, return and local; not a field of an event schema | a schema has no record fields; a variable that holds "no record yet" is the case ADR 0052 met |

**Nesting.** Refused where the `sce:type` is written:

- `optional<optional<T>>` (`validation/invalid-attribute`, naming what may stand inside): Kotlin's
  `T??`, Python's `None`, ECMAScript's `null` and JSON's `null` collapse the two absences into
  one, so four of the carriers cannot hold it, and nothing asks for it.
- `optional<list<T>>` (the same): a list is a variable and never a value (§2.15, "List
  variables"), and its resting state, empty, is its own.
- `list<optional<T>>` (`validation/attribute-rule-violated`): a list holds fixed-width numbers,
  `bool`s or records, so that a bound in elements is a bound in bytes; a list of values that may be
  absent is a list of records with an optional field.
- `optional<T>` of an open enum: refused as the open enum itself is in a statechart.

**Where it is admitted.** A `sce-static` statechart's variable, a field of an event schema (and so
a record's field and a payload's field), an algorithm's parameter, return and local, and a CBOR or
content-line codec's entry (section 8). A positional codec's field, a transform's, lookup's,
interpolation's, condition's or validator's slot and a field that declares `sce:quantity` refuse
it (`validation/attribute-rule-violated`), because those positions are scalar by contract (the
reason `AlgorithmValueType` exists apart from `SceType`); a positional layout says by what a field
is left out (`sce:present-if`). A position whose step has not landed refuses it by the same code,
saying it does not hold one yet.

**In the model.** `optional` is a constructor of the value type, as `list` is:
`AlgorithmValueType` gains `Optional { elem }` over an element type of its own,
`OptionalElemType` (a scalar `SceType` or a record alias), serialised untagged as
`{"optional": "int64"}` and `{"optional": {"record": "Item"}}`, so the forge-AST of every
document that holds no optional value is byte-identical. An event schema's field, typed today by a
bare `SceType` it shares with every kind's `ForgeField`, is typed by the value type restricted to a
scalar or an optional scalar. The presence is never a boolean beside the scalar: every site that
reads a field's or a variable's type matches the constructor, so the compiler names each site that
has not said what it does with an optional value, as `ListElemType` keeps a record element from
being skipped. `InferredType` gains `Optional(OptionalElem)`, an element kept `Copy` as `ListElem`
is.

### 2. The expression surface

The forge expression language is a typed subset of ECMAScript, and it already spells absence the
way ECMAScript and JSON do. There is no read that can fail: a value is read out of an optional only
where the build has proven it present.

| What | Written | Type |
|---|---|---|
| absent | `null` | the `optional<T>` of the place it lands in |
| a present value | any value of `T` where an `optional<T>` is expected | `optional<T>` |
| is it present | `x !== null`, `x === null` | `bool` |
| the value, or a default | `x ?? d` (`d` of `T`, or of `optional<T>`) | `T` (or `optional<T>`) |
| the value, where the build has proven `x` present (section 3) | `x.value` | `T` |
| equality | `x === y`, `x !== y` | `bool` |

**`null` is typed by its place, and only an optional place takes it.** It stands where an
`optional<T>` is expected: an `<assign>` to, or `<data expr>` of, an optional variable, a
`<sce:set>` of an optional field, an argument of an optional parameter, an algorithm's optional
return, a branch of a conditional whose other branch is of `optional<T>` or of `T`, and an operand
of `===` / `!==` beside an optional value. Anywhere else it is `expression/type-mismatch`, naming
`null` as what stands only where an optional value is declared; `slot_admits` no longer admits
`Null` in a slot that declares a type (step 1). Every variable still declares its initial value:
an optional one starts at `null` or at a value, written in `expr` (or in its record's
`<sce:set>`), and nothing stands in.

**A value of `T` is a value of `optional<T>`.** Where an optional is expected, a value of `T` is
admitted by the rule that admits it where `T` is (an integer of any width checked into the width,
a text held to the bound), and is present. The other direction is never implicit: an optional
value where a `T` is expected is `expression/type-mismatch`, naming `??` and a test. ADR 0047
refused an implicit conversion of an enum to its number because "it would make `n = status` mean
the number in one place and a refusal in another"; an implicit read of an optional would make
`due + 1` succeed in one run and fail in another with nothing written that says it might.

**`x.value` is admitted only where `x` is proven present** (section 3); everywhere else it is
refused when the document is checked and generated, by a code of its own,
`expression/optional-read-unproven`, placed at the read, naming the optional and the two repairs:
a test that proves it (`x !== null`) around the read, or `x ?? d`. The record carries no `fix`,
since the repair is the author's choice between the two (SCE_ERROR_CONTRACT.md §3). So no
backend has a run-time path for reading an absent value: there is no failure to raise, no
`out-of-range` for an optional, and `found` is not involved. `.value` on a record holds the read:
`item.due.value`, `pick.value.title` for an `optional<record:Item>`; a record field named `value`
is still that field, since a member is judged by the type of what it is read from.

**`??` is the read with a default.** `x ?? d` is the value of a present `x` and `d` of an absent
one; `d` is evaluated only when `x` is absent (ECMAScript's rule), so a failure of `d` is a
failure only then. `x` is evaluated once. The lexer stops refusing `??`; `?.` stays refused: for
an optional record, `x !== null ? x.value.f : d` says what `x?.f ?? d` would.

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
- *`_event.data`.* A payload's optional field is read as an optional value (`_event.data.due`).
  `<assign location="shown" expr="_event.data"/>` takes a payload whose fields are those of the
  record, each of the same type: an optional field of the record takes the payload's optional
  field, and is not filled from a field of `T` (the schemas are one document, as §2.15 already
  requires for a record).
- *Algorithms (`<sce:signature>`).* A parameter, a return and a local may be of `optional<T>`; an
  argument of `T` is lifted, `null` is admitted, and an algorithm that answers "none" says so in
  its return type (`<sce:return type="optional&lt;int64&gt;"/>`). A `.value` in a body is held to
  section 3 as in a statechart, so it never makes an algorithm `may-fail`. A record whose schema has
  an optional field crosses an algorithm's call field by field as every record does (the adapters
  of "State an algorithm takes and gives back"), an optional enum field crossing by its variant's
  name when present.
- *Calls of other kinds (ADR 0048, ADR 0051).* A transform, condition, lookup, interpolation or
  validator takes and answers values, never an optional: a slot of `optional<T>` is refused where
  the callee is imported, and an argument that is optional is `expression/type-mismatch`, naming a
  test and `??`. A lookup whose miss answers `optional<T>` (the reading ADR 0048 turns into a
  failure) is not decided here; this type is what it would answer.
- *A host action's argument (`<sce:arg>`).* An argument whose expression is of `optional<T>` over
  a number, a `bool`, a text or a byte string is the host method's parameter in the backend's own
  optional type over `T` (step 10); the Interpreter's host is handed `null` for an absent one. The
  method's signature names the type, `optional<int32>` against `int32`, so two calls of one name
  that disagree on it are a signature conflict and nothing is coerced; `null` as an argument has no
  type, and a value of an enum or a record is no host argument, present or absent. C11's text and
  byte string stay refused by name: its struct is declared over a buffer of a bound, which a host
  method's parameter does not declare.
- *`<param>`, `<donedata>`, a typed request and result.* An optional value is carried as section 4
  writes it. A carrier that is text and not JSON — a BasicHTTP form, the `params` of a host-run
  `<invoke>` — has no spelling for absence, so an optional value there is refused at the `<param>`
  (`scxml/static-datamodel-rule`, naming `??`). A typed request is those `params` read back as a
  record, so it is that carrier: a `sce:request` schema with a field that may be absent is refused
  where the invoke is checked (`validation/typed-invoke-schema`, naming `??`), and a typed result,
  which the host completes with the record's JSON, holds an optional field and writes an absent
  one as `null` (step 10 of section 9). A `<send>` of an optional value to a
  field of `T` is refused by the rule that holds a send to the field that reads it (§2.15); a value
  of `T` to a field of `optional<T>` is admitted.
- *A child session's `<param>`.* A child's variable of `optional<T>` over a bool, a number or a
  text is handed the whole optional value its `<param>` computes (step 10): a value of `T` is
  present, `null` is absent, and an optional variable is handed as it is. `InvokeParams` already
  spells "not handed" as `None` / `null` / `std::optional`, and a variable not handed keeps its
  start, so an absent seed is a value not handed: the slot of an optional variable is the
  backend's optional type itself and holds no second absence, which Kotlin's `T??`, Python's
  `None` and ECMAScript's `null` could not. What that needs of the child is that absent and "not
  handed" are one state, which they are where the variable starts at `null`; where it starts at a
  value, the `<param>` is refused unless its value is never absent. A child's optional enum, byte
  string or record is refused by name, as an enum, a byte string and a record are.
- *`<log>`.* An absent value is logged as `null`, on every engine.

### 3. Where a value is proven present: narrowing

A **subject** is a path the build can follow: a variable; a field of a record variable; a field of
the payload (`_event.data.f`); a field of a `<foreach>`'s record item; an algorithm's parameter or
local, or a field of one that is a record; and any of those followed by `.value.f` through an
optional record already proven present. An element read by its index (`xs[i].f`) and the value of
a call are not subjects: a call answers anew each time, and an index is a value that may change
between the test and the read. A subject is proven present by a **test** on it, `s !== null` or
`null !== s` (proving it where the test is true) and `s === null` or `null === s` (where it is
false), and by nothing else.

**Which constructs narrow.**

- *Within one expression.* The right operand of `a && b` is read where `a` is true, the right
  operand of `a || b` where `a` is false, and the branches of `c ? t : e` where `c` is true and
  false; `!c` exchanges the two, and parentheses change nothing. What `a && b` proves where it is
  true is what either side proves; what `a || b` proves where it is false is what either side
  proves false. An expression writes nothing, so nothing inside one invalidates a fact.
- *`<if>`, `<elseif>`, `<else>` in a statechart's executable content.* The block of an `<if>` is
  read where its `cond` is true. Each `<elseif>`'s `cond` is read where every earlier `cond` of the
  chain is false, and its block where those are false and its own is true. The `<else>` block is
  read where every `cond` of the chain is false. Nothing a chain proves reaches past its `</if>`.
- *An algorithm's `<sce:if>` and `<sce:else>`*, by the same rule, and `<sce:while>`, whose `cond`
  proves its facts at the start of its body (re-tested on every turn).
- *A guard or an early return in an algorithm.* After `<sce:require cond="…"/>`, the statements
  that follow it in the same block are read where its `cond` is true (the algorithm has failed
  otherwise). After an `<sce:if>` with no `<sce:else>` whose block ends in `<sce:return>` on every
  path, the statements that follow it in the same block are read where its `cond` is false. The
  statechart's executable content has no return and no guard, so nothing else narrows there.
- *A transition's `cond`*, for the subjects of the payload only: a test of `_event.data.f` in the
  `cond` proves it in that transition's executable content, because the event's data is the same
  value from the `cond` to the end of the transition. A test of a variable in the `cond` proves
  nothing in the content: between the two the microstep runs the exit actions of the states it
  leaves and the content of every other transition it takes in document order, and any of those
  may write the variable. A test in a state's `<onentry>` or `<onexit>` proves nothing in another
  block.

**What invalidates a fact.** A fact about a subject ends at the first statement after it, on the
path from the test to the read, that may write the subject or anything that holds it:

- an `<assign>` (or `<sce:assign>`) whose location is the subject, a path below it, or a path
  above it (`r` for `r.f`, `r.value` for `r.value.f`), whatever value it writes;
- an assignment of a call's result to the variable that holds it (a record or a list an algorithm
  hands back whole);
- a native `<script>` block, which may write anything, and ends every fact;
- a `<send idlocation>` or an `<invoke idlocation>` naming the variable.

An append to or a clear of the list a loop's item is drawn from does not end a fact about the
item, which is a copy taken when the turn began. Re-assigning a present value does not re-establish
a fact: only a test proves presence.

**Loops.** A fact proven before a `<foreach>` (or an `<sce:while>`, or an algorithm's
`<sce:foreach>`) holds inside its body only if nothing in the body invalidates it, since the second
turn of the body follows the end of the first. A test inside the body proves its facts for the rest
of that turn, by the rules above. A `<foreach>`'s item and index are new on every turn and are
written by nothing in the body, so a fact about the item's field holds to the end of the turn.

**Where it is implemented.** One pass over the judged tree (`forge::static_datamodel` for a
statechart, the algorithm judge for a body) carries the set of proven subjects into each
expression it judges, and the expression typer reads `.value` as proven or refuses it. The same
set reaches the lowering, which writes a proven read without any failure plumbing (section 5).

### 4. The wire

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
host type of section 5.

**The codecs.** A CBOR codec writes an absent entry as a key left out of its map and a
content-line codec as a property left out of its component: in an encoding that has no revisions,
a missing key has no other meaning to keep, and both oracles already spell it so. Their host type
is section 5's. A positional codec has no absence of its own (section 1).

### 5. Each backend's lowering

One host type per backend, the one every SCE surface of that backend spells an optional value
with — the statechart, the algorithm, the schema unit, the typed host API and the codecs — and one
helper for each concern:

| | The type | Absent | Present | Is present | proven `x.value` | `x ?? d` | `===` |
|---|---|---|---|---|---|---|---|
| Rust | `Option<T>` | `None` | `Some(v)` | `is_some()` | `sce_forge_runtime::optional::proven(x)` | `match x { Some(v) => v, None => d }` | `==` |
| Kotlin | `T?` | `null` | `v` | `!= null` | `SceOptional.proven(x)` | `x ?: d` | `==` |
| C++ | `std::optional<T>` | `std::nullopt` | `v` | `has_value()` | `*x` | a runtime helper handed `d` as a lambda | `==` (C++17 defines it over `optional`) |
| Go | `optional.Of[T]` | `optional.None[T]()` | `optional.Some(v)` | `x.IsPresent()` | `x.Proven()` | a runtime helper handed `d` as a function literal | `x.Equal(y)` |
| Python | `Optional[T]` | `None` | `v` | `is not None` | `x` | a runtime helper handed `d` as a `lambda` | `==` |
| C11 | `sce_optional_<elem>_t`, `{ bool present; T value; }` | `sce_optional_<elem>_none()` | `sce_optional_<elem>_some(v)` | `.present` | `.value` | `SCE_OPTIONAL_VALUE_OR(x, d)`, a conditional over the flag (step 5) | `SCE_OPTIONAL_SAME`, `sce_optional_<elem>_eq` |
| Interpreter | the JSON value or `null` | `null` | `v` | `!== null` | `x` | `SceStatic.otherwise(x, function () { return d; })` | `===` |

Whatever the spelling, `x ?? d` evaluates `x` once and `d` only when `x` is absent, and a failure
of `d` is the statement's failure as any failure inside an expression is; the scenario of step 4
holds every backend to both (a `d` that would overflow, read beside a present `x`).

**A proven read has no failure path.** The build proved the value present, so each backend reads
it directly. Rust reads it through one runtime function, `proven`, whose absent arm is
`unreachable!`: an arm the build has proven no run reaches, written in one place so that no
backend's lint reads the test and the read as a pattern to rewrite. Kotlin reads it through
`SceOptional.proven` of its forge runtime for the same reason: a test of a stable value (a field of
a loop's item) smart-casts it to `T`, and `!!` on a `T` is a warning its build refuses. C++'s `*`,
Go's `Proven()` and C11's `.value` are the same direct reads. The narrowing rules of section 3 are what
make this sound, and the tests of step 3 hold each of them, an invalidating statement for each.

**What is shared.**

- *The type's spelling* is one function of the generator per backend (`LangCtx::optional_of`),
  which the statechart, the algorithm, the schema unit and the codecs' optional and gated wraps
  all call. The four copies of the wrap in `forge/generator.rs` (a gated field, a gated repeat, a
  gated chain, a gated embed) and the spellings of the CBOR and content-line generators are folded
  into it, so an optional is spelled one way per backend.
- *The wire* is one reader and one writer of `null` per runtime, in the module that already reads
  and writes a delivered value, called by every lift, the saved state and the typed host API.
- *The Go type* is `Of[T]` of a new package of the Go forge runtime,
  `github.com/newmassrael/sce-forge-runtime/optional`, beside `algorithm` and `codec`: a struct of
  two unexported members, built only by `Some` and `None`, read by `Get() (T, bool)`,
  `IsPresent()` and `Proven()`, compared by `Equal` (by value, and through the element's own
  equality for a byte string). It copies by value, never allocates and never aliases, and its zero
  value is absent. The owner chose it over a pointer (answer 2), and the Go codecs move to it
  (step 13), so Go has one optional representation.
- *The C11 type* is declared by one macro of a new runtime header (`sce/forge/optional.h`) per
  element type — the struct, its two constructors and its `_eq` — beside the type it wraps: in the
  runtime for the scalars, beside `sce_static_string_<N>_t` for a text of bound `N`, under the enum
  document's guard for an enum (as `c11_enum_declaration` is), and beside the machine's record type
  for a record, so a program that includes a machine and an algorithm holding the same type
  declares it once.

**C11: the one representation that is a flag and a value** (accepted by the owner, answer 1). C has
no optional type, and a pointer needs storage outside the value that a C machine without an
allocator does not have; a tagged union is the same two parts. So C11 holds
`{ bool present; T value; }`, as C++'s `std::optional` does under its interface, and closes what a
flag beside a value would let through: the struct is one value (an assignment, a parameter, a
return carry both parts together), it is built only by the two constructors (absent zeroes
`value`, so two absent values are the same bytes), and generated code reads `value` only through
`_eq` and a proven read. The document, the wire and the saved state never see the flag. The codecs'
`<name>_present` members become this type with the codec migration (step 14).

**The Interpreter** holds absence as `null` — what ECMAScript, the lowered document and JSON all
mean by it — and its library reads and writes it. `??` is lowered to a library call rather than left
to the engine, so the lowered document does not depend on which ECMAScript edition the engine reads.

**The bare-metal Rust profile** (ADR 0053) needs nothing: `Option` is `core`'s, an
`Option<heapless::String<N>>` is a bounded text, and the lift that reads a payload into a buffer of
its bound reads `null` where it reads a value.

### 6. Evolution

**A widening from `T` to `optional<T>` is the one change of type that keeps what was written
readable.** Every value of `T` is a value of `optional<T>` and is written the same on the wire and in
a saved state, so everything written before the widening reads after it, as present.

**In a saved state, a widening keeps the shape** (the owner's answer 6; amends ADR 0045 section 2).
The shape names each value by its **shape type**, the type a present value of it is read as: the
shape type of `optional<T>` is the shape type of `T`, and of every other type the type itself. So a
statechart variable widened from `int64` to `optional<int64>` keeps its line, `variable due int64
-`, a record's field widened the same way keeps `field item.due int64`, an `optional<enum:A>`
spells its variants as `enum:A` does, and the shape (format 1 and format 2 alike) is the digest it
was. Every other change of type still moves the shape and refuses the restore, as before.

What the shape no longer says, the restore asks of each value, by the restoring build's own types:

- A `null` is read only where the restoring build's type is optional. A statechart variable or a
  record's field of `T` that holds `null` is refused, naming the variable or the field, and the
  restore builds nothing (ADR 0045's rule that a refusal anywhere leaves everything as it was).
- A state saved before a widening restores into the widened build, each value present.
- A state saved after a widening restores into a build from before it when every value it holds
  is present, since each is then a value of `T`, and is refused at the first `null`. Nothing a
  state holds is dropped either way, which is what the shape exists to guarantee; a state that
  could not be read whole is refused.
- A record's field of a schema that grew is read by the type the field has at the revision the
  state names for the schema (ADR 0044 `revisions`): a `null` saved at a revision before the field
  was widened (`sce:optional-since`, below) is refused, as a field a revision does not declare is.

The digest of a revision in `revisions` names each field by its shape type too, so a revision that
only widens a field writes the digest the revision before it wrote, and the two builds agree on a
state saved at either; what a revision widened is the restoring build's schema to say, and it says
it with `sce:optional-since`.

**On the network, a widening is a revision.** Two machines that exchange an event read one
another's deliveries as they come, so the schema says which revision widened a field:

```xml
<data id="due" sce:type="optional&lt;int64&gt;" sce:direction="in" sce:optional-since="2"/>
```

- `sce:optional-since` is an integer past the revision the field was added at (1 for a field of
  the first revision, its `sce:since` otherwise) and at most the schema's `sce:revision`; it is
  written only on an `optional<T>` field, and only on an event schema (elsewhere
  `validation/sce-attribute-unread`). Out of range, it is `validation/attribute-rule-violated`. A
  field that was optional from the revision that added it declares none.
- At a revision before `sce:optional-since` the field is of `T`; from it on, of `optional<T>`.
- ADR 0044 decision 1, amended: two schemas of one event are revisions of one schema when (a) every
  field both declare is declared alike — the same type, bound, quantity, `since`, default and
  `optional-since` — or differs only in that one declares `T` and the other `optional<T>` with an
  `optional-since` past the first's `sce:revision`; and (b) is unchanged. The rule stays symmetric.
  `forge::schema_revision::divergence` answers a widening the other side's revision cannot hold, or
  two builds that disagree about which revision widened a field, as a `Redeclared` field, with the
  two types and revisions in the reason of `mesh/event-schema-mismatch`.
- A delivery from an older sender carries `T` and is read as present. A newer sender's `null`
  reaching a receiver built before the widening is refused there, as a value its field cannot hold
  (`error.execution`), as an older receiver refuses a variant added after it (ADR 0044 decision
  6). A deployment that widens a field upgrades the machines that receive it before the ones that
  send absence, and the build names the machines that exchange the event.
- Narrowing (`optional<T>` to `T`) and every other change of type remain refused on the network.

**A log written at an older revision** is a sequence of deliveries, and is replayed through the
lift of section 4: an entry written before the widening carries the field's value and reads
present, an entry written before the field existed reads its `sce:default` (absent when it declares
none). The language does not reinterpret what an entry holds: a number an application once wrote
to mean "none" is replayed as the day it is, and turning it into absence is the application's own
migration of its log.

### 7. "No refusal" is an absent record, and `none` is retired (supersedes ADR 0052 decision 2)

ADR 0052 rejected "an optional reason" because a per-field `sce:optional` "would be a new type
carried on six backends and in the saved state, to say what one variant says". Three things
changed:

- **A case a variant cannot serve.** `none` worked because an enum's variants are the author's to
  add. A day, a count, an amount have no value to spare: every `int64` is a day, so the only
  spellings of "no day" the language had were a sentinel number or a second field, both of which
  the owner's rule refuses.
- **It is a type, not a per-field attribute.** What ADR 0052 rejected was absence on a field. What
  is decided here is a type a variable, a field, a parameter, a return, a payload and a saved state
  all hold.
- **The cost ADR 0052 named is accepted** under the owner's rule.

By the same rule, `record_refusal.none` is a variant that stands for no refusal rather than for a
refusal — the list never sends it — and it is retired while the shared list is a pre-release part
(the owner's answer 4; ADR 0052 decision 3's condition still holds: no saved state of the list or a
refusal exists outside this repository's tests). A parent keeps its last refusal in a variable of
**`optional<record:Refused>`**, starting at `null`, and not in a record whose `reason` is
`optional<enum:Refusal>`:

- A refusal is the whole record — the row, the field, the command and the reason — and the record is
  what is absent before the first one. An optional reason inside a present record would leave the
  record's `id`, `field` and `kind` holding a row, a field and a command of a refusal that never
  happened, the impossible state ADR 0052 already named for `kind` ("the command beside `none` is
  the variable's starting value, `edit`, with no meaning").
- The schema `record_refused` stays a record of four present fields: every refusal the list sends
  has all four, so no field of it is optional.

`record_refusal` loses `none` and starts at `notWriter` = 1, its numbers unchanged; the change is
to the enum's first revision (removing a variant is not a revision, ADR 0044), which moves the saved
shape of every machine that holds the enum, the list's own among them, and its pin in
`sce-build/tests/fixtures/standard_saved_shapes.json` moves with it. The step lands after
`optional<record:X>` is lowered on every backend (step 11).

### 8. The codecs say optional the way everything else does

A CBOR or content-line entry is optional unless it is `sce:required="true"`: the reverse of the
rule decided here, where a bare `T` is always present. The codecs move to this rule (the owner's
answer 5): a bare `T` entry is required, an `optional<T>` entry is optional, and `sce:required` is
no longer part of the grammar, so the language has one way to say a value may be absent. For a
content-line parameter of a repeated property, `optional<T>` is optional of each line that exists,
as a parameter that is not `sce:required` is today.

**A codec document declares its format, and the format is permanent** (decided by the owner's
standing rule on 2026-10-21: no document is ever read with a meaning other than the one it was
written with). A document written in the earlier spelling cannot be told from one in the new
spelling by its entries alone: a codec whose entries are all bare types is a codec of optional
entries in the one and of required entries in the other. So the spelling a document is written in
is said on its root, beside `sce:encoding`:

```xml
<scxml ... sce:kind="codec" sce:encoding="cbor" sce:codec-format="2">
```

- **Format 2 is the only one read and generated.** A bare `T` entry is required, an `optional<T>`
  entry is optional, and `sce:required` is not in its grammar: written in a format-2 document it is
  refused (`validation/attribute-rule-violated`, naming `optional<T>` and the bare type as the two
  spellings of what it meant).
- **A document without `sce:codec-format`, or with `sce:codec-format="1"`, is format 1, and is
  refused, never read.** The refusal is a code of its own, `validation/codec-format-superseded`
  (in the appendix of docs/SCE_ACCEPTED_SUBSET.md and SCE_ERROR_CONTRACT.md with step 12), placed
  on the root, and its repair is the document's own rewrite, which the tool computes from what
  format 1 says: each entry with `sce:required="true"` becomes its bare type with the attribute
  removed, each entry without it (or with `="false"`) becomes `optional<T>`, and the root gains
  `sce:codec-format="2"`. The record carries the format-2 spelling of every entry, and
  `sce-codegen codec-format <document>` prints the whole document in format 2, so the author
  applies the rewrite rather than retyping it. Any other value of the attribute is
  `validation/invalid-attribute`, naming `2`.
- **It stays.** The refusal of format 1 is part of the grammar, not a migration aid: a document
  written before this decision and built for the first time years after it is refused and told its
  rewrite, exactly as one built the day after, and is never reinterpreted. There is no step that
  ends a transition, and no release note carries a rule the build does not.
- **Which documents declare it.** The CBOR and content-line codecs (`sce:encoding="cbor"`,
  `"content-line"`), the two whose entries may be absent. A positional codec's field is never
  absent by a declaration of its own (it is left out by `sce:present-if`, which no format changes),
  so no reading of a positional document changes and it declares no format; written there, the
  attribute is refused as `validation/sce-attribute-unread`. A positional codec that ever gains an
  entry that may be absent gains the attribute in the same step, by the same rule.

**The migration.** One step moves every CBOR and content-line codec document of the repository
(the standard Mesh envelope among them) to format 2 by that rewrite, together with the grammar, the
generators and the conformance cases.

**One representation.** The Go codecs' pointer fields and the C11 codecs' `<name>_present` members
become section 5's types, Go's in step 13 and C11's in step 14, each after the move to format 2, so a value read
from a codec and held by a statechart has one type on each backend.

### 9. How it lands

Each step lands alone, refuses by name what it does not yet lower (`generate/unsupported-feature`,
"an optional value has no <language> lowering yet", the pattern every model feature has landed
by), and carries the tests that hold it. Expected values in every scenario are derived from the
documents, not observed from a backend.

1. **The type in the model and the grammar** (implemented with this ADR).
   `AlgorithmValueType::Optional` over `OptionalElemType`; the statechart variable's reader
   (`read_static_value_type`) with the element and nesting rules; every other position refusing
   `optional<…>` by name (`read_type_attr`), a list of optional elements refused in
   `read_algorithm_value_type`; an optional `string` or `bytes` variable bounded by
   `sce:capacity`; the `optionalValueType` of `schemas/sce-forge-ext.xsd`, which, as the list
   pattern admits `list<string>` for the parser to refuse by its rule, admits lexically every type
   composed of well-formed spellings (`optional<optional<E>>`, `optional<list<E>>`,
   `list<optional<E>>`, an algorithm's `optional<T>`), so each reaches the parser's refusal that
   names the rule, and only a misspelled keyword is refused by the schema; the forge-AST shape (`apis/forge-ast.v1.schema.json`); every
   backend and the Interpreter refusing a document that holds one
   (`StaticTarget::lowers_optional`, false on every target); and `null` refused in a slot that
   declares a type. Test: `sce-build/tests/an_optional_value_is_declared_by_its_type.rs`.
   Meanwhile the statechart's judge types an optional variable as a value it does not yet know
   (`InferredType::Unknown`), as it types a variable with no type: nothing about one is judged
   until step 3, and no backend generates one.
2. **An event schema's field typed by the value type** (implemented). The schema field's type moves
   from `SceType` to the value type (a scalar or an optional scalar) at every reader, which the
   compiler enumerates; `optional<T>` is read on a schema field and refused by name at generation;
   the lift still refuses such a schema on every backend. Test: the schema-field cases added to the
   step-1 test file, and the forge-AST of a schema with an optional field. As built, the field is
   `ForgeField<FieldType>` (`SchemaField`) while every other kind's stays `ForgeField<SceType>`, so
   only a reader of a schema's field had to say what it does with an optional one; a lowering
   that refuses one before it reads the schema reads each field's `present_type`, which the
   backend step that lowers the type revisits. An algorithm's record of such a schema is judged
   and then refused where each backend renders the algorithm.
3. **The expression surface and narrowing** (implemented). `InferredType::Optional`; the lift of `T`; `=== null`;
   `??` (the lexer's refusal removed); conditionals; section 3's narrowing over expressions,
   `<if>` chains, transitions and algorithm bodies, and its invalidation; `.value` proven or refused
   with `expression/optional-read-unproven` (in the appendix of docs/SCE_ACCEPTED_SUBSET.md and
   SCE_ERROR_CONTRACT.md); the refusals of ordering, arithmetic, `len`, text joins, truth tests,
   other kinds' slots, text carriers and child params; `null` beside a non-optional operand of
   `===`; the enum judge holding an `optional<enum>`. Tests:
   `sce-build/tests/an_optional_value_is_tested_read_and_compared.rs`, one case per row of section
   2's table and per refusal, and `sce-build/tests/a_value_is_proven_present_only_where_tested.rs`,
   one case per narrowing construct and one per invalidating statement (each read refused after
   it), a transition whose `cond` tests a payload field (proven) and a variable (not), and a loop
   whose body assigns the subject. As built, the narrowing is one module, `forge::narrowing`:
   what a test proves (`proves`), an `<if>` chain (`Chain`), a loop's entry (`loop_entry`), an
   algorithm's sequencing (`after_algorithm_statement`) and what ends a fact (`Facts::after` over
   the `Writes` of a statement, one `Statement` implementation for executable content and one for
   an algorithm's body). The statechart's judge carries the facts on `TypeCtx::proven` into each
   expression it judges; an algorithm's body is walked by `narrowing::check_algorithm` before any
   backend renders it, judging only the rules of an optional value, since the renderer that would
   judge the rest refuses the body by name until step 8. An optional enum is registered with the
   enum judge at the path of its present value (`status.value`), so `status` is no value of the
   enum and `status.value` is. An `<invoke>`'s `idlocation` is written when the invocation starts,
   after every block of the macrostep, so it ends no fact a block holds; a `<send idlocation>` may
   name an `optional<string>`.
4. **The shared scenario and the drivers' vocabulary** (implemented). `scenarios/static_optional.json` (a variable,
   a record's field, a payload's field and a list of records holding an optional `int64`, an
   `optional<string>` and an `optional<enum:…>`: set, cleared, tested, read where proven, `??` with
   a `d` that would fail only when evaluated, equality, a delivery with a value, with `null`, and
   without the key); `scenario_replay` admitting `null` where the machine holds an optional; each
   scanning driver listing the scenario as not yet lowered, with the reason, until its backend's
   step. Tests: `every_replay_scenario_keeps_to_the_vocabulary_its_drivers_replay` reading `null` in
   the new scenario and refusing it in a scenario whose variable is not optional. As built, the
   machine is `static_optional.scxml` over two schemas, `schema_due.scxml` (the record and the
   payload of `due.set`) and `schema_due_sent.scxml` (what the machine sends itself); the scenario
   adds what section 2 and section 3 read in a statechart (each narrowing construct of executable
   content, a `<log>`, a `<send>` and a `<donedata>` of optional values). `null` is held by
   `scenario_replay::machine_problems`, which reads what each place holds from the machine
   (`Holds`: a variable's type, a record's and a list's record's fields, the payload of an event
   the machine imports a schema for, the pairs of a top-level `<final>`'s `<donedata>` typed as the
   judge types them) and refuses a `null` anywhere else as `null-not-optional`. "Not yet lowered"
   is listed in three places: `NOT_YET_LOWERED` in
   `sce-build/tests/a_static_datamodel_holds_every_variable_to_a_type.rs`, an engine to a line,
   which admits a scenario no engine lowers only while every engine says why and fails for a line
   whose engine lowers the machine; `NOT_YET_LOWERED` of the Rust scenario table
   (`backends/rust/scenarios/src/machines.rs`, read by its wasm32 hosts too); and an expectation
   of the Interpreter's scan (`AStaticDatamodelRunsLoweredUnderTheInterpreterTest`). The drivers
   that name each scenario they replay (Rust `static_scenarios`, Kotlin, Go, Python, C++, C11)
   list nothing: the test above already holds them to replay what their backend lowers. Tests:
   `the_optional_scenario_states_null_only_where_its_machine_declares_one` and
   `null_is_refused_where_the_machine_declares_no_optional_value` in
   `sce-build/tests/every_replay_scenario_keeps_to_its_vocabulary.rs`, and
   `null_stands_only_where_the_machine_declares_an_optional_value` in `scenario_replay`.
5. **One backend at a time, in any order (seven steps, each its own branch):** Rust (with
   `sce_forge_runtime::optional::proven`), Kotlin, Go (with the `optional` package and `Of[T]`),
   Python, C++, C11 (with `sce/forge/optional.h`), the Interpreter (with `SceStatic.otherwise`).
   Each adds its runtime's `null` reader and writer and lift case, its spelling in
   `LangCtx::optional_of`, the lowering of every row of section 2, the typed host API's optional
   arguments and readers, sets `lowers_optional`, takes itself off the scenario's list, and replays
   `static_optional.json` (Rust `static_scenarios`, Kotlin `StaticScenarioTest`, Go
   `static_scenarios_test.go`, Python `test_static_scenarios.py`, C++
   scenario suite `tests/integration/static_scenarios/`, the Interpreter
   `AStaticDatamodelRunsLoweredUnderTheInterpreterTest`, and for C11 a hand-written
   `backends/c/tests/integration/test_static_optional.c`, as the other C scenarios are). The
   committed trees of its backend that its template change reaches are regenerated with it.
   As built for C11, `sce/forge/optional.h` declares each element's type, its two
   constructors and its `_eq` through one macro, `SCE_OPTIONAL_DECLARE(elem, T, same)`: the
   numbers and `bool` in the header (`sce_optional_int64_t`), a text and a byte string of
   bound N beside their buffers (`SCE_OPTIONAL_DECLARE_STRING(N)`, `sce_optional_string_<N>_t`;
   `_BYTES(N)`), an enum under its document's guard (`sce_optional_enum_<document>_t`). An
   absent value is every byte zero. `x ?? d` is `SCE_OPTIONAL_VALUE_OR(x, d)`, the
   conditional `(x).present ? (x).value : (d)` (`SCE_OPTIONAL_TEXT_OR` for a text,
   `SCE_OPTIONAL_EITHER` where `d` may be absent too): `d` is evaluated only when `x` is
   absent, and `x` is read twice, for its flag and its value, since C11 has no portable way
   to name a value inside an expression (a statement expression is a GNU extension) and a
   forge expression writes nothing, so the two reads are one value. On the wire an absent
   value is `sce_forge_wire_null()`, and the lift tells a value, `null` and a missing key
   apart through `sce_payload_read_presence`. Equality is `SCE_OPTIONAL_SAME(a, b, same)`
   of two optional values and `SCE_OPTIONAL_HOLDS(x, v)` of one and a present value, so an
   expression names no type; `null` and a lifted value inside an expression are spelled
   only for a number or a `bool`, whose type the element names, and refused by name for a
   text, a byte string or an enum, whose type carries a bound or a document (written whole
   where a variable or a field takes them, they are the walk's). A payload's optional text
   is the buffer of its `sce:max-size`, as a record's field is, so one taken whole is one
   type; an optional text with no `sce:max-size` is refused on C11.
6. **The saved state (Rust, Kotlin)** (implemented). `null` in the two saved-state schemas; the
   shape type; the per-value check of a `null`; `saved/static_optional.json` and
   `saved/format1/static_optional.json` (one absent and one present value of each element),
   validated by `sce-build/tests/saved_state_schema.rs` and restored by each backend's saved-state
   tests; a state of a machine whose variable is `int64` restored into the machine widened to
   `optional<int64>` (restores, present), and a state holding `null` restored into the machine of
   `int64` (refused, naming the variable); a test that the shape of a widened machine is the shape
   of the unwidened one, and that `int64` to `int32` still moves it.
   As built, the Rust and Kotlin backends are the two that save; Go, C++ and C11 declare the
   format-2 shape only, which now names a machine that holds an optional value too. The shared
   walk spells a variable by its shape type (`static_lowering::shape_type`, a field already by its
   present type), so `a_tree_can_hold` no longer refuses a machine that holds an optional value.
   Rust saves one through `SavedValue for Option<T>` and, for a text or a byte string held to a
   bound, `saved_state::optional` and `optional_to_saved`; Kotlin through `SavedValues.optional`,
   a record's own `toSaved` / `fromSaved` included. The per-value check is each runtime's reader of
   `T`: a `null` where the restoring build's type is `T` is refused as "'<variable>' is absent
   (null), and its type is not optional", a record's field named `<variable>.<field>`. The machine
   before the widening is `static_optional_unwidened.scxml` (over `schema_due_unwidened.scxml`):
   the states and variables of `static_optional.scxml`, each optional value of its present type.
   Tests: `a_widening_to_an_optional_value_keeps_the_saved_shape` in `static_lowering`; the
   runtime's `a_value_that_may_be_absent_is_saved_as_null_or_its_present_value` and
   `a_saved_null_is_refused_where_the_type_is_not_optional`; the `static_optional` section of
   `backends/rust/tests/tests/static_datamodel.rs` and of the Kotlin `StaticDatamodelTest`; and
   `an_absent_value_is_null_in_both_formats` in `saved_state_schema.rs`. The scenario replays
   on Rust and Kotlin read the machine back through its saved state again.
7. **Widening across revisions.** `sce:optional-since` in the grammar, the forge roster of known
   names, the reader and the rendering; the amended `divergence` and its `Redeclared` reason; the
   shape type in the per-revision digests; the restore refusing `null` saved at a revision before
   the widening; the absent default. Tests: `forge::schema_revision`'s unit tests (widened alike,
   widened at two revisions, narrowed, widened past the other's revision); a new
   `scenarios/static_optional_widened.json` on the seven engines (an entry of revision 1 read
   present, one of revision 2 read absent, one lacking a first-revision field refused, an added
   optional field with no default read absent); a revision-1 saved instance restored into the
   revision-2 build and one holding `null` at revision 1 refused; the Mesh check over two machines
   built from the two revisions. Docs: ADR 0044's amendments in docs/SCE_ACCEPTED_SUBSET.md "An event
   schema that grows" and "What a revision does not make safe".
   As built, `sce:optional-since` is `ForgeField::optional_since`, read only on an optional field of
   an event schema (`read_optional_since`; on a field of `T` it is unread, so
   `validation/sce-attribute-unread`), declared in `schemas/sce-forge-ext.xsd`, on the forge roster of
   known names, and rendered and read back as the field clause `optional-since <n>`. The rule is
   `forge::schema_revision::divergence` with `field_at`, a field as a given revision declares it (of
   its present type before the revision that widened it): two fields of one id are alike as
   declared, or alike at the revision of the schema that predates the other's widening.
   `Divergence::Redeclared` carries each side's `DeclaredType` (the `sce:type` and the widening
   revision), which `mesh/event-schema-mismatch` names in its reason; its id is unchanged. The
   per-revision digests already named each field by its present type (`revision_digest`), so a
   revision that only widens writes the digest of the one before it. The absent default is
   `schema_revision::default_wire` answering `null` for an optional field a later revision added
   with no `sce:default`: every generated lift's defaults object holds it, the Interpreter's `field`
   is handed it as its fourth argument, and a saved record grows the field as `null`. The fixtures
   are `schema_due_widened_first.scxml` and `schema_due_widened.scxml`, the two revisions of
   `due.widened`, and the machine `static_optional_widened.scxml` over the second; the scenario
   holds variables only, so the seven drivers read the day and the reminder as they read
   `static_optional`'s optional variables. The Mesh check is
   `a_field_widened_by_a_later_revision_builds_as_one_deployment_in_either_direction` and
   `a_change_of_type_other_than_a_widening_is_refused_with_both_types` in
   `sce-build/tests/event_schema_kind.rs`. An older receiver's refusal of `null` is each runtime's
   reader of `T` (the Rust runtime's
   `a_field_of_t_refuses_null_as_an_older_receiver_of_a_widened_field_does`); the scenario
   vocabulary admits `null` only where the machine's field is optional, so no scenario states it.
   As built, the restore refuses a `null` a record field holds in a state saved at a revision
   before the one that widened the field. Such a state is one no build writes (the field was `T`
   there). The saved shape the generator computes (`SavedRecordLineage`) carries, beside `added`,
   `widened`: each field a revision widened, with the revision that added it (1 for a field of the
   first revision) and the one that widened it (`SavedWidenedField`). Every machine that restores a
   saved state writes it into its shape constant (`WidenedField` in the Rust, Kotlin, Python, Go
   and C++ runtimes), and each runtime's `upgrade` judges it with the revision the state names for
   the schema, on the values as saved and before a record is grown by defaults (which may be `null`
   for an optional field added with no default): a record variable, or each record of a list,
   holding `null` in a field with `since <= at < optional_since` is refused as `'<variable>.<field>'
   is null, which revision <at> of '<event>' holds as a value: revision <optional_since> widened
   it`. A field the revision does not yet declare keeps the refusal of a field a revision does not
   declare, and a state saved at or after the widening reads `null` as absent. C11 holds no shape
   constant: its generated record reader takes the revision of the schema the state was saved at
   as `at` and refuses the same `null` with the same text. The tests are the unit tests of each
   runtime (`a_null_saved_before_the_field_was_widened_is_refused` in the Rust runtime,
   `ANullSavedBeforeAFieldWasWidenedIsRefusedTest` in Kotlin,
   `test_a_null_saved_before_the_field_was_widened_is_refused` in Python,
   `TestANullSavedBeforeAFieldWasWidenedIsRefused` in Go, `SavedState.ANullSavedBeforeAFieldWasWidenedIsRefused`
   in C++), and in `static_lowering` the shape's `widened` and the C reader's text. No machine
   of the fixtures holds a record of a widened schema, so no saved instance of a generated machine
   exercises the refusal; the runtimes' `upgrade` is what the generated machines call.
8. **Algorithms** (implemented). Optional parameters, returns and locals (the algorithm reader stops refusing
   them); the adapters' copy of an optional field and of an optional enum crossing by name; section
   3 in a body. Tests: `scenarios/static_state_algorithm_optional.json` (an algorithm that answers
   the earliest due day of a list of records, absent for an empty list or one with none due, using
   a guard and an early return) on every engine.
   As built, an algorithm holds an optional value of a fixed-width number or a `bool` on every
   generated backend: its slots, its locals and a field of a record it reads or returns (step 9
   widens what the reader admits, below, and `generator::algorithm_optional_spelled` is the one
   answer, a language and an element, which the renderers, a call's crossing and the event schema's
   unit share). An optional text, byte string, value of an enum or record in an
   algorithm's slot, and an algorithm whose record has an
   optional text or enum field, and the unit of such a schema, are refused by name where each
   backend renders them; the optional enum crossing an adapter would map is left to the step that
   holds those elements in an algorithm. Each backend spells the value in its statechart's type
   (`algorithm_optional_type`), the walk of a body threads the narrowing into each expression it
   lowers (`lower_algorithm_block`, the Interpreter's `statements`), and the integer analysis
   (`forge::int_ranges`) types a body where the same facts hold, so a proven read is judged by the
   arithmetic contract rather than passed over. A statechart's call names an optional slot
   `StaticSlot::Optional`, the same type on both sides, which crosses an adapter as it is and is
   called directly where the call has no record or list; another algorithm's body calls one through
   `CalleeSlot::Optional`. Tests: `sce-build/tests/an_algorithm_holds_an_optional_value.rs`, and the
   algorithm cases of `sce-build/tests/a_value_is_proven_present_only_where_tested.rs`, now
   generated.
   Another algorithm's body calls one with an optional enum or an optional record slot as it
   does one of a number: `CalleeEnumSlots` names the enum of the slot as the caller imports it and
   says it may be absent, so `forge::static_enum` holds an argument and the answer to the enum
   document the slot is of, and `CalleeSlot::OptionalRecord` carries the schema of an optional
   record slot, which `forge::optional_record_call` holds an argument (`null`, a record or an
   optional record of that schema) and an answer to, since the type an expression has names no
   schema. Tests: `sce-build/tests/an_algorithm_calls_another_with_an_optional_enum_or_record.rs`.
9. **`optional<record:X>`** (implemented). A variable and an algorithm slot of an
   optional record: started at `null`, taken whole from a record, a payload, a list's element;
   `r.value.f` where proven; a field write to `r.value.f` where proven, and refused elsewhere.
   Tests: `scenarios/static_optional_record.json` on every engine, and its saved instance.
   As built, the shared part lands on the Interpreter, and each generated backend spells an
   optional record in its own step, in the hooks the shared walk gives it:
   - *What the language says.* A variable starts at `null` and only there (`expr="null"`; a record
     is built from `<sce:set>` children, which a variable that may hold none has no use for). A
     whole record is put into it by the rule a record variable takes one by (`Judge::optional_record_assign`):
     a record variable, a loop's record item, `_event.data` when the event's schema is the record's,
     the element of a list of its schema at an index, what an imported algorithm returns as a record or
     as an optional record, and another optional record of the same schema; `null` makes it absent.
     The lattice's `OptionalElem::Record` carries no schema, as `ListElem::Record` carries none, so the
     schema is held where the record is named: a record of another schema is refused at the statement
     that puts it, in a statechart and in an algorithm's body (`AlgorithmWalk::record_fits`).
     `x.value.f` reads a field of the present record: the present value is a closed record of the
     schema's fields at the path `<x>.value` (`TypeCtx::insert_record`, `static_record_paths`), so a field
     the schema does not declare is `expression/unknown-member`, and the read stands only where `x` is
     proven present (`expression/optional-read-unproven`, the same message as every unproven read, which
     for a record names the test and a default that a record has none of; the refusal of `??`
     says what to write instead). `x.value` taken whole is no expression (a record is no operand), `x ?? d`
     on an optional record and `===` of two of them are refused by name; `x === null` and `x !== null`
     are its one comparison. A field write, `<assign location="x.value.f">` or `<sce:assign
     target="x.value.f">`, is judged where it is written: refused as `expression/unsupported-construct`
     ("a write to ... where no test proves ... present") unless `x` is proven, and a write below
     the present value ends no fact about `x` itself (`narrowing::ends`): a second field may be written
     after the first. An optional record local of an algorithm is written the same way, and takes a
     record by its name (`<sce:assign target="held" expr="it"/>`).
   - *Algorithm slots.* `algorithm_optional_admitted` is gone: the reader admits every element a
     statechart's variable holds one of (a number, a `bool`, a text, a byte string, a value of an enum, a
     record), and the decision of what a backend spells is one function, a language and an element
     (`generator::algorithm_optional_spelled`), whose arm for a language each backend's step edits and no
     other's. Every generated backend still answers a number or a `bool` alone, and refuses the others by
     name where it renders the slot, the local or the record field (`algorithm_optional_type`,
     `AlgorithmTypes::refuse_optional`, the event schema's unit), `generate/unsupported-feature`, "an
     optional value has no <language> lowering yet". The Interpreter spells all but a byte string, an
     algorithm's own array of numbers, which a statechart's variable is not: an optional text is a
     string, an enum its declared name, a record the plain object it is, written again with a field
     changed (`SceStatic.set`). A statechart's call crosses an optional text and an optional record
     (`StaticSlot::OptionalRecord`, handed by the name of the variable that holds it, a record variable or
     a loop's item or the payload lifted, or `null`, state code `n`) and an optional value of an enum, which
     the Interpreter's call hands as the declared name it holds; an optional byte string in a slot is
     refused by name where the import is read (`value_signature`), and a
     record's optional text or enum field copies across a call as it is, the enum field by the declared
     name of its variant as an enum field does.
   - *The hooks.* `StaticTarget::lowers_optional_record` (by default `ExprTarget::lowers_optional_record`,
     true for the Interpreter alone) gates the document that holds one, a call that takes or returns one
     and an expression that reads one (`reject_unlowered_optional`); a target that answers `true` spells
     `optional_type`, `optional_none` and `optional_some` over the record's own type (the walk hands
     them `record_type` as the present type), `optional_record_def` where the optional type is declared
     beside the record, `assign_present_field` (with its string and bytes forms) for the write of a
     field, `record_at` for an element of a list taken whole, and, in its expression emitter, the proven
     read `x.value` over which the member `.f` is read. A target that answers `true` and spells no
     `assign_present_field` is refused by name where a document declares one.
   - *The saved state.* The shape type of `optional<record:X>` is the shape type of `record:X`, by the
     one rule step 6 used for scalars: a variable widened from `record:X` to `optional<record:X>` keeps its
     line and its fields', and an optional record variable is a holder of its schema's lineage as a
     record variable is. A saved state holds `null` or the record as an object whose members are its
     fields, which the saved-state schemas already admit; `saved/static_optional_record.json` and its
     format-1 twin hold one absent (`found`) and one present (`pick`) beside the machine's other values.
     Each generated backend that restores a saved state restores it, in the test of its own paragraph below.
   - *The scenario.* `static_optional_record.json` drives `static_optional_record.scxml` over
     `schema_item_pick.scxml` and `algorithm_item_pick_first.scxml`: an optional record taken from a payload,
     a record variable and a list's element, read and written under a test, made absent, and answered by an
     algorithm; `every_replay_scenario_keeps_to_its_vocabulary` holds its `null`s to the machine's optional
     records. The Interpreter replays it, and so does each generated backend, in the driver its own
     paragraph below names. Tests: `an_optional_record_is_held_read_and_written_where_proven`
     and the algorithm cases of `an_algorithm_holds_an_optional_value`.
   - *Python.* Python lowers all of it, in `Optional[T]` over the type a present value has there.
     A statechart's optional record is `Optional[<the machine's record>]`, started at `None`, the record being
     the frozen dataclass a record variable is: a whole record is put into it as the variable would take it (a
     list's element by the runtime's checked read, `StaticTarget::record_at`, which raises
     `AlgorithmFailure` past the list as every checked operation does), `x.value.f` is the member of the
     record `x` is, and a field is written by replacing the record with one whose field is changed
     (`assign_present_field`, the text and byte-string forms being the same statement over a value the walk
     has already held to the bound of the field). The saved form is `null` or the record's own
     (`saved_write_optional` and `saved_read_optional` over `to_saved` and `from_saved`), and the shared saved
     instance is restored and written again byte for byte. A call hands an optional record slot the variable
     that holds one, a record, or `null` (the state code `n`, spelled `None`): the adapter tests the argument
     (`None if p is None else <the algorithm's payload built from it>`) and builds the machine's record from
     what the algorithm answers where it is present. `algorithm_optional_spelled` answers `true` for every
     element on Python: an algorithm's slot, local or record field may be an optional text (`Optional[str]`),
     byte string (`Optional[bytes]`), value of an enum (`Optional[<the enum document's class>]`) or record
     (`Optional[<the schema's payload class>]`), and the unit of a schema with an optional text or enum field
     is generated. A record put into an optional record in an algorithm's body is a copy
     (`sce_algorithm.copied`), since the payload class is a mutable object and a name would otherwise share it;
     a field of the present record is written in place (`held.f = e`, a text by `lower_text_write` to the bound
     its field declares), where a test proves the record present. An optional enum field of a record crosses an
     adapter by the declared name of its variant where it is present and stays `None` where it is not. Still
     refused by name, for a reason that is not Python's: a statechart's call of an algorithm whose own slot is
     an optional byte string (refused where the import is read, since the Interpreter's byte string is another
     value). A call whose slot is an optional value of an enum maps it by the declared name of its variant,
     in the parameter and the answer (`AdapterSpec::optional_enum_crossing`). Tests:
     `an_optional_record_is_spelled_by_generated_python`, `static_optional_record` in
     `backends/python/tests/integration/static_datamodel/test_static_scenarios.py` and its saved instance in
     `test_a_state_another_backend_saved_is_restored.py`.
   - *C++.* An optional record is a `std::optional` over the machine's own record
     (`<Machine><Alias>Record`), started at `std::optional<…>()`; `x.value.f` is `(*x).f` and a
     field written is `(*x).f = e;` (`assign_present_field`, its text and byte-string forms
     the same statement over a value already held to its bound). A list's element taken whole
     is the runtime's checked read, `SCE::Forge::Checked::at(sce_failure_, list, index)`
     (`record_at`), which records an index outside the list as the failure every statement ends
     its block at; a record variable and a payload are lifted by `std::optional`'s own
     conversion, and the saved state writes the variable as null or through the record's own
     `<Record>ToSaved` and reads it through `<Record>FromSaved` (`cpp_saved_value.jinja2`).
     A call hands an optional record over as the adapter's `const std::optional<Record> &`
     (an optional record variable, a record variable and `null`, which is `std::nullopt`) mapped
     into the algorithm's record by `SCE::Forge::mapPresent`, and a payload and a loop's item
     as a record lifted into the present value; what the algorithm answers is mapped back the
     same way. In an algorithm an optional record is the same `std::optional` over the type the
     imported schema declares, in a parameter, a local and a return, and a field is written as
     `(*x).f = e;` where the narrowing proves `x` present, its text from a bounded literal or a
     text field of a record, as a bare text field is (`lower_field_text_write`).
     `generator::algorithm_optional_spelled` answers true for C++ for a number, a `bool`, a text, a
     value of an enum and a record, so an algorithm's slot or local of one, a record with an
     optional text or enum field (the unit of its schema, and what an adapter copies) are
     spelled: an optional text field is written from a literal held to its bound or `null`, and
     an optional enum field crosses an adapter by the declared name of its variant, mapped where
     it is present (`AdapterSpec::record_field_crossing`). What C++ keeps refused by name is the
     optional byte string in an algorithm's slot, local and record field: an algorithm holds a
     byte string as a borrowed view in a parameter, as an append buffer in a local and as an
     owned vector in a return, three types no one `std::optional` holds as it is, and a record
     with a byte string field is no record of an algorithm (`resolve_record`). A statechart's call
     whose slot is an optional value of an enum crosses it by the declared name of its variant
     (`optional_enum_crossing` through `mapPresent`), in the parameter and the answer. Tests:
     `an_optional_record_is_spelled_by_the_cpp_backend`, the scenario `static_optional_record.json`
     replayed by the C++ scenario suite, and `saved/static_optional_record.json` restored and saved
     again by `AStateAnotherBackendSavedIsRestoredTest`.
   - *Kotlin.* A variable is `T?` over the record's own data class (the one spelling
     `optional_type` gives every element), absent is `null` and a present record is the data
     class itself, so a record put into it is the reference a record variable is given. A
     field is read as `SceOptional.proven(x).f`, the function every proven read is spelled
     with, and written as `x = SceOptional.proven(x).copy(f = e)` (`assign_present_field`;
     the walk holds a text or a byte string to its bound before it hands the hook the value,
     as it does for a record variable's field). A list's element is taken whole by
     `SceChecked.at` (`record_at`), which a record variable takes one from a list by too, in
     place of the field-by-field copy it had. A call hands an optional record to the machine's
     adapter by the name of the variable that holds it: the parameter of the adapter is the
     machine's `T?`, so an optional record variable, a record variable (lifted) and the literal
     `null` are served alike (the codes `v` and `n`), and the payload and a loop's item, which
     are records, are lifted where the adapter builds the algorithm's record from them; what
     the algorithm answers is read back as `r?.let { <machine record> }`. An algorithm's
     parameter, return and local of `optional<record:X>` are `T?` over the schema's payload
     class (`algorithm_optional_type` resolves the record through the algorithm's imports),
     and a field of the present value is written as the data class replaced with that field
     changed, a text field from a literal or a record's text field the build bounds
     (`lower_optional_record_field_write`). The saved state is `SavedValues.optional` over
     the record's own `toSaved` and `fromSaved`, a published one is handed to a host as it is
     (a record with a byte string, detached where present), and the saved instance of the
     scenario is restored by `StaticDatamodelTest`; the scenario is replayed by
     `StaticScenarioTest`. Tests: `a_kotlin_machine_holds_an_optional_record`.
     Kotlin's arm of `algorithm_optional_spelled` answers every element: an algorithm's
     slot and local of an optional text is `String?`, of a byte string `ByteArray?`, of a value
     of an enum the enum document's class with `?`, and of a record its payload class with `?`,
     and the unit of an event schema and the record an algorithm reads hold a field of one in
     the same type. The enum judge registers the enum a parameter or local of
     `optional<enum:…>` holds at its present value (`status.value`), as it does a record's
     optional enum field, so that a value of it is compared with its own variant. A text an
     algorithm writes to a record's field of `optional<string>` is judged as a text field's is
     (`judge_text_write`), from a literal or a record's text field that fits the field's
     `sce:max-size`, and from `null` (`TextSource::Absent`), so no optional text of a record
     is held past the bound its schema declares, in the Interpreter's lowering too, which wrote
     such a field unbounded. A statechart's call copies a record's optional text as it is and
     its optional value of an enum by the declared name of its variant where it is present
     (`AdapterSpec::optional_enum_crossing` through `optional_map`), and so does an optional
     value of an enum in a slot of an imported algorithm (`value_signature` names it as
     `StaticSlot::Optional` over the statechart's alias of the enum; the enum judge's
     `CallEnums` carries `EnumSlot { alias, optional }`, so an argument of such a parameter is
     `null`, a value of the enum or an optional value of it, and a call that returns one is an
     optional value of the enum). A record put into an optional record local, or returned as
     one, by its name is the local itself, since a bare record name is no expression. Kept
     refused by name: an optional byte string in the slot of an imported algorithm that a
     statechart calls (`value_signature`), since a byte string an algorithm holds is not the
     one a statechart's variable holds on every backend; an algorithm's call of another with
     an optional value of an enum keeps its refusal of a variant as the argument.
     Tests: `a_kotlin_algorithm_holds_every_optional_element`.
   - *Rust.* A variable of an optional record is an `Option` of the record's own type, started at
     `None`, and lent to a host as `&Option<…>` where the record owns text (the field's `view`, which the
     snapshot clones). A whole record is put into it as a copy, a clone where the record owns text and
     `Copy` otherwise: a record variable, a loop's item, another optional record, the payload made whole
     (each text checked against its bound first) and a list's element, which `StaticTarget::record_element`
     reads by reference and clones through the runtime's `at_ref`, or by value through `at` for a record
     that is plain data. `x.value.f` is read as `sce_forge_runtime::optional::proven_ref(&x).f` and a field
     is written in place as `proven_mut(&mut x).f = e` (`StaticTarget::assign_present_field`): the record
     stays where it is held and is never moved out, and the absent arm of both is `unreachable!`, written
     once in the runtime as `proven`'s is. A call crosses an optional record through one adapter for each way
     its argument is held: a record variable (`v`), the payload (`p`) and a loop's item (`i`) are lifted into
     `Some(…)`, an optional record variable (`o`) is lent as `&Option<…>` and copied where it is present, and
     `null` (`n`) is lent as `&None::<…>`; `o` is a code of its own beside `v`, since the two are values of
     different types on a backend that converts neither at the call, and the optional record an algorithm
     answers is mapped into the machine's own record. In an algorithm an optional record is an
     `Option<schema_unit::…Payload>` in a parameter, the return and a local; a field of a local is written in
     place where a test proves it present, a parameter is read only, and a whole record is taken into it by a
     clone. Its saved state needs nothing of its own (`SavedValue for Option<T>` over the record's), and a
     `null` where the restoring build holds a record that is never absent is refused naming the variable, as
     every `null` is (`record_member`). Rust's arm of `algorithm_optional_spelled` answers every element but a
     byte string: a text is an `Option<&str>` in a parameter, an `Option<String>` in a return and a local and
     the schema's bounded `Option<SceString<N>>` in a record's field, which an adapter copies where it is
     present; a value of an enum is an `Option` of its own type, and a record's optional enum field crosses
     an adapter by the declared name of its variant. A byte string stays refused by name in a slot, since a
     slot declares no bound for it (an algorithm's bytes carry theirs beside the buffer they build). A call
     whose slot is an optional value of an enum is named where the import is read (`value_signature`, one
     reading for every backend: the enum judge holds an argument to the enum of its slot), and its adapter is
     the machine's `Option` over its own enum, mapped by the declared name of the variant where it is present,
     in the parameter and the answer. Tests: `sce-build/tests/an_optional_record_is_held_read_and_written_where_proven.rs`
     (the `rust_…` cases), the runtime's `optional` and `saved_state` tests, `static_scenarios`'s
     `static_optional_record_holds_reads_and_writes_a_record_that_may_be_absent` and the saved-state tests of
     `static_datamodel.rs`.
   - *Go.* A variable is `optional.Of[R]` over the machine's own record struct, built by the package's
     two constructors and started at `optional.None[R]()`. A present record is a value `Proven()`
     answers, so `r.value.f` is `r.Proven().f`, and a write of a field is a block that copies the
     present record, writes the field into the copy (a text or a byte string held to its bound by the
     helpers a record's field takes) and puts `optional.Some(copy)` back (`assign_present_field`). A
     list's element is read whole through `scealgorithm.At` (`record_at`), which a record variable
     assigned from `items[i]` takes now too, where it was copied a field at a time. The adapter of a call
     names the algorithm's record type for a parameter only: it builds the algorithm's optional record
     from the machine's present one, and takes an optional record back through `Get()`, building the
     machine's own struct field by field, so a return alone imports no schema package. `o` names an
     optional record variable among the codes of an optional record slot's argument, apart from `v`,
     a record variable the call lifts, since Go types tell the two apart; `null` is
     `optional.None[R]()` (`state_argument`). A saved state writes the record through its own `ToSaved`
     and reads it through `<R>FromSaved`, as it does an enum.
     `generator::algorithm_optional_spelled` answers `true` for every element on Go: an algorithm's
     parameter, return and local of `optional<string>`, `optional<bytes>`, `optional<enum:E>` and
     `optional<record:X>` are `optional.Of[T]` over the type the algorithm holds a present value in
     (the enum document's own type, the schema's payload struct), a record's optional text or enum
     field is the same type in the schema's unit, and an optional enum field crosses an adapter by
     `optional.Convert` over the declared name of the variant. The absent value of an enum or a
     record names a type the expression layer does not carry, so a place that is given `null` whole (a
     local's start, an assignment to a local or parameter, the return) writes its own
     (`TypedAbsences`), and `null` of one of them inside an expression is refused by name
     as it is in a statechart. A field of the present value of an optional record an algorithm holds
     is written through the same block (`<sce:assign target="x.value.f">`), a text from a literal or
     a record's text field held to the bound the schema declares, and an optional text field lifted.
     A statechart's call whose slot is an optional value of an enum converts it by the package's `Convert`
     (`optional.Convert`), in the parameter and the answer. Still refused, on every backend: an imported
     algorithm's slot of an optional byte string in a statechart's call (the byte string an algorithm
     holds is not every backend's statechart one), at the
     import, `x ?? d` of an optional enum where the lattice names no type for `d`, and a record
     whose optional field is a byte string, which no algorithm's record carries. Tests:
     `go_lowers_the_optional_record_scenario_machine`, `go_lowers_an_algorithm_that_answers_an_optional_record`,
     `go_reads_and_writes_a_field_of_an_algorithms_optional_record` and
     `go_spells_an_optional_text_byte_string_enum_and_record_in_an_algorithms_slots`, the Go runtime's
     `TestConvertChangesTheTypeOfAPresentValueOnly`, and `static_scenarios_test.go`
     `TestARecordThatMayBeAbsentIsHeldReadAndWrittenWhereProven` with the saved instance in
     `a_state_another_backend_saved_is_restored_test.go`.
   - *C11.* The variable is `sce_optional_record_<type>_t`, declared by `SCE_OPTIONAL_DECLARE_RECORD` of
     `sce/forge/optional.h` (the constructors and no `_eq`, since two records are no operand of `===`) beside
     the machine's own record type, named by that type so that the machine's record and the algorithm's, two
     structs, each have their own. It starts at `_none()` and takes a record whole as `_some(copy)`: a payload
     field by field and held to the bounds as a record variable takes one, a record variable by value, an
     element of a list by a checked index read that gives a zero record when the index is outside the list
     (`StaticTarget::record_at`), and what an algorithm answers. `x.value.f` is the member `.value.f` of the
     struct, a text field through its buffer (`.value.f.data`), and a field is written through `.value` by the
     three hooks, the text and the byte string by the copy a record variable's field takes. A call hands an
     algorithm one as the machine's optional value whatever holds it (a record variable is lifted at the call,
     which the code `v` names alike for it and for an optional record variable), and the adapter builds the
     algorithm's own from it field by field, and the machine's from the algorithm's answer. A saved state holds
     it as `null` or the record, and the C11 saved-state test restores `saved/static_optional_record.json` and
     its format-1 twin (a text is held to the machine of the longest stem that begins its name, since
     `static_optional_record` begins with `static_optional`). The scenario is replayed by
     `test_static_scalars.c` and is off the `NOT_YET_LOWERED` list.
     `algorithm_optional_spelled` answers, for C11, a number, a `bool`, a value of an enum, a record and a text
     as the field of a record over the buffer of its `sce:max-size`. An algorithm's parameter, local and return
     of an optional record is `sce_optional_record_<SchemaPayload>_t`, of an optional enum
     `sce_optional_enum_<document>_t`, each declared in the unit that holds one under a guard the machine, the
     schema and the algorithm share, so a program that includes them declares it once; the schema's unit
     declares the optional text and enum its fields hold. The value `null` and a record or variant lifted into
     one are written whole where a place says the type (`c11_whole_optional`: an initial value, an
     `<sce:assign>` by `__typeof__` of its place, a return), as a statechart's variable takes them, and a
     `null` or a lift nested in an expression stays refused by name, as it is for a text. A text in an
     algorithm's own parameter, local or return declares no bound for the buffer an optional text is held in
     (section 1: a text is bounded by `sce:capacity` or `sce:max-size`), so it keeps the named refusal, as does
     a byte string, which no algorithm's slot or record holds. Tests:
     `c11_unit_forge_optional` (the record's type and the constructors by place), the C11 cases of
     `an_optional_record_is_held_read_and_written_where_proven` and of
     `an_optional_value_is_declared_by_its_type`, `c11_spells_an_optional_record_over_its_struct_type`, and the
     `static_optional_record` scenario and saved state of the C11 suite.
10. **The edges left refused by name**: a child session's optional variable as a `<param>` (with
    the shape of `InvokeParams` it needs), a host action's optional argument, a host-run
    `<invoke>`'s typed request and result with optional fields. Each with the scenario or test of
    its edge.
    As built for the first edge, a child session's optional variable as a `<param>` (edge A):
    - *The shape.* `InvokeParams` keeps one slot per seedable variable, and a variable of
      `optional<T>` is that slot in its own optional type: `Option<T>`, `T?`, `std::optional<T>`,
      `optional.Of[T]` and `Optional[T]`, each left absent when nothing is handed, and tested
      before it is taken (`is_some()`, `?.let`, `if (params.v_x)`, `IsPresent()`, `is not None`);
      C11 has no such object and the invoking machine writes the whole `sce_optional_<elem>_t`
      into the child's variable. No backend nests an optional in an optional, which the first
      draft of section 2 feared and four of the carriers could not hold. `StaticField::seeded`
      says which variables a slot is made for (one rule, `static_datamodel::seed_slot`, for the
      judge, the lowering and every template), and `Param::native_seed_optional` that a seed is
      such a whole value, so that a template does not wrap it in the optional of the slot a
      scalar's seed is put in (Rust `Some(…)`, Go `&sceValue`, C++ `::value_type`).
    - *The value.* The seed of an optional variable is lowered by the rule an `<assign>` to it
      writes by (`optional_whole_value`, shared with `lower_optional_assign`): `null` is the
      absent value, an optional value is handed as it is, a value of `T` is lowered into the
      element and lifted, and a text is held to the `sce:capacity` the child declares, a value past
      it failing as any other does (left out, `error.execution`, the child keeps its start). The
      expected slot is the variable's `optional<T>`.
    - *Absent is "not handed".* An absent seed is no more than a seed not handed, which is the
      child's own start, so the child's variable is `null` either way only where it starts at
      `null`. Where it starts at a value (`expr="3"`), a value that may be absent (an optional
      variable, `null`, a conditional of them) would leave the child at 3 and read as absent, a
      silent difference between what the document says and what runs: it is refused at the
      `<param>` as `scxml/static-datamodel-rule`, naming the start and the two ways to write a
      value that is never absent, a value of `T` and `x ?? d`. A value of `T` lifts into such a
      variable and replaces its start.
    - *What stays refused.* A child's optional enum, byte string or record, by the wording that
      names a list, a record, an enum and bytes (`unseedable_variable`).
    - *The Interpreter.* The lowered `<param>` carries the optional value as the library spells
      it (`null` or the value, a text's present value through `SceStatic.mapPresent` and the
      bound); a `namelist` name that lands in an optional text the child bounds is refused by
      name, as one that lands in a bounded text is.
    Tests: `scenarios/static_invoke_optional.json` on every engine (five children end on what
    they hold, the sixth, handed an absent variable and `null`, never ends),
    `sce-build/tests/a_childs_optional_variable_is_handed_an_optional_value.rs` (the judge's rules
    and each backend's slot) and the unit tests of `static_datamodel`.
    As built for the second edge, *a host action's optional argument* (implemented, edge B). The parameter of a host method for
    an argument that may be absent is the declared type of an event schema's field, a scalar or an
    optional scalar (`FieldType`), so no second type and no flag stands beside it:
    `native_action::static_argument_type` answers `FieldType::Optional { elem }` for an expression of
    `optional<T>` over a number, a `bool`, a text or a byte string, and the signature table, the
    generated interface, the recording hosts and the receivers of a failing argument
    (`StaticTarget::receiving_host_call`) read that one type. Each backend spells it as its own
    optional type over the type a present value is passed as (`generator::host_param_type`):
    `Option<T>` on Rust, with a text as `Option<&str>` and a byte string as `Option<&[u8]>`, borrowed
    inside the `Option` as an algorithm's parameter borrows them, so the machine's expression reads
    them as `x.as_deref()`; `T?`; `optional.Of[T]`; `Optional[T]`; `std::optional<T>`, by value; and
    on C11 the struct `sce/forge/optional.h` declares for a number or a `bool`, held in a local of its
    type where the call receives a failing argument. The signature's canonical string is
    `optional<int32>` against `int32` (`canonical_argument_type`), so two calls of one name that
    disagree are the existing `validation/native-action-signature-conflict`, and an argument is never
    coerced to the type another call gave the position. The Interpreter lifts the edge by its engine:
    the lowering leaves each argument for the engine to compute, `ActionExecutorImpl::executeNativeAction`
    takes `ScriptNull` among the values a host takes (it raised `error.execution` for it), and the host
    is handed `null` for an absent argument, as a generated host is handed its language's absent
    optional. Refused, kept: `null` as an argument has no type, since it is typed by the optional place
    it stands in (`validation/native-action-argument`, as every argument of no type is); a value of an
    enum or a record is no host argument, present or absent (the same code, naming the type: a host
    method takes no enum or record argument today); C11's text and byte string
    (`generate/unsupported-feature`, "an optional value has no C11 lowering yet", from
    `StaticTarget::holds_host_optional` where the walk meets the action), since its struct is declared
    over a buffer of a bound and a host method's parameter declares none; and the payload channel of a
    document that is not `sce-static` (`validate_args`: "an enum-typed or optional EventSchema field"),
    where an argument is a bare `_event.data.<field>` that no scope types, which stays as it was. Under
    `sce-static` an optional field of the payload is an argument as the expression it is
    (`_event.data.due`), and `_event.data.due.value` is one of `T` where the `cond` of the transition
    proves it present: the call is rendered under the facts validation judged it by
    (`native_action::facts_of_condition`, handed to `lower_static_argument`), where it used to be judged
    without them and a `.value` the first admitted panicked in the second. A unit that names the
    runtime's optional type only in the host interface (`flag ? 3 : null` as the argument, no optional
    variable) imports it as one that holds a variable does (`Rewrites::host_optional`). The scenario
    `static_host_call.json` calls `showDue(day, urgent)`, an `optional<int32>` and an `optional<bool>`,
    from three events: an absent argument is `null` in `host_calls`, and
    `scenario_replay::MachineFacts::host_arguments` admits a `null` exactly at the positions the
    machine's action declares optional (`arguments_that_may_be_absent`). Tests:
    `sce-build/tests/a_host_action_takes_an_argument_that_may_be_absent.rs` and the unit tests of
    `forge::native_action` (the interface, the recorders and the signature of each backend), the
    scenario on the seven drivers, each driver's own host of `static_host_call`
    (`static_datamodel.rs`, `StaticDatamodelTest`, `static_scenarios_test.go`,
    `test_a_static_host_action.py`, `AStaticHostIsHandedTypedValuesTest`, `test_static_host_call.c`),
    and `AnArgumentThatIsNullIsHandedToTheHostAsAbsent` for the Interpreter's executor.
    *As built, for a host-run `<invoke>`'s typed request and result.* The edge is decided by the
    carrier of each half, as section 2 and section 4 say a carrier decides it. A host-run record
    crosses to the host in two ways: the request as text (each `<param>` is the text of its value,
    in the `params` a handler reads, and the generated adapter parses them back), and the result as
    JSON (`complete_host_invoke` with the record's JSON, which the machine's payload lift reads).
    - *The request keeps its refusal, and it is final.* A field of a `sce:request` schema that may
      be absent is refused where the invoke is checked (`validation/typed-invoke-schema`, in
      `forge::typed_invoke::validate`), naming `??`: declare the field as its present type and
      write `x ?? d` at the `<param>`. The carrier has no `null`, and a key left out of the
      `params` is not free to mean "absent": an untyped `<param>` whose value cannot be evaluated
      is reported and left out, so a handler that reads the `params` without the adapter would
      read as absent what the untyped invoke reports as not evaluated, which is the second
      meaning of one absence that this language refuses (no second absence, and never a
      sentinel). The message names both the carrier and the repair. The same schema named as the
      request and the result is held to the request's rule.
    - *The result lifts.* A field of a `sce:result` schema may be `optional<T>` of a number, a
      `bool`, a text or a byte string; an optional enum stays refused with the enum, since a
      host-run record carries no field of an enumeration (the existing refusal now reads the type
      a present value has). The record the generated interface hands the host holds the field in
      the backend's one optional type (`Option<T>`, `T?`, `optional.Of[T]`, `Optional[T]`,
      `std::optional<T>`, `sce_optional_<elem>_t`) through one function of the module that
      renders the interface (`record_field_type`, over `forge::generator::optional_of`), and its `wire`
      writes the key of every field, `null` where the field is absent and what the field is
      written as, on that backend, where it is present (Rust's `optional_json`, Go's
      `optional.Wire`, Python's `None`, which `json.dumps` writes as `null`, Kotlin's `encode`,
      C++'s `EventPayloadFields::absent`, C11's wire writer). The machine reads the completion
      with the payload lift of section 4: `null` is absent and a key left out is refused, as any
      typed payload the data does not fit is. The request-side readers and the descriptors every
      runtime holds (`RequestFieldSpelling`, `request_field`, `TypedRequest`) are unchanged, since
      no request holds an optional field.
    - *Whatever the data model.* The generated interface does not depend on the data model, so a
      machine of any data model holds the result's optional field: Go lists the forge runtime's
      `optional` package among its imports where the interface names it (`generator::render_go`),
      and C11 includes `sce/forge/optional.h` before the record and `sce/forge/wire.h` before the
      function that writes it (`C11HostInvokerInterface::header_includes`, `source_includes`),
      and declares the buffer of an optional text or byte string and the optional value over it
      under the guards every unit that holds one shares
      (`static_lowering::c11_optional_buffer_def`). A C11 optional text or byte string with no
      `sce:max-size` has no buffer to be held in and is refused by name, as it is in a payload
      (`generate/unsupported-feature`, "an optional value has no c11 lowering yet").
    - *The Interpreter* runs no host-run `<invoke>` (`InvokeHandlerFactory::createHandler`
      answers none for the type, and `InvokeExecutor::executeInvoke` raises `error.execution`, SCE
      Accepted Subset §2.12), so a result has nothing to lower there.
    - Tests: in `sce-build`, `a_host_run_result_may_hold_a_field_that_may_be_absent` (every
      backend checks one, and each renders its record's type and its `null`, and C11 refuses an
      unbounded optional text), `a_typed_host_invoke_is_held_to_its_schemas` (a result accepts
      one, a request, and a schema that is both, refuse it) and
      `an_optional_value_is_declared_by_its_type`; on every backend that runs a host-run
      `<invoke>`, `statechart_host_invoker_optional.scxml` and its `LookupResult` record
      (a day, a label and a flag that may be absent beside a count that never is): Rust
      `a_host_result_may_hold_a_value_that_is_absent`, Go `host_invoker_optional_test.go`, Kotlin
      `AHostResultMayHoldAValueThatIsAbsentTest`, Python
      `test_host_result_may_hold_a_value_that_is_absent.py`, C++
      `HostResultMayHoldAValueThatIsAbsentAotTest`, C11 `test_host_invoker_optional.c`. Each reads
      the all-absent and the all-present answer, a zero, an empty text and a false flag (present,
      not absent), a mixed one, the record's JSON with `null` for an absent value, and a
      completion that leaves a key out (refused: `answered` does not move).
11. **`record_refusal.none` retired** (implemented) (section 7): the enum, `record_list.scxml`,
    `record_list_parent.scxml` holding `optional<record:Refused>`, `scenarios/record_list_parent.json`
    and the Go, C and C++ drivers that read the refusal by hand, the pin in
    `standard_saved_shapes.json`, docs/SCE_ACCEPTED_SUBSET.md "The standard shared record list".
    Test: the scenario's first step expecting `refused` to be `null`.
    As built, `record_refusal` declares eight variants, `notWriter` = 1 to `clockExhausted` = 8, and
    no variant of 0: an enum's variants are the author's numbers (`enum_item_status` starts at 2), so
    nothing asked for a first variant of 0. `record_list.scxml` is not edited: it never named `none`
    (its `probe` and `spoiledFor` are each written, before a refusal is told, by the statement that
    raises it, and start at `changeTooLarge` because every variable declares a start; the list's own
    logic asks for no "no reason yet"), so what moves in it is the enum's saved shape alone. `record_list_parent.scxml` holds `refused` as an `optional<record:Refused>` of `expr="null"`
    and takes the payload whole in the handler of `record_list.refused` (`<assign location="refused"
    expr="_event.data"/>`, the form of `item.picked` in `static_optional_record`), so the record is the
    four fields of one refusal and is present only after the first. The document names no variant of
    either enum, so it no longer imports `record_command` or `record_refusal`: the record holds each in
    the machine's type for the enum document the schema imports (ADR 0054). `static_payload_enum_unimported`,
    which started `last` at `Why.none`, holds it as an `optional<enum:Why>` that starts `null`, so it still
    names the enum it imports. The Go and C drivers read `refused` as an optional record, as they read
    `pick` of `static_optional_record` (`optional.Of[R].Get()`, and the C presence reader over
    `sce_optional_record_record_list_parent_record_refused_t`); the Rust, Kotlin and Python drivers replay
    the scenario through the saved state and need no change; the Interpreter does not lower the parent (a
    child invoked by `src`). The pin of `stdlib/shared/record_list.scxml` in `standard_saved_shapes.json`
    is the digest the enum's change moves, and is written from the value `a_standard_statechart_that_saves_keeps_its_shape`
    reports.
12. **Codec format 2** (implemented) (section 8): `sce:codec-format` in the grammar (`schemas/sce-forge-ext.xsd`)
    and the roster of known names; `CborEntry` and the content-line entry typed by the value type;
    `sce:required` out of the format-2 grammar; the refusal of format 1,
    `validation/codec-format-superseded`, with the rewrite of every entry, and
    `sce-codegen codec-format`; every repository CBOR and content-line document and conformance case
    moved to format 2. Tests: the codec conformance oracle unchanged in its bytes; a document with no
    `sce:codec-format`, one with `="1"` and one with `="3"`, each refused; a format-1 document of
    bare entries only (all optional in format 1) refused with each entry's rewrite as `optional<T>`,
    and one mixing `sce:required="true"`, `="false"` and bare entries refused with the rewrite of
    each; `codec-format` printing a document that `check` then accepts and that generates the bytes
    the format-1 one did; `sce:required` in a format-2 document refused; the attribute on a
    positional codec refused.
    As built, `CborEntry::sce_type` and `ContentLineEntry::sce_type` are a `FieldType` (the type an
    event schema's field has: a scalar or an optional scalar), so no flag stands beside the type;
    `is_required()` answers whether a decode refuses a map or component without the entry, and
    `present_type()` the type a present value is, which every bound and wire kind is asked of. The
    format is judged first, before any entry is read (`forge::codec_format::judge`), and the rewrite
    is one computation (`codec_format::rewrite`) read by the refusal and by the command. The
    refusal's fix is a variant the contract did not have, `rewrite_codec_format` (`format` and the
    `id` and `type` of every entry): the rewrite is several edits, one of each entry and one of the
    root, and none of the six variants names several, so the contract's section 3.1 gains a seventh,
    which is additive. The record is placed on the root; with a declaration written, `actual` is
    its name (`sce:codec-format`) and the record stands at it, and with none it reports no `actual`,
    since no text of the row is the value refused. A document in "format 1" that writes `optional<T>`
    on an entry is not rewritten but refused on that entry's type, as
    `validation/attribute-rule-violated` naming the declaration it lacks, because format 1 has no such
    type and a rewrite would have to guess; a `sce:required` that is neither `true` nor `false` in
    format 1 is the `validation/invalid-attribute` it always was, for the same reason. `sce:required`
    stays in `schemas/sce-forge-ext.xsd` and on the roster, so that its refusal in format 2 is by
    the rule that names the two spellings, and not a schema violation or an unknown name. The rule
    "a required parameter needs a required property, except on a repeated property" is stated in
    types: a parameter of a bare type needs a property of a bare type, or a repeated one.
    `sce-codegen codec-format <document>` reads the file as written and prints it with the edits
    applied and every other byte the author's (comments, spacing and the order of attributes
    included); the declaration stands beside `sce:encoding`. A document already in format 2 is printed
    as it is; one that is no CBOR or content-line codec is a usage error; and one that holds an
    `<xi:include>` or `<sce:use>` is refused, because the entries those expand to are no text of the
    document. The page (`sce-codegen pseudo`) writes an entry's type as the model holds it
    (`entry <id>: optional<uint8> key 12`), and its `required` word is gone. The generators read
    `is_required()` into the context key every template already branches on, so no template
    changed and no generated byte moved; Rust, Kotlin, C++ and Python spell an optional entry through
    `LangCtx::codec_optional_of` (the spelling a statechart's variable of the type has), while Go's
    pointer and C11's `<name>_present` stayed until steps 13 and 14 (Go's moved in step 13, below). The
    documents moved by the rewrite,
    by hand, entry by entry: `stdlib/mesh/envelope.scxml` and the five codec fixtures
    `tests/forge/resources/codec_cbor_map.scxml` and `codec_content_line_{event,enums,records,required_lists}.scxml`
    (every entry that had neither spelling became `optional<T>`, and `sce:required="true"` became the
    bare type); the oracle that reads the content-line fixtures
    (`tests/forge/conformance/content_line_model.py`) reads the type, and
    `a_codec_declares_the_format_it_is_written_in` holds that the tool's rewrite of two of them in
    format 1 is the document the repository has, byte for byte. Tests: that file, the codec
    declaration tests (`a_codec_can_write_one_cbor_map`,
    `a_codec_can_read_and_write_one_content_line_component`) in format 2, and the golden records of
    `validation/codec-format-superseded`.
13. **Go codecs on `optional.Of[T]`** (implemented): the CBOR and content-line codecs' pointer fields,
    and the gated positional fields, move to the Go optional type, through `LangCtx::optional_of`. Tests:
    the Go conformance cases, regenerated, unchanged in their bytes.
    As built, `LangCtx::codec_optional_of` answers Go too, so a CBOR entry, a content-line entry or
    parameter (a line record's included) and every field gated by `sce:present-if` are spelled by the
    one function a statechart's variable is, and a value a codec decodes is taken whole by a variable
    of its type with no conversion. A CBOR decode holds each key it reads as an `optional.Of[T]`
    (`<name>Held`), which tells a key given twice and a required key never given from the value, and
    hands a required entry over as `Proven()` once its presence is checked; a content-line decode
    stores a present value with `optional.Some`. The gated positional fields are one type for every
    layout the gate admits: a fixed-width or variable-width number, a text or byte string
    (`optional.Of[string]`, `optional.Of[[]byte]`), a repeat, a TLV chain (`optional.Of[[]T]`, the
    list built in a local and handed over whole once the loop and its guards have accepted it) and
    an embedded codec (`optional.Of[T]`, over the body's value). A gated list or byte string used to
    be the slice's own nilness, which could not tell a list that was on the wire and empty from one
    that was not (a decode of an empty tail was `append([]byte(nil), raw...)`, a nil slice); the type
    tells them apart, and an encode of a present empty one writes its (empty) bytes as the bare
    field does. Presence is `IsPresent()` and a read is `Proven()` only under the test that proves
    it (the encoder's own check of a field gated on `<chain>.has(…)`, a gated length or count read
    where the same gate holds, a gated chain walked where it is present), the same rule section 3
    gives a statechart. The runtime's `optional.Of[T]` gained `MarshalJSON` and `UnmarshalJSON`
    (absent is `null`, section 4), which is how the generated conformance harness holds the
    oracle's decoded cases in the codec's own type and compares them by `Equal`. The bytes did not
    move: the conformance cases are unchanged, and `tests/forge/expected/*.go` (every golden of a
    codec with a gated field) and `backends/go/forge-runtime/round_trip/generated` regenerate.
    Tests: `sce-build/tests/a_go_codec_holds_an_absent_value_as_the_one_optional_type.rs` (what the
    generator writes, for each kind of entry and each layout), `backends/go/forge-runtime/round_trip/optional_field_test.go`
    (a field a decode fills is assigned to a statechart-typed variable and written back from one, a
    present `false` or zero told from absence, for a gated field and a CBOR entry), the JSON test of
    `backends/go/forge-runtime/optional`, and the Go drivers of `forge_conformance.rs` that read
    gated fields (`marker_go_program`, `chain_flag_go_program`, `chain_prev_go_program`).
14. **C11 codecs on `sce_optional_<elem>_t`** (implemented): the same for C11's `<name>_present` members. Tests:
    the C11 conformance cases, unchanged in their bytes.
    As built, an entry that may be absent is the struct `sce/forge/optional.h` declares for its
    element, in a CBOR codec and, in a content-line codec, in an entry that holds one value on one
    line and in a parameter, whether of the codec or a member of a line record. The type and the
    declaration its header makes first are one answer, `LangCtx::c11_codec_optional`, which asks the
    functions the C11 lowering of a statechart asks (`static_lowering::c11_optional_type`,
    `StaticTarget::optional_def`), so a codec's entry and a variable of the same type are spelled and
    declared by one text under one guard, and a program that includes a machine and a codec declares
    each once. A text is `sce_static_string_<N>_t`, the buffer of its bound ended by its
    terminator, with no length beside it; a byte string is `sce_static_bytes_<N>_t`, that buffer and
    its length; an enum is `sce_optional_enum_<document>_t` over the machine's name for the
    document's type, of which the codec declares the alias the machine would
    (`typedef <Pascal>_t <document>_enum_t;`, a repeated typedef of one type being C11's own). A
    bare type keeps what it was, a fixed array beside its length for a text or a byte string, so the
    struct of an entry that is required is unchanged. A C string cannot hold the byte it ends at, so
    decoding an optional CBOR text that carries a U+0000 is refused as a value that does not fit its
    entry (`SCE_FORGE_CODEC_CBOR_OUT_OF_RANGE`) instead of being cut short; a content-line value
    cannot carry one (a control character is refused on every backend). A list, a list of values and
    a line record keep their counts. The generators hand each template the member each entry is
    reached by (`value_ref`, `data_ref`, `len_ref`, `present_ref`), so a template names none by
    spelling `<name>_present`; the conformance harness reads `present` and `value` of what it
    decodes and builds. A positional codec's gated field is not a `<name>_present` member: C11 holds
    it as the plain type, its presence the carrier flag bit the codec's own contract names, and that
    contract is unchanged here; whether an array, a repeat or an embed a flag gates is held as an optional value
    is not decided by this step. Tests: `sce-build/tests/a_c11_codec_holds_what_may_be_absent_in_the_optional_type.rs`,
    `a_c11_codec_entry_is_held_in_the_type_a_statechart_variable_of_its_type_is` in
    `forge::generator`, the C11 conformance harness over `codec_cbor_map` and the content-line
    fixtures, which builds the headers and replays the vectors, and
    `backends/c/forge-runtime/tests/conformance/codec_optional_test.c` (ctest `codec_optional_c`),
    which decodes a map and a component and takes each optional member whole into a variable of the
    runtime's own type, so a codec's entry and a statechart's variable are one type or it does not
    compile.

A gated sibling named by `sce:length-from` of an embedded codec is refused where the codec is read
(`validate_codec_embed_length_from`, `validation/attribute-rule-violated`, on every backend): an
embed's decode reads its length sibling as the number it is, which a gated field is not wherever it is
held as an optional value, and an absent length bounds nothing. Test:
`sce-build/tests/a_codec_embed_is_bounded_by_a_length_every_frame_carries.rs`.

There is no step 15. The step the first accepted text listed there, "the end of the codec
transition", which removed two temporary refusals, is withdrawn: the refusal of format 1 is
permanent (section 8).

Steps 1, 2 and 3 come first and in that order; 4 after 3; the seven of step 5 after 4 and
independently of one another; 6 after the Rust and Kotlin steps of 5; 7 after 6; 8, 9 and 10 after
the backends they lower on; 11 after 9; 12 after every step of 5; 13 after 12 and the Go step; 14
after 12 and the C11 step.

## Alternatives considered

- **A sentinel value** (`-9223372036854775808`, `0`, a far-future day). It is a day: arithmetic on
  it answers a day, a comparison orders it among the days, and nothing in the type tells a reader
  to test it first. The owner's rule refuses it.
- **A companion `bool`** (`hasDue` beside `due`). Two fields that must agree, a `due` that holds a
  day while `hasDue` is false, and a wire, a saved state and a scenario that can carry the
  disagreement. The owner's rule refuses it. (The C11 representation is not this: section 5.)
- **A sum type, an enum whose variants carry values.** It would express this and more, and it has
  no native carrier on Go, Python or C, where it would be a hand-built tagged structure, and none
  on the wire. Every backend and the wire have a native "absent or a value"; the general feature
  is not asked for. If it is ever added, this type is what it must agree with.
- **A date type.** It answers a different question: a date may be absent too, and a day count is
  already the `int64` an application computes with (`sce:std/time`).
- **A per-field `sce:optional="true"`** (ADR 0052's framing). Absence on a field only: a variable
  that copies the field, a parameter it is handed to, a return that answers it would each need the
  attribute again, which is a type spelled as an attribute in every place.
- **Absence as a missing key on the event wire.** A missing key already means "the sender's
  revision does not have this field" (ADR 0044); a field added later with a present default would
  then read two meanings from one delivery. (A codec has no revisions, so a missing key is free to
  mean absent there.)
- **Every type admitting `null`.** A value that may be absent everywhere is a test owed
  everywhere; a bare `T` stays present.
- **An implicit read** (`due + 1` failing when `due` is absent). ADR 0047's reason against an
  implicit conversion holds: one expression would mean a value in one run and a failure in
  another with nothing written that says so.
- **A read that fails at run time** (`x.value` raising `error.execution` through `found` when `x`
  is absent), which the proposal decided. The owner refused it (answer 7): it leaves a run-time
  path the build could have closed, and a statement that may fail where the author could have been
  told when the document was built. A read is admitted where the build has proven presence, and
  `??` is the read for every other place.
- **A read spelled as a builtin** (`value(x)`, `present(x)`). `value` is a name documents already
  give their variables and fields (the shared list's `record_value` among them); a presence
  builtin would be a second spelling of `x !== null`.
- **A transition's `cond` narrowing a variable in its content.** The microstep runs exit actions
  and other transitions' content between the two, any of which may write it; only the payload is
  the same value throughout.
- **A pointer for Go**, which the proposal chose because the Go codecs already use one. The owner
  chose the value struct (answer 2): a pointer aliases, compares by address, allocates, and has a
  zero value only by convention; the codecs move to the struct instead.
- **A pointer for C11.** It needs storage elsewhere, which a C machine without an allocator does
  not have for a value it holds.
- **A widening that moves the saved shape** (the proposal, for a statechart variable). The owner
  refused it (answer 6): every value written before a widening reads after it, so refusing the
  restore would lose a user's state over a change that loses nothing. The shape type keeps the
  shape, and the per-value check refuses exactly the states that cannot be read whole.
- **`optional<enum:Refusal>` for "no refusal"** (section 7). A present record around an absent
  reason holds a row, a field and a command of a refusal that never happened.
- **Keeping `sce:required` beside `optional<T>`.** Two spellings of one thing, with opposite
  defaults in two kinds of document; section 8.
- **Temporary transition refusals for the codecs** (the first acceptance of this ADR): refuse
  `sce:required` and a codec of bare entries only for a while, then remove the refusals. Nothing in
  a document of bare entries only says which spelling it was written in, so once the refusals were
  gone such a document from before the move would be read with every entry required, in silence.
  A format the document declares makes the spelling a fact of the document, and the refusal of
  format 1 never has to end.
- **Inferring the format** (format 1 when any entry writes `sce:required`). It cannot tell a
  format-1 codec of optional entries only from a format-2 codec of required entries only, which is
  the one case that matters.

## Consequences

- A value that may be absent is declared as such, read only where the build has proven it present
  or with a default, and carried on the wire, in a saved state and in a scenario as `null`.
- An application widens a variable, a record's field or a schema field from `T` to `optional<T>` and
  keeps its logs and its users' saved states; it upgrades the machines that receive the event
  before the ones that send absence.
- `null` is no longer admitted in a slot that declares a type, which a document that relied on it
  (if any does) finds out at build time.
- Every committed tree of a machine that holds an optional value is new; the trees of the codecs
  whose optional wrap is folded into `LangCtx::optional_of` regenerate unchanged on Rust, Kotlin,
  C++ and Python, and changed on Go with step 13 and on C11 with step 14.
- Every CBOR and content-line codec document of the repository is rewritten once, to format 2; a
  format-1 document anywhere, built at any time, is refused with its rewrite and never read as
  format 2.
- ADR 0052 decision 2 is superseded; its parents hold `optional<record:Refused>`.

## The owner's answers (2026-10-21)

1. **C11's representation**: accepted as proposed — a struct of a flag and a value, through the
   macros of `sce/forge/optional.h` (section 5).
2. **Go's representation**: a generic value struct in the Go forge runtime, not a pointer; the Go
   codecs adopt it, so Go has one optional representation, and the migration of their pointer
   fields is a step of its own (sections 5 and 8, step 13).
3. **A first-revision optional field whose key a delivery lacks**: refused, as proposed; absence is
   an explicit `null` (section 4).
4. **`record_refusal.none`**: retired now, while the shared list is pre-release; "no refusal" is an
   absent optional. The shape is `optional<record:Refused>`, for the reason section 7 gives; the
   list's saved-shape pin moves with it, in its own step after the backends lower optional values
   (step 11). ADR 0052 records that this supersedes its decision 2.
5. **Codecs**: moved from `sce:required` to `optional<T>`, with every repository codec migrated in
   one step (section 8). The proposal's temporary transition refusals left a document of bare
   entries only able to flip in silence once they were removed; the owner's standing rule (no
   silent flip, ever) decided on 2026-10-21 that a codec document declares its format
   (`sce:codec-format="2"`), that format 2 is the only one read and generated, and that a format-1
   document is refused with its rewrite, permanently, so no step ends a transition.
6. **A widening keeps a saved state restorable**: a statechart variable, or a record field it
   holds, widened from `T` to `optional<T>` keeps the saved shape, and only other changes of type
   move it (section 6). This changed the proposal.
7. **No failing read**: `x.value` only where narrowing proves presence, refused when the document
   is built everywhere else, with `??` as the read with a default and no run-time path for an
   absent read (sections 2 and 3). This changed the proposal.
