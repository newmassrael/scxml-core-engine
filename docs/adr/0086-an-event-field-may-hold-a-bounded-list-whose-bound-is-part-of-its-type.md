# ADR 0086 — An event's field may hold a bounded list of any type, and every composed type has a size the build knows

- Status: Accepted (2026-11-03). Written at the owner's order of that day as landing 1 of ADR 0085 (its open
  question 1, recommended there as an ADR of its own), under the rule of 2026-10-26 and 2026-10-27: the most
  long-term correct structure, textbook in its foundations, cost ignored. The first draft restricted what a list may
  hold and refused nesting. On review the owner replaced that with one orthogonal type system, a machine's event slot
  sized by the build and signed values in deterministic CBOR; on a second review, the same day, with every text and
  byte string bounded without exception and one notation for every bound, written in the type. The owner accepted
  the result the same day ("Owner's answers"). Amended after acceptance on 2026-11-04, at the owner's order: every
  place of a Rust machine holds its bound in its Rust type (section 7.4, answer G, landing step 4). Amended again
  on 2026-11-06, at the owner's order: a list states its lower count in its type, beside its bound, so "every
  bound, written in the type" covers the fewest elements too (section 15, answer H, landing step 9). An amendment
  of section 8 is proposed on 2026-11-08 and awaits the owner: who owns an event's data in a C11 machine and its
  runtime, and the inputs of the slot section 8.1 does not list (section 8.5).
- Date: 2026-11-03; amended 2026-11-04 and 2026-11-06; amendment proposed 2026-11-08
- Amends:
  - ADR 0087 section 1.1: the JSON Schema of a list that states a lower count carries `minItems`, and its GBNF
    grammar leaves the shorter lists out (section 15.9, 15.10); section 2.3: a fourteenth refusal code, `tooFew`
    (section 15.11);
  - ADR 0042 section 2: its sentence "an event cannot carry a list" stops being true (section 12);
  - ADR 0077 section 7: a bound "absent where the protocol states none" stays a reading of a codec's frame, which
    describes a protocol SCE does not own; a value type of the language (an event schema's field, a variable, an
    algorithm's slot) states every bound, and one that does not is refused (section 1.4);
  - ADR 0085 section 5: a C11 machine's slot is sized by the build from the events it takes, and
    `SCE_MAX_DATA_LEN` is retired (section 8), in place of an assertion against that knob;
  - ADR 0085 section 6: the bound is written in the type (`list<record:Op>[shared_parts=64]`, section 1.2), not
    as `sce:capacity`; the Mesh carries the list inside the JSON text of
    `_event.data`, not as a CBOR array; a list a later revision added takes no `sce:default`; and a list enters the
    signed bytes as deterministic CBOR (section 9), not as "its length and then each element's fields";
  - ADR 0076: a `<foreach>` over a list whose bound is declared has its passes counted (section 6.3);
  - ADR 0055: `optional<T>` may stand inside a list and a list inside an optional;
  - SCE_FORGE.md section 4.12 (the list rule and the record rule) and docs/SCE_ACCEPTED_SUBSET.md section 2.15
    ("List variables"): a list's element is any bounded type, and a record may hold any bounded type, a list
    included; a payload's text is held to its bound when the payload is read, not only when it is assigned
    (section 4.4).
- Scope:
  - the forge model: one value type for every position, which replaces `FieldType`'s scalar-or-optional, the
    `ListElemType` and `OptionalElemType` element sets, and gives a codec entry a type of its own;
  - `sce-build`'s readers (`forge/parser.rs`), revision rules (`forge/schema_revision.rs`), statechart typing, send
    rules, internal-queue bound, and the generator's schema units, lifts, send lowerings and C11 slot sizing;
  - the payload readers and writers of the eight runtimes: Rust, Go, Kotlin, Python, C++, Swift, C11, and the
    Interpreter's `sce_static.js`;
  - the C11 runtime and templates that hold `_event.data` in `SCE_MAX_DATA_LEN` bytes;
  - a deterministic CBOR encoder of typed values, with vectors, in `sce-rust-trust` (used for signing from ADR 0085
    landing 5 on);
  - `schemas/sce-forge-ext.xsd`, `apis/forge-ast.v1.schema.json`, the replay-scenario and scenario-set
    vocabularies; SCE_FORGE.md section 4.12, docs/SCE_ACCEPTED_SUBSET.md, ARCHITECTURE.md ("JSON Object Key
    Order");
  - a new diagnostic code and fix (section 11) in `sce-build/src/forge/diagnostic.rs`, SCE_ERROR_CONTRACT.md and
    the acceptance-doc appendix; a `sce-codegen bound-spelling` printer;
  - every committed document that writes a bound as an attribute (rewritten into the type) and the five committed
    event schemas with an unbounded text (section 1.4).
- Related: ADR 0005 (bound and declare), ADR 0044 (the revisions of a schema), ADR 0047 (the number a variant
  declares), ADR 0050 (`sce:limit`), ADR 0055 (optional values), ADR 0065 (a codec is a total printer and a partial
  parser), ADR 0077 (a declared bound is part of a field's type), ADR 0085 (a change is one signed unit).

## Context

ADR 0085 sends a change of several operations as one event, `record_list.change`, whose `ops` is an ordered,
bounded list of records of a declared schema. An event schema cannot declare such a field today.

**What a field may be.** An event schema's field is typed by `FieldType`: a scalar or an optional scalar
(`sce-build/src/forge/model.rs:4821-4848`, "Neither has record or list members"). Its reader,
`read_schema_field_type` (`parser.rs:12120-12160`), refuses everything else, `list<…>` included. The same reader and
type serve the entries of a CBOR or content-line codec (`parser.rs:2023`, `2341`). A content-line codec has a list of
its own, spelled `sce:repeated="true"` with `sce:max-count` (`parser.rs:2471-2474`).

**Lists exist elsewhere, with a narrow element.** An algorithm's slot (`AlgorithmValueType::List`,
`model.rs:4605-4616`) and a `sce-static` variable (docs/SCE_ACCEPTED_SUBSET.md, "List variables") may be `list<T>`.
`T` is one of `ListElemType` (`model.rs:4929`): a fixed-width number or `bool` (`list_elem_admitted`,
`model.rs:4687-4702`), or a record. A string or bytes element is refused, "a bounded list of them has no size to
declare" (`parser.rs:12061`), and so are an enum, an optional and a list. Yet a record may hold a bounded string
(SCE_FORGE.md section 4.12), so `list<record:R>` with one text field holds what `list<string>` may not. What may
stand inside an optional is a third set (`OptionalElemType`, `model.rs:4770`). The three sets grew one
position at a time, and none follows from the others.

**Each backend holds a list in its own type**: Rust `Vec<R>`, Kotlin `List<R>`, C++ `std::vector<R>`, Go `[]R`
(`static_lowering.rs:3406`, `2493`, `7237`, `8434`), and C11 `{ size_t len; R data[N]; }` (`bounded_record_list`,
`static_lowering.rs:10810`).

**The wire already carries arrays.** `_event.data` is a JSON object (Rust `PayloadFields::decode`,
`backends/rust/runtime/src/event_payload.rs:76-100`). Its keys are ordered by UTF-8 bytes "at every depth", an
array keeps its order, and a `<param>` name that repeats collects its values into an array (ARCHITECTURE.md:826-835).
C11's reader can already step past a nested array (`sce_payload_skip_container`,
`backends/c/runtime/include/sce/event_payload.h:121`). A 64-bit integer travels as the text of its digits
(`event_payload.rs:204-240`).

**Nothing reads or writes one.** Every runtime's reader looks a field up by name in a flat object and reads one
scalar: Rust `PayloadFields`, Go `LiftPayload` (`backends/go/runtime/event_payload.go:43`), Kotlin
`EventPayload.decode` (`EventPayload.kt:241`), Python `_decode`/`_as` (`event_payload.py:148`, `189`), C++
`EventPayloadFields` (`sce/include/common/EventPayloadLift.h:56`), Swift `PayloadFields` (`EventPayload.swift:46`),
C11 `sce_payload_*`, and the Interpreter's `SceStatic.field` (`sce-build/src/forge/static_js/sce_static.js:215`). A
`<param>` value needs a wire slot, and a list has none (`forge/types.rs:688-700`; refused at
`static_lowering.rs:14234-14236`).

**Where a bound is held, and where it is not.** A schema's bounded text is not held to its bound when the payload
is lifted (`PayloadFields::text`, `event_payload.rs:303-308`), only when an assignment takes it into a variable.
Rust's `HeapList` treats `N` as advisory (`backends/rust/portable-bytes/src/lib.rs:53-71`). A schema's text or byte
string may declare no bound at all: of the 174 committed event-schema documents, five do
(`sce-build/tests/fixtures/event_schema/schema_job_completed_multi.scxml:16`, `job_id`;
`event_schema/schema_on_builtin_error.scxml:14`, `reason`; `host_processor/host_invoker_perm_request.scxml:15`,
`scope`; `no_std/schema_portable_probe.scxml:18`, `label`; `static_datamodel/schema_echo.scxml:17`, `tag`), and so
do inline documents in five tests of `sce-build/tests` (section 1.4).

**A bound has five spellings.** A schema's text writes `sce:max-size`, a variable's list or text `sce:capacity`, an
algorithm's local `capacity=` and its return `returns-max-size=`, each with `sce:limit` beside it; a content-line
list writes `sce:max-count`, and ADR 0085 section 6 proposed `sce:capacity` for an event field. Each is an
attribute of the declaration, so it can bound only the outermost type, and a type nested inside another has nowhere
to write its bound.

**C11 holds every delivery in one global slot.** `SCE_MAX_DATA_LEN` (256 by default,
`backends/c/runtime/include/sce/types.h:76`) sizes the data of every event record of every machine in a build, of
the scheduled sends, of the host processor's buffers and of the autoforward carrier (`types.h:369`;
`host_processor.h:157`, `629`, `641`, `893`; `tools/codegen/templates/c/state_machine.h.jinja2:375`, `468`,
`524-536`, `575-579`). A send that does not fit is refused (`sce_forge_wire_end`, `state_machine.c.jinja2:131-137`);
a copy into the slot cuts the text short (`types.h:183-194`, `330`). The standard host asks a build to raise the knob
by hand (`backends/c/host-features/include/sce/standard_host.h:18-20`). The number is a guess the build makes for
every machine at once, while the generator knows, for a bounded schema, the exact largest text.

**Around the payload.** The Mesh carries `_event.data` as its JSON text (`PayloadCodec::Json`); CBOR and typed
payloads are refused when they arrive (`backends/rust/mesh/src/router.rs:670-685`). A saved state keeps a waiting
send's data as text (`schemas/sce-saved-state.v2.schema.json:181`, `240`). The trust layer signs typed values, not
the JSON text, in a scheme of its own: each value as decimal or UTF-8 text with its length in front, in an order a
hand-written table gives (`backends/rust/trust/src/message.rs:4-23`, `48-53`, `355-421`). The replay vocabulary
types a variable's list of records (`Holds::List`, `sce-build/src/scenario_replay.rs:722-790`) and every payload
field as a present scalar (`Holds::of_schema`, `scenario_replay.rs:772`). The scenario set refuses "a list as a
payload value" (`sce-build/src/scenario_set.rs:1266`). The internal-queue bound marks a `<foreach>` whose body
raises as not proven (`sce-build/src/internal_queue_bound.rs:479-492`).

**Why ADR 0042 said no.** Section 2 reads "a parent statechart cannot [read `values`], and an event cannot carry a
list", so the list tells its parent one value per event. That is a limit of the language, not of the protocol.

## The textbook structure

