# Where ECMAScript becomes Lua, per backend

Measured 2026-08-27 against `0824c496c7`. This file is the starting point for
closing the ECMA-262 divergences (`tests/ecmascript/lua_engine_divergences.json`,
`tests/ecmascript/kotlin_lua_divergences.json`), and it exists because the
answer is not the same on every backend and nothing wrote it down.

## The question

`datamodel="ecmascript"` is a claim about a language. Lua is not that language,
so every backend that runs SCXML expressions on Lua has to translate. There are
exactly two places to do it:

- **Build time.** `sce-build`'s ECMAScript frontend parses the expression once
  and emits Lua. The filters are the single source of truth —
  `sce-build/src/filters.rs` registers `to_lua_guard`, `to_lua_expr`,
  `to_lua_script`, `to_lua_data_content`, `to_lua_assign_content`, each
  delegating to `crate::ecmascript`.
- **Run time.** The generated code hands the *original ECMAScript text* to an
  engine, and a text rewriter turns it into Lua on the spot —
  `sce/src/scripting/EcmaScriptToLuaTransformer.cpp` (2127 lines) and
  `backends/kotlin/lua/.../EcmaScriptToLuaTransformer.kt`.

The divergences live entirely in the second place. The divergence file states
the prescription itself: *"Closing these means parsing the expression, which SCE
already does once, in the build-time frontend that every backend uses."*

## What each backend actually emits

`cond` and `expr` take the same route within a backend, so one column covers
both. Sites are the guard emission; the `expr` column names one representative
site of the same shape.

<!-- sce:lua-translation-seam — parsed by sce-build/tests/script_engine_language_parity.rs -->

| `--lang` | Guard site | Filter chain | Hands the engine |
|---|---|---|---|
| `cpp` | `tools/codegen/templates/process_transition.jinja2:638` | `{{ trans.cond \| escape_cpp }}` | **ECMAScript source** |
| `kotlin` | `tools/codegen/templates/kotlin/transition_actions.kt.jinja2:24` | `{{ trans.cond \| escape_kotlin }}` | **ECMAScript source** |
| `rust` | `tools/codegen/templates/rust/state_machine.rs.jinja2:996` | `{{ trans.cond \| to_lua_guard \| escape_rust }}` | translated Lua |
| `go` | `tools/codegen/templates/go/process_transition.go.jinja2:170` | `` {{ trans.cond \| to_lua_guard }} `` | translated Lua |
| `python` | `tools/codegen/templates/python/process_transition.py.jinja2:35` | `{{ transition.cond \| to_lua_guard \| py_string_literal }}` | translated Lua |
| `c11` | `tools/codegen/templates/c/state_machine.c.jinja2:3554` → `se.lua_eval_guard` | `{{ cond \| to_lua_guard \| escape_c }}` (`c/scriptengine.jinja2:612`) | translated Lua |

The first column is the `--lang` spelling rather than a display name because
`script_engine_language_parity` parses this table and compares it with what
the code derives — three surfaces (this table, the manifest field, the field's
doc) held to one answer instead of agreeing by inspection.

Representative `expr` sites, same split: C++ `datamodel_macros.jinja2:21`
(`escape_cpp`), Kotlin `kotlin/scriptengine_helpers.kt.jinja2:49`
(`escape_kotlin`), Rust `rust/datamodel_macros.rs.jinja2:45`, Go
`go/actions/log.go.jinja2:7`, Python `python/actions/log.py.jinja2:8`, C11
`c/invoke_methods.jinja2:309` — the last four all `to_lua_expr`.

**Correction to the earlier reading.** Go and Python were reported as
translating and C++ as passing source; that holds. What had not been measured
is that **Kotlin is on the C++ side, not the Lua-translating side**, and that
**Rust and C11 are on the translating side**. Kotlin matters most here: it is
one of the two files (E) names, and it is a runtime rewriter precisely because
its generated code receives source text.

Two other facts the table would otherwise hide:

- C++ and Kotlin both lower a guard **natively** when they can
  (`trans.cond_cpp` / `trans.cond_kt`, `is_pure_in_predicate`,
  `cond_constant`). The source-text branch is the *script-engine* branch only.
- The only `to_lua` mention under the C++ templates is a comment
  (`invoke_methods.jinja2:240`), not a use. C++ templates live at the template
  tree root, so a per-directory count misses them — count from the root.

### Re-deriving this table

```sh
# who translates at build time
grep -rn "to_lua_guard" tools/codegen/templates/
# who hands over source text (C++ templates are at the tree ROOT)
grep -rn "escape_cpp\|escape_kotlin" tools/codegen/templates/ | grep -i "cond\|expr"
```

## What moving to engine-fixed codegen would actually cost

Pre-translating means the emitted artifact is Lua-shaped, so it can only run on
a Lua engine — the generated code stops being engine-agnostic. Measured, here
is what that costs on each side:

- **C++: nothing that exists today.** The engine is selected at *configure*
  time — `SCE_SCRIPT_ENGINE` is a CMake cache option with `STRINGS
  "quickjs" "lua"` (`sce/CMakeLists.txt:399`) — and every one of the 247
  in-tree call sites obtains it from `ScriptEngineProvider::getScriptEngine()`,
  a singleton fixed by that option. The `IScriptEngine&` parameter on generated
  code is an indirection over a compile-time choice, not a runtime one. No
  in-tree consumer constructs an engine and injects a different one.
- **Kotlin: a real choice, and its default is a correct engine.** Generated
  Kotlin takes `scriptEngine: ScxmlScriptEngine` as a *constructor argument*
  (`kotlin/state_machine.kt.jinja2:69`), and `EngineFactory` offers three —
  Rhino, Lua 5.4, QuickJS. Its callers in-tree are the JMH benchmarks
  (`backends/kotlin/benchmark/src/jmh/...`) and the Android app's benchmark
  harness; the W3C suite selects with `-Psce.script.engine=`, and
  `W3CTestBase.engineFor` **refuses an unknown name rather than defaulting**
  (it used to end `else -> RhinoScriptEngine()`, which made a misspelt engine
  a green run under another engine's name). `W3CTestBase.DEFAULT_ENGINE` is
  **`"rhino"`** — so unlike C++, whose default `quickjs` already bypasses the
  rewriter, Kotlin's default is an ECMAScript engine reached through the same
  constructor slot a Lua-shaped artifact would close.

**The one sentence.** Moving translation to build time costs the ability to run
one generated artifact on more than one engine, and the only place that ability
is exercised in this tree is Kotlin — where the two engines that answer every
ECMA-262 case (Rhino, QuickJS) reach the machine through the same constructor
slot a Lua-shaped artifact would close, and Rhino is the default — so the
design must take the target engine as a codegen input (`script_engine_language`
already exists on the manifest, `sce-build/src/manifest.rs:209`) rather than
assume one.

## Found while measuring: the manifest names the wrong engine on two backends

`needs_script_engine` tells a host it must supply an engine;
`script_engine_language` tells it *which kind*, and the field's own doc says it
exists because "a consumer reading only the flag, and the document's
`datamodel="ecmascript"`, would supply the wrong one"
(`sce-build/src/manifest.rs:199`).

It is derived from the flag alone
(`sce-build/src/bin/sce_codegen.rs:782`) and the constant is hard-coded
`"lua"` (`manifest.rs:45`), justified by a claim this table refutes: *"Not
per-backend: the lowering happens in `sce-build`, before any backend renders,
so every language's generated machine evaluates the same Lua."* That holds for
Rust, Go, Python and C11. It does not hold for C++ and Kotlin, which hand the
engine ECMAScript source.

Measured 2026-08-27 on `examples/ai_loop/ai_loop.scxml`:

```
lang=cpp     needs_script_engine=True  script_engine_language='lua'
lang=kotlin  needs_script_engine=True  script_engine_language='lua'
lang=go      needs_script_engine=True  script_engine_language='lua'
lang=python  needs_script_engine=True  script_engine_language='lua'
lang=rust    needs_script_engine=True  script_engine_language='lua'
lang=c11     needs_script_engine=True  script_engine_language='lua'
```

So a C++ or Kotlin host that obeys the manifest supplies a Lua engine for a
machine that hands it ECMAScript — and on both, the default engine is *not*
Lua (`SCE_SCRIPT_ENGINE=quickjs`, `W3CTestBase.DEFAULT_ENGINE="rhino"`). This
is the same root cause as the divergences, surfacing on a wire surface rather
than in an answer.

**Fixed 2026-08-27.** The field is now the target backend's answer, and the
wire vocabulary is `"lua" | "ecmascript"` (`SCRIPT_ENGINE_LANGUAGES`).

**It is derived, not listed.** `Language::script_engine_language` reads
`Language::lowers_expressions_at_build_time`, which asks the embedded template
registry — the same one the renderer uses — whether any template this backend
OWNS applies `to_lua_guard`. Ownership is `Language::template_owned_subdir`,
already the tree's answer to "whose templates are these": five backends own a
subdirectory and C++ owns whatever no other backend claims. So the table above
and the field cannot drift: moving a backend across the seam is a template
edit and nothing else. This is the shape the mesh-rpc refusal uses, which
reads `templates/mesh/<lang>/` rather than asserting which backends have a
mesh arm.

⚠ Jinja comments are stripped before that search, because a template may
*mention* the filter while emitting source — the C++ tree's only mention of
these filters is exactly that (`invoke_methods.jinja2`). Measured both ways on
2026-08-27: adding a live `{{ 'x' | to_lua_guard }}` to a Kotlin-owned
template flipped its reported language to `lua` and turned the gate red
(`kotlin` emits no lowered Lua … but the manifest says `lua`), while leaving
only `{# … to_lua_guard #}` behind left it green. The schema's `enum` was
widened to match and its description — which had asserted the refuted "Not
per-backend" claim — rewritten; `SCE_ERROR_CONTRACT.md` §10.1 gained the row
it never had. A run that spans backends (`check` sweeps all six) omits the
field rather than picking one language to be wrong about.

`sce-build/tests/script_engine_language_parity.rs` holds it: it generates this
document on all six backends and decides which side each is on **from the
emitted artifact**, not from the templates. The marker is `_scxml_truthy(` —
what build-time lowering leaves behind. It is deliberately not the author's
ECMAScript, because the first run of that gate reported C11 as source-passing:
every backend echoes the original expression into a comment beside the guard,
so a scan that reads comments cannot tell output from documentation.

Restoring the old constant turns that gate red naming `cpp`, which is the
witness that it measures the thing it claims to.

## The rewriters are an engine's input adapter, so the close is retirement

Measured 2026-08-27: the C++ rewriter is called from `LuaEngine.cpp:535`
(`transformer_.transform`) and `:573` (`transformScript`), and `LuaEngine` is
compiled in only under `SCE_SCRIPT_ENGINE_LUA`. The Kotlin one is called from
`LuaScriptEngine.kt:143`. Neither is a stage in the pipeline; each is the
**input adapter of a Lua engine**, reached only when someone selects Lua.

Two consequences for closing the ECMA-262 divergences:

1. Nothing needs to be *fixed* in either rewriter. A backend generated with
   build-time lowering hands its Lua engine Lua, so the adapter has nothing
   left to adapt and that path stops being able to diverge. The lists empty
   because the code is bypassed, not because it was repaired — which is what
   `lua_engine_divergences.json` meant by "Closing these means parsing the
   expression, which SCE already does once".
2. The **C++ Interpreter cannot be closed that way.** It has no build step:
   it parses SCXML at run time and evaluates the author's expressions
   directly, so on the Lua selection it reaches the adapter no matter what
   codegen emits. That is the residue this axis is most likely to end up
   contracting rather than closing, and `sce-build`'s cdylib is the only
   candidate route — `sce-build/Cargo.toml:138` records that nothing in-tree
   consumes it, and `sce-build-wasm` has already built that wrapping once.

## Moving C++ across: the engine seam comes first, and here is why

Measured 2026-08-28, before touching a template. The obvious order — split the
C++ templates, then see what breaks — is wrong, and it fails silently.

**38 sites must move together — the number survived a re-derivation but the
SET did not.** This section first said 26, counted by looking for `escape_cpp`
on an expression-bearing field. Re-measured 2026-08-28 against `37b1452386`,
that method is wrong **five** ways, each of which hides sites rather than
showing them:

1. **A site can hand the engine raw source with no filter at all.**
   `actions/foreach.jinja2:48,94` pass `{{ action.array }}` unfiltered to
   `ForeachHelper::executeForeachWithActions` / `…WithoutBody`, which
   evaluates it. No `escape_cpp`, so no filter-keyed scan can see them.
2. **A helper can re-enter the engine with the ORIGINAL text — but only where
   it is actually passed one.** `ScriptResultUtils::resultToString` takes
   `(result, engine = nullptr, sid = {}, originalExpression = {})`
   (`sce/include/scripting/ScriptResultUtils.h:38`), and the last three
   default. Of the eighteen `resultToString` call sites in the C++ templates,
   exactly **three** pass them — `actions/send.jinja2:80,113,533`. The other
   fifteen call the one-argument form, which cannot reach the engine at all.
   ⚠ This paragraph previously also named `actions/send.jinja2:901`,
   `actions/log.jinja2:12` and `entry_exit_actions.jinja2:269`; all three are
   one-argument calls. Sort them by whether `sessionId_` appears inside the
   parentheses, not by the function's name.
3. **A shape can appear twice.** `DataModelInitHelper::initializeVariableFromExpr`
   is at `datamodel_macros.jinja2:20` *and* `scriptengine_helpers.jinja2:164`;
   the first count caught only the one it happened to grep for.
4. **An entry point the list does not name.** `actions/send.jinja2:213,569,656`
   hand `{{ action.delayexpr }}` / `{{ action.eventexpr }}` to
   `IScriptEngine::getVariable` (`IScriptEngine.h:114`) — a *name lookup*, not
   an evaluator, so an entry-point list built from "what evaluates" omits it
   while the templates route model expressions through it anyway. These three
   are what replace the three phantom `resultToString` sites above: the total
   was right by two errors cancelling.
5. **A multi-line call.** Five sites put the call on one line and the
   expression on the next — `datamodel_macros.jinja2:20`,
   `scriptengine_helpers.jinja2:164`, `entry_exit_actions.jinja2:301`,
   `actions/foreach.jinja2:48,94`. Any line-keyed grep, entry-point or filter,
   reports 33 rather than 38 unless it joins the call.

Count by ENTRY POINT instead — what the generated code calls to reach the
engine — and re-derive rather than trusting this number:

```sh
# C++-owned templates = everything outside rust/ kotlin/ go/ python/ c/
# getVariable( is in the list because of shape 4; the -A2 is because of shape 5.
grep -rnE -A2 "evaluateExpression\(|safeEvaluateGuard\(|executeScript\(|initializeVariableFromExpr\(|resultToString\(|resultToStringArray\(|executeForeachWith|getVariable\(" \
  tools/codegen/templates/*.jinja2 tools/codegen/templates/actions/*.jinja2 \
  tools/codegen/templates/_macros/*.jinja2
```

Measured 2026-08-28 on `37b1452386`: **38** such sites carry a model
expression (`cond` / `expr` / `content` / `typeexpr` / `targetexpr` /
`sendidexpr` / `delayexpr` / `contentexpr` / `srcexpr` / `eventexpr` /
`array`) — 33 on the call's own line, 5 on the line after it. ⚠ This method
is still shape-based: a NEW helper that evaluates on the caller's behalf would
be missed the same way `ForeachHelper`, `resultToString` and `getVariable`
were. Whoever does the split should widen the entry-point list from
`sce/include/scripting/` rather than assume this one is closed.

**One site carries no model expression and still has to move.**
`scriptengine_helpers.jinja2:87` evaluates the literal `"undefined"` for a
`<data>` element with neither `expr` nor child content. It is ECMAScript text
the engine evaluates, with no model field for any field-keyed filter to catch,
and under a Lua-shaped artifact it has to be emitted as `nil`. Counting "sites
carrying a model expression" is the right frame for *the split*; it is not the
same set as "text the engine must be able to read".

**Noted while counting, and NOT part of this axis.** Routing `delayexpr` and
`eventexpr` through `getVariable` means only a bare variable name resolves —
`getVariable` looks a name up, it does not evaluate. `delay`'s expression form
(§scxml-6.2) and `eventexpr` (§scxml-6.2.1) admit any expression, so
`{{ action.delayexpr }}` spelled `d.delay` or `'e' + n` would fail there under
*any* engine, Lua or not. That is a conformance question about C++ codegen,
independent of which language the string is in, and it should be measured on
its own rather than folded into the seam work.

Splitting a subset is not a smaller version of this change: the engine would
receive Lua from some sites and ECMAScript from others, in one session — and
the sites easiest to miss are exactly the ones no filter marks.

**The blocker.** `LuaEngine` transforms on every first evaluation, at **three**
sites — one per expression entry point, including the one generated code never
calls:

| Transform call | Enclosing function |
|---|---|
| `LuaEngine.cpp:535` | `validateExpression` (dead surface in the templates, but it still rewrites) |
| `LuaEngine.cpp:573` | `executeScriptInternal` (`transformScript`) |
| `LuaEngine.cpp:618` | `evaluateExpressionInternal` |

Nothing else in `sce/src` calls the transformer — `LuaDOMBinding.cpp` includes
the header without using it — so those three are the whole runtime surface.
The "fast path" above each (`:563`, `:598`) is a memo cache keyed on the
*input string*, not a bypass for text that is already Lua. Feeding it lowered
Lua therefore runs the rewriter over the frontend's own output, and
`transformArrayIndexing` (`EcmaScriptToLuaTransformer.cpp:1101`) rewrites
`arr[0]` to `arr[0 + 1]` — so an index the frontend already made 1-based is
shifted **again**. That is an off-by-one with no diagnostic, which is worse
than the divergences this change exists to remove.

**`IScriptEngine` has no seam for it, and the entry-point set is bigger than
a grep of the templates suggests.** Derived from the headers rather than from
call sites, because a method the templates do not call today is still surface
the seam has to answer for:

| Entry point | Where | Called by generated C++ today |
|---|---|---|
| `executeScript` | `IScriptEngine.h:76` | yes |
| `evaluateExpression` | `IScriptEngine.h:84` | yes |
| `validateExpression` | `IScriptEngine.h:93` | **no** — implemented by both engines, called by no template and nowhere in `sce/src` outside them |
| `DataModelInitHelper::initializeVariableFromExpr` | `sce/include/core/` | yes (2 sites) |
| `ForeachHelper::executeForeachWithActions` / `…WithoutBody` | `sce/include/core/ForeachHelper.h` | yes (2 sites, unfiltered) |
| `ForeachHelper::setLoopVariableFromExpr` | `ForeachHelper.h:121` | no |
| `ScriptResultUtils::resultToString(…, originalExpression)` | `ScriptResultUtils.h:38` | yes (**3** sites — `send.jinja2:80,113,533`; the other fifteen `resultToString` calls pass one argument) |
| `ScriptResultUtils::resultToStringArray(…, originalExpression)` | `ScriptResultUtils.h:49` | no |
| `getVariable` | `IScriptEngine.h:114` | yes (**3** sites — `send.jinja2:213,569,656`, each handed `delayexpr`/`eventexpr`) |

All of them mean "the author's source", and nothing in
`sce/include/scripting/` or `sce/src/scripting/` accepts pre-lowered text.
This section previously said the interface had "exactly two evaluation entry
points"; it has three, and `validateExpression` being dead surface is its own
decision — extend it with the seam or retire it — rather than something to
discover while implementing.

**Answered 2026-08-28 for the three `IScriptEngine` rows**, which now take a
`ScriptSource` — see "Landed 2026-08-28" below; `validateExpression` was
extended rather than retired. The six helper rows still take a bare string and
are what the template split has to widen next.

⚠ `getVariable` is on this table for a reason worth keeping: it is **not** an
evaluation entry point, and it was missed precisely because the table was
built by asking which methods evaluate. What decides membership is whether a
generated call carries the AUTHOR'S TEXT across the boundary, not what the
callee then does with it. A row that took the last-argument default —
`resultToString`'s `originalExpression` — was over-counted for the mirror-image
reason: the name was read instead of the call.

**The seam is not "skip the transformer".** `LuaEngine::evaluateExpressionInternal`
does five things to the text, and only the first is the rewrite. Measured
2026-08-28 at `LuaEngine.cpp:617-660`:

1. `transformer_.transform(expression)` — the rewrite. **This is the only step
   a pre-lowered path skips.**
2. `isUndeclaredSimpleVariable(...)` → `ReferenceError: <expr> is not defined`.
   A W3C semantic, not an optimisation: JavaScript throws for an undeclared
   variable and Lua silently returns `nil`, so dropping this would make
   lowered generation answer `nil` where the language answers an error.
3. `"return " + luaExpr` — a lowered expression still has to be wrapped to
   yield a value.
4. The per-session chunk cache, keyed on the wrapped text.
5. The assignment fallback: when `return <expr>` fails to compile (`x = 5`),
   retry as a statement — with a §scxml-5.9 guard so a bare `return`, a valid
   Lua chunk but not a JS expression, still fails (W3C test 344).

A pre-lowered entry point that only does 3+4 would compile and pass casual
tests while quietly losing 2 and 5. Steps 2–5 belong to *evaluating Lua*, not
to *translating ECMAScript*, so the seam has to be a branch inside this
function — or a shared tail both paths call — rather than a second, simpler
implementation beside it.

**Kotlin's seam is wider, not narrower.** Measured the same way, on
`backends/kotlin/lua/.../LuaScriptEngine.kt`: **five** transform call sites
against C++'s three, and the same W3C semantics riding on them.

| Transform call | Enclosing | Note |
|---|---|---|
| `:143` | `evaluateCondition` | passes `ExpressionContext.Guard` — a context argument C++ has no equivalent of |
| `:151` | `evaluateExpr` | followed by the `ReferenceError` check (`:153-155`) and `return` wrapping (`:161`) |
| `:185` | `executeScript` | |
| `:224` | `assign` | `return` wrapping at `:234` |
| `:301` | `executeForeach` | the array expression; `return` wrapping at `:304` |

⚠ **That table is the 2026-08-28 measurement and it is now history.** All five
entry points take a `ScriptSource`, and the five transform calls have collapsed
into the **two** that live inside the seam branch — see "Landed 2026-08-30"
below. Do not re-derive from these line numbers: what answers "which members
still rewrite unconditionally" is
`ScriptLanguageSeamTest.everyRewriteIsReachedThroughTheSeamBranch`, which reads
the file rather than this table, and which names the offending member when the
answer is not "none".

So the same rule holds on both: skip only the rewrite, keep the
`ReferenceError` check and the `return` wrapping. The extra two sites
(`assign`, `executeForeach`) are places C++ reaches through helpers rather
than through the engine directly, which is why the transform counts differ.

⚠ **But the work is NOT "the same, distributed differently", as this
paragraph used to end.** Re-measured 2026-08-28 while landing the C++ seam,
the SOURCE half is load-bearing in far more places on Kotlin. C++'s
`LuaEngine` names the expression at exactly **two** sites — the
`ReferenceError` and the debug log — and returns Lua's own message everywhere
else. Kotlin's `LuaScriptEngine` formats its own failures and interpolates the
author's text at **eight**: `:155`, `:175`, `:178` (`evaluateExpr`), `:231`,
`:238`, `:244` (`assign`, all three assignment paths) and `:308`, `:313`
(`executeForeach`, `$array`). Every one of them would name lowered Lua under a
one-string entry point. Whoever does the Kotlin seam should count from
`ScriptEngineException("` rather than from the transform calls: the transform
sites say where the rewrite is skipped, and these say where the source is
still needed afterwards.

**So the seam cannot be a one-string signature.** Measured 2026-08-28 at
`LuaEngine.cpp:618-626` and `LuaScriptEngine.kt:151-155`: both engines run the
undeclared-variable check on the LOWERED text and then build the diagnostic
from the ORIGINAL — `"ReferenceError: " + expression + " is not defined"` in
C++, `"ReferenceError: $expr is not defined"` in Kotlin. Step 2 of the five is
therefore two strings, not one. An entry point that receives only pre-lowered
Lua would report `ReferenceError: <lua text> is not defined`, naming a language
the author never wrote, and the C++ debug log (`LuaEngine.cpp:632`, which
prints `expression -> wrapped`) would lose the same half.

That is not a message-formatting detail: `_event.data` on `error.execution`
carries this text, so it is a wire-visible answer. The seam has to carry the
author's source ALONGSIDE the lowered text — the pre-lowered call is
`(lowered, source)`, and the rewriting call stays `(source)` with `lowered`
computed. Both paths then converge on one shared tail holding steps 2–5, which
is what keeps this from becoming a second implementation beside the first.
⚠ The build-time frontend must therefore emit both halves, and the sourcemap
surface is where that pairing already has a home.

**So the order is: seam, then templates.** The seam is a contract about *what
language the string is*, which means an engine that cannot evaluate that
language must refuse rather than try — QuickJS handed Lua is the case, and the
mesh-rpc refusal in `sce-build/src/generator.rs` is the shape that refusal
should take. Only once an engine can be handed lowered text safely does
`--script-engine lua` have anywhere to send it.

## Landed 2026-08-28: the seam exists on `IScriptEngine`

`sce/include/scripting/ScriptSource.h` is the pair this section argued for.
`ScriptLanguage` is `ECMAScript | Lua`, spelled to match the manifest's
`script_engine_language` wire vocabulary so the tag an engine is handed and the
field a host reads cannot become two names for one answer. `ScriptSource`
carries `text()` — what the engine evaluates — beside `source()`, the author's
ECMAScript. There is deliberately no one-argument `lua()`: a caller with no
authored text must pass the Lua twice and thereby say that its diagnostics will
name Lua.

The three entry points now take one. They are **non-virtual**, and what each
engine implements is a `do*` hook:

| Entry point | Takes | Hook |
|---|---|---|
| `executeScript` | `ScriptSource`, or `std::string` meaning `ecmascript(...)` | `doExecuteScript` |
| `evaluateExpression` | same | `doEvaluateExpression` |
| `validateExpression` | same | `doValidateExpression` |

Non-virtual because the refusal is a **contract, not an engine detail**: the
public entry point asks `acceptsLanguage()` and returns `refuseLanguage()`
before the hook is reached, so a third engine cannot forget it. Two new virtuals
carry the engine's own answer — `nativeLanguage()` (the engine-side mirror of
the manifest field) and `acceptsLanguage()` (true for the native language and
for any language the engine owns an *adapter* for). `LuaEngine` answers
`Lua` + accepts both, because `EcmaScriptToLuaTransformer` is that adapter;
`JSEngine` answers `ECMAScript` + accepts only ECMAScript, which is what makes
QuickJS-handed-Lua a refusal rather than a syntax error in someone else's
language.

