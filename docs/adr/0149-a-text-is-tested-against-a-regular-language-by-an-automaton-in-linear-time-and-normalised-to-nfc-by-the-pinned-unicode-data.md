# ADR 0149 — A text is tested against a regular language by an automaton in time linear in the text, and normalised to NFC by the pinned Unicode data

- Status: Proposed (2026-11-27), for the owner's acceptance. Written under the owner's standing rule: the most
  long-term correct structure, textbook in its foundations, cost ignored; decided, not asked. Each open question is
  decided below by the textbook rule, and the acceptance may overturn any of them.
- Date: 2026-11-27
- Amends, once accepted (each amended document gains an "Amended by: ADR 0149" line in the landing that moves it):
  `SCE_FORGE.md` §3.4 (the row of regular expression literals keeps its refusal and changes its reason; the vocabulary
  gains `matches`, `search`, `pattern`, `nfc` and `is_nfc`); `docs/SCE_ACCEPTED_SUBSET.md` (the type
  `pattern[N]`, the vocabulary, and the appendix rows of D2 and D3); `SCE_ERROR_CONTRACT.md` (the same codes); the
  comment of `stdlib/text/utf8_length.scxml` that says normalisation is not provided (D5).
- Scope: the contract names no language. Built first: `sce-build` (the grammar, the compiled automata, the type) and
  a new Rust crate `sce-rust-pattern` (`no_std`, no heap), with normalisation in the Rust runtime, on native targets
  and on `wasm32-unknown-unknown`. Every other backend refuses by name (D6).
- Related: ADR 0050 (a bound read by name), ADR 0056 (buffers are the document's size), ADR 0067 (a table row is as
  small as its numbers), ADR 0077 (a declared bound is part of a type and every decode refuses what breaks it), ADR
  0092 §12 (every constructor of the type grammar is held on every lane), ADR 0133 (one classification of a failure),
  ADR 0148 (the pin of Unicode and CLDR, D6, and the code of its D8).

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
- **Regular expressions are only an output.** The build writes ECMA-262 patterns into the JSON Schema of a model's
  answer (`regex_range` in `sce-build/src/forge/value_type.rs`, ADR 0087) and checks them with the `regex` crate in a
  test of that file; `template.rs` compiles an XSD pattern with it in a test. `regex` 1.12.3 with `regex-automata`
  0.4.14 is a dependency of `sce-build` alone (`cargo tree -i regex`), never of a generated machine.
