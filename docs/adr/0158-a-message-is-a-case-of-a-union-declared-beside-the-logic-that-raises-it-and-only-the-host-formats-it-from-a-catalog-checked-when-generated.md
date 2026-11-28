# ADR 0158 — A message is a case of a union declared beside the logic that raises it, a catalog per language holds it in MessageFormat 2 and is checked when generated, and only the host formats it, over pinned Unicode and CLDR data

- Status: Accepted (2026-11-28) under the owner's standing rule (the most long-term correct, textbook structure, cost
  ignored), by the main session the owner delegated the decision to, after an independent review; amended on
  acceptance as listed in "Amended on acceptance".
- Date: 2026-11-27
- Numbered 0148 when first written; renumbered on 2026-11-28 because that number also named ADR 0148, "A machine
  whose document leaves Appendix D's main loop one step per event takes a reactive turn", an accepted record the
  runtimes' code cites.
- Amends (each amended document gains an "Amended by: ADR 0158" line in the landing that moves it):
  - `docs/SCE_ACCEPTED_SUBSET.md` §2.1 (a new kind, `message-catalog`, counted when it lands) and its appendix (the
    codes of D3 and D8);
  - `SCE_ERROR_CONTRACT.md` (the same codes);
  - `docs/SCE_HOST_FEATURES.md` (a new section, formatting at the edge, D5, numbered when it lands: section 16 is the
    event journal's);
  - `docs/SCE_SERVER_DEPLOY.md` §2 (deploy.yaml gains `languages:`, D5);
  - ADR 0094 §1 (a third entry of the input closure that is not a document: the pin file, D6);
  - ADR 0097 §2 (the pin file among an action's inputs, so the cache key digests it, D6);
  - the comment of `stdlib/text/utf8_length.scxml` that says Unicode's tables are carried into no backend (D6).
- Scope: the contract names no language. Built under the owner's scope rule of 2026-11-22 (a new facility is built
  only on the backends a consumer needs, Rust by default; ADR 0117 and ADR 0151 to ADR 0153 are written under it),
  not under ADR 0109 D0, which builds its facilities for Rust and C++. Built first: `sce-build` (the kind, the
  MessageFormat 2 reader and every check of D3) and a new Rust crate `sce-rust-text` (formatting, selection, and the
  host's collator of D7), on native targets and on `wasm32-unknown-unknown`, which is the Rust lane (ADR 0117). Every
  other backend, the C++ client host included, refuses by name (D8).
- Related: ADR 0044 (an enum that grows), ADR 0050 (a bound read by name), ADR 0087 §3 (`text-template`: no locale
  enters), ADR 0092 (the union; §1.1 item 1, a union whose cases are all empty is its enum; §8.3, an older reader and
  a newer value), ADR 0094 §1 and ADR 0097 §2 (what a generated unit's identity covers), ADR 0109 D0 (Rust and C++;
  this record departs from it under the later scope rule, D8), ADR 0117 (wasm32 is the Rust lane), ADR 0149 (it shares
  the pin of D6), ADR 0156 D1 and D2 (catalogs and the formatter are presentation, outside every machine's model, D5).

## Context

A statechart that puts a sentence in front of a person has two ways to do it today, and both are wrong:

- **It writes the sentence.** A `<send>` or an `<assign>` builds the text, in one language, with `+`
  (SCE_FORGE.md §3.4, "String concatenation"). The sentence is then state: it is in the saved state, it is compared,
  it cannot be shown in another language, and a plural is an `if` on the count that is right for English alone.
- **It sends a code and the host owns the strings.** An enum crosses (ADR 0047), and the host maps it to a text of its
  own. Nothing holds the host's table to the enum: a variant added without its text, an argument the text names but
  the event does not carry, and a translation that drops a placeholder are found by a person looking at a screen.

What exists, and why none of it is the answer:

- **`text-template`** (ADR 0087 §3; `sce-build/src/forge/text_template.rs`; `docs/SCE_ACCEPTED_SUBSET.md` §2.1) is
  a text with typed holes rendered by one pure function, built to write a model's prompt. Its rule is that "no locale,
  no number format and no clock enter the function" (ADR 0087 §3, item 4): an integer is its ASCII digits, an enum its
  declared name, and there is no selection. SCE's earlier answer to this request was not to extend it, and that answer
  stands (Alternatives rejected).
- **The union** (ADR 0092) is a typed value that is exactly one of several cases, each a record, chosen by a tag that
  is a closed enum. It crosses the wire, is kept in a saved state, and is read where a test of its tag proves the case.
  It is exactly the shape of "an id with typed arguments".
- **No locale exists anywhere.** `sce-build` has none; a text is compared as its UTF-8 bytes (SCE_HOST_FEATURES.md
  section 3, "There is no normalisation"; `backends/rust/session-store/src/sqlite.rs`, the `BINARY` collation); Swift's
  `String` comparison, which is canonical equivalence and collation order, is replaced by a comparison of UTF-8 bytes
  so that every lane agrees (`sce-build/src/forge/expr/swift.rs`, the header comment).
- **Unicode's tables are carried by no backend.** `stdlib/text/utf8_length.scxml` says why it counts code points and
  does not normalise: normalisation "would carry Unicode's composition tables into every backend and a new table with
  each Unicode version". That is the cost this record pays, once, on one lane, with the version named.
- **ICU4X is already in the lock, by accident.** `icu_normalizer`, `icu_properties`, `icu_locale_core` and
  `icu_provider` 2.2.0 are in `Cargo.lock`, reached only through `url`'s IDNA support (`cargo tree -i url`:
  `reqwest`, a dependency of `sce-rust-tests`). No crate of SCE's own depends on them, and nothing names the Unicode
  or CLDR version they carry.

SCE's earlier answer to this request, which this record keeps: do not extend `text-template`; pin Unicode and CLDR.

## The textbook structure

- **The program names a message; a translator writes its text.** Internationalisation separates the identity of a
  message and its arguments, which the program owns, from the sentence in each language, which a catalog owns
  (GNU gettext; ICU MessageFormat; Mozilla Fluent; Unicode MessageFormat 2). The program never builds a sentence
  out of pieces, because word order, agreement and plural differ by language.
- **Unicode MessageFormat 2.0** (Unicode Technical Standard #35, Part 9, "MessageFormat"; a Final Candidate in LDML 46
  and stable from LDML 47, 2025) is the standard syntax and data model of one message: declarations (`.input`,
  `.local`), placeholders with a function and options (`{$count :integer}`), selection over variants
  (`.match`, with the catch-all `*` required), a default function set, namespaced custom functions, and a list of
  data-model errors an implementation reports before formatting. Its date and time functions are Draft in LDML 47.
- **Plural selection is CLDR's** (UTS #35 Part 3, "Language Plural Rules"): each language has its cardinal and its
  ordinal categories among `zero`, `one`, `two`, `few`, `many`, `other`, defined by rules over the number's digits, and
  CLDR gives sample numbers for each rule. A catalog's plural forms are checked against the language's, both ways:
  GNU gettext's `msgfmt --check` reads a catalog's plural forms against the count its `Plural-Forms` header declares.
- **A language is a BCP 47 tag** (RFC 5646), and the language shown is chosen from the user's preferences and the
  languages an application supports by RFC 4647's Lookup, ending at a default. A locale inherits from its parent
  locale (UTS #35 Part 1, CLDR's parent locales: `en-GB` to `en-001` to `en`), so a variety states only what differs.
- **Collation is presentation, not a property of text.** UTS #10, section 1.8, "What Collation is Not": a collation
  order is not fixed across versions, and a collation must be versioned. An order a program saves, or computes on two
  machines with two versions of the data, changes when the data changes. At a strength below identical, two different
  texts compare equal, so a collation is not a total order on texts. A program orders text by its code points; a
  person's order of a language is applied where the text is shown (UTS #35 Part 5, a language's tailoring).
- **Data is an input with a version.** A build that formats under Unicode and CLDR data names the versions as it names
  its compiler's, so the same inputs give the same output on every machine and a change of data is a change of input.
- **What can be checked before running is checked before running.** A catalog is a program in a small language; it is
  parsed and type-checked against the declaration of its messages when it is built, as `msgfmt --check-format` does
  for gettext's format strings, and not discovered at a screen.

## Decision

### D1. A message is a case of a union, declared beside the logic that raises it

A **message set** is a `union` document (ADR 0092 §1). Its tag is a closed enum whose variants are the messages' ids;
each case is the message's arguments, an imported event schema, or no record for a message that takes none. Two
messages with the same arguments share one record, as two cases may (ADR 0092 §1.1).

```xml
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="union" name="checkout_messages">
  <sce:import kind="enum" src="checkout_message_id.scxml" as="CheckoutMessageId"/>
  <sce:import kind="event-schema" src="item_count.scxml" as="ItemCount"/>
  <sce:import kind="event-schema" src="payee.scxml" as="Payee"/>
  <datamodel>
    <data id="message" sce:type="enum:CheckoutMessageId">
      <sce:case variant="cartEmpty"/>
      <sce:case variant="cartItems" record="ItemCount"/>
      <sce:case variant="paidTo" record="Payee"/>
    </data>
  </datamodel>
</scxml>
```

The documents live beside the statechart that raises the messages and are imported by it, so a change to the logic and
the change to its messages are one change. The statechart raises a message by holding a value of the union in a field
of an event it sends, built as ADR 0092 builds any union value. It never formats one (D5).

No new type and no new wire form: a message is a union value, so its JSON, its codec, its place in a saved state, its
revision rules (ADR 0044: a message added is a variant added, and a revision never renumbers one) and its narrowing are
ADR 0092's. A union is a message set because a catalog names it (D2), not because it is marked.

**A message set whose messages take no arguments is its enum.** ADR 0092 §1.1 item 1 refuses a union all of whose
cases are empty: "it is its enum, and one fact has one spelling". Such a message set is a closed enum document, its
variants the messages' ids, and a catalog names the enum as it names a union (D2). The statechart raises one by
holding a value of the enum. An open enum (`sce:strict-variants="false"`) is refused as a message set, as ADR 0092
refuses it as a tag: a number no variant names would be a message with no text.

### D2. A catalog is a document of one language: `message-catalog`

```xml
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="message-catalog" name="checkout_en" xml:lang="en">
  <sce:import kind="union" src="checkout_messages.scxml" as="CheckoutMessages"/>
  <sce:messages of="CheckoutMessages">
    <sce:message case="cartEmpty">Your cart is empty.</sce:message>
    <sce:message case="cartItems">.input {$count :integer}
.match $count
one {{You have {$count} item in your cart.}}
*   {{You have {$count} items in your cart.}}</sce:message>
    <sce:message case="paidTo">You paid {$name}.</sce:message>
  </sce:messages>
</scxml>
```

- `xml:lang` is the catalog's language: a well-formed BCP 47 tag (RFC 5646), written in its canonical form (UTS #35
  Part 1, "Canonical Unicode Locale Identifiers").
- `of` names the message set: a union or a closed enum the catalog imports (`<sce:import kind="union">` or
  `<sce:import kind="enum">`, D1).
- Each `<sce:message case>` names a case of the union, or a variant of the enum, and holds one MessageFormat 2
  message, simple or complex, as its XML character data after XML's own end-of-line handling and entity expansion,
  nothing trimmed: the same rule as a `text-template`'s text (ADR 0087 §3, item 2). A variable `$x` names the field
  `x` of the case's record; a case without a record, and every variant of an enum, has no field to name.
- One message set has any number of catalogs, at most one per language.
- **A catalog inherits from its parent locale.** The chain of a language is its CLDR parent locales at the pinned CLDR
  (UTS #35 Part 1), ending at the root. A catalog holds the entries in which its language differs from the catalog
  next on its chain, and may omit the rest: an `en-GB` catalog holds the sentences British English writes otherwise,
  and the `en` catalog the others. Completeness is judged over the chain when the build runs (D3), and the build
  resolves each case of each language to the entry its chain reaches (D5), so nothing falls back at run time.

### D3. A catalog is checked against its message set when it is generated

The build reads every entry by the grammar of UTS #35 Part 9 and reports its data-model errors there, not when a
person reads the text. Three new codes, each with its appendix row (`acceptance_doc_covers_every_code`):

- **`validation/message-catalog`**: a case of the message set that no entry of the catalog's chain holds (D2), an
  entry naming no case, two entries for one case, a language tag that is not well-formed or not canonical, two
  catalogs of one message set with one language, an open enum named as a message set (D1), and a language the
  deployment supports, its default included, whose chain reaches no catalog of a message set the build carries (D5).
  The diagnostic names the catalog, its language and the case.
- **`validation/message-syntax`**: an entry that the grammar does not accept, every data-model error the
  specification lists (Variant Key Mismatch, Missing Fallback Variant, Missing Selector Annotation, Duplicate
  Declaration, Duplicate Option Name, Duplicate Variant), and markup (`{#b}`, D4).
- **`validation/message-argument`**: a variable that is neither a field of the case's record nor declared in the
  message (the specification's "Unresolved Variable", made a build error); a function that is not in the registry of
  D4; a function applied to a field of a type it does not take; an option the function does not define, or whose value
  is not a literal it accepts; a selector key that can never be chosen, and a category the selector can reach with no
  variant of its own (below); a key of a `:string` selector over an enum field that is not one of the enum's variant
  names; and a language function used in a catalog of a language it does not serve.

**Plural selection is checked both ways.** A selector of `:integer` or `:number` selects by its `select` option,
whose value is a literal (above): `plural`, the default, selects by CLDR's cardinal categories of the catalog's
language; `ordinal` by its ordinal categories; `exact` by the number alone. At the pinned CLDR (UTS #35 Part 3), a
category is reachable for a selector when its rule has a sample of the operand's kind: an `@integer` sample for
`:integer`, an `@integer` or `@decimal` sample for `:number`. The build refuses:

- a category key that is not reachable (`one` in a `ko` cardinal selector; `few` is reachable in an `en` ordinal
  one, and is accepted there);
- a category key under `select=exact`, and a numeric key the operand cannot equal (a fraction under `:integer`);
- a reachable category with no variant: for every combination of reachable categories, one per selector, a variant
  matches it in which each key is that category or, for `other` alone, the catch-all `*`. A Polish catalog whose
  cardinal selector has `one` and `*` and no `few` or `many` is refused, not written with its `few` forms falling into
  `*`. A numeric key covers no category.

The diagnostic names the catalog, its language, the case, the selector and the category. An entry a catalog inherits
along its chain (D2) is checked under each language that reaches it, since two languages of one chain can have two
sets of categories.

A catalog is whole over its chain or it is refused: there is no fallback to another language at run time, since the
acceptance is that a missing entry fails generation. An argument the catalog does not use is not refused: a language
may say without it what another says with it.

### D4. The functions are a closed registry, and a language function is one of them

Each function is a row: its name, the argument types it takes, its options with the values each accepts, and what it
writes. A function not in the registry is `validation/message-argument`.

| Function | Takes | Writes |
|---|---|---|
| `:string` | a text field; an enum field, in a selector only | the text as it is; selection by exact match on the variant's declared name |
| `:integer` | an integer field | the number in the language's digits and grouping; selection by the integer (D3) |
| `:number` | an integer or a real field | the number, with the options the specification defines that the pin serves; selection (D3) |
| `:sce:date` | a `record:CivilDate`, a `record:CivilDateTime` or a `record:Instant` | the date in the language's pattern of the chosen length |
| `:sce:time` | a `record:CivilDateTime` or a `record:Instant` | the time in the language's pattern of the chosen length and precision |
| `:sce:datetime` | a `record:CivilDateTime` or a `record:Instant` | the date and the time, as the two above |
| `:sce:particle` | any field another function formats, Korean catalogs only | the operand's text followed by the particle its final sound takes |

- **The date and time functions are SCE's own.** MessageFormat 2's `:date`, `:time` and `:datetime` are Draft in
  LDML 47, and their options have changed between revisions of the specification (`fields`, `length`). A catalog
  checked against a Draft's options would change its meaning with the revision. SCE names its own, in the `sce`
  namespace the specification reserves for custom functions, with options this record fixes:
  - `length`: `long`, `medium` (the default) or `short`, read as CLDR's date and time format lengths at the pin
    (UTS #35 Part 4, "Dates"), on all three;
  - `precision`: `minute` (the default) or `second`, on `:sce:time` and `:sce:datetime`.
  A change of these options is a revision of this registry, decided by a record. When MessageFormat 2's functions are
  Stable, a record may map them onto these rows.
- **An instant is a type, not an `int64`.** `sce:std/time/instant.scxml` (new) is a record of one field,
  `utc_ms`, the `utc_ms` of `clock_now_result` (SCE_HOST_FEATURES.md section 2). An instant is formatted in the
  presentation zone (D5). A number is not formatted as a date because a field is named like one.
- **A civil date-time is a type.** `sce:std/time/civil_date_time.scxml` (new) is a record of `year` (`int32`, as
  `civil_date.scxml` holds it), `month`, `day`, `hour`, `minute` and `second`: a proleptic Gregorian date and a time
  on a wall clock, with no zone. It is written as it is, in no zone. A machine that must speak of the time of a zone
  other than the user's sends the civil date-time it means (Open question 4).
- **A placeholder without a function takes its type's.** `{$x}` is read as `{$x :string}` for a text field,
  `:integer` for an integer, `:number` for a real, `:sce:date` for a `CivilDate`, and `:sce:datetime` for a
  `CivilDateTime` and an `Instant`. An enum field placed without a selector is refused: its variant's name is a
  program's identifier, not a word.
- A `bool`, a byte string, an optional value, a list, a record other than the three above, and a union are taken by no
  function, so a catalog that places one is refused: a document that wants one sends the text or the enum it means.
- `:string`, `:integer` and `:number` read their options as the specification at the pinned LDML version defines
  them, and the registry lists each option it admits with its values. Markup is refused (D3): how a span is shown is
  the user interface toolkit's, and a record of its own decides it when one needs it.
- **`:sce:particle`** (option `pair`: `topic`, `subject`, `object`, `with`, `toward`) chooses between the two forms
  of a Korean particle by the final sound of the word before it, and writes the word and the particle. The operand is
  first formatted by its type's function, and the final sound is read from the text that function wrote:
  - the text ends in a Hangul syllable (U+AC00 to U+D7A3): its final consonant index, (code point - 0xAC00) mod 28
    (The Unicode Standard, section 3.12, "Conjoining Jamo Behavior"); 0 is none, and 8 is rieul;
  - the text ends in a number, a run of ASCII digits with the grouping and decimal separators of the `ko` number
    format at the pin: its reading in Sino-Korean numerals. Digits after the decimal separator are read one by one,
    so the last word is the last digit's name (yeong for 0). Without a decimal separator, the last word is the last
    non-zero digit's name when the units digit is not zero, and otherwise the unit of the lowest non-zero position
    (ten, a hundred, a thousand, and above them the unit of the lowest non-zero group of four digits: man, eok, jo,
    gyeong);
  - the text ends in anything else (a Latin letter, a percent sign, a symbol): the reading is not known, and both
    forms are written, the one after a consonant first and the other in parentheses, as Korean typesetting does where
    it cannot know.
  The consonant form follows a final consonant; the vowel form follows none; for `toward`, a final rieul takes the
  vowel form. The particle texts are the function's own, so no catalog writes them.

A language function is added as a row with its languages, its operand types and its test vectors; an application does
not register one at run time, so every catalog of a build is checked against one registry.

### D5. Formatting is the host's, at the edge where text meets a person

- **The build compiles each catalog.** For each case and each supported language it writes a plan: the entry the
  language's chain reaches (D2), its literal parts, each placeholder with its resolved function and literal options,
  and each selection with its variants in the order the specification's pattern-selection algorithm prefers them. The
  plan is checked once (D3) and carries no text to parse at run time, and no choice of catalog is left to run time.
- **The runtime formats a plan.** `sce-rust-text` holds the functions of D4 over ICU4X (`icu_decimal`,
  `icu_plurals`, `icu_datetime` with their compiled data, D6) and formats a value of the message set: it selects the
  plan of the language and the case, then the variant, and writes the text with the formatting data of the language,
  so an `en-GB` sentence taken from the `en` catalog writes its dates as British English does.
- **Placeholders are isolated by MessageFormat 2's Default Bidi Strategy** (UTS #35 Part 9) with the isolates of
  UAX #9. The message's direction is the catalog language's. A formatted value whose direction is known is enclosed
  in LRI or RLI by that direction, a value whose direction is not known in FSI, each closed by PDI; the strategy
  states when a left-to-right value in a left-to-right message is written without isolates, and the implementation
  follows its text at the pinned LDML version. A right-to-left name does not reorder the sentence around it.
- **The application supports a set of languages, and chooses one.** deploy.yaml names them, as it names the other
  options every document of a run is generated under (ADR 0094 §1): `languages:` with `supported`, a list of
  canonical BCP 47 tags, and `default`, one of them. Every message set the build carries is complete for every
  supported language over its chain, and a language whose chain reaches no catalog of a message set, the default's
  included, is `validation/message-catalog` (D3). The host gives the formatter the user's preferred languages, which
  it reads from its platform; RFC 4647 Lookup runs once per application over the supported set, ending at the default,
  and every message the application shows is formatted in the one language it chose. A screen that shows messages of
  two message sets never shows two languages.
- **The presentation zone is the host's.** It chooses it as it chooses the language: its IANA zone and the offsets
  `sce:std/time/utc_offset_at` reads (SCE_HOST_FEATURES.md section 2). An `Instant` is formatted in it; a
  `CivilDateTime` is written as it is.
- **Logic never receives a language.** No expression reads a language; no host operation answers a formatted text or
  a language to a statechart; a saved state holds the message (the value of the message set) and never its text. A
  change of the user's language shows the same state in the new language with no event.
- **Catalogs and the formatter are presentation, outside every machine's model** (ADR 0156 D1 and D2). A machine's
  model holds the value of the message set it raises, as it holds any union or enum value; the catalogs, the plans
  and the formatter are below the seam where text meets a person, and no judgement of a machine reads them.

SCE_HOST_FEATURES.md gains a new section, "Formatting at the edge", numbered when it lands, that states this, beside
the operations a statechart invokes: it is the one feature a host performs without being asked by a machine.

### D6. Unicode and CLDR are one pinned input of the build

- **The data.** ICU4X's compiled data, one version for every `icu_*` crate, required exactly (`=`) in the
  workspace's `[workspace.dependencies]`. A file `unicode/pin.toml` names the Unicode version, the CLDR version, the
  ICU4X version, and the SHA-256 of each conformance file the tests read, which are kept under `unicode/`.
- **One version per build.** `sce-build` reads the same data to check plural keys and parent locales (D2, D3) as the
  runtime formats with, and ADR 0149's character classes and normalisation read it too, so a build has one Unicode
  version.
- **The pin is an input, by an amendment of ADR 0094 and ADR 0097.** ADR 0094 §1 leaves the generator out of the
  input closure and admits two entries that are not documents, `deploy.yaml` and the limits. The plural rules and
  parent locales the build checks against are compiled into the generator, so as ADR 0094 stands, a change of the
  pin would change what a catalog is checked against and leave its source hash where it was. The pin file becomes a
  third such entry, in the closure of every document that holds a catalog or reads Unicode data (ADR 0149), and an
  input of ADR 0097 §2's action, so the cache key digests it. The unit declares `UNICODE_VERSION` and
  `CLDR_VERSION`, and `sce-rust-text` asserts at compile time (a constant assertion) that they are its own, so a
  catalog is never formatted under data it was not checked against.
- **A change of the pin is one change**: the crates, the pin file, the conformance files and every regenerated tree,
  judged by every test of this record.

Whichever of this record and ADR 0149 lands first lands the pin.

### D7. Logic orders text by its bytes; a collation is the host's

- **Logic orders text by its UTF-8 bytes**, which is the order of its code points. The order of a store's keys, of a
  map's keys, of a codec's entries, and of any texts a machine sorts is that order, as today. No name is added to the
  expression vocabulary.
- **Why no collation in logic.** UTS #10, section 1.8, "What Collation is Not": a collation order is not fixed, it
  changes between versions of the data, and a collation must be versioned. An order a machine saved, and an order two
  machines computed under two pins, would change with the pin, and a test of it would depend on the pin. At a
  strength below identical two different texts compare equal, so a sort by a collation is not a function of the texts
  alone. Code point order has none of these faults: it is fixed by the encoding, total, and the same on every lane.
- **A person's order is presentation.** A list shown to a person in the order of the user's language (Swedish sorts
  some letters after z) is sorted where it is shown, by the host, with the language Lookup chose (D5). `sce-rust-text`
  gives the host that collator, over `icu_collator` and the pinned data (CLDR's root collation and the language's
  tailoring, UTS #35 Part 5), so a list is shown in one order on native and wasm32. A machine never calls it, and no
  order it computes is saved.

### D8. Lanes

The lanes follow the owner's scope rule of 2026-11-22, not ADR 0109 D0: a facility is built where a consumer needs it,
and the consumer of formatting is the Rust lane, native and wasm32. `message-catalog` units, the formatter, its
collator and the formatting of `Instant` and `CivilDateTime` are built for Rust. A build for any other backend of a
catalog document is refused by name with a new code, `codegen/text-facility-not-served`, naming the facility, the
lane and the lanes that serve it (the same code ADR 0149 uses). The C++ client host, for which ADR 0109 D0 builds its
facilities, is one of the lanes that refuse: it formats no message until a consumer needs it there, in a step of its
own. A machine on any lane may still raise messages: a value of a union or an enum crosses every lane already; only
formatting is Rust's.

### Not in scope

- A host operation that takes a message (a notification whose text is a message). A `sce:request` record cannot hold
  a union (ADR 0092 §1.1, item 3: the `params` carrier has no object); it is a record of its own with that carrier.
- Catalogs loaded at run time (a translation shipped without a build). It is a record of its own: a signed catalog
  package, checked at load by the same checker as D3.
- A zone a catalog names for a time (MessageFormat 2's `timeZone` option): the zone is the host's (D5), and a time of
  another zone is a `CivilDateTime`.
- Translation exchange formats (XLIFF 2, gettext PO): an import and an export of `message-catalog` documents.
- Grammatical gender and case beyond the particles of D4, list formatting, units and currencies, and parsing what a
  person types.
- Any change to `text-template`.

## Consequences

- A message added to the message set without its sentence in every supported language fails the build that carries
  the catalogs, and so does a sentence that names an argument the message does not have, formats it with the wrong
  function, or leaves a plural category of its language without a variant.
- A saved state holds what was said, not how; a language switched by the user re-renders without the machine.
- A message whose arguments have a new shape needs an event schema for them; shapes are shared, so the count of
  records is the count of distinct argument shapes, not of messages. A message set none of whose messages take
  arguments is an enum, and needs no record.
- **A new message reaches an older client as an unknown variant** (ADR 0092 §8.3). A client built before a message was
  added refuses, as `unknownVariant`, the whole event that carries it. Clients that format messages are upgraded
  before the servers that send new ones, as for any union or enum that grows.
- The Rust lane carries ICU4X's data (tens to hundreds of kilobytes per component, for the languages the build names);
  a lane that does not format carries none.
- An upgrade of Unicode or CLDR is a deliberate change with a test run, never a side effect of a lock refresh; it
  moves the source hash of every catalog unit, and can move the order a host shows a list in, but never an order a
  machine computed.

## Alternatives rejected

- **Extending `text-template`** with plural selection and number formats. Its contract is that no locale enters
  (ADR 0087 §3, item 4), because a prompt must render the same bytes everywhere; a message must render differently per
  language. Two contracts in one kind would make each weaker. This was SCE's earlier answer.
- **A message as a free-standing id and a map of arguments.** It loses the type of each argument and the closed set of
  ids, which are exactly what the check of D3 reads; the union has both already.
- **ICU MessageFormat 1** (the syntax of ICU4C's and ICU4J's `MessageFormat`). It has no data model to check against,
  nests selection without a rule for its order, and quotes with apostrophes that translators break; MessageFormat 2
  replaces it in the same standard body.
- **Project Fluent.** A sound design, but not a Unicode standard and not aligned with CLDR's function set; MF2 took
  much of it and is the standard.
- **gettext.** Its id is the source-language sentence, its plural rule a C expression in each catalog's header, and
  its arguments untyped.
- **The machine formats, given a language.** The language would be state, a saved state would hold a sentence, a test
  would depend on a locale, and a language switch would need an event.
- **Each platform's own formatter** (`Intl`, Foundation, `java.text`). Each carries the CLDR version of its operating
  system, so one catalog would give different text on two machines, and the acceptance asks for the same text on the
  Rust and wasm lanes.
- **A catalog per message set of every language in one document.** Translators work per language, and a review of one
  language should not touch the others.
- **A fallback to another language for a missing entry at run time.** It hides the missing entry the acceptance asks
  to fail on. Inheritance along CLDR's parent locales (D2) is not a fallback: it is resolved and checked at build.
- **A root collation in logic** (a `collate` builtin, tertiary strength). Its order changes with the pin, so an order
  saved or computed on two pins would differ, and two different texts compare equal under it (D7; UTS #10, section
  1.8).
- **MessageFormat 2's `:date`, `:time` and `:datetime`.** They are Draft, and their options have changed between
  revisions; a catalog's meaning would move with the specification's revision (D4).
- **A choice of language per message set.** Lookup over each message set's own catalogs can choose two languages for
  one screen; one supported set and one choice per application cannot (D5).
- **A mark on a union that is a message set.** It would state twice a fact the catalog's `of` states.

## Order of work

Each step lands with a test that fails without it.

1. This record, accepted.
2. **The pin (D6).** `unicode/pin.toml`, the exact requirements, the conformance files with their digests, and the
   amendments of ADR 0094 §1 and ADR 0097 §2. Tests: `the_unicode_pin_names_the_data_the_lock_holds`
   (`sce-build/tests`): it reads the pin and `Cargo.lock` and requires every `icu_*` crate at the pinned version and
   every conformance file at its digest; without the pin file there is nothing to read and it fails.
   `a_catalog_units_source_hash_moves_with_the_unicode_pin`: a catalog generated under two pin files differs in its
   source hash and in its action's cache key; without the pin in the closure both stay.
3. **The MessageFormat 2 reader** (`sce-build/src/forge/message_format.rs`: the grammar and the data-model errors).
   Test: `a_message_is_read_by_the_messageformat_2_grammar`, over the specification's published test suite vendored at
   the pinned version (its syntax, syntax-error and data-model-error cases, Duplicate Variant among them); without the
   reader no case is accepted.
4. **The kind and its checks (D1, D2, D3)**, the three codes and their appendix rows. Test:
   `a_catalog_that_misses_a_case_or_mistypes_an_argument_fails_generation`, one fixture per refusal of D3 (a case with
   no entry on its chain, an entry naming no case, `$cuont` for `$count`, `:integer` on a text field, the key `one`
   in a `ko` cardinal selector, a Polish cardinal selector without `few`, a category key under `select=exact`,
   `:sce:particle` in an `en` catalog, markup, an open enum as a message set, a supported language with no catalog),
   each asserting its code and the case it names; without the checks each fixture generates. And the acceptances
   beside them: `few` in an `en` ordinal selector, an `en-GB` catalog that holds only what differs from `en`, an enum
   message set. The `ko` fixtures carry Hangul and are declared in the registry of
   `non_latin_prose_declares_why_it_is_here`.
5. **`sce-rust-text` (D4, D5).** Tests: `every_plural_sample_of_the_pinned_cldr_selects_its_category` (each
   `@integer` and `@decimal` sample of CLDR's cardinal and ordinal rules, for every language the pin carries, selects
   the category its rule names); `a_particle_follows_the_final_sound_of_its_word` (a table of vectors: a syllable
   without a final consonant, with one, with rieul under `toward`, the numbers 1, 2, 10, 20, 100, 105, 1000, 20000 and
   1000000 as the `ko` format writes them, 3.5 and 2.0, a percent, a `ko` date, and a Latin word);
   `placeholders_are_isolated_by_the_default_bidi_strategy` (a right-to-left, a left-to-right and an undetermined value
   in a left-to-right and a right-to-left message, against the isolates the strategy names);
   `a_formatted_message_never_takes_the_fallback_path` (every case of the fixture catalogs at argument vectors formats
   without the specification's fallback representation).
6. **The Rust unit of a catalog and the negotiation (D5, D6).** Tests, in `backends/rust/scenarios`:
   `the_same_catalog_gives_the_same_text_on_native_and_wasm32`: the fixture catalogs formatted at fixed argument
   vectors, languages and zone, written to one golden file, compared byte for byte on native and under the
   `wasm32-scenarios` gate; without the unit there is no text to compare.
   `one_language_is_chosen_for_every_message_set`: two message sets whose catalogs cover the supported set, a
   preference list that Lookup resolves to one language, both formatted in it.
7. **The host's collator (D7).** Test: `every_line_of_the_cldr_collation_test_sorts_after_the_one_before`, over the
   root collation test files CLDR publishes (`CollationTest_CLDR_NON_IGNORABLE` and `CollationTest_CLDR_SHIFTED`,
   each with its settings), on native and wasm32.
8. **The refusals (D8)** and `codegen/text-facility-not-served` with its row. Test:
   `a_message_catalog_is_refused_by_name_on_a_lane_that_does_not_serve_it`, one assertion per backend, C++ included.
9. The documents named in "Amends".

## Open questions

Each answered on acceptance, with its reason.

1. **Is a message set marked?** No: a union, or a closed enum, is a message set because a catalog's `of` names it
   (D1, D2). A mark would be a second statement of a fact the catalog already states (ADR 0092 §1.1: one fact has one
   spelling).
2. **What happens to an unreachable plural key, and to a reachable category with no variant?** The first is refused,
   and the second is refused (D3), judged by the categories of the selector's `select` at the pinned CLDR (UTS #35
   Part 3). Refusing what CLDR cannot reach is SCE's rule, stricter than the specification's; it catches a
   translator's mistake. Requiring every reachable category is gettext's `msgfmt --check` rule, made exact.
3. **Is there a fallback language for a missing entry?** Not at run time (D3). A catalog inherits along its CLDR
   parent locales (UTS #35 Part 1), so `en-GB` may hold only what differs from `en`, and completeness is checked over
   the chain at build (D2).
4. **Who chooses the time zone of a time?** The host, as it chooses the language (D5). `:sce:time` and
   `:sce:datetime` also take a `CivilDateTime` (D4), so a machine that must speak of another zone's time sends the
   civil date-time it means, and it is written as it is.
5. **How are bidirectional texts isolated?** By MessageFormat 2's Default Bidi Strategy with the isolates of UAX #9,
   the message's direction being the catalog language's (D5).
6. **Where do catalogs come from at run time?** They are compiled in, as gettext's `.mo` files and ICU's resource
   bundles are built ahead of time (D5); a catalog loaded at run time is a record of its own.

## Amended on acceptance

Each amendment comes from the independent review of this record (2026-11-28); the numbers are the review's.

1. A message set whose messages take no arguments is a closed enum, and `of` names an enum or a union (D1, D2; ADR
   0092 §1.1 item 1).
2. `collate` is removed from logic: logic orders text by its UTF-8 bytes, and a collation is the host's presentation
   step (D7; UTS #10 section 1.8).
3. Plural selection is checked both ways, by the selector's `select`, and Duplicate Variant is listed (D3; UTS #35
   Part 3 and Part 9; gettext's `msgfmt --check`).
4. The date and time functions are SCE's own, `:sce:date`, `:sce:time` and `:sce:datetime`, with fixed options (D4;
   MessageFormat 2's are Draft in LDML 47).
5. `:sce:particle` reads the final sound from the formatted text, digits after a decimal separator included (D4).
6. Bidirectional isolation is MessageFormat 2's Default Bidi Strategy, LRI, RLI or FSI by the value's direction, not
   first-strong isolates throughout (D5; UTS #35 Part 9; UAX #9).
7. One Lookup per application over one supported set every message set covers, declared in deploy.yaml, with
   `validation/message-catalog` for a missing catalog (D5, D3; RFC 4647).
8. The pin file becomes an input of ADR 0094 §1's closure and of ADR 0097 §2's action, and both are amended (D6,
   Amends).
9. The lanes follow the owner's scope rule of 2026-11-22, not ADR 0109 D0, and the C++ client host refuses formatting
   (Scope, D8).
10. A new message reaches an older client as `unknownVariant`, so formatting clients upgrade first (Consequences; ADR
    0092 §8.3).
11. The Amends list numbers no section or kind that is taken: a new kind, and a new section of SCE_HOST_FEATURES.md
    numbered when it lands (Amends, D5).
12. The record is ADR 0158 (the title line; the file's name and the citations of ADR 0149 and ADR 0150 move in the
    renumbering change).
13. Open questions 3 and 4 are answered as the review answers them: a CLDR parent-locale chain checked at build (D2),
    and a civil date-time record (D4); questions 1, 2, 5 and 6 keep their answers, extended as above.

## Sources

- The Unicode Consortium, Unicode Technical Standard #35, Unicode Locale Data Markup Language (LDML), version 47,
  2025: Part 9, MessageFormat (the syntax, the data-model errors, the Default Bidi Strategy, the Draft status of the
  date and time functions); Part 3, Numbers, "Language Plural Rules"; Part 4, Dates (the format lengths); Part 5,
  Collation; Part 1, Core, "Unicode Language and Locale Identifiers" and the parent locales.
- The Unicode Consortium, Unicode Technical Standard #10, Unicode Collation Algorithm, revision for Unicode 16.0, 2024:
  section 1.8, "What Collation is Not", and the conformance test.
- The Unicode Consortium, Unicode Standard Annex #9, Unicode Bidirectional Algorithm (isolates).
- The Unicode Standard, section 3.12, Conjoining Jamo Behavior (the arithmetic of a Hangul syllable).
- RFC 5646, Tags for Identifying Languages; RFC 4647, Matching of Language Tags (section 3.4, Lookup).
- ICU4X 2 (the Unicode Consortium's Rust internationalisation components) and its compiled data.
- U. Drepper, J. Meyering, F. Pinard and B. Haible, GNU gettext utilities, Free Software Foundation, manual of
  gettext 0.22, 2023: "The Format of PO Files" and `msgfmt --check`; Project Fluent's syntax guide.
- ADR 0044, ADR 0087 §3, ADR 0092 (§1.1, §8.3), ADR 0094 §1, ADR 0097 §2, ADR 0109 D0, ADR 0117, ADR 0156 D1 and D2
  in this repository.
