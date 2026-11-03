# ADR 0086 — An event's field may hold a bounded, ordered list, and its bound is part of its type

- Status: Proposed (2026-11-03). Written at the owner's order of that day as landing 1 of ADR 0085 (its open question
  1, recommended there as an ADR of its own), under the rule of 2026-10-26 and 2026-10-27: the most long-term correct
  structure, textbook in its foundations, cost ignored. It waits for the owner's answers to "Open questions for the
  owner".
- Date: 2026-11-03
- Amends:
  - ADR 0042 section 2: its sentence "an event cannot carry a list" stops being true. What section 2 decides about
    the telling stays as it is in this landing (section 11);
  - ADR 0085 section 6: the bound is spelled `sce:max-count`, not `sce:capacity`. The Mesh carries the list inside the
    JSON text of `_event.data`, not as a CBOR array. A list a later revision added takes no `sce:default` (sections
    1.2, 2.2 and 7);
  - ADR 0076: a `<foreach>` over a list whose bound is declared has its passes counted (section 5.3);
  - SCE_FORGE.md section 4.12, the record rule: a record type is a schema that holds no list field (section 1.4).
- Scope:
  - the forge model (`FieldType` and a type of its own for a codec entry), the event-schema reader in
    `sce-build/src/forge/parser.rs`, the revision rules in `forge/schema_revision.rs`, the typing of `_event.data`
    in a `sce-static` statechart, the send-side rules, the internal-queue bound, the generator's schema units, lifts
    and send lowerings on every backend;
  - the payload readers and writers of every runtime: Rust, Go, Kotlin, Python, C++, Swift, C11, and the
    Interpreter's `sce_static.js`;
  - `schemas/sce-forge-ext.xsd` (documentation only), `apis/forge-ast.v1.schema.json`, the replay-scenario and
    scenario-set vocabularies;
  - `sce-rust-trust` (`message.rs`): how a list field enters the signed bytes. No signed event uses it until ADR 0085
    landing 5;
  - docs/SCE_ACCEPTED_SUBSET.md (sections 2.15 and "EventSchema kind"), SCE_FORGE.md section 4.12, ARCHITECTURE.md
    ("JSON Object Key Order").
