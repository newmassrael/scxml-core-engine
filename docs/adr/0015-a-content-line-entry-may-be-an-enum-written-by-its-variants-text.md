# ADR 0015 — A content-line entry may be an enum, written and read by its variant's text

- Status: Accepted
- Date: 2026-10-10
- Scope: the `codec` kind's `sce:encoding="content-line"` (`SCE_FORGE.md` §4.6.4) and the `enum`
  kind's `<sce:variant>` (`SCE_FORGE.md` §4.3), `sce-build` (`forge/parser.rs`, `forge/model.rs`,
  `forge/content_line_codec.rs`, `forge/enum_naming.rs`), the runtime and the generator of each of
  the six backends, `apis/forge-ast.v1.schema.json`, the conformance corpus
- Related: `docs/adr/0010-a-codec-reads-and-writes-one-content-line-component.md` (this closes its
  first *Not now* item), `docs/adr/0014-a-repeated-content-line-property-is-a-list-of-line-records.md`

## Context

ADR 0010 first admitted an `enum:<alias>` entry and then refused it by name, for a reason that is a
cost and not a limit: a codec generator reaches an enum import's carrier (`from_underlying`), not the
text of a variant, and a text-to-variant table is six new lookups in six enum spellings. It also said
the entry waits "for a consumer that reads `STATUS`, `PARTSTAT` and `FREQ` as text". That second
reason put the feature's shape in the hands of one consumer, which a general facility must not be:
RFC 5545 and every format that shares the content-line syntax (vCard among them) has properties and
parameters whose value is one of a closed vocabulary, and the facility is judged by whether it
states that capability once, not by who reads it first.

What is missing is small and is the same in every backend:

- A **closed vocabulary needs a text for each member**, and the text is not the variant's declared
  name. A declared name is the source each backend turns into an identifier (`invalidArgument`
  becomes `InvalidArgument`, `INVALID_ARGUMENT`, `invalid_argument`); it is chosen for six languages.
  The wire text of an iCalendar `PARTSTAT` is `NEEDS-ACTION` and of a `STATUS` is `IN-PROCESS`, and
  the parser does not even say whether a name may hold a hyphen. Using the declared name as the text
  would tie the format a document speaks to the spelling of an identifier, so renaming a variant for
  one language would change the wire.
- A text read off the wire must be **told from a variant** by the rule the format states. RFC 5545
  writes its vocabularies as quoted strings of its ABNF, which RFC 5234 §2.3 defines as case-insensitive,
  so `accepted` and `ACCEPTED` are one value.
- A closed enum has no member for a text it does not declare, and an enum is a **type**, so the field
  that holds one is that type (as a CBOR entry's already is) and not a `string`.

## Decisions

### 1. An enum entry is `sce:type="enum:<alias>"`, wherever a `string` entry may be

`<alias>` names an `<sce:import kind="enum">` of the codec, as it does for a CBOR entry. The entry may
be a single-valued property's value, a repeated property's value (a list of the enum, or the value of
each line record, ADR 0014), or a parameter. Its field is the enum's own type in every backend: a
required entry starts at the enum's first declared variant, as a CBOR enum field does, and an optional
one is absent in the language's own way.

It takes none of the string entry's own attributes. `sce:max-size`, `sce:value="text"`,
`sce:separator` and `sce:max-values` on an enum entry are refused by name: a variant's text is bounded
by the enum that declares it, it is no TEXT, and a list of enums on one line is a later addition (see
*Not now*).

### 2. A variant's text is `sce:text`, else its declared name

`<sce:variant name="needsAction" value="0" sce:text="NEEDS-ACTION"/>`. The text is optional; where it
is absent the declared name is the text. It is the ASCII letters, digits and `-` of an RFC 5545
`iana-token`, non-empty, and unique in its enum under ASCII case folding; a text that breaks either is
refused at the enum document, not at a codec that imports it. The text belongs to the **enum**, which
is the one place that says what a variant is called outside a program, so a second codec, or a
JSON form later, reads the same text instead of copying it.

Nothing else in the enum's generated type changes: no backend emits the text into the enum, so an
embedded target that never reads one carries no string table, and no committed enum tree moves.

### 3. Wire rules

*Decode.* The value of the line (or of the parameter, after its quotes are taken off) is compared with
the text of each variant, ASCII case-insensitively, and is that variant on the first match. A text no
variant declares is `line-bad-value`, and so is any value longer than the longest text: nothing is
kept of it, as a string would keep it. This holds for an open enum (`sce:strict-variants="false"`)
too: the codec's job is to name a variant, and a carrier no text names has no wire form. A format
that must pass an unknown token through reads it as a `string`.

*Encode.* The variant's text is written exactly as the enum declares it, case included. A value of an
open enum that no variant declares has no text, and is `line-bad-value` on encode.

*Everything else stands.* The value is judged after its parameters are read (ADR 0014), a required
enum entry that has no line is `line-required-missing`, folding and quoting are the runtime's. No new
failure name.

### 4. Where the table lives

The codec generator writes, for each enum entry, a table of the enum's variants in declaration order:
each text and the carrier value it stands for. Decode finds the row by text and takes the variant
through the enum's own `from_underlying`, the conversion a CBOR enum field already uses, so a closed
and an open enum are read by one rule in each backend. Encode takes the carrier by `to_underlying` and
the text from the row with that value. The matching, which is the part with a rule, is the runtime's
(`read_enum`, once per backend); the rows are data in the generated codec, built from the one
variant list the import already carries.

### 5. Order of work

The order ADR 0014 followed, for the same reason:

1. Declare: `sce:text` on a variant (parse, checks, AST schema, the enum page), the enum entry's parse
   and its refusals, `SCE_FORGE.md`, and a refusal by name of generating a codec that has one.
2. The conformance model and its vectors: a fixture with an enum as a property, as a parameter, as a
   repeated property and as the value of a line record, a variant whose text is not its name, a
   text in another case, a text no variant declares, and an open enum. The model is written from this
   page, so no backend's output is its own oracle.
3. Generate it for Python, Go, Kotlin, Rust, C++ and C11, one commit each, against the same vectors.

## Not now

- *A list of enums on one line* (`sce:separator` on an enum entry). It needs the list and the enum
  rules to meet, and no format asked for it before this one.
- *A case-sensitive match.* RFC 5545's vocabularies fold case (through RFC 5234); a format that
  defines a case-sensitive vocabulary needs the match to be a choice of the entry, and waits for one.
- *An open enum that keeps an unknown token.* It would need a place for the text beside the carrier,
  which is a record and not an enum; a `string` entry is that place today.
- *`BINARY` values and a parameter of several values* — their own decisions, on the same grounds.

## Rejected

- *A `from_text` / `to_text` on every enum type.* It puts the mapping where the enum is, which is
  right, and pays for it everywhere: every enum in every backend, the `no_std` and no-allocation ones
  included, would carry a string table whether or not a codec ever reads it, and every committed enum
  tree would change. The table is data the codec that reads a text asks for.
- *The declared name as the wire text, with no `sce:text`.* It makes the wire format depend on the
  spelling of an identifier that six backends each rewrite, and it does not reach a text such as
  `NEEDS-ACTION` that no one would pick as a name.
- *A `string` entry and an algorithm that maps it.* The `algorithm` kind has no string processing
  (ADR 0010, *Rejected*), and the mapping would be written again by every consumer.
- *A case-sensitive match, with the case written into each `sce:text`.* It makes every conforming
  document that writes `accepted` unreadable, to save one ASCII fold.
