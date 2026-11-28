# ADR 0158 — A message is a case of a union declared beside the logic that raises it, a catalog per language holds it in MessageFormat 2 and is checked when generated, and only the host formats it, over pinned Unicode and CLDR data

- Status: Proposed (2026-11-27), for the owner's acceptance. Written under the owner's standing rule: the most
  long-term correct structure, textbook in its foundations, cost ignored; decided, not asked. Each open question is
  decided below by the textbook rule, and the acceptance may overturn any of them.
- Date: 2026-11-27
- Numbered 0148 when first written; renumbered on 2026-11-28 because that number also named ADR 0148, "A machine
  whose document leaves Appendix D's main loop one step per event takes a reactive turn", an accepted record the
  runtimes' code cites.
- Amends, once accepted (each amended document gains an "Amended by: ADR 0158" line in the landing that moves it):
  `docs/SCE_ACCEPTED_SUBSET.md` §2.1 (a thirtieth kind, `message-catalog`) and its appendix (the codes of D3 and D8);
  `SCE_ERROR_CONTRACT.md` (the same codes); `docs/SCE_HOST_FEATURES.md` (a new section 16, formatting at the edge,
  D5); the comment of `stdlib/text/utf8_length.scxml` that says Unicode's tables are carried into no backend (D6).
- Scope: the contract names no language. Built first: `sce-build` (the kind, the MessageFormat 2 reader and every
  check of D3) and a new Rust crate `sce-rust-text` (formatting, selection, collation), on native targets and on
  `wasm32-unknown-unknown`, which is the Rust lane (ADR 0117). Every other backend refuses by name (D8).
- Related: ADR 0044 (an enum that grows), ADR 0050 (a bound read by name), ADR 0087 §3 (`text-template`: no locale
  enters), ADR 0092 (the union), ADR 0094 and ADR 0097 (what a generated unit's identity covers), ADR 0109 D0 (a new
  facility is built for the lanes a consumer needs), ADR 0117 (wasm32 is the Rust lane), ADR 0149 (it shares the pin of
  D6).

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
  data-model errors an implementation reports before formatting.
- **Plural selection is CLDR's** (UTS #35 Part 3, "Language Plural Rules"): each language has its categories among
  `zero`, `one`, `two`, `few`, `many`, `other`, defined by rules over the number's digits, and CLDR gives sample
  numbers for each rule.
- **A language is a BCP 47 tag** (RFC 5646), and the language shown is chosen from the user's preferences and the
  catalogs available by RFC 4647's Lookup, ending at a default.
