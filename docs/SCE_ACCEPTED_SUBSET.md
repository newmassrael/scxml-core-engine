# SCE Accepted Subset

**Positive-form counterpart to `SCE_ERROR_CONTRACT.md`.** The error
contract catalogues the signals SCE emits when it *rejects* an input;
this document catalogues what SCE *accepts*. Upstream automation (LLM
drafters, IDE drafters, repair loops) should consult this doc to
determine whether a given SCXML document is in the accepted subset
before invoking `sce-codegen` — turning acceptance into a static
property instead of a trial-and-error loop.

Audience: authors and tooling producing SCXML for SCE, not runtime
consumers of generated code. Runtime semantics (W3C execution order,
event routing, datamodel behaviour) are documented in `ARCHITECTURE.md`
and the W3C SCXML recommendation; this doc is **build-time acceptance
only**.

The appendix at the bottom enumerates every `DiagnosticCode` the
toolchain can emit and partitions them by whether the author can
prevent them by writing better SCXML (*Acceptance boundary*) or not
(*Diagnostic-only*, i.e. I/O and infrastructure failures). The
enumeration is kept honest by the `acceptance_doc_covers_every_code`
test in `sce-build/src/forge/diagnostic.rs` — adding a new
`DiagnosticCode` variant without listing it here breaks the build.

---

## §1 W3C SCXML inclusions

SCE-codegen accepts the W3C SCXML 1.0 subset that all five backends
(C++, Kotlin, Rust, Go, Python) currently ship at **202/202** parity
against the W3C IRP suite. See the per-backend memoranda for the
concrete pass sets — `rust_backend_next_steps.md`,
`python_bindings_progress.md`, `go_backend_status.md`,
`kotlin_lua_engine.md` (JVM/Android), and the C++ test targets in
`tests/CMakeLists.txt`.

The accepted surface comprises:

- **Core constructs**: `<state>`, `<parallel>`, `<final>`, `<history>`
  (shallow and deep — W3C §3.10.2 requires the default `<transition>`
  child; see [state-reference
  resolution](#statechart-state-reference-resolution)), `<initial>`,
  nested compound states.
- **Transitions**: event triggers, cond guards, targets, target sets,
  `type="internal"`, eventless transitions, wildcard event
  descriptors (`*`, `event.*`).
- **Executable content**: `<onentry>`, `<onexit>`, `<if>`/`<elseif>`/
  `<else>`, `<foreach>`, `<raise>`, `<send>` (internal targets,
  `#_internal`, `#_parent`, `#_invokeid`, delayed sends),
  `<cancel>` (W3C §6.3 — MUST carry `sendid` or `sendidexpr`; the
  both-empty shape is rejected at parse time, wire
  `validation/require-either`), `<assign>`, `<log>`, `<script>`.
- **Datamodel**: `<datamodel>` / `<data>` with expression-language
  assignment, under the two data models SCE implements — see
  [the `datamodel` attribute](#the-datamodel-attribute) for which
  values are accepted, what the Null data model withholds, and what
  declaring `ecmascript` currently obliges.
- **Invoke**: `<invoke type="scxml">` with inline `<content>` or
  external `src`, static param binding via `<param>` and `<finalize>`.
- **HTTP event processor**: `<send type="BasicHTTPEventProcessor">`
  for W3C §C.2 conformance (all five backends tested against the
  `HttpAotTest` harness).
- **Communication**: `_ioprocessors`, `_sessionid`, `_name`,
  `_event` (excluding the exclusions listed in §3).

**Identifier-bearing attributes are checked against the grammar W3C
gives them, at parse.** W3C SCXML types a state's `id` as `ID`
(§3.3.1, §3.4.1, §3.7.1 — *"A valid id as defined in [XML Schema]"*)
and describes an event name as alphanumeric tokens separated by `.`
(§3.12.1). `schemas/sce-forge.xsd` is `xs:any lax` for W3C structural
elements and cannot enforce either, so `scxml_identifier::reject_malformed`
does: one sweep of the post-expansion document from
`SCXMLParser::parse_impl`, driven by a table of every attribute W3C
types `ID`, `IDREF(S)` or event. Until 2026-09-14 nothing did, and a
document with `<state id="s0*/X">` and `<transition event="go*/Y"
target="done*/Z">` generated without a diagnostic — the ids became
code identifiers, `…_STATE_S0*/X` in C and `S0*/X = 1` in Python, so
the emitted source did not compile.

**An `sce:` attribute on a W3C element has a reader, or is refused.**
The same `lax` wildcard admits any `sce:*` attribute on a W3C element,
and XSD 1.0 cannot say which global attribute may sit on which element.
So both parsers read every `sce:` attribute through one channel
(`sce_attr`) that records the read on the parse's ledger
(`read_ledger`), and when the parse ends every `sce:` attribute on a W3C
element that nothing asked for is refused as
`validation/sce-attribute-unread`, one record per attribute at its own
row. What an element takes is therefore the parser's own reads, not a
list kept beside them. Until 2026-09-28 none of this was checked:
`sce:unresolved` on an `<onentry>`, `sce:req` on a forge root, a CBOR
codec's input `sce:type`, and 79 conformance documents' `sce:codec-id`
were all accepted and read by nobody. Two kinds of attribute are left
alone on purpose: one a reader consults and chooses not to carry
(`sce:unit`, documentation only per SCE_FORGE.md §3.3; a `<data>`'s
typed I/O declaration outside `datamodel="sce-static"`), which is
acknowledged at the site that makes the choice; and one inside a value
carried as written — `<content>`, a `<data>` or `<assign>` value — which
is the value's, for whoever reads it.

This was never the comment-encoding problem §2.10 describes, and the
repair is not an encoder. The comments are already safe: the
generator encodes every value a template writes into one, ids and
free-text fields alike. What the grammar is owed for is the CODE an id
becomes — an identifier, and the string literals §2.10 registers as
open — and an id that satisfies the W3C grammar cannot carry `*/`, a
line terminator or a trailing `\` there either.

Two grammars, because W3C gives two, and they differ at the first
character: an XML Name may not begin with a digit and an event token
may. Both readings were measured rather than recalled, and each
refutes the other's over-reach — taking §3.12.1's "alphanumeric"
literally rejects W3C's own conformance documents 364 and 576, whose
event is `In-s11p112`, while reading it as an XML Name rejects
§3.12.1's own calculator example, `<transition event="DIGIT.0">`. The
wire codes are `validation/malformed-identifier` and
`validation/event-name-grammar`, split so a consumer branching on the
code opens the clause its document actually broke.

⚠ **Where SCE is narrower than W3C, and why.** Both grammars are
ASCII, and the event grammar refuses `:`. W3C is wider on both counts
— an XML Name admits most of Unicode's letters, and §3.12.1 shows
`<transition event="ccxml:connection.alerting"/>` under the words
*"This markup is legal"*. SCE narrows because these values do not stay
in the document: an id and an event name each become a code identifier
in C, C++, Kotlin, Rust, Go and Python. A value the sweep admits is one
all six can carry. Measured 2026-09-14 over the 735 SCXML documents
this repository commits — `git ls-files '*.scxml'`, the enumeration the
corpus test uses — the narrowing costs nothing here: no document uses a
Unicode or `:`-bearing identifier. But it is a boundary SCE draws and
not one W3C drew, which is why it is written here rather than left for
a rejected author to find. The names SCE's own elements carry, and a
forge document's `<data id>`, are held to a narrower grammar still —
§2.14.

⚠ The enumeration is named because the number alone rotted once. This
paragraph read "794 documents" until 2026-09-14, a count taken by
walking the filesystem past a hand-written skip list — which counted
generated build output as source, and which the corpus test dropped for
exactly that reason. A bare total invites the reader to re-take it with
whatever walk comes to hand, and the two walks do not agree — a `find`
over this tree answers a different number on every machine, because it
counts whatever the last build wrote.

The **AOT code generator** is the default path; the Interpreter exists
as a fallback for documents that cannot be statically generated. At
HEAD, `tests/CMakeLists.txt` lists every W3C IRP test in
`W3C_AOT_TESTS` and `W3C_INTERPRETER_ONLY_TESTS` is empty — i.e. the
full IRP suite generates statically on every supported backend. The
categories in §3 describe the *kinds* of constructs that would route a
document to the interpreter fallback if it contained them, independent
of whether the IRP suite happens to exercise those categories today.

The acceptance boundary for each specific rejection is linked from the
appendix. Codes are grouped by pipeline stage, matching the Stage
taxonomy in `SCE_ERROR_CONTRACT.md` §4.

### The `datamodel` attribute

W3C SCXML §3.2 gives `datamodel` the valid values `"null"`,
`"ecmascript"`, `"xpath"` "or other platform-defined values", and leaves
its default platform-specific. §B adds the obligation: a conformant
processor MUST support the null data model and MAY support the others.

SCE accepts three values and rejects the rest at parse:

| Value | Status |
|---|---|
| `null` | Accepted — §B.1 enforced (see below) |
| `ecmascript` | Accepted — evaluated by the injected script engine |
| `sce-static` | Accepted — SCE's platform-defined, statically typed data model (§2.15) |
| `xpath` | Rejected, `scxml/unsupported-datamodel` — a spec-defined data model SCE has not implemented |
| anything else | Rejected, `scxml/unsupported-datamodel` — §3.2 permits platform-defined values and SCE defines no other |
| absent | `ecmascript` |

The default is a choice, not an inference. §3.2 leaves it to the
platform, and SCE picks `ecmascript` because a document that omits the
attribute and then writes `cond="x > 1"` is asking for a value
expression language that the Null data model does not have. The choice
is made in one place (`sce_build::model::Datamodel`'s `Default`) so the
engines cannot answer it differently.

**Null data model.** §B.1 is not "a data model with nothing in it" — it
withholds four languages separately, and SCE reports each under its own
sub-section as `scxml/null-datamodel-forbids-construct`:

| Rule | Withheld |
|---|---|
| §B.1.1 | The underlying data model — `<datamodel>`, `<data>`, `<assign>`, `<foreach>` |
| §B.1.2 | Boolean expressions other than `In(id)` |
| §B.1.3 | Location expressions — `location=`, `idlocation=` |
| §B.1.4 | Value expressions — `expr=`, `srcexpr=`, `targetexpr=`, `delayexpr=`, `eventexpr=`, `typeexpr=` |
| §B.1.5 | Scripting |

Three deliberate narrowings, all extensions rather than readings:

- **Literal `<donedata>` / `<content>` / `<param>` are admitted.** §B.1.7
  withholds the §5 elements wholesale. SCE refuses only those that need
  the data model itself (the §B.1.1 row above) and admits the other three
  when they carry literals, because §B.1 withholds four *languages* and a
  literal payload names an expression in none of them. An `expr` on any
  of them is still refused — by the §B.1.3/§B.1.4 rows, under the
  sub-section that actually withholds the language.
- **Native `<script><cpp>…</cpp></script>` / `<kt>` is admitted.** It is
  SCE's native host action (§2.11), lowered straight into the generated
  language with no script engine involved, so §B.1.5 withholds nothing it
  uses. A `<script>` carrying data model script text — or mixing text
  with native blocks — is still refused.
- **Native `cond="cpp:…"` / `cond="kt:…"` is admitted**, on `<transition>`
  and on `<if>` / `<elseif>` alike. The same door as the `<script>` form
  above, for the same reason: the prefix is stripped and the body lowered
  into the generated language, so §B.1.2 withholds nothing it uses. Until
  2026-09-16 only the `<script>` half was admitted, and a consumer pairing
  `cpp:` guards with `datamodel="null"` was refused beside documents whose
  `<script><cpp>` passed — the asymmetry, not the rule, was the defect.
  ⚠ The same day, admitting it revealed a second asymmetry one layer down:
  the `<if>` templates had no native arm and folded such a guard to
  `if (false)`. See §3 for that half. The prefix must be the literal start
  of the condition, matching the sites that lower it; a leading space would
  be an admission the backend cannot honour. ADR 0003 records the decision
  and pairs it with `docs/SCE_SCRIPT_ENGINE_CENSUS.md`, which counts
  native-prefix use so an escape hatch cannot quietly become the path.

A nested `<scxml>` inside `<content>` declares its own data model and is
judged as the document it is, not by its parent's declaration. `sce:`
extension elements are governed by §2 below, not by Appendix B.

**What `ecmascript` currently means.** §B.2 obliges an implementation
that accepts this value to support the third edition of ECMAScript. The
answer depends on the backend, and this paragraph used to say otherwise
("SCE does not ship an ECMAScript engine: an expression is **parsed** at
generation time and emitted as Lua"). Measured 2026-08-27 —
`docs/SCE_LUA_TRANSLATION_SEAM.md` carries the per-backend table:

- **Rust, Go, Python, C11** receive Lua. The expression is **parsed** at
  generation time and lowered by `sce-build/src/ecmascript/`, and the
  injected engine evaluates that. It is a translation, not an ECMAScript
  implementation — but the boundary of what it translates is declared
  below, which is what this paragraph used to record as open.
- **C++** receives the author's ECMAScript source and hands it to the
  injected engine unchanged, because its generated code takes that engine
  by injection and cannot know at generation time which one arrives. It
  ships and defaults to a real ECMAScript engine — QuickJS
  (`SCE_SCRIPT_ENGINE=quickjs`) — so on its default configuration a
  document IS evaluated as ECMAScript, and the ECMA-262 case table
  (`tests/ecmascript/ecma262_semantics.json`) is answered in full.
- **Kotlin** was on that side until 2026-08-30 and is on the first one
  now: its templates render through the pair filter, so a run's
  `--script-engine` selects which language the artifact carries, and the
  DEFAULT is lowered Lua. A Kotlin host that configures nothing is
  therefore handed lowered Lua and the Lua engine that evaluates it —
  `W3CTestBase.DEFAULT_ENGINE` and the Spring starter's
  `scxmlScriptEngine` bean both name it, which is what makes the default
  artifact and the default host one configuration rather than two.
  Generating with `--script-engine ecmascript` puts it back on the C++
  side, and Rhino or QuickJS then evaluate the author's own text.

  Selecting Lua on C++, or ECMAScript-carrying input on the Kotlin Lua
  engine, reaches `sce-build`'s ECMAScript frontend, which PARSES the
  author's text and lowers it at run time — the same frontend the
  build-time lowering uses, linked into the engine. Its
  disagreements with ECMA-262, if any, are enumerated in
  `tests/ecmascript/lua_engine_divergences.json` and
  `tests/ecmascript/kotlin_lua_divergences.json`; both lists are EMPTY as
  of 2026-08-30. Each entry names the PATH it is about (`diverges_on`):
  the run-time route, or the build-time lowering a run reaches by asking
  for `--script-engine lua`. Two routes into one engine fail differently,
  and a list that did not separate them could only ever shrink when the
  route it named was repaired.

  ⚠ Both lists reached empty through the frontend rather than through a
  repair of what came before it — a per-backend `EcmaScriptToLuaTransformer`
  that rewrote the same text WITHOUT parsing it, and so could not say
  where an operand ended. C++ has deleted its copy; Kotlin's is still
  compiled in as the fallback for text the frontend refuses, so an
  expression outside the case table is still guessed at there rather than
  refused.

The manifest reports which of the two a given run produced, in
`script_engine_language` (`SCE_ERROR_CONTRACT.md` §10.1).

The frontend is `sce-build/src/ecmascript/`: a recursive-descent reader
of the ECMAScript expression grammar (and, for `<script>` bodies and
function literals, the statement grammar), sharing its lexer with the
Forge dialect so the two cannot disagree about what a literal or an
identifier is.

*Accepted*: the operators and literals of ECMA-262 expressions —
including `==`/`!=`, which the Forge dialect forbids — plus array and
object literals, `typeof`, `instanceof Array`, `new`, function
expressions, `++`/`--`, assignment and compound assignment; and as
statements, `var`, `if`, `while`, `for`, `for…in`, `return`, `break`,
`continue`, function declarations and blocks.

*Refused, by name*: a reserved word used as a value; `instanceof`
against anything but `Array`; a `Math` member outside the mapped set;
arrow functions, template literals, `??`, `?.` and spread (refused at
the lexer, as in the Forge dialect); `switch`, `do…while`, `try`/
`throw`, `with` and labelled statements; a parameter or local named
after a Lua keyword.

**A refusal is not a build failure.** §5.9.1 requires that a `cond`
which cannot be evaluated raise `error.execution` and read as false, and
§5.4 says the same of `<assign expr>`; W3C tests 309 and 344 write
`cond="return"` for exactly that. Refusing at generation time would make
those documents ungeneratable rather than conformant, so a refused
expression is emitted as Lua that raises when evaluated, carrying the
parser's message.

**A refusal is reported.** Not being a build failure is not a reason to
be silent, and it was: the verdict was reached at generation time and
survived only as a string literal inside the generated source, so
`check` answered `status: "ok"` and `generate` exited `0` listing
artifacts for a document that cannot run. Every refusal is now emitted
as an `expression/*` diagnostic on stderr — the stage
`SCE_ERROR_CONTRACT.md` §4 already defines for "ECMAScript unsupported
constructs" — anchored on the element that wrote the expression and
naming which of that element's expressions was refused. The run still
succeeds: records on stderr with exit `0` and a manifest on stdout is
how a reported refusal differs from a fatal one, which no consumer needs
a new field to read (§1, §10.2).

`--lint` promotes it to fatal. That flag already separates the two kinds
of author — the design-time statechart lints are off by default because
the W3C corpus declares unreachable states on purpose — and a refused
expression is the same shape of claim: conformance obliges SCE to
generate one, and nothing obliges an author to write one.
`sce-build/src/ecmascript_acceptance.rs` is the walker;
`sce-build/tests/ecmascript_acceptance_parity.rs` binds it to the
filters by comparing, over every document this repository tracks, the
refusals it reports against the raises the generated artifacts carry.

**A native `cond` is not an ECMAScript expression, and only one backend
lowers it.** `cpp:` and `kt:` name the language the guard is written in;
the C++ and Kotlin templates strip the prefix and emit the body, and no
other backend has that branch. Generating such a document for another
backend is refused with `generate/unsupported-feature` on the backend
axis. Before that refusal existed, Rust, Go and C11 emitted the guard
verbatim — `if cpp:hardware.hasPower()`, which no compiler accepts — and
Python lowered it through this frontend, producing a guard that raises
on every evaluation and a transition that can never be taken. All four
reported success.

**A native `cond` is admitted on `<if>` and `<elseif>`, not only on
`<transition>`.** The two are the same guard written in two places and
lower identically. ⚠ Until 2026-09-16 they did not: the admission landed
on the transition door alone, and an `<if cond="cpp:…">` fell past every
arm to the constant fold, which emits `if (false)`. The branch body
became dead code and an `<else>` beside it ran unconditionally — a wrong
output rather than a missing one, with no diagnostic and exit 0. The
decision now lives in one place (`parser::resolve_cond`) that all three
sites call, because a second site is what let the two answers differ.
`generate/unsupported-feature` also refuses a `cond` that reaches no
lowering arm at all, so the next kind of guard a template has no branch
for stops the build instead of folding to `false`.

**Operators Lua does not share** — `+` (concatenation or addition
depending on the operands), `==` (coercing), `%` (truncating), the
bitwise family (over ToInt32), and computed indexing (zero-based over a
one-based store) — are emitted as calls into
`sce/include/scripting/ecma_semantics.lua`, one definition that every
backend's engine loads. `sce-build/tests/ecmascript_semantics.rs` pins
each against the ECMA-262 clause it comes from, by evaluating the
emitted Lua in the production engine rather than in a test-local one.

**XML is a DOM structure, and DOM Level 1 Core is the vocabulary.**
§B.2.1 obliges the Processor to *"create the corresponding DOM
structure"* for a `<data>` element's XML content or `src`, and §B.2.8.1
says the same for an XML `_event.data`. Those two are the only places
the Recommendation asks for a DOM — an `<assign>`'s children are
§5.4.2's "in-line specification of a legal data value" and §B.2.7 names
only `expr`, so the string reading they get is not a gap.

What a handle answers:

| | Members |
|---|---|
| Node | `nodeType` (1 element, 3 text, 4 CDATA section, 9 document), `nodeName`, `nodeValue`, `parentNode`, `childNodes`, `firstChild`, `lastChild`, `nextSibling`, `previousSibling`, `hasChildNodes()`, `textContent` (DOM Level 3) |
| Element | `tagName`, `getAttribute()`, `hasAttribute()` (DOM Level 2), `getElementsByTagName()`, and SCE's own `getTagName()` |
| CharacterData | `data` |
| Document | `documentElement`, plus the Element vocabulary for its document element |
| NodeList | the host language's array: `length` and `[i]` |

The variable holds the *document*, which reports `nodeType` 9 and
answers the Element vocabulary for its document element — the delegation
`getAttribute()` and `getTagName()` performed before this surface
existed, widened rather than narrowed. `getTagName()` is SCE's own name
for the `tagName` property and stays because committed trees call it.

Three narrowings, all of them the reference backend's:

- **A whitespace-only text run is not a node**, and neither is a comment
  or a processing instruction — pugixml's `parse_default`, which the C++
  backend parses with, omits `parse_ws_pcdata`, `parse_comments` and
  `parse_pi`. Whitespace that is *not* the whole run is kept: `<p>a <b/>
  c</p>` has three children and a `textContent` of `"a  c"`.
- **The surface is read-only.** `setAttribute`, `appendChild`,
  `createElement`, the namespace-aware getters and the rest of DOM's
  mutation and factory vocabulary are refused by name
  (`expression/unsupported-builtin`, §3.5's rule applied to a second
  specification): a DOM built from `<data>` content or an arriving
  `_event.data` is the document that was parsed, and §B.2.4's location
  expressions are how this datamodel changes values.
- **`item(i)` is refused too**, because a NodeList is the host
  language's own array in every backend — `[i]` reads it and `length`
  measures it, and there is no receiver for a method to bind.

Seven bindings implement this — two C++ engines, three Kotlin engines,
Rust, Go, Python and C11 — and they are measured against one table,
`tests/ecmascript/dom_read_surface.json`, which carries every case twice:
the author's ECMAScript and the Lua the frontend lowers it to.
`sce-build/tests/dom_read_surface_table.rs` asserts the second IS the
frontend's own lowering of the first, so a reader cannot be asked a
spelling the emitter does not produce. Before that table, all seven
carried `getElementsByTagName`, `getAttribute` and `getTagName` and
nothing else, which is exactly the vocabulary the W3C IRP suite reads:
`d.tagName` answered nil on every backend with 204/204 fixtures green.

---

## §2 SCE extensions grammar

SCE adds a small set of extension elements and attributes for
functionality beyond plain SCXML. All SCE extensions live under the
namespace URI `https://sce.example/ns/1` (constant `SCE_NAMESPACE` in
`sce-build`). Unqualified or wrongly-namespaced attributes are rejected
as schema violations (`xml/schema-validation`).

That rejection is the AOT pipeline's, and it is worth saying which
engine makes it, because the two do not accept the same documents.
`sce-build` validates against `sce-forge.xsd` before codegen, so a
wrongly-namespaced attribute stops there. The C++ Interpreter has no
XSD stage — it parses with pugixml, which has no schema-validation
surface, and `sce/` links libxml2 nowhere — so the same document is
parsed rather than refused, and what stands in the schema's place is
the structural checking in `sce/src/parsing/` (for the root namespace,
`SCXMLParser::parseInternal` answering `ParseWrongRootElement`).

This is a decided asymmetry, not a gap: giving the Interpreter a stage
would mean a second XML library in one binary, replacing pugixml across
its parser, or hand-writing a validator for `schemas/sce-forge.xsd`.
The axis is whether schema validation is AVAILABLE rather than which
language a backend is written in — `validate_or_skip` guarantees only
that validation runs when a schema is, returning
`NotValidated(SchemaNotFound|FeatureDisabled)` otherwise, so what the
producer side really holds is that it says when it did not validate.
`SCE_WIRE_CONTRACTS.md` and the comment on
`ParsingCommon::isScxmlNamespace` carry the same distinction.

### §2.1 Forge kinds — `sce:kind`

The `sce:kind` attribute on the root `<scxml>` element selects the
forge kind the document compiles to. The closed value set is the
eighteen variants of `ForgeKind`, written in the `sce:kind` attribute
as the kebab-case tokens below (source of truth:
`sce-build/src/forge/model.rs` `ForgeKind::from_attr`; see
`forge_kinds_catalog.md` for the stateful/stateless/inline-eligible
matrix):

```
statechart   procedure   transform   lookup       condition
codec        validator   filter      interpolation
timer        observer    algorithm   link         worker
buffer-pool  bounded-collection      enum         event-schema
```

Omitting `sce:kind` defaults to `Statechart`. Values outside this set
are rejected as `validation/unsupported-kind`. The phase 2/3 runtime
packages for the stateful kinds (Validator, Filter, Timer, Observer)
are described in `forge_phase3_complete.md`.

**A kind may also be declared IN PLACE**, on a `<data>` element of an
outer statechart's `<datamodel>`. The element takes the role the
`<scxml>` root takes in a document of its own, so the content is a
standalone kind's content and the same parsers, validators and
renderers read it; the artifact is a sibling named
`<machine>_<data id>`. Only a kind that keeps no state between calls
can be declared this way (`is_inline_eligible()` → true): transform,
lookup, condition, codec.

These declarations emit companion artifacts for the host to call. They do
not bind the declaration id to a script-engine function or datamodel value;
automatic invocation from a guard or action is not supplied by this syntax.

An `sce:kind` on a `<data>` that this site cannot admit — a value no
kind goes by, or a kind requiring a standalone document — is rejected as
`validation/kind-not-inline-eligible`, whose `fix.candidates` are the
four above rather than all eighteen. ⚠ Until 2026-09-22 both were
accepted and the `<data>` became an ordinary datamodel variable
carrying the same id, so a guard naming it read the variable and the
document's `sce:kind` decided nothing.

**Document name — the file stem, not the `name` attribute, except for
`sce:kind="algorithm"`.** The compiled model's name, which every
backend derives its type and file names from, is the document's file
stem. A `name` attribute on the root `<scxml>` element of a forge
document is accepted and ignored: `enum_hex_values.scxml` declaring
`name="opcode"` compiles to `EnumHexValues`, and a document with no
`name` attribute at all compiles the same way. Only the root element's
attribute is inert — `name` on a child (`<sce:variant name>`,
`<sce:flag name>`, `<sce:link name>`) names the thing it sits on and is
used normally.

`sce:kind="algorithm"` is the one exception, because its artifact is a
function rather than a type: the root `name` names the emitted
function, and the file stem does not appear in the output at all.
`algorithm_bytes_equal.scxml` declaring `name="bytes_equal"` emits
`bytes_equal.rs` with `pub fn bytes_equal`, the C11 symbol
`bytes_equal`, and the C++ namespace `SCE::Generated::BytesEqual` —
which is what a cross-document caller resolves against, so the name is
load-bearing rather than decorative there. The rule as stated above
carried no exception until the corpus was measured against it:
`the_identity_rule_holds_for_every_kind_the_corpus_declares` now
generates every committed document whose root `name` disagrees with its
stem and checks which one the artifact takes, so a kind cannot leave
the rule silently.

The grammar admits the attribute (`name` is optional on the root) and
many in-tree documents carry one, so this is stated rather than
enforced: rejecting it would redefine the language against its own
corpus instead of fixing a defect. Examples in this document that
show a root `name` — the `sce:kind="enum"` opt-out sample under
"Opt-out for open-set vocabularies" among them — are naming the
document for the reader, not selecting the emitted type.

This is the one place a forge kind departs from
`sce:kind="statechart"`, where W3C SCXML 5.10 requires the root `name`
attribute to be bound to the `_name` system variable (W3C tests
323/324/329/346).

### §2.2 Typed fields — `<sce:field>`

Structured data carriers used by codec / validator / filter / etc.
kinds. Required attributes:

- `id` — unique within the enclosing kind (duplicates are rejected as
  `validation/duplicate-id`). ⚠ Unique across the document's WHOLE
  expression namespace, not only among `<data>`: fields, algorithm
  parameters, consts and locals, procedure helpers, observer monitors,
  codec flag inputs and every `<sce:import as="…">` alias share one scope,
  because every backend lowers them into one. An algorithm local that
  reuses an earlier name keeps its own code,
  `algorithm/local-shadows-param`. Measured 2026-09-21 before this was
  one check: two inputs named alike generated on all six backends, an
  input and an output named alike were refused as a transform cycle,
  a parameter shadowed an import alias silently, and a duplicate
  parameter was refused by the renderer as one backend's gap.
- `sce:type` — closed value set of fixed-width scalar tokens
  (source of truth: `SceType::from_attr` in
  `sce-build/src/forge/model.rs`): `uint8`, `uint16`, `uint32`,
  `uint64`, `int8`, `int16`, `int32`, `int64`, `float32`, `float64`,
  `bool`, `string`, `bytes`. An enum-typed field uses the
  `enum:<alias>` form referencing an imported `sce:kind="enum"`
  document (§2 EventSchema / NL→IR Item C1). Values outside this set
  are rejected as `validation/invalid-attribute`.

Field cardinality and direction constraints are enforced per-kind —
e.g. `Transform` requires at least one input and one output field
(`validation/empty-collection`, `validation/invalid-direction`).

**Naming an enum's variant in an expression.** An `expr=` (or any
other forge expression) refers to a variant of an imported enum as
`<alias>.<variant>`, with the alias the `<sce:import as="…">` gave the
enum document and the variant spelled exactly as that document declares
it:

```xml
<sce:import as="Tone" src="tone.scxml" kind="enum"/>
<data id="alarm" sce:type="bool" sce:direction="in"/>
<data id="tone" sce:type="enum:Tone" sce:direction="out"
      expr="alarm ? Tone.LOUD : Tone.QUIET"/>
```

Each backend receives its own spelling of the reference (`Tone::Loud`,
`TONE_LOUD`, …) from the one place that also spells the declaration, so
the two cannot drift. This holds in every forge kind whose expressions
read fields — transform, condition, validator, observer, procedure,
codec, lookup, filter and algorithm — not only the transform.

⚠ **A member the enum does not declare is refused**, as
`expression/unknown-enum-variant`, with the declared set as the fix.
The set is offered whole, not narrowed to a guess: `Tone.RED` against
an enum declaring `QUIET SOFT LOUD` is not a spelling of any of them,
and which one the author meant is a decision for whoever wrote the
specification. Measured 2026-09-21: before this refusal such a
reference generated with exit 0 on all six backends and named nothing
in any of them.

⚠⚠ **So is an operand nothing declares.** A forge expression may read
only its own kind's fields, its imports and its `<sce:helper>`
declarations — there is no host behind it — so `conut + 1` beside
`<data id="count">` is refused as `expression/unknown-identifier`
with `count` offered, the same code and the same suggestion rule the
ECMAScript datamodel uses. Measured the same day: it too generated
with exit 0, and Python met the undeclared name only when the line
first ran.

⚠⚠ **So is a member of a value.** Only a record has members a forge
expression may read — a stateful import's alias (`frame.msg_id`), a
procedure's `_event`, and an algorithm's `<sce:foreach>` item over a
bounded collection (`entry.pattern`). Every other declaration — a
field, a parameter, a const, a local, a byte item, an enum-typed value
— is a value, so `label.length` beside `<data id="label"
sce:type="string">` is refused as `expression/member-of-non-record`,
with no fix: `len(label)` is how an expression asks for a length, and
which read the author meant does not follow from the member written.
The same holds one level down: `frame.msg_id.foo` asks a record's
scalar field for a member. Measured 2026-09-21: `x.foo` on a `uint8`
input generated with exit 0 on all six backends and named nothing in
any of them.

⚠⚠ **And so is a member a record does not declare.** A stateful
import's members are its fields and methods — a codec imported as
`frame` has `frame.msgId` and `frame.encode()` — and a bounded
collection's item has its element's fields, so `frame.msgIdd` is
refused as `expression/unknown-member` with every declared member
offered, as an enum's variants are. Two records are not judged, and
say so rather than pass by accident: `_event`, whose members are the
triggering event's and belong to its schema, and a collection item
compiled without its element schema (a single document on its own),
whose element type lives in a document that compile never reads.
Measured 2026-09-21: the only member check ran on algorithms, where an
import's alias is not a value at all, so a misspelled field of an
import in a procedure generated with exit 0.

**An `sce:` attribute this tree does not read is refused**, as
`validation/unknown-sce-attribute`, with the nearest known names as
the fix. The parser looks attributes up by name, so one it does not
look for is not unused — it is invisible, and the author's sentence
and the machine's behaviour part company in silence. Measured
2026-09-18: `sce:totallyMadeUpAttribute` generated with exit 0, and a
conformance fixture had carried `sce:pre-transform="…"` — announced in
its own comment as pre-processing the filter's input — while nothing
read it and the filter smoothed the raw input. A misspelling is the
same failure with a likelier cause: `sce:directon="out"` left the
field at its default and said nothing.

⚠ It is a NAME check, not a placement check: a known name on an
element that does not accept it still passes. ⚠⚠ It covers the forge
kinds only. Statecharts carry a different `sce:` vocabulary
(`sce:req` on states and transitions, the datamodel families) which
has not been measured the same way, and running one list over both
would refuse valid documents.

**`sce:default-covers` — answering the value-space coverage report.**
On an enum-typed input `<data>` of a `sce:kind="transform"`, it names
the variants of that input's value space which reach the expression's
default branch *on purpose*:

```xml
<data id="terrain" sce:type="enum:Terrain" sce:direction="in"
      sce:default-covers="COASTAL ALPINE DESERT"/>
```

`sce-codegen coverage` reports which variants nothing in a document
tests for (§B). That report is deliberately exit-0 — falling through to
a default is legal and often right — but a report with no way to answer
it re-asks the same question forever, and a list that never shrinks is a
list nobody reads.

⚠ **It names the variants rather than being a flag, so the claim
expires.** Add a variant to the enum document and the new one is
unacknowledged, so the question comes back — which is exactly right, because
a value space growing is when the author has something new to decide. A
boolean `sce:default-is-deliberate` would have gone silent forever on the
day the platform added a value.

⚠⚠ **A claim can be false, and a false claim is refused rather than
reported.** The gap is legal; an untrue statement about the document is
not — the same failure `unknown-sce-attribute` exists to prevent. Three
ways, three codes, because the repairs have three shapes:

- *names something the value space does not declare* — usually drift,
  after a variant was renamed upstream. Refused as
  `validation/default-covers-unknown-variant`; the fix offers the
  declared variants.
- *names a variant the document's own conditions test for* — the name is
  real and the list is stale. Refused as
  `validation/default-covers-tested-variant`; the fix drops that name.
- *sits on a field with no value space* — a non-enum type, or an output.
  Refused as `validation/default-covers-not-a-value-space`; the fix drops
  the attribute.

⚠ The codes are written inline here rather than as a table, because the
appendix below is keyed on a `| `code` |` row and each code must sit in
exactly one of those (`acceptance_doc_covers_every_code`).

**`sce:retain` / `sce:initial` — a value that outlives the program.**
On a forge `<data>` field, the pair declares that the field's value is
kept between runs and names the value it has before anything has ever
been stored:

```xml
<data id="prevMode" sce:type="enum:Mode" sce:direction="in"
      sce:retain="battery" sce:initial="NORMAL"/>
```

⚠ **SCE does not implement persistence** — where a value is kept between
runs belongs to the host. SCE declares it, carries it into the forge AST
so the host knows what to provision, and checks the one question it can
answer: whether `sce:initial` names a value the field's type could ever
hold (an enum variant, `true`/`false`, an integer inside the declared
width). That check earns its place because the initial value is taken on
the day a system is first switched on and on no other day — the value
least likely to be reached by a test, and the one whose mistake survives
longest.

⚠⚠ **The scope label is opaque.** SCE compares it for equality and never
interprets it, so `"battery"`, `"ignition"` and `"sd-card"` are all
simply labels. The measurement behind this surface is automotive, and
enumerating its two stores here would put one domain's vocabulary in a
general tool — the same reason `sce:req` ids are never normalised. What
a label means, and whether that store exists, is the deployment's
business.

⚠⚠⚠ **A retained field needs an initial value, and an initial value
needs a reader.** Both orphans are refused as
`validation/attribute-rule-violated` with the missing partner named. A
retained field with no initial value is undefined on its first run. An
initial value is read by a store (`sce:retain`) or, on a transform, by
`previous(<field>)` on the first activation (§3.4.1); a field with
neither is computed afresh every cycle, so nothing reads it. That second
rule is decided in validation rather than while parsing, because whether
a field is read through `previous()` is known only once the expressions
are, and the refusal carries the row `sce:initial` is written on. No new
diagnostic code: this is the orphan shape `sce:quantity` / `sce:scale` /
`sce:offset` already established.

**`<sce:cycle>` — an order the document states.** An ordered sequence of
named alternatives drawn from an imported value space, each optionally
carrying the condition under which it is present:

```xml
<sce:cycle id="panels" of="Panel">
  <sce:step name="LIST"/>
  <sce:step name="GRID"   when="gridAvailable"/>
  <sce:step name="DETAIL" when="itemSelected"/>
  <sce:step name="MAP"    when="locationKnown"/>
</sce:cycle>
```

⚠ **The order belongs to the document, not to the value space**, and
that is the whole reason the element exists. The obvious design walks
the imported enum's declaration order and skips what is unavailable — no
new element, one intrinsic. It is wrong, and measurably so. In surveyed
material a prose specification and the platform value space it draws on
listed the **same seven alternatives in two different orders**: they
agreed on the first three, and one variant the specification put
**fourth** the value space declared **last**. So "the next alternative"
would have been wrong in a way a fixture that steps one place cannot
see — the two orders share a prefix, which is exactly what a small
example fails to distinguish. A value space says *which* values exist; a
cycle says *in what order* a user walks them.

⚠ The shape of that disagreement is the measurement and it is stated
here; the variant names are the surveyed document's identity and are
not. The example above is therefore an invented one, chosen to show a
conditional step rather than to reproduce anything observed.

⚠⚠ **The steps are a subset.** A value space usually holds values that
are not stops at all — an `OFF`, an `INVALID`, a `MAX` sentinel — and
requiring every variant would force conditions for values that are not
alternatives. The names are checked against the value space, so a
misspelled stop is refused rather than becoming a position no cursor can
reach.

⚠⚠⚠ **`when` sits on the step** because that is how the source notation
writes it: one table whose rows are the alternatives in order, each row
carrying the condition under which it is present. An earlier design
passed availability in as a bitmask, which made the author write bit
positions by hand — positions meaningful only relative to a list
declared elsewhere.

Refused, as `validation/invalid-attribute` with the legal names as
candidates or as the existing cardinality codes: a step naming no
value, an `of` that is not an imported enum alias (a document that
imports no enum has no candidate, and breaks the rule instead:
`validation/attribute-rule-violated`), fewer than two stops (nothing to navigate), and a
repeated name (one value in two positions makes `next` ambiguous). One
value space may carry several cycles, and one alternative may be a stop
on more than one of them.

**Navigating a cycle** — four calls, usable in any `transform`
expression:

| Call | Answers |
|---|---|
| `cycle_has(c, cur)` | is `cur` a stop that is currently present |
| `cycle_first(c, cur)` | the first present stop, or `cur` when none is |
| `cycle_next(c, cur)` | the next present stop after `cur`, wrapping |
| `cycle_prev(c, cur)` | the previous present stop, wrapping |

The boundary rules are decided once, in
`sce-build/src/forge/cycle_expand.rs`, and hold for every target:

- **Nothing present.** `cycle_first` answers the `cur` it was given.
  That is why it takes one: naming the first *declared* stop instead
  would be the primitive claiming a presence the document's own
  condition denies.
- **The cursor is not a stop.** `cycle_next` / `cycle_prev` answer it
  unchanged. Walking the cycle is the only thing they do, and "not on
  the cycle" is not a walk. Snapping to the first stop is a defensible
  rule — the surveyed specification states exactly that one — which is
  precisely why it belongs in the document, written with `cycle_has`;
  folded into the primitive it would vanish from the document and the
  specification's sentence would have nothing to read against. Standing
  still is also the louder failure: a document that forgets the rule
  gets a cursor that sticks rather than one that silently jumps.
- **A cursor on an absent stop still navigates from its position.**
  `cycle_has` reports the absence; what to do about it is the
  document's call.

⚠ **These are expanded, not lowered.** The four calls are rewritten into
ordinary conditional expressions once, before any backend sees the
document, because nothing about walking a declared list differs per
language — unlike `round`, whose halfway rule genuinely does. One
rewrite instead of six emitter arms, and a seventh backend inherits it.

⚠⚠ **On Go they are exactly as portable as any conditional.** Go has no
conditional expression, so its emitter lowers `c ? a : b` to a typed,
immediately invoked function literal, which evaluates only the chosen
branch as the other backends do. It refuses a conditional only when it
cannot name the literal's result type — neither the slot the value flows
into nor the branches are typed (`expression/go-ternary-unsupported`).
The cycle surface is exactly as portable as the rest of the expression
language, no more and no less.

⚠⚠⚠ **`cycle_next` expands to O(n²) terms** — the expression language
has no way to bind the cursor's position once and reuse it. At the arity
this was built for (seven stops, 49 terms) that is fine; the escape
hatch, if it ever is not, is a generated helper function per cycle,
which is O(n) with a local.

### §2.3 Context objects — `<sce:context>`

Per-kind context objects carrying stateful scratch data. Rules:

- The element is matched by NAMESPACE, not by prefix: it must be
  `context` in `http://sce.dev/ext`, which a document declares as
  `<scxml … xmlns:sce="http://sce.dev/ext">`. Stated here because the
  failure is silent in the worst way — a declaration under any other URI
  is not an `sce:context` element to the parser, so a `cpp:` guard
  naming objects reports *"references objects but no `<sce:context>`
  declarations found"*, the identical diagnostic to declaring none at
  all. The message cannot separate absent from present-under-the-wrong-key,
  and a reader who trusts it goes looking for a missing element that is
  sitting in front of them.
- The `id` is unique across all context objects in the document
  (`validation/duplicate-context-object`).
- The `id` does not collide with a type alias the C++ codegen emits
  on the generated state-machine class. At HEAD the reserved set is
  `{ policy }` — comparison is case-insensitive because Jinja2's
  `capitalize` filter maps `policy`, `Policy`, and `POLICY` to the
  same `PolicyType` alias (`validation/reserved-context-id`). The
  set is not maintained by hand: `RESERVED_CONTEXT_IDS` in
  `sce-build/src/parser.rs` is a `LazyLock` that scans
  `tools/codegen/templates/state_machine.jinja2` for literal
  `using {Id}Type =` aliases at first access. Adding a new class-
  scope alias to the template therefore extends the reserved set
  automatically — no parallel const to update, and no drift window
  between template and parser.
- Kinds that require a context object (e.g. Validator for history
  tracking) reject documents that omit it
  (`validation/missing-context`).

### §2.4 Cross-file composition — `<sce:import>`

Imports a standalone SCXML document that declares a non-Statechart
kind, for use in an outer statechart via `<invoke>` or inline via
`<data>`. Required attribute:

- `src` — filesystem path to the imported SCXML file (resolved
  relative to the importing file). The path must resolve
  (`import/file-not-found`) and parse (`import/not-forge`) to a
  document with a recognised `sce:kind`.

Optional attribute:

- `kind` — if present, the importer asserts the imported document's
  `sce:kind` matches. Mismatches are rejected as
  `import/kind-mismatch`.

Circular imports across the manifest graph are rejected as
`manifest/circular-dependency`.

The documents built together — every input of one `orchestrate` run, or
of one `check` over a document set — share one namespace. Two of them
declaring the same name are rejected as
`manifest/duplicate-document-name`, whatever their kinds: a reference
by name would have two answers, and each document's generated symbols
derive from its name. The record is located at the later input and
names the earlier one as a `related` site. Two inputs whose generated
artifacts land on the same path — which distinct names do not rule
out, since each kind derives its file names by its own rule (a
statechart `machine` and a forge document `machine_sm` both write
`machine_sm.rs`) — are rejected as `manifest/artifact-path-collision`
rather than one overwriting the other.

A statechart's document name is its file stem, not its `name`
attribute, so two statecharts sharing a file name in different
directories are one name declared twice.

An import the document never names — no `enum:<alias>` type, no
expression reading `<alias>`, no structural reference such as a codec
body — is still resolved and checked by every rule above, but the
generated code does not depend on it: no backend emits an include or
import for it. Go refuses an unused import outright, and the backends
must agree on what a document depends on, so the question is answered
once for all of them (`sce-build/src/forge/import_use.rs`).

**Name references between documents must resolve.** `<sce:import>` is
the path-based route; a `sce:kind="link"` document also names sibling
documents by *name*, and those names are joined against the build:

- `<sce:framer ref>` names a `sce:kind="codec"` document
  (`link/framer-ref-not-declared` when it names none).
- `<sce:rx-pool ref>`, `<sce:tx-pool ref>` and `<sce:stage-pool ref>`
  name a `sce:kind="buffer-pool"` document
  (`link/pool-ref-not-declared`).

A ref resolves either way it can be spelled: the named document may be
one of the build's inputs, or an `<sce:import>` alias of the matching
kind on the link document itself. Both diagnostics carry the reachable
names of that kind as `Fix::ReplaceOneOf` candidates.

These joins fire from the multi-document entry points — `orchestrate`
and `check` over a document set — and not from a single-document
`generate`, which is handed one file and cannot tell a name declared
elsewhere in the build from one declared nowhere. The distinction is
load-bearing rather than lenient: downstream checks that follow these
refs (`link/pool-slot-smaller-than-framer-max`, the deploy-time
burst-absorption and reassembly validators) skip the link when a ref
does not resolve, which is right for a partial topology and would
otherwise let a misspelt ref switch them off silently.

### §2.5 Communication patterns — `sce:pattern`

For distributed deployments under `--deploy`, the `sce:pattern`
attribute on `<send>` declares the communication pattern the send
follows. The enum carries seven variants defined in
`sce-build/src/mesh/pattern.rs` (source of truth):

```
FireForget   Request   Reply   Notify   Subscribe   Publish   Field
```

Per-transport capability tables constrain which patterns each
transport (`local`, `shm`, `someip`, `zenoh`) can realise
(`mesh/topology-pattern-capability-violation`). **Realization status:**
at HEAD only `FireForget` is fully realized end-to-end across all
transports; the remaining six patterns have partial codegen and are
tracked in `mesh_pattern_realization_gap.md`. Documents using
non-`FireForget` patterns are accepted by the validator (they match the
enum) but may generate code that panics or under-realises the
pattern semantics. Treat non-`FireForget` patterns as **experimental**
until the pattern-realization session lands.

### §2.6 Mesh-RPC invoke — `<invoke type="sce:mesh-rpc">`

Explicit extension for RPC-style cross-machine invokes under
`--deploy`. Documented in `SCE_MESH.md` §9.5. At HEAD this extension is
recognised by the parser but end-to-end realization is in progress —
see `next_session_task6_mesh_rpc_invoke.md` for the current state.
Acceptance is conditional on the deploy topology resolving both ends
of the RPC pair (`mesh/topology-receiver-not-declared`,
`mesh/topology-unresolved-targets`).

### §2.7 XInclude composition — `<xi:include>`

Multi-file SCXML composition via W3C XInclude (namespace
`http://www.w3.org/2001/XInclude`) is processed at parse time so
the AOT code generator consumes the same effective document as
the C++ runtime (`PugiXMLDocument::processXInclude`). Accepted
shape:

```xml
<scxml xmlns:xi="http://www.w3.org/2001/XInclude">
  <xi:include href="guards.xml"/>
</scxml>
```

Semantics match the minimal subset the runtime implements: the
children of the included document's root element are spliced
in place of the `<xi:include>` node — the root element itself
is discarded, so authors bundle N top-level fragments under any
XML wrapper (e.g. `<fragment>…</fragment>`) without affecting
SCXML validity, and a single `<xi:include>` composes them all.
`href` resolves absolute-first, then relative to the including
file, then against any operator-configured include directories
(the repeatable `--include-dir` / `-I` flag, in declaration
order), then relative to the current working directory; recursion
is bounded by a documented depth limit (mirrored from the
runtime), and cycles are detected. The include-directory search
path lets a fragment be referenced by bare name independent of
the including file's directory depth; the C++ runtime mirrors the
same precedence via `PugiXMLDocument::setIncludeDirs`.

Unsupported W3C XInclude features are rejected explicitly rather
than silently ignored — accepting them at build time would
produce state machines diverging from runtime parse:

- `parse="text"` — `xml/xinclude-unsupported`.
- `xpointer=` — `xml/xinclude-unsupported`.
- `<xi:fallback>` — `xml/xinclude-unsupported`.
- Inclusion of the root element itself — a fragment whose root holds
  no element and no text beyond XML whitespace, so the children rule
  would splice nothing where W3C XInclude would include the root —
  `xml/xinclude-unsupported`, naming the root. Wrap the element in a
  container: `<fragment><data id="x"/></fragment>`.

Rejections the AOT pipeline hard-errors on (the C++ runtime
warns-and-skips the same inputs; matching behaviour at
build-time is preferable to silent divergence): missing or empty
`href` (`xml/xinclude-missing-href`, fixable), unresolvable
`href` (`xml/xinclude-not-found`), filesystem read failures
(`xml/xinclude-read-error`), cycles (`xml/xinclude-cycle`),
depth overflow (`xml/xinclude-too-deep`), and malformed
included files (`xml/xinclude-malformed`).

### §2.8 Deploy manifest (`deploy.yaml`)

Accepted when `--deploy <path>` is passed. Schema is enforced by
serde with `deny_unknown_fields` (`mesh/deploy-parse` on unknown
keys). Only `version: 1` is accepted
(`mesh/deploy-unsupported-version`); machine names are globally unique
across all devices (`mesh/deploy-duplicate-machine`). The
`transports:` block is device-level; `bindings:` is per-target
(see `mesh_phase3_patterns.md`). External event/group/field catalogues
referenced from `bindings:` follow the rules in §2.5 and must resolve
in full (`mesh/external-unresolved-names`).

#### §2.8.1 `variant_defaults:` (RFC variant-default-overlay Atomic A)

Optional top-level map carrying per-codec default-arm overrides for
`<sce:variant>` peek-byte dispatch. Wire-spec invariants (bit
positions and `<sce:flag value=...>` MID constants) stay in the
SCXML — they are shared by every consumer. The *choice* of which
arm a freshly-constructed `Default::default()` dispatches to is
per-consumer convention and lives here instead:

```yaml
variant_defaults:
  codec_zenoh_request: 0x03    # client convention: query is the default
  codec_zenoh_response: 0x04   # reply is the default response body
```

Resolution order at codegen time:
1. If `variant_defaults` names the codec, the overlay value selects
   the default arm. `<sce:arm value="V"/>` matching `V == overlay
   value` becomes the Default-trait body; all peer arms have any
   SCXML-side `default="true"` marker cleared.
2. Otherwise the SCXML's own `<sce:arm default="true"/>` marker
   selects the default arm (legacy Atomic α-γ path, unchanged).
3. Otherwise `codec/variant-no-default-arm` fires at the cross-doc
   gate (§5.B Atomic γ-3 contract).

Overlay entries naming a value that no `<sce:arm value=...>`
declares fire `codec/variant-default-overlay-arm-not-declared`;
the `Fix::ReplaceOneOf` candidate set is the codec's declared
arm values (sorted, hex-formatted).

Backward-compat: deploy paths that omit `variant_defaults` (or
omit a specific codec entry) preserve the SCXML's existing
`default="true"` markers byte-identically. The 107 existing
`compile_forge_with_imports` call sites (no deploy) are unaffected.

#### §2.8.2 `<sce:variant-dispatch>` import-site dispatch (RFC §5.B B5-ν inversion)

B5-ν inversion places dispatch ownership at the composition root:
the parent codec declares — at its `<sce:import>` site — which of
its own flags drives an imported variant codec's arm selection.
The leaf codec describes only its body (variant arms + their wire
shapes); it carries no `tag=` attribute and no
`<sce:requires-parent-flags>` block for B5-ν purposes.

```xml
<!-- Leaf: pure body, no parent reference -->
<scxml sce:kind="codec" name="codec_zenoh_keyexpr">
  <datamodel>
    <sce:variant>
      <sce:arm value="0x00" type="codec_keyexpr_nonlocal" default="true"/>
      <sce:arm value="0x01" type="codec_keyexpr_local"/>
    </sce:variant>
  </datamodel>
</scxml>

<!-- Parent declares dispatch at the import site -->
<scxml sce:kind="codec" name="codec_zenoh_push">
  <sce:import src="codec_zenoh_keyexpr.scxml" kind="codec" as="key">
    <sce:variant-dispatch flag="header.M"/>
  </sce:import>
  <datamodel>
    <sce:flags id="header" sce:type="uint8">
      <sce:flag name="mid" bit="0" width="5" value="0x1d"/>
      <sce:flag name="M" bit="6"/>
    </sce:flags>
    <sce:embed id="key" type="key" sce:byte="1"/>
  </datamodel>
</scxml>
```

The leaf's decode signature gains a `tag: u8` parameter; the leaf
matches `tag` directly to pick the arm. Encode is unchanged — the
active arm is the language-level enum discriminant. The parent's
decode extracts the dispatch tag from its own flag carrier
(`(carrier >> bit) & mask`) and passes it to the leaf. The parent's
encode pre-computes the carrier's bit value from the embedded
variant's active arm and ORs it into the carrier before emitting
the carrier bytes.

Parents importing a variant codec **without** `<sce:variant-dispatch>`
fall back to the leaf's `<sce:arm default="true"/>` arm as the
construction-time tag input (Q-D-3 (a)). This is the case when arm
bodies happen to be wire-distinguishable by other means, or when the
author selects the arm at construction.

Cross-doc constraints (parent-local validator):

- `<sce:variant-dispatch flag="X.Y"/>` must resolve against the
  parent's own fields — both the carrier `X` and the flag `Y` must
  exist on the parent →
  `codec/variant-dispatch-flag-not-resolved`
  (`Fix::ReplaceOneOf` candidates = available carriers / flags).
- The named flag's `width` must fit the imported variant's arm count →
  `codec/variant-dispatch-bit-width-mismatch`.
- A parent without `<sce:variant-dispatch>` importing a variant codec
  without a `default="true"` arm cannot resolve the dispatch tag →
  `codec/variant-dispatch-arms-not-distinguishable-without-default`.
- The named flag must not carry a static `<sce:flag value="V"/>`
  constant (the bit is derived, not constant) →
  `codec/variant-dispatch-flag-has-static-value`.
- The flag carrier field must precede the embed field in the parent's
  `<datamodel>` declaration order →
  `codec/variant-dispatch-carrier-after-embed`.

Multi-bit dispatch: B5-ν inversion preserves B5-β's bit-range width
semantics. A `flag="X.Y"` form on a 3-bit flag dispatches over 8
arm values.

### §2.9 Composition extensions — `<sce:template>`

`<sce:template>` / `<sce:use>` / `<sce:param>` add parameterised XML
composition adjacent to XInclude (§2.7). XInclude handles
byte-identical reuse; `sce:template` handles fragments that differ by
a small closed set of constants. Both paths expand templates: the
AOT pipeline in sce-build (`crate::template::expand`) per RFC §6.5
Phase A, and the C++ Interpreter runtime in
`SCE::PugiXMLDocument::processSceTemplate` per RFC §3 Phase B M5 —
documents produced by each path are byte-equivalent after
canonicalisation, pinned by the CTest harness under
`tests/w3c_phase_b_parity/`. Each failure mode raises a typed
`SCE::parsing::Template<Variant>` subtype agreeing 1:1 with the Rust
`xml/template-*` DiagnosticCode set (pinned by
`cpp_template_subtypes_match_rust_diagnostic_codes`).

Expansion semantics (RFC §3):

- A template declaration is a standalone XML file whose root is
  `<sce:template name="...">`. Children `<sce:param name="..."
  required="true"|default="...">` declare parameters; remaining
  children form the template body.
- `<sce:use template="relative/path.xml" ...>` at the call site
  resolves the template file with XInclude precedence
  (absolute-first, then base-directory, then operator-configured
  include directories via `--include-dir` / `-I`, then cwd), binds
  every non-reserved attribute as a parameter value, and splices
  the rendered body in place of the `<sce:use>` node. Attributes
  named `template` are reserved. With an include directory on the
  search path a case file can reference a shared template by bare
  name (`template="guard.sce-template.xml"`) instead of a
  depth-coupled relative path.
- `{$name}` tokens inside the template body (attribute values and
  text nodes) are replaced by the parameter's bound string in a
  single lexical pass. Substitution does not cascade — a bound
  value that itself contains `{$other}` is emitted verbatim.
- Nesting is bounded by `MAX_TEMPLATE_DEPTH = 10` (mirrors
  XInclude). Cycles are detected via the same path-stack mechanism.

Rejections the AOT pipeline hard-errors on: unresolvable template
path (`xml/template-not-found`), filesystem read failures
(`xml/template-read-error`, Diagnostic-only), malformed template
file or malformed `<sce:param>` declaration
(`xml/template-malformed`), `<sce:use>` missing the required
`template` attribute (`xml/template-missing-attribute`, fixable),
omitted `required="true"` parameter
(`xml/template-missing-param`, fixable), unknown attribute on
`<sce:use>` (`xml/template-unknown-param`), cycles
(`xml/template-cycle`), and depth overflow
(`xml/template-too-deep`).

Those eight name ways expansion can fail. Expansion *not having been
attempted* is named separately by `xml/preprocessor-not-run`, raised
when a `<sce:use>` or `<xi:include>` survives into parsing. Both
pipelines hold the precondition: the file-based entries
(`SCXMLParser::parse_file`, `compile_forge_file`) run the expander
themselves, but the in-memory entries take already-read content, so a
caller that drives the pipeline itself can hand them unexpanded bytes.
Both parsers then select children by tag name with no else-branch and
skip the directive in silence. In a `lookup` with `sce:default` that
turns a dropped row into a plausible answer rather than a visible
failure; in a statechart it drops whole states from a model that
reports no error.

The check cannot live in the XSD. `<sce:use>` is a declared element
whose containers are `xs:any processContents="lax"`, so the schema
calls an unexpanded document valid by construction — and it must keep
doing so, since template authoring and editor integrations both work
on documents that have not been expanded yet. Only the document tree
can tell "not yet expanded" from "not expandable".

Post-expansion diagnostic attribution (RFC §6.3 Q3 depth-1 rule, as
implemented by `crate::position_map::Origin::CallSite` and
`Origin::File` emitted during `template::expand`):

- When a diagnostic fires in bytes produced by `{$param}`
  substitution, `location.{file, row, col}` points at the offending
  `<sce:use>` element in the **caller document** — the call site
  that supplied the parameter bindings (attributes on `<sce:use>`,
  per the XSD in `schemas/sce-forge-ext.xsd`). Column precision
  inside the substituted value is deliberately collapsed (every
  byte of every substituted region shares the same single (row,
  col) — the `<sce:use>`'s element position).
- When a diagnostic fires in template-body bytes (regions copied
  1:1 from the template file during expansion, i.e. not produced
  by `{$param}` substitution), `location.{file, row, col}` points
  at the template file's own (row, col). This lets template authors
  navigate to the body they wrote rather than to a caller that did
  nothing wrong.

Other XML meta-processing primitives (parameter entities,
conditional inclusion, computed attributes, Turing-complete
templating) remain out of scope — see `ARCHITECTURE.md` → "Scope &
Composition" for the discipline gate.

---

## §3 Exclusions (cannot be statically generated)

Documents using any of the following constructs fall outside the
statically-generated subset. At HEAD these would be routed to the
Interpreter fallback by `sce-codegen generate`; a document that
otherwise targets the statically-generated AOT path
(`sce-codegen` with `-l <lang>`) is rejected rather than silently
downgraded. The current W3C IRP pass matrix contains no such
documents, so `W3C_INTERPRETER_ONLY_TESTS` in `tests/CMakeLists.txt`
is empty at HEAD — but the rejection categories remain load-bearing
for arbitrary author-supplied input.

The categories and their primary rejection signals:

### §3.1 Dynamic file I/O at the invoke boundary

Nothing here is excluded any more. This section used to say that
`<invoke srcexpr="pathVar"/>` was rejected when generating AOT code
or compiling under `--deploy`, signalled as
`validation/dynamic-features`, "because the set of reachable invoke
targets must be known at build time to drive codegen". Every backend
generates the construct instead, and the reachable set being known at
build time is *how* — §2.13 describes the path each one takes. The
conformance registry carries fixture 216 (`srcexpr runtime
evaluation`) on the static path, and compiling the same document
under `--deploy` writes the same child stub rather than refusing it.

What the original sentence was reaching for survives in §2.13 as a
residue rather than a refusal, and it is sharper than the sentence
was: on five of the six backends the build-time set is a set of
**one**, so the value the expression computes cannot select between
targets — it is evaluated, and then the pre-generated child runs
whatever it said.

### §3.2 Documents without an initial state

Nothing here is excluded any more. This section used to say that a
document relying on W3C SCXML's default-initial-state semantics —
neither an `initial=` attribute nor an `<initial>` child — was
rejected at generation time as `validation/missing-element` or
`validation/require-either`. The parser resolves the default before
any gate sees the model: `SCXMLParser` fills the root's `initial`
with the first child state in document order (§scxml-3.2, §scxml-3.3),
and does the same for every compound state. A parsed document
therefore never arrives at the branch that would refuse it, and the
conformance registry carries fixture 350 (`Default initial state
(first child in document order)`) on the static path.

`can_generate_static` still holds that branch, still raising
`validation/dynamic-features` — not either code this section named —
for a caller that builds an `SCXMLModel` without going through the
parser. Its unit test says as much in its own comment. Reachable only
that way, it is not something a document can be refused for, and an
author choosing between engines on the strength of this paragraph was
being sent to the Interpreter for a construct the static path
handles.

### §3.3 Runtime event metadata references

Nothing here is excluded any more. This section used to say that a
guard reading `_event.origintype` was rejected as
`expression/unsupported-construct` because AOT output had no way to
populate the slot. The generated code populates it, and five W3C
fixtures the conformance registry carries — 198, 230, 253, 336 and
352 — read the field and pass on every backend, so the exclusion had
outlived its cause and the document was stating a refusal the producer
does not make.

Every field §scxml-5.10.1 names reads on every backend.
`ecmascript_member_access::every_field_the_specification_names_still_reads`
runs each of the seven on the engine a generated machine uses, so this
paragraph cannot go stale again without a test going red. What a call
on one of those fields does is §3.7.

### §3.4 Unsupported expression-language constructs

The forge expression language (SCE_FORGE.md §3.4) is a typed subset of
ECMAScript with a Lua-compatible runtime. Constructs that either have
no typed interpretation or are explicitly excluded:

- Loose equality / inequality (`==`, `!=`) —
  `expression/strict-equality`. Use `===` / `!==`. Extended SCXML is a
  typed language and admits no implicit coercion, so the operator whose
  ECMAScript meaning *is* coercion has no interpretation here
  (SCE_FORGE.md §3.4). The rejection carries a
  `replace_with` fix, and both lowerings agree on the result: codegen
  emits the target language's `==`, and the script-engine path rewrites
  `===` → Lua `==` / `!==` → `~=`.

  This applies to Extended SCXML expressions — a transition `cond` that
  reads typed `_event.data.<field>` from an imported EventSchema. A
  `cond` on an un-schema'd event is plain ECMAScript evaluated by the
  script engine, where `==` is legal and stays legal (the W3C corpus
  depends on it).
- A conditional expression whose result type Go cannot name (Go-only:
  the emitter lowers `c ? a : b` to a typed function literal and needs
  a type from the slot the value flows into or from its branches) —
  `expression/go-ternary-unsupported`. Give the value a typed
  destination, or type one of the branches.
- A value of one kind where the place it flows into declares another —
  `expression/type-mismatch`. An output, a local, a returned value, a
  condition and a parameter each declare the type of what lands in them,
  and with no implicit coercion a `bool` is not a number and a number is
  not a `bool`. A real is not an integer either: the backends do not
  agree on rounding it, so a real where an integer is declared is refused
  — `round(…)` or `floor(…)` says which the document means. An integer
  stands as any integer width, wrapped to it, or as a real; a real as a
  real of either width; a string as bytes. Judged once, before any
  backend emits, so every backend refuses the same documents. ⚠ Until
  2026-09-24 nothing judged it: a comparison assigned to a `uint16` local
  generated on all six backends and compiled only on those that convert
  on their own, and a real assigned there reached Rust as an `f32`
  assigned to a `u16`.
- A call of a function the document registered — an imported algorithm,
  transform, condition, lookup or interpolation, a `<sce:helper>`, a
  stateful import's method — with more or fewer arguments than it takes
  — `expression/argument-count-mismatch`, placed at the callee.
- An integer literal the type it takes cannot hold —
  `expression/literal-out-of-range`, placed at the literal. A literal
  takes the type of the place it lands in, of the operand it meets, or of
  the parameter it is passed to; a leading minus is part of it, so `-128`
  stands as an `int8` and `-1` as no unsigned type, and a literal wider
  than 64 bits stands as none. ⚠ Until 2026-09-24 nothing judged it:
  `300` as a `uint8`'s initial value generated on all six backends —
  rustc and `go build` refused it, C, C++ and Kotlin made it 44, and
  Python kept 300.
- Free-form tokens not part of the grammar —
  `expression/unsupported-construct`, `expression/unexpected-token`,
  `expression/invalid-lvalue`, `expression/type-coercion`,
  `expression/parse-mismatch`, `expression/lex` and `expression/empty`.
  ⚠ This line also said an integer literal's overflow dispatches as
  `validation/numeric-parse`; no expression literal reaches that code,
  which judges attribute values — measured 2026-09-24, an expression
  compared with `99999999999999999999999` checked clean.

### §3.4.1 The forge expression vocabulary — four names

A forge expression (`expr=` on an `sce:kind` document's `<data>`) may call
exactly four names without the document registering them:

| Name | What it does |
|---|---|
| `len(x)` | length of a `bytes` or `string` |
| `eq(a, b)` | bytes comparison |
| `round(x)` | nearest whole number, **half away from zero** |
| `floor(x)` | largest whole number not greater than `x`, **toward −∞** |

**And, in a transform's output expressions only, `previous(x)`** — the
value field `x` of the same document held at the end of the previous
activation. `x` is an input or an output of that document and nothing
else; any other argument is refused as `expression/parse-mismatch`, and
an unknown name as `expression/unknown-identifier`. On the first
activation `previous(x)` is `x`'s `sce:initial`, which is therefore
required on any field read this way (`validation/missing-attribute`,
with the attribute to add as the fix). A read through `previous()` is not
a dependency, so it cannot close an output cycle: `x = previous(x) + 1`
is legal where `x = x + 1` is `validation/transform-output-cycle`.

**How every backend lowers it.** Each output stays a pure function. A
field read through `previous()` adds one parameter, `previous_<x>`, to
every output function, after the inputs and in the order inputs then
outputs — so a field of the document spelled `previous_<x>` collides
with it and is refused as `validation/duplicate-id`. Beside the
functions, the transform gains a **holder**: the object that keeps each
of those values between activations. It has three operations and one
record type, named `<Name>` and `<Name>Outputs` for the document's
PascalCase name:

| | C++ / Kotlin / Python | Rust | Go | C11 |
|---|---|---|---|---|
| a holder at every `sce:initial` | `<Name>()` | `<Name>::new()` | `New<Name>()` | `<name>_init(&h)` on a `<name>_state_t` |
| back to every `sce:initial` | `reset()` | `reset()` | `Reset()` | `<name>_reset(&h)` |
| one activation | `update(inputs…)` | `update(inputs…)` | `Update(inputs…)` | `<name>_update(&h, inputs…)` |
| a holder from stored values ¹ | `<Name>.restored(stored…)` ² | `<Name>::restored(stored…)` | `Restored<Name>(stored…)` | `<name>_restore(&h, stored…)` |

¹ Only when a field read through `previous()` is also `sce:retain`. A
retained value outlives the program, keeping it is the host's, and this
is where the host hands it back: each retained field's stored value, in
cell order, while every other kept value starts at its `sce:initial`.
`reset()` still returns every one to `sce:initial` — the value of the
first day, not of the last boot. ² A static member on C++, a companion
function on Kotlin, a classmethod on Python.

One activation computes EVERY output from this activation's inputs and
the kept values, and only then replaces each kept value — an input's
with this activation's input, an output's with what it just computed —
and returns every output in the record (`<name>_outputs_t` on C11). No
field a document declares can take a name the holder introduces for
itself: a field named `holder`, `out` or `self` is legal, and the
holder's own name moves out of its way. A host learns that a document has
a holder, and the names to call, from the `holder` object in
`sce-codegen generate`'s manifest (`SCE_ERROR_CONTRACT.md` §10.1) —
whether a transform keeps state is its content, not its `sce:kind`.

⚠ **Two cells are refused**, each as `generate/unsupported-feature`
pointing at the read, before any renderer runs: a `bytes` field on every
backend (a buffer kept between activations needs a capacity no cell
declares), and a `string` field on C11 (a string is a pointer into the
caller's buffer, which the next activation may overwrite). The other
five backends keep an owned string.

⚠ **A transform that reads `previous()` cannot be called through an
import yet.** An `<sce:import kind="transform">` stands for a pure
function the importing document calls with its own values, and such a
transform's functions also take the values its holder keeps — which the
importer has nowhere to keep. A document that names such an import is
refused as `generate/unsupported-feature` on the `<sce:import>`
element's own line, rather than emitted as a call with too few
arguments. One that only declares it depends on nothing (§2.4) and
passes.

Everything else callable reaches a forge expression by being REGISTERED —
a stateless cross-file import, an `<sce:helper>`, or a stateful import's
method. A name in neither place is refused as
`expression/unsupported-builtin`, and the message names *the forge
expression layer* rather than the ECMAScript datamodel: they are different
vocabularies, and `Math.round` is in the second but not the first.

**`round` rounds half away from zero, and SCE enforces that rather than
inheriting it.** The backends disagree:

- C/C++ `std::llround`, Rust `f64::round`, Go `math.Round` — half away from
  zero (`0.5 → 1`, `2.5 → 3`, `-0.5 → -1`)
- Python `round`, Kotlin `kotlin.math.round` — **half to even**
  (`0.5 → 0`, `2.5 → 2`)

Python and Kotlin are therefore lowered through `floor(x + 0.5)` /
`ceil(x - 0.5)` by sign instead of the language builtin. The divergence is
visible only at `.5`, so `tests/forge/resources/transform_rounding.scxml`
feeds `0.5`, `2.5` and their negatives — `1.5` alone would not separate the
two rules, because that is the one boundary case where they agree.

The result type is the declared output's integer type, not a fixed width:
`<data sce:type="int32" expr="round(raw * 0.1 - 40.0)"/>` yields `int32`. The
same holds for `floor`.

**`floor` goes toward −∞, which is not what truncation does.** `floor(-2.7)`
is `-3`; discarding the fractional part gives `-2`. They agree on every
non-negative input, so a fixture of positives cannot tell them apart and would
accept either lowering in all six backends —
`tests/forge/resources/transform_floor.scxml` therefore carries `-2.7`,
`-0.5`, `-0.999` and `-1.999`, `-0.5` being the sharpest (toward −∞ it is
`-1`, toward zero it is `0`).

⚠ Unlike `round`, the backends already agree here: C/C++ `std::floor`, Rust
`f64::floor`, Go `math.Floor`, Python `math.floor` and Kotlin
`kotlin.math.floor` all go toward −∞, so none of them needs the sign branch
`round` needs in Python and Kotlin. The fixture exists anyway, and its history
is the reason: it was first written the other way round, on the reading that
the source notation's word means "discard the digits". The implementation this
generator must agree with uses floor throughout and truncation nowhere. A
word's connotation is not evidence about a boundary.

⚠⚠ Neither `round(x, digits)` nor `floor(x, digits)` is provided, and neither
is needed: "round down to two decimals" is `floor(x * 100) / 100` and a
quantise-to-a-grid is `floor(x / step) * step`. Both compose from these names
plus arithmetic that already lowers. A digit-taking overload would be a second
spelling of something the vocabulary can already say, and a second place for
the boundary rule to drift.

### §3.5 ECMAScript standard-library names the datamodel does not carry

W3C SCXML Appendix B.2 names ECMAScript as a data model but does not
oblige a processor to implement all of ECMA-262's standard library.
SCE implements the part its corpus reaches for, in one shared
`ecma_semantics.lua` every engine loads, and refuses the rest **by
name** — `expression/unsupported-builtin`:

- **Method calls.** `.map()`, `.filter()`, `.trim()`, `.getTime()`,
  `.startsWith()` and the rest of ECMA-262's prototype vocabulary.
  What the datamodel lowers is `charAt`, `concat`, `indexOf`, `join`,
  `push`, `replace`, `reverse`, `slice`, `sort`, `split`, `substring`,
  `toLowerCase`, `toString`, `toUpperCase`, plus `length` as a
  property; the diagnostic carries that list as its
  `fix: replace_one_of` candidates. The method is named without the
  call — `actual` is `.map`, a candidate `.join` — because the call
  carries the author's arguments: the name is what stands on the
  reported row, and a candidate replaces it and leaves the arguments
  where they were written.
- **Members of a namespace SCE installs.** `Math` — the functions of
  ECMA-262 15.8.2 minus the names Lua has no primitive for, and all
  eight constants of 15.8.1 — plus `JSON.parse` / `JSON.stringify` and
  `Object.keys`. A member outside one of those sets — `JSON.serialize`,
  `Math.tanh` — is refused against the set.

A method name in *neither* list is emitted as an ordinary field call,
because an author's own object is entitled to it:
`<data id="handlers" expr="{ retry: function() {…} }"/>` followed by
`handlers.retry()` is accepted and lowered.

Like every other expression refusal (§10.2 of `SCE_ERROR_CONTRACT.md`)
this is reported without failing the build — W3C §5.9.1 obliges an
unevaluable expression to raise `error.execution` at runtime rather
than be refused at generation time — and `--lint` promotes it to fatal
for documents this repository writes.

The record is placed at the refused token, on the row and column it
sits on, and its `actual` is the token as the document spells it: a
document that wrote `arr['map'](f)` is told `['map']`, and a `cond`
continued below its `<transition` row is refused on the row that holds
the token. A candidate replaces exactly that text, so the literal key
and the dot spelling take the same repair. A `<script>` body is element
text rather than an attribute, and its refusals are placed at the
`<script>` element.

`sce_build::ecmascript::builtins` is the single owner of both lists;
`sce-build/tests/ecmascript_builtin_vocabulary.rs` binds them to the
emitter and to the shared Lua library, so a name cannot be promised in
one place and missing from the other.

### §3.6 Identifiers the document does not declare

§3.5 decides a name from the name alone. A bare identifier cannot be
decided that way: `Date` is a global ECMA-262 defines, and it is also a
legal `<data id="Date">`. So the frontend resolves identifiers against
the document — every `<data id>` at any depth, every `<foreach item>`
and `<foreach index>`, every `<assign location>` and `<send
idlocation>` naming a variable, and every top-level `var` and
`function` a `<script>` declares (those are emitted without `local`
precisely so a later `cond` can read them, W3C test 302). Names bound
inside one expression — function parameters, a `var` in a function
body — are lexical and tracked as the expression is walked, including
ECMA-262's `var` hoisting.

A name left over is refused in one of two readings:

- **`expression/unsupported-builtin`**, when ECMA-262 defines it as a
  global and SCE does not install it — `Date`, `isNaN`, `Promise`,
  `console`. The candidates are the globals this datamodel *does*
  bind: `Math`, `JSON`, `Object`, `String`, `Number`, `Boolean`,
  `parseInt`, `parseFloat`, `In`.
- **`expression/unknown-identifier`**, when nothing defines it at all
  — a misspelling, or a `<data>` yet to be written. The candidates are
  the document's own declarations within a small edit distance, so
  `conut` beside a `<data id="count">` carries `count` as its
  `fix: replace_one_of`. There may be none, in which case the record
  carries no `fix` (§3 of `SCE_ERROR_CONTRACT.md`).

Two positions are deliberately exempt. `typeof x` on an undeclared name
is ECMA-262 11.4.3's one non-throwing read — it is how a document asks
whether something exists — and an `<assign location>` is a *write*,
which is how this datamodel's globals come into existence (§scxml-5.4
makes a location that cannot be created a runtime `error.execution`,
which every backend's assign path already raises).

`sce_build::ecmascript::scope` owns the declarations and
`sce_build::ecmascript::resolve` owns the walk;
`sce-build/tests/ecmascript_identifier_scope.rs` binds both to the
corpus and to the installed vocabulary.

### §3.7 Names this datamodel provides, written as a call

§3.5 and §3.6 both ask whether a name exists. This one asks what was
done with a name that does: `t.length()`, `Math.PI()` and
`_sessionid()` each reach for something this datamodel provides and
then call it. ECMA-262 11.2.3 answers that with a TypeError, and
before this rule the datamodel answered it with a `nil` at runtime and
`status: "ok"` at generation time.

Four closed lists carry the names, and each is closed for a reason
rather than by sampling:

- **`length`** — the only data property ECMA-262 gives a value in this
  datamodel's value space, on a string (15.5.5.1) and on an array
  (15.4.5.2). The emitter already lowers `.length` to Lua's `#` for
  every receiver, so the call form cannot be reaching an author's own
  method.
- **`Math`'s constants** — all eight of 15.8.1.
- **The system variables** — `_event`, `_sessionid`, `_name`,
  `_ioprocessors`, `_x`. A session binds each to a string or a table.
- **`_event`'s fields** — the seven §scxml-5.10.1 obliges every event to
  carry: `name`, `type`, `sendid`, `origin`, `origintype`, `invokeid`,
  `data`. The clause types the first six as character strings or URIs,
  and `data` is whatever the sender included, which reaches the
  datamodel as a `ScriptValue` — a union with no callable member. So
  `cond="_event.name() == 'go'"` calls a string.

  This list decides what a *call* is refused on, and nothing else. The
  clause obliges those fields to be **present**; it does not say an
  event carries only them, and W3C test178 reads `_event.raw` — a field
  the specification never names, which an Event I/O Processor supplies
  and which this repository registers, generates and runs. A member
  outside the list is left alone in both positions, exactly as an
  author's own object is, and the receiver is what decides the rule:
  `handlers.name()` is the author's own function and stays legal.

The refusal is `expression/property-not-callable`, and its repair is
the name itself: the record carries `fix: replace_with` naming the
property, so `t.length()` becomes `t.length` without the consumer
consulting anything. A call that carried arguments is the one shape
with no single replacement — dropping it would discard them — so that
record names the property and carries no `fix` (§3 of
`SCE_ERROR_CONTRACT.md`).

This is why the code is distinct from `expression/unsupported-builtin`
rather than reusing it. That code states the name is absent and offers
what is present in its place; stating it of `Math.PI` was false for as
long as the two shared a variant.

Every rule in §3.5 through §3.7 is stated about the property being
named, not about the syntax that names it. ECMA-262 11.2.1 defines
`t.length` as `t['length']` — one operation with two spellings — so the
frontend folds a literal key into the member form as it parses, before
any of these rules run. While the two spellings were two AST nodes,
each rule reached only one of them: `t['length']` became a field access
that read a `nil` where `t.length` is measured, `Math['PI']` indexed a
table this datamodel does not install, and `_event['name']()` generated
cleanly one token away from a refusal. A key that names nothing this
datamodel knows lowers exactly as it did.

A fifth list decides the other position the same mistake reaches: the
namespace itself. `Math`, `JSON` and `Object` are reached through a
member — `Math.abs(x)`, `JSON.parse(s)` — and written as the call
itself none of them is a function. `Math()`, `Object()` and
`new Object()` used to reach the engine verbatim, where `Math` is not
bound at all (the emitter rewrites `Math.<member>` to Lua's own `math`,
so the capitalised name exists nowhere) and the other two are tables no
engine makes callable. All three died on evaluation with `status: "ok"`
at generation time.

The refusal is `expression/namespace-not-callable`, and it carries no
`fix`. Dropping the call is not the repair — `Math` alone is refused
too, so that edit turns one refusal into another — and naming a member
means naming its arguments, which is the author's decision. The members
that may stand there ride `expected` as metadata instead, which is the
non-overlap shape §3.2 of `SCE_ERROR_CONTRACT.md` gives a producer with
no structured repair to propose.

This is why the code is distinct from `expression/property-not-callable`
rather than reusing it: that code's whole promise is that the name it
reports holds a value, and a namespace holds none this datamodel hands
out.

The rule states one thing about three positions, and the third is the
read. `<assign expr="Math"/>` used to generate on every backend and mean
two different things: the four that lower to Lua answered it with a
`ReferenceError` — `Math.<member>` is rewritten to Lua's own `math`, so
the capitalised name is bound nowhere — and the two that hand the
author's ECMAScript to an ECMAScript engine answered it with the object
the language defines. One document, two meanings, nothing reported. That
read is `expression/namespace-not-a-value`, and its member list carries
both halves of the vocabulary because a read may legally name `Math.PI`
where a call may not.

A literal written as the thing being called is the third position, and
the only one whose callee needs no list at all: `1()`, `'abc'()` and
`null()` are typed by having been written. What they produced was worse
than a wrong lowering — `1()` and `true()` are not Lua at all, so the
chunk carrying one failed to load rather than failing to run, while
`check` answered ok. The refusal is `expression/literal-not-callable`
and it carries neither `expected` nor `fix`: dropping the call leaves
the literal, which is not what the author was reaching for, and nothing
else follows from what was written.

The rule stops at literals. `(1 + 2)()` is a call on something this
datamodel could also prove is not a function, but proving it means
inferring a type, and the ECMAScript frontend has no inference pass
between its AST and its emitter — the W3C datamodel is untyped and so is
the engine underneath.

The member reach itself is checked for every namespace now, not only for
`Math`. `JSON` and `Object` are ordinary tables this repository installs,
so `JSON.serialize` was emitted as a field access on one of them and read
`nil` at runtime — the same silence the call form
`JSON.serialize(x)` had already stopped answering with. Membership is a
fact for all three namespaces, so it is asked in both positions for all
three: an unknown member is `expression/unsupported-builtin` whether it
was read or called.

`sce_build::ecmascript::builtins` owns the five lists,
`sce_build::ecmascript::parser` folds the two spellings, and
`sce-build/tests/ecmascript_property_calls.rs` plus
`sce-build/tests/ecmascript_member_access.rs` bind each rule to the
engine that runs the lowered expression.

### §3.8 Inline `<content>` text is not an expression that must parse

§3.5 through §3.7 judge text that *claims* to be an expression — an
`expr` attribute, a `cond`, a `<script>` body. Inline `<content>` text
makes no such claim, and §B.2 gives it ordered readings instead: an
expression if it is one, XML if it opens with `<`, and otherwise a
string. `<content>21</content>` is therefore the number 21 (W3C test
529), `<content>'foo'</content>` the string `foo` (test 294), and
`<content>inline payload</content>` the string `inline payload` — no
diagnostic, because nothing was refused.

The three places the specification puts inline content take the same
readings: `<data>`, `<send><content>` and `<donedata><content>`. The
last of them did not, and the difference was visible from outside — the
inline body shared a model variant with the `expr` attribute, so a
payload of ordinary prose was lowered as an expression, reported as
`expression/unexpected-token` against a document that had written no
expression, and reached `error.execution` at runtime.

`<content expr="X"/>` keeps the reading its attribute names: X is an
expression, and one that cannot be evaluated raises `error.execution`
rather than being refused at build time (§scxml-5.9.1, W3C test 344).
The two are distinct model variants for that reason.

`sce_build::filters::to_lua_data_content` carries the readings for the
backends that lower to Lua and `to_author_data_content` for the two that
hand the author's ECMAScript to an ECMAScript engine;
`sce-build/tests/donedata_inline_content.rs` binds both to the engine
that runs the result.

### §3.9 Where a document writes

§3.5 through §3.8 judge text a document *reads*. Four attributes name a
place it **writes**: `<assign location>`, `<send idlocation>`,
`<foreach item>` and `<foreach index>`. §scxml-B-2 restricts each to a
location expression — an identifier, or a member/index path rooted at
one — and SCE lowers all four through the same ECMAScript frontend the
reads go through.

That the two go through one frontend is the point, not an
implementation detail. ECMA-262 11.2.1 defines `arr[0]` once, and a
Lua table's first element is at index 1, so a write spliced verbatim
and a read that was lowered name *different cells*: `<assign
location="arr[0]" expr="99"/>` followed by `cond="arr[0] == 99"` took
the false branch on every backend that runs the datamodel on Lua, with
no diagnostic and exit 0.

A target that is not a location expression — `location="1 + 1"`,
`item="'continue'"`, an unterminated string — is reported at the
element that wrote it, as `expression/invalid-lvalue` or whichever
`expression/*` code the parse failed with. Like a `cond` that cannot be
evaluated (§3.4), it does not make the document ungeneratable:
§scxml-5.4 makes a location that denotes nothing a runtime
`error.execution`, so the artifact carries an assignment target that
raises the same message when the engine reaches it.

Unlike a read, a write target is **not** resolved against the
document's declarations — writing is how this datamodel's globals come
into existence (§3.6 states the same exemption from the other side).
`<param location>` is a read, not a write: the value at that location
becomes the payload field, so it goes through the value seam.

`sce_build::ecmascript::to_lua_location` lowers the target,
`sce_build::filters::to_lua_location` is the seam every backend
template goes through, and
`sce-build/tests/ecmascript_write_targets.rs` runs a write and a read
of the same authored text on the engine a generated machine uses.

### §2.10 Metadata annotations — `sce:req` / `sce:provenance` / `sce:unresolved`

NL→IR Mapping Roadmap Items 1, 5, and 6 add three metadata
attribute families that any IR generator (NL→IR pipeline,
hand-authored DSL, ARXML transcoder) may attach to `<state>`,
`<parallel>`, `<final>`, `<transition>`, `<onentry>`, `<onexit>`,
the executable-content actions inside those blocks, `<invoke>`, and
a statechart's `<data>` — in the document's `<datamodel>` and in a
state's. The annotations are pure metadata — emitted code is
byte-identical to the unannotated form, byte-stable goldens stay
unchanged.

⚠ **`<data>` joined the list on 2026-09-28, and it is where a guess
most often sits**: a threshold a specification names without a number
is a variable's initial value. Before then the grammar accepted the
attributes there and the parser dropped them, so `sce-codegen
unresolved` listed every other marker in a document and not these
(measured: two of seven markers one document wrote). A marker written
in element form inside a `<data>` is read as an annotation and is not
part of the variable's in-line value (§scxml-5.4). What `<data>` does
not yet have is the generated-source comment the other sites carry
(see below): its annotations reach `requirements`, `unresolved`, the
transition table, the manifest comparison and `--strict-unresolved`,
and no backend echoes them next to the variable's declaration.

**`sce:req`** — whitespace-separated requirement IDs.

```xml
<state id="armed" sce:req="REQ_AB_12345 REQ_CD_67890">
  <transition event="go" target="firing" sce:req="REQ_AB_12346"/>
</state>
```

Tokens are opaque to SCE (no shape enforcement — IR generators
own the semantic layer). Duplicates on a single node fail at
parse time with `validation/duplicate-requirement-id`. Block
annotations on `<onentry>` / `<onexit>` inherit onto every
action inside the block — including actions nested inside
`<if>` / `<foreach>` there — appended after any per-action ids.
`sce-codegen requirements <file>` emits one NDJSON record per
annotated node for downstream req-coverage tooling — annotated
by `sce:req` or by `sce:provenance`, since the two are
orthogonal and either alone is worth reporting.

With `--manifest`, several files are read as ONE design:
`sce-codegen requirements front.scxml press.scxml --manifest m.json`.
A statechart that closes its interface is checked together with the
event schemas it imports, and a schema claims nothing, so measured file
by file every requirement reads `missing` in each of them, which is true
of the schema and says nothing about the design. The claims of every
document are pooled and classified once, by the function that answers
for a single document: a requirement is met when a node of any of them
carries it, and an id the manifest does not hold is `dangling` wherever
it is cited. Each node path then names its document
(`front.scxml#states.idle`), because `states.idle` is a place in every
statechart; with one document the paths are unqualified as before.
Several documents without `--manifest` are refused (`cli/usage`):
without a denominator the claims of several files in one stream would
not say which file a node is in.
`sce-build/tests/a_design_of_several_documents_is_measured_together.rs`
holds each of these.

⚠ "Opaque" is checked, not merely promised:
`sce-build/tests/requirement_id_opacity.rs` drives ten id
spellings through the parser — a leading digit, a bare number,
a slash, a hash, a colon — and fails if any is refused or
rewritten. That sentence was unenforced until the file existed,
and the tree contradicted it: a `RequirementId::validate`
enforcing a letter-or-underscore first character sat in
`provenance.rs` with no production caller. It is deleted.
Re-adding shape enforcement means changing this paragraph
first, and note what it would cost — ISO 13400-2 numbers its
requirements `3.DoIP-152`, which a leading-character rule
refuses.

⚠⚠ **Annotation text is encoded for the comment it lands in.**
Opacity has a downstream cost, and it was wider than first
registered. An id may contain `*/` or end in `\`, and an
`sce:unresolved` reason may carry a newline (`&#10;`). Measured
2026-09-13 through all six backends, each of those put author
text into generated code: the C11 block comment closed at `*/`;
the five line-comment backends broke at the newline, where
Python then failed to compile and Go parsed the injected line as
a statement; and a trailing `\` spliced the next C++ line into
its comment. The emitter's own header had called the newline case
"author-side hygiene, not a SCE invariant" — text that is opaque
by contract cannot be the author's job to keep out of the
emitter's syntax.

Every template value written into a comment is therefore encoded,
and not by the templates. `generator::register_template` is the one
way a template enters an environment; it reads the template in the
language it emits (`sce_build::template_lexing`) and routes each
`{{ … }}` that sits inside a comment through
`sce_build::comment_text::encode`, which writes `\`, LF, CR, the `/`
of `*/` and the `*` of `/*` as `\xHH` and leaves every other
character alone. An ordinary value is byte-identical, and the
grammar is the same in all six backends. The annotation values were
the first case and not the only one: measured 2026-09-13, 963
template interpolations sat inside a comment and none was encoded —
the C11 comment echoing a `<log>` element's `expr` closed at `*/`,
and the Go comment echoing a `<data>` element's `expr` put a line
break into code. The one macro whose comment delimiters come from a
variable (`_macros/sce_annotation_marker.jinja2`) cannot be read that
way and writes `| comment_text` itself. **A reader recovering ids,
anchors or reasons from generated source must decode them** with
`comment_text::decode`, which refuses a payload the encoder did not
write. `sce-build/tests/a_value_written_into_a_comment_is_encoded.rs`
holds the arrangement — one registration door, a per-syntax census,
no second encoder inside a comment, and hostile documents rendered
through all six backends — and
`sce-build/tests/sce_annotation_emission.rs::hostile_annotation_text_stays_inside_its_comment`
holds the annotation macro.

**A value written into a string literal is escaped for it, at the
same door.** A comment has one encoder; a string literal's is the
emitted language's own escaper, and templates applied one at some
sites and not at others — so whether an author's text was safe
depended on whether whoever wrote that template line remembered.
`generator::register_template` now reads each template once and routes
every interpolation landing inside a literal through that syntax's
escaper (`literal_text::encode_template_literals`), exactly as it
already does for comments, so the guarantee is a property of the door
rather than of a filter a template author can forget.

The census is **derived rather than recorded** — a number this document
kept would be a number nothing re-measures — by
`the_census_the_documentation_cites_is_derived_from_the_tree` in
`sce-build/tests/a_value_written_into_a_string_literal_is_escaped.rs`,
which prints it. Read 2026-09-14 over the 282 `(template, syntax)`
pairs the loaders register: 1868 interpolations land in an escapable
literal and 1435 carry no escaper of their own. Most are ids, events
and derived names, which §1 above checks against W3C's grammar **at
parse** and which therefore cannot carry a quote or a line break; the
remainder — 349 sites — is free text and expressions (`cond`, `expr`,
`location`, `<log label>`, `src`, `namelist`), which no grammar
constrains and which W3C does not permit SCE to constrain. The
measured breaking case was a `<log label>` holding a line break,
written unescaped into a C++ (`SCE_LOG_INFO("…")`) and a Go
(`fmt.Println("…")`) literal so that the emitted source did not
compile.

**A RAW literal takes the opposite treatment, and the door tells the
two apart.** Go's `` ` ``, Rust's `r#"…"#`, C++'s `R"d(…)d"` and
Kotlin's `"""` process no escape sequence, so escaping a value for one
corrupts it rather than protecting it — measured 2026-09-14, when a
first version of this door wrote `<data>` XML content into the Rust
and Go backends as `<books xmlns=\"\">\n …`, wrong in the parsed
document and silent at compile time. `template_lexing` therefore
classifies a raw literal as its own class, and the door answers it two
ways. Where the closing sequence is a character free text all but
never carries — Go's backtick, which holds the sites that put Lua
source into readable Go — the value passes through untouched and a
value that does carry one is refused, naming it. Where it is not —
Rust's `"#`, which an `href="#top"` attribute is enough to produce —
the template itself is refused at registration, and its repair is to
write an escapable literal. One template did: the Rust `<data>` DOM
site, which now emits an escaped literal of the same string.

⚠ **Still open — a literal that is a FORMAT string needs more than
escaping.** `SCE_LOG_*` expands to `fmt`, where `{` and `}` are a
replacement field, and Go's `fmt.Printf` reads `%` as a verb. Escaping
for the literal correctly leaves both alone, so a value carrying one
must not be spliced into a format string at all — it belongs in an
argument. `escape_cpp_format` is the stronger filter for the sites
that still splice, and the door defers to it where a template applies
it. Registered, not fixed.

**Sourcemap markers are read back through the grammar that wrote
them.** A `SCE-MAP:` marker is a comment, so the path and attribution
it carries are encoded like any other comment value, and
`forge::sourcemap::read_marker` — the reader the ownership walker runs
after every generate — decodes each field with `comment_text::decode`
and refuses a field the encoder did not write. A marker spelled in
Rust rather than rendered from a template, such as a rejected
document's stub, is written by `forge::sourcemap::marker_payload`
through the same encoder. A Go `//line` directive is the exception,
because its reader is the Go toolchain, which decodes nothing:
`template_lexing` classifies it as a directive rather than a comment,
and the door passes a value in one through untouched and refuses the
line break that would end it. The C-family `#line N "…"` and Rust
`#[doc = "…"]` forms are string literals, escaped at the literal door.
`sce-build/tests/sourcemap_ownership_walker.rs` generates a document
whose name the encoder changes through all six backends and reads
every marker and directive back.

**`sce:provenance`** — spec-document anchors.

Two equivalent forms:

```xml
<state id="armed" sce:provenance="OEM-SPEC-01@23#4.4.2:page=118"/>

<state id="armed">
  <sce:provenance doc-id="OEM-SPEC-01" rev="23" section="4.4.2" page="118"/>
  <sce:provenance doc-id="WORKBOOK-2" section="Sheet1" row="41"/>
  <sce:provenance doc-id="ISO-14229-1" section="11.2.1"/>
</state>
```

The compact URI form is `doc_id[@rev][#section[:position]]`. The
element form decomposes the same grammar — `doc-id` is its only
required attribute — and allows multi-document anchoring on a
single node.

An anchor names a **division** of the source in `section` and,
optionally, a **position** inside it. The two are separate
because they answer different questions and only one of them is
shaped by the kind of source: every source has divisions a
reviewer can be sent to — a numbered subclause, a worksheet, a
package — and the per-division coverage counts group by that key
for all of them alike. Where inside the division is source-shaped,
so it is one of a closed set:

| Position | Compact spelling | Element attribute | A source that uses it |
|---|---|---|---|
| page | `:page=118` | `page="118"` | a paginated specification |
| row | `:row=41` | `row="41"` | a worksheet |
| path | `:path=/Elem/x` | `path="/Elem/x"` | a structured document |

A compact form whose trailing `:` segment spells none of these
leaves it part of the section — a division id may legitimately
contain a colon, and losing the division would be worse than
carrying no position. The bare spelling `#4.4.2:118` is also
still read as a page: it predates the closed set, and removing it
would not fail an unmigrated document but silently re-read the
number as part of a longer division id. Both forms attach to every element `sce:req` does
(`<state>`, `<final>`, `<parallel>`, `<transition>`, `<data>`, `<onentry>`,
`<onexit>`, executable content, `<invoke>`), compose additively
in document order, and inherit from `<onentry>` / `<onexit>` onto
every action in the block exactly as `sce:req` does — matched on
`doc_id`, so an action's own anchor for a document is not
overwritten by the block's. Pass-through to the diagnostic
`spec_provenance` field — SCE never infers it.

Two rejections, both defending the `(doc_id, rev)` set a
consumer compares against the revisions actually in force. An
anchor that names no document — an empty value, a compact URI
with an empty `doc_id` (`@23`), or an element without a usable
`doc-id` — fails at parse time with
`validation/provenance-malformed`; accepting it would be
indistinguishable downstream from a node that was never
annotated. The same `doc_id` twice on one node, in any
combination of the two forms, fails with
`validation/provenance-duplicate` — the node would otherwise
carry two answers to which revision governs it.

`sce-codegen requirements <file>` carries the anchors on each
record's `spec_provenance`, verbatim and in document order, and
omits the field on a node that has none. SCE reports what the IR
claims to depend on; comparing that set against the revisions
actually in force belongs to whoever owns the document set.

The anchors reach two further surfaces. Generated source carries
them as a comment on every backend, one line per anchor in the
same compact spelling the attribute uses, next to the `sce:req`
line — so a reader of the emitted code can go to the paragraph
without going back to the SCXML. And a rejection raised about an
anchored node carries them on the diagnostic wire's
`spec_provenance`, so a CI gate that refuses a build hands the
reader the document to consult rather than only the line to look
at.

**An anchor governs what it encloses.** A diagnostic carries the
anchors of the innermost anchored node that *encloses* its
location, not only of the exact node complained about — so
annotating `<state id="s0">` answers for a `<transition>` inside
it, and an author does not have to repeat the attribute on every
descendant to keep the link. The consequence worth writing down
is what an *empty* `spec_provenance` means, and it means exactly
one thing: no node enclosing that location carried an anchor. It
never means the complaint came from a stage that does not carry
them. `SCE_ERROR_CONTRACT.md` §2.1.2 is the normative statement, and
which codes satisfy it today is a lookup rather than something to
assume: `sce-codegen provenance-roster` publishes one line per
code, with a verdict and a reason, so nobody has to read SCE's
source to find out. The table behind it is compile-time
exhaustive and carries a written reason for every code that does
not carry — `forge::diagnostic::anchor_carriage`, which left
`#[cfg(test)]` on 2026-09-14 for exactly that reason. ⚠ A code
the roster reports as `unknown` or `pending` has not been shown
to fail this clause; it has not been shown to meet it either.

**`sce:unresolved`** — explicit "revisit later" markers.

Two equivalent forms (attribute carries one marker; element
form allows multiple per node):

```xml
<state id="armed"
       sce:unresolved="tbd_threshold"
       sce:unresolved-reason="awaiting calibration"
       sce:unresolved-candidates="42 50 65"/>

<state id="armed">
  <sce:unresolved id="tbd_target" reason="route TBD" candidates="left right"/>
</state>
```

Default builds carry the marker silently in the model — the
`sce-codegen unresolved <file>` NDJSON report surfaces it for IDE
/ linter / dashboard consumers. `--strict-unresolved` on
`generate` lifts the marker to a build-failing
`validation/unresolved-placeholder` so production CI gates
cannot merge unresolved IR.

The three families compose freely on a single node — `sce:req`,
`sce:provenance`, and `sce:unresolved` are orthogonal axes.

#### §2.10.1 Why the document is its kind — `<sce:kind-basis>`

The families above say where a node comes from. None of them says why the
document is a transform rather than a lookup, and that is the first
decision an author makes from a specification — one the product cannot
make, because the evidence is prose. Until this element the reason lived
only in the conversation that produced the document, so the owner
reviewing the pseudocode saw a kind and never the clauses it was chosen
from.

```xml
<scxml ... sce:kind="transform" name="adc_to_volts">
  <sce:kind-basis>
    <sce:evidence provenance="SPEC-7@2#4.1">the voltage is one formula over
      the current count</sce:evidence>
    <sce:rejected kind="interpolation">the text gives a formula, not values
      at breakpoints</sce:rejected>
  </sce:kind-basis>
  ...
</scxml>
```

- **Where**: directly under the `<scxml>` root, at most once, on every kind
  alike — a statechart's root as much as a forge root. An inline kind
  (`<data sce:kind>`) takes its statechart's.
- **`<sce:evidence>`**, one or more: the author's account of what the
  specification states, as text. `provenance`, optional, is the compact
  anchor of `sce:provenance` (`doc_id[@rev][#section[:position]]`).
- **`<sce:rejected kind="…">`**, any number: a kind considered and not
  chosen, and as text the behaviour that ruled it out.
- **The kind left open**: when the specification does not decide between
  kinds and a draft is written anyway, the choice is marked as undecided
  with the `sce:unresolved` family on the `<sce:kind-basis>` element
  itself — `sce:unresolved="kind"`, `sce:unresolved-reason` saying what the
  specification would have to state, and `sce:unresolved-candidates`
  naming the OTHER kinds still open (never the declared one, never one it
  also rejects). It is the marker it is everywhere else:
  `--strict-unresolved` refuses the build, `sce-codegen unresolved` lists
  it with `node_path` `<kind-basis>` in both pipelines, and the page
  writes it inside the `kind-basis:` block.
- **Metadata only**: no code generator reads it, so it changes no
  generated construct (the input-hash headers still move with the
  document's bytes, as they do for a comment). It reaches `sce-codegen pseudo`, as a `kind-basis:` block under the
  document's head line, the acceptance report, the AST export
  (`ParsedForge.kind_basis`), and the manifest, as
  `document_kind.basis_recorded`.

Every fault is `validation/kind-basis-malformed`, raised by the parser in
every build: a second basis, one anywhere but on the root, an
`<sce:evidence>` or `<sce:rejected>` outside a basis, a basis with no
evidence, an element with no text, a child it does not take, a
`<sce:rejected>` with no kind or a kind SCE does not have, the same kind
rejected twice, and — the contradiction the element exists to catch — a
`<sce:rejected>` naming the kind the document declares. An anchor that
names no document is `validation/provenance-malformed`, as everywhere
else. The schema (`sce-forge-ext.xsd`) is deliberately looser than the
parser, so that one code carries every fault whether or not the build
compiles the schema in.

### §2.11 Native host actions — `<sce:action>` (W3C SCXML G.7)

A `<sce:action>` is a W3C SCXML §G.7 Custom Action Element that
names a host operation dispatched **without a runtime script
engine**. It is the engine-free counterpart, for *effects*, of the
typed `_event.data` guard lowering: the statechart keeps the
operation symbolic (language-neutral SSOT), each argument flows
through the imported EventSchema's typed-payload channel, and the
host supplies the behaviour by implementing a generated trait.

```xml
<transition event="fragment.received" target="assembling">
  <sce:action name="append_fragment_payload">
    <sce:arg expr="_event.data.payload"/>
    <sce:arg expr="_event.data.offset"/>
  </sce:action>
</transition>
```

**Every backend** lowers it, each to its own language's expression of
an interface. One lowering serves all six — the SCXML names the
operation symbolically and the emitter spells it the way a host author
in that language would write it, so the six cannot drift into six
lowerings:

| backend | interface | how the host arrives |
|---|---|---|
| Rust | `pub trait <Machine>Actions` | `Policy<A: …Actions>` type parameter |
| C++ | `struct <Machine>Actions` (abstract) | the machine's only constructor |
| Kotlin | `interface <Machine>Actions` | first constructor parameter |
| Go | `type <Machine>Actions interface` | `New<Machine>Policy(actions)` |
| Python | `class <Machine>Actions(Protocol)` | `__init__(self, actions)` / `create_engine(actions)` |
| C11 | `<machine>_actions_t` (function-pointer vtable + `user_data`) | `<machine>_init_with_actions(sm, &vtable)` |

Parameter types come from the schema field types — Rust `bytes → &[u8]`
and `uint32 → u32`, Go `[]byte` / `uint32`, Kotlin `ByteArray` / `UInt`,
C++ `const std::vector<uint8_t>&`, Python `bytes` / `int`, C11 a
`const uint8_t *` plus its `size_t` length sibling.

Kotlin also generates `Recording<Machine>Actions`, from the same signatures:
an implementation that performs nothing and records every call, in order,
as a value of its sealed `Call` type (a `data class` per operation, a
`data object` for one with no argument), read through `calls`. A test drives
the machine with it and no hand-written host: send an event, read what the
machine asked the host to do, answer with an event, check the snapshot. A
`bytes` argument is recorded as a `List<Byte>` copy, so a recorded call
compares by value. The other five backends do not generate one yet.

**The Interpreter** loads a document at run time and has no interface to
generate, so its host is `INativeActionHost` (`sce/include/runtime/`),
installed with `StateMachine::setNativeActionHost` before `start()` — an
`<onentry>` of the initial state performs its actions during `start()`, the
reason the AOT host is a constructor requirement. The action's `name` is the
operation, and each `<sce:arg expr>` is an expression of the document's data
model, evaluated when the action runs, in document order, to a `bool`, an
integer, a real or a string; the host answers whether it performed the
operation. An action nobody performs is **not dropped**: no host installed, a
host that answers it provides no such operation, an argument that cannot be
evaluated, and an argument that is no value a host operation takes (an array,
an object, `undefined`) each raise `error.execution` — the rule §scxml-6.4.1
gives an `<invoke>` of a type the processor does not implement — and an
argument that fails stops the action before the host is called. It runs wherever
executable content does (inside an `<if>` too: the Interpreter has no v1
placement rule). A child session an `<invoke>` starts is given the host its parent's
host answers for it (`hostForChild`, "Child sessions" under §2.15).
Until this existed the Interpreter's action parser skipped the element like any
foreign one (§scxml-4.10), and the document's request vanished with no event.
`tests/integration/NativeActionRunsUnderTheInterpreterTest.cpp` holds it, and
`sce-codegen lower` lowers a `sce-static` document's arguments for it, so
`static_host_call` replays on the Interpreter with the same calls the
generated backends' hosts are given.

The host arrives where the machine is CONSTRUCTED on every backend, not
through a setter, and that is a requirement rather than a style: an
`<onentry>` in the initial state performs its act during
initialisation, so a host installed afterwards arrives one act too
late. Five backends make "the host was supplied" a compile-time fact;
C has no way to require a filled function pointer, so
`_init_with_actions` refuses a vtable with a NULL member and returns
`false`, leaving the machine uninitialised rather than running it into
a null call.

No backend's emitted machine carries a script engine for this
construct, so a statechart whose only effects are `<sce:action>`s
compiles under `#![no_std]` on Rust and links without Lua at all on
C11.

v1 acceptance contract (enforced at the validation stage):

- A `<sce:action>` is a **direct child** of a `<transition>`, an
  `<onentry>` / `<onexit>` block, or initial executable content (an
  `<initial>` transition or a history state's default transition).
  Nesting inside `<if>` / `<foreach>` is rejected — that call site is
  conditional or iterated, which v1 does not lower
  (`validation/native-action-placement`).
- Arguments require the triggering event's typed payload in scope,
  which happens only on a `<transition>`. An `<onentry>` / `<onexit>` /
  initial position has no triggering event, so only a **no-argument**
  `<sce:action>` is admissible there; an arg-bearing one is rejected
  (`validation/native-action-argument`).
- On a transition, each `<sce:arg>` is a bare `_event.data.<field>`
  reference (the `name` attribute, when present, names the trait
  parameter). A literal or derived argument, or one whose triggering
  event imports no EventSchema or whose schema is not all-primitive (an
  enum-typed field — the same eligibility rule as the typed-guard
  channel), is rejected (`validation/native-action-argument`).
- An argument's `<field>` must exist on the triggering event's
  imported EventSchema (`validation/invalid-reference` via the
  cross-kind field resolver).
- A `name` that recurs on more than one transition must carry the
  same argument types each time, so one generated trait method
  serves every call site; a divergence is rejected
  (`validation/native-action-signature-conflict`).
- A no-argument `<sce:action>` (e.g. `reset_slot()`) needs no
  schema and lowers to a bare trait call.

An arg-bearing action reads its values from the event's typed
payload, so the triggering event must be raised via its generated
typed inject. An event raised by NAME carries no payload and cannot
supply the arguments; the emitted call is wrapped in that backend's
typed-payload tag check, so the operation does not fire rather than
receiving a zero value the host would take for data. Rust says so as
well as skipping — a debug build `debug_assert!`s, and a release build
compiles the check away so an MCU pays nothing for it.

The construct is engine-free **by definition** — it never degrades to a
runtime fallback. That is why an ineligible argument is rejected at the
validation stage instead: there is no script-engine path to fall back
to, unlike the typed-guard channel one node over.

Until 2026-08-24 only the Rust backend lowered `<sce:action>` and the
other five refused it (`generate/unsupported-feature`). That refusal
was backend coverage rather than a design constraint — validation had
always been language-neutral — and it is retired. What replaced it is
`sce-build/tests/native_action_backend_parity.rs`, which asks all six
emitters the same questions from one place, plus six runtime channels
driving a real host implementation of the same document.

### §2.12 Unsupported `<invoke type>` (W3C SCXML 6.4.1)

An `<invoke>` whose `type` names no processor this platform
implements is **accepted, not rejected as malformed**: §scxml-6.4.1
defines the case — the processor "MUST place error.execution in the
internal event queue" — so the document is valid SCXML with a
defined meaning. Its entire observable is that one raise at invoke
time. No child session starts, `done.invoke.<id>` never fires, and
state exit has nothing to cancel.

```xml
<state id="probe">
  <invoke type="urn:example:no-such-processor"/>
  <transition event="error.execution" target="handled"/>
</state>
```

The supported set is closed: the `scxml` shorthand, the SCXML
processor URI with and without its trailing slash, and
`sce:mesh-rpc` (SCE_MESH.md §9.5). `typeexpr` resolves the type at
runtime and is therefore never classified at build time.

Both engines raise the same event, by different routes:

| Engine | §6.4.1 behaviour |
|---|---|
| Interpreter | `InvokeHandlerFactory::createHandler` returns null for a type outside the supported set and `InvokeExecutor::executeInvoke` raises `error.execution`. |
| AOT — C++ / Rust / Kotlin / Go / Python / C11 | The `<invoke>` is deferred at entry and the raise fires at macrostep end, preserving §scxml-6.4 ordering. Leaving the state first cancels the pending entry along with every other deferred invoke, so a machine that exits before the macrostep boundary raises nothing. |

No backend refuses the document. Refusing it would be a conformance
break rather than a coverage boundary: §6.4.1 assigns the construct a
meaning instead of declaring it malformed, so an AOT target that
rejected it would reject valid SCXML. The `<sce:action>` refusal above
is not a precedent here — that construct is an SCE extension with no
W3C meaning to preserve.

Accepting the document while emitting no raise is the failure this
subset clause guards against, because it reproduces the original
defect, in which the `<invoke>` vanished from the model entirely and
AOT produced no observable at all where the Interpreter produced one.
Wiring one backend does not close it for the rest: the model variant
that carries the unsupported invoke is skipped by each template's
`scxml`-family filter unless that backend is wired explicitly, so the
silent drop moves from the parser into the templates rather than
disappearing.

The runtime witness is
`integration_resources/invoke_unsupported_type/invoke_unsupported_type.scxml`,
driven on all seven channels (C++ Interpreter + AOT, Rust, Kotlin, Go,
Python, C11). It rests in its `probe` state and never completes on any
channel that drops the `<invoke>`.

#### Types the host runs (`--host-invoker TYPE`)

§6.4.1 leaves the invokable set to the platform, so a host may run a type
of its own. A build that declares `--host-invoker TYPE` makes every
`<invoke type="TYPE">` a start the host serves instead of the raise above.
The lifecycle is the W3C one, and the six AOT engines hold the bookkeeping,
not the host:

- **Start.** When the macrostep that entered the state settles, the host's
  registered invoker receives the request: `src` (or `srcexpr`, evaluated
  now), each `<param>` (a repeated name keeps every value, in order), the
  `namelist` values, the `<content>` (or `contentexpr`), and a **token**
  that identifies this start of this invoke. A declared type with no
  invoker registered raises `error.execution`.
- **Completion.** The host reports it with the start's token, and the
  document receives `done.invoke.<id>` — or the generic `done.invoke` when
  it names no specific one — with `_event.invokeid` set. A completion is
  accepted at most once, and only while its start is still running: one
  that arrives after the state exited, after a restart, or a second time is
  refused, as is a `done.invoke` for a host-run invoke raised through the
  ordinary event API.
- **Failure.** The host may instead report that the run failed, through the
  same door and under the same rules, and the document receives
  `error.invoke.<id>` — or the generic `error.invoke` — with the host's data
  and `_event.invokeid`. A run ends once, by completion, failure or
  deadline, whichever comes first. Either report may name the
  `_event.origin` / `_event.origintype` it carries, for a host whose
  invocation stands for another party (a Mesh router's `sce:mesh-rpc` names
  the peer that answered, SCE_MESH.md §9.5); left empty it carries what a
  host completion always has. The calls: Rust `fail_host_invoke` /
  `complete_host_invoke_from`, Go `FailHostInvoke` /
  `CompleteHostInvokeFrom`, C11 `_fail_host_invoke` /
  `_complete_host_invoke_from`, and `failHostInvoke` / `fail_host_invoke`
  beside an optional origin on the completion in Kotlin, C++ and Python.
- **Cancel.** Leaving the state cancels a start still running, once, with
  its token. A start that never ran or already completed is not cancelled.
- **Deadline.** The reserved `<param name="_sce_deadline_ms">` is the
  engine's and is not handed to the host. Its value is one or more ASCII
  digits, optionally followed by `.` and one or more `0`, within a signed
  64-bit count — one grammar for every runtime, held to
  `sce-build/tests/fixtures/host_processor/host_invoke_deadline_values.json`.
  If the deadline passes while the start is still running, the host is told
  to stop and the document receives `error.invoke.<id>` (or the generic
  `error.invoke`) with `_event.invokeid` set and `_event.data` the string
  `"deadline"`; a completion before it disarms it. A value outside the
  grammar raises `error.execution` and starts nothing. No `<cancel>` reaches
  a deadline. `sce:mesh-rpc` accepts the same name beside its own
  `_mesh_deadline_ms` (SCE_MESH.md §9.5).

The witness is `sce-build/tests/fixtures/host_processor/statechart_host_invoker.scxml`,
driven on the six AOT channels.

#### Typed interface — `sce:request` / `sce:result`

A host-run invoke may name the records its request and its completion
carry, each an alias of an imported event schema:

```xml
<sce:import kind="event-schema" src="perm_request.scxml" as="PermRequest"/>
<sce:import kind="event-schema" src="perm_result.scxml" as="PermResult"/>
...
<invoke type="x-app-host" id="perm" sce:request="PermRequest" sce:result="PermResult">
  <param name="scope" expr="'storage'"/>
  <param name="reason" expr="why"/>
</invoke>
```

`sce:request` makes the request a record: the `<param>`s are the schema's
fields, one each, so the host receives exactly one value per field and no
other parameter. `sce:result` types `_event.data` of that invoke's
completion, `done.invoke.<id>` — bound by the invoke, not by an event name,
so it does not conflict with the rule that refuses an event schema on
`done.invoke.*`. A guard reading a result field lowers natively like any
schema'd event's, and a completion whose data is not the record is refused
as any typed payload the data does not fit is: `error.execution`, and the
guard does not fire. Such a completion has no typed inject seam (`raise_…`):
the engine accepts a host-run completion only through its completion call,
which checks that the start is still running. The generic `done.invoke`
stays untyped — several invokes share it.

A typed request is held to its record where the invocation starts: each
`<param>`'s value, as the data model evaluated it, must be one its field's
type holds — a whole number within the declared width for an integer field
(refused rather than narrowed), a finite number for a fractional one, a
truth value for `bool`, a text for `string`, and for `bytes` a text of
characters up to U+00FF no longer than `sce:max-size`. A value that cannot
be evaluated or does not fit is an argument that cannot be evaluated
(§scxml-6.4.1): `error.execution`, and the host is never asked to start
the invocation — unlike an untyped `<param>`, which is reported and left
out, because the host was promised the whole record. The request still
crosses as text, each value spelled as its field's type spells it, so the
generated adapter below reads it back without a way to fail.

For each declared `type` the document types an invoke of, the backends
generate the host's side: the request and result records, an invoker
interface with a `start_<id>` taking the request record and a
`cancel_<id>` per typed invoke, an adapter that registers that interface
as the type's handler — dispatching by invoke id, and handing an invoke of
the same type the document does not type to a fallback handler the host
supplies with it — and a `complete_<id>` taking the result record. The
engine's own contract is unchanged; the interface is generated above it.

Refused at parse, on the row that shows the problem:

- `validation/typed-invoke-schema` — the attribute sits on an invoke SCE
  runs itself (`scxml`, `sce:mesh-rpc`) or one whose type is `typeexpr`;
  the invoke has no `id` (the generated interface is named after it); the
  attribute is empty; the alias names no imported event schema; or the
  schema has an enum-typed field, which has no text spelling every backend
  shares — declare it as its underlying integer type.
- `validation/typed-invoke-request` — a `<param>` the schema lacks, a field
  no `<param>` supplies, a name given twice, a `namelist` / `<content>`
  beside the typed request, or a string literal its field cannot hold (a
  literal is the one value known before run time, so it is held to its
  field here rather than where the invocation starts).

### §2.13 Hybrid `<invoke>` — `srcexpr` / `contentexpr` (W3C SCXML 6.4)

An `<invoke>` that names its child through an expression rather than a
literal `src` or an inline `<content>` is **accepted on every
backend**, and §3.1 used to say the opposite. What it generates is a
*hybrid* invoke: `generate_hybrid_child_scxmls` synthesizes one
`<document>_hybrid<N>.scxml` per hybrid invoke — a stub whose only
state is an immediate `<final>` — and the parent instantiates that
child. The generator states the reasoning where it writes the stub: an
immediate-`<final>` child produces the §scxml-6.4 `done.invoke`
sequence "regardless of what the original SCXML expression would have
named".

```xml
<state id="a">
  <invoke type="scxml" srcexpr="pathVar"/>
  <transition event="done.invoke" target="b"/>
</state>
```

The contract every AOT backend implements — C++, Rust, Go, C11, Python
and Kotlin, with no row for any of them to differ in — is therefore
"the expression evaluates at invoke-fire time" (§scxml-6.4.3), not
"the evaluated string selects the child". The C11 template says so in
as many words,
and a failure to evaluate raises `error.execution` under one wording on
all six (`sce-build/tests/one_wording_for_an_invoke_expression_failure.rs`).

⚠ The consequence for an author is one sentence, and it is now a
choice rather than a limit: **declare `sce:candidates` and the value
selects among them; declare none and the child is the stub.**

```xml
<invoke type="scxml" srcexpr="pick"
        sce:candidates="chosen.scxml other.scxml"/>
```

With a set declared, the build generates one child per candidate and
the runtime picks by the evaluated value, matched on the document STEM
so `file:x.scxml`, `./x.scxml` and an absolute path all name one child.
A value naming none of them raises `error.execution` — the same answer
the Interpreter gives when a document will not load. Three refusals
guard the declaration at build time: beside a `contentexpr` (which
PRODUCES a document rather than naming one, so there is no finite set),
two entries sharing a stem (two documents claiming one answer), and an
attribute written and left empty.

Without a set the residue stands: the child is fixed at build time and
a child reached through `srcexpr` **does nothing**. That is the right
default rather than a gap — a build cannot know what an expression will
compute, and inventing a candidate set would be the generator guessing.
`--deploy` changes none of this; it writes the same stub.

How the child was named decides which document runs; it does not change
what the invoke's **arguments** are. A hybrid invoke's `namelist` and
`<param>`s are evaluated in the invoking session exactly as a static
invoke's are (§scxml-6.4.1): an unreadable `namelist` name is one
`error.execution` and no child, a failing `<param>` is reported and its
pair left out, and a value reaches the child only under a name the
CHOSEN candidate's own top-level `<data>` declares (§scxml-6.4.3). The
arguments are read after the value has chosen a candidate, so a value
naming none of them reads nothing — the Interpreter's order, which
loads the document first. Measured 2026-09-28: every AOT channel
evaluated none of them, the parser did not keep a hybrid invoke's
`namelist` at all, and each candidate was started with the invoke-level
child metadata, which a hybrid invoke never populates. The witness is
`integration_resources/a_hybrid_invoke_carries_its_arguments/`, whose
two candidates declare different names so a filter by the wrong one
leaks a name into the child the value chose.

**A hybrid `<invoke>` under `datamodel="sce-static"`** is the same declaration,
typed. A machine of this model has no script engine, so what the declaration
promises is held to what a machine can compute from its own fields, and what it
cannot is refused where it is written (`scxml/static-datamodel-rule`):

- `srcexpr` is a string the machine computes from its fields when the
  invocation starts. One that cannot be computed — a checked integer
  operation that overflows — is an attribute that cannot be evaluated:
  `error.execution`, and nothing starts. A `srcexpr` that is not a string is
  refused.
- `sce:candidates` is required. A hybrid invoke that declares none has no
  finite set of children to lower, and one whose `<content expr>` PRODUCES the
  child's document has none either; each is refused, and a document that
  must start a child by its text names it in `src` or in-line.
- Every candidate is a `datamodel="sce-static"` document this build read,
  beside the document, because a value has to arrive in a typed variable of
  it.
- The value names a candidate by the STEM of the document it names: the text
  after the last `/` or `\`, without a `file:` scheme, without the extension
  after its last `.`. The rule is one for every engine and for the build, which
  reads each declared candidate by it (`tests/document_stem/document_stem.json`
  is the table each is held to), so `file:x.scxml`, `./x.scxml` and an
  absolute path name one candidate and a value naming none of them is
  `error.execution`.
- The invoke's `<param>`s and `namelist` go to EVERY candidate, and each keeps
  what it declares a variable for (§scxml-6.4.3), typed against THAT
  candidate's variable. A candidate that declares none for a name evaluates the
  argument and leaves it out, and an argument that cannot be evaluated is
  reported (§scxml-5.7.1) whichever candidate was chosen: an argument is the
  invoke's, not the child's. A name no candidate declares would be dropped by
  every one, which is refused as a static child's is.

`static_invoke_hybrid.scxml` and its two candidates (`static_hybrid_first`,
`static_hybrid_second`) run four phases — a `file:` value, an absolute path, a
value whose `extra` cannot be held, and a document not declared — and each
`done.invoke` adds a power of ten, so the sum and the two errors say which
candidates ended and what was reported. Rust, Go, Kotlin, Python, C++ and C11
drive it (`a_static_hybrid_invoke_starts_the_candidate_its_value_names` in each),
and each is held to the stem table.

A machine that holds a hybrid `<invoke>` has the save API (§2.15, "Saving and
restoring"). A saved state names the invocation by its id and not the candidate
that was running, and a restore starts it again as entering the state would: the
start reads `srcexpr` from the restored variables and hands the arguments'
current values to every candidate. Rust and Kotlin, the backends that have the
API, drive it (`a_static_hybrid_invoke_is_started_again_by_a_restore` in each).

The Interpreter runs one once it is lowered (§2.15, "It runs once lowered"):
`sce-codegen lower --out-dir` writes the document and each candidate, lowered,
into one directory, the `srcexpr` becomes a call of the library's `candidate`,
which reduces the value to its stem by the same table and answers the file name
the lowered candidate is written under, and the Interpreter loads that file from
beside the invoking one. A value naming no declared document throws, so the
attribute cannot be evaluated: `error.execution`, and nothing starts.
`static_invoke_hybrid.scxml` runs under it as on the others, to 111 and two
errors (`AHybridInvokeStartsTheCandidateItsValueNames`).

The runtime witness for the selection itself is
`integration_resources/invoke_candidate_selects_the_child/`, driven on
all seven channels. Its two candidates announce themselves differently,
so the right child, the wrong child, a failure to load and a stub each
rest in a different state — the discriminator the note below says no
fixture had.

⚠⚠ Two of the six did not hold that contract when it was written down,
and both are recorded here rather than quietly repaired:
**Python** evaluated nothing at all, so a failing expression raised
nothing and the child started as though the document were fine.
**Kotlin** resolved the value into a document through
`ScxmlRuntimeInterpreter` — an interpreter fallback inside an AOT
backend, which ARCHITECTURE.md forbids, and one that could not ship:
that class lives in this repository's Kotlin **test** module, so every
generated file carrying a hybrid invoke imported a symbol a consumer's
runtime does not have and did not compile for them. It now spawns the
same stub as the rest.

⚠⚠⚠ What Kotlin lost when it moved onto the stub — it was the one AOT
channel whose child was the document the expression named — is what
`sce:candidates` gives back, and to all six rather than to Kotlin
alone. The sequence was wrong, and saying so is the point: the
capability was removed before its replacement existed, so the product
did less for the length of that gap. The replacement is not a
Kotlin-shaped patch; it is the build knowing a set and the runtime
choosing from it, which is why every backend has it now.

⚠⚠ No fixture's oracle can currently tell a stub from the named
document, which is why the divergence above could persist unremarked.
Conformance fixture 216 exists to prove that a `srcexpr` is evaluated
at runtime rather than at parse time — its own comment says the
invocation "will fail" if the pre-assignment value is used — but the
document it names is itself an immediate `<final>`, so a stub and the
real child are indistinguishable by `done.invoke` alone. What 216
pins on the static path is that evaluation happens and that
`done.invoke` arrives in order. What it cannot pin is the thing it was
written to test.

### §2.14 Names the generated code spells

A name an `sce:` element declares or refers to — a codec field, an
import alias, an enum variant, an algorithm variable, a native action —
and a forge document's `<data id>` reach generated source verbatim, in
C, C++, Kotlin, Rust, Go and Python, and forge expressions read them by
name. Each is held to one grammar, the **code identifier**: an ASCII
letter or `_`, then ASCII letters, digits or `_`. A reference through a
record (`header.S`, `telemetry.reset`) is code identifiers joined by
`.`. A violation is refused at parse, on the attribute's own line, as
`validation/malformed-code-identifier`.

A code identifier of that shape is also refused when it is a word one of
the six languages reserves, as that language spells that kind of name:
`override` (Rust), `pass` (Python), `object` (Kotlin), `auto` (C++) for a
`<data id>`, and `self` for a variant, which Rust spells `Self`. The
spelling differs by kind and by backend — a const is `UPPER_SNAKE`
everywhere, so a const `default` is `DEFAULT` and is accepted; a variant
is `Pascal` in Rust and C++, so a variant `match` is `Match` and is
accepted — and a name that never becomes an identifier (a reference, a
`cycle` id, a `fold` binding) is not asked at all. It is refused for
every backend at once, as `validation/reserved-code-identifier` naming the
language and the spelling, rather than escaped per backend: these names
are spelled at many sites per kind, and an escape at some of them is a
mismatch at the rest. A statechart's
`<data id>` is not held to this; where it becomes a typed reader, the
reader is escaped or withheld instead (`unreadable_variables`,
`SCE_ERROR_CONTRACT.md` §10.1).

Python has a second kind of refusal, for a name that is no keyword and
still cannot be the author's. A generated Python decoder is one function
that reads from `cursor`, calls `bytes(…)` and `range(…)`, builds its
result with `cls(…)` and binds a local per field in that scope, so an
author's names and the generator's own met there: a field called `bytes`
made `bytes = bytes(raw)` an `UnboundLocalError`, one called `cursor`
replaced the cursor, one called `cls` made `cls(…)` a call on an integer —
accepted, generated, and wrong. The decoder now binds a field's value under
`f_<field>`, a prefix nothing the generator writes begins with, so a field
called `len`, `bytes`, `list`, `body`, `value` or `raw` works and nothing
about it is refused. What cannot be kept apart that way is a name that is
itself the public name: a codec field is the dataclass attribute, a flag
its accessor, a flag-input a parameter of `decode` and `encode`. Those are
refused, as `validation/reserved-code-identifier` with the reason in the
message, when the generated class or call already uses the name for
something else: `decode`, `encode` and `encode_to_bytes` (methods every
generated codec class defines), `classmethod`, `field` and `list` (what the
class body evaluates while it is built), any `__x__` name, and for a
flag-input `cls`, `cursor`, `self`, `w`, `tag`, `parent_flags` or a name
beginning `f_`. The lists are not written by hand:
`a_python_codec_keeps_an_authors_names_apart_from_its_own` derives each
from the committed Python output and fails when a template change makes one
stale, and runs every Python builtin and every name that output uses as a
field of each shape that binds a local.

Go was measured, not assumed. It writes a codec field and its decode local
Pascal and keeps its own locals lower case, so the Python defect was not
expected there; but a Go struct holds a field and a method in one table, and
every codec struct carries `Encode` and `EncodeToBytes`. The names are derived
as the Python oracle derives them — every identifier that at least three
committed Go outputs use, each as written and in the snake_case an author
would write for it, joined with Go's universe scope and its keywords — and
`a_go_codec_keeps_an_authors_names_apart_from_its_own` gives each to a field of
the same eleven shapes, builds every generated package, runs the ones that
build (a frame decoded and encoded back, the field read back through
reflection) and finds none that breaks: 431 names, 35 refused in the document,
4411 generated modules. What stops `Encode` and `EncodeToBytes` is the
method list above, which is compared as snake_case, so `encode` and `Encode`
are one name to it; with `encode` taken off the list, 22 renamings fail with
`field and method with the same name Encode`, which is the oracle shown able to
fail.

The other forge kinds whose Python is a function — algorithm, condition,
filter, interpolation, lookup, observer, transform, validator — meet the
same question with the author's names as the function's own locals, and
refuse nothing for it. A name that the function's code would otherwise hide
is spelled with a trailing `_` (PEP 8's spelling), and only that name: an
algorithm variable called `len` is `len_`, so `n = len(data)` still calls the
builtin; a validator input called `abs` is `abs_`, and `delta` no longer meets
the template's own temporary, which is `_delta` now. The escape is a shift, so
an author's own `len_` is `len__` and two names never meet at one spelling.
One function spells the declaration, every read and every call, so a name
cannot be escaped in one place and not another — an interpolation input
written `engineRpm` was declared `engine_rpm` and read as `engineRpm`, a
`NameError`, until its body was spelled by the same function. A record field
of a transform's outputs is an attribute and is not escaped: a host reading
`outputs.abs` by name still finds it. The names escaped are
`PYTHON_GENERATED_NAMES` in `sce-build/src/forge/generator.rs`, and
`a_python_kind_keeps_an_authors_names_apart_from_its_own` runs every document
of those kinds with each name it declares renamed to each builtin and each
name the kind's committed Python uses, and compares what the generated module
does — about 15 000 renamings — and a second test fails when the committed
output starts to use a name that list does not carry.

The same kinds in Go were measured the same way and were not safe. A Go
function writes an author's name as written, in the scope of the predeclared
names its body calls, so `a_go_kind_keeps_an_authors_names_apart_from_its_own`
renames every name each document declares to every predeclared identifier,
every package a generated file imports and every name the kind's committed Go
uses, builds the result, and its first run found 1 982 of about 14 000 accepted
renamings that did not compile. The causes were four. An `<sce:param name>` was
the one declaration of the function that was not held to the reserved-word
check, so a parameter called `for` or `func` was accepted and generated; it is
refused now, as a variable is. A local that hides a predeclared name or a
package the body names (`len`, `uint16`, `math`, `lookup`) is spelled with a
trailing `_`, by one function (`go_local_spelling`) for the declaration, every
read and every call, as a shift, so an author's own `len_` is `len__` and two
names never meet. What the generator itself declares inside a function — the
receiver, the queue of an observer, the temporary of a rate-of-change check —
is under the prefix `sce` (`sceSelf`, `sceEvents`, `sceDelta`), and so are the
package-level tables (`sceKeys`, `sceValues`, `sceAxisRpm`), which an author's
`values` or `axisRpm` used to hide or redeclare; a name that begins `sce` is
shifted off them. And the names a file declares at the package level at a
fixed spelling (`ValidationResult`, `ForgeDomainTag` and its constants, a
lookup's `<Output><Value>` constants) are kept off the locals that read them,
and an observer monitor called `Update` is a struct field beside the method of
that name, so it is spelled `Update_`. A fifth cause appeared when the oracle
was taught to write the sibling packages a document imports beside it, each
generated from its own document under its own name, so that the documents that
import one are renamed like the rest: an input, a variable or a parameter
called `condition_threshold` hid the package a condition of that name was
generated into, and a call through it was a call on a float (`imported and not
used`, or `not enough arguments`). The package's name is the imported
document's, which the author of the importing one never chose, so a STATELESS
import (the kinds called as a function in a body) is imported under
`sce_<package>`, a name that begins with the generator's prefix and which the
shift keeps every author name off. A stateful one is held as a member and used
by its type at the package level, and a codec reaches its sibling through the
package's own name in a dozen places that are not names an author chooses, so
it keeps the name. The final run built 17 640 accepted renamings of 75
documents (2 711 refused in the document, 47 left out because two of the
author's own names fold to one) and none failed to compile; 10 documents are
not renamed because their own text is refused (the fixtures refused on purpose
and the ones that need an option a bare run does not give). Two tests derive
the lists rather than hold them: the predeclared names are read
from `go doc -all builtin`, and the packages from the import blocks of the
committed Go, so a Go release that adds a builtin or a template that imports
one more package fails with the name in the message.

The oracle builds, it does not run, and it counts a renaming as built only
when the package was in the build: a file name that ends in `_386` is a
build constraint to Go, `go build ./...` leaves it out without a word, and the
case numbered 386 had been counted as built without being compiled, which
`go list ./...` now rules out, and a sibling that does not build is reported as
the module's defect, since `go build` reports it and leaves its importers
uncompiled and unreported.

Building the controls first found two documents whose unrenamed text did not
build as Go, and both were defects of their own and not of a name. A transform
of an `int8` carrying `sce:quantity`, read as `raw * 0.5`, lost the conversion
the same body without the annotation carries: the emitters' coercion did not
see through a quantity to its numeric base, so Go wrote an integer times a
float (`0.5 truncated to int8`) and Rust the same (`i8 * 0.5`). A quantity is
now emitted as its numeric base, on the operand and on the context it is
asked for. And an observer that imports a condition emitted the alias it was
given (`isOverheat`) and no import, in every backend: its expression map was
empty. It resolves a stateless import as a validator does now, and the
committed outputs of that document are pinned in five backends. What an
observer does not do is hold a STATEFUL import (a filter, a codec): it keeps a
threshold state per monitor and no member for another kind's state, and the
import used to be accepted and the member it names left undeclared, in every
backend. It is refused, in all six, as `generate/unsupported-feature` naming
the observer, the import and why.

Python has the fault the Go oracle found in a stateless import, and the Python
oracle found it when it was taught to run the documents that import a sibling
(it loads each module as a member of the package its directory is, with the
siblings it imports beside it, instead of skipping them): an input called
`condition_threshold` replaced the module of that name for the rest of the
function, and the call through it was an `AttributeError`. A stateless import is
brought in as `from . import condition_threshold as sce_condition_threshold`
and called through the alias (`python_module_alias`), and a name that begins
`sce_` is shifted off it as every other name the generator keeps is. A stateful
one (`from .simple_codec import SimpleCodec`) is used by its type and keeps its
spelling. It ran 74 of 85 documents and 28 673 renamings and found nothing else
that a name decides, besides the two states above.

C is the backend where a name is likeliest to decide, because a function, a
variable and a typedef name share one namespace per scope and the headers the
generated C includes bring in names of all three. `a_c_kind_keeps_an_authors_names_apart_from_its_own`
compiles every renaming of the eight kinds as a translation unit
(`-std=c11 -Wall -Wextra -Werror -fsyntax-only`, the contract the generated
headers are held to), with the sibling headers a document includes beside it,
and its first run found 325 of 21 929 accepted renamings that did not build.
Four causes. A parameter or variable called like a library type or function
(`uint32_t`, `size_t`, `strcmp`, `floor`) was a declaration in the scope of the
body that used it. The generator's own locals were plain names an author could
equally choose (`x`, `tx`, `v0`, `c0`, `sum`, `count`, `out`, `delta_`, `self`,
`events`), redeclared in the same scope as an author's parameter. A name under a
prefix the generator already used (`sce_failure_`, `sce_cond1_`, `_st`, `_i`)
was one an author could equally choose. And a local called like the function of
an imported document hid it. The generator's locals are now under `sce_`, and an
author's name is spelled by one function (`c11_local_spelling`) for the
declaration, every read and every call, as the same shift the other backends
use: a name that begins `sce_` or `_`, or is one of `C11_RESERVED_NAMES`, gets
one more trailing `_`, so no two names meet. `C11_RESERVED_NAMES` is derived,
and a second test fails with the name when the committed C starts to use a
library name it does not carry: it preprocesses the library headers the committed
C includes and takes every macro, typedef and function they declare. The final
run built 22 060 accepted renamings and none failed. What is left out is stated:
C cannot bring a function in under another name, so an author's name that is the
function of a document it imports (`condition_threshold_check`) hides it, and 23
renamings of that kind are counted and not asked. Go and Python bring such a
function in under an alias and call through it, which is why they do not have it.

C++ keeps most names apart by scope, since an imported function is called through
its namespace (`SCE::Generated::X::f`) and a class member is looked up in its
class, so no name is left out as in C. `a_cpp_kind_keeps_an_authors_names_apart_from_its_own`
compiles every renaming of the eight kinds as a translation unit
(`-std=c++20 -Wall -Wextra -Werror -fsyntax-only`; C++20 because a `bytes`
parameter is a `std::span`), and what it found is four causes, all of the same
kind: a name the generator writes itself that an author could equally choose.
A parameter or datum called like a library type the body names after it
(`int32_t`, `size_t`) or like the generated `ForgeDomain` was a declaration in
the scope of that use. The generator's own locals were plain names (`events` in
an observer, `delta` in a validator's rate-of-change check, the `KEYS` and
`VALUES` of a lookup, the axis tables and `VALUES` member of an interpolation)
and are now under `sce_`. A name ending in `_` is the spelling of a member of a
generated class (`alarm_`, `smoother_`, `impl_`), so an input called that read
the member's `double` or `bool` instead of itself. And a name that begins `SCE_`
was a macro of an include guard (`SCE_FORGE_…_H`). An author's name is spelled
by one function (`cpp_local_spelling`) for the declaration, every read and every
call, as the same shift the other backends use: a name whose stem is one of
`CPP_RESERVED_NAMES`, or begins `sce_` or `SCE_`, or ends in `_`, gets one more
trailing `_`, so no two names meet. The final run built 18 917 accepted renamings
and none failed; 2 993 more were refused by the generator, 10 are left out
because two of the author's own names in the document fold to one, and 10
documents are not renamed because the generator refuses them unrenamed. Taking
`int32_t` out of the reserved list, and the trailing `_` out of the shift,
each turned the run red (39 renamings that did not build). C has a second test
that reads the library headers' own declarations; C++ has none, because a C++
header declares mostly members and namespace members that no local meets, and a
derivation that cannot tell those apart would need a list of exceptions. The
oracle's candidates are every identifier the committed C++ writes, so a template
that starts to write one more library name bare is asked about it.

Rust keeps a local, a type, a module and a field in namespaces of their own, and
an imported document's function is called through its module, so far fewer names
decide anything there, and what it found is four faults.
`a_rust_kind_keeps_an_authors_names_apart_from_its_own` compiles every renaming
of the eight kinds as a module of a crate (`--edition=2021 --crate-type=lib -D
warnings`) against the two runtime crates a generated file names, built once from
this tree with the workspace's lock file. The first run built 15 723 accepted
renamings and 103 did not; the last built 15 763 and none failed. A function the
file brings in with `use`
(`lookup`, `linear`, `bilinear`) was hidden by an input of that name, so each is
imported under the generator's prefix (`use … as sce_lookup`), and the
observer's own queue is `sce_events`, so an input called `events` no longer
reads it. An author's name is spelled by one function (`rust_local_spelling`) for
the declaration, every read and every call: snake_case, which the compiler holds
a binding to where warnings are denied, with one more trailing `_` when that
begins `sce_`. Two faults were not about a collision at all. An interpolation
declared its inputs snake_case in the signature and read them as the author wrote
them in the body, so any camelCase input was `cannot find value`; the body reads
the name the signature declared. And an observer wrote each monitor as a struct
field as written, so `coolantTemp` was a `non_snake_case` warning, an error where
warnings are denied (66 of the renamings). It is now snake_case like every other
name an author gives, which makes two monitors that fold to one field a refusal:
`overHeat` beside `over_heat` is `validation/colliding-code-identifier`, as
`observer-monitor` in the table. Three changes at once (the shift, the monitor
field and the interpolation body each undone) turned the oracle red with 85
renamings that did not build, in those three classes.

Kotlin has the most to hide, because a class in qualifier position wins over a
local of the same name: an input called `UByte` is read as the type's companion
in `UByte.toInt()`, and one called `ForgeDomainTag`, or like a sibling document's
class, does not reach the object it names. The classes in scope are every one of
the standard library's and the platform's, so no list of them is complete and
the rule is the shape and not the list. `kotlin_local_spelling` shifts a name
that begins with a capital, the package roots a call is written through (`com`,
`kotlin`) and the generator's own prefix, for the declaration, every read and
every call. The generator's own names follow it: the observer's queue is
`sceEvents`, a validator's kept value is `scePrev<Name>` (an input called
`prevSpeed` beside the member for `speed` was assigned to), and the signed
element a bytes loop iterates is `sceRaw_<item>`. A lookup's enum is named for its
output, so an output called `uByte` was the enum `UByte` and stood in for the
type of the input in the same file: a type the generator declares from an
author's id is kept off the standard type names (`kotlin_declared_type_name`,
with `KOTLIN_TYPE_NAMES` pinned to the type mapping), and an interpolation read
each input as the author wrote it where the signature spelled it, as the Rust
one did. `a_kotlin_kind_keeps_an_authors_names_apart_from_its_own` compiles the
renamings against the forge runtime built once from this tree, hundreds in a run
because `kotlinc` is a JVM that takes seconds to start, and tells each error back
to the renaming whose file it names. The first run built 15 456 accepted
renamings and 73 did not; the last built 15 863 and none failed. Two controls
are not renamed because they do not build alone: one reads a record type the host
supplies and one is a test file that imports `kotlin.test`. A third did not build
either, for a reason that is not a name and is fixed: a scalar `<sce:const>` of an
unsigned type was written `const val POLY: UShort = (4129).toUShort()`, a call and
so not a constant, and was read in `word.toInt() and POLY`, an `Int` beside a
`UShort`, because a scalar constant was typed `Unknown` and so never converted. It
is now an unsigned literal (`4129u`) typed by its declared type like a parameter,
and `a_kotlin_scalar_const_of_an_unsigned_type_compiles` compiles it, which no
committed output did.

What each oracle asks was widened when Kotlin showed what a list of committed
outputs leaves out. The candidates of a renaming are the identifiers of the
committed outputs of its kind and of the language, and now also every identifier
the generator writes for the document itself, read from its own unrenamed output:
a name the generator derives from a document's names (a member, a loop index) is
asked about whether or not a committed output still spells it, and a document
with no committed output is asked at all. That found names every backend had
written plainly. The loop over a bounded collection used `slot_idx` (`slotIdx`)
and an item-derived `<item>_ptr` or `<item>_opt`, now under the generator's
prefix, and Go reads the collection's bound through its `Capacity()` method and
no longer through the package-qualified constant, which a parameter named like the
package hid. A transform that reads `previous()` calls a function named from each
output (`computeDelta`) from a method that has the inputs as parameters, and an
input of that name hid it: C++ and Rust qualify the call, and Python and Go call
it through a name under the prefix. C reads library names too (`llround`,
`fprintf`, `stderr`) that no committed output holds, so its pin reads the
generated output of every document, and a C name that ends `_t` is shifted whole,
because every typedef the generated C declares is `<name>_t` and the suffix is
POSIX's. Three kinds of name that no keyword list holds were accepted and read as
something else, and are refused for every backend like a keyword: `_` (the
wildcard in Rust, an unnamed parameter in Kotlin, the blank identifier in Go), a
Python `__x__` name (`__init__` as a field replaced the constructor, `__import__`
as a parameter replaced the builtin), and a forge procedure's state or final id
that is a keyword once spelled as the enum member it becomes (`false` is Python's
`False`). A Python helper is written bare where it is called, so its name is
checked as Python writes it. The Python oracle now reports a case whose source
does not parse as a case, where it stopped the run without saying which.

A procedure and a timer are classes, and an author's names meet different
names there. A procedure stores each input, internal and helper as
`self._<name>` on a subclass of `ProcedureStateMachine`, so an input called
`is_final` hid the method `run_to_completion` calls (`self._is_final(state)`,
a call on a bool), one called `done_data` replaced the dictionary a `donedata`
writes and one called `service_handler` replaced the handler. The storage is
private, so it is spelled under the same shift as a local (`is_final_`) when the
name is one the class or its base owns (`PYTHON_PROCEDURE_MEMBERS`), and
nothing is refused for it. The generated `execute(handler, …)` wrapper binds
`handler` and `sm` itself, so an input called either was a duplicate argument
or was overwritten by the machine, and its parameters are spelled under the
same shift (`PYTHON_PROCEDURE_WRAPPER_NAMES`). One name cannot be kept apart:
each input and helper has a public setter `set_<name>`, and the base class has
`set_service_handler`, which the wrapper calls to hand the service handler
over, so an input or helper of that name is refused
(`validation/reserved-code-identifier`, the reason in the message). A timer
spells its events and its state into method names behind a prefix
(`fire_<event>`, `on_reset_<event>`, `on_cancel_<state>_exit`), where nothing
of the class's own can meet them, and the same oracle runs it against a
recording timer and handler and compares what they were asked. A transition's
`ok` and `fail` are not names an author chooses: they are the two answers a
service gives, which the generated code wires to `Event.Ok` and `Event.Fail`.
The three lists are read off the committed output and the runtime's base class
by `the_names_a_generated_procedure_keeps_for_itself_are_the_ones_its_output_shows`.

Two of an author's own names that a backend spells as one are a different
fault: `minRpm` beside `min_rpm` is two names in the document and one
parameter to the generated Python, which the interpreter refuses as a
duplicate argument, one field in Rust and a codec decoder that reads two
fields into one local. They are refused for every backend at once, as
`validation/colliding-code-identifier`, on the later declaration's own
attribute, naming both names and every backend that folds them — the same
narrowing as a reserved word, and for the same reason: whether a document is
accepted must not depend on which backend a deployment builds. Two names are
compared only where the generated code puts them in one namespace, so `Foo`
beside `foo` is refused as two codec fields (Rust, Python, C11 and Go spell
them alike) and a codec member beside a flag's accessor only in C++, Go and
Python, where a class holds both in one table; Kotlin and C++ write a codec
member as written, so they fold nothing and are not named in a message that
they do not cause. What is compared is `SCOPES` in
`sce-build/src/forge/declared_names.rs`: a forge document's input (its
`<data id>` where `sce:direction="in"`, for the kinds that spell it
snake_case; an algorithm's `<sce:param>`), a codec's members and its flag
accessors and inputs, a const, and a procedure's data of every direction beside
its helpers: `seedKey` and a helper `seed_key` were one `self._seed_key` in
Python, so the helper replaced the datum and `seed_key(seedKey)` called the
helper with itself, and one field in Rust and one member in C11. A procedure's
states are the members of one enum, spelled Pascal in Rust, C++, Kotlin and
Python, so `init` and `init__` are both `Init`: Python refuses the class at
import (`TypeError: 'Init' already defined`) and the others declare one variant
twice. The Python oracle found it when it began to run the documents that
import a sibling, as a rename of a state to `init__`. An observer's monitors are
fields of one struct, which Rust spells snake_case, so `overHeat` beside
`over_heat` is one field there; C11 spells its flag `<snake>_active`, which is
the same fold behind a suffix the measurement cannot see, and the refusal is for
every backend all the same.
`a_forge_declaration_is_spelled_the_way_the_collision_rule_says` holds the
table to the templates: it renames every declaration of every committed forge
document and reads the name back in each backend, so a convention that
changes fails there.

What the rule does not compare is stated, not assumed safe. An output, an
internal (a procedure's excepted) and a lookup's output type are spelled by
the role they play and are not claimed; a `<sce:var>` is function-local and two of them in different
blocks are legitimately two; a C++ embed is a member written as written and a
decode local in snake_case, which is two spellings and compares as neither; a
codec's field local in a Rust decoder and a flag input of the same spelling
share a function and are not compared. Measured with
`measure_the_spelling_of_every_declaring_attribute`, which is how a row is
added.

The document's own name reaches generated Python too. The first line of a
module the library returns is the source-map marker, `# SCE-MAP:
<document>:<line>`, and a name ending in `coding` (`zenoh_encoding`,
`transcoding`) made it `…coding:23`, which PEP 263 reads as the declaration of
an encoding called `23`: the file was a `SyntaxError: unknown encoding` to
every import, and invisible to anything that compiles the text as a string,
which ignores the declaration. A module whose first two lines the interpreter
would read as an encoding declaration is now headed `# -*- coding: utf-8 -*-`,
which is what the templates emit; every other module is byte for byte as it
was. `a_python_module_is_never_read_as_declaring_an_encoding` imports the
modules of such names and reads every committed Python golden the way the
interpreter reads a file; the one it found was the committed golden of
`codec_zenoh_encoding`, unreadable for as long as it had been pinned as text.

This is the narrowing §1 draws for W3C's identifiers, drawn for the
names SCE owns, and it is narrower than an XML Name in the same
direction for the same reason: `-` and `.` are operators in every target
language and in the forge expression language, and non-ASCII letters are
not spelled alike by all six. A statechart's `<data id>` stays §1's
`xs:ID`; a forge document's is a code identifier because its own
expressions and generated code use it as one. Which attributes are
covered is `SCE_IDENTIFIER_ATTRIBUTES` in
`sce-build/src/scxml_identifier.rs`, and a name of another document — a
pool's or a framer's `ref` — is that document's name and is not among
them.

Measured 2026-09-22, before the rule existed, the forge pipeline checked
none of these: a transform whose `<data id>` was two Hangul letters
passed the parse and was refused by the expression lexer with no line,
and one whose `<data id>` was `raw-value` passed `check` in all six
languages and generated the C++ parameter `int32_t raw - value`. Over
every tracked `.scxml` document the rule refuses none.

#### §2.14.1 Names a statechart's generated code declares

A statechart's state ids, `<history>` ids and event names are not held
to the code identifier grammar — they stay §1's `xs:ID` and W3C SCXML
3.12.1's event tokens — but every backend declares a member for each of
them, in its own convention: C++ capitalises a state's first letter
(`idle` is `Idle`), Rust, Go and Kotlin write PascalCase, Python writes
`UPPER_SNAKE`, and C11 upper-cases under a machine prefix and names a
function after each state's `<onentry>` and `<onexit>` blocks in lower
case. Two names the document keeps apart can therefore become one
member. XML Names are case-sensitive, so `idle` and `Idle` are two
conforming states, and every backend spells both alike; `doorOpen` and
`door_open` are spelled alike by Rust, Go, Python and Kotlin. A name can
also become a member the generated code declares for itself: C11's
`<PREFIX><MACHINE>_STATE_COUNT`, `_HIST_NONE` and `_EVENT_NONE`, C++'s
`Event::NONE`, the eventless `Null` of Rust and Go, Python's `NULL`, and
in Kotlin the `Self` object of an event that is also the prefix of
others (`foo` beside `foo.zoo`).

Such a document is refused before any code is written, for every
backend at once, as `scxml/generated-name-collision` naming both names
and each backend that folds them. It is not a W3C violation — §3.14 is
satisfied — and it is not escaped per backend, for §2.14's reason: each
name is spelled at many sites per backend, and a rename at some of them
is a mismatch at the rest. The spelling asked is each template's own
(`sce-build/src/member_names.rs` calls the filter the template calls),
so what this rule refuses and what generation would have written cannot
disagree.

Measured 2026-09-29, before the rule existed: `idle` beside `Idle`
passed `check --lint` and `generate` with exit status 0 in all six
backends, and C++ generated `enum class State : uint8_t { Idle, Idle };`.
The Python generated for `doorOpen` beside `door_open` did not import
(`TypeError: 'DOOR_OPEN' already defined as 0`). Over every tracked
`.scxml` document the rule refuses none, and moving the C and C++
templates' spelling into the filters it calls left every byte those
documents generate unchanged.

### §2.15 Static data model — `datamodel="sce-static"`

§3.2 permits "other platform-defined values" of `datamodel`, and
`sce-static` is the one SCE defines: a statically typed data model whose
variables are native fields of the generated machine and whose
expressions are written in the forge expression language (§3.4.1), so a
document that declares it needs no script engine. `ecmascript` is
unchanged by it.

Its rules, each refused at parse as `scxml/static-datamodel-rule`, on the
line of the element or attribute that breaks it:

| Construct | Rule |
|---|---|
| `<data>` without `sce:type` | Every variable declares its type |
| `<data src>` | The initial value is `expr` — `src` is read at run time and has no type |
| `<data>` with in-line content | The initial value is `expr` — in-line content has no type |
| `<data>` without `expr` | Every variable declares its initial value; no zero, empty string or first variant stands in. A record variable's is its `<sce:set>`s, and it takes no `expr`; a list starts empty and takes `sce:capacity` instead |
| `<script>` with script text | No scripting language; a native `<script><cpp>` / `<kt>` block is admitted, as under `null` |
| `<send typeexpr>`, a `<send idlocation>` that names no string variable the id fits (see **Generated send ids** below) | No typed form: each is evaluated as script-engine text by every backend's templates |
| a `<send targetexpr>` that declares no `sce:targets`, one written beside a `target`, or a `sce:targets` that names a route the SCXML Event I/O Processor cannot address, a Mesh peer, two entries of one route, no entry, a send with no `targetexpr`, or a `type` other than the SCXML processor | See **A computed target** below. A `targetexpr` is a string expression; the routes it can take are declared, since a machine with no script engine has nothing to learn them from when the send runs. Refused at the attribute as `scxml/static-datamodel-rule` (the missing declaration and the two targets) or where `sce:targets` is written (`validation/incompatible-attributes`, `validation/empty-value`) |
| a hybrid `<invoke>` that declares no `sce:candidates`, one whose `<content expr>` produces the child's document, a candidate that is not a `sce-static` document this build read, a `srcexpr` that is not a string, or an argument no candidate declares a variable for or of another type than the candidate's variable | See **A hybrid `<invoke>`** under §2.13. Refused at the `<invoke>` (or its `<param>`) as `scxml/static-datamodel-rule`: with no declared set there is no finite list of children to lower, and an argument every candidate would drop is refused as a static invoke's is |
| `<invoke idlocation>` | Refused for want of a reader, not of a form: the id it would store is the one the build already wrote (the `id`, or `<state>.platform_N`), and this model reads no `_event.invokeid` — only `_event.data` — so a stored id has nothing to be compared with. A document that must name its invocation writes `id`, which `done.invoke.<id>` and `error.invoke.<id>` already match |
| a `<param>` or a `namelist` name of an `<invoke type="scxml">` whose child is not a `sce-static` document this build read, does not declare the name as a top-level `<data>`, declares it as a list, a record, an enum or bytes, is handed it twice, or is handed a value not of the variable's type | See **Child sessions** below. Refused at the `<param>` as `scxml/static-datamodel-rule` (a value of the wrong type as the expression's own refusal) rather than accepted and never delivered |
| a `<finalize>` of an `<invoke type="scxml">` | §6.5 runs it in the invoking machine before a child's event is processed, but the model keeps its body as one script text and the generated code hands that text to a script engine this model never builds (measured 2026-10-01: the Rust body is an empty block, Kotlin finds no engine): the assignment would be accepted and never run. Refused at the `<invoke>` as `scxml/static-datamodel-rule`; the invoking state takes what the child sent in a transition. An EMPTY `<finalize/>` beside a `<param location>` or a `namelist` is the same refusal: §6.5.2 gives it the meaning "update each from the event's data of that name", which the model writes out as that script text. Lowering a body is not the obstacle — a `<finalize>` runs before any child event is processed, to read that event's `_event.data`, and no type rule reaches a payload that arrives from whichever event comes next; a body that reads none has no consumer. Under `ecmascript` the same document runs it |
| a `<param>` of a `<send>`, of a host-run `<invoke>` or of a `<donedata>` whose value is not a bool, a string, a byte string, an integer of at most 32 bits, a real or an enum value held by a variable, a field of a record variable or a field of the payload, or reads a payload that is not in scope | See **Params** below. Refused at the `<param>` as `scxml/static-datamodel-rule`. An `<invoke>` typed by `sce:request` takes only literals |

**Expressions.** Every other expression is a forge expression judged
against one closed scope — the declared variables at their `sce:type`, each
record variable's `<id>.<field>`, the triggering event's
`_event.data.<field>` when the event carries an imported schema, the
imported enums, and `In(<state id>)` — gathered once by
`crate::forge::type_ctx::StaticScope`, which validation, the host-action
check and the lowering all read, and which shares its payload registration
with the typed guard path. Each is judged against the
place it lands in: a `<data expr>` and an `<assign expr>` against the
variable's type (the `location` must name a declared variable), a
transition's or `<if>`/`<elseif>`'s `cond` as `bool`, a `<log expr>` as
whatever it is, a `<param expr>` as one of the values a param carries (below).
A name the scope does not carry, and a
value of a kind the place does not admit, are refused at the expression's
own range with the expression layer's codes (`expression/unknown-identifier`,
`expression/type-mismatch`, …). The ECMAScript frontend is never asked to
lower a `sce-static` document's expressions.

**An enum field of a payload.** An event whose schema declares an enum field
(`<data id="layout" sce:type="enum:ViewMode" sce:direction="in"/>`) is read as
`_event.data.layout`, a value of the enum as a variable of it is: compared with `===`
and `!==` to a variant or to another value of the same enum, stored in a variable
of it, taking no number and no arithmetic. It travels as the variant's declared
name, a JSON string, as a saved state holds it, and a name the enum does not
declare — or a value that is no text — does not fit the schema: the delivery is
refused as any payload that does not is, and writes nothing. The typed channel holds
the field in the machine's own type for the enum, so the document imports the enum
under the alias the schema writes, as it does for a record's field; one that does not
is refused naming the alias (`generate/unsupported-feature`, "this document does not
import it under that alias"). Rust, Go, Kotlin, Python, C++ and C11 lift the field
into that type and write it back as the variant's declared name (Rust's `match`, Go's
function literal over the name, Kotlin's `declaredName`, Python's `sce_name`, C++'s
`sceLogName` and C11's `strcmp` chain over a buffer as long as the longest name, with
the wire writer's `switch`); the Interpreter reads it through the library's `field`,
given the variants the enum declares. C11 declares the enum before the payload channel
in the header, since a struct names a type only after it is declared.
`static_payload_enum` holds every backend to it, `a_payload_holds_an_enum_field.rs`
holds what the judge knows of it, and
`a_payload_enum_field_is_held_in_the_machines_own_enum_on_every_backend_that_lowers_it`
reads the generated machines. Measured 2026-10-04, Rust, Go, Kotlin, Python and C++
had stopped on such a read with a panic (exit 101, "reached a context built by
`LangCtx::primitive`") and only C refused; the payload structs of a machine that
holds an enum derive no `Default` in Rust, since the enum has no value to start from.
A conditional whose branches are a payload's enum field and a variant has no type Go
can name and is refused there as it was for any value of no type.

**Params.** A `<param>` of a `<send>`, of an `<invoke>` the host runs
(§2.12) and of a `<final>`'s `<donedata>` is a typed expression read from the
machine's fields at the moment the element runs — W3C SCXML 6.2.3 evaluates a
`<send>`'s arguments once, at the send, a start of an invoke is the same
instant, and a `<final>`'s donedata is evaluated as the state is entered (5.5)
— and lowered to native
code, so it needs no script engine and reads the value the field holds now,
which a copy kept anywhere else would not. A `location` names a variable and is
read as `expr="<variable>"`. A `<send>`'s `namelist` is the `<param name="x"
expr="x"/>` of each name in it, after the `<param>`s the element writes, held to
the same rule: a name no variable declares, or a variable of a type that has no
spelling below, is refused at the attribute. The Interpreter's lowering leaves
the attribute as written and its own element reads the names
(`scenarios/static_send_namelist.json`, on the six generated backends and the
Interpreter). The `namelist` of an `<invoke>` a host runs is the same: the host
is handed the pairs of its names beside the `<param>`s, and the generator writes
the same code for `namelist="a b"` as for the two `<param>`s it abbreviates
(`a_host_run_invoke_namelist_is_the_params_it_abbreviates_and_needs_no_engine`,
and `statechart_static_host_params`, whose invoke names `delta` and `ratio` this
way, on the Rust, Go, Kotlin, Python and C11 machines). C11 refuses a name that
a `<param>` of the same invoke already names, as it does for two `<param>`s; a
request typed by `sce:request` takes no `namelist` under any data model. The
`srcexpr` of that `<invoke>` is a string the machine computes from its fields
when the invocation starts, and it is the `src` the host is handed; it takes no
`src` beside it, a machine with one needs no script engine, and C11 writes an
expression that joins text into a buffer the data model sizes, as it does for a
delay (below). A saved
state holds the request as it was handed, so a restore starts the invocation
again from the `src` it was saved with and never computes it a second time
(`statechart_static_host_invoke`, whose invoke reads its `src` from the string
`place`, on the Rust, Kotlin and C11 machines). A host-run `<invoke>`'s
`<content expr>` is the body the service runs, not event data, so it is a string
the machine computes from its fields when the invocation starts, as the `src`
is, and it is the `content` the host is handed as the text it is — a value of any
other type is refused at the attribute. An invocation whose body cannot be
computed is an attribute that cannot be evaluated (§scxml-6.4.1): `error.execution`
is raised and nothing starts, the host is never asked, and a machine with one
needs no script engine. C11 writes an expression that joins text into a buffer
the data model sizes, as it does in a `srcexpr`; the saved request already holds the `content`, so
a restore starts the invocation with the body it was saved with
(`statechart_static_host_params`, whose invoke reads its body from `label` through
a condition over `huge`). A
`<send>`'s `<content expr>` that names a record — a record
variable, the item of a `<foreach>` over a list of records, or the payload of the
event the transition is on (`_event.data`) — is the `<param name="f"
expr="record.f"/>` of each field the record's schema declares, held to the same
rule; it takes no `<param>` or `namelist` beside it (§scxml-6.2.4: the element
carries its data one way). Any other expression is the one value the event
carries, whole: it is held to the rule a `<param>`'s value is (a bool, a string, an
integer of at most 32 bits, a real or an enum value, read from the machine's fields
when the send runs; anything else is refused at the attribute, naming the
`<content expr>`), and it is the event's data as the JSON the value is (`8`,
`"busy"`) and, in the request a host serves, the `content` as the text it is (`8`,
`busy`), both from the one reading. One that cannot be computed is the evaluation
that failed (§scxml-5.6.2): `error.execution` is raised and the empty string is the
content's value, so the message still goes, with `""` as its data and no content,
and the block ends after it as it does for a `<param>` (`statechart_static_host_params`,
whose `lost` is `count * 2000000000`). A machine whose only data is such a value
needs no script engine. The Interpreter reads a record as the object it holds
(`scenarios/static_send_content.json`, on the six generated backends and the
Interpreter) and a value as the data it is, its expression lowered in the attribute
it is written in. A `<final>`'s `<donedata><content expr>` is the same, read as the
state is entered, where no event's payload is in scope (so `_event.data` is refused
there): a record variable is the pairs of its fields
(`scenarios/static_donedata_record.json`), and any other expression is the one value
the done event carries, whole, held to the same rule — `8`, `"tally"`, or, for one that
cannot be computed, `""` with `error.execution` raised
(`scenarios/static_donedata_content_value.json`, `static_donedata_content_text.json`
and `static_donedata_content_lost.json`, on the six generated backends and the
Interpreter). A `<send>`'s
`delayexpr` is a string, the CSS2 time the delay is written in (`wait + 'ms'` for
an integer `wait`), computed from the machine's fields when the send runs and read
as a time by the one duration reader every backend shares: a value that is no
time, a bare number included, and an operation that fails are the argument error
of W3C SCXML 6.2.4, so `error.execution` is raised and the message is not sent
under some default wait. It takes no `delay` beside it. A machine that has one
must be driven with `tick()`, as one with a written delay is
(`scenarios/static_send_delay.json`, whose `advance_ms` steps move a manual clock;
the Interpreter waits the time out, and C11 replays it as the others do). C11
holds a string in a bounded buffer, so it writes an expression that joins text
into one the generator sizes from what the data model declares
(`SCE_FORGE_CONCAT`, `sce/forge/wire.h`): a string variable by the capacity it
declares, a literal by its text, an integer by the digits of its type — the ten of
a `uint32`, so `wait + 'ms'` is a buffer of thirteen bytes with its terminator —
and a chain `a + b + c` is one buffer, not a join of a join. The text of each
part is the one the wire writes (a string as itself, an integer as its decimal
digits), and a buffer sized from the bounds cannot overflow, so a join never
holds a truncated text. The buffer lives as long as the block that reads it,
which is where a delay, an event name, an id and a request's source are read.
C11 refuses a join with an operand the model declares no size for, and one with a
floating-point value, a boolean or another type whose text is not the same in
every backend, as the other backends refuse the latter. A `<send>`'s
`eventexpr` is the same for the name of the event the send delivers: a string
computed from the machine's fields when the send runs, so the one `<send>`
delivers a different event once the name it reads has changed, and a name that is
empty, or that an operation fails to compute, names no event, which is the
argument error (`error.execution`, nothing sent). It takes no `event` beside it,
and needs no script engine
(`scenarios/static_send_event.json`, on the six generated backends and the
Interpreter, where C11 holds the name in a string variable). A `<send>`'s
`targetexpr` is **a computed target**: a string computed from the machine's
fields when the send runs, chosen among the routes the document declares as
`sce:targets="#_internal #_parent"` (docs/adr/0005, decision 3), because a machine
with no script engine has nothing to learn from when the send runs where it can
send. The value is compared with the entries when the send runs, and the send goes
by the entry that matches; a value in none of them is `error.communication` and
nothing is sent — whether or not the route is one the machine could send by, since
the declaration is what the send was held to. An entry is a target of the SCXML Event
I/O Processor as a written `target` is — `#_internal`, `#_parent`, `#_scxml_<session>`,
`#_<invokeid>` or `sce://scxml/<session>`; a Mesh peer (`#name`) is a service a
backend gains with its own runtime (decision 5) and is refused in the set, and so is
anything the processor cannot address (§scxml-6.2.4). Four things are refused where
the attribute is written, as `sce:candidates` is: an entry that names no route, one
named twice (`validation/incompatible-attributes`), none at all
(`validation/empty-value`), and `sce:targets` on a send with no `targetexpr` or whose
`type` is another processor's. A `targetexpr` that declares no set is refused at the
attribute (`scxml/static-datamodel-rule`: there is no finite list to lower), and so
is one beside a `target`, since the element takes one. The attribute is read under
every data model, as `sce:candidates` is, and held to only under `sce-static`: under any
other the engine routes whatever the expression computes, so the declaration is an
audit of the routes and the document an Interpreter lowering writes, which keeps it, is
the document it was. The expression costs no script engine,
and a delayed send whose declared routes include a `#_` location other than
`#_internal` is a send waiting on another session for the saved state's purposes.
The Interpreter runs the document's own `<send>`, so its lowering rewrites the
attribute to `SceStatic.route(<expression>, [<entries>])`: a value that is none of
the entries is the empty target, which its engine answers as it answers any address
nobody is at (`scenarios/static_send_target.json`, `fire` declaring `#_internal` and
`#_parent` and `narrow` declaring `#_parent` alone; the lowering is refused by name,
`a <send> with a targetexpr has no <language> lowering yet`, on a language that has
not written it). `typeexpr` is still refused as above. A `<cancel>`'s
`sendidexpr` is the same for the id of the delayed send it removes: a string
computed from the machine's fields when the cancel runs, so the one `<cancel>`
removes another send once the id it reads has changed. An id no send holds, the
empty one included, cancels nothing and raises nothing; an id that an operation
fails to compute is the argument error, `error.execution`, and removes none. It
takes no `sendid` beside it, and needs no script engine
(`scenarios/static_cancel_expr.json`, on the six generated backends and the
Interpreter, whose `advance_ms` steps move the clock the sends wait on). The id
names a send the document wrote with an `id`, or one whose generated id an
`idlocation` handed the document (**Generated send ids** below). The value
crosses twice, as the text a form or a
host's `params` carries and as a JSON value in `_event.data`, and both are
rendered from one typed value: a bool is `true` / `false` and a JSON boolean, a
string is itself and a JSON string, an integer is its decimal digits and a JSON
number, a real is its decimal spelling and a JSON number. ⚠ The text of a real
is the backend's own spelling, and the two differ where one writes an exponent
(Kotlin `1.0E-5`, Rust `0.00001`): the wire helpers every `<param>` already
crosses through have always done so, and a receiver reads the text as a number.
Only the
values every backend renders alike are admitted: a bool, a string, an integer of
at most 32 bits (`int8` … `uint32`, widened to `i64` / `Long`, which a `double`
holds exactly) and a real (`float32` widened to `f64` / `Double`, exactly). A
64-bit integer is refused because a backend that reads a JSON number through a
`double` carries one past 2^53 with its low bits wrong and no error, while
another carries it exactly: two backends giving one document two values. Bytes,
a list and a record have no spelling yet. An enum value is admitted as the name
its enum document declares for it, a string and as a saved state holds one — a
variable declared `enum:<alias>`, a field of a record variable, an
`<alias>.<variant>`, or a conditional of two of them — written through the
enum's own name on each backend (Kotlin's `declaredName`, Rust's `sce_name()`,
Go's `String()`, Python's `sce_name`, C++'s `sceLogName` and C11's
`<enum>_declared_name`), so the wire carries `agenda_list` and not an
identifier a backend made of it. One read from a loop's record item has no
lowering yet and is refused with the values that have no spelling. A value read
from the payload of the event the transition is on is carried on as any other,
an enum field of it as the name its enum declares: the transition's content then
runs only for a delivery that carried the payload, as an `<assign>` that reads
it does. It is read there and nowhere else: an entry, an exit, a host-run
`<invoke>` and a `<final>`'s `<donedata>` run when no event's payload is in
scope, and a transition on an event that declares no schema has no typed
payload to read, so a read in any of them is refused, saying where the payload
is read. `scenarios/static_payload_relay.json` carries a payload on as the
params of a `<send>` on the six generated backends and the Interpreter.
`scenarios/static_wire_enum.json`
holds an enum value in the params of a `<send>` on the six generated backends and
the Interpreter, whose lowering leaves the name its enum declares as the variant
lowered it, and `static_donedata` one in a `<donedata>` the same way. The Interpreter's lowering
rewrites the pairs of a `<donedata>` where each is written, and finishes an
inline `<content>` to the string it spells at the place it is written —
`<content>42</content>` becomes `<content>&quot;42&quot;</content>`, which an
engine reads as the string `"42"` where it would read the first as a number —
so the done event carries what the generated backends carry. A `<content>` that
holds an element is refused by name there, since an engine reads it as a
document and the generated backends carry it as a string.

A value that cannot be computed — a checked integer operation that overflows —
is the evaluation that failed (W3C SCXML 5.7.1): `error.execution` is raised
when the document declares it, the pair is left out, and the message still goes
and the invocation still starts, exactly as a pair a script engine could not
evaluate. A literal string is folded at build time and crosses as written.
`sce-build/tests/fixtures/host_processor/statechart_static_host_params.scxml`
holds Rust and Kotlin to it: a run that changes every variable before the send
and the invoke read them is told from a copy taken at start-up, and a `<param>`
that overflows is told from one dropped in silence. Measured 2026-10-01, a
`<param expr>` here had been typed, accepted and never lowered: the Kotlin
machine sent an empty payload, the Rust machine called a script engine it had
not been given and did not compile, and a host invoke read a copy of the
variable inside an engine that `<assign>` never wrote.

A `<donedata>` `<param>` is the same value on the same wire: the pairs that
survive are the JSON object a `<final>`'s done event carries — `done.state.<parent>`
for a final inside a state, and what the invoking parent reads as
`done.invoke.<id>` (stashed by `donedataAtFinal`) for a top-level one — written
by name, so the order the pairs were written in is not part of the answer. A pair
that cannot be computed is left out and the event is still raised, with `{}` when
no pair survived. `sce-build/tests/fixtures/static_datamodel/static_donedata.scxml`
holds Rust, Kotlin and C++ to it. Measured 2026-10-03, this one too had been judged
and accepted and never lowered: the Rust machine called a script engine it had not
been given and did not compile, and the Kotlin machine raised its done event with no
data at all, silently. C++ computes each value into a `ScriptValue` and hands the
pairs that survive to `DoneDataHelper::collectParams`, which writes the object with
the one writer `evaluateParams` uses for the pairs an engine evaluated. A
`<content expr>` has no typed form and is refused at parse, as it was.

A `<donedata>` that carries inline `<content>` in place of `<param>`s hands its done
event the text as the string it spells, finished at build time as a `<send>`'s
literal content is (the JSON string, whitespace-normalised, or the XML as written)
and copied into the event's data: `<content>42</content>` is the string `"42"` on
every backend, never the number an engine would read, and no script engine reads it.
Measured 2026-10-04, Rust, Go, Kotlin, Python and C++ generated a call to one for it,
which made `needs_script_engine` true for a machine that was to have none, and C
refused it by name. `static_donedata_content` holds every backend to it, and
`a_donedata_content_is_the_text_it_spells_and_no_engine_reads_it` reads the generated
machines.

Rust writes a string into a variable owned — `"busy".to_string()`, and a read
of another variable cloned — because a string inside an expression is borrowed
and a field holds a `String`; a string handed to a host action stays borrowed
(`&str`). Kotlin has one `String` for both.

**Generated send ids.** A `<send>` with an `idlocation` has the machine generate
its id (§scxml-6.2.4), and the document keeps it in a variable a later `<cancel
sendidexpr>` names (`scenarios/static_send_idlocation.json`, on the six generated
backends and the Interpreter). The id is `_auto_send_` and the number of ids this
machine has generated, counted from one. The count is the machine's own, so two
sends of one machine never hold one id and a document's ids do not depend on
what ran beside it. An id belongs to one execution of the element, not to the
element: a `<send>` run twice leaves two sends waiting under two ids, and the
variable holds the latest. It is written before the send reads any other
argument, so it is there even when a later argument cannot be evaluated and the
message is discarded (`error.execution`), and the error carries it.

The place is a string variable the document declares — not a record's field, a
list's element, or what a `<foreach>` binds, which would be a copy no one reads
— and it declares an `sce:capacity` of at least 32. The id is at most 31 bytes
(`_auto_send_` and the twenty digits of the largest count), and a machine is
built with no error to raise for a write that does not fit (§scxml-4.9), so a
bound the id could exceed is refused at the attribute, not carried to run time;
32 leaves a C buffer the room of its terminator. The element takes one id, an
`id` or an `idlocation`, never both. No script engine is asked
(`needs_script_engine` is `false`); a C machine carries a count and a 32-byte
buffer for it, only when it generates an id, and asserts that `SCE_MAX_ID_LEN`
holds them.

⚠ The text is one spelling on the generated backends, but not the same count
everywhere: the Interpreter generates the id with its own generator, so a
scenario compares what a `<cancel>` removes and never the id, and Python draws
the ids of its anonymous delayed sends from the same count, so the number its
document sees can run ahead of the other backends'. The count of a Rust or Kotlin
machine is saved (`sendseq`, above), and a restored machine's `<cancel>` reaches
the send by the id the document holds
(`a_generated_send_id_survives_a_restore`, on both, over
`saved/static_send_idlocation_armed.json`).

**Child sessions.** An `<invoke type="scxml">` hands its child the values its
`<param>`s and `namelist` name (W3C SCXML 6.4.1), each to the child's variable of
the same name; a `namelist` name is the `<param name="x" expr="x"/>` it
abbreviates. Under this model a child's variable is a native field that only the
child's own code sets, so a value is accepted only where the child gives it a
field to arrive in: the child is a `sce-static` document (written inline, or beside
this document under `src`), declares the name as a top-level `<data>`, and declares
it as a bool, a string, an integer or a real. The value is held to that variable's
type as an `<assign>` to it would be, read from the invoking machine's fields when
the invoke executes — at the end of the macrostep that entered its state, after
the entry actions, not when the state was entered — and lowered to native code, so
it costs no script engine. A string is held to the bound the child declared for
the variable, in UTF-8 bytes, whatever it came from: past it the value fails as
any other does (below), and is left out. A name the child does not declare, a variable of
another kind, a name handed twice and a child under another data model are refused
at the `<param>` as `scxml/static-datamodel-rule`: each would be typed, accepted
and never delivered (measured 2026-10-01 on Rust and Kotlin, when no generated code
handed a parent's `<param>` to a child).

A child that declares `<sce:action>`s takes the host that performs its acts when it
is built, because its first `<onentry>` can already perform an act and a host
installed afterwards would arrive one act too late. So the host has to exist when
the invocation starts, and the parent is the one that obtains it: its own host
interface gains, for each `<invoke type="scxml">` whose child declares acts, one
operation that answers the child's host (`fun actionsForWorker(): WorkerActions` in
Kotlin, `def actions_for_worker(self)` in Python, `ActionsForWorker()` in Go,
`virtual WorkerActions& actionsForWorker()` in C++, a reference to a host the answering
host owns and keeps alive past the child's end, a function pointer answering a
pointer to the child's table in C11, and in Rust an associated type bounded by the
child's own trait with the operation that answers a value of it, since the child's
machine is generic over its host and the parent's field names it
`<A as ParentActions>::ActionsForWorker`), and for
each candidate of a hybrid `<invoke>` that declares acts one of its own
(`actionsForWorkStaticHostedFirst`), since each candidate is a document with acts of
its own. The machine calls it each time the invocation starts — on entry of the
invoking state, and on a restore, which starts the child again from its beginning —
and builds the child with what it returns, so a state invoked again is given a host of
its own and nothing a host kept for one child is carried into the next
(docs/adr/0005, decision 6; `static_child_host` and `static_child_host_hybrid`, driven
by `AChildIsGivenItsHostByItsParentTest` and
`AHybridCandidateIsGivenItsHostByItsParentTest` in Kotlin and by
`test_a_child_is_given_its_host_by_its_parent.py` in Python, whose machine asks a
Python host object and keeps what it answered, and by
`a_child_is_given_its_host_by_its_parent_test.go` in Go and
`AChildIsGivenItsHostByItsParentTest.cpp` in C++ and `test_static_child_host.c` in C11,
none of which has a saved state and so a restore; the C11 one adds the table that answers
NULL, which is an invocation that does not happen; and by
`a_child_is_given_its_host_by_its_parent.rs` in Rust, which restores). A parent that
declares no act of its own still takes that host, and the recording host it generates
takes one source per child, the function a test hands it (in Rust it is generic over what
each answers, and derives neither `Default` nor `Debug`), and records each question
beside the acts. An act
whose name is spelled as one of those operations, in any language's convention, and
two invokes whose ids differ only in spelling, are refused where the second is
written (`<sce:action name="…">` names one host method, not two).

Every generated language does this, for a parent of either data model, and so does the
Interpreter, which has no interface to generate: its child sessions are built by the engine
that runs the parent, and the host a child takes is the one its parent's host answers to
`INativeActionHost::hostForChild(invokeId, document)`, asked each time the invocation
starts (entry and a restoration of a snapshot alike) and installed on the child before
anything of it runs. `document` is the stem of the document the child was loaded from —
the last path segment of its `src`, or of its evaluated `srcexpr`, without extension, which
tells the candidates of one hybrid `<invoke>` apart as `actionsForWork<Stem>` does on a
generated parent — and is empty for a child written inline as `<content>`. A parent with
no host, or a host that answers none (the default), leaves the child without one, and an
act it performs is `error.execution` in the child, as in any machine nobody installed a
host on. `sce-codegen lower` lowers an `<invoke>` of a child that declares acts, with the
`<sce:action>` carried as written (docs/adr/0005, decision 6;
`NativeActionRunsUnderTheInterpreterTest`, which starts a child from a document, and
`AStaticDatamodelRunsLoweredUnderTheInterpreterTest`, which runs `static_child_host` and
`static_child_host_hybrid` lowered). The code a generated parent would write otherwise
is the child's constructor called with the host left out: measured 2026-10-06, it does
not compile in Rust, Kotlin, Go or C++ and Python fails when the invoke starts, and C11
held the child as a value with no act table to give it; and measured 2026-10-07, a
parent that is not `sce-static` was not refused anywhere, since only the static lowering
asked. Each language was refused by name until it gave its host interface the operation
and replayed the fixtures, and the refusals were removed when the last of them did
(2026-10-07, the Interpreter included).

Each generated `sce-static` machine with such a variable carries the way in:
Kotlin a nested `InvokeParams` and `acceptParams`, Rust `<Machine>InvokeParams` and
`accept_params`, C++ a nested `InvokeParams` (a `std::optional` per variable) and
`acceptParams`, each variable `null` / `None` / empty when nothing is handed it and
keeping the value its `<data>` gave it; C11 has no such object, and the invoking
machine writes the values into the child's own variables between the two steps its
start takes (`<machine>_invoked_begin`, `<machine>_invoked_enter`, described with C11
below).
The invoking machine builds one from its fields and gives it to the child before
the child starts. A value that cannot be
computed — a checked integer operation that overflows — is the evaluation that
failed (W3C SCXML 5.7.1): `error.execution` is raised when the document declares
it, that one value is left out, and the child still starts. The witness is
`sce-build/tests/fixtures/static_datamodel/static_invoke_params.scxml`, driven on
Rust, Kotlin and C++ (the restore half is not, for C++ has no saved state): its
child ends only when it holds both a `<param>`'s value, read
after the entry action that changes it, and a `namelist`'s, and a child handed
nothing keeps its declared values and never ends. A string's bound has its own,
`static_invoke_string.scxml`, driven on all six backends (Rust, Kotlin, C++, Go,
Python and C11): its children hold four bytes, and one handed four ends on them
while one handed eight bytes and one handed two characters of five bytes are
left out and reported, each starting with the value its `<data>` gave it. A bound
counted in characters lets the second through, and a child that took a value past
its bound never ends.

A child is handed its values once, when it starts: a field the machine changes
while the child runs does not reach it. A restore starts each running child again
from its beginning (§2.15, "Saving and restoring"), and a start evaluates its
arguments, so the restarted child is handed what the restored machine's fields
hold, which differs from what the first start read when a field changed while the
child ran. That is the one definition of a start, not a loss: the child's own
progress is not saved either, so a saved state carries no copy of the values it
was handed. The fixture holds it: `watcher` is handed 7, `bump` makes the field 8
while it runs, and a restored machine's new `watcher` is handed 8.

**Integer operations.** A machine receives the failures of what it runs
(SCE_FORGE.md §3.4.1): every integer operation is checked at its own width —
the operands' type, not the comparison or the variable it feeds — and so is a
call of an imported algorithm that declares `may-fail`, which is therefore
admitted here. A failure never wraps and never panics; it is
`error.execution` (W3C SCXML 5.10) on the internal queue, when the document
declares that event, and in its place: an `<assign>`, `<log>`,
`<sce:append>` or host action whose expression fails does nothing; a
transition's or `<if>`/`<elseif>`'s `cond` that fails is false (W3C SCXML
5.9). A variable's initial value is computed before the machine runs, with no
session to raise in, so one that could fail is refused where it is written.
`level + 3` on a `uint8` at 253 therefore leaves `level` at 253 on every
backend — `sce-build/tests/fixtures/static_datamodel/static_overflow.scxml`
holds each backend to it.

**A 32-bit real.** `sce:type="float32"` holds a variable in the single-precision
type the backend has — `float` in C and C++, `Float` in Kotlin, `f32` in Rust,
`float32` in Go — and every operation on one is rounded to binary32 where it is
made, not later and not wider: `16777216 + 1` is `16777216` on every backend,
where a double makes `16777217`. An operation is made at the precision of the
place its value lands. In a `float32` slot it is a single's; in a `float64` slot
it is a double's, a narrower operand widened first, which is exact; and where no
real is expected — an operand of a comparison, a condition — it is made at the
width of the wider operand, a literal taking its partner's. A literal beside a
single is the single nearest to what is written (`0.2` is `0.2f`), and a variable's
initial value is rounded to one. A `float32` that leaves the machine — in a
`<param>`, in the saved state, read by a host — leaves as the `float64` it widens
to, exactly: the single nearest `0.1` is `0.10000000149011612`, not `0.1`.
Python and the Interpreter hold every real in a double, so the generator writes
the rounding itself (`sce_algorithm.to_f32` and `Math.fround`) around each operand
and result made as a single and around a value that lands in a `float32` slot;
C and C++ compute a `float` operation as a `float` where `FLT_EVAL_METHOD` is 0
(x86-64 with SSE, AArch64). The element of a `list<float32>` is a single like the
variable: a `float64` appended to one lands as the binary32 nearest it. A payload
field a schema declares `float32` — read into a variable, a list or a record's
field — is the binary32 nearest the number the payload carries: a JSON number is a
double wherever it is read, so every engine rounds once, from that double, and a
number past the largest single (`3.4028234663852886e38`) does not fit the field, as
an integer past its width does not, and is refused as it is. `static_real32` and
`static_record_real32` hold the seven engines to the same numbers, derived from
IEEE 754 and observed from none of them.

Under `null` or `ecmascript` an `sce:type` on `<data>` is not refused and
not a field type: with `sce:direction` and `sce:initial` it is the
statechart's typed input and output declaration that the authoring tool
(`tools/authoring`) drives a machine through. `sce-static` is the model
that makes the same attribute the variable's type.

`sce:type` is a field's type grammar — a scalar keyword or `enum:<alias>`
naming an enum the document imports — and is read with the same reader,
so a misspelled type is refused as `validation/invalid-attribute` with
the types the document could have written. A variable's `<data id>`
names a field of generated code, so it is held to the code identifier
grammar of §2.14, as a forge document's is. A `<data sce:kind>` declares
a kind, not a variable, and keeps its own rules.

**Record variables.** `sce:type="record:<alias>"` holds a variable in the
event-schema the document imports as `<alias>` — the record an algorithm's
local is held in (SCE_FORGE.md §4.12), by the same rules. It is built whole
from one `<sce:set name expr>` child per field of the schema, each given
exactly once and none the schema does not declare, and it takes no `expr`:

```xml
<sce:import kind="event-schema" src="schema_day.scxml" as="Day"/>
<datamodel>
  <data id="shown" sce:type="record:Day">
    <sce:set name="year" expr="2026"/>
    <sce:set name="month" expr="9"/>
    <sce:set name="dayOfMonth" expr="24"/>
  </data>
</datamodel>
```

A field is read as `shown.<field>`, typed as the schema types it; the
variable is closed over its fields, so any other member is
`expression/unknown-member`. It is updated a field at a time,
`<assign location="shown.<field>">`, or taken whole from another record of the
same schema by its name, `<assign location="shown" expr="chosen"/>` — a record
variable, or the record item of a `<foreach>` over a list of it. Nothing in an
expression makes a record, so any other value assigned to the whole record is
`scxml/static-datamodel-rule` on the `expr`, which says which names it takes.
A missing field is refused on the
`sce:type` that names the record, an unknown or repeated one on its `name`,
both as `validation/attribute-rule-violated`. `<sce:set>` is its own element
because `<sce:field>` is the codec's byte-layout field.

**A payload taken whole.** The payload of the event a transition is on is a record
of its schema, and `_event.data` names it: `<assign location="shown"
expr="_event.data"/>` replaces the record in one assignment, and `<sce:append
target="seen" expr="_event.data"/>` adds it whole to a list of the same schema, with no
`<assign>` per field to keep in step with the schema. It is accepted when the
schema of the event declares the fields the record's does, each of the type it
does, in any order; one that declares other fields, or the same fields of other
types, is `scxml/static-datamodel-rule` on the `expr`, as is a transition on an
event that has no schema. A payload that does not fit its schema — a name its
enum does not declare, an integer past its width — is refused before the
transition runs, so the record is not replaced and the list not appended to, the
fields that did fit included. Every generated backend and the Interpreter replay
`scenarios/static_whole_payload.json`; Python's lift reads an integer at the width
the schema declares, as the others do, since its `int` has none to refuse a value
by. The fixture is not named `static_record_payload`: C11 names a machine's
payload struct `<machine>_<event>`, and `static_record` has an event `payload`, so
a machine of that name put both in one header.

**Enum variables.** `sce:type="enum:<alias>"` holds a variable in a variant of
the enum the document imports as `<alias>` (`<sce:import kind="enum">`). It
starts at one of the variants, written as the expression `<alias>.<variant>`
(a `<data>` with no `expr` is refused like any other):

```xml
<sce:import kind="enum" src="enum_view_mode.scxml" as="ViewMode"/>
<datamodel>
  <data id="layout" sce:type="enum:ViewMode" expr="ViewMode.month" sce:direction="out"/>
</datamodel>
```

An enum value is one of three things: `<alias>.<variant>`, a variable
declared `enum:<alias>`, or a conditional whose two branches are values of one
enum. What it may be used for is deliberately small: stored (`<assign>`) in a
variable of its own enum, logged, and compared with `===` or `!==` to a value
of that same enum. Anything else is `expression/unsupported-construct` on the
expression: ordering, arithmetic, a call argument, a comparison with a number
or with another enum, a value of one enum stored in a variable of another or
in a number. An enum value is also carried as a `<param>`, as the name its
enum document declares for it (**Params** above); a value of a loop's record
item is not, and is refused as `scxml/static-datamodel-rule` like a list or a
record. The expression typer
declines to type an enum value (its integer belongs to the enum document), so
this is the one place that holds it to where it may stand
(`forge/static_enum.rs`).

The enum must be a closed set: one that declares `sce:strict-variants="false"`
admits values no variant names, which a machine has no type to hold, and is
refused on the `sce:type` that names it as `scxml/static-datamodel-rule`. The
type is declared in the machine's own unit, as a record's is — a statechart
does not import the enum document's generated type — so the variable is the
machine's own `<Machine><Alias>Enum`, whose variants are spelled as the enum
document's own generation spells them (`enum_naming::variant_ident`: Rust
`AgendaList`, Kotlin `AGENDA_LIST`). It carries no integer: a host that needs
the enum document's value reads it by name.

**A record's enum field.** An event-schema field declared `enum:<alias>` is held
in a record as the machine's own enum type for it, so a record variable (or a
list of records) of such a schema needs the document to import the enum under
the alias the schema writes — the convention the typed payload's width check
already keeps; one that does not is `scxml/static-datamodel-rule` on the
`sce:type`, naming the alias. The field is judged as a variable of the enum is:
its `<sce:set>` and an `<assign location="shown.layout">` take a variant (or
another value of the enum), it is compared with `===` and `!==` to its own
enum, and it takes no number. It reads the same through a `<foreach>`'s record
item (`v.layout === ViewMode.week`), which is read and not written. A saved
record holds the field as the variant's declared name, and the saved shape
names each enum field's variants as a variable's are.
`scenarios/static_record_enum.json` holds this on every engine that runs the
model.

**A record's string field.** A record variable (or a list of records) whose
schema has a `string` field holds it within the `sce:max-size` the schema writes on
the field (`docs/adr/0005`, decision 1), as a string variable holds its
`sce:capacity`. The bound is required: a field that declares none is refused where
the record is declared, as `scxml/static-datamodel-rule`, by every backend, and no
default stands in for it. The field starts at a string literal of at most that many
UTF-8 bytes (`<sce:set name="label" expr="'a'"/>`), since the machine is built with
no error to raise. A value written to it past the bound — an `<assign
location="last.label">` of a literal, of a string variable or of a payload field, or
a payload taken whole into the record (`<assign location="last" expr="_event.data"/>`,
`<sce:append target="labels" expr="_event.data"/>`) — fails as any assignment does:
nothing is written, `error.execution` is raised and the block ends (§scxml-4.9), so
the record and the list are left as they were; a saved state that
claims a longer one is refused when read, naming the field. The bound counts UTF-8
bytes, not characters. `scenarios/static_record_string.json` holds this on every
backend: Rust, Kotlin, Go, Python, C++ and C11.

Each holds the field as the value its language holds a string in. In Rust the record
is a struct that owns a `String` and so is `Clone` and not `Copy`, a host reads a
published one through a borrow, and a copy of one — an append, an assignment from
another record — is a clone. In Kotlin the record is the immutable data class it
already is, so a copy is the same value, and a saved string is read back through the
runtime's bounded reader. In Go the record is a struct copied by value, so a copy is
the same value too; a Go machine is not saved, so there is no restore to refuse. In
Python the record is a frozen dataclass changed a field at a time by replacement, so
a copy is the same value, and a Python machine is not saved either. In C++ the record
is an aggregate with a `std::string` member, copied by value, and a C++ machine is not
saved. In C11 the field is a buffer of the bound, `sce_static_string_<N>_t`, the type
a string variable of that bound is held in, so the record is a struct that is still
copied by assignment: an expression reads the field as the buffer's text
(`last.label.data`), a write copies the text and its terminator into it, a record
made whole from a payload fills the buffer from the text the bound already admitted,
and a C machine is not saved. Measured 2026-10-06, before any backend held the field,
`check` had answered ok for Rust, Kotlin, Go, C++ and Python while Rust wrote
`#[derive(Clone, Copy)]` over a `String`, which does not compile (E0204); only C11
refused. The Interpreter runs the same scenario lowered
(`AStaticDatamodelRunsLoweredUnderTheInterpreterTest`, which finds scenarios by
scanning their directory), under the same bound.

**A byte string.** A `bytes` variable declares the most bytes it holds with
`sce:capacity`, and a record's `bytes` field is bounded by the `sce:max-size` its
schema writes (`docs/adr/0005`, decision 2). No default stands in for either: a byte
string with no bound is refused where it is declared, as
`scxml/static-datamodel-rule`, by every backend. It starts at a literal of printable
ASCII with no backslash — the one literal whose bytes every engine spells alike
(`decode_bytes_literal`) — of at most that many bytes. Measured 2026-10-06, before
any backend held one, `check` had answered ok for a `bytes` variable with no capacity
on Rust and Kotlin and for a record with a `bytes` field on Rust, Kotlin, Go, C++ and
Python, while Rust wrote `#[derive(Clone, Copy)]` over a `Vec<u8>`, which does not
compile (E0204). Each backend was refused where it had not yet lowered a place, by
name, as `generate/unsupported-feature`, and lifted the refusal in the commit that
lowered it and replayed the shared scenario; the refusal was removed from the shared
check when the last backend did (2026-10-07), so every backend holds a byte string
in every place below.

A byte string is assigned from a printable-ASCII literal or from another byte string,
held to its bound where it is written (past it the assignment fails as any other does:
nothing is written, `error.execution` is raised, the block ends), compared with `===`
and `!==`, and measured by `len`. A string that is not such a literal — another
variable, a concatenation — is refused where it would be held as bytes, since no
backend takes a text for a byte string. A saved state writes it as its byte-exact
Latin-1 text, each byte the character of that code point, and reads it back only as
Latin-1 text of at most the bound: a character past U+00FF is no byte and is refused,
not cut. **Rust** holds a `bytes` variable as a `Vec<u8>`, **Kotlin** as a `ByteArray`,
**Go** as a `[]byte`, **Python** as a `bytes`, **C++** as a `std::vector<uint8_t>` and
**C11** as a buffer of the bound and the length it holds
(`scenarios/static_bytes.json`), and the **Interpreter**'s data model, which holds no
types, as the text of its bytes, one character to a byte, held to its bound by
`SceStatic.boundedBytes`.

A record's `bytes` field, bounded by the `sce:max-size` its schema declares and written
a field at a time from a literal or from a `bytes` variable, is held by **Kotlin**,
**Rust**, **Go**, **Python**, **C++**, **C11** and, as the text of its bytes in the
object that holds it, the **Interpreter** (`scenarios/static_record_bytes.json`), in a
variable and in a list of records alike.

A transition on an event whose payload carries a `bytes` field reads it into a `bytes`
variable, into a record's field or into a whole record, held to the bound of the place
it is written to (`scenarios/static_payload_bytes.json`), and compares it in a guard. The wire spells it as
its byte-exact Latin-1 text: a byte above 0x7F is one byte, and the two bytes of its
character in the UTF-8 text; a character past U+00FF is no byte, so the payload that
carries one does not read as its schema and is refused (`error.execution`, nothing
written), whichever way the text spells it, an escape included. **Kotlin**, **Rust**,
**Go**, **Python**, **C++**, **C11** and the **Interpreter** hold it: the Interpreter's data
model holds a payload's byte string as the text its wire spells, one character to a byte,
and reads it through the library's `field` as it reads every field of a payload, which
refuses a value that is no text or holds a character past U+00FF.

The bound is held where the bytes are written, as a string's is, and not where they are
read, with one structural exception the shared scenario does not state: an engine whose
payload buffer is the bound itself (C11, and Rust without an allocator) refuses a payload
longer than its field's `sce:max-size` whole at the lift, even on a route that writes none
of it, where the others read it and only compare it. Measured 2026-10-07 by the scenario,
the C++ payload lift had read the bytes of the UTF-8 text as the bytes of the field and
written them into a JSON string as they were, and C11 had read a character written as
itself byte by byte. Both now read the Latin-1 text (C++ through `SCE::Latin1Bytes`, which
a typed host request shares), and C++ reads a `\u` escape in a text field as the character
it names.

A Kotlin machine holds a copy of the array a host raised the event with
(`SceChecked.bounded`), so a host that kept the array and wrote into it changes nothing
of what the machine read; a Go machine does the same (`BoundedBytes`), a Rust machine
copies the borrowed bytes into the `Vec<u8>` it writes them to, and a Python machine keeps
the host's `bytes` itself, which nothing can write into. A C11 payload holds its bytes as
an array of the schema's bound and the length beside it, which an expression reads as the
view a byte string held by the machine is read as. A Kotlin record that holds a byte string compares
and hashes by the bytes and not by the identity of the array, as a data class would
otherwise, and is handed to a host as `detached()`, a copy of each array, as is each
record of a published list. A Rust record that holds one is `Clone` and no longer `Copy`
(as one that holds a string already was), is lent to a host by reference, and is saved as
the object of its fields with the byte string as its Latin-1 text, read back only within
the bound its schema declares. A Go record keeps its fields unexported and answers each
through a reader, which for a byte string is a copy; the machine replaces a slice and
never writes into it, so a copy of a record shares it safely. A Python record is a frozen
dataclass of immutable `bytes`, so a host is handed the value itself and two records of
the same bytes are equal. A C++ record holds a `std::vector<uint8_t>`, is handed to a host
by value (a copy), and a list of records is lent as a constant reference, both asserted
where the test is compiled. A C11 record holds the buffer of the bound and the length it
holds, the same type a `bytes` variable of that bound has, declared before the record and
shared by every field and variable of the bound; it is handed to a host by value and a list
of them is lent as a view, and a write to the field copies both the bytes and the length.

A Kotlin or Go machine never writes into its array
and hands a host a copy of a published one, so the bound the machine keeps cannot be
changed from outside; a Kotlin snapshot's byte string is such a copy, and two snapshots
are equal by the arrays' identity, as any `ByteArray` in a data class is. A Python
`bytes` cannot be written into, so a host is handed the value itself, and a C++ host is
lent a `const std::vector<uint8_t>&` (asserted where the test is compiled) and a C11
host a `sce_forge_bytes_view_t` over the buffer. A C11 byte string is compared by its
length and then its bytes, assigned by copying both, and held to its bound by the view
of the value, which carries its length; a comparison of two of the same length and
other bytes is a miss on every backend (`other` in the scenario). A Go, Python, C++ or
C11 machine is not saved, so its host reads the value through its accessor.

A byte string is a `<param>` of a `<send>`, of a host-run `<invoke>` and of a
`<donedata>` as it is a payload's field: its byte-exact Latin-1 text, each byte the
character of that code point, as the text a request carries and as the JSON string of
the event's data (a quote, a backslash and a control byte escaped as any string's are).
The machine that reads that event back through a typed payload holds the bytes it sent
(`scenarios/static_bytes_wire.json`, which sends a variable to itself and finishes with
it in a `<donedata>`). Every backend and the Interpreter carry it, each spelling the text in
the type it carries a string in (`StaticTarget::wire_bytes`): the Interpreter's data model
already holds it, and the C11 wire value (`sce/forge/wire.h`) holds it with its length,
since a C string is NUL-terminated and a byte string may hold a 0x00. That is also the one
place C11 differs: where a request carries a `<param>` as text, which is a C string,
a byte string that holds a 0x00 is refused and not cut short; it crosses whole as the JSON
of the event's data (`forge_wire_bytes_test.c`).

A saved state holds the variant by its declared name, `"agenda_list"`, which is
the same on every backend and is not the constant a backend spells for it; one
that holds a name the enum does not declare is refused when read. The saved
shape names each variable's variants, sorted, so an enum document that renamed
or added a variant refuses a state saved before it, and one that reordered or
renumbered them does not. `scenarios/static_enum.json` and `saved/static_enum.json`
hold this on every engine that lowers the model. The Interpreter's ecmascript
lowering holds an enum value as the variant's declared name, a string
(`ViewMode.month` lowers to `'month'`), so it compares, stores and logs as
the generated backends do and replays the same scenario.

**String variables.** `sce:type="string"` holds text, and declares the most
UTF-8 **bytes** it ever holds with `sce:capacity`, a whole number of at least one
that fits 32 bits. The bound is required — a machine holds the same value
wherever it runs, and an engine with no heap holds it at all — and it counts
bytes, not characters: a character is one to four of them, and no backend's own
string measures that alike (a Kotlin string is UTF-16 units, a Python one is code
points), so each counts the bytes of the text it holds. A string starts at a
string literal that fits its bound, written in `expr`: the machine is built with
no error to raise, so a value that could fail to fit is refused where it is
written and not copied from another variable at run time. An `<assign>` whose
value holds more bytes than the variable's bound fails as an integer operation
that overflows does — nothing is written, `error.execution` is raised and the
block ends (W3C SCXML 4.9) — whatever the value came from: a literal, another
string variable, or a field of the event's payload.
`scenarios/static_string_capacity.json` holds this on every engine that lowers
the model: `copy` assigns a sixteen-byte `body` to a four-byte `title`, and an `é`
and a `€` (five bytes in two characters) are refused where two `é` (four bytes)
are not. The refusal is the checked helpers' own (`capacity-exceeded`), so each
backend reports it as it reports any checked failure.

**List variables.** `sce:type="list<T>"` (in XML `list&lt;T&gt;`) holds a
sequence of `T`, a fixed-width number or `bool` — the element an algorithm's
list admits (SCE_FORGE.md §4.12). It starts empty and takes no `expr`; it
declares the most elements it ever holds with `sce:capacity`, which is
required on a list and on a string, and refused on any other variable. Two
statements write it, both naming it by `target` as E8's does:

```xml
<data id="picked" sce:type="list&lt;uint8&gt;" sce:capacity="3"/>
...
<sce:append target="picked" expr="_event.data.dayOfMonth"/>
<sce:clear target="picked"/>
```

The appended value is judged against the element as an assignment is judged
against its variable. The bound holds on every backend: an append to a full
list appends nothing and raises `error.execution` (W3C SCXML 3.12.2) — unlike
an algorithm's list, which grows past its capacity on the heap backends,
because a machine must hold the same list wherever it runs — and, as any
element that raised does, ends the block it stands in (W3C SCXML 4.9: the
elements after it are not processed). So does an `<assign>` or a `<log>` whose
checked integer operation fails, and an `<if>` or `<elseif>` whose condition
cannot be evaluated once its chain has run. `scenarios/static_block_ends*.json`
hold this on every engine that runs the model. An expression
measures a list with the `len(…)` builtin — `len(picked) === 3` in a guard,
`len(picked)` assigned to a count — and reads it no other way: a list read as
a value (in an expression or a host action's argument) and a whole-list
assignment are both `expression/unsupported-construct`. A list is walked with
`<foreach>`, below, and read by the host through the snapshot.
A `target` that names no list is `scxml/static-datamodel-rule`, naming the
lists there are, and so is either statement under any other data model.

**A list of records.** `sce:type="list&lt;record:Day&gt;"` holds records of an
imported event-schema, declared as a record variable is (a type of the
machine's own, `<Machine><Alias>Record`) and bounded and cleared as a list of
numbers is. A record is built by its `<sce:set>`s and updated a field at a
time, so nothing in an expression makes one: `<sce:append target="days"
expr="draft"/>` takes a record **by its name** — a record variable declared
`record:Day`, or the item of a `<foreach>` over a list of `Day` — and the list
holds a copy of it as it stands then. Anything else written there is
`scxml/static-datamodel-rule` on the `expr`, which says which names it takes.
A schema with an enum-typed field is held as a record variable's is (above). The host
reads a published list through the snapshot (an immutable `List<…Record>` in
Kotlin, a slice in Rust); a saved state writes it as an array of the record's
objects, and the saved shape names the record's fields.
`scenarios/static_record_list.json` holds this on every engine that runs the
model.

**Iterating a list.** `<foreach array="picked" item="v" index="i">`
(W3C SCXML 4.6) walks a list variable: `array` names one the machine
declares, `item` is each element typed as the list's element, and `index`,
which may be left out, is its position from 0, a `uint32` as `len` is. Both are
the body's own: they mean nothing after the `</foreach>`, and a body's
expressions read them typed, so `total + v * (i + 1)` is a checked integer
operation like any other. Each is a name nothing in scope already means (a
variable, an enum, an imported algorithm, an enclosing loop's variable — a loop
in a loop takes names of its own), a code identifier (§2.14) that no backend
reserves as a keyword and that does not begin `sce_`, which the generated code
keeps for its own; anything else is `scxml/static-datamodel-rule` on the
attribute. An `array` that names no list is the same. Over a list of records the
item is a closed record of that schema: its fields are read typed as `d.dayOfMonth`
(any other member is `expression/unknown-member`), and it is appended whole to a
list of the same schema. What a loop binds is read and not written: an
`<assign>` to the item, to a field of a record item or to the index is
`expression/unsupported-construct`.

The walk is of the list as it was when the loop began (a shallow copy, as 4.6
says), so a body that appends to the list it walks adds to the machine's list
and not to the walk. A body element that fails — a checked operation that
overflows, an append to a full list — ends the loop and the block that holds
it (4.9), the same rule the elements outside a loop keep; the elements after
the `</foreach>` do not run. The loop needs no script engine: Kotlin lowers it
to a `for` over the typed `List`, Rust to a `for` over a copy of the `Vec`, and
the Interpreter's ecmascript keeps the document's own `<foreach>` over the
array the list is, with its body lowered as every other expression is.
`scenarios/static_foreach.json` holds this on every engine that runs the
model.

**Algorithm calls.** A `sce-static` document calls an algorithm it imports
(`<sce:import kind="algorithm" src="…" as="DaysInMonth"/>`) the way a forge
kind does, `DaysInMonth(shown.year, shown.month)`, in a guard, an assignment
or a host action's argument, judged against the signature the forge import
pass discovers — its parameters' and return's types. An algorithm whose
signature takes or returns a `list<T>` or a `record:` is not called from a
statechart — a host calls it, and another algorithm when its slots are
records (SCE_FORGE.md §4.12) — and is refused where it is called. The import is
read where the document is parsed and refused there if its file is missing
or is not an algorithm (`import/file-not-found`, `import/kind-mismatch`,
`import/not-forge`); under any other data model an algorithm import is
`scxml/static-datamodel-rule`, as is one the document never calls — unlike a
forge kind, which drops an unnamed import, a statechart keeps no import for
later. Kotlin calls the function the algorithm's own generation emits and
imports it by the line a forge kind importing it writes
(`import com.sce.generated.<name>.*`); `sce-codegen generate` does not
generate the algorithm itself, so it is generated beside the machine.

**Code generation.** A backend lowers the model once its templates hold
the variables as fields and route every expression through the forge
expression lowerer; until then `sce-codegen` refuses the document for
that backend as `generate/unsupported-feature`, naming the backends that
do lower it. The refusal is deliberate: every backend's templates read a
`<data>` as a script-engine variable, so generating anyway would evaluate
forge-language expressions in Lua or QuickJS — a language the document
never declared. Which backends lower it is `STATIC_DATAMODEL_BACKENDS` in
`sce-build/src/generator.rs`.

Kotlin lowers it (`crate::forge::static_lowering`): each variable is a field
of the generated machine, written only by the machine — `var <name>: <type>
= <init>` with a private setter when it is published, a `private var` when it
is the machine's own — and each expression is lowered through
the forge expression lowerer into the slot the Kotlin templates already
render as native code — a condition as a native guard (`In(id)` as the
machine's active-state test), an `<assign>` or `<log>` value as the text the
engine-free arm pastes. A condition that reads the triggering event's typed
payload takes the payload channel's own null guard. A transition whose
content reads it runs that content only for a delivery that carried the
payload: otherwise none of it runs, checked once for the whole block because
an error stops the block (W3C SCXML 4.9). The error itself is reported once,
as `error.execution` on the internal queue, by the engine where it failed to
read the payload from the delivery — the check does not report it a second
time. A record variable is a field of an immutable data class,
`<Machine><Alias>Record`, and a field update lowers to
`shown = shown.copy(<field> = …)`. A list variable is an immutable
`List<T>` field that starts `emptyList()`; an append lowers to
`if (picked.size < N) { picked = picked + (…) } else { <error.execution> }`
and a clear to `picked = emptyList()`, so a snapshot shares the list it
publishes without copying it. The generated machine carries no script
engine. An enum variable is an `enum class <Machine><Alias>Enum` of the
machine's own unit, one constant per variant, with its saved form on the type
(`toSaved` / `fromSaved`, as a record's is).

Rust lowers it through the same walk (`StaticTarget`, one per backend: what
differs is a spelling, never a meaning). Each variable is a field of the
generated `<Machine>Policy`, `snake_case`, initialised in `new()` in
declaration order; a published one has an accessor of its name, by value, or
as `&str` / `&[u8]` / `&[T]` for a string, bytes or list, so a host cannot
grow a list past the bound the machine keeps. Each expression lands in the
slots every backend's templates read before their own spellings — a
transition's guard in `native_guard`, an `<if>`'s condition in
`native_cond`, a whole statement in `native_code`. A condition that reads the
payload is the typed guard's own shape, `matches!(&self.pending_payload,
<Machine>Payload::<Event>(ev) if …)`, and content that reads it binds `ev`
once or does not run, as Kotlin's does. A record is a plain `Copy` struct,
`<Machine><Alias>Record`, updated in place; a list is a `Vec<T>` whose append
lowers to `if picked.len() < N { picked.push(…); } else { <error.execution> }`.
A failing integer operation is received in a closure that is a `Result`, so
the statement returns out of it before it writes.

C++ lowers the model through the same walk, and refuses what it does not by name
(`CppTarget::unsupported`: `generate/unsupported-feature`, "has no C++ lowering
yet"): scalar variables, a transition's guard, `<assign>`, `<if>` /
`<elseif>`, `<log>`, `<raise>`, `<send>` / `<cancel>` — the `<param>`s of a
`<send>` are computed into `ScriptValue`s from the machine's own fields and put in
the typed map the event's JSON is built from (a pair whose value failed is
reported and left out and the message still goes, §scxml-5.7.1), and a literal
`<content>` is the normalised text handed to the helper `<donedata>` takes with
no data model; a BasicHTTP `<send>` that carries a `<param>` is refused by name,
because it needs each value as the text a form carries — `In()`, a `<sce:action>`
whose arguments are typed expressions
of the machine's variables, an event's typed payload, an enum variable, a
record variable, and a list of numbers, bools or records with `<sce:append>`,
`<sce:clear>` and `<foreach>`, a call of an imported algorithm, and the
`<param>`s of a `<final>`'s `<donedata>`, and an `<invoke type="scxml">` — the
child is one more generated machine the parent starts with its own invoke code,
after handing it the values through `acceptParams`, and an `<invoke>` the host
runs, whose `<param>`s are read from the policy's fields into the request's
`params` (as text) and `eventData` (as the pairs' JSON) when the invocation
starts — as a host-served `<send>`'s are, which carry the same two renderings.
`tests/integration/StaticHostParamsAotTest.cpp` drives
`statechart_static_host_params` and holds the value on the wire and the pair a
failed computation leaves out. A hybrid `<invoke>` (§2.13) reads the stem of the
string its `srcexpr` computes (`SCE::documentStem`) and starts the candidate it
names, handing it the values it keeps and evaluating the rest. A bytes variable
and a mesh invoke are not lowered yet. A call is the
algorithm's own free function, `SCE::Generated::<Name>::<name>(…)`, wrapped in
`Checked::take(sce_failure_, …)` when the algorithm can fail, so a failed call
is received as any failed operation is; the machine's header includes the
algorithm's header, which is generated on its own (`sce_add_algorithm()` in
CMake) and not by the machine. A list is a bounded
`std::vector<T>` the machine alone grows (a host reads it as
`const std::vector<T>&`); an append checks the room first and computes its value
into a local, so a full list or a failed value leaves it as it was and ends the
block, and a `<foreach>` walks a copy made when the loop began (`sceCopy`,
`sceIndexed`). A record is a plain struct, `<Machine><Alias>Record`, whose
fields are the schema's in the schema's order; a value is built whole by
designated initializers, a field is assigned in place after the same
check-then-write rule as any other target, and a record item of a `<foreach>` is
read field by field. An enum is a scoped enumeration
(`<Machine><Alias>Enum`, over the enum document's own carrier, each variant
holding the value the document gives it) declared in the machine's unit, with
`sceLogName(value)` answering the name the document gives a value — what a
`<log>` shows. A guard that reads `_event.data` is
the payload channel's own shape, `pendingPayloadTag_ == <Machine>PayloadTag::<Event>
&& (…)`, over the `pending<Event>Payload_` member the engine fills; content that
reads it opens with the same tag check and does not run for a delivery that
carried none, raising nothing (the engine already said `error.execution` where
it failed to read the delivery). Each variable is a member of the
generated policy, `v_<snake_case id>`, initialised where it is declared; a
published one has a reader of the author's spelling (`count()`), on the policy
and forwarded by the machine, by value or as `const std::string&`. The
machine carries no script engine (`NEEDS_SCRIPT_ENGINE = false`) and its entry
and exit methods are not `static`, because the lowered statements read and write
the members. A failing integer operation is the runtime's checked one
(`SCE::Forge::Checked`, from `<sce/forge/algorithm.h>`): it records the failure in
an `AlgorithmFailure` and answers a zero, so a statement that can fail is a
lambda called where it stands, answering whether it failed. The value is
computed into a local first and written only when it did not fail, so a failed
`<assign>` leaves its variable as it was, and the block ends (W3C SCXML 4.9). A
host call's arguments that can fail are computed into locals the same way, and
the host is called only when none of them failed; otherwise `error.execution`
is raised in the call's place (the block does not end, as in Kotlin and Rust).
`tests/integration/AStaticDatamodelRunsGeneratedCppTest.cpp` replays the
scenarios `static_counter`, `static_counter_bound`, `static_overflow`,
`static_block_ends`, `static_payload`, `static_payload_enum`, `static_enum`, `static_list`, `static_foreach`,
`static_real`, `static_real32`, `static_record_real`, `static_record_real32` and `static_block_ends_list` against the generated machines (an
event's `data` goes in as the JSON text every other producer fills, and the
machine lifts the typed fields out of it), and drives `static_host_call`
and `static_host_call_arguments` with a recording host.

Go lowers the model through the same walk (`GoTarget`), and refuses what it does
not by name: scalar variables of a number, a bool or a string, an enum, a record,
and a list of numbers, bools or records with `<sce:append>`, `<sce:clear>` and
`<foreach>`; a transition's guard, `<assign>`, `<if>` / `<elseif>`, `<log>`,
`<raise>`, a `<send>` / `<cancel>`, the `<param>`s and a literal `<content>` of a
`<send>` (the text it spells, as `Python` below says), `In()`, an event's typed
payload, a call of an imported
algorithm, the `<param>`s of a `<final>`'s `<donedata>`, an `<invoke
type="scxml">` and the values it hands its child, an `<invoke>` the host runs with
its `<param>`s (read from the policy's fields into the request's `Params` and
`EventData` when the invocation starts, as a `<send>`'s are), and a `<sce:action>`
whose arguments are typed expressions of the machine's variables.
`statechart_static_host_params`, driven by `host_params_test.go`, holds the value
on the wire for both the send and the invoke, and the pair a failed computation
leaves out. A hybrid `<invoke>` (§2.13) reads the stem of the string its
`srcexpr` computes (`sce.DocumentStem`) and starts the candidate it names,
generated into the parent's package, handing it the values it keeps. Bytes, and a
mesh invoke, are not lowered yet. Each variable is
a field of the generated policy, `v<PascalCase id>`, initialised in the
constructor; a published one has an exported reader of the author's name
(`Count()`), which answers a copy of a list. An enum is a named integer over the
enum document's own carrier, one constant per variant, with a `String` that
answers the name the document gives it; a record is a struct of the schema's
fields spelled as the author wrote them (an expression reads them so), each with
an exported reader (`Year()`) for a host. A list is a slice the machine alone
grows: an append checks the room first and computes its value into a local, so a
full list or a failed value leaves it as it was and ends the block, and a
`<foreach>` walks a copy made when the loop began. A conditional of two enum
variants or two reads is `scealgorithm.Choose`, which Go types from the values;
any other is a function literal, which has to name its result type. The machine
carries no script engine, and its package imports the
forge runtime's `algorithm` package for the checked operations. A failing
integer operation records its failure in a `sceFailure` and answers a zero, as
in C++, so a statement that can fail is a function literal called where it
stands, answering whether it failed; the value is computed into a local first and
written only when it did not fail, and `return` leaves the closure the block
runs in (W3C SCXML 4.9). A host call's arguments that can fail are computed
into locals the same way, and `error.execution` is raised in the call's place.
An imported algorithm is called as the package its own generation puts it in
(`days_in_month.DaysInMonth(...)`), and the machine imports that package by the
Go module path the packages live under (`--go-module-prefix`): a Go import path
has no valid bare form, so a document that calls an algorithm without one is
refused as a configuration error (`generate/invalid-config`), as a forge kind
importing another is. A donedata `<param>` is computed into a local with the
same failure flag, and a pair whose value failed is left out of the done event's
JSON (5.7.1) while `error.execution` is raised in its place; the value crosses as
`ScriptValueToJSON` writes it (a narrow integer widened to `int64`, a real to
`float64`). A child's variable of a scalar type takes the value its parent's
`<param>` or `namelist` names through `<Machine>InvokeParams` and `AcceptParams`
(a pointer per variable, nil when nothing is handed), called before the child
initializes, so a value is read when the invoke executes and a variable nothing
hands a value to keeps the one its `<data>` gave it; a value that failed is
reported and the child still starts one value short (5.7.1). A `<send>` `<param>` is the same value on the same wire, appended
to the typed list `BuildJSONFromTypedParams` writes: a pair that failed is left
out and reported, and the message still goes (6.2) — so a receiver reading the
event through its schema finds the field missing, which is an `error.execution`
of its own.
`scripts/regen_static_datamodel_go.sh` commits one package per machine the
generator lowers for Go — asked of it, not listed — and one per algorithm those
machines import, read from their `<sce:import kind="algorithm">`, and
`backends/go/tests/integration/static_datamodel/static_scenarios_test.go`
replays the scenarios `static_counter`, `static_counter_bound`,
`static_overflow`, `static_block_ends`, `static_payload`, `static_payload_enum`, `static_enum`,
`static_list`, `static_foreach`, `static_real`, `static_real32`, `static_block_ends_list`,
`static_record_fields`, `static_record_list`, `static_record_enum`,
`static_record_real`, `static_record_real32`, `static_record`, `sync_client`, `static_donedata` (the done event's pairs are
read back from `DonedataAtFinal`), `static_donedata_content` (its text, the same way) and `static_send_params` against them (an event's `data` goes in as the
JSON text every other producer fills; a
variable the machine keeps to itself is read by reflection, which only reads),
and drives `static_host_call` and `static_host_call_arguments` with a recording
host.

Python lowers the model through the same walk (`PythonTarget`), and refuses what
it does not by name: scalar variables, an enum, a record, and a list of numbers,
bools or records with `<sce:append>`, `<sce:clear>` and `<foreach>`; a
transition's guard, `<assign>`, `<if>` / `<elseif>`, `<log>`, `<raise>`, a
`<send>` / `<cancel>` that carry no value of the data model, `In()`, an
event's typed payload, and a call of an imported algorithm — the call is the
module's name and the algorithm's function (`days_in_month.days_in_month(…)`),
and the machine imports the module by the line a forge kind importing it writes
(`from . import days_in_month`), so a machine and the algorithms it calls are
modules of one package. A final's `<donedata>` `<param>` is lowered too: each
pair's value is native code reading the policy, written to the done event as the
JSON the runtime's `to_json_literal` gives it, and a pair whose value failed is
reported and left out while the others cross (5.7.1). A `<send>`'s `<param>` is
the same: each value is native code, the pairs are the dict the engine-evaluated
payload builds, a pair that failed is reported and left out and the message
still goes (5.7.1, 6.2); and transition content that reads the event's payload
opens with a check that the delivery carried one, which an unreadable delivery
did not, so it raises nothing of its own (the lift already did). An
`<invoke type="scxml">` is started by the machine's own invoke code and handed
its values by the build: each `<param>` and `namelist` name is native code read
when the invoke executes, put in a `<Machine>InvokeParams` and given to the
child's `accept_params` before it initializes (6.4.1), a value that failed is
reported and the child starts one value short (5.7.1), and the child is imported
as a module beside its parent. A `<send>`'s literal `<content>` is the text it
spells: the build finishes the event's data (the XML as written, or the string,
whitespace-normalised, that Go and Rust already wrote when no engine was there),
so a `<content>123</content>` is the string `"123"` on every backend rather than
a number on the ones that happened to carry an engine. A `<sce:action>` whose
arguments are typed expressions of the machine's variables is a call on the
machine's `<Machine>Actions` `Protocol`, each argument read when the call is made;
an argument whose computation failed (an exception, as every checked operation
is) costs the call and not the block: the call stands in a `try`, the host is not
called, and `error.execution` is raised in its place, as on Kotlin, Rust, Go and
C++. A call that reads the event's payload sits in a block that checks the
delivery carried one. An `<invoke>` the host runs reads its `<param>`s from the
policy's attributes when it starts, into the request's `params` (the payload
rendered as text) and `event_data` (its JSON), a value that failed left out and
reported (5.7.1); `test_static_host_params.py` drives `statechart_static_host_params`
and holds the value on the wire. A hybrid `<invoke>` (§2.13) reads the stem of the
string its `srcexpr` computes (`document_stem`) and starts the candidate it names,
a module beside its parent, handing it the values it keeps. A mesh `<invoke>` and
`bytes` are not lowered yet. Each variable is an attribute of
the generated policy, `v_<snake_case id>`, set in its constructor from the
variables declared before it; a published one has a reader of the author's name
(`count()`), which answers a copy of a list. An enum is an `IntEnum` over the
enum document's own values, with a `sce_name` that answers the name the document
gives it; a record is a frozen dataclass of the schema's fields, spelled as the
author wrote them, replaced whole when one field changes. Python's failure
channel is an exception: a checked integer operation raises `AlgorithmFailure`
in place of a value, so a statement that can fail is wrapped where it stands —
the exception stops it before it writes anything, `error.execution` is raised in
its place, and the sentinel every action block of the generated module already
catches ends the block (W3C SCXML 4.9) — and a condition that can fail is
computed through a helper that answers `false` and raises the same event
(5.9.1). The machine carries no datamodel in a script engine: the runtime still
hands every engine one session for its system variables, which is why a Python
machine is constructed with a script engine, and no lowered expression is
evaluated by it. Unlike the Rust, Kotlin and Go trees, the generated Python is
not committed (`backends/python/tests/integration/*/*_sm.py` is ignored, and in
the `static_datamodel` package every module but its marker and its test):
`scripts/regen_static_datamodel_python.sh` derives the machines the generator
lowers — asked of it, not listed — and the algorithms they import, read from
their `<sce:import kind="algorithm">`, and `scripts/gates/w3c-python.sh` runs it
before pytest, and
`backends/python/tests/integration/static_datamodel/test_static_scenarios.py`
replays the scenarios `static_counter`, `static_counter_bound`,
`static_overflow`, `static_block_ends`, `static_payload`, `static_payload_enum`, `static_enum`,
`static_list`, `static_foreach`, `static_real`, `static_real32`, `static_block_ends_list`,
`static_record_fields`, `static_record_list`, `static_record_enum`,
`static_record_real`, `static_record_real32`, `static_record`, `sync_client`, `static_send_params`, `static_donedata` (the
done event's pairs are read back from the engine's `done_data`) and `static_donedata_content` (its text,
the same way) against
them (an event's `data` goes in as the JSON text every other producer fills; a
variable the machine keeps to itself is read from its attribute, which only
reads), and `test_a_static_child_is_handed_its_params.py` drives
`static_invoke_params`, whose children are handed values once, when they start,
and `test_a_static_host_action.py` drives `static_host_call` and
`static_host_call_arguments` with a recording host.

C11 lowers the model through the same walk (`CTarget`), and refuses what it does
not by name (`generate/unsupported-feature`, "has no C11 lowering yet"):
variables of the integer types, `bool`, an enum, a string and a real of either width
(a `float`, which the wire writes as the `double` it widens to), the join
of strings and integers the data model sizes (`wait + 'ms'`), a transition's
guard, `<assign>`, `<if>` / `<elseif>`, `<log>`, `<raise>`, `In()`, `<cancel>`, an
event's typed payload of numbers, bools, strings and enums, a call of an imported
algorithm, a `<sce:action>` whose arguments are typed expressions of them, a
record whose fields are numbers, bools, reals of either width and enums, a list of integers, bools, reals
of either width or such records with its `<sce:append>`, `<sce:clear>` and `<foreach>`, the
`<param>`s of a final's `<donedata>`, a `<send>` to the machine's own event
processor or to one the host serves (`--host-processor`) with its `<param>`s or its
literal `<content>` (the text it spells, finished at build time and copied into the
event's data, as on every other backend), an
`<invoke type="scxml">` of a child that
declares no `<sce:action>`, handed numbers, bools and strings, and an `<invoke>`
the host serves (`--host-invoker`) with its `<param>`s, and a hybrid `<invoke>`
whose candidates are `sce-static` documents (§2.13): the machine reads the stem of
the string its `srcexpr` computes (`sce_document_stem`) and starts the candidate it
names as a static child is started — begun, handed the values it keeps, entered,
driven — evaluating the arguments it keeps no variable for. Bytes, a `<send>` to
another processor, a mesh `<invoke>`, an `<invoke>` or a `<send>` of a
type the host was not declared to serve, a `namelist` name that a `<param>` of
the same `<send>` or `<invoke>` or an earlier name of the `namelist` already names
and a
transition on an event whose payload carries a bytes field are refused
until their spellings are written: bytes need a capacity the C11 contract does
not carry yet.
A `<param>` whose value is a 32-bit real is not refused:
the contract fixes the 64-bit form only, so it is written as the `double` it
widens to, exactly, as Rust and Kotlin write it. The pairs of a `<donedata>` or of a `<send>` are written as the JSON object
an event carries as its data, by the header-only wire writer of the forge runtime
(`sce/forge/wire.h`), in the one order every engine writes the members in —
ascending by the name's UTF-8 bytes, whatever order the document declared its
`<param>`s in (ARCHITECTURE.md, "JSON Object Key Order") — which the generator
fixes once by listing the pairs sorted, a pair a failed value leaves out leaving
the others' order as it was. A `<param>` name that repeats is one array on every
engine, its values in the order the document declares them: the generator lists a
name's pairs one after another, and the writer, which remembers the last pair,
turns the second value of a key into the array it joins (`[3,4,5]`), so a name
with one value left — the others failed to compute — is that value and not an
array of one. Where a `namelist` name stands among the `<param>`s that share it
is not an order the engines were held to, so one that a `<param>` or an earlier
name of the `namelist` already names is refused by name.
`a_c_machines_event_data_follows_the_shared_key_order.rs` holds the order to the
table the other engines' writers are held to, `unit/forge_wire_repeat_test.c`
holds the writer to that table's case and to what it cannot say, and the C-only
document `integration_resources/static_param_repeat/` holds a send, an invoke and a
`<donedata>` of the generated machine to it. A `<send>`
the host serves carries the same pairs as the text of its request's `params`, from
the one value each is written from. A `<donedata>`'s go into the `done_data` buffer the machine
holds, which a host reads through `<machine>_done_data(sm)` and a compound
final's done event is built from; a `<send>`'s into the event's own data buffer,
which every delivery of the send reads. A `<send>` with a `delay` waits in the
machine's own scheduler and is delivered when its clock reaches it: a host that
owns the clock (`<machine>_init_with_clock`, `<machine>_advance_time_ms`) has
the sends that fall due together delivered in the order they were made, and a
`<cancel>` removes the one whose id it names; `test_static_timers.c` drives
`static_timers` so. A value is a bool, a string — its `"`,
`\` and control characters escaped, its UTF-8 as it is — an integer, at the
widest of its signedness, or a 64-bit real, written as ECMAScript spells it
(ARCHITECTURE.md, "JSON Number Text"): a JSON number, `null` where it is not
finite, and `NaN` / `Infinity` / `-Infinity` where a request carries it as text.
`test_static_host_params.c` drives `statechart_static_host_params` so. A pair whose
value failed to compute, or whose
location is empty, raises `error.execution` and is left out, every other pair
still crosses (5.7.1) — a `<send>`'s message still goes, and the error ends its
block (4.9) — and an object that does not fit the buffer is `{}` and raises it
too. A string a typed payload carries is read from the buffer the machine lifts
it into, and is held to its variable's bound by the `<assign>` it lands in. A
string is a struct of the buffer its bound
declares, `sce_static_string_<capacity>_t { char data[<capacity> + 1]; }`,
declared once in the machine's header per bound and held by value in the policy:
an expression reads its `data`, so a comparison is `strcmp` over it, and an
`<assign>` is `sce_forge_bounded_string` — the value, or the empty string with a
capacity failure recorded, counted in bytes by `strlen` as every backend counts
them — received as any failing value is and then moved, terminator included, into
the buffer by `memmove`, which the value may overlap (`title = title`). A
published string's reader answers the text of its buffer, a `const char *` the
host reads and does not write. A list is a struct of its bound,
`sce_static_list_<element>_<capacity>_t { size_t len; T data[<capacity>]; }`,
declared once in the machine's header per element type and capacity and held by
value in the policy: an append writes the element into the slot at `len` and
only then counts it, so one whose element failed leaves the list as it was, and
one past the capacity appends nothing and raises `error.execution` — both end
the block the append stands in by `return`ing from its function (W3C SCXML 4.9);
`<sce:clear>` sets `len` to zero. A `<foreach>` walks a copy of the list as the
loop began, by a count of its own (4.6), so a body that appends to the list it
walks neither lengthens the walk nor reads what it has just written. A published
list's reader answers the library's borrowed view of its elements,
`sce_forge_<element>_view_t { data, len }`, which a host reads and cannot grow
past the bound. A record is a struct of its schema's fields in the schema's
order, `<machine>_record_<alias>_t`, declared in the machine's own header — the
machine's alone, so two machines that import one schema declare two — and
copied by assignment: a field is written as `<record>.<field> = …` under the
failure rules of any assignment, the variable is built whole from its
`<sce:set>`s as a compound literal, and an append or a loop takes a copy. A list
of records is the same struct of its bound over that element, published through
`<machine>_record_<alias>_view_t { data, len }`. A published record is read by
value. A record's enum field is held in the machine's own type for the enum,
declared before the record. An enum is a C `enum` of the enum document's own
values, declared in the machine's header under a guard named for the document, so
that a program including two machines which import it declares it once; its
constants carry the document's name (`<DOC>_<VARIANT>`, as the enum kind's own C
artifact spells them) and `<doc>_declared_name(value)` answers the name the
document declares, or NULL for a value no variant names. A value is compared and
assigned as any other; a `<log>` shows its value, because C has no overloading to
choose its name by. An algorithm's C artifact is a header of `static inline`
functions the machine includes (`sce-codegen generate` writes it beside the
machine's and does not include it for the machine), the function named by the
bare symbol the algorithm declares, and a call that can fail is received through
the `<name>_take` the header declares — the one spelling a forge kind calling the
same algorithm uses (`algorithm_qualified_call`). A host action is a call through the vtable the
machine is initialised with (`sm->actions.<op>(sm->actions.user_data, …)`), each
argument read when the call is made; an argument that can fail is computed into a
local of its declared type first, and the host is called only when none of them
failed — otherwise `error.execution` is raised in the call's place and the block
goes on, as on every backend.
A typed payload is read through the channel the machine already declares for
it (`sm->pending_payload.as.<event>.<field>`, lifted from the `data` the event
carries): a guard that reads it is held to `pending_payload.tag` naming that
event, inside the value it computes so that an operation over a payload that
did not arrive is not run, and content that reads it runs only for a delivery
that carried one — a delivery with none runs nothing of it, rather than reading
a zeroed buffer. Each
variable is a member of the generated `<machine>_policy_t`, `v_<snake_case id>`,
set in `_init` from the variables declared before it; a published one has a
reader, `<machine>_get_<id>(sm)`, which answers it by value, and the machine's
own variables have none. The machine carries no script engine and is linked
without one: the checked integer operations a lowered expression calls are the
forge runtime's header-only `<sce/forge/algorithm.h>`, and a failed one leaves
its record in a `sce_forge_algorithm_failure_t` local of the statement. C has no
closure and no statement expression, so what the other backends spell as an
expression that runs a statement is spelled here as the statement itself: it
computes the value into a local, and when that failed raises `error.execution`
and `return`s, which ends the block (W3C SCXML 4.9) because every block is a C
function, and writes only when it did not. A condition that can fail is the head
of its `if` — the statements that evaluate it, leaving `false` when they could
not, and `if (<verdict>)` — so the `<if>`, an `<elseif>` and a guard keep the
meaning they have everywhere (5.9.1). `In("id")` in a lowered expression is the
machine's own `<machine>_in_state(sm, <ENUMERATOR>)`, which the template writes
from the call, as it does for a guard that is only `In()`.
`backends/c/tests/integration/test_static_scalars.c` replays the scenarios
`static_counter`, `static_counter_bound`, `static_overflow`,
`static_block_ends`, `static_list`, `static_foreach`, `static_real` (a real is a
`double` field, compared as the 64 bits it is), `static_real32` (a `float` field,
widened to the `double` it is and compared the same way), `static_block_ends_list`,
`static_record_fields`, `static_record` (a guard that calls an algorithm over
two of its fields), `static_record_list`, `static_record_enum`,
`static_record_real` (a record's real field is read by a reader of its own and
compared as the 64 bits it is, after a payload carried it), `static_record_real32`
(the same with a `float` field, a payload number past the largest `float` refused),
`static_string_capacity`, `static_donedata` (the done data read through
`_done_data` and held to the pairs it states and no others), `static_donedata_content`
(the same buffer, held to the string an inline `<content>` spells), `static_send_params`
(a string carried by a `<send>` and read back through a typed payload),
`static_payload`, `static_payload_enum` (a payload's enum field, lifted from the
variant's declared name the data carries), `static_enum` (a value stated as the
name its document declares) and `sync_client` (four standard sync algorithms
called over the payload of each answer) against machines generated from the
shared fixtures, reading `scenarios/<machine>.json` itself (`static_scenario.h`,
an event's `data` included) rather than writing the expected values out a second
time, and states the two things no scenario can — a delivery that carried no
payload, and the wire writer's own escaping and its refusal of a full buffer.
`test_static_host_call.c` drives `static_host_call` and
`static_host_call_arguments` with a recording vtable, to the calls the C++ suite
states. A child session an `<invoke type="scxml">` starts is a struct the parent
holds (`sm->child_<id>`), which has no constructor to be handed values, and
`_init` enters its initial configuration at once; a `sce-static` child is started
in two steps instead — `<machine>_invoked_begin`, which gives each variable its
declared value, installs the parent's clock and, for a child that sends to its
parent, the routing `_init_with_parent` stamps, and `<machine>_invoked_enter`,
the entry walk — and the parent writes between them each value a `<param>` or
`namelist` name hands over, into the child's variable of that name, read from the
parent's fields when the invoke executes (the lowering the other backends share,
spelled as a typed local that is written only when it was computed). A string is
bounded to the child's own bound where it is lowered and then copied, terminator
and all, into the child's buffer, which a value past it would have written beyond.
A value that
cannot be computed raises `error.execution` and is left out, and the child still
starts holding the value its `<data>` gave it (5.7.1). A child that declares
`<sce:action>`s is begun through a door of its own: `_invoked_begin` takes, after the
child, the table of the host that performs its acts — the pointer its parent's table
answers for it, as `const <child>_actions_t *(*actions_for_<invoke>)(void *user_data)` —
and answers whether it accepted one, refusing a NULL or incomplete table as
`_init_with_actions` does and leaving the child's storage as it was. The parent
reports a refusal as `error.execution` where the document declares one, and spawns
nothing (**Child sessions**, above; `test_static_child_host.c`). `test_static_invoke.c` drives
`static_invoke` and `static_invoke_params` live (their saved halves have no C
counterpart, since a C machine is not saved) and `static_invoke_entry`, which
sits beside the C++ suite's own fixtures and whose child reads in its `<onentry>`
what it was handed, sends its parent from there, and is handed a value that
overflows; the string fixture `static_invoke_string` is driven there too. An
`<invoke>` the host serves is written into the request when the invocation
starts, each `<param>` computed from the machine's fields into a typed local with
the failure flag every operation has: the value crosses as the text of the
request's `params` (`sce_forge_wire_text` — `true` / `false`, decimal digits, a
string as itself) and as a pair of the JSON object the request's `event_data` is
(`sce_forge_wire_pair`, which the `<donedata>` and the `<send>` use), so the
host's own copy of the pairs and the one it forwards are one value rendered twice.
A value that failed is reported and left out of both, and the invocation still
starts with the others (5.7.1); a request whose event data does not fit its buffer
starts nothing. `test_static_host_invoke.c` drives `statechart_static_host_invoke`
for the value on the wire and `static_host_invoke_overflow`, a document of this
channel kept beside its tests, for the pair a failed computation leaves out, and
`test_static_host_send.c` drives `statechart_static_delayed_host_send` for a send
the host serves: its `<param>`s are read when the send is made, the wait carries
that request (`job` 7, not the 8 the field holds when the wait ends), and the
`event_data` is the one text every engine writes, byte for byte.
`statechart_static_host_params`, which has both a send and an invoke, is the
document every channel drives, C11 included (`test_static_host_params.c`, the row
that holds a `float64` on the wire there: the text `1.5` in the request's `params`
and the JSON number `1.5` in its event data). A
`--c-symbol-prefix` build carries the prefix to every symbol a lowered
expression or a host action names — the machine's `_in_state` and
`_raise_platform_error` and their enumerators — while the payload channel's own
tag constants stay `<MACHINE>_PAYLOAD_<EVENT>`.

**Snapshot.** A Kotlin `sce-static` machine publishes what a host observes
as one immutable value, `snapshot: StateFlow<Snapshot>`: the full active
configuration (every active state, each `<parallel>` region included), the
published variables as a `Data` value in declaration order, and `truncated`.
A variable is published by `sce:direction="out"`; `internal`, the default,
keeps it the machine's own — a private field, in no snapshot — so what a
host is written against is what the document chose to show, and an internal
variable can be renamed without breaking it. A document that publishes
nothing snapshots its configuration and `truncated` alone. `sce:direction="in"`
is refused as `scxml/static-datamodel-rule`: it would make the variable the
host's to write, which nothing lowers. It is
published once per completed macrostep, at the W3C SCXML Appendix D point
— inner loop drained, invokes started, just before the next external event —
through the runtime hook `onMacrostepComplete`, and never between two
microsteps. A macrostep stopped at the microstep ceiling still publishes,
with `truncated` set, because the machine moves on and a host that stopped
hearing would hold a stale view.

A Rust machine offers the same value on demand, `engine.snapshot()` through
the generated `<Machine>Observe` trait: an owned `<Machine>Snapshot` of the
configuration (`get_active_states()`), the published variables as
`<Machine>Data`, and `truncated` (`Engine::last_macrostep_truncated`). Rust
needs no hook for it: a host drives the machine with `step()` / `tick()`, and
it can borrow the engine only between two of them — a macrostep boundary —
so no snapshot is ever taken between two microsteps. A host holding one keeps
what it saw, since it owns its copy.

**Saving and restoring.** A host whose process can be killed saves the
machine at a macrostep boundary and restores it into a new process in place
of `initialize`: Kotlin `sm.save()` / `sm.restore(saved)` on a machine not
yet started, Rust `engine.save()` / `Engine::<P>::restore(policy, &saved)`
through the generated `<Machine>Persist` trait. A saved state holds every
variable, the machine's own included, the configuration, the current leaf,
what each `<history>` recorded, the delayed `<send>`s still waiting, the
`<invoke>`s whose child is running, the ones a host is running (with the request
each was started with) and the
external queue in order — only the internal queue is empty at a macrostep
boundary, so an event a host raised and has not yet driven the machine
through is part of the state — as one JSON document (`SavedState::to_json` / `SavedState.toJson`, schema
`schemas/sce-saved-state.v1.schema.json`, a `pre-release` surface in
`SCE_WIRE_CONTRACTS.md`). The document is the same on every backend — keys are
the document's ids, the configuration is in document order, a 64-bit integer
is a text and a real that is not finite is `NaN` / `Infinity` / `-Infinity` —
so what one backend saved another restores; both are held to the shared
instances in `sce-build/tests/fixtures/static_datamodel/saved/`.

What a `<history>` recorded is part of the state because it decides where a
resumed machine goes: `history` is an object keyed by the history's id and
ordered by it, each value the state ids it recorded in document order — the
children of its parent for a shallow history, the atomic states below it for a
deep one (§scxml-3.10). A history that has recorded nothing is absent, so a
machine that resumes through it takes its default transition, as the saved one
would have; a document with no `<history>` writes `{}`. The field is always
present, as `external` is.

**Waiting sends.** A delayed `<send>` that has not been delivered is part of
the state too (§scxml-6.2): a saved machine that left it out would restore
into one that never delivers it. `pending` lists each in the order the machine
would deliver them — earliest first, entries due at the same moment in the
order they were sent — and an entry is one of three acts: `raise`, an event for
this session's external queue; `internal`, one for its internal queue
(`#_internal`); `host`, a request a host-served processor performs
(§scxml-6.2.5), with every field the document wrote. Each carries the send's
`sendid`, so a `<cancel>` still finds it after a restore.

An entry is saved as the moment it comes due on the host's WALL clock (`due`,
a text of milliseconds since the Unix epoch), not as a wait. A wait would start
again when the process came back, and a timer that ran out while it was dead
would be late by exactly as long as it was dead. The engine's own clock is
monotonic and has no epoch, so the host says what time it is on the wall: Rust
`save_at(wall_now_ms)` and `restore_with(policy, &saved, clock, wall_now_ms)`,
Kotlin `save(wallNowMs)` and `restore(saved, wallNowMs)`; the forms without it
read the system's wall clock, and a Rust build for a target that has none
(`wasm32-unknown-unknown`) does not have them. A restore arms each entry to come due
`due - wall_now_ms` after now on the machine's own clock — which is installed
before it, as before `initialize`, so that a delay armed against one clock is
not judged against another — and an entry already due when the machine comes
back is armed to come due now, behind the ones due before it, so the next tick
delivers them in the order the saved machine would have, one macrostep apart. A
host that owns time (`SceClock::Manual`, `ManualClock`) passes whatever its
notion of the wall is. The same run saves the same text on every backend,
`pending` included: `static_timers.json` and `static_timers_midway.json` in the
shared instances are the text each backend writes, and each restores from them.
A `host` entry is held the same way through the machine a generator wrote:
`sce-build/tests/fixtures/host_processor/statechart_static_delayed_host_send.scxml`
and `saved/statechart_static_delayed_host_send_waiting.json` carry a host-served
send armed with `job` 7 and saved after `job` became 8, so a restore is shown to
hand the host the request the document made and not one evaluated again.

**Running invocations.** A child session an `<invoke type="scxml">` started is
part of what a machine is doing (§scxml-6.4: the invocation lives as long as its
state is active), and the process that ran it is gone. A saved state lists the
`<invoke>`s whose child is running as `invokes`, by the id the document gives
each, in document order; a restore starts each again, from the beginning of its
child, under the same id. So `done.invoke.<id>` still names the invocation the
document wrote, and a child's PROGRESS is lost: a machine saved after its child
took one of two events and restored needs that event again. The child's own
delayed `<send>`s are part of that progress, and start over with it. The
children are started after the saved external queue and the waiting sends are
restored, as entering the state starts them — deferred, then run together — so
what a child sends as it starts stands behind what was already queued, and a
child that ends as it starts raises `done.invoke` the way it would have.

"Running" is started and not ended. A child that has ended is absent from
`invokes` — its `done.invoke` is in the saved external queue, or already taken,
and the invocation is complete — so it is not started a second time; the
backends agree on that although the engine keeps an ended child until its state
exits on one and drops it at the macrostep's end on the other. A restore refuses
an id the document does not invoke, one whose state the saved configuration does
not stand in, and one named twice. The field is always present, as `pending` is:
a document with an `<invoke>` that restored from a state with no `invokes` would
stand in "working" with nobody working. `sce-build/tests/fixtures/static_datamodel/static_invoke.scxml`
and `saved/static_invoke_working.json` hold it on both backends.

A hybrid `<invoke>` (§2.13) is listed the same way, by its id, and the candidate
that was running is not recorded: a restore runs the start again, which reads
`srcexpr` from the restored variables and hands every candidate the arguments'
current values, as entering the state would. So a `srcexpr` that reads a
variable changed after the child started names, after a restore, the candidate
that variable names NOW, and the child is handed the values current at the
restore and not those it was first handed — the same loss of progress a static
child has, and one the document can see. `static_invoke_hybrid_saved.scxml` and
its two candidates run all three on both backends: the same candidate again with
a changed argument, the other candidate once `srcexpr` names it, and the
unchanged arguments again, and show a child that ended not started a second
time.

**Running host invocations.** An `<invoke>` a declared host invoker serves
(§scxml-6.4.1) is part of the state for the same reason, and what is lost with
the process that ran it is the host's side of it. A saved state lists each one
as `hostinvokes`, ordered by type and then id, with the REQUEST the host was
handed — `src`, `params` (by name; without the engine's own `_sce_deadline_ms`),
`data` (the namelist and `<param>` pairs as JSON, as the backend built them),
`content` — and `due`, the wall-clock moment its deadline comes due, or `null`
for one without a deadline. Nothing the element reads is evaluated again: the
request is what the document sent, and `job` or `label` may have changed since.

A restore does not call the host. A Rust host registers its invokers on the
engine the restore returns, so at that moment nobody could run one, and Kotlin
does the same so that the two agree: each invocation starts again at the top of
the first macrostep the host drives, where one the document entered would have
started, ahead of the saved external queue. It goes through the one start a
document's own invocation goes through, so the host receives a request with
`restarted` set (`HostInvokeRequest.restarted`; always `false` for a start the
document makes), a NEW token, and the deadline it had left — `due` less the wall
clock the restore was given. A host that kept its own record of the work under
the id may take it up; one that did not takes it as any other start, which is the
right reading. An invoker that is not registered by then is the `error.execution`
a document's own start gets, and a state that leaves before the restart began
cancels it without telling the host anything: the engine never started it.

`hostinvoketoken` carries the token the next start receives, and a restore carries
on from it. A token is distinct for every start within one engine (§scxml-6.4: a
cancelled process's late reply is ignored), and a restored process that counted
from 0 again would give a restarted invocation the token an earlier run already
handed to a host that may still answer with it. With the counter carried on, that
reply is stale. What it cannot cover is a token the earlier process handed out
AFTER the save, which is a fork and not a restore.

A deadline that came due while the machine was away ends the invocation without a
start: the host is not asked to begin what it would be told to stop at once, and
is told nothing — it never began it in this process — while the document receives
`error.invoke.<id>` with `_event.data` `"deadline"`, as an expiry delivers it.
`external[].hostinvoketoken` is the stamp an accepted completion carries. The
engine refuses a host `done.invoke` that has none when it is dequeued, because it
may be a cancelled run's late reply, so a completion that was accepted and is
still queued keeps its stamp across a save — written as it was, never inferred
from the event's name, since an event a host raised through the ordinary door and
the engine had yet to refuse is one the restored machine must still refuse.

"Running" is started and not ended, as above: an invocation whose completion is
already queued is absent from `hostinvokes`. One restored and saved again before
it was driven is still listed, with the same request and deadline, so repeating
the round trip never loses the work. A restore refuses an invocation the
document does not hand to a host under that `(type, id)`, one whose state the
saved configuration does not stand in, one named twice, a `due` that is not a
whole number of milliseconds, and a token that is not a text of digits within a
signed 64-bit count; both fields are always present.
`sce-build/tests/fixtures/host_processor/statechart_static_host_invoke.scxml` and
`saved/statechart_static_host_invoke_running.json` hold it on both backends.

`sendseq` carries how many ids the engine has handed to a `<send>` that asked
for one, and a restore carries on from it. The id is `_auto_send_` and that
count (§scxml-6.2.4: the processor generates an id that is unique among the
sessions' ids and that a later `<cancel>` can name), counted from 1, so the
id is at most 31 bytes and a variable that stores it holds 32. A restored
engine that counted from 0 again would hand a new send the id of one still
waiting in `pending`, and a `<cancel>` of that id would cancel both. The field
is a text of digits within a signed 64-bit count and is always present, as
`hostinvoketoken` is; a state saved before the field existed is not this format.

The request's `event_data` is a JSON object whose members are written in one
order on every backend — ascending by name, whatever order the document declared
its `<param>`s in (ARCHITECTURE.md, "JSON Object Key Order") — so a host that
compares a request as bytes rather than reading it as JSON sees the same text
from any backend. The fixture declares its `<param>`s out of that order on
purpose, so a shared instance being one text shows the rule at work.

A restore runs no `<onentry>` and evaluates no `<data>` (the saved run did
both, and its host calls cannot be made twice), and it is refused, leaving
the machine as it was, for a state saved from a document of another shape, a
configuration that is not one of the document, a value its variable's type
or bound cannot hold, a waiting send that names an event the document does not
name, an invocation the document could not have been running, or a history value the document could not have
recorded: an id it does not declare or names twice, a state it does not name,
none at all, one that is not below the history's parent (or, for a shallow
history, not a child of it), a deep history's state that has children, or
states no configuration could hold at once — a compound state with two active
children, a `<parallel>` with a region missing. The last is the child arity a
whole configuration is held to, applied to the history's subtree
(`helpers::configuration::validate_history`, Kotlin `validateHistory`). The
JSON reader refuses an object that repeats a name, on both backends: one that
took the first and one that took the last would restore two machines from one
text. A save is refused for a machine that is not running and
for one whose last macrostep was `truncated`, and on Kotlin for a machine its
own coroutine drives (`start`): its macrosteps run on another thread while the
host would read it, and its queued events sit in a channel nothing can read
without taking them, so whether a save caught it at a boundary would depend on
timing. The refusal follows the mode the host chose for the run, never the
moment.

A saved state is bound to the document's SHAPE, not its source hash: a
SHA-256 the generator computes over every state with its kind and parent,
every `<history>` with its id, kind and parent, every `<invoke type="scxml">` with
its id and the state that holds it, every hybrid `<invoke>` the same under a name
of its own, every `<invoke>` a declared host invoker
serves with its type as well, and every variable with its type and bound
(`static_lowering::saved_shape`); a document with no `<history>` or `<invoke>`
hashes what it did before either was saved. A guard, an action, an initial
value or a comment changed leaves it restorable, so an app update does not
lose its users' state; a state, history or variable renamed, re-typed,
re-parented or re-bounded refuses it. What the shape cannot see — a
variable of the same name and type whose meaning changed — is the author's to
avoid until a document can declare a migration.

A document that waits on ANOTHER SESSION whose start a restore cannot repeat has
state this version of the format does not carry: a Mesh request (`sce:mesh-rpc`,
whose one request may already have reached its peer and would be acted on twice
if sent again — it is told from a host's own invocation by the type SCE reserves
for it, which a host may not declare), a peer on another device, or a delayed
`<send>` whose target is a `#_`
location other than `#_internal` — the parent, an invocation, a child session —
which is delivered through a session the state does not hold. Such a machine is
generated WITHOUT `save` /
`restore`, so a host finds out when it compiles rather than when a restore
drops part of the state. A `delayexpr` counts as a delay, since the time it
computes is known only when the send runs and may be one that waits; the target
is read as written, or, for a `targetexpr`, as the routes the document declares
as `sce:targets` — one of them a `#_` location other than `#_internal` is a send
that may wait on another session. A send to another
session that is not delayed leaves nothing waiting, and does not take the API
away.
`sce-build/tests/a_machine_waiting_on_another_session_has_no_save_api.rs`
pins this end to end, because the generator decides it from a model the
analyzer has read, and a unit test of the lowering alone cannot see a send the
analyzer found.

**Host actions.** Under `sce-static` a `<sce:action>` argument (§2.11) is
any typed expression over the same scope, not only a bare
`_event.data.<field>`, and its type is the value's
(`InferredType::to_sce_type`; a literal no context typed is `int64` /
`float64`). An argument that reads only the datamodel is admitted where no
event is in scope — `<onentry>`, `<onexit>`, initial content — and one that
reads `_event.data` there is refused as `validation/native-action-argument`,
because no payload is in scope to type it. The host method's parameter types
are the arguments' types, and every call site of one name must agree on them
as it must under any data model.

**The Interpreter does not run it as written.** Generated code is what runs a
`sce-static` document (§3.2 lets a platform decide which data models it
supports). The Interpreter hands every expression to its script engine, which
evaluates it as ECMAScript, so it would give the document a meaning it does not
have: measured 2026-09-30 against the scenarios every backend replays, a
`uint8` at 253 written `level + 3` became 256 where the generated machines keep
253 and raise `error.execution`, a guard over an overflowing sum was true where
theirs is false, and an imported algorithm was an undefined name. The
Interpreter therefore refuses the value at the root, with
`scxml/unsupported-datamodel` (`actual` the value written, `fix.candidates` the
data models it runs: `null`, `ecmascript`), instead of accepting a document and
running it as something else — the counterpart of the generated side's
`generate/unsupported-feature` for a backend that cannot lower the model.
`sce-codegen check` still accepts the document; the two engines are asked
different questions.
`tests/integration/AStaticDatamodelRunsUnderTheInterpreterTest.cpp` holds it
for every `sce-static` statechart under the shared fixture directory.

**It runs once lowered.** `sce-codegen lower <document>` prints the document as
a `datamodel="ecmascript"` one, and the Interpreter runs that. Only the
document's expressions change: each is replaced, at the place the document
wrote it, with the ECMAScript the typed expression means, and every other byte
— comments, extension elements, formatting — is the author's. Each integer
operation is a call of a small library the document's first `<data>` installs
(`SceStatic`), which computes exactly and throws for a result the operand's
type does not hold; the Interpreter's ECMAScript data model turns a throwing
statement into a skipped one and a throwing condition into a false one, each
raising `error.execution`, which is the outcome the generated backends give
the same operation. An integer is an ECMAScript Number where a Number holds it
exactly, up to ±2^53, and a BigInt where it does not, so one value is always one
JavaScript value and `===` between two integers is the comparison the document
wrote; an integer literal past 2^53 is a BigInt literal. A BigInt is a value only
inside the expressions that compute it: the Interpreter's data model holds a
Number and loses a BigInt without a word, so a 64-bit value that leaves an
expression for a variable, an event or a log goes through the library's `out`,
which fails `unrepresentable` for one — the statement is skipped and
`error.execution` is raised, as for an overflow. A 64-bit variable of a lowered
statechart therefore holds ±2^53, and what a statechart computes on the way to
it is exact. The bitwise operators and shifts are the library's, at the width of
the operation, and wrap there (SCE_FORGE.md §3.4.1); a Number's own are 32-bit
and signed. Two defects of the QuickJS the Interpreter embeds are routed round,
and both are measured against Node on the same cases: it orders a BigInt against
a Number wrongly when both are negative, so two integers one of which may be a
BigInt are ordered as BigInts, and `BigInt.asUintN` gives a negative result for
a width of 32 or 64 whose top bit is set, so a result is masked instead.

An event's data reaches the Interpreter as untyped JSON, which a generated
machine reads through the event's schema. So a read of a schema field is a call
of the library's `field`, at the type the schema declares, and it refuses what
the generated machines' lift of a payload refuses — no data, a bare value, a
missing field, a value of another type, a value beyond the field's width — by
throwing, so the expression that read it fails as an overflow does. Four
differences stand, all for a malformed delivery only: a generated machine
lifts the payload once when the event is dequeued and raises `error.execution`
once, where the Interpreter raises one for each expression that reads a field;
a generated machine refuses a delivery that lacks a field of the schema that no
expression reads, where the Interpreter, which reads a field only when an
expression does, runs the transition; a transition that writes one field
before it reads another that does not fit has written the first in the
Interpreter, where a generated machine refused the delivery before any write;
and JSON reaches the Interpreter already parsed, so `5.0` and `5` are one value
there, where the generated lift refuses the first as not a whole number. A
scenario that a malformed delivery is part of reads every field it lacks, and
reads the one that does not fit before it writes any, so that the engines
answer it alike (`static_send_params`, `static_record_real32`).

A list is an array and a record a plain object, and neither is changed in
place: `<sce:append>` becomes the `<assign>` of the list written again with the
value at its end, through the library, which throws for a full one — so nothing
is appended, `error.execution` is raised and the block ends, as on the
generated backends; `<sce:clear>` assigns `[]`; and an `<assign>` to
`record.field` assigns the whole record, written again with that field changed.
The `<data>` of a list or a record is replaced by one that holds its initial
value (an empty array, an object built from its `<sce:set>`s). Those, and
`<sce:append>` and `<sce:clear>`, are the only elements a lowered document does
not keep as the author wrote them.

An imported algorithm travels in the document: the `<data>` that installs the
library installs each algorithm the document calls as `SceStatic.algorithms.<name>`,
and the guard or assignment that calls it calls that. Its body is lowered through
the same expression lowerer and the same typing as the generated backends', so a
name, an operand type and a checked integer operation are judged as they are for
Kotlin; a `may-fail` algorithm's failure, and a `<sce:require>` that does not
hold, are throws, and the expression that called it fails as an overflow of its
own does. An algorithm is lowered with everything it imports: the algorithms it
calls are installed beside it, once each, and the event-schemas its records come
from type its expressions as they do on every backend. A list is an array and a
record a plain object, as in a statechart, and a parameter is read-only: a
`<sce:var type="list<T>">` buffer is an array that starts empty, written again
with each `<sce:append>` and held to its `capacity` (an append past it fails
`capacity-exceeded`, as on the bounded backends; the heap backends grow instead),
a record is built whole by its `<sce:set>`s and changed a field at a time by
`<sce:assign target="r.field">`, which writes the record again, and
`<sce:foreach>` reads each element of a list in order. What is lowered of an
algorithm today is `<sce:var>`, `<sce:assign>`, `<sce:append>`, `<sce:if>`,
`<sce:while>`, `<sce:foreach>`, `<sce:require>`, `<sce:call>` and
`<sce:return>`, over scalar, list, record and `bytes` values. Bytes are an array
of numbers, a `bytes` buffer is appended to as a list is (an append of a `bytes`
value extends it), and a `<sce:const>` table is built when the algorithm is
installed, by the same evaluator every backend's is.

The pairs of a `<send>` and of a `<donedata>` are expressions the lowering
rewrites where each is written, and the text of an inline `<content>` is
finished to the string it spells (above, **Params**). A child session written
inline in an `<invoke type="scxml">` is a `sce-static` document of its own and is
lowered as one, where it stands in its parent's text: the values the `<invoke>`
hands it are lowered `<param>` expressions, and a string the child bounds is
handed through that bound (`SceStatic.bounded`), so that a value past it fails and
is left out as on the generated backends, with an `error.execution`. A child
under another data model is left as it was written.

A hybrid `<invoke>` (§2.13) starts one of the documents its `sce:candidates`
declares, which the Interpreter loads at run time from beside the invoking
document, so its lowering is a set of documents and not one: the invoking
document and each candidate, lowered as a document of its own and written under
its stem, and the candidates of those in turn. `sce-codegen lower <document>
--out-dir <dir>` writes the set and prints one JSON line naming the files, the
document asked for first; without `--out-dir` a document of the kind is refused,
naming the option, since there is no one document to print. The `srcexpr`
becomes `SceStatic.candidate(<the value>, [<the stems>])`, which reduces the
value to its stem by the one table every engine reads
(`tests/document_stem/document_stem.json`) and answers `<stem>.scxml`, the file
name the lowered candidate is written under; a value that names none of the
stems throws, so the attribute cannot be evaluated, `error.execution` is raised
and nothing starts, as on the generated backends. The `<param>`s and the
`namelist` go to every candidate, each keeping the names it declares, and the
Interpreter carries ONE expression in an attribute, so an argument the
candidates take as variables of different types or bounds — one candidate
bounding a string another bounds otherwise or leaves unbounded — has no single
lowering and is refused by name, as is a `namelist` name that lands in a string a
candidate bounds. Two candidates of one stem, however far apart in the set, are
refused: the stem is the name each is written under.

A construct with no lowering yet — a `bytes` literal, a top-level `<script>`, an
`<invoke>` that is not an inline child or a hybrid one (by `src`, a mesh or a
host-run one) or whose `namelist` hands a child a string it bounds, a `<donedata>`
or a `<send>` whose `<content>` is an expression or holds an element — is
refused with `generate/unsupported-feature` naming it, never passed through half
lowered.
`<sce:action>` is lowered (§2.11): its `<sce:arg>` expressions are lowered as
any expression is, and the action is performed by the host installed on the
machine.
`tests/integration/AStaticDatamodelRunsLoweredUnderTheInterpreterTest.cpp`
replays the scenarios the generated backends replay
(`sce-build/tests/fixtures/static_datamodel/scenarios/*.json`) against the
lowered documents, so one oracle judges the engines; the four fixtures that
invoke a child have no scenario of their own, and the same file runs each of them
under the Interpreter and holds it to what the generated backends'
`a_static_child_is_handed_its_params` and its siblings hold them to — the fourth,
`static_invoke_hybrid`, from the directory `lower --out-dir` writes. The same
file runs the table of document stems through the library's `candidate`.

**An algorithm on its own.** `sce-codegen lower-algorithm <document>` lowers one
`sce:kind="algorithm"` document without a statechart around it — an
`sce:std/...` name or a path — and prints one JSON line naming the symbol and the
expression that installs the library and the algorithm. It is refused as an
import of it would be, and for the same reasons. A failure throws an Error whose
`sceFailure` is the name a generated backend reports (`overflow`,
`divide-by-zero`, `precondition`, `out-of-range`, `capacity-exceeded`), so a
caller compares failures by name and not by message. A signed minimum divided by
`-1` fails `overflow` and so does its remainder, which is `0` mathematically:
SCE_FORGE.md §3.4.1 makes the pair one answer on every backend. The one name no
backend has is `unrepresentable`, for an integer the Interpreter's data model
cannot hold, which is one beyond the 2^53 a Number holds exactly handed to a
variable or an event, or a Number that is not an integer a Number holds exactly
used as an operand; inside the algorithm the integers are exact to the width of
their type. A BigInt is how the algorithm holds one beyond 2^53, so an argument
or an answer past it is a BigInt, which the differential check writes as a BigInt
literal and reads back from a marker object.
`tests/integration/AnAlgorithmRunsLoweredUnderTheInterpreterTest.cpp` holds every
algorithm fixture of the conformance catalog to the cases the six backends are
held to (`tests/forge/conformance/numerical_reference.json`). A fixture the
lowering refuses is reported with the construct it names, a case that fails
`unrepresentable` is counted and not compared, and a case a value of the
reference has no form for is not asked; each count is printed, so a comparison
that stopped being made shows as a number that fell.

### §2.16 A closed interface — `sce:interface="closed"`

An event-schema types the payload of the one event it names; an event no
imported schema names keeps the dynamic `_event.data` baseline with no
diagnostic (EventSchema kind, "Schemaless fallback"). That stays the
default, and it means nothing holds a statechart to the interface its owner
accepted. A statechart root may instead declare
`<scxml sce:interface="closed">`, and is then held to the event-schemas it
imports (`<sce:import kind="event-schema">`) in both directions:

| What crosses | Admitted when |
|---|---|
| an event a transition takes | a descriptor that matches an event some imported schema declares (W3C SCXML 3.12.1 matching, so `coin` admits `coin.inserted`), an event the statechart gives itself on its INTERNAL queue (a `<raise>`, or a `<send>` to `#_internal`), or a platform event (`error.*`, `done.state.*`, `done.invoke.*`); `*` declares nothing and is admitted. A `<send>` to the session itself with no `target` is NOT an event it gives itself in this sense: W3C SCXML 6.2.4 puts it on the EXTERNAL queue, the one a caller delivers to, so a caller can send the same name, and a transition that takes it is refused as `scxml/undeclared-interface-event` with the reason (`ReceivesSelfSent`). Declare the event in an imported schema, or send it to `#_internal` |
| a `<send>` out of the session | its `event` is exactly an event an imported schema declares |
| a `<send>` with `eventexpr` | never: a computed name cannot be checked, so name the event |
| a `<send>` to the session itself (no `target`/`targetexpr`/`typeexpr`, or `#_internal`, through the SCXML Event I/O Processor) | some transition takes its event — otherwise it is a message the machine sends itself and discards, which is what an output with no destination looks like |

The first event that breaks the rule is refused as
`scxml/undeclared-interface-event`, naming the state and the event, and
listing in its message every event the imported schemas declare. The only other value
of the attribute is its absence; anything else is refused as
`validation/attribute-rule-violated`. A forge root does not take it.

What the rule buys is a property, not a list: on a closed statechart that
builds, **the events a caller can deliver are the events its imported schemas
declare** (the Rust backend's `EXTERNALLY_DRIVABLE_EVENTS`, from the model's
`externally_drivable_events` in `apis/forge-ast.v1.schema.json`), plus the
platform's own. A timer or a retry the machine sends itself is the
case that broke it: sent with no `target` it travels the external queue, so it
was both "the statechart's own" to this check and a name any caller could send
to skip a wait. Measured 2026-09-30 on a retry client (three 200 ms deadlines,
each taken by a transition): with the deadlines sent without a target the
closed document was accepted and its drivable events were the two declared
inputs and `internal.deadline1..3`; with `target="#_internal"` it is accepted
and they are the two declared inputs alone. ⚠ It does not judge intent: an
owner who wants a caller able to fire the timer declares it, and the machine
then says so in that constant.

An event that carries no data is declared by a fieldless schema (EventSchema
kind, "Fieldless schema"), not left out of the interface. An imported schema
the seam could not read — missing, unreadable, or itself refused — is
reported in its own words, at its own file and row, and not as the interface
declaring nothing: a closed document is judged against schemas that read.

The default interface is open and stays it, and it is not refused. But a
statechart that imports event-schemas and does not declare its interface
closed is holding schemas that describe a boundary nothing holds it to, so the
run says so: the manifest's `open` (and the acceptance report's block B and
record) carries an `interface` line naming the imported aliases and the
repair — declare it closed, or tell the owner the boundary is open. A
statechart with no schema says nothing, since nothing tells the product it was
meant to have one, and a statechart that declares it closed says nothing
because it is. Measured 2026-09-30: a draft handed on as passing had its
`sce:interface="closed"` removed to get past a check, every import still in
place, and its answer said nothing of the change.

A closed interface does not say that a CALLER may send an event the machine
also sends itself, and the owner is the one to say it. A `<send>` to the
session with no `target` goes to the external queue (W3C SCXML 6.2.4), so an
event a transition takes and the machine sends itself that way is one a caller
can send too, and for a timer that is a way to skip the wait. Declaring it in an
imported schema is one of the two repairs the closed rule names, and it makes
the timer a listed input of the interface with nothing said about it. So the
manifest's `open` carries a `self-delivered` line for every statechart, open or
closed, naming those events (`open_matters::self_delivered_events`: literal
names, sent to itself with no `target`, taken by a transition or an
autoforwarded child, not also put on the internal queue) and the two ways
out: send each with `target="#_internal"`, or the owner says callers may. A send
to `#_internal` and a `<raise>` are the internal queue and are not listed. The
acceptance report shows it as `caller sends` and the record keeps it in
`open_at_acceptance`, so an accepted design says what a caller could do to it.
Measured 2026-09-30, fifteen drafts by a real client under a closed-interface
profile: fourteen declared a timer they send themselves as an input event, and
the answer to the owner said nothing of it.

It is judged where a statechart's imports are read, the parser's import
seam, so every entry point that parses a document from a file judges it.
The in-memory path, which reads no sibling documents and so resolves no
schema, does not judge it — as it does not judge the typed payload paths
that seam validates.

Measured 2026-09-29, before the rule existed: five drafts of one
specification whose prose left the interface open invented five
interfaces — where a price came from, what an input event was called and
what it carried — and all five wrote their outputs as `<send>`s to the
session itself. The declaration is how an interface the owner accepted
first, as its own event-schema documents, holds every later draft of the
behaviour to one boundary. No tracked document declares it, so the rule
moves no existing verdict.

### §2.17 An authoring profile — `--profile`

SCE judges a document by its grammar and by what SCXML means, and it cannot
know what its author was asked for. A statechart with no event-schema and no
`sce:interface` is a W3C conformance document, a legacy machine, or a design
about to be shown to an owner, and nothing in it says which. Reading intent
off a proxy — a recorded kind basis, an import — attributes an expectation to
a feature that means something else, so intent is stated instead, in a file
the owner keeps beside the specification:

```json
{ "record": "sce-authoring-profile", "v": 1, "name": "owner-review",
  "interface": "closed", "evidence": "anchored",
  "names": { "state": { "style": "snake" },
             "event": { "style": "snake", "tokens": { "min": 2, "max": 3 }, "prefix_free": true } },
  "guidance": ["Ask before writing."] }
```

| Field | Class | Meaning |
|---|---|---|
| `record`, `v` | | `"sce-authoring-profile"` and `1`. Read and checked before any setting, so a profile from a newer tool is refused for its version. |
| `name` | | A label a report may print. Configures nothing, and is part of the file's bytes like every other character. |
| `interface` | enforced | `"closed"`: every statechart declares `sce:interface="closed"` (§2.16). Absent: the interface is not constrained. |
| `names` | enforced | How the names a document defines are spelled, by class of name. See below. |
| `evidence` | enforced | `"anchored"`: every `<sce:evidence>` of the `<sce:kind-basis>` carries a `provenance` anchor. An unanchored one is refused as `profile/evidence-unanchored`, quoting the evidence. |
| `traceability` | enforced | `"required"`: every state and every transition claims a requirement (`sce:req`). One that claims none is refused as `profile/element-untraced`, naming it. See below. |
| `house_rules` | reported | The owner's standing answers to gaps that recur, each `{id, rule}` and, where the owner's words were kept, `quote` and `confirmation`. A draft that applies one cites it, `sce:assumed="<id>"`; every citation is listed. See below. |
| `guidance` | guidance | Instructions to whoever writes the document, handed over as written. Nothing checks them. |

Every setting belongs to one class and the schema fixes it, not the file:
**enforced** (a draft that breaks it is refused), **reported** (a departure is
listed and the owner decides) or **guidance** (handed to the author, checked by
nothing, and said so). The one reported setting is `house_rules`; a term
dictionary would be another, since which term a name stands for is a reading of
the prose that can be listed and not refused. A profile that names a setting this build does not
know, a value a setting does not take, another `record`, or a version it does
not read is refused **whole** as `cli/profile-unusable`; applying the part that
was understood would say a document was held to a profile it was not. The
record is keyed on which refusal it is, and the path the caller typed is its
`actual`. A setting that is present says something: an empty `names`, a rule
with no entry, a list with nothing in it or a range that runs backwards is
refused the same way, by the reader and by the schema.

`--profile <PATH>` is taken by `check` (one document and a document set),
`generate` and `orchestrate`, which judge the same way — `orchestrate` is the
producer a set-route `check` predicts, and `cli_orchestrate_check_parity`
holds the two flag lists together. A statechart that is valid and is not what
the profile asks for is refused, and every finding of every statechart is
listed, since the owner is deciding about the whole design: in the order the
settings are described (the interface, the names by class, the evidence). On a
set the profile is judged after `--strict-unresolved` and before `--lint`, the
order a single document asks them in, and a statechart that does not read is
left to the compile that follows. It is judged on the model as parsed, before
the analyzer, so a design the profile refuses generates nothing.

`profile/interface-not-closed` names the statechart, the profile's `name` when
it has one, and the event-schemas it imports and so describes a boundary with.

#### The `names` setting

Four classes of name are the ones a document introduces: `document` (the `name`
of the `<scxml>` root), `state` (the id of a state, parallel, final or history),
`event` (an event the document raises, sends or takes by a literal name) and
`data` (a `<data>` id). Each takes a rule with any of `style`, `max_length`,
`forbidden_words` and `required_prefix`; `event` takes in addition `tokens`
(how many dot-separated tokens, `min` and `max`), `first_tokens` (the tokens a
name may begin with) and `prefix_free`.

What a document does not choose is not judged. An event an imported
event-schema declares is a fact about the platform the schema describes, and
is taken as it is, wherever the style would have spelled it otherwise. The
platform's own events (`error.*`, `done.state.*`, `done.invoke.*`), any name
that begins with an underscore, and a descriptor that is a pattern (`door.*`,
which matches events and declares none, §3.12.1) are nobody's choice either.

A style is ASCII and is applied to one token: a state id, a data id, the
document name, or each dot-separated token of an event name. `snake` is
`door_open`, `upper_snake` `DOOR_OPEN`, `camel` `doorOpen`, `pascal`
`DoorOpen`, `kebab` `door-open` and `lower` `dooropen`. A capital letter in
`camel` and `pascal` starts a word and is followed by a lower-case letter or a
digit, so an acronym is a word (`doorHttp`, not `doorHTTP`), which is what
makes the rule decidable and also means a one-letter word after the first has
no camel or Pascal spelling. A forbidden word is compared with the words a name
is made of, without regard to case, so `door`, `Door` and `DOOR_OPEN` all
contain `door`. The respelling a style implies is in the message when the style
accepts it, and only then.

`prefix_free` is a correctness rule in the form of a naming rule. W3C SCXML
3.12.1 matches an event descriptor by token prefix, so a transition on `door`
also takes `door.open`. With the rule on, no literal event name of the
document, and no name its event-schemas declare, is a token prefix of another;
the finding is made once per pair, on the shorter name.

The four codes are `profile/name-style`, `profile/name-limit` (a forbidden word,
a length, a prefix), `profile/event-structure` (the token count or the first
token) and `profile/event-prefix-of-another`, each keyed on the class, the name
and the rule and never on the profile's label, and none carrying a `fix`: the
repair renames a definition and everything that refers to it, an edit at more
than one place that no single `fix` locates.

A spelling is the part of a naming preference a machine can hold every draft
to, and it is only that part. Which word a draft uses for a concept — `open
request`, `request.open`, `hold elapsed`, `auto close` for one clause — is a
reading of the prose, and no setting here decides it. Measured 2026-09-30, five
drafts of one door specification under a profile of spellings all kept to it,
and the number of classes the drafts fall into did not fall (the words parted
where the spellings had): a naming rule makes drafts agree on how a name is
written, not on which name.

#### The `traceability` setting

`"traceability": "required"` asks that each state and each transition of a
statechart claim a requirement (`sce:req`), so that the design says, element by
element, which sentence of the specification it is there for. An element that
claims none is refused as `profile/element-untraced`, once per element, naming
the state or the transition (its state, its event, its target) and placed where
it is written. Which id is the right one is a reading of the specification that
only its author makes, so the refusal offers no fix; `scxml_requirement_set`
(`tools/authoring`) makes the ids from the words of the specification a client
quotes.

It exists because an instruction did not do it. Measured 2026-10-01, fifteen
drafts by a real client under a closed-interface profile, the server's
instructions telling it to put each requirement's id on the element that carries
it: none did, and none named the tool that lists them, because the request asked
for a kind, a draft, a check and what is left open, and that is what it did. An
owner who wants each sentence found in the design says so here, and a draft that
ignores it is refused. Absent, nothing changes: a statechart with no `sce:req`
is as accepted as it was.

It reads states and transitions and NOT what runs inside them. A transition's
own actions do not inherit its `sce:req` (an `<onentry>`'s do), so the
acceptance report's table counts a `<cancel>` inside a claimed transition as
claiming nothing (`requirements_report.rs`, "The unclaimed block dilutes"; on one
real document its unclaimed block went from 2 to 7 with no new behaviour), and a
rule over every node would refuse a design that claims every sentence. A state
and a transition are the units that do not dilute. The rule says an element
CLAIMS a requirement and never that the claim is right: whether the id belongs
on it is what the acceptance report puts beside the sentence for the owner to
judge, and whether every requirement is claimed is `scxml_requirements` with the
specification's manifest.

#### The `house_rules` setting

A house rule is the owner's answer to a gap that recurs across specifications,
decided once: *an event a state does not mention is ignored*; *the initial
state is the first condition the specification lists*. A draft that meets such
a gap does not ask, and does not guess; it applies the rule and cites it by its
id on the element it applies to — `sce:assumed="H1"`. The product reads the
citation and nothing else about the rule, which is prose.

A citation is the owner's standing answer and not a value chosen without one,
so it is said apart: the manifest's `open` carries a `house-rule` entry (the
places, and how many times each rule was applied), the marker's record in
`unresolved` and in `sce-codegen unresolved --profile` carries `house_rule:
true`, and an acceptance record keeps the entry as `house-rule` rather than as
an assumed value. Only an `sce:assumed` counts: an `sce:unresolved` that names
a rule's id is still a question. A run given no profile cannot tell a rule's id
from any other and reports every `sce:assumed` as before. An id names one rule
(a profile that lists it twice is refused whole, since a draft citing it could
not say which it applied), and a decision of the same id in the owner's
decision record wins over a rule for its clause.

What this cannot see is a rule applied without its citation: a draft that
ignores an unmentioned event and says nothing has applied the rule silently, and
no check reads a document's behaviour against prose. It is the same limit the
decision record has, and the authoring core's `decisions` check is what licenses
a citation — it refuses an `sce:assumed` that cites neither a decision nor one of
the profile's rules.

So the manifest says what it can see about it: under a profile that holds house
rules, `profile.house_rules` is `{held, cited}`, the rules the profile holds and how
many distinct ones the run cites (a rule applied at three places is one; an id the
profile does not hold is none). `cited: 0` of `held: 3` is an explicit zero, because
a design that applied every rule and cited none reads, in the rest of the manifest,
as one that applied none, and an acceptance of it records no applied rule. Measured
2026-10-02, eight drafts under a profile of three rules all applied them and none
cited one; once the check's answer said `held 3, cited 0`, all eight cited, each on
the element the rule applies to. The product still cannot say that a citation is
RIGHT.

A forge document is judged only where a setting reaches its kind. `evidence`
reaches every kind that states a kind basis. `names` reaches an event-schema
document: the event it declares (`sce:event-name`) is a name of the `event`
class, judged by the same rule, `tokens` and `first_tokens` included, and the
ids of its fields are names of the `data` class. That is where a boundary event
is spelled once: a statechart takes the name from the schema it imports and is
not judged for it, so a rule that reached only the statechart would leave the
names an interface is made of outside the profile. The other kinds have names
of their own — a transform's outputs, a codec's fields — that no setting
judges yet. A finding in a forge document names the document and no row, since
the model keeps none for an event's name, a field or an evidence.

The manifest's optional `profile` object carries the profile's `name`, its
`sha256`, `judged` (the number of documents it was applied to: the ones it asks
something of — a statechart when some setting reaches statecharts, a forge
document when some setting reaches its kind) and `guidance` (how many
instructions it handed over, none of which was checked; omitted when there are
none). A profile of house rules and guidance alone asks nothing of any document,
so a run under it, and a run whose documents no setting reaches, report
`judged: 0` and are not read as a pass; a run given no profile omits the
object, and one that failed the profile emits no manifest. The in-memory path,
which reads no sibling documents, takes no profile. `prefix_free` is judged
over the names a statechart defines and the event-schemas it imports, so it
needs the statechart: a schema judged alone has one event and no other to be a
prefix of.

An acceptance record pins the profile beside the specification and the
decision record, under the role `profile`, and holds it to the same rules: at
most one, compared by content, and a role left out is part of the answer — a
design accepted under no profile does not answer for a request that names one,
and the reverse. `accept --profile` judges the design first and refuses a
statechart that departs from the profile, since an acceptance is the owner's
statement that this design is what they accept. `acceptance-check --profile`
asks whether the record was taken under this profile. A design is *held to* a
profile rather than authored from it, and the lapse sentences say so.

The record also keeps WHICH house rules the design applied, as `applied_rules`:
each rule the design cites (`sce:assumed="H1"` that the profile holds), with the
text the profile gave it when the design was accepted and the number of places
that cite it, sorted by id; a rule the profile holds and the design never cites is
not there, and neither is an id the profile does not hold. Optional, omitted when
empty, and refused when read if no profile is pinned for the rules to have come
from, if a rule is named twice, or at no place. It is the content a reader of the
acceptance needs without asking for the profile. And when the profile's bytes move
and it still loads, the lapse names each applied rule that moved, beside the
profile's own lapse: `house rule H1, which the design applied at 3 places, now says
"…"; it said "…" when the design was accepted`, or that it is no longer in the
profile. An edit to a rule the design never applied lapses the profile and names no
rule, and a profile that no longer loads names none either, since a rule it cannot
be read for is not said to be unchanged.

A rule may say where it came from. `quote` is the owner's own words, copied as they
were said; `confirmation` is `relayed` where a client reported that the owner said
yes to the rule as worded. The word is modest on purpose: the product was not in
the conversation, so it records a client's report and never "the owner confirmed".
A rule written into the profile by hand carries neither, and is never read as
confirmed. `confirmation` needs the `quote` (a yes says what it was a yes to), and
an empty `quote` is refused. Each applied rule in the acceptance record repeats both
(`applied_rules[].quote`, `.confirmation`, omitted when absent), so the acceptance
says on what authority the design applied it. They are recorded as the rule stood
at acceptance and are not compared afterwards: the lapse of a rule is about what it
says. The authoring server's `scxml_house_rule` makes such rules from the owner's
words and saves nothing until the client reports the owner's yes. On a local server
it can then write the profile (`out`): whole or not at all (a temporary file renamed
over the target), and over a file already there only when that file still holds the
bytes the rules were added to, so an edit made since is never overwritten. The
profile's revision is its digest, which the acceptance record already pins; no
counter is kept beside it.

#### Which acceptances a change touches

A profile is shared by every specification an owner keeps, and editing a rule raises
a question `acceptance-check` answers one record at a time. `sce-codegen
acceptance-impact RECORD... --root DIR` asks it of many: each record is rechecked
for the variant it was taken for, and the answer is one JSON line per record
(`record`, `variant`, `holds`, and `lapses`), then a summary line (`records`,
`holding`, `lapsed`, `unusable`). A lapse is data beside its sentence: `kind`
(`variant`, `manifest`, `missing`, `moved`, `added`, `unparseable`, `source`, `rule`,
`not-authored-from`), the fields of that kind under their own names, and `message`,
the sentence `acceptance-check` would have printed. For a house rule that is `id`,
`places`, `recorded` and `current` (null when the rule is gone), so a consumer finds
the designs that applied a changed rule without reading an id out of prose. A design
accepted under the profile that did not apply the changed rule still lapses as
`source`, since the file it was accepted under is other bytes; one accepted under no
profile holds. It is a report: exit 0 whenever it ran, and a record that cannot be
read is a line of its own (`unusable`, with `kind` and `detail`) that does not end
the scan. The authoring server offers it, on a local server only, as
`scxml_acceptance_impact`, which also gathers the rule lapses by rule id.

The scenario set (§2.18) is pinned the same way under the role `examples`:
`accept --scenarios` and `acceptance-check --scenarios`, at most one, compared by
content, a role left out being part of the answer. The examples whose passing
closed a requirement (`scenario-passed`, §2.19) are part of what the owner accepted
the design WITH, and a set edited afterwards is another set. The record pins the
set's bytes and says nothing of how its scenarios came out: that is a run's, on an
engine, asked again by `requirements --scenarios --trace`. A file that is no
scenario set is refused as `cli/closure-input-unusable` and not pinned; a set with
problems is a finding, not a refusal.

What a profile cannot configure: the obligation to mark a guess, W3C SCXML
semantics, the kind catalog and `<sce:kind-basis>`, and acceptance by a person.
No setting is named so that it could. The open-interface line of §2.16 stays
as it is — it says a mismatch the document itself shows, where the profile
states what the owner expects.

### §2.18 A scenario set — `sce-codegen scenarios`

Requirement closure asks whether a node claims a requirement's id, which is
evidence only for a requirement met by something existing. A requirement met by
something NOT happening has no honest answer among its outcomes, and
`needs-scenario` says what would carry it: a scenario asserting the thing does
not occur, and passing. A scenario set is the input of that column: examples of
what a specification says a machine does, in the owner's words, kept in a file
beside the specification.

```json
{ "record": "sce-scenario-set", "v": 1,
  "specification": { "doc_id": "retry-client", "rev": "1" },
  "origin": "ai-proposed",
  "interface": { "inputs": [{ "name": "RequestNeeded" }],
                 "outputs": [{ "name": "SendRequest" }] },
  "scenarios": [
    { "id": "T2-boundary",
      "quote": "If 200ms pass without a response, it sends SendRequest again.",
      "steps": [
        { "send": "RequestNeeded", "expect": { "outbound": [{ "event": "SendRequest" }] } },
        { "advance_ms": 199, "expect": { "outbound": [] } },
        { "advance_ms": 1,   "expect": { "outbound": [{ "event": "SendRequest" }] } } ] } ] }
```

A step sends one event, or lets virtual time pass, or does neither and only
observes (step 0 is judged on the initial configuration, so what the start
itself sent is part of it). What is observed is a closed list: `outbound` (every
event the machine sent to something outside itself during the step, in order;
`[]` means none), `finished`, `condition` (a state the specification names) and
`data` (a data item the specification names). Anything left out is not checked.
Time is virtual and never wall-clock.

The `interface` is the part a specification rarely supplies. It names the events
sent in, the outputs, the payload fields and their types, and the conditions and
data items an example may mention, and it is a PROPOSAL the owner accepts with
the examples. Measured on four specifications (door, connection, vending,
retry), only the retry specification named its signals; the others left the
inputs, the outputs, or both for the example to invent. An output may carry the
`via` route it leaves by; one that carries none is an output whose route nobody
has decided, and an engine cannot observe it — which is correct, because under
W3C SCXML a send to a target that cannot be reached is `error.communication`,
not a delivery.

A scenario that cannot simply run says why rather than being dropped.
`assumes` lists what the example takes as given that the specification does not
state (the condition the machine starts in). `awaiting-decision` is a question
the specification leaves open, put as an example, with the expectation written
for each answer (`decision.alternatives` names the scenarios of which one is
kept). `blocked` waits on a fact nobody has decided (`blocked_by`). A sentence
that claims something no run can show, such as "indefinitely", carries a
`bound`, and a result says it was bounded.

`sce-codegen scenarios <file> [--specification <path>]` reads a set and writes
a summary record, a record per scenario and a record per problem, one JSON
object per line. The problems are findings: the command exits 0 with them and
the summary's `usable` says whether there were any. It reports every problem at
once (`PROBLEM_CODES` in `sce-build/src/scenario_set.rs` lists them): a name used
that the interface does not declare, a payload of the wrong type or missing a
field, a status that does not agree with what the scenario carries, a runnable
scenario that asserts nothing (it would pass whatever the machine did), and,
with `--specification`, a quote that is not in the specification word for word
(runs of white space are one space; nothing else is forgiven). A file that is
not a scenario set at all (not JSON, another `record`, another `v`, a field this
build does not know) is refused as `cli/closure-input-unusable` with `what` set
to `scenario set`, and prints nothing.

`origin` is a declaration (`ai-proposed` or `owner-written`) that SCE cannot
verify and repeats in every result. This surface runs nothing and says nothing
about any design: which engine drives a machine is not part of it, and the
judgement of what an engine observed against these expectations is §2.19. The
sets shipped in `sce-build/tests/fixtures/scenario_sets/` (34 scenarios) are
held to the schema, read, counted and quoted against their specifications by
`every_set_the_product_reads_validates_against_the_wire_schema`.

### §2.19 An observation trace and its judgement — `sce-codegen judge-scenarios`

A scenario set says what a machine is expected to do. What it did is observed by
a driver: a program separate from SCE that runs a design on an engine, sends the
events and lets the virtual time pass as each scenario says, and writes an
observation trace. The comparison is made here, once, so that a second driver on
a second engine only writes traces and cannot disagree with the first about what
a result means.

```json
{ "record": "sce-observation-trace", "v": 1,
  "engine": { "name": "python-lowering" },
  "observes": { "outbound": true, "finished": true, "configuration": true,
                "data": { "unavailable": "the design declares no data" } },
  "runs": [
    { "scenario": "T2-boundary",
      "observations": [
        { "outbound": [{ "event": "SendRequest" }], "finished": false, "configuration": ["waiting"] },
        { "outbound": [], "finished": false, "configuration": ["waiting"] },
        { "outbound": [{ "event": "SendRequest" }], "finished": false, "configuration": ["waiting"] } ] } ] }
```

There is one observation per step of the scenario. `outbound` lists the events
the machine sent outward DURING the step, in order, with their payload when they
have one, and is empty when there were none. `finished` and `configuration` (every
active state with its ancestors) are the state AFTER the step. `data` holds the
names the driver could read. `observes` says which of the four a driver can see
at all, and a channel that is observed is present in every observation. A run the
driver did not make says why (`refused`) instead of carrying observations.

A data name left out of an observation's `data` is a gap whatever the cause. A
driver that knows the cause says it once, at the top of the trace, in
`unreadable` (`{"count": "the generator gives it no reader"}`), and the gap for
that name ends with the driver's words. Without it the gap says only that the
name could not be read, which sends an owner to look for a fault the driver
already knew about.

A refusal says whether another machine would refuse the same run. `refused`
may carry a `cause`: `design` when what the design did made the example
unplayable (an error no state answered, an open route, a macrostep the engine
cut short, a design that would not start), `environment` when the machine that
ran it did (time, memory, a crash, a kill), `decision` when the design leaves
open a question the example needs answered. The last is judged `blocked`, not
failed and not a gap, with the decision named: it is neither the design's
defect nor a fact about the machine, and it ends when the owner answers. The
verdict and gap records repeat the cause, so a client can tell a design that
never settles from a run that was only slow. A refusal that names no cause is
reported with none, and is not read as
`design`. The trace may also name the `limits` the driver ran under (events,
instructions, seconds, bytes) and the `isolation` it was kept in; the summary
repeats both unread, so a verdict states the bound it was made under.

`sce-codegen judge-scenarios <set> <trace>` writes a summary record, a record per
scenario (`pass`, `fail`, `not-judged`, `blocked`, `awaiting-decision`), and a
record for each failed check, each gap and each problem. The comparison:
`outbound` is the same events in the same order and number, an expected payload
is a subset of what was sent, and an expected event with no payload checks none;
`finished` is equal; `condition` is a member of `configuration`; `data` is equal
as a number, text or boolean, and 3 and 3.0 are one value.

`not-judged` is not `fail`, and the judge keeps them apart on purpose. A driver
that cannot see a channel must not make a design look wrong, and must not make it
look right: a scenario that asks about a channel declared unavailable is
`not-judged` with the driver's reason, and so is one whose run was refused or
absent, or whose trace is defective (a channel declared observed and missing from
a step). A check that WAS observed and did not hold is a `fail` even when other
checks of the same scenario could not be judged: a failure is conclusive and a
gap is not. A scenario that is `blocked` or `awaiting-decision` is never judged,
whatever the trace holds. A set that has problems, or a trace that names (by
`scenario_set.sha256`) another set than the one given, judges nothing, and
the summary's `judged` is false: a verdict from either would be about nothing.

The summary carries the engine's name, the `origin` of the examples and the
digest of the set, and says what a pass means: that the machine behaved as these
examples say, on this engine, over these inputs. It does not say the design is
right, it does not say the examples are the owner's, and a scenario with a
`bound` passed only up to it. The command exits 0 with whatever it finds; a file
that is not a scenario set, or not an observation trace, is refused as
`cli/closure-input-unusable` and prints nothing.

#### The names, held to the design

A scenario set proposes the names its examples are written against (its
`interface`), and the owner accepts them with the examples. A design is drafted
separately, from the same prose, and nothing held the two to each other: measured
2026-09-29, five drafts of one specification invented five interfaces, and an
example that names a state the design calls something else FAILED, which reads as
the design misbehaving when it is two names for one thing.

So the design's half is published. `sce-codegen check` and `generate` carry
`surface` on their manifest for a single statechart (omitted for a document set,
whose union is nobody's interface): the events a caller can deliver (the §2.16
set), the events a `<send>` puts outside the session by their literal names, its
states and its data. Each is the analyzer's own fact, not a second reading of the
document; `computed_outputs` says a send names its event by expression, so the
outputs a design never sends cannot be listed, and `takes_any_input` says a
transition takes `*`. A driver copies the surface into the trace (`surface`, the
same shape), and the judge compares it with the set's `interface` in both
directions, as one `interface` record after the summary:

- `unserved_inputs` / `unaccepted_inputs`: an input the examples send that no event
  the design takes answers, and an event the design takes that no input names.
  Inputs match as event descriptors do (W3C SCXML 3.12.1: a transition on `coin`
  takes `coin.inserted`, and not the other way round).
- `unsent_outputs` / `unaccepted_outputs`: outputs the examples name that the
  design never sends, and events it sends that the interface does not name. A
  design with a computed send has no `unsent_outputs`, since it may send any.
- `missing_conditions` / `missing_data`: states and data items the interface names
  that the design does not have.
- `matches`: nothing differs and nothing is unknown. `checked: false` (with `why`)
  is a trace that carried no surface: no comparison was made, and silence would
  read as a match.

One verdict moves. An example that expects a `condition` the design has no state
for is `not-judged` (cause `design`), not `fail`: a state called something else is
a name, not behaviour, and the gap lists the states the design has. Everything else
is reported and changes no verdict: an output that is never sent still fails the
example that expects it, because a design that leaves it out is a defect and not a
spelling.

#### A played example closes the requirement it names

`needs-scenario` (what a `shall_not` requirement reads as, §2.18) says what would
carry a requirement met by something NOT happening: a scenario asserting it does not
occur, and passing. A
scenario names the requirements it is about (`requirements`), and the judge carries
those ids on each verdict. `sce-codegen requirements <document>... --manifest <list>
--scenarios <set> --trace <trace>` holds the list against the examples: the set is
judged HERE, from the trace, and not read from a verdict file, so a requirement is
never closed by a verdict somebody typed. The flags come as a pair and need the list.

Per requirement, over every scenario that names it:

- one scenario `fail` -> `scenario-failed`, for any requirement the document is meant
  to carry (`implemented`, `unresolved`, `missing` or `needs-scenario`), whatever the
  annotation said: a node carrying the id is a claim, a failed scenario an observation
  against it, and `node_paths` still names the nodes that claim it;
- otherwise, for a `needs-scenario` requirement, every scenario that names it `pass`
  -> `scenario-passed`;
- anything else leaves the row where it was: a scenario that is `not-judged`,
  `blocked` or `awaiting-decision` closes nothing.

Each row an example names carries `scenarios`: `scenario`, `verdict` (the judge's own
word), `bound` for a bounded pass and `reason` for a verdict that is neither a pass
nor a fail. A `scenario-evidence` record comes right after `extraction`, before the
first row: the set's `doc_id`, `rev`, `set_sha256` and `origin`, the `engine` the
driver ran on, `used` (and `why_not_used` when it was not), and `problems` for a
scenario that names a requirement the list does not hold. Without the flags there is
no such record and no row has `scenarios`.

⚠ `scenario-passed` is not `implemented`. `implemented` says a node carries the id and
nothing was run. `scenario-passed` says every example that names the requirement
passed on the engine the record names, over those inputs, and a bounded example only
up to its bound. It does not say the examples are the whole of what the requirement
means, that they are the owner's (`origin`: `ai-proposed` is the author's until the
owner confirms), or that another engine agrees.

⚠ Evidence about another specification is no evidence: a set whose `specification`
is not the list's `doc_id` and `rev`, and a set the judge did not judge (a problem in
the set, or a trace taken against another set), move nothing, and `used: false` says
why.

### Cross-kind typed binding (NL→IR Mapping Roadmap Item 2)

When a forge expression reads an imported kind's member via
`<sce:import as="alias"/>` + `alias.member`, the expression layer
judges it where it reads the expression, in every kind: a member the
import does not declare is `expression/unknown-member` (§2.2), with the
import's fields and methods as the fix. ⚠ This used to be a separate
walk over algorithm bodies only — where an import's alias is not a
value, so every reference it accepted was refused next as an
undeclared name — while a procedure, where the alias is a value, was
never walked (measured 2026-09-21). Three codes remain on this axis:

- `validation/cross-kind-field-not-found` — a statechart guard's
  `_event.data.<field>` names no field of the event schema the
  triggering event carries. Diagnostic carries a closed
  `Fix::ReplaceOneOf` set = the schema's declared fields (sorted,
  deduplicated) for `did_you_mean`-style typo repair.
- `validation/cross-kind-type-mismatch` — an event-schema field
  resolves but what the statechart compares with it or sends into it
  cannot be a value of its declared type (a literal the type cannot
  represent, an enum value wider than the enum's underlying type).
  Silent when the use site does not constrain the expected type
  (`Unknown` context). NL→IR Mapping Roadmap Item 4 also routes
  physical-quantity unit mismatches in arithmetic to this same code
  (typed payload `ValidationError::QuantityUnitMismatch`) — adding
  `<sce:param sce:quantity="celsius"/>` + `<sce:param sce:quantity="kelvin"/>`
  inputs to a Transform whose body combines them arithmetically
  triggers the code with the operator and both unit names in the
  diagnostic message.
- `validation/cross-kind-circular-dependency` — the `<sce:import>`
  graph contains a cycle. Defensive check; without it, the enrichment
  pass recurses into infinite open-file work or surfaces as an opaque
  stack-overflow at codegen.
- `validation/transform-output-cycle` — a Transform's outputs depend on
  each other in a cycle. An output MAY read a sibling output: a
  specification routinely names an intermediate value that several
  outputs consume, and the generator lowers such a read to a call of the
  sibling's own `compute_*` function, which is sound because every one
  of them is a pure function of the same parameters. A cycle is the one
  shape that lowering cannot serve — the emitted functions would call
  each other until the stack ends — so it is refused before any language
  is rendered. ⚠ Before this pair existed, a sibling read of ANY kind
  emitted an identifier the signature never bound, with exit 0. The
  reads are taken from the parsed expression, not its text: a string
  literal that spells an output's name is not a read, and neither is
  `previous(<output>)`, which reads the activation before this one and
  so cannot be part of a cycle (§3.4.1).

The `validation/cross-kind-*` codes are emitted by the statechart's
event-schema check and the physical-quantity check; the import graph's
cycle check runs on every forge compile. A forge expression's
`alias.member` never reaches them — the expression layer answers it
first.

---

### Statechart state-reference resolution

Every id an SCXML document uses to name a state must resolve to a
`<state>`, `<parallel>`, `<final>`, or `<history>` declared in that
document. Four reference positions carry the rule:

| Position | Spec | Rule |
|---|---|---|
| `<transition target>` | W3C SCXML §3.5, §3.11, §3.13 | Every whitespace-separated token resolves independently, and the tokens together are a legal state specification. A targetless transition (no `target` attribute) is not a reference. |
| `<state initial>` | W3C SCXML §3.3, §3.11 | Every token resolves to a descendant of the owning state, and the tokens together are a legal state specification. |
| `<initial>` child | W3C SCXML §3.6 | The initial element's transition target resolves; the parser folds it into the owning state's `initial`. |
| `<history>` default | W3C SCXML §3.10.2 | The default `<transition>` child is **required**, and its target resolves. |

Rejection codes:

- `validation/invalid-reference` — a token names nothing. `actual`
  carries the unresolved id and `fix.candidates` the legal set: every
  declared state for a transition target, the owning state's children
  for a compound `initial` (§3.3 restricts the initial configuration to
  descendants, so a wider list would offer illegal values).
- `validation/missing-element` — a `<history>` declares no default
  `<transition>`. This is a declaration rule, not a use rule: the child
  is required whether or not any transition names the pseudostate,
  because without it the pseudostate can never be entered. The legal
  default targets travel in `message` — SCE_ERROR_CONTRACT §3.1 has no
  add-child-element `fix` variant.
- `validation/attribute-rule-violated` — every token resolves and the
  value, taken whole, is not a **legal state specification** (W3C SCXML
  §3.11). All four positions carry the rule: no state on the list is an
  ancestor of another; every two of them meet at a `<parallel>` (a
  compound state or `<scxml>` holds one child, so two states meeting there
  cannot both be active); and an `initial` or `<history>` default names
  only descendants of the state that holds it. `expected` carries the
  rule and `actual` the value as written; no `fix` is offered, because
  which state to drop is the author's decision. The identifier normalizes
  the value's whitespace, since the C++ Interpreter holds it only as a
  list. ⚠ Before this rule every engine accepted such a value in silence —
  the reference pass asks each token alone — and what an engine does with
  one is unspecified. A state named twice is not a breach; the list is a
  set.

These rules are enforced on every path that parses a document, and both
engines carry them: the Rust producers are
`sce-build/src/scxml_references.rs` and the `<history>` arm of
`parser.rs`, and the C++ Interpreter's counterparts are
`SemanticTransitionTargetUnknown` / `SemanticInitialStateUnknown` /
`SemanticHistoryDefaultMissing` / `SemanticIllegalStateSpecification`
thrown from `SCXMLParser::validateModel`. The legal-specification
judgement on the C++ side is `SCE::Core::checkStateSpecification`
(`sce/include/core/ConfigurationHelper.h`), shared with the generated
code; `tests/parsing/fixtures/cross_producer/` holds the two producers to
one code and one identifier for it.

Why these are rejections rather than warnings: the code generators
lower a transition target to a `State` enum variant. An id that names
nothing lowers to a variant the generated enum never declares, so the
document would otherwise pass `check` with `status: ok`, pass
`generate`, and fail in the consumer's compiler.

---

### Design-time lints are opt-in (`--lint`)

Four validators below — graph reachability, event-set exhaustiveness,
guard analysis and recording interception — **reject legal SCXML**. Each flags a document the
W3C algorithms accept and an Interpreter runs; what they assert is
design intent, not validity. They are therefore off by default and
enabled with `sce-codegen check --lint` / `generate --lint`, which call
the same `sce_build::lint_statechart` the library entry points run
(`sce-build/tests/cli_lint_parity.rs` pins the two verdicts equal).

The W3C IRP corpus is why the default is off — these are conformance
documents that build and pass:

| Document | Shape the lint flags | Why the document is correct |
|---|---|---|
| `resources/278` | `s1` unreachable | `s1` exists only to host a `<datamodel>`; the test checks that `s0` can read a variable from outside its lexical scope |
| `resources/576` | `s0` unreachable | The test proves `<scxml initial>` is honoured, which requires the document-order-first state to stay unentered |
| `resources/355` | `s1` unreachable | The test distinguishes default entry by document order; entering `s1` would be the failure |

Turn the lints on for authored documents, where an orphan region or a
sibling missing an event handler is nearly always a mistake.

The rules a document must satisfy to be lowered **at all** — reference
resolution, a resolvable `initial`, at least one state, a loadable
top-level `<script>` — are not lints and always run, on every entry
point (see [state-reference
resolution](#statechart-state-reference-resolution)).

---

### Statechart graph reachability (NL→IR Mapping Roadmap Item 3 Phase A)

Every `<state>`, `<parallel>`, and `<final>` declared in an SCXML
document must be reachable from the document's initial configuration
through the W3C SCXML §3 entry semantics:

- the document `initial` attribute (or the default-first-child fallback
  when omitted)
- compound-state initial-cascade — entering `<state initial="X">`
  enters X (and recurses)
- parallel-all-children — entering `<parallel>` enters every
  non-history child region
- transition `target` edges
- history pseudostate default-target redirection (W3C SCXML §3.10)
- ancestor entry — entering a state enters every compound ancestor
  (§3.6), and an entered ancestor is a full member of the
  configuration: its own transitions are live (§3.13 selection climbs
  the ancestor chain) and, when it is a `<parallel>`, its every region
  is entered (§3.4). A walk that marked ancestors reached without
  following their edges reported `fail` states as orphans across the
  W3C IRP suite

After the parse completes, a BFS over those edges computes the
design-time reach set. A state outside the closure is dead code —
codegen would still emit per-state surface for it, but no execution
path ever enters it. Two rejection codes:

- `scxml/unreachable-state` — the orphan-state form, emitted when an
  unreachable `<state>` / `<parallel>` / `<final>` declares no
  `<transition>` children. The diagnostic carries only the state id;
  closest-match candidate lists are not surfaced because the orphan's
  id is typically correct — the topology is the bug.
- `scxml/dead-transition` — the per-transition form, emitted when an
  unreachable state contains at least one `<transition>`. The
  per-(source, target) granularity points the author at a concrete
  edge to delete or re-wire. Outranks the state-level form so each
  orphan subgraph reports its first transition rather than just the
  containing state.

Both rejection paths sit in the `scxml/*` family because reachability
is a Statechart-graph rule with no analog on the Forge-kind side
(Forge kinds carry no control-flow surface).

---

### A document set whose member sends to a parent nobody is

`<send target="#_parent">` (W3C SCXML §6.2.4) names the session that
invoked this one. A machine started on its own has none, and every such
send raises `error.communication` at runtime — while the document is
valid SCXML and builds. Whether a parent exists is a fact about the
deployment, so a single document only PUBLISHES the need (the manifest's
`needs_parent` / `parent_sends`, SCE_ERROR_CONTRACT.md §10) and the
document SET judges it: the set compile (`orchestrate`, and `check` over
`--scxml-set`) refuses a member that sends to `#_parent` when no member
invokes it, as `scxml/parent-send-without-parent`, located on the
member's first such `<send>`.

A member counts as invoked when another member's `<invoke>` names it —
by the child name a static `<invoke src>` resolves to, or by `#<name>`
as a Mesh target. It runs with the design-time lints (the set compile
runs them on every member), because the set cannot see a member
invoked from OUTSIDE it: a host that supplies the parent itself, or a
peer in another build, looks exactly like this from inside. Two cases
are not judged, and are said to be: a set in which any member names a
child by expression (`srcexpr`), since whom it invokes is decided at
runtime; and a set compiled against a deploy topology, whose machines
may be invoked by peers the set does not contain.

A scenario driver is not a parent either, and does not pretend to be one.
It gives the machine a parent only when the interface the examples were
accepted with routes an output through `#_parent` (`via`), which is the
owner's word that there is a caller; the parent records what it is sent and
answers nothing, and the route the driver reports is the engine's own (the
SCXML Event I/O Processor), not the type the interface names. Without that
route the run stops at the first send that reaches for a parent, found by
the send and not by the `error.communication` it would raise (a machine that
finishes in the same macrostep never handles it). The reason says whose
question it is: with an open question on the send (the manifest's
`parent_sends[].decisions`) the example is BLOCKED by that decision; with
none recorded it is refused as the design's, with the two ways out named,
a route through `#_parent` or the question recorded on the send.

#### A route chosen from data the specification leaves open

A `<send>` that writes a `typeexpr` or a `targetexpr` chooses where it
goes when it runs, so the build cannot name the value, and the open
question is usually not on the send: a draft writes
`targetexpr="callerTarget"` because a send needs a target, and marks the
`<data id="callerTarget">` it reads `sce:unresolved`, because the
specification never says who the caller is. The manifest lists every such
send (`computed_routes`, SCE_ERROR_CONTRACT.md §10): the data items the
route expressions name (`reads`, found by the ECMAScript lexer, so a name
after a `.` is a property and not an item) and the ids of the questions
open on the send or on any of those items (`decisions`). A value chosen
without an answer (`sce:assumed`) is applied and is not one.

The same reading is written onto the send in the model, the type's and the
target's apart (`Action::type_decisions`, `Action::target_decisions`; the
manifest's `decisions` is their union), and a generated Python machine raises a
failure OF THE ROUTE with the questions it rests on
(`raise_internal(..., rests_on=...)`). The type and the target fail
independently, so each failure carries only the questions ITS expression rests on
(and the send's own): the `typeexpr` not evaluating, or naming a processor this
platform does not support, carries the type's; the `targetexpr` not evaluating,
producing an address this processor cannot use, evaluating to nothing, or the
delivery to it being refused or reaching nobody, carries the target's. A type
that names a variable nobody declared, beside an open target, is the draft's: an
answer about the target would not mend it (measured 2026-10-02, a merged list
laid it to the caller).
The engine holds the questions with the `error.*` event and gives them back for
the last error nothing answered (`last_unhandled_error_rests_on`). A scenario
driver reads that and calls the example BLOCKED by the decision
(`cause: decision`) instead of `not-judged` with the design as the cause: the
failure is the owner's question showing through, the same run fails the same way
on any machine, and answering the question is what unblocks it. A comparison and
a verification have no verdict word of that kind, so they say it in the clause
they already give for an error nothing answered (`lowering.stopped_run`): the
failure rested on an open decision, named, and the draft is not to be mended for
it. All three use one sentence (`lowering.route_decision_clause`).

Only the route's own failure carries the question. The same send can fail in its
event name, its delay, its namelist or its payload, and those are faults of the
draft that no answer to the route's question would mend: they are raised without
it and stay the design's, so the owner is not asked to name a caller when the
draft has to be mended. (An earlier version registered the questions when the
send began and attached them to every error the send raised; measured
2026-10-02, it blamed the open question for a missing event-name variable.)
An error another send raised, one the document answered, and a route that rests
on a plain or an assumed value are likewise not laid to a question, and stay what
every unanswered error was before: the design's. Only a question the machine
named is a decision; the driver does not guess one from the shape of the failure.

A delayed send is refused, or its target found gone, when its wait is over, long
after the generated send site returned. The questions therefore ride the
scheduler entry (`ScheduledEvent.rests_on`), handed over when the send is armed
(`send_to_target`, `schedule_host_send`) and raised with the refusal the drain
makes (`error.communication` for an invocation that ended, `error.execution` for
a host act nobody performed). A delayed send whose route rests on no question
raises its refusal without one, as an immediate one does.

---

### Statechart event-set exhaustiveness (NL→IR Mapping Roadmap Item 3 Phase B)

A compound `<state>`'s sibling children are expected to agree on
event coverage when they share a vocabulary: if children A, B, and C
all handle the `cmd.*` event family, but only A and B declare a
transition for `cmd.stop` while C does not (and the parent has no
fallthrough), the gap in C is almost always an authoring mistake.
AI-generated SCXML produces this pattern frequently — the model
emits a coherent handler set for some siblings and forgets the
others.

The validator uses a narrow heuristic to keep false positives at
zero across the W3C IRP, conformance, and downstream-consumer corpora:

- The compound parent must be a non-`<parallel>`, non-`<final>`
  `<state>` (parallel regions are orthogonal by design and do not
  participate in this check).
- The siblings under consideration are direct child `<state>` /
  `<parallel>` nodes that have at least one `<transition>`
  (`<final>` and history pseudostates excluded — they have no
  transition surface to compare). At least two such siblings must
  exist for the check to fire.
- The siblings must share **common ground**: there must exist at
  least one event matched by every transition-carrying sibling
  (W3C SCXML §5.10 prefix-match semantics apply). The "sequential
  protocol stages with disjoint event vocabularies" pattern that
  prevails in the W3C IRP suite (e.g., one stage handles
  `childToParent` only, the next stage handles `pass`/`fail`/
  `timeout` only) has no common ground and is silently accepted.
- For each event `E` in the union of literal event tokens (no
  wildcards), if at least one sibling handles `E` and at least one
  does not, and the parent itself has no transition matching `E`,
  the validator emits `scxml/non-exhaustive-event-handling`.

Author escape hatch: `sce:unhandled="E1 E2"` on **the child that
leaves the events unhandled** — not on the compound parent. The
declaration exempts exactly the (child, event) pairs it names.

The attribute sits on the child because that is the grain at which
the author's claim is true: "`berserk` does not handle
`combo_timeout`", not "this compound is exempt". A parent-level
opt-out (`sce:exhaustive="false"`) existed in an earlier revision
and was withdrawn — it silenced every gap under the parent,
including gaps introduced after it was written, so a sibling added
later inherited an exemption nobody had judged. A document still
carrying `sce:exhaustive` rejects via `validation/attribute-rule-violated`
rather than being ignored, because an unrecognised `sce:` attribute
is accepted and ignored and the exemption would otherwise be lost
silently.

Token rules: whitespace-separated literal event names, at least
one, each named at most once, no wildcards. Wildcards are rejected
so the declaration is checked against the literal gap set under one
matching rule rather than two.

The declaration is checked in both directions, so it cannot decay
into unverified prose:

- A state declaring an event it actually handles (directly, by
  token-prefix, or via a wildcard transition) rejects with
  `scxml/contradictory-unhandled-declaration`.
- A state declaring an event that is not a gap under its parent —
  no sibling handles it, the parent absorbs it, or the state has no
  compound parent — rejects with
  `scxml/stale-unhandled-declaration`.

Both directions are judged on **every** build, not only under
`--lint`. A declaration is a claim about the document, and a false
claim is false whether or not the operator asked for design advice —
the same reason the attribute's *shape* (wildcards, repeats, an empty
value) has always been refused unconditionally. The gap **report**
stays opt-in: "which of these compounds should an author be told
about" is the design-intent question `--lint` exists for, and
default-on would refuse the W3C IRP corpus. The two therefore sit on
opposite sides of the flag.

The common-ground precondition is **not** among those reasons, and
that is deliberate. It decides what is worth *reporting*; whether a
declaration is *true* is a fact about the document — a sibling
handles the event, this child does not — and holds whatever shape
the rest of the compound has. While the two shared one set, the one
shape a document is quiet in was also the one shape its author could
not write an intention down in: the declaration became sayable only
in the round where a later edit gave the siblings an event in common
and the report began demanding it. So the check could not be paid in
advance, and every author met it first as a rejection. Declaring a
gap under a protocol-stage compound is accepted now, and reports
nothing either way.

Repair guidance, in author preference order:

1. Add the missing `<transition event="E" ...>` to the non-handling
   sibling.
2. Add a parent-level `<transition event="E" ...>` so the event is
   absorbed by the compound state regardless of which child is
   active.
3. Declare `sce:unhandled="E"` on the non-handling child if the gap
   is genuinely intentional.

---

### Statechart guard analysis (NL→IR Mapping Roadmap Item 3 Phase C)

`<transition cond="...">` guards can be statically false (the
transition never fires) or be shadowed by an earlier unconditional
sibling (per W3C SCXML §5.10 transition selection, the first
matching transition in document order wins). Both patterns are
authoring mistakes that survive parse + reachability today.

The validator stops short of full SMT to keep the false-positive
surface at zero — it recognises only the structurally trivial
cases:

`scxml/always-false-guard` fires when the `cond` attribute matches
one of:

- The literal `false` (lowercase per W3C SCXML §B ECMAScript
  convention).
- The numeric literal `0`.
- A binary equality `N == M` where both sides parse as decimal
  numeric literals with differing values (`1==2`, `0==1`,
  `42==99`). Whitespace around `==` is tolerated.
- A binary inequality `N != M` where both sides parse as decimal
  numeric literals with equal values (`1!=1`, `0!=0`).

Language-prefixed `cond` values (`cpp:expr`, `kotlin:expr`,
`rust:expr`) remain opaque — the validator never inspects them.
Their semantics depend on the host language's expression
evaluator, which the parser cannot reason about statically without
risking false positives.

`scxml/shadowed-transition` fires when a state's `<transition>`
list contains an unconditional transition (empty `cond`, literal
`cond="true"`, or literal `cond="1"`) followed by a same-event
sibling. The shadowing transition matches every event the shadowed
one matches, so per W3C SCXML §5.10 it always wins and the later
one is dead. The validator requires literal equality of the
`event` attribute between the two transitions — token-prefix
superset cases (`event="foo"` shadowing `event="foo.bar"`) depend
on ancestor-priority rules the parser-stage walker cannot
disambiguate without running the full selection algorithm, so they
are deliberately not flagged.

Repair guidance:

1. Remove the dead transition.
2. Rewrite the guard to a satisfiable expression.
3. For shadowed transitions, reorder so the more specific transition
   precedes the unconditional one, or add a guard to the previously
   unconditional transition.

---

### Recording intercepted by an inner transition

`scxml/recording-intercepted` fires when a state's transition on an
event assigns a location (`<assign location="L">`, nested blocks
included), and a proper descendant — reached through `<state>`s only —
has a transition whose event descriptors overlap it and which does not
assign `L` itself. W3C SCXML §3.13 selection walks out from each atomic
state and takes the first enabled transition, so while that descendant
is active its transition is taken and the ancestor's recording never
runs.

The document is legal and the skip may be intended, which is why this
is a `--lint` finding. It was added on a measurement: a door lock
recorded the vehicle speed on its outer state while the unlocked state
locked itself on a fast speed update; the fast update left the recorded
speed at its previous, slow value, and the next unlock request passed
its low-speed guard at 20 km/h. The same machine with the recording in
a `<parallel>` region of its own behaved.

Not flagged: an outer `<parallel>`, or a `<parallel>` between the two
states — each region selects its own transition, and the cross-region
case turns on conflict resolution the walk does not model; and an
eventless transition, which records on no event.

Repair guidance:

1. Assign the location in the descendant's transition too.
2. Or move the recording into a `<parallel>` region beside the states
   that react to the event.
3. Or, when skipping the record in that state is intended, leave it —
   the lint is design advice.

---

### A message the machine sends itself and discards

`scxml/self-send-discarded` fires on a `<send>` with a literal `event`
that goes to the session itself and whose event no transition in the
document takes. W3C SCXML §6.2.4 sends a `<send>` with neither `target`
nor `targetexpr` to the session's own external queue, and `#_internal`
to its internal queue — both through the SCXML Event I/O Processor,
written or defaulted. An event no transition matches is taken off the
queue and nothing happens.

The document is legal, which is why this is a `--lint` finding. It was
added on a measurement: drafted statecharts wrote an output whose
receiver the specification did not name as `<send event="…"/>`, and the
machine announced each output to itself and threw it away, as all five
drafts of one specification did. `check --lint` accepted every one of
them.

Not flagged: a send with a `target`, `targetexpr`, `type` or `typeexpr`,
which leaves the session through the processor it names; an `eventexpr`,
whose event is known only at runtime; an event any transition takes, by
a prefix or `*` included; and an event sent to the external queue of a
session with an `<invoke autoforward="true">`, which W3C SCXML §6.4.1
forwards to the child.

A document that declares `sce:interface="closed"` (§2.16) refuses the same
send on the same reading, as `scxml/undeclared-interface-event`.

Repair guidance:

1. When the event is an output, name its receiver: the `type` of an
   Event I/O Processor the host serves (declared to the build with
   `--host-processor`), or `target="#_parent"` when the specification
   names the statechart that invokes this one — then check the two
   together, since started on its own the machine has no parent.
2. When the machine is meant to act on it, add the transition that takes
   it.

---

### Physical-quantity annotation (NL→IR Mapping Roadmap Item 4)

`<sce:field>` and `<data>` elements may carry an
`sce:quantity="<unit>"` attribute, optionally paired with
`sce:scale="<rational>"` and `sce:offset="<rational>"`. The triple
declares a linear `physical = raw * scale + offset` conversion in
the named opaque unit (SI base units `s` / `m` / `kg` / `A` / `K` /
`mol` / `cd` are recommended but not enforced — the unit string is
treated as an opaque equality key across operands).

The conversion is **codegen-effective** (not documentation-only):

* Codec fields carrying a quantity annotation emit a raw↔physical
  accessor pair (`<id>_phys()` / `set_<id>_phys()` per backend; or
  the language-idiomatic case-converted equivalent — `<id>Phys()`
  for Kotlin, `<Id>Phys()` for Go). The raw struct member retains
  its wire-level integer/float type; the accessor performs the
  conversion in IEEE-754 double precision.
* Transform `<data direction="in">` parameters and outputs may also
  declare a quantity. The generated `compute_*` function signature
  is unchanged (the function consumes raw, returns the body
  expression's typed result); a doc-comment block on the function
  surfaces the unit annotation for downstream readers.

`sce:scale` accepts decimal integers (`42`, `-17`), decimal
fractions (`0.5`, `-40.25`), and explicit `<num>/<denom>` ratios
(`1/100`). Scientific notation, hexadecimal, leading `+`, and zero
denominator are rejected at parse time with
`validation/attribute-rule-violated`. `sce:scale="0"` is also rejected —
a zero scale means the raw value never influences the physical
reading, which makes the annotation observably equivalent to
deleting both the scale and the unit.

`sce:scale` or `sce:offset` without `sce:quantity` is rejected
(the conversion factor needs a unit to anchor against). An empty
`sce:quantity=""` is rejected as a missing unit name.

The type system threads the quantity through expression inference:
binary arithmetic between two `Quantity`-typed operands carrying
**different** unit tags surfaces as
`validation/cross-kind-type-mismatch` via the typed
`ValidationError::QuantityUnitMismatch` payload (no new
`DiagnosticCode` slot — the validator reuses the cross-kind code
under the "type incompatibility" concept umbrella). Quantity
combined with concrete bare numeric strips the unit
(explicit-typed authorship opts out of unit checking at that
site); quantity combined with an untyped literal keeps the
annotation sticky.

ARXML COMPU-METHOD blocks map onto this layer directly: the
INTERNAL value is the raw, the PHYS value is the physical, and the
COMPU-NUMERATOR / COMPU-DENOMINATOR pair becomes
`sce:scale="<num>/<denom>"`. The widely-deployed automotive
temperature encoding `physical = raw * 0.5 - 40` Celsius for an
`int8` raw byte authors as:

```xml
<data id="raw_temp" sce:type="int8" sce:direction="in"
      sce:quantity="celsius" sce:scale="0.5" sce:offset="-40"/>
```

---

### EventSchema kind (NL→IR Mapping Roadmap Item C1 Path A)

A typed contract for the `_event.data` payload of a named SCXML
event — each schema names exactly one event and declares the typed
fields authors may read via `_event.data.<field>` in transition
`cond` attributes and write via `<send>/<param>` payloads.

**Surface forms** (DL-8' — both produce identical IR shapes):

- Top-level form (primary):
  `<scxml sce:kind="event-schema" sce:event-name="job.completed">…</scxml>`
- Inline form (sugar, deferred to F-ζ — out of scope for Path A
  α): `<sce:event-schema event-name="job.completed">…</sce:event-schema>`
  as a child of `<scxml>`. The top-level form is the only Path A
  α surface; inline lowering ships when an inline consumer
  surfaces.

**Field declaration** uses `<data id="..." sce:type="..."
sce:direction="in"/>` inside a `<datamodel>` wrapper. `sce:type`
may be any of the primitive `SceType` values (`uint8` / `uint16`
/ `uint32` / `uint64` / `int8` / `int16` / `int32` / `int64` /
`float32` / `float64` / `bool` / `string` / `bytes`) or
`enum:<enum-alias>` for an enum drawn from an imported
`sce:kind="enum"` document (Path A's 17th kind).

**Direction invariant** (DL-5'): `sce:direction` must be `in` —
the payload is the receiver's read-only view. `out` / `internal`
directions raise `validation/invalid-attribute` at parse time.

**Fieldless schema.** An event the specification says carries no data, or
does not say, is declared as one rather than left out: a closed interface
(§2.16) admits only events an imported schema declares. A schema may have no
`<data>` field only if its `<datamodel>` says why:

- `<datamodel sce:payload="none"/>` — the specification says the event carries
  no data. Nothing is left open.
- `<datamodel sce:unresolved="…" sce:unresolved-reason="…"/>` — the
  specification does not say. It is the marker every other open question
  uses: accepted, published in the manifest's `unresolved` and `open`, and
  refused by `--strict-unresolved`.
- `<datamodel sce:assumed="…" sce:assumed-reason="…"/>` — the specification
  does not say and the author chose no payload without it. The same marker
  everywhere else it appears: accepted, published in `unresolved` and `open`
  as an assumed value the owner confirms or corrects, and NOT refused by
  `--strict-unresolved`, which refuses only the question.

A `<datamodel>` with none of these and no field is refused as
`validation/empty-collection` — it reads the same as a field the author forgot
— and its message names the ways out. `sce:payload` with any value but `none`
is `validation/invalid-attribute`; `none` beside a field, or beside
`sce:unresolved` or `sce:assumed`, contradicts itself and is
`validation/incompatible-attributes`.
(`sce:payload` on a procedure `<send>` is another attribute of the same name;
schemas/sce-forge-ext.xsd says which is which.) The page writes the statement
under the head — `payload none`, or the open marker with its reason — so it
never reads as an event that carries nothing when the specification left that
open.

A fieldless schema is a build-time contract and nothing else, so no W3C
behaviour moves. No backend emits a payload struct for it (an empty `data
class` is not Kotlin, an empty `struct` is not ISO C). The machine generated
for an event it names is the machine generated with no schema at all — the
same output on all six backends but for the `source-hash` header and the source
positions — so an event delivered with data is delivered as W3C SCXML 5.10.1
has it, and none of the typed-payload machinery below exists for it. What the
build does refuse is a use that needs a field: `_event.data.<field>` in a guard
(`validation/cross-kind-field-not-found`, with no candidates to offer), a
`<param>` sent with the event (`validation/event-payload-field-unknown`), and a
record made of it — a `record:<alias>` variable or parameter
(`validation/attribute-rule-violated`) or a host-run `<invoke>` request or
result (`validation/typed-invoke-schema`).

**Schemaless fallback** (DL-9'): events without an imported
EventSchema retain the dynamic `_event.data` baseline — no
diagnostic, identical W3C IRP behavior. W3C built-in event
namespaces are explicitly excluded:

- `validation/event-schema-on-builtin-event` — an EventSchema
  document declares `sce:event-name` against a W3C SCXML reserved
  event prefix (`error.*`, `done.invoke.*`, `done.state.*`). The
  platform raises these events with implementation-defined
  payload shape; an authored schema cannot meaningfully constrain
  them. Repair: rename the schema's `sce:event-name` to a
  non-reserved value or delete the schema document.

**Receive-side typecheck** (DL-5'): `_event.data.<field>`
expressions in transition `cond` attributes resolve against the
imported EventSchema for the transition's event. Field-not-found
surfaces through `validation/cross-kind-field-not-found` (existing
code reused per Item 4 precedent; carries the schema's declared
field surface as `Fix::ReplaceOneOf` candidates). Comparison
type-mismatch (e.g. `_event.data.elapsed_ms === 'forty-two'`
against a `uint32` field) surfaces through
`validation/cross-kind-type-mismatch` (also reused per Item 4
precedent).

A `bytes`-typed field compares against a printable-ASCII string
literal by value (`_event.data.raw === 'ack'`). Such a guard lowers
natively on all six backends — each to its own byte-equality
primitive over the same decoded constant (Rust `== b"ack"`, C++
`== std::vector<uint8_t>{…}`, Go `string(..)== "ack"`, Kotlin
`.contentEquals("ack".toByteArray())`, Python `== b"ack"`, C11
`_len == N && memcmp(.., "ack", N) == 0`) — so no script engine is
required. The C11 payload field is a no-alloc bounded buffer
(`uint8_t[CAP]; size_t _len`) whose `CAP` comes from `sce:max-size`
(default 256). Two `bytes`-specific rejections layer on top of the
shared receive-side checks (RFC `rfc-eventschema-bytes-guard.md` §3):

- `validation/bytes-comparison-not-equality` — an ordering operator
  (`<`, `>`, `<=`, `>=`) applied to a `bytes` field. Lexicographic
  ordering of an opaque payload byte-blob is not a meaningful author
  intent; only equality (`===` / `!==`) lowers to a well-defined,
  byte-identical comparison on every backend. A distinct
  operator-domain rule, so a dedicated wire code.
- `validation/cross-kind-type-mismatch` (reused per Item 4
  precedent) — the string literal carries a backslash escape or a
  non-ASCII byte. Such a literal has no unambiguous cross-backend
  byte constant, so it is rejected as a type-category mismatch; the
  printable-ASCII boundary is a validated, forward-compatible
  literal-syntax scope (the byte carrier widens later without
  touching the wire form).

**Send-side payload typecheck** (DL-4'): a `<send event="X">` or
`<raise event="X">` (in transition `actions`, `<onentry>`, or
`<onexit>` content, including nested `<if>` / `<foreach>` bodies)
whose event name `X` resolves to an imported EventSchema has its
`<param name="F" expr="..."/>` children validated against the
schema's declared field surface. Two rejections:

- `validation/event-payload-field-unknown` — `<param name="F">`
  carries a name `F` that is not declared on the schema. The
  schema's full field surface ships as a closed
  `Fix::ReplaceOneOf` candidate set, mirroring the receive-side
  `validation/cross-kind-field-not-found` shape so `did_you_mean`-
  style typo repair surfaces identically on both sides.
- `validation/cross-kind-type-mismatch` (reused per Item 4
  precedent) — `<param expr="...">` is a primitive literal whose
  syntactic type does not unify with the schema field's declared
  `sce_type`. Non-literal expressions (variable references,
  nested computations, function calls) defer to the existing
  typed-expression pipeline at codegen time.

**Mesh cross-machine validation** (DL-7'): when `<send target="#X">`
appears on a statechart deployed to machine A and addresses
machine B (the `#X` resolves to B's machine id via the
deploy.yaml topology), the sender and receiver EventSchemas for
the send's event name must agree on field shape. One rejection:

- `mesh/event-schema-mismatch` — either (a) both sides declare an
  EventSchema but their canonical structural hashes diverge
  (fields sorted by id, normalized JSON via schemars), (b) sender
  declares a schema while receiver does not, or (c) receiver
  declares a schema while sender does not. The `Display` form
  names the specific divergence reason so consumers do not have
  to substring-grep the prose. Repair is two-axis (realign the
  two schemas, or declare a schema on the side that is missing
  it) — author-domain choice, no closed candidate set.

**Per-backend payload codegen** (DL-6' continuation) ships the
per-backend payload struct surface (`struct <Schema>Payload` on
C++, `pub struct <Schema>Payload` on Rust, `data class
<Schema>Payload` on Kotlin, typed struct on Go, `@dataclass` on
Python, `typedef struct { … } <Schema>Payload_t` on C11) so the
cross-backend parity gate (§6.2.6) closes in lockstep.
Enum-typed fields reference the imported Enum kind's qualified
type per backend (`SCE::Generated::<E>::<E>` on C++, `<e>::<E>`
on Rust, wildcard-import bare `<E>` on Kotlin, `<e>.<E>` on Go,
`<e>.<E>` on Python, `<E>_t` on C11) — no variant re-emission;
the Enum document remains the single source of truth.

**Which carrier the payload arrives on** — a typed payload has TWO
sources, and a natively lowered guard reads the same value from either:

- The generated per-event inject seam (`raise_<event>` on Rust and
  Python, `Raise<Event>` on Go, `raise<Event>` on C++ and Kotlin,
  `<machine>_raise_<event>_typed` on C11) supplies the typed payload and
  the `_event.data` wire, so a document that also reads
  `_event.data.<field>` through the script engine — an `<assign>`, a
  `<log>`, an un-lowered `cond` — sees the values the guard saw.
  C11 returns `false` without enqueueing when a value or its complete JSON
  exceeds the bounded buffer, or a float is non-finite. A null payload
  uses zero values. String fields are owned through the JSON and lifted
  on dequeue, so queued events never borrow caller string storage.
- Every OTHER producer fills only the wire: `<send>` with `<param>`,
  namelist or `<content>`, an invoke forwarding an event in either
  direction, autoforward, BasicHTTP, the mesh. The typed view is then
  LIFTED out of that wire when the event is dequeued, read as
  §scxml-B-2-8-1's second rung — the same read the script engine takes
  for the same string.

⚠ Before 2026-09-22 only the first of the two existed, so the same
guard answered differently depending on which producer sent its event,
and a typed payload could not cross an invoke boundary at all.

**A payload that cannot be read as its schema** raises
`error.execution` and leaves the guard unfired. That is W3C SCXML
3.13's answer for a guard that cannot be evaluated, and — measured on
`statechart_lifted.scxml` through the script engine — what the Lua path
already answered for the same cases: an event with no data, data that
is not an object of named fields, a field the data does not name, and a
value of another type or outside the declared width. A native lowering
that answered differently would make the optimisation observable, which
is the one thing it may not be. A document that declares no
`error.execution` transition has nothing to answer with
(§scxml-3.12.2); the guard still does not fire, and a well-typed payload
the guard merely rejects is not an error at all.

⚠ `bytes` on the wire: JSON has no byte string, so a `bytes` field
rides as its byte-exact Latin-1 text and is read back the same way.
Printable ASCII — what a `bytes` guard compares — is identical under
either reading.

⚠ Rust `no_std`: `EventMetadata` carries no `data` there, and no script
engine exists whose reading this one would have to match, so the typed
payload is the whole channel and neither the wire fill nor the lift is
emitted.

**Cross-doc Enum literal width narrowing** (DL-5'): integer
literals compared against (receive-side) or assigned to
(send-side) an enum-typed field are narrowed against the
imported enum's declared `underlying_type`. Decimal, hex (`0x`),
binary (`0b`), and octal (`0o`) literal forms all parse, with an
optional leading `-` — the carrier may be signed
(`int8`/`int16`/`int32`/`int64` alongside the unsigned four), so a
negative sentinel is an ordinary comparison. Values outside the
carrier's range in either direction raise
`validation/cross-kind-type-mismatch` (reused per Item 4
precedent — no new wire code). The narrowing fires only when the
statechart's own `<sce:import kind="enum" as="<alias>">` resolves
the enum alias; otherwise the narrowing silent-skips
(conservative-accept default — category check still requires the
literal to be an integer).

**Strict variant membership** (F-κ): after the width narrowing
layer accepts an integer literal against an enum-typed field, the
membership layer verifies the literal's value is one of the
imported enum's declared `<sce:variant value="…"/>` set. A
comparison like `_event.data.status === 7` against an enum
declaring `{ok=0, error=1, timeout=2}` fits the underlying carrier
but is not a declared variant, and raises
`validation/cross-kind-type-mismatch` (same reuse precedent — no
new wire code). The diagnostic enumerates the declared variant set
in declaration order so authors can pick the value they meant.
Receive-side and send-side branches share a single membership
helper so the diagnostic shape is identical on both sites; the
width-overflow diagnostic always wins when both conditions hold
(a literal that overflows the underlying carrier is the more
fundamental violation and surfaces first).

**Opt-out for open-set vocabularies**: enum documents that declare
only a partial set of values with the expectation that wire-side
values outside the declared set are legal (open-set status
vocabularies — e.g. UDS NRC, OEM-extensible response codes) opt
out of the membership check via `sce:strict-variants="false"` on
the enum's `<scxml>` root:

```xml
<scxml sce:kind="enum" name="UdsNrc"
       sce:underlying-type="uint8"
       sce:strict-variants="false">
  <datamodel>
    <data id="variants">
      <sce:variant name="generalReject" value="16"/>
      <!-- OEM extensions accepted at runtime; declared set is partial -->
    </data>
  </datamodel>
</scxml>
```

The opt-out is owned by the declaring vocabulary (not the
consuming statechart) so the decision lives with the schema-of-
record. Width narrowing still runs regardless of this opt-out —
overflow of the declared underlying carrier remains a wire-level
violation. Default is `true` (strict): an enum that does not set
the attribute is checked as closed-set, matching the typed-
vocabulary intent of `sce:kind="enum"`.

The attribute also decides what the generated code does with a
value no variant declares, because a codec field typed by an enum
travels on the wire as that enum's carrier. A closed set refuses
such a value at decode: the Rust, Go, Python and C11 runtimes
report `UndeclaredEnumValue`, and C++ and Kotlin return the same
truncation sentinel they return for malformed text. An open set's
generated type carries the value instead, so it survives a decode
and a re-encode unchanged. A holder of either kind that must be
constructed before a decode fills it starts at the first variant
the document declares, never at the carrier's zero — a closed set
does not hold a value it never declared.

**Declared names at run time.** Every backend's generated enum answers
which `<sce:variant name>` a value carries, spelled exactly as the
document spells it (`deadlineExceeded`, not the backend's identifier
`DeadlineExceeded` / `DEADLINE_EXCEEDED`), so a consumer that writes an
enum into a log line or a JSON field reads the name from the vocabulary
of record instead of keeping a second list that can drift from it:
Rust `declared_name()`, Kotlin `declaredName()`, Python
`declared_name()`, Go `DeclaredName()`, C++ `declared_name(v)` and C11
`<snake_name>_declared_name(v)`. A value no variant names has no
declared name, and each backend says so in its own absent form — Rust
`None`, Kotlin and Python `null`/`None`, Go `("", false)`, C++
`nullptr`, C11 `NULL`. On Rust, Kotlin and Python that absence exists
only for an open set (`sce:strict-variants="false"`), since only an
open set's generated type can hold such a value; a closed set's
accessor returns the name unconditionally. C++, C11 and Go types hold
every carrier value whatever the set, so their accessor is always the
fallible form. The conformance harness pins the accessor against the
document's names on all six backends.

---

## Appendix — `DiagnosticCode` index (384 codes)

This appendix is the **drift-guarded coverage target** for the
`acceptance_doc_covers_every_code` test. Every slash-path string in
`DiagnosticCode::as_str` appears in exactly one of the two tables
below. The prose sections §1–§3 above reference a subset of these
codes inline, but the appendix is the coverage contract.

The guard matches each code against the table-row anchor
`` | `code` | ``. Reformatting the tables (extra whitespace between
`|` and the code, changing to bullet lists, wrapping across lines)
will break the anchor and fire the test — the markdown shape above
is the load-bearing format. Keep rows on a single line with exactly
one space on either side of the leading backtick-wrapped code.

### Acceptance boundary

Codes that the author can avoid by writing a better SCXML /
`deploy.yaml` / CLI invocation. Listed in pipeline-stage order.

| Code | Stage |
|---|---|
| `xml/parse` | Xml |
| `xml/schema-validation` | Xml |
| `xml/file-not-found` | Xml |
| `xml/wrong-root-element` | Xml |
| `xml/xinclude-missing-href` | Xml |
| `xml/xinclude-not-found` | Xml |
| `xml/xinclude-cycle` | Xml |
| `xml/xinclude-too-deep` | Xml |
| `xml/xinclude-malformed` | Xml |
| `xml/xinclude-unsupported` | Xml |
| `xml/template-not-found` | Xml |
| `xml/template-malformed` | Xml |
| `xml/template-missing-attribute` | Xml |
| `xml/template-missing-param` | Xml |
| `xml/template-unknown-param` | Xml |
| `xml/template-cycle` | Xml |
| `xml/template-too-deep` | Xml |
| `xml/preprocessor-not-run` | Xml |
| `validation/missing-element` | Validation |
| `validation/missing-attribute` | Validation |
| `validation/invalid-attribute` | Validation |
| `validation/attribute-rule-violated` | Validation |
| `validation/exactly-one-attribute` | Validation |
| `validation/send-operand-type` | Validation |
| `validation/unexpected-child-element` | Validation |
| `validation/unknown-sce-attribute` | Validation |
| `validation/default-covers-unknown-variant` | Validation |
| `validation/default-covers-tested-variant` | Validation |
| `validation/default-covers-not-a-value-space` | Validation |
| `validation/unsupported-kind` | Validation |
| `validation/kind-not-inline-eligible` | Validation |
| `validation/duplicate-id` | Validation |
| `validation/malformed-identifier` | Validation |
| `validation/event-name-grammar` | Validation |
| `validation/malformed-code-identifier` | Validation |
| `validation/reserved-code-identifier` | Validation |
| `validation/colliding-code-identifier` | Validation |
| `validation/duplicate-context-object` | Validation |
| `validation/reserved-context-id` | Validation |
| `validation/empty-collection` | Validation |
| `validation/count-mismatch` | Validation |
| `validation/incompatible-attributes` | Validation |
| `validation/missing-context` | Validation |
| `validation/invalid-reference` | Validation |
| `validation/invalid-direction` | Validation |
| `validation/numeric-parse` | Validation |
| `validation/empty-value` | Validation |
| `validation/singleton-violation` | Validation |
| `validation/require-either` | Validation |
| `validation/wrong-pipeline` | Validation |
| `validation/dynamic-features` | Validation |
| `validation/native-action-placement` | Validation |
| `validation/typed-invoke-schema` | Validation |
| `validation/typed-invoke-request` | Validation |
| `validation/native-action-argument` | Validation |
| `validation/native-action-signature-conflict` | Validation |
| `validation/mesh-rpc-reserved-param` | Validation |
| `validation/mesh-rpc-missing-target` | Validation |
| `validation/mesh-rpc-duplicate-target` | Validation |
| `validation/removed-attribute` | Validation |
| `validation/sce-attribute-unread` | Validation |
| `validation/bytes-max-size-violation` | Validation |
| `validation/duplicate-requirement-id` | Validation |
| `validation/provenance-malformed` | Validation |
| `validation/provenance-duplicate` | Validation |
| `validation/kind-basis-malformed` | Validation |
| `validation/unresolved-placeholder` | Validation |
| `validation/cross-kind-field-not-found` | Validation |
| `validation/cross-kind-type-mismatch` | Validation |
| `validation/cross-kind-circular-dependency` | Validation |
| `validation/transform-output-cycle` | Validation |
| `validation/enum-no-variants` | Validation |
| `validation/enum-variant-duplicate-name` | Validation |
| `validation/enum-variant-duplicate-value` | Validation |
| `validation/enum-variant-value-overflows-underlying` | Validation |
| `validation/enum-unsupported-underlying-type` | Validation |
| `validation/event-schema-on-builtin-event` | Validation |
| `validation/event-payload-field-unknown` | Validation |
| `validation/bytes-comparison-not-equality` | Validation |
| `mesh/event-schema-mismatch` | Validation |
| `algorithm/local-shadows-param` | Validation |
| `algorithm/lvalue-unsupported` | Validation |
| `algorithm/return-missing` | Validation |
| `algorithm/foreach-source-not-iterable` | Validation |
| `algorithm/call-target-unknown` | Validation |
| `algorithm/call-target-method-unknown` | Validation |
| `algorithm/bc-mutation-forbidden` | Validation |
| `algorithm/foreach-source-bc-with-bytes-item-type` | Validation |
| `algorithm/call-arg-count-mismatch` | Validation |
| `algorithm/append-target-not-buffer` | Validation |
| `algorithm/append-type-mismatch` | Validation |
| `algorithm/undeclared-integer-failure` | Validation |
| `algorithm/require-without-may-fail` | Validation |
| `algorithm/const-not-foldable` | Generate |
| `algorithm/const-fold-budget-exceeded` | Generate |
| `algorithm/const-yield-type-mismatch` | Generate |
| `algorithm/const-integer-failure` | Generate |
| `codec/variant-arm-unreachable` | Validation |
| `codec/variant-duplicate-default-arm` | Validation |
| `codec/variant-arm-mid-mismatch` | Validation |
| `codec/variant-arm-inner-mid-undeclared` | Validation |
| `codec/variant-arm-body-caller-tag-unsupported` | Validation |
| `codec/variant-no-default-arm` | Validation |
| `codec/variant-default-overlay-arm-not-declared` | Validation |
| `codec/variant-dispatch-flag-not-resolved` | Validation |
| `codec/variant-dispatch-bit-width-mismatch` | Validation |
| `codec/variant-dispatch-arms-not-distinguishable-without-default` | Validation |
| `codec/variant-dispatch-flag-has-static-value` | Validation |
| `codec/variant-dispatch-carrier-after-embed` | Validation |
| `codec/flag-bind-input-not-declared` | Validation |
| `codec/flag-bind-source-not-resolved` | Validation |
| `codec/flag-bind-width-mismatch` | Validation |
| `codec/flag-input-unbound` | Validation |
| `codec/flag-bind-duplicate-input` | Validation |
| `codec/flag-bind-carrier-after-embed` | Validation |
| `codec/present-if-refs-later-field` | Validation |
| `codec/repeat-count-refs-later-field` | Validation |
| `algorithm/test-vector-unsupported-kind` | Validation |
| `codec/tlv-chain-depth-unspecified` | Validation |
| `codec/tlv-chain-truncate-under-entry-flag` | Validation |
| `codec/dma-alignment-unsatisfiable` | Validation |
| `codec/peek-byte-flag-layout-mismatch` | Validation |
| `link/framer-missing` | Validation |
| `link/link-class-unknown` | Validation |
| `link/backpressure-undeclared` | Validation |
| `link/class-unsupported-on-target` | Validation |
| `link/pool-slot-smaller-than-framer-max` | Validation |
| `link/pool-ref-not-declared` | Validation |
| `link/framer-ref-not-declared` | Validation |
| `mem/pool-section-conflict` | Validation |
| `mem/pool-too-large` | Validation |
| `mem/inter-pool-padding-not-emitted` | Validation |
| `mem/cache-line-alignment` | Validation |
| `mem/dcache-line-size-not-power-of-two` | Validation |
| `mem/alignment-not-power-of-two` | Validation |
| `mem/slot-size-not-alignment-multiple` | Validation |
| `mem/cache-policy-unsupported-on-no-dcache-core` | Validation |
| `pool/cache-maintenance-misplaced` | Validation |
| `pool/speculative-prefetch-flag-missing` | Validation |
| `pool/cache-pre-arm-invalidate-missing-on-speculative-core` | Validation |
| `pool/sample-typestate-attributes-disabled` | Validation |
| `pool/sample-take-without-stage-pool` | Validation |
| `pool/sample-callback-signature-non-borrow` | Validation |
| `worker/shared-mutable-state` | Validation |
| `worker/link-rx-ref-unknown` | Validation |
| `worker/inbox-ordering-unspecified` | Validation |
| `worker/inbox-ordering-relaxed-across-cores` | Validation |
| `worker/scheduler-unsupported` | Validation |
| `worker/outbox-ref-unknown` | Validation |
| `worker/outbox-target-wrong-kind` | Validation |
| `worker/outbox-target-suffix-invalid` | Validation |
| `mem/reassembly-pool-variant-missing-max-fragments` | Validation |
| `mem/reassembly-pool-variant-missing-timeout` | Validation |
| `mem/reassembly-slot-size-below-declared-mtu` | Validation |
| `reassembly/max-fragments-insufficient-for-mtu` | Validation |
| `reassembly/expected-fragmentation-rate-high` | Validation |
| `reassembly/untrusted-link-binding` | Validation |
| `reassembly/trust-class-missing-on-fragmenting-link` | Validation |
| `reassembly/stage-copy-wcet-exceeds-slot-budget` | Validation |
| `reassembly/peer-id-not-zid-on-established-session` | Validation |
| `link/listener-link-not-paired-with-established-sibling` | Validation |
| `reassembly/binding-on-unpaired-listener` | Validation |
| `link/concurrent-count-exceeds-scheduler-slots` | Mesh Deploy |
| `link/per-link-budget-exceeds-tick-period` | Mesh Deploy |
| `link/inbound-event-queue-unsized` | Validation |
| `collection/ordering-sorted-requires-index-by` | Validation |
| `collection/overflow-policy-oldest-wins-requires-ordering-insertion` | Validation |
| `collection/element-type-not-a-kind` | Validation |
| `collection/index-by-field-missing` | Validation |
| `collection/multi-writer-without-atomics` | Validation |
| `collection/capacity-unresolved` | Validation |
| `timer/period-below-tick-rate` | Validation |
| `timer/slot-overflow` | Mesh Deploy |
| `extern/symbol-not-in-whitelist` | Validation |
| `extern/abi-mismatch` | Validation |
| `extern/signature-mismatch` | Validation |
| `extern/ordering-unspecified` | Validation |
| `extern/target-plugin-symbol-conflict` | Validation |
| `scxml/top-level-script-unloaded` | Validation |
| `scxml/unsupported-datamodel` | Validation |
| `scxml/null-datamodel-forbids-construct` | Validation |
| `scxml/static-datamodel-rule` | Validation |
| `scxml/unreachable-state` | Validation |
| `scxml/dead-transition` | Validation |
| `scxml/parent-send-without-parent` | Validation |
| `scxml/generated-name-collision` | Validation |
| `scxml/undeclared-interface-event` | Validation |
| `profile/interface-not-closed` | Validation |
| `profile/name-style` | Validation |
| `profile/name-limit` | Validation |
| `profile/event-structure` | Validation |
| `profile/event-prefix-of-another` | Validation |
| `profile/evidence-unanchored` | Validation |
| `profile/element-untraced` | Validation |
| `scxml/non-exhaustive-event-handling` | Validation |
| `scxml/contradictory-unhandled-declaration` | Validation |
| `scxml/stale-unhandled-declaration` | Validation |
| `scxml/always-false-guard` | Validation |
| `scxml/shadowed-transition` | Validation |
| `scxml/recording-intercepted` | Validation |
| `scxml/self-send-discarded` | Validation |
| `scxml/on-sample-invalid-parent` | Validation |
| `scxml/on-sample-link-duplicate-in-state` | Validation |
| `scxml/on-sample-event-name-conflict` | Validation |
| `scxml/on-sample-link-not-declared` | Validation |
| `scxml/on-sample-link-wrong-kind` | Validation |
| `scxml/unknown-session-role-kind` | Validation |
| `scxml/duplicate-session-role-declaration` | Validation |
| `link/deploy-role-listener-without-scxml-accept-side-role` | Validation |
| `scxml/accept-side-role-without-listener-link` | Validation |
| `link/role-listener-with-non-session-arming-trust-class` | Validation |
| `scxml/accept-side-states-without-role-declaration` | Validation |
| `reassembly/per-peer-quota-build-invariant-violated` | Validation |
| `codegen/no-std-script-not-supported` | Generate |
| `codegen/no-std-http-not-supported` | Generate |
| `codegen/no-std-fs-load-not-supported` | Generate |
| `codegen/no-std-invoke-not-supported` | Generate |
| `expression/empty` | Expression |
| `expression/lex` | Expression |
| `expression/unsupported-construct` | Expression |
| `expression/unsupported-builtin` | Expression |
| `expression/unknown-identifier` | Expression |
| `expression/unknown-enum-variant` | Expression |
| `expression/unknown-member` | Expression |
| `expression/member-of-non-record` | Expression |
| `expression/property-not-callable` | Expression |
| `expression/namespace-not-callable` | Expression |
| `expression/namespace-not-a-value` | Expression |
| `expression/literal-not-callable` | Expression |
| `expression/strict-equality` | Expression |
| `expression/parse-mismatch` | Expression |
| `expression/unexpected-token` | Expression |
| `expression/invalid-lvalue` | Expression |
| `expression/type-coercion` | Expression |
| `expression/type-mismatch` | Expression |
| `expression/argument-count-mismatch` | Expression |
| `expression/literal-out-of-range` | Expression |
| `expression/go-ternary-unsupported` | Expression |
| `import/file-not-found` | Import |
| `import/kind-mismatch` | Import |
| `import/not-forge` | Import |
| `manifest/circular-dependency` | Manifest |
| `manifest/duplicate-document-name` | Manifest |
| `manifest/artifact-path-collision` | Manifest |
| `cli/unknown-language` | Cli |
| `cli/unsupported-language` | Cli |
| `cli/unknown-script-engine` | Cli |
| `cli/unsupported-script-engine` | Cli |
| `cli/missing-metadata-field` | Cli |
| `cli/not-a-directory` | Cli |
| `cli/invalid-format-option` | Cli |
| `cli/format-style-not-found` | Cli |
| `cli/formatter-unavailable` | Cli |
| `cli/format-failed` | Cli |
| `cli/no-scxml-tag` | Cli |
| `cli/invalid-suite-package` | Cli |
| `cli/generator-source-drift` | Cli |
| `cli/generator-source-unverifiable` | Cli |
| `cli/usage` | Cli |
| `cli/query-no-match` | Cli |
| `cli/closure-input-unusable` | Cli |
| `mesh/deploy-parse` | Mesh Deploy |
| `mesh/deploy-unsupported-version` | Mesh Deploy |
| `mesh/deploy-duplicate-machine` | Mesh Deploy |
| `mesh/deploy-invalid-ordering-timings` | Mesh Deploy |
| `mesh/deploy-invalid-dedup-window` | Mesh Deploy |
| `mesh/deploy-invalid-custom-tcp-socket` | Mesh Deploy |
| `mesh/deploy-invalid-dds-qos` | Mesh Deploy |
| `mesh/deploy-invalid-liveliness` | Mesh Deploy |
| `mesh/deploy-invalid-server-response-deadline` | Mesh Deploy |
| `mesh/deploy-invalid-outbound-buffer` | Mesh Deploy |
| `mesh/deploy-invalid-retry-policy` | Mesh Deploy |
| `mesh/deploy-invalid-auth-policy` | Mesh Deploy |
| `mesh/deploy-discovery-not-supported` | Mesh Deploy |
| `mesh/deploy-pool-not-supported-by-transport` | Mesh Deploy |
| `mesh/deploy-pool-missing-member-list` | Mesh Deploy |
| `mesh/deploy-pool-empty-member-list` | Mesh Deploy |
| `mesh/deploy-pool-binding-field-not-supported` | Mesh Deploy |
| `mesh/deploy-pool-dispatch-without-member` | Mesh Deploy |
| `mesh/deploy-pool-invalid-placeholder` | Mesh Deploy |
| `mesh/deploy-server-pool-not-supported` | Mesh Deploy |
| `mesh/deploy-cross-target-reply-not-supported` | Mesh Deploy |
| `mesh/deploy-invalid-reply-from` | Mesh Deploy |
| `mesh/deploy-unknown-binding-field` | Mesh Deploy |
| `mesh/deploy-stage-pool-not-declared` | Mesh Deploy |
| `mesh/deploy-stage-pool-wrong-kind` | Mesh Deploy |
| `mesh/deploy-stage-pool-transport-mismatch` | Mesh Deploy |
| `mesh/deploy-scxml-invoke-target-conflict` | Mesh Deploy |
| `mesh/deploy-partition-duplicate-name` | Mesh Deploy |
| `mesh/deploy-partition-multi-device` | Mesh Deploy |
| `mesh/deploy-partition-unit-duplicate` | Mesh Deploy |
| `mesh/deploy-partition-machine-not-listed` | Mesh Deploy |
| `mesh/deploy-partition-empty` | Mesh Deploy |
| `mesh/deploy-partition-name-not-identifier` | Mesh Deploy |
| `mesh/deploy-partition-synth-infix-collision` | Mesh Deploy |
| `mesh/deploy-partition-uncovered-unit` | Mesh Deploy |
| `mesh/deploy-partition-partial-coverage-requires-default` | Mesh Deploy |
| `mesh/deploy-partition-pool-machine` | Mesh Deploy |
| `mesh/deploy-partition-transport-binding-unsupported` | Mesh Deploy |
| `mesh/deploy-scxml-invoke-cross-device-transport` | Mesh Deploy |
| `mesh/deploy-someip-scxml-invoke-service-id-overflow` | Mesh Deploy |
| `mesh/deploy-someip-scxml-invoke-service-id-pin-out-of-range` | Mesh Deploy |
| `mesh/deploy-someip-scxml-invoke-service-id-pin-collision` | Mesh Deploy |
| `mesh/deploy-someip-liveness-service-id-overflow` | Mesh Deploy |
| `mesh/deploy-someip-liveness-service-id-pin-out-of-range` | Mesh Deploy |
| `mesh/deploy-someip-liveness-service-id-pin-collision` | Mesh Deploy |
| `mesh/deploy-someip-machine-liveness-service-id-overflow` | Mesh Deploy |
| `mesh/deploy-someip-machine-liveness-service-id-pin-out-of-range` | Mesh Deploy |
| `mesh/deploy-someip-machine-liveness-service-id-pin-collision` | Mesh Deploy |
| `mesh/deploy-partition-barrier-timeout-invalid` | Mesh Deploy |
| `mesh/partition-parallel-root-undesignated` | Mesh Deploy |
| `mesh/partition-parallel-root-ambiguous` | Mesh Deploy |
| `mesh/partition-parallel-root-not-in-machines` | Mesh Deploy |
| `mesh/partition-parallel-root-non-host` | Mesh Deploy |
| `mesh/partition-barrier-timeout-without-root` | Mesh Deploy |
| `mesh/partition-wire21-custom-tcp-unimplemented` | Mesh Deploy |
| `mesh/distributability-r1-shared-write` | Mesh Deploy |
| `mesh/distributability-r2-cross-region-transition` | Mesh Deploy |
| `mesh/deploy-platform-class-os-mismatch` | Mesh Deploy |
| `deploy/worker-stack-budget-missing` | Mesh Deploy |
| `deploy/worker-slot-budget-missing` | Mesh Deploy |
| `deploy/keepalive-jitter-budget-missing` | Mesh Deploy |
| `deploy/scheduler-incompatible-with-worker-count` | Mesh Deploy |
| `deploy/link-driver-unknown` | Mesh Deploy |
| `deploy/link-mtu-missing-on-fragmenting-link` | Mesh Deploy |
| `deploy/link-mtu-below-driver-floor` | Mesh Deploy |
| `deploy/link-expected-p99-exceeds-mtu` | Mesh Deploy |
| `deploy/link-burst-pps-missing-on-isr-dispatch` | Mesh Deploy |
| `deploy/link-not-declared-in-deploy` | Mesh Deploy |
| `deploy/link-not-declared-in-forge` | Mesh Deploy |
| `deploy/link-burst-absorption-insufficient` | Mesh Deploy |
| `deploy/link-rx-dispatch-worker-tick-on-high-burst` | Mesh Deploy |
| `deploy/link-driver-class-mismatch` | Mesh Deploy |
| `pool/stage-copy-policy-error` | Validation |
| `pool/stage-copy-accept-rejected-under-forbid` | Validation |
| `deploy/stage-copy-policy-unknown` | Mesh Deploy |
| `deploy/session-arming-quota-missing` | Mesh Deploy |
| `deploy/accept-rate-config-missing` | Mesh Deploy |
| `deploy/session-arming-fields-on-non-arming-link` | Mesh Deploy |
| `deploy/stateless-accept-required-on-untrusted-source` | Mesh Deploy |
| `deploy/stateless-accept-key-rotation-shorter-than-lifetime` | Mesh Deploy |
| `deploy/session-arming-quota-vs-peer-table-invariant-violated` | Mesh Deploy |
| `deploy/stateless-accept-extern-not-whitelisted` | Mesh Deploy |
| `mesh/external-parse` | Mesh External |
| `mesh/external-unresolved-names` | Mesh External |
| `mesh/external-ambiguous-event-group` | Mesh External |
| `mesh/external-empty-event-group` | Mesh External |
| `mesh/external-named-reference-without-config` | Mesh External |
| `mesh/external-reserved-someip-id-keys` | Mesh External |
| `mesh/external-someip-field-on-non-someip-transport` | Mesh External |
| `mesh/external-conflicting-event-schema` | Mesh External |
| `mesh/external-conflicting-event-field-kinds` | Mesh External |
| `mesh/external-empty-event-entry` | Mesh External |
| `mesh/topology-unresolved-targets` | Mesh Topology |
| `mesh/topology-machine-not-found` | Mesh Topology |
| `mesh/topology-receiver-not-declared` | Mesh Topology |
| `mesh/topology-absolute-source-path` | Mesh Topology |
| `mesh/topology-uncovered-events` | Mesh Topology |
| `mesh/topology-pattern-capability-violation` | Mesh Topology |
| `mesh/topology-missing-binding-field` | Mesh Topology |
| `mesh/topology-invalid-binding-field` | Mesh Topology |
| `mesh/topology-event-binding-unused` | Mesh Topology |
| `mesh/topology-ordering-cannot-be-guaranteed` | Mesh Topology |
| `mesh/topology-pool-param-name-missing` | Mesh Topology |
| `mesh/topology-subscription-source-unbound` | Mesh Topology |
| `mesh/topology-machine-lifetime-subscription-unsupported` | Mesh Topology |
| `mesh/codegen-unsupported-language` | Mesh Codegen |
| `mesh/codegen-unsupported-transport` | Mesh Codegen |
| `mesh/codegen-event-name-collision` | Mesh Codegen |
| `mesh/codegen-pool-with-rpc-client-unsupported` | Mesh Codegen |
| `mesh/codegen-host-core-unsupported` | Mesh Codegen |

### Diagnostic-only

I/O and infrastructure failures that the author cannot prevent by
editing the SCXML document. Consumers routing repairs should not
attempt an SCXML-level fix for these; they indicate build-environment
or SCE-internal issues.

| Code | Stage | Reason diagnostic-only |
|---|---|---|
| `xml/xinclude-read-error` | Xml | Filesystem read failure on an `<xi:include>` target |
| `xml/template-read-error` | Xml | Filesystem read failure on a `<sce:use>` template target |
| `import/read-error` | Import | Filesystem read failure on imported file |
| `manifest/io` | Manifest | Filesystem failure during manifest resolution |
| `generate/invalid-config` | Generate | SCE-internal codegen config |
| `generate/template-load` | Generate | SCE template asset load failure |
| `generate/template-render` | Generate | SCE template rendering failure |
| `generate/unsupported-feature` | Generate | SCXML construct exists in the model but the requested target language has no codegen path for it (e.g. `<invoke type="sce:mesh-rpc">` with `--lang rust`) |
| `codegen/mcu-class-kind-on-non-mcu-language` | Generate | Shell-only at PR-0; producer + matrix walker land with the algorithm kind in Phase A3. MCU-class kind authored against a non-MCU language target (SCE Protocol-Synthesis RFC §5.J.4) |
| `codegen/generic-kind-backend-emit-missing` | Generate | Shell-only at PR-0; producer lands with the matrix walker. Generic-class kind expected to emit on a backend per the parity matrix but the per-kind template is absent (SCE bug, SCE Protocol-Synthesis RFC §5.J.5) |
| `io/filesystem` | Io | Generic filesystem failure |
| `cli/read-input` | Cli | Input file read error |
| `cli/write-output` | Cli | Output file write error |
| `cli/create-output-dir` | Cli | Output directory creation error |
| `cli/scxml-generate` | Cli | SCE-internal codegen dispatch |
| `cli/json-serialization` | Cli | SCE-internal serde failure |
| `cli/project-root-not-found` | Cli | Build-environment discovery |
| `mesh/deploy-read` | Mesh Deploy | `deploy.yaml` read error |
| `mesh/external-read` | Mesh External | External catalog read error |
| `mesh/topology-receiver-source-read` | Mesh Topology | Receiver SCXML read error |
| `mesh/topology-receiver-source-parse` | Mesh Topology | Receiver SCXML parse error (I/O-adjacent; the receiver file is not authored by the producing machine) |
| `mesh/codegen-template-read` | Mesh Codegen | Mesh template asset read |
| `mesh/codegen-template-render` | Mesh Codegen | Mesh template rendering failure |
| `mesh/io` | Mesh Io | Generic mesh codegen filesystem failure |
| `cli/acceptance-lapsed` | Cli | `sce-codegen acceptance-check` found that the manifest, the variant, a file the design was read from, authored from or held to (the profile and the scenario set among them) or a house rule it applied moved since the acceptance record was taken; not preventable by authoring SCXML (a person accepts again with `sce-codegen accept`, or reverts what moved) |
| `cli/reserved-host-type` | Cli | A host declared a `<send>` or `<invoke>` type under `sce:` (`--host-processor`, `--host-invoker`, or the same lists on the `build.rs` facade) — the prefix SCE keeps for the processors it defines itself (`sce:mesh`, `sce:mesh-rpc`), so the declaration would replace one of them with nothing on the wire saying so; not preventable by authoring SCXML (the host picks another prefix, such as `x-`) |
| `cli/requirement-closure-broken` | Cli | `sce-codegen requirement-closure` found a claim that points out of its document and does not land in the manifests given — a `delegated` destination that never took the requirement, a delegation cycle, a decomposition child that does not exist, or a destination no manifest on the command line describes; not preventable by authoring SCXML (edit the manifest, or name the missing manifest) |
| `cli/review-table-unavailable` | Cli | `sce-codegen review-table` was asked for a kind SCE reads no requirement annotation in — no node of that kind is read for `sce:req`, so the requirement column would be empty on every row for a reason that is about SCE rather than about the document; reported instead of rendering an empty table, which a reviewer would read as a clean result. ⚠ "reads", not "the grammar refuses": a `sce:req` on a W3C-namespace element of a forge document (the `<scxml>` root, a `<data>`) is accepted by `schemas/sce-forge.xsd` — its `processContents="lax"` wildcards accept any attribute carrying no global declaration — and, being read by nobody, is refused when the parse ends as `validation/sce-attribute-unread` (since 2026-09-28; before that it was dropped in silence). Not preventable by authoring (the repair is to admit `sce:req` on that kind's nodes in `schemas/sce-forge-ext.xsd`, read it through `collect_sce_req`, and answer for the kind in `forge::requirement_nodes`) |
| `cli/pseudo-unavailable` | Cli | `sce-codegen pseudo` was given a document carrying a construct `forge::pseudo::render` does not cover. The rendering exists so that a reviewer who approves it has approved the document, which makes it total by contract: every field of the model reaches the output. A document is therefore rendered in full or refused by name, never abbreviated — a text missing part of the document reads exactly like one missing none of it, and signing it would turn an unreviewed document into a signed one. ⚠ The refusal is per DOCUMENT, not per kind: all eighteen kinds render, and the message names the construct (an `<invoke>`, an `<sce:on-sample>` block, an `<sce:context>` object) because naming the kind alone once told an author their kind was unrendered when it was not. ⚠⚠ Distinct from `cli/review-table-unavailable`, which is about a kind carrying no `sce:req` site at all; this one says nothing about annotation. Not preventable by authoring (the repair is to render the construct in `forge::pseudo`) |
| `cli/diagram-unavailable` | Cli | `sce-codegen diagram` was given a document carrying something a print figure cannot say: an action the page's own action lines (`forge::pseudo::action_lines`) refuse, a character the generated font table (`diagram::metrics`) has no measured width for, or a page language the figure's phrases (`diagram::words`) do not cover. Refused rather than drawn without it, for the pseudocode's reason — a figure is read as the whole document, and one missing a part reads exactly like one missing none. ⚠ A width is never guessed: whether a figure fits its page rests on it. ⚠ Also refused: `--manifest` on a document that is not a statechart, since the checklist's rows name a statechart's boxes and table rows (the requirements of any other kind are in `acceptance-report`). Not preventable by authoring (the repair is to render the construct, measure the character, or add the phrases) |
| `cli/diagram-does-not-fit` | Cli | `sce-codegen diagram` laid a figure out at the requested minimum type size and it is larger than the page's printable area. Refused with both sizes, never shrunk: the minimum is what the reader was promised, and the prototype that shrank figures to fit kept two of five legible on A4. Figures are already flat (one container and its direct children), so there is no deeper fold to try — the message names the container. ⚠ The same code answers a table of a non-statechart kind (`fields-<n>.svg`): its columns cannot be set in the page's width at the narrowest a column is set and the table has no row-by-row alternative, or one row is taller than the page; the message then names the field table. Preventable by the invocation (a larger page or a smaller minimum) or by authoring (fewer direct children in that container, by grouping some under a compound state) |
| `cli/profile-unusable` | Cli | `--profile` (on `check`, `generate`, `orchestrate`, `accept`, `acceptance-check`) named an authoring profile that cannot be used: unreadable, not JSON, not the shape a profile has (a setting this build does not know, a value a setting does not take), another kind of file, a version this build does not read, or an empty `name`. Refused whole rather than applied in part: a profile is the owner's statement of what a design is held to, so a run that read half of it and passed a document would say the document was held to something it was not. Keyed on which refusal it is, never on the path or the reader's sentence. Not preventable by authoring SCXML (the repair is an edit to the profile file) |
| `forge/source-hash-mismatch` | Cli | `sce-codegen verify` detected drift between an emitted file's embedded §6.2.6 header hash and the recomputed value over current source + template state; not preventable by authoring SCXML (regenerate via `sce-codegen` to repair) |
| `forge/source-hash-input-uncovered` | Cli | the §6.2.6 `source-hash` about to be embedded in generated output would not describe the input that produced it — the collected set is empty (the header would carry the empty-input digest) or, where the root was inferred from the input's own location, omits that input; an invocation-layout failure, not an authoring one (re-point `--input-root` at a directory containing the input) |
| `forge/source-hash-walk-unbounded` | Cli | the §6.2.6 source set could not be enumerated within the walk's descent ceiling — a directory symlink naming a sibling contributes under every name that reaches it, so nested levels of such links name a path count exponential in the depth; refused rather than truncated, since a digest folded over the prefix the walk reached describes a subset of the input and is unauditable in the same way the empty-input digest is. An invocation-layout failure, not an authoring one (re-point `--input-root` below the aliasing, or remove it) |
| `forge/generated-output-changed` | Cli | an `--assert-unchanged` run did all of its generation without writing and found files on disk not as that generation would leave them — holding other bytes, absent, or present though the generation would remove them. The §6.2.6 drift check done on content: it sees a hand edit, a changed template and a changed generator alike, which no header hash records. Not an authoring failure of the document — the repair is rerunning the same generation without the flag, or undoing the hand edit the comparison exposed |
| `traceability/scxml-line-range-missing` | Generate | SCE Protocol-Synthesis RFC §5.O Atomic 0 IR provenance pre-emit guard: a node eligible for SCE-MAP marker emission reaches the codegen pre-emit walker with `source_location: None`. Codegen-internal invariant — authors never see this signal in practice; the fix lives in the parser site that produced the IR node. |
| `traceability/state-id-collision` | Generate | SCE Protocol-Synthesis RFC §5.O Atomic 1 — symbol mangling collision. Two distinct IR nodes mangle to the same `<machine>__<state_path>__<artifact>` identifier (typically XInclude or `sce:template` composition importing a state fragment whose id collides with a top-level state). The repair (rename one of the two ids) is author-facing, but the wire payload also carries the two colliding `<file>:<line>` sites as candidates. |
| `traceability/symbol-name-exceeds-c-identifier-limit` | Generate | SCE Protocol-Synthesis RFC §5.O Atomic 1 — mangled symbol exceeds C99 §5.2.4.1 external-identifier limit (31 chars). Default rendering is warn (sourcemap still emits); `platform.strict_c99_identifiers: true` in deploy.yaml escalates to hard-error. Repair is to shorten any of the contributing names (machine id, state id, artifact suffix) or relax the strict flag. |
| `traceability/sourcemap-source-hash-mismatch` | Generate | SCE Protocol-Synthesis RFC §5.O Atomic 1 — sourcemap `source_hash` field drifted from the §6.2.6 header's `source-hash`. Codegen-invariant — every `sce_sourcemap.json`'s top-level `source_hash` MUST be byte-equal to the per-file header (spec lines 3321-3324). Not preventable by authoring SCXML; regenerate via `sce-codegen generate` to repair. |
| `traceability/sce-map-attribute-stripped` | Generate | SCE Protocol-Synthesis RFC §5.O Atomic 1 — Rust SCE-MAP `#[doc]` preservation heads-up (OQ-W16 b). The dual-emit fallback (`// SCE-MAP:` line comment, default since Atomic 0c) covers the strip, so this is a warn-only signal that the `#[doc]` form was not preserved by rustdoc under the named profile / target. |
| `traceability/meta-generated-source-line-marker-missing` | Generate | SCE Protocol-Synthesis RFC §5.O Atomic 1 follow-up — codegen-internal traceability invariant: every SCE-emitted file (one carrying a §6.2.6 drift header) must contain at least one `SCE-MAP:` marker line. Fires from `forge::sourcemap::validate_emitted_files_have_markers` walking `out_dir` after every successful `cmd_generate` / `cmd_generate_w3c`. ARCHITECTURE.md "Traceability Ownership Boundary" pins the scope: external meta-generator output (protoc, bindgen, cbindgen) carries no drift header and is silently out-of-scope. Not author-preventable — fix lives in the template that lost its `sce_map_marker` macro call. |
| `mcu/driver-header-not-found` | Validation | SCE Protocol-Synthesis RFC §5.2 Round F-α — a top-level `<sce:driver href="..."/>` reference on the SCXML root cannot be resolved against `deploy.yaml`'s `platform.driver_root` (or the SCXML file's parent directory as fallback). The referenced header is the author's contract with the C11 backend: `*_sm.c` `#include`s the resolved path, so absence breaks cross-TU symbol resolution before any C compiler can speak up. Repair is author-domain — fix the `href` value, add the missing file, or set `platform.driver_root` so the relative path resolves. |
| `mcu/section-attribute-on-non-mcu-target` | Generate | SCE Protocol-Synthesis RFC §5.2 Round F-α — `platform.c11_section_attribute` is set in `deploy.yaml` but the target codegen backend is not `c11`. The section attribute injects `__attribute__((section("...")))` syntax that only the C11 backend emits; non-MCU backends (cpp / rust / kotlin / go / python) have no equivalent contract and reject the field by design (Q-Round-F-D3 lock, mirrors Q-Call-7 non-MCU pattern). Repair is multi-axis — remove the section attribute, switch the backend to `c11`, or split deploy configurations per target. |
| `mcu/section-attribute-name-invalid` | Generate | SCE Protocol-Synthesis RFC §5.2 — `platform.c11_section_attribute.class` names a section the C11 emitter cannot place verbatim into a string literal. The name reaches two nested string contexts (a plain C string in `__attribute__((section("...")))`, and a string inside a string in `_Pragma("location=\"...\"")` on IAR), so a quote or backslash would terminate one of them. Accepted characters are letters, digits, `.`, `_`, `$` and `-`; the name must be non-empty. Repair is a rename in `deploy.yaml` and in the linker script that places the section. |

---

## Maintenance

- When a new `DiagnosticCode` variant is added, the compile-time
  guard `all_diagnostic_codes_is_exhaustive` fires first (in
  `sce-build/src/forge/diagnostic.rs`). Add the variant to
  `ALL_DIAGNOSTIC_CODES`, then place its slash-path in exactly one
  of the two appendix tables. The `acceptance_doc_covers_every_code`
  runtime check will otherwise fail with the specific missing code
  name.
- If a code's *classification* changes (boundary ↔ diagnostic-only),
  move the row between tables. `acceptance_doc_covers_every_code`
  asserts exactly one appendix row per code, so duplication or
  deletion during the move is caught.
- When §1 / §2 / §3 prose gains a new construct, link it from the
  relevant appendix row's "Section" column so reviewers can trace
  prose ↔ code coverage. Inline prose mentions are not guarded; only
  the appendix substring is.
- Accurate reflection of partial realization is explicit policy: if a
  §1/§2/§3 feature is in flight, state its status (see §2.5
  communication patterns / §2.6 mesh-rpc). Do not under- or
  over-claim.