- Related: ADR 0005 (bound and declare), ADR 0044 (the revisions of a schema), ADR 0050 (a bound read by its name,
  `sce:limit`), ADR 0055 (optional values), ADR 0065 (a codec is a total printer and a partial parser), ADR 0077 (a
  declared bound is part of a field's type), ADR 0085 (a change is one signed unit).

## Context

ADR 0085 sends a change of several operations as one event, `record_list.change`, whose `ops` is an ordered,
bounded list of records of a declared schema. An event schema cannot declare such a field today.

**What a field may be.** An event schema's field is typed by `FieldType`: a scalar or an optional scalar
(`sce-build/src/forge/model.rs:4821-4848`, "Neither has record or list members"). The reader,
`read_schema_field_type` (`parser.rs:12120-12160`), reads a scalar through `TypeGrammar::ScalarOrEnumRef` and refuses
everything else, `list<…>` included. The same reader and the same type serve the entries of a CBOR or content-line
codec (`parser.rs:2023`, `parser.rs:2341`). A content-line codec already has a list of its own, spelled
`sce:repeated="true"` with `sce:max-count` (`parser.rs:2471-2474`).

**Lists exist everywhere else.** An algorithm's parameter, return and local may be `list<T>`
(`AlgorithmValueType::List`, `model.rs:4605-4616`), and so may a `sce-static` statechart's variable, bounded by
`sce:capacity` (docs/SCE_ACCEPTED_SUBSET.md, "List variables" and "A list of records"). The element is one type,
`ListElemType`: a fixed-width number, `bool`, or a record of an imported event schema (`model.rs:4929`, with the
numbers `list_elem_admitted` takes at `model.rs:4687-4702`). A string or bytes element is refused
(`parser.rs:12061`: "a bounded list of them has no size to declare"). Each backend holds such a list in its own type: Rust `Vec<R>`, Kotlin `List<R>`, C++
`std::vector<R>`, Go `[]R` (`static_lowering.rs:3406`, `2493`, `7237`, `8434`). C11 uses
`{ size_t len; R data[N]; }` (`bounded_record_list`, `static_lowering.rs:10810`). The record rule (SCE_FORGE.md
section 4.12) says a record's fields are fixed-width numbers, `bool`, an enum or a bounded string. A record is held
and copied by value.

**The wire already carries arrays.** `_event.data` is a JSON object (Rust `PayloadFields::decode`,
`backends/rust/runtime/src/event_payload.rs:76-100`). Its keys come in ascending order by UTF-8 bytes "at every
depth", and an array keeps its order. A `<param>` name that repeats collects its values into an array
(ARCHITECTURE.md:826-835). C11's reader can already step past a nested array
(`sce_payload_skip_container`, `backends/c/runtime/include/sce/event_payload.h:121`). A 64-bit integer travels as
the text of its digits (`event_payload.rs:204-240`).

**Nothing reads or writes one.** Every runtime's reader looks a field up by its name in a flat object and reads one
scalar there. These readers are Rust `PayloadFields`, Go `LiftPayload` (`backends/go/runtime/event_payload.go:43`),
Kotlin `EventPayload.decode` (`EventPayload.kt:241`), Python `_decode`/`_as` (`event_payload.py:148`, `189`), C++
`EventPayloadFields` (`sce/include/common/EventPayloadLift.h:56`), Swift `PayloadFields` (`EventPayload.swift:46`),
C11 `sce_payload_*` and the Interpreter's `SceStatic.field` (`sce-build/src/forge/static_js/sce_static.js:215`).
On the send side, a `<param>` value has to have a wire slot. A list has none (`InferredType::wire_param_slot`,
`forge/types.rs:688-700`; refused at `static_lowering.rs:14234-14236`). Section 2.15 also refuses a list read as a
value.

**Where a bound is held, and where it is not.** A text field of an event schema with `sce:max-size` is not held to
that bound when the payload is lifted (`PayloadFields::text`, `event_payload.rs:303-308`). It is held only when an
assignment takes it into a variable (SCE_FORGE.md section 4.12). Rust's growable list, `HeapList`, treats its `N` as
advisory (`backends/rust/portable-bytes/src/lib.rs:53-71`). C11 keeps a delivery in a fixed slot,
`SCE_MAX_DATA_LEN` (256 by default, `backends/c/runtime/include/sce/types.h:76`). A C11 send refuses a payload that
does not fit (`sce_forge_wire_end`, `tools/codegen/templates/c/state_machine.c.jinja2:131-137`), but a copy into
the slot cuts the text short (`sce_copy_bounded_data`, `types.h:183-194`, `330`).

**Around the payload.** The Mesh carries `_event.data` as its JSON text (`PayloadCodec::Json`). A CBOR or typed
payload is refused when it arrives (`backends/rust/mesh/src/router.rs:670-685`). A saved state keeps a waiting send's
data as text (`schemas/sce-saved-state.v2.schema.json:181`, `240`). The trust layer signs typed values in a fixed
order, not the JSON text (`backends/rust/trust/src/message.rs:4-23`, `355-421`). It knows three kinds of value:
unsigned, signed and text (`message.rs:48-53`). The replay vocabulary already types a variable's list of records
(`Holds::List`, `sce-build/src/scenario_replay.rs:722-790`). It types every field of a payload as a present scalar
(`Holds::of_schema`, `scenario_replay.rs:772`). The scenario set refuses "a list as a payload value"
(`sce-build/src/scenario_set.rs:1266`). The internal-queue bound marks a `<foreach>` whose body raises an event as
not proven, because it does not count the passes (`sce-build/src/internal_queue_bound.rs:479-492`).

**Why ADR 0042 said no.** ADR 0042 section 2 reads: "a parent statechart cannot [read `values`], and an event cannot
carry a list". So the list tells its parent one value per event, `record_list.register` followed by
`record_list.value` with an ordinal. The reason given is what the language could not do, not something the protocol
needed.

## The textbook structure

**A bounded sequence is a refinement type.** The value set of a list of `T` bounded by `N` is
`{ xs : [T] | len xs <= N }`. This is the refinement-type reading ADR 0077 gives a bounded byte string (Rondon,
Kawaguchi and Jhala, "Liquid Types", PLDI 2008). Interface languages meant for nodes of finite memory put the bound
in the type itself:

- ASN.1: `SEQUENCE (SIZE (0..N)) OF T` (ITU-T X.680, section 51, size constraint);
- XDR: the variable-length array `T x<N>` (RFC 4506 section 4.13);
- CDDL: `[ * N T ]` with the occurrence `0*N` (RFC 8610 section 3.2).

Protocol Buffers leaves a repeated field unbounded. That is why its embedded profiles add a `max_count` to each one.
So the bound belongs to the field's type and is the same on every node. A decoder accepts exactly the values of that
type and refuses the rest before it stores or allocates anything (King, "Parse, don't validate", 2019; ADR 0077
section 2).

**A list is ordered, and order is part of the value.** A JSON array is "an ordered sequence of zero or more values"
(RFC 8259 section 5). `[a, b]` and `[b, a]` are two values, and every encoding of the list, the signed one
included, keeps them apart.

**Nesting is normalised away.** A repeating group inside a repeating group is the case first normal form removes
(Codd, "A Relational Model of Data for Large Shared Data Banks", CACM 13(6), 1970). A list of records whose records
hold lists becomes two flat lists, with a key that links them. The flat form also keeps every record a plain value
(SCE_FORGE.md section 4.12).

**Sign the abstract value, not one of its texts.** A distinguished encoding writes each abstract value as exactly one
string of bytes (ITU-T X.690, DER). This trust layer already works that way: each value of an event is written in a
fixed order, each with its length in front (`message.rs:355-385`), so a host that rewrites the JSON changes no signed
byte. XDR writes a variable-length array as its count followed by its elements (RFC 4506 section 4.13). A count in
front, with every part length-prefixed, gives a prefix code, so two different values never give the same bytes.

**Adding a repeated field is a compatible change when absent means empty.** In Protocol Buffers' proto3, a repeated
field a sender does not write reads as empty. In Avro, a reader's schema gives a default to any field the writer did
not have. A list that a later revision of a schema adds is therefore read as empty from a delivery made under an
earlier revision.

## Decision

**An event schema's field may be `list<E>`. Its bound, `sce:max-count`, is required and is part of the field's type.
`E` is the element every other list in the language admits: a fixed-width number, `bool`, or a record of an event
schema that holds no list. On the wire it is a JSON array in list order. Every reader refuses a list longer than the
bound, and any element that is not a value of `E`, before it stores the list. No sender can write a list longer than
the bound: the build refuses such a send, and the runtime refuses such an injected value when it is made. A signature
covers the list as its count and then its elements, in order.**

### 1. The grammar

1. **The spelling.** `<data id="ops" sce:type="list&lt;record:Op&gt;" sce:max-count="64" sce:limit="shared_parts"
   sce:direction="in"/>`. `sce:max-count` is required and is a positive integer of 32 bits. The XSD already types the
   global attribute as `xs:positiveInteger` (`schemas/sce-forge-ext.xsd:578`). `sce:limit` names it for a build to
   give another number (ADR 0050), as it already does beside a schema text's `sce:max-size`.
2. **Why `sce:max-count`, not `sce:capacity`.** ADR 0077 section 7 separates two numbers. A **bound** belongs to the
   protocol and is part of the language (`sce:max-size`, `max-count`). A **capacity** is how much a storing form holds
   (`sce:capacity` on a variable). An event schema states a protocol, and its text fields already bound themselves
   with `sce:max-size`. A content-line codec already bounds a repeated property with `sce:max-count`. The variable a
   list is taken into keeps `sce:capacity` (section 5). ADR 0085 section 6 wrote `sce:capacity`, and this replaces it.
3. **The element** is exactly `ListElemType`, read by the same arm of the reader that reads an algorithm's list and
   a variable's list. That way the three positions cannot disagree about what a list holds, and a payload's list is
   the same type as a variable's list. The reader refuses:
   - a `string` or `bytes` element, for the reason `parser.rs:12061` already gives. A list of texts is a list of
     records with one bounded text field;
   - an `optional<…>` element: an absent element is not an element (ADR 0055);
   - a `list<…>` element, by item 4;
   - an `enum:` element, which `ListElemType` does not admit anywhere today. Widening it is one decision for every
     list position (owner question 2), and this ADR does not make it for one position alone.
4. **No nesting.** A record element's schema may not declare a list field. Such a schema is not a record type, so
   `record:<alias>` naming it is refused everywhere: as an element, a record variable, an algorithm's slot, or a
   host-run request or result. This is the refusal the record rule already gives a string field with no bound.
   - The record rule makes a record a value: copied by assignment in C11, `Copy` or `Clone` in Rust, compared, and
     appended whole. A list inside makes it a container. Every place that holds, copies, compares or saves a record
     would then need bounded storage inside a value, and C11's element size would become a product of bounds.
   - First normal form gives the same structure flat.
   - No need exists: `record_change.ops` is a flat list.
   - The choice can be undone later without breaking anything: a refusal can be lifted additively, and an admission
     cannot be withdrawn.
   - Recursion (a schema whose element is itself, directly or through another schema) is refused by the same rule
     before any cycle check is reached.
5. **An event schema may import an event schema** (`<sce:import kind="event-schema">`), so that an element can name
   it. The alias is judged as `read_record_alias` (`parser.rs`) judges every `record:` alias.
6. **The model.** `FieldType` gains a third constructor, `List { elem: ListElemType, max_count: u32, limit:
   Option<String> }`, with the bound inside the type, not beside it. That is what makes the bound part of the type
   in the code:
   - ADR 0044's comparison of two schemas (`schema_revision::divergence`) compares the bound with the rest of the
     field;
   - the AST export carries it;
   - no reader can reach the element without the bound.

   `FieldType::present_type()` has no answer for a list. Every reader that calls it today then has to say what it
   does with a list, which is why `FieldType` is a constructor and not a flag (its own doc comment).
7. **A codec entry gets a type of its own.** The CBOR and content-line codec entries move off `FieldType` onto
   `EntryType` (a scalar or an optional scalar), read by a reader that admits only those. A content-line list is
   `sce:repeated`, and admitting `list<…>` there would give one fact two spellings. Making the split a type, and not a
   check, keeps the compiler naming every entry reader.

### 2. The wire form

1. **JSON.** A list field is a JSON array in `_event.data`, with its elements in list order.
   - A scalar element is spelled as a scalar field of its type is spelled: a 64-bit integer as the text of its
     digits, a real as a number, a truth value as `true` or `false`.
   - A record element is a JSON object of its schema's fields. Its keys come in ascending order by UTF-8 bytes,
     because ARCHITECTURE.md's order holds "at every depth". Each field is spelled as a top-level field of its type
     is, and an optional field is `null` when absent, wherever the record rule admits one.
   - The empty list is `[]`. A writer of the revision that declares the field always writes the key, as an optional
     field is always written as `null` (`optional_json`, `event_payload.rs:383`).
2. **The Mesh carries it unchanged.** The list travels inside the JSON text under `PayloadCodec::Json`
   (`router.rs:670-685`). Neither the envelope nor the CBOR and typed payload codecs change, and those two stay
   refused when they arrive. This corrects ADR 0085 section 6, which said the list would travel as "a CBOR array".
3. **A saved state is unaffected.** A waiting send's data is held as its text.
4. **The bytes a signature covers.** These are not the JSON text. `Kind` (`message.rs:48`) gains `List`, whose
   element is one of the scalar kinds or the fields of the element's schema. `build` (`message.rs:355`) writes a list
   field as:
   - `part(count)`, the count in decimal;
   - then each element in list order. A scalar element is one part, in the text its kind already has. A record
     element is one part per field, in the order the element's schema declares its fields. JSON key order plays no
     part. A field that a later revision of the element schema added, and that an older element lacks, is written as
     its default.

   Every part is length-prefixed. The count fixes how many elements follow, and each element adds a number of parts
   fixed by its schema. So the bytes parse back one way only, and two different lists never sign alike: `[1, 23]` and
   `[12, 3]` differ, and so do `[a, b]` and `[b, a]`. A list field that a later revision added, absent from a
   delivery, is written as `part("0")`, the empty list. An event that gains a list field gains a new meaning and so a
   new domain. ADR 0085 takes that step once, as `sce-trust/event/3`, and this ADR changes no signed event.
5. **The unit names its largest delivery.** A schema unit already declares its event, its revision and the bound of
   each text (docs/SCE_ACCEPTED_SUBSET.md, "The unit names its event and its revision"). It now also declares
   `<FIELD>_MAX_COUNT` for each list field and `DATA_MAX_LEN`, the length in bytes of the longest `_event.data` text
   the schema admits. The generator computes that length from the bounds, at every depth:
   - braces, keys, colons and commas;
   - each number type's longest spelling (a 64-bit integer as its quoted digits);
   - each bounded text at its worst escape (six bytes for each byte written `\u00XX`, plus the quotes);
   - each enum's longest name;
   - a list as `max_count` times its element, plus the commas and brackets.

   A schema with an unbounded field has no `DATA_MAX_LEN`. One function in the generator computes the number, and
   both C11 (section 6) and any transport that sizes a frame read it.

### 3. Validation where a delivery is read

1. **Every lift refuses before it stores.** A list field is refused when it is:
   - not an array;
   - longer than `max_count`. The elements are counted, and the walk stops at the first element past the bound,
     before decoding it (ADR 0077 section 2);
   - holding an element that is no value of `E`. The element is judged exactly as a field of that type is judged:
     width, digits, enum name, truth value.

   A record element must be an object holding every field of its schema. A field it lacks is read from the element
   schema's own defaults when that schema grew (section 7). Its text must fit its `sce:max-size`, and this is checked
   at the lift, not later: a list holds an element whole and copies it whole, so no assignment ever judges one of its
   fields. A key the element's schema does not declare is stepped over, as one in the payload is
   (`event_payload.rs:63-65`).
2. **The delivery is refused whole.** One bad element refuses the whole delivery. The transition does not run, and
   `error.execution` is raised once, as for any payload that does not fit (`scenarios/static_whole_payload.json`). The
   reason names the place as a JSON Pointer (RFC 6901): `'items' holds 4 elements, past the 3 its schema declares`,
   `'items/2/label' is 9 bytes, past the 8 its schema declares`.
3. **One reader shape per runtime.** Each runtime's scalar readers become readers of one value at one path. A field
   read by its name and an element read by its index then use the same code, and a list adds one walk:
   - Rust `PayloadFields`; Go `LiftPayload`; Kotlin `EventPayload.Fields`; Python `_decode`/`_as`; C++
     `EventPayloadFields`; Swift `PayloadFields`;
   - C11 adds a cursor over an array, made of `sce_payload_skip_value` and an `sce_payload_fields_t` over each
     element's object. It stays header-only, does not allocate, and keeps one static sentence per refusal, as the
     header requires;
   - the Interpreter: `SceStatic.field` takes the type `{list: E, max: N}` (with `{record: {...}}` for a record
     element) and throws as the generated lifts refuse. The Interpreter still raises one `error.execution` for each
     expression that reads the payload, a difference docs/SCE_ACCEPTED_SUBSET.md section 2.15 already states.
4. **A top-level text field.** A bounded text field outside a list is still held to its bound only when it is
   assigned. Owner question 4 asks whether to close that gap in the same landing.

### 4. Validation where a delivery is written

1. **A statechart sends a list by its name.** `<param name="ops" expr="staged"/>` names a list whole. The name is
   either a list variable or, to forward one, the list field of the payload of the event the transition is on. The
   send is accepted when both of these hold:
   - the element is the field's element: the same element-schema document for records (a record type is its
     document, SCE_FORGE.md section 4.12), or the same scalar type;
   - the source's bound is no more than the field's `sce:max-count`, each read after `sce:limit`. A variable's bound
     is its `sce:capacity`, and a payload field's bound is its `sce:max-count`.

   Otherwise the send is refused as `scxml/static-datamodel-rule`, naming the field, its reader and both bounds. Two
   existing rules, "What a send carries is held to the field that reads it" and "A text is held to the bound of the
   field that reads it" (docs/SCE_ACCEPTED_SUBSET.md:3907, `:3927`), extend here to the element and to the count. To
   narrow a list, the author appends into a list variable of the field's capacity. The build has then proved that the
   list fits, so the send has nothing to check when it runs. A list written as an expression that is not a name is
   refused (`expression/unsupported-construct`), as a list read as a value already is.
