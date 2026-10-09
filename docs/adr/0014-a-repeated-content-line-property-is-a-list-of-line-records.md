# ADR 0014 — A repeated content-line property is a list of line records

- Status: Accepted
- Date: 2026-10-09
- Scope: the `codec` kind's `sce:encoding="content-line"` (`SCE_FORGE.md` §4.6.4), `sce-build`
  (`forge/parser.rs`, `forge/model.rs`, `forge/content_line_codec.rs`), the runtime and the generator
  of each of the six backends, `apis/forge-ast.v1.schema.json`, the conformance corpus
- Related: `docs/adr/0010-a-codec-reads-and-writes-one-content-line-component.md` (this closes two
  of its *Not now* items), `SCE_FORGE.md` §4.6.4

## Context

ADR 0010 reads and writes one RFC 5545 component. It left two things out and named the reason for
each: a parameter on a property that occurs on several lines (`ATTENDEE;CN=Kim;PARTSTAT=ACCEPTED:…`),
because each line would be a record, and a property whose value is a list on one line
(`EXDATE:…,…`), because the codec had no `sce:separator`. It ranked the first as no consumer's first
stage and the second as "the next addition".

Run through the generated codec, a time-zoned exception shows what the two omissions cost together:

```
EXDATE;TZID=Asia/Seoul:20261008T093000
EXDATE;TZID=Asia/Seoul:20261015T093000,20261022T093000
```

decodes to `exdate = ["20261008T093000", "20261015T093000,20261022T093000"]`. Two things are wrong,
and they are not the same kind of wrong.

- **The zone is gone, silently.** A parameter no entry declares is skipped, and an entry that holds
  several lines cannot declare one (ADR 0010, decision 4). The values come back as floating
  date-times, and the algorithm that reads them reads Seoul's 09:30 as nobody's. Nothing is refused
  and the author has no declaration that would stop it. That is a wrong answer, not a missing
  feature.
- **A comma list is one value.** The second line is a single string the algorithm cannot cut,
  because the `algorithm` kind has no string processing (ADR 0010, *Rejected*). That is a missing
  feature, and it is the smaller of the two.

A `sce:separator` alone closes only the second. It also has to be put somewhere the first has not
already put a line's parameters, and the place is the same: a line of a repeated property is a unit
with its own parameters and its own value or values, and the codec has no such unit. The two
belong to one design, decided once, so the second does not have to be moved when the first arrives.

## Decisions

### 1. A repeated property may declare parameters; each of its lines is a record

An entry with `sce:max-count` may be followed by `sce:param` entries of its property, as a
single-valued property's are. Its field is then a bounded list of **line records**: one record per
line, in line order, holding the parameters that line carries and its value.

```xml
<data id="exdate" sce:type="string" sce:property="EXDATE" sce:max-count="64" sce:max-size="32"/>
<data id="exdateTzid" sce:type="string" sce:property="EXDATE" sce:param="TZID" sce:max-size="64"/>
```

`exdate` is a list of at most 64 records, each `{ tzid, value }`. A parameter is optional or
required **per line** (`sce:required="true"` on it means every line of the property carries it).
A required parameter of a repeated property needs no required property, as one of a single-valued
property does: it is required of each line that exists, and the property itself may have none.
An entry with no parameter declared keeps its present shape, a list of values; the record exists
only where there is something for it to hold, so no codec that compiles today changes.

### 2. `sce:separator` makes a line's value a list

`sce:separator` on a `string` entry names the character that separates the values of one line: `,`
or `;` (RFC 5545 uses the comma for lists such as `EXDATE` and `CATEGORIES` and the semicolon in
structured values such as `GEO`). It requires `sce:max-values`, at least 2, the most values a line
holds; each value is held to `sce:max-size`, as a value is now. A value of any other type is not a
list, and `sce:separator` on an integer or a `bool` is refused.

A single-line property with a separator is a list of values (`CATEGORIES:a,b`). A repeated one is a
list of line records whose value is a list of values (`EXDATE;TZID=…:a,b`). `sce:max-count` counts
lines and `sce:max-values` counts the values of one line; neither stands for the other.

