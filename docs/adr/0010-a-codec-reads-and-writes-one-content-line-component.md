# ADR 0010 — A codec reads and writes one RFC 5545 content-line component

- Status: Accepted
- Date: 2026-10-08
- Scope: the `codec` kind (`SCE_FORGE.md` §4.6), `sce-build` (`forge/parser.rs`, `forge/model.rs`,
  the generator of each of the six backends), the conformance corpus
- Related: `SCE_FORGE.md` §4.6.1 (`sce:encoding="cbor"`, the precedent this follows),
  `docs/adr/0003-one-portable-document-compiled-per-target.md`

## Context

An `.ics` file is text: content lines, folded at 75 octets, with `\` escapes in text values and
`;`-separated parameters. SCE has no string processing in its `algorithm` kind, so the read and the
write cannot be an algorithm, and a host library per platform gives two parsers that disagree
(the reason `SCE_FORGE.md` generates a CBOR codec instead of asking each host for one). A
calendar consumer needs the import and export of one component, a `VEVENT`, with its dates,
title, attendees, `RRULE`, `EXDATE` and `RECURRENCE-ID`, and gets the rule itself (the `RRULE`)
from the algorithm kind once the line is out of the text.

The `codec` kind already has a second encoding, `sce:encoding="cbor"`: the root declares the wire
shape, each `<data>` is an entry, and every position attribute is refused because a map has none.
A line-oriented text format is the same kind of thing, and the same seam.

## Decisions

### 1. The encoding is `sce:encoding="content-line"`, with the component it carries

```xml
<scxml sce:kind="codec" sce:encoding="content-line" sce:component="VEVENT" name="event">
  <datamodel>
    <data id="raw"     sce:type="bytes"  sce:direction="in"/>
    <data id="uid"     sce:type="string" sce:property="UID"     sce:required="true" sce:max-size="256"/>
    <data id="summary" sce:type="string" sce:property="SUMMARY" sce:value="text"    sce:max-size="512"/>
    <data id="dtstart" sce:type="string" sce:property="DTSTART" sce:required="true" sce:max-size="32"/>
    <data id="dtstartTzid" sce:type="string" sce:property="DTSTART" sce:param="TZID" sce:max-size="64"/>
    <data id="rrule"   sce:type="string" sce:property="RRULE"   sce:max-size="256"/>
    <data id="exdate"  sce:type="string" sce:property="EXDATE"  sce:max-count="64" sce:max-size="32"/>
    <data id="attendee" sce:type="string" sce:property="ATTENDEE" sce:max-count="100" sce:max-size="256"/>
  </datamodel>
