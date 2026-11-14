# ADR 0104 — A content-line value is of the RFC 5545 value type its property has, and a parameter value is escaped by RFC 6868

- Status: Accepted (2026-11-14). The owner decided the five points of "Decision" (RFC 6868 for every parameter
  value; a value type for every property value, defaulting as RFC 5545 defaults it, escaped exactly when it is TEXT
  and written with `VALUE=` when it is not the default; codec format 3, with format 2 refused and rewritten; the
  diagnostics; every backend, document and lane) on 2026-11-14, under the standing rule: the most long-term correct
  structure, textbook in its foundations, cost ignored; decided, not asked. Two things those points require and do
  not settle were decided in writing this record, under the same rule, and are marked where they stand: an entry
  whose property admits several types (section 2.5), and what the rewrite writes for each kind of format-2 entry
  (section 3.2). "Landing order" numbers the steps; none has landed.
- Date: 2026-11-14
- Amends:
  - ADR 0010 decision 1, the row of `sce:value="text"` ("Without it the value is carried as written"), and decision
    3 ("Values are text"): a value is of a value type, which is TEXT unless the document or RFC 5545 says otherwise
    (section 2);
  - ADR 0014 decision 2 (a value cut at a separator): "a TEXT" is a value whose type is TEXT; a line record gains the
    line's type where its entry lists several (section 2.5);
  - ADR 0015 decision 1, "it is no TEXT": an enum entry's value is a TEXT, as RFC 5545 types every vocabulary
    property, and is read as one (section 2.3);
  - ADR 0055 section 8: the format a codec document declares is a format of its encoding; the current format of a
    content-line document is 3 and of a CBOR document stays 2 (section 3.1);
  - `SCE_FORGE.md` §4.6.4 (the attribute table, the wire rules, the failure table) and §4.6.1's paragraph "Codec
    format";
  - `SCE_ERROR_CONTRACT.md` §3.1, the payload of `rewrite_codec_format` (section 4).
- Scope: the content-line codec, everywhere it is stated or implemented:
  - `sce-build`: `forge/parser.rs` (`parse_content_line_entry`), `forge/model.rs` (`ContentLineEntry`), a new
    `forge/content_line_value_type.rs` (the table of section 2.2, written once), `forge/codec_format.rs` and
    `sce-codegen codec-format`, `forge/encode_plan.rs` (`ContentLinePlan`), `forge/content_line_codec.rs`,
    `forge/codec_size.rs`, `forge/codec_failure.rs`, `forge/diagnostic.rs` and `forge/error.rs`, `conformance.rs`;
  - the seven runtimes' content-line reader and writer (`backends/rust/forge-runtime/src/content_line.rs`,
    `backends/c/forge-runtime/include/sce/forge/content_line.h`, `backends/cpp/forge-runtime/include/sce/forge/
    content_line.h`, `backends/go/forge-runtime/codec/content_line.go`, `backends/kotlin/forge-runtime/.../
    ContentLine.kt`, `backends/python/forge-runtime/sce_forge_runtime/content_line.py`,
    `backends/swift/forge-runtime/Sources/SceForgeRuntime/ContentLine.swift`) and each runtime's codec error;
  - the seven templates `tools/codegen/templates/forge/{rust,c,cpp,go,kotlin,python,swift}/codec_content_line.*`
    and the Swift printer `forge/generator/swift_codec.rs`;
  - `schemas/sce-forge-ext.xsd` (`sce:value`), `apis/forge-ast.v1.schema.json` (`ContentLineEntry`, pre-release);
  - `SCE_FORGE.md` §4.6.4, `SCE_ERROR_CONTRACT.md`, `docs/SCE_ACCEPTED_SUBSET.md` (its codec-format paragraph and
    the appendix), `docs/SCE_CODEC_CONSUMER_MIGRATION.md`;
  - every content-line document of the tree (section "Migration" lists them), the independent model
    `tests/forge/conformance/content_line_model.py`, `fixtures.json` and `numerical_reference.json`;
  - the TTCN-3 lane's content-line binding, which is the downstream's (section "Conformance").
- Related: RFC 5545 (§3.1 content lines, §3.2 parameters, §3.2.7 `ENCODING`, §3.2.20 `VALUE`, §3.3 value types,
  §3.7 and §3.8 properties, §3.8.8.1 IANA properties, §3.8.8.2 non-standard properties); RFC 6868 (parameter value
  encoding in iCalendar and vCard, which updates RFC 5545 and RFC 6350); ADR 0081 (an encode is planned once);
  ADR 0091 and `docs/SCE_TTCN3_LANE.md` (the lane whose binding found this).

## Context

### What was found

On 2026-11-14 the TTCN-3 lane's first content-line binding, written over libical v4.0.7, was run against SCE's
oracle (`codec_content_line_event` in `tests/forge/conformance/numerical_reference.json`) and refused 14 of 98
decodes and 39 of 52 encodes. The downstream design records the three causes: libical's line reader strips white
space at the end of a line, and its writer doubles a backslash of an X- value and writes a `^` of a parameter value
as `^^`. The downstream then wrote its binding without a library, from RFC 5545 §3.1 and `SCE_FORGE.md` §4.6.4
alone, and that binding agrees with the oracle on every case.

Agreement with the oracle is not agreement with the RFCs. Two of the three differences are SCE's, not libical's.

### (a) A parameter value is not read by RFC 6868

