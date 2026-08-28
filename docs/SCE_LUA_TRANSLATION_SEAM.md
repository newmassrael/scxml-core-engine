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

### The 23rd, and why the control is a floor rather than "all of them"

`typeof missingVariable !== 'undefined'` (d14) comes out CORRECT through the
source-passing artifact. Not because the rewriter gained it: §scxml-5.9.1 makes
a `cond` the engine refused evaluate to FALSE, and false is this entry's
ECMA-262 answer, so the refusal and the language coincide. Confirmed on the same
tree — `ecmascript_semantics_test` reports it as *"failed to evaluate as a
condition"*, not as a wrong answer.

That is why `MIN_SOURCE_DIVERGENCES = 22` is a floor. The allowance is not an
exemption, though: `EveryEntryTheSourceArtifactGetsRightIsAnEngineRefusal`
asserts that an entry the control answers correctly must be one the engine
REFUSED, so an entry the rewriter has actually repaired is red and the list must
shrink. The probe that answers "refused?" is measured, and it has its own
control on the same run (it must report both outcomes at least once, or it is
not distinguishing anything).

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
- The C++ **Interpreter** has no build step at all, so it cannot receive
  build-time-translated text by this route. `sce-build`'s cdylib is the
  candidate answer — `Cargo.toml` records that there is no in-tree consumer of
  it today, and `sce-build-wasm` has already built that wrapping once.
- ~~Whether the target engine becomes a codegen argument, a per-artifact
  variant, or a runtime-selected pair of emissions.~~ **Answered 2026-08-28:
  a codegen argument.** The other two were not chosen against so much as ruled
  out by what the artifact is — a Lua-shaped artifact can only run on a Lua
  engine, so the choice has to be made where the artifact is produced, and it
  has to be reported on the manifest for the host that will supply the engine.

This file records the measurement, and now also the decisions taken on it.
