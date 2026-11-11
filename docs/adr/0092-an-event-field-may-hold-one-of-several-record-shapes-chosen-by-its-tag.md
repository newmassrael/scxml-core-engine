# ADR 0092 — An event field may hold one of several record shapes, chosen by its tag

- Status: Accepted (2026-11-06). The owner accepted it under the standing rule of 2026-10-26 and 2026-10-27, the
  most long-term correct structure, textbook in its foundations, cost ignored, and took the recommendation of each
  of the nine questions in "Open questions", which are therefore decided as recommended there. Numbered 0092
  because 0090 is taken on `main` and 0091 on a branch. Amended 2026-11-07: a union variable has no default value
  ("Amendment (2026-11-07)"); amended 2026-11-11: what the Kotlin lane decided where this record is silent
  ("Amendment (2026-11-11)"); what landing step 5 decided is in "Decided while landing step 5 (2026-11-11)" and
  "Decided while landing step 5 on Kotlin, with the crossing (2026-11-11)". Landed on `main` ("Landing order"
  numbers the steps):
  - step 1, the type and its document: f3786292f6 (the kind), 40d50d4506 (the type and its size), a1fcd00872 (the
    revision rule);
  - step 2, the grammar: 62b22223c2 (`S` and `G`), 07f662da90 (the unit's payload schema), 9ec6bdef31
    (`payload-schema`);
  - step 3, the runtimes: ff8c45d2a9 (Rust), 07d8880036 (Go), 5bfc0a6a2a (Kotlin), f6dc8f133a (Python), 0ab2a3307d
    (C++), 9a863ded7d (Swift), b1ac3d69cd (C11), b93168cde0 (the Interpreter), held to one refusal per text by
    3646559436;
  - step 4, on five of the eight lanes: the narrowing and `expression/union-read-unproven` (cb1576c1ed,
    3dfbb314df), the union's unit (049d239b73 Rust; cda35b42f7 and e3c33224b5 Kotlin; fb6c5e760b Swift; 1955e08e52
    Go; the change that adds "Decided while landing ADR 0092 on C++" for C++) and the replay vocabulary
    (dd2d10284a); the statechart's payload read, variable, write, lift and saved state on Rust (f357451c13,
    9b0c344640, f91e3c1d5a, 64e63e4f51, 656fd9b7f8), on Swift (ab6cf2535b, 908c2f7511, ce24a056f8), on Kotlin (the
    change that adds "Amendment (2026-11-11)"), on Go (43698d9e6e) and on C++ (the change that adds "Decided while
    landing ADR 0092 on C++"). Python, C11 and the Interpreter still refuse a statechart that holds a union by name
    (`generate/unsupported-feature`);
  - step 5, on four of the seven generated lanes: the judge of a body that holds a union and the Rust lowering
    (2b7a12c0de), and the Kotlin lowering with a statechart's call that hands a union over or takes one back on
    Rust and Kotlin (the change that adds "Decided while landing step 5 on Kotlin, with the crossing"), the Go
    lowering and crossing (ad4d2f5984, 5ad49544dd) and the C++ lowering and crossing (the change that adds
    "Decided while landing ADR 0092 on C++"). Python, C11, Swift and the Interpreter refuse such an algorithm and
    such a call by name;
  - steps 6 (the standard library's `date_expression` and `resolve_date`), 7 (the deterministic CBOR encoder) and
    8 (the census) have not landed: `stdlib/time/date_expression.scxml` is still the flat record.
- Date: 2026-11-06
- Amends:
  - ADR 0086 section 1.1 (the type grammar gains `union:<alias>`), section 2.1 (the size calculus gains its row),
    section 3.1 (the wire form of a union), section 5.1 ("fits"), section 7.1 (the types each backend generates),
    section 9.2 (the deterministic CBOR of a union), section 10 (how a union grows);
  - ADR 0087 section 1.1 (the folds `S` and `G` gain a row; the payload meta-schema gains `oneOf` and `const`),
    section 2.1 (`<sce:lift>` may read a text into a union), section 4.1 (`sce:answer` may name a union), section 5.1
    (the date expression becomes a union and `resolve_date` takes it); section 2.6's rejection of a sum type "justified
    by one use" is answered here by the uses of section "Context", and the lift's two variables are unchanged;
  - ADR 0055 section 3 (a fact about a subject may say which cases of a union it is in, beside whether it is present);
  - docs/SCE_ACCEPTED_SUBSET.md section 2.1 (a twenty-third kind, `union`) and section 2.15 (a union variable, its
    construction and its narrowing); SCE_FORGE.md section 4.12 (a union local and parameter of an algorithm).
- Scope:
  - the forge model and readers: the kind `union`, the element `<sce:case>`, the type `union:<alias>`
    (`TypeOf::Union` in `sce-build/src/forge/value_type.rs`), `schemas/sce-forge-ext.xsd`,
    `apis/forge-ast.v1.schema.json`;
  - the folds of `value_type.rs` (sizes, depth, longest pointer, `fits`, JSON Schema, GBNF) and
    `schemas/sce-payload-schema.v1.schema.json`;
  - the payload readers and writers of the eight runtimes: Rust, Go, Kotlin, Python, C++, Swift, C11, and the
    Interpreter's `sce-build/src/forge/static_js/sce_static.js`;
  - the statechart and algorithm judges (`forge::static_datamodel`, the algorithm judge), the generator's schema
    units, types, lifts, sends and proven reads on every backend; the saved state and the replay vocabulary;
  - the deterministic CBOR encoder of `sce-rust-trust`;
  - three diagnostic codes (section 9) in `sce-build/src/forge/diagnostic.rs`, SCE_ERROR_CONTRACT.md, the
    acceptance-doc appendix and `schemas/sce-diagnostic.v1.schema.json`;
  - the standard library: `stdlib/time/date_expression.scxml` becomes a union, five case records are added beside
    `civil_date.scxml`, and `resolve_date.scxml` takes the union.
- Related: ADR 0044 (the revisions of a schema), ADR 0047 (the number a variant declares), ADR 0055 (a value that may
  be absent, and narrowing), ADR 0072 (a codec's variant arm), ADR 0077 (a declared bound is part of a field's type,
  and every decode refuses what breaks it), ADR 0078 (a consumer follows SCE one landing at a time), ADR 0086 (one
  type grammar; every composed type has a size the build knows), ADR 0087 (a statechart asks a model for a value of a
  schema).

## Context

**What a consumer measured.** On 2026-11-06 a consumer reported a run of 200 cases of a planner application's date
questions on a phone (Gemma 4 E2B and E4B, on the device, free decoding: no grammar), each answer lifted into a record
that holds `sce:std/time/date_expression` and resolved by `sce:std/time/resolve_date`. It found three kinds of
failure that come from the shape of the record, not from the model's reading of the text:

- **Keys left out.** About 30 answers were refused by the lift as `missing`, at `/weekday`, `/year` or `/count`: the
  model wrote the kind and the numbers that kind reads, and left out the keys of numbers it does not read. The lift
  is right to refuse them: a field of the first revision is carried by every delivery, and a missing key means a
  sender whose revision lacks the field (ADR 0055 section 4; ADR 0087 section 1.1, "the key stays in `required`").
- **Numbers that do not match the kind.** About 15 answers were lifted and then refused by the resolver
  (`precondition`, so no date): a number the kind reads was `null`, or the numbers given were another kind's.
- **Nulls.** Each answer wrote 12 to 15 `null` fields, the numbers its kinds do not read, and on a phone every
  token is decoding time.

The measurements are the consumer's; this repository holds none of them, and this ADR rests on their kinds, not on
their numbers.

**Why the flat record has those failures.** `date_expression` is a record of `kind` (`enum:DateKind`, eleven variants)
and five `optional` numbers, `year`, `month`, `day`, `weekday` and `count` (ADR 0087 section 5.1). Which numbers a kind
reads is stated in three places, none of them the type: a sentence of the document's comment ("a field the kind does
not read is written `null`"), the table of ADR 0087 section 5.1, and the preconditions of `resolve_date.scxml`
(`<sce:require cond="year !== null &amp;&amp; month !== null &amp;&amp; day !== null"/>` and nine more like it).
Every grammar of the record states the five numbers independently, so it admits every combination: eleven kinds,
each with 2^5 patterns of present and absent numbers, 352 shapes, of which eleven are what the kinds read. A
grammar-constrained decoder (ADR 0087, "The textbook structure") is held to the 352; the lift accepts all of them,
and only the resolver, after the lift, refuses the 341 that are wrong, with no pointer into the answer. Since no
artifact a build writes states the rule, a consumer that wanted to describe it in a prompt recovered it by calling
the resolver with each number left out in turn.

The texts themselves show the cost. Four expressions, each as a grammar of the flat record writes it and as a
grammar of one shape per kind would:

| Expression | Flat record (bytes) | One shape per kind (bytes) |
|---|---|---|
| no date | `{"kind":"none","year":null,"month":null,"day":null,"weekday":null,"count":null}` (79) | `{"kind":"none"}` (15) |
| in two days | `{"kind":"relativeDays","year":null,"month":null,"day":null,"weekday":null,"count":2}` (84) | `{"kind":"relativeDays","count":2}` (33) |
| the coming Friday | `{"kind":"comingWeekday","year":null,"month":null,"day":null,"weekday":5,"count":null}` (85) | `{"kind":"comingWeekday","weekday":5}` (36) |
| 3 March 2027 | `{"kind":"absolute","year":2027,"month":3,"day":3,"weekday":null,"count":null}` (77) | `{"kind":"absolute","year":2027,"month":3,"day":3}` (49) |

By the size calculus of ADR 0086 section 2.1, the largest text of the flat record is 105 bytes and the largest of
the shapes below (section 10) is 60.

**What the type grammar has.** `T ::= scalars | enum:<alias> | string[B] | bytes[B] | optional<T> | record:<alias> |
list<T>[B] | list<T>[M..B]` (ADR 0086 section 1.1). A record is a product: it holds every one of its fields. The
grammar has no sum, a value that is one of several shapes and says which. ADR 0087 section 2.6 rejected one for the
lift's result because "the type grammar has no sum type; adding one for this statement would be a constructor of the
language justified by one use". The uses are now several: the date expression; any answer whose fields depend on a
choice the model makes (an action of one of several kinds, a reminder that is either at a time or relative to an
event); and any event whose payload carries one of several records, which today is spelled as the flat record above
or as one event per shape.

**What SCE already has that resembles one.** A codec document (`sce:kind="codec"`) may end in a `<sce:variant>`, a
discriminated union of a foreign frame: a numeric tag field, one `<sce:arm value type>` per tag value naming a codec
body, and a `<sce:default>` for the values no arm names (`schemas/sce-forge-ext.xsd`, the `arm` and `variant`
elements; ADR 0072). Each backend generates it as a sum type ("Rust enum, Kotlin sealed class, C11 tagged union",
`CodecModel::variant`, `sce-build/src/forge/model.rs`). It describes a byte layout SCE does not own; it is not a value
type of the language, it has no JSON form, and no event schema, variable or algorithm can hold one.

**The words already taken.** "Variant" names three things in SCE: an enum's declared name and number
(`<sce:variant name value>` in an enum document), the codec's discriminated-union suffix (`<sce:variant tag>`), and a
buffer pool's discriminator (the XSD's comment on `variant` lists the three). "Arm" is the codec's alternative, the
element ADR 0072 lets a build feature gate. The construct decided here is a value type, chosen by a value of a closed
enum and holding a record, so it takes neither word: it is a **union**, and each alternative is a **case**. The
literature's "variant" (Pierce, below) is the same idea, and the collision is the reason it is not used.

## The textbook structure

**Sums and products.** A record type is a product, holding a value of each of its fields; a sum (a variant type, a
tagged or discriminated union) holds a value of exactly one of its alternatives, labelled, and is taken apart by case
analysis on the label (Pierce, *Types and Programming Languages*, MIT Press 2002, sections 11.9 and 11.10, "Sums"
and "Variants"). The ML
family's datatypes, Rust's `enum`, Swift's `enum` with associated values and Kotlin's sealed hierarchies are sums. A
record of a tag and optional fields is the encoding of a sum into a product, and it holds every value the sum holds
and many it does not.

**Make illegal states unrepresentable.** The phrase is Minsky's ("Effective ML", 2010; "OCaml for the Masses",
CACM 54(11), 2011): a type that admits only the meaningful values moves a class of checks from run time to the type.
It is the rule ADR 0055 applied to absence ("never a companion flag that admits impossible states") and ADR 0086
section 7.4 to bounds. A kind whose numbers are absent is such an impossible state.