RFC 6868 is the standards-track update of the parameter-value grammar of RFC 5545 §3.2 (and of vCard's). RFC 5545's
grammar gives a parameter value no way to hold a double quote (it would end a quoted value) or a line break, so RFC
6868 gives `^` an escape role inside a parameter value: `^n` is a line break, `^^` is `^`, and `^'` is `"`. A writer
writes those three characters that way; a reader decodes the three sequences and leaves a `^` followed by any other
character in place, with that character.

`SCE_FORGE.md` §4.6.4 reads a parameter value as written and says, of the writer: "A parameter value holds no `"` for
the same reason: it is refused (`line-bad-value`) rather than written between quotes it would end." So SCE cannot
carry a `"` or a line break in a parameter at all, and reads `CN=O^'Neil` as `O^'Neil` where every RFC 6868 reader
reads `O"Neil`. The other direction disagrees too: SCE writes the value `x^n` as `TZID=x^n`, which an RFC 6868 reader
reads as `x` and a line break, and the value `a^^b` as `a^^b`, read as `a^b`. libical 4.0.7 implements RFC 6868 on
both sides (`icalmemory_append_encoded_string` and `icalmemory_append_decoded_string`): its `^^` is the RFC's rule
and SCE's `^` is the departure.

### (b) A property value has no value type

RFC 5545 §3.3 defines fourteen value types (BINARY, BOOLEAN, CAL-ADDRESS, DATE, DATE-TIME, DURATION, FLOAT, INTEGER,
PERIOD, RECUR, TEXT, TIME, URI, UTC-OFFSET, §3.3.1 to §3.3.14). Every property's definition in §3.7 and §3.8 states
its value type and any other it admits, and §3.2.20 says how a line says which it carries: a property whose value is
of its default type need not write `VALUE`, and one whose default is overridden by another admitted type MUST write
`VALUE=<type>`. A property no section defines has a default too: an IANA-registered property this reader does not
know (§3.8.8.1) and an `X-` name (§3.8.8.2) are TEXT by default and may be set to any type. A line's value type is
therefore never unknown: it is its `VALUE`, or else its property's default.

The escapes of §3.3.11 (`\\`, `\;`, `\,`, `\n` or `\N`) belong to TEXT and to no other type.

SCE's codec has no value type. It has one flag: `sce:value="text"` makes a `string` an RFC 5545 TEXT, and "without it
the value is carried as written — a date-time, an `RRULE` and a URI are not TEXT" (`SCE_FORGE.md` §4.6.4). On decode
a `VALUE` parameter is a parameter like another, skipped when no entry declares it. Three things follow, each a
disagreement with every RFC 5545 reader:

1. **A property whose default is TEXT and that the document does not mark is read as no RFC reader reads it.** An
   `X-` property, an unregistered one, and a registered TEXT property such as `UID` or `RESOURCES` written without
   `sce:value="text"` are carried as written. `X-MEMO:a\,b` is `a,b` to RFC 5545 and `a\,b` to SCE; SCE writes the
   value `a;b` as `X-MEMO:a;b`, which is no TEXT at all (§3.3.11 admits `;`, `,` and `\` in a TEXT only escaped), and
   the value `a\b` as `X-MEMO:a\b`, which a reader refuses as a bad escape. This is the backslash libical doubled:
   libical was right.
2. **A line of another type is read as if it were of the declared one.** `DTSTART;VALUE=DATE:20261008` is read by a
   `string` DTSTART entry with its `VALUE` skipped, and the program cannot tell a date from a date-time except by
   the length of the text.
3. **A value of a type other than the default is written without saying so.** A `bool` entry on `X-ALL-DAY`
   writes `X-ALL-DAY:TRUE`, which RFC 5545 reads as the TEXT `TRUE`; a `string` DTSTART entry that holds a date
   writes `DTSTART:20261008`, which RFC 5545 reads as a malformed DATE-TIME.

The tree confirms each statement: `parse_content_line_entry` (`sce-build/src/forge/parser.rs`) admits `text` as the
only value of `sce:value` and refuses it anywhere but on a `string` property entry; the independent model reads a
parameter value with no escape (`read_text(values[0], False, ...)` in `content_line_model.py`) and refuses a `"` in
one on encode; and the oracle holds a vector whose note says the opposite of §3.2.20, "a parameter no entry declares
is skipped" over `DTSTART;VALUE=DATE-TIME;...`.

### (c) Trailing white space, where SCE is right

libical's line reader removes white space at the end of a line. RFC 5545 §3.1 admits it in a value (`VALUE-CHAR`
includes `WSP`) and states no rule that removes it, and SCE keeps it. This is libical's defect and nothing here
changes it.

### What the RFC texts in reach say, and what was checked

The tree quotes no RFC text: no file names RFC 6868, and RFC 5545 appears by section number only. What this record
states of the RFCs is from their sections as cited, cross-checked against the one implementation of both in reach,
libical 4.0.7 (`build/third_party-src/libical-v4.0.7` of the downstream checkout): `src/libical/icalmemory.c` for
RFC 6868, and `design-data/ical-properties.csv` for the default value type of every property. That table agrees with
section 2.2 below on every RFC 5545 property but one, `GEO`, which it gives as TEXT where RFC 5545 §3.8.1.6 says
"Value Type: FLOAT" (two FLOAT values separated by `;`); the RFC is the authority, and landing step 2 checks every row
against the RFC text before the table is closed.

## Decision

### 1. Every parameter value is read and written by RFC 6868

On every backend, for every parameter an entry declares (a `string` or an `enum:<alias>`, of a single-valued
property or of a line record):

- **Decode.** The parameter's value is taken by the grammar §4.6.4 already states (a quoted string with no `"` inside,
  or text up to `;`, `:`, `,` or `"`), its quotes are removed, its raw text is held to the existing rule (no control
  character but a tab, `line-bad-value`), and it is then decoded: `^n` is a line feed (U+000A), `^^` is `^` and `^'`
  is `"`; a `^` followed by any other character, or ending the value, is kept, with that character. The decoded
  value is held to UTF-8 and to `sce:max-size` (counted after decoding, as a TEXT is after unescaping), and an enum
  parameter is matched (ADR 0015) against the decoded text.
- **Encode.** `^` is written `^^`, `"` is written `^'` and a line feed is written `^n`; every other control
  character is still `line-bad-value`, and a carriage return among them. The value is then written between double
  quotes when it holds `:`, `;` or `,`, as today. A `"` and a line feed in a parameter are no longer refused.
- **Folding.** An RFC 6868 escape is one unit of the fold, as a TEXT escape is: the writer never cuts between `^`
  and the character after it.
- **Only `^n` is a line break.** RFC 6868 names the lower-case sequence; `^N` is kept as written, by the rule for any
  other character. A TEXT keeps both `\n` and `\N` (RFC 5545 §3.3.11).
- **A parameter nobody declares** is skipped as before, after the grammar has found its end, and is not decoded.

The decode is applied after quotes are removed and not before the grammar, so the sequences cannot end a value or
open a quote: `^'` is two characters the grammar already admits.

### 2. A property value is of an RFC 5545 value type

#### 2.1 The rule

A value entry (an entry with no `sce:param`) has a **value type**, one of the fourteen of RFC 5545 §3.3. `sce:value`
names it, in lower case: `binary`, `boolean`, `cal-address`, `date`, `date-time`, `duration`, `float`, `integer`,
`period`, `recur`, `text`, `time`, `uri`, `utc-offset`. Without `sce:value` the type is the property's **default**:

- for a property RFC 5545 §3.7 or §3.8 defines, the default its section states (the table of section 2.2);
- for any other name, an `X-` name (§3.8.8.2) or an `iana-token` the table does not hold (§3.8.8.1), TEXT.

The name is matched without regard to case, as decode matches it (`Uid` is `UID`).

A type the property does not admit is refused at the document: a property of the table admits its default and the
types its section lists as alternatives (column "Also admitted"), and any other property admits all fourteen. A type
the entry's `sce:type` cannot hold is refused too (section 2.3). Both are
`validation/content-line-value-type-not-admitted` (section 4).

**The type decides two things on the wire, and nothing else:**

