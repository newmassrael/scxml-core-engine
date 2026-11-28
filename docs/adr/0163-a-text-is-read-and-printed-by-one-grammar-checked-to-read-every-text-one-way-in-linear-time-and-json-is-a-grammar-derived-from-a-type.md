# ADR 0163 — A text is read and printed by one grammar, checked to read every text one way in linear time, and JSON is a grammar derived from a type

- Status: Accepted (2026-11-29) under the owner's standing rule (the most long-term correct, textbook structure, cost
  ignored), by the main session the owner delegated the decision to, after two independent reviews and a downstream
  consumer's review; reworked on the first review and the consumer's, and amended on acceptance with every amendment of
  the second, as listed in "Amended on acceptance".
- Landed: step 1 of "Order of work" (this record, accepted, and the "Amended by: ADR 0163" lines of ADR 0087 and ADR
  0164). No other step is built, and no theorem, model or test of it has been run.
- Date: 2026-11-28 (proposed); 2026-11-29 (reworked, and accepted).
- Written under the owner's standing rule (the most long-term correct structure, textbook in its foundations, cost
  ignored; decided, not asked) and the owner's scope rule of 2026-11-22: a facility is built on the backends a consumer
  needs, Rust by default; its contract is general, and what is special to one application stays in the application. A
  downstream consumer's census asked for it; every example below is a standard's or a generic one.