**`validateExpression` was extended, not retired.** It is dead in the templates
but live in `tests/engine/JSEngineBasicTest.cpp` and
`ShutDownEngineAnswersTest.cpp`, so retiring it is a removal with its own
witnesses to move; treating it like its two siblings costs one hook and keeps
the interface uniform. Retirement stays available and is now a smaller change
than it was.

Inside `LuaEngine`, the seam is **one branch**, exactly as this document
demanded: `loweredTextOf` / `loweredScriptOf` pass Lua through and send
ECMAScript to the transformer, and everything after — the `ReferenceError`
check, the `return` wrapping, the chunk cache, the assignment fallback —
is the shared tail both paths run. The `ReferenceError` is built from
`source()` while the check runs on the lowered text, which is the two-string
requirement above, landed.

⚠ **The per-session fast paths are keyed on the incoming text, so they carry
the language too.** `arr[1]` means two different chunks depending on which tag
it arrived with; a cache that answered the first caller's chunk to the second
would reintroduce the same silent off-by-one from the other direction. A
language mismatch is a miss.

⚠ **`IScriptEngine.h` must not carry these bodies.** They are defined in
`sce/src/scripting/IScriptEngine.cpp` (in `sce_base`, since they name no
engine). Measured while landing this: with the bodies inline, that header —
which every generated state machine includes — changed GCC 13's inlining and
surfaced the known `-Wmaybe-uninitialized` false positive in `std::variant`'s
move constructor, failing `W3CTestRunner_Test561.cpp` under `-Werror` when it
had compiled clean at `8023a18b41`. The repo already names that false positive
at `tests/CMakeLists.txt:221`; suppressing it on one more target would have
hidden a header that had simply grown code it did not need to carry.

`tests/engine/ScriptLanguageSeamTest.cpp` (ctest `ScriptLanguageSeam`, 7 cases)
is the witness, and it names `LuaEngine` and `JSEngine` rather than reading the
provider, for the reason `DomReadSurfaceTest` records: no gate configures
`-DSCE_SCRIPT_ENGINE=lua`, so whichever half this build did not select would be
compiled by every build and run by none. Each case carries its own control, so
none of them can pass by measuring nothing:

- lowered `arr[1]` answers the author's **first** element, and the same
  characters tagged ECMAScript answer the **second** — the control is what
  proves the skip is a real bypass rather than a no-op;
- the `ReferenceError` names `nosuchvar[0]` and not `nosuchvar[1]`;
- QuickJS refuses lowered Lua naming both languages, **and** evaluates
  ECMAScript in the same session, so the refusal is about the language;
- the provider's engine reports the language `SCE_SCRIPT_ENGINE` selected.

Re-derive the red witness rather than trusting this paragraph: ignoring the tag
in `loweredTextOf`, reporting `text()` instead of `source()` in the
`ReferenceError`, and making `JSEngine::acceptsLanguage` answer `true` turn
**5 of the 7** red, each attributable to one break (measured 2026-08-28; the
two that stay green are the two neither break touches).

What this does **not** yet do: no template emits a `ScriptSource::lua(...)`
call, so nothing in the tree crosses the seam in anger. The 38 sites are the
next step, and the entry-point list they must widen from is
`sce/include/scripting/` — not this document's table.

## Landed 2026-08-28: the helpers, and the set was not the one named

The helper row of the table above named three — `DataModelInitHelper`,
`ForeachHelper`, `ScriptResultUtils`. Re-derived from the tree rather than from
that list, the generated C++ calls **eight**:

```sh
# which helpers the C++-owned templates actually call
for h in DataModelInitHelper ForeachHelper ScriptResultUtils \
         AssignmentExecutionHelper DataModelReadHelper DoneDataHelper \
         FinalizeHelper GuardHelper; do
  grep -rhoE "$h::[A-Za-z_]+" tools/codegen/templates/*.jinja2 \
       tools/codegen/templates/actions/*.jinja2 \
       tools/codegen/templates/_macros/*.jinja2 | sort -u
done
```

`AssignmentExecutionHelper`, `DoneDataHelper`, `FinalizeHelper`,
`GuardHelper` and `DataModelReadHelper` were the five nobody had counted —
which is the failure mode this document predicted in the same breath as the
38: *"a NEW helper that evaluates on the caller's behalf would be missed the
same way"*. All eight now take a `ScriptSource` on every parameter that
carries the author's text.

**A bare string still means the author's ECMAScript.** `ScriptSource` gained an
implicit constructor from `std::string` / `const char *`, the way
`std::filesystem::path` does, so several hundred existing call sites keep
saying what they already said instead of being churned to say it explicitly.
The three `std::string` overloads on `IScriptEngine` went away with it: with an
implicit conversion a string LITERAL would be ambiguous between the two
readings, and one entry point per operation is the better shape anyway. The
failure mode of the implicit reading is the safe one — a site that should have
passed lowered Lua and did not gets the author's text rewritten, which is
exactly today's behaviour, so a missed site stays *diverging* rather than
becoming newly wrong.

### The part a type swap could not carry: composition

Two helpers do not forward the expression, they **glue onto it** —
`AssignmentExecutionHelper` builds `location = (expr);` and runs it as a
script; `<donedata>` joins params. `ScriptSourceBuilder` is what keeps the two
halves in step there: evaluated text accumulates with evaluated text, authored
with authored, and punctuation lands in both. A part in the wrong language is
refused from both halves rather than mixed in.

Shape questions stay on `source()`: *is this a system variable*, *is this
location a simple name*, *is this a function literal* are questions about what
the AUTHOR wrote — §scxml-5.10 names `_event`, not whatever a lowering spells
it as.

### And the part composition could not carry either: the spelling differs

Some of that glue is not punctuation, it is **ECMAScript**. Measured
2026-08-28 there are **eight** composition sites, and `sce/include/scripting/ScriptDialect.h`
is now the single place that says how each is spelled per language:

| question | ECMAScript | Lua | same? |
|---|---|---|---|
| stringify | `JSON.stringify(x)` | `JSON.stringify(x)` | **yes** — `JSON` is a real Lua table (`json_builtins.lua:14`) |
| isArray | `(x) instanceof Array` | `_isArray(x)` | no (`ecma_semantics.lua:102`) |
| typeOf | `typeof (x)` | `_typeof(x)` | no (`ecma_semantics.lua:78`) |
| lengthOf | `(x).length` | `#(x)` | no |
| elementAt | `(x)[i]` | `_scxml_index(x, i)` | no (`ecma_semantics.lua:248`) |
| temp bind | `var n = (x)` | `n = (x)` | no — Lua has no `var` |

Only the first survives verbatim. ⚠ `elementAt` takes the index the CALLER
counts in — 0-based in both spellings, because `_scxml_index` does the shift to
Lua's 1-based storage itself. A caller that pre-shifted would move the element
twice, which is the seam's own off-by-one arriving by another road.

**Two of these are on the generated path, not one.** `resultToString`'s
`stringify` is the harmless one; `ForeachHelper::evaluateForeachArray`'s
`isArray` probe is reached from both `<foreach>` entry points the templates
emit, and as `(<lowered lua>) instanceof Array` it would have been a syntax
error that this helper reports as *"not an iterable collection"* — a
`<foreach>` over a perfectly good array raising `error.execution` and iterating
nothing.

### One template moved, because a vector cannot convert element-wise

`entry_exit_actions.jinja2` emits donedata params as
`std::vector<std::pair<std::string, std::string>>`, and the implicit
conversion that saved every other call site does not reach inside a container.
That site now emits `ScriptSource::ecmascript(...)` explicitly — the first
template in the tree to name the language. Since `template-hash` covers the
whole template tree, every committed generated tree was re-pinned with
`scripts/regen_all_committed_trees.sh`; nothing but the hash line moved.

### What the gate measures, and one thing it does NOT

`ScriptLanguageSeam` is 11 cases now. Breaking the builder (one half into
both) and the dialect (`isArray` always ECMAScript) turns **2** red:
`ComposingKeepsTheEvaluatedAndAuthoredHalvesApart` and
`EachComposedQuestionMeansTheSameInBothLanguages` — the latter asks each
composed question of BOTH engines in each engine's spelling and compares the
answers, so a Lua spelling that does not exist, or exists and means something
else, disagrees with the ECMAScript one.

⚠ `ForeachCarriesATaggedArrayExpressionEndToEnd` stays **green** under that
same `isArray` break, and the case says so in its own docstring rather than
implying otherwise. The probe sits behind a short-circuit a well-formed array
never reaches — `arrayResult.isArray()` is already true for a Lua sequence,
and for a keyed table an honest `false` and a Lua syntax error are the same
outcome. The probe's language-correctness is observable only where it is asked
directly. That is a limit of the helper's shape, not of the gate, and it is
recorded here so nobody re-derives it as a gap.

## Landed 2026-08-28: the target engine is a codegen input

`sce-codegen generate --script-engine lua|ecmascript` (and the same on
`check`, because the flag is verdict-bearing). Omit it and every backend emits
for the engine it already emitted for, so a run that does not ask is
byte-identical to one from before the flag existed.

`ScriptEngineTarget` is the type; its spellings ARE the manifest's
`script_engine_language` vocabulary, so the value a host reads and the value
the run selected cannot become two names for one answer. The manifest now
reports the **selection**, falling back to the backend's derived default —
reporting the default over a selection would tell a host to supply the engine
the machine was *not* generated for, which is the mis-supply that field exists
to prevent.

**The pair is emitted by one filter or not at all.**
`to_script_source_expr` / `_guard` / `_script` emit a complete
`::SCE::ScriptSource::ecmascript("…")` or `::SCE::ScriptSource::lua("…", "…")`.
A template site that assembled the two arguments by hand would be one edit
away from lowering one half and not the other, and every diagnostic the
artifact raised would then name a language the author never wrote. The filters
close over the run's selection rather than reading it from the render context:
a template that could ask is a template that could ask *inconsistently*.
The lowering half is fallible and its refusal propagates — falling back to the
author's text on a parse error would emit an artifact half in each language
and say nothing.

### The refusal, and why it is not a lint

A backend that cannot honour the request refuses. Both directions, and each
names its own reason, because naming the wrong one sends a reader to the wrong
repair:

- **Rust / Go / Python / C11 asked for `ecmascript`** — no template arm emits
  the author's source. Nothing walks that way yet; building one is a template
  change, not a flag.
- **C++ asked for `lua`** — refused *while any site remains*, and the refusal
  lists them. This is the case with teeth. This document already states the
  hazard: splitting a subset is not a smaller version of the change, because
  the engine would receive Lua from some sites and ECMAScript from others **in
  one session**, with no diagnostic anywhere saying so. So a half-migrated
  backend must refuse, and the refusal is derived from a count taken off the
  template tree — it lifts by itself when the last site moves, the way the
  mesh-rpc refusal lifts when `templates/mesh/<lang>/` appears.

### 29 sites, and why that is not 38

`Language::unmigrated_expression_sites()` counts, and it counts **by model
field**, not by call shape — the lesson the 38-site count paid for twice. A
filter-keyed scan misses `{{ action.array }}`, passed with no filter at all; an
entry-point-keyed scan misses a helper nobody thought to list. The author's
text can only reach a template through one of these fields, so asking which
interpolations mention one catches both shapes.

Measured 2026-08-28: **29** distinct `(template, interpolation)` pairs. That is
not in conflict with this document's 38 — 38 counts CALL SITES, and one
interpolation serves several. 29 is the number of template edits the migration
actually needs.

### The count only means something once the roles are split

The 29 counted three different things wearing one spelling.
`{{ param.expr | escape_cpp }}` is the same eleven characters whether it is
handed to an engine, printed in a log line, or checked for emptiness — and
counting all three made the number **unable to reach zero**, which would have
left `--script-engine lua` permanently unofferable. A termination condition
that cannot be met is not a strict gate; it is a broken one.

`classify_expression_site` now splits them, and the split is keyed on the
CALLEE, not on the quoting:

- **EngineBound** — the line calls something in `ENGINE_ENTRY_POINTS`. The
  migration's population.
- **Message** — the interpolation sits inside a C++ string literal carrying
  other text (`SCE_LOG_ERROR("failed: {{ x }}")`). The author's own text is
  exactly what belongs there; nothing to migrate.
- **NotAHandOff** — the callee does not evaluate. Measured:
  `ForeachValidator::validateForeachAttributes` takes the array text and only
  asks whether it is empty (`ForeachValidator.h:27`), so lowering it would
  answer a question nobody asked.

⚠ **And the third bucket needed its own escape hatch.** An unknown callee also
reads as `NotAHandOff` — correct for the validator, a silent hole anywhere
else, and precisely how this document's own table came to miss five helpers.
So `unclassified_expression_sites()` lists them instead. Measured 2026-08-28:
**0 unmigrated, 9 unclassified.**

⚠⚠ **The Lua target requires BOTH lists empty.** Keying the refusal on the
migrated count alone would have opened it the moment `unmigrated` hit 0 —
with 9 sites unadjudicated, any one of which might evaluate. The escape hatch
would have defeated the gate that owns it. The gate case had the same bug and
now asks the same two questions.

### Adjudicated 2026-08-28: both lists are empty, and the target opens

`--script-engine lua -l cpp` now **generates**. Measured in the turn that
claimed it, on `examples/ai_loop/ai_loop.scxml`: manifest
`"script_engine_language":"lua"`, and **61** `ScriptSource::lua(...)` calls in
the artifact whose two halves are genuinely different text —

```
ScriptSource::lua("_scxml_add(restarts, 1)",        "restarts + 1")
ScriptSource::lua("_scxml_truthy(_event.data.done)", "_event.data.done")
ScriptSource::lua("(_scxml_add(turns, 1) >= max_turns)", "turns + 1 >= max_turns")
```

— the evaluated half lowered by the build-time frontend, the authored half the
author's own ECMAScript, in one call, from one filter.

Getting there needed the counting fixed **three more times**, each found by
adjudicating a site rather than by reading the number:

1. ⚠ **`.content` was not in the field list**, so the `<script>` BODY — the
   most obvious hand-off in SCXML — was invisible. The scan said 0 while
   `actions/script.jinja2` still passed a bare literal to `executeScript`.
2. **A multi-line call keeps its callee on an earlier line.** This document
   already records paying for that shape once ("33 rather than 38"); the scan
   repeated it, and `executeForeachWithActions(` two lines above its
   `{{ action.array }}` read as reaching nobody. Fixed with a 3-line window —
   which the INERT list needed too, for `emitContentLiteral(`.
3. ⚠⚠ **A template that spells `ScriptSource::ecmascript(...)` inline is an
   UNMIGRATED hand-off, not a finished one.** The donedata param site did
   exactly that: under the Lua target it would have emitted ECMAScript while
   everything around it emitted Lua — the mixed artifact, arriving through a
   constructor instead of a string, and reading as inert because the callee on
   its line *was* the constructor.

**Inert is adjudicated, not assumed.** `INERT_DESTINATIONS` is the mirror of
`ENGINE_ENTRY_POINTS`: a site is inert because its DESTINATION is known not to
evaluate — `validateForeachAttributes` (emptiness only, `ForeachValidator.h:27`),
`emitContentLiteral` (the template's own comment: "no evaluation, no script
engine"), `isValidLocation`, `getVariable` (a NAME lookup — the §scxml-6.2
question this document carved out), and the host-request fields that travel out
of the machine. Keyed on the destination rather than on a list of sites,
because an allowlist of sites is a hole one line wide.

And `Inert` is kept distinct from `Unadjudicated`. Collapsing them makes "we
checked" indistinguishable from "we have not looked" — which is precisely how
the `<script>` body went missing for a round.

### The 9 that were adjudicated (historical)

Each needs one decision: the callee evaluates (add it to `ENGINE_ENTRY_POINTS`
and route the site through `to_script_source_*`) or it does not (say so where
it is used).

| site | what it appears to be |
|---|---|
| `actions/assign.jinja2: {{ action.expr.rstrip(';') }}` | assigned into a `std::string expr` local, then passed to `executeAssignment` — an engine hand-off wearing a variable |
| `actions/assign.jinja2: {{ action.location }}` | the assignment target; crosses as executable text in the complex-path arm |
| `actions/foreach.jinja2: {{ action.array }}` | `validateForeachAttributes` — inert, and the `executeForeachWith*` calls beside it are the real hand-off |
| `actions/log.jinja2: {{ action.expr \| replace("'", '"') }}` | interpolated as a C++ EXPRESSION, not a string — needs reading before touching |
| `actions/send.jinja2: {{ action.delayexpr }}` | `getVariable` — a NAME lookup, and §scxml-6.2's own conformance question this document already carved out |
| `actions/send.jinja2: {{ action.eventexpr }}` | same |
| `entry_exit_actions.jinja2: {{ (param.location if param.location else param.expr) \| escape_cpp }}` | donedata params — already `ScriptSource` on the C++ side, needs the ternary rendered through the filter |
| `entry_exit_actions.jinja2: {{ invoke_info.srcexpr }}` | invoke src path |
| `invoke_methods.jinja2: {{ param.expr \| escape_cpp }}` | remaining occurrence in a `<param>` path |

**Migration: 29 → 23 → 0 engine-bound** (same day). Moved so far, all of them
guards or `<data expr>` — the two shapes the seam document names as
representative, and guards first because §scxml-5.9 truthiness is where the
runtime rewriters' ECMA-262 divergences concentrate:

| template | what moved |
|---|---|
| `process_transition.jinja2` | 3 guard sites → `to_script_source_guard` |
| `actions/if.jinja2` | `<if>` and `<elseif>` guards |
| `datamodel_macros.jinja2` | `<data expr>` → `initializeVariableFromExpr` |
| `scriptengine_helpers.jinja2` | `<data expr>` ×2, plus the composed `id = expr` |

`utility_methods.jinja2`'s `safeEvaluateGuard` moved with them — it is itself
template-emitted, so the receiving signature is part of the same edit, and its
three log lines now name `guardExpr.source()`.

The composed one needed a filter of its own: `to_script_source_assignment(location)`
builds `<name> = <expr>` with the evaluated half taking the LOWERED expression
and the authored half the author's. Spelling that at the site would have been
two interpolations glued together, one edit away from lowering one and not the
other — the hand-assembly the pair filter exists to prevent.

Then the rest of the engine-bound population: `<send>`'s `typeexpr`,
`targetexpr`, `contentexpr`, `delayexpr` and `<param>` expressions,
`<cancel>`'s `sendidexpr`, `<log>`'s `expr`, `<invoke>`'s `srcexpr` /
`contentexpr` / `<param>`, and the three `resultToString(…, originalExpression)`
sites this document had already counted. Nine templates now call the filter.

Verified in the same turn: the emitted artifact for `examples/ai_loop/ai_loop.scxml`
carries `::SCE::ScriptSource::ecmascript(...)` calls where it previously
carried bare string literals, the full C++ tree builds, every lane passes, and
`tests/forge/expected/inline_mixed_sm.inl` — the one committed C++ expectation —
re-pinned to exactly that change and nothing else.

Two corrections the scan needed, both measured rather than assumed:

- **Word boundaries.** `trans.cond_cpp` is the *natively lowered* guard, a C++
  expression the engine never sees. A substring test for `.cond` reported it as
  an unmigrated script site and would have held the refusal shut forever over
  something that was never on the wrong side of the seam.
- **`forge/` is out of scope.** C++ owns everything no other backend claims, and
  that complement includes `forge/` — which has its own per-language
  directories and whose `expr` / `cond` are Forge AST fields emitted as native
  code (`forge/cpp/procedure.h.jinja2` renders `if ({{ tr.cond }})` and names no
  engine entry point at all). Ownership and "does this reach an engine" are
  different questions.

`sce-build/tests/script_engine_target_input.rs` holds all of it (9 cases).
Breaking the pair filter (one half into both) and the migration scan (read
nothing) turns **3** red.

⚠ The scan case needed a second pass to be worth anything. Its first cut asked
`supports_script_engine_target` in the empty-scan branch — which reads the SAME
scan, so breaking the scanner made both answers say "migration complete" and
left the case green. It now asks the independent question
(`migrated_expression_sites()` must be non-empty), and *that* catches it. A
gate whose two halves share a source is not a gate.

## Landed 2026-08-29 (third round): a Lua tree emits Lua, and what that does NOT do

`sce_add_state_machine` derives the artifact's target language when the caller
names none: a `-DSCE_SCRIPT_ENGINE=lua` tree gets `--script-engine lua`. An
artifact built in a tree can only run on the engine that tree compiled in, so
the language it hands that engine should be the one the engine speaks, whether
or not the CMake call says so.

Scoped to `LANGUAGE cpp`, and that scoping is the substance rather than caution:
`SCE_SCRIPT_ENGINE` is a cache option on the C++ runtime library whose
definition is PUBLIC on `sce_scripting`, so it describes the C++ tree and
nothing else. A Rust, Go, Python or Kotlin artifact generated in the same tree
runs against whatever engine ITS host supplies. Deriving for them would also
hand `--script-engine lua` to a backend that may refuse it
(`supports_script_engine_target`), turning an unrelated cache value into a build
failure. `quickjs` stays UNSET rather than being spelled `ecmascript`, so a
non-Lua tree emits what it emitted before this existed.

**It is measured on the artifact, not asserted in a comment.** The lowered gate
now builds the same document a THIRD time from a call that names nothing —
`ecma262_default` — and two independent readings hold it: the gate counts its
`ScriptSource::lua(...)` pairs from outside, and the suite
(`ATreeThatSelectedLuaEmitsLuaWithoutBeingAsked`) asks whether it answers
ECMA-262 case for case like the explicitly-lowered one. Two artifacts can agree
by both being wrong, so "it was emitted lowered" and "it answers like the
lowered one" are kept as different questions.

Measured in the turn that landed it: **785 pairs asked-for-lua, 785 asked-for-
nothing, 0 in the control.** Red witness, same turn: disabling the derivation
makes the gate fail with `the artifact generated with NO SCRIPT_ENGINE_LANGUAGE
carries 0 ScriptSource::lua(...) call(s) against the explicitly-lowered one's
785`. The control's explicit `SCRIPT_ENGINE_LANGUAGE ecmascript` became
LOAD-BEARING with this change — omit it now and the control becomes a second
copy of the subject.

### ⚠ The prescription this round carried was wrong, and the measurement says so

The previous round's ledger said: make the flag the default so generated C++
stops reaching the rewriter, **and then the 23 entries start leaving
`lua_engine_divergences.json`.** Re-measured before implementing, the second
half is false. The two suites holding the `runtime-rewriter` column reach the
engine by routes no codegen default can touch:

- `tests/engine/EcmaScriptSemanticsTest.cpp` carries **zero** `Generated::`
  references — it calls `SCE::LuaEngine::instance()` directly, so it has no
  generated machine to change;
- `LoweredEcma262`'s control names `SCRIPT_ENGINE_LANGUAGE ecmascript`
  explicitly (`tests/CMakeLists.txt`), which is what keeps it a control.

So this step moves **zero rows**. Re-derive rather than trusting the paragraph:
`grep -c "Generated::" tests/engine/EcmaScriptSemanticsTest.cpp` and
`grep -n SCRIPT_ENGINE_LANGUAGE tests/CMakeLists.txt`.

**The order is three steps, not one**, and naming it as one is what made the
count look reachable:

1. generated C++ in a Lua tree stops handing the engine ECMAScript — **this
   round**, 0 rows;
2. the C++ **Interpreter** gets a runtime lowering route, or the `lua`
   selection is declared AOT-only. It has no build step, so it reaches the
   adapter no matter what codegen emits;
3. `EcmaScriptToLuaTransformer` is **retired** — `LuaEngine::acceptsLanguage`
   stops admitting ECMAScript. Then `EcmaScriptSemanticsOnLuaEngine` has no
   subject and the 23 rows go with it.

Step 2 is the one with an unmeasured cost. The route this document already
names is `sce-build`'s cdylib, and it is still not built: measured this round,
`sce-build/Cargo.toml` declares `crate-type = ["rlib"]` only, `sce/src` links no
Rust library, and the only C ABI in the crate is Forge's `extern_emit`. So the
frontend that answers all 98 cases correctly has no way to be called at run
time, which is the whole of step 2.

⚠ And a residue this round could not close: the gates the lowered lane added
are **not reachable by the mutation corpus**. `lua_engine_reads_ecmascript.cases`
drives `ctest --test-dir build` — the developer's quickjs tree — where
`LoweredEcma262` does not exist. Its witnesses are made by hand each round. The
casefile's header used to say "no gate configures it", which stopped being true
on 2026-08-29; it now says which suite it cannot reach and why.