- **Text is ordered by the Unicode Collation Algorithm** (UTS #10) with CLDR's root collation and a language's
  tailoring (UTS #35 Part 5); its conformance is a file of strings in ascending order.
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
- Each `<sce:message case>` names a case of the union and holds one MessageFormat 2 message, simple or complex, as
  its XML character data after XML's own end-of-line handling and entity expansion, nothing trimmed: the same rule as
  a `text-template`'s text (ADR 0087 §3, item 2). A variable `$x` names the field `x` of the case's record.
- One union has any number of catalogs, at most one per language.

### D3. A catalog is checked against its message set when it is generated

The build reads every entry by the grammar of UTS #35 Part 9 and reports its data-model errors there, not when a
person reads the text. Three new codes, each with its appendix row (`acceptance_doc_covers_every_code`):

- **`validation/message-catalog`**: a case of the union with no entry, an entry naming no case, two entries for one
  case, a language tag that is not well-formed or not canonical, and two catalogs of one union with one language. The
  diagnostic names the catalog, its language and the case.
- **`validation/message-syntax`**: an entry that the grammar does not accept, and every data-model error the
  specification lists (a duplicate declaration, a duplicate option, a variant whose keys do not match the selectors, a
  missing fallback variant, a selector without a function), and markup (`{#b}`, D4).
- **`validation/message-argument`**: a variable that is neither a field of the case's record nor declared in the
  message (the specification's "Unresolved Variable", made a build error); a function that is not in the registry of
  D4; a function applied to a field of a type it does not take; an option the function does not define, or whose value
  is not a literal it accepts; a selector key that can never be chosen (a key of a `:string` selector over an enum
  field that is not one of the enum's variant names, and a key of a `:number` or `:integer` selector that is neither
  a number nor a plural category of the catalog's language at the pinned CLDR, as `one` is not in Korean); and a
  language function used in a catalog of a language it does not serve.

A catalog is whole or it is refused: there is no fallback to another language for a missing entry, since the
acceptance is that a missing entry fails generation. An argument the catalog does not use is not refused: a language
may say without it what another says with it.

### D4. The functions are a closed registry, and a language function is one of them

Each function is a row: its name, the argument types it takes, its options with the values each accepts, and what it
writes. A function not in the registry is `validation/message-argument`.

| Function | Takes | Writes |
|---|---|---|
| `:string` | a text field; an enum field, in a selector only | the text as it is; selection by exact match on the variant's declared name |
| `:integer` | an integer field | the number in the language's digits and grouping; plural selection by the integer |
| `:number` | an integer or a real field | the number, with the options the specification defines that the pin serves; plural or exact selection |
| `:date` | a `record:CivilDate` (`sce:std/time/civil_date.scxml`) or a `record:Instant` | the date in the language's pattern |
| `:time`, `:datetime` | a `record:Instant` | the time, or the date and the time, in the presentation zone (D5) |
| `:sce:particle` | any field another function formats, Korean catalogs only | the operand's text followed by the particle its final sound takes |

- **An instant is a type, not an `int64`.** `sce:std/time/instant.scxml` (new) is a record of one field,
  `utc_ms`, the `utc_ms` of `clock_now_result` (SCE_HOST_FEATURES.md section 2). A number is not formatted as a date
  because a field is named like one.
- **A placeholder without a function takes its type's.** `{$x}` is read as `{$x :string}` for a text field,
  `:integer` for an integer, `:number` for a real, `:date` for a `CivilDate` and `:datetime` for an `Instant`. An
  enum field placed without a selector is refused: its variant's name is a program's identifier, not a word.
- A `bool`, a byte string, an optional value, a list, a record other than the two above, and a union are taken by no
  function, so a catalog that places one is refused: a document that wants one sends the text or the enum it means.
- The default functions read their options as the specification at the pinned version defines them. Markup is refused
  (D3): how a span is shown is the user interface toolkit's, and a record of its own decides it when one needs it.
- **`:sce:particle`** (option `pair`: `topic`, `subject`, `object`, `with`, `toward`) chooses between the two forms
  of a Korean particle by the final sound of the word before it, and writes the word and the particle. The final sound
  is read from the operand as it is written:
  - the last character a Hangul syllable (U+AC00 to U+D7A3): its final consonant index, (code point - 0xAC00) mod 28
    (The Unicode Standard, section 3.12, "Conjoining Jamo Behavior"); 0 is none, and 8 is rieul;
  - the operand a number: its reading in Sino-Korean numerals, whose last word is the last non-zero digit's name when
    the units digit is not zero, and otherwise the unit of the lowest non-zero position (ten, a hundred, a thousand,
    and above them the unit of the lowest non-zero group of four digits: man, eok, jo, gyeong);
  - anything else (a Latin letter, a symbol): the reading is not known, and both forms are written, the one after a
    consonant first and the other in parentheses, as Korean typesetting does where it cannot know.
  The consonant form follows a final consonant; the vowel form follows none; for `toward`, a final rieul takes the
  vowel form. The particle texts are the function's own, so no catalog writes them.

A language function is added as a row with its languages, its operand types and its test vectors; an application does
not register one at run time, so every catalog of a build is checked against one registry.

### D5. Formatting is the host's, at the edge where text meets a person

- **The build compiles each catalog.** For each case it writes a plan: the literal parts, each placeholder with its
  resolved function and literal options, and each selection with its variants in the order the specification's
  pattern-selection algorithm prefers them. The plan is checked once (D3) and carries no text to parse at run time.
- **The runtime formats a plan.** `sce-rust-text` holds the functions of D4 over ICU4X (`icu_decimal`,
  `icu_plurals`, `icu_datetime`, `icu_collator` with their compiled data, D6) and formats a value of the union: it
  selects the language, the case and the variant, and writes the text. Placeholders are isolated by the
  specification's default bidirectional strategy (first-strong isolates, UAX #9), so a right-to-left name does not
  reorder the sentence around it.
- **The language is the host's.** The host gives the formatter the user's preferred languages, which it reads from its
  platform, and RFC 4647 Lookup chooses among the catalogs the build carries, ending at the language the deployment
  names as its default (a catalog of which is required). The presentation zone of `:time` and `:datetime` is the
  host's too: its IANA zone and the offsets `sce:std/time/utc_offset_at` reads (SCE_HOST_FEATURES.md section 2).
- **Logic never receives a language.** No expression reads a language; no host operation answers a formatted text or
  a language to a statechart; a saved state holds the message (the union value) and never its text. A change of the
  user's language shows the same state in the new language with no event.

SCE_HOST_FEATURES.md gains a section 16, "Formatting at the edge", that states this, beside the operations a
statechart invokes: it is the one feature a host performs without being asked by a machine.

### D6. Unicode and CLDR are one pinned input of the build

- **The data.** ICU4X's compiled data, one version for every `icu_*` crate, required exactly (`=`) in the
  workspace's `[workspace.dependencies]`. A file `unicode/pin.toml` names the Unicode version, the CLDR version, the
  ICU4X version, and the SHA-256 of each conformance file the tests read, which are kept under `unicode/`.
- **One version per build.** `sce-build` reads the same data to check plural keys (D3) as the runtime formats with,
  and ADR 0149's character classes and normalisation read it too, so a build has one Unicode version.
- **The pin is an input.** The source hash of a generated catalog unit (ADR 0094) covers the pin file; the unit
  declares `UNICODE_VERSION` and `CLDR_VERSION`, and `sce-rust-text` asserts at compile time (a constant assertion)
  that they are its own, so a catalog is never formatted under data it was not checked against.
- **A change of the pin is one change**: the crates, the pin file, the conformance files and every regenerated tree,
  judged by every test of this record.

Whichever of this record and ADR 0149 lands first lands the pin.

### D7. Two orders of text: the root collation for logic, a language's for display

- **The root collation is a pure function any logic may call.** `collate(a, b)` answers `-1`, `0` or `1` by CLDR's
  root collation (UTS #35 Part 5, the root of UTS #10's tables as CLDR modifies them), tertiary strength, alternate
  non-ignorable. It takes no language, so a machine that sorts a list by it gives one order on every lane that serves
  it, for one pin. It is added to the expression vocabulary beside `len`, `eq`, `round` and `floor`
  (`EXPR_BUILTINS`, `sce-build/src/forge/expr.rs`).
- **A language's order is presentation.** A list shown to a person in the order of the user's language (Swedish
  sorts some letters after z) is sorted by the host's formatter with that language's tailoring. A machine does not.
- **Keys stay bytes.** The order of a store's keys, of a map's keys and of a codec's entries is the order of their
  UTF-8 bytes, as today; a collation is never a key's order.

### D8. Lanes

`message-catalog` units, `collate` and the `instant` formatting are built for Rust, native and wasm32. A build for
any other backend of a catalog document, and of a machine that calls `collate`, is refused by name with a new code,
`codegen/text-facility-not-served`, naming the facility, the lane and the lanes that serve it (the same code ADR 0149
uses). A machine on any lane may still raise messages: a union value crosses every lane already; only formatting is
Rust's.

### Not in scope

- A host operation that takes a message (a notification whose text is a message). A `sce:request` record cannot hold
  a union (ADR 0092 §1.1, item 3: the `params` carrier has no object); it is a record of its own with that carrier.
- Catalogs loaded at run time (a translation shipped without a build). It is a record of its own: a signed catalog
  package, checked at load by the same checker as D3.
- Translation exchange formats (XLIFF 2, gettext PO): an import and an export of `message-catalog` documents.
- Grammatical gender and case beyond the particles of D4, list formatting, units and currencies, and parsing what a
  person types.
- Any change to `text-template`.

## Consequences

- A message added to the union without its sentence in every language fails the build that carries the catalogs, and
  so does a sentence that names an argument the message does not have or formats it with the wrong function.
- A saved state holds what was said, not how; a language switched by the user re-renders without the machine.
- A message whose arguments have a new shape needs an event schema for them; shapes are shared, so the count of
  records is the count of distinct argument shapes, not of messages.
- The Rust lane carries ICU4X's data (tens to hundreds of kilobytes per component, for the languages the build names);
  a lane that does not format carries none.
- `collate` becomes a vocabulary name, so a document that declares a callable named `collate` is refused where it
  declares it (`crate::forge::namespace`), as for the four builtins today.
- An upgrade of Unicode or CLDR is a deliberate change with a test run, never a side effect of a lock refresh.

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
- **A fallback to another language for a missing entry.** It hides the missing entry the acceptance asks to fail on.

## Order of work

Each step lands with a test that fails without it.

1. This record, accepted.
2. **The pin (D6).** `unicode/pin.toml`, the exact requirements, the conformance files with their digests. Test:
   `the_unicode_pin_names_the_data_the_lock_holds` (`sce-build/tests`): it reads the pin and `Cargo.lock` and
   requires every `icu_*` crate at the pinned version and every conformance file at its digest. Without the pin file
   there is nothing to read and it fails.
3. **The MessageFormat 2 reader** (`sce-build/src/forge/message_format.rs`: the grammar and the data-model errors).
   Test: `a_message_is_read_by_the_messageformat_2_grammar`, over the specification's published test suite vendored at
   the pinned version (its syntax, syntax-error and data-model-error cases); without the reader no case is accepted.
4. **The kind and its checks (D2, D3)**, the three codes and their appendix rows. Test:
   `a_catalog_that_misses_a_case_or_mistypes_an_argument_fails_generation`, one fixture per refusal of D3 (a case with
   no entry, an entry naming no case, `$cuont` for `$count`, `:integer` on a text field, the key `one` in a `ko`
   catalog, `:sce:particle` in an `en` catalog, markup), each asserting its code and the case it names; without the
   checks each fixture generates. The `ko` fixtures carry Hangul and are declared in the registry of
   `non_latin_prose_declares_why_it_is_here`.
5. **`sce-rust-text` (D4, D5).** Tests: `every_plural_sample_of_the_pinned_cldr_selects_its_category` (each
   `@integer` and `@decimal` sample of CLDR's plural rules, for every language the pin carries, selects the category
   its rule names); `a_particle_follows_the_final_sound_of_its_word` (a table of vectors: a syllable without a final
   consonant, with one, with rieul under `toward`, the numbers 1, 2, 10, 20, 100, 105, 1000, 20000 and 1000000, and a
   Latin word); `a_formatted_message_never_takes_the_fallback_path` (every case of the fixture catalogs at argument
   vectors formats without the specification's fallback representation).
6. **The Rust unit of a catalog and the negotiation (D5, D6).** Test, in `backends/rust/scenarios`:
   `the_same_catalog_gives_the_same_text_on_native_and_wasm32`: the fixture catalogs formatted at fixed argument
   vectors, languages and zone, written to one golden file, compared byte for byte on native and under the
   `wasm32-scenarios` gate. Without the unit there is no text to compare.
7. **`collate` (D7).** Test: `every_line_of_the_cldr_collation_test_sorts_after_the_one_before`, over the root
   collation test files CLDR publishes (`CollationTest_CLDR_NON_IGNORABLE` and `CollationTest_CLDR_SHIFTED`, each
   with its settings), and a generated machine that sorts a list by `collate` on native and wasm32.
8. **The refusals (D8)** and `codegen/text-facility-not-served` with its row. Test:
   `a_message_catalog_is_refused_by_name_on_a_lane_that_does_not_serve_it`, one assertion per backend.
9. The documents named in "Amends".

## Open questions

Decided here, each with its reason; the acceptance may overturn any.

1. **Is a message set marked?** No: a union is a message set because a catalog names it (D1). A mark would be a second
   statement of a fact the catalog already states.
2. **What happens to an unreachable plural key?** It is refused (D3): a variant that can never be chosen is a
   translator's mistake, and the CLDR rules that decide it are the pinned ones.
3. **Is there a fallback language for a missing entry?** No (D3): the acceptance is that a missing entry fails.
4. **Who chooses the time zone of a time?** The host, as it chooses the language (D5); a machine that must speak of
   another zone's time sends the civil date and time it means.
5. **How are bidirectional texts isolated?** By the specification's default strategy (D5).
6. **Where do catalogs come from at run time?** They are compiled in (D5); a loaded catalog is a later record.

## Sources

- Unicode Technical Standard #35 (LDML), Part 9: MessageFormat; Part 3: Numbers, "Language Plural Rules"; Part 5:
  Collation; Part 1: Core, "Unicode Language and Locale Identifiers".
- Unicode Technical Standard #10, Unicode Collation Algorithm, and its conformance test.
- Unicode Standard Annex #9, Unicode Bidirectional Algorithm (isolates).
- The Unicode Standard, section 3.12, Conjoining Jamo Behavior (the arithmetic of a Hangul syllable).
- RFC 5646, Tags for Identifying Languages; RFC 4647, Matching of Language Tags (section 3.4, Lookup).
- ICU4X 2 (the Unicode Consortium's Rust internationalisation components) and its compiled data.
- GNU gettext manual, "The Format of PO Files" and `msgfmt --check-format`; Project Fluent's syntax guide.
- ADR 0044, ADR 0087 §3, ADR 0092, ADR 0094, ADR 0097, ADR 0109 D0, ADR 0117 in this repository.