- **There is no normalisation.** `stdlib/text/utf8_length.scxml` counts the code points it is given and says why it
  does not normalise: a Hangul syllable is one code point in NFC and two or three in NFD, so "a product that wants one
  cap to mean one thing writes its text in NFC (UAX #15) at the boundary where the text enters", but "Normalization is
  not a document here; it would carry Unicode's composition tables into every backend and a new table with each
  Unicode version". A key is compared as its UTF-8 bytes (SCE_HOST_FEATURES.md section 3), and Swift's canonical
  equivalence is replaced by a comparison of bytes so the lanes agree (`sce-build/src/forge/expr/swift.rs`).
- **`icu_normalizer` 2.2.0 is already in `Cargo.lock`**, reached only through `url`'s IDNA support in a test crate.

## The textbook structure

- **A regular language is recognised by a finite automaton** (Kleene 1956; Rabin and Scott 1959). Thompson's
  construction (1968) turns a regular expression of size m into an automaton of O(m) states with empty moves; the
  subset construction makes it deterministic; Hopcroft's algorithm (1971) makes the deterministic one minimal. A
  deterministic automaton decides membership with one transition per input symbol: time linear in the text, and
  space constant.
- **Determinisation can cost states exponential in the pattern**: `(a|b)*a(a|b){k}` needs 2^(k+1) states, since the
  automaton must remember the last k+1 symbols. Where that is too many, the nondeterministic automaton is simulated
  directly, a set of states advanced per symbol (Thompson 1968; Pike's VM): time O(n*m), still linear in the text.
- **Backtracking is not an automaton.** Perl's, PCRE's, Java's, Python's and ECMAScript's matchers search the space of
  ways to match, which is exponential in the worst case: the denial of service a crafted text causes against them is
  why linear matchers exist (Cox, "Regular Expression Matching Can Be Simple And Fast", 2007; RE2, 2010). Their
  patterns also go beyond regular languages: a backreference makes a language that is not regular.
- **An interoperable regular syntax exists**: I-Regexp (RFC 9485, 2023), a subset of XML Schema's regular expressions
  that has no backreference and no lookaround, is matched against the whole text, works on Unicode code points with
  General Category classes, and maps to every common dialect. JSONPath (RFC 9535) uses it for its `match()` and
  `search()` functions (sections 2.4.6 and 2.4.7): the whole text in the language, and some part of it.
- **Normalisation is defined by UAX #15** over the Unicode Character Database: NFC is canonical decomposition followed
  by canonical composition. Its conformance is `NormalizationTest.txt` of the same version. Its stability policy says
  a text of characters assigned in one version that is normalised stays normalised in every later version; a code
  point not yet assigned can normalise differently once it is. UAX #15 states how far each form can grow a text: NFC at
  most three times in UTF-8.

## Decision

### D1. A pattern is an I-Regexp, and two tests read it

- **The syntax and meaning are RFC 9485's**: its grammar (section 3) and its semantics (section 4), over Unicode code
  points; `.` is any code point but line feed and carriage return; `\p{..}` and `\P{..}` name General Category
  values and their groups. There are no flags: a case-insensitive test is a class (`[Aa]`).
- **`matches(text, p)`** is true when the whole of `text` is in the language of `p` (RFC 9535 section 2.4.6).
  **`search(text, p)`** is true when some part of `text` is (section 2.4.7). Both are `bool` and total.
- **The text is read as UTF-8 bytes**, and the automaton's byte classes spell RFC 3629's grammar as
  `utf8_length.scxml` does, so a byte string that is not UTF-8 is in no language.
- **A regular expression literal stays refused.** The row of SCE_FORGE.md §3.4 keeps `/pattern/` refused, with its
  reason stated: an ECMAScript pattern is not regular in general and is matched differently by each engine. A pattern
  is the text of a string literal or a value of D3, read by one grammar.

### D2. A pattern written in a document is compiled when the machine is built

When the second argument of `matches` or `search` is a string literal, the build:

1. reads it by RFC 9485's grammar, refusing a text that is not an I-Regexp as **`expression/invalid-pattern`** at the
   offset where the grammar stops;
2. expands each class from the pinned Unicode data (ADR 0148 D6), so a pattern and a normalisation in one build read
   one Unicode version;
3. builds Thompson's automaton over UTF-8 bytes, makes it deterministic and minimal (for `search`, the automaton of
   any text, then the language, then any text), and refuses a pattern whose minimal automaton has more states than the
   named bound `pattern_states` (library number 4096) as **`expression/pattern-too-large`**, naming the count and the
   bound; `sce-codegen --limit pattern_states=N` moves it (ADR 0050);
4. prints the automaton as data: a class for each of the 256 bytes, a table of states by classes in the narrowest
   unsigned type that numbers its states (ADR 0067), and the accepting states. One walk of the runtime reads it, one
   table read per byte, with no heap and no recursion; the walk is one function for every pattern.

A pattern is named and shared as any computation is: an algorithm (`is_slug(text) -> bool`) holds the literal. No
element declares a pattern.

### D3. A pattern supplied at run time is a value of a type that carries its bound

- **`pattern[N]`** is a text of at most N bytes that is an I-Regexp whose automaton (Thompson's, every counted
  repetition expanded) has at most `pattern_nfa_states` states (a named bound, library number 1024). It stands where
  a `string[N]` stands: a variable, an event field, an algorithm's parameter. It is a new constructor of the type
  grammar, held to every lane by ADR 0092's census (`every_constructor_of_the_type_grammar_is_held_on_every_lane`).
- **It is made by a check.** `pattern(t)` makes one from a text and fails when `t` is not an I-Regexp
  (`invalid_pattern`) or its automaton passes the bound (`pattern_too_large`), read as every failure is (ADR 0133).
  Every decode of a field of the type refuses such a text (ADR 0077), so no value of it is ever invalid; a saved state
  holds its text.
- **It is matched by simulation.** `matches` and `search` with such a value build Thompson's automaton into storage
  the bound sizes and advance a set of states per byte, each state entered at most once per byte: time O(n*m) with m
  at most `pattern_nfa_states`, space O(m) bits, no heap. The build is per test, O(m), and does not change the bound
  in the text.
- `(a{1000}){1000}` is a short text with a million states: it is refused when it is made, never compiled.

### D4. One grammar and one construction, at build time and at run time

`sce-rust-pattern` holds the reader of RFC 9485's grammar, Thompson's construction into storage its caller gives,
the simulation, and the walk of a printed automaton. `sce-build` depends on it and adds the subset construction and
the minimisation, which only the build runs. So the language a build-time pattern accepts and the one the same text
accepts at run time are the same by construction, not by two implementations kept in step. Its General Category
data are `icu_properties`' at the pin.

### D5. NFC, from the pinned data

- **`nfc(t)`**, for `t` a `string[B]`, is the NFC of `t` (UAX #15), a `string[3B]`: UAX #15's bound on the growth
  of NFC in UTF-8 makes it total. **`is_nfc(t)`** is whether `t` is in NFC. Both read the pinned Unicode version
  (ADR 0148 D6) through `icu_normalizer`'s composing normaliser.
- **Where it is used is the document's.** utf8_length's advice stands: normalise at the boundary where the text
  enters, before a cap is checked or a key is made. Its comment is amended: normalisation is now a vocabulary operation
  of the lanes of D6, over data the build pins, and still not a document.
- **Equality does not change.** Two texts are equal when their bytes are; a document that means canonical
  equivalence writes `nfc(a) === nfc(b)`. Nothing is normalised without being asked: a decoded text keeps its bytes,
  which a signature or a digest may cover.
- **A change of pin is a change of answer only for code points the earlier pin did not assign** (UAX #15's stability
  policy); ADR 0148 D6 makes the change deliberate.

### D6. Lanes

`matches`, `search`, `pattern[N]`, `nfc` and `is_nfc` are built for Rust, native and wasm32. A build for any
other backend, and the Interpreter's lowering, of a machine or an algorithm that uses one is refused by name with
`codegen/text-facility-not-served` (ADR 0148 D8). A build-time automaton is data any backend can print; a lane
joins by printing it and its walk, and the simulation and the normalisation with its runtime, in the change that lands
its tests.

### D7. Linear time is a tested property

- **Steps are counted.** Under a test feature the walk and the simulation count what they do. A family of patterns
  that drive a backtracking matcher to exponential time (`(a|a)*b`, `(a*)*b`, `(a|b)*a(a|b){10}`, `.*.*.*x`) is run
  on texts of 1 to 2^20 bytes; a printed automaton takes exactly one step per byte, and the simulation at most m per
  byte. A count is exact where a clock is noise.
- **The language is the one RFC 9485 means.** Every pattern of a corpus (RFC 9485's examples, the `match()` and
  `search()` cases of the JSONPath compliance test suite, and the families above) is tested against every text over a
  small alphabet up to a length, and the answer agrees with the `regex` crate given RFC 9485's mapping of the pattern
  (a test-only oracle, itself linear).

### Not in scope

- A schema field that declares a pattern its decode holds it to (an XSD `pattern` facet): it changes every codec of
  every lane and the grammar a model is held to (ADR 0087), and is a record of its own when one needs it.
- Captures, replacement and the position of a match; flags; NFD, NFKC and NFKD; grapheme clusters (UAX #29); case
  folding.
- Any other lane.

## Consequences

- No text a document is handed, however crafted, makes a test take more than a step per byte (D2) or m steps per
  byte (D3); a document cannot write a pattern that is not regular.
- A pattern the author writes wrong fails the build where it is written, at the offset.
- `matches`, `search`, `pattern`, `nfc` and `is_nfc` become vocabulary names, so a document that declares a callable
  of one of those names is refused where it declares it (`crate::forge::namespace`), as for the builtins today.
- A machine that normalises carries ICU4X's normalisation data on the Rust lane; one that does not carries none.
- A pattern's automaton is part of the generated unit, so its size is the build's to report and to bound.

## Alternatives rejected

- **ECMAScript regular expression literals.** Not regular (backreferences, lookaround), backtracked by the engines
  that run them, and different in each.
- **A backtracking matcher with a step budget.** It turns a slow match into a failure; the answer to "is this text of
  this form" is then "unknown", and the bound is a guess.
- **Each platform's regular expressions.** `java.util.regex`, Python's `re`, Swift's `Regex` and ECMAScript's
  backtrack; Go's `regexp` and RE2 do not, but each has its own syntax and Unicode tables.
- **The `regex` crate at run time.** Linear, but its syntax is not I-Regexp, its Unicode tables are a second Unicode
  version in the build, and it needs a heap.
- **XML Schema's regular expressions.** Regular, but its multi-character escapes and block names are XML's; I-Regexp
  is the subset of them that is meant to be portable.
- **A deterministic automaton only, refusing every pattern past the bound at run time.** A run-time pattern's
  automaton would be built while the machine runs, at a cost exponential in the worst case; simulation is linear.
- **Normalisation as an algorithm document.** utf8_length's reason stands: the composition tables would be written
  per backend and per Unicode version.
- **Normalising every decoded text.** It changes bytes a signature or a digest covers, and the deterministic forms of
  ADR 0101 would no longer be the bytes sent.

## Order of work

Each step lands with a test that fails without it.

1. This record, accepted.
2. **The pin** (ADR 0148, step 2), if that record has not landed it.
3. **`sce-rust-pattern`: the grammar, the construction and the simulation (D3, D4).** Tests:
   `an_iregexp_is_read_by_rfc_9485s_grammar` (the RFC's examples and refusals);
   `a_pattern_agrees_with_the_regex_crate_on_every_short_text` (D7, second item);
   `no_input_makes_matching_super_linear` (D7, first item, for the simulation). Without the crate none compiles.
4. **Build-time patterns (D1, D2)**: `matches` and `search` with a literal, the Rust template's table and walk, the
   two codes and their rows. Tests: `a_pattern_literal_is_compiled_to_its_minimal_automaton` (`(a|b)*abb` is the
   textbook's four states over its two letters, plus the dead state of the other bytes);
   `a_pattern_whose_automaton_passes_its_bound_is_refused` (`(a|b)*a(a|b){12}`, 8193 states with the dead one,
   refused at the library number and accepted under `--limit pattern_states=16384`); and the walk's half of
   `no_input_makes_matching_super_linear`.
5. **Run-time patterns (D3)**: the type, its decode refusal, `pattern(t)`. Tests:
   `a_run_time_pattern_is_refused_past_its_bound_when_made` (`(a{1000}){1000}`) and the census of ADR 0092 §12,
   item 7, which fails until the constructor is in every table it reads.
6. **NFC (D5).** Tests: `the_nfc_conformance_data_of_the_pinned_unicode_passes` (`NormalizationTest.txt` at the pin:
   for every line of its parts, c2 = NFC(c1) = NFC(c2) = NFC(c3) and c4 = NFC(c4) = NFC(c5), and every code point part 1
   does not list is its own NFC; `is_nfc` agrees with each); `nfc_grows_a_text_at_most_three_times` (U+1D160, four
   bytes, becomes twelve, and no line of the data grows more).
7. **The refusals (D6).** Test: `a_text_facility_is_refused_by_name_on_a_lane_that_does_not_serve_it`, one assertion
   per backend and the Interpreter.
8. The documents named in "Amends".

## Open questions

Decided here, each with its reason; the acceptance may overturn any.

1. **Which syntax?** I-Regexp (D1): it is regular by definition, standard, and maps to every dialect, where any
   engine's own syntax is that engine's.
2. **What does a pattern match, a whole text or a part?** Both, as two names (D1), because JSONPath, the standard that
   uses I-Regexp, gives both and a document needs both.
3. **What are the library numbers of the two bounds?** 4096 states for a printed automaton and 1024 for a run-time
   one; each is named, so a build moves it.
4. **Does `nfc` fail when its result would not fit?** No: its type is three times its operand's (D5), so it is total.

## Sources

- S. C. Kleene, "Representation of Events in Nerve Nets and Finite Automata", 1956.
- M. O. Rabin and D. Scott, "Finite Automata and Their Decision Problems", IBM J. Res. Dev., 1959.
- K. Thompson, "Regular Expression Search Algorithm", CACM 11(6), 1968.
- J. E. Hopcroft, "An n log n Algorithm for Minimizing States in a Finite Automaton", 1971.
- A. V. Aho, M. S. Lam, R. Sethi, J. D. Ullman, "Compilers: Principles, Techniques, and Tools", 2nd ed., chapter 3.
- R. Cox, "Regular Expression Matching Can Be Simple And Fast", 2007, and "Regular Expression Matching in the Wild",
  2010.
- RFC 9485, I-Regexp: An Interoperable Regular Expression Format; RFC 9535, JSONPath, sections 2.4.6 and 2.4.7.
- RFC 3629, UTF-8.
- Unicode Standard Annex #15, Unicode Normalization Forms, and `NormalizationTest.txt` of the Unicode Character
  Database; Unicode Technical Standard #18, Unicode Regular Expressions.
- ADR 0050, ADR 0067, ADR 0077, ADR 0092, ADR 0133, ADR 0148 in this repository.
