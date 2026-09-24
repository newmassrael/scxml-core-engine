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
<scxml sce:kind="codec" sce:codec-id="codec_zenoh_keyexpr">
  <datamodel>
    <sce:variant>
      <sce:arm value="0x00" type="codec_keyexpr_nonlocal" default="true"/>
      <sce:arm value="0x01" type="codec_keyexpr_local"/>
    </sce:variant>
  </datamodel>
</scxml>

<!-- Parent declares dispatch at the import site -->
<scxml sce:kind="codec" sce:codec-id="codec_zenoh_push">
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
the executable-content actions inside those blocks, and
`<invoke>`. The annotations are pure metadata — emitted code is
byte-identical to the unannotated form, byte-stable goldens stay
unchanged.

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
(`<state>`, `<final>`, `<parallel>`, `<transition>`, `<onentry>`,
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

The runtime witness is
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
| `<send eventexpr/targetexpr/delayexpr/typeexpr/idlocation/namelist>`, `<send><content expr>`, `<cancel sendidexpr>`, `<foreach>`, `<invoke idlocation>`, a hybrid `<invoke>` (`srcexpr` / `<content expr>`), `<donedata><content expr>` | No typed form: each is evaluated as script-engine text by every backend's templates |

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
transition's or `<if>`/`<elseif>`'s `cond` as `bool`, a `<log expr>` or
`<param expr>` as whatever it is. A name the scope does not carry, and a
value of a kind the place does not admit, are refused at the expression's
own range with the expression layer's codes (`expression/unknown-identifier`,
`expression/type-mismatch`, …). The ECMAScript frontend is never asked to
lower a `sce-static` document's expressions.

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
`<assign location="shown.<field>">`; assigning the whole record is refused
as `expression/unsupported-construct`. A missing field is refused on the
`sce:type` that names the record, an unknown or repeated one on its `name`,
both as `validation/attribute-rule-violated`. `<sce:set>` is its own element
because `<sce:field>` is the codec's byte-layout field.

**List variables.** `sce:type="list<T>"` (in XML `list&lt;T&gt;`) holds a
sequence of `T`, a fixed-width number or `bool` — the element an algorithm's
list admits (SCE_FORGE.md §4.12). It starts empty and takes no `expr`; it
declares the most elements it ever holds with `sce:capacity`, which is
required on a list and refused on any other variable. Two statements write
it, both naming it by `target` as E8's does:

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
because a machine must hold the same list wherever it runs. A list is not a
value an expression reads, and assigning one whole is refused; both are
`expression/unsupported-construct`. The host reads it through the snapshot.
A `target` that names no list is `scxml/static-datamodel-rule`, naming the
lists there are, and so is either statement under any other data model.

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
payload: otherwise none of it runs and `error.execution` goes on the internal
queue (W3C SCXML 3.12.2, 4.9) — what a host action whose argument reads the
payload does, checked once for the whole block because an error stops the
block. A record variable is a field of an immutable data class,
`<Machine><Alias>Record`, and a field update lowers to
`shown = shown.copy(<field> = …)`. A list variable is an immutable
`List<T>` field that starts `emptyList()`; an append lowers to
`if (picked.size < N) { picked = picked + (…) } else { <error.execution> }`
and a clear to `picked = emptyList()`, so a snapshot shares the list it
publishes without copying it. The generated machine carries no script
engine. An enum-typed variable is refused for Kotlin until a statechart
imports its enum into the generated unit.

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

Three validators below — graph reachability, event-set exhaustiveness,
and guard analysis — **reject legal SCXML**. Each flags a document the
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

---

## Appendix — `DiagnosticCode` index (379 codes)

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
| `validation/native-action-argument` | Validation |
| `validation/native-action-signature-conflict` | Validation |
| `validation/mesh-rpc-reserved-param` | Validation |
| `validation/mesh-rpc-missing-target` | Validation |
| `validation/mesh-rpc-duplicate-target` | Validation |
| `validation/removed-attribute` | Validation |
| `validation/bytes-max-size-violation` | Validation |
| `validation/duplicate-requirement-id` | Validation |
| `validation/provenance-malformed` | Validation |
| `validation/provenance-duplicate` | Validation |
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
| `algorithm/const-not-foldable` | Generate |
| `algorithm/const-fold-budget-exceeded` | Generate |
| `algorithm/const-yield-type-mismatch` | Generate |
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
| `scxml/non-exhaustive-event-handling` | Validation |
| `scxml/contradictory-unhandled-declaration` | Validation |
| `scxml/stale-unhandled-declaration` | Validation |
| `scxml/always-false-guard` | Validation |
| `scxml/shadowed-transition` | Validation |
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
| `cli/acceptance-lapsed` | Cli | `sce-codegen acceptance-check` found that the manifest, the variant or a file the design was read from moved since the acceptance record was taken; not preventable by authoring SCXML (a person accepts again with `sce-codegen accept`, or reverts what moved) |
| `cli/requirement-closure-broken` | Cli | `sce-codegen requirement-closure` found a claim that points out of its document and does not land in the manifests given — a `delegated` destination that never took the requirement, a delegation cycle, a decomposition child that does not exist, or a destination no manifest on the command line describes; not preventable by authoring SCXML (edit the manifest, or name the missing manifest) |
| `cli/review-table-unavailable` | Cli | `sce-codegen review-table` was asked for a kind SCE reads no requirement annotation in — no node of that kind is read for `sce:req`, so the requirement column would be empty on every row for a reason that is about SCE rather than about the document; reported instead of rendering an empty table, which a reviewer would read as a clean result. ⚠ "reads", not "the grammar refuses": a `sce:req` on a W3C-namespace element of a forge document (the `<scxml>` root, a `<data>`) is accepted by `schemas/sce-forge.xsd` — its `processContents="lax"` wildcards accept any attribute carrying no global declaration — and then read by nobody, which is a separate silent-drop defect of row S2's class. Not preventable by authoring (the repair is to admit `sce:req` on that kind's nodes in `schemas/sce-forge-ext.xsd`, read it through `collect_sce_req`, and answer for the kind in `forge::requirement_nodes`) |
| `cli/pseudo-unavailable` | Cli | `sce-codegen pseudo` was given a document carrying a construct `forge::pseudo::render` does not cover. The rendering exists so that a reviewer who approves it has approved the document, which makes it total by contract: every field of the model reaches the output. A document is therefore rendered in full or refused by name, never abbreviated — a text missing part of the document reads exactly like one missing none of it, and signing it would turn an unreviewed document into a signed one. ⚠ The refusal is per DOCUMENT, not per kind: all eighteen kinds render, and the message names the construct (an `<invoke>`, an `<sce:on-sample>` block, an `<sce:context>` object) because naming the kind alone once told an author their kind was unrendered when it was not. ⚠⚠ Distinct from `cli/review-table-unavailable`, which is about a kind carrying no `sce:req` site at all; this one says nothing about annotation. Not preventable by authoring (the repair is to render the construct in `forge::pseudo`) |
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