</scxml>
```

The name is the format's (RFC 5545 §3.1 defines the content line; RFC 6350 vCard uses the same), not
the calendar's, so a second component or format that shares the grammar reuses it. A format that
does not (CSV, a log line) is another encoding with its own page in `SCE_FORGE.md`, not a mode of
this one.

An entry is a `<data>` of the codec's `<datamodel>`:

| Attribute | Meaning |
|---|---|
| `sce:property` | The property name the entry reads and writes (`SUMMARY`). Matched case-insensitively on decode, written as declared. |
| `sce:type` | `string`, an integer (`uint8`–`uint64`, `int8`–`int64`) or `bool` (`TRUE`/`FALSE`). A parameter is a `string`. |
| `sce:value="text"` | On `string`: the value is an RFC 5545 TEXT, so `\\`, `\;`, `\,` and `\n` are escapes on both sides. Without it the value is carried as written. Only TEXT escapes; a date-time, an `RRULE` and a URI are not TEXT. |
| `sce:param` | The entry is the named parameter of the property `sce:property` names, not its value (`TZID` of `DTSTART`). A parameter value that holds `:`, `;` or `,` is written between double quotes and read back without them. |
| `sce:required="true"` | A decode of a component without the property is refused, and one whose property lacks a required parameter. A required parameter belongs to a required property. Optional otherwise, in every language. |
| `sce:max-size` | Required on every `string`: the most bytes it holds, after unescaping. A text is as long as its sender wrote it, so the bound is the codec's to state, and every backend then holds a value in storage it can size. |
| `sce:max-count` | On a value entry, the most lines of the property a component holds, at least 2; the entry is then a bounded list of values in line order. |

### 2. The wire rules are written once, and every backend writes the same bytes

Encode writes `BEGIN:<component>`, the entries present in declaration order (a property's
parameters after its name, in declaration order, then `:` and the value), and `END:<component>`,
each line ending in CRLF. A line longer than 75 octets is folded: CRLF and one space, at a place
that does not split a UTF-8 sequence. Decode unfolds, takes the first `BEGIN:<component>` to its
`END:<component>` (so a `VCALENDAR` around it is no concern of the codec's), reads the properties
it declares, skips a property it does not declare and any nested component (`VALARM`) whole, and
refuses a malformed line, a property that repeats where the entry holds one value, a list past its
`sce:max-count`, a value past `sce:max-size`, a bad escape in a TEXT, an integer or `bool` value its
entry cannot hold, and a required property that is absent. The rules are written out once, in
`SCE_FORGE.md` §4.6.4, which is the page a backend generates from.

What is not promised: a component decoded and encoded again does not carry the properties the codec
does not declare. A calendar's merge works on recurrence rules and not on text, and `.ics` is an
import and export format here, not a store.

### 3. Values are text; the date and the rule are the algorithm's

A date-time (`20261008T093000Z`), a date and an `RRULE` are `string` entries of this codec. The
algorithm kind reads them once they are out of the text (`sce:std/time/` and the `RRULE`
expansion). The codec does not know a calendar's types, and gains none from this ADR.

### 4. Refused on a content-line codec

Every position attribute (`sce:byte`, `sce:bit-offset`, `sce:bit-size`, `sce:endian`,
`sce:default-endian`, `sce:length-field`, `sce:length-arith`, `sce:present-if`,
`sce:dma-burst-align`), `sce:key` (a CBOR key), `sce:length`, and the `<sce:field>`,
`<sce:flags>`, `<sce:repeat>`, `<sce:tlv-chain>`, `<sce:embed>`, `<sce:variant>`,
`<sce:flag-inputs>` and `<sce:test-vector>` elements. A parameter on an entry that has
`sce:max-count`: a repeated property whose lines each carry their own parameters (`ATTENDEE;CN=`)
is a list of records, which this encoding does not have (see **Not now**).

### 5. Order of work

The CBOR codec is the precedent for the order, and the reason it landed without a backend lagging
the contract: one commit declares the encoding and refuses generation by name, then one commit per
backend generates it, each against the same conformance vectors.

1. Declare: the parse and the checks, the page and the AST, `SCE_FORGE.md`, the schema, the
   refusals above, and a refusal by name of generation for every backend.
2. The conformance vectors (`tests/forge/conformance/`): the RFC 5545 §3.1 examples, and cases an
   independent Python model of the grammar writes (`gen_cases.py`'s way), so no backend's output is
   its own oracle.
3. Generate it for Rust, Kotlin, C++, Go, Python and C11, one commit each.

## Not now

- *An `enum:<alias>` entry.* The decision above first admitted one, as a property or a parameter
  whose text is a variant's declared name. Writing the generation found what it needs and nothing
  offers: a codec generator reaches an enum import's carrier (`from_underlying`), not the name of a
  variant, and a name-to-variant table would be six new lookups in six enum spellings for a calendar
  consumer that reads `STATUS`, `PARTSTAT` and `FREQ` as text its algorithm compares. The entry is
  refused by name until a codec that names variants is built, and a `string` carries the text.
- *A parameter on a repeated property.* `ATTENDEE;CN=Kim;PARTSTAT=ACCEPTED:mailto:…` needs each
  line to be a record of a value and its parameters, so a bounded list of records and a record the
  codec embeds. That is new to every backend's codec generator, and a calendar consumer's first
  stage reads an attendee's address and nothing else (the entry above keeps the address and drops
  the parameters, which is the stated limit of decision 2).
- *A property whose value is a list on one line* (`EXDATE:…,…`, `CATEGORIES:a,b`). The value is a
  `string` the algorithm kind cuts at the comma once the algorithm kind has the means; a
  `sce:separator` that splits it in the codec is the next addition, not this one.
- *Base64 `BINARY` values.* An attachment is out of a calendar consumer's first stages.

## Rejected

- *A host library per platform.* It was the first plan and was withdrawn: two parsers is the
  divergence this design exists to avoid.
- *String functions in the `algorithm` kind.* They would let the parser be an algorithm, and put
  Unicode, escape and fold rules into every backend's expression lowering, where nothing holds
  them to one answer; a codec's page is the place a wire rule is written once.
- *One generic "line" record (`name`, parameters, `value`) and the mapping to fields in an
  algorithm.* It moves the problem and not the work: the mapping is string processing again.