1. **Escapes.** A TEXT is written and read with the escapes of §3.3.11, as `sce:value="text"` is today: on encode
   `\` as `\\`, `;` as `\;`, `,` as `\,` and a line feed as `\n`; on decode `\\`, `\;`, `\,`, `\n`, `\N`, and any
   other `\` is `line-bad-escape`. A value of any other type has no escape: a `\` is a byte like another.
2. **`VALUE`.** A value whose type is not its property's default is written with `VALUE=<TYPE>` (the type's name in
   upper case, as §3.2.20 spells it); a value of the default type is written without it. On decode the line's type
   is its `VALUE` if it has one, compared without regard to case, and its property's default if it has none; a line
   whose type is not the entry's is refused with the new failure `line-value-type` (section 2.4).

**What the type does not decide.** A value of a type other than TEXT is carried as its text, held only to the rules
every value is held to (no control character but a tab, valid UTF-8, `sce:max-size`): the codec does not check that
a DATE-TIME is a date-time or that a RECUR is a rule. ADR 0010 decision 3 stands on that point: a date and a rule are
read by the `algorithm` kind once they are out of the text, and a lexical check of each type is a decision of its
own, with a grammar per type and a failure per grammar, which this record does not take (section "Alternatives
rejected", last item).

#### 2.2 The table

The default and the admitted types of every property RFC 5545 defines. It is format 3's table: written once, in
`sce-build/src/forge/content_line_value_type.rs`, and stated in `SCE_FORGE.md` §4.6.4. No runtime carries it: the
generator gives each entry its type and its default as constants.

| Property | RFC 5545 | Default | Also admitted (written with `VALUE=`) |
|---|---|---|---|
| `CALSCALE` | §3.7.1 | TEXT | none |
| `METHOD` | §3.7.2 | TEXT | none |
| `PRODID` | §3.7.3 | TEXT | none |
| `VERSION` | §3.7.4 | TEXT | none |
| `ATTACH` | §3.8.1.1 | URI | BINARY |
| `CATEGORIES` | §3.8.1.2 | TEXT | none |
| `CLASS` | §3.8.1.3 | TEXT | none |
| `COMMENT` | §3.8.1.4 | TEXT | none |
| `DESCRIPTION` | §3.8.1.5 | TEXT | none |
| `GEO` | §3.8.1.6 | FLOAT | none |
| `LOCATION` | §3.8.1.7 | TEXT | none |
| `PERCENT-COMPLETE` | §3.8.1.8 | INTEGER | none |
| `PRIORITY` | §3.8.1.9 | INTEGER | none |
| `RESOURCES` | §3.8.1.10 | TEXT | none |
| `STATUS` | §3.8.1.11 | TEXT | none |
| `SUMMARY` | §3.8.1.12 | TEXT | none |
| `COMPLETED` | §3.8.2.1 | DATE-TIME | none |
| `DTEND` | §3.8.2.2 | DATE-TIME | DATE |
| `DUE` | §3.8.2.3 | DATE-TIME | DATE |
| `DTSTART` | §3.8.2.4 | DATE-TIME | DATE |
| `DURATION` | §3.8.2.5 | DURATION | none |
| `FREEBUSY` | §3.8.2.6 | PERIOD | none |
| `TRANSP` | §3.8.2.7 | TEXT | none |
| `TZID` | §3.8.3.1 | TEXT | none |
| `TZNAME` | §3.8.3.2 | TEXT | none |
| `TZOFFSETFROM` | §3.8.3.3 | UTC-OFFSET | none |
| `TZOFFSETTO` | §3.8.3.4 | UTC-OFFSET | none |
| `TZURL` | §3.8.3.5 | URI | none |
| `ATTENDEE` | §3.8.4.1 | CAL-ADDRESS | none |
| `CONTACT` | §3.8.4.2 | TEXT | none |
| `ORGANIZER` | §3.8.4.3 | CAL-ADDRESS | none |
| `RECURRENCE-ID` | §3.8.4.4 | DATE-TIME | DATE |
| `RELATED-TO` | §3.8.4.5 | TEXT | none |
| `URL` | §3.8.4.6 | URI | none |
| `UID` | §3.8.4.7 | TEXT | none |
| `EXDATE` | §3.8.5.1 | DATE-TIME | DATE |
| `RDATE` | §3.8.5.2 | DATE-TIME | DATE, PERIOD |
| `RRULE` | §3.8.5.3 | RECUR | none |
| `ACTION` | §3.8.6.1 | TEXT | none |
| `REPEAT` | §3.8.6.2 | INTEGER | none |
| `TRIGGER` | §3.8.6.3 | DURATION | DATE-TIME |
| `CREATED` | §3.8.7.1 | DATE-TIME | none |
| `DTSTAMP` | §3.8.7.2 | DATE-TIME | none |
| `LAST-MODIFIED` | §3.8.7.3 | DATE-TIME | none |
| `SEQUENCE` | §3.8.7.4 | INTEGER | none |
| `REQUEST-STATUS` | §3.8.8.3 | TEXT | none |
| any other `iana-token` | §3.8.8.1 | TEXT | every type |
| an `X-` name | §3.8.8.2 | TEXT | every type |

**The table is part of the format.** A row added or changed moves the reading of every document that names that
property, so it is a new codec format by the rule of ADR 0055 section 8, never an edit of format 3. In particular
the properties later RFCs register (RFC 7986's `NAME`, `REFRESH-INTERVAL`, `SOURCE`, `COLOR`, `IMAGE`, `CONFERENCE`)
are §3.8.8.1 IANA properties to format 3, TEXT by default. By libical's design data, `NAME` and `COLOR` are TEXT and
the other four have no default and must always carry `VALUE`, so a format-3 entry that declares their type writes the
`VALUE` RFC 7986 requires and reads what an RFC 7986 writer writes; a format that adds their rows checks that against
RFC 7986's text first.

**The table is iCalendar's.** The codec reads one RFC 5545 component (ADR 0010), whatever its name. vCard (RFC 6350)
shares the content-line syntax and RFC 6868, and gives its properties defaults of its own; a codec of a vCard is a
later decision, with its own table, and is not read by this one.

#### 2.3 What each `sce:type` holds

| `sce:type` (or `optional<T>` of it) | Value types it holds | Encode writes | Decode reads |
|---|---|---|---|
| `string` | every type | the text, escaped when TEXT | the text, unescaped when TEXT |
| an integer (`uint8`–`int64`) | `integer`, `text` | decimal digits, as today | the integer grammar of §4.6.4, after unescaping when TEXT |
| `bool` | `boolean`, `text` | `TRUE` or `FALSE`, as today | `TRUE` or `FALSE` in either case, after unescaping when TEXT |
| `enum:<alias>` | `text` | the variant's text (ADR 0015) | the variant whose text the unescaped value is |

An integer or a truth value may be a TEXT because RFC 5545 lets an `X-` or IANA property carry one as TEXT, which is
how most writers of such properties write it, and a codec that reads it must read the line it is given. It may be an
INTEGER or a BOOLEAN because RFC 5545 lets the same property say so with `VALUE`. Which one a line is, is the
document's to state, and the default decides where it does not: `X-ALL-DAY` declared `bool` is a TEXT whose text is
`TRUE` or `FALSE`, and `SEQUENCE` declared `uint16` is an INTEGER.

An enum entry is a TEXT. RFC 5545 types every property whose value is a closed vocabulary (`STATUS`, `CLASS`,
`TRANSP`, `ACTION`, `METHOD`, `CALSCALE`) as TEXT, and a reader of TEXT applies its escapes; ADR 0015's "it is no
TEXT" is withdrawn. No escape produces a character of an `iana-token`, so the variants an enum entry matches are the
ones it matched; what changes is the failure of a malformed escape in an enum value, which is `line-bad-escape` now
and was `line-bad-value` (`STATUS:IN\-PROCESS`, `STATUS:IN-PROCESS\`).

A parameter has no value type (RFC 5545 types property values, not parameters): `sce:value` on a parameter entry is
refused, as `sce:value="text"` on one is today (`validation/attribute-rule-violated`).

#### 2.4 `VALUE` and `ENCODING` are the codec's

The codec writes and reads `VALUE` itself, from the entry's type, and `ENCODING` with it (RFC 5545 §3.2.7: a BINARY
value MUST be written with `ENCODING=BASE64`, and `8BIT`, the default, is the encoding of every other value). So:

- **Declaring them is refused.** An entry whose `sce:param` is `VALUE` or `ENCODING`, in any case, is
  `validation/content-line-parameter-reserved` (section 4). A document cannot read a line's type as a string beside
  its value: that pair would admit a type the value is not of, and section 2.5 gives the type its own place.
- **Encode.** A property's line is its name, then `;VALUE=<TYPE>` when the type is not the default, then
  `;ENCODING=BASE64` when the type is BINARY, then the declared parameters in declaration order, then `:` and the
  value. The codec's own parameters come first because they are facts of the line's type, written once; the declared
  ones keep the order of the document's entries.
- **Decode.** While reading a declared property's parameters the reader keeps its `VALUE` and its `ENCODING`. A
  second `VALUE` or `ENCODING` on one line is `line-too-many`, and one of several values (`VALUE=DATE,DATE-TIME`) is
  `line-bad-value`, as for any declared parameter. Once the parameters are read, and before anything else of the
  line is judged (a record line's required parameters, then its value), the line's type is judged: its `VALUE`, or
  the property's default, must be the entry's type, or one of its types (section 2.5); a BINARY line must carry
  `ENCODING=BASE64`, and a line of any other type no `ENCODING` but `8BIT`. Anything else is `line-value-type`. A
  `VALUE` naming a type RFC 5545 does not define (an `x-name` or an `iana-token` value type) is a type other than
  the entry's, so it is `line-value-type` too.
- **Undeclared properties** are skipped as before; their `VALUE` is not judged.

`line-value-type` is the seventh `line-` failure and the fourteenth of `CodecFailure::ALL`
(`sce-build/src/forge/codec_failure.rs`): `CodecError::LineValueType` in Rust, `codec.ErrLineValueType` in Go,
`SCE_FORGE_CODEC_LINE_VALUE_TYPE` in C11 (appended after the existing statuses, whose numbers do not move),
`CodecError::LineValueType` in C++ (appended likewise), `CodecError.LineValueType` in Kotlin, `LineValueType` in
Python and `CodecError.lineValueType` in Swift. It is a decode failure only: an encode writes the type it holds, and
has no line of another type to refuse.

The type is judged before the value because it says what the value is: a line of another type is not a line of this
entry, whatever its text. A line that breaks both is refused for its type.

#### 2.5 A property that admits several types (decided in writing this record)

Eight properties admit more than one type (`DTSTART`, `DTEND`, `DUE`, `RECURRENCE-ID` and `EXDATE` a DATE-TIME or a
DATE; `RDATE` a DATE-TIME, a DATE or a PERIOD; `TRIGGER` a DURATION or a DATE-TIME; `ATTACH` a URI or a BINARY), and
an `X-` or IANA property admits all fourteen. A codec of real calendars reads both an all-day `DTSTART;VALUE=DATE`
and a timed `DTSTART`. With one type per entry, format 3 would refuse one of the two, where format 2 read both (with
the type lost). Decision 2 therefore needs an entry of several types, and this is its shape:

- **`sce:value` may list several types**, separated by white space as an XML list is (`sce:value="date-time date"`),
  each admitted by the property and held by the `sce:type`, none twice (`validation/attribute-rule-violated`
  otherwise). A list of one type is that type, and everything below is of a list of two or more.
- **The entry holds its line's type with its value, as one value.** Its value is a record of two members: `type`, an
  enum generated for the entry whose variants are the listed types in the order listed (named by the type in
  lower camel case, `dateTime`, `date`, `period`; its wire texts the types' names in upper case), and `value` (or
  `values`, for an entry with `sce:separator`). The record is `<Codec><Entry>Typed` (`<codec>_<entry>_typed_t` in
  C11). An entry that reads line records (ADR 0014) gains `type` as a member of its line record and no second record.
  A repeated entry without parameters is a list of `Typed` records. `optional<T>` is optional of the whole record,
  and a required entry starts at the first listed type.
- **Encode** writes `VALUE=<TYPE>` from `type` when it is not the property's default. **Decode** fills `type` from
  the line's type (its `VALUE` or the default), which must be one of the listed ones (`line-value-type` otherwise).
  The match is ADR 0015's (a table of texts in the generated codec, matched by the runtime without regard to case),
  with the property's default standing for an absent `VALUE`.

This is a sum of one-string cases written as a closed tag beside the string. Every combination of the two members is
a value the wire can carry, so the record admits no state the line cannot be in; a single enum of all fourteen types
shared by every entry would admit types the entry does not list, and a type field beside an optional value would be
present while the value is absent. Both are refused for that reason (section "Alternatives rejected").

### 3. Codec format 3

#### 3.1 The format is the encoding's

Decision 2 changes what an existing document means: a `string` `X-` entry without `sce:value` was carried as
written and is now a TEXT; a line with a `VALUE` was read with it skipped and is now judged by it; a parameter with a
`^` in it is now decoded. ADR 0055 section 8 settled what follows from such a change: no document is read with a
meaning other than the one it was written with, so the meaning is a format the document declares, the old format is
refused for good, and the refusal carries the document's own rewrite.

So a content-line document declares `sce:codec-format="3"`, the only content-line format read and generated.
`sce:codec-format="2"` and `="1"` on a content-line document, and no declaration, are refused as
`validation/codec-format-superseded` on the root; the rewrite of a format-1 document is format 1's rewrite into
format 2 followed by section 3.2's, computed in one pass. Any other value is `validation/invalid-attribute`, naming
`3`.

**A CBOR document stays in format 2.** Nothing of this record changes what a CBOR document means, and refusing one
whose rewrite is its own text with a digit changed would refuse a document whose meaning is intact, which the
refusal exists to prevent. The format number is therefore the format of the document's encoding: format 2 is the
current CBOR format and format 3 the current content-line format, `sce:codec-format="3"` on a CBOR document is
`validation/invalid-attribute` naming `2`, and `codec_format.rs`'s single `CURRENT` becomes the current format of
each encoding. Format numbers stay one sequence, so a number names one decision (`3` is this record), and an
encoding moves only when its own meaning changes, to the next number no decision has taken: a later change of what a
CBOR document means moves CBOR from 2 to that number, not to 3.

#### 3.2 The rewrite (decided in writing this record)

The rewrite is computed from what format 2 says of each entry, and states, entry by entry, where format 3 reads
differently. It never changes an entry's `sce:type`, so the program a document generates keeps the types it had:
where keeping the type narrows what a decode reads, the record says so, and the author decides whether to list a
second type (section 2.5), which does change the program.

| Format-2 entry | Format 3 reads it as | The rewrite writes |
|---|---|---|
| A value entry with `sce:value="text"`, on a property that admits TEXT | TEXT, as before | nothing |
| A value entry with no `sce:value`, on a property whose default is not TEXT (`DTSTART`, `RRULE`, `ATTENDEE`, `GEO`, ...) | its default type: the same bytes, and a line whose `VALUE` names another type is refused | nothing; where the property admits other types, the entry is named in the record: a line of the other type, read before with its `VALUE` skipped, is refused, and listing the types reads both |
| A `string` value entry with no `sce:value`, on a property whose default is TEXT (`UID`, `RESOURCES`, an `X-` or unregistered name) | TEXT: `\`, `;`, `,` and a line feed are escaped on encode and read as escapes on decode | `sce:value="text"`, and the entry is named in the record |
| An integer or `bool` entry whose property's default its `sce:type` holds (`SEQUENCE`, an `X-` or unregistered name) | that default: the same bytes; a line whose `VALUE` names another type is refused | nothing |
| An enum entry on a property whose default is TEXT | TEXT: the same variants; a bad escape is `line-bad-escape` | nothing |
| A value entry with `sce:value="text"` on a property that does not admit TEXT | its default type: a `\`, `;` or `,` in the value is written and read as itself | `sce:value` removed, and the entry is named in the record |
| An integer, `bool` or enum entry on a property whose admitted types its `sce:type` holds none of (an integer `DTSTART`) | nothing format 3 can say: RFC 5545 gives that property no type that is a number | nothing; the entry is named as one the author decides, and `check` then refuses it as `validation/content-line-value-type-not-admitted` |
| An entry whose `sce:param` is `VALUE` or `ENCODING` | the codec's own parameter | nothing; named as above, then refused as `validation/content-line-parameter-reserved` |
| Any other parameter entry | read and written by RFC 6868 | nothing |

The root changes to `sce:codec-format="3"`. RFC 6868 and the judging of `VALUE` change every document alike, so
the record states them once, on the root, and not per entry.

**Why TEXT for a `string` whose property defaults to TEXT.** Format 2 carried such a value "as written", which is no
RFC 5545 type: the bytes it wrote were read by every other reader as TEXT, because that is the property's default.
Keeping the old bytes exactly would need a type that has no escape, and every such type (URI, CAL-ADDRESS, ...)
asserts a grammar the author never stated and makes the codec write `VALUE=` on every line, which changes every line
rather than the few whose value holds `\`, `;`, `,` or a line feed. TEXT changes the bytes of exactly those values,
and on them it is SCE that disagreed with every reader, not the reverse. Where the value really is of another type
(an `X-ORIGINAL-URL` that holds a URI), the author writes that type, the record names the entry so that the choice
is seen, and the line is then written with the `VALUE` that says so.

**Why the default and not every admitted type, for `DTSTART` and its kind.** Listing every admitted type would keep
what format 2's decode accepted, and change the entry's value from a string to a typed record: every program that
reads it would stop compiling. Keeping the default keeps the program and the encode's bytes (format 2 wrote no
`VALUE`, so its every line was a DATE-TIME to a reader), and narrows the decode to the lines a DATE-TIME entry can
say anything true about. The author who reads all-day events lists both types, knowing the program changes.

**When an entry cannot be carried**, the fix still rewrites every other entry and the root, and the record names the
entries left as related sites (`SCE_ERROR_CONTRACT.md` §2.4). `sce-codegen codec-format <document>` prints the
rewritten document, names each entry it could not carry on standard error, and exits non-zero when there is one, so
a script that replaces a document with its output does not replace it with one `check` refuses.

### 4. Diagnostics

Two new codes, each a row of the appendix of `docs/SCE_ACCEPTED_SUBSET.md` ("Acceptance boundary",
`acceptance_doc_covers_every_code`) and of the generated catalog of `SCE_ERROR_CONTRACT.md` §5.1
(`error_contract_catalog`), with a golden in `sce-build/src/forge/diagnostic.rs` and the spec anchor
`SCE Forge §4.6.4`:

| Code | Stage | Raised for | Fix |
|---|---|---|---|
| `validation/content-line-value-type-not-admitted` | `validation` | A value entry whose type, written or defaulted, is one its property does not admit (section 2.2) or its `sce:type` does not hold (section 2.3); with a list, the first such type. The message names the property, its RFC 5545 section, the types it admits and the types the `sce:type` holds. | `replace_one_of`, the types both admit, when there is one; no `fix` when there is none (an enum on `ATTENDEE`, a `bool` on `SEQUENCE`), since the repair is then another `sce:type` or another property. |
| `validation/content-line-parameter-reserved` | `validation` | An entry whose `sce:param` is `VALUE` or `ENCODING`, which the codec writes and reads from the type of its property's entry (section 2.4). | none: the repair is to remove the entry and state the type on the property's entry, which is not an edit of this attribute. |

Existing codes, extended:

- `validation/codec-format-superseded` refuses a content-line document in format 2 as well as 1 (section 3.1). Its
  fix, `rewrite_codec_format`, gains one optional member per entry, `value`: the `sce:value` format 3 writes (a
  string), or `null` where it removes one. A member absent means the entry's `sce:value` is left as it is. Additive
  under `SCE_ERROR_CONTRACT.md` §8; `SCE_ERROR_CONTRACT.md` §3.1's row is amended to say so, and the 2-to-3 entries
  carry `type` as the entry's unchanged `sce:type`, so every entry of the payload keeps the shape it has.
- `validation/invalid-attribute` on `sce:codec-format` names `3` on a content-line document and `2` on a CBOR one; on
  `sce:value` it names the fourteen types.
- `validation/attribute-rule-violated` holds the rules that need no code of their own: `sce:value` on a parameter, a
  type listed twice.

Generation refuses by name (`codegen/unsupported`, through `content_line_codec::refusal`, which the conformance
schedule reads) what a backend has not landed, as every content-line feature has landed (section "Landing order").

### 5. Scope: every backend, every document, every lane

The rules above are the page's, `SCE_FORGE.md` §4.6.4, and every backend implements exactly them: Rust, C11, C++,
Go, Kotlin, Python and Swift, each in its runtime's reader and writer and in its generated codec. The plan of ADR
0081 carries, per entry, its type or types, its property's default and whether `VALUE` is written, so no backend
computes them; `codec_size.rs` counts the `VALUE` and `ENCODING` a line writes and two octets for each byte of a
parameter value (an RFC 6868 escape, as it counts a TEXT escape), so C11's `<NAME>_MAX_BYTES` stays a bound. The XSD
admits the fourteen names and a list of them on `sce:value`. The forge AST (`apis/forge-ast.v1.schema.json`,
`pre-release`, changed in place under `SCE_WIRE_CONTRACTS.md`'s policy) replaces `ContentLineEntry.text` with
`value_types` (the entry's types, defaults resolved, never empty) and `default_value_type` (its property's default),
so a reader of the AST needs no table. Every content-line document of the tree moves to format 3 by the rewrite, in
the step that makes format 3 current (section "Migration"). The conformance corpus gains the cases of section
"Conformance", and the TTCN-3 lane's binding reads the same rules.

The Interpreter lane runs no codec: a content-line codec is generated, never interpreted, so the Interpreter has
nothing to change.

## Consequences

- SCE's content-line codec reads and writes what RFC 5545 and RFC 6868 say, on every line: the two departures the
  lane found are closed, and the third difference stays libical's.
- A parameter can carry a `"` and a line break. Encode refuses fewer values (a `"` or a line feed in a parameter);
  decode reads `^n`, `^^` and `^'` as RFC 6868 says, which changes what a format-2 document read from a parameter
  that held `^`.
