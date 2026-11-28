# ADR 0163 — A text is read and printed by one grammar, checked to read every text one way in linear time, and JSON is a grammar derived from a type

- Status: Proposed (2026-11-28), for the owner's acceptance after an independent review.
- Landed: nothing. No step of "Order of work" is built, and no theorem, model or test of it has been run.
- Date: 2026-11-28
- Written under the owner's standing rule (the most long-term correct structure, textbook in its foundations, cost
  ignored; decided, not asked) and the owner's scope rule of 2026-11-22: a facility is built on the backends a consumer
  needs, Rust by default; its contract is general, and what is special to one application stays in the application. A
  downstream consumer's census asked for it; every example below is a standard's or a generic one.
- Amends, once accepted (each amended document gains an "Amended by: ADR 0163" line in the landing that moves it):
  - `SCE_FORGE.md` §4.6 (the encodings `grammar`, `json` and `json5`, as sections 4.6.5 and 4.6.6, D1) and §8 (the codec
    row names its encodings);
  - `schemas/sce-forge-ext.xsd` (the `encoding` enumeration, `sce:of`, and the grammar's elements; `pre-release`);
  - ADR 0065 §2 and `sce-build/src/forge/codec_totality.rs`: the places a text codec's encode refuses (D4, D7);
  - ADR 0086 §3.1 and `ARCHITECTURE.md` "JSON Number Text": a text codec prints a negative zero as `-0` and a `float32`
    by its shortest spelling (D7); the rows that govern `_event.data` and the typed payload's writer are unchanged;
  - ADR 0087 §2.1 (`<sce:lift>` names a text codec and a place, D10), §2.3 (four codes appended, D10) and §2.4 (the
    refusal record's second revision, with `offset`, D10);
  - ADR 0108 D7 and its Open question 1: a record's field may hold a map or a set where a text codec reads and writes
    it, and the JSON form of a map (D8);
  - ADR 0115 D7: `Refuses_C` gains the places of D4 and D7, and the statements of D10 are read as functions the model
    does not interpret (D11);
  - `docs/SCE_ACCEPTED_SUBSET.md` (§2.15: a record that holds a map, `<sce:print>`; the appendix rows of D14) and
    `SCE_ERROR_CONTRACT.md` (the same codes);
  - `stdlib/lift/refusal.scxml` and `stdlib/lift/refusal_code.scxml` (D10).
- Scope: `sce-build` (the grammar's reader, its checks and its plan; the fold of a type into its JSON and JSON5
  grammars; the JSON Schema import; the statements; the model's reading of a text codec), a new crate
  `sce-grammar-reference` (the reference reader and printer the theorems are stated over), the Rust forge runtime
  (`sce_forge_runtime::grammar`: one reader and one printer for every plan) and the Rust lane, native, `no_std` with no
  heap, and `wasm32-unknown-unknown`. Every other backend, and the Interpreter, refuses a text codec by name (D12). The
  automata of the tokens are ADR 0149's.
- Related: ADR 0010, 0014, 0015 and 0104 (the content-line codec), ADR 0044 (revisions), ADR 0050 and ADR 0086 (a bound
  by name, part of a type; the JSON wire form), ADR 0055 (optional), ADR 0058 (a codec's laws), ADR 0065 (declared
  totality of an encode), ADR 0077 (every decode refuses what breaks a bound), ADR 0081 (an encode planned once), ADR
  0087 (the payload grammar, `<sce:lift>` and its refusal record), ADR 0092 (a union and its JSON form), ADR 0094 (the
  source hash), ADR 0099 (a failure is not a value), ADR 0101 (one deterministic form per type), ADR 0102 and ADR 0111
  (a codec bound at a boundary), ADR 0108 and ADR 0154 (maps and their storage), ADR 0115 and ADR 0156 (the formal
  levels and the generated model), ADR 0133 (totality where the build proves it), ADR 0149 (automata in linear time, the
  Unicode pin, well-formed text), ADR 0155 (a cold codec is a table one walker reads), ADR 0158 (the pin of Unicode).

## Context

What exists, read from the tree at `662e4d060cc`:

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
  each written because the readers, read in their code, disagreed on what a text is. The Rust reader builds a tree
  (`json::Value`, a number kept in its spelling) that the lift then walks, and is `std` only.
- **JSON is written for a deployment's queryables** by `backends/rust/link-runtime/src/json.rs` (ADR 0041): a declared
  document measured by a counting sink, then written into the caller's buffer.
- **A type's JSON grammar is written, for a model to answer in.** `sce-build/src/forge/value_type.rs` folds every type
  into its JSON Schema `S(T)`, its GBNF grammar `G(T)` (ADR 0087 §1.1) and its largest JSON text `J(T)`
  (`json_max_len`); `payload_schema.rs` tests the GBNF with an Earley recognizer (Aycock and Horspool). No reader is
  generated from any of them.
- **Regular languages.** ADR 0149 (accepted on 2026-11-28; no crate `sce-rust-regexp` is in the tree yet) gives I-Regexp
  (RFC 9485) compiled at build time to a minimal deterministic automaton within `regexp_states`, and the rule that every
  `string` is well-formed UTF-8 on every lane (its D8). It leaves out captures and the position of a match, so a regular
  expression says whether a text has a form and never what its parts are.
- **Maps.** ADR 0108 gives `map<K, V>[B]` and `set<K>[B]`, a text key among them, ordered by when each key was put (D2).
  A map in an event schema's field, and so in any codec, is refused as `validation/map-in-wire-position` (D7), and its
  wire form is that record's Open question 1. ADR 0154 lets a Rust map keep its slots in storage its host lends.
- **Text in expressions.** A text is joined with `+` (to a string or an integer) and measured by `len` (`SCE_FORGE.md`
  §3.4); nothing cuts it, searches it or reads a structure out of it. `stdlib/text/utf8_length.scxml` writes RFC 3629's
  grammar by hand, byte by byte, as an algorithm.
- **No value of unknown type.** ADR 0101 makes a CBOR entry any value of the one type grammar, and every value of that
  grammar is typed; no type of the language holds a value whose type is not known where it is read.
- **No JSON Schema is read.** `schemars` and `jsonschema` are development dependencies of `sce-build` that describe and
  check SCE's own surfaces (ADR 0087, Context).
- **What the model generator does with a codec.** ADR 0115 D7 (amended 2026-11-23) reads a codec as a partial function
  of values: the values its encode refuses (`Refuses_C`, from `codec_totality::refusals`) and the round trip of every
  other (ADR 0058 §5, law 1); a frame a decode refuses enters only as a channel's `corruption`. A string has no model
  yet (`sce-build/src/formal/document_model.rs`, `Construct`).

A downstream consumer that moves a network stack's logic into documents counted what stays in host code for want of
these: 85 rows of parsing, printing, cutting and comparing structured text (addresses, keys, selectors, parameter lists,
schema files) and of tables keyed by such text, and 27 rows of JSON and JSON5 documents (configuration, administrative
replies, profiles) held, taken apart and written. Its own recommendation was a codec of text grammars, a format of its
own that refuses by rule, rather than a widening of the content-line encoding. This record is that codec, with JSON and
JSON5 as two of its encodings, each derived from a type.

## The textbook structure

- **A syntax is described once, and parsing and printing are its two readings.** Rendel and Ostermann ("Invertible
  Syntax Descriptions: Unifying Parsing and Pretty Printing", Haskell Symposium 2010) build a description from partial
  isomorphisms between a value and its parts, and from product, choice and repetition, and read one description as a
  parser and as a printer; each is the other's partial inverse because both are read off the same description. Jansson
  and Jeuring ("Polytypic data conversion programs", Science of Computer Programming 2002) derive a printer and a parser
  from a datatype's structure, with the round trip as the property they prove.
- **A transformation in two directions has laws, and forgets only what it says it forgets.** Foster, Greenwald, Moore,
  Pierce and Schmitt ("Combinators for bidirectional tree transformations: a linguistic approach to the view-update
  problem", POPL 2005; TOPLAS 2007) state a lens's laws. Boomerang (Bohannon, Foster, Pierce, Pilkiewicz and Schmitt,
  "Boomerang: Resourceful Lenses for String Data", POPL 2008) gives lenses between strings and values, typed by regular
  languages. Quotient lenses (Foster, Pilkiewicz and Pierce, ICFP 2008) hold the laws up to an equivalence that
  canonizers name: whitespace, the order of fields, the spelling of an escape. The value is the abstract side and the
  text the concrete one: reading what was printed gives the value exactly, and printing what was read gives the text up
  to the equivalence the grammar declares.
- **A grammar is unambiguous by construction when it is LL(1)** (Lewis and Stearns, "Syntax-directed transduction", JACM
  1968; Knuth, "Top-down syntax analysis", Acta Informatica 1971; Aho, Lam, Sethi and Ullman, *Compilers: Principles,
  Techniques, and Tools*, 2nd ed., Addison-Wesley 2006, §4.4). The tokens each alternative of a choice can begin with
  are disjoint, and so, for a part that can be empty, are its first tokens and what may follow it; the build decides
  this by the FIRST and FOLLOW sets. Such a grammar has at most one derivation of each text, and a predictive parser
  finds it with one token of lookahead, without backtracking, in work linear in the text. A Parsing Expression Grammar
  (Ford, "Parsing Expression Grammars: a recognition-based syntactic foundation", POPL 2004) is unambiguous too, by
  ordered choice, and a packrat parser reads it in linear time with a table as large as the text times its rules (Ford,
  "Packrat parsing: simple, powerful, lazy, linear time", ICFP 2002); D2 and "Alternatives rejected" say why it is not
  chosen.
- **The tokens are regular languages, read by automata, and the text is cut by the longest match** (Aho et al., chapter
  3). A scanner told by the parser which tokens it expects where it stands (context-aware scanning: Van Wyk and
  Schwerdfeger, "Context-aware scanning for parsing extensible languages", GPCE 2007) reads the small languages in which
  a token of one place is text of another, as an address and a name are. The longest match costs time quadratic in the
  text when a token can be read far past its end before the attempt fails (Reps, "'Maximal-munch' tokenization in linear
  time", TOPLAS 1998); a scanner whose read past its last accepting state is bounded stays linear.
- **A record's fields may come in any order** in some syntaxes (a JSON object, a list of properties). A permutation
  phrase (Cameron, "Extending context-free grammars with permutation phrases", ACM LOPLAS 1993; Baars, Löh and
  Swierstra, "Parsing permutation phrases", Journal of Functional Programming 2004) reads each member once in any order,
  where writing every order out as alternatives would be exponential in the members and not LL(1).
- **A number read from text and printed again is itself only when both conversions are exact.** A decimal is read to the
  nearest binary floating number, correctly rounded (Clinger, "How to read floating point numbers accurately", PLDI
  1990), and the shortest decimal that reads back to the same number is printed (Steele and White, "How to print
  floating-point numbers accurately", PLDI 1990; Adams, "Ryū: fast float-to-string conversion", PLDI 2018). A binary32
  read by way of a binary64 can be rounded twice and miss the nearest binary32.
- **Recognise the whole input as a language before acting on it** (language-theoretic security: Sassaman, Patterson,
  Bratus and Shubina, "The Halting Problems of Network Stack Insecurity", ;login: 2011): a reader of a deterministic
  context-free language at the boundary, which hands the logic a typed value or a refusal and nothing in between. Logic
  that cuts and compares text where a typed value would do repeats the parser at every use and disagrees with it at
  some.
- **A specification can stand beside the program as a second, naive program** the program is held to: here the general
  context-free parser of Earley ("An efficient context-free parsing algorithm", CACM 1970), which finds every derivation
  of a text, so a test sees whether a text has none, one or several.

## Decision

### D1. A text codec is a codec of one more encoding, and its value is a type of the language

`sce:encoding="grammar"` on a codec's root (`SCE_FORGE.md` §4.6) reads and writes a text by a grammar the document
declares (D2, D3); `sce:encoding="json"` and `sce:encoding="json5"` read and write one by a grammar the build derives
from a type (D7). The three are **text codecs**. A text codec names the type it reads and writes, `sce:of="record:R"` or
`sce:of="union:U"` of an imported event schema or union, so what it gives is a value a statechart holds, sends, saves
and compares; it declares no entries of its own. A grammar codec may import another, and a rule may read the imported
codec's root rule in place, as a byte codec embeds another (`<sce:embed>`).

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
the one spelling a table keyed by text is keyed by (D6, law 2). A rewrite that is not a matter of spelling (an algebra
of the value) is the document's own algorithm over the typed value.

### D2. A grammar is LL(1) over regular tokens, and the build proves that it reads every text one way

A grammar document declares:

- **terminals**, each a regular language written in I-Regexp (RFC 9485, ADR 0149 D1):
  - `<sce:text>`, a literal (`/`, `;`, a keyword), matched as written or, with `case="ascii-insensitive"`, in either
    case of its ASCII letters, and printed as written;
  - `<sce:token name regexp value>`, a token, whose lexeme one conversion of a closed set turns into a value (D3);
  - `<sce:skip name regexp print>`, a stretch read and forgotten (layout, a comment), which may be empty, printed as its
    `print` spelling (which a use may override);
- **rules**, `<sce:rule name type>`, each giving a value of one type, whose body is a sequence of terminals, rules and
  the binding forms of D3. The first rule is the root, of the type `sce:of` names.

The build reads the document into a **grammar plan**, the one form of a written grammar and of a derived one (D7), and
holds the plan to five checks. Each is a decision procedure over finite sets of rules and over automata, and each
refusal names the place and a witness text:

1. **No token matches the empty text** (`grammar/empty-token`). A skip may, and is then a part that can be empty under
   check 2.
2. **LL(1).** At each choice the FIRST sets of the alternatives are disjoint, and wherever a part can be empty, its
   FIRST set and the FOLLOW set of where it stands are disjoint (Aho et al. §4.4.3); otherwise `grammar/not-ll1`, naming
   the rule, the two alternatives (or the part and what follows it) and a token both can begin with. A rule that reaches
   itself is refused (`grammar/recursive-rule`): a type of the language does not contain itself (ADR 0086 §1.5), so a
   value has no place for a derivation that does, and admitting one is a question of the type grammar, with a depth
   bound by name, not of this record. A grammar with no recursion has a deepest derivation, which the build computes and
   which bounds the reader's stack.
3. **One token at each point.** Where the parser stands, the terminals it expects (the FIRST set of what may come next)
   are scanned by the longest match. Two of them whose languages share a text are refused (`grammar/terminals-overlap`,
   with the text) unless one is declared to win, `<sce:token ... over="other">` (as a keyword wins over a name). The one
   that loses then reads only what the winner does not, a language that is the difference of two regular ones and so
   regular, and a value the printer would spell in the winner's language is a place the encode refuses (D4).
4. **A bounded read past each token.** At each point, the deterministic automaton of the expected terminals (their
   union, by ADR 0149's construction) must not read an unbounded stretch past an accepting state and then fail: no cycle
   of live states that do not accept is reachable from an accepting one. Otherwise `grammar/unbounded-lookahead`, with
   the two terminals and a family of texts that grows the read. Under this check the scanner reads at most `k` bytes
   past the end of each token, `k` below the automaton's number of states, so the longest match costs linear time (Reps
   1998 for why it otherwise does not).
5. **What the printer writes is read back as the same tokens.** For two terminals that can stand next to each other in a
   derivation, the build decides whether a lexeme of the first followed by a lexeme of the second can be scanned at that
   point as anything but those two (the language of the first, followed by each non-empty prefix of the second, meets a
   language expected there). Where it can, the grammar places a skip between them whose `print` spelling keeps them
   apart, which the build checks the same way; otherwise `grammar/tokens-merge`, with the two terminals and the text
   that merges. A literal's text, and each skip's `print`, must be in the language read where it stands
   (`grammar/canonical-spelling-not-read`).

The automata of checks 3 to 5 are built per point within `grammar_scanner_states`, a bound by name (ADR 0050) whose
library number is 4096, the number ADR 0149 gives `regexp_states` for the same textbook budget. A grammar whose automata
pass it is refused (`grammar/scanner-too-large`, naming the point, the count and the bound), and is not simulated as ADR
0149 D2 simulates a large regular expression, because checks 3 to 5 are decided on the deterministic automaton. Each
token's own expression is held to `regexp_nfa_states` (ADR 0149 D2) as every regular expression is, and a grammar whose
classes are expanded from the Unicode data declares `UNICODE_VERSION` as ADR 0149 D4 requires.

Under checks 1 to 5 the grammar, read with its scanner, has at most one derivation of each text, and its reader finds
that derivation with one token of lookahead and no backtracking (D6, law 4; T1 of D11).

### D3. A rule binds its parts to a value, and each binding is a partial isomorphism

| Form | Binds | Read | Printed |
|---|---|---|---|
| `<sce:field name="f" rule=".."/>` or `token=".."` | field `f` of the rule's record | the rule or token, into `f` | `f`, by that rule or token |
| a rule's body | a record: each field bound exactly once, in the order the document writes; a terminal carries no value | in order | in order |
| `<sce:option field="f">` | `f` of `optional<T>`, present when its body is read | the body, when the next token is in its FIRST set | the body when `f` is present, nothing when absent |
| `<sce:list field="f">`, with an optional `<sce:separator>` | `f` of `list<T>[M..B]` | elements while one begins; past `B` `tooMany`, short of `M` `tooFew` | each element in order, separated |
| `<sce:map field="f">`, one `<sce:key>` and one `<sce:value>` | `f` of `map<K, V>[B]`, or of `set<K>[B]` with no value | each entry put in the order read; a key read twice `repeated`; past `B` `tooMany` | each entry in the map's own order (ADR 0108 D2) |
| `<sce:members>` of `<sce:member name=".." field="..">` | a record's fields by name, in any order (a permutation phrase) | a member's name, then what it names, each at most once; a required one absent `missing`; one read twice `repeated`; an undeclared name refused (`unexpected`) or stepped over by a declared `<sce:skip-member>`, as `unknown="refuse"` or `"skip"` says (no default) | the present members, in the order declared |
| `<sce:cases>` of `<sce:case name="c">` | a union's case, or an enum's variant | the alternative whose FIRST set holds the next token | the alternative of the value's case |
| `<sce:spellings>` of two or more `<sce:spelling>` | one value written more than one way | the alternative whose FIRST set holds the next token | the first alternative whose language holds the value |

A token's `value` names one conversion of a closed set, each a partial isomorphism between the lexemes it reads and a
type, with one canonical spelling it prints:

- `text`: the lexeme itself, into a `string[B]`. With `case="ascii-lower"` it is read in either case and held in lower
  case, so a value with an ASCII capital is outside what the printer can write (D4). With `escapes` it is unescaped by
  one scheme, `json` (RFC 8259 §7), `json5` (JSON5 1.0.0 with ECMAScript 5.1's escapes), `quoted-pair` (RFC 9110 §5.6.4)
  or `c` (the escapes of C), read with any escape the scheme admits and printed with the one the scheme makes canonical
  (for `json`, `ARCHITECTURE.md` "JSON Text");
- `decimal` and `hex`: an integer of the field's type, read with whatever sign and leading zeros the token's language
  admits, printed with none but a `-` below zero (`hex` in lower case);
- `float`: a `float32` or `float64`, read correctly rounded into the type's own format, printed by the shortest digits
  that read back to the same value (D7 states the notation);
- `hex-bytes`, `base64` and `base64url` (RFC 4648): a `bytes[B]`, printed in the RFC's canonical form.

The build checks that every lexeme of the token is one the conversion reads (`grammar/conversion-mismatch`, with a
lexeme it does not), and every binding against the field it binds (`grammar/binding-mismatch`: a field bound twice or
never, a list form on a field that is no list, a rule or a conversion of another type, a key of a type ADR 0108 D1 does
not admit). These are Rendel and Ostermann's combinators: a rule's body is their product, with the record's constructor
as its isomorphism; `<sce:cases>` and `<sce:spellings>` are their choice; a list and a map their repetition; a
conversion an isomorphism on one token.

**An example: RFC 9110's media type** (§8.3.1), without the empty parameter RFC 9110 admits (`;;`) and without its
octets above 0x7F in a quoted string. Its record holds a map, which D8 admits:

```xml
<!-- media_type.scxml, an event schema read as a record -->
<data id="type"       sce:type="string[media_name=127]"/>
<data id="subtype"    sce:type="string[media_name=127]"/>
<data id="parameters" sce:type="map&lt;string[media_name=127], string[media_value=1024]&gt;[media_params=16]"/>
```

```xml
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0"
       sce:kind="codec" sce:encoding="grammar" sce:codec-format="2" name="media_type_text"
       sce:of="record:MediaType" sce:encode-may-fail="true">
  <sce:import kind="event-schema" src="media_type.scxml" as="MediaType"/>
  <sce:token name="name"   regexp="[!#$%&amp;'*+.^_`|~0-9A-Za-z-]+" value="text" case="ascii-lower"/>
  <sce:token name="token"  regexp="[!#$%&amp;'*+.^_`|~0-9A-Za-z-]+" value="text"/>
  <sce:token name="quoted" regexp="&quot;([\t !#-\[\]-~]|\\[\t -~])*&quot;" value="text" escapes="quoted-pair"/>
  <sce:skip  name="ows"    regexp="[ \t]*" print=""/>
  <sce:rule name="media_type" type="record:MediaType">
    <sce:field name="type" token="name"/>
    <sce:text>/</sce:text>
    <sce:field name="subtype" token="name"/>
    <sce:map field="parameters">
      <sce:skip ref="ows"/><sce:text>;</sce:text><sce:skip ref="ows" print=" "/>
      <sce:key token="name"/>
      <sce:text>=</sce:text>
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
empty and `;` is not in its language, so check 2 holds; `" "` is in `ows`, so check 5 holds.
`TEXT/HTML ;Charset="utf-8"` reads to `{type: "text", subtype: "html", parameters: {charset: "utf-8"}}`, which prints as
`text/html; charset=utf-8`: the case of the names, the layout and the quotes the value did not need are what the grammar
forgets. A parameter value holding a control character other than a tab is in neither spelling's language, and a type
holding an ASCII capital is outside `name`'s, so the codec's encode can refuse (D4) and says so on its root.

### D4. The printer's domain is computed, and what it cannot print is a refusal the build finds

A value of the type may have no text in the grammar: a text outside its token's language (a name holding a space), a
text a winning keyword takes (check 3), an integer the token does not spell, a value outside a conversion's image (an
ASCII capital under `ascii-lower`), a float that is not finite where the encoding has no spelling for it (D7). For each
binding the build computes the canonical spellings of the values of the field's type, a regular language for every
bounded type, and its difference with what the token reads there; where it is not empty, the printer refuses such a
value, `notInLanguage` at that place. These are places an encode refuses (ADR 0065 §2), held to the codec's
`sce:encode-may-fail` by `codec_totality` as every such place is (`codec/undeclared-encode-refusal`,
`codec/encode-may-fail-without-refusal`), and they are the disjuncts of `Refuses_C` the model reads (D11). A codec that
can print every value is total, and its printer returns only its sink's error (ADR 0065).

### D5. One reader and one printer over the plan, in linear time and bounded space

- **The plan is data, and one walker reads it in each direction.** `sce-build` prints a codec's plan as tables: the
  scanner's deterministic automata by point, the LL(1) prediction table, the binding sites and their conversions.
  `sce_forge_runtime::grammar` holds one reader and one printer for every plan, as ADR 0149 D2 prints one walk for every
  automaton and ADR 0155 gives every cold codec one walker. The generated code is the plan, and a sink for the type with
  one method per binding site, which builds the value as the reader drives it and presents it as the printer asks.
- **Linear time.** The reader takes each token by the longest match with at most `k` bytes read past it (check 4), and
  between two tokens makes at most a number of predictions the plan bounds, since no rule reaches itself. Its work on a
  text of `n` bytes is at most `c * n + d`, `c` and `d` constants of the plan the build reports. The printer writes each
  part once.
- **Bounded space and no heap.** The reader's stack is the plan's deepest derivation, sized when it is built; what it
  builds is the value, in the storage its type is held in (ADR 0077 §7, ADR 0108 D4, ADR 0154). The printer writes into
  a sink, and the longest text it can write, `MAX_TEXT`, is a fold of the plan the build computes (each token's longest
  canonical lexeme from the type's bounds, each list and map at its bound), as `J(T)` is for JSON; a buffer of that size
  never overflows, and a counting sink measures a value first.
- **The text read is a `string`**, well-formed UTF-8 (ADR 0149 D8). At a boundary that hands the reader bytes, it holds
  them to RFC 3629's grammar first, and a byte sequence that is not UTF-8 is `unexpected` at its first ill-formed byte.
- **A refusal is one record** (D10): its code, the byte offset where the reader stopped, and the place in the value
  being read, as an RFC 6901 pointer. It is the first refusal in the order of the text, and nothing of a refused text is
  written into the place the reader was asked to fill.

### D6. The laws

For a text codec whose plan passes D2's checks, `parse` its reader and `print` its printer:

1. **Round trip.** For every value `v` of the type that `print` accepts (D4), `parse(print(v)) = v`. Equality is that of
   the values' deterministic forms (ADR 0101), so the sign of a zero, a map's order and a list's order count.
2. **One spelling.** For every text `t` that `parse` accepts, `parse(print(parse(t))) = parse(t)`: `print(parse(t))` is
   the one spelling of `t`'s value, and printing after reading is idempotent. It follows from law 1, since `parse(t)` is
   a value `print` accepts.
3. **The quotient.** `print(parse(t))` differs from `t` only by what the grammar declares it forgets: the text a skip
   read, the case of what is read without regard to case, an escape's spelling, a number's spelling, which of several
   spellings was read, the order of a `<sce:members>`'s members, and a member stepped over. Nothing else of `t` is lost,
   and a map's entries keep the order they were read in.
4. **One way.** Each text has at most one derivation in the plan, and `parse` is a function of the text.
5. **Linear.** `parse` does at most `c * n + d` steps on a text of `n` bytes, and `print` at most `c' * m + d'` on a
   text of `m` bytes it writes.

These are ADR 0058 §5's law 1 (totality and the round trip) for a codec of text, stated for every value and every text,
and D11 says at which level each is judged.

### D7. JSON and JSON5 are grammars derived from a type

- **The fold.** For every type `T` of the language the build derives the plan of `T`'s JSON grammar by structural
  recursion over `ValueType`, beside `S(T)`, `G(T)` and `J(T)` in `value_type.rs`, one exhaustive match whose arms are
  the constructors, so a constructor added to the type grammar is a compile error until the fold handles it (ADR 0087
  §1.1). A record is a `<sce:members>` of its fields between `{` and `}`; a union is the members of its tag and of every
  case's fields, the case named by the tag (ADR 0092 §2.1); a list is a `<sce:list>` between `[` and `]` separated by
  `,`; a scalar is a token and its conversion; RFC 8259's whitespace is a skip, printed as nothing, between every two
  tokens. Every derived plan passes D2's checks by construction, and a test holds every type of the corpus to them.
- **It reads what the lift reads.** With `sce:unknown-members="skip"` the JSON codec of `T` reads exactly the language
  every lane's lift reads for `T` (ADR 0086 §4, ADR 0087 §2.1 item 5, ADR 0092 §4), and refuses each text with the code,
  pointer and bound `tests/json_text/payload_refusal.json` gives it: that table is its conformance table. With
  `"refuse"`, a member its type does not declare is refused `unexpected` at its pointer. The attribute is required on a
  JSON or JSON5 codec and has no default (`validation/missing-attribute`): whether a text read from outside may carry
  what its type does not name is the document's decision, and the lift's choice was made for a schema's later revisions
  (ADR 0044), not for a file a person writes.
- **It prints ADR 0086 §3.1's spelling**, which every lane's typed payload writer writes: keys ascending by their UTF-8
  bytes (`ARCHITECTURE.md`, "JSON Object Key Order"), a union's tag among them; a text as "JSON Text" writes it; a
  64-bit integer as the text of its digits; a byte string as the Latin-1 text of its bytes; an absent optional as `null`
  under its key. It differs where that spelling is not invertible, and only there:
  - a negative zero is printed `-0`: ECMAScript's Number::toString, which "JSON Number Text" follows, writes `0`, which
    reads back as a positive zero, and ADR 0101 holds the two apart;
  - a `float32` is printed by the shortest digits that read back to the same `float32`, in Number::toString's notation
    (decimal when `1e-6 <= |x| < 1e21`, exponent otherwise): "JSON Number Text" pins only the 64-bit form;
  - a `float64` is printed as `real_text.json` writes it, the sign of zero aside.

  A number is read correctly rounded into its own format (Clinger 1990), a `float32` directly and never through a
  `float64`. A float that is not finite has no JSON spelling: the printer refuses it (`notInLanguage`, a place of D4),
  and no text reads to one.
- **A union's tag may come after its case's fields.** A JSON object's members are unordered (RFC 8259 §4), so the reader
  keeps the span of each member it cannot read before the tag is known, at most one per member name the union's cases
  declare, and reads each once the tag is read. Each byte is then scanned at most twice, and the reader stays linear and
  needs no heap: the spans are offsets into the text it holds whole.
- **JSON5** (The JSON5 Data Interchange Format, 1.0.0, 2018) is the same fold over JSON5's tokens: a member name that is
  an identifier of ECMAScript 5.1, whose letters are General Categories I-Regexp names, or a single- or double-quoted
  string; comments and JSON5's white space as skips; a trailing comma; JSON5's numbers (hexadecimal, a leading or
  trailing point, a plus sign, `Infinity` and `NaN`) by its conversion; its escapes. A name read twice is `repeated`. It
  prints what the JSON codec prints, and a float that is not finite as `Infinity`, `-Infinity` or `NaN` (the one NaN of
  ADR 0101 §2.1), so it refuses no float. JSON is a subset of JSON5, so the round trip holds.
- **An open enum** names no text for a number no variant declares, and a JSON or JSON5 codec whose type reaches one is
  refused (`grammar/open-enum`), as ADR 0087 §1.1 refuses its grammar.

```xml
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0"
       sce:kind="codec" sce:encoding="json5" sce:codec-format="2" name="node_config_text"
       sce:of="record:NodeConfig" sce:unknown-members="refuse">
  <sce:import kind="event-schema" src="node_config.scxml" as="NodeConfig"/>
  <sce:import kind="codec" src="endpoint_text.scxml" as="EndpointText"/>
  <sce:within at="/listen/*" codec="EndpointText" bound="endpoint_text=256"/>
  <sce:capture at="/extensions/*"/>
</scxml>
```

Here `NodeConfig` holds `mode`, an `enum:Mode`; `listen`, a `list<record:Endpoint>[listen_max=8]`; and `extensions`, a
`map<string[extension_name=64], string[extension_text=4096]>[extensions_max=16]`. The text
`{ mode: 'peer', listen: ['tcp://0.0.0.0:4000'], extensions: { store: { volumes: 2 } }, }` reads to a value whose
endpoint is a record the grammar codec `EndpointText` read out of the string (D9), and whose `store` extension is held
as the text `{"volumes":2}` until a codec of its type reads it (D9). It prints as
`{"extensions":{"store":{"volumes":2}},"listen":["tcp://0.0.0.0:4000"],"mode":"peer"}`.

### D8. A record may hold a map where a text codec reads it, and a map's JSON form

This amends ADR 0108 D7, and decides its Open question 1 for the text encodings.

- An event schema's field may be a `map<K, V>[B]` or a `set<K>[B]` where the schema is the type of a text codec, of a
  `sce-static` variable, or of a `<sce:lift>` or `<sce:print>` location, on a lane that serves text codecs (D12). Every
  other position (a delivery's payload, a CBOR codec, a signed value, a send's data) keeps refusing such a schema as
  `validation/map-in-wire-position`, which then names the position and the field. The CBOR form of a map stays ADR
  0108's question.
- **In JSON and JSON5, a map whose key is a text, an enum or an integer is an object** keyed by the key's text: the text
  itself, the variant's name, the integer's decimal digits (read by the `decimal` conversion with no leading zero, so
  `"007"` is `badNumber`). A map whose key is a `bool`, a byte string or a record is an array of two-element arrays,
  `[key, value]`. A set is an array of its keys. A key read twice is refused: in a JSON object as every repeated name is
  (`notJson`, ADR 0087 §2.3), elsewhere `repeated`.
- **A map is printed in its own order and read in the text's**, so the round trip keeps its order (D6, law 1): the same
  entries in another order are another value of the language (ADR 0108 D2), and printing them sorted would break the law
  for every map whose keys were not put in order.

### D9. A value of unknown type is captured as its text, and a string may hold another codec's text

- **A capture.** A JSON or JSON5 codec may name places whose value its type does not state, `<sce:capture at="..."/>`,
  each a field of `string[B]` (`grammar/capture-type` otherwise). `at` is an RFC 6901 pointer into the type, in which
  `*` stands for every element of a list and every value of a map. The reader reads any value of the encoding there,
  holds its nesting to what the text's bound allows (one bit per level, an object or an array, at most `B / 2` levels,
  so no other bound is named), and keeps its canonical text: no skip and no trailing comma, every name and string
  double-quoted in the encoding's canonical escapes, names in the order read, a number as written, a repeated name
  refused, and `tooLong` past `B` bytes. The printer writes the text as it is held. A capture is read later by a codec
  of a type, by `<sce:lift>` (D10), and nothing walks it before: no untyped tree is handed to logic, and every value a
  document computes on is a value of a type, as ADR 0101 has it, with no new constructor of the type grammar for ADR
  0092 §12 to hold on every lane.
- **A text within a text.** `<sce:within at="..." codec="Alias" bound="name=N"/>` reads the JSON string at each place
  `at` names by another text codec, into a field of that codec's type. `bound`, a bound by name, is the most bytes the
  unescaped string may hold (`tooLong` past it) and the buffer it is unescaped into; on the way out the inner value is
  printed, at most the inner codec's `MAX_TEXT`, and escaped. A field of another type, or a `MAX_TEXT` past `bound`, is
  `grammar/within-type`. Both laws compose, as two lenses composed are a lens (Foster et al. 2007).

### D10. A statechart reads a text into a value, at a place, and prints a value

- **Reading.** `<sce:lift>` (ADR 0087 §2.1) takes `codec="Alias"`, an imported text codec, in place of `schema`: `text`
  is read by that codec, and `location` is of the codec's type or an optional of it (`validation/lift-codec` otherwise).
  Everything else is as ADR 0087 §2.1 states it: the statement raises nothing, and writes `location` whole or `refusal`,
  never both.
- **A place, by a pointer.** `at="name"` names a `string[N]` holding an RFC 6901 pointer into the type of `location`.
  The pointer is read by a grammar the build derives from that type, whose language is the pointers to its places (`~0`
  and `~1` unescaped; a field's name; an index or `-` under a list; a key under a map, read as D8 reads one), so it is
  resolved in one pass over its text. The text is read as a value of the type at that place, and that place alone is
  replaced: a field of a record; a field of a union's case while the union holds that case; a list's element within its
  length, or `-` to append one; a map's key, put. A pointer that names no place of the value is refused `noSuchPlace`,
  and an append to a full list or a put into a full map `tooMany`. This is a lens's `put` through a path (Foster et al.
  2007).
- **Printing.** `<sce:print codec="Alias" expr="..." location="text" at="name" refusal="why"/>` writes the text of the
  value of `expr` (or of the place `at` names in it) into a `string[N]` (`validation/print-location` otherwise). It is
  total exactly where the build proves it (ADR 0133): the codec's printer refuses nothing of the value's type (D4), its
  `MAX_TEXT` is at most `N`, and there is no `at`. Then a `refusal` is refused; otherwise one is required, an
  `optional<record:LiftRefusal>` (`validation/print-refusal` for either), and the statement writes `location` or the
  refusal, never both.
- **The refusal record and its codes.** `sce:std/lift/refusal.scxml` gains, in its second revision (ADR 0044), `offset`,
  an `optional<uint32>`: the byte offset where a text codec's reader stopped, present for a statement through a text
  codec and absent for a lift by `schema`, so a lift by schema writes the same record on every lane, as today.
  `sce:std/lift/refusal_code.scxml` gains, after its fourteen codes:
  - `unexpected` (15): where the reader stands, no token it expects; the text ended where one was expected; text after
    the value; a member the type does not declare, under `"refuse"`. In the JSON codec a text that is not JSON stays
    `notJson`, as ADR 0087 §2.3 classes it, so the shared table holds;
  - `repeated` (16): a member, a name or a key read twice, but a JSON object's repeated name, which stays `notJson`;
  - `notInLanguage` (17): a value the printer has no text for (D4);
  - `noSuchPlace` (18): a pointer that names no place of the value.

  Every other refusal keeps its code: `missing`, `tooLong`, `tooMany`, `tooFew`, `outOfRange`, `badNumber`,
  `unknownVariant`, `tooDeep`, `nulInText`. A text codec bound at a boundary (ADR 0102, ADR 0111) is bound as any codec
  is, and its decode's failure (ADR 0099) is the same code, offset and pointer.

### D11. How each claim is judged, and how the generated model reads a text codec

Each claim is judged at the levels of ADR 0115, and none of D6's laws rests on a random test alone.

| Claim | Level | Why that level |
|---|---|---|
| D2's checks decide what they say | the build's algorithms, each refusal a case that must be refused by its code; every printed plan checked again by `sce-grammar-reference` before it is printed (ADR 0156 D11) | decision procedures on automata and finite rules; checking each output is cheaper and smaller than proving the generator (Pnueli, Siegel and Singerman 1998) |
| T1, one way (law 4): a plan that passes the checks has at most one derivation of each text, and the reader computes it | Verus theorem for every plan and every text, over the specification functions of `sce-grammar-reference` (ADR 0115 D3, D13), named in `docs/formal/README.md` and `docs/formal/proofs.json` with what it takes as given | a statement of every input; a test samples it |
| T2, round trip (law 1, and law 2 as its corollary) | Verus theorem, by induction on the derivation the printer chooses, given T1, check 5, and a lemma per conversion that it is a partial isomorphism on its canonical spellings (`text` with each escape scheme, `decimal`, `hex`, the byte conversions) | the walker and those conversions are small and inductive |
| The `float` conversion's round trip | tests: every `float32`, all 2^32 patterns; `float64` on every case of `real_text.json` and on values drawn from recorded seeds, each against the exact rational arithmetic of `sce-grammar-reference` | correct rounding is a theorem about the reals whose machine proof is a work of its own; an exhaustive test is complete for binary32, and binary64 is held to an exact oracle |
| T3, linear (law 5) | Verus theorem: the walker's ranking function bounds its steps by `c * n + d`, from check 4's `k` and the plan's depth; and step counts in tests on texts of 1 to 2^20 bytes, as ADR 0149 D7 counts them | a bound on every input, and a count a test reads where a clock is noise |
| Law 3, the quotient | property test: texts drawn from each plan's derivations with random skips, cases, escapes, spellings and member orders, each read and printed and compared with the original up to the declared equivalence | the equivalence is the grammar's declaration, and is tested as one |
| The JSON codec reads the lift's language (D7) | `payload_refusal.json` replayed through the codec; a differential test against Rust's lift on texts drawn from recorded seeds; the RFC 8259 corpus of JSONTestSuite read through a capture | an equality of two programs' languages, tested where they can differ |
| A text has none, one or several derivations | property test: Earley's parser over the plan's rules, on the token sequence the scanner gives, finds exactly the reader's derivation or none | an independent second reading of the same grammar |

**The generated model** (ADR 0115 D7, ADR 0156 D3):

- **At a boundary**, and wherever a send's data is encoded by a text codec, a text codec is read as ADR 0115 D7 reads
  every codec: by its values, with `Refuses_C` one disjunct per place of D4 (D7's floats that are not finite among
  them), which `sce-codegen model` writes from `codec_totality` as for any codec, and with the round trip, which T2
  proves of the code. A text its decode refuses enters as the channel's `corruption`.
- **The statements of D10** read and write a `string`, which has no model yet (`Construct` in
  `sce-build/src/formal/document_model.rs`; ADR 0156 D3 puts the finite domains of a string first in its order). Until
  it has one, `sce-codegen model` refuses a document that holds `<sce:lift codec>`, `<sce:print>` or `at` by a
  `Construct` of its own, `TextCodec`, and writes no partial model. Once strings have a model it reads `Parse_C` and
  `Print_C` as functions it does not interpret (Burch and Dill, CAV 1994, as ADR 0149 D6a reads `matches`), constrained
  only by what this record makes true of every input: `Parse_C` is total, a value of the type or one refusal of D10's
  closed set; `Parse_C(Print_C(v)) = v` for every `v` outside `Refuses_C` (T2); the value of
  `Parse_C(Print_C(Parse_C(t)))` is that of `Parse_C(t)` (law 2); `Print_C(v)` is at most `MAX_TEXT` bytes. A property
  the model proves then holds for every grammar and every text, and a property that needs one function's value at one
  input is not claimed.
- **A record that holds a map** (D8) is read as a map variable is, and refused by the same name wherever a map is.
- A plan has no TLA+ module of its own: a grammar is a function, not a protocol, and a function is judged by a theorem
  over every input (ADR 0115 D3, D13), where a model explores states of a machine.

### D12. Lanes

- **Rust** serves every construct of this record, native, `no_std` with no heap, and on `wasm32-unknown-unknown`.
- **Every other backend** (Kotlin, Python, Go, Swift, C++, C11) and the Interpreter's lowering refuse, by name, a
  document that holds a text codec, a `<sce:lift codec>`, a `<sce:print>` or a record that holds a map (D8):
  `codegen/text-codec-not-served`, naming the lane and the construct, so a lane that does not serve one says so and
  never answers otherwise. The lift by `schema` is unchanged on every lane. The JSON Schema import (D13) writes ordinary
  documents and serves every lane.
- **A lane joins** by printing the plan and writing the two walkers and the conversions, which have no heap and no
  recursion, in the change that lands its tests against the reference's cases. C11 is the first candidate, since a
  device's image reads addresses at its boundary; it joins in a step of its own at a consumer's request, as ADR 0109
  opened its C11 lane.

### D13. A JSON Schema is imported at build time as the documents it states

- `<sce:import kind="json-schema" src="catalog.schema.json" as="Catalog"/>` reads a JSON Schema (draft 2020-12) when the
  build runs and gives the build the event schemas, unions and enums it states and the JSON codec of its root, as if
  they were documents; `sce-codegen import json-schema <file>` prints those documents, for a reader and for a repository
  that prefers to commit them. The schema file is an input of every generated unit that reads it, and its source hash
  covers it (ADR 0094). It is one source shared with tools outside SCE, as a `.proto` file is to be imported by a record
  of its own once a type may contain itself.
- **The mapping is the inverse of `S(T)`** (ADR 0087 §1.1, ADR 0092 §3): an `object` with `properties` and `required` is
  a record, a property not in `required` an optional field, `"additionalProperties": false` giving
  `sce:unknown-members="refuse"` and its absence `"skip"`; an `object` whose `additionalProperties` is a schema and
  whose `propertyNames` are bounded is a `map<string[K], V>[B]`, `maxProperties` its bound; an `array` with `items` and
  `maxItems` a list, `minItems` its lower count; a `oneOf` of objects each holding one `const` under one key, a union
  tagged by that key; an `enum` of strings a closed enum; an `anyOf` of a schema and `{"type": "null"}` an optional; an
  `integer` whose `minimum` and `maximum` are a type's range, that type; a `number` a `float64`; a `string` a
  `string[B]`; a `$ref` into `$defs` the type it names. For every type `T` of the corpus, importing `S(T)` gives `T`
  back.
- **A bound is part of a type**, so a text, a list or a map with no bound is refused (`import/json-schema-unbounded`,
  naming the pointer) unless the import names one for that place by name, `<sce:bound at="..." bound="peer_name=64"/>`
  (ADR 0050). A text's bound counts bytes and `maxLength` counts characters: `x-sce-max-bytes`, which `S(T)` writes, is
  the bound; `maxLength` alone is refused as unbounded unless the import names a byte bound, and a byte bound named
  below four times `maxLength` narrows the schema's language, which the generated documents state in their comment.
- **Refused by name, never dropped**: a keyword that narrows a value within its type (`pattern`, `format`, `multipleOf`,
  `minimum` and `maximum` that are no type's range, `minLength`, `uniqueItems`, a `const` outside a union's tag,
  `dependentRequired`, `if`, `not`, `patternProperties`), as `import/json-schema-refinement`; a keyword outside the
  mapping (`allOf` beyond one `$ref`, `prefixItems`, `unevaluatedProperties`, a `type` list beyond one type and `null`),
  as `import/json-schema-unsupported`; a cycle of `$ref`s, as `import/json-schema-recursive` (D2's reason); a `$ref`
  that names nothing, as `import/json-schema-unresolved`. A refinement is a predicate over a type's values, a facility
  of the type system and not of an import; refusing it keeps the codec's language the schema's, or says where it is not.

### D14. Refusals

Each is a `DiagnosticCode` with its appendix row in `docs/SCE_ACCEPTED_SUBSET.md` and its entry in
`SCE_ERROR_CONTRACT.md`, added in the step that makes it: `grammar/empty-token`, `grammar/not-ll1`,
`grammar/recursive-rule`, `grammar/terminals-overlap`, `grammar/unbounded-lookahead`, `grammar/tokens-merge`,
`grammar/canonical-spelling-not-read`, `grammar/scanner-too-large`, `grammar/conversion-mismatch`,
`grammar/binding-mismatch`, `grammar/open-enum`, `grammar/capture-type`, `grammar/within-type`, `validation/lift-codec`,
`validation/print-location`, `validation/print-refusal`, `import/json-schema-unbounded`,
`import/json-schema-refinement`, `import/json-schema-unsupported`, `import/json-schema-recursive`,
`import/json-schema-unresolved` and `codegen/text-codec-not-served`. Existing codes it uses:
`validation/map-in-wire-position` (narrowed by D8), `validation/missing-attribute` (`sce:unknown-members`, `unknown` or
`sce:of` absent), `codec/undeclared-encode-refusal` and `codec/encode-may-fail-without-refusal` (D4).

The bounds by name: `grammar_scanner_states` (D2), ADR 0149's `regexp_nfa_states` on each token, a `<sce:within>`'s
`bound` (D9), and the bounds of the types read and printed (ADR 0050, ADR 0086). Every other size (the reader's stack,
the spans of D7, a capture's nesting, `MAX_TEXT`) is computed from those by the build and reported.

Construct by construct:

| Construct | What the build checks | Its refusals | Its bounds by name | Lanes (D12) | The generated model (D11) |
|---|---|---|---|---|---|
| A grammar codec (D2 to D5) | checks 1 to 5, the bindings, the conversions, the printer's domain against `sce:encode-may-fail` | `grammar/*` of D2 and D3, `codec/undeclared-encode-refusal`, `codec/encode-may-fail-without-refusal` | `grammar_scanner_states`, `regexp_nfa_states`, its type's | Rust; every other lane and the Interpreter `codegen/text-codec-not-served` | by its values: `Refuses_C` and the round trip |
| A JSON or JSON5 codec (D7) | the fold passes D2's checks; `sce:unknown-members` declared; no open enum | `grammar/open-enum`, `validation/missing-attribute` | its type's | as above | as above, with the floats that are not finite in JSON's `Refuses_C` |
| A record that holds a map (D8) | the positions the schema stands in | `validation/map-in-wire-position` | the map's | Rust; refused by name elsewhere | as a map variable, refused where a map is |
| A capture, a text within a text (D9) | the field's type; the inner codec's type and `MAX_TEXT` | `grammar/capture-type`, `grammar/within-type` | the field's text bound; `bound` | Rust | by the value of the field |
| `<sce:lift codec>`, `<sce:print>`, `at` (D10) | the location's type; the totality of a print (ADR 0133) | `validation/lift-codec`, `validation/print-location`, `validation/print-refusal` | the location's and the text's | Rust | `Construct::TextCodec` until a string has a model, then `Parse_C` and `Print_C` uninterpreted |
| The JSON Schema import (D13) | the mapping of each keyword, every bound | `import/json-schema-*` | the schema's, or named on the import | every lane (its output is documents) | its types, as any types |

## Not in scope

A grammar given at run time (a document read when the program runs is a codec document read at run time, a question of
its own); a rule that reaches itself, and so nested structure of unbounded depth, until a type may contain itself; an
ambiguous grammar, and a general context-free parser at run time; PEG; a context-sensitive rule (a count or a length
that says how much text follows, a closing tag that repeats an opening one, layout by indentation); reading a text in
pieces as it arrives; reporting more than the first refusal, and recovering after one; the content-line encoding, which
keeps its own runtime (its folding of lines at 75 octets is below its tokens, and no grammar of this record states it);
case folding beyond ASCII, collation and normalisation of what is read (ADR 0149 decided that a decode does not
normalise); the refinements of a JSON Schema (D13); YAML, TOML and every other format, each an encoding or a grammar of
its own when one is needed; the move of every lane's lift onto the generated reader (Open questions, 8); and any one
application's grammars.

## Consequences

- A text with structure is read once, at the boundary or by one statement, into a typed value or a refusal record; logic
  computes on the value and never on the text, and writes text only by printing a value.
- A grammar is checked when it is built: an ambiguity, a token that swallows its neighbour, a read that could run
  quadratic and a value the printer cannot write are each refused or declared before anything runs.
- Reading costs time linear in the text and no heap, on every grammar, by theorem; no text, however crafted, makes a
  reader backtrack.
- Reading what was printed gives the value back, by theorem, and printing what was read gives the one spelling of its
  value, so a table keyed by text can be keyed by the canonical spelling.
- JSON and JSON5 need no grammar written: the type is the grammar, and the JSON codec reads the language every lane's
  lift reads.
- A record may hold a map on Rust where a text codec reads it, and a map has a JSON form.
- A JSON Schema shared with other tools is one source; what the type grammar cannot state is refused by name and not
  dropped.
- A value of unknown type is a bounded text until a type reads it.
- A new format of text is a grammar document, and none is written again by hand in seven runtimes.

## Alternatives rejected

- **A Parsing Expression Grammar.** Its ordered choice makes the language of a choice depend on the order of its
  alternatives, so a printer can write a text that the reader takes by an earlier alternative and reads as another
  value: the round trip then needs a check PEG does not give. Packrat parsing is linear only with a table as large as
  the text times the rules, and whether a PEG's language is empty is undecidable (Ford 2004), so a build cannot say of a
  PEG what D2 says of an LL(1) grammar.
- **A general context-free parser at run time** (Earley, GLR). Cubic work in the worst case, and an ambiguity is found
  only when a text with two readings arrives, at run time. Earley's parser is kept as the test's second reading (D11).
- **A reader and a printer written by hand for each format.** Two statements of one grammar that drift apart: the JSON
  readers and writers of this repository disagreed until four shared tables held them, and the content-line codec is one
  grammar written seven times.
- **A scanner with no context, one longest match for the whole text** (lex). In the small languages of addresses and
  parameters, one place's token is another place's text (an IPv4 address is a host name, a number is a name), and a
  scanner that does not know where the parser stands refuses grammars a context-aware one reads one way.
- **A grammar over characters with no tokens, LL(1) character by character.** A keyword and a name, a number and its
  exponent, need more than one character of lookahead, so the grammar would be LL(k) for a large `k` or rewritten around
  its lexemes by hand; regular tokens read by automata keep the parser LL(1) and the scan linear.
- **Regular expressions with captures as the parser.** A regular language has no nesting and no repetition with
  structure (a list of records), and which part a capture holds differs between engines; ADR 0149 left captures out for
  that reason.
- **Extending the content-line encoding.** It is RFC 5545's grammar, with that format's lines, parameters and folding; a
  general grammar would be a second language inside one format's attributes.
- **An untyped JSON tree handed to documents.** Logic would walk a tree whose shape no build has checked, and every walk
  would refuse or misread in its own way: the gap the census names.
- **A capture held as the CBOR of the value** (RFC 8949 §6.2's conversion from JSON). The conversion chooses a type for
  each number and loses its spelling, and JSON5's numbers have no form there; the text is exact, and the lift that will
  read it exists.
- **RFC 8785's canonical JSON as the printed form.** Its numbers are binary64 only, so a 64-bit integer is not printed
  exactly; it writes a negative zero as `0`; and it orders keys by UTF-16 code units, where every writer of this
  repository orders them by code point.
- **A map printed in the order of its keys.** It breaks the round trip for every map whose keys were not put in order
  (D8).
- **A new type constructor for a value of unknown type.** ADR 0092 §12 would hold it on every lane, and a document could
  compute on it only by walking it, which is what is refused.
- **Refinements of a JSON Schema checked by the codec** (a `pattern` as a facet). A facet beside a type is a second type
  system, held by the codec and not by the statechart that makes the value; refinement types are a decision of the type
  grammar.

## Order of work

Each step lands with a test that fails without it.

1. This record, accepted.
2. **ADR 0149's automata**, its steps 4 and 5, if that record has not landed them: the tokens are compiled by its
   construction.
3. **The grammar's reader, its checks and its plan** (D1 to D4, the build's refusals of D14):
   `sce-build/tests/a_text_grammar_is_checked_to_read_every_text_one_way.rs`. Red without it: `sce:encoding="grammar"`
   is not an encoding. Each refusal has a case that must be refused by its code: `a | a b` (`grammar/not-ll1`); a host
   name and an IPv4 address expected at one point (`grammar/terminals-overlap`); tokens `a` and `a*b` at one point
   (`grammar/unbounded-lookahead`); two names with nothing between them (`grammar/tokens-merge`); a rule that reaches
   itself; a field bound twice; `decimal` on `[0-9a-f]+` (`grammar/conversion-mismatch`). And
   `a_value_outside_its_tokens_language_is_an_encode_refusal_the_build_finds` in `sce-build/tests/codec_totality.rs`:
   the media type of D3 declared total is `codec/undeclared-encode-refusal`.
4. **The reference**, `sce-grammar-reference`: the specification functions, the second check of each plan, T1, T2 and T3
   proved in Verus and named in `docs/formal/README.md` and `docs/formal/proofs.json`, its command
   `sce-codegen grammar eval <codec> <text>`, and cases written from recorded seeds by
   `tests/forge/conformance/gen_cases.py` with the values the reference gives:
   `sce-build/tests/a_grammar_reads_what_it_prints.rs` (laws 1 to 3 on the reference, for every grammar of the corpus,
   and Earley's parser finding exactly the reference's derivation). Red without it: the crate does not exist, and a plan
   with an ambiguity the build missed is printed.
5. **The Rust walkers and the generated codecs** (D5): `backends/rust/forge-conformance/tests/forge_text_grammar.rs`,
   with `a_printed_value_is_read_back_as_itself` (values from recorded seeds, every grammar of the corpus),
   `a_text_is_read_as_the_reference_reads_it` (value, or code, offset and pointer),
   `a_text_read_and_printed_is_its_one_spelling`, `reading_takes_steps_linear_in_the_text` (texts of 1 to 2^20 bytes, a
   count exact per byte, red for a walker that rescans), `the_media_type_grammar_reads_rfc_9110s_examples`, run native,
   under `no_std` and on `wasm32-unknown-unknown`. Red without it: the runtime has no walker.
6. **JSON and JSON5** (D7): `backends/rust/forge-conformance/tests/forge_json_codec.rs`, with
   `the_json_codec_refuses_as_the_shared_table_says` (`payload_refusal.json` replayed through the codec),
   `the_json_codec_reads_as_the_lift_reads` (texts from recorded seeds, against Rust's lift),
   `a_value_prints_as_the_typed_payload_writer_writes_it` (equal bytes for every value whose floats hold no negative
   zero and no `float32`), `every_float32_is_read_back_as_itself`, `a_float64_is_read_back_as_itself`,
   `negative_zero_is_printed_as_minus_zero`, `a_union_whose_tag_comes_last_is_read`,
   `json5_reads_the_examples_of_its_specification_and_prints_json` and `the_json_test_suite_is_read_as_rfc_8259_says`
   (each accepted text through a capture, each refused text refused). Red without it: `sce:encoding="json"` is not an
   encoding.
7. **A record that holds a map** (D8), on Rust: `sce-build/tests/a_record_holds_a_map_where_a_text_codec_reads_it.rs`
   (the media type of D3 is generated; the same schema as a delivery's payload or a CBOR entry is
   `validation/map-in-wire-position`, naming the position) and, in `forge_json_codec.rs`,
   `a_map_is_printed_in_its_order_and_read_back_in_it` (a map whose keys were put out of order). Red without it: the
   media type's schema is refused.
8. **Captures, texts within texts, the statements and the record's second revision** (D9, D10):
   `backends/rust/scenarios/tests/text_codec.rs`, with `a_lift_through_a_text_codec_keeps_its_refusal_and_its_offset`,
   `a_lift_by_schema_writes_no_offset`, `a_print_that_cannot_refuse_has_no_refusal_path` (the generated code holds no
   error path, ADR 0133 D2), `a_place_named_by_a_pointer_is_replaced_whole`, `a_pointer_to_no_place_is_refused`,
   `a_capture_is_read_later_by_a_codec_of_its_type`, run native and on `wasm32-unknown-unknown`. Red without it:
   `<sce:lift codec>` is `validation/lift-codec` and `<sce:print>` an unknown element.
9. **The model** (D11): in the tests of `sce-build/src/formal/document_model.rs`,
   `refuses_c_names_every_place_a_text_codec_refuses` (the media type's two places as two disjuncts) and
   `a_statement_over_a_text_codec_is_refused_by_name_by_the_model_generator` (`Construct::TextCodec`). Red without it:
   the model's `Refuses_C` of the media type is empty.
10. **The lanes** (D12): `a_text_codec_is_refused_by_name_on_a_lane_that_does_not_serve_it`, one assertion per backend
    and the Interpreter, for a text codec, `<sce:lift codec>`, `<sce:print>` and a record that holds a map. Red without
    it: a lane generates a document it cannot run.
11. **The JSON Schema import** (D13): `sce-build/tests/a_json_schema_is_imported_as_the_types_it_states.rs`, with
    `importing_the_payload_schema_of_a_type_gives_the_type_back` (every type of the corpus) and a case per refusal. Red
    without it: `kind="json-schema"` is `import/kind-mismatch`.
12. The documents named in "Amends".

## Open questions

Each is decided here, with its reason; the acceptance may overturn any.

1. **LL(1), or PEG?** LL(1) over regular tokens with a context-aware scanner. It is unambiguous by a check the build
   makes and names a conflict in, reads in linear time with a stack the grammar bounds and no table as large as the
   text, and its derivation is unique, which is what the round trip rests on (T2). PEG's ordered choice gives a unique
   parse but not that the printer's text is parsed by the printer's alternative.
2. **A kind, or an encoding?** An encoding of the codec kind (D1): it reads and writes a value of a type from bytes by
   rules fixed at build time, which is a codec, and SCE_FORGE §10 names the scoped extension of a kind as this form.
3. **In what order is a map printed?** In its own (D8): the round trip is stated of values, and a map's order is part of
   its value (ADR 0108 D2).
4. **Is a negative zero printed `-0`, and a `float32` by its own shortest digits?** Yes (D7): otherwise law 1 fails for
   both, and the rows of "JSON Number Text" that govern `_event.data` stay as they are.
5. **Does `sce:unknown-members` default to the lift's `skip`?** No default: a type read from a file a person writes
   should refuse a misspelt member, a type read from a newer peer should step over it, and only the document knows which
   it is.
6. **How is a value of unknown type held?** As its canonical text, read later by a codec of a type (D9); not as a new
   type constructor, which would be held on every lane and walked by logic, and not as CBOR, which loses a number's
   spelling.
7. **Is C11 built with Rust?** No: the owner's scope rule builds on the backend a consumer needs, Rust by default; the
   walkers have no heap and no recursion, so C11 joins in a step of its own when a consumer asks (D12).
8. **Does every lane's lift move onto the generated reader?** Not in this record. A delivery's `_event.data` is read on
   each lane into that lane's tree, which serves the data model that is not typed as well as the lift, and moving it is
   a change of the delivery, not of the codec; the JSON codec is held to the lift's language by the shared table and a
   differential test (D7, D11), so the two do not drift while both exist.
9. **What bound does a JSON Schema's `maxLength` give?** None alone: it counts characters and a type counts bytes, so
   the byte bound is `x-sce-max-bytes` or one the import names, and a narrower language is stated (D13).
10. **One refusal record, or one for text codecs?** One: the lift's record in its second revision, its codes appended,
    so a statechart reads one type of refusal whatever read its text, and the JSON codec keeps the lift's
    classification, so the shared table holds unchanged (D10).

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
- B. Ford, "Packrat parsing: simple, powerful, lazy, linear time", ICFP 2002; "Parsing Expression Grammars: a
  recognition-based syntactic foundation", POPL 2004.
- E. Van Wyk and A. Schwerdfeger, "Context-aware scanning for parsing extensible languages", GPCE 2007.
- T. Reps, "'Maximal-munch' tokenization in linear time", ACM TOPLAS, 1998.
- R. D. Cameron, "Extending context-free grammars with permutation phrases", ACM LOPLAS, 1993; A. I. Baars, A. Löh and
  S. D. Swierstra, "Parsing permutation phrases", Journal of Functional Programming, 2004.
- W. D. Clinger, "How to read floating point numbers accurately", PLDI 1990; G. L. Steele Jr. and J. L. White, "How to
  print floating-point numbers accurately", PLDI 1990; U. Adams, "Ryū: fast float-to-string conversion", PLDI 2018.
- J. Earley, "An efficient context-free parsing algorithm", Communications of the ACM, 1970.
- L. Sassaman, M. L. Patterson, S. Bratus and A. Shubina, "The Halting Problems of Network Stack Insecurity", ;login:
  36(6), 2011.
- A. Pnueli, M. Siegel and E. Singerman, "Translation validation", TACAS 1998; J. R. Burch and D. L. Dill, "Automatic
  verification of pipelined microprocessor control", CAV 1994.
- RFC 8259 (JSON), RFC 6901 (JSON Pointer), RFC 9485 (I-Regexp), RFC 3629 (UTF-8), RFC 4648 (base encodings), RFC 9110
  (HTTP semantics: §5.6.4 quoted strings, §8.3.1 media types), RFC 8949 §6.2 (JSON to CBOR), RFC 8785 (JSON
  canonicalization); The JSON5 Data Interchange Format 1.0.0 (2018) and ECMAScript 5.1 §7.6 (identifier names); JSON
  Schema draft 2020-12 (draft-bhutton-json-schema-01 and draft-bhutton-json-schema-validation-01); the JSONTestSuite
  corpus (N. Seriot, "Parsing JSON is a Minefield", 2016), cited for practice and not as a foundation.
- In this repository, read for this record: ADR 0151 and ADR 0152 whole (the form of the record); `SCE_FORGE.md` §4.6
  and §4.6.4, §3.4 and §10; ADR 0149 whole; ADR 0087 §1.1 and §2.1 to §2.5; ADR 0101 §1 and §2; ADR 0092 §2 and §4; ADR
  0086 §3 and §4; ADR 0058 §5; ADR 0108 D1, D2, D7 and its open questions; ADR 0154's header; ADR 0115 D7; ADR 0156's
  Context and D1 to D5; ADR 0133 D1 and D2; `ARCHITECTURE.md`'s four JSON sections; the `about` of the four tables under
  `tests/json_text/`; `stdlib/lift/refusal.scxml` and `refusal_code.scxml`; the heads of
  `backends/rust/runtime/src/json.rs`, `event_payload.rs` and `backends/rust/link-runtime/src/json.rs`.

The papers, the RFCs and the JSON5 specification are cited from the texts as known and were not read again for this
record: no passage of them was checked while writing it. The passages its decisions rest on are to be checked against
the texts before acceptance or in the step that builds each: Jansson and Jeuring's proof of the round trip (the textbook
structure); Rendel and Ostermann's statement that a description's printer is its parser's partial inverse (D3, T2); the
LL(1) condition with empty parts and the uniqueness of an LL(1) derivation (D2); Reps's quadratic case and its remedy
(D2, check 4); Cameron's permutation phrases (D3, D7); Ford's result that a PEG's emptiness is undecidable
("Alternatives rejected"); the JSON5 specification's grammar of names, numbers and escapes and what it says of a
repeated name (D7); RFC 9110's grammar of a media type and of a quoted string (the example of D3); JSON Schema 2020-12's
meaning of `maxLength` and of an absent `additionalProperties` (D13); and RFC 8785's number and key order ("Alternatives
rejected").
