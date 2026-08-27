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

**38 sites must move together — and do NOT enumerate them by grepping for a
filter.** This section first said 26, counted by looking for `escape_cpp` on
an expression-bearing field. That method is wrong three ways, each of which
hides sites rather than showing them:

1. **A site can hand the engine raw source with no filter at all.**
   `actions/foreach.jinja2:48,94` pass `{{ action.array }}` unfiltered to
   `ForeachHelper::executeForeachWithActions` / `…WithoutBody`, which
   evaluates it. No `escape_cpp`, so no filter-keyed scan can see them.
2. **A helper can re-enter the engine with the ORIGINAL text.**
   `ScriptResultUtils::resultToString(result, engine, sid, originalExpression)`
   takes the expression for a `JSON.stringify` fallback on complex objects
   (`sce/include/scripting/ScriptResultUtils.h:38`), so
   `actions/send.jinja2:80,113,533,901`, `actions/log.jinja2:12` and
   `entry_exit_actions.jinja2:269` reach the engine a second time.
3. **A shape can appear twice.** `DataModelInitHelper::initializeVariableFromExpr`
   is at `datamodel_macros.jinja2:20` *and* `scriptengine_helpers.jinja2:164`;
   the first count caught only the one it happened to grep for.

Count by ENTRY POINT instead — what the generated code calls to reach the
engine — and re-derive rather than trusting this number:

```sh
# C++-owned templates = everything outside rust/ kotlin/ go/ python/ c/
grep -rnE "evaluateExpression\(|safeEvaluateGuard\(|executeScript\(|initializeVariableFromExpr\(|resultToString\(|resultToStringArray\(|executeForeachWith" \
  tools/codegen/templates/*.jinja2 tools/codegen/templates/actions/*.jinja2 \
  tools/codegen/templates/_macros/*.jinja2
```

Measured 2026-08-28: **38** such sites carry a model expression
(`cond` / `expr` / `content` / `typeexpr` / `targetexpr` / `sendidexpr` /
`delayexpr` / `contentexpr` / `srcexpr` / `array`). ⚠ This method is itself
shape-based: a NEW helper that evaluates on the caller's behalf would be
missed the same way `ForeachHelper` and `resultToString` were. Whoever does
the split should widen the entry-point list from
`sce/include/scripting/` rather than assume this one is closed.

Splitting a subset is not a smaller version of this change: the engine would
receive Lua from some sites and ECMAScript from others, in one session — and
the sites easiest to miss are exactly the ones no filter marks.

**The blocker.** `LuaEngine` transforms on every first evaluation. Its "fast
path" (`LuaEngine.cpp:563`, `:598`) is a memo cache keyed on the *input
string*, not a bypass for text that is already Lua — the slow path calls
`transformer_.transformScript` / `transform` unconditionally. Feeding it
lowered Lua therefore runs the rewriter over the frontend's own output, and
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
| `ScriptResultUtils::resultToString(…, originalExpression)` | `ScriptResultUtils.h:38` | yes (6 sites) |
| `ScriptResultUtils::resultToStringArray(…, originalExpression)` | `ScriptResultUtils.h:49` | no |

All of them mean "the author's source", and nothing in
`sce/include/scripting/` or `sce/src/scripting/` accepts pre-lowered text.
This section previously said the interface had "exactly two evaluation entry
points"; it has three, and `validateExpression` being dead surface is its own
decision — extend it with the seam or retire it — rather than something to
discover while implementing.

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

**So the order is: seam, then templates.** The seam is a contract about *what
language the string is*, which means an engine that cannot evaluate that
language must refuse rather than try — QuickJS handed Lua is the case, and the
mesh-rpc refusal in `sce-build/src/generator.rs` is the shape that refusal
should take. Only once an engine can be handed lowered text safely does
`--script-engine lua` have anywhere to send it.

## What is not yet decided

- The C++ **Interpreter** has no build step at all, so it cannot receive
  build-time-translated text by this route. `sce-build`'s cdylib is the
  candidate answer — `Cargo.toml` records that there is no in-tree consumer of
  it today, and `sce-build-wasm` has already built that wrapping once.
- Whether the target engine becomes a codegen argument, a per-artifact variant,
  or a runtime-selected pair of emissions.

This file records the measurement, not that decision.