2. **The lowering.** `wire_param_slot` gains a list slot, and `lower_wire_value` (`static_lowering.rs:14124`) gains a
   list arm that writes through the runtime's writer. The runtimes listed in ARCHITECTURE.md's key-order table each
   gain one array writer, held to `tests/json_text/object_key_order.json` with a case of a list of records. C11's
   `sce_forge_wire` (`backends/c/forge-runtime/include/sce/forge/wire.h`) gains the opening and closing of an array
   and of an element object. It already closes an array for a repeated name (`wire.h:300`). A send past the C11
   buffer is still refused by `sce_forge_wire_end`.
3. **A host's injected payload is refused where it is made.** ADR 0077 section 3 applies:
   - Rust: the field of a schema unit's payload struct is `BoundedList<R, N>`, a new type in `sce-portable-bytes`.
     It keeps storage of either profile (on the heap under `alloc`, inline otherwise), and every constructor of it
     refuses more than `N` elements on both profiles. `SceOwnedList` cannot serve, because its `N` is advisory under
     `alloc`;
   - Kotlin: an `init` block with `require(ops.size <= N)`. Python: a `__post_init__` that checks the same;
   - Go, C++ and Swift have no checked constructor on a plain struct. The generated `raise_<event>` refuses the
     value there, before encoding it, from the same bound;
   - C11: `{ size_t len; R data[N]; }`, and the encode refuses a `len` past `N`.

   Data a host writes itself, without any of these seams, is judged when it is read (section 3).