- Every `X-` and unregistered property of a document, and every registered TEXT property it did not mark, is a TEXT.
  The bytes of a value that holds `\`, `;`, `,` or a line feed change; no other value's do.
- A line whose `VALUE` names a type other than its entry's is refused where it was read. A document that read
  `DTSTART;VALUE=DATE` through a single-typed entry no longer does, and the record of its rewrite says so; it lists
  both types and its program holds a `Typed` record.
- A value whose type is not its property's default is written with `VALUE`, so an `X-` property declared `uri`,
  `integer` or `boolean`, and a `DTSTART` declared `date`, write a parameter they did not write.
- Each runtime's codec error gains `LineValueType`. An exhaustive match over it (Rust, Swift, Kotlin's `when`, a C
  `switch` with `-Wswitch-enum`) stops compiling until it names the case: a consumer-visible change, stated in
  `docs/SCE_CODEC_CONSUMER_MIGRATION.md` by the step that adds it.
- Two vectors of `codec_content_line_enums` change their expected failure from `line-bad-value` to
  `line-bad-escape`, and the drawn vectors whose `UID` (four fixtures), `TAG`, `REC` or `RESOURCES` value holds `\`,
  `;` or `,` change their bytes: the oracle's expected errors and bytes move, which the consumer-guide gate (ADR
  0078) requires the migration file to state.
- The C11 bound `<NAME>_MAX_BYTES` of a codec with a TEXT that was carried as written, or with a parameter, grows
  (a TEXT and a parameter value may each write two octets per byte).
- A CBOR document is untouched, and keeps `sce:codec-format="2"`.
- A format-2 content-line document is refused, for good, with its rewrite, as a format-1 one is.
- The default table is part of the format: a property registered after RFC 5545 is read as TEXT by default until a
  later format adds its row.

## Alternatives rejected

- **RFC 6868 only, and "as written" kept.** It closes (a) and leaves (b): an `X-` value SCE writes would still be read
  differently by every RFC reader.
- **"As written" kept as a fifteenth type**, the default where no `sce:value` is written. No RFC reader knows such a
  type; the bytes would be TEXT to them and not to SCE, which is the disagreement this record closes.
- **Reading an absent `VALUE` as the entry's type.** It accepts `DTSTART:20261008` into a DATE entry because the text
  looks like one; RFC 5545 §3.2.20 says that line is a DATE-TIME. A codec that read it would read a line by the
  document's expectation instead of by what the line says, which is the reading (b) found wrong.
- **Ignoring `VALUE` on decode**, as format 2 does. The line's type is lost, and a line of another type is read into
  an entry that says something false of it.
- **Reading format-2 documents with their format-2 meaning beside format 3.** Two readers of one encoding, and a
  document whose meaning depends on a digit the author may not have looked at; ADR 0055 section 8 refused this for
  format 1 and the reason has not changed.
- **One format number for every encoding, CBOR moved to 3.** It refuses every CBOR document of the tree and of every
  consumer with a rewrite that changes only the digit, for a change that does not touch them.
- **Rewriting a format-2 `string` on an `X-` property to a non-TEXT type, to keep its bytes.** It invents a grammar
  the author never stated, and writes `VALUE=` on every line, changing more bytes than TEXT does (section 3.2).
- **Rewriting a multi-type property's entry to every type it admits.** It keeps format 2's decode and breaks the
  program that reads the entry; the rewrite changes no type, and the author decides (section 3.2).
- **A type parameter declared as a `string` entry (`sce:param="VALUE"`).** The type would be a text beside a value,
  free to name a type the value is not of or to be present while the value is absent, and a writer would have to
  keep the two in step.
- **One runtime enum of the fourteen types for every multi-type entry.** It admits, in every entry, the types the
  entry does not list, and the encode would have to refuse a state the generated type could exclude.
- **The default table in each runtime.** Seven copies of one table, and the document's checks need it at build time
  anyway; the generator gives each entry its constants and the runtime matches what it is given.
- **A lexical check of each type** (a DATE-TIME's digits, a RECUR's grammar, BASE64) in the codec. It is a grammar
  per type and a failure per grammar in seven runtimes, for values the `algorithm` kind reads once they are out of
  the text (ADR 0010 decision 3). Nothing in this record depends on it, and it is not taken here.
- **Decoding `^N` as a line break.** RFC 6868 names `^n`; reading `^N` as one would be a guess at what a writer meant,
  and RFC 6868 tells a reader to keep an unknown sequence as it is.

## Migration

### The documents of the tree

Every content-line document is rewritten by `sce-codegen codec-format` in the step that makes format 3 current, and
the rewrite is checked by hand against this list. No document of the tree has an entry the rewrite cannot carry.

| Document | Entry (property) | Format 3 | Rewrite |
|---|---|---|---|
| `tests/forge/resources/codec_content_line_event.scxml` | `uid` (`UID`) | TEXT | `sce:value="text"` |
| | `dtstart` (`DTSTART`) | DATE-TIME; `VALUE=DATE` refused | none; named |
| | `dtstartTzid`, `organizerCn` (parameters) | RFC 6868 | none |
| | `summary`, `description` | TEXT, as before | none |
| | `organizer` (`ORGANIZER`) | CAL-ADDRESS | none |
| | `rrule` (`RRULE`) | RECUR | none |
| | `exdate` (`EXDATE`) | DATE-TIME; `VALUE=DATE` refused | none; named |
| | `sequence` (`SEQUENCE`), `priority` (`PRIORITY`) | INTEGER | none |
| | `allDay` (`X-ALL-DAY`, `bool`) | TEXT holding `TRUE`/`FALSE`: the same bytes | none |
| `tests/forge/resources/codec_content_line_records.scxml` | `uid` (`UID`) | TEXT | `sce:value="text"` |
| | `attendee` (`ATTENDEE`) | CAL-ADDRESS | none |
| | `attendeeCn`, `attendeeRole`, `exdateTzid` | RFC 6868 | none |
| | `exdate` (`EXDATE`, `,` list) | DATE-TIME; `VALUE=DATE` refused | none; named |
| | `categories` (`CATEGORIES`) | TEXT, as before | none |
| | `geo` (`GEO`, `;` list) | FLOAT | none |
| `tests/forge/resources/codec_content_line_required_lists.scxml` | `uid` (`UID`) | TEXT | `sce:value="text"` |
| | `tag` (`TAG`, unregistered) | TEXT | `sce:value="text"` |
| | `cats` (`CATS`) | TEXT, as before | none |
| | `rec` (`REC`, unregistered) | TEXT | `sce:value="text"` |
| | `recKind` | RFC 6868 | none |
| `tests/forge/resources/codec_content_line_enums.scxml` | `uid`, `note` | TEXT, as before | none |
| | `status` (`STATUS`), `clazz` (`CLASS`), `history` (`X-HISTORY`), `step` (`X-STEP`) | TEXT: same variants, a bad escape is `line-bad-escape` | none |
| | `attendee` (`ATTENDEE`) | CAL-ADDRESS | none |
| | `attendeeRole`, `attendeeStatus`, `stepNote` | RFC 6868 (an enum parameter is matched after decoding) | none |
| `tests/forge/resources/codec_content_line_open.scxml` | `uid` (`UID`) | TEXT | `sce:value="text"` |
| | `description`, `categories` | TEXT, as before | none |
| | `attendee` (`ATTENDEE`) | CAL-ADDRESS | none |
| | `attendeeCn` | RFC 6868 | none |
| | `resources` (`RESOURCES`) | TEXT | `sce:value="text"` |
| `sce-build/tests/a_codec_can_read_and_write_one_content_line_component.rs`, `EVENT` and the documents built from `document()` | `uid` (`UID`) | TEXT | `sce:value="text"` |
| | `dtstart`, `exdate`, `rrule`, `sequence`, `allDay` | as in the event fixture | none |
| | the probes on `N` (`uint8`) | TEXT holding digits | none |
| `sce-build/tests/forge_conformance.rs`, `OPEN_LINE` | `categories` (`CATEGORIES`, `,` list, no `sce:value`) | TEXT: `\,` is part of a value, not a cut | `sce:value="text"` |
| | `resources` (`RESOURCES`) | TEXT | `sce:value="text"` |
| | `description`, `attendee` | as in the open fixture | none |
| `sce-build/src/forge/codec_size.rs`, the content-line tests (`CONTENT_LINE_ATTRS`) | `uid` (`UID`, `sce:max-size="8"`) | TEXT: the largest line is 16 octets of value, not 8 | `sce:value="text"`; the expected bound (`77`) is computed again |
| | `seq` (`SEQUENCE`), `status` (`STATUS`, enum) | INTEGER; TEXT | none |
| `sce-build/tests/a_codec_declares_the_format_it_is_written_in.rs` | `content_line()` documents declare `3`; `MIXED_CONTENT_LINE` (format 1): `uid` (`UID`) and `note` (`NOTE`) | TEXT | the expected rewrite of format 1 gains `sce:value="text"` on both and declares `3`; `CONTENT_LINE_RECORDS_FORMAT_1`'s expected rewrite is the format-3 records fixture |
| `SCE_FORGE.md` §4.6.4, the example | `uid` (`UID`) | TEXT | `sce:value="text"`, and `sce:codec-format="3"` |

The comments of the fixtures that say "a value carried as it is" (`rrule` in the event fixture) say "a value of a
type that is not TEXT" instead. The ADRs 0010, 0014 and 0015 are records and keep their examples.

The enum documents (`enum_content_line_status.scxml`, `enum_content_line_class.scxml`,
`enum_content_line_role.scxml`) are not codecs and do not change.

### Generated trees

A changed fixture moves the `source-hash` of every committed tree generated from it. The trees that hold the
content-line fixtures are the Swift tests'
(`backends/swift/tests/Sources/StaticDatamodelMachines/CodecContentLine*.swift`, regenerated by
`scripts/regen_static_datamodel_swift.sh`); the Rust, Go, Kotlin and Python `static_datamodel` trees
(`scripts/regen_static_datamodel_{rust,go,kotlin,python}.sh`) are regenerated in the same step wherever a fixture of
this list enters their source set, and the Go round-trip tree (`backends/go/forge-runtime/round_trip/generate.sh`)
where its source set takes one. The landing step that changes the fixtures lists the trees it found moved.

### A consumer

`docs/SCE_CODEC_CONSUMER_MIGRATION.md` gains one section per step that changes what a consumer compiles or decodes
(steps 1, 3 and 4 below), each naming its commit. The consumer, at the step that makes format 3 current:

1. runs `sce-codegen codec-format <document>` on each content-line document it owns, reads the entries the output
   names, and writes the type of each value that is not what the rewrite chose (a URI on an `X-` property, both types
   of a `DTSTART` that reads all-day events);
2. adds `LineValueType` to every exhaustive match over the codec error;
3. regenerates and builds, and expects the bytes of a TEXT value that holds `\`, `;`, `,` or a line feed, and of a
   parameter that holds `^`, `"` or a line feed, to change on the wire, which is the change that makes them readable
   by every other iCalendar reader.