**Interface languages state discriminated unions.** ASN.1 `CHOICE` (ITU-T X.680, its choice-type clause), XDR's
discriminated union, a `switch` on a declared enum with one arm per value (RFC 4506 section 4.15), CDDL's choices
(RFC 8610 section 2.2.2), Protocol Buffers' `oneof` and Avro's unions all let a protocol say "one of these shapes".
XDR's form is the closest to this ADR: the discriminant is a value of a declared enumeration, and each value of it
names the shape that follows.

**Narrowing by a test of the tag.** Occurrence typing gives a variable a narrower type in the branch where a test on
it is known to hold (Tobin-Hochstadt and Felleisen, "Logical Types for Untyped Languages", ICFP 2010). TypeScript's
discriminated unions apply it to a property: in the branch where `e.kind === "absolute"` holds, `e` has the fields of
that case and no other (the TypeScript Handbook, "Narrowing", discriminated unions). ADR 0055 section 3 is the same
analysis for `x !== null`; the forge expression language is a typed subset of ECMAScript, so the test a document
already writes for an enum is the test that narrows.

**One spelling on the wire.** JSON has no sum. A sum is spelled as an object that carries its label: beside the
fields ("internally tagged"), around them ("externally tagged"), or as two keys, label and content ("adjacently
tagged"); serde names the three ("Enum representations", serde.rs). Section 2 chooses among them.

**Constrained decoding follows the grammar.** A decoder held to a grammar can only write sentences of it (Geng,
Josifoski, Peyrard and West, EMNLP 2023; ADR 0087, "The textbook structure"). The grammar of a sum is an alternation,
one alternative per case; if each alternative begins with its label, the decoder's choice of label is the choice of
case, and every token after it is held to that case's fields alone.

## Decision

**The type grammar gains a sum: `union:<alias>`, a value that is exactly one of the cases a `union` document declares.
A union document declares one tag, a closed enum, and one case per variant of that enum, each case a record (an
imported event schema) or no record at all. On the wire a union is one object, its tag under a key the document names
and the case's fields beside it. Its JSON Schema is a `oneOf` of one object per case, each with its tag as a `const`;
its GBNF grammar is one alternative per case, the tag first. Every reader reads the tag first and then exactly the
case's record, and refuses with the codes it already has. A statechart or an algorithm reads a case's fields where a
test of the tag proves the case. Each backend generates its own sum type. `sce:std/time/date_expression` becomes a
union with one case per date kind, so the rule of which numbers a kind reads is its type.**

### 1. The type and the document that declares it

#### 1.1 Rule

1. **The document.** A `union` document is a kind of its own, as an enum and an event schema are:

   ```xml
   <scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
          version="1.0" sce:kind="union" name="date_expression">
     <sce:import kind="enum" src="date_kind.scxml" as="DateKind"/>
     <sce:import kind="event-schema" src="civil_date.scxml" as="CivilDate"/>
     <sce:import kind="event-schema" src="month_day.scxml" as="MonthDay"/>
     <sce:import kind="event-schema" src="date_shift.scxml" as="DateShift"/>
     ...
     <datamodel>
       <data id="kind" sce:type="enum:DateKind">
         <sce:case variant="none"/>
         <sce:case variant="absolute" record="CivilDate"/>
         <sce:case variant="monthDay" record="MonthDay"/>
         <sce:case variant="relativeDays" record="DateShift"/>
         <sce:case variant="relativeWeeks" record="DateShift"/>
         ...
       </data>
     </datamodel>
   </scxml>
   ```

   - **The tag.** The datamodel holds exactly one `<data>`, the tag. Its `id` is the key the tag is written under
     (section 2) and its `sce:type` is `enum:<alias>` of a closed enum the document imports. An open enum
     (`sce:strict-variants="false"`) is refused: a number no variant names would be a value with no case.
   - **The cases.** One `<sce:case>` per variant of the tag's enum, in any order, naming it by `variant`. The cases
     and the variants are one to one: a variant with no case, a case naming no variant, and a variant given two
     cases are refused (`validation/union-cases`, section 9). The case's `record` names an imported event schema, the
     record the case holds; a case with no `record` is empty, its tag alone. A union all of whose cases are empty is
     refused by the same code: it is its enum, and one fact has one spelling.
   - **The tag's key is no field.** No case's record declares a field whose `id` is the tag's key
     (`validation/union-tag-collision`), since the two would stand under one key of one object (section 2).
   - **Two cases may hold one record.** `relativeDays` and `relativeWeeks` both hold `DateShift`; the tag tells them
     apart, and the record is one type wherever it is held.
2. **The type.** `union:<alias>` names the union a document imports (`<sce:import kind="union" src as>`). The grammar of
   ADR 0086 section 1.1 gains one line:

   ```
   T ::= ... | record:<alias> | union:<alias> | list<T>[B] | list<T>[M..B]
   ```

   A union states no bound of its own: its largest value is its largest case (section 5). A bracket on it is refused
   as a bracket on a record is (ADR 0086 section 11, `validation/attribute-rule-violated`).
3. **Where it stands: everywhere a type does.** An event schema's field, at any depth: inside a list
   (`list<union:Shape>[1..3]`), an optional (`optional<union:Shape>`) and a record's field; a `sce-static`
   variable; an algorithm's parameter, return and local; the `schema` of `<sce:lift>` and the `sce:answer` of a model
   invocation (section 4). A case's record may hold any type, a union included. ADR 0086's orthogonality (its answer
   A) is kept: nothing is refused by its shape except what section 1.5 of that ADR refuses, and a union that holds
   itself through its cases' records is a type that contains itself (`validation/cross-kind-circular-dependency`).
   Three positions refuse it, each for a reason that is not the union's:
   - an event's payload as a whole is a record, its event schema; a union is a field's type. An event whose payload
     is one of several shapes holds one union field, or is several events, as SCXML dispatches by event name;
   - a CBOR or content-line codec's entry is an `EntryType` (ADR 0086 section 1.7) and describes a foreign frame,
     which says its alternatives with `<sce:variant>`: `validation/invalid-attribute`, with the entry types;
   - a `sce:request` schema of a host-run invocation is read back from the `params` carrier, which has no object, so
     a union field there is refused as an optional one is (ADR 0055 section 2, `validation/typed-invoke-schema`). A
     `sce:result`, which the host completes with JSON, may hold one.
4. **The model.** `TypeOf` gains `Union { alias: String }`, beside `Record`. Every fold over the type is an exhaustive
   match (ADR 0087 section 1.1 item 1), so each one fails to compile until it handles the constructor. `TypeEnv` gains
   `union_type(alias)`, which answers the tag's key, the tag's enum and each case with its variant's name and number
   and its record's alias, in the enum's declaration order; the forge-AST export writes `{"union": "DateExpression"}`,
   so every export of a document that holds no union is unchanged.

#### 1.2 Reason

The tag is an enum because everything a tag needs, an enum already has: a closed set of names, a declared number for
each (ADR 0047), revisions that add names and never renumber one (ADR 0044, "An enum that grows"), `fromNumber` and
`toNumber`, a JSON spelling, a grammar, and a refusal for a name it does not declare. A union that declared its own
labels would be a second enum. The cases are records because a record is already a type that can be held, passed,
compared field by field, bounded and lifted; a case written as fields inside the union document would be a second
record grammar, and its value could not be handed to an algorithm that takes the record (the `absolute` case holds
`civil_date`, which `days_from_civil`'s callers already use). The one-to-one rule makes the union total over its
enum, so a revision that adds a kind cannot build until the kind has a case.

#### 1.3 Alternatives rejected

- **Labels declared by the union** (`<sce:case name="absolute" value="1">`). A second enum, with its own numbering
  and revisions to keep in step with the one the resolver already reads.
- **Cases declared inline** (`<sce:case variant="absolute"><data id="year" .../></sce:case>`). See the reason above.
- **A sum of arbitrary types** (`union<int64, string[8]>`, untagged). JSON cannot tell some of them apart (an
  `int64` is a quoted text, ADR 0086 section 3.1), a reader would have to guess, and a model would have no label to
  choose.

### 2. The wire: one object, the tag beside the case's fields

#### 2.1 Rule

1. **The form.** A value of `union:U` in case `c` is a JSON object holding the tag's key, whose value is `c`'s
   variant name as the enum spells it (ADR 0086 section 3.1), and the fields of `c`'s record, each spelled by its
   type. An empty case is the tag alone:

   | Value | Text |
   |---|---|
   | `none` | `{"kind":"none"}` |
   | `relativeWeeks`, `count` 2 | `{"count":2,"kind":"relativeWeeks"}` |
   | `absolute`, 2027-03-03 | `{"day":3,"kind":"absolute","month":3,"year":2027}` |

   A writer writes the object's keys in the one ascending order of ARCHITECTURE.md ("JSON Object Key Order"), the tag
   among them; the grammar of section 3 writes the tag first. The union adds no level and no key of its own: a
   field of the case stands at the union's place, so the pointer to it is the union's pointer and the field's id
   (`/when/count`).
2. **Every key of the case is carried.** A case's record is read as a record is: a first-revision field is always
   carried, `null` only where its type is optional, and a field a later revision of that record added is read from
   its defaults (ADR 0086 section 10.3). A key the case does not declare is stepped over, as a key a record does not
   declare is (ADR 0086 section 4.1), so a text that carries another case's keys is read as the case its tag names.
3. **The flat record's texts.** It follows from item 2 that every text of the flat `date_expression` whose kind's
   numbers are present is read as the union's value of that kind: the `null`s under the other numbers' keys are
   stepped over. The texts it does not read are the ones `resolve_date` refused (section 10.4).

#### 2.2 Reason

Of the three spellings of a sum in JSON, the tag beside the fields is chosen:

| Spelling | `relativeWeeks`, `count` 2 | Against it |
|---|---|---|
| beside (chosen) | `{"kind":"relativeWeeks","count":2}` | the tag's key is reserved in every case's record (item 1.1, the collision rule) |
| around | `{"relativeWeeks":{"count":2}}` | the tag is a key, not a value of the enum, so an unknown one is a key stepped over and the answer is then refused as `missing` somewhere else; an empty case is `{"none":{}}`; one level deeper |
| two keys | `{"kind":"relativeWeeks","value":{"count":2}}` | one level and one key more in every answer; a case's fields are no longer at the union's place |

The chosen spelling is the smallest (the decoding cost the measurement found), keeps the depth of a record
(`depth(R)` of ADR 0087 section 2.1 item 4 counts a union as one level, as it counts a record), keeps the tag a value
of its enum (so its refusal is the enum's, section 4), and is the shape the flat record's texts already have, so a
consumer's stored answers stay readable.

### 3. The grammar of a union

#### 3.1 Rule

1. **The folds gain one row** (ADR 0087 section 1.1, item 1):

   | Type | `S(T)` | `G(T)` |
   |---|---|---|
   | `union:U` | `{"$ref":"#/$defs/<U's document name>"}`, defined once in `$defs` as `{"oneOf":[o1, ..., on]}`, one object per case in the enum's declaration order: `{"type":"object","properties":{"<tag>":{"const":"<variant>"}, each field of the case's record: S(Tf)},"required":[the tag, then every field],"additionalProperties":false}` | one rule per union document: one alternative per case, each `"{"` the tag's key `":"` the variant's quoted name, then `","` each field of the case's record in the order its document declares them `":"` its value, then `"}"` |

   The tag stands first in each case's `properties`, `required` and grammar, then the record's keys in their declared
   order, which is the order a decoder held to the schema writes them in (ADR 0087 section 1.1 item 2, 2026-11-05).
   A case's fields are written into its object, not referred to by a `$ref` to the record's definition: a record's
   definition says `additionalProperties: false`, which would refuse the tag beside them, and composing the two with
   `unevaluatedProperties` is a keyword the converters that turn a schema into a grammar hold least.
2. **Exactly one case matches, by construction.** The `const` of each object's tag is a different name, so no value
   is an instance of two of the objects; `oneOf` and `anyOf` therefore accept the same values, and `oneOf` is written
   because it states what the type is, one case and only one. A grammar converter has no way to enforce the
   exclusivity `oneOf` adds, and needs none here. Checked against the two converters consumers use (2026-11-06):
   llama.cpp's `common/json-schema-to-grammar.cpp` builds `oneOf` and `anyOf` as the same node and writes it as one
   alternation (`_generate_union_rule`, `string_join(rules, " | ")`); llguidance (the constraint engine of LiteRT-LM,
   pinned at 1.3.0 there) refuses a `oneOf` it cannot prove disjoint unless `coerce_one_of` is set (default false,
   `process_one_of` in `parser/src/json/compiler.rs`), and its `normalize` (`parser/src/json/schema.rs`) turns a
   `oneOf` into `anyOf` when every pair is verifiably disjoint, which two objects are when a key both require holds
   different string `const` literals (`is_verifiably_disjoint_from`: the object arm over `required`, the literal arm
   `lit1 != lit2`). The cases written here meet that test by construction: the tag is required in every case, its
   `const` is a literal, the literals differ, and the cases are written inline (a `$ref` is never proven disjoint).
   The schema emitter's tests therefore pin that the tag is in each case's `required` and is a string `const`, since
   dropping either would turn the accepted `oneOf` into a refused one on llguidance. `Keyword` (`value_type.rs`) gains `OneOf` and `Const`, `Const` before `Enum` and `OneOf` after
   `AnyOf` in the one keyword order, and `schemas/sce-payload-schema.v1.schema.json` admits both, additively.
3. **Example.** Three of the eleven cases of `date_expression`, revision 2:

   ```json
   "$defs":{"date_expression":{"oneOf":[
     {"type":"object","properties":{"kind":{"const":"none"}},"required":["kind"],"additionalProperties":false},
     {"type":"object","properties":{"kind":{"const":"absolute"},
       "year":{"type":"integer","minimum":-2147483648,"maximum":2147483647},
       "month":{"type":"integer","minimum":0,"maximum":255},
       "day":{"type":"integer","minimum":0,"maximum":255}},
      "required":["kind","year","month","day"],"additionalProperties":false},
     {"type":"object","properties":{"kind":{"const":"relativeDays"},
       "count":{"type":"integer","minimum":-2147483648,"maximum":2147483647}},
      "required":["kind","count"],"additionalProperties":false},
     ...]}}
   ```

   ```
   date-expression ::= de-none | de-absolute | de-relativeDays | ...
   de-none         ::= "{\"kind\":\"none\"}"
   de-absolute     ::= "{\"kind\":\"absolute\",\"year\":" int32 ",\"month\":" uint8 ",\"day\":" uint8 "}"
   de-relativeDays ::= "{\"kind\":\"relativeDays\",\"count\":" int32 "}"
   ```

   (Rule names as `GbnfRules` names them; `int32` and `uint8` are the range rules of ADR 0087 section 1.1.) Every
   alternative begins `{"kind":"`; the variant's name and its closing quote decide the case, and from there a decoder
   writes that case's keys, in order, and no other.
4. **A union as the whole answer.** A unit of a union document declares `SCHEMA_ID`
   (`urn:sce:schema:<document>:<revision>`, the revision of section 8), `JSON_SCHEMA` and `DATA_MAX_LEN`, as a schema
   unit does, so a union may be named as an answer by itself. Its root is the `oneOf`, not an object; a consumer whose
   provider asks for an object at the root names a record that holds the union instead (`{"when": ...}`).

#### 3.2 Alternatives rejected

- **`if`/`then`/`else` on the tag** (one object of every key, with a conditional subschema per kind requiring that
  kind's keys). A grammar is a context-free alternation; a conditional subschema has no direct form in one and would
  have to be compiled into the alternation this section writes directly, and the object of every key still admits a
  key of another kind wherever the conditions do not forbid it.
- **OpenAPI's `discriminator`.** It is a keyword of OpenAPI's Schema Object, in no vocabulary of JSON Schema 2020-12;
  a 2020-12 validator reads it as an unknown keyword and enforces nothing, and OpenAPI itself places it beside a
  `oneOf` or `anyOf` that does the work. The `const` on each case is the portable statement of the same fact.
- **A one-name `enum` for the tag** (`{"enum":["absolute"]}`). It accepts the same values as `const`; `const` says
  that the key holds one value, which is what a reader of the schema is told.

### 4. Every reader reads the tag first

#### 4.1 Rule

1. **The order of the refusals**, the same on every runtime, so one text is one refusal everywhere (ADR 0087 section
   2.1 item 5): a value that is not an object (`wrongKind`; `notObject` for the whole text of a lift); then the tag,
   looked up by its key wherever it stands in the object: absent (`missing`, pointer the tag), `null`
   (`nullNotOptional`), not a text (`wrongKind`), a name the enum does not declare (`unknownVariant`); then the case's
   record, by the reader of a record, its first refusal pointing at the field (`missing` then means a field the chosen
   case reads).
2. **No new refusal code.** The tag is a value of a closed enum, so a tag that names no variant is exactly the fact
   `unknownVariant` states ("a name the enum does not declare", ADR 0087 section 2.3), with the pointer at the tag. A
   code of its own (`unknownCase`) would split one fact in two. Every other refusal of a union is a refusal of its
   tag or of its case's record, with the codes those already have, so `sce:std/lift/refusal_code.scxml` keeps its
   fourteen codes.
3. **The depth and the pointer.** `depth(union:U)` is one level plus the deepest field of any case, as a record's;
   the longest pointer into a union is the longest into any case (`longest_pointer`, `validation/lift-path-bound`).
4. **Each runtime's reader** gains one combinator beside the record's: Rust `PayloadValue`, Go `LiftPayload`, Kotlin
   `EventPayload.Fields`, Python `_decode`, C++ `EventPayloadFields`, Swift `PayloadFields`, C11 `sce_payload_*` (the
   tag looked up in the `sce_payload_fields_t` of the object, then the case's fields from the same object), and the
   Interpreter's descriptor `{union: {tag: "kind", cases: {"none": null, "absolute": {record: {...}}, ...}}}`. Each
   writer gains the case: the tag's name and the record's fields, in the key order of item 2.1.
5. **`<sce:lift>` and `sce:answer`.** The lift's `schema` may name an imported union, and its `location` is then a
   variable of `union:U` or `optional<union:U>` (ADR 0087 section 2.1 item 2, `validation/lift-location`); a model
   invocation's `sce:answer` may name one (ADR 0087 section 4.1), and the machine's answer-schema table holds its
   JSON Schema and GBNF grammar as for a record.

### 5. Sizes, equality and signing

1. **The size calculus** (ADR 0086 section 2.1) gains one row. For a case `c` with variant name `nc`, declared number
   `kc` and record fields `f1..fm` (none for an empty case), and `t` the tag's key:

   | Type | `J(T)`, bytes of JSON | `C(T)`, bytes of deterministic CBOR |
   |---|---|---|
   | `union:U` | the largest over the cases of 2 + (`J(quoted t)` + 1 + 2 + the escaped length of `nc`) + Σ (`J(quoted id)` + 1 + `J(Tf)`) + m | the largest over the cases of h(1 + m) + `C(t)` + the head of `kc` (ADR 0086 section 9.2's enum rule) + Σ (`C(id)` + `C(Tf)`) |

   Each case is counted with its own name and number, so the size is tight (ADR 0086 section 2.3): a maximal value
   is the largest case at its maximum. C11's storage is the compiler's `sizeof` of the layout of section 7, the tag
   and the largest case's record.
2. **Equality and order.** Two values of a union are equal when they are in one case and their records are equal,
   which every generated type implements structurally (section 7) and the saved state and the replay vocabulary
   compare. A statechart compares a union as it compares a record: not at all, `===` of two unions being refused as
   a composed value compared is (ADR 0086 section 6.1, `expression/unsupported-construct`); `e.kind === DateKind.x`
   is a comparison of enum values. There is no order on a union, as there is none on a record.
3. **Deterministic CBOR** (ADR 0086 section 9.2) gains one mapping: a union is a map of the tag's key, whose value is
   the case's declared number (as an enum is signed), and the case's fields, keys sorted by the bytes of their
   encodings. Vectors, each a test of the encoder:

   | Value | Encoding |
   |---|---|
   | `none` (0) | `a1 64 6b 69 6e 64 00` |
   | `relativeDays` (3), `count` 2 | `a2 64 6b 69 6e 64 03 65 63 6f 75 6e 74 02` |
   | `absolute` (1), 2027-03-03 | `a4 63 64 61 79 03 64 6b 69 6e 64 01 64 79 65 61 72 19 07 eb 65 6d 6f 6e 74 68 03` |

### 6. A statechart and an algorithm read a union by testing its tag

#### 6.1 Rule

1. **The tag is always readable.** `e.<tag>` (`when.kind`) is a value of the tag's enum wherever `e` is a value of
   `union:U`, and is compared as one.
2. **A case's field is read where its case is proven.** A **subject** is what ADR 0055 section 3 calls one: a
   variable, a field of a record variable, a field of the payload, a field of a `<foreach>`'s item, an algorithm's
   parameter or local, and those followed by `.value` of an optional proven present or by a field. For a subject of
   `union:U` the build keeps, beside whether it is present, the set of cases it may be in, all of them until a test
   says otherwise. A **test** is `s.<tag> === E.v` or `E.v === s.<tag>` (the subject in case `v` where it holds,
   not in `v` where it does not) and `!==` the other way round. `s.f` is admitted where every case of the set holds a
   record that declares `f` with one type, and is of that type; elsewhere it is `expression/union-read-unproven`
   (section 9), naming the cases that declare `f` and the test that proves one. A field no case declares is
   `expression/unknown-member`.
3. **Which constructs narrow, and what ends a fact**: exactly ADR 0055 section 3's, with its rules for `&&`, `||`,
   `!`, `? :`, `<if>`/`<elseif>`/`<else>`, an algorithm's `<sce:if>`, `<sce:while>`, `<sce:require>` and early
   return, a transition's `cond` for payload subjects only, and its invalidations (an assignment to the subject or a
   path above or below it, an assignment of a call's result, a `<script>`, an `idlocation`). An `<else>` after a chain
   of tests leaves the cases no `cond` of the chain named.
4. **A point where a subject has no case left is not reached.** After a chain of tests whose blocks each end in
   `<sce:return>`, a subject whose every case was named has none left, and the statements after the chain are read as
   unreachable: an algorithm whose path reaches the end of its body only through such a point needs no return there
   (`algorithm/return-missing` is not raised for it). A union that later gains a case has that case left after the
   chain, so the path is reachable again and the build refuses the body with `algorithm/return-missing` until it
   handles the case: a kind a revision adds is found where it is not resolved, at build time.
5. **A union is built whole.** A variable or an algorithm's local of `union:U` is built as a record is
   (docs/SCE_ACCEPTED_SUBSET.md section 2.15, "Record variables"; SCE_FORGE.md section 4.12), from `<sce:set>`
   children: one names the tag, with a variant of its enum written as a literal (`expr="DateKind.relativeWeeks"`),
   and one names each field of that case's record, exactly once. A tag's `expr` that is not such a literal is refused
   (`scxml/static-datamodel-rule`; in an algorithm `expression/unsupported-construct`): the build must know the case
   to know which fields follow. A whole union is put into one by the rules a record takes one by: from another value
   of the same union by its name (a variable, a payload field, a `<foreach>` item, a list's element at an index,
   what an algorithm returns), from a lift, and `null` into `optional<union:U>`.
6. **A field is written where its case is proven**, `<assign location="when.count" expr="3"/>` (in an algorithm
   `<sce:assign target>`), by item 2's rule. The tag alone is never written: `<assign location="when.kind">` would
   leave the fields of the old case under the new tag, so it is refused (`scxml/static-datamodel-rule`), and the
   case changes by a whole assignment.
7. **A union crosses as a composed value does.** It is sent, forwarded and returned by its name to a field whose type
   it fits (ADR 0086 section 5.2); "fits" gains one rule: `union:U` fits `union:V` when both name one document or two
   revisions of one union (section 8). It is held in a saved state as its JSON object (section 2), and the saved
   shape names it by its document and each case's record shape, in the enum's order, so a change of a case moves
   the shape as a change of a record does (ADR 0045). The replay vocabulary writes it as the same object in `data`
   and `variables`, and `sce_build::scenario_replay` reads it by the union's type.

#### 6.2 Reason

The test that narrows is the test a document already writes to ask which kind it holds, and its analysis is the one
ADR 0055 built for absence, extended from a two-point fact (present or not) to a set of cases. Reading a field by its
own name (`when.count`) keeps the statechart's text as close to the wire as a record's is; a case's record is still
one type in the generated code (section 7). The rule of item 4 is what turns the precondition "a kind this document
does not resolve" from a run-time failure into a build refusal.

### 7. The types each backend generates

| Backend | `union:U` | A case holding record `R` | An empty case | `e.kind` | A proven read of case `c` |
|---|---|---|---|---|---|
| Rust | `enum U { ... }`, `Clone`, `PartialEq`, `Debug` | `Absolute(R)` | `None` | `fn kind(&self) -> E` | one generated method per case, `proven_absolute(&self) -> &R`, whose other arms are `unreachable!()`, the one place they are written (as `proven` is for an optional, ADR 0055 section 5) |
| Kotlin | `sealed interface U` | `data class Absolute(val value: R) : U` | `data object None : U` | `val kind: E` on the interface | `provenAbsolute()` on the interface, written once |
| C++ | `std::variant<U::None, U::Absolute, ...>` in a struct `U` | `struct Absolute { R value; }` | `struct None {}` | `kind()` from the alternative's index | `std::get_if<U::Absolute>` dereferenced, as `*x` is for an optional |
| Go | a struct of an unexported tag and one unexported member per case holding a record, built only by `UAbsolute(r R) U`, `UNone() U` | the member | no member | `Kind() E` | `ProvenAbsolute() R` |
| Python | `U = Union[UNone, UAbsolute, ...]` | `@dataclass class UAbsolute: value: R`, checked in `__post_init__` | `@dataclass class UNone` | a `kind` class attribute of each case class | the object itself, after `isinstance` (Python 3.9 has no `match`) |
| Swift | `enum U: Equatable, Sendable` | `case absolute(R)` | `case none` | `var kind: E` | a computed `provenAbsolute: R`, written once |
| C11 | `typedef struct { <enum>_t tag; union { R absolute; ... } as; } <doc>_t;` | the union member | no member (C11 has no empty struct) | `.tag` | `.as.absolute` |
| Interpreter | a plain object, the wire's form | its fields beside the tag | the tag alone | `e.kind` | `e.f` |

1. **Names.** A case's identifier is its variant's identifier as each backend already spells the enum's variant (the
   naming module that spells `DateKind.none` as Rust's `DateKind::None` and Python's escaped name), prefixed by the
   union's type name where the backend has no nesting (Go, Python, C11), so a union is spelled with no name the
   enum's spelling has not already proven safe.
2. **One alternative per case, even where two cases hold one record.** A `std::variant` whose alternatives repeat a
   type cannot name one of them by type, which the generated code does (ADR 0072, "a variant's alternatives are
   distinct types"); each case's alternative is a struct of its own holding the record, so `relativeDays` and
   `relativeWeeks` are two types holding one `DateShift`. Every other backend holds the record in the case directly,
   since its sum names a case by its constructor.
3. **The values are built only whole.** Each backend's constructors build a case from its record (C11's empty-case
   constructor zeroes the union, so two equal values are the same bytes, as the C11 optional's absent constructor
   zeroes its value, ADR 0055 section 5); a host's injected value is checked where it is made, by its record's own
   check (ADR 0086 section 5.4).
4. **Go holds a value, not an interface.** A sealed interface with an unexported method is Go's usual sum, but a
   value held in an interface may be moved to the heap, and Go's optional of ADR 0055 (the owner's answer 2) was
   chosen to copy by value, never allocate and never alias. The struct of item 1 keeps that, at the cost of storage
   equal to the sum of the cases' records rather than the largest (open question 7).
5. **The Rust machine** holds every bound in its type (ADR 0086 section 7.4): a case's record is the machine's record,
   so a union of records of bounded types is a value of bounded types with no rule of its own.

A backend or position that does not lower a union yet refuses it by name (`generate/unsupported-feature`, naming the
field and the constructor), as ADR 0086 section 7.3 does for a composition; that is a step of the landing, not a rule
of the language.

### 8. How a union grows

1. **A union's revision is its tag enum's revision.** It declares none of its own: its cases are its enum's
   variants, one to one, so the union changes exactly when its enum gains a variant, and the revision a unit's
   `SCHEMA_ID` carries is the enum's (`date_expression` over `DateKind` revision 2 is
   `urn:sce:schema:date_expression:2`). A variant the enum adds by a revision (`sce:since="k"` on it) has its case in
   the same change, which the one-to-one rule of section 1.1 requires.
2. **Two builds' unions are revisions of one** (ADR 0044 decision 1, extended) when they write the tag under one
   key, their tag enums are revisions of one enum, and each case both declare holds one record or two revisions of
   one record schema (ADR 0086 section 10.3). Anything else (another key, a case moved to another record, a case
   removed) parts them, and `schema_revision::divergence` names the union, as `mesh/event-schema-mismatch` names a
   field.
3. **An older reader and a newer value.** A value in a case a newer revision added reaches an older reader with a tag
   its enum does not declare, and is refused there as `unknownVariant`, `error.execution` for a delivery, as a newer
   enum value is (ADR 0044 decision 6). A deployment that adds a case upgrades the readers before the writers, and
   the build names the machines that exchange the event. A newer reader reads every older value.

### 9. Diagnostics

Three codes, each with a golden, a spec anchor naming this ADR's section, a row in docs/SCE_ACCEPTED_SUBSET.md's
appendix (`acceptance_doc_covers_every_code`), the generated catalog of SCE_ERROR_CONTRACT.md section 5 and
`schemas/sce-diagnostic.v1.schema.json`. None carries a fix: each repair is the author's choice.

| Code | Stage | Refuses | Section |
|---|---|---|---|
| `validation/union-cases` | validation | cases that do not match the tag enum's variants one to one (a variant with no case, a case naming no variant, a variant given two), naming each; a union whose every case is empty | 1.1 |
| `validation/union-tag-collision` | validation | a case's record that declares a field whose `id` is the tag's key, naming the record, the field and the case | 1.1 |
| `expression/union-read-unproven` | expression | a read or a write of a case's field where the build has not proven the union in cases that each declare it, naming the cases that declare it and the test that proves one | 6.1 |

Existing codes keep their classes: an open enum as the tag, no tag or two, or a bracket on a union is
`validation/attribute-rule-violated`; an alias the document does not import is `validation/invalid-attribute` with the
candidates, and so is a union on a codec entry; a union field in a `sce:request` is `validation/typed-invoke-schema`; a
union that holds itself is `validation/cross-kind-circular-dependency`; a tag written alone, or built from an `expr`
that is not a variant literal, is `scxml/static-datamodel-rule` (`expression/unsupported-construct` in an algorithm);
two builds whose unions part are `mesh/event-schema-mismatch`; a lane that does not lower a union yet is
`generate/unsupported-feature`. A refusal at run time is a lift's refusal record or a delivery's `error.execution`, not
a diagnostic.

### 10. The standard library: `date_expression` becomes a union

#### 10.1 The cases

`sce:std/time/date_expression.scxml` becomes the union of section 1.1, its tag `kind` over `DateKind` (revision 2),
one case per kind, each case's fields in the order the flat record declared them:

| Case (number) | Record | Fields |
|---|---|---|
| `none` (0) | none | |
| `absolute` (1) | `civil_date.scxml` (exists) | `year` `int32`, `month` `uint8`, `day` `uint8` |
| `monthDay` (2) | `month_day.scxml` (new) | `month` `uint8`, `day` `uint8` |
| `relativeDays` (3), `relativeWeeks` (4), `relativeMonths` (5), `relativeYears` (6) | `date_shift.scxml` (new) | `count` `int32` |
| `comingWeekday` (7), `previousWeekday` (8) | `date_weekday.scxml` (new) | `weekday` `uint8` |
| `weekdayOfWeek` (9) | `week_weekday.scxml` (new) | `weekday` `uint8`, `count` `int32` |
| `dayOfMonthOffset` (10) | `month_offset_day.scxml` (new) | `day` `uint8`, `count` `int32` |

`date_kind.scxml` is unchanged: its names, numbers and revisions are the tag's. Every number is present in the case
that reads it; none is optional, and the five `optional` fields and the sentence "a field the kind does not read is
written `null`" go.

#### 10.2 The resolver takes the union

`resolve_date` becomes `(today int64, expression union:DateExpression) -> optional<int64>`, `may-fail`, one
`<sce:if>` per case that tests the tag and returns:

```xml
<sce:if cond="expression.kind === DateKind.absolute">
  <sce:return expr="daysOf(expression.year, expression.month, expression.day)"/>
</sce:if>
```

The ten `!== null` preconditions go, since a case's numbers are present by its type; the trailing
`<sce:require cond="kind === DateKind.weekdayOfWeek"/>` goes, since after the eleven tests no case is left (section
6.1 item 4), and a kind a later revision adds is refused at build (`algorithm/return-missing`) until the resolver
handles it, where the comment of today's document says it "fails here until this document resolves it" at run time.
What stays is what no type of the grammar states: a month within 1 to 12, a day the month has, a weekday within 0 to
6, a `dayOfMonthOffset` day within 1 to 31, and the `overflow` failures. The resolver takes the union only once an
algorithm's slot may hold one (Landing order, step 5); until then it is not changed.

#### 10.3 Reason

The kind-to-numbers rule moves from three statements (a comment, an ADR table, ten preconditions) into one, the type,
which every artifact a build writes then states: the JSON Schema and GBNF grammar a consumer gives its model, the
generated types its host compiles against, the lift that reads the answer and points at the number a kind lacks. On
the measurement's three failures: a key the kind does not read is no longer one the text must carry; a number the kind
reads and the answer lacks is refused by the lift as `missing` with its pointer, which the repair loop of ADR 0087
section 6 can name, rather than by the resolver with none; a number of another kind is stepped over under free
decoding and unwritable under a grammar; and no `null` is written.

#### 10.4 What a consumer migrates

- **Its stored texts** need nothing: a flat text whose kind's numbers are present is read as the union's value of that
  kind (section 2.1 item 3). The texts the union does not read are those whose kind lacks a number it reads, which
  the resolver already refused, and those whose `kind` is an unknown name, which both refuse.
- **Its code** changes where it reads the generated `DateExpression` type, from a struct of a kind and five optional
  numbers to the backend's sum (section 7), and where it calls `resolve_date`, which takes the union. The landing
  records the migration in docs/SCE_CODEC_CONSUMER_MIGRATION.md (ADR 0078: each API change says how).
- **No alias of the flat record is kept.** Keeping it under another name would keep a second statement of the rule
  this section moves into the type, and every value of it that resolves is a value of the union already. The schema
  id moves with the change (`urn:sce:schema:date_expression:1` to `...:2`, section 8.1), so no host table holds one id
  for two schemas.

### 11. Wire surfaces (SCE_WIRE_CONTRACTS.md)

No status flips; every change is additive, each surface `pre-release` except the replay scenario.

- **Authoring grammar** (`pre-release`): the kind `union`, the element `<sce:case>` (attributes `variant`, required,
  and `record`) as a child of a union document's `<data>`, the import kind `union`, and `union:` in the type alphabet.
- **Forge AST export** (`pre-release`): the kind, and the type `{"union": "<alias>"}`.
- **Payload schema** (`pre-release`): the keywords `oneOf` and `const`, and the unit of a union document.
- **Diagnostics** (`pre-release`): three codes, no fix kind.
- **Replay scenario** (`registered`): a union value is the object of section 2, judged by the union's type; the
  checker and every driver change with it, additively.
- **Saved state**: a union is held as its JSON object, which `schemas/sce-saved-state.v2.schema.json`'s value already
  admits as an object.

### 12. Tests on every lane

Each test fails when the rule it pins is removed; the landing checks each once against a mutant of that rule.

1. **Fixtures** (`sce-build/tests/fixtures/static_datamodel/`): `union_shape.scxml`, a union over
   `enum_shape_kind.scxml` with an empty case (`dot`), two cases holding one record (`circle` and `disc`, both
   `shape_radius.scxml`: `radius` `uint16`), a case of two fields (`rect`: `width`, `height`), a case holding a text
   and an optional (`label`: `text` `string[8]`, `note` `optional<string[4]>`) and a case holding a list (`path`:
   `points` `list<uint8>[3]`); `event_shapes.scxml`, `shapes.sent`, whose fields are `one` `union:Shape`, `many`
   `list<union:Shape>[1..3]` and `maybe` `optional<union:Shape>`; `static_union.scxml`, a statechart that takes
   `shapes.sent`, narrows each field by `<if>` chains and in a `<foreach>`, reads a field shared by two cases, builds a
   union variable, writes a case's field, sends `shapes.sent` back through `#_internal` by name, and lifts a text into
   `optional<union:Shape>` beside a refusal variable.
2. **The scenario** `sce-build/tests/fixtures/static_datamodel/scenarios/static_union.json`: every case, the empty
   case, the tag last in the text, the keys in another order, a key of another case (stepped over), a `null` under a
   key of another case (the flat spelling); each refusal of section 4.1 at the top and inside `many` (no tag, a `null`
   tag, a tag that is a number, an unknown tag, a field of the case missing, `null` in a field that is not optional, a
   text past its bound inside `label`); the lift's refusal record for each; the round trip, written in canonical key
   order. It replays on Rust (`backends/rust/tests/tests/static_scenarios.rs`,
   `backends/rust/scenarios/src/machines.rs`), Kotlin (`StaticScenarioTest.kt`), Python (`test_static_scenarios.py`), Go
   (`static_scenarios_test.go`), C11 (`test_static_scalars.c`), Swift (`StaticDatamodelTests`, `Replay.swift`) and the
   Interpreter (`AStaticDatamodelRunsLoweredUnderTheInterpreterTest.cpp`); C++ derives its suite from the directory.
   Eight lanes.
3. **Each runtime's reader and writer**: the shared table `tests/json_text/payload_refusal.json` gains the union
   descriptor and a case per refusal of section 4.1, at depths 1 and 2, so every reader is held to one refusal per
   text; a maximal value written at exactly `J` bytes.
4. **`sce-build`**: goldens of `S` and `G` for `union_shape` and for `date_expression`; every emitted schema validated
   against the 2020-12 meta-schema and `schemas/sce-payload-schema.v1.schema.json`; the Earley recognizer of
   `payload_schema.rs` accepts each case's smallest and largest value and refuses a value with a field of another
   case, with a field of its case missing and with the tag after a field (which the lift accepts: asserted as the
   lift's looseness, ADR 0087 section 1.1 item 2); the size calculus per case and tight against a maximal value;
   `fits`; the revision rule of section 8 (a case added with its variant revises, a case moved to another record does
   not); `a_union_field_is_read_only_where_its_case_is_proven.rs`, one test per narrowing construct and per
   invalidation of section 6.1 item 3, beside `a_value_is_proven_present_only_where_tested.rs`; the unreachable point
   of item 4 (no `algorithm/return-missing` after an exhaustive chain, the code raised once a case is added); the
   goldens of the three codes.
5. **The CBOR encoder**: the vectors of section 5.3, and `C` of a maximal value equal to its encoding's length.
6. **The standard library**: the calendar table of `tests/forge/conformance/gen_cases.py` writes each case as a union
   value, `numerical_reference.json` is written again from it (`gen_cases.py`, then `--check`), and the numerical
   conformance harness (`sce-build/src/conformance.rs`, the `resolve_date` entry of
   `tests/forge/conformance/fixtures.json`) hands the union. The table's cases of an absent number a kind reads have no
   spelling any more and leave the table, which says how many and why; the mutation casefile
   `sce-build/tests/mutations/a_date_expression_resolves_each_kind_as_its_table_says.cases` is re-aimed at the new
   body (`scripts/mutate --check`).
7. **A census of the type grammar**: `every_constructor_of_the_type_grammar_is_held_on_every_lane` (new, `sce-build`)
   lists the constructors of `TypeOf` through an exhaustive match and requires each to appear in the refusal table's
   descriptors, in the size goldens and in the schemas of a scenario every lane replays, so a constructor added
   without its cases fails there and not in a consumer.
8. **Mutation casefiles** under `sce-build/tests/mutations`: the tag read after the record (the order of section 4.1),
   the case's record chosen by another case's tag, the `const` left out of `S`, an alternative of `G` that does not
   begin with its tag, the false branch of a test keeping its case, `J` of the first case rather than the largest.

## Landing order

1. **The type and its document**: `TypeOf::Union`, the `union` kind, `<sce:case>`, the import kind,
   `TypeEnv::union_type`, the refusals of section 1.1 and the two `validation` codes with their appendix rows, the XSD
   and the AST export, the revision rule (`schema_revision`), the size calculus, depth, longest pointer and `fits`;
   every backend refuses a union by name. No committed document holds one, so no generated tree changes.
2. **The grammar**: the `S` and `G` rows, `oneOf` and `const` in `Keyword` and the payload meta-schema, the unit of a
   union document, `sce-codegen payload-schema` for one, the goldens and the recognizer's cases.
3. **The runtimes**: the reader's union combinator with its refusal order and the writer, in the eight runtimes, with
   the refusal table's cases.
4. **The generator for event fields and statechart variables**: the types of section 7, the narrowing of section 6
   in `forge::static_datamodel` with `expression/union-read-unproven`, construction, writes, sends, `<foreach>`, the
   lift and `sce:answer` over a union, the saved state and the replay vocabulary, then the fixtures and the scenario
   on eight lanes. A composed field needs ADR 0086 step 3's composed lowering on the lane. The committed generated
   trees are regenerated where the fixtures enter them (Rust, Go, Kotlin, Python and Swift `static_datamodel`; the Go
   round-trip tree where its source set takes the fixtures).
5. **Algorithms**: a union parameter, return and local on every backend, the narrowing in a body and the unreachable
   point of section 6.1 item 4; after ADR 0086 step 6 (algorithms' slots over every composition) on each lane.
6. **The standard library**: the union `date_expression`, its five new case records, `resolve_date` over the union,
   the calendar table, the oracle and the conformance harness, the mutation casefile, the consumer's migration note;
   every committed tree that holds `date_expression` or `resolve_date` regenerated (the Swift `static_datamodel`
   tree's `DateExpression.swift` and `ResolveDate.swift` among them).
7. **The deterministic CBOR encoder**: the union mapping and its vectors in `sce-rust-trust`.
8. **The census** of section 12 item 7, last, once every lane holds the constructor.

## Decided while landing step 5 (2026-11-11)

Step 5 lands its shared part in `sce-build` and its lowering on Rust. Each question below is one sections 6, 7 and
the landing order leave open, decided by a rule this record or ADR 0086 already states; none changes a document, a
wire or a diagnostic code.

1. **Which lanes lower it.** The judge of a body that holds a union (the narrowing of section 6.1, a union local's
   build, a union taken whole, the point of item 4) runs once, before any backend, so every backend refuses the same
   documents. Rust lowers the algorithm; every other backend, and the Interpreter's lowering, refuses it by name
   (`generate/unsupported-feature`) where it renders it, by the last paragraph of section 7. The list is
   `ALGORITHM_UNION_BACKENDS` in `sce-build/src/forge/codegen_matrix.rs`; a backend joins it only once it is in
   `UNION_BACKENDS`, since an algorithm's union is the type the union's own unit declares.
2. **A union is held whole, and nowhere inside another type.** An algorithm's parameter, return and local may be
   `union:U` (section 1.1 item 3). `optional<union:U>` and `list<union:U>` in an algorithm are compositions no
   algorithm's slot holds yet, refused by name as `optional<list<T>>` is (ADR 0086 section 7.3); they come with ADR
   0086 step 6 on each lane. Every case's record holds entries only, as a record an algorithm holds does (ADR 0086
   section 7.3); a union one of whose cases holds a list, a record or a union is refused by name.
3. **A union local is built, and not written.** Section 6.1 item 5 builds it from `<sce:set>` children, the tag a
   variant literal (`expression/unsupported-construct` otherwise, as that item says for an algorithm). The algorithm
   imports the record of the case it builds, as a statechart does (step 4), since a backend names that record's type
   where it builds the case; the import is kept for the renderer though no expression names it. The local takes no
   `init` (no value an algorithm holds is handed to it whole yet) and is not written afterwards: neither a field
   (item 6) nor a whole assignment, each refused by name, until a lane spells a write of the unit's type.
4. **Where a union stands whole.** Only in a `<sce:return>` of its own union, by the name of a parameter or a local,
   as a record is returned (SCE_FORGE.md section 4.12). It is no operand, no argument and no value of another place.
   An algorithm whose slot holds a union is called by a host alone for now: another algorithm's call of it is
   refused by the callee's `host_only` reason, and a statechart's call by name in `forge::static_imports`, which is
   how a union crosses between a statechart and an algorithm (section 6.1 item 7) once a lane spells the crossing.
5. **The point no path reaches** (section 6.1 item 4) is judged as a path through the body
   (`forge::algorithm_union::reaches_end`) over the one sequencing rule of an algorithm's body
   (`forge::narrowing::after_algorithm_statement`): a path ends at a `<sce:return>`, at an `<sce:if>` whose two
   blocks each end every path, and at a point where a subject of a union has no case left. A body that holds no
   union still ends in a `<sce:return>`, judged where it is read; one that holds a union is judged where its
   imports resolve, and `algorithm/return-missing` is placed at the condition of the body's last test. Its message
   now says the fact for both ("a path reaches the end of the body without `<sce:return>`"). The judgement is
   conservative: an `<sce:if>` with no `<sce:else>` whose block ends at such a point, but not in a return, proves
   nothing after it, so a body that relies on that is refused where a finer analysis would admit it.
6. **The end a backend's compiler cannot see is unreached.** The body is closed there with the target's statement
   that says so, Rust's `unreachable!`, written only where the body's last statement does not return on every path
   the compiler itself sees.
7. **Rust reads an algorithm's union on its unit's type** (section 7, the Rust row), not on a machine's own sum: the
   tag by its method, a field of the one case left through `proven_<case>()` and the payload's snake-case field, and
   a field several cases leave by a `match` over them whose other arm the narrowing proved is never taken, so the
   unit gains no accessor. A field is copied out as it is read, so a text or a byte string of a case is not read in
   an algorithm yet and is refused by name. The unit's spelling is chosen through `TypeCtx::unit_unions`, the type
   each union of an algorithm is spelled as, and one function per lane in `forge::composed_lowering`
   (`rust_unit_union_read`), the seam a lane fills to join: Kotlin with the unit's `provenAbsolute()` reads, its
   build of a case and its unreached end, beside the classification of a member read the Kotlin lane of step 4
   keeps in `composed_lowering` for the machine.

Step 6 makes `resolve_date` take a union, and the numerical conformance harness and
`every_standard_algorithm_generates_on_every_algorithm_backend` hold every standard algorithm on every backend that
generates algorithms, so step 6 needs step 5 on Rust, Kotlin, C++, Go, Python, C11 and Swift, and the harness's
argument of a union on each. The one committed tree that holds `date_expression` and `resolve_date` is Swift's
`static_datamodel` (`DateExpression.swift`, `ResolveDate.swift`, and the harness `NumericalConformanceTests.swift`).
A statechart that hands `resolve_date` a lifted `date_expression` needs, besides, the crossing of item 4 on its
lane.

## Decided while landing step 5 on Kotlin, with the crossing (2026-11-11)

Item 4 above leaves open when a statechart's call crosses a union ("once a lane spells the crossing") and no step
of the landing names it. Each question is decided by a rule this record, ADR 0086 or SCE_FORGE.md section 4.12
already states; none changes a document, a wire or a diagnostic code.

1. **The crossing is part of step 5, lane by lane.** Section 6.1 item 7 is a rule of the language: a union crosses
   as a composed value does, and a record crosses a statechart's call by the statechart's own import of the same
   document, copied into the algorithm's type and back (SCE_FORGE.md section 4.12). An algorithm's union parameter
   that no statechart can fill is half of step 5's "a union parameter, return and local", and the last paragraph of
   section 7 makes a missing position a step of the landing, not a rule. So a lane lowers the crossing in the change
   that lowers its algorithms over a union, and `ALGORITHM_UNION_BACKENDS` lists a lane only with both: the list is
   the one gate a statechart's call is held to as well, refused by name elsewhere (`generate/unsupported-feature`,
   naming the call and the union) where the statechart is rendered.
2. **A union crosses case by case, by a total case analysis.** The machine holds its own sum and the algorithm the
   type the union's unit declares (section 7; "Amendment (2026-11-11)" item 1 keeps the two apart). The call's
   adapter maps one to the other by a `match` (Rust) or a `when` (Kotlin) over every case, each case's record copied
   field by field exactly as a record argument is, an enum field by the declared name of its variant; the result is
   mapped back the same way. Every case is an arm and none is a default, so a case either side gains is a build
   error, the property the enum crossing already has (docs/SCE_ACCEPTED_SUBSET.md section 2.15, "A value of an
   enum"). Holding one type on both sides was rejected: it would undo item 1 of the amendment of 2026-11-11 and make
   the machine's saved form and sends the unit's. Crossing through the JSON text, written by one side and read by
   the other, was rejected: it adds a refusal where the type already proves the value fits, and a parse per call.
3. **What a slot takes and gives back.** A union slot is the statechart's own import of the same union document, by
   identity (as a record slot is), and each case's record the statechart's own import of that schema, which it needs
   already to hold the union (step 4); a case whose record a call does not copy (a byte string, an unbounded text)
   refuses the import, naming the case. An argument is the name of a variable of the union; one that may be absent
   is refused, naming the repair (put its present value into a union variable where a test proves it present, which
   section 6.1 item 5 admits). A loop's item and a payload's field of a union are refused by the rule a computed
   argument is, until a lane spells them. What the algorithm returns is taken whole into a variable of the same
   union by an `<assign>`, as a record is; into an `optional<union:U>` it is refused, naming the type. A call whose
   only state is the union it returns goes through the adapter too, which builds the machine's value.
4. **Kotlin lowers an algorithm over a union on its unit's sealed interface** (section 7, the Kotlin row): the tag
   by `kind`, a field of the one case left by `provenAbsolute().f`, a field several cases leave by a `when` over
   them that tests the parameter or local itself (both are never written, so each arm is smart-cast) and whose
   `else` throws `IllegalStateException`, written only where a case is not left. A local is a `val` of the
   interface's type, so a test of its case reads it as the union; the end no path reaches is a `throw` of
   `IllegalStateException`, the exception the unit's proven reads throw, inside the `try` of a `may-fail` body,
   whose `catch` takes only `AlgorithmFailure`. The classification of a read of a unit's union
   (`composed_lowering::unit_union_member_of`) is one for Rust and Kotlin, beside the machine's
   (`union_member_of`): the unit's admits a field the cases left declare with one type whatever the others declare,
   since the read is a case analysis over the cases left and needs no accessor on the type.

Step 6 still needs, lane by lane, step 5 with its crossing on C++, Go, Python, C11 and Swift (each first holding a
union in a statechart, step 4, except Swift), the Interpreter's lowering of an algorithm over a union and of the
call, and on every lane the numerical conformance harness's argument of a union; then the standard library's
change itself.

## Amendment (2026-11-07): a union variable has no default value

Section 6.1 item 5 says how a union is built and is silent on a variable that builds none. A record has a value
every field of which is its type's zero, but a sum type has no such value: no case is the zero of the others, and
choosing one (the first in the enum's order, say) would make a variable hold a shape its author never wrote, and
would move with the enum's order. So:

1. A statechart's variable of `union:U` is built at its declaration, as section 6.1 item 5 says: its `<data>`
   takes `<sce:set>` children, one naming the tag with a variant literal of the tag's enum and one naming each
   field of that case's record, exactly once, and no `expr`.
2. A variable that has no case to start at is declared `optional<union:U>`. It starts absent (`expr="null"`, and
   no `<sce:set>`), is filled by a whole assignment, and is read through the presence narrowing of ADR 0055 and
   then the tag narrowing of section 6.1.
3. A `union:U` variable with no `<sce:set>` is refused (`scxml/static-datamodel-rule`, on its `sce:type`, naming
   both repairs), and so is one with an `expr`. No diagnostic code is added.

## Decided while landing step 4 on Go (2026-11-11)

Step 4 on Go lands the statechart's part: a payload that holds a union, a union variable, its narrowing, its
writes, a lift into one and its saved state. The union's own unit (`codegen_matrix::UNION_BACKENDS`) has not
landed on Go, so a union document and a schema unit's union field are still refused there by name, and so,
by "Decided while landing step 5" item 1, is an algorithm over a union and a statechart's call that crosses
one. Each question below is decided by a rule this record already states; none changes a document, a wire or
a diagnostic code.

1. **A machine's union is a value struct of the machine** (section 7 item 4, open question 7), as the
   machine's records are its own ("Amendment (2026-11-11)" item 1): `<Machine><Document>Union`, an unexported
   `tag` of the machine's enum and one unexported member `case<Case>` per case that holds a record, built only
   by `<Union><Case>(r R)` or `<Union><Case>()`; `Kind()` answers the tag whatever key it is written under, as
   Kotlin's `kind` does, and `Proven<Case>()` and `ProvenField<Field>()` the reads the narrowing proves, whose
   other cases panic, the one place they are written.
2. **A field of a case is written in place.** A Go struct is written through a pointer to it, as a Rust and
   a Swift union are, so the write of section 6.1 item 6 goes through the pointer an unexported
   `proven<Case>Place()` or `provenField<Field>Place()` answers (`ComposedSpelling::union_member_place`). The
   accessor is unexported, so a host that holds a union builds a new one by a constructor rather than writing
   into a case, which keeps item 3 of section 7.
3. **The writer tells a case by its tag's declared name.** The walk that writes a value names no enum
   (`composed_lowering::writer`), and the declared name of the tag's variant is what the machine's enum
   answers (`String()`) and what the tag is written as (section 2.1), so the JSON writer switches on it.
4. **A union is copied by value**, as a record is (`ComposedSpelling::copy_union`, which every other backend
   spells as its copy of a value not copied as it is read): the machine changes a case by a whole assignment
   and writes a field through its place, never into a list a case holds, so the copy shares nothing the
   machine writes into.
5. **An index read of a list of unions is read**, as on Kotlin ("Amendment (2026-11-11)" item 3): Go's
   checked read fails the statement for an index outside the list, and the zero it answers beside the failure
   is the struct's, which no statement reads.
6. **The saved form is carried by the type**, `ToSaved` and `<Type>FromSaved`, the names a Go record of
   entries and a Go enum have, the tag beside the case's fields by the runtime's `SavedWriteUnion` and read
   first by `SavedReadUnionTag`; a case's record that holds a composition carries its saved form too, a
   list's lower count held by `SavedReadListWithin` (docs/adr/0086, section 15.6).

## Decided while landing the union's unit, step 5 and the crossing on Go (2026-11-11)

Go now generates a union document (`codegen_matrix::UNION_BACKENDS`), lowers an algorithm over a union and
hands one across a statechart's call (`ALGORITHM_UNION_BACKENDS`), which lifts the refusals the section above
names. Each question below is decided by a rule this record already states; none changes a document, a wire or
a diagnostic code.

1. **The unit's union has the machine's shape** (section 7 item 4, and item 1 of the section above): a value
   struct `<Union>` of the package of the union's document, an unexported `tag` of the tag's enum and one
   unexported member `case<Case>` per case that holds a record, built only by `<Union><Case>(r R)` or
   `<Union><Case>()`, `Kind()` and `Proven<Case>()`, each named by the one spelling the machine's union is
   named by (`composed_lowering::go`), so a reader of one reads the other. The two stay two types, each over
   its own records ("Amendment (2026-11-11)" item 1). The unit declares no `ProvenField<Field>()` and no
   place of a field: an algorithm never writes a union ("Decided while landing step 5" item 3) and reads a
   shared field by a case analysis (item 3 below), as Rust's and Kotlin's units declare no accessor for it.
   The unit's constants are an event schema unit's Go names (`Revision`, `DataMaxLen`, `SchemaID`,
   `JSONSchema`); a constructor spelled as one of them, or two variants of one Pascal spelling, is refused by
   name (`generate/unsupported-feature`), as Kotlin refuses two cases of one class.
2. **A tag is told apart by its declared name where no package of its enum is imported.** A test template's
   canonical form, an algorithm's shared read and a call's adapter switch over the declared name the tag
   answers (`Kind().DeclaredName().Proven()` on a unit, `Kind().String()` on a machine), the name the tag is
   written as (section 2.1) and the one a Go machine's writer already switches on (item 3 of the section
   above), so none of them imports the enum's package only to name its variants. A test template's matcher,
   which imports what its text names, compares `Kind()` with the variant itself.
3. **A field several cases leave is read by a function literal of the read's type** (section 6.1 item 2, and
   "Decided while landing step 5 on Kotlin" item 4): Go has no expression that takes a case apart, so the read
   is a function literal called where it is written, whose `switch` returns the field from each case left and
   whose end panics, the one statement no path reaches. Go spells the result of a function literal, which is
   the type the expression names the read's value by, as Go's conditional names its own; a field of a type the
   expression does not name (a value of an enum) is refused by name there, naming the repair (test one case),
   as a Go conditional over such values is refused where its arms are not plain reads. A field read under one
   case left needs no literal.
4. **The end no path reaches is a `panic`** (section 6.1 item 4, "Decided while landing step 5" item 6), the
   terminating statement a Go function with a result may end in and what the unit's proven reads do in a case
   the narrowing proved the value is not in. A `may-fail` body hands a failure on as an `error`, never as a
   panic, so the two are not confused. A union local is a short declaration of the constructor's value, whose
   type is the union's.
5. **The crossing is a total `switch` that the compiler does not hold to every case** ("Decided while landing
   step 5 on Kotlin" item 2). Go checks no `switch` for exhaustiveness, so the property that a case either side
   gains is a build error is the adapter's regeneration with the document, which writes one arm per case of the
   union as the statechart imports it; the arm of a value in no case, which no constructor builds, panics rather
   than cross as the zero struct. The adapter declares the arriving value and assigns it in each arm, since a Go
   `switch` is a statement; what it hands over names the unit's package and each case's record's, what it takes
   back is read through the unit's methods and names neither, as a returned record is
   (`StaticTarget::adapter_names_returned_schema`).

Step 6 still needs, lane by lane, step 5 with its crossing on C++, Python, C11 and Swift (each but Swift first
holding a union in a statechart, step 4, and C++, Python and C11 the union's unit), the Interpreter's lowering of
an algorithm over a union and of the call, and on every lane the numerical conformance harness's argument of a
union; then the standard library's change itself.

## Decided while landing ADR 0092 on C++ (2026-11-11)

C++ now holds a union in a statechart (step 4), generates a union document
(`codegen_matrix::UNION_BACKENDS`), lowers an algorithm over a union and hands one across a statechart's call
(`ALGORITHM_UNION_BACKENDS`). Each question below is decided by a rule this record already states; none changes
a document, a wire or a diagnostic code.

1. **A machine's union is a struct of one `std::variant`** (section 7, the C++ row, and its item 2):
   `<Machine><Document>Union`, nesting one alternative struct per case, named as the tag's enum names the
   variant (`Circle`), holding the case's record as `value`, or empty; the variant is the member `cases`, its
   alternatives in the tag enum's order. `kind()` answers the tag from the alternative's index, so the tag and
   the case cannot disagree. `proven<Case>()` and `provenField<Field>()` are the reads the narrowing proves, and
   `proven<Case>Place()` and `provenField<Field>Place()` the references a write of a field goes through (section
   6.1 item 6), each ending the process with `std::abort()` in a case the narrowing proved the value is not in,
   the one place that is written. A value is built only whole, `U{U::Circle{r}}`. The variant's first
   alternative is the value of `U{}`, which is the place a reader fills before it answers, as Go's zero struct
   is; no statement reads it.
2. **The writer tells a case by its alternative**, `std::get_if` and `std::holds_alternative` on the union's
   type, which the walk hands it, so the writer names no enum; the tag is written as the variant's declared name
   among the case's fields by the runtime's `EventPayloadFields::unionCase`, and as a `ScriptObject` member for a
   `<send>`.
3. **A lift reads a union whole** through the runtime's `EventPayloadFields::liftValue`, beside `liftRecord`,
   which hands the composed reader the text's object at its root, so a refusal inside it points below the root
   as a field of a lifted record does (section 6.1 item 5).
4. **The saved form is two functions beside the type**, `<Type>ToSaved` and `<Type>FromSaved`, the names a C++
   record of entries and enum already have: the tag beside the case's fields by the runtime's `savedWriteUnion`,
   read first by `savedReadUnionTag`; a case's record that holds a composition carries its saved form too, a
   list's lower count held by `savedReadOwnedListWithin` (docs/adr/0086, section 15.6). An index read of a list
   of unions is read, as on Kotlin and Go: the checked read records an index outside the list as the failure
   the statement ends at, and the value it answers beside the failure is never read.
5. **The unit's union has the machine's shape** in the unit's namespace (`SCE::Generated::<Pascal>::<Pascal>`),
   named by the same spellings (`composed_lowering::cpp`), with `kind()` and `proven<Case>()` and no place of a
   field, since an algorithm never writes a union ("Decided while landing step 5" item 3). Two variants of one
   alternative's name, and a case named as the union's own struct, which C++ refuses as a member, are refused
   by name. A schema unit's union field names the unit's struct.
6. **The test profile is a step of its own.** C++ generates a union's unit, and its test-template walk spells
   no case of a union and its runtime no canonical form of one yet, so a template, a component and a
   configuration that reach a union stay refused by the union's name. The backends whose test profile matches
   a union are a list of their own, `codegen_matrix::TEST_PROFILE_UNION_BACKENDS`, each in
   `UNION_BACKENDS`, by the last paragraph of section 7.
7. **An algorithm reads its union on the unit's struct**: the tag by `kind()`, a field of the one case left by
   `proven<Case>().f`, and a field several cases leave by a lambda called where it is written, which takes each
   case left apart by `std::get_if` on its alternative and whose result is deduced from its returns, of one type
   by `unit_union_member_of`. A union local is `const`, never written. The end no path reaches is
   `std::abort();`, `[[noreturn]]`, so the compiler asks the body for nothing after it; a `may-fail` body hands
   a failure on as its `AlgorithmResult`, never by ending the process.
8. **The crossing is total by the compiler** ("Decided while landing step 5 on Kotlin" item 2): the adapter
   maps the leaving value by `std::visit` with a struct declared where it is visited, one call operator per
   alternative, each answering the case the value arrives as, its record copied as a record argument is.
   `std::visit` asks the struct for every alternative of the leaving variant, so a case either side gains does
   not build, as a Rust `match` and a Kotlin `when` do not.

Step 6 still needs, lane by lane, step 5 with its crossing on Python, C11 and Swift (each but Swift first
holding a union in a statechart, step 4, and Python and C11 the union's unit), the Interpreter's lowering of an
algorithm over a union and of the call, and on every lane the numerical conformance harness's argument of a
union; then the standard library's change itself. C++'s test profile matching a union
(`TEST_PROFILE_UNION_BACKENDS`) is not on step 6's path.

## Amendment (2026-11-11): the Kotlin lane where this record is silent

Step 4 on Kotlin met four questions sections 6 and 7 leave open. Each is decided by a rule this record already
states, and none changes a document, a wire or a diagnostic:

1. **A machine's case is named as the machine's enum names the variant.** Section 7 item 1 says a case's
   identifier is "its variant's identifier as each backend already spells the enum's variant", so that a union is
   spelled with no name the enum's spelling has not already proven safe. A Kotlin machine's enum class names a
   variant as an upper snake constant (`ABSOLUTE`), so the machine's sealed interface nests `data class
   ABSOLUTE(val value: R)` and `data object NONE`; the Kotlin row's `Absolute` is the unit's spelling
   (`forge/kotlin/union.kt.jinja2`), which predates the machine's and is the type a host imports. The two are
   different types already, each over its own records, so naming them apart costs nothing a host reads, and the
   machine's names stay clear of the `Companion` its saved form extends.
2. **A field of a case is written by assigning the holder the next value whole.** Section 6.1 item 6 says a field
   is written where its case is proven, and section 7 states no write. A Kotlin record is an immutable data class,
   whose field a machine writes as `r = r.copy(f = v)`, so a union, which holds such records, is immutable too: the
   write `shape.radius = v` under one case left is `shape = U.CIRCLE(shape.provenCircle().copy(radius = v))`, and
   under several cases that share the field `shape = shape.provenFieldRadiusWith(v)`, a function of the interface
   beside the shared read, written once. A Rust and a Swift union are written in place, through a place of the
   field; the one spelling decides which (`ComposedSpelling::union_member_write`).
3. **An index read of a list of unions is read.** Swift refuses it because its checked read answers an index
   outside the list with the element type's zero, which a union does not have (the amendment of 2026-11-07). A
   Kotlin machine's checked read fails the statement instead and answers nothing, as Rust's does, so the read
   needs no zero and is lowered.
4. **The saved form is carried by the type**, as a Kotlin record's is (ADR 0087 item 8): `toSaved` and a
   `fromSaved` on the interface's companion, extensions written beside the type, the tag beside the case's fields
   by the runtime's `SavedValues.union` and read first by `SavedValues.unionTag` (section 6.1 item 7).

## Consequences

- The type grammar has a sum beside its product, so a value whose fields depend on a choice states the dependence in
  its type, and every grammar, JSON Schema, generated type and reader the build writes states it too.
- A grammar-constrained decoder chooses a case by its tag and is then held to that case's fields; the date
  expression's 352 admitted shapes become eleven, and its largest text falls from 105 to 60 bytes.
- `resolve_date` loses ten preconditions and its trailing kind check, and a kind a later revision adds is refused at
  build where it is not resolved.
- A consumer's code that reads the flat `DateExpression` migrates once; its stored texts that resolved read
  unchanged.
- Every runtime's reader gains one combinator and every backend one generated sum; no refusal code is added.
- Go pays storage for every case of a union in each value (section 7 item 4).
- ADR 0087 section 2.6's reason for keeping the lift's result in two variables, "the type grammar has no sum type",
  no longer holds; its other reasons do, and the lift is not changed here.

## Alternatives rejected

- **Keep the flat record and leave the optional numbers out of `required`.** The schema would still admit every
  combination of numbers with every kind, and a missing key would mean two things, a sender's older revision (ADR
  0044, ADR 0055 section 4) and a number the kind does not read.
- **A lift that reads an absent optional key as `null`.** The JSON Schema the model is given would say `required`
  and the reader would not, a reader accepting what its own schema refuses; and it would change the meaning a missing
  key has on every wire.
- **A host-side table of each kind's numbers.** A second statement of the rule, outside the type, in each consumer.
- **`if`/`then`/`else` in the schema**, **OpenAPI's `discriminator`**, **a one-name `enum` for the tag**: section 3.2.
- **The tag around the fields or as one of two keys**: section 2.2.
- **Labels declared by the union**, **cases declared inline**, **an untagged sum**: section 1.3.
- **One event per kind.** A model's answer is one text of one schema; an event per kind would move the choice out of
  the type the answer is held to.
- **A precondition that names the kind's numbers, kept in the resolver.** It is today's design; it refuses only after
  the lift, with no pointer, and no grammar can state it.

## Open questions

Decided (2026-11-06): the owner took each recommendation below; the questions stay as the record of what was chosen
and why.

1. **Is the tag's key declared by each union, or one key for every union of the language** (`kind`, `type` or
   `tag`)? **Recommendation: declared by each union**, as the `id` of its tag `<data>`. The date expression keeps
   `kind`, so the flat record's texts read unchanged (section 2.1 item 3); a fixed key would reserve one name in every
   record that can stand in a case, and an application's answer schemas already have a word of their own for the
   choice.
2. **`oneOf` or `anyOf` for the cases?** They accept the same values (section 3.1 item 2). **Recommendation:
   `oneOf`**, which states the type; `anyOf` is the fallback if a provider's converter that consumers use is found to
   refuse `oneOf`, a change of the fold and of no document.
3. **May a case be empty, its tag alone?** **Recommendation: yes** (`none`). A union whose every case is empty stays
   refused, being its enum. Keeping `none` as a case, and not making the answer `optional<union:DateExpression>`,
   keeps "the text names no date" a choice the model writes by name, as today, and `DateKind` unchanged.
4. **May unions nest: in a list, an optional, a record, and a case's record?** **Recommendation: yes, everywhere**, by
   ADR 0086's answer A; only a union that holds itself is refused.
5. **How does a union evolve?** **Recommendation: as section 8 says.** Its revision is its tag enum's; adding a case
   with its variant is a revision; an older reader refuses the new case's tag as `unknownVariant`; moving a case to
   another record, removing one or changing the tag's key is no revision.
6. **Is the flat record kept as a deprecated alias?** **Recommendation: no** (section 10.4).
7. **Does Go hold a union as a value struct or as a sealed interface?** **Recommendation: a value struct** (section 7
   item 4), the reason the owner gave for Go's optional; the cost is the sum of the cases' storage in each value.
8. **May a field be read under a set of several cases that each declare it** (`count` after `kind === relativeDays ||
   kind === relativeWeeks`)? **Recommendation: yes, when every case of the set declares it with one type** (section
   6.1 item 2), the rule TypeScript applies to a property common to the remaining cases.
9. **May a case name a build feature, as a codec's arm may (ADR 0072)?** **Recommendation: no.** A union is a value
   type of the language, whose set of values is one in every build; a codec's arm describes which frames a target
   receives, which is the deployment's.
