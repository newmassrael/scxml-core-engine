# ADR 0149 — A text is tested against a regular language by an automaton in time linear in the text, and normalised to NFC by the pinned Unicode data

- Status: Accepted (2026-11-28) under the owner's standing rule (the most long-term correct, textbook structure, cost
  ignored), by the main session the owner delegated the decision to, after an independent review; amended on
  acceptance as listed in "Amended on acceptance".
- Date: 2026-11-27
- Amends, once accepted (each amended document gains an "Amended by: ADR 0149" line in the landing that moves it):
  `SCE_FORGE.md` §3.4 (the row of regular expression literals keeps its refusal and changes its reason; the vocabulary
  gains `matches`, `search`, `regexp`, `nfc` and `is_nfc`); `docs/SCE_ACCEPTED_SUBSET.md` (the type `regexp[N]`, the
  vocabulary, the appendix rows of D2 and D3, and the paragraph of string variables, which gains the rule of D8);
  `SCE_ERROR_CONTRACT.md` (the same codes); ADR 0158 D6 (its constant assertion of the Unicode version covers the
  units of D4 as well as catalog units); the comment of `stdlib/text/utf8_length.scxml` that says normalisation is not
  provided (D5).
- Scope: the contract names no language. Built first: `sce-build` (the grammar, the compiled automata, the type) and
  a new Rust crate `sce-rust-regexp` (`no_std`, no heap), with normalisation in the Rust runtime, on native targets
  and on `wasm32-unknown-unknown`. The type `regexp[N]`, its check and the rule of D8 are built on every lane; the
  tests `matches` and `search` and the operations `nfc` and `is_nfc` are refused by name on every other lane (D6).
- Related: ADR 0050 (a bound read by name), ADR 0056 (buffers are the document's size), ADR 0067 (a table row is as
  small as its numbers), ADR 0077 (a declared bound is part of a type and every decode refuses what breaks it), ADR
  0092 §12 (every constructor of the type grammar is held on every lane), ADR 0094 (a generated file's source hash
  covers what it is generated from), ADR 0101 (a CBOR text string that is not UTF-8 is refused), ADR 0133 (one
  classification of a failure), ADR 0158 (the pin of Unicode and CLDR, D6, and the code of its D8).

## Context

A document cannot ask whether a text is of a given form (an identifier, a code, a slug) except by writing the
recogniser by hand as an algorithm, byte by byte, as `stdlib/text/utf8_length.scxml` writes RFC 3629's grammar. And
it cannot bring two spellings of one text to one.

What exists:

- **Regular expression literals are refused.** SCE_FORGE.md §3.4 lists `/pattern/` among the rejected constructs
  ("Engine-specific implementation"). The expression lexer has no token for one: `/` is `Token::Slash`
  (`sce-build/src/forge/expr.rs`, the byte match of the lexer), so `/a/` is read as a division and fails. The
  vocabulary a document calls without declaring it is `len`, `eq`, `round` and `floor` and the eight wrapping and
  saturating operations (`EXPR_BUILTINS` and `vocabulary_names` in the same file).
- **Regular expressions are only an output.** The build writes ECMA-262 regular expressions into the JSON Schema of a
  model's answer (`regex_range` in `sce-build/src/forge/value_type.rs`, ADR 0087) and checks them with the `regex`
  crate in a test of that file; `template.rs` compiles an XSD regular expression with it in a test. `regex` 1.12.3
  with `regex-automata` 0.4.14 is a dependency of `sce-build` alone (`cargo tree -i regex`), never of a generated
  machine.