A document that needs a property's second type does not move its pin past step 3 until step 4 has landed on its
backend: step 3 refuses a multi-type entry by name, and a single-typed one refuses the lines of the other type.

## Landing order per backend

Each step lands alone and green: what a backend has not landed it refuses by name (`codegen/unsupported` through
`content_line_codec::refusal`), as ADR 0010, 0014 and 0015 landed. A step that changes a public header of the Forge
C or C++ runtime that `scripts/embed_manifest.py` names emits `embed/MANIFEST.json` and `embed/MANIFEST.inputs` in
the same commit.

1. **The runtimes learn the rules, unused.** In each runtime, one commit each, in this order: Rust, Go, Python,
   Kotlin, C++, Swift, C11. The reader decodes a parameter by RFC 6868 and keeps a line's `VALUE` and `ENCODING`;
   it offers the judging of a line's type (the expected type or types, and the property's default) and the TEXT
   reading of an integer, a truth value and an enum; the writer escapes a parameter by RFC 6868, writes `VALUE` and
   `ENCODING`, and folds an RFC 6868 escape whole; the codec error gains `LineValueType`. The generated codecs do
   not yet call the new entry points, so no vector moves: where a new rule replaces an old one (a parameter read by
   RFC 6868, an enum read as a TEXT), the runtime keeps the old entry point beside the new until step 3, which
   removes it. Tests, each in the runtime's own suite: Rust
   `content_line::tests::a_parameter_value_is_read_and_written_by_rfc_6868` and
   `a_line_of_another_value_type_is_refused`; Go `TestContentLineAParameterValueIsReadAndWrittenByRFC6868` and
   `TestContentLineALineOfAnotherValueTypeIsRefused`; Python `test_a_parameter_value_is_read_and_written_by_rfc_6868`
   and `test_a_line_of_another_value_type_is_refused`; Kotlin `ContentLineCodecTest` (the same two names); Swift
   `ContentLineTests.swift`'s `@Test` functions `aParameterValueIsReadAndWrittenByRFC6868` and
   `aLineOfAnotherValueTypeIsRefused`;
   C++ `content_line_runtime` (ctest); C11 `content_line_runtime_test.c`. Each holds: `^n`, `^^`, `^'` decoded; `^x`
   and a final `^` kept; `"`, `^` and a line feed written escaped and a carriage return refused; a fold never between
   `^` and its character; a `VALUE` of the expected type in either case read; a `VALUE` of another type, an absent
   `VALUE` where the expected type is not the default, a BINARY line without `ENCODING=BASE64`, and an `x-name` type
   each `line-value-type`; a second `VALUE` `line-too-many`; `VALUE=A,B` `line-bad-value`.