**Orthogonality.** A language is orthogonal when a small set of type constructors combines in every way, each
combination meaning what its parts mean (van Wijngaarden et al., *Revised Report on the Algorithmic Language
Algol 68*, 1975; the report's design states orthogonality as its principle). Restricting a constructor's argument
to some types is a non-orthogonality a programmer has to learn case by case, and it is how `list<record:R>` came to
hold a text that `list<string>` may not.

**A bound composes.** ASN.1 is the precedent for a bounded, composable interface type: `SEQUENCE (SIZE (0..N)) OF
T` for any `T`, `UTF8String (SIZE (0..N))`, `OCTET STRING (SIZE (0..N))`, a `SEQUENCE` of any of them, and so on
to any depth (ITU-T X.680, its SEQUENCE OF, string and size-constraint clauses). Its packed encoding uses the
constraints to choose the size of every length it encodes (ITU-T X.691). XDR does the same with `T x<N>` and
`string s<N>` (RFC 4506 sections 4.10-4.13), and
CDDL with occurrences (RFC 8610 section 3.2). A bounded list of `T` is the refinement `{ xs : [T] | len xs <= N }`
(Rondon, Kawaguchi and Jhala, "Liquid Types", PLDI 2008), which ADR 0077 applies to a byte string.

**A finite type has a finite size.** If every leaf type has a largest encoding and every constructor's largest
encoding is a function of its parts' largest encodings, the largest encoding of any type is computed by structural
recursion over the type. The recursion ends because a type is a finite tree: a type that contains itself has no
largest value and is refused.

**Parse, don't validate.** A decoder accepts exactly the values of its type and refuses the rest before it stores
or allocates anything (King, "Parse, don't validate", 2019; ADR 0077 section 2).

**Order.** A JSON array is "an ordered sequence" (RFC 8259 section 5); a CBOR array keeps its order too (RFC 8949
section 3.1). `[a, b]` and `[b, a]` are two values in every encoding.

**Sign one encoding of the abstract value.** A distinguished encoding writes each abstract value as exactly one
string of bytes (ITU-T X.690, DER). CBOR's core deterministic encoding (RFC 8949 section 4.2.1) is that for CBOR:
preferred serialization (the shortest head of every integer and length, the shortest float that keeps the value),
definite lengths, and map keys sorted by the bytes of their encodings. A signer that writes deterministic CBOR needs
no scheme of its own, and any CBOR library on any platform can check it.

**Adding a repeated field is compatible when absent means empty** (proto3's repeated fields; Avro's reader-side
defaults). A list a later revision adds reads as empty from an earlier delivery.

## Decision

**The language has one value type, and every constructor takes every type: `list<T>` and `optional<T>` over any
type, and a record over fields of any type. Every text, byte string and list states its bound, without exception,
in the type itself and at every depth, so every type has a largest encoding the build computes. An event schema's
field may be any such type. Every reader refuses a value outside its type before it stores it, naming the path to
the place; no sender can write one. A C11 machine's event slot is always computed by the build from the events it
can receive, and `SCE_MAX_DATA_LEN` is retired. A signed value is encoded as deterministic CBOR; this landing adds
the encoder, and ADR 0085's domain bump starts signing with it.**

### 1. One type grammar

1. **The types.** One grammar, read by one reader, for every position that declares a value: an event schema's
   field, a `sce-static` variable, an algorithm's parameter, return and local.

   ```
   T ::= uint8 | uint16 | uint32 | uint64 | int8 | int16 | int32 | int64 | float32 | float64 | bool
       | enum:<alias>
       | string[B] | bytes[B]
       | optional<T>
       | record:<alias>
       | list<T>[B] | list<T>[M..B]
   B ::= <positive integer> | <limit name>=<positive integer>
   M ::= <whole number of zero or more, at most the number of B>
   ```

   The lower count `M..` of a list was added by the amendment of 2026-11-06 (section 15); `list<T>[B]` is
   `list<T>[0..B]`.

   A record is the struct an imported event schema declares, whatever its fields are. A schema that holds a list, an
   optional, a byte string or another record is a record type like any other.
2. **One notation for every bound: in the type, at every depth.** A bound is written in brackets after the type it
   bounds, the outermost included: `sce:type="list&lt;list&lt;string[8]&gt;[4]&gt;[3]"` is at most 3 lists of at
   most 4 texts of at most 8 bytes, and `sce:type="string[merge_text=96]"` is a text of at most 96 bytes whose bound
   a build moves with `--limit merge_text=N` (ADR 0050). The same spelling serves an event schema's field, a
   variable and an algorithm's slot: `<sce:var type="list&lt;int64&gt;[16]"/>`, `<sce:return
   type="bytes[64]"/>`. A type is then a value that carries everything a reader needs, the way ASN.1's `SEQUENCE
   (SIZE (0..N)) OF T` and XDR's `T x<N>` carry their bounds, and a bound cannot be written where nothing reads it.
3. **The attributes are legacy synonyms of the outermost bound.** `sce:max-size` (a text or byte string),
   `sce:max-count` (a list), `sce:capacity` (a variable's list, text or byte string), and an algorithm's
   `capacity=` (a local) and `returns-max-size=` (a return), each with `sce:limit` beside it, are read in this landing
   as the bracket on the outermost type they bound, with a diagnostic that says so and carries the rewrite
   (`validation/bound-attribute-superseded`, section 11). They are refused in the next revision of the authoring
   grammar (SCE_WIRE_CONTRACTS.md, the "Authoring grammar" row). Both forms on one declaration (a bracket on the
   outermost type and an attribute that bounds it) are refused as `validation/incompatible-attributes`, even when the
   two numbers agree: one bound, one spelling. The attributes that are not bounds of a value type are untouched: a
   binary codec field's `sce:max-size` and a `<sce:repeat max-count>` describe a foreign frame's layout (ADR 0077),
   a content-line entry's `sce:max-count` beside `sce:repeated` belongs to a codec, and `sce:capacity` on
   `<scxml>` is the depth of the event queue.
4. **Every text and byte string is bounded, without exception.** A `string` or `bytes` without a bound is refused
   wherever it stands, an event schema's top-level field included (`validation/attribute-rule-violated` on the
   `sce:type`, naming the place in the type that lacks it). A type is a set of values with a largest member, or it
   is not a type of this language. ADR 0077 section 7 is amended to say so: a bound "absent where the protocol states
   none" remains a reading of a codec's frame, which describes a protocol SCE does not own (a peer's Zenoh frame
   keeps no bound the peer does not state), and not of a value type, whose protocol SCE writes.

   **Migration of the committed schemas.** Each unbounded field gets a bound at least the longest value any test or
   scenario of the tree sends it, so no test value changes. The values seen while writing this are at most 21 bytes
   (`TRANSPORT_UNAVAILABLE`); the landing measures each field's longest value before it sets the bound, and raises
   the number below where one is longer:

   | Document | Field | Bound |
   |---|---|---|
   | `sce-build/tests/fixtures/event_schema/schema_job_completed_multi.scxml:16` | `job_id` | `string[64]` |
   | `sce-build/tests/fixtures/event_schema/schema_on_builtin_error.scxml:14` | `reason` | `string[64]` |
   | `sce-build/tests/fixtures/host_processor/host_invoker_perm_request.scxml:15` | `scope` | `string[64]` |
   | `sce-build/tests/fixtures/no_std/schema_portable_probe.scxml:18` | `label` | `string[32]` |
   | `sce-build/tests/fixtures/static_datamodel/schema_echo.scxml:17` | `tag` | `string[32]` |

   `schema_on_builtin_error` exists to be refused for its event name; it gains the bound so that the refusal it pins
   is still the first one met. `schema_portable_probe` pins the `no_std` text's storage; the test that reads it moves
   from the profile's default capacity to the declared bound. The inline documents of five tests
   (`a_host_run_result_may_hold_a_field_that_may_be_absent.rs`, `a_static_datamodel_holds_every_variable_to_a_type.rs`,
   `a_typed_host_invoke_is_held_to_its_schemas.rs`, `an_algorithm_rule_is_refused_at_the_attribute_that_breaks_it.rs`,
   `event_schema_bytes_guard.rs`) gain a bound the same way, except where a test's subject is the unbounded field
   itself: that test now asserts the refusal of item 4. The generated trees that hold `schema_echo` (Rust, Go, Kotlin
   and Swift `static_datamodel`) are regenerated. An application's own schemas move the same way, told by the
   refusal which field to bound.
5. **What stays refused, and why each is principle rather than convenience.**
   - `optional<optional<T>>`: JSON's `null` and CBOR's `null` are one value, so two absences cannot be told apart on
     any wire (ADR 0055's rule, kept).
   - A type that contains itself, directly or through records: it has no largest value (section 2.1). Refused at
     import resolution as `validation/cross-kind-circular-dependency`.
   Nothing else is refused by its shape.
6. **The model.** One enum, `ValueType`, with a constructor per line of the grammar; a bound is a field of the
   constructor it bounds (`List { elem: Box<ValueType>, max: Bound }`, `Text { max: Bound }`), so the bound is part
   of the type in the code, `schema_revision::divergence` compares it, and the AST export carries it. `FieldType`,
   `AlgorithmValueType`, `ListElemType` and `OptionalElemType` are replaced by it; a position that does not yet lower
   a composition says so where it lowers it (section 7.3), never in the grammar.
7. **A codec entry keeps a type of its own.** A CBOR or content-line codec describes a foreign protocol's frame, not
   a value of the language; its entries move from `FieldType` to `EntryType` (a scalar or an optional scalar), and
   its list stays `sce:repeated`.
8. **An event schema may import an event schema** (`<sce:import kind="event-schema">`) to name a record anywhere in
   a field's type.

### 2. The size calculus

1. **Three sizes, one recursion.** For every type the generator computes, by structural recursion, the largest
   JSON text `J(T)` of a value, the largest deterministic CBOR encoding `C(T)`, and, for C11, the storage the static
   layout takes (section 7.2). A type that contains itself is refused before the recursion runs, and every other type
   is a finite tree, so the recursion ends. `h(n)` is the length of a CBOR head for the argument `n`: 1 below 24, 2
   below 2^8, 3 below 2^16, 5 below 2^32, 9 otherwise.

   | Type | `J(T)`, bytes of JSON | `C(T)`, bytes of deterministic CBOR |
   |---|---|---|
   | `uint8`, `uint16`, `uint32` | 3, 5, 10 | 2, 3, 5 |
   | `int8`, `int16`, `int32` | 4, 6, 11 | 2, 3, 5 |
   | `uint64`, `int64` | 22 (the quoted digits; `-9223372036854775808` is 20) | 9 |
   | `float32`, `float64` | 25, 25 (a single crosses as the double it widens to; ECMAScript's spelling, ARCHITECTURE.md "JSON Number Text", is longest as `-0.00000` and seventeen digits, in the band from 1e-6 to 1e-5) | 5, 9 |
   | `bool` | 5 | 1 |
   | `enum:E` | 2 + the longest escaped name of `E` | a closed `E`: the greatest h over its declared numbers, h(n) for n of zero or more and h(-1 - n) below zero (section 9.2); an open `E` (`sce:strict-variants="false"`): `C` of its underlying type |
   | `string[B]` | 2 + 6B (every byte at its worst escape, `\u00XX`) | h(B) + B |
   | `bytes[B]` | 2 + 6B (the Latin-1 text the wire spells) | h(B) + B |
   | `optional<T>` | max(4, `J(T)`) | max(1, `C(T)`) |
   | `record:R` with fields f1..fn | 2 + Σ (`J(quoted id)` + 1 + `J(Tf)`) + (n - 1) | h(n) + Σ (`C(id)` + `C(Tf)`) |
   | `list<T>[B]` | 2 + B·`J(T)` + (B - 1) | h(B) + B·`C(T)` |

2. **A schema unit names its sizes.** A unit already declares its event, its revision and each text's bound
   (docs/SCE_ACCEPTED_SUBSET.md, "The unit names its event and its revision"). It now also declares each outer
   bound (`<FIELD>_MAX_COUNT`, `<FIELD>_MAX_SIZE`) and `DATA_MAX_LEN`, `J` of the payload as a record of its fields.
   One function of the generator computes each size, and every reader of a size calls it.
3. **The sizes are tight.** For each composition the conformance fixtures declare, a maximal value (every list full,
   every text at its bound of control characters, every number at its longest spelling) is written by each runtime's
   writer, and its length is exactly `J` (`C` for the CBOR encoder). A size that is only an upper bound would waste a
   slot and hide a writer that spells a value longer than the rule.

### 3. The wire form

1. **JSON, as today.** A value is spelled by its type, at every depth: a 64-bit integer as the text of its digits,
   another number as a JSON number, a truth value as `true` or `false`, an enum as its variant's declared name, a text
   as a JSON string, a byte string as the Latin-1 text of its bytes, an absent optional as `null`, a record as an
   object with its keys ascending by UTF-8 bytes (ARCHITECTURE.md, "at every depth"), a list as an array in its
   order. The empty list is `[]`, and a writer of the revision that declares a field always writes its key.
2. **The Mesh carries it unchanged**, inside the JSON text under `PayloadCodec::Json` (`router.rs:670-685`). The
   envelope does not change; ADR 0085 section 6's "CBOR array" is corrected. A saved state keeps a waiting send's
   data as text, unchanged.
3. **Signing is not the wire.** The JSON text is what travels and what a host may rewrite; the signed bytes are the
   deterministic CBOR of the typed value (section 9). The two orders of a record's keys differ on purpose: JSON orders
   by the key's bytes (ARCHITECTURE.md), deterministic CBOR by the bytes of the encoded key, which puts a shorter key
   first. Each follows its own standard.

### 4. Validation where a delivery is read

1. **Every lift refuses before it stores**, at every depth, a value outside its type:
   - a list that is not an array, or longer than its bound: elements are counted and the walk stops at the first one
     past the bound, before decoding it (ADR 0077 section 2);
   - a text or byte string past its bound; a number outside its width, or not in its spelling; an enum name its enum
     does not declare (in a closed enum); a record that is not an object or lacks a field of its schema (one an
     element schema's later revision added is read from that schema's defaults, section 10); `null` where the type is
     not optional.

   A key a record's schema does not declare is stepped over, as one in the payload is (`event_payload.rs:63-65`).
2. **The delivery is refused whole**, the transition does not run and `error.execution` is raised once, as for any
   payload that does not fit (`scenarios/static_whole_payload.json`). The reason names the path through every depth
   as a JSON Pointer (RFC 6901): `'items' holds 4 elements, past the 3 its schema declares`,
   `'items/2/tags/0' is 9 bytes, past the 8 its type declares`, `'items/1/due' is null, and its type is not
   optional`.
3. **One reader shape per runtime.** Each runtime's scalar readers become readers of one value at one path, and
   three combinators walk a list, an optional and a record, each calling the reader of its part. The generator writes
   each field's reader by walking its type, one call per constructor, so a composition of any depth is read by the
   same few functions: Rust `PayloadFields`, Go `LiftPayload`, Kotlin `EventPayload.Fields`, Python `_decode`/`_as`,
   C++ `EventPayloadFields`, Swift `PayloadFields`, C11 `sce_payload_*` (a cursor over an array and a
   `sce_payload_fields_t` over an object, header-only, no allocation, one static sentence per refusal as the header
   requires; the path is composed by the caller from generation-time literals and the indices), and the
   Interpreter's `SceStatic.field`, which takes a recursive type descriptor (`{list: T, max: B}`,
   `{optional: T}`, `{record: {...}}`, `{string: B}`).
4. **A top-level text is held at the lift too.** Every bounded text and byte string is held to its bound wherever
   it stands, the top level included, so one payload follows one rule (ADR 0077 section 2). SCE_FORGE.md section
   4.12's "judged against its bound only by the assignment" is amended.

   **Amended 2026-11-09: the rule holds on every lane.** The walk over a composed type held every bound, but the
   lift of a schema of entries (each field a scalar, a text, a byte string or an optional of one) held a text to no
   bound on any lane, and a byte string to none but on Rust and C11, whose buffer is the bound. Each lane's lift now
   reads a text and a byte string at the top level by the reader that reads one inside a composition, with the bound
   its type declares (`lift_text_bound` and `lift_bytes_bound` in `sce-build/src/forge/generator.rs`), and a present
   value of an optional text or byte string the same way:

   | Lane | A text of `string[B]` | A byte string of `bytes[B]` |
   |---|---|---|
   | Rust | `PayloadFields::text(name, B)`, held by `PayloadValue::text_within`, which `text::<N>` calls | `PayloadFields::bytes::<B>`, the capacity of `SceBytes<B>` |
   | Go | `PayloadTextWithin(fields, name, B)`, held by `PayloadValue.Text` | `PayloadBytes(fields, name, B)`, held by `PayloadValue.Bytes` |
   | Kotlin | `Fields.string(name, B)`, held by `Value.string` | `Fields.bytes(name, B)`, held by `Value.bytes` |
   | Python | `Text(B)` in the lift's field, read by `read_value` | `Bytes(B)`, read by `read_value` |
   | C++ | `EventPayloadFields::value`, then `EventPayloadValue::readText(B, …)` | `EventPayloadValue::readBytes(B, …)` |
   | C11 | `sce_payload_field`, then `sce_payload_value_text(…, B)` into a buffer of `B + 1` bytes | `sce_payload_value_bytes(…, B, …)` |
   | Swift | `PayloadFields.text(_:bound:)`, held by `PayloadValue.text(bound:)` | `PayloadFields.bytes(_:capacity:)` |
   | Interpreter | `SceStatic.field(_event.data, name, {string: B})` | `SceStatic.field(_event.data, name, {bytes: B})` |

   A number was held to its width, and a list to both its counts and every bound inside it, on every lane already.
   `scenarios/static_record_string.json` takes a label past its bound into a wider variable and
   `scenarios/static_payload_bytes.json` measures a frame past its bound: each is refused whole on every lane that
   replays them, and `scenarios/static_union.json` refuses a list short of its lower count, one past its bound and a
   text past its bound inside a case. The composed payload of each lane's `a_composed_payload_is_read_at_any_depth`
   test is refused whole for a nested text, a nested byte string, the top-level text and the top-level count.

   The Interpreter reads a delivery whole at its door too. The lowering installs, beside the library, the record of
   each event whose payload the document reads (`SceStatic.door`), and the Interpreter, when it takes an event and
   before it selects a transition (`StateMachine::processTakenEvent`), asks the library's reader of it, which reads
   every field as `SceStatic.field` reads one and answers the sentence of the first refusal. A delivery so refused is
   replaced by the `error.execution` that carries the sentence, as on every lane (amended the same day, below). A
   field past its bound refuses the delivery even on a route that reads none of it
   (`scenarios/static_payload_bytes.json`, a sensor of 40 to 49).

   The door covers a composed payload too (amended 2026-11-09). An event whose schema holds a list, a record or an
   optional value of either is installed with the descriptor of its schema's record and marked composed
   (`SceStatic.door(<event>, <type>, true)`): its door reads the data whole, as `SceStatic.payload` does, refuses a
   delivery outside its type before any transition is selected, and keeps the value it read for the event being
   processed. Every expression of the content reads that value through `SceStatic.taken(<event>, _event.data)`, so a
   delivery is walked once, as a generated machine lifts it once; one that arrived under a longer name the
   transition's descriptor matches, which no door of that name took, is read there whole instead. The read that
   used to open the content of each transition on such an event (`leading_read_edits`), which refused inside the
   transition and so let the transition be taken, is removed with it
   (`AComposedDeliveryOutsideItsTypeTakesNoTransition`, on `static_composed_payload`).

   The shared record list (ADR 0046) follows: an edit whose text is past `shared_typed_text` is refused where it is
   read, as every delivery outside its type is, and the list's own check of the text (`typed`, of the same bound),
   which no delivery could reach any more, is retired with the variant `textTooLong` (ADR 0046, section 6, amended
   2026-11-09). A statechart cannot send such an edit (ADR 0050), and a host that does is answered `error.execution`;
   `scenarios/record_list_parent.json` states it.

   A call that hands an algorithm `_event.data` whose schema holds text stays refused, for its real reason: the value
   fits now, and the Rust payload struct of a schema of entries still holds the text in the engine's own text type,
   where the algorithm's record holds `SceString<N>`, so the copy would be a conversion that can fail in its type. It
   is lifted when that struct holds the field's bounded text, as a composed payload's struct does (section 7.4).

5. **Amended 2026-11-09: a refused delivery is replaced at the door, on every lane.** Item 2 says a delivery the
   door refuses is refused whole: its transition does not run, and `error.execution` stands in its place. No lane did
   that until this amendment. Each lifted the payload where it took the event, raised `error.execution` into the
   internal queue when the lift refused, and then selected transitions for the refused event with no payload bound:
   a guard that reads the payload was false, content that reads it was skipped, and a transition that reads none of
   it was taken, its target entered. The generator recorded the reason beside every lift: a script engine given the
   same guard on the same data raises `error.execution` and treats the guard as false (measured 2026-09-22), and a
   native lowering that answered otherwise would make the optimisation observable. The Interpreter's door, installed
   earlier the same day, kept that behaviour by rewriting each guard that reads the payload into
   `SceStatic.admitted() && (…)` and each such content into `<if cond="SceStatic.admitted()">`.

   That measurement is superseded. It compared the typed lift with a reader that holds no type, so the one answer it
   could match was the untyped one. An event schema is a contract at the boundary: an event that fails its type is
   not that event, and validating at the boundary means it is never delivered. Taking its transition anyway let a
   delivery the document cannot read move the machine (a transition that reads none of the payload, or a target
   whose content was skipped), which is what item 2 forbids. The owner's standing rule decides it: the rule of this
   ADR, not the old measurement, is what every lane implements.

   On every lane the door is where the event is taken off its queue, and the replacement happens there:

   | Lane | The door | What it answers |
   |---|---|---|
   | Rust | `Engine::bind_lifted_payload`, asked by `bind_queued` | `Admission::{Admitted, Replaced, Refused}` |
   | Go | `Engine.liftTypedPayload` | the event taken and `admitted`, `inPlace` or `refusedWhole` |
   | Kotlin | `StateMachineEngine.bindTypedPayload` | the event taken, or `null` |
   | Python | `Engine._admit`, over the policy's `lift_event_payload` (the lift moved out of `set_current_event`) | the event taken and whether it is the replacement, or `None` |
   | C++ | `StaticExecutionEngine::bindTypedPayload` | `Admission::{Admitted, InPlace, Refused}` |
   | C11 | `<sm>_lift_payload`, which rewrites the dequeued event in place | `<SM>_DOOR_ADMITTED`, `_IN_PLACE` or `_REFUSED` |
   | Swift | `Engine.liftPayload` | the event taken and whether it is the replacement, or `nil` |
   | Interpreter | `StateMachine::processTakenEvent`, asking `__sceDequeued` | the refusal's sentence, or the empty text |

   - The replacement is the platform error `error.execution` whose data is the refusal's sentence (`` `<event>`
     payload: <reason> ``), bound as `_event` with no typed payload. No transition is selected for the refused
     delivery.
   - It takes the refused delivery's position: it is answered at once, in that delivery's turn. Taken off the
     external queue, it is the engine's own error all the same: no `<finalize>` or autoforward runs for it, an error a
     handler raises while answering it is a link in its chain, and one no transition matches is counted as an
     unhandled error, not as a discarded external event.
   - A generated machine whose document declares no `error.execution` has no event to put in the delivery's place:
     the delivery is refused and nothing is taken. It is not dropped unseen. An external delivery so refused is
     counted as a **discarded external event**, and named the last one: the hook every lane already has for an
     external event that moved nothing (`discarded_external_events` and `last_discarded_event` on Rust,
     `DiscardedExternalEvents` and `LastDiscardedEvent` on Go, `discardedExternalEvents()` and `lastDiscardedEvent()`
     on Kotlin, `discarded_external_events` and `last_discarded_event` on Python, `discardedExternalEvents` and
     `lastDiscardedEvent` on Swift and C++, `_discarded_external_events` and `_last_discarded_event` on C11). The
     unhandled-error count cannot serve: it names an `error.*` event, and such a document declares none. The
     Interpreter, which names events by text, takes the error and finds no transition for it, which is its own
     answer to the same question (`TransitionResult::success` false, `getStatistics().failedTransitions`). A
     delivery refused off the internal queue is not counted, as an internal event no transition matches is not:
     both ends of it are in the document.
   - Nothing is added to a queued entry and no saved state changes. Every door is at the dequeue, so a saved queue
     holds a delivery as it arrived, and the restored machine judges it when it takes it, with the same answer. The
     queue never holds a refused delivery's error: the internal queue's bound counts none for the lift any more
     (`sce-build/src/internal_queue_bound.rs`).
   - The Interpreter's `SceStatic.admitted()` and the rewriting that held guards and content to it (`door_edits`) are
     removed: no refused delivery reaches a guard. On the generated lanes the check a guard or content makes that the
     payload bound is its own event's (the tag, `!= null`, `is not None`, `matches!`) stays, for a reason that is not
     the door's: an event of another name that the transition's descriptor also matches (`a` matching `a.b`) carries
     no payload of the transition's event, and a Rust `no_std` build lifts no wire.
   - `scenarios/sync_client.json`, run 9, is the case the old behaviour got wrong: a `sync.page` whose `more` is a
     number is not taken, so the last-page transition, which reads nothing of the payload and was taken before, does
     not run, the listing goes on, and the `error.execution` is counted in its place. Every other step of every
     scenario that refuses a delivery keeps its expectation: each such delivery met only guards that read the payload
     or targetless content that reads it, where the two behaviours agree.

   The rule is the type's, not the data model's. An `ecmascript` document whose guard the generator lowers to a
   typed comparison (NL to IR Item C1), or whose `<sce:action>` reads a payload through its arguments, has the same
   door on the generated lanes, and the lowering for the Interpreter (`sce-codegen lower`) gives it the same door:
   it installs the library and the record of each such event (`install_doors` in
   `sce-build/src/forge/static_js.rs`, in a `<datamodel>` of its own when the document declares none) and changes
   nothing else of the document. The Interpreter then refuses such a delivery whole as every generated machine does
   (`AnEcmascriptDocumentRefusesADeliveryOutsideItsSchemaWhole`, on `event_schema/statechart_lifted.scxml`). A
   document that reads no payload through a schema is lowered as it is.

### 5. Validation where a value is written

1. **The relation "fits".** A value of type `S` may stand where `T` is expected when `S` fits `T`:
   - a number, `bool` or `enum` fits only its own type (an enum: the same enum document or a revision of it, ADR
     0044);
   - `string[a]` fits `string[b]`, and `bytes[a]` fits `bytes[b]`, when `a <= b`;
   - `list<S>[a]` fits `list<T>[b]` when `S` fits `T` and `a <= b`;
   - `optional<S>` fits `optional<T>` when `S` fits `T`; `S` fits `optional<T>` when `S` fits `T`;
   - `record:R` fits `record:Q` when both name one document (a record type is its document, SCE_FORGE.md section
     4.12) or two revisions of one schema.

   Each bound is read after its limit name and `--limit`. This is the rule
   the build already keeps for a text sent to a field (docs/SCE_ACCEPTED_SUBSET.md:3927) and for the type a field
   reads (`:3907`), extended pointwise through the type.
2. **A statechart sends a composed value by its name.** `<param name="ops" expr="staged"/>` names a variable, a
   field of a record variable, or a part of the payload of the event the transition is on (to forward it). It is
   accepted when its type fits the field's; otherwise it is refused as `scxml/static-datamodel-rule`, naming the
   field, its reader and the first place, by path, where the type does not fit. Because the build proved the fit, the
   send checks nothing at run time. A composed value written as an expression that is not a name is refused
   (`expression/unsupported-construct`), as a list read as a value is today.
3. **The lowering.** `wire_param_slot` (`types.rs:688`) gives a composed value a slot, and `lower_wire_value`
   (`static_lowering.rs:14124`) writes it through the runtime's writer by walking its type. Each writer of the
   ARCHITECTURE.md key-order table gains the array and nested-object cases, held to
   `tests/json_text/object_key_order.json` with composed cases. C11's `sce_forge_wire`
   (`backends/c/forge-runtime/include/sce/forge/wire.h`, which closes an array for a repeated name at `:300`) gains
   the opening and closing of an array and of an object at any depth.
4. **A host's injected value is refused where it is made** (ADR 0077 section 3). Rust holds a list in
   `BoundedList<T, N>`, a new type of `sce-portable-bytes` whose every constructor refuses more than `N` on both
   profiles (`SceOwnedList`'s `N` is advisory under `alloc`), and a text in a type that refuses past its bound; the
   nested type is the composition of those (`BoundedList<BoundedList<SceString<8>, 4>, 3>`; the text type is
   `SceString<N>` since section 7.4, which folded `BoundedText<N>` into it). Kotlin and Python check
   in the record's constructor (`init`, `__post_init__`), recursively through a generated check per composition. Go
   and Swift have no checked constructor on a plain struct; the generated `raise_<event>` runs the same generated
   check before it encodes. C++ holds every upper bound in its type since section 7.4's amendment of 2026-11-09
   (`OwnedList<T, N>`, `OwnedText<N>`, `OwnedBytes<N>`), so its `raise<Event>` checks only a list's lower count,
   which no type holds. C11's encode refuses a `len` past a bound at any depth.

### 6. A statechart reads a composed payload as it reads a variable of the same type

1. **What reads it**, at any depth: `len(_event.data.items)`, `len(_event.data.items[i].tags)`, an index into a
   list of any element (`_event.data.items[i].tags[j]`), a field of a record element, `<foreach>` over any list in
   the payload or in an item of an enclosing `<foreach>`, and `<assign location="kept" expr="_event.data.items"/>`,
   which takes the value whole into a variable whose type it fits (section 5.1), a copy that cannot fail. A value that
   does not fit is refused (`scxml/static-datamodel-rule`), naming the narrowing to use: an append in a `<foreach>`.
   A composed value as an operand, or a comparison of two composed values, is refused
   (`expression/unsupported-construct`).
2. **The typing.** A payload path is typed by the one value type, so a statechart rule of section 2.15 is written
   once for every type and every depth.
3. **The queue bound** (ADR 0076). A `<foreach>` over a list with a declared bound runs its body at most that many
   times, and nested loops multiply their bounds (`internal_queue_bound.rs:479-492`). A receiver that raises one
   event per element, as ADR 0085's does, is then proven.

### 7. The types each backend generates

1. **The table.** `N` is a bound after its limit name and `--limit`; each constructor composes with the others to
   any depth.

   | Backend | `list<T>[N]` | `string[N]` / `bytes[N]` | `optional<T>` | record |
   |---|---|---|---|---|
   | Rust (schema unit) | `BoundedList<T, N>` | bounded text / `SceBytes<N>` checked | `Option<T>` | struct, `Clone` |
   | Rust (machine) | `sce_portable_bytes::BoundedList<T, N>` | `SceString<N>` / `SceBytes<N>` | `Option<T>` | the machine's record |
   | Kotlin | `List<T>` | `String` / `ByteArray` | `T?` | data class, checked in `init` |
   | C++ (schema unit) | `SCE::Forge::OwnedList<T, N>` | `OwnedText<N>` / `std::vector<uint8_t>` (entry), `OwnedBytes<N>` (composed) | `std::optional<T>` | struct |
   | C++ (machine) | `SCE::Forge::OwnedList<T, N>` | `OwnedText<N>` / `OwnedBytes<N>` | `std::optional<T>` | the machine's record |
   | Go | `[]T` | `string` / `[]byte` | the backend's optional (ADR 0055) | struct |
   | Python | `list[T]` | `str` / `bytes` | `Optional[T]` | the record class, checked in `__post_init__` |
   | Swift | `[T]` | `String` / `[UInt8]` | `T?` | struct |
   | C11 | `{ uint32_t len; T items[N]; }` | `sce_static_string_<N>_t` / `{ uint32_t len; uint8_t data[N]; }` | `{ bool present; T value; }` | struct |
   | Interpreter | array | string | `null` or the value | plain object |

2. **C11 lays out every composition statically.** Each distinct composition is one named type, its name mangled
   from its structure (`sce_list_3_list_4_string_8_t`), declared once under a guard as `sce_static_string_<N>_t` is
   today, so the same composition is one type across the documents of a build. A record holding a list is still
   copied by assignment, since a C struct holding an array is. Storage is the compiler's `sizeof` of that layout,
   known at build time.
3. **A backend or position that does not lower a composition yet refuses it by name** with
   `generate/unsupported-feature` (as `swift_event_schema_unlowered`, `generator.rs:28396`, does today), naming the
   field and the constructor. That is a step of the landing, not a rule of the language.
4. **A Rust machine holds every bound in its type** (amended 2026-11-04, answer G). The machine row of the table is
   every place a Rust machine has, not only a composed one: a variable, a record the machine declares, the payload
   struct of an event it reads, at the top level and at any depth. A `list<T>[N]` is
   `sce_portable_bytes::BoundedList<T, N>`, a `string[N]` is `SceString<N>`, a `bytes[N]` is `SceBytes<N>`, an
   `optional<T>` is `Option<T>` and a record is the machine's record, one representation per type of the machine.
   The first landing of step 3 held a list in a `Vec<T>` and a text in a `String`, with the bound kept by the
   machine at run time (each append and each assignment comparing a length to a number the generator wrote beside
   it); the schema unit already held the same values in bounded types, and C11 in fixed layouts. Four reasons, each
   one the textbook's, decide for the bounded types:
   - **An illegal value cannot be represented.** A list past its bound is not a value of `BoundedList<T, N>`: every
     constructor and every append refuses it. The refinement `{ xs : [T] | len xs <= N }` (Liquid Types, above) is
     then held by construction, where it was held by every write remembering to check (King's "parse, don't
     validate", applied to the machine's own storage).
   - **One representation per type.** A value of `list<T>[N]` is one Rust type in the machine, in the schema unit and
     in a composed payload, so nothing converts it where it crosses from one to another, and two places of one type
     are copied by `Clone`.
   - **The build's proof is checked again by rustc.** Where the judge proved that a value of `S` fits a place of `T`
     (section 5.1) and the two bounds differ, the copy is a widening conversion of `sce-portable-bytes` whose bounds
     are const parameters, `BoundedList::<T, N>::widened(&list)`, whose body names a constant that asserts `M <= N`
     (`Fits::<M, N>::OK`, evaluated by rustc where the conversion is instantiated; the minimum Rust version has no
     inline `const` block), as `SceString::widened` already does. A generator that writes a copy the judge did not prove fails to compile where
     it is written, rather than refusing at run time a value that was never meant to arrive.
   - **No allocator is needed.** `sce-portable-bytes` holds these types inline (`heapless`) on the profile without
     `alloc` and in growable storage on the others, behind one name. A composed value therefore lowers on a machine
     built `--no-std`, and the refusal of a composed read and copy for that profile (`rust no_std`) goes; what is
     still refused there is refused by name for what the profile lacks, the JSON wire (section 3), not for its storage.
   - **One type for each, hard on every profile** (decided 2026-11-04, after the first reason above was found not to
     hold for text). Until then `SceString<N>` and `SceBytes<N>` named a codec's storage profiles, whose heap form
     keeps `N` advisory, beside `BoundedText<N>` and `BoundedBytes<N>`, whose bound was hard: two types for one value
     type, one of which could hold an illegal value under `alloc`. Now `SceString<N>` is the one text type (what
     `BoundedText<N>` was, which goes), `SceBytes<N>` the one byte-string type (`BoundedList<u8, N>`, so a byte string
     bounds and widens as every list does; `BoundedBytes<N>` goes), and every constructor and append of both refuses
     past `N` on every profile. A codec's frame keeps its own storage profiles (`HeapStr`/`InlineStr`,
     `HeapBytes`/`InlineBytes`), whose heap form's advisory `N` is the reading of a protocol SCE does not own that
     section 1.4 keeps for codecs. Nothing is cut to fit a bound: a literal is built by `sce_text!` or
     `sce_bytes!`, which a literal past the bound does not compile; any other value by a constructor that refuses
     it, or by a widening from a smaller bound. A generated machine names the three types through the runtime it
     already depends on (`sce_rust_runtime::portable`), so a crate that holds a machine needs no other dependency.

   **What waits for section 4.4 on every lane.** The struct of an entry event's payload and a host-run
   invocation's request and result records keep the wire's text and byte string on Rust, the bound held where a
   value of one is written into a machine's place, as every other lane holds it. Holding them in
   `SceString<N>` would refuse a payload whose text is past its bound where the payload is read, which is section
   4.4, and doing that on Rust alone would make a delivery one lane refuses whole and another refuses at the
   assignment, reopening the atomicity of a record list's change. Those records move to the bounded types with
   section 4.4 on every lane at once, at ADR 0085 landing 4 (one change, one event), beside the lift that holds a
   top-level payload text to its bound (commit `509061aded`, parked for the same reason). A composed payload's
   text is held at the lift already (section 4.1), on every lane that reads one. Until section 4.4 lands, a call
   that hands an algorithm `_event.data` whose schema holds text stays refused at check on every lane
   (docs/SCE_ACCEPTED_SUBSET.md section 2.15, "State an algorithm takes and gives back").

   The bound refusal stays where the language puts it: an assignment or an `<sce:append>` that would pass a place's
   bound is refused at that statement and raises `error.execution` (section 5, docs/SCE_ACCEPTED_SUBSET.md
   section 2.15); a value the judge proved to fit is copied by a conversion that cannot fail. The generated public
   types a host compiles against change with it (the published `Data` struct, the payload structs, the records): the
   consumer's migration is in docs/SCE_CODEC_CONSUMER_MIGRATION.md, "A Rust machine holds a list's bound in its type".

   **C++ is the next target of the same reasoning**, which holds there unchanged: the C++ runtime gains bounded
   container templates, a list and a text whose bound is a template argument, and the C++ machine row moves to them
   in a later landing; this amendment does not change C++. **Kotlin, Go, Python and Swift keep the rows of the
   table**, a checked constructor or a bound kept by the machine: none of their type systems has a constant
   parameter a bound could be, so a bound in the type has no spelling there.

   **A C++ machine holds every bound in its type too** (amended 2026-11-09, the later landing answer G names). C++
   holds exactly what Rust holds, in C++ spellings: the owned forms of the forge runtime's `Bounded<L, Max, Min>`
   (`backends/cpp/forge-runtime/include/sce/forge/bounded.h`), the counterparts of the views `Text`, `Bytes` and
   `List` a codec presents, are `SCE::Forge::OwnedText<N>` (`Bounded<std::string, N>`, Rust's `SceString<N>`),
   `SCE::Forge::OwnedBytes<N>` (`Bounded<std::vector<std::uint8_t>, N>`, `SceBytes<N>`) and
   `SCE::Forge::OwnedList<T, N>` (`Bounded<std::vector<T>, N>`, `BoundedList<T, N>`). A machine's variable, its
   record's field and a composed payload's place are one of them, and the schema unit's `string[N]` field and every
   composition it holds are too, so a CBOR codec's list of records with a text field is held within every bound by
   its types on C++ as on Rust and is printed with an encode that refuses nothing. Every write goes through the
   runtime's checked construction (`SCE::Forge::Checked::held<B>`, `heldList<N>`), which records the capacity
   failure an assignment past the bound raises as `error.execution`, and never cuts a value; an append through the
   list's own `push`, which refuses past `N`; a literal through `literal`, whose length the compiler judges against
   the bound (`static_assert`), so a literal past it does not compile; a copy the judge proved to fit through the
   widening constructor, which exists only where the bounds hold (`widenedMap` for a list of a converted element);
   an id a `<send idlocation>` or an `<invoke idlocation>` generates through the variable's type's `make`, refused
   whole past the bound, as on Rust. A host is lent the plain value a place holds (`const std::string &`,
   `const std::vector<T> &`) and builds a value for a payload through `make` or `literal`. The type has no mutable
   access to an element, throws nothing of its own, and is C++17. What waits for section 4.4 on Rust waits on C++
   alike: an entry event's payload struct and a host-run invocation's records keep the wire's `std::string` and
   `std::vector<std::uint8_t>`, the bound held where a value is written into a machine's place.

### 8. C11 sizes each machine's slot, and `SCE_MAX_DATA_LEN` is retired

1. **The slot is the machine's.** Each generated machine already declares its own event record
   (`<sm>_event_with_meta_t`, `state_machine.h.jinja2:375`). Its `data` becomes `char data[<SM>_DATA_LEN]`, where the
   generator computes `<SM>_DATA_LEN` as the largest of:
   - `DATA_MAX_LEN` of every schema whose event the machine can receive (its transitions' descriptors, its closed
     interface, the results of its host-run invocations);
   - the largest data each of its own sends to itself can write (a literal's length, which the template already
     asserts at `state_machine.c.jinja2:151`, or the `J` of its typed params);
   - the events an `<invoke autoforward="true">` parent forwards to it, for a machine that is such a child.

   The number is always computed; no document or build states it. Every value of a machine without a script engine
   is typed, and every type is bounded (section 1.4), so every input above has a size. The data of an event no schema
   declares is not kept by such a machine: under `sce-static` no expression can read it (a payload is read through
   its schema), so its slot holds nothing for it, and its enqueue keeps the event's name and metadata only. The
   scheduled-send records, the host request buffers and the delayed-host-send rows the template declares
   (`state_machine.h.jinja2:468`, `524-536`, `575-579`) take the same number, or the `J` of what they hold when that
   is smaller.

   A machine whose data model is a script engine (Lua, the C11 backend's) computes its values in the engine, where no
   type bounds them. Such a machine already allocates, through the engine's own heap, so it holds an event's data in
   storage the engine allocates for that event, sized to the text it carries, and has no fixed slot. A fixed slot
   exists for a machine that must not allocate, and that machine has no script engine. Neither kind of machine reads
   a number from its build.
2. **The runtime holds no slot of its own.** The structs of the C11 runtime that hold data by the global knob hold a
   view of the machine's buffer instead (`const char *data; size_t len`) or are declared by the generated machine at
   its size: the autoforward carrier (`types.h:369`), the host processor's response and request records
   (`host_processor.h:157`, `629`, `641`, `893`), and `sce_copy_bounded_data` (`types.h:330`), which becomes a copy
   that refuses a text past the destination rather than cutting it.
3. **Migration for a host that sets the knob.** `SCE_MAX_DATA_LEN` is removed from `types.h`. A build that still
   defines it fails with `#error` in `types.h` saying that the slot is computed, so a stale flag is never silently
   ignored. A host drops the flag. Where it raised the number so that some event's data fit, that event's data is
   now declared: by a schema, with a bound on every text, whose `DATA_MAX_LEN` sizes the slot. The standard host's
   note (`standard_host.h:18-20`) becomes a schema: its HTTP result is a
   declared result schema whose `DATA_MAX_LEN` the generator reads. The C11 tests that pass the flag
   (`backends/c/tests/CMakeLists.txt`, `test_static_scalars.c`, `test_host_processor.c`, `test_host_features.c`,
   `test_ai_loop.c`) move with the landing.
4. **What this replaces.** ADR 0085 section 5 asked for a static assertion against the knob. A slot sized by the
   build makes the assertion unnecessary: no slot can be smaller than what its machine receives.
5. **Who owns an event's data, and for how long (Proposed 2026-11-08, awaiting the owner).** Landing step 5's
   first slice computes `<SM>_DATA_LEN` (item 1, from the schemas whose event a transition takes or whose record a
   host-run invocation completes with) and declares every buffer the machine owns at it, with `SCE_MAX_DATA_LEN`
   kept as its floor for the inputs nothing sizes yet. The rest of step 5 needs two decisions item 2 and the
   paragraph on script engines in item 1 leave open, and the slice found inputs item 1 does not list. Each is
   stated below with its options, the recommendation and why.

   **The rule every recommendation applies.** Every buffer of event data has exactly one owner, and the owner
   knows its size. A function that reads data only during a call takes a borrowed view, a pointer and a length,
   valid for that call and never kept. Storage that outlives a call belongs to the one holder that keeps it, and a
   value that changes holder is moved (the old holder keeps nothing) or copied into storage the new holder owns,
   never shared. This is the ownership and borrowing discipline of the textbook (Rust's affine moves and borrows,
   C++'s `unique_ptr`), spelled in C as it already spells it in this runtime: a caller-allocated buffer with its
   capacity, filled by a writer that refuses what does not fit (`snprintf`, `sce_host_response_push`); and a value
   that crosses to another thread is handed over, never shared (Hoare's communicating processes, which the inbox
   of ADR 0084 already is).

   **(a) A machine whose data model is a script engine.** What holds event data in such a machine: its two event
   queues (entries by value), its scheduled-send entries, the rows of a delayed host-served send, the buffers a
   send writes its payload into, and `_event`, which the engine copies into its own string when the event is
   dequeued. It has no saved state (a C11 machine with an engine has no save API) and no rows of host requests.
   - a1. **An owned text per event, allocated through the engine's allocator** (`lua_getallocf`), moved and never
     copied in place: an enqueue allocates the text and copies the data in; the queue entry owns it; the dequeue
     moves it out, hands it to the engine, and frees it; a scheduled entry owns its text until it fires (moved into
     a queue), is cancelled or the machine is destroyed (freed); a send to another session copies into storage
     the receiver allocates. An allocation that fails refuses the act: a send raises `error.execution`, and a
     host's enqueue answers false, as a full queue does.
   - a2. A reference into the engine's registry (`luaL_ref`). Tried and retired (the scheduled entry's comment in
     `state_machine.h.jinja2`): a reference means nothing outside the engine that issued it, so it cannot cross to
     another session, and the queue would hold a handle whose release depends on an engine call at every exit path.
   - a3. An arena per machine. No type bounds a script value, so its size is a number of the build again, which
     answers B and E exclude.
   - a4. The fixed slot of today. That is `SCE_MAX_DATA_LEN`, which item 1 retires.

   Recommended: a1. It is the only option with no number of the build and no cut text; each move clears its
   source, so a text is freed exactly once by construction rather than by every exit path remembering; and the
   allocator is the one the engine already uses, which a host may replace when it creates the engine, so the
   machine has one heap. With it, the record a host hands an event in, `<sm>_event_with_meta_t`, carries its data
   as a borrowed view (`const char *data` and its length, valid for the call of `_raise_external`) on both kinds of
   machine, and the queue entry becomes a private type that owns: a slot of `<SM>_DATA_LEN` on a machine without
   an engine, an owned text on one with. A host that wrote into `meta.data` writes the view instead; the C11
   tests that do so move with the landing. The inbox of a machine with an engine cannot call the engine's
   allocator, since a post comes from another thread or an interrupt handler and the engine is not reentrant; a
   post there hands over storage the poster allocated with the function that releases it (a move with its
   deleter), called by the owner once the event is applied. A poster in an interrupt handler hands static storage
   with a release that marks it free.

   **(b) The runtime's records that hold data by the knob.** Each takes the rule above by the lifetime of its data:

   | Record | Lifetime of its data | Recommended |
   |---|---|---|
   | `sce_forwarded_event_t` (`types.h`) | one synchronous call, parent to child | a view; the child copies into its own slot and keeps nothing for an event it has no schema for (item 1) |
   | `sce_host_send_response_t` (`host_processor.h`) | written by the handler during the call, read by the machine after it returns | the list carries views into text storage the machine declares (`SCE_MAX_HOST_RESPONSES` texts of `<SM>_DATA_LEN`), filled only through `sce_host_response_push`, which already refuses what does not fit |
   | `sce_host_invoke_response_t` (`done_data`, `refusal`) | the same | two caller-provided buffers with their capacities, set through two writers that refuse what does not fit |
   | `sce_host_invoke_ending_t` (the ADR 0084 inbox) | posted from another context, applied later by the owner | the inbox input becomes the machine's own record (`<sm>_inbox_input_t` already is, and holds the ending by the runtime's type); its data slot is `<SM>_DATA_LEN`; the post takes a view and copies, and a post whose data does not fit is refused and counted, as a post into a full inbox is |
   | `sce_mesh_call_t` (`host_processor.h`) | queued by the endpoint, taken by the machine | the take fills a record the machine provides, whose data is a buffer of `<SM>_DATA_LEN` with its capacity; a call whose data does not fit is refused at the endpoint and reported to the machine as the delivery that failed |
   | `sce_copy_bounded_data` (`types.h`) | any copy into a slot | a copy into a destination of a stated capacity that answers whether it fit, in place of one that cuts |

   The options against which these are recommended: every record declared by the generated machine (the handler
   types are the runtime's and serve many machines, so a handler would need one function per machine); a view
   everywhere (a view into the poster's frame dangles once the post returns, so the inbox needs a copy); and a
   heap per record (a machine without an engine must not allocate). Each recommendation is the textbook's choice
   for its lifetime: borrowed for a call, caller-allocated with a capacity for an out-parameter, copied into the
   receiver's storage for a value that crosses a thread.

   **(c) Inputs item 1 does not list**, which have to be settled before the floor goes (step 5's last slice):
   - c1. **A delivery's text is not bounded by `J`.** JSON's insignificant whitespace and a key the schema does not
     declare (stepped over by section 4.1, and sent by the scenario of section 14.2) make an accepted text longer
     than `J`, and a delivery past a bound has to reach the lift to be refused whole with `error.execution`
     (section 4.2), not be refused by the door. Recommended: the door lifts the delivery when it is enqueued and the
     queue holds the canonical JSON the writer spells of the lifted value, which is at most `J` by section 2.3, or
     for a delivery that does not fit its schema a mark and its reason, which raise `error.execution` in the
     event's place in the queue, so the order of events is kept. The alternative, a door that refuses any text
     longer than the slot, answers a valid delivery differently from every other lane.
   - c2. **The reasons a machine raises itself.** An `error.execution` carries its reason in `_event.data`, a
     sentence of the build or the runtime and, for a lift, a path. Recommended: each sentence is a literal the
     build knows and a path is bounded by the schema's keys and the digits of its bounds, so the largest reason is
     one more input of item 1, computed by the same calculus.
   - c3. **What a child session sends to its parent.** A child's done data and its sends to `#_parent` are sized by
     the child's own document, which is generated with the parent. Recommended: an input of the parent's slot,
     from the child's own computation.
   - c4. **Memory.** Every queue entry holds a slot, so a machine whose largest schema is large pays it per entry:
     the standard host's HTTP result (`stdlib/host/http_fetch_result.scxml`, a body of 16384 bytes and headers of
     4096) has `J` = 123074 bytes, about 7.9 MB for the two queues at `SCE_MAX_EVENTS` = 32. Recommended: keep the
     queue value-typed and statically reservable, as item 1 decides, and let the C11 queue take the depth a
     document declares (`sce:capacity`, which the Rust profile without `alloc` already reads), so a machine that
     takes large events takes few of them at a time.

### 9. Signed values are deterministic CBOR

1. **The rule.** From ADR 0085's one domain bump, `sce-trust/event/3`, the bytes a signature covers are the
   deterministic CBOR encoding (RFC 8949 section 4.2.1, core deterministic encoding requirements) of the array
   `[domain, list, event, payload]`: the domain text, the list's name, the event's name, and the payload as a map of
   every field of its schema. The subject bytes (`Signing::subject`) are the same encoding of the payload restricted
   to the fields whose role is not `Change`. The operation bytes (`operation_bytes`, `message.rs:318`) are the
   encoding of `[operation domain, signed message bytes, link]`, with the link as a map of its fields. The length
   prefixes of `part` (`message.rs:380`) go.
2. **The mapping from a type to CBOR.**
   - An unsigned number: major type 0. A signed number: major type 0 when it is zero or more, major type 1 otherwise.
     Shortest head in both (preferred serialization).
   - `float32`, `float64`: major type 7, the shortest of half, single and double precision that holds the value
     exactly (RFC 8949 section 4.2.1 with section 4.1); a NaN is `0xf97e00`.
   - `bool`: `0xf4`, `0xf5`.
   - `enum:E`: **its declared number** (`value=`, ADR 0047), as an integer of `E`'s underlying type. The JSON wire
     spells the name, and a signature over the name would be closer to the text; but an open enum
     (`sce:strict-variants="false"`) holds numbers no variant names, so only the number covers every value, and the
     number is what the underlying type declares for the wire. A revision of an enum adds variants and never
     renumbers one (ADR 0044), so the number is as stable as the name.
   - `string[N]`: major type 3. `bytes[N]`: major type 2, the bytes themselves, not the Latin-1 text JSON carries.
   - `optional<T>`: **`null` (`0xf6`) when absent**, the encoding of `T` when present; the key is always in the map.
     A map's keys are then a function of the schema's revision alone, so a delivery and its rewrite sign alike, and
     an absent value is never confused with a field the sender's revision lacks; `optional<optional<T>>` is refused
     (section 1.5), so `null` is unambiguous.
   - `record:R`: a map of every field of `R`, keys the field ids as text, values by their types; a field a later
     revision of `R` added and a value lacks is encoded as its default, so an older and a newer delivery that mean
     the same sign alike (`message.rs`'s rule today). Keys sorted by the bytes of their encodings.
   - `list<T>[N]`: an array of definite length, elements in order.
   - A key of the delivery that the schema does not declare is not signed, as today.
3. **Vectors** (hexadecimal; each is a test of the encoder):

   | Value | Type | Encoding |
   |---|---|---|
   | 0 | `uint8` | `00` |
   | 500 | `uint32` | `19 01 f4` |
   | 18446744073709551615 | `uint64` | `1b ff ff ff ff ff ff ff ff` |
   | -1 | `int64` | `20` |
   | -500 | `int32` | `39 01 f3` |
   | -9223372036854775808 | `int64` | `3b 7f ff ff ff ff ff ff ff` |
   | 1.5 | `float64` | `f9 3e 00` |
   | 0.1 | `float32` | `fa 3d cc cc cd` |
   | 0.1 | `float64` | `fb 3f b9 99 99 99 99 99 9a` |
   | true | `bool` | `f5` |
   | variant `week`, declared 2 | `enum:E` over `uint8` | `02` |
   | "ab" | `string[8]` | `62 61 62` |
   | 00 ff | `bytes[4]` | `42 00 ff` |
   | absent | `optional<int64>` | `f6` |
   | [] | `list<uint8>[3]` | `80` |
   | [1, 2] | `list<uint8>[3]` | `82 01 02` |
   | [[1], []] | `list<list<uint8>[2]>[2]` | `82 81 01 80` |
   | {id: 1, label: "a"} | record | `a2 62 69 64 01 65 6c 61 62 65 6c 61 61` |
   | {aa: 1, b: 2} | record | `a2 61 62 02 62 61 61 01` (`b` first: its encoding is shorter) |

4. **Where it lands.** This ADR adds the encoder, a function of a value and its type descriptor, in
   `sce-rust-trust`, with the vectors above and a property test: the encoding of a value decodes, by any conforming
   CBOR decoder, to the same value, and re-encoding gives the same bytes. Each Rust schema unit declares its type
   descriptor (`TYPE`), so the trust layer reads a schema's types from the generated unit and its table states only
   each signed event, its author field and its fields' roles, held to the schemas as today. No signed event changes
   here: `sce-trust/event/2` keeps its bytes, and ADR 0085 landing 5 switches the domain and the encoding at once.

### 10. Revisions of a schema unit (ADR 0044)

1. **A field of the first revision** is carried by every delivery; an empty list is `[]`, an absent optional `null`.
2. **A list a later revision adds** declares `sce:since="k"` and no `sce:default`; a delivery without it reads the
   empty list, and `defaults_wire` (`schema_revision.rs:144`) writes `[]` for it. `sce:default` on a list is refused
   (`validation/attribute-rule-violated`): the empty list is the only value meaning "the sender said nothing", and the
   language has no literal for any other. An `optional<list<T>>` a later revision adds reads absent, as any optional
   does.
3. **A record inside a field grows by its own schema's revisions.** Each record value is read with its schema's
   defaults, at any depth. ADR 0044's rule (a) judges a record type inside a field by applying the rule to the two
   record schemas, not by their texts, so two builds whose records are revisions of one schema declare one field.
4. **No bound is raised by a later revision.** Rule (a) refuses another bound at any depth, another element, another
   record document or a change of constructor. Raising a bound is safe for the reader and unsafe for an older
   receiver of a newer delivery, so the symmetric rule refuses it. A limit name (`[name=N]`) moves a bound for the
   whole build.

### 11. Diagnostics

1. **One new code, for the legacy spelling.** A bound written as an attribute is not a refusal in this landing: it
   is read, and the author is told the spelling is superseded and given its rewrite. No existing code states that
   fact. `validation/removed-attribute` is a refusal of an attribute that is gone, with a fix that deletes a key at a
   configuration path; `validation/codec-format-superseded` names a codec document's format. So the landing adds
   **`validation/bound-attribute-superseded`**, stage `validation`, its spec anchor this ADR's section 1.3, with a
   new deterministic fix kind **`move_bound_into_type`** (payload `element`, `attrs[]` the attributes to remove, and
   `type` the `sce:type` that carries the bound, `[name=N]` where a `sce:limit` named it). `sce-codegen
   bound-spelling <document>` prints a document so rewritten, every other byte as written, as `sce-codegen
   codec-format` does for a codec; the landing runs it over every committed document, so the tree carries none.
   - **How it is reported.** Diagnostics carry no severity (SCE_ERROR_CONTRACT.md section 1); a record that does not
     stop the run is emitted with a successful run's manifest and exit 0, as an expression the SCXML rule for an
     unevaluable `cond` obliges a build to accept already is (same section), and `sce-codegen --lint` promotes it to
     fatal. The next revision of the authoring grammar refuses the
     attributes, with the same code and fix.
   - **Where it is registered.** The `DiagnosticCode` variant and the `Fix` variant in
     `sce-build/src/forge/diagnostic.rs`; the code's row in docs/SCE_ACCEPTED_SUBSET.md's appendix, which
     `acceptance_doc_covers_every_code` requires; the generated catalog of SCE_ERROR_CONTRACT.md section 5 and the
     fix's row in section 3.1; a golden of each attribute it reads.
2. **Every other refusal is an existing class of fact**, and the goldens of its sites gain cases:

| Refusal | Code | Stage |
|---|---|---|
| a bound written as an attribute (the legacy spelling; reported, not refused, in this landing) | `validation/bound-attribute-superseded`, fix `move_bound_into_type` | parse |
| a bracket on the outermost type and an attribute that bounds it, on one declaration | `validation/incompatible-attributes` | parse |
| a `string`, `bytes` or `list` with no bound, at any depth, an event schema's top-level field included | `validation/attribute-rule-violated`, naming the place in the type | parse |
| a bound that is not a positive 32-bit integer, or a malformed `[name=N]` | `validation/attribute-rule-violated` | parse |
| a bracket on a type that has no bound (a number, `bool`, an enum, a record, an optional) | `validation/attribute-rule-violated` | parse |
| `optional<optional<T>>` | `validation/invalid-attribute`, with the candidates | parse |
| an alias the document does not import | `validation/invalid-attribute`, with the candidates | parse |
| a type that contains itself | `validation/cross-kind-circular-dependency` | import resolution |
| `sce:default` on a list | `validation/attribute-rule-violated` | parse |
| a composed type on a CBOR or content-line entry | `validation/invalid-attribute`, with the entry types | parse |
| a value sent, forwarded or assigned whole where its type does not fit | `scxml/static-datamodel-rule`, naming the path | check |
| a composed value as an operand, or compared | `expression/unsupported-construct` | check |
| two machines whose fields are not one field | `mesh/event-schema-mismatch` | deploy |
| a position or backend that does not lower a composition yet | `generate/unsupported-feature` | generate |

A run-time refusal is a sentence carried by `error.execution`, not a diagnostic.

### 12. ADR 0042, and why its reason no longer holds

ADR 0042 section 2 announced a register in one event and each further value in another because "an event cannot
carry a list". That limit is gone. What section 2 decides stays: only registers that moved are told, once the change
is whole, to a parent, and only when the list was started with `announce`. ADR 0085 landing 4 changes the telling:
`record_list.register` carries all of a register's live values as one `values` list, bounded by `shared_roster` (a
register holds at most one live value for each member who wrote it at the same time), and `record_list.value` with
its ordinal goes. The constraint behind the old sentence holds in another form, a receiver must be able to hold what
a delivery carries, and the bound in the type meets it. Section 1's third point ("a record is spread over several
fields of one row") is how the list stores what an application writes, and stands.

### 13. Wire surfaces (SCE_WIRE_CONTRACTS.md)

No status flips.

- **Authoring grammar** (`pre-release`). The XSD's patterns for `sce:type` are regular and cannot match a
  recursive grammar, so the type attribute becomes a lexical string of the type alphabet, and the parser judges the
  structure with a diagnostic that names the rule, as the patterns' comments already say of `list<string>`
  (`sce-forge-ext.xsd:131-160`, `275-283`). The comments of `sce:max-size` (`:601`), `sce:max-count` (`:578`),
  `sce:capacity` (`:296-303`) and `sce:limit` say that on a value type each is the legacy spelling of a bracket,
  read with `validation/bound-attribute-superseded`. Two changes are not additive, both allowed while the surface is
  `pre-release` (its policy item 1): an event schema's text or byte string with no bound, valid until now, is
  refused (the five committed documents are migrated in the landing, section 1.4); and the next revision of the
  grammar refuses the legacy attributes.
- **Forge AST export** (`pre-release`). A type is the recursive `ValueType`, its bounds inside it; a number, `bool`
  or enum keeps its bare string, and a bounded type is exported in one shape whichever spelling the document used,
  so the export does not tell a bracket from a legacy attribute.
- **Diagnostics** (`pre-release`). One code and one fix kind are added (section 11), additively.
- **Replay scenario** (`registered`). `Holds::of_schema` types a field by its whole type, so a `null` at any depth is
  judged by the type at that place; the checker and every driver change with it.
- **Scenario set** (`pre-release`). Its refusal of "a list as a payload value" becomes a refusal of a value whose
  shape the machine's schema does not declare.
- `_event.data` has no row; ARCHITECTURE.md's key order and docs/SCE_ACCEPTED_SUBSET.md hold its shape.

### 14. Tests on every lane

Each test fails when the rule it pins is removed; the landing checks each once against a mutant of that rule.

1. **Fixtures** (new, `sce-build/tests/fixtures/static_datamodel/`), every bound in the type:
   - `event_batch_item.scxml`, a record: `id` `uint32`, `amount` `int64`, `label` `string[8]`, `level`
     `enum:SignalLevel`, `due` `optional<int64>`, `blob` `bytes[4]`, and `tags` `list<string[8]>[4]`, so a record
     holds a list;
   - `event_batch.scxml`, `batch.sent`, revision 2: `items` `list<record:Item>[batch_items=3]`; `grid`
     `list<list<uint8>[2]>[2]`; `levels` `list<enum:SignalLevel>[2]`; `maybe` `list<optional<uint16>>[2]`; `spare`
     `optional<list<uint8>[2]>`; `note` `string[4]`; and `late` `list<uint8>[2]` with `sce:since="2"`;
   - `legacy_bound_spelling.scxml`, the same schema as `event_batch_item` written with `sce:max-size` and
     `sce:max-count`: it generates what `event_batch_item` generates, byte for byte, and its check reports one
     `validation/bound-attribute-superseded` for each attribute, with the rewrite that turns it into
     `event_batch_item`'s text;
   - `static_list_payload.scxml`, which takes `batch.sent`, copies `items` whole into `kept`, walks it and each item's
     `tags` in nested `<foreach>`es, reads `grid[i][j]`, `len` at each depth and an optional element, and on
     `batch.echo` sends `batch.sent` to `#_internal` with `<param name="items" expr="kept"/>`, so a list it sent
     returns through its own lift.
2. **The scenario** `scenarios/static_list_payload.json`, expected values derived from the documents: the empty
   lists; the maximal value of every field (every list full at every depth, every text at its bound); one past each
   bound at each depth (`items`, `items/0/tags`, `grid/1`), refused whole with the path; a 9-byte `items/2/tags/0`;
   an element missing `amount`; `null` in `items/1/id` and accepted in `maybe/0` and `items/0/due`; an enum name
   `levels` does not declare; an `amount` past 2^53 as digits; `items` given as an object; an unknown key inside an
   element; a 5-byte top-level `note`; a revision-1 delivery without `late`; the round trip, with the `sent` text in
   canonical key order at every depth; and the order of `[a, b]` kept. It replays on Rust
   (`backends/rust/tests/tests/static_scenarios.rs`, `backends/rust/scenarios/src/machines.rs`), Kotlin
   (`StaticScenarioTest.kt`), Python (`test_static_scenarios.py`), Go (`static_scenarios_test.go`), C11
   (`test_static_scalars.c`), Swift (`StaticDatamodelTests`, `Replay.swift`) and the Interpreter
   (`AStaticDatamodelRunsLoweredUnderTheInterpreterTest.cpp`); C++ derives its suite from the directory. Eight lanes.
3. **Each runtime's reader and writer**: each constructor at depths 1, 2 and 3; the bound refused before the element
   past it is decoded; the path in every refusal; a maximal value written at exactly `J` bytes.
4. **`sce-build`**: a golden for each refusal of section 11; the size calculus per constructor and per composition
   against the table of section 2.1, and tight against a maximal value (section 2.3);
   `a_schema_unit_names_its_event_revision_and_bounds.rs` gains the outer bounds and `DATA_MAX_LEN`;
   `a_send_is_held_to_the_schema_its_receiver_reads.rs` gains "fits" at each depth; the queue bound proves nested
   `<foreach>`es that raise; a C11 machine's `<SM>_DATA_LEN` equals the largest of its inputs, a machine that takes
   an event with no schema keeps no slot for its data, and a build that defines `SCE_MAX_DATA_LEN` stops at the
   `#error`; an event schema with an unbounded text, at the top level and nested, is refused at the place in the
   type; a bracket and an attribute on one declaration are refused even when they agree; `sce-codegen
   bound-spelling` rewrites each legacy attribute and its `sce:limit`, and a document so rewritten checks with no
   record and generates the same bytes; `--lint` turns the legacy record into a refusal.
5. **The CBOR encoder** (`sce-rust-trust`): every vector of section 9.3; preferred serialization of every integer and
   length boundary (23/24, 255/256, 65535/65536, 2^32 - 1/2^32); key order by encoding; a property test of decode
   and re-encode; `C` of a maximal value equal to its encoding's length.
6. **Mutation casefiles** under `sce-build/tests/mutations`: each lift's count comparison, the inner text bound, the
   "fits" comparison of bounds, the slot computation, and the CBOR key order and head length.

### 15. A list states its lower count (amended 2026-11-06, answer H)

1. **Why.** Every bound is written in the type (section 1.2), and until this amendment a list's bound was its most
   elements only. The answer schema a build writes from `list<T>[n]` (ADR 0087 section 1.1) carries `maxItems` and
   no `minItems`, so a grammar-constrained decoder may write `[]` for a list the application needs filled. Measured
   on 2026-11-06, a Gemma 4 model under constrained decoding answered an empty list in 32 of 45 cases of a list that
   must hold at least 3, and in 36 of 55 cases of one that must hold at least 1. Constrained decoding distorts a
   model's distribution toward the grammar's shortest path (Park, Wang, Berg-Kirkpatrick, Polikarpova and
   D'Antoni, "Grammar-Aligned Decoding", 2024); the textbook remedy is to take that path out of the grammar, and the
   grammar is folded from the type, so the path is taken out of the type, the single source. ASN.1 writes the same
   fact in the same place: `SEQUENCE (SIZE (1..N)) OF T` (ITU-T X.680, its size-constraint clause), and CDDL as the
   occurrence `m*n` (RFC 8610 section 3.2).
2. **The notation.** `list<T>[M..B]` is a list of at least `M` and at most `B` elements, `0 <= M <= B`, the lower
   count written before `..` in the one bracket the bound already stands in. `list<T>[B]` stays and means
   `list<T>[0..B]`; a lower count of zero is not printed, so `list<T>[0..8]` and `list<T>[8]` are one type and one
   text, `list<T>[8]`. A named bound pairs with a lower count as `[M..name=N]`: the name moves the upper bound only
   (ADR 0050), and the lower count is a literal of the type, which no build moves. A build whose `--limit name=N`
   puts `N` below `M` would give the type no value, and is refused where the type is read, as `M` past a written
   bound is. Refused:
   - `M` past the bound (`list<uint8>[4..3]`, or `[3..name=8]` under `--limit name=2`):
     `validation/lower-count-past-bound`;
   - a lower count that is not a whole number of zero or more (`[-1..3]`, `[..3]`, `[3..]`): the malformed-bound
     class of section 11, `validation/attribute-rule-violated`;
   - a lower count on a type that is not a list, and on a list where nothing reads the count (item 7, item 8):
     `validation/lower-count-misplaced`.

   **Texts and byte strings take none.** Their bracket counts bytes, and a lower bound in bytes is not trivially
   symmetric with the upper one: JSON Schema's `minLength` counts characters, so a text of at least `M` bytes is no
   `minLength` the schema could state (a character takes one to four bytes); its GBNF grammar would need a chain
   that counts from below; and the lift would need a code of its own for a text too short. None of the grounds above
   measured a text. `string[1..8]` and `bytes[0..4]` are refused, and the legacy attribute `sce:max-count`, which
   states no lower count, stays a synonym of the upper bound alone: a list with a lower count is written in the
   bracket.
3. **The model.** `Bound` gains `min`, the fewest elements, 0 where none is stated (and on every text and byte
   string). Two types are equal only when their lower counts are, and the forge-AST export writes `"min"` beside
   `"max"` in a list's object when it is past zero and leaves it out otherwise, so every export written before this
   amendment is unchanged.
4. **The size calculus is unchanged.** The largest value of `list<T>[M..B]` is a full list, so `J` and `C` read `B`
   alone, and so do C11's layout and Rust's `BoundedList<T, N>`.
5. **Where a delivery is read (section 4).** A list with fewer than `M` elements is refused where it is read, at
   any depth, the delivery whole and `error.execution` raised once, as a list past its bound is: the elements are
   counted before any is decoded, the count judged after the upper bound, and the reason names the path:
   `'items' holds 0 elements, fewer than the 3 its type declares`. Each runtime's list reader takes the lower count
   beside the bound: Rust `PayloadValue::list_within`, Go `ReadListWithin`, Kotlin `Value.listWithin`, Python
   `ListOf(..., least=M)`, C++ `readListWithin`, Swift `list(least:bound:)`, C11 `sce_payload_list_open_within`,
   and the Interpreter's descriptor `{list: T, max: B, min: M}`; each reader of a list that states none reads it as
   it did.
6. **Where a value is written (section 5).** "Fits" gains the lower count: `list<S>[a..b]` fits `list<T>[c..d]`
   when `S` fits `T`, `b <= d` and `a >= c`. A copy, an assignment or a forward takes only a value that fits, so
   none of them checks a count at run time. A send is the one place a value may be written whose lower count the
   build does not prove: a variable filled by appends fits a field but for its lower count (item 7). Its send checks
   the count at run time before it encodes, and refuses the event with `error.execution` naming the path, the
   sentence of item 5. That is hybrid type checking: what the build can decide it decides, and what it cannot is
   checked where the value crosses (Flanagan, "Hybrid Type Checking", POPL 2006). A host's injected value is checked
   where it is made, beside its upper bound (section 5.4): Kotlin `ValueBounds.listWithin` in `init`, Python
   `check_value` in `__post_init__`, the generated check of Go (`CheckListWithin`), C++
   (`EventPayloadBounds::listWithin`) and Swift (`checkList(least:bound:)`), Rust's `check_count`, and C11's
   `<name>_holds_at_least` beside `<name>_within` (one C11 layout serves a bound of every lower count). The lower count
   is not held in a Rust type, against the grain of section 7.4: a type with a lower count has no empty value, and
   every list a machine builds passes through counts below it, one append at a time, so the type that holds a list
   while it is built cannot be the type that states the count. Section 7.4's reason holds for the upper bound, which
   no append may pass; the lower count is held at the two crossings, where a value is read and where it is written
   whole, on Rust as on every other lane.
7. **A variable and an algorithm's slot state no lower count.** Each starts empty and is filled an element at a
   time (docs/adr/0076, `<sce:append>`), so a lower count in its own type, at any depth, would be broken by every
   value it holds before it is full, and is refused (`validation/lower-count-misplaced`). A record's schema may
   state one in its fields: a value of a record enters a place whole (a lift, a copy of a payload's record, a
   forward), never built. A copy of a list that states a lower count into a variable that states none fits.
8. **A list a later revision adds states none.** A delivery that lacks it reads the empty list (section 10.2), which
   a lower count would refuse, so such a list is refused with one (`validation/lower-count-misplaced`). An
   `optional<list<T>[M..B]>` a later revision adds reads absent, and may state one.
9. **JSON Schema.** `S(list<T>[M..B])` writes `"minItems": M` when `M` is past zero, between `items` and `maxItems`
   in the one keyword order (ADR 0087 section 1.1, item 2): `{"type":"array","items":S(T),"minItems":3,
   "maxItems":8}`. `schemas/sce-payload-schema.v1.schema.json` admits it, an integer of at least 1, beside
   `maxItems`, and states the keyword order with it.
10. **GBNF.** A list that states a lower count writes its required elements as a chain, as every bound of the
    grammar is written (ADR 0087's `GbnfRules`: llama.cpp makes a repetition past 2000 unbounded, so no bound is a
    repetition `{m,n}`). With `X` the rule of an element, `G(list<T>[M..B])` for `M >= 1` is
    `"[" X X-least-(M-1) X-tail-(B-M) "]"`, where `X-least-k ::= "," X X-least-(k-1)` and `X-least-1 ::= "," X`,
    and a term whose index is zero is left out: exactly the language of `"[" X ( "," X ){M-1,B-1} "]"`. For `M = 0`
    the rule is unchanged. The empty list is then no sentence of the grammar of a list that needs one element, and
    the shortest path of the grammar holds `M` elements.
11. **The lift (ADR 0087 section 2.3).** A fourteenth refusal code, `tooFew`, number 14: a list short of its lower
    count. Its pointer is the list, and its `bound` is the lower count `M` in elements, as `tooMany`'s is the upper
    bound. `sce:std/lift/refusal_code.scxml` gains the variant; every runtime's code carries it, and its reader
    raises it where item 5 refuses. The reference harness's own code enum repeats the lift's codes by number
    (`enum_harness_code.scxml`), so it repeats `tooFew` at 14 and moves its own `notOffered` from 14 to 15. The
    outcome class table (`tests/host_features/model_outcome_classes.json`) names `tooFew` deferred beside `tooMany`
    until a schema a lift reads holds a list (ADR 0087, "Landing order", item 8).
12. **Revisions (section 10, ADR 0044).** Lowering a list's lower count is a widening, as `T` to `optional<T>` is
    (ADR 0055 section 6): every value of `[a..n]` is a value of `[c..n]` when `c <= a`, so a delivery an older
    sender wrote reads in a newer receiver. Two schemas of one event are revisions of one when, beside the rule of
    section 10, each list both declare has one element type and one bound, and the later revision's lower count is
    at most the earlier's; two schemas of one revision state one lower count. A newer sender's shorter list reaching
    a receiver built before the lowering is refused there (`tooFew`, `error.execution`), as a newer sender's `null`
    is, and a deployment that lowers a count upgrades the receivers first. Raising a lower count is not a revision:
    a value an older sender wrote may then be refused, so the two schemas part at the field, as at any other change
    of its type (section 10.4). No marker such as `sce:optional-since` is needed: the revisions' own numbers order
    the two, and nothing reads a value as of an earlier revision's count (a saved state names a list by its element
    and upper bound). The rule is `schema_revision::list_count_revises`; deploy judges no two revisions of a schema
    that holds a list yet, and refuses such a pair at the field (`composed_parting`) until a build sends its event
    between machines (section 5.2).
13. **Diagnostics (section 11).** Two codes, stage `validation`, each anchored to this section:
    - **`validation/lower-count-past-bound`**: the lower count is past the bound, as written or after `--limit`,
      so the type has no value. No fix: swapping the two numbers changes what the type says, and which of them to
      lower is the author's choice.
    - **`validation/lower-count-misplaced`**: a lower count where nothing reads it: on a string or bytes, in a
      variable's or an algorithm slot's type, on a list a later revision adds. Fix `replace_with`, the type with
      every lower count removed: the one type that keeps everything else the declaration says, and the edit is no
      choice.
    The codes stand in docs/SCE_ACCEPTED_SUBSET.md's appendix (`acceptance_doc_covers_every_code`), in the generated
    catalog of SCE_ERROR_CONTRACT.md section 5 and in `schemas/sce-diagnostic.v1.schema.json`.
14. **Wire surfaces (section 13).** No status flips, and every change is additive. Authoring grammar
    (`pre-release`): `[M..B]` was refused as a malformed bound and is read now; the XSD's type alphabet already holds
    `.`, and its comment states the grammar. Forge AST (`pre-release`): the optional `min`. Diagnostics
    (`pre-release`): two codes, one existing fix kind. Payload schema (`pre-release`): the optional `minItems`.
15. **Tests.** `value_type.rs`: the notation read, printed and refused (each refusal of item 2, the limit that
    passes the lower count), the export of `min`, "fits" with a lower count against the set of values each type
    holds, the JSON Schema's `minItems` and the GBNF chain, each against the recognizer; `payload_schema.rs`: the
    fixture `sce-build/tests/fixtures/payload_schema/answer_counted.scxml`, whose smallest value holds each lower
    count and whose value one short of a count both forms refuse; `schema_revision.rs`: lowering is a revision,
    raising and a changed bound are not; each runtime's reader refuses one short with `tooFew`, its pointer and
    bound, on the shared table `tests/json_text/payload_refusal.json`, which gains `min` in its type descriptor;
    each writer's check refuses one short; the parser refuses each misplaced count; the goldens of the two codes.

## Landing order

1. **The type grammar and the model**: `ValueType`, the bracket bounds and the legacy attributes read as their
   synonyms, `validation/bound-attribute-superseded` with `move_bound_into_type` and `sce-codegen bound-spelling`,
   the codec entry's `EntryType`, the refusals (an unbounded text or byte string among them), the revision rules, the
   size calculus, the XSD, the AST export. In the same change, every committed document is rewritten by
   `sce-codegen bound-spelling` and the five unbounded schemas get their bounds (section 1.4), so the tree checks
   with no record, and the generated trees are regenerated (they change only where `schema_echo`'s bound appears).
   Every backend refuses a composition it does not lower by name.
2. **The runtimes**: the reader of one value at one path with its three combinators, the writers, `BoundedList`
   and the bounded text type, in the eight runtimes, with their unit tests.
3. **The generator for event fields and statechart variables**: schema units with their sizes, lifts, sends,
   statechart reads at any depth, the queue bound, then the fixtures and the scenario on eight lanes, and every
   committed generated tree regenerated (Rust, Go, Kotlin and Swift `static_datamodel`; the Go round-trip tree where
   its source set takes the fixtures).
4. **The Rust machine holds every bound in its type** (section 7.4, added 2026-11-04), after step 3's Rust lane and
   before its other lanes: `sce-portable-bytes` gains the widening conversions and the operations a machine performs
   on a list, a text and a byte string, each refusing past its bound on both profiles; the Rust target spells every
   place of a machine (variables, records, payload structs, saved state, host views) in the bounded types, and every
   lowering of an assignment, an `<sce:append>`, a `<foreach>`, `len`, a comparison and a copy over them; the
   refusal of a composed value on a machine built `--no-std` goes; the Rust trees are regenerated.
5. **C11's slot**: `<SM>_DATA_LEN`, the runtime's views, the engine-allocated data of a script-engine machine,
   `SCE_MAX_DATA_LEN` retired with its `#error`, the tests that set it moved.
6. **Algorithms' slots and bodies** over every composition, on every backend.
7. **The deterministic CBOR encoder** with its vectors; unused for signing until ADR 0085 landing 5.
8. **The next revision of the authoring grammar**, after this ADR's landings: the legacy attributes are refused,
   with the same code and fix.
9. **The lower count of a list** (section 15, added 2026-11-06), in five changes: the notation in the model, the
   parser, the XSD's comment and the AST export, with its refusals and the revision rule; the JSON Schema's
   `minItems` and the GBNF chain, with the payload-schema fixture; the code `tooFew` in the standard library's
   refusal code, the reference harness's enum and every runtime's code; the count in every runtime's list reader
   and writer's check, and in the Rust composed lift; the two diagnostic codes. A send that lowers a composed value
   (step 3) calls the writer's count check where the build does not prove the count (section 15.6). No committed
   document states a lower count, so a tree changes only where the standard library's refusal code and the
   harness's enum are generated.

ADR 0085 landing 4 is the first user of steps 1 to 5, and its landing 5 of step 7. (Steps 5 to 8 were numbered 4
to 7 before the amendment of 2026-11-04 inserted step 4.) ADR 0085's amendment of 2026-11-08 applies this grammar to
a CBOR codec's entry, a bounded list of records, in its landing 3a, for the Mesh envelope's causal context; section
3.2's "the envelope does not change" stays true of the event fields this ADR adds.

## Consequences

- One type grammar, in which every constructor takes every type and every bound is written in the type; a
  document's author learns no list of exceptions and no second spelling.
- Every type has a largest JSON text and a largest CBOR encoding the build knows, and C11 sizes each machine's slot
  from them; the global guess `SCE_MAX_DATA_LEN` is gone, and no number replaces it in a document or a build.
- Five committed schemas gain a bound, and every committed document is rewritten to the bracket spelling; an
  application's documents are told each rewrite by a diagnostic until the next revision of the grammar refuses the
  attributes.
- Every bound is held at every depth on every backend: at the lift, at the send by the build, at an injected value
  by its constructor or raise.
- Signatures move to a standard encoding any CBOR library checks, at the one domain bump ADR 0085 already makes.
- More generated code per composition, and more cases in each runtime's reader and writer.
- No later revision raises a bound.
- A list states its fewest elements in its type (section 15), so an answer schema and its grammar leave out the
  lists an application cannot use, and every reader refuses them with `tooFew`; a later revision may lower a lower
  count and not raise it.

## Alternatives rejected

- **Restricting a list's element to fixed-width numbers, `bool` and records** (the first draft, and the language
  until now). It was the size of the implementation, not a principle: the stated reason, that a list of texts "has no
  size to declare", is false once a text states its bound, and the restriction was inconsistent, since a record
  element could hold the bounded text that `list<string>` could not.
- **Refusing nesting** (the first draft). Its reasons were the record rule as it stood, first normal form, and that a
  refusal can be lifted. The record rule was itself the restriction above; normalisation is a modelling choice an
  author may make, not one the type system needs to force; and a bound composes, so a nested type has a size like
  any other (ASN.1 has nested bounded sequences since its first edition).
- **Excluding enums from list elements.** An enum is a bounded value with a declared number; nothing about a list
  makes it different.
- **A global `SCE_MAX_DATA_LEN`, or an assertion against it** (the first draft and ADR 0085 section 5). The build
  knows the exact largest delivery of a bounded schema; a number guessed once for every machine wastes memory in
  most and is too small in some.
- **An unbounded top-level text or byte string kept legal** (the first revision of this ADR, after ADR 0077 section
  7). It left one kind of type without a largest value, so a schema holding one had no `DATA_MAX_LEN`, could not be a
  record type, and needed a number from outside the type to size a C11 slot. ADR 0077 section 7's reading is right
  for a frame of a protocol SCE does not own, and stays there; a value type of the language is SCE's own protocol,
  and states its bound.
- **A document-level `sce:max-data` for data the build cannot bound** (the first revision). It was
  `SCE_MAX_DATA_LEN` moved into the document. Once every type is bounded, a machine without a script engine has no
  such data, and one with a script engine allocates already.
- **The outermost bound as an attribute and inner bounds in the type** (the first revision). Two notations for one
  fact, chosen by depth; a type moved into a list would have to change spelling. One notation in the type keeps a
  type the same text wherever it stands, and the attributes stay only as synonyms for one revision of the grammar.
- **Refusing the attributes at once.** Every document written until now, the standard library's included, would stop
  checking in one landing. Codec format 1 was refused at once because a bare entry meant the opposite in format 2;
  a bound attribute means exactly what its bracket means, so it can be read for one revision while each document is
  told its rewrite.
- **A signing scheme of the trust layer's own** (length-prefixed texts, extended with a count for a list). It is
  injective, but no tool outside this repository reads it, every new type needs a new rule, and decimal texts are a
  second spelling of numbers CBOR spells natively.
- **Signing the JSON text in a canonical form** (RFC 8785). Its numbers are IEEE doubles, which cannot hold a 64-bit
  value, and it ties a signature to a text a host rewrites.
- **An enum signed as its name.** An open enum holds numbers that no name spells.
- **An absent optional as a missing key in the signed map.** It makes the key set depend on the value, and an absent
  value would sign like a field the sender's revision lacks.
- **A default other than the empty list**, **cutting a value past its bound to fit**, **repeated `<param>` names as a
  list**, **one event per element with an ordinal**: each invents or loses values, or brings back the reassembly ADR
  0085 removes.

## Owner's answers

The owner reviewed the first draft on 2026-11-03 and answered:

- **A. One orthogonal, fully composable bounded type system.** `list<T>` admits every type of the language as `T`:
  fixed-width numbers, `bool`, enums, bounded texts and byte strings, `optional<T>`, records of event schemas and
  lists, nested. A schema with a list field is a record type like any other. Every composition has a size the build
  knows, by the calculus of section 2. The first draft's restrictions are under "Alternatives rejected".
- **B. C11 sizes each machine's slot** from the largest encoding of the events it can receive, and the global
  `SCE_MAX_DATA_LEN` is retired, with the migration of section 8.3. Every composition is laid out statically.
- **C. Signed values are deterministic CBOR** (RFC 8949 section 4.2.1) from ADR 0085's one domain bump; the wire
  stays JSON. This ADR adds the encoder and its vectors; ADR 0085 switches the domain (section 9).
- **D. Kept from the first draft's recommendations**: a top-level bounded text is held when a payload is read, in
  this landing; the record list tells a register's values as one `values` list bounded by `shared_roster`, in ADR
  0085 landing 4; a later revision may not raise a bound (ADR 0044); the tests run on eight lanes. (The first
  draft's `sce:max-count` is superseded by answer F.)

The owner reviewed the revision the same day and answered:

- **E. Every type is bounded, without exception.** Every text and byte string of an event schema declares its bound,
  and one that does not is refused; ADR 0077 section 7 is amended to keep its "absent bound" for codec frames only.
  The five committed schemas that have none are migrated (section 1.4). No `sce:max-data` is introduced, and the C11
  slot is always computed (section 8). Keeping an unbounded top-level text legal is under "Alternatives rejected".
- **F. One bound notation.** A bound is written in the type at every depth, the outermost included
  (`list<string[8]>[4]`, `[name=N]` for a limit). `sce:max-count`, `sce:max-size` and `sce:capacity` are legacy
  synonyms of the outermost bound, reported with `validation/bound-attribute-superseded` in this landing and refused
  in the next revision of the grammar; both forms on one declaration are refused (sections 1.2, 1.3, 11).
- **Kept from the revision**: `optional<optional<T>>` refused, for its reason (section 1.5); an enum signed as its
  number, and an absent optional as `null` with its key present (section 9.2).

Decided while writing, within those answers, and open to the owner's review: an algorithm's `capacity=` and
`returns-max-size=` are legacy synonyms too, since they bound a value type as the three attributes do (section 1.3);
the bounds given to the five schemas (section 1.4); a machine with a script engine holds event data in storage its
engine allocates, since no type bounds a script value and such a machine allocates already, and a machine without
one keeps no data for an event no schema declares, since nothing can read it (section 8.1); the new code and the
new fix kind, `move_bound_into_type` (section 11).

The owner reviewed the first landing of step 3 on 2026-11-04 and answered:

- **G. Every place of a Rust machine holds its bound in its Rust type**, one representation per machine type:
  `list<T>[N]` is `sce_portable_bytes::BoundedList<T, N>`, `string[N]` `SceString<N>`, `bytes[N]` `SceBytes<N>`,
  `optional<T>` `Option<T>`, a record the machine's record, for variables, records and payload structs, at the top
  level and composed alike. The reasons are section 7.4's: an illegal value is unrepresentable, the machine and the
  schema unit hold one type and convert nothing between them, each proven fit is checked again by rustc through a
  widening conversion whose bounds are const parameters, and the types are inline without an allocator, so the
  `rust no_std` refusal of a composed value goes. C++ is the next target of the same reasoning, with bounded
  container templates in its runtime, in a later landing (landed 2026-11-09: section 7.4, `OwnedText<N>`,
  `OwnedBytes<N>`, `OwnedList<T, N>`); Kotlin, Go, Python and Swift keep checked constructors and
  bounds kept by the machine, having no constant parameter for a bound. The change of the generated public types is
  a consumer's migration, recorded with the landing (landing step 4).

The owner decided on 2026-11-06, from a measurement of the answer schemas under constrained decoding:

- **H. A list states its lower bound in its type, beside its upper bound**: "every bound, written in the type"
  covers the fewest elements. The grounds are section 15.1's: an answer schema from `list<T>[n]` carries
  `maxItems` and no `minItems`, so grammar-constrained decoding admits `[]`; a Gemma 4 model so constrained answered
  an empty list in 32 of 45 cases of a list that must hold at least 3 and in 36 of 55 of one that must hold at
  least 1; and the textbook remedy for a decoder drawn to the grammar's shortest path is to take that path out of
  the grammar, which is folded from the type. Raising a lower count is not additive, since an old value may then be
  refused; lowering one is. Decided while writing, within that answer, and open to the owner's review: the spelling
  `[M..B]` and `[M..name=N]`, a name moving the upper bound only (section 15.2); texts and byte strings left out
  (section 15.2); the lower count held at the two crossings rather than in a Rust type, and a send's run-time count
  check where the build does not prove it (section 15.6); a variable, an algorithm's slot and a list a later
  revision adds stating none (sections 15.7, 15.8); the GBNF chain in place of a repetition (section 15.10); the
  number 14 for `tooFew` and 15 for the harness's `notOffered` (section 15.11); and the two codes, the second with
  `replace_with` (section 15.13).
