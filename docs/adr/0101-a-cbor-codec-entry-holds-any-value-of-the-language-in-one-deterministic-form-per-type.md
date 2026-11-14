# ADR 0101 — A CBOR codec's entry holds any value of the language, in one deterministic form per type

- Status: Accepted (decided by the owner 2026-11-14), under the standing rule of 2026-10-26 and 2026-10-27: the
  most long-term correct structure, textbook in its foundations, cost ignored. The owner decided six points: the
  entry's type is the language's one value type, the wire form of each type, a decode that accepts every well-formed
  encoding and refuses only what is no value of the type, the size calculus over every type, the scope over the seven
  backends and the TTCN-3 lane, and that no existing CBOR document changes meaning. Where the six are silent this
  record decides, and says so in place: a float entry (section 1.1), the sign of an enum's number (section 2.1),
  indefinite lengths and a text written in chunks (section 3.1), the refusal kinds (section 3.2), a text or byte
  string with no bound (section 1.3), one new diagnostic code (section 6). Nothing of it has landed; "Landing
  order" lists the steps.
- Date: 2026-11-14
- Amends:
  - ADR 0086 section 1.7 ("A codec entry keeps a type of its own"): it now holds for a content-line codec alone;
    a CBOR codec's entry is typed by `ValueType` (section 1). Section 11's row "a composed type on a CBOR or
    content-line entry" keeps its content-line half;
  - ADR 0092 section 1.1 item 3: its bullet refusing a union in a CBOR or content-line codec's entry keeps its
    content-line half;
  - ADR 0077 section 7: a bound "absent where the protocol states none" stays a reading of a positional or
    content-line codec's frame; a CBOR codec's text and byte string state their bound, as every value type's do
    (section 1.3);
  - ADR 0085 Landing order step 3a: a list of records whose records hold unsigned integers, `bool`s, texts and byte
    strings becomes one case of the rule of section 1, with the same bytes (section 8);
  - ADR 0081 section 1 and step 2's CBOR plan: the plan of a CBOR map becomes a tree over the entry's type
    (section 5.1);
  - SCE_FORGE.md section 4.6.1 (its table, "A list of records", "Wire rules", "Generation"): rewritten to this
    record in the last step;
  - docs/SCE_TTCN3_LANE.md sections 12.1, 12.4 and 14 as the change that adds the corpus's codec documents writes
    them: the two message types without a codec form get their codec documents, and section 14's item "the record
    types no codec format holds" is closed (section 9).
- Scope:
  - the forge model and reader of a CBOR codec (`sce-build/src/forge/model.rs`: `CborEntry`, `CborEntryType`,
    `RecordList`; `sce-build/src/forge/parser.rs`: the CBOR entry reader; `sce-build/src/forge/bound_spelling.rs`);
    `sce-build/src/forge/cbor_codec.rs`, `codec_totality.rs`, `codec_size.rs`, `encode_plan.rs` (`CborPlan`);
  - the printers of each backend (`sce-build/src/forge/generator/rust_source/cbor.rs`, `cpp_source/cbor.rs`,
    `swift_codec.rs`) and the templates
    `tools/codegen/templates/forge/{rust,cpp,go,kotlin,python,c,swift}/codec_cbor.*`;
  - the CBOR module of each forge runtime: Rust `backends/rust/forge-runtime/src/cbor.rs`, C++
    `backends/cpp/forge-runtime/include/sce/forge/cbor.h`, Go `backends/go/forge-runtime/codec/cbor.go`, Kotlin
    `Cbor.kt`, Python `sce_forge_runtime/cbor.py`, C11 `backends/c/forge-runtime/include/sce/forge/cbor.h`, Swift
    `Cbor.swift`, with their runtime tests;
  - one diagnostic code (section 6) in `sce-build/src/forge/diagnostic.rs`, SCE_ERROR_CONTRACT.md's generated
    catalog and the appendix of docs/SCE_ACCEPTED_SUBSET.md;
  - `schemas/sce-forge-ext.xsd` (comments), `apis/forge-ast.v1.schema.json` (`CborEntry`), SCE_WIRE_CONTRACTS.md
    (no status flips);
  - the cross-backend codec corpus (`tests/forge/conformance/fixtures.json`, the `codecs` section of
    `tests/forge/conformance/numerical_reference.json`, new resources under `tests/forge/resources/`);
  - the TTCN-3 corpus: two codec documents beside `sce-build/tests/fixtures/test_profile/`, and the coverage test
    that holds every mapped message type to a codec document;
  - SCE_FORGE.md section 4.6.1, docs/SCE_TTCN3_LANE.md, docs/SCE_CODEC_CONSUMER_MIGRATION.md.