2. **The declaration and the table** (`sce-build`, one commit): `content_line_value_type.rs` with section 2.2's table,
   each row checked against the RFC 5545 text and citing its section; a content-line document declaring
   `sce:codec-format="3"` is parsed and checked by this record (`sce:value`'s fourteen names in the parser and the
   XSD, defaults resolved, the checks of sections 2.3 and 2.4, a multi-type `sce:value`), and its AST carries
   `value_types` and `default_value_type`; every generator refuses a format-3 document by name, so nothing generates
   from it yet. Format 2 stays the current content-line format, read and generated as before, and no document of the
   tree changes, so no vector moves: for one step two formats are read, the new one by `check` only. The two new
   codes land with their goldens, appendix rows and catalog rows. Tests: a new
   `sce-build/tests/a_content_line_value_has_the_type_rfc_5545_gives_it.rs`
   (`every_property_of_rfc_5545_defaults_to_the_type_its_section_states`,
   `an_x_name_or_an_unregistered_property_defaults_to_text`, `a_type_the_property_does_not_admit_is_refused`,
   `a_type_the_entry_cannot_hold_is_refused_with_the_types_both_admit`, `value_and_encoding_are_the_codecs_own`,
   `a_parameter_takes_no_value_type`, `a_type_listed_twice_is_refused`), the goldens
   `forge/content-line-value-type-not-admitted` and `forge/content-line-parameter-reserved` in `diagnostic.rs`,
   `acceptance_doc_covers_every_code` and `error_contract_catalog`.