## Landed 2026-08-29 (second round): the lane became a RATCHET

The section below records the gate that first compiled and ran a lowered
artifact. It measured; it did not ratchet, and re-reading it against the tree
turned up three defects that a green run could not have shown. Each is written
here with what it cost, because each is a shape this repository keeps paying
for.

### 1. The gate could not be green, and then could not be green on the other machine

`scripts/gate ecma262-lowered-cpp` ended by asserting a line of ctest's own
summary. CI failed it over a log whose summary said every test passed:
CI's ctest prints `100% tests passed, 0 tests failed out of 2`, the gate looked
for `100% tests passed out of 2`. Correcting the regex to CI's spelling then
made it red on the BUILD MACHINE, whose ctest prints the form without the comma.
Measured both ways on 2026-08-29 — the same artifact, the same verdict inside
the suite, opposite gate verdicts on two machines.

**The defect is not the regex, it is asking a tool's human-readable summary a
machine question.** The gate now reads `ctest --output-junit` and asks the XML:
a `testcase` named `LoweredEcma262` must exist, and no case may carry
`failure`, `error` or `skipped`. `skipped` is in that list because it is the one
hole ctest's exit status leaves — a skipped case keeps the status at 0 while
asking nothing. Re-derive the predicate rather than trusting this paragraph:
feed it a report with the case passing, with it skipped, with it absent, and
with its fixture failed; the four verdicts are 0, 1, 1, 1.

### 2. The population excluded the role that could reach zero

The suite asked the 23 entries of `lua_engine_divergences.json`, because the
fixture was expanded FROM that list. So:

- The shared table's **other 75 cases were never asked through a lowered
  artifact at all.** Build-time lowering could answer any of them wrongly with
  nothing in the tree to say so. A path's divergences cannot be enumerated by a
  list built from a DIFFERENT path's failures — which is what that list is.
- **Deleting an entry deleted its own question.** A list only ratchets while
  something keeps asking what the list no longer claims.

The generator's own refusal already prescribed the repair, in the branch it
takes when the list empties: *"Retire the gate or repoint it at the shared table
in full."* It is repointed. The population is `ecma262_semantics.json`, all of
it, and the divergence list is read by the HARNESS as the expectation about
which cases lowering gets wrong. The two halves of the gate no longer share a
source.

Repointing meant expressing two cases the generator had been REFUSING, and
neither could be waved through — a skipped case reads as a passing one:

- **`o.missing`, whose ECMA-262 answer is `undefined`.** An engine's JSON
  encoding omits such a key, so "the answer is undefined" and "this case was
  never asked" arrived as the same absence. The fixture now writes `answers.rN`
  as the first action of every case and parks the sentinel in the answer slot
  BEFORE the setup, which makes four readings out of one absence: not reached /
  refused / evaluated-to-undefined / a value. The sentinel goes down before the
  setup rather than after it so a setup that raises lands on *refused* instead
  of being reported as the answer `undefined`.
- **`++v == 2`, a condition with a side effect.** The refusal probe evaluated
  the expression once in the same state as the guards, so the guards saw a
  datamodel the probe had already moved: probing leaves `v` at 2, the positive
  guard makes it 3 and answers false, the negation makes it 4 and answers true —
  the fixture would have recorded FALSE for a case whose answer is true, with
  nothing wrong with the engine. The probe now has its own state and the case
  re-runs its setup. That retires the refusal by construction rather than by
  exempting the shared table's `side-effecting` group, which would have been an
  exemption list quietly excusing the three cases that exercise evaluation
  order.

### 2b. Found by the wider population: the fixture was asking a DERIVED expression

The `cond=` protocol asked the guard, its NEGATION, and an unguarded
fallthrough, and read the fallthrough as *"neither held, which only an
expression the engine refused can produce."* That claim is false, and widening
the population is what surfaced it: the control came back wrong on three cases
the list does not name — §7.1.2 ToBoolean of `0`, `''` and `NaN`, each asked as
`cond="a"`.

What actually happened is that `cond="a"` with `var a = 0` answers false
**correctly** under the runtime rewriter, and `cond="!(a)"` answers false too,
because the rewriter hands Lua `not a` and Lua counts `0` as true. Both guards
false, fallthrough taken, and the harness reported an engine refusal that had
not happened — while the probe on the same run said the expression evaluated
fine.

**The defect is asking an expression the author did not write.** `!a` is a
divergence in its own right (§12.5.9) and the shared table already asks it as a
case; folding it into every other condition case's protocol made ONE entry's
divergence surface as three other entries' refusals. Had those three been
"declared" to make the gate green, the list would have recorded a property of
the harness as a property of the engine.

The fixture now asks only the author's expression — guard, or fallthrough — and
"could the engine evaluate this at all" is the PROBE's question, which is what
the probe was already for. `agrees` refuses to read a condition verdict unless
the probe says the expression evaluated, so a lowered artifact emitting
unparseable Lua still cannot pass a case whose expected answer is false. The
probe is shown to distinguish both outcomes on the same run before its word is
used.

