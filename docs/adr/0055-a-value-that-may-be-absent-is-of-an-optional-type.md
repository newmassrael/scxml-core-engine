# ADR 0055 — A value that may be absent is of an optional type, and absent is no value of any other

- Status: Accepted (2026-10-21). The owner decided every question the proposal left open ("The
  owner's answers", below), under the standing rule: the most long-term correct structure, cost
  ignored; never a sentinel number and never a companion flag that admits impossible states. Steps
  1 to 6 are implemented; the others follow in the order section 9 gives. Section 8 and step 12 amended the
  same day by that rule: a codec document declares its format, and format 1 is refused for good.
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
- *`<param>`, `<donedata>`, a typed request.* An optional value is carried as section 4 writes it.
  A carrier that is text and not JSON — a BasicHTTP form, the `params` of a host-run `<invoke>`
  with no `sce:request` — has no spelling for absence, so an optional value there is refused at
  the `<param>` (`scxml/static-datamodel-rule`, naming `??`). A `<send>` of an optional value to a
  field of `T` is refused by the rule that holds a send to the field that reads it (§2.15); a value
  of `T` to a field of `optional<T>` is admitted.
- *A child session's `<param>`.* A child's variable of `optional<T>` is not handed a value until a
  step decides the shape of `InvokeParams` (which already spells "not handed" as `None` / `null` /
  `std::optional`, so an optional variable needs a second absence inside the first): refused by
  name at the `<param>` until then.
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
   `AStaticDatamodelRunsGeneratedCppTest`, the Interpreter
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
8. **Algorithms.** Optional parameters, returns and locals (the algorithm reader stops refusing
   them); the adapters' copy of an optional field and of an optional enum crossing by name; section
   3 in a body. Tests: `scenarios/static_state_algorithm_optional.json` (an algorithm that answers
   the earliest due day of a list of records, absent for an empty list or one with none due, using
   a guard and an early return) on every engine.
9. **`optional<record:X>`.** A variable and an algorithm slot of an optional record: started at
   `null`, taken whole from a record, a payload, a list's element; `r.value.f` where proven; a field
   write to `r.value.f` where proven, and refused elsewhere. Tests:
   `scenarios/static_optional_record.json` on every engine, and its saved instance.
10. **The edges left refused by name**: a child session's optional variable as a `<param>` (with
    the shape of `InvokeParams` it needs), a host action's optional argument, a host-run
    `<invoke>`'s typed request and result with optional fields. Each with the scenario or test of
    its edge.
11. **`record_refusal.none` retired** (section 7): the enum, `record_list.scxml`,
    `record_list_parent.scxml` holding `optional<record:Refused>`, `scenarios/record_list_parent.json`
    and the Go, C and C++ drivers that read the refusal by hand, the pin in
    `standard_saved_shapes.json`, docs/SCE_ACCEPTED_SUBSET.md "The standard shared record list".
    Test: the scenario's first step expecting `refused` to be `null`.
12. **Codec format 2** (section 8): `sce:codec-format` in the grammar (`schemas/sce-forge-ext.xsd`)
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
13. **Go codecs on `optional.Of[T]`**: the CBOR and content-line codecs' pointer fields, and the
    gated positional fields, move to the Go optional type, through `LangCtx::optional_of`. Tests:
    the Go conformance cases, regenerated, unchanged in their bytes.
14. **C11 codecs on `sce_optional_<elem>_t`**: the same for C11's `<name>_present` members. Tests:
    the C11 conformance cases, unchanged in their bytes.

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
  C++ and Python, and change on Go and C11 with steps 13 and 14.
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