- Amends (each amended document gains an "Amended by: ADR 0163" line: the accepted records whose decisions it amends,
  ADR 0087 and ADR 0164, gained it on acceptance; every other document gains it in the landing that moves it):
  - `SCE_FORGE.md` §4.6 (the encodings `grammar`, `json` and `json5`, as sections 4.6.5 and 4.6.6, D1) and §8 (the codec
    row names its encodings);
  - `schemas/sce-forge-ext.xsd` (the `encoding` enumeration, the codec root's `sce:type`, the grammar's elements;
    `pre-release`);
  - ADR 0065 §2 and `sce-build/src/forge/codec_totality.rs`: the places a text codec's encode refuses (D4, D8);
  - ADR 0086 §3.1 and §4, and `ARCHITECTURE.md` "JSON Number Text": every reader of a `float32` rounds its decimal once
    into binary32, and every writer writes a `float32` by its own shortest digits, on every lane (D9); the bullet "A
    `float32` is written as its platform spells it; only the 64-bit form is pinned" is deleted; a text codec prints a
    negative zero as `-0` and refuses a float that is not finite (D8). "JSON Text", "JSON Number Text" and "JSON Object
    Key Order" gain the text codec as a writer, with a Rust row and a C11 row, and name the one Rust crate every Rust
    writer delegates to (D13, D16);
  - ADR 0087 §1.1 (the payload schema annotates an enum's numbers, D17), §2.1 (`<sce:lift>` names a text codec and a
    place, D12), §2.1 item 5 (after "then the fields": the refusal of the type at the least byte offset, D8), §2.3
    (codes 17 to 21, after ADR 0164's 16, D12), §2.4 (the refusal record's second revision, with `offset`, D12) and §2.5
    (Rust's typed lift reads through the generated JSON codec, D13);
  - ADR 0108 D7 and its Open question 1: a record's field may hold a map or a set where a text codec reads and writes
    it, and the JSON form of a map (D10);
  - ADR 0115 D7: `Refuses_C` gains the places of D4 and D8, and a lift and a print are read as D15 says;
  - ADR 0149 D4: `sce-rust-regexp` gains the operations on automata a grammar's checks need (D2);
  - ADR 0164 D4 and D12: a grammar rule may reach itself through a member of an inductive document, and the fold of a
    type into its JSON binding has that record's arm (D5, D8); D4's Rust streaming reader and T1b: the binder's
    inductive arm is that reader, and T4 discharges T1b's decode half for JSON, so Rust holds one streaming reader of a
    typed JSON value (D13, D14);
  - `docs/SCE_ACCEPTED_SUBSET.md` (§2.15: a record that holds a map, `<sce:print>`; the appendix rows of D18) and
    `SCE_ERROR_CONTRACT.md` (the same codes);
  - `stdlib/lift/refusal.scxml` (its revision and its comment on what a path names) and `stdlib/lift/refusal_code.scxml`
    (D12), with `sce-build/tests/fixtures/static_datamodel/enum_harness_code.scxml` and
    `tests/host_features/model_outcome_classes.json`, which repeat the lift's codes; and a new
    `stdlib/text/place_step.scxml` and `stdlib/text/json_pointer.scxml` (D12).
- Scope: `sce-build` (the grammar's reader, its checks and its plan; the JSON and JSON5 syntax plans and the fold of a
  type into its binding; the JSON Schema import; the statements; the model's reading); a new crate
  `sce-grammar-reference` (the specification functions the theorems are stated over, and an executable check of a plan
  proved to imply their premises); `sce-rust-regexp` (D2's operations, the build's behind a feature with a heap); the
  Rust forge runtime (`sce_forge_runtime::grammar`: the reader, the printer and the binder, proved) and a new crate
  `sce-json-text` (the one JSON text writer, `no_std` with no heap, D13); the Rust lane, native, `no_std` with no heap
  and `wasm32-unknown-unknown`; the C11 lane (`sce/forge/grammar.h`, in the freestanding core of ADR 0110); the
  `float32` reading and writing of every lane (D9); and the order of a type's refusals in every lane's lift (D8). Every
  other backend, and the Interpreter, refuses a text codec by name (D16).
- Related: ADR 0010, 0014, 0015 and 0104 (the content-line codec), ADR 0044 (revisions), ADR 0049 (a decoded codec is a
  view over its bytes), ADR 0050 and ADR 0086 (a bound by name, part of a type; the JSON wire form), ADR 0055
  (optional), ADR 0058 (a codec's laws), ADR 0065 (declared totality of an encode), ADR 0077 (every decode refuses what
  breaks a bound), ADR 0081 (an encode planned once), ADR 0087 (the payload grammar, `<sce:lift>` and its refusal
  record), ADR 0092 (a union and its JSON form), ADR 0094 (the source hash), ADR 0099 (a failure is not a value), ADR
  0101 (one deterministic form per type), ADR 0102 and ADR 0111 (a codec bound at a boundary), ADR 0108 and ADR 0154
  (maps and their storage), ADR 0110 (the freestanding C11 core), ADR 0115 and ADR 0156 (the formal levels and the
  generated model, with ADR 0156's amendment, in review beside this record, that schedules the translation of the
  `sce-static` data model), ADR 0133 (totality where the build proves it), ADR 0139 and ADR 0140 (a codec on a hot
  path), ADR 0149 (automata in linear time, the Unicode pin, well-formed text), ADR 0151 (a total comparison of values),
  ADR 0155 (a cold codec is a table one walker reads), ADR 0158 (the pin of Unicode), ADR 0162 (a decimal of declared
  precision and scale), ADR 0164 (a recursive type held in an arena), ADR 0166 (documents generic in application types),
  and ADR 0168 (a polytypic encoding, which follows this record and builds on it, D19).

## Context

What exists, read from the tree at `662e4d060cc`, with the records accepted beside it:

- **Three encodings of a codec, one of them text.** A codec (`SCE_FORGE.md` §4.6) is positional (fields at bit
  positions), `cbor` (§4.6.1, ADR 0101: an entry is any value of the one type grammar, in one deterministic form) or
  `content-line` (§4.6.4; ADR 0010, 0014, 0015, 0104: one RFC 5545 component). The content-line codec is the one codec
  of text. Its grammar is written once in prose, in §4.6.4, and once by hand in each runtime
  (`backends/rust/forge-runtime/src/content_line.rs`, 1938 lines, and its siblings in six runtimes), held together by
  vectors an independent model writes (`tests/forge/conformance/content_line_model.py`). Each of its rules (a name
  matched without regard to case, a TEXT escape, an undeclared property skipped, a required property, a count past its
  bound, a line folded at 75 octets) is a rule of that one format, and a second format of text would be written again
  the same way, in prose and in seven runtimes.
- **JSON is read and written on every lane as an event's typed payload.** ADR 0086 §3.1 fixes a value's JSON spelling by
  its type: a 64-bit integer as the text of its digits, a byte string as the Latin-1 text of its bytes, an absent
  optional as `null` under its key, a record's keys ascending by their UTF-8 bytes. Every runtime lifts `_event.data`
  into an event schema's fields and refuses a value outside its type, naming the place as an RFC 6901 pointer (ADR 0086
  §4; ADR 0087 §2.3 and §2.4, the fourteen codes of `stdlib/lift/refusal_code.scxml`), and `<sce:lift>` reads a text a
  statechart holds the same way (ADR 0087 §2.1). The readers are written by hand per lane (Rust
  `backends/rust/runtime/src/json.rs`, "the runtime's one JSON reader and writer", and `event_payload.rs`; Go
  `backends/go/runtime/event_payload.go`; Kotlin `Json.kt` and `EventPayload.kt`; Python `sce_runtime/event_payload.py`;
  Swift `EventPayload.swift`; C++ `sce/include/common/EventPayloadLift.h`; C11
  `backends/c/runtime/include/sce/event_payload.h`; the Interpreter's `sce_static.js`) and held to shared tables
  (`tests/json_text/payload_refusal.json`, 142 cases; `string_escape.json`, `real_text.json`, `object_key_order.json`),
  each written because the readers, read in their code, disagreed on what a text is. The Rust lift parses the text into
  its own tree (`PayloadValue::decode` over `json::Value`, a number kept in its spelling) and then walks it, `std` only;
  it reads a `float32` through a binary64 on purpose and refuses a double above `f32::MAX` (`event_payload.rs`,
  `PayloadValue::float32`).
- **JSON is written for a deployment's queryables** by `backends/rust/link-runtime/src/json.rs` (ADR 0041): a declared
  document measured by a counting sink, then written into the caller's buffer.
- **A type's JSON grammar is written, for a model to answer in.** `sce-build/src/forge/value_type.rs` folds every type
  into its JSON Schema `S(T)`, its GBNF grammar `G(T)` (ADR 0087 §1.1) and its largest JSON text `J(T)`
  (`json_max_len`); `payload_schema.rs` tests the GBNF with an Earley recognizer (Aycock and Horspool). No reader is
  generated from any of them.
- **Regular languages.** ADR 0149 (accepted on 2026-11-28; its step 4's crate `sce-rust-regexp`, at
  `backends/rust/regexp`, `no_std` with no heap, is landing item 481) gives I-Regexp (RFC 9485) compiled at build time
  to a minimal deterministic automaton within `regexp_states`, and the rule that every `string` is well-formed UTF-8 on
  every lane (its D8). It leaves out captures and the position of a match, so a regular expression says whether a text
  has a form and never what its parts are. Its operations are the reader of RFC 9485, Thompson's construction, the
  subset construction, minimisation and one walk; a grammar's checks need more (D2).
- **Maps.** ADR 0108 gives `map<K, V>[B]` and `set<K>[B]`, a text key among them, ordered by when each key was put (D2).
  A map in an event schema's field, and so in any codec, is refused as `validation/map-in-wire-position` (D7), and its
  wire form is that record's Open question 1. ADR 0154 lets a Rust map keep its slots in storage its host lends.
- **Text in expressions.** A text is joined with `+` (to a string or an integer) and measured by `len` (`SCE_FORGE.md`
  §3.4); nothing cuts it, searches it or reads a structure out of it. `stdlib/text/utf8_length.scxml` writes RFC 3629's
  grammar by hand, byte by byte, as an algorithm.
- **Recursive types and decimals.** ADR 0164 (being accepted) declares a recursive type in one document of the kind
  `inductive`, holds a value of it as a tree of at most its node bound in an arena, refuses the node past the bound as
  `tooManyNodes` (a refusal code numbered after the fourteen), and gives the JSON form of such a value (its D4). ADR
  0162 (being accepted) adds `decimal(p, s)`, whose JSON spelling is its canonical text as a string (its D9).
- **No value of unknown type.** ADR 0101 makes a CBOR entry any value of the one type grammar, and every value of that
  grammar is typed; no type of the language holds a value whose type is not known where it is read: ADR 0101 has no
  "any" value.
- **No JSON Schema is read.** `schemars` and `jsonschema` are development dependencies of `sce-build` that describe and
  check SCE's own surfaces (ADR 0087, Context).
- **What the model generator does with a codec.** ADR 0115 D7 (amended 2026-11-23) reads a codec as a partial function
  of values: the values its encode refuses (`Refuses_C`, from `codec_totality::refusals`) and the round trip of every
  other (ADR 0058 §5, law 1); a frame a decode refuses enters only as a channel's `corruption`. No `Refuses_C` is
  written yet (ADR 0156 D3 orders it third), a `sce-static` document is refused whole (`Construct::Datamodel` in
  `sce-build/src/formal/document_model.rs`), and a string has no model.

A downstream consumer that moves a network stack's logic into documents counted what stays in host code for want of
these: 85 rows of parsing, printing, cutting and comparing structured text (addresses, keys, selectors, parameter lists,
schema files) and of tables keyed by such text, and 27 rows of JSON and JSON5 documents (configuration, administrative
replies, profiles) held, taken apart and written. Its own recommendation was a codec of text grammars, a format of its
own that refuses by rule, rather than a widening of the content-line encoding. Read by this record from the census's
classification, three of those rows read nested structure (a schema language's nested declarations, a payload's nested
fields named by a path, and a configuration document's value tree); the others read structure of a depth their grammar
bounds. This record is that codec, with JSON and JSON5 as two of its encodings, each a fixed syntax and a binding
derived from a type. The same consumer's review of the first draft asked for C11, a declared member order and float
notation, maps read in the order of their keys, a value that borrows its text, recursion through ADR 0164, a place
chosen by a field's value, a model of a lift before strings have one, and a hot codec printed as code; "Amended on
acceptance" lists the decision on each.

## The textbook structure

- **A syntax is described once, and parsing and printing are its two readings.** Rendel and Ostermann ("Invertible
  Syntax Descriptions: Unifying Parsing and Pretty Printing", Haskell Symposium 2010) build a description from partial
  isomorphisms between a value and its parts, and from product, choice and repetition, and read one description as a
  parser and as a printer. The printer is the parser's partial inverse under conditions the description must meet: each
  isomorphism is one, and a choice's alternatives are told apart by what they read. Their `chainl1` reads a
  left-associative chain of operators through a `foldl` isomorphism. Jansson and Jeuring ("Polytypic data conversion
  programs", Science of Computer Programming 2002) derive a printer and a parser from a datatype's structure, one rule
  per type constructor, with the round trip as the property they prove: a polytypic reading, which is how a type's JSON
  is read here (D8) and what ADR 0168 generalises (D19).
- **A transformation in two directions has laws, and forgets only what it says it forgets.** Foster, Greenwald, Moore,
  Pierce and Schmitt ("Combinators for bidirectional tree transformations: a linguistic approach to the view-update
  problem", POPL 2005; TOPLAS 2007) state a lens's laws. Boomerang (Bohannon, Foster, Pierce, Pilkiewicz and Schmitt,
  "Boomerang: Resourceful Lenses for String Data", POPL 2008) gives lenses between strings and values typed by regular
  languages, requires each concatenation of two languages to be unambiguous, and matches the chunks of a list by a key
  (dictionary lenses). Quotient lenses (Foster, Pilkiewicz and Pierce, ICFP 2008) hold the laws up to equivalences on
  either side that canonizers name: on the text, whitespace, the order of fields, the spelling of an escape; on the
  value, an order the value does not mean.
- **A grammar is unambiguous by construction when it is LL(1)** (Lewis and Stearns, "Syntax-directed transduction", JACM
  1968; Knuth, "Top-down syntax analysis", Acta Informatica 1971; Aho, Lam, Sethi and Ullman, *Compilers: Principles,
  Techniques, and Tools*, 2nd ed., Addison-Wesley 2006, §4.4.3): the tokens each alternative of a choice can begin with
  are disjoint; at most one alternative derives the empty text; and where one does, its FIRST set and the FOLLOW set of
  the choice are disjoint. Such a grammar has at most one derivation of each text, and a predictive parser finds it with
  one token of lookahead, without backtracking. Recursion that is not left recursion is read with a stack as deep as the
  nesting. A Parsing Expression Grammar (Ford, "Parsing Expression Grammars: a recognition-based syntactic foundation",
  POPL 2004) is unambiguous too, by ordered choice, and a packrat parser reads it in linear time with a table as large
  as the text times its rules (Ford, "Packrat parsing: simple, powerful, lazy, linear time", ICFP 2002); "Alternatives
  rejected" says why it is not chosen.
- **An expression is printed with the parentheses its precedence needs and no others** (Ramsey, "Unparsing expressions
  with prefix and postfix operators", Software: Practice and Experience 1998): a reader of a chain of operators of
  declared precedence and associativity builds the tree, and a printer that adds a parenthesis only where the tree's
  shape differs from what precedence would read gives a text that reads back to the same tree.
- **The tokens are regular languages, read by automata, and the text is cut by the longest match** (Aho et al., chapter
  3; Lesk's `lex`, whose rule is the longest match and then the earliest declared, a lexical precedence). A scanner told
  by the parser which tokens it expects where it stands (context-aware scanning: Van Wyk and Schwerdfeger,
  "Context-aware scanning for parsing extensible languages", GPCE 2007, for LR parsers) reads the small languages in
  which a token of one place is text of another, as an address and a name are. The longest match costs time quadratic in
  the text when a token can be read far past its end before the attempt fails; Reps ("'Maximal-munch' tokenization in
  linear time", TOPLAS 1998) makes it linear for every automaton by tabulating the failed attempts, in `n * |Q|` bits of
  workspace.
- **A record's fields may come in any order** in some syntaxes (a JSON object, a list of properties). A permutation
  phrase (Cameron, "Extending context-free grammars with permutation phrases", ACM LOPLAS 1993; Baars, Löh and
  Swierstra, "Parsing permutation phrases", Journal of Functional Programming 2004) reads each member once in any order,
  where writing every order out as alternatives would be exponential in the members and not LL(1).
- **Telling whether names repeat is linear in their bytes.** Names are strings over an alphabet of 256 bytes, and a trie
  of them (de la Briandais 1959; Fredkin, "Trie memory", CACM 1960; Knuth, *The Art of Computer Programming*, volume 3,
  §6.3) finds a repeated one in time linear in their total length, with no hash and so no key a sender could learn; a
  lexicographic sort of strings of total length `L` over an alphabet `Σ` takes `O(L + |Σ|)` (Aho, Hopcroft and Ullman,
  *The Design and Analysis of Computer Algorithms*, Addison-Wesley 1974, §3.2). A comparison sort costs `O(L log m)` for
  `m` keys of total length `L`.
- **Which strings a context-free language can follow is a reachability question.** Whether some text of a context-free
  language, cut anywhere, takes a finite automaton from one state to another is decided by reachability over the
  grammar's rules with a summary relation per nonterminal (CFL-reachability: Yannakakis, "Graph-theoretic methods in
  database theory", PODS 1990; Reps, "Program analysis via graph reachability", Information and Software Technology
  1998), in time cubic in the automaton's states and linear in the grammar.
- **A number read from text and printed again is itself only when both conversions are exact.** A decimal is read to the
  nearest binary floating number of the target format, correctly rounded once (Clinger, "How to read floating point
  numbers accurately", PLDI 1990), and the shortest decimal that reads back to the same number is printed (Steele and
  White, "How to print floating-point numbers accurately", PLDI 1990; Adams, "Ryū: fast float-to-string conversion",
  PLDI 2018). Which digits are printed is then fixed; where the point and the exponent stand is a notation, a separate
  choice. A binary32 read by way of a binary64 is rounded twice and can miss the nearest binary32: the text
  `1.000000059604644775390625000001` lies just above the midpoint of `1` and the next binary32, rounds to that midpoint
  as a binary64, and then to `1` by ties to even, where the nearest binary32 is `1.0000001`.
- **Recognise the whole input as a language before acting on it** (language-theoretic security: Sassaman, Patterson,
  Bratus and Shubina, "The Halting Problems of Network Stack Insecurity", ;login: 2011): a reader of a deterministic
  context-free language at the boundary, which hands the logic a typed value or a refusal and nothing in between. Logic
  that cuts and compares text where a typed value would do repeats the parser at every use and disagrees with it at
  some.
- **A view presents a value without building it** (Wadler, "Views: a way for pattern matching to cohabit with data
  abstraction", POPL 1987; ADR 0049): a reader that keeps where each part of the text lies, and converts a part when it
  is asked for, gives the same value as one that copies, at the cost of holding the text while the view is read.
- **An abstraction of an input by a nondeterministic choice over-approximates**: a model that lets the environment
  choose any value of a type, where the program reads one by a function, has every behaviour the program has (Clarke,
  Grumberg and Long, "Model checking and abstraction", TOPLAS 1994). A finite exploration of it cuts the choice to a
  domain and judges only within it; and a violation found in the abstraction may have no run of the program, so it is
  made concrete or reported as spurious (counterexample-guided abstraction refinement: Clarke, Grumberg, Jha, Lu and
  Veith, "Counterexample-guided abstraction refinement", CAV 2000).
- **A parser can be proved, not only tested.** Lasser, Casinghino, Fisher and Roux ("A Verified LL(1) Parser Generator",
  ITP 2019) prove an LL(1) table generator and its parser sound and complete in Coq; Egolf, Lasser and Fisher
  ("Verbatim: A Verified Lexer Generator", LangSec 2021) prove a longest-match lexer; EverParse (Ramananandro et al.,
  USENIX Security 2019) generates parsers of binary formats proved to be the inverse of their serializers; Narcissus
  (Delaware, Suriyakarn, Pit-Claudel, Ye and Chlipala, ICFP 2019) derives encoders and decoders with their round trip
  proved. The theorems of D14 are of this kind, over the walkers that run. Where a decision is expensive, a checker need
  not decide it again: a certifying algorithm returns, with its answer, a certificate that a small checker verifies
  (McConnell, Mehlhorn, Näher and Schweitzer, "Certifying algorithms", Computer Science Review 2011).
- **A specification can stand beside the program as a second, naive program** the program is held to: here the general
  context-free parser of Earley ("An efficient context-free parsing algorithm", CACM 1970), which finds every derivation
  of a text, over a scanner that tries every split of the text, so a test sees whether a text has none, one or several
  derivations without trusting the scanner under test.

## Decision

### D1. A text codec is a codec of one more encoding, and its value is any type of the language

`sce:encoding="grammar"` on a codec's root (`SCE_FORGE.md` §4.6) reads and writes a text by a grammar the document
declares (D2 to D5); `sce:encoding="json"` and `sce:encoding="json5"` read and write one by a fixed syntax and a binding
derived from a type (D8). The three are **text codecs**. A text codec names the type it reads and writes on its root,
`sce:type="..."`, any type of the one type grammar (a record, a union, a list, a map, a scalar, a `decimal(p, s)` of ADR
0162, a member of an inductive document of ADR 0164), so what it gives is a value a statechart holds, saves and computes
on; it declares no entries of its own, and without `sce:type` it is refused (`validation/missing-attribute`). A JSON
reply whose root is an array is a codec of a list.

A grammar codec may import another, and a rule may read the imported codec's root rule in place, as a byte codec embeds
another (`<sce:embed>`). The checks of D2 run on the composed plan, the imported rules in place, and a conflict between
the two names both codecs and the rule of each.

It is an encoding of the codec kind and not a kind of its own: what it does is what a codec does, a value read from and
written to a sequence of bytes by rules fixed at build time, and SCE_FORGE §10 ("When admission is borderline") names "a
scoped extension to an existing kind" as the form such a facility takes. The three axes hold of it all the same: its
texts and values are deterministic and the same on every lane that serves it; its grammar, its type and its tables are
build-time and only the text is runtime; and it serves network protocols (addresses, selectors, header fields), the
configuration of every system configured by a file, build and CI systems (manifests), GUI and scene state (a document's
saved text) and telemetry (a log line read into a record).

A text codec is the one place a document reads structure out of text. Logic computes on the value it gives: an address
cut at its separators is a record whose fields are its parts, a key's chunks are a list, a parameter list is a map, and
the text is written again by printing the value. Two spellings of one value read to one value, and `print(parse(t))` is
the one spelling a table keyed by text is keyed by (D7, law 2). A rewrite that is not a matter of spelling (an algebra
of the value) is the document's own algorithm over the typed value.

### D2. A grammar is LL(1) over regular tokens, and the build proves that it reads every text one way

A grammar document declares:

- **terminals**, each a regular language written in I-Regexp (RFC 9485, ADR 0149 D1):
  - `<sce:literal>`, a fixed text (`/`, `;`, a keyword), matched as written or, with `case="ascii-insensitive"`, in
    either case of its ASCII letters, and printed as written;
  - `<sce:token name regexp value>`, a token, whose lexeme one conversion of a closed set turns into a value (D3);
  - `<sce:skip name regexp print>`, a stretch read and forgotten (layout, a comment), which may be empty, printed as its
    `print` spelling (which a use may override);
- **rules**, `<sce:rule name type>`, each giving a value of one type, whose body is a sequence of terminals, rules and
  the binding forms of D3 and D5. The first rule is the root, of the type `sce:type` names.

The build reads the document into a **grammar plan**, one form for a written grammar and for the fixed syntaxes of D8,
and holds the plan to six checks. Each is a decision procedure over finite sets of rules, over automata and, for check
5, over the intersection of a context-free language with a regular one (Bar-Hillel, Perles and Shamir, "On formal
properties of simple phrase structure grammars", 1961); each refusal names the place and a witness text:

1. **No token matches the empty text** (`grammar/empty-token`). A skip may, and is then a part that can be empty under
   check 2. A token that no binding reads, whose lexeme would be forgotten without being declared a skip, is refused
   (`grammar/unbound-token`): what a grammar forgets is a skip, so law 3 can list it.
2. **LL(1)** (Aho et al. §4.4.3). At each choice the FIRST sets of the alternatives are disjoint; at most one
   alternative derives the empty text; where one does, its FIRST set and the FOLLOW set of the choice are disjoint; and
   the part a repetition repeats does not derive the empty text. Otherwise `grammar/not-ll1`, naming the rule, the two
   alternatives (or the part and what follows it) and a token both can begin with. A rule that reaches itself is read
   with a stack in one of the three forms of D5, and refused in every other case (`grammar/recursive-rule`); left
   recursion is not LL(1) and is refused by this check.
3. **One token at each point.** Where the parser stands, the terminals it may meet next (the FIRST set of what follows,
   with FOLLOW where what follows can be empty) are scanned by the longest match. Two of them whose languages share a
   text are refused (`grammar/terminals-overlap`, with the text) unless one is declared to win,
   `<sce:token ... over="other">`: the lexical precedence of `lex`, as a keyword wins over a name. The one that loses
   reads only what the winner does not, a regular language. This is Van Wyk and Schwerdfeger's context-aware scanning,
   which they give for LR parsers; its form for an LL(1) parser, the expected set read off FIRST and FOLLOW, is this
   record's.
4. **A linear read past each token.** In the deterministic automaton `Q_p` of the terminals expected at a point `p`
   (their union, by ADR 0149's construction), where the subgraph of the live states that accept nothing, restricted to
   those reachable from an accepting state, has no cycle, the scanner reads at most `k` bytes past the end of each
   token, `k` less than the automaton's states, and needs no workspace. The condition is sufficient for a linear longest
   match and not necessary (tokens `a` and `a*b` before a `;` read linearly though it fails), so where it fails the
   point is printed with Reps's tabulation: a table of `n * |Q_p|` bits that records the pairs of a position and a state
   already known to fail, `n` the codec's input bound (the type of the text a lift reads, the frame a boundary hands
   it), which keeps the longest match linear for every automaton (Reps 1998), as ADR 0149 D2 prints the form RE2 falls
   back to rather than refusing. The build reports the table's bits for each such point, and refuses a codec whose
   tables pass `grammar_scan_table_bits`, a bound by name whose library number is 65536 (8 KiB; no textbook gives a
   number, and it is named so a build moves it), as `grammar/scan-table-too-large`, naming the points, the bits and the
   bound.
5. **What the printer writes is read back as the same tokens.** For each terminal `t` and each point it is printed at,
   no printed lexeme of `t` followed by a non-empty prefix of any text the printer can write after it (across as many
   tokens as follow) is in a language expected at that point. The language after `t` is context-free and the languages
   expected are regular, so the check is CFL-reachability over the plan's rules with a summary relation per rule on the
   point's automaton (Yannakakis 1990; Reps 1998), in time of the order of `|G| * |Q_p|^3` per point and terminal, with
   no blow-up in the length of what follows. It cannot be cut to the `k` bytes of check 4, since a merge may pass
   through accepting states for as long as the text after `t` runs. It is `sce-build`'s, tested in step 6. Where the
   intersection is not empty, the author places a skip after `t` whose `print` spelling stops the merge, checked the
   same way; otherwise `grammar/tokens-merge`, with the terminals and the text that merges. It is Boomerang's
   unambiguous concatenation, extended across every token that follows. A literal's text, and each skip's `print`, must
   be in the language read where it stands (`grammar/canonical-spelling-not-read`).
6. **What the reader gives, the printer writes back.** At each binding, every value the reader can produce (the image of
   the conversion over the token's language, less what a winner takes) prints to a text the same terminal reads back
   there (`grammar/read-value-not-printable`, with the text read and the text printed). It refuses a token `[0-9]+`
   under a winning literal `7`, whose `07` reads 7 and prints `7`, which the literal then takes; and
   `case="ascii-lower"` on a token `[A-Z]+`, whose read value prints in a case the token does not read. For every
   conversion but `float` the image is the image of a regular language under a finite transducer, and is decided so. A
   float read correctly rounded and printed by its shortest digits is no rational transduction, so for a `float` token
   check 6 holds exactly when the token's language contains the whole language of the declared notation (D4's
   condition), and the grammar is refused otherwise. Without check 6 law 2 would fail.

The automata of checks 3 to 6 are built per point within `grammar_scanner_states`, a bound by name (ADR 0050) whose
library number is 4096, the number ADR 0149 gives `regexp_states` for the same textbook budget. A grammar whose automata
pass it is refused (`grammar/scanner-too-large`, naming the point, the count and the bound), not simulated as ADR 0149
D2 simulates a large regular expression, because checks 3 to 6 are decided on the deterministic automaton. Each token's
own expression is held to `regexp_nfa_states` (ADR 0149 D2), and a grammar whose classes are expanded from the Unicode
data declares `UNICODE_VERSION` as ADR 0149 D4 requires.

The operations the checks and the reader need beyond ADR 0149's are added to `sce-rust-regexp` (`backends/rust/regexp`,
landing item 481), each tested against its definition (Order of work, step 2), so the build and the runtime read tokens
through one crate. The crate stays `no_std` with no heap (ADR 0149, Scope): the reader's longest-match walk and Reps's
tabulation over a table the caller lends are in its core, and the build's operations (union, intersection, difference,
concatenation with the closure of prefixes, a witness text of a non-empty language, the cycle test of check 4, the
CFL-reachability of check 5 and the image of a conversion) are behind a feature that brings a heap, which only
`sce-build` turns on.

The build writes, with each plan, the certificates of its checks (the FIRST and FOLLOW sets, the order that shows check
4's subgraph acyclic or the tabulated points, the summary relations of check 5, the images of check 6), and the
reference of D14 checks the certificates rather than deciding the checks again (McConnell, Mehlhorn, Näher and
Schweitzer 2011).

Under checks 1 to 6 the grammar, read with its scanner, has at most one derivation of each text, its reader finds that
derivation with one token of lookahead and no backtracking, and what it reads it can print back (D7; T1 and T2 of D14).

### D3. A rule binds its parts to a value, and each binding is a partial isomorphism

| Form | Binds | Read | Printed |
|---|---|---|---|
| `<sce:field name="f" rule=".."/>` or `token=".."` | field `f` of the rule's record | the rule or token, into `f` | `f`, by that rule or token |
| a rule's body | a record: each field bound exactly once, in the order the document writes; a literal and a skip carry no value | in order | in order |
| `<sce:option field="f">` | `f` of `optional<T>`, present when its body is read | the body, when the next token is in its FIRST set | the body when `f` is present, nothing when absent |
| `<sce:list field="f">`, with an optional `<sce:separator>` | `f` of `list<T>[M..B]` | elements while one begins; past `B` `tooMany`, short of `M` `tooFew` | each element in order, separated |
| `<sce:map field="f" order="...">`, one `<sce:key>` and one `<sce:value>` | `f` of `map<K, V>[B]`, or of `set<K>[B]` with no value | each entry put; `order` is `read` or `by-key` (D10), with no default; a key read twice `repeated`; past `B` `tooMany` | `read`: in the map's own order (ADR 0108 D2); `by-key`: in the order of its keys |
| `<sce:members unknown-members="...">` of `<sce:member name=".." field="..">` | a record's fields by name, in any order (a permutation phrase) | a member's name, its assignment literal, then what it names, each at most once; a required one absent `missing`; one read twice `repeated`; an undeclared name refused (`refuse`: `unexpected`) or stepped over by a declared `<sce:skip-member>` (`skip`), with no default | the present members, in the order declared |
| `<sce:cases>` of `<sce:case name="c">` | a union's case, or an enum's variant | the alternative whose FIRST set holds the next token | the alternative of the value's case |
| `<sce:spellings>` of two or more `<sce:spelling>` | one value written more than one way | the alternative whose FIRST set holds the next token | the first alternative whose language holds the value |

**Members.** A member's name is one token of the `<sce:members>` (a name token, a string token, or either, as the
grammar says), and its converted value, after any unescaping, is compared with the declared names, so an escaped
spelling of a name is that name. The declared names must be distinct after conversion (`grammar/member-names-collide`,
naming both). Between the name and the member's value stands the members' `<sce:assign>` literal (`:`, `=`), between two
members their `<sce:separator>`, and `trailing="allowed"` admits a separator after the last. The dispatch from a name to
a member is a lookup in a table the build prints, not a choice among alternatives, so it adds nothing to check 2; checks
3 to 6 run on the name token and on each member's value as on any binding. A declared member read twice is found by one
bit per declared member; a stepped-over name is not compared with other stepped-over names in a grammar codec, so a
grammar's members cost no workspace (D8 states what JSON adds).

A token's `value` names one conversion of a closed set, each a partial isomorphism between the lexemes it reads and a
type, with one canonical spelling it prints:

- `text`: the lexeme itself, into a `string[B]`. With `case="ascii-lower"` it is read in either case and held in lower
  case. With `escapes` it is unescaped by one scheme, `json` (RFC 8259 §7), `json5` (JSON5 1.0.0 with ECMAScript 5.1's
  escapes), `quoted-pair` (RFC 9110 §5.6.4) or `c` (the escapes of C), read with any escape the scheme admits and
  printed with the one the scheme makes canonical (for `json`, `ARCHITECTURE.md` "JSON Text"). A text that holds U+0000,
  raw or escaped, is refused `nulInText`, as no text of SCE holds it (ADR 0087 §2.3);
- `decimal` and `hex`: an integer of the field's type, read with whatever sign and leading zeros the token's language
  admits, printed with none but a `-` below zero (`hex` in lower case); past the type `outOfRange`;
- `float`: a `float32` or `float64`, read correctly rounded once into the type's own format (D9), printed by the
  shortest digits that read back to the same value, in a notation the codec declares (D8);
- `fixed-decimal`: a `decimal(p, s)` of ADR 0162, read and printed as its canonical text (ADR 0162 D9);
- `hex-bytes`, `base64` and `base64url` (RFC 4648): a `bytes[B]`, printed in the RFC's canonical form.

The build checks that every lexeme of the token is one the conversion reads (`grammar/conversion-mismatch`, with a
lexeme it does not), and every binding against the field it binds (`grammar/binding-mismatch`: a field bound twice or
never, a list form on a field that is no list, a rule or a conversion of another type, a key of a type ADR 0108 D1 does
not admit, a binding inside a rule that declares no type). These are Rendel and Ostermann's combinators, under their
conditions: a rule's body is their product, with the record's constructor as its isomorphism; `<sce:cases>` and
`<sce:spellings>` are their choice, its alternatives told apart by check 2; a list and a map their repetition; a
conversion an isomorphism on one token, made one in both directions by check 6.

**An example: RFC 9110's media type** (§8.3.1), without the empty parameter RFC 9110 admits (`;;`) and without its
octets above 0x7F in a quoted string. Its record holds a map, which D10 admits. Media type and parameter names are
case-insensitive (RFC 9110 §8.3.1) and a parameter's position carries no meaning, so the map is read in the order of its
keys, and two texts that differ in the order of their parameters read to one value:

```xml
<!-- media_type.scxml, an event schema read as a record -->
<data id="type"       sce:type="string[media_name=127]"/>
<data id="subtype"    sce:type="string[media_name=127]"/>
<data id="parameters" sce:type="map&lt;string[media_name=127], string[media_value=1024]&gt;[media_params=16]"/>
```

```xml
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0"
       sce:kind="codec" sce:encoding="grammar" sce:codec-format="2" name="media_type_text"
       sce:type="record:MediaType" sce:encode-may-fail="true">
  <sce:import kind="event-schema" src="media_type.scxml" as="MediaType"/>
  <sce:token name="name"   regexp="[!#$%&amp;'*+.^_`|~0-9A-Za-z-]+" value="text" case="ascii-lower"/>
  <sce:token name="token"  regexp="[!#$%&amp;'*+.^_`|~0-9A-Za-z-]+" value="text"/>
  <sce:token name="quoted" regexp="&quot;([\t !#-\[\]-~]|\\[\t -~])*&quot;" value="text" escapes="quoted-pair"/>
  <sce:skip  name="ows"    regexp="[ \t]*" print=""/>
  <sce:rule name="media_type" type="record:MediaType">
    <sce:field name="type" token="name"/>
    <sce:literal>/</sce:literal>
    <sce:field name="subtype" token="name"/>
    <sce:map field="parameters" order="by-key">
      <sce:skip ref="ows"/><sce:literal>;</sce:literal><sce:skip ref="ows" print=" "/>
      <sce:key token="name"/>
      <sce:literal>=</sce:literal>
      <sce:value>
        <sce:spellings>
          <sce:spelling token="token"/>
          <sce:spelling token="quoted"/>
        </sce:spellings>
      </sce:value>
    </sce:map>
  </sce:rule>
</scxml>
```

`name` and `token` have one language and are never expected at one point, so check 3 holds; the skip before `;` can be
empty and `;` is not in its language, so check 2 holds; `" "` is in `ows`, and no lexeme of a token followed by a prefix
of `;`, `/` or `=` is a token's, so check 5 holds; `name` gives only values in lower case, which it prints and reads
back, so check 6 holds. `TEXT/HTML ;Charset="utf-8"` reads to
`{type: "text", subtype: "html", parameters: {charset: "utf-8"}}`, which prints as `text/html; charset=utf-8`: the case
of the names, the layout, the quotes the value did not need and the order of the parameters are what the grammar
forgets. A parameter value holding a control character other than a tab is in neither spelling's language, and a `type`
holding an ASCII capital is outside what `name` prints, so the codec's encode can refuse (D4) and says so on its root.

### D4. The printer's domain is computed, and what it cannot print is a refusal the build finds

A value of the type may have no text in the grammar: a text outside its token's language (a name holding a space), a
text a winning literal takes (check 3), an integer the token does not spell, a value outside a conversion's image (an
ASCII capital under `ascii-lower`), a float that is not finite where the encoding has no spelling for it (D8). For each
binding the build decides whether every value of the field's type prints into the language read there, and where it does
not, the printer refuses such a value, `notInLanguage` at that place. How it decides depends on the type:

- **an integer**, by inclusion: the canonical numerals of the type's range are a finite regular language, and the
  binding prints every value when that language is included in the token's (the language a token reads need not be a
  union of intervals: `[0-9]*[02468]` reads the even numbers). Where it is not included, the printer's refusal walks the
  printed numeral through the token's automaton, and `notInLanguage` is the walk's failure;
- **a float**, by whether the token's language contains the whole regular language of the declared notation, with the
  spellings of the values that are not finite where the encoding has them;
- **a text or a byte string**, by the image of the conversion's printer over every text within the type's bound: the
  escape scheme is a finite transducer, the image of a regular language under it is regular, and it is compared with the
  token's language;
- **an enum, a bool, a union's case**, by each alternative's literal;
- **a raw place** (D11), always: its text is held as a `string[B]` a document may have written, so the printer reads it
  with the encoding's syntax plan and its normalization before it writes it, and refuses one that is not a value of the
  encoding in its normalized form, `notInLanguage` at the raw place. Printed as held, a field holding `1,"admin":true`
  would write a member the type never had, and law 1 would fail. A codec with a raw place therefore declares
  `sce:encode-may-fail="true"`.

These are places an encode refuses (ADR 0065 §2), held to the codec's `sce:encode-may-fail` by `codec_totality` as every
such place is (`codec/undeclared-encode-refusal`, `codec/encode-may-fail-without-refusal`), and the predicates of D15's
model. A codec that can print every value is total, and its printer returns only its sink's error (ADR 0065).

### D5. A rule may reach itself in three forms, each with its stack bounded by name

LL(1) reads recursion that is not left recursion with a stack as deep as the nesting (Aho et al. §4.4.4), so what needs
a bound is the stack, not the grammar. A rule that reaches itself, directly or through others, is admitted in exactly
three forms; every other cycle is `grammar/recursive-rule`, naming the cycle:

1. **A cycle that binds no value**, under a depth bound: `<sce:rule name="comment" depth="comment_depth=16">` declares
   no type, binds nothing (`grammar/binding-mismatch` otherwise), and is read with at most that many nested frames; one
   more is refused `tooDeep`, with the bound. The bound is a bound by name, or one the build computes and states: the
   generic value D8's syntax plan reads is of this form, its bound `depth(T)` (ADR 0087 §2.1 item 4). Nested comments,
   skipped brackets and a value stepped over are of this form too. A use of such a rule prints its `print` spelling,
   which must be in the rule's language (check 5).
2. **A guarded cycle that binds a value of a recursive type**, exactly where the type is a member of an inductive
   document of ADR 0164: each rule on the cycle is of a member type, the cycle follows the member's own recursion, and
   every path around the cycle passes a binding that allocates a node of the member. A cycle with a path that allocates
   nothing (a rule that passes its value through, a group that only wraps) is refused as `grammar/recursive-rule`,
   naming that path, because its frames are not counted by any node. The reader's stack is then at most the member's
   node bound times the most frames one node's rules open, plus the bound of each group of form 3 (and a depth bound,
   where ADR 0164 gives a use one); the node past the bound is refused `tooManyNodes` before it is stored, as ADR 0164
   D4 refuses it, and the value is built into the member's arena. A schema language's nested declarations, a payload's
   nested fields and a configuration document's value tree, the three nested rows of the census, are of this form.
3. **A chain of operators**, `<sce:chain type="union:Expr" operand="atom">` with
   `<sce:operator literal="+" case="add" precedence="1" assoc="left"/>` entries and one
   `<sce:group open="(" close=")" depth="expr_group_depth=32"/>`: the build expands it into the standard LL(1) grammar
   of one rule per precedence level (each level its operand level and a repetition of an operator and an operand, Aho et
   al. §4.3.3), and the reader folds each level's repetition into the tree by its associativity, as Rendel and
   Ostermann's `chainl1` folds through a `foldl` isomorphism. A group allocates no node (`((((x))))` is one node), so it
   declares its own depth bound by name, and a group nested past it is refused `tooDeep`. The printer adds a group only
   where the tree's shape differs from what precedence and associativity would read (Ramsey 1998), so the parentheses a
   text did not need are what the grammar forgets (law 3). The type is a member of an inductive document, each
   operator's case a record of two operands, so every operator allocates a node and the chain's cycle is guarded but
   through its groups, whose bound counts them. Two operators of one literal, a precedence given twice with two
   associativities, an operator literal in the operand's FIRST set and a group with no depth bound are refused
   (`grammar/chain-ill-formed`).

Forms 2 and 3 need ADR 0164's arena and land after it (Order of work); form 1 needs nothing. No row of the census reads
an expression of operators: form 3 is decided now because the textbook answer to a left-recursive expression grammar is
the one above, and a grammar facility with no answer for it refuses the first expression syntax it meets for a reason
the textbook removes.

### D6. One reader, one printer and one binder over the plan, in time linear in the text, and bounded space

- **The plan is data, and one walker reads it in each direction.** `sce-build` prints a codec's plan as tables: the
  scanner's deterministic automata by point (with Reps's table where check 4 tabulates), the LL(1) prediction table, the
  binding sites and their conversions, and, for JSON and JSON5, the binding of D8. `sce_forge_runtime::grammar` holds
  one reader, one printer and one binder for every plan, as ADR 0149 D2 prints one walk for every automaton and ADR 0155
  gives every cold codec one walker. The generated code is the plan, and a sink for the type with one method per binding
  site, which builds the value as the reader drives it and presents it as the printer asks.
- **A hot codec is printed as code**, once its proof is. A text codec the build names hot (ADR 0139) is printed as code
  from the same plan, its body reads inlined where its hot sites hold them (ADR 0140), with a Verus specification over
  the plan that Verus checks for each printed codec, as ADR 0164's T1b is checked per document (D14). Until that form
  exists no hot text codec is printed, and every codec is walked; whether a codec is hot is measured where it is read,
  as ADR 0139 decides it.
- **The order of refusals.** A grammar codec refuses at the least byte offset where the text and the plan disagree, the
  first in the order of the text. A JSON or JSON5 codec keeps the order ADR 0087 §2.1 item 5 fixes for every reader, as
  this record amends it: within the type's refusals, the one at the least byte offset (D8).
- **Time.** The reader takes each token by the longest match with at most `k` bytes read past it, or by Reps's table
  (check 4), and between two tokens makes at most a number of predictions the plan bounds. Its work on a text of `n`
  bytes is at most `c * n + d`, where `c` and `d` depend only on the plan's shape (its states, its rules, the longest
  chain of predictions between two tokens and the greatest number of passes D8's spans make), never on a bound of a type
  or of the text, so the statement is not made true by a bounded text. Holding names distinct adds no term:
  - a record's declared members are held distinct by one bit each; the names a type admits without declaring them
    (members stepped over, a raw place's object, another case's members, a span kept for a later tag) go into a trie of
    their decoded bytes, its children of each node a sorted array of at most 256, freed in stack order when the object
    closes, in time linear in the names' bytes (Knuth §6.3); a map's keys read as an object are held distinct by the
    map;
  - a map's puts cost what ADR 0108 D8 states: expected constant time under the map's keyed hash, whose seed a host
    without entropy passes, and up to the map's bound per put in `Linear` storage (ADR 0108 D8, as amended 2026-11-22),
    which the build states for such a map;
  - a map read `by-key` is relinked in the order of its keys when it closes: by a radix sort for a text or an integer
    key, linear in the keys' bytes (Aho, Hopcroft and Ullman 1974 §3.2), and by a merge sort of its links otherwise,
    `O(L log m)` for `m` keys of `L` bytes, a term the build states for such a map.

  The printer writes each part once.
- **Bounded space, and no heap.** The reader's stack is the plan's deepest derivation where no rule reaches itself, and
  the bound of D5's form otherwise; what it builds is the value, in the storage its type is held in (ADR 0077 §7, ADR
  0108 D4, ADR 0154, ADR 0164 D5). Its workspace is computed per plan: one trie node per decoded name byte, only where
  the type admits a name it does not declare, and none elsewhere; Reps's tables where check 4 tabulates; a radix sort's
  buckets where a map is read `by-key`. The printer writes into a sink, and the longest text it can write, `MAX_TEXT`,
  is a fold of the plan the build computes (each token's longest canonical lexeme from the type's bounds, each list and
  map at its bound), as `J(T)` is for JSON; a buffer of that size never overflows, and a counting sink measures a value
  first.
- **A value may be read as a view of its text.** Beside the owned value, the build generates the type's view: a text
  token read with no escape and no conversion of case is a span of the input, and one read with either is a span and a
  mark, converted into storage the caller lends when it is asked for; a raw place (D11) is a span and a mark, its
  normalized text written when it is asked for; a list, an option and a union are the view of their parts; a map is its
  entries' spans in its order, looked up by a scan, or by binary search where it is read `by-key`. A view borrows the
  text and does not outlive it: on Rust by its lifetime, on C11 by a pointer the caller keeps valid, which the generated
  header states. It is the form ADR 0049 gives a decoded byte codec, and its law is Wadler's: the value materialised
  from the view of a text `t` is `parse(t)` (D14). A view exists only of a text the reader accepted whole.
- **The text read is a `string`**, well-formed UTF-8 (ADR 0149 D8). At a boundary that hands the reader bytes, it holds
  them to RFC 3629's grammar first, and a byte sequence that is not UTF-8 is `unexpected` at its first ill-formed byte.
- **A refusal is one record** (D12): its code, the byte offset of the refusal, and the place in the value being read, as
  an RFC 6901 pointer. Nothing of a refused text is written into the place the reader was asked to fill.

### D7. The laws

For a text codec whose plan passes D2's checks, `parse` its reader and `print` its printer. **Equality** of values is
observational equality of their terms: the same constructors with equal parts, a list's and a map's entries in their
order, a float by its bits with the one NaN of ADR 0101. It is finer than ADR 0151 D2's total comparison, which compares
a map by its keys' order, and implies it.

1. **Round trip.** For every value `v` of the type that `print` accepts (D4), `parse(print(v)) = v`, up to the one
   equivalence on values the grammar may declare: a map read `by-key` (D10) is the same value in any order of its
   entries, and its printer writes the order of its keys. For a value the reader gave, which holds such a map in that
   order, the equality is exact.
2. **One spelling.** For every text `t` that `parse` accepts, `parse(print(parse(t))) = parse(t)`: `print(parse(t))` is
   the one spelling of `t`'s value, and printing after reading is idempotent. It follows from law 1 because check 6
   makes every value the reader gives one `print` accepts.
3. **The quotient.** `print(parse(t))` differs from `t` only by what the grammar declares it forgets: the text a skip or
   a value-free rule read, the case of what is read without regard to case, an escape's spelling, a number's spelling
   and notation, which of several spellings was read, the order of a `<sce:members>`'s members, a member stepped over,
   the order of a map read `by-key`, a group a chain did not need, and, in a raw place, what its normalization removes
   (D11). Nothing else of `t` is lost.
4. **One way.** Each text has at most one derivation in the plan, and `parse` is a function of the text.
5. **Linear.** `parse` does at most `c * n + d` steps on a text of `n` bytes, a JSON text's names held distinct within
   them, with the cost of a map's puts and of a `by-key` map's relinking stated beside them where the plan has a map
   (D6), and `print` at most `c' * m + d'` on a text of `m` bytes it writes; the constants depend only on the plan's
   shape.

These are ADR 0058 §5's law 1 (totality and the round trip) for a codec of text, stated for every value and every text,
and D14 says at which level each is judged.

### D8. JSON and JSON5 are each one fixed syntax, and a type's binding to it is derived by a fold

A JSON text is not read by a grammar derived from its type, because JSON's language is not a type's: a member a type
does not declare holds any JSON value, a member is chosen by the decoded value of a string (an escaped spelling of a
name is that name), a union's tag may come after its case's fields, and the refusals `wrongKind` and `nullNotOptional`
exist only because any JSON value is read first and then held to the type. So each encoding is two parts:

- **The syntax plan**, one for JSON (RFC 8259) and one for JSON5 (The JSON5 Data Interchange Format 1.0.0, 2018),
  written once in `sce-build` as a grammar plan, checked by D2 and proved by D14 like any other. It reads any value of
  the encoding: objects and arrays by a cycle of D5's first form, its depth bound the `depth(T)` the pre-scan below
  fixes; strings by the `json` or `json5` escapes; a number as one token whose lexeme is kept with its syntax class (its
  sign, its fraction, its exponent, and for JSON5 its hexadecimal form, `Infinity` and `NaN`); JSON's whitespace, and
  JSON5's white space and comments, as skips. JSON5's member names are identifiers of ECMAScript 5.1 (letters by the
  General Categories I-Regexp names) or strings in single or double quotes, and it admits a trailing comma. Read, it
  emits the events of its derivation: an object's or an array's beginning and end, a member name's span with its decoded
  bytes, and a scalar's span with its syntax class.
- **The binding**, derived from the type by structural recursion over `ValueType` beside `S(T)`, `G(T)` and `J(T)` in
  `value_type.rs`: one exhaustive match whose arms are the constructors (with ADR 0162's decimal and ADR 0164's
  inductive member), so a constructor added to the type grammar is a compile error until the fold handles it (ADR 0087
  §1.1). It is Jansson and Jeuring's polytypic reading, one rule per type constructor, and it runs as a deterministic
  pushdown transducer over the syntax plan's events, its stack the type's places: the binder of D6. It classifies each
  JSON value at its place against the type there:
  - a record or a union is an object, its members found by their decoded names in a table; a union's tag is read first
    wherever it stands (ADR 0092 §4), and a member read before the tag is kept as a span and bound once the tag is
    known, so a byte is read at most one more time than the nesting of unions whose tag comes after their fields, a
    number of the type, not of the text; a field absent from the text is filled from its schema's revision defaults (ADR
    0044) and is `missing` only where none fills it;
  - a list is an array; a map is D10's form; an optional is its element or `null`; an inductive member is ADR 0164 D4's
    form, read by this arm, which is the streaming reader ADR 0164 D4 gives Rust (D13);
  - an integer of up to 32 bits is a number with no fraction, no exponent and no `-0` (`1.5`, `1e2` and `-0` are
    `badNumber`); a 64-bit integer is the text of its canonical digits, or a number of magnitude at most 2^53 - 1
    (`ARCHITECTURE.md`, "JSON Number Text"); past the width `outOfRange`;
  - a float is read as D9 says; a decimal as ADR 0162 D9 says; a byte string is the Latin-1 text of its bytes
    (`notByteText` above U+00FF); an enum is its variant's name (`unknownVariant`); a `bool` is `true` or `false`;
  - a value of another JSON kind is `wrongKind`, and `null` where the type is not optional `nullNotOptional`.

**JSON5's numbers in typed places.** In an integer's place, a hexadecimal number and a leading `+` are read (each spells
one integer exactly), a number written with a point (`.5`, `5.`, `5.0`) or an exponent is `badNumber` as in JSON, and
`Infinity` and `NaN` are `badNumber`; a 64-bit integer is read as a number only up to 2^53 - 1 in magnitude, as in JSON.
In a float's place every JSON5 number is read, a hexadecimal one rounded once into the format, `Infinity` and
`-Infinity` to the infinities and `NaN` to the one NaN of ADR 0101. `tests/json_text/json5_refusal.json` holds these
cases beside `payload_refusal.json`.

**The order of the refusals** is ADR 0087 §2.1 item 5's, the same on every reader, as this record amends it: a text that
is empty or only whitespace is `notJson`; then a pre-scan over the brackets refuses a container opened deeper than
`depth(T)` as `tooDeep` (a raw place of D11 counting as its own bound of levels, and an inductive member as its
`depth(...)` of ADR 0164 D4); then one pass reads the whole text by the syntax plan, `notJson` for a text that is not of
the encoding (a repeated name in any object included, at any depth); then `notObject` for a root of a record or a union
that is not an object (`wrongKind` at the empty pointer for a root of another type); then **the refusal of the type at
the least byte offset**: each refusal's offset is where its value begins, a missing field's the offset of its object's
closing `}`, a refusal inside a span kept for a later tag its own offset in the text; the binder keeps the least, not
the first it finds, and goes on validating the syntax. The rule is the one D6 gives a grammar codec, it gives the
record's `offset` its meaning, and a streaming reader computes it without holding the text. ADR 0087 §2.1 item 5 stopped
at "then the fields", `payload_refusal.json` says no case depends on the order, and today's lifts walk a parsed tree in
the order of their generated code; this record amends the item with the sentence above, and every lane's lift moves to
it (Order of work, step 5). Both passes, and the re-reading of spans, are counted in D6's constants, and the names of
each open object are held distinct as D6 states (a trie of the names the type does not declare, a bit for each it does),
which is what a repeated name anywhere requires.

**It reads what the lift reads.** With `sce:unknown-members="skip"`, and with no `<sce:raw>` or `<sce:within>`, the JSON
codec of `T` reads exactly the language every lane's lift reads for `T` (ADR 0086 §4, ADR 0087 §2.1, ADR 0092 §4), with
the same revision defaults, a member of another case of a union stepped over (`"radius":null` under a case with no
radius), and refuses each text with the code, pointer and bound `tests/json_text/payload_refusal.json` gives it: that
table is its conformance table. The codes it answers are the lift's (`notJson`, `notObject`, `tooDeep`, `missing`,
`nullNotOptional`, `wrongKind`, `badNumber`, `outOfRange`, `tooLong`, `tooMany`, `tooFew`, `unknownVariant`,
`notByteText`, `nulInText`, and ADR 0164's `tooManyNodes`), `unexpected` for a member the type does not declare under
`"refuse"`, and `repeated` for a key read twice in a map's array form (D10). `sce:unknown-members` is required on a JSON
or JSON5 codec and has no default (`validation/missing-attribute`): it changes the language read, whether a text read
from outside may carry what its type does not name is the document's decision, and the lift's choice was made for a
schema's later revisions (ADR 0044), not for a file a person writes.

**It prints in the house's order and notation unless the codec declares another.** Both are canonizers of law 3's
quotient: they choose how a value is printed and change nothing that is read, so each defaults to the form
`ARCHITECTURE.md` fixes, and is declared only to match another producer's bytes:

- `sce:member-order`, by default `"sorted"`: a record's members ascending by their UTF-8 bytes (`ARCHITECTURE.md`, "JSON
  Object Key Order"), a union's tag among them; `"declared"` writes them in the order the record's document declares
  them, a union's tag first, the order ADR 0087 §1.1's schema and grammar write;
- `sce:float-notation`, by default `"ecmascript"`: the digits are always the shortest that read back to the same value
  in the value's own format (Steele and White 1990), and the notation places them. `"ecmascript"` places them by
  ECMAScript's Number::toString rule, from the shortest digits' count `k` and exponent `n` (decimal when `-6 < n <= 21`,
  that is `1e-6 <= |x| < 1e21`, an exponent otherwise), computed for a `float32` from its own binary32 digits and not a
  binary64's, since Number::toString is stated for binary64 alone; `"positional"` never writes an exponent. The set is
  closed so every lane prints the same bytes; a notation another producer uses joins it by an amendment that states its
  rule;
- under the defaults the printer writes what ADR 0086 §3.1's typed payload writer writes, as D9 pins a `float32` for
  every writer (a text as "JSON Text" writes it, a 64-bit integer as its digits, a byte string as its Latin-1 text, an
  absent optional as `null` under its key), except where that spelling is not invertible, and only there:
  1. a negative zero is printed `-0`, which ECMAScript writes `0` and reads back as a positive zero, where ADR 0101
     holds the two apart;
  2. a float that is not finite has no JSON spelling and is refused (`notInLanguage`, a place of D4), where "JSON Number
     Text" writes `null`, which reads back as an absent value.

  "JSON Number Text" gains these two as the text codec's rows, and the three tables of `ARCHITECTURE.md` gain the codec
  as a writer, on Rust and on C11.

**JSON5** prints what the JSON codec prints, and a float that is not finite as `Infinity`, `-Infinity` or `NaN` (the one
NaN of ADR 0101 §2.1), so it refuses no float; JSON is a subset of JSON5, so the round trip holds. A JSON5 text that is
not of the encoding, a repeated name included, is `notJson`, which for a JSON5 codec reads "not a text of its encoding".

**An open enum** names no text for a number no variant declares, and a JSON or JSON5 codec whose type reaches one is
refused (`grammar/open-enum`), as ADR 0087 §1.1 refuses its grammar. An inductive member is read and printed in the form
ADR 0164 D4 gives it, the node past its bound `tooManyNodes`.

```xml
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0"
       sce:kind="codec" sce:encoding="json5" sce:codec-format="2" name="node_config_text"
       sce:type="record:NodeConfig" sce:unknown-members="refuse" sce:member-order="declared"
       sce:map-order="read" sce:encode-may-fail="true">
  <sce:import kind="event-schema" src="node_config.scxml" as="NodeConfig"/>
  <sce:import kind="codec" src="endpoint_text.scxml" as="EndpointText"/>
  <sce:within at="/listen/*" codec="EndpointText" bound="endpoint_text=256"/>
  <sce:raw at="/extensions/*"/>
</scxml>
```

Here `NodeConfig` holds `mode`, an `enum:Mode`; `listen`, a `list<record:Endpoint>[listen_max=8]`; and `extensions`, a
`map<string[extension_name=64], string[extension_text=4096]>[extensions_max=16]`. The text
`{ mode: 'peer', listen: ['tcp://0.0.0.0:4000'], extensions: { store: { volumes: 2 } }, }` reads to a value whose
endpoint is a record the grammar codec `EndpointText` read out of the string (D11), and whose `store` extension is held
as the text `{"volumes":2}` until a codec of its type reads it (D11). It prints as
`{"mode":"peer","listen":["tcp://0.0.0.0:4000"],"extensions":{"store":{"volumes":2}}}`. Its encode can refuse, since a
raw place is held to its encoding when it is printed (D4).

### D9. Every reader rounds a `float32`'s decimal once into binary32, and every writer writes its own shortest digits

One rule for every reader of a `float32` from text, the lift of every lane and every text codec: the decimal is rounded
once, correctly, into binary32, to nearest with ties to even (Clinger 1990), and a decimal whose magnitude is at least
`2^128 - 2^103`, the binary32 overflow threshold of IEEE 754-2019 §4.3.1 (the midpoint between `f32::MAX`,
`2^128 - 2^104`, and `2^128`, which rounds to infinity), is refused `outOfRange`. A `float64` is read the same way into
binary64, a decimal that rounds to infinity refused as today.

And one rule for every writer of a `float32` into JSON text, the typed payload writer of every lane and every text
codec: the shortest digits that read back to the same binary32 under the rule above, placed by D8's `ecmascript`
notation from the binary32's own digits. "JSON Number Text" pins it as its own row, and its bullet "A `float32` is
written as its platform spells it; only the 64-bit form is pinned" is deleted.

Today's lift on Rust (`PayloadValue::float32`) and on the lanes that follow it reads through a binary64 and refuses a
double above `f32::MAX`, and the writers spell a `float32` as their platforms do. Three things change:

- a text that rounds differently once and twice reads to the nearest binary32: `1.000000059604644775390625000001` is
  `1+2^-24` as a binary64 and then `1` (bits `3f800000`) by ties to even, where rounded once it is `1.0000001` (bits
  `3f800001`);
- a text between `f32::MAX` and the threshold reads to `f32::MAX` instead of being refused: `3.4028235e38`, the shortest
  spelling of `f32::MAX`, is above it as a real. Kotlin's payload writer writes this value today (`EventPayload.literal`
  spells `3.4028235E38`) and Rust's lift refuses it: D9 mends that defect;
- every writer spells a `float32` alike, where today each writes its platform's spelling.

`tests/json_text/float32_text.json` joins `real_text.json`: each reading case a text and the binary32 bits it reads to
or its refusal, the cases above among them, read by every lane's lift and by the codec; each writing case the bits of a
binary32 and the text every writer writes for it, written by every lane's payload writer and by the codec.

### D10. A record may hold a map where a text codec reads it, and a map's JSON form

This amends ADR 0108 D7 and decides its Open question 1 for the text encodings.

- **Where.** An event schema's field may be a `map<K, V>[B]` or a `set<K>[B]` where the schema is the type of a text
  codec, of a `sce-static` variable, or of a `<sce:lift>` or `<sce:print>` location, on a lane that serves text codecs
  (D16). Every other position (a delivery's payload, a CBOR codec, a signed value, a send's data) keeps refusing such a
  schema as `validation/map-in-wire-position`, which then names the position and the field. The CBOR form of a map stays
  ADR 0108's question.
- **The order read.** A map is a value whose entries are in an order (ADR 0108 D2), so the order a reader puts them in
  is part of what it gives. `read` puts them in the order of the text, and the round trip keeps it. `by-key` puts them
  in the order of their keys by ADR 0151 D2's total comparison of the key's type (a text by its code points, an integer
  by its number, an enum by its declaration), relinked when the map closes (D6), and prints them in that order; the
  text's order is then forgotten (law 3), and two texts that differ only in it read to one value. A grammar's
  `<sce:map>` states its order (D3); a JSON or JSON5 codec whose type reaches a map states `sce:map-order` on its root,
  and `<sce:map at="..." order="...">` for a place that differs. Neither has a default: whether a table's order means
  something is the document's to say, and a protocol whose parameters are unordered (RFC 9110's media type, an
  endpoint's metadata sorted by key) is read `by-key`, so a table keyed by its printed text has one key per value.
- **The JSON form.** A map whose key is a text, an enum or a byte string is an object keyed by the key's text: the text
  itself, the variant's name, or the byte string's Latin-1 text, the form ADR 0086 §3.1 gives each. A map with any other
  key (an integer, a `bool`, a record, and the bounded list ADR 0164 D10 makes a key) is an array of two-element arrays,
  `[key, value]`: an integer's decimal digits would be an object key that ECMAScript's `JSON.parse` moves before every
  other key, so an object would not keep the map's order in the reader most JSON meets. A set is an array of its keys. A
  key read twice is refused: in an object as every repeated name is (`notJson`), in an array of pairs or of keys
  `repeated` at its place. This differs from a map's saved form, which ADR 0108 D8 writes as an array of entries
  whatever its key: the saved form is read by SCE alone, and a text codec's form is read by others.
- **What an object of a map relies on.** RFC 8259 §4 calls an object's members unordered, and a reader outside SCE may
  drop their order; a map printed as an object keeps it in its text, and a reader that drops it reads a `read` map into
  another value. `ARCHITECTURE.md`'s "JSON Object Key Order" gains the exception: a map's object is written in the map's
  order, and a record's in its codec's `sce:member-order`.

### D11. A value of unknown type is held as its text, and a string may hold another codec's text

- **A raw place.** A JSON or JSON5 codec may name places whose value its type does not state, `<sce:raw at="..."/>`,
  each a field of `string[B]` (`grammar/raw-type` otherwise). `at` is an RFC 6901 pointer into the type, in which `*`
  stands for every element of a list and every value of a map. The reader reads any value of the encoding there, its
  nesting held to `B / 2` levels by the pre-scan (one level takes at least two bytes), and keeps its normalized text: in
  the codec's encoding, with no skip and no trailing comma, every name and string double-quoted in the encoding's
  canonical escapes, names in the order read, a number as written, a repeated name refused (`notJson`), a text holding
  U+0000, raw or escaped, refused (`nulInText`, at its place, since every later reading would refuse it), and `tooLong`
  past `B` bytes. Under JSON5 the held text is JSON5. The printer reads the held text by the syntax plan and its
  normalization before it writes it, and refuses one that is not normalized text of the encoding (`notInLanguage`, a
  place of D4), since a document may have written the field. Normalized, not canonical: two texts of one value whose
  numbers are spelled differently are held as two texts, since the type that would say what a number is is not known
  there.
- **It is read later by a codec of a type**, by `<sce:lift>` (D12), and nothing walks it before: no untyped tree is
  handed to logic, and every value a document computes on is a value of a type, as ADR 0101 has it, with no new
  constructor of the type grammar for ADR 0092 §12 to hold on every lane. It is the form a raw JSON value has in other
  typed readers (Go's `json.RawMessage`, Rust's `serde_json::value::RawValue`), and a document whose logic needs a tree
  of JSON declares that tree as a member of an inductive document (ADR 0164's own example is a configuration value),
  which the fold reads and prints.
- **A text within a text.** `<sce:within at="..." codec="Alias" bound="name=N"/>` reads the JSON string at each place
  `at` names by another text codec, into a field of that codec's type. `bound`, a bound by name, is the most bytes the
  unescaped string may hold (`tooLong` past it) and the buffer it is unescaped into; on the way out the inner value is
  printed, at most the inner codec's `MAX_TEXT`, and escaped. A field of another type, or a `MAX_TEXT` past `bound`, is
  `grammar/within-type`. Both laws compose, as two lenses composed are a lens (Foster et al. 2007); a refusal inside the
  string is the inner codec's, its pointer the place of the string, its offset in the outer text.

### D12. A statechart reads a text into a value, at a place, and prints a value

- **Reading.** `<sce:lift>` (ADR 0087 §2.1) takes `codec="Alias"`, an imported text codec, in place of `schema`: `text`
  is read by that codec, and `location` is of the codec's type or an optional of it (`validation/lift-codec` otherwise).
  Everything else is as ADR 0087 §2.1 states it: the statement raises nothing, and writes `location` whole or `refusal`,
  never both.
- **A place, by a path of typed steps.** `at="name"` names a `list<union:PlaceStep>[B]`,
  `sce:std/text/place_step.scxml`, whose cases are a field's name, a list's index, `append`, a map's key given as its
  text, and a match: a list of records' element chosen by a field whose type is a key (ADR 0108 D1) and a text of the
  field's value. The text is read as a value of the type at that place, and that place alone is replaced: a field of a
  record; a field of a union's case while the union holds that case; a list's element within its length, or a new one at
  its end; a map's key, put; under a match, the first element in the list's order whose field equals the step's value,
  or, when none does, a new element at the end. A step's text is read by one rule: in a map read as an object, it is the
  member's name unescaped; everywhere else it is read at the key's or the field's type, by the binding site's conversion
  in a grammar codec and by the fold at that type in a JSON or JSON5 codec. A value written under a match must hold the
  matched value in its key field, so that reading the place back gives what was written (a lens's PutGet, Foster et al.
  2007); one that holds another is refused `keyMismatch`, its pointer the key field, and nothing is written. A path that
  names no place of the value is refused `noSuchPlace`, the refusal's pointer the steps that did resolve; an append to a
  full list or a put into a full map `tooMany`. Steps that pass through a raw place (D11) name no place of the type and
  are `noSuchPlace` at the raw field: the pointer and the type tell that case from a misspelt name, so one code is one
  fact, "the type has no place there", as ADR 0092 §4 keeps one code per fact.
- **The path is a value, read from text by a codec.** `sce:std/text/json_pointer.scxml` is a grammar codec of this
  record that reads an RFC 6901 pointer into the steps (`~0` and `~1` unescaped, an index or `-` under a list); a
  configuration's own path syntax, as one that picks an element by a field's value, is a grammar codec of its own into
  the same type. The match is the step of a dictionary lens (Boomerang 2008) and of a JSONPath filter selector on
  equality (RFC 9535 §2.3.5), restricted to one key field so its meaning is equality of one value. Each step resolves by
  a lookup in the tables the build prints for the type's places, and a match by a scan of the list.
- **Which reader reads at a place.** In a grammar codec each place of the type has one binding site in the plan, the
  case of a union fixed by the value's current case, and the text is read by the plan from that site; in a JSON or JSON5
  codec, by the syntax plan and the fold at the type of the place. The value is built in a scratch of the place's type,
  which the generated machine reserves once for each type a statement reads at a place (so a `no_std` machine holds it
  in its own storage), and moved into the place only when the read succeeded whole.
- **Why a place.** The census's rows of a configuration written at a path (a key's subtree replaced, a plugin's section
  merged, a runtime write into a document a person wrote) read one place of a large value from a short text; reading the
  whole value again for each write would cost the value's size and refuse a write for a part it did not touch.
- **Printing.** `<sce:print codec="Alias" expr="..." location="text" at="name" refusal="why"/>` writes the text of the
  value of `expr` (or of the place `at` names in it) into a `string[N]` (`validation/print-location` otherwise). It is
  total exactly where the build proves it (ADR 0133): the codec's printer refuses nothing of the value's type (D4), its
  `MAX_TEXT` is at most `N`, and there is no `at`. Then a `refusal` is refused; otherwise one is required, an
  `optional<record:LiftRefusal>` (`validation/print-refusal` for either), and the statement writes `location` or the
  refusal, never both.
- **The refusal record and its codes.** `sce:std/lift/refusal.scxml` gains, in its second revision (ADR 0044), `offset`,
  an `optional<uint32>`: the byte offset of the refusal (D6, D8: where the refused value begins, a missing field's the
  offset of its object's end), present for a statement through a text codec and absent for a lift by `schema`, so a lift
  by schema writes the same record on every lane, as today, and a record saved at the first revision is restored at the
  second with `offset` absent. `sce:std/lift/refusal_code.scxml` gains five codes, numbered after ADR 0164's
  `tooManyNodes` (16; `tooFew` is 14 and the harness's `notOffered` 15, ADR 0086 §15.11), with `sce:since` the enum's
  revision after ADR 0164's; `sce-build/tests/fixtures/static_datamodel/enum_harness_code.scxml` and
  `tests/host_features/model_outcome_classes.json`, which repeat the lift's codes, gain them at the same numbers:
  - `unexpected` (17): where the reader stands, no token it expects; the text ended where one was expected; text after
    the value; a member the type does not declare, under `"refuse"`, its pointer the object that holds it (a name the
    type does not declare has no bound, and the object's place has one). A JSON or JSON5 text that is not of its
    encoding stays `notJson`, as ADR 0087 §2.3 classes it, so the shared table holds;
  - `repeated` (18): in a grammar codec, a declared member or a map's key read twice; in a JSON or JSON5 codec, a key
    read twice in a map's array of pairs or a set's array (D10);
  - `notInLanguage` (19): a value the printer has no text for (D4);
  - `noSuchPlace` (20): a path that names no place of the value;
  - `keyMismatch` (21): a value written under a match whose key field does not hold the matched value.

  Every other refusal keeps its code (`notJson`, `notObject`, `tooDeep`, `missing`, `nullNotOptional`, `wrongKind`,
  `badNumber`, `outOfRange`, `tooLong`, `tooMany`, `tooFew`, `unknownVariant`, `notByteText`, `nulInText`,
  `tooManyNodes`). A text codec bound at a boundary (ADR 0102, ADR 0111) is bound as any codec is, and its decode's
  failure (ADR 0099) is the same code, offset and pointer.
- **The pointer's length.** A refusal's `path` is a `string[lift_path=256]` (ADR 0087 §2.4). For a lift through a text
  codec the build computes the longest pointer into the type with a map's keys at their bound, each `~` and `/` written
  as two bytes, and refuses a lift whose longest pointer passes `lift_path` (`validation/lift-path-bound`), as ADR 0087
  §2.4 refuses one for a schema, so no pointer is cut. A path then names a place of the value whose length the build
  bounds from the type, a map's keys at their bound, and `refusal.scxml`'s comment "Every path but a `tooDeep` one names
  a place the schema declares" is amended to say so.

### D13. One JSON text writer per lane, and Rust's typed lift reads through the generated codec

- **One writer on Rust.** `ARCHITECTURE.md` says not to write a new escaper and to call the engine's writer. Rust's JSON
  Text escaper and its shortest-digits number text (`float64`, and `float32` for D9) move into a new crate named for
  them, `sce-json-text`, `no_std` with no heap, which writes into a sink; `sce-portable-bytes` keeps its purpose (owned
  bytes and the storage of scalar lists) and does not hold it. `json::push_escaped`, `json::quote` and
  `json::number_text` of `backends/rust/runtime/src/json.rs`, and the text codec's `json` escapes and `float`
  conversion, delegate to it. `tests/json_text/string_escape.json`, `real_text.json` and `float32_text.json` are read
  through it. The writer of `backends/rust/link-runtime/src/json.rs` (ADR 0041) is named as the next to delegate, and
  the content-line codec's runtime as the next format to move onto a grammar plan; neither is a step of this record.
- **One writer on C11.** The C11 codec's `json` escapes and its `float` conversion call the runtime's writers,
  `sce_payload_quote_span` and `sce_number_text`, the latter gaining the `float32` shortest-digits writer of D9; the
  tables of `ARCHITECTURE.md` gain the C11 codec's rows beside Rust's.
- **One streaming reader of a typed JSON value on Rust.** ADR 0164 D4 gives Rust a streaming lift for an inductive type
  beside `json.rs` and `event_payload.rs`, with a decode theorem per document (its T1b). Two streaming typed JSON
  readers in one runtime would be the duplication `ARCHITECTURE.md` forbids, so the binder's inductive arm (D8) is that
  reader, and T4 discharges T1b's decode half for JSON; the step that lands D8 removes the separate reader.
  `PayloadValue`'s decode, which parses its own tree and walks it, becomes the generated JSON codec of the event schema
  with `sce:unknown-members="skip"` and its revision defaults in the same step (Order of work). `json.rs` then keeps the
  reader of what has no type (the saved state's text and the data of the data model that is not typed) and the one
  writer's delegations. The lifts of the other lanes stay as they are and are held to the codec's language by the shared
  table (Open question 8).

### D14. How each claim is judged: theorems over the code that runs

Each claim is judged at the levels of ADR 0115, and none of D7's laws rests on a random test alone. On Rust the theorems
are proved in Verus over the file that ships, `sce_forge_runtime::grammar` (the reader, the printer and the binder), as
`docs/formal/proofs.json` proves `client_mailbox.rs` (ADR 0115 D13), against the specification functions of
`sce-grammar-reference`. Each generated codec's sink, and each hot codec printed as code (D6), carries a Verus
specification over the plan that Verus checks for that output (ADR 0115 D10, ADR 0156 D11), as ADR 0164's T1b is checked
per document; T2 rests on the sink, so a codec whose sink is not checked is not printed. The reference crate holds the
plan's well-formedness as an executable checker of the certificates the build writes with each plan (D2; McConnell et
al. 2011), proved in Verus to imply every premise the theorems take; `sce-build` calls it on every plan it prints
(translation validation: Pnueli, Siegel and Singerman 1998), so a plan whose checks the build got wrong is not printed,
and the build's algorithms for D2's checks are outside the trusted base. C11 is judged at ADR 0164 D12's level: its
walkers are held to the reference by differential tests on every case of the corpus now, and by CBMC once the owner
approves its use; `docs/formal/README.md` names them unproved until then.

| Claim | Level | Why that level |
|---|---|---|
| D2's checks decide what they say | the build's algorithms, each refusal a case that must be refused by its code beside a grammar it must accept; the reference's proved checker of their certificates run on every printed plan | decision procedures on automata and finite rules; checking a certificate is smaller than deciding again, and smaller than proving the generator |
| T1, one way (law 4): a plan the reference's checker accepts has at most one derivation of each text, and the reader computes it | Verus theorem, for every plan and every text, over the runtime reader | a statement of every input; a test samples it |
| T2, round trip (law 1, and law 2 by check 6) | Verus theorem, by induction on the derivation the printer chooses, given T1, checks 5 and 6, the sink's checked specification, and a lemma per conversion that it is a partial isomorphism on its canonical spellings (`text` with each escape scheme, `decimal`, `hex`, `fixed-decimal`, the byte conversions), over the runtime printer | the walker and those conversions are small and inductive |
| T3, linear (law 5) | Verus theorem: a ranking function bounds the reader's steps by `c * n + d` with `c` and `d` functions of the plan's shape, both passes of D8, the spans' re-reading and the trie of names counted within them, and a map's puts and relinking stated as their own terms; and step counts in tests on texts of 1 to 2^20 bytes, as ADR 0149 D7 counts them | a bound on every input, and a count a test reads where a clock is noise |
| T4, the binder classifies as specified (D8) | Verus theorem: for every type's binding plan and every JSON value, the binder's answer is the reference's `Classify_T` over the value's tree, a value or the refusal of the type at the least byte offset by ADR 0087 §2.1 item 5's order as amended; with a lemma, from the syntax plan's T1, that the events it emits are the tree in preorder | the fold is the part a table cannot test whole |
| A view's law (D6) | Verus theorem over the runtime's view: the value materialised from the view of an accepted text `t` is `parse(t)` | Wadler's law of a view, over every text |
| The `float` conversion's round trip and D9's rules | an assumption named in `docs/formal/README.md` (ADR 0115 D6), held by tests: every `float32`, all 2^32 patterns, printed and read back; every case of `real_text.json` and `float32_text.json`; `float64` values from recorded seeds, each against the exact rational arithmetic of `sce-grammar-reference` | correct rounding is a theorem about the reals whose machine proof is a work of its own; binary32 is tested whole and binary64 only sampled, which the README says |
| Law 3, the quotient | property test: texts drawn from each plan's derivations with random skips, cases, escapes, spellings, notations, member and map orders and groups, each read and printed and compared with the original up to the declared equivalence | the equivalence is the grammar's declaration, and is tested as one |
| The JSON codec reads the lift's language (D8) | `payload_refusal.json` and `json5_refusal.json` replayed through the codec; a differential test against each lane's lift on texts from recorded seeds, drawing only texts with at most one refusal of the type until that lane's lift keeps the least offset (step 5); JSONTestSuite read through a raw place, each departure listed (D18) | an equality of two programs' languages, tested where they can differ |
| A text has none, one or several derivations | property test: Earley's parser over the plan's rules, on every split of the text into tokens a naive backtracking longest-match scanner finds, finds exactly the reader's derivation or none | a second reading that trusts neither the scanner nor the parser under test |
| The printed JSON grammar and its sizes agree | tests: every text the printer writes under `declared` is in `G(T)` by the Earley recognizer of `payload_schema.rs`; `MAX_TEXT` equals `J(T)` for a type with no float, and passes it by the computed difference for one with floats | one language stated three ways, held together until `G(T)` is printed from the plan |
| C11's walkers | differential tests against the reference on every case of the corpus; CBMC on the owner's approval | ADR 0164 D12's level for the same lane |

`docs/formal/proofs.json` names each proved file with its mutants, each of which must make Verus fail: check 2 with the
FOLLOW condition left out of the reference's checker; check 4's condition left out without the tabulation; a printer
that takes a later spelling where an earlier one holds the value; a reader that scans a token again from its start after
a failed longer match; a binder that reads a union's member before its tag without keeping its span; a binder that keeps
the first refusal it finds instead of the least.

### D15. How the generated model reads a text codec

- **At a boundary**, and wherever a send's data is encoded by a text codec, a text codec is read as ADR 0115 D7 reads
  every codec: by its values, with `Refuses_C` one disjunct per place of D4. A disjunct whose condition is a printed
  text's membership in a regular language (a value outside a token's language, an integer whose numeral the token does
  not read, a raw place's text) is written as an uninterpreted predicate `Printable_{C,p}` for the place `p`,
  constrained only by what D4 makes true of every value (Burch and Dill, CAV 1994, as ADR 0149 D6a reads `matches`),
  unless the build recognises an integer's printable values as a union of intervals, which it then writes; a float that
  is not finite is written as itself. A text its decode refuses enters as the channel's `corruption`. No `Refuses_C` is
  written by `sce-codegen model` today; ADR 0156 D3 orders it third, and a text codec's disjuncts join it there.
- **A lift through a text codec** is, until strings have a model, a nondeterministic choice of any value of the
  location's type or of any refusal code of D12's closed set with its bound; the refusal's path and offset, and the text
  itself, are left out of the model. The choice has every behaviour the code has (Clarke, Grumberg and Long 1994). An
  exploration cuts it to the domain its exploration document names (ADR 0115 D15), as it cuts every input, and the
  verdict is *holds within* that domain, naming the domain as reached; it is not a statement of every text. A violation
  is made concrete before it is reported: the chosen value is printed by the codec (law 1 makes the text read back to
  it) or a witness text of the chosen code is written from the plan, and the run is replayed on the lanes; one that
  cannot be made concrete is reported as spurious, not as violated (counterexample-guided abstraction refinement:
  Clarke, Grumberg, Jha, Lu and Veith 2000). It needs the location's type to have a model, which ADR 0156's amendment
  schedules for the `sce-static` data model; a document whose text variable is read by anything but such statements
  waits for strings. Once strings have a model the lift reads `Parse_C` and the print `Print_C` as functions the model
  does not interpret, constrained by T2 (`Parse_C(Print_C(v)) = v` for every `v` with `Printable_C(v)`), law 2, and
  `|Print_C(v)| <= MAX_TEXT`.
- **A print, and a statement at a place**, read or write a string, and a document that holds one is refused by
  `sce-codegen model` by a `Construct` of its own, `TextCodec`, until strings have a model; it writes no partial model.
- **A record that holds a map** (D10) has no translation while a map has none, and is refused by the same `Construct`
  that refuses a map.
- A plan has no TLA+ module of its own: a grammar is a function, not a protocol, and a function is judged by a theorem
  over every input (D14), where a model explores the states of a machine.

### D16. Lanes

- **Rust** serves every construct of this record, native, `no_std` with no heap, and on `wasm32-unknown-unknown`.
- **C11** serves every construct of this record in the freestanding core of ADR 0110: the plan's tables are `const` data
  a device keeps in read-only memory, the walkers (`sce/forge/grammar.h`) have no heap and no recursion, their stack is
  an array the plan sizes and the machine or its caller reserves, the value's storage is the type's (fixed, or lent, ADR
  0154 where a map's slots are lent), and a view is the C11 view of D6. The consumer that asked generates its core for
  Rust and C11 from one set of documents, and its rows on a device and in a C interface read addresses, keys and replies
  at their boundary; ADR 0164 serves the same two lanes. C11 is held to the same reference cases as Rust, and judged at
  the level D14 states for it: tested, not yet proved.
- **Every other backend** (Kotlin, Python, Go, Swift, C++) and the Interpreter's lowering refuse, by name, a document
  that holds a text codec, a `<sce:lift codec>`, a `<sce:print>` or a record that holds a map (D10):
  `codegen/text-codec-not-served`, naming the lane and the construct, so a lane that does not serve one says so and
  never answers otherwise. The lift by `schema` stays on every lane, with D9's reading of a `float32` on every lane. The
  JSON Schema import (D17) writes ordinary documents and serves every lane.
- **A lane joins** by printing the plan and running the walkers in its language, in the change that lands its tests
  against the reference's cases.

### D17. A JSON Schema is imported at build time as the documents it states

- `<sce:import kind="json-schema" src="catalog.schema.json" as="Catalog"/>` reads a JSON Schema (draft 2020-12) when the
  build runs and gives the build the event schemas, unions, enums and inductive documents it states and the JSON codec
  of its root, as if they were documents; `sce-codegen import json-schema <file>` prints those documents, for a reader
  and for a repository that prefers to commit them. The schema file is an input of every generated unit that reads it,
  and its source hash covers it (ADR 0094). It is one source shared with tools outside SCE, as a `.proto` file is to be
  imported by a record of its own.
- **The mapping is the inverse of `S(T)`** (ADR 0087 §1.1, ADR 0092 §3), and recognises `S(T)`'s own forms before it
  reads any keyword as a refinement: a string whose `pattern` is the decomposition of a 64-bit range that ADR 0087 §1.1
  writes is that integer type; a string with `maxLength` and the Latin-1 `pattern` is a `bytes[B]`; a number whose
  `minimum` and `maximum` are `-f32::MAX` and `f32::MAX` is a `float32`; a string whose `pattern` is ADR 0162 D9's
  canonical text of `decimal(p, s)` is that decimal; a text's byte bound is its `x-sce-max-bytes`; a `$defs` cycle that
  carries ADR 0164's `x-sce-max-nodes` is an inductive document. Beyond them: an `object` with `properties` and
  `required` is a record, a property not in `required` an optional field, `"additionalProperties": false` giving
  `sce:unknown-members="refuse"` and its absence `"skip"`; an `object` whose `additionalProperties` is a schema, whose
  `propertyNames` are bounded and with `maxProperties` is a `map<string[K], V>[B]`; an `array` with `items` and
  `maxItems` a list, `minItems` its lower count; a `oneOf` of objects each holding one `const` under one key, a union
  tagged by that key; an `enum` of strings a closed enum; an `anyOf` of a schema and `{"type": "null"}` an optional; an
  `integer` whose `minimum` and `maximum` are a type's range, that type; a `number` a `float64`; a `string` with a byte
  bound a `string[B]`; a `$ref` into `$defs` the type it names.
- **The law.** For every type `T` of the corpus, importing `S(T)` gives `T` back. An enum's numbers (ADR 0047) are not
  in JSON, so `S(T)` gains the annotation `x-sce-enum-numbers` beside an enum's names (amending ADR 0087 §1.1,
  additively), which the import reads; a schema without it gives each variant its position.
- **A bound is part of a type**, so a text, a list or a map with no bound is refused (`import/json-schema-unbounded`,
  naming the pointer) unless the import names one for that place by name, `<sce:bound at="..." bound="peer_name=64"/>`
  (ADR 0050). A text's bound counts bytes and `maxLength` counts characters: `maxLength` alone is refused as unbounded
  unless the import names a byte bound, and a byte bound named below four times `maxLength` narrows the schema's
  language, which the generated documents state in their comment.
- **Refused by name, never dropped**: a keyword that narrows a value within its type and is not one of `S(T)`'s own
  forms (`pattern`, `format`, `multipleOf`, `minimum` and `maximum` that are no type's range, `minLength`,
  `uniqueItems`, a `const` outside a union's tag, `dependentRequired`, `if`, `not`, `patternProperties`), as
  `import/json-schema-refinement`; a keyword outside the mapping (`allOf` beyond one `$ref`, `prefixItems`,
  `unevaluatedProperties`, a `type` list beyond one type and `null`), as `import/json-schema-unsupported`; a cycle of
  `$ref`s with no node bound, as `import/json-schema-recursive`; a `$ref` that names nothing, as
  `import/json-schema-unresolved`. A refinement is a predicate over a type's values, a facility of the type system and
  not of an import; refusing it keeps the codec's language the schema's, or says where it is not.

### D18. Refusals, bounds, and each construct's account

New `DiagnosticCode`s, each with its appendix row in `docs/SCE_ACCEPTED_SUBSET.md` and its entry in
`SCE_ERROR_CONTRACT.md`, added in the step that makes it: `grammar/empty-token`, `grammar/unbound-token`,
`grammar/not-ll1`, `grammar/recursive-rule`, `grammar/terminals-overlap`, `grammar/scan-table-too-large`,
`grammar/tokens-merge`, `grammar/canonical-spelling-not-read`, `grammar/read-value-not-printable`,
`grammar/scanner-too-large`, `grammar/conversion-mismatch`, `grammar/binding-mismatch`, `grammar/member-names-collide`,
`grammar/chain-ill-formed`, `grammar/open-enum`, `grammar/raw-type`, `grammar/within-type`, `validation/lift-codec`,
`validation/print-location`, `validation/print-refusal`, `import/json-schema-unbounded`,
`import/json-schema-refinement`, `import/json-schema-unsupported`, `import/json-schema-recursive`,
`import/json-schema-unresolved` and `codegen/text-codec-not-served`. Existing codes it uses:
`validation/map-in-wire-position` (narrowed by D10), `validation/missing-attribute` (`sce:type`, `sce:unknown-members`,
`unknown-members`, `sce:map-order` or a map's `order` absent where it applies; `sce:member-order` and
`sce:float-notation` have their defaults), `codec/undeclared-encode-refusal` for a codec with a raw place that does not
declare `sce:encode-may-fail="true"`, `validation/lift-path-bound` (D12), `codec/undeclared-encode-refusal` and
`codec/encode-may-fail-without-refusal` (D4). Run-time refusals are the codes of D12 (17 to 21 new), not diagnostics.

The bounds by name: `grammar_scanner_states` and `grammar_scan_table_bits` (D2), ADR 0149's `regexp_nfa_states` on each
token, a value-free rule's depth bound and a group's (D5), an inductive member's node bound (ADR 0164), a
`<sce:within>`'s `bound` (D11), and the bounds of the types read and printed (ADR 0050, ADR 0086). Every other size (the
reader's stack, the trie of names, Reps's tables, a radix sort's buckets, the spans of D8, a raw place's nesting, a
scratch of D12, `MAX_TEXT`) is computed from those by the build and reported.

| Construct | What the build checks | Its refusals | Its bounds by name | Lanes (D16) | The generated model (D15) |
|---|---|---|---|---|---|
| A grammar codec (D2 to D6) | checks 1 to 6 on the composed plan, the bindings, the conversions, the printer's domain against `sce:encode-may-fail`; the reference's checker of the certificates on every printed plan | `grammar/*` of D2, D3 and D5, `codec/undeclared-encode-refusal`, `codec/encode-may-fail-without-refusal` | `grammar_scanner_states`, `grammar_scan_table_bits`, `regexp_nfa_states`, D5's, its type's | Rust (proved) and C11 (tested); every other lane and the Interpreter `codegen/text-codec-not-served` | by its values: `Refuses_C` with `Printable_{C,p}`, and the round trip |
| A JSON or JSON5 codec (D8) | the fixed syntax plans checked once; the fold's binding; the required attributes; no open enum | `grammar/open-enum`, `validation/missing-attribute` | its type's | as above | as above, the floats that are not finite in JSON's `Refuses_C` |
| `float32` read once and written by its own digits (D9) | nothing new | `outOfRange` at and past `2^128 - 2^103` | none | every lane's lift and payload writer, and every text codec | a float is read as the value it is |
| A record that holds a map (D10) | the positions the schema stands in, the map's order declared | `validation/map-in-wire-position` | the map's | Rust and C11; refused by name elsewhere | refused by the `Construct` of a map |
| A raw place, a text within a text (D11) | the field's type; the inner codec's type and `MAX_TEXT`; a raw place's codec declares that its encode can refuse | `grammar/raw-type`, `grammar/within-type`, `codec/undeclared-encode-refusal` | the field's text bound; `bound` | Rust and C11 | by the value of the field, a raw place's printing in `Printable_{C,p}` |
| `<sce:lift codec>`, `<sce:print>`, a place (D12) | the location's type; the totality of a print (ADR 0133); the longest pointer | `validation/lift-codec`, `validation/print-location`, `validation/print-refusal`, `validation/lift-path-bound` | the location's, the text's and the path's | Rust and C11 | a lift by a choice of any value or code, its verdict within the explored domain and its violations made concrete, then `Parse_C`; a print and a place by `Construct::TextCodec` until strings, then `Print_C` |
| The JSON Schema import (D17) | the mapping of each keyword, every bound, `S(T)`'s own forms first | `import/json-schema-*` | the schema's, or named on the import | every lane (its output is documents) | its types, as any types |

### D19. What ADR 0166 and ADR 0168 build on

- **ADR 0168, a polytypic encoding,** follows this record and builds on what D6 and D8 make: a syntax plan read by the
  walkers of D6 and proved by D14, and a binding derived from a type by one rule per type constructor and run by the
  binder (T4). D8 writes the two bindings of JSON and JSON5 into `sce-build`; ADR 0168 lets a document declare such a
  binding (its rule per constructor, Jansson and Jeuring 2002) over a syntax plan of its own, so a type-directed
  encoding a consumer needs is a document and not a third built-in. JSON and JSON5 are then two instances of it, and its
  laws are D7's with T2 and T4 carried over. The binding plan D8 prints is the form ADR 0168 lets a document write. ADR
  0168 builds on D7 to D9, D14 and D18 as they stand, and does not change them without amending this record.
- **ADR 0166** raises whether a grammar codec may be generic in its type. A codec whose type is a parameter is an
  instance of ADR 0168's binding (the rule per constructor is what a generic codec is), so it is decided there; a
  grammar codec of this record names one type.
- **ADR 0162**'s `decimal(p, s)` is read by the `fixed-decimal` conversion (D3) and by the fold's arm (D8), its spelling
  its canonical text as a string.

## Not in scope

A grammar given at run time (a document read when the program runs is a codec document read at run time, a question of
its own); an ambiguous grammar, and a general context-free parser at run time; PEG; left recursion but by D5's chain; a
context-sensitive rule (a count or a length that says how much text follows, a closing tag that repeats an opening one,
layout by indentation); reading a text in pieces as it arrives; reporting more than the first refusal, and recovering
after one; writing through a view; the content-line encoding, which keeps its own runtime until it moves onto a plan
(D13); case folding beyond ASCII, collation and normalisation of what is read (ADR 0149 decided that a decode does not
normalise); the refinements of a JSON Schema (D17); YAML, TOML and every other format, each an encoding or a grammar of
its own when one is needed; the move of the other lanes' lifts onto the generated reader (Open questions, 8); a
type-directed encoding a document declares, which is ADR 0168's (D19); and any one application's grammars.

## Consequences

- A text with structure is read once, at the boundary or by one statement, into a typed value or a refusal record; logic
  computes on the value and never on the text, and writes text only by printing a value.
- A grammar is checked when it is built: an ambiguity, a token that swallows its neighbour, a read that could run
  quadratic, a value the reader gives that the printer cannot write back, and a value the printer cannot write at all
  are each refused or declared before anything runs.
- Reading costs time linear in the text, in constants of the grammar's shape and not of any bound, names held distinct
  within it by a trie, with the cost of a map's puts (ADR 0108 D8's, under a keyed hash whose seed a host without
  entropy must supply, or up to the bound in `Linear` storage) and of relinking a `by-key` map stated beside it; no text
  makes the reader backtrack. This is proved on Rust, of the walkers and the generated sinks that ship, and tested on
  C11 (D14).
- Reading what was printed gives the value back, by theorem, and printing what was read gives the one spelling of its
  value, so a table keyed by text can be keyed by the canonical spelling.
- JSON and JSON5 need no grammar written: the type's binding is derived, and the JSON codec reads the language every
  lane's lift reads; on Rust the typed lift is that codec. Its printed order and notation are the house's unless
  declared, so it can match another producer's bytes.
- Every reader of a `float32` rounds its decimal once, and every writer writes its own shortest digits; a
  double-rounding text and the shortest spelling of `f32::MAX`, which Kotlin's writer writes today and Rust's lift
  refuses, read differently than before (D9).
- A record may hold a map on Rust and C11 where a text codec reads it, a map has a JSON form, and a table whose order
  means nothing is read in the order of its keys.
- A value can be read as a view of its text, with no copy of its texts, for as long as the text lives.
- A JSON Schema shared with other tools is one source; what the type grammar cannot state is refused by name and not
  dropped.
- A value of unknown type is a bounded text until a type reads it, and is held to its encoding when it is printed.
- A type's refusals come at the least byte offset on every lane, once each lift has moved (D8).
- A new format of text is a grammar document, and none is written again by hand in each runtime.

## Alternatives rejected

- **A Parsing Expression Grammar.** Its ordered choice makes the language of a choice depend on the order of its
  alternatives, so a printer can write a text that the reader takes by an earlier alternative and reads as another
  value: the round trip then needs a check PEG does not give. Packrat parsing is linear only with a table as large as
  the text times the rules, and whether a PEG's language is empty is undecidable (Ford 2004), so a build cannot say of a
  PEG what D2 says of an LL(1) grammar.
- **A general context-free parser at run time** (Earley, GLR). Cubic work in the worst case, and an ambiguity is found
  only when a text with two readings arrives, at run time. Earley's parser is kept as the test's second reading (D14).
- **JSON read by a grammar derived from its type**, the first draft's design. JSON's language is not a type's (D8): a
  member a type does not name holds any value, names are compared after decoding, a tag may come last, and the refusals
  of the shared table classify a JSON value against a type. The derived grammar was not LL(1) and did not refuse in the
  table's order.
- **Refusing every point check 4's condition fails.** The condition is sufficient and not necessary, so it would refuse
  grammars that read linearly; Reps's tabulation serves them, at a workspace the build states and bounds by name (D2).
- **Holding names distinct by a comparison sort, or by a hash.** A sort costs a logarithm per name where a trie costs
  nothing beyond the names' bytes; a hash is linear only in expectation and only under a key the sender does not know.
- **A reader and a printer written by hand for each format.** Two statements of one grammar that drift apart: the JSON
  readers and writers of this repository disagreed until four shared tables held them, and the content-line codec is one
  grammar written in each runtime.
- **A scanner with no context, one longest match for the whole text** (lex). In the small languages of addresses and
  parameters, one place's token is another place's text (an IPv4 address is a host name, a number is a name), and a
  scanner that does not know where the parser stands refuses grammars a context-aware one reads one way.
- **A grammar over characters with no tokens, LL(1) character by character.** A keyword and a name, a number and its
  exponent, need more than one character of lookahead, so the grammar would be LL(k) for a large `k` or rewritten around
  its lexemes by hand.
- **Regular expressions with captures as the parser.** A regular language has no nesting and no repetition with
  structure, and which part a capture holds differs between engines; ADR 0149 left captures out for that reason.
- **Extending the content-line encoding.** It is RFC 5545's grammar, with that format's lines, parameters and folding; a
  general grammar would be a second language inside one format's attributes.
- **An untyped JSON tree handed to documents, or a library type of any JSON value** (an inductive `json_value` once ADR
  0164 is accepted). Logic would walk a tree whose shape no build has checked, each walk refusing or misreading in its
  own way, the gap the census names; and an arena of the largest tree for each such place costs the tree's node bound
  where a raw text costs its bytes. A document whose logic needs a tree declares its own type (D11).
- **A raw value held as the CBOR of the value** (RFC 8949 §6.2's conversion from JSON). The conversion chooses a type
  for each number and loses its spelling, and JSON5's numbers have no form there.
- **RFC 8785's canonical JSON as the printed form.** Its numbers are binary64 only, so a 64-bit integer is not printed
  exactly; it writes a negative zero as `0`; and it orders keys by UTF-16 code units, where every writer of this
  repository orders them by code point.
- **A map always printed in the order of its keys, or always in its own.** The first breaks the round trip of a map
  whose order means something; the second gives two values for one table whose order means nothing. The document says
  which (D10).
- **A fixed member order and notation for the codec**, or each required with no default. Fixed, a text compared byte for
  byte with another producer's cannot be printed; required, every codec would restate the house's form for a choice that
  changes nothing it reads. They default to the house's form and are declared to match another producer (D8).
- **Keeping `float32` read through a binary64**, and printing a `float32` as its widened binary64. The reader would stay
  the one in the house that rounds twice, a codec reading the same text as a lift would read another value, and the lift
  would still refuse the shortest spelling of `f32::MAX` a writer of the house writes today; one rule for every reader
  and every writer is correct (D9).
- **A place as RFC 6901 text alone.** It cannot pick an element by a field's value, and a configuration's own path
  syntax would be a second pointer language inside the statement; a typed path that any grammar codec reads serves both
  (D12).
- **The first refusal found, in place of the least offset.** With a union whose tag comes last, the first refusal found
  is not the first in the text, so the reported place would depend on how a reader walks, which is what the shared table
  exists to prevent (D8).
- **A separate code for a path through a raw place.** It is one fact with a misspelt name, "the type has no place
  there", and the pointer tells the two apart (D12).
- **Refinements of a JSON Schema checked by the codec** (a `pattern` as a facet). A facet beside a type is a second type
  system, held by the codec and not by the statechart that makes the value; refinement types are a decision of the type
  grammar.

## Order of work

Each step lands with a test that fails without it.

1. This record, accepted, with the "Amended by: ADR 0163" lines of ADR 0087 and ADR 0164.
2. **The operations on automata** (D2), on ADR 0149's crate `sce-rust-regexp` at `backends/rust/regexp` (its step 4,
   landing item 481) and its construction (its step 5): the reader's longest-match walk and Reps's tabulation in the
   `no_std` core; union, intersection, difference, concatenation with the closure of prefixes, a witness of a non-empty
   language, the cycle test of check 4, the CFL-reachability of check 5 and the image of a conversion behind the feature
   that brings a heap. `backends/rust/regexp/tests/automata_operations_meet_their_definitions.rs`: each operation
   against its definition on every text up to a length over a small alphabet, and the core built with no heap. Red
   without it: the operations do not exist.
3. **One JSON text writer on Rust and on C11** (D13): the crate `sce-json-text`, and `json::push_escaped`, `json::quote`
   and `json::number_text` delegating to it; `sce_number_text`'s `float32` writer. Tests: `string_escape.json` and
   `real_text.json` read through the crate, under `no_std`; `a_float32_is_printed_by_its_own_shortest_digits`, on Rust
   and in `backends/c/tests/unit/json_text_test.c`. Red without it: the crate does not exist.
4. **A `float32` read once and written by its own digits, on every lane** (D9): `tests/json_text/float32_text.json`, its
   reading cases read by the lift and its writing cases written by the payload writer on Rust, Go, Kotlin, Python,
   Swift, C++, C11 and the Interpreter, beside each lane's reader of `real_text.json`, and `ARCHITECTURE.md`'s row. Red
   without it: `1.000000059604644775390625000001` reads as `1`, `3.4028235e38` is refused, and Kotlin writes
   `3.4028235E38`.
5. **The order of a type's refusals on every lane** (ADR 0087 §2.1 item 5 as amended, D8): `payload_refusal.json` gains
   cases with two refusals of the type in both orders of the text, and a union whose tag comes last with a bad field
   before the tag and one after it, each refused at the least byte offset; every lane's lift keeps the least. Red
   without it: a lift that walks its tree in its generated order reports the later refusal.
6. **The grammar's reader, its checks and its plan** (D1 to D4, D5's first form, and D18's build refusals):
   `sce-build/tests/a_text_grammar_is_checked_to_read_every_text_one_way.rs`. Red without it: `sce:encoding="grammar"`
   is not an encoding. Each check has a grammar it must refuse by its code and one it must accept: `a | a b` and
   `a b | c` (`grammar/not-ll1`); two empty alternatives, and a repetition of an optional part; a token matching the
   empty text and a skip matching it (`grammar/empty-token`); a token no binding reads (`grammar/unbound-token`); a host
   name and an IPv4 address expected at one point, and the same with `over` (`grammar/terminals-overlap`); tokens `a`
   and `a*b` before a `;`, printed with Reps's table and reading linearly, and the same past `grammar_scan_table_bits`
   (`grammar/scan-table-too-large`); three names `a`, `b`, `c` adjacent with `abc` expected, and the same with a skip
   the author placed, each decided by the CFL-reachability of check 5 (`grammar/tokens-merge`); a skip whose `print` it
   does not read (`grammar/canonical-spelling-not-read`); `[0-9]+` under a winning `7`, and `[1-9][0-9]*` under it, and
   a `float` token whose language lacks the notation's exponent form (`grammar/read-value-not-printable`); a scanner
   past `grammar_scanner_states` and the same under `--limit` (`grammar/scanner-too-large`); `decimal` on `[0-9a-f]+`
   (`grammar/conversion-mismatch`); a field bound twice; two member names equal after unescaping
   (`grammar/member-names-collide`); a value-binding cycle with no inductive type and a value-free one with a depth
   bound (`grammar/recursive-rule`); two imported codecs whose composed plan conflicts, the refusal naming both. And
   `a_value_outside_its_tokens_language_is_an_encode_refusal_the_build_finds` in `sce-build/tests/codec_totality.rs`:
   the media type of D3 declared total is `codec/undeclared-encode-refusal`, and so is a JSON codec with a raw place
   that does not declare that its encode can refuse.
7. **The reference and its theorems** (D14): `sce-grammar-reference`, its specification functions, its executable
   checker of the build's certificates with the Verus proof that it implies the theorems' premises, its command
   `sce-codegen grammar eval <codec> <text>`, and cases written from recorded seeds by
   `tests/forge/conformance/gen_cases.py` with the values the reference gives.
   `sce-build/tests/a_grammar_reads_what_it_prints.rs`: laws 1 to 3 on the reference for every grammar of the corpus;
   Earley's parser over a naive backtracking scanner finding exactly the reference's derivation; every plan `sce-build`
   prints accepted by the reference's checker, and a plan whose certificate hides a FOLLOW conflict refused by it. Red
   without it: the crate does not exist.
8. **The Rust walkers and the generated sinks, proved, and the generated codecs** (D6): `sce_forge_runtime::grammar`
   with T1 to T3 and the view's law proved and entered in `docs/formal/proofs.json` with D14's mutants, and each
   generated sink's specification checked by Verus; `backends/rust/forge-conformance/tests/forge_text_grammar.rs` with
   `a_printed_value_is_read_back_as_itself`, `a_text_is_read_as_the_reference_reads_it` (value, or code, offset and
   pointer), `a_text_read_and_printed_is_its_one_spelling`, `a_view_materialises_to_the_parsed_value`,
   `reading_takes_steps_linear_in_the_text` (texts of 1 to 2^20 bytes, a count exact per byte, red for a walker that
   rescans), `the_largest_value_prints_in_exactly_max_text_bytes` (and a buffer one byte short refused by the measure),
   `the_media_type_grammar_reads_rfc_9110s_examples`, and, once a hot codec's checked form exists,
   `a_hot_codec_printed_as_code_reads_as_the_plan_reads`; run native, under `no_std` and on `wasm32-unknown-unknown`.
   Red without it: the runtime has no walker, and Verus finds each mutant.
9. **JSON and JSON5** (D8), with T4 proved, Rust's typed lift moved onto the codec, and ADR 0164 D4's separate Rust
   streaming reader removed, its arm the binder's (D13): `backends/rust/forge-conformance/tests/forge_json_codec.rs`,
   with `the_json_codec_refuses_as_the_shared_table_says` (`payload_refusal.json` through the codec),
   `the_json5_codec_refuses_as_its_table_says` (`json5_refusal.json`), `the_json_codec_reads_as_the_lift_reads` (texts
   from recorded seeds), `a_value_prints_as_the_typed_payload_writer_writes_it` (under the defaults, equal bytes for
   every value with no negative zero and no float that is not finite),
   `a_declared_member_order_prints_the_schemas_order`, `a_positional_float_has_no_exponent`,
   `every_float32_is_read_back_as_itself`, `a_float64_is_read_back_as_itself`, `negative_zero_is_printed_as_minus_zero`,
   `a_union_whose_tag_comes_last_is_read`, `a_refusal_inside_a_kept_span_is_reported_at_its_offset`,
   `a_repeated_name_at_any_depth_is_not_json`, `names_are_held_distinct_with_no_workspace_where_all_are_declared`,
   `json5_reads_the_examples_of_its_specification_and_prints_json`, `an_inductive_value_is_read_by_the_binders_arm`,
   `an_open_enum_is_refused_for_a_json_codec` (`grammar/open-enum`) and `the_json_test_suite_is_read_as_rfc_8259_says`
   (each accepted text through a raw place, each refused text refused, and the departures listed in the test: every
   `y_object_duplicated_key*` text is valid JSON SCE refuses as `notJson`, and each `i_` text is decided there, a lone
   surrogate escape and a number past binary64 refused, the others read); the Rust lift's existing tests and ADR 0164's
   lift tests pass through the codec. Run native, under `no_std` and on `wasm32-unknown-unknown`. Red without it:
   `sce:encoding="json"` is not an encoding.
10. **A record that holds a map** (D10), on Rust: `sce-build/tests/a_record_holds_a_map_where_a_text_codec_reads_it.rs`
    (the media type of D3 is generated; the same schema as a delivery's payload or a CBOR entry is
    `validation/map-in-wire-position`, naming the position; a codec whose type reaches a map with no order declared is
    `validation/missing-attribute`) and, in `forge_json_codec.rs`, `a_map_read_in_order_is_printed_in_it`,
    `a_map_read_by_key_reads_two_orders_to_one_value`, `an_integer_keyed_map_is_an_array_of_pairs` and
    `a_key_read_twice_in_an_array_of_pairs_is_repeated`. Red without it: the media type's schema is refused.
11. **Raw places and texts within texts** (D11): in `forge_json_codec.rs`, `a_raw_place_holds_its_normalized_text`,
    `a_raw_place_refuses_an_escaped_nul`, `a_raw_place_holding_no_json_text_is_refused_by_the_printer` (a field holding
    `1,"admin":true` is `notInLanguage`), `a_raw_text_is_read_later_by_a_codec_of_its_type`,
    `a_text_within_a_text_reads_by_its_codec`; and in
    `sce-build/tests/a_text_grammar_is_checked_to_read_every_text_one_way.rs` `grammar/raw-type` and
    `grammar/within-type` (a `MAX_TEXT` past `bound`). Red without it: `<sce:raw>` is an unknown element.
12. **The statements, the path and the record's second revision** (D12): `backends/rust/scenarios/tests/text_codec.rs`,
    with `a_lift_through_a_text_codec_keeps_its_refusal_and_its_offset`, `a_lift_by_schema_writes_no_offset`,
    `a_refusal_saved_at_revision_one_is_restored_at_two`, `an_undeclared_member_points_at_its_object`,
    `a_print_that_cannot_refuse_has_no_refusal_path` (the generated code holds no error path, ADR 0133 D2),
    `a_print_refuses_a_value_not_in_its_language`, `a_place_named_by_a_path_is_replaced_whole`,
    `a_match_step_replaces_the_first_match_or_appends`, `a_value_written_under_a_match_with_another_key_is_refused`
    (`keyMismatch`), `a_path_to_no_place_is_refused` and `a_path_through_a_raw_place_names_no_place`, run native and on
    `wasm32-unknown-unknown`; the codes 17 to 21 in `refusal_code.scxml`, `enum_harness_code.scxml` and
    `model_outcome_classes.json` at one number each; and in `sce-build` the refusals `validation/lift-codec`,
    `validation/print-location`, `validation/print-refusal` and `validation/lift-path-bound` for a map whose keys pass
    the pointer's bound. Red without it: `<sce:lift codec>` is `validation/lift-codec` and `<sce:print>` an unknown
    element.
13. **The C11 lane** (D16): `sce/forge/grammar.h` and the lowerings, held to the reference's cases of steps 7 to 12 by
    `backends/c/tests/unit/forge_text_grammar_test.c` and `forge_json_codec_test.c`, every fixture of them compiled and
    run in the freestanding gate of ADR 0110, and `docs/formal/README.md` naming the C11 walkers unproved. Red without
    it: a text codec on C11 is `codegen/text-codec-not-served`.
14. **Recursion through an inductive type and chains** (D5, forms 2 and 3), after ADR 0164's arena lands on the lane:
    `a_rule_reaches_itself_through_an_inductive_member` (a nested schema language's declarations read into an arena, the
    node past the bound `tooManyNodes`), `a_pass_through_cycle_is_refused` (`grammar/recursive-rule`, naming the
    unguarded path), `a_group_past_its_depth_bound_is_too_deep` (`((((x))))` one level past `expr_group_depth`) and
    `a_chain_prints_only_the_groups_it_needs` (`a-(b-c)` keeps its group, `(a-b)-c` loses it, both read back to their
    trees), in `forge_text_grammar.rs` and on C11; `grammar/chain-ill-formed` in the build's test. Red without it: such
    a rule is `grammar/recursive-rule`.
15. **The model** (D15): in the tests of `sce-build/src/formal/document_model.rs`,
    `refuses_c_names_every_place_a_text_codec_refuses` (the media type's places as disjuncts, `Printable`
    uninterpreted), once ADR 0156 D3 writes `Refuses_C`; `a_lift_through_a_text_codec_is_a_choice_of_any_value_or_code`
    and `a_violation_through_a_lift_is_made_concrete_or_reported_spurious`, once the `sce-static` data model has its
    translation; and `a_print_is_refused_by_name_by_the_model_generator` (`Construct::TextCodec`). Red without it: the
    media type has no disjunct.
16. **The other lanes** (D16): `a_text_codec_is_refused_by_name_on_a_lane_that_does_not_serve_it`, one assertion per
    backend and the Interpreter, for a text codec, `<sce:lift codec>`, `<sce:print>` and a record that holds a map. Red
    without it: a lane generates a document it cannot run.
17. **The JSON Schema import** (D17): `sce-build/tests/a_json_schema_is_imported_as_the_types_it_states.rs`, with
    `importing_the_payload_schema_of_a_type_gives_the_type_back` (every type of the corpus, 64-bit integers, byte
    strings, `float32`, decimals, enums with their numbers and inductive members among them) and a case per refusal. Red
    without it: `kind="json-schema"` is `import/kind-mismatch`.
18. The documents named in "Amends".

## Open questions

Each was decided with its reason, and the acceptance kept each as amended below.

1. **LL(1), or PEG?** LL(1) over regular tokens with a context-aware scanner. It is unambiguous by a check the build
   makes and names a conflict in, reads in linear time with a stack its bounds fix and no table as large as the text,
   and its derivation is unique, which is what the round trip rests on (T2).
2. **A kind, or an encoding?** An encoding of the codec kind (D1): it reads and writes a value of a type from bytes by
   rules fixed at build time, which is a codec, and SCE_FORGE §10 names the scoped extension of a kind as this form.
3. **In what order is a map read and printed?** As the document declares, `read` or `by-key`, with no default (D10): a
   map's order is part of its value (ADR 0108 D2), so a table whose order means something keeps it, and one whose order
   means nothing is read by key so one table is one value. A quotient on the value's side is a canonizer of a quotient
   lens (Foster, Pilkiewicz and Pierce 2008).
4. **Is a negative zero printed `-0`, and a `float32` by its own digits, and how is a `float32` read?** Printed so, or
   law 1 fails for both; read by one rule for every reader, rounded once (D9), so the codec and the lift read one value
   from one text and the printed shortest spelling of `f32::MAX` reads back.
5. **Does `sce:unknown-members` default to the lift's `skip`?** No default: a type read from a file a person writes
   should refuse a misspelt member, a type read from a newer peer should step over it, and only the document knows
   which.
6. **How is a value of unknown type held?** As its normalized text, read later by a codec of a type (D11); a tree of it,
   where logic needs one, is a type the document declares (ADR 0164).
7. **Is C11 built with Rust?** Yes (D16): the consumer that asked generates its core for both from one set of documents,
   its rows on a device and in a C interface read text at their boundary, and the scope rule builds a facility on the
   backends a consumer needs. The walkers have no heap and no recursion, so C11 costs the lane and not a second design.
8. **Does every lane's lift move onto the generated reader?** Rust's does (D13), because two JSON readers of a typed
   value in one runtime would be the duplication `ARCHITECTURE.md` forbids. The other lanes keep their lifts, held to
   the codec's language by the shared table and a differential test (D14), and move when each lane serves text codecs.
9. **What bound does a JSON Schema's `maxLength` give?** None alone: it counts characters and a type counts bytes, so
   the byte bound is `x-sce-max-bytes` or one the import names, and a narrower language is stated (D17).
10. **One refusal record, or one for text codecs?** One: the lift's record in its second revision, its codes appended,
    so a statechart reads one type of refusal whatever read its text, and the JSON codec keeps the lift's
    classification, so the shared table holds unchanged (D12).
11. **Is a JSON codec's member order and float notation declared?** Each may be declared, and defaults to the house's
    form (D8): both are canonizers of what law 3 forgets, so they change no reading and need no decision where the
    house's form serves, and a text compared with another producer's bytes declares the producer's. The notations are a
    closed set so every lane prints alike; a new one is an amendment that states its rule.
12. **May a value borrow its text?** Yes, as a view (D6): Wadler's views and ADR 0049's decoded codec give the form, it
    materialises to the value the reader gives (Wadler's law, proved, D14), and on a device it holds spans in place of
    each text's bound. It is read only, and lives no longer than the text.
13. **May a place pick a list's element by a field's value?** Yes, by a typed step of equality on one key field (D12),
    the step of a dictionary lens and of a JSONPath filter on equality; a path is a typed value that any grammar codec,
    RFC 6901's among them, reads from text.
14. **How is a lift modelled before strings are?** As a nondeterministic choice of any value of the location's type or
    any refusal code (D15): it over-approximates the code (Clarke, Grumberg and Long 1994), an exploration judges it
    within the domain it names, and a violation is made concrete or reported as spurious (Clarke et al. 2000).
15. **Is a hot text codec a table walk?** Not where the build names it hot (ADR 0139) once a printed codec's Verus
    specification exists: it is then printed as code from the same plan, checked by Verus for each output and held to
    the same cases (D6, D14); until then every codec is walked. Whether it is hot is measured.
16. **Which recursion?** Three forms (D5): a cycle that binds nothing under a depth bound, a guarded cycle of an
    inductive member under its node bound (ADR 0164), and a chain of operators under the same with its groups under
    their own depth bound; each has its stack bounded by name, and every other cycle is refused.17. **In what order are
    a type's refusals reported?** At the least byte offset (D8), on every reader: it is the order of the text a grammar
    codec keeps, it gives `offset` a meaning, and a streaming reader computes it without holding the text.

## Amended on acceptance

Three reviews, each item by its number there, and what was decided on it: the independent review of the first draft
(verdict: rework), and a downstream consumer's review, on which the record was reworked (2026-11-29); and the second
independent review of the reworked record (verdict: amend, then accept), every one of whose amendments the main session
took on acceptance (2026-11-29). Where a later review changed what an earlier one decided, the earlier entry says so.

### The first review, taken in the rework

1. JSON derived as one grammar from the type did not hold: D8 makes JSON and JSON5 each one fixed syntax plan, checked
   and proved once, and a binding the fold derives per type (Jansson and Jeuring's polytypic reading), proved by T4.
2. The order of refusals: a grammar codec refuses in the order of the text; JSON and JSON5 keep ADR 0087 §2.1 item 5's
   order, a bracket pre-scan, then a syntax pass that records the first refusal of the type and validates on, both
   passes counted in T3 (D6, D8). The first refusal found became the least offset (second review, 1).
3. Linear time: the constants depend only on the plan's shape; names are held distinct by sorting spans per object,
   `O(m log m)` (Ben-Or 1983); a map's puts cost ADR 0108 D8's, the seed passed where a host has no entropy; the
   Consequences no longer say "however crafted" without the terms (D6, D7 law 5). The sort and Ben-Or's bound became a
   trie, linear in the names' bytes (second review, 4).
4. Recursion: D5 admits three forms, a value-free cycle under a depth bound, a cycle of an ADR 0164 inductive member
   under its node bound, and a chain of operators printed with only the groups its precedence needs (Ramsey 1998); forms
   2 and 3 land after ADR 0164. Context counts three census rows that read nested structure.
5. A read value the printer refuses: check 6, `grammar/read-value-not-printable`, cited by law 2 and in T2.
6. The theorems' target: T1 to T4 are proved over the runtime walkers that ship, against the reference's specification
   functions; the reference's check of a plan is executable and proved to imply their premises, run on every printed
   plan; `proofs.json` lists the mutants named (D14).
7. `float32`: one rule for every reader, rounding once and refusing at `2^128 - 2^103`, on every lane, with a table that
   holds a double-rounding witness and `3.4028235e38` (D9). Every writer's `float32` was pinned too (second review, 7).
8. The third difference from "JSON Number Text" (a float that is not finite refused) is listed, and the codec is a
   writer row in the three tables (D8).
9. Duplication of the JSON writer: one `no_std` module in `sce-portable-bytes` that Rust's escaper, number text and the
   codec delegate to (D13).
10. Rust's typed lift becomes the generated JSON codec; `json.rs` keeps what has no type; Open question 8 is of the
    other lanes (D13).
11. Law 1's equality is observational equality of terms, finer than ADR 0151 D2's comparison (D7).
12. A map's JSON form: text, enum and byte-string keys an object, the reliance on member order stated; integer, `bool`
    and record keys an array of pairs; "JSON Object Key Order" names the exception (D10).
13. Check 5 covers merges across every token that follows, as Boomerang's unambiguous concatenation (D2).
14. Check 2 adds at most one empty alternative and no repetition of a part that derives the empty text (D2).
15. Check 4: the LL(1) form of context-aware scanning is this record's, `over` is `lex`'s lexical precedence, the
    condition is stated on the subgraph of live states that accept nothing (D2). Reps's tabulation now serves the points
    where the condition fails (second review, 17).
16. An imported codec's rules: checks run on the composed plan, a conflict naming both codecs (D1).
17. A token outside any binding is refused (check 1); members' names, assignment and separators are specified, names
    distinct after conversion; a text holding U+0000 is `nulInText` (D3).
18. The printer's domain is decided per type: integers as ranges, floats by the notation's language, texts by the image
    of the escape transducer (D4).
19. "Reads what the lift reads" holds with revision defaults, another case's members stepped over, and without raw or
    within places; the list of codes it answers is whole (D8, D12).
20. A value of unknown type: held as normalized text, named `<sce:raw>`, JSON5 under JSON5, an escaped U+0000 refused;
    the inductive `json_value` is a rejected alternative (D11).
21. The pointer's length through a map's keys is computed and refused past `lift_path` (D12).
22. A place: read from its binding site, or by the fold for JSON; built in a scratch the machine reserves and moved in
    on success; justified by the census's rows of a configuration written at a path (D12).
23. The model: `Refuses_C` is ADR 0156 D3's third step; `Printable_{C,p}` is uninterpreted; a record that holds a map is
    refused with maps; the float lemma is an assumption named in `docs/formal/README.md`, binary64 sampled; ADR 0156's
    amendment is cited (D14, D15).
24. The Earley test reads through a naive backtracking scanner of its own (D14).
25. The JSON Schema import recognises `S(T)`'s own forms first, and `S(T)` annotates an enum's numbers, so the law holds
    (D17).
26. One language stated three ways: printed texts are held in `G(T)`, `MAX_TEXT` to `J(T)`; `link-runtime`'s writer and
    the content-line codec are named as what moves next (D13, D14).
27. `sce-rust-regexp` gains the operations the checks need, as step 2, on ADR 0149's steps 4 and 5 (D2).
28. Every check has a grammar it must accept beside one it must refuse; tests for `<sce:within>`, `grammar/raw-type`,
    `grammar/open-enum`, the print refusals, `notInLanguage`, `MAX_TEXT` exact and one byte short, a refusal record
    across revisions, `no_std` and `wasm32` for the JSON steps, and JSONTestSuite's departures (Order of work).
29. Rendel and Ostermann's inverse is stated with its conditions; the verified parsers are cited (the textbook
    structure).
30. Names: `sce:type` on the codec's root, `<sce:literal>`, one name `unknown-members`, required with no default.
31. A codec's type is any type of the grammar (D1).
32. A byte is read at most one more time than the nesting of unions whose tag comes last (D8).
33. ADR 0162's decimal has a conversion and a fold arm; ADR 0166's generic codec is ADR 0168's (D19).

### The consumer's review, taken in the rework

1. C11 is a lane of this record (D16, step 13), proved on Rust and tested on C11 (second review, 8): the consumer
   generates for Rust and C11 from one source, and the walkers need neither a heap nor recursion.
2. Member order and float notation may be declared on a JSON codec, as canonizers of law 3 (D8), each defaulting to the
   house's form (second review, 22). The digits stay the shortest; a notation of another producer joins the closed set
   by an amendment that states its rule, since a notation named by a library and not by its rule cannot be printed alike
   on every lane.
3. A map read in the order of its keys: `order="by-key"`, a value-side quotient (D10), law 1 stated up to it (D7).
4. A view that borrows its text (D6): accepted, as Wadler's views and ADR 0049 give it, read only and no longer than the
   text.
5. Recursion once ADR 0164 is accepted: D5's second form, and "Amends" names ADR 0164 D4 and D12. The depth bound the
   consumer asks of ADR 0164 bounds the reader's stack wherever ADR 0164 gives it.
6. A place step that picks an element by a field's value: a typed path with a match step (D12); a path through a raw
   place is `noSuchPlace` and not a code of its own, one fact being one code.
7. A lift before strings have a model: a nondeterministic choice, of any value of the type or any code, judged within
   the explored domain, its violations made concrete or reported as spurious (D15, second review, 9).
8. A codec on a hot path: printed as code where the build names it hot and measured, once a printed codec's Verus
   specification exists (D6, second review, 8).

### The second review, taken on acceptance

1. A type's refusals are reported at the least byte offset (a missing field at its object's closing `}`, a refusal in a
   kept span at its own offset), the binder keeping the least; ADR 0087 §2.1 item 5 is amended and carries the line;
   step 5 adds the table's cases and moves every lane's lift; D14's differential test draws texts with at most one type
   refusal until a lane has moved (D6, D8, D12, D14).
2. D5's second form admits only guarded cycles, a pass-through cycle refused; a group declares its own depth bound; the
   stack's bound is stated; step 14 tests both.
3. A raw place is held to its encoding when printed, `notInLanguage` at the place, so its codec declares that its encode
   can refuse (D4, D11, the example of D8, step 11).
4. Distinctness by a trie of decoded name bytes where the type admits undeclared names, a bit per declared member, no
   added term; a `by-key` map relinked by radix sort for text and integer keys, merge sort otherwise; `Linear` storage's
   cost per put stated (the textbook structure, D6, D7 law 5, D14 T3, Consequences, Sources).
5. The pre-scan counts an inductive member by its `depth(...)` of ADR 0164 D4 (D8).
6. The binder's inductive arm is ADR 0164 D4's Rust streaming reader, T4 discharges T1b's decode half for JSON, and step
   9 removes the separate reader; ADR 0164 carries the line (D13, Amends).
7. A `float32` is written by its own shortest digits by every writer, pinned in "JSON Number Text", whose "does not fix"
   bullet goes; Kotlin's `3.4028235E38`, which Rust's lift refuses today, is the defect D9 mends; D8's second exception
   is gone; `float32_text.json` gains writing cases (D9, step 4).
8. Each generated sink and each hot codec printed as code is checked by Verus per output, and no hot codec is printed
   until that form exists; C11 is judged at ADR 0164 D12's level, tested now and named unproved; the Consequences say
   "proved on Rust, tested on C11" (D6, D14, D16).
9. A lift is a choice of any value of the type or any code, path and offset left out, its verdict within the explored
   domain, its violation made concrete or reported as spurious (CEGAR) (the textbook structure, D15).
10. The syntax plan emits its derivation's events, the binder is a deterministic pushdown transducer over them, T4 is
    stated over the value tree with a lemma from the syntax plan's T1, and form 1's bound may be computed (D5, D8, D14).
11. JSON5's numbers in typed places are decided, with `json5_refusal.json` (D8).
12. A key read twice in a map's array form is `repeated` (D10, D12).
13. The new codes are 17 to 21, after `tooFew` 14, `notOffered` 15 and `tooManyNodes` 16, with the harness's enum and
    outcome table in Amends (D12).
14. An undeclared member's `unexpected` points at its object, and `refusal.scxml`'s comment is amended (D12).
15. A value written under a match must hold the matched key, else `keyMismatch`; a step's text is read by one rule per
    codec (D12).
16. An integer's printability is decided by inclusion of the range's numerals, its run-time refusal a walk of the
    automaton, the model's predicate uninterpreted unless intervals are recognised (D4, D15).
17. Where check 4's condition fails, Reps's tabulation is printed, its workspace reported and bounded by
    `grammar_scan_table_bits` (`grammar/scan-table-too-large` replaces `grammar/unbounded-lookahead`) (D2, D18).
18. Check 6 on a `float` token holds exactly when the token reads the whole notation's language (D2).
19. Check 5 is CFL-reachability with summary relations, its cost stated, not cut to `k` bytes, `sce-build`'s and tested
    in step 6; the author places a skip (the textbook structure, D2).
20. C11's codec calls `sce_payload_quote_span` and `sce_number_text`, the latter gaining the `float32` writer, and the
    tables gain C11 rows (D13, step 3).
21. `ecmascript` places a `float32` from its own digits' `k` and `n` (D8).
22. `sce:member-order` and `sce:float-notation` default to the house's form; `sce:map-order` and `sce:unknown-members`
    stay required (D8, D18, Open question 11).
23. A view's law is that it materialises to `parse(t)`, proved; a raw place in a view is a span and a mark (D6, D14).
24. Every key but a text, an enum or a byte string, a list key among them, is an array of pairs; the difference from ADR
    0108's saved form is noted (D10).
25. `sce-rust-regexp` stays `no_std` with no heap, the build's operations behind a feature; its path and landing item
    are named (Context, D2, step 2).
26. D19 states ADR 0168's dependence as a constraint, and names D8's binding plan as the form ADR 0168 lets a document
    write.
27. The writer goes in a crate named for it, `sce-json-text` (D13).
28. The reference checks the build's certificates rather than deciding the checks again (McConnell et al. 2011) (D2,
    D14).
29. `offset` is the refusal's offset (D6, D12).

Declined: none.

## Sources

- T. Rendel and K. Ostermann, "Invertible Syntax Descriptions: Unifying Parsing and Pretty Printing", Haskell Symposium,
  2010.
- P. Jansson and J. Jeuring, "Polytypic data conversion programs", Science of Computer Programming, 2002.
- J. N. Foster, M. B. Greenwald, J. T. Moore, B. C. Pierce and A. Schmitt, "Combinators for bidirectional tree
  transformations: a linguistic approach to the view-update problem", POPL 2005; ACM TOPLAS, 2007.
- A. Bohannon, J. N. Foster, B. C. Pierce, A. Pilkiewicz and A. Schmitt, "Boomerang: Resourceful Lenses for String
  Data", POPL 2008.
- J. N. Foster, A. Pilkiewicz and B. C. Pierce, "Quotient Lenses", ICFP 2008.
- P. M. Lewis II and R. E. Stearns, "Syntax-directed transduction", Journal of the ACM, 1968; D. E. Knuth, "Top-down
  syntax analysis", Acta Informatica, 1971; A. V. Aho, M. S. Lam, R. Sethi and J. D. Ullman, *Compilers: Principles,
  Techniques, and Tools*, 2nd ed., Addison-Wesley, 2006, chapters 3 and 4.
- Y. Bar-Hillel, M. Perles and E. Shamir, "On formal properties of simple phrase structure grammars", Zeitschrift für
  Phonetik, Sprachwissenschaft und Kommunikationsforschung, 1961.
- B. Ford, "Packrat parsing: simple, powerful, lazy, linear time", ICFP 2002; "Parsing Expression Grammars: a
  recognition-based syntactic foundation", POPL 2004.
- N. Ramsey, "Unparsing expressions with prefix and postfix operators", Software: Practice and Experience, 1998.
- E. Van Wyk and A. Schwerdfeger, "Context-aware scanning for parsing extensible languages", GPCE 2007.
- T. Reps, "'Maximal-munch' tokenization in linear time", ACM TOPLAS, 1998.
- R. D. Cameron, "Extending context-free grammars with permutation phrases", ACM LOPLAS, 1993; A. I. Baars, A. Löh and
  S. D. Swierstra, "Parsing permutation phrases", Journal of Functional Programming, 2004.
- R. de la Briandais, "File searching using variable length keys", Western Joint Computer Conference, 1959; E. Fredkin,
  "Trie memory", Communications of the ACM, 1960; D. E. Knuth, *The Art of Computer Programming*, volume 3, *Sorting and
  Searching*, 2nd ed., Addison-Wesley 1998, §6.3; A. V. Aho, J. E. Hopcroft and J. D. Ullman, *The Design and Analysis
  of Computer Algorithms*, Addison-Wesley 1974, §3.2.
- M. Yannakakis, "Graph-theoretic methods in database theory", PODS 1990; T. Reps, "Program analysis via graph
  reachability", Information and Software Technology, 1998.
- R. M. McConnell, K. Mehlhorn, S. Näher and P. Schweitzer, "Certifying algorithms", Computer Science Review, 2011.
- W. D. Clinger, "How to read floating point numbers accurately", PLDI 1990; G. L. Steele Jr. and J. L. White, "How to
  print floating-point numbers accurately", PLDI 1990; U. Adams, "Ryū: fast float-to-string conversion", PLDI 2018.
- P. Wadler, "Views: a way for pattern matching to cohabit with data abstraction", POPL 1987.
- E. M. Clarke, O. Grumberg and D. E. Long, "Model checking and abstraction", ACM TOPLAS, 1994; E. M. Clarke, O.
  Grumberg, S. Jha, Y. Lu and H. Veith, "Counterexample-guided abstraction refinement", CAV 2000.
- J. Earley, "An efficient context-free parsing algorithm", Communications of the ACM, 1970.
- L. Sassaman, M. L. Patterson, S. Bratus and A. Shubina, "The Halting Problems of Network Stack Insecurity", ;login:
  36(6), 2011.
- S. Lasser, C. Casinghino, K. Fisher and C. Roux, "A Verified LL(1) Parser Generator", ITP 2019; D. Egolf, S. Lasser
  and K. Fisher, "Verbatim: A Verified Lexer Generator", LangSec 2021; T. Ramananandro et al., "EverParse: Verified
  Secure Zero-Copy Parsers for Authenticated Message Formats", USENIX Security 2019; B. Delaware, S. Suriyakarn, C.
  Pit-Claudel, Q. Ye and A. Chlipala, "Narcissus: Correct-by-Construction Derivation of Decoders and Encoders from
  Binary Formats", ICFP 2019.
- A. Pnueli, M. Siegel and E. Singerman, "Translation validation", TACAS 1998; J. R. Burch and D. L. Dill, "Automatic
  verification of pipelined microprocessor control", CAV 1994.
- RFC 8259 (JSON), RFC 6901 (JSON Pointer), RFC 9535 (JSONPath, §2.3.5 filter selectors), RFC 9485 (I-Regexp), RFC 3629
  (UTF-8), RFC 4648 (base encodings), RFC 9110 (HTTP semantics: §5.6.4 quoted strings, §8.3.1 media types), RFC 8949
  §6.2 (JSON to CBOR), RFC 8785 (JSON canonicalization); The JSON5 Data Interchange Format 1.0.0 (2018) and ECMAScript
  5.1 §7.6 (identifier names); JSON Schema draft 2020-12 (draft-bhutton-json-schema-01 and
  draft-bhutton-json-schema-validation-01); IEEE 754-2019; the JSONTestSuite corpus (N. Seriot, "Parsing JSON is a
  Minefield", 2016), and the documentation of Go's `encoding/json` and Rust's `serde_json`, cited for practice and not
  as a foundation.
- In this repository, read for this record and its rework: ADR 0151 and ADR 0152 whole (the form of the record);
  `SCE_FORGE.md` §4.6 and §4.6.4, §3.4 and §10; ADR 0149 whole; ADR 0087 §1.1 and §2.1 to §2.5; ADR 0101 §1 and §2; ADR
  0092 §2 and §4; ADR 0086 §3 and §4; ADR 0058 §5; ADR 0108 D1, D2, D7, D8 and its open questions; ADR 0154's header;
  ADR 0115 D7; ADR 0156's Context and D1 to D5; ADR 0133 D1 and D2; ADR 0164 as accepted (its header, D4, D12, D13 and
  the table of T1a and T1b) and ADR 0162's draft (its D9 and D10); `ARCHITECTURE.md`'s four JSON sections; the `about`
  of the four tables under `tests/json_text/`; `stdlib/lift/refusal.scxml` and `refusal_code.scxml`;
  `docs/formal/proofs.json`'s form; the heads of `backends/rust/runtime/src/json.rs`, `event_payload.rs` (its `float32`
  reader whole) and `backends/rust/link-runtime/src/json.rs`.

The papers, the RFCs and the JSON5 specification are cited from the texts as known and were not read again for this
record or its rework: no passage of them was checked while writing it. The double-rounding witness of the textbook
structure and D9 (`1+2^-24` as a binary64, then `3f800000` by ties to even, where rounded once it is `3f800001`), the
values `f32::MAX = 2^128 - 2^104` and the threshold `2^128 - 2^103` of IEEE 754-2019 §4.3.1, and `3.4028235e38` lying
between them, were computed while writing and checked exactly by the second review. The passages its decisions rest on
are to be checked against the texts before acceptance or in the step that builds each: Jansson and Jeuring's proof of
the round trip, and Rendel and Ostermann's conditions for the printer to be the parser's inverse and their `chainl1`
(D3, D5, T2); the LL(1) conditions with empty parts and the uniqueness of an LL(1) derivation (D2); Reps's quadratic
case and his tabulation (D2, check 4); Van Wyk and Schwerdfeger's construction (check 3); Boomerang's unambiguous
concatenation and dictionary lenses (check 5, D12); Ramsey's rule for the parentheses an expression needs (D5);
Cameron's permutation phrases (D3, D8); the linear bound of a trie and of lexicographic sorting (D6); Yannakakis's and
Reps's CFL-reachability and its cost (check 5); CEGAR's treatment of a spurious counterexample (D15); Ford's result that
a PEG's emptiness is undecidable ("Alternatives rejected"); the JSON5 specification's grammar of names, numbers and
escapes and what it says of a repeated name (D8); RFC 9110's grammar of a media type and of a quoted string (the example
of D3); JSON Schema 2020-12's meaning of `maxLength` and of an absent `additionalProperties` (D17); RFC 9535's filter
selector (D12); and RFC 8785's number and key order ("Alternatives rejected").