- Related: ADR 0044 (revisions of a schema), ADR 0047 (the number a variant declares), ADR 0055 (a value that may
  be absent; section 8, a codec's optional entry), ADR 0058 and ADR 0065 (the one encode, declared totality), ADR
  0077 (a declared bound is part of a field's type), ADR 0078 (a consumer follows SCE one landing at a time), ADR
  0081 (a codec's encode is planned once), ADR 0085 (the envelope's causal context, step 3a), ADR 0086 (one type
  grammar, the size calculus, deterministic CBOR of a signed value), ADR 0091 (a test is a document of the test
  profile, and its TTCN-3 export runs), ADR 0092 (unions), docs/SCE_TEST_PROFILE.md section 12.5 (the canonical
  value form of a message).

## Context

Read from the tree at `eaec7ae62e`.

**What an entry may be.** SCE_FORGE.md section 4.6.1 lets a CBOR codec's entry be an unsigned integer
(`uint8`-`uint64`), `bool`, `string`, `bytes`, `enum:<alias>`, a bounded list of records `list<record:R>[N]`, or
`optional<T>` over one of them. The model says the same: `CborEntryType` is `Leaf(EntryType)` or
`RecordList(ValueType)` (`model.rs`), and a leaf is read by `read_entry_type` (`parser.rs`), which "takes no record,
no list and no bracket". A record a list holds is a map of unsigned integers, `bool`s, texts and byte strings
(`cbor_codec::record_field`); a record with a field of any other type is refused by name
(`cbor_codec::record_list_unlowered`, `generate/unsupported-feature`). ADR 0086 section 1.7 gave the reason: "a CBOR
or content-line codec describes a foreign protocol's frame, not a value of the language". ADR 0092 section 1.1
item 3 refuses a union in a codec's entry on the same ground. No entry may be a signed integer, a float, a record, a
union, a list of anything but records, or a list inside a record.

**The consequence found on 2026-11-14.** The TTCN-3 lane carries a message on a mapped port through a codec of the
reference bindings, which read SCE's codec documents (docs/SCE_TTCN3_LANE.md section 2.1, the row of codec
documents in its table of surfaces, and section 12.4: "a codec document for every message type a mapped port
carries, in CBOR or content-line"). Two message types of the corpus have no codec form:

- `job_status` (`sce-build/tests/fixtures/test_profile/job_status.scxml`): `codes` is `list<uint8>[4]`, a list of
  integers;
- `order_placed` (`sce-build/tests/fixtures/test_profile/imports/order_placed.scxml`): `lines` is
  `list<record:Line>[4]` whose record holds `enum:Color`; `customer` is `record:Customer`; `payment` is
  `union:Payment`, whose cases hold a text, a number and a byte string.

The change of slice S2 that adds the corpus's codec documents (`test: Give every mapped message type of the corpus
its codec document`) gives the four other mapped records theirs, as
`sce-build/tests/fixtures/test_profile/codecs/<record>_codec.scxml`, each a CBOR map whose entries are the record's
fields by their ids, and adds `sce-build/tests/a_mapped_message_type_has_its_codec_document.rs`, which holds every
mapped record to its codec document through the parser's model of the codec (`CborEntryType`, `EntryType`,
`max_size`) and names the two above in `NO_CODEC_FORM`, each with the field no format holds (`codes`, `lines`). Its
`held_by` encodes today's rule that a CBOR entry holds no signed integer. The same change writes the gap into
docs/SCE_TTCN3_LANE.md: section 12.1 says `job_suite`, `orders_suite` and `nested_suite` "run natively and cannot yet
run in the lane", and section 14 leaves "the record types no codec format holds" to "a decision of its own, with its
ADR". This is that record. The record of the trace forms, `form_extras` (`trace_forms/form_extras.scxml`), holds an
`int64`, a list of optionals and a list of lists, and no mapped port carries it.

**What a decode accepts today.** A head that is not in its shortest form, and keys in any order, are read (section
4.6.1, "Wire rules"). An indefinite-length item is refused as malformed; the corpus pins it
(`numerical_reference.json`, `codec_cbor_map`, reject "an indefinite-length map, which the codec never reads"), and
so does each runtime's own test (`cbor_runtime_test.c`, `cbor_runtime_test.cpp`). A key given twice and an item of
another major type than the entry declares are both `CborMalformed`, the name of an input that is not CBOR at all.

**The sign of an enum.** An enum is written as "the unsigned integer its carrier holds"; a negative variant of a
signed carrier has none, so its encode refuses it (`Carrier::Signed` in `encode_plan.rs`, the fixture
`sce-build/tests/fixtures/cbor_codec/codec_cbor_signed_enum.scxml`, which declares `sce:encode-may-fail="true"` for
that reason alone).

**Sizes.** `<NAME>_MAX_BYTES` counts every integer and enum entry at 9 bytes, a text at its `sce:max-size`, and a
list of records by `cbor_max_len` (`cbor_codec.rs`), which it saturates to `u64::MAX` where the sum overflows.

**A text with no bound.** A CBOR text or byte string entry may state no bound (ADR 0077 section 7). C11 refuses
such a codec by name (`cbor_codec::refusal`); every other backend's string grows. No committed `.scxml` CBOR
document has one: the eight in the tree (`stdlib/mesh/envelope.scxml`, `tests/forge/resources/codec_cbor_map.scxml`,
`tests/forge/conformance/broadcast_log_text/broadcast_log_conformance_text_event.scxml`,
`tests/forge/conformance/codec_cbor_open_enum/codec_cbor_open_enum.scxml`, and the four codecs of
`sce-build/tests/fixtures/cbor_codec/`) state each bound by `sce:max-size` or `sce:length`. Inline documents of three
tests do not: `a_rust_cbor_codec_with_text_or_bytes_has_an_owned_mirror.rs` (`free`), `broadcast_log_kind.rs`
(`note`) and `forge_conformance.rs` (`note`).

**CBOR elsewhere in SCE.** The trust layer signs the deterministic CBOR of a typed value
(`backends/rust/trust/src/cbor.rs`, ADR 0086 section 9, domain `sce-trust/event/3`): an absent optional there is
`null` under a key that stays in its record's map. ADR 0092 section 5.3 gives a signed union the tag's number under
the tag's key beside the case's fields; its step 7 has not landed. The test log of a native run writes a message's
value in the canonical form of docs/SCE_TEST_PROFILE.md section 12.5, a JSON tree: a record an object of its fields
with an absent field left out, a list an array with `null` for an absent element, a union an object of one member
named by its case's variant.

## The textbook structure

**CBOR's data model covers the language's types.** RFC 8949 section 2 defines the generic data model: integers,
simple values (among them `false`, `true` and `null`), floats, byte strings, text strings, arrays and maps, and
tags. Every constructor of the type grammar (ADR 0086 section 1.1, ADR 0092 section 1.1) has an injective image in
it: a number an integer or a float, a truth value a simple value, a text and a byte string themselves, a list an
array, a record a map, an absent value `null` or a missing key. A protocol on CBOR states which items it accepts (a
"specific data model", section 2.2); this record states it once, by the type.

**Well-formed, valid, expected.** RFC 8949 section 1.2 separates a well-formed item (it follows the encoding,
section 3 and Appendix F) from a valid one (it also meets the data model's rules: text is UTF-8, a map holds no key
twice, section 5.3.1 and section 5.6) and from one an application expects. A decoder's refusals fall into these three
classes, and naming each class apart tells a peer that sent garbage from one that sent a value outside the protocol.

**One encoding to write, every encoding to read.** Preferred serialization (RFC 8949 section 4.1) writes every head
and length in its shortest form, a float in its shortest exact width, and a definite length wherever the length is
known; core deterministic encoding (section 4.2.1) adds that a map's keys are sorted by the bytes of their
encodings, so one value has one string of bytes. Section 4.1 also names the reader's side: a decoder that does not
rely on receiving preferred serializations is "variation-tolerant", and a full decoder is variation-tolerant by
definition. A reader that refused a non-preferred encoding would refuse a value, not an error.

**CDDL describes items, not bytes.** A CDDL rule (RFC 8610) matches data items of the generic data model; it says
nothing of their serialization, and a map's members match in any order (section 3.5.3). A bareword key is a text
string (section 3.5.1), `?` an optional member (section 3.2), `n*m` an occurrence (section 3.2), `.size` a bound on a
string's bytes (section 3.8.1), `a..b` a range (section 2.2.2.1), `&( name: value )` a choice of named values
(section 2.2.2.2). Validating an item against a CDDL rule and refusing only what does not match is exactly the
decoder RFC 8949 section 4.1 calls variation-tolerant, with the protocol's rules applied to the value.

**A sum is tagged by its name, outside its fields.** ASN.1's JSON encoding rules (ITU-T X.697) encode a `CHOICE` as
an object of one member whose name is the chosen alternative's identifier; TTCN-3 writes a union's value as
`{ card := { ... } }` (ES 201 873-1, its union type clause). The tag is then a key, its case's value one level below,
and an empty case an empty value.

**Robustness is tolerance of variants, not of errors.** RFC 9413 ("Maintaining Robust Protocols") argues against a
reader that quietly accepts invalid input. Every encoding a variation-tolerant reader accepts denotes a valid item;
the tolerance stops at validity, so it is not the tolerance RFC 9413 argues against.

**Orthogonality** (ADR 0086, "The textbook structure"). A position that takes only some types is a rule an author
learns case by case; `list<record:R>` holding what `list<uint8>` may not is that rule again.

## Decision

**A CBOR codec's entry is a value of the language: any type of the one type grammar, at any depth. Each type has one
wire form, written in RFC 8949's core deterministic encoding, whose tree is the tree of the canonical value form of a
message (docs/SCE_TEST_PROFILE.md section 12.5). A decode reads every well-formed encoding of a value of the type and
refuses what is no such value, each refusal by a name of its class. Every entry has a largest encoding the build
computes, and the encode buffer is sized by it. A content-line codec keeps its own entry type. Every backend
generates every such codec, the cross-backend corpus holds them to one set of bytes and refusals, and the two
message types of the TTCN-3 corpus get their codec documents.**

### 1. A CBOR entry is a value of the language

1. **The rule.** The `sce:type` of a CBOR codec's entry is read by the one type grammar (ADR 0086 section 1.1, with
   ADR 0092's `union:<alias>`) and held as `ValueType`:

   ```
   T ::= uint8 | uint16 | uint32 | uint64 | int8 | int16 | int32 | int64 | float32 | float64 | bool
       | enum:<alias> | string[B] | bytes[B] | optional<T> | record:<alias> | union:<alias>
       | list<T>[B] | list<T>[M..B]
   ```

   Every constructor at every depth: a list of integers, a list of lists, a record holding a list of unions of
   records, an optional record. The owner's decision names integers, `bool`, texts, byte strings, enums, records,
   unions, optionals and lists; the grammar it names also holds `float32` and `float64`, and this record keeps them:
   a float has one deterministic form (RFC 8949 section 4.2.1 with section 4.1), the one ADR 0086 section 9.2 already
   signs, and leaving it out would be the non-orthogonality this record removes. What the type grammar refuses
   stays refused (ADR 0086 section 1.5): `optional<optional<T>>` and a type that contains itself.
2. **What the codec adds to the type.** The entry keeps what is a fact of the codec and not of the value:
   - `sce:key`, its key in the codec's own map, `0`-`23`, unique within the codec (section 2.2);
   - `sce:length` on a `bytes` or `optional<bytes>` entry whose type writes no bracket: the exact length, which is
     also its bound (the type is `bytes[L]`). The language has no byte string of an exact length (ADR 0086 section
     15 gives a byte string no lower count), so the exactness stays the top-level entry's, as it is today; a bracket
     beside `sce:length` on one entry is refused as `validation/incompatible-attributes`, one bound and one spelling
     (ADR 0086 section 1.3). A record's field states no exact length, as no event schema's field does;
   - the deployment's capacity of a storing form (ADR 0077 section 7), for a top-level text or byte string entry;
   - `sce:codec-format="2"` and `sce:encode-may-fail`, as today.
3. **Every bound in the type.** A text, a byte string and a list state their bound in the type at every depth
   (`string[64]`, `list<record:R>[2]`). On a top-level `string` or `bytes` entry, `sce:max-size` is read as the
   bracket on the outermost type, with `validation/bound-attribute-superseded` and its rewrite
   (`move_bound_into_type`), as on an event schema's field (ADR 0086 section 1.3); `bound_spelling.rs` gains the CBOR
   entry as a site, and `sce-codegen bound-spelling` rewrites a CBOR codec too. A text or byte string with no bound
   is refused wherever it stands (ADR 0086 section 1.4, `validation/attribute-rule-violated`). This is where the
   owner's decision meets ADR 0077 section 7, which let a codec's text state no bound where the protocol states none:
   a CBOR entry is now a value of the language, a value type has a largest value or is not a type, and section 4
   sizes the encode by it. A foreign protocol that states no bound is written with a named bound
   (`string[peer_text=4096]`, ADR 0050), which each build sets, and the deployment's capacity stays below it where a
   storing form needs less. No committed document changes for this rule; the three inline documents of the Context
   become refusal tests.
4. **A content-line codec keeps `EntryType`, and its list stays `sce:repeated`.** RFC 5545's content lines are a
   grammar of properties, each a name, parameters and one value of an RFC 5545 value type, or a comma-separated
   list of values; a component repeats a property line. That grammar has no image of a record inside a record, of a
   union, of a list inside a list, or of a list of anything but a property's values or lines. The CBOR data model
   holds every constructor of the language; RFC 5545 holds a few, in its own shapes. ADR 0086 section 1.7's reason,
   that a codec describes a foreign frame, therefore stays true of a content-line codec and stops being true of a
   CBOR one: the content-line entry describes RFC 5545, which SCE does not own, and the CBOR entry describes a value,
   whose CBOR form SCE does own. `EntryType` also stays the view a lowering that holds no composition reads an event
   schema's field through (a native action's argument, a host's parameter).
5. **The model.** `CborEntry.sce_type` becomes `ValueType`; `CborEntryType`, `RecordList` and `CborEntry.max_size`
   go, the bound living in the type; `CborEntry.length` stays the exact length of section 1.2. A union import
   (`<sce:import kind="union">`) is read by a CBOR codec, beside its enum and event-schema imports.

### 2. The wire form

1. **One form per type**, written in core deterministic encoding (RFC 8949 section 4.2.1): every head, length and
   count in its shortest form, definite lengths only, a map's keys sorted by the bytes of their encodings.

   | Type | Form |
   |---|---|
   | `uint8`-`uint64` | major type 0 |
   | `int8`-`int64` | major type 0 for zero or more, major type 1 below zero |
   | `float32`, `float64` | major type 7, the shortest of half, single and double precision that holds the value exactly; a NaN is `f9 7e 00` (ADR 0086 section 9.2) |
   | `bool` | simple value 20 (`f4`) or 21 (`f5`) |
   | `string[B]` | major type 3 |
   | `bytes[B]` | major type 2 |
   | `enum:E` | the integer of the variant's declared number (ADR 0047), major type 0, or major type 1 below zero; an open enum's number that names no variant is written the same way |
   | `record:R` | a map of each present field, keyed by the field's id as a text string, keys in section 4.2.1 order; an absent optional field is left out, its key with it |
   | `list<T>[M..B]` | an array of definite length, its elements in their order |
   | `optional<T>` as a list's element | the element's form, or `null` (simple value 22, `f6`) when absent |
   | `optional<T>` as a record's field or a codec's entry | the field's form when present; absent, the key is not written |
   | `union:U` | a map of exactly one entry, keyed by the case's variant name as a text string, whose value is the case's record as a map, or the empty map (`a0`) for a case that holds none |

   The enum's sign is decided here, where the owner's decision says "the unsigned of its declared value": for an
   unsigned carrier the two are one rule, and for a signed carrier the type has negative values, which section 1.1's
   "any type" makes the codec carry. The encode of `codec_cbor_signed_enum` then refuses nothing (section 8).
2. **The codec's own map.** The codec is the outermost record, keyed by its entries' `sce:key`s: a definite-length
   map of the present entries in ascending key order, an absent optional entry not written. Its keys are unsigned
   integers because they are the codec's numbering of its entries, which the document states, where a nested
   record's keys are its schema's field ids, which the schema states. It is the one map of the form whose keys are
   integers.
3. **Why each choice.**
   - *An enum is its number.* ADR 0086 section 9.2's reason holds: an open enum holds numbers no variant names, so
     only the number covers every value; a revision adds variants and never renumbers one (ADR 0044). It is also what
     every CBOR codec writes today.
   - *A union's tag is its variant's name, around the case.* The tag's enum is closed (ADR 0092 section 1.1), so the
     name covers every value and section 9.2's reason for the number does not arise. The tag stands as a key, and
     every other key below the codec's own map is a text naming a member; a number there would make the union's map
     the one nested map keyed by an integer. It is the tree of TP section 12.5, of X.697's `CHOICE` and of TTCN-3's
     union value, so a binding maps it to a TTCN-3 union by name, with no table of numbers. ADR 0092 section 2.2
     chose the tag beside the fields for the JSON wire and rejected this spelling for three reasons, and none holds
     here: an unknown tag is refused at the union and not stepped over (section 3.2); one level deeper costs a CBOR
     reader nothing, where the measured cost there was a model's constrained decoding; an empty case's `{}` is one
     byte, `a0`.
   - *An absent field is left out; an absent element is `null`.* A map names its members, so absence is a member
     not written, which is CDDL's `?` (RFC 8610 section 3.2) and what the codec's own map does with an optional entry
     (ADR 0055 section 8). An array names its elements by position, so an absent element cannot be left out without
     moving the elements after it, and is `null`, the one item no other form takes.
4. **The tree is the canonical value form's, on purpose.** A message's value in the test log (TP section 12.5) and
   its CBOR form are one tree: a record a map keyed by field ids with absent fields left out, a list an array with
   `null` for an absent element, a union a one-member map named by its variant. The leaves differ only by what each
   medium holds: the log, JSON, spells an integer as the text of its digits (a JSON number is a double), a byte string
   as upper-case hex (JSON has none) and an enum by its name (the log is read by a person), where CBOR carries the
   integer, the bytes and the number. The two are related by one map over the type, leaf by leaf, and below the codec's
   own map nothing else; at the codec's own map the log's field id and the codec's `sce:key` name one entry. The
   identity is deliberate: a decoded message, the value a run logs and the value a trace replays are one value, and a
   binding that reads a codec's CBOR writes the log's tree without a second mapping.
5. **The signed form is another encoding, and stays one.** The trust layer's bytes (ADR 0086 section 9) encode a
   value for a signature, not for a wire: an absent optional is `null` under a key that stays in the map, so a
   record's keys are a function of its schema's revision alone, and a field a later revision added is written as its
   default. A codec carries what a sender sent, and a key absent from a map is how CBOR protocols spell absence. The
   two forms differ in that one place and in the union's (section 2.3; ADR 0092 section 5.3, step 7 not landed, see
   "Open questions" 1). Nothing signed changes here.
6. **The form in CDDL.** The rule, by type (`B` the bound, `M` the lower count, `L` an exact length, `k` an entry's
   key):

   ```cddl
   ; uint8 .. uint64, int8 .. int64
   uint8 = 0..255
   uint16 = 0..65535
   uint32 = 0..4294967295
   uint64 = 0..18446744073709551615
   int8 = -128..127
   int16 = -32768..32767
   int32 = -2147483648..2147483647
   int64 = -9223372036854775808..9223372036854775807
   ; float32 is written as float16-32, float64 as float; a reader takes any width that holds the value (section 3)
   ; string[B]          tstr .size (0..B)
   ; bytes[B]           bstr .size (0..B)
   ; sce:length L       bstr .size L
   ; enum:E (closed)    E = &( name1: n1, name2: n2, ... )
   ; enum:E (open)      the carrier's range
   ; record:R           R = { id1: T1, ? id2: T2, ... }       ; ? on an optional<T> field, T its element
   ; list<T>[M..B]      [ M*B T ]
   ; optional<T> in a list   T / null
   ; union:U            U = { v1: R1 } / { v2: {} } / ...     ; one alternative per case
   ; the codec          { k1 => T1, ? k2 => T2, ... }         ; ? on an optional<T> entry
   ```

   CDDL constrains the item; the deterministic encoding of section 2.1 constrains the bytes an encode writes, and is
   stated beside it because CDDL has no notation for it.
7. **The two message types of the corpus, in CDDL** (the codec documents of section 9):

   ```cddl
   job-status = {
     0 => uint32,                  ; elapsed_ms
     ? 1 => tstr .size (0..32),    ; note
     2 => [ 0*4 uint8 ],           ; codes
     ? 3 => uint8,                 ; tag
     ? 4 => uint8,                 ; retry
     5 => uint32,                  ; count
   }

   order-placed = {
     0 => uint32,                  ; order_id
     1 => [ 0*4 line ],            ; lines
     ? 2 => tstr .size (0..32),    ; note
     ? 3 => uint8,                 ; priority
     4 => customer,
     5 => payment,
   }
   line = { sku: tstr .size (0..16), quantity: uint16, color: color }
   color = &( red: 0, green: 1, blue: 2 )
   customer = { full_name: tstr .size (0..24), vip: bool }
   payment = { card: card } / { cash: {} } / { voucher: voucher }
   card = { last4: tstr .size (0..4), amount: uint32 }
   voucher = { voucher_code: bstr .size (0..4) }
   ```

8. **Vectors** (hexadecimal; each is a case of the corpus, section "Conformance"):

   | Value | Type | Encoding |
   |---|---|---|
   | -1 | `int8` | `20` |
   | -500 | `int16` | `39 01 f3` |
   | 1.5 | `float64` | `f9 3e 00` |
   | 0.1 | `float32` | `fa 3d cc cc cd` |
   | `blue` (2) | `enum:Color` | `02` |
   | `fault` (-1) | `enum:Status`, carrier `int8` | `20` |
   | [1, 2] | `list<uint8>[4]` | `82 01 02` |
   | [7, absent, 300] | `list<optional<uint16>>[3]` | `83 07 f6 19 01 2c` |
   | {full_name: "Ann", vip: true} | `record:Customer` | `a2 63 76 69 70 f5 69 66 75 6c 6c 5f 6e 61 6d 65 63 41 6e 6e` (`vip` first: its encoding is shorter) |
   | {id: 1, note: absent} | a record of `id` `uint8` and `note` `optional<string[4]>` | `a1 62 69 64 01` |
   | `cash` | `union:Payment` | `a1 64 63 61 73 68 a0` |
   | `card` {last4: "1234", amount: 500} | `union:Payment` | `a1 64 63 61 72 64 a2 65 6c 61 73 74 34 64 31 32 33 34 66 61 6d 6f 75 6e 74 19 01 f4` |
   | elapsed_ms 1500, codes [1, 2], tag 7, count 3, note and retry absent | the `job_status` codec | `a4 00 19 05 dc 02 82 01 02 03 07 05 03` |
   | order_id 42, one line {sku "A1", quantity 2, color blue}, customer {"Ann", vip}, payment `cash`, note and priority absent | the `order_placed` codec | `a4 00 18 2a 01 81 a3 63 73 6b 75 62 41 31 65 63 6f 6c 6f 72 02 68 71 75 61 6e 74 69 74 79 02 04 a2 63 76 69 70 f5 69 66 75 6c 6c 5f 6e 61 6d 65 63 41 6e 6e 05 a1 64 63 61 73 68 a0` |

### 3. A variation-tolerant decode, a deterministic encode

1. **What a decode accepts.** Every well-formed encoding (RFC 8949 section 3) of a value of the entry's type, as a
   variation-tolerant decoder does (section 4.1): a head or a length in a longer form than its shortest; a float in a
   wider precision than its shortest, where the value is one the type holds (a `float32` given as a double that is
   exactly a single); a map's keys in any order; and an indefinite-length array, map, text string or byte string
   (section 3.2), read up to its break. A text written in chunks is the concatenation of its chunks. A key a record or
   the codec does not declare is stepped over with its value, as today, an indefinite one included, to the nesting
   depth of 16 the skip already holds (`CborTooDeep` past it). The decode accepts a value it would never write: a
   sender that wrote it longer said the same thing, and section 4.1 makes a full decoder variation-tolerant by
   definition.

   The owner's decision is "decode accepts any well-formed encoding". The tree refuses an indefinite length today
   (Context), and the corpus pins that refusal; this record takes the decision as written and moves those cases
   from refused to accepted (section "Conformance"). It is the one widening of what an existing codec reads (section
   8).
2. **What a decode refuses**, each by the class RFC 8949 section 1.2 puts it in, and by one name per runtime:

   | Refusal | Class | Kind (corpus name) |
   |---|---|---|
   | an item that is not well-formed (RFC 8949 Appendix F): reserved additional information 28-30, a break outside an indefinite item, a chunk of another major type or of indefinite length inside an indefinite string, a simple value in the two-byte form below 32 | not well-formed | `CborMalformed` (`cbor-malformed`) |
   | the input ends inside an item | not well-formed | `NeedMoreBytes`, as today |
   | an item of a major type, a simple value or a tag the type at that place does not take: a text where a number stands, a negative integer where an unsigned type stands, `null` where the place is not a list's optional element, a tagged item (a bignum among them: the types are defined over the basic generic data model), a top-level key that is not an unsigned integer, a record's or a union's key that is not a text string | expected | `CborWrongType` (`cbor-wrong-type`), new |
   | a map that holds one of its declared keys twice, at any depth | invalid (RFC 8949 section 5.6) | `CborDuplicateKey` (`cbor-duplicate-key`), new |
   | a text that is not UTF-8, a chunk included (a chunk ends at a code point, section 3.2.3) | invalid (section 5.3.1) | `InvalidUtf8`, as today |
   | a required entry or a record's field that is not optional, absent | expected | `CborRequiredKeyMissing` (`cbor-required-key-missing`), as today |
   | a union's map whose one key names no case | expected | `CborUnknownVariant` (`cbor-unknown-variant`), new |
   | a union's map of no entry or of more than one | expected | `CborUnionEntries` (`cbor-union-entries`), new |
   | an integer outside its type's width, a float that is not a value of `float32`, a text or byte string past its bound, a list past its bound or below its lower count | expected | `CborOutOfRange` (`cbor-out-of-range`), as today |
   | an exact-length byte string of another length | expected | `CborWrongLength` (`cbor-wrong-length`), as today |
   | a number a closed enum does not declare | expected | `UndeclaredEnumValue`, as today |
   | an undeclared value nested past 16 levels | expected | `CborTooDeep` (`cbor-too-deep`), as today |

   The spellings follow each runtime's convention (SCE_FORGE.md section 4.6.1): Rust `CodecError::CborWrongType`,
   `CborDuplicateKey`, `CborUnknownVariant`, `CborUnionEntries`; Go `codec.ErrCborWrongType`, `ErrCborDuplicateKey`,
   `ErrCborUnknownVariant`, `ErrCborUnionEntries`; Swift `cborWrongType`, `cborDuplicateKey`, `cborUnknownVariant`,
   `cborUnionEntries`; C11 `SCE_FORGE_CODEC_CBOR_WRONG_TYPE`, `_DUPLICATE_KEY`, `_UNKNOWN_VARIANT`,
   `_UNION_ENTRIES`; the C++ runtime's `CodecError` gains the four. A Kotlin, Python or C++ decode answers absence
   for every refusal, its codec convention, so those three are held to refusing, not to the name ("Open questions"
   2). Two refusals change their name: a key given twice and an item of another major type, both `CborMalformed`
   until now, which named them as not being CBOR.

   *The order of the checks.* A list's count is judged against its bound and its lower count before any element is
   decoded (ADR 0077 section 2), and an indefinite array's elements are counted as they come, refused at the first one
   past the bound before it is decoded. A text's or byte string's length is judged before its bytes are read, and a
   chunked one's running length after each chunk. A union's map is judged for one entry before its key is read
   (an indefinite one at the second key), then its key, then the case's record. Duplicates are found among the keys a
   map declares, by one mark per declared key; a key the map does not declare is stepped over and not remembered, so
   a map that repeats an undeclared key is read as if it held it once. That is the one invalid map the decode does not
   refuse: finding it would need memory in proportion to the input, which a bounded reader does not have.
3. **Why the refusals are about the value.** A refusal says that what arrived is not a value of the protocol. An
   encoding that is not the deterministic one still is, so refusing it would refuse a value. The bytes that must be
   canonical are the bytes a signature covers, and SCE never verifies a signature over received bytes: the trust
   layer re-encodes the typed value it read (ADR 0086 section 9.1), so a variant encoding gives an attacker no second
   string to sign. Two backends given one value write one string of bytes, because every encode writes the
   deterministic form; two backends given one string of bytes read one value, because every decode reads the data
   model. The cost is that an input's length is not bounded by `C(T)` (section 4.3).
4. **A Rust view of a text written in chunks.** The Rust borrowed form presents a top-level text or byte string
   entry as a view of the input (`Text<'a, N>`, `Bytes<'a, N, M>`, `Bounded` over a slice). A text written in chunks
   stands in several runs of the input, so no one slice holds it. The CBOR codec's view of a text and a byte string
   becomes the runs of the input the string stands in, one run for a definite-length string: still a view, still no
   copy, still `Bounded` by the string's total length. Its readers (display, equality, iteration over the bytes, the
   copy into the owned mirror) read the runs in order; an accessor for one slice answers where the string is one
   run, and a consumer that needs one slice of any string takes the owned mirror, which every CBOR codec now has
   (every text is bounded, section 1.3). The positional codecs' `Text` and `Bytes` do not change. The Mesh's reads of
   the envelope move with it: `source` and `event_type` are read through `to_string`, which reads the runs, and
   `data`, read as one slice today (`backends/rust/mesh/src/router.rs`, `into_inner`), is read as one slice where it
   is one run and copied out of its runs otherwise, within its bound. This is a change to the public
   type of a generated Rust CBOR codec, migrated per ADR 0078 in docs/SCE_CODEC_CONSUMER_MIGRATION.md. Every other
   backend holds an owned text and copies the chunks into it, refusing past the bound as they come.
5. **The encode.** It writes the deterministic form of section 2 and nothing else, on every backend, so it never
   writes an indefinite length, a longer head, or a wider float. It refuses only a value outside the entry's type
   where the backend's types do not exclude it (ADR 0086 section 7.1's table): a list below its lower count on every
   backend; a list, text or byte string past its bound where the language's types carry no bound (Kotlin, Go,
   Python, Swift, C11's `len`); an integer past its width on Python. `codec_totality` decides which, from the plan
   (section 5.2), and a codec whose encode can refuse declares `sce:encode-may-fail="true"`, as today.

### 4. The size calculus

1. **`C(T)`, the most bytes an entry's deterministic encoding takes**, by structural recursion over the type, as ADR
   0086 section 2.1 computes it, `h(n)` the length of a CBOR head for the argument `n` (1 below 24, 2 below 2^8, 3
   below 2^16, 5 below 2^32, 9 otherwise):

   | Type | `C(T)` |
   |---|---|
   | `uint8`, `uint16`, `uint32`, `uint64` | 2, 3, 5, 9 |
   | `int8`, `int16`, `int32`, `int64` | 2, 3, 5, 9 |
   | `float32`, `float64` | 5, 9 |
   | `bool` | 1 |
   | `enum:E` | a closed `E`: the longest over its declared numbers `n` of `h(n)` for `n` of zero or more and `h(-1 - n)` below zero; an open `E`: `C` of its carrier |
   | `string[B]`, `bytes[B]` | `h(B) + B` |
   | `optional<T>` | `max(1, C(T))` as a list's element; `C(T)` as a record's field or a codec's entry, counted present |
   | `record:R`, fields `f1..fn` | `h(n) + Σ (C(id) + C(Tf))`, with `C(id) = h(len id) + len id` |
   | `list<T>[M..B]` | `h(B) + B · C(T)` |
   | `union:U`, case `c` named `nc` holding fields `f1..fm` | the largest over the cases of `h(1) + h(len nc) + len nc + h(m) + Σ (C(id) + C(Tf))`; an empty case's record is `h(0)`, one byte |
   | the codec, entries `e1..en` | `h(n) + Σ (1 + C(Te))`, each key one byte (`sce:key` is below 24); an exact-length entry `h(L) + L` |

   Every row but two is ADR 0086 section 2.1's. The optional row is that table's value counted where a key is left
   out, which is never larger. The union row differs from ADR 0092 section 5.1's, which measures the signed form of
   that ADR's section 5.3; the build keeps both measures, one per form, and the codec reads this one.
2. **Tight, and the bound of the encode buffer.** A value at its largest (every list full, every text at its bound,
   every number at its longest head, every optional present, every union in its largest case) encodes to exactly
   `C(T)`, so `<NAME>_MAX_BYTES` is `C` of the codec, which sizes C11's encode buffer, a broadcast log's slot (ADR
   0062) and every backend's counting sink, and is held tight by a test (section "Conformance"). It replaces today's
   count, which put every integer and enum entry at 9 bytes; a codec with a narrower one gets a smaller, exact
   constant. Worked: the `job_status` codec is `1 + (1 + 5) + (1 + 34) + (1 + 9) + (1 + 2) + (1 + 2) + (1 + 5) = 64`
   bytes; `union:Payment` is `max(30, 7, 28) = 30`, its `card` case.
3. **An input is not bounded by `C(T)`.** A variant encoding is longer than the deterministic one (a 9-byte head, a
   text in many chunks), so a receiver does not size an input buffer by `<NAME>_MAX_BYTES`; the transport bounds the
   input, and the decode reads within the buffer it is given. Storage is bounded by the type alone: nothing a decode
   holds grows with a variant.
4. **A size past what the build counts is refused.** `C` is computed in 64 bits; a type whose largest encoding is
   past `2^64 - 1` bytes has no encode bound, and is refused with the code of section 6, where the codec now
   saturates to `u64::MAX`. The schema unit's `DATA_MAX_LEN` and `C`, whose refusal is a `generate/unsupported-feature`
   today, move to the same code: one fact, one code.

### 5. The plan and the backends

1. **The plan is a tree over the type** (ADR 0081 section 1). `CborPlan` keeps its entries in declaration order and
   the order by key it writes them in, and each entry's `CborValue` becomes the shape of its type: an integer with
   its width and sign, a float with its width, `bool`, a text or byte string with its bound (and an exact length at
   the top level), an enum with its carrier and whether it is closed, a record with its fields in the order of
   their encoded keys and whether each is optional, a union with its cases (variant name, number for the generated
   type's tag, record or none), a list with its element, bound and lower count, and an optional with where it stands
   (an entry or field, written by omission; a list's element, written as `null`). The planned order of a record's
   fields is computed once, from the bytes of their encoded keys (`cbor_codec::record_fields_by_key`'s rule), and no
   printer sorts. Every printer reads the codec only through the plan, and prints a walk of the shape: one reader
   and one writer per record and per union document the codec reaches, each named for its document, so a record
   met at two places is read by one function.
2. **Totality from the shape.** `codec_totality` walks the shape and names each refusal the encode can make on the
   backend at hand (section 3.5), at the place in the type it stands, so a codec that can refuse declares it and one
   that cannot is total. The signed carrier's refusal (`Carrier::Signed`) goes.
3. **The types each backend holds.** A composed entry is held in the type the backend's units declare for it (ADR
   0086 section 7.1, ADR 0092 section 7): a record is the payload type of its event schema's unit, a union the type
   of its union document's unit, a list the backend's list of section 7.1 (`BoundedList<T, N>` on Rust,
   `OwnedList<T, N>` on C++, the language's list elsewhere, the fixed layout on C11), an optional the backend's
   optional. The codec declares no record or union type of its own; it brings the units into scope as step 3a does
   for a record. A top-level scalar, text or byte string entry keeps the form it has today on each backend (Rust's
   view of section 3.4 aside).
4. **During the landing a backend refuses by name** (`generate/unsupported-feature`) what its printer does not lower
   yet, naming the entry and the constructor, through `cbor_codec::refusal`, whose answer the conformance schedule
   reads, so a fixture runs exactly where it generates. `cbor_codec::lowers_record_list` becomes the per-backend
   answer of which shapes are lowered, one arm per backend; it is removed with the last backend's step.

### 6. Diagnostics

- **One new code: `validation/encoded-size-overflow`**, stage `validation`, spec anchor this record's section 4.4, no
  fix kind: "the largest encoding of `<type>` at `<place>` is past `2^64 - 1` bytes", raised where `C` or `J` of a
  CBOR entry or of an event schema's payload overflows. No existing code states that fact: the schema's site raises
  `generate/unsupported-feature`, which states that a backend does not lower something yet, and the codec raises
  nothing. The variant in `sce-build/src/forge/diagnostic.rs`, its golden, its row in the appendix of
  docs/SCE_ACCEPTED_SUBSET.md (`acceptance_doc_covers_every_code`) and the regenerated catalog of
  SCE_ERROR_CONTRACT.md section 5 land with it. Additive (SCE_ERROR_CONTRACT.md section 8).
- **Codes that gain a site**: `validation/bound-attribute-superseded` and its fix `move_bound_into_type` at a CBOR
  entry's `sce:max-size` (section 1.3); `validation/incompatible-attributes` for a bracket beside `sce:length`;
  `validation/attribute-rule-violated` for a CBOR text or byte string with no bound, at any depth.
- **Refusals that go**: `validation/invalid-attribute` "with the entry types" for a list, a record, a union or a
  nested optional on a CBOR entry (it stays on a content-line entry, and its candidates there are unchanged);
  `generate/unsupported-feature` for a record of a CBOR list whose field is not an unsigned integer, `bool`, text or
  byte string (`record_list_unlowered`), and C11's for a text with no bound (`cbor_codec::refusal`), which section
  1.3 refuses at parse on every backend.
- A decode's refusal is a run-time error of the runtime (section 3.2), not a diagnostic.

### 7. Wire surfaces (SCE_WIRE_CONTRACTS.md)

No status flips.

- **Authoring grammar** (`pre-release`): the `sce:type` of a CBOR codec's `<data>` takes the type alphabet, judged
  by the parser as every value type is (ADR 0086 section 13); `sce:max-size` on a CBOR entry is the legacy spelling of
  a bracket (its comment in `schemas/sce-forge-ext.xsd` says so); a CBOR codec may `<sce:import kind="union">`. Not
  additive, allowed while `pre-release` (its policy item 1): a CBOR text or byte string with no bound, accepted until
  now on six backends, is refused.
- **Forge AST export** (`pre-release`): `CborEntry.sce_type` is the recursive `ValueType`, its bound inside it;
  `max_size` leaves `CborEntry`, and `CborEntryType` and its `EntryType` arm leave its schema. A text entry's export
  changes shape (`"string"` beside `"max_size": 64` becomes the bounded text's object), allowed while `pre-release`.
- **Diagnostics** (`pre-release`): one code, additive.
- **The CBOR form of a codec** has no row: SCE_FORGE.md section 4.6.1 holds it, and the corpus of section
  "Conformance" pins it.

### 8. No existing CBOR document changes meaning

1. **The claim.** For every CBOR codec document the tree holds, and for every value of it: the encode writes the
   bytes it writes today, and every string of bytes a decode reads today it reads to the same value. What changes is
   only what was refused: a value the encode refused, a string of bytes the decode refused, and the name of two
   refusals.
2. **The proof, by the form of each entry a document can hold today.**
   - An unsigned integer, `bool`, text, byte string, exact byte string: section 2.1 writes each as section 4.6.1
     does (major type 0, 2 or 3 with the shortest head; `f4` or `f5`), and the decode reads each as before.
   - An enum of an unsigned carrier: its number, major type 0 with the shortest head, as today. Of a signed carrier:
     a number of zero or more as today; a negative number, which the encode refused and the decode refused as a
     major type 1 item, is now written and read (`codec_cbor_signed_enum`, whose `sce:encode-may-fail="true"` stays
     allowed and now names no refusal of that entry).
   - An optional entry: written when present, left out when absent, as today; `null` under its key, refused today
     as an item of the wrong type, is refused as `CborWrongType`.
   - A list of records of step 3a: its records hold only unsigned integers, `bool`s, texts and byte strings
     (`record_field`), none optional, so each record is a map of every field, keyed by the field's id as text in the
     order of section 4.2.1, each value as ADR 0086 section 9.2 writes it; section 2.1's record row, with no optional
     field, is that form, and its leaf rows are those of 9.2 for the four leaves. The array is definite-length in
     both. A decode of it refuses what 3a's refuses (a count past the bound or below the lower count, a field absent,
     a key twice) and steps over the keys 3a steps over.
   - The codec's own map: definite length, ascending integer keys, shortest heads, as today.
   Hence a value's bytes are equal. The decode's widening is section 3.1's (indefinite lengths, a float wider than
   its shortest), and its renaming section 3.2's (a key twice, an item of the wrong type).
3. **The test.** The landing's first step (step 0) captures the bytes each backend writes today, before anything
   moves: a byte case for every committed CBOR document (`envelope`, `codec_cbor_map`, the text event of
   `broadcast_log_text`, `codec_cbor_open_enum`, and the four codecs of `sce-build/tests/fixtures/cbor_codec/`), at a
   smallest and a largest value and every optional both ways, written into the corpus. Every later step regenerates
   the backends and holds them to those cases unchanged, beside the Mesh golden envelope (`forge_cbor_codec`,
   `CborCodecTest`). A case of a negative `Status` moves from the encode's refusals to its byte cases, and the
   indefinite-length map from the decode's refusals to its variant cases, each in the step that changes it, with the
   reason in its note. A characterization test written before the change, and kept, is what turns "the same bytes"
   from an argument into a check.
4. **What changes outside the bytes.** The generated `<NAME>_MAX_BYTES` of a codec with a narrow integer or an enum
   entry shrinks to its exact size (section 4.2); a broadcast log's slot over such a codec shrinks with it. The
   committed documents are rewritten by `sce-codegen bound-spelling` (each `sce:max-size` into its type), which changes
   their source hash, so every tree generated from them is regenerated in that step: the Mesh envelope's
   (`backends/rust/mesh/src/generated/envelope.rs`, `backends/kotlin/mesh/src/commonMain/kotlin/generated/Envelope.kt`,
   `backends/swift/mesh/Sources/SceMesh/Generated/Envelope.swift`, `sce/src/mesh/generated/envelope.h`), the Swift
   test machines' (`CodecCborMap.swift`, `CodecCborOpenEnum.swift`, `Envelope.swift` under
   `backends/swift/tests/Sources/StaticDatamodelMachines/`) and the Go round-trip tree's `codec_cbor_map`. The
   corpus's four codec documents (section 9.1) are rewritten in the same step, with any tree generated from them.

### 9. The TTCN-3 lane

1. **Two codec documents**, in the directory and by the naming of the corpus's other four
   (`sce-build/tests/fixtures/test_profile/codecs/<record>_codec.scxml`): `job_status_codec.scxml`, a CBOR codec of
   no import whose entries are `job_status`'s fields one to one, each `id` and `sce:type` the field's, keys `0`-`5` in
   declaration order; and `order_placed_codec.scxml`, the same of `order_placed`, importing `Line` and `Customer`
   (event schemas) and `Payment` (a union) from `../imports/`. Their CDDL is section 2.7; their bytes are cases of
   the corpus (section "Conformance"). An entry's `sce:type` is now the field's text exactly (`string[32]`, not
   `string` beside `sce:max-size="32"`), so the corpus's four codec documents are rewritten by `bound-spelling` with
   the rest (section 8.4).
2. **The coverage test loses `NO_CODEC_FORM`.** `a_mapped_message_type_has_its_codec_document` reads the parser's
   model of a codec, which section 1.5 changes: in step 1 it compares a CBOR entry's `ValueType` with its field's
   `ValueType` for equality, and its `held_by` holds every type for CBOR. In step 6 `NO_CODEC_FORM` becomes empty and
   is removed with its reason, and docs/SCE_TTCN3_LANE.md sections 12.1, 12.4 and 14 say the three suites can run in
   the lane once their traces exist.
3. **The reference binding reads this form.** The downstream's CBOR binding (over libcbor, DD section 8.4) reads a
   codec document's parameters and writes and reads this record's form, so:
   - it writes the deterministic form whatever its library does by default: libcbor writes an integer at the width
     the item was built with and a map's pairs in the order they were added, so the binding builds each integer and
     length at its narrowest width and adds a map's pairs in the order of their encoded keys;
   - its decode is libcbor's, a full decoder, so it is variation-tolerant as SCE's is (section 3.1), and the
     refusals of section 3.2 are the binding's to apply to the item it decoded;
   - its mapping is structural: a record to a TTCN-3 record by field name, a union to a TTCN-3 union by alternative
     name, a list to a `record of`, an absent field to `omit`; and because the tree is TP section 12.5's (section
     2.4), the value it decodes is logged by the logger plugin with no second mapping. The TTCN-3 export refuses a
     list of optionals (TP section 6.3), so `null` in an array never reaches the TTCN-3 side.
4. **What the lane checks, and what it does not.** In the lane the native side holds values and uses no codec (TP
   section 12.2): the bytes are the binding's, and C3 compares the two runs' logs, not bytes. The corpus's byte cases
   for the two codec documents are data the binding's own tests can hold it to, but running SCE's generated decode on
   the binding's bytes stays outside this record, as docs/SCE_TTCN3_LANE.md section 14 leaves it ("SCE's own codecs
   in the lane"). With the two codec documents, the types the lane's table lists as exported only (lists, records
   within records, a union, a byte string, enums) can be carried on a mapped port once S2's traces exist.

## Conformance

Each test fails when the rule it pins is removed; the landing checks each once against a mutant of that rule.

1. **The cross-backend corpus.**
   - A resource `tests/forge/resources/codec_cbor_composed.scxml` with its imports beside it: an entry of every
     constructor and at depth: `int8`, `int64`, `float32`, `float64`, an enum of a signed carrier, a closed and an open
     enum, `list<uint8>[4]`, `list<optional<uint16>>[3]`, `list<list<uint8>[2]>[2]`, `list<bool>[1..2]`, a record
     holding a list, an optional text and a nested record, an optional record, a union with an empty case, a case
     holding a byte string and a case holding a list, `list<union:U>[1..3]` and `optional<union:U>`; registered in
     `tests/forge/conformance/fixtures.json`, with a field type for a composed value that the harness compares
     structurally.
   - Its cases in the `codecs` section of `numerical_reference.json`: the smallest and the largest value (every list
     full, every text at its bound, every number at its longest head) with their bytes, the largest written at
     exactly `C` bytes; every vector of section 2.8. A composed value is written in a case as section 2.4's tree in
     JSON, its leaves as TP section 12.5 spells them.
   - A new kind of case, `variants`: an encoding the encode never writes, the value it decodes to, and the
     deterministic bytes that value re-encodes to: a longer head (`18 01` for 1), a double that is a single, keys in
     reverse order, an indefinite array (`9f 01 02 ff`), an indefinite map, an indefinite union map
     (`bf 64 63 61 73 68 a0 ff`), a text in chunks (`7f 61 61 61 62 ff` for "ab"), and an undeclared key whose value
     is indefinite. `codec_cbor_map`'s reject "an indefinite-length map" moves here.
   - Its `rejects`, one per row of section 3.2 at the top and at depth 2: each new kind and each renamed one; a chunk
     of another major type (`7f 41 61 ff`), a chunk that splits a code point, `null` in a record's field, `null` under
     an optional field's key, a bignum where a `uint64` stands (`c2 41 01`), a union map of no and of two entries, an
     unknown variant, a fifth element of `list<uint8>[4]` (refused before it is decoded), an empty list of lower count
     1, -129 for an `int8` (`38 80`), a double that is no single for a `float32`. `codec_cbor_map`'s "key 7 is given
     twice" becomes `cbor-duplicate-key` and "a text string where the urgent entry expects a truth value"
     `cbor-wrong-type`.
   - The step-0 cases of section 8.3, unchanged through every step.
   Run on the seven backends by each backend's conformance harness, as `codec_cbor_map` is: Rust, C++, Go, Kotlin,
   Python, C11 and Swift.
2. **Each runtime's CBOR module**: signed integers and floats at every head and precision boundary (23/24, 255/256,
   65535/65536, 2^32 - 1/2^32; a value that is a half, a single, a double only; a NaN); indefinite arrays, maps and
   strings, nested and empty; the skip over an indefinite value; each refusal kind (`cbor_runtime_test` on Rust,
   C++ and C11, and its twin on Go, Kotlin, Python and Swift). The existing tests that refuse an indefinite length
   change to accept it.
3. **`sce-build`**:
   - the reader: every constructor read on a CBOR entry; `sce:max-size` read as the bracket with its
     `bound-attribute-superseded` record, and `bound-spelling`'s rewrite of a CBOR codec generating the same bytes;
     a bracket beside `sce:length` refused; an unbounded text at the top level and nested refused, the three inline
     documents of the Context turned into these refusals; a union import read by a codec; a content-line entry still
     refusing every composition with its candidates unchanged;
   - the plan: a golden of `CborPlan` for `codec_cbor_composed` and for each committed CBOR document, the record's
     fields in the order of their encoded keys;
   - the size: `C` per row of section 4.1, tight against the largest value of the corpus, the codec's
     `<NAME>_MAX_BYTES` equal to it, and `validation/encoded-size-overflow` from a type nested past `2^64 - 1`, at a
     codec entry and at a schema's payload, with its golden;
   - the totality: a codec that can refuse, per backend, names the refusal at its place; `codec_cbor_signed_enum`'s
     entry names none;
   - `cbor_codec::refusal` per backend through the steps, and its removal.
4. **The TTCN-3 corpus**: the two codec documents generate on every backend and round-trip the trace values of
   `order_placed` and `job_status` from `trace_forms/` (section 2.8's bytes among them); the coverage test of section
   9.2 without `NO_CODEC_FORM`.
5. **Mutation casefiles** under `sce-build/tests/mutations`: a union's one-entry check, its unknown-variant check, the
   duplicate-key mark, a nested record's key order, the omitted absent field (written as `null` instead), the
   `null` element, the lower count of a nested list, the running length of a chunked text against its bound, the
   union row of `C`.

## Landing order

Each step leaves every lane green and moves no byte the step-0 cases pin.

0. **Pin.** The byte cases of section 8.3, captured from today's encoders on every backend, added to the corpus. No
   code changes.
1. **Grammar and validator.** `CborEntry.sce_type: ValueType`; the CBOR entry reader by the one type grammar;
   `sce:length`'s rule; `bound_spelling.rs`'s CBOR site; the refusal of an unbounded text; the union import; ADR
   0092 section 1.1 item 3's CBOR half lifted; `validation/encoded-size-overflow`; the AST export and its schema; the
   XSD comments. Every backend refuses by name (section 5.4) every shape step 3a did not lower, so no generated file
   changes. The committed CBOR documents rewritten by `bound-spelling`, and every tree generated from them regenerated
   (section 8.4). The coverage test of the TTCN-3 corpus moves to the new model (section 9.2), its `NO_CODEC_FORM`
   kept. Tests: section "Conformance" item 3's reader and refusals, the goldens.
2. **The plan.** `CborPlan`'s tree, `C` of section 4.1, `codec_totality` over the shape, `<NAME>_MAX_BYTES` from `C`.
   Each printer reads the new plan for the shapes it lowers today, so the regeneration changes only the
   `<NAME>_MAX_BYTES` constants of section 8.4. Tests: the plan goldens, the size and totality tests.
3. **The runtimes**, each with its tests of "Conformance" item 2: signed integers, floats, indefinite lengths, the
   four new kinds and the two renamed refusals, on Rust, C++, Go, Kotlin, Python, C11 and Swift; Rust's view of
   section 3.4 with its migration note. The corpus's renamed rejects and the indefinite-length variant move here.
4. **The backends, one at a time**, each printing the whole shape from the plan and flipping its arm of
   `cbor_codec::refusal`, each held to the composed corpus as it lands: 4a Rust, the reference (with
   `codec_cbor_composed` and its cases); 4b C++; 4c Go; 4d Kotlin; 4e Python; 4f C11; 4g Swift, whose
   `swift_codec::cbor_unlowered` empties. ADR 0081 section 4's order, with Swift last as its section 2 leaves it. A
   backend's step regenerates the committed trees that hold a codec it now lowers differently (none of the committed
   documents' bytes move). The last step removes `lowers_record_list` and the refusal's composition arms.
5. **The corpus**, completed: every case of "Conformance" item 1 on all seven backends, the mutation casefiles.
6. **The lane's fixtures.** The two codec documents, their round trips on every backend, the coverage test without
   `NO_CODEC_FORM`.
7. **The documents.** SCE_FORGE.md section 4.6.1 rewritten to this record (the table, the list of records as a case
   of the rule, the wire and decode rules, the generation of each backend); docs/SCE_ACCEPTED_SUBSET.md's appendix
   row; the catalog of SCE_ERROR_CONTRACT.md; docs/SCE_TTCN3_LANE.md sections 12.1, 12.4 and the table of 12.2;
   docs/SCE_CODEC_CONSUMER_MIGRATION.md (Rust's view, the renamed refusals, the new kinds a consumer's match meets).
   Each earlier step states in place what it lands, in the paragraph of SCE_FORGE.md it changes; this step closes the
   section.

## Consequences

- A CBOR codec carries every value a statechart, an algorithm or an event schema can hold, so a message type is
  codec-able by being typed; the TTCN-3 corpus's message types have codec documents, and the next type the corpus
  adds has one by construction.
- One form per type for a codec, and its tree is the log's. A value moves from a wire to a log to a trace with no
  mapping but the leaves'.
- Two CBOR encodings of a typed value stay in SCE, the codec's and the signature's, differing in an absent field
  (omitted, or `null` under its key) and, once ADR 0092's step 7 lands as written, in a union. Each is stated with its
  reason, and "Open questions" 1 asks about the second.
- A decode reads what any conforming encoder writes, so a peer whose library writes indefinite lengths or wide heads
  interoperates; the cost is a reader on every runtime that walks indefinite items, and a Rust view type for a text
  in chunks.
- Two refusals are renamed and four kinds are added: a consumer that matched `CborMalformed` for a key twice or a
  wrong type changes its match (ADR 0078's note).
- `<NAME>_MAX_BYTES` becomes exact, and smaller for narrow integer and enum entries.
- A CBOR text with no bound is refused; a protocol that states none is written with a named bound.
- `EntryType` remains, for content-line codecs and leaf views; `CborEntryType` and `RecordList` go.

## Alternatives rejected

- **Keep `EntryType` and add constructors one at a time** (signed integers, then lists of scalars, then records),
  as step 3a added one. Each addition is a rule an author learns, and the two message types of the corpus need four
  of them at once; the grammar every other position reads already holds them all.
- **A union beside its tag, as the JSON wire and the signed form write it** (ADR 0092 sections 2 and 5.3). It
  reserves the tag's key in every case's record and makes the union's map the only map whose members are of two
  kinds; its tree is not TP section 12.5's, so the binding would need a second mapping to the log, and TTCN-3's union
  value and X.697's `CHOICE` are named around. The JSON wire's reason for it (a model's constrained decoding) does not
  reach a CBOR reader.
- **A union keyed by the variant's number.** It makes the one nested integer key, and its value names nothing a
  TTCN-3 type or the log names; the tag's enum is closed, so the number covers nothing the name does not.
- **An absent field written as `null`, as the signed form does.** Absence is then two items, a missing key from an
  older sender and `null` from a newer one, unless the decode refuses one of them, and CDDL's `?` and the codec's own
  map already spell absence by omission. The signed form keeps `null` for a reason a wire does not have (its keys a
  function of the revision).
- **A decode that refuses every encoding but the deterministic one.** It refuses values, not errors: an encoder that
  writes `18 01` for 1 or an indefinite array has written a valid item, and a full decoder reads it (RFC 8949 section
  4.1). Determinism matters for bytes a signature covers, which SCE never takes from the wire.
- **Accept every variant except an indefinite length**, as the tree does today. It keeps the Rust view a slice, but
  draws the line at a serialization variant, not at validity, which is the line the owner's decision draws; a
  chunked text costs Rust one view type and no copy.
- **A CDDL document per codec as the source of truth.** CDDL describes items and cannot state the deterministic
  encoding, a bound's limit name or an exact length's place; the codec document already states every fact and the
  CDDL of section 2.6 follows from it. Emitting CDDL from a codec document is a printer this record does not need.
- **Give the content-line codec the same type.** RFC 5545 has no image of most constructors (section 1.4): every one
  of them would be refused at the codec, which is the refusal by shape ADR 0086 removed from the language.

## Open questions

1. **The signed form of a union.** ADR 0092 section 5.3 gives a signed union the tag's number under the tag's key,
   beside the case's fields; its step 7 has not landed, so no signed bytes depend on it. Signing a union in this
   record's form (one entry keyed by the variant's name) would make the codec and the signature differ only in an
   absent field. Recommended: amend ADR 0092 section 5.3 before its step 7 lands, so the trust layer's encoder and
   the codec share the union's writer and its row of `C`.
2. **Typed decode refusals on Kotlin, Python and C++.** The codec convention of those three answers absence for
   every refusal of every codec, positional included (SCE_FORGE.md section 4.6.1), so the refusal kinds of section 3.2
   are named on Rust, Go, Swift and C11 only, and the corpus holds the other three to refusing. Giving them the kind
   changes every generated codec's decode on three backends, a consumer change of its own (ADR 0078). Recommended: a
   record of its own, over every codec, not this one.