### 5. A statechart reads a payload's list as it reads a list variable

1. **What reads it.**
   - `len(_event.data.ops)`;
   - `_event.data.ops[i]` for a list of numbers, and `_event.data.ops[i].field` for a list of records, under the
     index rules a list variable has (an index out of range fails, and the statement it stands in is skipped);
   - `<foreach array="_event.data.ops" item="op" index="i">`, with `op` a closed record of the element's schema;
   - `<assign location="kept" expr="_event.data.ops"/>`, which takes the list whole into a list variable whose element
     is the same and whose `sce:capacity` is at least the field's bound. Such a copy cannot fail. A smaller capacity
     is refused (`scxml/static-datamodel-rule`), naming the narrowing to use instead: an append in a `<foreach>`.
     This is the list's counterpart of "A payload taken whole".

   A list as an operand, a list compared to a list, or a guard that compares `_event.data.ops` to a literal is
   refused (`expression/unsupported-construct`, `validation/cross-kind-type-mismatch`).
2. **The typing.** The path a `sce-static` scope gives a payload field (`forge/static_datamodel.rs`, "the payload of
   the event the transition is on") types a list field as `InferredType::List(ListElem)`, the type a list variable
   already has. So no rule of section 2.15 has to be written twice.
3. **The queue bound** (ADR 0076). A `<foreach>` over a list with a declared bound runs its body at most that many
   times: a variable's `sce:capacity`, or a payload field's `sce:max-count`. Its cost is the body's cost times the
   bound (`internal_queue_bound.rs:479-492`). Today a body that raises an event is marked not proven, so a receiver
   that raises one event per element, as ADR 0085's does, could never be proven.

### 6. The types each backend generates

A schema unit is the type a host builds or reads a payload with. A machine's payload is what its lift fills; for a
list field, it holds the type a list variable of the same element already has, so a whole copy (section 5.1) is
an ordinary copy.

| Backend | Schema unit's field | Machine payload's field | The bound is held by |
|---|---|---|---|
| Rust | `::sce_portable_bytes::BoundedList<R, N>` | `Vec<R>` | the type's constructors; the lift |
| Kotlin | `List<R>` | `List<R>` | `init { require(...) }`; the lift |
| C++ | `std::vector<R>` | `std::vector<R>` | `raise_<event>`; the lift |
| Go | `[]R` | `[]R` | `Raise<Event>`; the lift |
| Python | `list[R]` | `list[R]` | `__post_init__`; the lift |
| Swift | `[R]` | `[R]` | `raise<Event>`; the lift |
| C11 | `{ size_t len; R data[N]; }` | `<record>_list_<N>_t` (`bounded_record_list`) | the encode; the lift; the storage |
| Interpreter | an array of plain objects | the same | `SceStatic.field`; `SceStatic.within` (`sce_static.js:393`) |

`R` is the element's payload type (`event_schema_payload_type`, `generator.rs:3263`) in a schema unit, and the
machine's own record type in a machine. `N` is `max_count` after `sce:limit`.

**C11 sizes everything at build time.** The list's storage is `N` elements of a fixed-size record. The event slot
must also hold the longest delivery. So the generated machine header carries
`_Static_assert(SCE_MAX_DATA_LEN >= <n>, "...")`, where `<n>` is the largest `DATA_MAX_LEN` (section 2.5) of every
schema whose event the machine reads or sends. A build whose slot is too small fails to compile; it does not cut a
delivery at run time. The header also declares `<MACHINE>_DATA_MAX_LEN` as that number, so a build can set
`-DSCE_MAX_DATA_LEN` from it. This follows two assertions the backend already makes: a literal payload must fit
(`state_machine.c.jinja2:151`), and so must a generated send id (`types.h:200-203`). A host's raise of data longer
than the slot is refused, not cut. A cut payload was already refused as malformed, but with the wrong reason. ADR
0085 section 5 relies on this section.

**A backend that does not lower a list field yet refuses it by name.** It refuses the field with
`generate/unsupported-feature`, as Swift refuses what it does not lower today (`swift_event_schema_unlowered`,
`generator.rs:28396`). It never reads the field as something else.

### 7. Revisions of a schema unit that has a list field (ADR 0044)

1. **A list field of the first revision** is carried by every delivery; an empty list is written `[]`.
2. **A list field a later revision adds** declares `sce:since="k"` and has no `sce:default`. A delivery without it
   reads the empty list, and `defaults_wire` (`schema_revision.rs:144`) writes `[]` for it. Writing `sce:default` on
   a list field is refused (`validation/attribute-rule-violated`) for two reasons:
   - the empty list is the only value that means "the sender said nothing";
   - the language has no record literal in which to write any other list (SCE_FORGE.md section 4.12).

   This mirrors an optional field, which may leave its default out and then reads as absent (`parser.rs`,
   `read_field_revision`).
3. **An element schema may grow under its own revisions.** Each element is read with its own schema's defaults.
   ADR 0044's rule (a), "declared alike", judges a list field's element by applying the same rule to the two
   element schemas. It does not compare their texts. So two builds whose element schemas are revisions of one schema
   declare one field, and the Mesh check (`mesh/event-schema-mismatch`) admits them.
4. **What stays refused across revisions** is what rule (a) already refuses: another bound, another element, or a
   list turned into a scalar or back. Raising a bound is safe for a reader and unsafe for an older receiver of a
   newer delivery. The symmetric rule therefore refuses it (owner question 6). `sce:limit` moves the bound for the
   whole build, both ends of a deployment together.
5. **`sce:optional-since` does not apply**, and `optional<list<…>>` is refused. For a list, absent and empty are one
   value.

### 8. Diagnostics

The refusals are existing classes of fact, so no `DiagnosticCode` is added and the appendix that lists every code
in docs/SCE_ACCEPTED_SUBSET.md gains no row. The goldens of the sites below gain cases.

| Refusal | Code | Where |
|---|---|---|
| `list<…>` with no `sce:max-count` | `validation/missing-attribute` | parse |
| `sce:max-count` past 32 bits; written on a field that is not a list | `validation/attribute-rule-violated` | parse |
| `sce:capacity` on an event schema's field (the rule names `sce:max-count`) | `validation/attribute-rule-violated` | parse |
| a `string`, `bytes`, `optional`, `list` or `enum` element | `validation/attribute-rule-violated` | parse |
| a `record:` element whose alias the schema does not import | `validation/invalid-attribute`, with the candidates | parse |
| a record element whose schema holds a list (nesting, recursion) | `validation/attribute-rule-violated` | import resolution |
| `record:<alias>` naming a schema with a list anywhere else | `validation/attribute-rule-violated`; for a host-run request or result `validation/typed-invoke-schema` | import resolution |
| `sce:default` on a list field | `validation/attribute-rule-violated` | parse |
| `list<…>` on a CBOR or content-line entry | `validation/invalid-attribute`, with the entry types | parse |
| a list sent past the field's bound, or with another element | `scxml/static-datamodel-rule` | check |
| a list taken whole into a variable of a smaller capacity | `scxml/static-datamodel-rule` | check |
| a list read as an operand, or compared | `expression/unsupported-construct`, `validation/cross-kind-type-mismatch` | check |
| two machines whose list fields are not one field | `mesh/event-schema-mismatch` | deploy |
| a backend that does not lower a list field | `generate/unsupported-feature` | generate |

At run time, a refusal is a sentence carried by `error.execution`, as every lift refusal is. It is not a diagnostic.

### 9. Wire surfaces (SCE_WIRE_CONTRACTS.md)

No surface flips its status.

- **Authoring grammar** (`pre-release`). `dataValueType` already admits `list<…>` lexically
  (`sce-forge-ext.xsd:131-160`, `275-283`), so no pattern changes. Three comments change, the ones that say only a
  statechart's variable admits a list: `dataValueType` (`275-283`), `sce:capacity` (`296-303`) and `sce:max-count`
  (`578`), which gains the event-schema field. The change is additive.
- **Forge AST export** (`pre-release`). A field's type may be `{"list": <element>, "max_count": N}`. The schema file
  is regenerated from the model. The change is additive.
- **Replay scenario** (`registered`). A payload's list field is an array: `Holds::of_schema` types it as
  `Holds::List`, so a `null` inside an element is judged by the element's schema. The vocabulary checker and every
  driver read it in the same change, as that surface's row requires.
- **Scenario set** (`pre-release`). Its refusal of "a list as a payload value" becomes a refusal of an array where
  the machine's schema declares no list.
- `_event.data` itself has no row. Its shape is held by this ADR, by ARCHITECTURE.md's key order and by
  docs/SCE_ACCEPTED_SUBSET.md.

### 10. Tests on every lane

Every test below is written to fail when the rule it pins is removed. The landing checks each one once against a
mutant of that rule.

1. **Fixtures** (new, under `sce-build/tests/fixtures/static_datamodel/`):
   - `event_batch_item.scxml`, the element: `id` `uint32`, `amount` `int64`, `label` `string` with `sce:max-size="8"`;
   - `event_batch.scxml`, `batch.sent`, revision 2: `items` `list<record:Item>` with `sce:max-count="3"
     sce:limit="batch_items"`; `marks` `list<uint16>` with `sce:max-count="4"`; `tag` `uint8`; and `late`
     `list<uint8>` with `sce:max-count="2" sce:since="2"`;
   - `static_list_payload.scxml`, which takes `batch.sent` and does each thing section 5 admits: it copies `items`
     whole into `kept` (`sce:capacity="3"`), walks it with `<foreach>` summing `amount`, counts `marks` with `len` and
     reads `marks[len(marks) - 1]`. On `batch.echo` it sends `batch.sent` to `#_internal` with `<param name="items"
     expr="kept"/>`, so that a list it sent comes back through its own lift.
2. **The scenario** `scenarios/static_list_payload.json`. Its expected values are derived from the documents, not
   observed from a backend. Its steps:
   - the empty list;
   - the maximal list: 3 items and 4 marks;
   - one element past each bound, refused whole with nothing written;
   - a `label` of 9 bytes in the second element, refused;
   - an element missing `amount`, refused;
   - an `id` past `uint32`, refused;
   - an `amount` past 2^53 written as digits, accepted;
   - `items` given as an object, refused;
   - an element with a key its schema does not declare, accepted;
   - a delivery of revision 1 without `late`, read as empty;
   - the round trip: the `sent` data is the canonical text (keys ascending at every depth) and the list read back
     equals the list sent;
   - the order kept: `[a, b]` read back as `[a, b]`.

   The scenario replays on Rust (`backends/rust/tests/tests/static_scenarios.rs`,
   `backends/rust/scenarios/src/machines.rs`), Kotlin (`StaticScenarioTest.kt`), Python (`test_static_scenarios.py`),
   Go (`static_scenarios_test.go`), C11 (`test_static_scalars.c`), Swift (`StaticDatamodelTests`, `Replay.swift`) and
   the Interpreter (`AStaticDatamodelRunsLoweredUnderTheInterpreterTest.cpp`). C++ derives its suite from the
   directory.
3. **Each runtime's reader and writer**, in its own unit tests. The cases are:
   - the bound refused, and the element past it never decoded (a malformed element past the bound gets the bound's
     refusal);
   - the empty list, and the maximal list;
   - the path in each refusal;
   - an array of records written with ascending keys, and read back unchanged.
4. **`sce-build`**:
   - a golden for each refusal of section 8;
   - `a_schema_unit_names_its_event_revision_and_bounds.rs` gains `ITEMS_MAX_COUNT` and `DATA_MAX_LEN`;
   - `DATA_MAX_LEN` is tight: the Rust writer's text of a generated maximal value of the schema is exactly that long;
   - `a_send_is_held_to_the_schema_its_receiver_reads.rs` gains the count rule and the element rule;
   - the queue bound proves a `<foreach>` over a bounded payload list that raises once per pass;
   - the C11 header carries the assertion with the computed number, and a C11 build with `SCE_MAX_DATA_LEN` one
     below it fails to compile while one at the number compiles and carries the maximal list.
5. **Trust**, in `message.rs`:
   - `Kind::List` is injective (`[1, 23]` against `[12, 3]`, and one element against two);
   - order matters;
   - an absent list added by a later revision signs as `[]`;
   - the signed bytes of a record element follow the schema's field order whatever the JSON key order.
6. **Mutation casefiles** under `sce-build/tests/mutations`: the count comparison in each runtime's lift, the
   element's text bound, the C11 assertion, the send-side count rule, and the count prefix in the signed bytes.

### 11. ADR 0042, and why its reason no longer holds

ADR 0042 section 2 announces a register in one event and each further value in another, because "an event cannot
carry a list". That was a limit of the language, not a choice the protocol needed. After this ADR it no longer holds.
What section 2 decides stays true and is not touched by this landing:

- only registers that moved are told;
- they are told once the change is whole;
- they are told to a parent, and only when the list was started with `announce`.

ADR 0085 landing 1 moves nothing of the list. Whether `record_list.register` should carry all of a register's live
values as one `values` list, in place of `record_list.value` with an ordinal, is a change to the list's protocol.
Owner question 5 asks that. The constraint that once explained the absence of lists also holds in a different form:
a receiver has to be able to hold whatever a delivery carries. The bound in the type is what meets it now.

Section 1's third point ("a record is spread over several fields of one row") describes how the list stores what an
application writes. It does not depend on what an event can carry, and it stands.

## Landing order

1. **The model and the grammar.** `FieldType::List`, the codec entry's own `EntryType`, the reader and its
   refusals, the record rule's new refusal, the revision rules, the XSD comments, and the AST export. Every backend
   refuses a list field by name (`generate/unsupported-feature`), so nothing half-works in between.
2. **The runtimes.** The reader of one value at one path, the list walk and the array writer in each of the eight,
   with their unit tests; `BoundedList` in `sce-portable-bytes`.
3. **The generator.** Schema units (types, `_MAX_COUNT`, `DATA_MAX_LEN`, the C11 assertion), lifts, send lowering,
   a statechart's reads, and the queue bound. Then the fixtures and the scenario on every lane, and every committed
   generated tree regenerated: Rust, Go, Kotlin and Swift `static_datamodel`, and the Go round-trip tree if its
   source set takes the fixtures.
4. **The trust layer's `Kind::List`**, with its tests. No signed event uses it until ADR 0085 landing 5.

ADR 0085 landing 4 is the first user.

## Consequences

- An event can carry a bounded, ordered list of numbers or of records, with the same meaning on eight engines.
- A list's bound is held the same way everywhere: at the lift on every backend, at the send by the build, and at an
  injected value by its constructor or its raise. In C11 it is held at build time by the slot's size.
- A record type is defined one degree more tightly: a schema with a list field is a message, not a record.
- `DATA_MAX_LEN` gives every transport the size of the largest delivery of a bounded schema.
- One more case in each runtime's payload reader and writer, and in each generated lift and send.
- A schema can no longer raise a list's bound without becoming a different field.

## Alternatives rejected

- **An unbounded list, bounded only by each backend's storage.** ADR 0077 rejected exactly this for codec fields: a
  frame one backend refuses and another accepts means the backends implement two codecs.
- **`sce:capacity` as the spelling** (ADR 0085 section 6). It names what a variable stores. An event schema states the
  protocol, as `sce:max-size` and `sce:max-count` do (section 1.2).
- **Nesting admitted now.** See section 1.4. It can be admitted later without breaking anything, while an admission
  could not later be withdrawn.
- **A list of texts as an element.** Each text needs a bound of its own, and a record of one bounded text field
  already gives it one, within the record rule.
- **Repeated `<param>` names**, the array a repeated name collects. It has no type, no bound and no record elements,
  and a single value is not distinguishable from a list of one.
- **One event per element with an ordinal** (ADR 0042's telling, ADR 0046's parts). This is the reassembly that ADR
  0085 removes.
- **The list as a text holding JSON.** That needs a second parser inside every reader and leaves the list untyped.
- **Signing the JSON text in a canonical form** (RFC 8785). Its numbers are IEEE doubles, which cannot hold a 64-bit
  value exactly, and it ties a signature to a text that a host rewrites. The trust layer signs typed values for that
  reason (`message.rs:4-10`).
- **A default other than the empty list.** It would invent elements no sender sent, and the language has no record
  literal to write them in.
- **Cutting a list past the bound to fit.** A cut list is another value; re-encoding it does not give the frame back.

## Open questions for the owner

1. **The spelling of the bound: `sce:max-count` or `sce:capacity`.** ADR 0085 section 6 wrote `sce:capacity`.
   Recommendation: `sce:max-count`, because the schema states the protocol's bound (ADR 0077 section 7) and the
   language already spells a repeated value's bound that way.
2. **Enum elements (`list<enum:E>`).** `ListElemType` admits no enum in any position today. Recommendation: not in
   this ADR. Widening `ListElemType` is one decision for algorithms, variables and fields together, so it should be
   an ADR of its own. Until then, a record with one enum field carries an enum value.
3. **Nesting.** Recommendation: refuse it (section 1.4), for the record rule, first normal form, and because the
   refusal can be lifted later without breaking anything.
4. **A top-level text field held to its bound at the lift.** Today such a field is held only when it is assigned
   (`event_payload.rs:303-308`); this ADR holds a text inside a list element at the lift. Recommendation: hold every
   bounded text at the lift in the same landing, so that one payload has one rule (ADR 0077 section 2).
5. **ADR 0042's telling as one event with a list.** Recommendation: yes, in ADR 0085 landing 4. Its bound would be
   `shared_roster`, since a register holds at most one live value for each member who wrote it at the same time.
6. **Raising a list's bound in a later revision.** Recommendation: refuse it, as ADR 0044 rule (a) refuses any
   re-bound. A deployment raises the bound for every node at once with `sce:limit`. The alternative is a widening
   marker like `sce:optional-since`, which would leave older receivers refusing newer deliveries.
7. **C11's slot.** Recommendation: the build-time assertion against `SCE_MAX_DATA_LEN` together with a generated
   `<MACHINE>_DATA_MAX_LEN`, as ADR 0085 section 5 asks. The alternative is a slot sized per machine, which every
   machine sharing one build's queue types would have to agree on.