3. **Format 3, the model, the vectors and the seven generators, in one commit.** `sce:codec-format` per encoding,
   content-line current `3`; format 2 refused with section 3.2's rewrite and the extended `rewrite_codec_format`; the
   generators' refusal of format 3 lifted for a single-typed entry;
   `sce-codegen codec-format`; the plan carries the types and the `VALUE` text; `codec_size.rs`'s counts; every
   content-line document of the tree rewritten (section "Migration") and its generated trees regenerated; the model
   (`content_line_model.py`) holding the new rules from its own table, written from RFC 5545 and not from
   `content_line_value_type.rs`, so no backend and not the table is its own oracle; the vectors regenerated
   (`python3 tests/forge/conformance/gen_cases.py`, then `--check`) with the new fixture
   `tests/forge/resources/codec_content_line_types.scxml` registered in `fixtures.json`; the seven templates (and
   `swift_codec.rs`) calling step 1's entry points; `SCE_FORGE.md` §4.6.4 and §4.6.1, `SCE_ERROR_CONTRACT.md` §3.1,
   `docs/SCE_ACCEPTED_SUBSET.md`, and the migration file's section. One commit, because the vectors move with the
   fixtures and a lane on the old generator would fail every moved vector: the backends are held to one corpus at
   every commit. Tests: `a_codec_declares_the_format_it_is_written_in.rs` gains
   `a_content_line_document_in_format_2_is_refused_with_its_rewrite`, `a_cbor_document_in_format_3_is_refused_naming_2`,
   `the_rewrite_writes_text_where_the_reading_becomes_text`, `the_rewrite_names_each_entry_whose_decode_narrows`,
   `a_content_line_document_in_format_1_is_rewritten_into_format_3`, and its existing format-1 rewrite tests expect
   format 3; `the_content_line_vectors_say_what_the_page_says.rs` takes the new fixture and asserts that
   `line-value-type` is refused by a vector; `codec_failure.rs`'s `wire_names_round_trip` covers the new name; and the
   seven conformance lanes run the corpus (Rust `forge_content_line_codec` and the numerical harness, ctest
   `numerical_conformance` and `numerical_conformance_c`, the Go, Kotlin and Python harnesses, Swift
   `NumericalConformanceTests`). Multi-type entries stay refused by name.