- **There is no normalisation.** `stdlib/text/utf8_length.scxml` counts the code points it is given and says why it
  does not normalise: a Hangul syllable is one code point in NFC and two or three in NFD, so "a product that wants one
  cap to mean one thing writes its text in NFC (UAX #15) at the boundary where the text enters", but "Normalization is
  not a document here; it would carry Unicode's composition tables into every backend and a new table with each
  Unicode version". A key is compared as its UTF-8 bytes (SCE_HOST_FEATURES.md section 3), and Swift's canonical
  equivalence is replaced by a comparison of bytes so the lanes agree (`sce-build/src/forge/expr/swift.rs`).
- **`icu_normalizer` 2.2.0 is already in `Cargo.lock`**, reached only through `url`'s IDNA support in a test crate.
- **No record decides that a `string` is well-formed on every lane.** Most entries refuse a text that is not. Every
  JSON reader refuses half of a UTF-16 pair in a `\u` escape (`tests/json_text/payload_refusal.json`). A CBOR text
  that is not UTF-8 is refused as `InvalidUtf8` on Rust, Go, Python, C11, C++ and Kotlin (ADR 0099, ADR 0101). A
  Kotlin text value makes no value of a lone surrogate (ADR 0081). A string literal refuses a surrogate that is not
  half of a pair (`docs/SCE_ACCEPTED_SUBSET.md` §3). The Interpreter is the exception. It holds a `string` as an
  ECMAScript string of UTF-16 code units. Its typed decode (`text` in `sce-build/src/forge/static_js/sce_static.js`)
  refuses a value that is not a string or holds U+0000, and takes one that holds half of a UTF-16 pair; its
  `utf8Length` counts such a half as three bytes. A value that reaches it as an ECMAScript string, and not through
  the JSON reader, can therefore hold a code unit that is no Unicode scalar value. ADR 0133 D1 item 6 records the same
  lane's byte-string store as checked for a reason no bound rules out.
- **The word "pattern" has a meaning in the contract already.** `sce:pattern` names a communication pattern of a
  distributed deployment (`docs/SCE_ACCEPTED_SUBSET.md` §2.5). This record names its type and its constructor
  `regexp`, so one word keeps one meaning.

## The textbook structure

- **A regular language is recognised by a finite automaton** (Kleene 1956; Rabin and Scott 1959). Thompson's
  construction (1968; Aho, Lam, Sethi and Ullman, Algorithm 3.23) turns a regular expression of size m into an
  automaton of O(m) states with empty moves; the subset construction makes it deterministic; Hopcroft's algorithm
  (1971) makes the deterministic one minimal. A deterministic automaton decides membership with one transition per
  input symbol: time linear in the text, and space constant.
- **The alphabet of an automaton is a choice.** Over code points, a character class such as `\p{L}` is one transition
  that holds a set of ranges, and the size of the automaton is the size of the expression. Over UTF-8 bytes, the same
  class is a sub-automaton of byte ranges whose size follows the class's ranges, and so changes with each Unicode
  version. A deterministic table is indexed by bytes; a simulated automaton reads code points, decoding UTF-8 by
  RFC 3629's grammar as it reads.
- **Determinisation can cost states exponential in the expression**: `(a|b)*a(a|b){k}` needs 2^(k+1) states, since
  the automaton must remember the last k+1 symbols. Where that is too many, the nondeterministic automaton is simulated
  directly, a set of states advanced per symbol (Thompson 1968; the form Pike's virtual machine gives it, Cox 2009):
  time O(n*m), still linear in the text. The set is kept as a sparse set (Briggs and Torczon 1993), O(m) words, so
  each step clears it in constant time and adds a state at most once. RE2 builds the deterministic automaton within a
  budget of states and falls back to the simulation past it (Cox 2010): both are linear in the text.
- **Backtracking is not an automaton.** Perl's, PCRE's, Java's, Python's and ECMAScript's matchers search the space of
  ways to match, which is exponential in the worst case: the denial of service a crafted text causes against them is
  why linear matchers exist (Cox 2007; Cox 2010). Their syntax also goes beyond regular languages, but only by
  backreferences: a backreference makes a language that is not regular, and matching with backreferences is
  NP-complete (Aho 1990). Lookaround without backreferences keeps a language regular (Morihata 2012; Miyazaki and
  Minamide 2019).
- **An interoperable regular syntax exists**: I-Regexp (RFC 9485, 2023), a subset of XML Schema's regular expressions
  that has no backreference and no lookaround, is matched against the whole text, works on Unicode code points with
  General Category classes, and maps to every common dialect. It states interoperability as its aim (section 2), maps
  to the other dialects (section 5), and says it can be matched in linear time and advises limiting range quantifiers
  (section 8). It names no Unicode version. JSONPath (RFC 9535) uses it for its `match()` and `search()` functions
  (sections 2.4.6 and 2.4.7): the whole text in the language, and some part of it.
- **A well-formed text is a sequence of Unicode scalar values.** A scalar value is any code point but a surrogate.
  UTF-8 encodes exactly these (RFC 3629, whose section 4 gives the grammar of its byte sequences), and a UTF-16 text
  spells one with no half of a pair left alone.
- **Normalisation is defined by UAX #15** over the Unicode Character Database: NFC is canonical decomposition followed
  by canonical composition. Its conformance is `NormalizationTest.txt` of the same version. Its stability is the
  Unicode Character Encoding Stability Policy's: a text of characters assigned in one version that is normalised stays
  normalised in every later version; a code point not yet assigned can normalise differently once it is. UAX #15
  section 9 states that NFC grows a text at most three times, in UTF-8 too; the Stability Policy's guarantee on
  Decomposition_Mapping, in force since Unicode 2.0.0, keeps that bound in every later version. The same Policy adds
  no new General_Category value, so the set of names a class may use is closed.

## Decision

### D1. A regular expression is an I-Regexp, and two tests read it

- **The syntax and meaning are RFC 9485's**: its grammar (section 3) and its semantics (section 4), over Unicode code
  points; `.` is any code point but line feed and carriage return; `\p{..}` and `\P{..}` name General Category
  values and their groups. There are no flags: a case-insensitive test is a class (`[Aa]`).
- **`matches(text, r)`** is true when the whole of `text` is in the language of `r` (RFC 9535 section 2.4.6).
  **`search(text, r)`** is true when some part of `text` is (section 2.4.7). Both are `bool` and total.
- **The text is a `string`**, a sequence of Unicode scalar values in well-formed UTF-8 on every lane (D8). Both tests
  read its code points. A printed automaton (D2) reads its bytes and spells RFC 3629's grammar as `utf8_length.scxml`
  does, so its answer on a byte sequence that is not UTF-8 is false; no value of D8 is such a sequence.
- **"Some part" is surrounded by well-formed text.** For `search`, the text before and after the part is any sequence
  of scalar values in UTF-8 (RFC 3629 section 4), never any bytes. So `search` accepts no byte sequence that is not
  UTF-8, as `matches` accepts none.
- **A regular expression literal stays refused.** The row of SCE_FORGE.md §3.4 keeps `/pattern/` refused, with its
  reason stated: an ECMAScript regular expression is not regular in general (its backreferences) and is matched
  differently by each engine. A regular expression is the text of a string literal or a value of D3, read by one
  grammar.

### D2. A regular expression written in a document is compiled when the machine is built

When the second argument of `matches` or `search` is a string literal, the build:

1. reads it by RFC 9485's grammar, refusing a text that is not an I-Regexp as **`expression/invalid-regexp`** at the
   offset where the grammar stops;
2. builds Thompson's automaton over code points, each class one transition (D3), and refuses one with more states
   than the named bound `regexp_nfa_states` (library number 1024) as **`expression/regexp-too-large`**, naming the
   count and the bound; `sce-codegen --limit regexp_nfa_states=N` moves it (ADR 0050);
3. expands each class from the pinned Unicode data (ADR 0158 D6) into the byte ranges of its UTF-8 (RFC 3629), and
   builds the deterministic automaton over bytes by the subset construction: for `matches`, of the language; for
   `search`, of any well-formed text, then the language, then any well-formed text (D1). The construction stops once
   its reachable states pass the named bound `regexp_states` (library number 4096); `sce-codegen --limit
   regexp_states=N` moves it (ADR 0050). An automaton that stays within the bound is minimised (Hopcroft 1971). So
   the build's work is bounded by the two bounds, and a count is always there to report;
4. prints what step 3 gave as data. A deterministic automaton is printed as a class for each of the 256 bytes, a
   table of states by classes in the narrowest unsigned type that numbers its states (ADR 0067), and the accepting
   states; one walk of the runtime reads it, one table read per byte. An automaton whose construction stopped at the
   bound is printed as Thompson's automaton of step 2, with its classes' ranges, and D3's simulation walks it. Both
   walks have no heap and no recursion, and each is one function for every regular expression.

Past `regexp_states` nothing is refused: the build prints the form RE2 falls back to (Cox 2010), linear in the text.
Only a regular expression past `regexp_nfa_states` is refused. That is the bound a run-time value of D3 is held to,
so a text the build refuses is a text `regexp(t)` refuses, and the reverse.

A regular expression is named and shared as any computation is: an algorithm (`is_slug(text) -> bool`) holds the
literal. No element declares one.

### D3. A regular expression supplied at run time is a value of a type that carries its bound

- **`regexp[N]`** is a text of at most N bytes that is an I-Regexp whose automaton has at most `regexp_nfa_states`
  states. The automaton is Thompson's over code points, every counted repetition expanded, and each class one
  transition holding a set of ranges. The count reads the expression alone: no class's ranges enter it, and the set
  of General_Category values does not grow (the Stability Policy), so a value valid under one pin is valid under
  every later one, as ADR 0077 requires. It stands where a `string[N]` stands: a variable, an event field, an
  algorithm's parameter. It is a new constructor of the type grammar, held to every lane by ADR 0092's census
  (`every_constructor_of_the_type_grammar_is_held_on_every_lane`).
- **It is made by a check.** `regexp(t)` makes one from a text and fails when `t` is not an I-Regexp
  (`invalid_regexp`) or its automaton passes the bound (`regexp_too_large`), read as every failure is (ADR 0133).
  Every decode of a field of the type refuses such a text (ADR 0077), on every lane (D6), so no value of it is ever
  invalid; a saved state holds its text. The check reads no Unicode data: a class name is checked against the closed
  list of General Category values and their groups, and a class counts one state whatever its ranges.
- **It is matched by simulation.** `matches` and `search` with such a value build Thompson's automaton into storage
  the bound sizes, then read the text one code point at a time, decoding UTF-8 by RFC 3629's grammar in the same
  pass. Each class's transition tests the code point against the class's ranges, taken from the pinned
  General_Category data, by a binary search. A set of states is advanced per code point, each state entered at most
  once per code point (Thompson 1968; Cox 2009), the set kept as a sparse set (Briggs and Torczon 1993). `matches`
  answers whether an accepting state is in the set at the end. `search` adds the start state to the set before each
  code point and at the end (Thompson 1968), and answers true once an accepting state is in it. Time O(n*m)
  transitions with m at most `regexp_nfa_states`, space O(m) words, no heap. The build is per test, O(m), and does
  not change the bound in the text.
- `(a{1000}){1000}` is a short text with a million states: it is refused when it is made, never compiled.

### D4. One grammar and one construction, at build time and at run time

- **`sce-rust-regexp` holds** the reader of RFC 9485's grammar, Thompson's construction over code points into
  storage its caller gives, the expansion of a class into ranges of code points from `icu_properties` at the pin, the
  simulation, and the walk of a printed table. `sce-build` depends on it and adds the expansion of ranges into UTF-8
  byte ranges, the bounded subset construction and the minimisation, which only the build runs.
- **The same language by construction.** The language a build-time regular expression accepts and the one the same
  text accepts at run time rest on one reader and one expansion of classes, not on two implementations kept in step.
  A test of D7 holds the printed table to the simulation on every short byte sequence, ill-formed ones included.
- **One Unicode version per build, enforced.** Every generated unit that prints an automaton whose classes were
  expanded from the pin, or that calls `nfc`, `is_nfc`, `regexp`, or `matches` or `search` with a value of D3,
  declares `UNICODE_VERSION`, and the runtime that serves it asserts at compile time (a constant assertion) that it
  is its own, as ADR 0158 D6 does for a catalog unit. The source hash of such a unit covers the pin file (ADR 0094).
  So an automaton built from one pin never runs on a runtime built on another. On a lane whose runtime holds only the
  check of D3, the check reads no Unicode data, and there is no version to assert.

### D5. NFC, from the pinned data

- **`nfc(t)`**, for `t` a `string[B]`, is the NFC of `t` (UAX #15), a `string[3B]`. It is total: its operand is
  well-formed (D8), and NFC grows a text at most three times in UTF-8 (UAX #15 section 9), a bound the Stability
  Policy keeps in every later version, so `string[3B]` is right under every pin. **`is_nfc(t)`** is whether `t` is
  in NFC, total over the same values. Both read the pinned Unicode version (ADR 0158 D6) through `icu_normalizer`'s
  composing normaliser.
- **Where it is used is the document's.** utf8_length's advice stands: normalise at the boundary where the text
  enters, before a cap is checked or a key is made. Its comment is amended: normalisation is now a vocabulary
  operation of the lanes of D6, over data the build pins, and still not a document.
- **Equality does not change.** Two texts are equal when their bytes are; a document that means canonical
  equivalence writes `nfc(a) === nfc(b)`. Nothing is normalised without being asked: a decoded text keeps its bytes,
  which a signature or a digest may cover.
- **A change of pin is a change of answer only for code points the earlier pin did not assign** (the Stability
  Policy); ADR 0158 D6 makes the change deliberate.

### D6. Lanes

- **On every lane** (Rust, Kotlin, Python, Go, Swift, C++, C11 and the Interpreter): the rule of D8; the type
  `regexp[N]`, its decode check and `regexp(t)`. An event schema with a field of the type is decoded and checked on
  every lane as any field is, and no lane refuses it. A machine on any lane may hold, receive, send and save a value
  of the type. ADR 0092 §12 item 7 requires the constructor in a scenario every lane replays, and ADR 0158 D8 gives
  the same structure: a value crosses every lane, and only the facility that reads Unicode data is Rust's. Each
  lane's check is held to the shared refusal table (`tests/json_text/payload_refusal.json`), which gains the type's
  descriptor and a case per refusal, so the lanes agree on every text it lists.
- **On Rust, native and wasm32**: `matches` and `search`, and `nfc` and `is_nfc` (D5). A build for any other
  backend, and the Interpreter's lowering, of a machine or an algorithm that calls one is refused by name with
  `codegen/text-facility-not-served` (ADR 0158 D8).
- **A lane joins** by printing a build-time automaton and its walk, which is data any backend can print, and by the
  simulation and the normalisation with its runtime, in the change that lands its tests.

### D7. Linear time is a tested property

- **Steps are counted.** Under a test feature the walk and the simulation count what they do. A family of regular
  expressions that drive a backtracking matcher to exponential time (`(a|a)*b`, `(a*)*b`, `(a|b)*a(a|b){10}`,
  `.*.*.*x`) is run on texts of 1 to 2^20 bytes; a printed deterministic automaton takes exactly one step per byte,
  and the simulation at most m per code point. A count is exact where a clock is noise.
- **The two forms agree.** For every regular expression of the corpus, the printed deterministic automaton and the
  simulation give the same answer on every byte sequence up to a length, ill-formed UTF-8 included, where both
  answer false.
- **The language is the one RFC 9485 means.** Every regular expression of a corpus (RFC 9485's examples, the
  `match()` and `search()` cases of the JSONPath compliance test suite, and the families above) is tested against
  every text over a small alphabet up to a length, and the answer agrees with the `regex` crate given RFC 9485's
  mapping of the expression (a test-only oracle, itself linear).

### D8. A `string` value is a sequence of Unicode scalar values in well-formed UTF-8, on every lane

- **The rule.** Every value of `string[N]` on every lane is the UTF-8 (RFC 3629) of a sequence of Unicode scalar
  values: no surrogate code point and no ill-formed byte sequence. A lane that holds text as UTF-16 code units holds
  no half of a pair alone.
- **It is checked where a value enters.** Every decode of a text refuses one that breaks the rule, as each refuses a
  text past its bound (ADR 0077). A string literal refuses a surrogate that is not half of a pair. Every operation of
  the language on texts keeps the rule: a join of two such sequences is one. So `nfc`, `is_nfc`, `matches`, `search`
  and `regexp` are total over `string`, and none tests for well-formedness again.
- **Where it does not hold yet: the Interpreter** (Context). Its typed decode `text` refuses a value that holds half
  of a UTF-16 pair alone as `wrongKind`, as it refuses a value that is not a text: such a value is not a text of the
  language. Its `utf8Length` is then never given one. ADR 0133 D1 item 6 records the same lane's byte strings, whose
  store is checked for a reason no bound rules out; this rule leaves byte strings as they are. The change lands
  before the type of D3 (Order of work), so no lane refuses the type for want of it.
- **Every entry is held.** The landing adds a case per entry a lane holds to the shared tables
  (`tests/json_text/payload_refusal.json` and the CBOR vectors of ADR 0101), so a lane that admits half of a pair at
  any entry fails there.

### Not in scope

- A schema field that declares a regular expression its decode holds a text to (XSD's `pattern` facet): it changes
  every codec of every lane and the grammar a model is held to (ADR 0087), and is a record of its own when one needs
  it.
- Captures, replacement and the position of a match; flags; NFD, NFKC and NFKD; grapheme clusters (UAX #29); case
  folding.
- `matches`, `search`, `nfc` and `is_nfc` on any other lane.

## Consequences

- No text a document is handed, however crafted, makes a test take more than a step per byte (a deterministic
  table) or m steps per code point (a simulation); a document cannot write a regular expression that is not regular.
- A regular expression the author writes wrong fails the build where it is written, at the offset. A written one the
  build cannot determinise within its bound is simulated, never refused for that.
- `matches`, `search`, `regexp`, `nfc` and `is_nfc` become vocabulary names, so a document that declares a callable
  of one of those names is refused where it declares it (`crate::forge::namespace`), as for the builtins today.
- A value of `regexp[N]` crosses every lane; testing a text against it is Rust's.
- A `string` is well-formed on every lane, the Interpreter included.
- A machine that normalises carries ICU4X's normalisation data on the Rust lane; one that does not carries none.
- An automaton is part of the generated unit, so its size is the build's to report and to bound, and the unit names
  the Unicode version it was built from.

## Alternatives rejected

- **ECMAScript regular expression literals.** Backreferences make them not regular, and matching with them
  NP-complete (Aho 1990). Their lookaround alone would keep a language regular (Morihata 2012; Miyazaki and Minamide
  2019), but the engines that run them backtrack, and each is different.
- **A backtracking matcher with a step budget.** It turns a slow match into a failure; the answer to "is this text of
  this form" is then "unknown", and the bound is a guess.
- **Each platform's regular expressions.** `java.util.regex`, Python's `re`, Swift's `Regex` and ECMAScript's
  backtrack; Go's `regexp` and RE2 do not, but each has its own syntax and Unicode tables.
- **The `regex` crate at run time.** Linear, but its syntax is not I-Regexp, its Unicode tables are a second Unicode
  version in the build, and it needs a heap.
- **XML Schema's regular expressions.** Regular, but its multi-character escapes and block names are XML's; I-Regexp
  is the subset of them that is meant to be portable.
- **A run-time automaton over UTF-8 bytes.** A class's size would enter the count, and the count would change with
  the pin: a saved value could pass its own bound after an upgrade, which ADR 0077 forbids.
- **A deterministic automaton only.** At build time, a regular expression the run time accepts would be refused, and
  the subset construction would do unbounded work before any count could be checked. At run time, the automaton
  would be built while the machine runs, at a cost exponential in the worst case. Simulation is linear.
- **`regexp[N]` on Rust alone.** It breaks ADR 0092 §12 item 7. A value needs no Unicode data to be checked, so no
  lane has a reason to refuse it.
- **Checking well-formedness at each operation.** Every operation would carry a failure the type rules out once,
  where the value enters.
- **Naming the type `pattern`.** `sce:pattern` names a communication pattern already (`docs/SCE_ACCEPTED_SUBSET.md`
  §2.5).
- **Normalisation as an algorithm document.** utf8_length's reason stands: the composition tables would be written
  per backend and per Unicode version.
- **Normalising every decoded text.** It changes bytes a signature or a digest covers, and the deterministic forms of
  ADR 0101 would no longer be the bytes sent.

## Order of work

Each step lands with a test that fails without it.

1. This record, accepted.
2. **The pin** (ADR 0158, step 2), if that record has not landed it.
3. **A `string` is well-formed on every lane (D8).** The Interpreter's typed decode refuses half of a UTF-16 pair
   alone as `wrongKind`. Tests: a case in `tests/integration/AStaticDatamodelRunsLoweredUnderTheInterpreterTest.cpp`
   that hands a typed payload's text field U+D800 alone and reads the refusal; the cases of each entry in the shared
   tables, replayed by every lane's reader.
4. **`sce-rust-regexp`: the grammar, the construction and the simulation (D3, D4).** Tests:
   `an_iregexp_is_read_by_rfc_9485s_grammar` (the RFC's examples and refusals);
   `a_class_is_one_state_whatever_its_ranges` (`\p{L}` and `[a]` count alike);
   `a_regexp_agrees_with_the_regex_crate_on_every_short_text` (D7, third item);
   `search_finds_a_part_and_only_a_well_formed_text_around_it` (D1, D3);
   `no_input_makes_matching_super_linear` (D7, first item, for the simulation). Without the crate none compiles.
5. **Build-time regular expressions (D1, D2, D4)**: `matches` and `search` with a literal, the Rust template's table,
   its walk and the printed Thompson automaton, the two codes and their rows, the unit's `UNICODE_VERSION` and the
   runtime's assertion. Tests: `a_regexp_literal_is_compiled_to_its_minimal_automaton` (`(a|b)*abb` is the
   textbook's four states over its two letters, plus the dead state of the other bytes);
   `a_regexp_past_its_deterministic_bound_is_printed_as_thompsons_automaton` (`(a|b)*a(a|b){12}`, 8193 states with
   the dead one: printed as Thompson's automaton at the library number, as a table under `--limit
   regexp_states=16384`, the two answering alike); `a_regexp_literal_past_its_automaton_bound_is_refused`
   (`(a{1000}){1000}`, naming the count and the bound);
   `the_printed_table_and_the_simulation_agree_on_every_short_byte_sequence` (D7, second item);
   `a_unit_that_reads_unicode_declares_the_pins_version` (D4); and the walk's half of
   `no_input_makes_matching_super_linear`.
6. **Run-time regular expressions (D3, D6)**: the type on every lane, its decode refusal on every lane, `regexp(t)`.
   Tests: `a_run_time_regexp_is_refused_past_its_bound_when_made` (`(a{1000}){1000}`); the type's cases of
   `tests/json_text/payload_refusal.json`, replayed on every lane; a scenario every lane replays with a `regexp[N]`
   event field; and the census of ADR 0092 §12, item 7, which fails until the constructor is in every table it reads.
7. **NFC (D5).** Tests: `the_nfc_conformance_data_of_the_pinned_unicode_passes` (`NormalizationTest.txt` at the
   pin: for every line of its parts, c2 = NFC(c1) = NFC(c2) = NFC(c3) and c4 = NFC(c4) = NFC(c5), and every code point
   part 1 does not list is its own NFC; `is_nfc` agrees with each); `nfc_grows_a_text_at_most_three_times` (U+1D160,
   four bytes, becomes twelve, and no line of the data grows more).
8. **The refusals (D6).** Test: `a_text_facility_is_refused_by_name_on_a_lane_that_does_not_serve_it`, one
   assertion per backend and the Interpreter for each of `matches`, `search`, `nfc` and `is_nfc`, and one that a
   machine holding a `regexp[N]` is not refused.
9. The documents named in "Amends".

## Open questions

Decided here, each with its reason; the acceptance kept each.

1. **Which syntax?** I-Regexp (D1): it is regular by definition, standard, and maps to every dialect, where any
   engine's own syntax is that engine's. It names no Unicode version; the pin supplies one.
2. **What does a regular expression match, a whole text or a part?** Both, as two names (D1), because JSONPath, the
   standard that uses I-Regexp, gives both and a document needs both.
3. **What are the library numbers of the two bounds?** No textbook gives the numbers. The textbook structure is a
   budget of states with a fallback from the deterministic automaton to the simulation (RE2; Cox 2010). 4096 states is
   where the build stops determinising, and 1024 states over code points is the most a regular expression may have,
   which limits range quantifiers as RFC 9485 section 8 advises. Each is named, so a build moves it.
4. **Does `nfc` fail when its result would not fit?** No: its type is three times its operand's (D5), a bound the
   Stability Policy keeps in every version, so it is total.

## Amended on acceptance

Each line names the change and its source: the independent review of 2026-11-28 ("review") and the decision of the
main session that accepted the record.

1. D3, D4: the run-time automaton reads code points, each class one transition over ranges, UTF-8 decoded by
   RFC 3629 in the same pass; the count is free of the Unicode version (review, amendment 1).
2. D2: the subset construction stops at `regexp_states` and the Thompson automaton is printed and simulated; only
   `regexp_nfa_states` refuses (review, amendment 2; RE2's hybrid chosen by the main session).
3. D1, D3: the text around the part `search` finds is well-formed, and the simulation adds the start state at each
   code point (review, amendment 3).
4. D8 (new), D5: a `string` is a sequence of Unicode scalar values in well-formed UTF-8 on every lane, checked where
   a value enters; the Interpreter is named, and its decode refuses (review, amendment 4; decided by the main session).
5. D4: `UNICODE_VERSION` and the runtime's constant assertion cover every unit that prints a class-derived automaton
   or calls `nfc`, `is_nfc` or `regexp`; "Amends" names ADR 0158 D6 (review, amendment 5).
6. D6: the type, its decode check and `regexp(t)` cross every lane, event schemas included; the tests and NFC stay
   Rust's (review, amendment 6; chosen by the main session over amending ADR 0092 §12).
7. The textbook structure, Alternatives rejected, D3, D5: only backreferences leave the regular languages, Pike's
   virtual machine cites Cox 2009, NFC's bound cites UAX #15 section 9 and the Stability Policy, and the simulation's
   space is O(m) words (review, amendment 7).
8. The type, its constructor, the crate, the codes, the failures and the bounds are named `regexp`, since
   `sce:pattern` names a communication pattern (review, cross-record conflicts; decided by the main session).
9. The message and catalog record is cited as ADR 0158, its number after it was renumbered from 0148 (main session).

## Sources

- S. C. Kleene, "Representation of Events in Nerve Nets and Finite Automata", in Automata Studies, Annals of
  Mathematics Studies 34, Princeton University Press, 1956.
- M. O. Rabin and D. Scott, "Finite Automata and Their Decision Problems", IBM J. Res. Dev., 1959.
- K. Thompson, "Regular Expression Search Algorithm", CACM 11(6), 1968.
- J. E. Hopcroft, "An n log n Algorithm for Minimizing States in a Finite Automaton", in Theory of Machines and
  Computations, Academic Press, 1971.
- A. V. Aho, M. S. Lam, R. Sethi, J. D. Ullman, "Compilers: Principles, Techniques, and Tools", 2nd ed.,
  Addison-Wesley, 2006, chapter 3 (Algorithm 3.23).
- A. V. Aho, "Algorithms for Finding Patterns in Strings", in J. van Leeuwen (ed.), Handbook of Theoretical Computer
  Science, Volume A, Elsevier, 1990.
- A. Morihata, "Translation of Regular Expression with Lookahead into Finite State Automaton", Computer Software
  (Japan Society for Software Science and Technology), 2012.
- T. Miyazaki and Y. Minamide, "Derivatives of Regular Expressions with Lookahead", Journal of Information
  Processing, 2019.
- P. Briggs and L. Torczon, "An Efficient Representation for Sparse Sets", ACM Letters on Programming Languages and
  Systems, 1993.
- R. Cox, "Regular Expression Matching Can Be Simple And Fast", 2007; "Regular Expression Matching: the Virtual
  Machine Approach", 2009; "Regular Expression Matching in the Wild", 2010; each published at swtch.com.
- RFC 9485, I-Regexp: An Interoperable Regular Expression Format, 2023; RFC 9535, JSONPath, 2024, sections 2.4.6 and
  2.4.7.
- RFC 3629, UTF-8, a transformation format of ISO 10646, 2003.
- Unicode Standard Annex #15, Unicode Normalization Forms, and `NormalizationTest.txt` of the Unicode Character
  Database; Unicode Technical Standard #18, Unicode Regular Expressions; the Unicode Consortium, "Unicode Character
  Encoding Stability Policies", unicode.org, read 2026-11.
- ADR 0050, ADR 0067, ADR 0077, ADR 0081, ADR 0092, ADR 0094, ADR 0099, ADR 0101, ADR 0133, ADR 0158 in this
  repository.