⚠ This also retired `MIN_SOURCE_DIVERGENCES` — a floor ("at least 22 of the
declared entries must still come back") with an attribution test beside it for
the entries the floor let through. The hole was exactly the shape of the
allowance: a case the control got wrong while naming NO entry was counted by
neither, which is how three of them sat unremarked. The control is now an
EQUALITY in both directions, the same ratchet the lowered side has, so the
`runtime-rewriter` column can shrink through this lane too.

### 3. The list had no reachable end state, and now it does

Every entry said `needs` — a CAUSE — and nothing said WHERE it was still wrong.
With one unmarked population, an entry could only leave the file when the
runtime rewriter was repaired, and the plan of record is to RETIRE that rewriter
for generated code rather than repair it. **So the count this axis exists to
drive to zero had no path to zero at all.**

Each entry now carries `diverges_on`, naming the routes into the Lua engine that
still answer it differently, and the file declares the `paths` an entry may
name:

| path | who reaches it | whose contract |
|---|---|---|
| `runtime-rewriter` | the artifact hands the engine the author's ECMAScript | `ecmascript_semantics_test` (the engine, direct) and `LoweredEcma262`'s CONTROL (the same engine, reached the way a document reaches it) |
| `build-time-lowering` | `sce-build`'s frontend emitted Lua already | `LoweredEcma262` |

An entry stays while ANY path answers it differently and LEAVES when
`diverges_on` would be empty. Both suites now filter to their own path, so
neither reports the other's divergence as one of its own that has been repaired.

**`paths` is derived, not typed.** `ecma262_scoreboard_contract` computes it
from the same answers the code generator gives — `runtime-rewriter` while
`default_script_engine_target()` is `EcmaScript` (the backend hands the engine
the author's text, so a Lua engine must adapt it), and `build-time-lowering`
once `supports_script_engine_target(Lua)` (the backend can emit a lowered
artifact) — and holds each file's key to it. C++ gets both; Kotlin gets one, and
the day the Kotlin templates cross the seam its list goes red asking which path
each of its 46 entries is about, instead of keeping 46 answers that quietly
became ambiguous.

**Unclassified is RED, not a default.** All three readers refuse an entry with
no `diverges_on` rather than assuming the runtime path. Defaulting would make
every future entry silently exempt from the OTHER suite — the escape hatch
defeating the gate that owns it, which is the same failure the Lua codegen
target already refuses on (it stays shut while any site is *unadjudicated*, not
merely while one is known-unmigrated).

**The scoreboard follows the split.** `ARCHITECTURE.md`'s Lua row is the table
minus the entries declared on `runtime-rewriter` — the route that row's consumer
takes, since C++ codegen hands over the author's text unless the run asked for
`--script-engine lua`. The number is unchanged today; the derivation is now the
one the cell means.

### What the ratchet is, in one sentence

`LoweredEcma262` holds BOTH paths in BOTH directions — the lowered artifact for
`build-time-lowering`, its un-lowered control for `runtime-rewriter`. A case an
artifact gets wrong without being declared is red, **and a case declared and
answered correctly is red**. The second direction is what lets the file shrink,
and it is the one the gate did not have.

### Measured in the turn that landed it

Re-derive rather than trusting these numbers — the gate prints its own census on
every green run, which is why `-V` is in the ctest line: a number that exists
only in a failure message is one nobody can cite from a green build.

```
LoweredEcma262 census: population=98 lowered-wrong=0 source-wrong=23 \
  declared-build-time-lowering=0 declared-runtime-rewriter=23
```

- the lowered artifact carries **785** `ScriptSource::lua(...)` pairs against the
  control's **0** — the 170 the section below reports was the 23-case fixture;
- **build-time lowering answers all 98 cases**, so `build-time-lowering` is
  declared on none of them. That column is a real, measured zero rather than an
  unasked one, which is exactly what the old population could not have said;
- the control gets **23** wrong and **23** are declared on `runtime-rewriter` —
  the equality, not a floor with an allowance under it.

⚠ Both halves of that line moved during the round and neither number was
predicted correctly. The first census read `source-wrong=25` against 23
declared, and reading it is what found the derived-guard defect (§2b); the count
of pairs was 842 under the three-guard protocol and is 785 without it. Both are
reasons the gate now prints its census on a GREEN run rather than only inside a
failure message.

**Two red witnesses, both made in the same turn, each from one break.**

1. **The stale marking — the direction that empties the file.** Adding
   `"build-time-lowering"` to one entry's `diverges_on` (`!a`, §12.5.9) turns
   **exactly one** of the six cases red,
   `TheLoweredArtifactDivergesExactlyWhereItIsDeclaredTo`, naming the entry:

   ```
   1 case(s) are declared to diverge on `build-time-lowering` and the lowered
   artifact answers them CORRECTLY.
     [!a] (12.5.9 logical NOT) — answered 1 (the guard held), and the engine
     evaluated the expression, which IS what ECMA-262 says
   ```

   and telling the reader to drop the path, and to delete the entry if
   `diverges_on` is then empty. The other five stayed green, so the attribution
   is the entry and not the suite.

2. **The population really grew.** Breaking the frontend's `typeof` lowering
   (`_typeof(x)` → Lua's `type(x)`, `sce-build/src/ecmascript/lua.rs:153`) turns
   the same case red from the other side, on three entries — and the middle one
   is the witness that matters:

   ```
   [typeof missingVariable !== 'undefined'] … [declared, but only on: runtime-rewriter]
   [typeof a] (13.5.3 typeof of an Array is 'object') — answered "table",
       ECMA-262 says {"string":"object"}   [not in the divergence list at all]
   [typeof JSON.parse('{"a":1}')] …        [declared, but only on: runtime-rewriter]
   ```

   `typeof a` is a case the OLD population never asked, so the old gate could
   not have seen this break at all. The two beside it are the per-path
   attribution working: entries declared on the runtime path only, now failing
   on the build-time one, and named as such rather than counted.
   `LoweringLosesNoAnswerTheRuntimeRewriterAlreadyHad` went red on the same run,
   which is correct — the control answers those and lowering stopped doing so.

## Landed 2026-08-29: a Lua-lowered C++ artifact is compiled and RUN

`scripts/gate ecma262-lowered-cpp` (`.github/workflows/ecma262-lowered-cpp.yml`,
`tests/engine/LoweredEcma262Test.cpp`, ctest `LoweredEcma262`). It configures a
throwaway tree with `-DSCE_SCRIPT_ENGINE=lua`, builds one C++ artifact generated
with `--script-engine lua`, and runs it. It is the only gate in the registry
that configures that selection, which is what the three comments in
`tests/CMakeLists.txt` said from the other side and no longer do.

**Which side of the seam it measures: BUILD-TIME lowering.** Stated on the gate,
in the workflow and in the harness, because the file it measures against —
`lua_engine_divergences.json` — is the RUN-TIME rewriter's list, and the two are
different code paths into the same engine.

**Why that file is nevertheless the population.** Every entry names an
expression the rewriter answers differently from ECMA-262, and a lowered
artifact hands its engine Lua, so the rewriter is never reached and each entry
has to answer the language. `tools/generate_lowered_ecma262_fixture.py` expands
each entry into one state of an SCXML document, joined to
`ecma262_semantics.json` for the `setup`/`form`/`expect` — so the population is
that file and nothing else, an entry added there grows a case on the same
commit, and an entry removed takes its case with it. The generator REFUSES an
entry it cannot express rather than skipping it, and refuses an empty list
rather than emitting a fixture that asks nothing.

**The expectations are not generated.** The harness reads both committed tables
at run time and recomputes the join itself. A harness reading the generator's
own idea of the population would be the shape this document already names: *"A
gate whose two halves share a source is not a gate."*

**Two artifacts from one document, which is what makes the green mean
something.** `ecma262_lowered` (with the flag) must answer every entry;
`ecma262_source` (`--script-engine ecmascript`, what C++ has always emitted)
must not. Measured in the turn that landed it, on the build machine:

- lowered artifact: **170** `ScriptSource::lua(...)` pairs; control: **0**
  (asserted by the gate, so a target that quietly stopped lowering fails with
  the cause named rather than as a wrong answer);
- `LoweredEcma262` green — **23 of 23** declared entries answered;
- red witness, same turn: dropping `SCRIPT_ENGINE_LANGUAGE lua` from the
  lowered target makes the gate refuse on the pair count, and running the suite
  past that guard reports **22 of the 23** by name, each citing its clause
  (`[5 ^ 3] as 125 … says {"number":6}`, `[-8 >> 1] as 9223372036854775804 …`,
  `[a + ''] as "table: 0x…" …`).

### The 23rd, and why the control was first written as a floor

`typeof missingVariable !== 'undefined'` (d14) comes out CORRECT through the
source-passing artifact. Not because the rewriter gained it: §scxml-5.9.1 makes
a `cond` the engine refused evaluate to FALSE, and false is this entry's
ECMA-262 answer, so the refusal and the language coincide. Confirmed on the same
tree — `ecmascript_semantics_test` reports it as *"failed to evaluate as a
condition"*, not as a wrong answer.

That is why the control was first written as a FLOOR — `MIN_SOURCE_DIVERGENCES
= 22`, "at least 22 of the declared entries must still come back" — with an
attribution test beside it for the entry the floor let through.

⚠ **Both of those names are gone, and this paragraph went on citing them for a
day.** Re-derived on 2026-08-29: `grep -rn MIN_SOURCE_DIVERGENCES` finds only
this file and one comment recording the retirement, and
`EveryEntryTheSourceArtifactGetsRightIsAnEngineRefusal` is not a test in the
tree at all. What replaced them is
`LoweredEcma262.TheSourcePassingArtifactDivergesExactlyWhereItIsDeclaredTo`, an
EQUALITY in both directions — the same ratchet the lowered side has, and the
reason the `runtime-rewriter` column can shrink through this lane too. A floor
with an allowance had a hole exactly the shape of the allowance; an equality has
nowhere to hide. The account of it is §"2b. Found by the wider population"
above. The probe that answers "refused?" is measured, and it has its own control
on the same run.

⚠⚠ **That control used to COUNT the table, and a control whose zero is
forbidden forbids the finish line.** Its first form swept the shared table's own
condition cases and required at least one refusal and at least one evaluation
among them. On 2026-08-29 the refusal count reached **zero** — not through a
defect but through the repair this whole axis is for: `LuaEngine::loweredTextOf`
began offering every closed expression to the frontend parser, so the engine
stopped refusing anything in the table. The control failed, and it failed on the
run that had made it obsolete. The population a control counts is not the thing
the control is about: the question is whether the PROBE distinguishes, and the
table is only where the outcomes used to come from by accident.

So both outcomes are now produced on purpose, by two states the fixture opens
with, ahead of the population and independent of it — `answers.missing.deep`,
which raises in ECMA-262 and in Lua alike because a member OF an absent object
cannot be read, and the literal `1`, which nothing can refuse. Both go through
the bare-name probe, so both ask the entry point a `cond=` asks.

⚠ **The readings are census fields, because an assertion that is deleted leaves
no trace and a census field does.** `assertProbeDistinguishes` is called from
the two ratchet tests; delete both calls and every test in the file still
passes, over a probe nothing controls. `scripts/gate ecma262-lowered-cpp` reads
`lowered-control-refused` / `lowered-control-evaluable` and the `source-`
pair off the census line and fails when any is missing or reads the wrong way,
so the control cannot leave quietly. Re-derive from a green run rather than
from this paragraph:

```sh
scripts/gate ecma262-lowered-cpp 2>&1 | sed -n 's/.*\(LoweredEcma262 census: .*\)/\1/p'
```

⚠ **"Delete both calls" is the SHAPE of the hole, not a runnable mutation, and
the difference cost a red witness.** `assertProbeDistinguishes` lives in this
file's anonymous namespace and the tree builds `-Werror`, so removing its two
callers makes it an unused function and the target does not compile — a
mutation the compiler refuses proves nothing. The witness that does run puts
the suite in the same state by a different route: neuter its two `EXPECT`s
(`EXPECT_EQ(reading(CONTROL_REFUSED), reading(CONTROL_REFUSED))`) and break the
probe in the fixture (`CONTROL_REFUSED_EXPR = "1"`). Measured 2026-08-29 on the
build machine, that pair reports `[  PASSED  ] 7 tests` and `100% tests passed
out of 2` while the gate exits 1 on *"the lowered artifact's refusal control
read '1', not the unevaluated sentinel"* — the suite green and the gate red on
one run, which is the whole claim this census field exists to make.

⚠ **The probe has to use the guard's own entry point.** Its first cut assigned
the expression to `answers.vN` — a dotted location, which §scxml-5.4 routes
through `executeScript`, which does NOT run `LuaEngine`'s undeclared-variable
`ReferenceError` check. It therefore reported d14 as evaluable and the
attribution came out backwards. A bare location routes through
`evaluateExpression`, which is what a `cond=` reaches. The two `<assign>` routes
are different engine entry points with different semantics, and which one a site
takes is decided by whether its location has a dot in it.

### What the gate does NOT do

It does not empty `lua_engine_divergences.json`. That file is the runtime
rewriter's measurement and `ecmascript_semantics_test` holds the engine to it in
both directions; the rewriter still diverges on all 23 and the entries are
correct. What this gate establishes is that the OTHER path answers them — so the
emptying comes from retiring the rewriter for generated code, and the residue is
the C++ **Interpreter**, which has no build step and reaches the adapter no
matter what codegen emits (see below).

### The entries it catches

Derived, not typed — the gate's population is the file. Re-derive with the join
the generator and the harness both make, rather than trusting this table:

```sh
python3 tools/generate_lowered_ecma262_fixture.py \
  --cases tests/ecmascript/ecma262_semantics.json \
  --divergences tests/ecmascript/lua_engine_divergences.json \
  -o /tmp/fixture.scxml   # prints the case count; refuses on an entry it cannot express
```

| key | source | form | clause | needs |
|---|---|---|---|---|
| `d0` | `!a` | condition | 12.5.9 logical NOT | operand-boundaries |
| `d1` | `a && b` | condition | 13.13.1 the AND yields its left operand when it is falsy | operand-boundaries |
| `d2` | `a \|\| b` | condition | 13.13.1 the OR yields its right operand when the left is falsy | operand-boundaries |
| `d3` | `a && b` | value | 13.13.1 the value is an operand, not a boolean | operand-boundaries |
| `d4` | `a \|\| b` | value | 13.13.1 the value is an operand, not a boolean | operand-boundaries |
| `d5` | `a ? 'yes' : 'no'` | value | 13.14 conditional uses ToBoolean | operand-boundaries |
| `d6` | `true ? a : 'other'` | value | 13.14 a false consequent is still the result | operand-boundaries |
| `d7` | `a + ''` | value | 7.1.1 ToPrimitive of an Array joins with commas | operand-boundaries |
| `d8` | `1 == '1'` | condition | 7.2.15 number vs string coerces | operand-boundaries |
| `d9` | `0 == false` | condition | 7.2.15 boolean is ToNumber'd | operand-boundaries |
| `d10` | `'' == 0` | condition | 7.2.15 empty string is ToNumber 0 | operand-boundaries |
| `d11` | `1 != '1'` | condition | 7.2.15 negation of the above | operand-boundaries |
| `d12` | `a == null` | condition | 7.2.15 null and undefined equal each other | operand-boundaries |
| `d13` | `-7 % 3` | value | 13.7 remainder truncates | operand-boundaries |
| `d14` | `typeof missingVariable !== 'undefined'` | condition | 13.5.3 typeof of an unresolvable reference is 'undefined' | operand-boundaries |
| `d15` | `++v` | value | 13.4.4 the operand is ToNumber'd, so this is not concatenation | operand-boundaries |
| `d16` | `n` | value | 14.7.3 while, with continue skipping one iteration | statement-structure |
| `d17` | `5 ^ 3` | value | 13.12 bitwise XOR | operand-boundaries |
| `d18` | `-8 >> 1` | value | 13.9.2 signed right shift | operand-boundaries |
| `d19` | `-8 >>> 28` | value | 13.9.3 unsigned right shift | operand-boundaries |
| `d20` | `(a == 1) \| (b != 2)` | value | 13.12 booleans are ToInt32'd, so the result is a number | operand-boundaries |
| `d21` | `+'3'` | value | 13.5.4 unary plus is ToNumber | operand-boundaries |
| `d22` | `typeof JSON.parse('{"a":1}')` | value | 15.12.2 the result of parsing a JSON object is an object | chain-operand |

### Three defects the gate found before it could be green

None of them is about lowering. Each was in the way, and each is the kind of
thing only a document that actually runs can find.

1. **The §scxml-5.3 read accessors were unreachable.**
   `state_machine.jinja2` emits `<var>()` on the POLICY, and `policy_` is
   `protected` on `StaticExecutionEngine` — so every generated machine carried a
   read surface no host could call, and nothing in the tree called one. The
   machine class now forwards them. An accessor no caller can reach is not a
   read surface; it is dead code that reads as one.

2. **The author's text reached C++ string literals unescaped.**
   `typeof JSON.parse('{"a":1}')` closed the literal early —
   `error: expected ')' before 'a'`, reported against the SCXML line by the
   `#line` map — in `actions/assign.jinja2`'s log lines. The braces are a second
   half of the same defect and are worse: `SCE_LOG_*` expands to `fmt`, so
   `{"a":1}` is a replacement field, and in a build with logging compiled out
   the macro forwards to a variadic sink and nothing parses it. `escape_cpp` and
   the new `escape_cpp_format` (C++ escaping plus brace doubling) are now
   applied at every C++-owned site where a model expression lands inside a
   literal — `assign`, `foreach`, `send` (typeexpr/targetexpr/eventexpr/
   contentexpr/idlocation/delayexpr), `invoke_methods`, `entry_exit_actions`.
   ⚠ Residue, measured and NOT fixed here: the same text also lands in `//`
   comments (`process_transition.jinja2`, `actions/if.jinja2`) where a newline
   would break the line, in `R"(...)"` raw strings (`datamodel_macros.jinja2:29`,
   `scriptengine_helpers.jinja2:173`) where `)"` would close them early, and in
   `actions/log.jinja2:19` where `action.expr` is interpolated as a C++
   EXPRESSION rather than a string. Each needs a different escape, so each is
   its own edit. Re-derive with
   `grep -rnE '"[^"]*\{\{ *(action|invoke_info|param)\.[a-z_]*(expr|cond|array|location|content)' tools/codegen/templates/*.jinja2 tools/codegen/templates/actions/*.jinja2`.

3. **`<assign>` to a non-bare location evaluated its expression TWICE.**
   `AssignmentExecutionHelper::executeAssignment` evaluated before branching and
   the complex path then re-evaluated inside `<loc> = (<expr>);`, discarding the
   first result. Measured: `<assign location="answers.d15" expr="++v"/>` with
   `v` at `'1'` recorded **3** where ECMA-262 13.4.4 and §scxml-5.4 say 2. The
   evaluation now happens once, on whichever path is taken. It is the Single
   Source of Truth both engines share, so the Interpreter carried it too, and
   one fix answers for both — this is Zero Duplication paying out rather than
   costing. `scripts/gate w3c-cpp` re-verified: 404 cases pass.

### And one in the CMake surface the gate added

`sce_add_state_machine`'s parse prefix is `SCE`, so a keyword named
`SCRIPT_ENGINE` parses into `SCE_SCRIPT_ENGINE` — the tree's own engine-selection
CACHE option. `cmake_parse_arguments` unsets the variable for a keyword the
caller omitted, and unsetting a normal variable reveals the cache entry
underneath, so **an omitted argument silently became "whatever engine this tree
was configured with"**. Found while building the red witness: dropping the
argument left the artifact fully lowered anyway, 170 pairs, because the tree was
configured `-DSCE_SCRIPT_ENGINE=lua`. A gate cannot be shown to catch a lost
flag while the flag cannot be lost. The keyword is now
`SCRIPT_ENGINE_LANGUAGE` — the manifest's own field name — and the function
asserts that no cache entry shadows it.

## Landed 2026-08-29 (fourth round): the axis's judge could be cancelled, and the vendored surface had drifted

Two things this axis had produced without noticing, both found in CI rather
than in the tree.

### The C surface the seam added is a vendored API, and the manifest said 283

`embed/MANIFEST.json` is the API-surface record consumers vendoring `embed/`
diff before re-syncing. The lowering seam's C entry points
(`scripts/gate embed-vendor` names them) are public headers under
`embed/include/scripting/`, so adding them changed that surface — and the
manifest was not regenerated with them. `Embed Vendor Smoke` went red on
`9192789c88` and again on `e74dc3a001`, saying `symbol_count: 283 -> 292`.

Re-derived in the turn that repaid it, and this is the command rather than the
number:

```sh
scripts/package_embed.sh            # or scripts/emit_embed_manifest.sh
git diff --stat embed/MANIFEST.json
```

Nine symbols, no removals: `sce_lower_condition`, `sce_lower_free`,
`sce_lower_location`, `sce_lower_script`, `sce_lower_value`,
`sce_scope_declare`, `sce_scope_declare_chunk`, `sce_scope_free`,
`sce_scope_new` — exactly the C-callable lowering surface §"The price of a
C-callable lowering surface" measured. ⚠ The regeneration is only reproducible
against the clang the manifest records: `clang_version` is a manifest field,
and the emit script defaults to `clang++-19` for that reason.

### The lane that judges this axis was cut by every push

`ecma262-lowered-cpp.yml` ran `cancel-in-progress: true`, so a push arriving
while it was working killed it. That is the right setting for a lane that
answers in a minute and the wrong one for this one: measured 2026-08-29 it
needs a **22.6 min median** against a **18.9 min median gap between pushes to
`main`** (n=39 gaps), so it cannot finish between two of them. It was cancelled
on `9192789c88` and `1a1f1169f8`, both times by a mid-session push 17 and 14
minutes behind the run it took down.

⚠ **Its cancellation RATIO hides this** — 2 of its last 11 runs, which reads
like a short lane's rate. The narrow `paths:` filter is why: it decides which
pushes reach the lane, not what happens to the two that do. The **duration** is
the rule; the ratio is corroboration, and only for a lane with a wide filter.

⚠⚠ **A global run window says something else again.** Read from
`gh run list --limit 300`, every lane under twenty minutes showed ZERO
cancellations, which made "ever cancelled" look like a clean discriminator. Per
workflow — `gh run list --workflow=<file> --limit 25` — even a 0.4-minute lane
has been cancelled three times. A global window is dominated by whichever lanes
ran most recently and under-samples the rest. The per-workflow query is the one
the table is built from.

Three lanes were over the line and all three are now `cancel-in-progress: false`:
this one, `w3c-tests.yml` (28.3 min, 13 of 25 cancelled) and
`rust-workspace-tests.yml` (32.4 min, 11 of 25). `cpp-suite.yml` had already
gone that way on the same day and its comment had recorded the other two
lanes' ratios **in prose**, which is why they survived it.

So the rule is a predicate now, not a paragraph:
`sce-build/tests/ci_supersession_policy.rs` carries every workflow's measured
median, derives the required setting from it, and reds a workflow no row
measures — an unclassified lane is not a pass. It runs from `tree-hygiene.sh`,
whose workflow declares no `paths:` filter and therefore starts for every
workflow edit.

⚠ It does NOT ride `rust-workspace-tests.yml`'s filter, and the attempt is
worth recording: adding `.github/workflows/**` there made the hook's
`unfiltered-workflow-self` case fail. A gate inherits its workflow's `paths:`
as its hook triggers, `workspace-tests` is `ci_only`, and the hook is never
offered a `ci_only` gate — so the glob would have classified every workflow
path as known while selecting nothing to check it, taking away the full-run
fail-safe that editing an unfiltered lane is supposed to buy.

`false` and not a per-commit key, deliberately: a newly queued run still
cancels a PENDING one, so `false` saves the run in flight and the latest commit
and defers the ones between. That is correct for a lane whose answer is about a
BRANCH — these re-ask a fixed population against whatever the tree now holds.
`mutation-rounds.yml` keys on `github.sha` because its answer is about a
COMMIT: selection is by change set and nothing re-selects it.

## What is not yet decided

- ~~The six helper entry points.~~ **Done 2026-08-28** — and there were eight
  helpers, not three; see "Landed 2026-08-28: the helpers" below.
- ~~Whether the target engine becomes a codegen argument.~~ **Decided and
  landed 2026-08-28**: `--script-engine`, defaulting to each backend's derived
  answer. See "the target engine is a codegen input" below.
- ~~The 29 C++ template sites.~~ **Done 2026-08-28** — every engine-bound site
  routes through `to_script_source_*`, both scan lists are empty, and
  `--script-engine lua -l cpp` generates. See "Adjudicated 2026-08-28" below.
  ⚠ The failure mode was never a build error: a site left behind compiles and
  quietly keeps the old behaviour, which is why the count — not the compiler —
  is the gate.
- ~~**Nothing yet RUNS a Lua-lowered C++ artifact.**~~ **Done 2026-08-29** —
  `scripts/gate ecma262-lowered-cpp` configures `-DSCE_SCRIPT_ENGINE=lua`,
  compiles an artifact generated with `--script-engine lua`, runs it, and holds
  it to all 23 declared divergences with the un-lowered artifact beside it as
  the control. See "Landed 2026-08-29" above.
  ⚠ It does NOT empty `lua_engine_divergences.json`, and saying it would was
  the wrong reading of this bullet. That file measures the RUNTIME rewriter,
  which still diverges on all 23 and is correctly listed. What the gate
  establishes is that the other path answers them; the emptying is a
  consequence of the rewriter stopping being reached, which for generated code
  means the `--script-engine lua` selection becoming the default and for the
  Interpreter means the next bullet.
- ~~**The lane measures; it does not ratchet.**~~ **Done 2026-08-29 (second
  round)** — each entry names the PATHS it still diverges on, both suites
  filter to their own, and each path is held in BOTH directions, so an entry
  leaves the file when `diverges_on` would be empty. See "the lane became a
  RATCHET" above; the population is the shared table in full, so the count that
  reaches zero is a measured one rather than an unasked one.
  ⚠ Still open, and it is the same residue as the bullet below: the build-time
  column is measured at ZERO, so the file can now only shrink through the
  RUNTIME column — and that column closes when the rewriter stops being
  reached, not when it is repaired. Making `--script-engine lua` the default
  for the `lua` selection is the move that would do it, and it is a decision
  about what a C++ consumer gets rather than a repair.
- The C++ **Interpreter** has no build step at all, so it cannot receive
  build-time-translated text by this route. `sce-build`'s cdylib is the
  candidate answer — `Cargo.toml` records that there is no in-tree consumer of
  it today, and `sce-build-wasm` has already built that wrapping once.
  **Re-measured on the morning of 2026-08-29 and then overtaken the same day**:
  this was step 2 of the three the list needs to empty (see the third round
  above). At that reading `sce-build/Cargo.toml` declared `crate-type =
  ["rlib"]` only, nothing under `sce/` linked a Rust library, and the crate's
  only C ABI was Forge's `extern_emit` — the frontend answered all 98
  shared-table cases correctly and had no way to be called at run time. That is
  what closing it was for; it is now closed, by the surface `SceLowering.h`
  declares and the link `sce/CMakeLists.txt` makes.
  **Its price was measured in full and the choice has since been made — see
  "D1 at a glance: five closed by measurement, one closed by decision" below.
  The owner chose to link, on 2026-08-29, and `LuaEngine` sent every
  CLOSED expression to the frontend's parser instead of to the rewriter. That
  took `tests/ecmascript/lua_engine_divergences.json` from 23 entries to 12 —
  the first time that list has emptied rather than grown. Later the same day
  the scope stopped being empty, and then the script path crossed too, and the
  list reached ZERO; see "The scope was the selector, so widening it was the
  whole repair" below. The rewriter is still linked and still the fallback, so
  an empty list is a statement about the shared table, not a retirement.**

### Measured after the seam landed: it took the corpus's grip with the work

The seam was landed with one new mutation case beside it and the rest of
`lua_engine_reads_ecmascript.cases` left alone. Run in FULL the next day, that
casefile came back **7 of 10 CAUGHT, 3 SURVIVED** — and not one of the three
mutations had stopped being a defect. Each had stopped being OBSERVABLE.

The three broke the rewriter's handling of `substring`/`charAt`, of
`Math.round`, and of `\uXXXX`. The shared table asks all three only of CLOSED
expressions — `'abcdef'.substring(1, 3)`, `Math.round(2.5)`, `'é'` — and a
closed expression is precisely what the seam now answers in the frontend. Break
the rewriter there and the engine's answer does not move.

**This is the failure mode a retiring component creates, and it is silent.**
Nothing in the casefile changed, nothing in the suite changed, and a verdict
flipped from CAUGHT to SURVIVED because work moved somewhere else. The
distinguishing question is not "make the mutation stronger" — it is **who
answers this expression now**. Three repairs, in the order to try them:

1. **Re-aim at an open expression**, when the subject is still the rewriter's.
   `sort` and `reverse` are asked of `xs`, which names a variable, so the case
   about a member method's shared definition kept its subject and changed its
   example.
2. **Follow the subject to its new owner.** `Math.round` and `\uXXXX` moved to
   `the_frontend_now_owns_rounding_and_escapes.cases`, against
   `sce-build --test ecmascript_semantics` — 2 of 2 CAUGHT there, red on
   `emitted_lua_answers_what_ecmascript_answers`. A case follows the behaviour;
   it is not deleted and not replaced.
3. **When neither is possible, say what the table does not sample.** The
   rewriter's rounding and escape handling is still live for an OPEN
   expression, and `tests/ecmascript/ecma262_semantics.json` samples neither
   feature open. That is the residue, and closing it is an expansion six
   backend readers answer.

⚠ So: **a round that moves a seam must re-run the casefiles aimed at what the
seam took work away from**, in the same round. The corpus cannot report this on
its own — a casefile is only judged when it changes, and none of these changed.

⚠⚠ And the frontend's cases cannot ride the ctest runner.
`cmake/SCEBuildLowering.cmake` builds the staticlib with `execute_process` at
CONFIGURE time and imports it, so `cmake --build` does not rebuild it after a
Rust edit: a frontend mutation driven through ctest never reaches the binary
and is reported INCONCLUSIVE. The cargo runner is what reaches it.

### The scope was the selector, so widening it was the whole repair

The seam landed asking an EMPTY scope, and that emptiness was doing all the
selecting: the frontend refuses any expression naming something its scope does
not declare, so an empty one admitted exactly the expressions that name
nothing. Eleven closed expressions crossed. The twelve left behind were not a
harder problem — `!a`, `a && b`, `a == null` and the rest were the SAME
problem, asked of a scope that had not been told what `a` is.

So nothing about the rewriter was touched again. A `LuaEngine` session now owns
a `LoweringScope` and tells it what the session holds:

- one `declare` per variable `setVariable` creates (`<data id>`, §scxml-5.3),
  alongside the `declaredVars` entry the engine already kept for its own
  undeclared-variable check;
- one `declare_chunk` per ECMAScript `<script>` that RAN (§scxml-5.8), which is
  where the variables no `<data id>` names come from.

That is the pair the `scope-obligation` and `scope-answer` rows measured as
sufficient — 301 sites, 298 by `<data id>` alone and the last 3 by the chunk —
and it is why the surface has those two entry points and no execution-time
third.

**Measured, in one run, on 2026-08-29**: `EcmaScriptSemanticsOnLuaEngine`
reported `11 declared divergence(s) no longer describe this engine` and NOT ONE
newly undeclared disagreement, so the list went 12 → 1 and
`ARCHITECTURE.md`'s Lua cell 86/98 → 97/98. Both halves matter: the suite is
red in both directions, so a widening that had broken something would have said
so in the same sentence.

**A cached lowering is an answer that depended on the scope.** The expression
cache is keyed on the author's text, and the same text lowers differently once
a `<script>` has declared a name — refused and rewritten before, parsed after.
So `ExprExecInfo` carries the scope generation it was lowered against and a
stale one is a MISS. Without that, whichever evaluation came first would pin
its answer for the life of the session and the later declaration could never
reach it: a correctness bug that would look exactly like a caching win.

#### The one that was left was not an expression, and it went the same way

`n | 14.7.3 while, with continue skipping one iteration` was the whole remaining
list, and it did not diverge in its expression — the expression is `n`. It
diverged in its SETUP, a statement sequence, which reaches the engine through
`loweredScriptOf` and `EcmaScriptToLuaTransformer::transformScript`. No scope
could reach that, because a scope only decides which NAMES resolve; what the
rewriter emitted for it was visible in the suite's own log, `_ = continue` and a
`return` in statement position.

So `loweredScriptOf` was routed through `sce_lower_script`, the frontend's
answer for that shape, by the same seam and the same fallback as its neighbour.
Three things made it a smaller step than it looks:

- **A chunk brings its own names.** `resolve::script` hoists every `var`
  binding into the chunk's frame before anything resolves, so a self-contained
  body is answered even by an EMPTY scope. What the chunk still asks the scope
  about is the names it only reads, which is why this takes the session's scope
  rather than a constant.
- **Refusal is still the fallback**, so the blast radius is bounded in the one
  direction that matters: a body the parser will not read is answered by the
  rewriter exactly as before.
- **`var` at chunk top level is a global assignment**, not a `local`
  (`lua.rs`, `Stmt::VarDecl` — the `local ` keyword is added only when
  `scope.in_function`). A chunk that stopped publishing its variables to the
  datamodel would have failed every setup in the shared table, which is the
  measurement that settles it rather than the reading.

**Measured, in one run**: `EcmaScriptSemanticsOnLuaEngine` reported the last
entry as `now agrees with ECMA-262` and NOT ONE newly undeclared disagreement,
so `tests/ecmascript/lua_engine_divergences.json` is EMPTY and
`ARCHITECTURE.md`'s Lua cell reads 98/98.

⚠ **An empty list is not a retired rewriter.** What the list measures is the
98-case shared table; retiring the rewriter is the claim that nothing takes the
fallback at all, and that needs its own witness rather than this one's silence.

**That witness was built the same day, and building it cost more than deleting
three call sites.** The `retire-rewriter` row and its
`retirement:rewriter-deleted` check are the standing half. The moving half was
what the removal EXPOSED: with no rewriter behind it, a refusal became the
engine's answer, and six cases across two suites went red — every one of them a
name the frontend had never been told about.

- **All 39 DOM reads.** `setVariableAsDOM` wrote a global and told the scope
  nothing, so `var1.nodeType` named a variable the frontend had never heard of.
  A plain missing call, invisible for as long as something answered without
  resolving names.
- **A name a LUA `<script>` introduced.** The scope learned from `<data id>`
  and from an ECMAScript chunk's parse, and the Lua door was skipped on the
  reasoning that a generated artifact lowers at build time and never asks for a
  lowering. That reasoning is about a DOCUMENT; the engine's contract is about
  a SESSION, and it accepts both languages into one namespace. `arr = {10, 20,
  30}` as Lua left `arr[1]` as ECMAScript unresolvable. Lua's own global table
  is now what answers, read against a baseline taken at session creation so
  that `string` and `math` do not leak into an ECMAScript datamodel.
- **Two tests whose SUBJECT was the rewriter**, repointed rather than deleted.
  One asserted that a member access binds the whole chain before it — a real
  property, now the parser's — on a setup written in Lua table syntax and
  handed over as ECMAScript, which only ever ran because a text pass forwards
  what it cannot read. The other compared two successful answers where one came
  from the rewriter; with nothing behind the refusal, the frontend's
  scope-dependence has exactly two outcomes, so it now asserts that a refusal
  is not remembered.

⚠⚠ **The reason all six were invisible is worth keeping.** A fallback that
answers everything makes a missing declaration unobservable — not rare, not
intermittent, *unobservable*, because the only thing that would have shown it
is a refusal that never happened. Removing the fallback is what turned six
silent gaps into six red tests in one run.

#### What the script seam turned out to be worth: a whole entry point

It was adopted to close one divergence. What it actually closed was the route a
DOCUMENT takes, which nothing had noticed was uncovered.

`ecma262-lowered-cpp` had been RED since the seam first landed, for five
pushes, and its census line said `source-wrong=14` against a direct-evaluate
suite reporting zero — one shared table, one declared path, two numbers. The
difference was not a second path. It was this:

- that fixture records all 98 of its answers with `<assign
  location="answers.dNN">`, and a member location is not a bare identifier;
- `AssignmentExecutionHelper` sends a non-bare location down its COMPLEX path,
  which builds `<location> = (<expr>);` and hands it to `executeScript` rather
  than evaluating the expression;
- the seam was on `loweredTextOf` and not on `loweredScriptOf`, so every such
  assignment fell straight back to the rewriter.

`5 ^ 3` answering 125 is the fingerprint: the rewriter only parenthesises
bitwise operands and leaves `^` to Lua, where it is exponentiation. **Measured
after routing `loweredScriptOf` through `sce_lower_script`: the same lane's
census reads `source-wrong=0`.**

⚠ The lesson is about WHERE a seam goes, not about this bug. An engine has more
than one entry point for the same author text, and a seam on one of them is
invisible to a suite that uses the other. `ecmascript_semantics_test` could not
have found this, and did not: it evaluates the expression itself. What found it
was a second suite reaching the same engine the way a document does — which is
what `LoweredEcma262`'s own header says it exists for.
`ScriptLanguageSeam`'s `AMemberLocationAssignmentReachesTheFrontend` now holds
that entry point directly, so the coverage no longer needs a whole lane behind
it.

⚠⚠ One of the list's own two witnesses could not survive the list emptying,
and that is worth stating rather than quietly fixing.** The corpus held the
list to both directions with two mutations: add an entry that has been
repaired, and remove one that is still live. The second is not expressible
against an empty list — there is nothing to remove — and a mutation case that
cannot apply reads as SURVIVED, not as absent. It is not replaced by another
edit to the list: the direction it covered is now carried by the two SEAM
cases, each of which makes the engine really disagree with the shared table
while the list stays empty. That is the undeclared-divergence failure in its
natural form rather than a simulation of it.

⚠ **And zero has to be reachable, which it was not.** The C++ suite opened with
`ASSERT_FALSE(declared.empty())`, meaning "the list was read" — but a list that
is read and EMPTY is the finish line this whole document is aimed at, and that
assertion would have failed on it. A counter whose zero is forbidden is not a
counter. The two questions are now separate: `loadDeclaredDivergences` returns
nothing when it could not READ the file, and an empty answer from a file it did
read is simply true. `ecma262_scoreboard_contract`'s `readers_of` is the other
half — it is what still fails if nothing opens the list at all, which is the
failure the old form was reaching for.

### The price of a C-callable lowering surface, measured 2026-08-29

Measured with a THROWAWAY probe — five `#[no_mangle] extern "C"` wrappers over
the frontend, built as a cdylib, then deleted. Nothing in the tree links Rust
today and nothing does now. Re-derive rather than trusting the table; every row
below names how.

| question | measured | how to re-derive |
|---|---|---|
| crate shape available | `sce-build` is `rlib` ONLY | `grep crate-type sce-build/Cargo.toml` |
| is the wasm wrapping reusable? | the SHAPE yes, the BINDINGS no | `sce-build-wasm/Cargo.toml` — a separate crate, `["cdylib","rlib"]`, depending on `sce-build`; its exports are `wasm_bindgen`, not C ABI |
| surface to expose | **four** lowering fns, not three | `grep '^pub fn' sce-build/src/ecmascript/mod.rs` → `to_lua_condition` / `to_lua_value` / `to_lua_script` / `to_lua_location` |
| what crosses the boundary | a scope, and it is **flat** | `DocumentScope` is one `BTreeSet<String>` (`sce-build/src/ecmascript/scope.rs:53`) — an opaque handle plus `declare` is enough; no model marshalling |
| build time | **7.00s** to recompile `sce-build` and link the cdylib; **14.96s** including its cold deps | `cargo rustc -p sce-build --lib --release --crate-type cdylib`, release, warm cargo cache, build machine (32 cores) |
| artifact size | ~~589 KB / 380 KB ⇒ ~214 KB~~ **Re-derived 2026-08-29: 634.5 KB against 410.8 KB ⇒ +223.7 KB**, and this is now the IN half of a NET — see `swap-net-footprint` in the D1 ledger | **`scripts/measure-lowering-footprint.sh`**, which builds the cdylib with and without `--features ffi` and asks cargo for both paths. ⚠ The first figure came from a throwaway probe that was deleted, so it could not be re-run and it was LOW: those wrappers returned `NULL`, which lets the linker drop the emitter the measurement exists to weigh. The probe is now committed, behind an off-by-default feature, and hands the lowered string back |
| native deps inherited | `libgcc_s`, `libc`, the loader. **libxml2 is NOT** | `ldd` the same path — the `xsd` feature's C library is unreachable from the lowering entry points, so a C++ consumer does not inherit it. `sce-build-wasm` needs `default-features = false` for that reason; a native link would not |
| who pays for it | **every C++ configure in the tree**, not just the `lua` selection | the natural link site is `sce_scripting` beside `lua54` (`sce/CMakeLists.txt:374`), guarded by `SCE_ENABLE_LUA`, which is `option(... ON)` at `sce/CMakeLists.txt:18`. Eight gates build a C++ target: `cpp-suite`, `w3c-cpp`, `w3c-c11`, `forge-cpp`, `lib`, `visualizer-wasm`, `w3c-python-bindings`, `ecma262-lowered-cpp` (`grep -l sce_gate_build scripts/gates/*.sh`) |

⚠ **Ask cargo for the artifact path; do not spell a profile directory.** The
first version of this table re-derived its two size rows with
`ls -l target/<profile>/libsce_build.so`, and `codegen_binary_resolution` turned
the tree-hygiene lane RED over exactly that: *"a second copy of the search is
also a second copy of the profile ORDER — which is what decides whether a stale
binary outranks a fresh one."* The gate is right, and it is worth recording that
the sentence which broke it was written to satisfy the rule that evidence must
be re-derivable. A re-derivation that hard-codes a layout is a second locator.
Cargo already knows where it put the file, so ask it:

```sh
cargo rustc -p sce-build --lib --release --crate-type cdylib \
  --message-format=json 2>/dev/null \
  | python3 -c 'import sys,json
for line in sys.stdin:
    m = json.loads(line)
    for f in m.get("filenames") or []:
        if f.endswith(".so"): print(f)'
```

### Per-call cost, measured 2026-08-29 — and the assumption was backwards

The first of the four unpriced items, decided by the owner as the one to
measure next. Same 98 sources from the shared table, **one host, one load
window** (a cross-machine comparison is not one — the first attempt timed C++
locally and Rust on the build machine and had to be redone).

**Re-derive it with one command; do not cite this table.**

```sh
cmake --build build --target benchmark_ecma_lowering_per_call
scripts/measure-lowering-per-call.sh          # prints the census line below
```

| path | ns/call | cache |
|---|---|---|
| `sce-build` frontend — PARSE then emit Lua, steady state | **577** | none at all |
| `sce-build` frontend — first pass over the table | 1185 | warm-up, *not* a memo |
| `EcmaScriptToLuaTransformer` — COLD text rewrite | **1085** | miss |
| `EcmaScriptToLuaTransformer` — WARM | **12** | its own memo hit |

```
LoweringPerCall census: population=98 frontend_steady_ns=577
  frontend_first_pass_ns=1185 rewriter_cold_ns=1085 rewriter_warm_ns=12
  cold_ratio=1.88 memo_speedup=88.8
```

⚠ **What a re-run should look like, so a reproducer does not read noise as a
contradiction.** Two runs on the same host minutes apart gave `577/1185/1085/12`
and `568/1208/1075/12` — under 2% on every column, and `cold_ratio` 1.88 against
1.89. This machine is shared between sessions, so treat a figure as agreeing
when the RATIO holds; the absolute nanoseconds are the part that moves. A run
that reports `cold_ratio` below 1 has measured something else — almost certainly
the rewriter warm, which is the trap named below.

**Parsing is 1.88x FASTER than rewriting on a cold call.** The intuition that a
parser must cost more than a text pass is refuted here: the rewriter runs a
sequence of full-string scans and rebuilds, and that is dearer than one parse.

⚠⚠⚠ **This table was re-derived on 2026-08-29 and every magnitude in the first
version moved.** It read 1112 / 1535 / 20 and "~1.4x". The direction survived;
none of the numbers did, and the frontend row was wrong in a way worth naming:
**1112 was the FIRST PASS, quoted as the steady-state cost.** A first pass over
a table pays page faults, cold i-cache and allocator warm-up, and here that is
a 2x tax — 1185 against 577 on the same run. The probe that produced it timed
one pass and divided. So the reported gap was understated (1.4x, when it is
1.88x) by comparing the rewriter's cold cost against the frontend's *coldest*
cost rather than its real one.

That the first version could not be checked is the reason it stayed wrong for
a round: it lived in a throwaway `/tmp` script that was deleted when the round
ended. Both halves are now committed targets —
`sce-build/examples/lowering_per_call.rs` and
`tests/benchmarks/EcmaLoweringPerCallBenchmark.cpp` — built by `cargo` and
CMake respectively, so they cannot rot unnoticed, and
`scripts/measure-lowering-per-call.sh` runs them in one window because that is
the only way the two columns are comparable. Neither asserts a bound: this
machine is shared, and the same 21 gates have been measured at 529s and 1161s.

⚠⚠ **The trap this measurement walked into first, and it is the third time on
this axis.** `EcmaScriptToLuaTransformer` keeps THREE `mutable` memo caches
inside itself — `generalCache_` / `guardCache_` / `scriptCache_`
(`sce/include/scripting/EcmaScriptToLuaTransformer.h:130-132`) — so a probe that
reuses one instance measures a hash lookup and reports a two-digit figure (12
here, 22-35 on the first attempt). That is a floor, not a cost, exactly as an
unexported cdylib's 380 KB was. A fresh instance per call is what makes every
call a miss, and the C++ benchmark now constructs one INSIDE the timed region
for exactly that reason, with the reason written above the fixture.

⚠ Note the symmetry the corrected numbers expose: **both halves had a
warm/cold confusion, in opposite directions.** The rewriter was nearly measured
warm and called cheap; the frontend was measured cold and called expensive. The
discriminator is the same in both cases — ask whether the second pass costs
what the first did.

**What it means for the decision.** A runtime lowering path is not slower per
translation — it is faster, and by more than the first measurement said. What
it does not have is the rewriter's memo, worth **89x** here (1085 against 12).
The 12-vs-577 gap is entirely that memo, not the algorithm. The frontend would
need one of the same shape, which is the cheapest part of the work: `LuaEngine`
already keys a per-session chunk cache on the incoming text one layer above.
⚠ And the memo is what a decision has to weigh, not the per-call figure: at
577ns a cold lowering is already cheap enough that only a hot guard evaluated
per event could notice, which is precisely the case `LuaEngine`'s existing
cache already covers.

⚠ **What the probe did NOT price, and a decision needs**:

- ~~**Per-evaluation cost.**~~ **Measured 2026-08-29, then re-derived the same
  day and corrected — see the table above.** Parsing is **1.88x** faster cold
  (577ns against 1085ns); the whole of the rewriter's advantage is its memo,
  worth 89x. Re-derive with `scripts/measure-lowering-per-call.sh`, which is
  in the tree precisely because the first version of this bullet cited a
  number from a deleted `/tmp` probe and was wrong by 2x.
- ~~**The scope obligation at run time.**~~ **Measured 2026-08-29 — see "The
  scope obligation, counted" below.** The obligation is real (301 of 1120
  sites), but **298 of those 301 are discharged by reading the datamodel
  alone**, which a run-time caller can do once, before running anything. The
  residue is **3 sites in 3 documents**, and they are named rather than
  counted. Re-derive with `scripts/measure-scope-obligation.sh`.
- **The error channel** — ~~unmeasured~~ **settled 2026-08-29, and without a
  benchmark, exactly as this bullet predicted. See "The error channel, counted"
  below.** 15 codes against 0. ⚠ The OWNERSHIP half of this bullet is still
  open and is stated at the end of that section.
- ~~**The size of what a C++ consumer takes on.**~~ **Re-priced 2026-08-29 as
  a SWAP, and the first version had the wrong SHAPE rather than a wrong
  number.** It priced the link as an ADDITION — the cdylib's reachable
  lowering code — when the frontend becoming callable at run time is what
  retires `EcmaScriptToLuaTransformer`, whose translation unit is listed
  UNCONDITIONALLY in `sce/sce_base_sources.cmake` and so is compiled by every
  C++ configure in the tree: the same population the link would be paid by,
  which is what makes a net a subtraction rather than two unrelated numbers.
  Measured: **+223.7 KB** in, **−76.0 KB** out, **net +147.8 KB**. Re-derive
  with `scripts/measure-lowering-footprint.sh`.
  ⚠ The tracked source that leaves is **2262 lines** — `.cpp` 2127 plus `.h`
  135. A count of "2262 + 135" is the header twice; the `.cpp` has gone
  1775 → 1872 → 1929 → 2127 and was never 2262. Kotlin's own rewriter (1175
  lines) is a separate path and does not leave with this one, and neither
  does `ecma_semantics.lua`: four language runtimes need that shim, and so
  does lowered Lua.
- ~~**What the scope residue actually asks a C surface for.**~~
  **Answered 2026-08-29 — see "The scope obligation" below.** The residue was
  three NAMES and is now a count of **zero**: all three are names a
  document-level `<script>` introduces, W3C SCXML 5.8 evaluates those at load
  time, and the stage ladder had them arriving *after* `<assign>`. With the
  order corrected the census reads `load_time_diverging=0`, so the surface
  needs `declare` + `declare_chunk` and no scope that tracks execution.
- ~~**Whether the link is unconditional.**~~ **DECIDED 2026-08-29 by the
  owner: link it, beside `SCE_ENABLE_LUA`, and retire the rewriter.** The
  question this bullet held open was never a missing number — both sides were
  costed — and it is recorded here as a decision rather than deleted, because a
  ledger that erases the item it closed cannot be audited. What was chosen and
  what it costs is the `link-beside-lua` row below; what remains of the
  reasoning is kept in the rest of this bullet, since it is why the answer is
  the capability and not the selection.
  `SCE_ENABLE_LUA` is a capability, not a selection, so a link
  placed beside `lua54` is paid by developers who never choose Lua. Scoping it
  to `SCE_SCRIPT_ENGINE=lua` instead would contradict `LuaEngine` being
  constructible whenever the capability is on — which is what
  `EcmaScriptSemanticsOnLuaEngine` relies on to measure it at all.
  ⚠ The price the ledger carried was measured on a cdylib DELTA (+223.7 KB),
  and the shape actually chosen is a staticlib linked into an image that had no
  Rust in it at all. Re-measured on the chosen shape: **474.6 KB** of stripped
  image, because the cdylib's baseline already carried Rust's runtime and a
  C++ image does not. Both numbers are real; only the second is the one a C++
  consumer pays.
  ⚠⚠ **~~HOW TO MEASURE IT~~ — there is nothing left to measure, and the
  prescription this bullet used to carry measured the wrong subject.** It said:
  configure one tree with `SCE_ENABLE_LUA=ON` and one with `OFF`, build the same
  target from cold, take the difference. Re-derived 2026-08-29 and withdrawn —
  that experiment priced `lua54` and two translation units, which every
  developer already pays for, while nothing in the tree linked a Rust artifact
  for it to price — a premise the link has since made false, deliberately, and
  the paragraph under "D1 at a glance" is where that is worked out. Both sides
  of the actual trade are already costed; see that section. What remains is the
  second half, which was never a
  number: does any in-tree consumer construct `LuaEngine` in a tree whose
  SELECTION is not lua?
  `EcmaScriptSemanticsOnLuaEngine` does — it is `#ifdef SCE_ENABLE_LUA`, not
  `SCE_SCRIPT_ENGINE_LUA` — so scoping the link to the selection would take that
  suite's subject away in every default build, and every row it holds would
  stop being measured anywhere. That is the trade, and it is a correctness cost
  against a build-time one.
- **Whether the retirement actually happened.** The bullet above records the
  DECISION; this one is the other half of it, and it was added the day the
  answer stopped being "not yet". The `link-beside-lua` row holds the tree to
  the link, and a link is a build-system fact: it can be true while every
  expression in the tree still reaches the rewriter, which is precisely what
  the first half of the decision left standing. Retirement is a different
  claim — **nothing takes the fallback at all** — and this document already
  said, in "An empty list is not a retired rewriter", that it *"needs its own
  witness rather than this one's silence"*. The `retire-rewriter` row is that
  witness, and `retirement:rewriter-deleted` is what re-derives it: every
  tracked C++ file, swept, and none of them reaching the rewriter in code.
  ⚠ **It swept the engine's two directories for a day, and that boundary was
  measured and replaced the next.** `sce/src` and `sce/include` are what the
  library is built from, so the boundary read as principled. What it actually
  did was leave `tests/benchmarks/EcmaLoweringPerCallBenchmark.cpp` — the one
  file outside them that really did construct the rewriter, in order to price
  it — not exempted but INVISIBLE: the sweep never opened it, nothing in the
  tree said whether it was an instrument or a caller, and a second file
  joining it would have been just as quiet. The boundary became a
  CLASSIFICATION over the whole tree, and the deletion then collapsed the
  classification too: with the unit and its one instrument both gone, no
  tracked C++ file may reach the rewriter for any reason and no exemption of
  any shape is left.
  ⚠⚠ **Deleting the unit deleted the control, and that is the interesting
  half.** While the unit existed, the sweep's positive control was its own
  files: the predicate that reports zero had to report THEM, or it had stopped
  being able to see the name at all. "No file names X" is also the answer a
  sweep gives when it read nothing, so a check that lost its control while
  keeping its claim now passes for the same reason an unread tree does. What
  buys it back is the PROSE. The engine, its suites and this document still
  explain what the rewriter was, so a sweep that is really opening files finds
  the name in RAW text many times over while finding it in CODE never, and the
  check asserts the raw half against a floor. ⚠ That floor CAN be driven to
  zero — by deleting every comment that remembers the rewriter — and that is
  precisely when this row has to be rewritten rather than quietly relaxed: with
  no mention anywhere, nothing in the tree can show that the sweep still reads.
  ⚠⚠⚠ **Two measurements retired with the subject rather than with the code.**
  `per-call-cost` timed the rewriter against the frontend and `swap-net-footprint`
  weighed the object file that has now gone; neither can be re-measured here.
  They keep their numbers and their `measurement` kind, and their check becomes
  a `retired-measurement:` carrying the commit that still holds the subject — a
  pin the gate resolves with `git cat-file -t` and `git ls-tree`, requiring the
  named commit to hold every departed file and this tree to hold none of them. A reader who wants the number back
  checks that commit out and runs the probe, which is why both probes are kept
  and why each one says so in its own header.
  ⚠⚠⚠⚠ **A shallow clone cannot answer that question, and the gate says so
  rather than answering for it.** `actions/checkout` fetches depth 1, so on
  2026-08-29 the pin — the tip's own parent that day — was absent wherever a
  lane cloned shallow, and the arm went red for a tree that was correct: TWO
  of the four lanes red at that commit were this arm (`tree-hygiene`, and
  `mutation-rounds`, where it surfaced as `baseline is not green (1 failing)`
  rather than as itself), while the other two were the deleted translation
  unit's own residue — two unbacked ledger bindings and the embed manifest's
  symbol count. Counting red lanes is not counting causes. A check that
  reads *cannot answer* as *the answer is no* is measuring the clone, not the
  ledger. The miss is now CLASSIFIED: the only way past the arm is for the
  repository to PROVE it is shallow, and a full clone that cannot resolve the
  pin is still red. ⚠⚠⚠⚠⚠ That classification would be an exemption if no
  lane ever ran with full history, so the precondition is asserted, whole and
  DERIVED rather than named: which `scripts/gates/` script selects
  `--test lowering_decision_ledger`, which push-triggered job runs that gate,
  and whether every one of those jobs asks for `fetch-depth: 0` in code. Half
  a precondition is the failure this repository has already paid for — a lane
  that stops RUNNING the test satisfies a fetch-depth assertion perfectly
  while verifying nothing.

### D1 at a glance: five closed by measurement, two closed by decision

Four things had to be priced before a person could choose whether `sce-build`
grows a C-callable lowering surface. All four were, two more numbers arrived
while they were being checked, and the decision the four informed then split
into two rows — the link and the retirement — so the table below carries seven.

**The choice was then made.** On 2026-08-29 the owner chose to LINK the
frontend and retire `EcmaScriptToLuaTransformer`; the `link-beside-lua` row
records it. That row is not a measurement and cannot be re-run, so what its
check holds is the tree still doing what was decided — the surface exists, a
CMake file links it, the link sits inside `if(SCE_ENABLE_LUA)` rather than
behind the engine selection, and `LuaEngine` actually calls it. The last is
there because a linked library nothing reaches is discarded by the linker,
which would leave the row unable to fail.

**And the second half of that decision has its own row**, because the two can
be true separately and were: for the whole of the day the link landed, the
frontend was called FIRST and the rewriter answered everything it refused.
`retire-rewriter` is the claim that the second call is gone.

⚠ **This table is not a second copy of the sections above — it is the
machine-readable one.** `sce-build/tests/lowering_decision_ledger.rs` parses
exactly the block between the markers below and holds every row to the check
the row itself declares. A row carrying a status, a kind or a check the gate
does not recognise is RED, not skipped: an unclassified row is how a ledger
stops being a ledger.

⚠⚠ **And a check that reads PROSE is not a check — measured here, not
reasoned.** `swap-net-footprint` rests on a probe some lane has to compile, so
the row's check requires one; the first form of that requirement asked whether
the text `ffi` occurred anywhere under `scripts/gates/` or
`.github/workflows/`. The comment in `tree-hygiene.sh` explaining why the
feature is named there satisfies that on its own, and a mutation that deleted
the feature from the cargo invocation while leaving the paragraph standing
**kept the gate green with nothing building the probe** — blind in precisely
the case it was written for, which is the same defect as a residue nobody
enumerates. The check now reads `--features` ARGUMENTS, with commentary and
commented-out invocations cut away first, and
`a_named_feature_is_not_a_passed_feature` pins that boundary so a later
simplification back to a substring search fails there before it can pass here.
Every row's check has since been shown red by a mutation: the ladder reordered,
the load-time stage orphaned, the feature dropped, the rewriter put behind a
generator expression, and its line count moved.

⚠⚠⚠ **And the same defect was then found a second time, in the row added to
record the decision — by running the mutation rather than by re-reading the
code.** `decision:linked-beside-lua` rests on `LuaEngine` actually CALLING the
surface, and the first form of that check searched the whole of
`LuaEngine.cpp` for `sce_lower_value(`. On 2026-08-29 the call was deleted from
`LuaEngine::loweredTextOf` and the paragraph explaining it left standing,
exactly as a careless revert would leave it: the C++ suite went red on eleven
expressions and **this ledger stayed green**. Two lessons, and the second is the
one worth carrying. First, the repair is the same one language over — the check
now reads CODE, with commentary and the contents of string literals cut away by
`cpp_code_only`, and `a_mentioned_call_is_not_a_made_call` pins the boundary.
Second, and this is why the paragraph above was not enough: **a lesson recorded
beside one check does not reach the next check written.** The prose warning had
been in this file for a day, three paragraphs above the row whose check
repeated the defect. What caught it was mutating the new row, so the rule is
that a row is not closed until its own mutation has been run — not until
someone has read the warning.

<!-- D1-LEDGER v1
     columns: id | status | kind | number | check | evidence
     Parsed by sce-build/tests/lowering_decision_ledger.rs.
     Do not restate these rows elsewhere in this file. -->

| id | status | kind | the number | check | evidence |
|---|---|---|---|---|---|
| `per-call-cost` | CLOSED | measurement | 577ns against 1085ns cold — parsing is **1.88x faster**; the rewriter's whole advantage is a memo worth **89x**. ⚠ RETIRED: the second side of this comparison was deleted with the rewriter, so the number cannot be re-measured in this tree — the pinned commit still holds the probe and the subject, and the check requires both | `retired-measurement:59eb7f96022fa4a10330fbfd70c05b45671af443` | `scripts/measure-lowering-per-call.sh` |
| `scope-obligation` | CLOSED | measurement | **301** of 1120 sites diverge with no scope; **298** of them are discharged by `<data id>` alone, before anything runs; residue **3**, named rather than counted | `census:ScopeObligation` | `scripts/measure-scope-obligation.sh` |
| `error-channel` | CLOSED | counting | **15** distinguishable failures against **0** — so the FFI carries a code plus a string, and the code already exists | `derive:expression-alphabet=15` | `sce-build/src/forge/diagnostic.rs` |
| `swap-net-footprint` | CLOSED | measurement | the link is a SWAP, not an addition: **+223.7 KB** in, **−76.0 KB** out ⇒ **net +147.8 KB**, so pricing it as an addition overstates by **34%**. The rewriter's **2262** tracked lines have now LEFT — the OUT half was actually paid. ⚠ RETIRED: the object this weighed no longer exists in the tree, so the number is re-derivable only at the pinned commit, which the check requires to still hold it | `retired-measurement:59eb7f96022fa4a10330fbfd70c05b45671af443` | `scripts/measure-lowering-footprint.sh` |
| `scope-answer` | CLOSED | measurement | **0** sites diverge once the caller has read every `<data id>` AND every document-level `<script>` — both readable before the first macrostep — so the surface needs `declare` + `declare_chunk` and NO execution-time scope | `derive:scope-ladder=LoadTime` | `sce-build/src/ecmascript/scope.rs` |
| `link-beside-lua` | CLOSED | decision | the owner chose to LINK, beside `SCE_ENABLE_LUA`, and retire the rewriter. Priced on the shape chosen: a staticlib costs **474.6 KB** of stripped image, not the **223.7 KB** the cdylib delta reported — that baseline already held Rust's runtime and a C++ image does not | `decision:linked-beside-lua` | `sce/CMakeLists.txt` |
| `retire-rewriter` | CLOSED | decision | the second half of that decision, carried out to the end: **zero** tracked C++ files reach `EcmaScriptToLuaTransformer` in code, and the unit is now DELETED — nothing is named after it, `sce/sce_base_sources.cmake` no longer lists it, and no exemption of any shape is left. The fallback is gone from all three sites — `loweredTextOf`, `loweredScriptOf`, `reset` — and refusal is the engine's own answer (§scxml-5.9.1) rather than a second translator's cue. ⚠ Deleting the unit deleted the control that made the zero mean something, so the check buys it back from the PROSE: the files that still explain the rewriter in comments must stay above a floor, because a sweep that read nothing answers zero the same way | `retirement:rewriter-deleted` | `sce/src/scripting/LuaEngine.cpp` |

<!-- /D1-LEDGER -->

**Why the ON/OFF experiment was withdrawn — and what has since made it
measurable.** The bullet above prescribed: configure one tree with
`SCE_ENABLE_LUA=ON` and one with `OFF`, build the same target from cold, take
the difference. Re-derived on the morning of 2026-08-29, **that experiment did
not contain its subject.** `SCE_ENABLE_LUA` guarded `lua54` and two translation
units (`sce/CMakeLists.txt:304-305`); no tracked CMake file linked a Rust
artifact at all, and `sce-build` was `crate-type = ["rlib"]` with no in-tree
consumer of any other shape. So the ON/OFF delta priced the Lua capability as
it already stood — a bill every developer already pays — and said nothing about
the surface whose link was the thing being decided. A real number about the
wrong subject is precisely the defect this file recorded once already, when
"159 of 382" was reused as the `cpp-suite` lane's size.

⚠ **That premise is false as of the afternoon of the same day, and it was made
false on purpose.** The link landed inside `if(SCE_ENABLE_LUA)`, so the ON/OFF
delta now does contain the surface — it would price the staticlib together with
`lua54`, which is the capability as it now stands. It is not re-run, because
the decision it existed to inform has been taken, and what the chosen shape
costs is the `link-beside-lua` row's own number, measured on the staticlib that
was actually linked rather than on a cdylib delta. What the withdrawal leaves
behind is the shape rather than the sentence: a premise that holds a row OPEN
has to be re-derived before the row is read, and the handler for one is kept in
`sce-build/tests/lowering_decision_ledger.rs` — with the reason it did not fire
when this link arrived — so the next premise is held the same way.

The subject's own price is already in the table above and needs no second
experiment: **7.00s** to rebuild `sce-build` and link the cdylib (14.96s
including cold deps), and **net +147.8 KB** once what leaves with the rewriter
is subtracted — paid by the eight gates that build a C++ target. The other
side of the trade is priced too, and it is not in seconds:
scoping the link to `SCE_SCRIPT_ENGINE=lua` would take
`EcmaScriptSemanticsOnLuaEngine` its subject in every default build — verified,
that suite is guarded by `#ifdef SCE_ENABLE_LUA` in
`tests/engine/EcmaScriptSemanticsTest.cpp`, not by `SCE_SCRIPT_ENGINE_LUA` —
and every divergence row it holds would stop being measured anywhere. How many
rows that is stays deliberately unwritten here: it is
`tests/ecmascript/lua_engine_divergences.json`'s own length, it moved three
times in one day, and a count restated beside the file that owns it is the
defect the whole of D1 was written against. The line number that used to
accompany that `#ifdef` is gone for the same reason — it was already wrong,
seventeen lines out, and nothing re-derived it.

**Neither side was missing a figure, so what remained was a judgement about
what a C++ consumer should carry — and on 2026-08-29 the owner made it: LINK
the frontend, beside `SCE_ENABLE_LUA`, and retire the rewriter.** The
`link-beside-lua` row records the decision and `decision:linked-beside-lua`
holds the tree to it — the surface exists, a CMake file links it, the link sits
inside the capability rather than behind the engine selection, and `LuaEngine`
actually calls it. That last clause carries the row: a linked library nothing
reaches is discarded by the linker, so without it the row would describe a
build-system fact with no behaviour behind it and could not fail.

⚠ **A check name spelled in this document is a promise that some gate
re-derives the sentence beside it, so a retired one may not be left standing.**
That is not a style rule. This section went on naming the precondition check
the OPEN row used to carry for a day after that row was replaced, promising
that the day anything in the tree linked a Rust artifact the gate would turn
red and the row would have to be re-adjudicated — and the tree linked one that
same afternoon, with no row left to turn. The name is not repeated here,
because repeating it is the defect;
`sce-build/tests/lowering_decision_ledger.rs` keeps it, beside the reason it
did not fire. `a_check_named_in_prose_is_a_check_a_row_declares` now reads
every check identifier this file spells and fails on one no row declares, so
the promise cannot outlive its row a second time.

### The error channel, counted 2026-08-29

The third of the four, and the only one this document said could be settled by
counting rather than by measuring. It was.

**Re-derive it with three commands; do not cite the numbers.**

```sh
# the frontend's alphabet: variants, and the codes they map to
grep -c 'ExprError::' sce-build/src/ecmascript/*.rs      # construction sites
sed -n '/pub enum ExprError {/,/^}/p' sce-build/src/forge/error.rs | grep -cE '^    [A-Z]'
grep -cE '^\s{4}Expression[A-Za-z]+,' sce-build/src/forge/diagnostic.rs

# what the run-time path can say at its boundary
sed -n '/^struct ScriptResult/,/^public:/p' sce/include/scripting/ScriptResult.h
grep -o 'createError([^;]*' sce/src/scripting/LuaEngine.cpp | sort -u
```

| | build-time frontend | run-time rewriter |
|---|---|---|
| failure kinds distinguishable at the boundary | **15** `ExprError` variants | **0** |
| machine-readable code | **15** distinct `DiagnosticCode`s, one per variant | none — `ScriptResult` is `bool` + `std::string` |
| structured repair data | `available` / `members` / `candidates` / `arguments`, driving `Fix::ReplaceOneOf` and `Fix::ReplaceWith` | none |
| author-facing message shapes | one per variant, `#[error(...)]` | 4 authored (`ReferenceError: …`, `Session not found: …`, `Syntax error: …`, `Unknown Lua error`) plus raw Lua strings passed through |

11 of the 15 variants are constructed by the ECMAScript frontend itself, over
30 sites in `sce-build/src/ecmascript/` (`lua.rs` 10, `builtins.rs` 10,
`parser.rs` 9, `resolve.rs` 1); the remaining four reach it through the shared
expression pipeline.

**The question this bullet posed is answered: the frontend's alphabet is
strictly richer — 15 codes against 0 — so the FFI must carry a code plus a
string, not a boolean.**

⚠ **And the code does not have to be invented.** `payload_for` already maps
every `ExprError` variant to its own `DiagnosticCode`, the mapping is a
bijection onto the enum's 15-member `expression/` family, and those spellings
are already wire-visible and pinned by `schemas/sce-diagnostic.v1.schema.json`.
A C surface should carry THAT code rather than a private enum of its own — the
alternative is a second alphabet for the same failures, which is the shape
`SCE_WIRE_CONTRACTS.md` exists to refuse.

⚠⚠ **The property the decision rests on was not asserted anywhere, and now is.**
That `payload_for` covers every variant is compiler-enforced; that no two
variants land on the SAME code is not, and a collapse would silently make the
code insufficient — the consumer would be back to parsing the string. Held by
`no_two_expression_errors_share_a_diagnostic_code`, which reads the golden
corpus (already exhaustive over the code enum via `every_code_has_a_golden`, so
a shrinking alphabet cannot read as agreement) and takes its floor from the
enum rather than restating it. **Verified by mutation in the turn that landed
it**: pointing `ExprError::LiteralNotCallable` at
`DiagnosticCode::ExpressionPropertyNotCallable` — a one-token change the
compiler accepts — failed it with *"expression/property-not-callable is
produced by 2 variants"*.

⚠ **What this does NOT settle: the ownership half.** The probe's four entry
points returned `NULL` and leaked; that is a leak test, not a design question.
**HOW TO MEASURE IT** — run the four entry points over the corpus under
ASan/LSan and require zero bytes still reachable. With a `free` entry point the
contract is trivially satisfiable, so the measurement exists to say the caller
actually called it, not to discover whether it can be.

⚠ **What it also does not settle, and is worth naming**: the 15 codes are what
the frontend can *say*, not what a consumer receives today. The run-time path
reports through `ScriptResult`, which has no code field at all, so adopting the
frontend's alphabet at an FFI boundary is a change to that struct as much as to
the FFI. That is the same open debt as the `_event.data` gap and should be paid
with it, not twice.

### The scope obligation, counted 2026-08-29

The second of the four, and the one that could have been answered "no
obligation" — which is why the way it is measured matters more than the
number.

**Re-derive it with one command; do not cite this table.**

```sh
scripts/measure-scope-obligation.sh
```

A build-time caller reads the whole document before it lowers anything, so it
always holds the full scope. A caller that lowers *while the document runs*
does not: when the first `<transition cond>` is evaluated, a `<script>` further
down has not executed and an `<assign>` further down has not written. The
frontend now names those stages — `ScopeStage`, in
`sce-build/src/ecmascript/scope.rs` — and the probe lowers every expression in
every tracked document once per stage, comparing each against the stage a
build-time caller has.

| what the caller has read | sites disagreeing with a full scope | documents |
|---|---|---|
| nothing — an FFI with no scope handle | **301** of 1120 (26.9%) | 116 of 225 |
| plus every `<data id>` | **3** (0.3%) | 3 |
| plus every DOCUMENT-LEVEL `<script>` | **0** | 0 |
| plus every write target (`<assign location>`, `<send idlocation>`, `<foreach>`) | **0** | 0 |

```
ScopeObligation census: documents=225 sites=1120 installed_diverging=301
  installed_documents=116 datamodel_diverging=3 datamodel_documents=3
  load_time_diverging=0 load_time_documents=0
  write_targets_diverging=0 write_targets_documents=0
```

⚠⚠ **The third row did not exist when this was first measured, and its
absence is why the answer read as a residue.** `ScopeStage` absorbed
document-level `<script>` declarations at `Everything` — the LAST stage —
so a name W3C SCXML 5.8 puts in the datamodel at load time was modelled as
arriving after an `<assign>` that runs mid-execution. In a ladder whose whole
purpose is the ORDER, that is the defect, and it had the exact shape this file
retired once already: a real number about the wrong subject. The stage now sits
between `DataModel` and `WriteTargets`, where the specification puts it, and
the count that was three is zero.

⚠ Note what did NOT change: the same three sites still diverge at `DataModel`,
and they are still listed below. What changed is that they are now the
population a SECOND pre-run call answers, rather than a remainder a caller
could only reach by running the document.

**What it means for the decision, and it is an ANSWER rather than a residue.**
The obligation is real — a C surface that took only a source string would be
wrong about a quarter of the corpus — but **all of it is discharged before
anything runs**, from two sources a caller reads out of the model it already
holds:

1. every `<data id>`, which W3C SCXML 5.3's early binding puts in the datamodel
   before the first macrostep — **298 of the 301**;
2. every document-level `<script>`, which W3C SCXML 5.8 evaluates at document
   load time, also before the first macrostep — **the remaining 3**.

So the C surface needs exactly two declaring calls, `declare` and
`declare_chunk`, both made once before anything runs, and **no scope that
tracks execution**: write targets discharge zero further sites, because not one
expression in the corpus depends on a name only an `<assign>` brings into
existence. That is the whole shape of the scope obligation, and the count that
says so is `load_time_diverging=0`.

⚠ **A zero is the answer here, which is what makes a FALSE zero the expensive
failure**, so it is asserted by `scope_obligation_census` and the instrument is
held open by `every_stage_boundary_is_observable` — four controls now, one per
boundary, including a document-level `<script>` that the census could not see
at all before the stage existed.

**The 3 that the second call answers, named rather than counted**, because a
population nobody can enumerate is a population nobody has classified. All
three are the same thing — a name a top-level `<script>` introduces — which is
why one entry point covers them:

| document | site | source | without the chunk |
|---|---|---|---|
| `resources/302/test302.scxml` | `<transition cond>` | `Var1 == 1` | `Var1 is not declared by this document` |
| `resources/304/test304.scxml` | `<transition cond>` | `Var1 == 1` | `Var1 is not declared by this document` |
| `resources/452/test452.scxml` | `<assign expr>` | `new testobject();` | `testobject is not declared by this document` |

Two of the three are the W3C tests written to check exactly this — that a
`<script>` declaration reaches the datamodel — so the population is not an
accident of this corpus, it is the specification's own witness for the feature.
That is the case `declare_chunk` has to exist for, and it is the whole of it.

⚠⚠ **A zero here would have been a decision, which is what makes a FALSE zero
the expensive failure.** Every way this measurement could go blind produces one
— a staging argument that is ignored, a glob that matches nothing, a parse that
fails everywhere — and a blind instrument reports "no obligation" in exactly the
words a real absence would. So the stage boundaries are not merely printed, each
is held open by a document written to cross it
(`every_stage_boundary_is_observable`). **Verified by mutation in the turn that
landed this**: widening `from_model_upto`'s stage to `Everything` — a change the
compiler accepts and which the census cannot feel — made the census report
`installed_diverging=0` and still pass, while the control failed with *"no site
lowers differently between installed and datamodel"*. The census is the
deliverable; the control is what makes it evidence.

⚠ **The probe is a filter over the acceptance sweep, not a second walk.**
`ecmascript_acceptance::sites` was extracted from `refusals` for this — the walk
reaches fourteen call sites, and a copy of it would have drifted from the
original without saying so. `refusals` is now that walk plus a verdict, which is
also why it did not need re-testing separately.

⚠ **Found while measuring, and it was a red already on `main`.** The sibling
script this one is modelled on, `scripts/measure-lowering-per-call.sh`, printed
its census header from `/proc/loadavg` directly, which
`sce-build/tests/build_jobs_has_one_owner` refuses: that arithmetic has one
home. It had been failing since the commit that added it. Repaired by sourcing
`scripts/lib/sce_build_jobs.sh` and reporting the free-core count it computes,
which is the more useful header anyway — the figures it heads are only
comparable between runs taken under similar load.
- ~~**Generated C++ in a Lua tree still hands the engine ECMAScript.**~~
  **Done 2026-08-29 (third round)** — `sce_add_state_machine` derives
  `--script-engine lua` for a `-DSCE_SCRIPT_ENGINE=lua` tree, measured on a
  third artifact generated by a call that names nothing.
  ⚠ It moves **zero** rows out of `lua_engine_divergences.json`, and the
  reason is a measurement rather than a guess — both suites holding that column
  reach the engine by routes a codegen default cannot touch.
- ~~Whether the target engine becomes a codegen argument, a per-artifact
  variant, or a runtime-selected pair of emissions.~~ **Answered 2026-08-28:
  a codegen argument.** The other two were not chosen against so much as ruled
  out by what the artifact is — a Lua-shaped artifact can only run on a Lua
  engine, so the choice has to be made where the artifact is produced, and it
  has to be reported on the manifest for the host that will supply the engine.

This file records the measurement, and now also the decisions taken on it.

## Measured 2026-08-29 (fifth round): the Kotlin rewriter's place, in numbers

The C++ half of this seam is closed — the rewriter is deleted, its divergence
list is empty, and a Lua-lowered artifact is compiled and run on a lane of its
own. `backends/kotlin/lua/.../EcmaScriptToLuaTransformer.kt` (**1175** lines) is
what remains. Before anything is moved, two questions had to be answered with
numbers rather than by analogy with the C++ side: is it answering the same
population, and is there a seat for it to be retired into. Both are measured
below, every figure with the command that re-derives it.

### 1. The population is the same, and the list is exactly the table's subset

| | count | |
|---|---:|---|
| shared table, `tests/ecmascript/ecma262_semantics.json` | **98** | 98 unique `(source, clause)` keys |
| Kotlin's declared divergences | **46** | `tests/ecmascript/kotlin_lua_divergences.json` |
| of those, inside the shared table | **46** | |
| of those, outside it | **0** | so it is a subset, not a second corpus |
| C++'s declared divergences today | **0** | `tests/ecmascript/lua_engine_divergences.json` |

So the Kotlin rewriter is measured against the same 98 claims the C++ one was,
by a reader that loads `cases` with no filter and refuses a table under 55 rows.
It is not a smaller or a differently-drawn population.

```sh
python3 - <<'PY'
import json
sem = json.load(open('tests/ecmascript/ecma262_semantics.json'))
kot = json.load(open('tests/ecmascript/kotlin_lua_divergences.json'))
shared = {(c['source'], c['clause']) for c in sem['cases']}
kd = {(d['source'], d['clause']) for d in kot['divergences']}
print(len(shared), len(kd), kd <= shared, sorted(kd - shared))
PY
```

**Re-made in the turn that wrote this**, because a subset relation between two
files says nothing about what the engine answers:
`./gradlew :sce-kotlin-tests:test --tests com.sce.ecmascript.EcmaScriptSemanticsTest`
ran the task — not UP-TO-DATE, `> Task :sce-kotlin-tests:test` with three
PASSED lines under it — and its three cases are the measurement:
`rhinoAnswersWhatEcmaScriptAnswers` and `quickJsAnswersWhatEcmaScriptAnswers`
put those two engines at 98 of 98, and `luaIsNotAnEcmaScriptEngineAndSaysSo`
holds the Lua engine to the declared set in BOTH directions — so 46 is the
live answer, not a stored one.

Where the 46 fall, which is the shape a repair would have to take:

| group | table | rewriter wrong |
|---|---:|---:|
| truthiness | 10 | 7 |
| builtins | 29 | 17 |
| equality | 7 | 4 |
| functions-and-statements | 5 | 3 |
| bitwise | 8 | 4 |
| arrays-and-objects | 12 | 4 |
| literals | 4 | 2 |
| json | 6 | 2 |
| addition-or-concatenation | 5 | 1 |
| remainder | 2 | 1 |
| side-effecting | 3 | 1 |
| typeof-and-instanceof | 7 | **0** |
| **total** | **98** | **46** |

### 1b. What the same population does NOT mean: three fixtures reach this engine, and two of them bypass the rewriter

`EcmaScriptSemanticsTest` is not the only Kotlin reader that instantiates
`LuaScriptEngine`. `DomReadSurfaceTest` (**39** cases) and
`EventDataReadingsTest` (**10**) do too — and both hand the Lua engine the
fixture's `lua` column, which is what `sce-build`'s frontend emitted, while
handing Rhino and QuickJS the `source` column. Those 49 cases are therefore
already exercising the OTHER path on this backend, today, green.

That is the encouraging reading, and measuring it costs it most of its force.
The two columns have to actually differ for a case to be evidence about
lowering at all, and mostly they do not:

| fixture | cases | `lua` differs from `source` |
|---|---:|---:|
| `tests/ecmascript/dom_read_surface.json` | 39 | **13** |
| `tests/ecmascript/event_data_readings.json` | 10 | **0** |
| `tests/ecmascript/ecma262_emitted_lua.json` | 98 | **89** |

So the standing evidence that Kotlin's Lua runtime can consume frontend-lowered
text is **13 cases, all DOM reads** (`#var1.childNodes` for
`var1.childNodes.length`, `var1:getAttribute("count")` for
`var1.getAttribute('count')`). The event-data fixture contributes nothing to
that question — its ten `lua` spellings are byte-identical to their `source`,
so the engine cannot tell which column it was handed and the run would pass
with the seam absent.

The corpus that WOULD answer it is the third row: the 98 lowered expressions,
89 of which differ. On the day this was measured it had three readers —
`backends/go/lua/ecma262_semantics_test.go`,
`backends/python/tests/ecmascript/test_ecma262_semantics.py` and
`sce-build/tests/ecmascript_semantics.rs` — and **none on Kotlin**. The round
below added the fourth; the `grep` in the fence is what answers how many there
are today, and this sentence is dated rather than current on purpose.

```sh
python3 - <<'PY'
import json
for p in ('tests/ecmascript/dom_read_surface.json',
          'tests/ecmascript/event_data_readings.json'):
    cs = json.load(open(p))['cases']
    print(p, len(cs), sum(c['lua'] != c['source'] for c in cs))
e = {c['source']: c['expression']
     for c in json.load(open('tests/ecmascript/ecma262_emitted_lua.json'))['cases']}
s = json.load(open('tests/ecmascript/ecma262_semantics.json'))['cases']
print('emitted_lua', len(s), sum(e[c['source']] != c['source'] for c in s))
PY
grep -rl ecma262_emitted_lua --include='*.kt' --include='*.go' --include='*.py' --include='*.rs' .
```

### 2. Kotlin has no build-time lowering path, and the seat it would take is empty rather than missing

The generator answers this itself; it does not have to be read off the
templates. `--script-engine lua` is refused by a backend that cannot emit a
wholly-lowered artifact, and the refusal counts what is left:

```sh
cargo build --bin sce-codegen --features cli -p sce-build
for L in kotlin cpp rust go python c; do
  ./target/debug/sce-codegen generate examples/widget_patterns/button.scxml \
      -o /tmp/seam -l "$L" --script-engine lua; echo "$L rc=$?"
done
```

| backend | `--script-engine lua` | sites still handing over source | sites unclassified |
|---|---|---:|---:|
| `kotlin` | **refused** (rc=1) | **6** | **29** |
| `cpp` | accepted (rc=0) | 0 | 0 |
| `rust` / `go` / `python` / `c11` | accepted — it is their default | — | — |

The **6** are `kotlin/actions/if.kt.jinja2` (twice),
`kotlin/actions/script.kt.jinja2`, `kotlin/process_event.kt.jinja2`,
`kotlin/scriptengine_helpers.kt.jinja2` and
`kotlin/transition_actions.kt.jinja2` — the guard, the `<script>` body, and the
two `<if>` conditions. The **29** are the `send` / `assign` / `foreach` /
`invoke` / `<data>` family, each mentioning a model expression and reaching a
callee this build has not been told about; each needs the same one-time
decision the C++ side's 29 needed, and the population is the same shape, not
the same list.

**What the C++ side put in the rewriter's place, so the seat can be named
rather than invented.** There are two, and both are already built in this tree:

* **Build time** — the `--script-engine lua` codegen target. C++ reaches it
  because both of its scan lists are empty.
* **Run time** — a C ABI over the same frontend: **9** `extern "C"` entry
  points in `sce-build/src/ffi.rs` (`sce_scope_new`, `sce_scope_declare`,
  `sce_scope_declare_chunk`, `sce_scope_free`, `sce_lower_value`,
  `sce_lower_condition`, `sce_lower_script`, `sce_lower_location`,
  `sce_lower_free`), built as a staticlib by `cmake/SCEBuildLowering.cmake` and
  wrapped by `sce/src/scripting/LoweringScope.cpp`, which is what `LuaEngine`
  calls instead of a rewriter.

Kotlin has **neither, and no obstacle in kind to the second**. The C ABI has
**0** Kotlin consumers — but `backends/kotlin/lua/src/main/cpp/` is already a
CMake project (`sce_lua_jni`) that compiles Lua 5.4 and **50**
`Java_com_sce_scripting_lua_LuaNative_*` entry points into a shared library the
JVM loads. The seat is reachable from there by the route the C++ side used; it
is empty, not absent.

```sh
grep -c '^pub unsafe extern "C" fn' sce-build/src/ffi.rs
grep -rl 'sce_lower_value' --include='*.kt' --include='*.cpp' --include='*.c' .
grep -o 'Java_com_sce_[A-Za-z0-9_]*' backends/kotlin/lua/src/main/cpp/lua_jni.cpp
```

### 3. Measured on the way past: the grip on this file is thinner than the C++ one ever was

| | Kotlin | C++, on the day its rewriter was deleted |
|---|---:|---:|
| rewriter, lines | 1175 | 2127 |
| test source files in the engine's own module | **0** | — |
| mutation casefiles naming the rewriter | **0** | 2 |
| mutation casefiles naming the backend at all | 1 of 95 | — |
| lane that runs a lowered artifact end to end | **none** | `scripts/gate ecma262-lowered-cpp` |

`scripts/gate w3c-kotlin` does read the 46 — it runs `:sce-kotlin-tests:test`
once per ECMAScript engine and `EcmaScriptSemanticsTest` measures all three
engines inside each run, so the list is answered twice per gate run. What no
lane asks is the question the C++ side had to answer before it could retire
anything: whether this backend's Lua, given the frontend's output, answers the
table. `ecma262-lowered-cpp` is that question for C++, with the un-lowered
artifact beside it as the control.

```sh
ls backends/kotlin/lua/src/test
grep -l 'EcmaScriptToLuaTransformer.kt' sce-build/tests/mutations/*.cases
ls sce-build/tests/mutations/*.cases
```

### What this measurement leaves open

- ~~**Nothing asks whether Kotlin's Lua answers the lowered 98.**~~ CLOSED by
  the round below. 89 of those expressions differ from their source, three
  backends read them, and this one did not; until it did, "the frontend
  already answers all 98" was a statement about `sce-build` plus somebody
  else's Lua, not about this engine's. `LoweredEcma262Test` now asks it, and
  the answer was not the one this bullet assumed — the question splits in two,
  because most of the emission reaches the engine and some of it cannot.
- **The 35 sites have no ratchet.** `supports_script_engine_target` derives a
  BOOLEAN, and the two lists behind it are only consulted to compute it, so a
  Kotlin template that gains a 36th site changes nothing anybody reads. The C++
  count went 38 → 29 → 0 under a person's attention, not under a gate's.
- **The rewriter has no mutation grip at all**, so "it still answers exactly
  these 46" rests entirely on one JVM suite. The C++ round that deleted its
  rewriter had to buy back a positive control it lost with the unit; here there
  is none to lose yet, which is the cheaper time to add one.
- ⚠ Found while measuring: `scripts/gates/w3c-kotlin.sh` described
  `EcmaScriptSemanticsTest` as measuring "the shared 58-case ECMA-262 table" —
  40 cases short, and one more count in prose about THIS table found stale.
  Repaired the way the engine's own KDoc was: by naming the file instead of
  restating its size.

## Landed 2026-08-29 (sixth round): Kotlin reads the emission, and the answer splits in two

`backends/kotlin/tests/src/test/kotlin/com/sce/ecmascript/LoweredEcma262Test.kt`
is the fourth reader of `tests/ecmascript/ecma262_emitted_lua.json` and the
first on this backend. The question the section above left open — does THIS
engine's Lua answer the table when it is handed what the frontend emitted —
now has an answer, and it is not a single number, because the question turned
out to contain a precondition nobody had asserted.

| | count |
|---|---:|
| shared table | 98 |
| emitted expressions that differ from their source | 89 |
| declared `unreachable`: the emission cannot reach the engine intact | 12 |
| **asked** on the lowered route | **86** |
| **answered as ECMA-262 says** | **84** |
| declared `divergences`: reached the engine, still wrong | 2 |

```sh
python3 - <<'PY'
import json
d = json.load(open('tests/ecmascript/kotlin_lowered_ecma262.json'))
s = json.load(open('tests/ecmascript/ecma262_semantics.json'))['cases']
u, v = len(d['unreachable']), len(d['divergences'])
print('table', len(s), 'unreachable', u, 'divergences', v,
      'asked', len(s) - u, 'answered', len(s) - u - v)
PY
grep -rl ecma262_emitted_lua --include='*.kt' --include='*.go' \
    --include='*.py' --include='*.rs' .
```

### The precondition, which is why the answer is two numbers and not one

This backend has NO lowered entry point. Every method on `LuaScriptEngine` —
`evaluateExpr`, `evaluateCondition`, `executeScript`, `executeForeach` — runs
`EcmaScriptToLuaTransformer` over its argument first, because the generated
Kotlin hands it the author's ECMAScript at every site. So a suite that feeds
the emission to `evaluateExpr` is running `rewriter(lowered)`, and wherever
the rewriter changes anything it reports ITS answer while claiming to measure
the frontend's.

That is not a hypothetical. `DomReadSurfaceTest` and `EventDataReadingsTest`
both say in their own comments that the Lua engine "is handed what the
frontend lowered", and neither can be: both go through the same rewriting
entry point. What makes their result mean anything is exactly this property,
and nothing was asserting it. `LoweredEcma262Test` asserts it first, before it
measures anything — the set of cases the rewriter CHANGES is enumerated, not
counted, and held in BOTH directions.

The 12 are destructive rather than cosmetic: a Lua table key `{["k"] = 1}`
comes out `{{"k"} = 1}`, a one-based `a[1]` is shifted again to `a[2]`, and
`table.concat(xs, ",")` becomes `_concat(table, xs, ",")`.

### The 2 divergences are one defect

Both reached the engine unchanged and both raise `ReferenceError`. The emitted
setup assigns without `var` — `Var1 = 1`, `total = 0` — so the session's
declared-identifier guard, which learns names from the rewriter's handling of
`var`, never learns them, and reading the variable afterwards raises an error
the frontend's own scope analysis had already ruled out. A lowered entry point
closes this by construction: it would trust the frontend's scope analysis
instead of re-deriving it from `var`.

Both arrays empty is the terminal state and there is a path to it — the same
seam `IScriptEngine` already carries on the C++ side. Neither array can be
emptied by editing the file: each is held in both directions, so an entry that
stops describing the backend fails as loudly as one that starts.

### The suite was shown red before it was believed

| mutation | what it models | verdict |
|---|---|---|
| `expression` → `source` in `loadEmission` | the suite quietly stops reading the emission and measures the author's ECMAScript instead | RED — 34 undeclared changes, 42 undeclared divergences |
| one more entry in `unreachable` | the exemption list is widened to make a failure go away | RED — 1 case "now pass(es) through the rewriter unchanged" |

The second is the one that matters for an escape hatch: the list cannot be
grown past what it describes, and the count of cases still ASKED carries the
same floor the shared table does, so an exemption list that swallowed the
corpus would leave a suite that passes by asking nothing.

### What this leaves open

- **The declarations were wrong when they were written from reasoning.** The
  first run of this suite refuted its own declaration file in three places —
  a case the rewriter destroys that was not listed, a divergence the file's
  prose NAMED (`total`) and its array did not hold, and a prose count of
  "today's two" over an array of one. Re-derived and repaired in the same
  turn; recorded here because the prose named the missing entry and nothing
  compared the two halves of the same file.
- **The mutation harness has no Gradle runner**, so this suite cannot carry a
  durable casefile the way a `cargo` or `ctest` suite can. The two reds above
  were applied and reverted by hand. `scripts/mutate` grows a third runner or
  this backend keeps buying its control one round at a time.
- **No gate holds "every backend that runs Lua answers the lowered route".**
  Four readers of the emission plus one artifact route (`ecma262-lowered-cpp`)
  is a population five wide, derived from nobody. That gate is what would have
  found this gap in the first place, and it is the next thing to build.

## Landed 2026-08-29 (seventh round): how much of Kotlin's 46 the seam buys back

The round above measured the lowered route across the whole shared table. It
did not answer the question the 46 are actually a list FOR, which is the one
anybody deciding whether to fund the seam asks first: **of the cases this
backend gets wrong today, how many would simply stop existing once translation
moves to build time?**

The two files could not answer it separately. `kotlin_lua_divergences.json`
knows which cases the runtime rewriter gets wrong and nothing about the other
route; `kotlin_lowered_ecma262.json` knows what the other route does and
nothing about which of its cases the rewriter also gets wrong. The answer is
their intersection, and until now nobody had taken it — so the 46 were one
undifferentiated number, and every argument for the seam had to be made on the
size of a list rather than on what closing it would remove.

Each entry now carries a `build_time_frontend` verdict. This is the split:

<!-- sce:kotlin-lowering-dividend — parsed by sce-build/tests/kotlin_lowering_dividend.rs -->

| verdict | entries | what it means for the seam |
|---|---:|---|
| `answers` | 0 | the frontend's Lua reached this backend's Lua engine and ECMA-262's answer came back — **the seam retires these** |
| `diverges` | 0 | reached the engine unchanged and was still answered wrong; the seam does not buy these back, the frontend defect does |
| `unmeasured` | 0 | the lowered route cannot put the case to the engine at all |
| **total** | **0** | every entry is in exactly one verdict |

⚠ **Every cell is zero because the LIST is empty, not because nothing was
measured.** The seventh round's answer to "how much does the seam buy back"
was 44 of 44; the eleventh round below collected the dividend without moving
translation to build time at all — it linked the same frontend into the engine
and had it answer at RUN time, so the 44 left through the `runtime-rewriter`
path instead of through the lowered one. A verdict is a statement about an
entry, so with no entries there is nothing to state, and the total agreeing
with the list's length is what keeps this from reading as "the lane stopped
looking".

```sh
python3 - <<'PY'
import collections, json
d = json.load(open('tests/ecmascript/kotlin_lua_divergences.json'))
low = json.load(open('tests/ecmascript/kotlin_lowered_ecma262.json'))
unreachable = {(c['source'], c['clause']) for c in low['unreachable']}
diverging = {(c['source'], c['clause']) for c in low['divergences']}
def verdict(e):
    k = (e['source'], e['clause'])
    return ('unmeasured' if k in unreachable
            else 'diverges' if k in diverging else 'answers')
def stated(e):
    p = set(e['diverges_on'])
    return ('answers' if p == {'runtime-rewriter'}
            else 'diverges' if p == {'runtime-rewriter', 'build-time-lowering'}
            else 'unreadable')
print(collections.Counter(verdict(e) for e in d['divergences']))
print(collections.Counter(stated(e) for e in d['divergences']))
PY
```

⚠ The second line used to read `e['build_time_frontend']`. That field retired
on 2026-08-29, the day `Language::Kotlin.supports_script_engine_target(Lua)`
became true: `build-time-lowering` became a path this backend HAS, so
`diverges_on` carries the verdict and a separate field for it would have been
a second vocabulary for one fact. `the_field_retires_when_the_seam_opens` is
what said so, on exactly that day, and this snippet is its instruction carried
out — the two counters still have to agree, and the second one now reads the
paths instead of a word beside them.

**44 of 44 — all of them — is what moving Kotlin's translation to build time is
worth on this table.** The number reached that in three steps on one day, and
the steps are the record worth keeping: 39 of 46 when the split was first
measured; 39 of 44 once two of the "seam closes these" cases turned out to be
an engine defect the seam had nothing to do with; 44 of 44 once the lowered
entry point existed and the last five could be asked instead of assumed.

⚠ **Every one of those three numbers was the honest reading at the time**, and
the two that moved did so because something was MEASURED, not because anyone
argued. The 5 were `unmeasured` for exactly one reason — no entry point — and
`unmeasured` was never a synonym for "probably fine".

⚠ **The table read 39 / 2 / 5 over 46 for the first hours of its existence,
and the two that left are the reason the next section exists.** They were
declared as a defect of the LOWERED route. They were not; they were a defect of
this engine's undeclared-variable guard, visible on both routes, and the round
below closed them in both files with one edit. A verdict pointing at a route is
a claim about where a defect lives, and this one was wrong the first time it
was written.

### The verdict is derived, not judged

Nothing types these three words from an opinion.
`sce-build/tests/kotlin_lowering_dividend.rs` re-derives every one of them from
`kotlin_lowered_ecma262.json` and fails on a disagreement in either direction —
and that file is itself held against the running Lua engine, in both
directions, by `LoweredEcma262Test`. So the chain from the number `39` back to
an execution has no prose in it:

```
LuaScriptEngine (running)  →  LoweredEcma262Test (both directions)
  →  kotlin_lowered_ecma262.json  →  kotlin_lowering_dividend (both directions)
  →  build_time_frontend verdicts  →  the tally  →  this table
```

The Rust lane rather than the Kotlin one for the same reason
`ecma262_scoreboard_contract` gives for checking the Kotlin divergence list
from Rust: it needs no JVM, so a verdict that stops describing the tree is
caught on every push instead of only when the Kotlin gate is selected. The
measurement stays where the engine is; what is re-derived here is agreement.

### Unclassified is RED, and the escape hatch is the one that is guarded

`unmeasured` is the only verdict that exempts an entry from the measurement, so
a missing `build_time_frontend` would silently mean exactly it. Every entry
therefore carries one, and an entry that carries none fails — the same rule
`diverges_on` already lives under one key above it. And `unmeasured` cannot be
claimed freely: it is admitted only for a case declared `unreachable` in
`kotlin_lowered_ecma262.json`, where `LoweredEcma262Test` refuses both an
undeclared mangling and a declared one the rewriter has stopped doing.

### This field has a path to zero, and it is not repair

The honest answer to "how do these three counts reach zero" is that they do not
go one at a time. They go together, on the day
`Language::Kotlin.supports_script_engine_target(Lua)` becomes true: at that
point `build-time-lowering` is a path this backend HAS, `diverges_on` can carry
the same fact beside the path it is about, and `build_time_frontend` becomes a
second vocabulary for it. `the_field_retires_when_the_seam_opens` fails on
exactly that day and says what to do — `answers` becomes an entry naming only
`runtime-rewriter`, `diverges` one naming both, `unmeasured` gets a real answer
for the first time. `ecma262_scoreboard_contract` goes red the same day asking
which path each entry is about; the two reds together are the migration.

### Shown red before it was believed

| mutation | what it models | verdict |
|---|---|---|
| one `answers` → `unmeasured` | a verdict is talked into the exempting value | RED — `says "unmeasured", … makes it "answers"` |
| `build_time_frontend` deleted from one entry | the field is quietly optional | RED — "1 entr(ies) … carry no `build_time_frontend`" |
| `measured.answers` 39 → 38 | the tally drifts from the list it summarises | RED — declared vs actual maps differ |
| the seam table's `39` → `38` | the document keeps a number the tree has moved past | RED — stated vs measured maps differ |

The casefile is
`sce-build/tests/mutations/the_lowering_dividend_is_derived_not_declared.cases`,
so this control is bought once rather than by hand each round
— which is what the sixth round could not do, its suite being Gradle's. All six
CAUGHT, each naming the assertion that owns it; the sixth is the one worth
reading, because `the_field_retires_when_the_seam_opens` measures nothing today
and a case that opens the Lua target is the only thing that can show it alive.

### What this leaves open

- **`39` is a number about THIS TABLE.** The shared table asks 98 expressions;
  it does not ask session lifecycle, `setCurrentEvent`, `executeForeach` or the
  `In()` callback, and the Kotlin gate's own comment says why that gap matters
  — a defect there is invisible to both an expression table and a Rhino-only
  suite. So `39/46` prices the seam against expression semantics and nothing
  else, and the residue is unpriced rather than zero.
- **The 5 `unmeasured` have no answer yet, and cannot get one here.** They are
  not "probably fine": the rewriter mangles the frontend's own Lua for them, so
  the only way to learn what the lowered route does is to build the lowered
  route. They are the cases most likely to move the 39 in either direction.
- ~~**The 2 `diverges` are a frontend defect that is still open.**~~ CLOSED by
  the round below, and the diagnosis in this bullet was wrong. It was not the
  frontend's `var`-less setup: it was this engine's guard asking one of the
  three questions its C++ sibling asks. Kept struck through rather than
  deleted, because "naming it in two files is not fixing it" was right about
  the shape and wrong about the cause, and the round after it found the cause
  by comparing the two files rather than by re-reading either.
- **The C++ list keeps no such field, deliberately.**
  `lua_engine_divergences.json` does not need one: C++ HAS both paths, so
  `diverges_on` already carries
  the same fact where it belongs. That asymmetry is the reason this field is
  written as temporary, and `the_field_retires_when_the_seam_opens` is what
  makes "temporary" a claim something can fault.

## Landed 2026-08-29 (eighth round): 46 → 44, and the seam was not what closed them

The round above split the 46 to price the lowered seam. Pricing it meant
reading the two lists side by side for the first time, and the reading found
something neither list could have said alone: **the same two cases,
`13.15.2 compound assignment` (`Var1`) and `14.7.4 the for statement`
(`total`), were declared in BOTH.** In
`kotlin_lua_divergences.json` as runtime-rewriter divergences, and in
`kotlin_lowered_ecma262.json` as the lowered route's only two.

A defect that shows up on both routes is not a defect of either. That is the
whole finding, and it refutes what the previous round wrote down: the lowered
file's own prose blamed the frontend's `var`-less emission and said "a lowered
entry point closes this by construction". It would not have. Both routes go
through the same guard, and the guard was the defect.

### The guard asked two of three questions

`LuaScriptEngine.isUndeclaredSimpleVariable` decided whether a name is one
ECMAScript would call undeclared. Its C++ sibling `SCE::isUndeclaredIdentifier`
(`sce/src/scripting/LuaEngine.cpp`) asks three things and answers "undeclared"
only when all three say no:

| question | C++ | Kotlin, before |
|---|---|---|
| is it a Lua keyword? | 22-name keyword set | a mixed list, keywords and globals together |
| did the session DECLARE it? | `declaredVars` | `declaredVars` |
| is it a live Lua GLOBAL? | `lua_getglobal`, non-nil | **not asked** |

`executeScript` files nothing in `declaredVars` — only `setVariable`, `assign`
and `executeForeach` do — so a document whose `<script>` runs `Var1 = 1` and
then reads `Var1` got `ReferenceError` for a variable sitting in the global
table. Adding the lookup closed both cases in both files with one edit.

**The lookup also retired the list.** Every non-keyword name it carried —
`math`, `In`, `JSON`, `Object`, `_scxml_truthy`, … — is registered as a global
by `registerBuiltins`, so the interpreter already knew them and the second copy
could only drift. What stays is Lua's keyword set, because `true`, `false` and
`nil` are literals rather than globals and a lookup cannot answer for them.

### The witnesses, both directions, in the turn that claimed it

The red came first and it was the lists' own second direction — the one added
so they could SHRINK, firing for the first time:

```
2 declared divergence(s) no longer describe this engine.
  Var1  (13.15.2 compound assignment)
  total  (14.7.4 the for statement)
```

…from `EcmaScriptSemanticsTest`, and the same two from `LoweredEcma262Test`
about the lowered route. Two lists, one edit, one pair of names. Then both
green with the entries removed, on `--no-build-cache --rerun-tasks` so the
result is this run's and not a restored JUnit XML.

Then the other direction, bought by hand because `scripts/mutate` still has no
Gradle runner: neutering the lookup (`return true || !isLuaGlobal(…)`) brings
both suites back red, naming the same pair and saying why —

```
2 expression(s) disagree with ECMA-262 on LuaScriptEngine without being declared to.
  [Var1] failed to evaluate: ReferenceError: Var1 is not defined
  [total] failed to evaluate: ReferenceError: total is not defined
```

That is what makes "the lookup closed them" a measurement rather than a
coincidence of the same round: with it the entries are unremovable, without it
they are undeclared divergences, and nothing else moved between the two runs.

### What this does to the count, and what it does not

`kotlin_lua_divergences.json` is **44 entries**, and
`kotlin_lowered_ecma262.json`'s `divergences` array is **empty** — the first of
its two arrays to reach the terminal state this seam is working toward.
`build_time_frontend` is now 39 `answers`, 0 `diverges`, 5 `unmeasured`.

⚠ **The 5 `unmeasured` are untouched, and the lowered entry point is still the
only thing that can move them.** This round did not build it. What it did was
remove a reason to build it that turned out to be false — two of the seven
cases the seam was credited with closing were never the seam's to close.

### Found while verifying: no CI lane measures this engine above expressions

Changing an engine means running the lane that engine is in, so this round ran
the Kotlin suite on Lua — `./gradlew :sce-kotlin-tests:test
-Psce.script.engine=lua`. **Two cases fail**, and neither is new:
`SendParamPayloadTest` (W3C SCXML 6.2, a repeated `<param>` name loses one of
its values) and `XmlDataIsADomTreeTest` (W3C SCXML B.2, a `<data>` element's
XML does not arrive as a document). An A/B against the guard's previous form
reproduced both unchanged, so they predate this round.

⚠ This paragraph said "361 tests, 2 failing" when it was drafted, and the
count was wrong six hours later — the ninth round added a suite and the total
became 367. The failing NAMES did not move. That is the third stale count in
this document's history and the shortest-lived; totals are now left out of
prose here and the cases are named instead.

They had been invisible because `scripts/gates/w3c-kotlin.sh` runs `rhino` and
`quickjs` only, and stood a sentence in place of running the third: *"Lua is
deliberately absent. It passes this suite (measured: 230 cases)."* The
exclusion is right — running Lua there would assert SCE offers it for the
ECMAScript datamodel, which is what `luaIsNotAnEcmaScriptEngineAndSaysSo`
denies. The sentence was not: the suite holds 361 cases, not 230, and Lua does
not pass it.

So the ECMA-262 table and the two divergence lists cover this engine's
EXPRESSIONS, and nothing covers the 226 generated machines it runs. The repair
is not to widen the array — that makes the conformance claim the gate must not
make — but to give Lua a declared-failure list held in both directions, the way
`kotlin_lua_divergences.json` holds its expressions. That list does not exist,
and the gate now says so where the next person to widen the array will read it.

### The count in prose is gone, because it was wrong within the day

`kotlin_lua_divergences.json` said "the set is the 46 entries below" in three
places. It was 46 for about four hours. Those sentences now name no number and
say why; `build_time_frontend.measured` is the only place a count lives, and
`kotlin_lowering_dividend` re-derives it from the list under it on every push.
That is the fifth time a typed count in this repository outlived its
measurement — and the first time the replacement was already in place when it
happened, so the gate reported it instead of a person noticing.

## Landed 2026-08-29 (ninth round): the Kotlin seam exists, and the last 5 were asked

`ScxmlScriptEngine` now carries the pair this document argued for and C++
landed on 2026-08-28. `com.sce.runtime.ScriptSource` is `(language, text,
source)` with `ecmascript(…)` and `lua(lowered, source)` and deliberately **no
one-argument `lua`**; `ScriptLanguage` spells `ecmascript` / `lua` to match the
manifest's `script_engine_language` wire vocabulary.

| member | who implements it |
|---|---|
| `evaluateExpr(sessionId, ScriptSource)` | **nobody** — it is the contract |
| `executeScript(sessionId, ScriptSource)` | **nobody** — same |
| `doEvaluateExpr` / `doExecuteScript` | the engine |
| `nativeLanguage()` / `acceptsLanguage(…)` | the engine, defaulting to ECMAScript-only |

The two entry points ask `acceptsLanguage` and refuse before any engine code
runs, so a third engine cannot forget it. C++ says that with `non-virtual`;
Kotlin cannot say `final` on an interface member, so
`theLanguageContractIsNotAnEngineDetail` says it instead — and it reads the
SOURCE, because the first version asked `Class.declaredMethods` and every
engine failed it, two of them overriding nothing. Kotlin emits a forwarding
method into each implementing class for an interface member with a body. A
question about what somebody WROTE has to be answered from what somebody wrote.

Inside `LuaScriptEngine` the seam is **one branch** — `loweredTextOf` /
`loweredScriptOf` pass Lua through and send ECMAScript to the transformer — and
everything after it is the tail both routes run, `ReferenceError` built from
`source` while the check runs on the lowered text.

### What the last five answered

`LoweredEcma262Test` now hands the emission over as `ScriptSource.lua(emitted,
authored)`, so the twelve cases the rewriter mangles are asked instead of
exempted. **All 98 cases of the shared table pass on the lowered route**, the
five that `kotlin_lua_divergences.json` had labelled `unmeasured` among them.
The verdict split is therefore **44 `answers`, 0 `diverges`, 0 `unmeasured`**.

The declaration file changed shape to match, because one array had come to
carry two meanings:

| array | means | count |
|---|---|---:|
| `rewriter_mangles` | the rewriter changes the frontend's own Lua for this case, so the two arms are genuinely different code paths | 12 |
| `unaskable` | the lowered route cannot put the case to the engine at all | **0** |
| `divergences` | asked on the lowered route, answered wrong | **0** |

`unreachable` was renamed rather than kept, because after the entry point
landed the name was false: those cases are reachable now. What remains true
about them is that the rewriter mangles them — which is not an exemption, it is
the evidence that the lowered arm is load-bearing.

### The red that proves the arm is real

Green here would be worthless without it, since a suite that quietly went
through the rewriter would also pass. Neutering the branch — `if (false &&
expr.language == ScriptLanguage.Lua)` — turns two suites red at once:

```
lowered `a[1]` must read the author's FIRST element … expected: <10> but was: <20>
6 of the 98 emitted expression(s) … disagree with ECMA-262 …
    a[0], xs.join('-'), xs.sort().join(','), xs.slice(1).join(','),
    xs.reverse().join(','), JSON.parse(JSON.stringify([7,8]))[1]
```

Two things worth reading in that. The control case shifts by exactly one
element, which is the defect a re-rewritten index actually causes. And **6, not
12** — the rewriter changes twelve of the emissions and only six of those
changes alter the answer, so "the rewriter touches it" and "the rewriter breaks
it" are different facts, and `rewriter_mangles` is the first of the two.

### What this does NOT do

- **No template emits `ScriptSource.lua(...)`.** Generated Kotlin still hands
  the engine the author's text at every site, so
  `Language::Kotlin.supports_script_engine_target(Lua)` is still false, the
  divergence list is still one path, and `the_field_retires_when_the_seam_opens`
  is still quiet. C++ was in exactly this position for a day: the seam exists,
  nothing crosses it in anger. The template sites are the next step.
- **The 44 are unchanged as runtime-rewriter divergences.** Every one of them
  is still answered wrong by the engine when it is handed the author's text,
  which is what generated Kotlin still does. What the round measured is that
  all 44 would stop existing the moment the templates cross.
- `executeForeach`, `assign` and `evaluateCondition` take no `ScriptSource`
  yet. They are the sites a template migration would need next, and until they
  move a lowered artifact could not be whole — which is exactly why
  `supports_script_engine_target` counts sites rather than trusting this note.
  **Closed 2026-08-30 — see the next section.**

## Landed 2026-08-30 (tenth round): the last three entry points, and a predicate instead of a count

The three the ninth round named are on the seam. `ScxmlScriptEngine` now
guards five entry points, not two:

| entry point | takes | hook | why it could not be folded into `evaluateExpr` |
|---|---|---|---|
| `evaluateExpr` | `ScriptSource` | `doEvaluateExpr` | — |
| `executeScript` | `ScriptSource` | `doExecuteScript` | — |
| `evaluateCondition` | `ScriptSource` | `doEvaluateCondition` | a guard reaches the rewriter with `ExpressionContext.Guard`, its own cache and its own wrapping; §scxml-5.9 `cond` is also the entry point a generated machine calls most |
| `assign` | `location: String`, `ScriptSource` | `doAssign` | §scxml-5.4: the engine evaluates AND stores, over three assignment paths |
| `executeForeach` | `ScriptSource`, `item`/`index`, body | `doExecuteForeach` | §scxml-4.6: the expression must evaluate to a COLLECTION, and a non-collection is `error.execution` rather than a wrong value |

`location`, `item` and `index` stay `String` deliberately. They are datamodel
locations, not expressions — §scxml-5.4 spells one in the datamodel's terms —
so they carry no language and the shape questions asked of them are asked of
what the author wrote. That is also why `isSystemVariableReference` moved onto
`source()`: §scxml-5.10 names `_event`, not whatever a lowering spells it as,
which is the rule C++ already states above.

### A predicate, because a count in a comment is what went wrong here twice

The interface's own header said the seam existed while three of its five entry
points still rewrote unconditionally, and the table two sections up named five
transform call sites by line number that had all moved. Both are the same
failure: a fact about the tree, written where nothing re-derives it.

`ScriptLanguageSeamTest.everyRewriteIsReachedThroughTheSeamBranch` is the
replacement. Its population is **every engine** — the Kotlin sources under
`backends/kotlin` declaring a `ScxmlScriptEngine`, derived the same way
`theLanguageContractIsNotAnEngineDetail` derives its own — and within each, the
members split at the class-level `fun` declarations. It asserts that **every
member calling the rewriter also asks what language the text is in**. It is not
a list of approved call sites: the failure this repository keeps paying for is
the sixth entry point nobody adds to the list, so an unclassified member is red
rather than exempt. Two floors — the engine count and the rewriter calls the
branch itself holds — keep a sweep that stopped finding the tree from passing
on an empty population.

⚠ The obvious wider population, "every file that calls the rewriter", is wrong
and was measured to be on 2026-08-30: `LoweredEcma262Test` calls it deliberately,
because comparing the rewritten arm against the lowered one IS its measurement.
Deriving the population as *engines* keeps that file out by what it is rather
than by an exemption entry — and an exemption entry is the shape this gate
exists to refuse.

### Witnesses, all re-made in the round that claims them

Green: `./gradlew :sce-kotlin-tests:test --tests
"com.sce.ecmascript.ScriptLanguageSeamTest"` — 10 cases, 10 passed, four of
them new (guard arm, assign arm, foreach arm, the predicate).

Red, three breaks, each attributable and each leaving the cases it does not
touch green:

| break | red |
|---|---|
| `doEvaluateCondition` calls `transformer.transform` directly again | the guard arm **and** the predicate, which names `doEvaluateCondition` |
| `evaluateCondition(…, ScriptSource)` drops `refuseUnlessAccepted` | the refusal case: `Rhino.evaluateCondition … it said: Guard evaluation failed: 'a[0]'` — Rhino silently evaluated the SOURCE half instead of refusing |
| `loweredTextOf`'s branch neutered to `… && false` | all four expression arms — `evaluateExpr`, guard, assign, foreach — and **not** the script arm, which goes through `loweredScriptOf` |

The third one is worth keeping: the predicate stays GREEN under it, because the
branch is still written. That is the honest limit of a source-reading gate, and
it is why the behavioural arms are not redundant with it — one names the member
that has no arm, the others name the arm that stopped working.

### ~~What this still does NOT do~~ — closed the same day

~~No template emits `ScriptSource.lua(...)`~~. They do, as of the round below.
What that round found is that the two lists which said so had been reading the
templates through a hole.

## Landed 2026-08-29 (eighth round): the Kotlin templates cross, and the scan that said C++ had finished was reading past three shapes

`sce-codegen generate … -l kotlin --script-engine lua` now succeeds, and the
generated Kotlin carries the pair at every site:

```kotlin
safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("_scxml_eq(Var1, \"ab\")", "Var1 == 'ab'"))
executeAssign(com.sce.runtime.ScriptSource.lua("Var1", "Var1"),
              com.sce.runtime.ScriptSource.lua("(\"a\" .. \"b\")", "'a' + 'b'"))
engineDD.evaluateExpr(sidDD, com.sce.runtime.ScriptSource.lua("(_scxml_tostring(Var1) .. \"z\")", "Var1 + 'z'"))
```

Re-derive with the loop the section above already carries — every backend now
answers `rc=0`:

```sh
cargo build --bin sce-codegen --features cli -p sce-build
for L in kotlin cpp rust go python c11; do
  ./target/debug/sce-codegen generate tests/integration/test_thermostat.scxml \
      -o /tmp/seam -l "$L" --script-engine lua >/dev/null 2>&1; echo "$L rc=$?"
done
```

### The three shapes the migration scan could not see

The Kotlin count was **6 unmigrated + 29 unclassified**. Closing it needed the
templates AND the scan, because the scan had three blind spots — and each one
was hiding a real hand-off in the backend the lists reported as **finished**.

| shape | what it hid | measured |
|---|---|---|
| a `{% set %}` that binds the author's text to a name emitted elsewhere | C++ `entry_exit_actions.jinja2` laundered `<donedata>`'s content into `DoneDataHelper::evaluateContent` | `--script-engine lua` emitted `ScriptSource::lua("(\"a\" .. \"b\")", …)` on one line and the author's `Var1 + 'z'` on another |
| a C++ RAW string literal, `R"( … )"`, read as prose because `(` and `)` are "other text in the quotes" | `<data>`'s inline content, at three sites, handed to `evaluateExpression` and to `DataModelInitHelper::initializeVariable` | the `expr` arm emitted `ScriptSource::ecmascript("'a' + 'b'")` and the `<content>` arm beside it emitted a bare `R"(…)"` — **target-independent**, so `--script-engine lua` changed it not at all |
| a callee more than three lines above its argument, or on the other side of a `{% else %}` | Kotlin's `HostSendRequest(content = …)`; and, in the other direction, the static `println` arm read as a hand-off | the window is now 6 lines and stops at a branch boundary it does not also close |

The first two are the same failure the seam document already recorded one
layer down, and they are why "both scan lists are empty" was not the finished
migration it read as. **The C++ side was not closed on 2026-08-28; it was
closed on 2026-08-29, by a scan that could see these.**

`DataModelInitHelper::initializeVariable` took the repair `initializeVariableFromExpr`
had already had: a `ScriptSource` rather than a `std::string`. §scxml-B-2's XML
and string readings are answered from `source()` — whether the children are a
document is a question about the document — and only the expression reading
uses the lowered half. `initializeVariableFromSrc` passes
`ScriptSource::ecmascript`, because a file read at run time has no build-time
half to pair with; that is the run-time seat this seam keeps on purpose.

### What crossed on the Kotlin side, and the one thing that did not

Twelve templates. Every `cond` through `to_script_source_guard`, every `expr`
through `to_script_source_expr`, `<script>` through `to_script_source_script`,
`<assign>` through `to_script_source_location` + `to_script_source_expr` /
`_assign_content`, `<send>`'s inline `<content>` through
`to_script_source_data_content`. The generated helpers took `ScriptSource`
parameters to match — `safeEvaluateGuard`, `executeAssign`,
`executeScriptBlock`, `evaluateSendContent`.

⚠ `<assign>`'s LOCATION now carries a language too, and the KDoc that said it
should not was wrong for a measurable reason: `LuaScriptEngine.doAssign`
splices the location into `"$location = $expr"` and runs the result as Lua, so
`Var1[0]` written in ECMAScript addressed the wrong element of a 1-based
table. C++ had already said so — `to_script_source_location` exists because
`AssignmentExecutionHelper` glues the location in front of `=`.

The one that did NOT cross is `<data>`'s inline `<content>`, and it is an
adjudication rather than a gap: the Kotlin site's destination is
`ScxmlScriptEngine.parseDataValue`, whose ladder on the engine a Lua-target
artifact actually runs (`LuaScriptEngine.parseDataValueInternal`) is
§scxml-B-2-8-1's three readings and no fourth — XML, JSON, space-normalized
string. Nothing there evaluates in the engine's language, so lowering it would
answer a question nobody asks. It is in `INERT_DESTINATIONS` with that
reasoning, not on an exemption list.

⚠ Found while adjudicating it, and NOT repaired here: `RhinoScriptEngine` and
`QuickJSScriptEngine` still carry the expression rung
(*"Step 2: Try as JS expression"*) that the 2026-08-17 round removed from the
other four. That is a §scxml-B-2-8-1 conformance defect in two engines, not a
language-seam question — they are ECMAScript engines, so the text they
evaluate is the author's own either way. It stays open and stated rather than
folded into this axis.

## Landed 2026-08-30 (eleventh round): the Kotlin engine reaches the frontend, and its list empties

`kotlin_lua_divergences.json` holds **nothing**. Re-derive rather than trusting
this sentence — it is the one number this axis exists to drive, and it has been
restated wrongly here before:

```sh
python3 -c "import json; print(len(json.load(open('tests/ecmascript/kotlin_lua_divergences.json'))['divergences']))"
```

### What was actually done, because it was not the thing the milestone named

The plan of record for this backend was: move the Kotlin templates across the
seam, then make `Language::Kotlin.lowers_expressions_at_build_time` true, so
generated Kotlin hands its engine Lua and the rewriter is never reached. The
first half landed in the eighth round. The second half was measured this round
and **it does not close a single entry**, for the same reason the C++ round
recorded on 2026-08-29 and for the same measurable cause:

```sh
grep -c "Generated" backends/kotlin/tests/src/test/kotlin/com/sce/ecmascript/EcmaScriptSemanticsTest.kt   # 0
```

The suite that holds the `runtime-rewriter` path reaches the engine by a DIRECT
evaluate. It has no generated machine, so no codegen default can move one of its
answers. Flipping that predicate would have changed what the manifest says and
what the templates emit, and left all 44 entries exactly where they were —
while `measurable_paths` would have stopped admitting the path they are declared
on, and the repair a reader takes from that red is to drop the path and delete
the entries. **A zero reached that way would be a false one**, and the two-sided
suite is the only thing that would have caught it, by going red for a reason
that names neither the cause nor the fix.

So the work went to the engine instead, which is where the C++ close happened
too: `sce-build`'s ECMAScript frontend is now linked into the Kotlin Lua engine
and answers the author's text at run time.

### The link is the same artifact, not a second one

`backends/kotlin/lua/src/main/cpp/CMakeLists.txt` includes
`cmake/SCEBuildLowering.cmake` — the module `sce/CMakeLists.txt` already uses —
and links `SCE::Lowering` into `sce_lua_jni`, the shared library this backend's
Lua already comes through. Two backends, one staticlib, one frontend: a
difference between what `datamodel="ecmascript"` means on C++ and on Kotlin can
no longer be a difference in which parser answered.

`lowering_jni.cpp` is the ONE translation unit on this backend that names the C
surface, mirroring `LoweringScope.cpp` on the other, and
`LoweringScope.kt`/`SceLoweringNative.kt` are the class everything above talks
to instead.

**The link is not optional, and that is the substance rather than caution.** A
library built without the frontend would answer the shared table from the
rewriter alone — a SECOND set of answers under one engine name, only one of
which the divergence list describes. There is no `#ifdef` on this side for the
same reason: one artifact, one answer.

### The scope is what decides how much gets answered, and every door feeds it

The frontend refuses any expression naming something its scope has not been
told about, so a session that declares nothing gets only the closed expressions
answered. Every door that puts a name in a session's namespace therefore goes
through `offerToScope`:

| door | clause |
|---|---|
| `setVariable` | §scxml-5.3, a `<data id>` |
| `setupSystemVariables` | §scxml-5.10, `_event` and the rest — without which every `cond="_event.data.x"` in the corpus is refused |
| `doAssign` | §scxml-5.4, the target's root name, taken from the AUTHOR'S spelling |
| `executeForeach` | §scxml-4.6, `item` and `index` |
| `doExecuteScript` | §scxml-5.8, `declareChunk` on a chunk that ran |

⚠ One door is NOT fed, and it is stated rather than hidden: a Lua-language
`<script>` introduces names the frontend's parser cannot be asked about, so an
ECMAScript expression naming one of them is refused and falls back to the
rewriter. C++ closes that by sweeping Lua's own global table
(`offerDocumentGlobalsToScope`), which needs a name for every global this engine
installs and is its own piece of work. Codegen does not produce the mixture —
an artifact is generated for one language and hands over that language
everywhere — so it is a residue of the engine accepting both, not a case a
document reaches.

### 44 to 0, and the A/B that says so

Measured in the turn that claims it, on this machine:

| `./gradlew :sce-kotlin-tests:test -Psce.script.engine=lua` | tests | failures |
|---|---:|---:|
| with the frontend | 371 | **0** |
| with `LoweringScope`'s four `lower*` neutered to `null` | 371 | **3** |

The three are the whole witness, and each is attributable:

- `EcmaScriptSemanticsTest.theLuaEngineDivergesExactlyWhereItIsDeclaredTo` —
  `44 expression(s) disagree with ECMA-262 on LuaScriptEngine`, which is the
  list this round emptied, arriving back in one piece;
- `SendParamPayloadTest.sendParamsReachEventDataFromChildAndInternalQueue` and
  `XmlDataIsADomTreeTest.aDataElementsXmlIsADomTreeTheDocumentCanWalk` — **the
  two cases `scripts/gates/w3c-kotlin.sh` names in its own comment** as failing
  under `-Psce.script.engine=lua`. They were not part of the plan and they close
  with it: the frontend answers what the rewriter could not, above the
  expression level as well as inside it.

That second bullet is why the A/B was run rather than only the forward
direction. The forward run says "371 green"; only the neutered one says the
frontend is what makes them green, and it names the same two cases a comment
written by hand on a different day already knew about.

### Empty is not retired, and the two claims are kept apart

`EcmaScriptToLuaTransformer` is still compiled in and is still the FALLBACK
behind every lowering entry point. What an empty list says is that the frontend
answers the 98 cases of the shared table; what retirement would say is that
there is no second answer left. An expression outside that table which the
frontend refuses is still rewritten rather than refused (§scxml-5.9.1), which
is the state C++ passed through between its 23-to-12 round and
`retire-rewriter`.

`EcmaScriptSemanticsTest` holds the engine's own KDoc to that distinction, and
the check is DERIVED so it retires itself: while the engine's class body still
calls `transformer.`, its documentation must name `EcmaScriptToLuaTransformer`.
The day the fallback goes, the assertion stops asking rather than having to be
remembered.

### The floor that forbade the finish line

`assertTrue(declared.isNotEmpty(), …)` stood in that suite to catch a list it
had stopped reading — a real failure — but it also failed on a list with
nothing left in it, which is the terminal state the whole axis is working
towards. **A counter whose zero is forbidden is not a counter.** It is gone,
the same way and for the same reason the C++ suite's went, and what still fails
if nothing opens the list is `ecma262_scoreboard_contract`'s `readers_of`
sweep.

### What this leaves open

- **`kotlin-retire-rewriter`.** Delete the fallback, and
  `LuaScriptEngine.acceptsLanguage(ECMAScript)` becomes a claim about the
  frontend being linked rather than an unconditional `true`. The 2262-line C++
  unit left its tree the day its row closed; the Kotlin one is 1175 lines and
  still there.
- **The Lua-language `<script>` door**, above.
- **`Language::Kotlin.lowers_expressions_at_build_time` is still false**, and
  this round is the measurement that says flipping it is a separate question
  from the divergence count rather than the way to close it. What it would
  change is the DEFAULT artifact and the committed Kotlin tree, which
  `w3c-kotlin` runs on Rhino and QuickJS — two engines that refuse Lua. So the
  flip needs the suite to generate per engine first, which is also what would
  let `lua` join `KOTLIN_ENGINES` and close that gate's own standing note.
  ✅ **The per-engine generation landed 2026-08-30 (the round below), and `lua`
  joined.** The array is `KOTLIN_ENGINE_PAIRS` now and the flip is no longer
  blocked by this gate; what the flip still needs is its own round.

### The one thing that made the flip unsafe, removed — and it changes nothing yet

`ecma262_scoreboard_contract`'s `measurable_paths` derived `runtime-rewriter`
from `default_script_engine_target` — a POLICY — and now derives it from
`supports_script_engine_target(EcmaScript)`, a CAPABILITY. That is what closes
the false route to zero described above: the path a divergence entry declares
must not vanish because a codegen default moved, since moving one changes no
answer either suite gives.

`supports_script_engine_target`'s ECMAScript arm was a flat `false` under the
comment *"a backend that lowers has no arm that emits the author's source"*.
The pair filters refuted that: a backend whose guard site is
`to_script_source_guard` emits `ScriptSource::ecmascript(...)` or
`ScriptSource::lua(...)` by the run's selection, so C++ and Kotlin have BOTH
arms. It is now `!lowers_expressions_at_build_time()`.

⚠⚠ **Measured, and stated rather than implied: that edit is a NO-OP in this
tree.** Forcing the arm back to `false` leaves every lane green, because the
`target == default` line above it already answers `true` for the two backends
that have both arms. Its value is entirely in what it makes SURVIVE — the
Kotlin default flip — and it will not be observable until that flip happens.
Recorded here because a change nothing can fault is a change a reader should
be told is unfaultable, not one to describe as a repair.

⚠⚠⚠ **What the round DID buy is a gate, and its first form was tautological.**
`the_targets_a_backend_claims_are_the_targets_it_generates_for` first compared
the predicate against whether the CLI accepted the flag — and the CLI derives
its refusal from that same predicate. Measured 2026-08-30 by forcing the arm to
`true`: **all twelve combinations were accepted**, and only the case's
non-empty-refusals floor noticed. It now reads the ARTIFACT for
`_scxml_truthy(`, which is the independent half, and the same break fails on
the substance instead:

```
`rust` accepted `--script-engine ecmascript` and emitted an artifact that
carries `_scxml_truthy(_event.data)`.
```

## Landed 2026-08-30 (twelfth round): the conformance gate generates per engine, and `lua` joined it

`scripts/gate w3c-kotlin` ran two engines against one tree. It now runs four
`engine:language` **pairs**, each against the tree that engine can read:

```sh
KOTLIN_ENGINE_PAIRS=(rhino:ecmascript quickjs:ecmascript lua:ecmascript lua:lua)
```

The row that did not exist before is `lua:lua` — the build-time lowering route,
measured above the expression level for the first time. `lua:ecmascript` is the
run-time route the gate already had, now labelled as the one it is rather than
described as covering both.

### Why an engine stopped identifying a run

This backend emits machines for two script-engine languages and the committed
tree can hold only one. Handing the other engine that tree is not a weaker
measurement, it is a broken one: `ScxmlScriptEngine` refuses a language it does
not accept, so `rhino` over lowered Lua is a suite of refusals. Two halves lift
that:

- **`sce-codegen generate-w3c --script-engine <LANG>`** — the batch counterpart
  of the single-document flag, resolved through the same
  `supports_script_engine_target` refusal. `KotlinBackend` and `CppBackend`
  carry the run's selection and call `generate_*_for_engine`; the other four
  backends have one arm each, and it is that refusal — not silence here — that
  turns asking them for the other one into an error.
- **`-Psce.generated.overlay=<tree>`** — the Kotlin test build compiles that
  tree's `com/sce/generated` and excludes the committed one. Only the machines
  move; the generated JUnit classes do not vary with the language, and the gate
  holds that to a `cmp` per class rather than to this sentence.

### What the gate derives instead of declaring

`COMMITTED_LANGUAGE` is read from the manifest of a probe generation
(`script_engine_language`), never written down. A literal would be correct
until the day `default_script_engine_target()` flips — the day this gate has to
be right — and silently wrong after it.
`the_kotlin_gate_runs_every_engine_it_claims` fails on a literal assignment.

⚠ **The first attempt asked the TREE and would have been wrong.** Counting
files carrying `ScriptSource.lua(` against `ScriptSource.ecmascript(` reads
**159 against 159 on the same tree**, and the gate would have refused it as
"mixed". A generated machine emits BOTH arms of the run-time helper that
re-wraps a `ScriptSource` it was handed — `evaluateSendContent` switches on
`source.language` — so both spellings appear in every machine that has one.
What varies with the selection is the call site carrying the author's
expression, and telling those apart by grep is a parse of Kotlin.

### The population claim splits along the seam, and each half is asserted where its answer lives

`KOTLIN_ENGINE_PAIRS` makes a POPULATION claim — "these are the ways a
generated machine can reach an engine in this backend" — and until this round
nothing checked it. The array said `(rhino quickjs)` for months while a third
engine shipped, and the omission survived every push because the only thing
that could have contradicted it was the same array.

The claim has two halves, and neither implies the other:

- **The generator half** — every language this backend can EMIT an artifact
  for has a row. Answered in Rust by
  `the_kotlin_gate_runs_every_language_the_generator_can_emit`, which sweeps
  `ScriptEngineTarget::ALL` through
  `Language::Kotlin.supports_script_engine_target`. A language it says yes to
  with no row is an artifact SCE ships and no lane executes.
- **The engine half** — every language each engine will ACCEPT has a row.
  Answered on the JVM by `GateEnginePairsTest`
  (`W3CTestBase.KNOWN_ENGINES` × `ScxmlScriptEngine.acceptsLanguage`), because
  it is the engines that answer. An engine added to the suite, or an existing
  one that gains an adapter, turns the array red rather than going unmeasured.

An emittable artifact nothing runs and a running engine nothing emits for are
different holes, which is why these are two assertions rather than one
restated twice.

⚠ **Containment on the generator side, equality on the engine side**, and the
asymmetry is deliberate. A row naming a language the backend cannot emit fails
LOUDLY the first time the gate runs — `generate-w3c` refuses through the same
`supports_script_engine_target` call — so nothing needs asserting for that
direction in Rust. On the JVM there is no such refusal for a pair that stopped
being supported, so equality is what keeps the minutes honest.

⚠⚠ **The escape hatch is closed too.** The generator-side check enumerates
through `ScriptEngineTarget::ALL`, so shrinking that constant would leave the
comparison running, passing, and asking about one language — a gate that
cannot be wrong rather than a gate that is right.
`the_target_population_is_the_wire_vocabulary` holds `ALL` to
`SCRIPT_ENGINE_LANGUAGES`, which is itself pinned to the manifest schema's
`enum`: a third target cannot reach the wire without reaching the sweeps, and
cannot be added to the sweeps without the schema admitting it. Both directions
are mutation cases in `kotlin_engine_selection.cases`, and the repairs they
point at are opposite — one says "add the row", the other says "the population
is not the array's to narrow".

⚠⚠ **The predicate was inert when written, and the mutation is what showed
it.** Deleting the `lua:lua` row left `:sce-kotlin-tests:test` reporting BUILD
SUCCESSFUL in 500 ms with no test run: Gradle cannot see a test reading a file
off disk, so the gate was not an input of the test task. The same shape the
shared ECMA-262 tables already carried, for the same reason. Declared now, and
`the_kotlin_suite_reruns_when_the_gate_it_reads_changes` fails if the
declaration goes — a suite cannot observe its own staleness, because the run
that would report it is the run that does not happen.

### Two gate defects found while writing it

- `sce_gate_fail` was reachable from inside a `$( … )`. It is an `exit`, so it
  would have ended the subshell and let the gate carry on past a tree it had
  just refused. The helper returns through a variable now.
- A `[ … ] && …` loop body made the loop's status that of its LAST iteration
  under `set -e`, which for today's array is `lua:lua` against a committed
  `ecmascript` — the gate would have ended silently, with no message, exactly
  as arranged.

### The refusal's Kotlin witness was vacuous, and a mutation is what showed it

`supports_script_engine_target(Lua)` is DEFINED as "both scan lists are
empty". The Kotlin case guarding it asserted that the two AGREE:

```rust
assert_eq!(
    Language::Kotlin.supports_script_engine_target(ScriptEngineTarget::Lua),
    remaining.is_empty() && unknown.is_empty(),
);
```

That holds however the scan answers — see the site and both sides go false,
miss it and both go true. It is `A == A` against the scan, and it earns its
place only for the `target == default` shortcut inside the refusal, which is a
different question.

Measured: the mutation case *"the callee sits further away than the window
reached"* — which restores an unmigrated Kotlin `<send>` site whose callee sits
above its argument, the shape the scan lost once already — was SURVIVED on CI
while its two C++ siblings were CAUGHT. The difference was that C++ had a
POSITIVE assertion
(`the_cpp_migration_is_complete_and_the_lua_target_is_offered`) and Kotlin had
only the biconditional. The Kotlin twin
(`the_kotlin_migration_is_complete_and_the_lua_target_is_offered`) turned the
same case CAUGHT the moment it existed.

⚠ The general shape, worth carrying past this seam: **when an oracle compares
two values and one is derived from the other, it cannot fail.** A refusal built
on a scan needs a witness that asserts the scan's ANSWER, not its consistency.

### What this leaves open

- **The flip itself.** `lowers_expressions_at_build_time` is still false. This
  gate no longer blocks it — the pairs keep meaning the same thing across it,
  and `COMMITTED_LANGUAGE` follows it — but nothing else was checked for the
  flip, and `EcmaScriptToLuaTransformer`'s retirement is a separate row.
- **`ecma262-lowered-kotlin`.** `lua:lua` runs the generated machines over
  lowered artifacts; it does not run the 98-case shared table over one. That is
  still `LoweredEcma262Test` feeding the engine, not an artifact.
- **C++ has the flag and no pair.** `generate-w3c --script-engine` reaches
  `CppBackend`, and `w3c-cpp` runs one language. The same argument applies
  there and was not made this round.

## Landed 2026-08-30 (thirteenth round): a Lua-lowered KOTLIN artifact is compiled and RUN

The bullet the round above left open — *"`ecma262-lowered-kotlin`. `lua:lua`
runs the generated machines over lowered artifacts; it does not run the 98-case
shared table over one"* — is closed. `scripts/gate ecma262-lowered-kotlin`
(`.github/workflows/ecma262-lowered-kotlin.yml`,
`backends/kotlin/lowered-ecma262/`, Gradle `:sce-kotlin-lowered-ecma262:test`)
generates ONE document twice, compiles both Kotlin artifacts into one unit, and
drives each to its final state.

**What it adds that nothing else had.** Kotlin's seam has been open since the
eleventh round: `ScriptSource.lua`, a lowered arm on `LuaScriptEngine`, and a
frontend that answers the shared table. What no lane did was run an ARTIFACT
through it. `EcmaScriptSemanticsTest` hands the engine the author's ECMAScript;
`LoweredEcma262Test` hands it Lua read out of a committed table. Both call an
engine entry point with text a test chose, so *"the frontend answers all 98
cases"* stayed a statement about `sce-build` plus somebody else's Lua.

**Which side of the seam it measures: BUILD-TIME lowering**, and it holds that
path's CONTRACT rather than merely measuring it.
`tests/ecmascript/kotlin_lua_divergences.json` says per entry which routes
answer a case differently, and the suite holds `build-time-lowering` in BOTH
directions: an undeclared wrong answer is red, AND a declared case answered
correctly is red. The second direction is what lets that list reach zero
honestly rather than by nobody asking.

**One expander, one population, two backends.** The document is generated by
`tools/generate_lowered_ecma262_fixture.py` — the same one the C++ lane uses —
from `ecma262_semantics.json` in full, and is never committed. A committed copy
would be a second population free to fall behind the table.

### The defect it found on its first run

The lane was not cosmetic. **The generated Kotlin evaluates a transition's
`cond` TWICE**: once in `processNull<State>()` to choose the target, and again
in `executeTransitionActions` to choose which arm's executable content to run.
For a pure guard that is invisible. For a guard with a SIDE EFFECT it is wrong:
`++v == 2` runs `++v` and reads 2 (the guard holds, the target is chosen), then
runs `++v` again and reads 3 (the guard does not hold), so the UNGUARDED arm's
content runs and the fixture records *"the guard did not hold"* for a
transition that was in fact taken.

Measured in the emitted machine — the same
`safeEvaluateGuard(ScriptSource.lua("_scxml_eq((function() v = _scxml_tonumber(v) + 1 return v end)(), 2)", "++v == 2"))`
appears in `processNullD43()` and in `executeTransitionActions`.

**NOT a language defect, and the evidence is the control.** Both routes into
the engine record it wrongly — and a divergence of a LOWERING route cannot show
up on the route that does no lowering. So it is declared in a file of its own,
`tests/ecmascript/kotlin_lowered_artifact_defects.json`, rather than folded into
the divergence list: folding it there would make that list non-empty and would
say the frontend gets a case wrong, which is false, and would send the next
reader to the wrong half of the seam. The fix is the one C++ already has — a
`transitionIndex` on the transition result, so action dispatch switches on the
transition SELECTED instead of re-deciding. That is a runtime signature plus two
templates plus a regeneration of every committed Kotlin tree, which is why it is
a round of its own rather than a fix smuggled into the round that found it.

### Two artifacts, and why the suite alone cannot tell them apart

Both artifacts answer the same 98 cases and — with both divergence arrays empty
— answer them the SAME WAY. So the suite cannot distinguish a real pair from a
subject compared against itself. What can is the shape of the emitted machines,
and the GATE is where that is read.

⚠ **The count is not `== 0` on the control**, which was the first spelling and
was measured wrong the same day. A generated machine emits BOTH arms of the
helper that re-wraps a `ScriptSource` it was handed, so ONE occurrence of each
spelling appears in every machine whatever it was generated for. `w3c-kotlin`
recorded the same trap from the other side, where counting one tree read 159
against 159. What is asserted instead is the MIRROR — what the subject spells
`lua` the control spells `ecmascript`, one call site for one call site — plus a
FLOOR, asserted separately because the mirror alone is satisfied by the failure
it exists to catch: a `--script-engine lua` that accepted the flag and emitted
the default anyway produces two IDENTICAL machines, and identical machines
mirror each other perfectly.

### The exclusion list, held from two sources

`kotlin_lowered_artifact_defects.json` is the one thing this lane introduced
that can be green over a wrong artifact, so it carries three separate limits:

- **Both directions.** An undeclared wrong answer is red; an entry the artifact
  now answers CORRECTLY is red too. Without the second, a list can only grow.
- **Two ceilings, in two languages.** The suite has `MAX_DEFECTS = 3` — and that
  constant lives in the same file as the assertion reading it, so raising the
  constant makes the assertion agree with you. The gate keeps its own ceiling
  over the `codegen-defects=` count the suite PRINTED. Neither implies the
  other: one counts entries in the file, the other counts entries that actually
  excused a case in the population.
- **No phantom entries.** An entry naming no case in the shared table can never
  be answered correctly and so can never be removed. The suite reds on one.

### Measured in the turn that landed it

- `GRADLE_OPTS="-Dorg.gradle.caching=false" scripts/gate ecma262-lowered-kotlin`
  **rc=0**, with `Task :sce-kotlin-lowered-ecma262:test` carrying **no suffix** —
  the run happened rather than being answered FROM-CACHE or UP-TO-DATE.
- census, printed by the GREEN run so a green run states what it measured:
  `LoweredEcma262Kotlin census: cases=98 declared=0 codegen-defects=1
  lowered-control-refused=<unevaluated> lowered-control-evaluable=1
  source-control-refused=<unevaluated> source-control-evaluable=1`
- the pair: **458** `ScriptSource.lua(...)` call sites in the subject against
  **458** `ScriptSource.ecmascript(...)` in the control, the helper's other arm
  once in each.
- from `:sce-kotlin-lowered-ecma262:clean`, every task EXECUTED:
  `generateLoweredFixture`, `generateEcma262Source`, `generateEcma262Lowered`,
  `compileKotlin`, `compileTestKotlin`, `test`.
- **red witness ①** (hand A/B — this backend has no gradle runner in
  `scripts/mutate`): emptying the defect list makes the gate **rc=1**, and BOTH
  routes name `[++v == 2] (13.4.4 prefix increment yields the new value) —
  expected {"bool":true}, read Value 2`. That both routes name it is the
  measurement behind calling it a code-generation defect rather than a
  divergence.
- **red witness ②**: declaring a case the artifact answers correctly (`!a`,
  12.5.9) makes the gate **rc=1** with *"The defect is fixed: remove the
  entry."* — so the list has a path to zero rather than only a path upward.
- **red witness ③** (Rust): `scripts/mutate
  sce-build/tests/mutations/ecma262_lowered_kotlin.cases` — **7/7 CAUGHT**,
  baseline 30/0.
- `GRADLE_OPTS="-Dorg.gradle.caching=false" scripts/gate w3c-kotlin` **rc=0**,
  4 pairs x 373 = **1492 PASSED**, `committed tree unchanged`, the test task run
  four times with no suffix. That lane is in the blast radius because the ~50
  lines of JDK resolution it carried moved to `scripts/gates/lib.sh`, where two
  gates now share one answer to "which JVM is this".

### ⚠ Two numbers this round re-derived, and both were wrong in the permissive direction

**The gate's `cost_s`.** `gate_registry.py` first carried `2` with the claim
*"measured … INCLUDING a `:sce-kotlin-lowered-ecma262:clean` first"*. Re-measured
here: **11s and 4s from clean, 1s and 1s warm.** 2 was a warm reading wearing a
cold justification. The row now declares **11**, and states its basis rather than
inheriting it: the table defends "warm" as *"a push happens on a tree the
developer has just built"*, and nothing but this gate builds this module, so a
warm reading here means "this gate ran before". The state a push actually finds
it in is the one its own `paths:` filter selects for — a change under
`tools/codegen/templates/**` or `sce-build/src/**` invalidates the generation and
the compile.

**The lane's supersession median.** `ci_supersession_policy` first carried `1.4`
from the same local wall clock. That is the permissive direction:
`must_not_supersede` fires only ABOVE the push gap, so an optimistic median is
what blesses `cancel-in-progress: true`. It now carries **4.8**, derived from
HOSTED runs of the nearest lane that has any — the `Kotlin W3C Tests` job of
`w3c-tests.yml`, at 3.4, 4.8 and 3.4 minutes over its last three successes. That
job builds the same Kotlin runtime and the same Lua JNI library plus QuickJS and
Rhino, and runs 4 x 373 cases where this lane runs 98, so its worst reading
bounds this lane from above.

### ⚠⚠ A scanner that matched its own documentation

Adding the gate turned `every_job_that_runs_the_pinned_validator_installs_it`
red, and the cause was not the lane. `reach_of` decides whether a job runs the
mutation corpus's oracles with `body.contains("sce-build/tests/mutations")` over
the whole gate script — comments included. The new gate's ceiling block
*mentions* its casefile by path in a comment, so the scan concluded the lane runs
`gate_registry_contract` and demanded it install the pinned `mnemosyne-cli`.

Fixed at the scanner: the match now runs over `code_lines(&body)`. The two gates
that really do reach the corpus (`mutation-rounds.sh`, `mutation-cases.sh`) both
assign the path to `CORPUS=`, so the code reading keeps them and drops the
sentence. The general shape is one this repository has already recorded from the
other side: **a scanner has to strip comments before it matches**, and a
false positive is only the cheap half of that — the expensive half is a scan that
matches a comment saying the opposite of what the code does.

### What this leaves open

- **The flip itself.** `Language::Kotlin.default_script_engine_target()` is
  still ECMAScript, and `EcmaScriptToLuaTransformer` still exists. What this
  round removes is the reason to distrust the flip on Kotlin: the lowered
  artifact now answers the shared table under EXECUTION, not by assertion.
- **The `cond`-twice defect.** One entry in
  `kotlin_lowered_artifact_defects.json`, with its cause and its fix, and the
  suite reds when it is fixed and not removed.
- **C++ still has the flag and no pair**, unchanged by this round.
- **This lane has no hosted history.** Its supersession row is an upper bound
  borrowed from a sibling; replace it with the lane's own median once it has
  runs to take one over.