4. **Several types per entry** (section 2.5): the `Typed` record and the entry's type enum in the plan, then one
   commit per backend in step 1's order, each lifting its refusal, with a second new fixture
   `tests/forge/resources/codec_content_line_several_types.scxml` (an all-day and a timed `DTSTART`, an `RDATE`
   line record of a `PERIOD` with a `TZID`, a `TRIGGER` of both types, an `X-` property of three types) whose vectors
   the model writes. Tests: each lane's harness on the fixture, and
   `a_content_line_value_has_the_type_rfc_5545_gives_it.rs` gains
   `an_entry_of_several_types_holds_the_type_of_its_line`.
5. **The lane.** The downstream's binding reads format 3 (section "Conformance") and moves its `SCE_PIN` past step 3,
   then past step 4.

## Conformance

### The corpus

The independent model (`tests/forge/conformance/content_line_model.py`) is written from this record and the page,
with its own table of section 2.2 written from RFC 5545's sections, and writes every case of the five fixtures again
plus those of the new fixture `codec_content_line_types.scxml`, whose entries reach each rule once: `uid` (`UID`, no
`sce:value`, so TEXT), `start` (`DTSTART`, `sce:value="date"`), `due` (`DUE`, its default) with `dueTzid`, `attach`
(`ATTACH`, `binary`), `link` (`X-LINK`, `uri`) with `linkLabel`, `memo` (`X-MEMO`, TEXT by default) with `memoNote`,
`count` (`X-COUNT`, `int32`, `integer`) and `flag` (`X-FLAG`, `bool`, TEXT by default). (`start` carries no `TZID`:
RFC 5545 §3.2.19 does not apply one to a DATE, and the corpus holds no line an RFC reader would refuse.) The new
cases:

Written and read back:
- a parameter (`memoNote`) holding `^`, `"` and a line feed is written `^^`, `^'`, `^n`, unquoted, and read back;
- a parameter (`dueTzid`) holding `"` and `,` is written between quotes with `^'` inside;
- a fold falls before an RFC 6868 escape and never inside it;
- `start` writes `DTSTART;VALUE=DATE:20261008`;
- `link` writes `X-LINK;VALUE=URI;LABEL=...:...`, `VALUE` before the declared parameter, with a `\` in the value as
  itself;
- `attach` writes `ATTACH;VALUE=BINARY;ENCODING=BASE64:...`;
- `memo` writes `\\`, `\;`, `\,` and `\n`;
- `uid` holding `a;b,c` writes `UID:a\;b\,c`;
- `count` writes `X-COUNT;VALUE=INTEGER:-5`; `flag` writes `X-FLAG:TRUE`.

Read, and not what the encoder writes:
- a `^` followed by another character, and a `^` that ends a value, are kept with it;
- `^N` is kept as written;
- a quoted `"a^'b"` is `a"b`;
- `VALUE=date` in lower case is read; `DUE;VALUE=DATE-TIME:...`, the default written out, is read;
- `ENCODING=8BIT` on a TEXT line is read;
- an undeclared parameter holding `^'` and an undeclared property with any `VALUE` are skipped;
- the event fixture's `DTSTART;VALUE=DATE-TIME;TZID=Z;X-Q="a,b",c:...`, whose note says now that the `VALUE` is read
  and is the entry's type.

Refused:
- `DUE;VALUE=DATE:20261008` and `DTSTART:20261008T090000` for the date entry: `line-value-type`;
- `X-FLAG;VALUE=BOOLEAN:TRUE`, whose entry is a TEXT by its default although a `bool` could hold a BOOLEAN:
  `line-value-type`;
- `ATTACH;VALUE=BINARY:...` without `ENCODING`, and `X-MEMO;ENCODING=BASE64:...`: `line-value-type`;
- `X-LINK;VALUE=X-WEB:...`: `line-value-type`;
- `VALUE` twice on one line: `line-too-many`; `VALUE=DATE,DATE-TIME`: `line-bad-value`;
- `X-MEMO:a\qb`: `line-bad-escape`;
- a line whose `VALUE` is wrong and whose value breaks its bound: `line-value-type`, judged first;
- `STATUS:IN\-PROCESS` and `STATUS:IN-PROCESS\` in the enums fixture: `line-bad-escape` (they were `line-bad-value`).

Every fixture's random draws widen with the rules: a parameter draws `"`, `^` and a line feed, and a `UID` draws as a
TEXT. `the_content_line_vectors_say_what_the_page_says.rs` holds every failure of the page to a vector, which now
includes `line-value-type`.

### The TTCN-3 lane

The lane's content-line binding (`reference/codec/content_line/` of `sce-ttcn3-titan`, D3) is written clean-room from
RFC 5545 §3.1 and `SCE_FORGE.md` §4.6.4, and reads the codec documents of the corpus for the format's parameters. It
reads the same rules from the same texts, so for the lane this record means:

- **Its sources widen.** The binding implements RFC 6868 (from RFC 6868's text), the value types, `VALUE` and
  `ENCODING` (from RFC 5545 §3.2.7, §3.2.20 and §3.3), and format 3 as §4.6.4 states it. It reads `sce:value` from
  the documents and writes its own table of section 2.2 from RFC 5545 §3.7 and §3.8, not from SCE's, so a row the two
  disagree on fails every vector that reaches it; that is the independence the lane exists to give, and here it rests
  on two readings of the RFC's table and not only of SCE's page.
- **Its counts are of a corpus that moves.** "98 of 98, 52 of 52, 77 of 77" is against the oracle at its current
  `SCE_PIN`. Step 3 moves the oracle; the binding is changed and run against the new vectors before the lane's
  `SCE_PIN` passes step 3, and the counts are recorded again at that pin. Step 4's fixture joins when the binding
  reads a multi-type entry, until which the lane refuses it by name, as it refuses an enum entry today.
- **libical agrees with SCE on two of the three differences.** Its RFC 6868 and its TEXT reading of an `X-` value are
  now SCE's too. Whether the binding goes back to libical for its line reader and writer is the downstream's decision
  (the owner's and SCE's reviewer's, 2026-11-14) and is not reopened here; its remaining difference is the trailing
  white space, where RFC 5545 keeps what SCE keeps.