### 3. Wire rules

*Decode.* A line's parameters are read as a single-valued property's are, into the record of that
line: a declared one is kept, an undeclared one is skipped, a declared one holds one value (a second,
after a `,`, is `line-bad-value`) and appears once (a repeat is `line-too-many`). A line that lacks a
required parameter is `line-required-missing`, raised at that line.

The value is cut at the separator **before** it is unescaped. For a TEXT (`sce:value="text"`) a
separator that a `\` precedes is part of the value and a `\` also escapes the next character, so
`a\,b,c` is the two values `a,b` and `c`; for any other value there is no escape and every separator
cuts. The parts are judged left to right and the first failure is the line's: a part past
`sce:max-values` is `line-too-many`, one that is empty (`a,,b`, `,a`, `a,`, a value of nothing) is
`line-bad-value` because RFC 5545 has no empty member of a list, and any other is unescaped and held
to `sce:max-size`. A line's parameters are read before its value, and a required one that the line
lacks is `line-required-missing` before the value is looked at.

*Encode.* A record writes its present parameters in declaration order, then `:` and its values
joined by the separator, with no space. A TEXT value writes the escapes it always does, and the
separator between values is written as it is; a value that holds the separator is therefore escaped
and still one value on the way back. A value that is not a TEXT has no escape, so one that holds the
separator, and any empty value of a list, is `line-bad-value` on encode: it would be read back as
other than it was. A record with no value, or a required list with none, is `line-required-missing`;
a line count past `sce:max-count` and a value count past `sce:max-values` are `line-too-many`.

Folding, the control-character rule, the quoting of a parameter and every other rule of
`SCE_FORGE.md` §4.6.4 stand as written.

### 4. What is refused changes in one place

ADR 0010 refused a parameter on an entry that has `sce:max-count`. That refusal goes; a parameter
there is now a field of the line record. Two entries of one property, two parameters of one name on
a property, and a parameter with no entry of its property before it stay refused. `sce:separator`
without `sce:max-values`, `sce:max-values` without `sce:separator`, and either on an entry that is
not a `string` value entry are refused by name.

### 5. Order of work

The order ADR 0010 followed, for the same reason: one commit declares the change and refuses
generation of it by name, so no backend lags the contract.

1. Declare: the parse, the checks and the AST (`apis/forge-ast.v1.schema.json` is pre-release, so the
   additive and the changed shape are both allowed), `SCE_FORGE.md` §4.6.4, and a refusal by name of
   generating a codec that uses either.
2. The conformance model (`tests/forge/conformance/content_line_model.py`) and its vectors: the
   TZID and comma-list case above, a list of records with and without separators, every refusal.
   The model is written from this page, so no backend's output is its own oracle.
3. Generate it for Rust, Kotlin, C++, Go, Python and C11, one commit each, against the same vectors.

## Not now

- *An `enum:<alias>` entry*, as in ADR 0010. It waits on a codec that names variants.
- *A property whose value is itself structured and typed* (`GEO`'s two numbers, `RRULE`'s parts).
  A separator makes `GEO` two strings; reading them as numbers is the algorithm's work and not
  this codec's.
- *Base64 `BINARY` values*, as in ADR 0010.
- *A parameter with several values* (`MEMBER="a","b"`). A parameter holds one, as it does now.

## Rejected

- *`sce:separator` alone, before records.* It closes the smaller of the two defects and leaves the
  zone dropped; the records would then have to be put under a list shape the separator had already
  chosen.
- *Parallel lists* (`exdate` and `exdateTzid`, the n-th of one belonging to the n-th of the other).
  They misalign the first time a line lacks a parameter, which is the common case, and no backend's
  type says they belong together.
- *One generic line record* (`name`, `params`, `value`) read into every codec. It moves the mapping
  from the codec into an algorithm, which is string processing again (ADR 0010, *Rejected*).
- *Keeping the silent skip and documenting it.* A parameter that changes the meaning of the value
  it sits on (`TZID`) is not a parameter an author should be able to lose without declaring so.
