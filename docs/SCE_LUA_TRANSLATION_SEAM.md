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

| Backend | Guard site | Filter chain | Hands the engine |
|---|---|---|---|
| **C++** | `tools/codegen/templates/process_transition.jinja2:638` | `{{ trans.cond \| escape_cpp }}` | **ECMAScript source** |
| **Kotlin** | `tools/codegen/templates/kotlin/transition_actions.kt.jinja2:24` | `{{ trans.cond \| escape_kotlin }}` | **ECMAScript source** |
| Rust | `tools/codegen/templates/rust/state_machine.rs.jinja2:996` | `{{ trans.cond \| to_lua_guard \| escape_rust }}` | translated Lua |
| Go | `tools/codegen/templates/go/process_transition.go.jinja2:170` | `` {{ trans.cond \| to_lua_guard }} `` | translated Lua |
| Python | `tools/codegen/templates/python/process_transition.py.jinja2:35` | `{{ transition.cond \| to_lua_guard \| py_string_literal }}` | translated Lua |
| C11 | `tools/codegen/templates/c/state_machine.c.jinja2:3554` → `se.lua_eval_guard` | `{{ cond \| to_lua_guard \| escape_c }}` (`c/scriptengine.jinja2:612`) | translated Lua |

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
- **Kotlin: a real choice, exercised by benchmarks and a demo.** Generated
  Kotlin takes `scriptEngine: ScxmlScriptEngine` as a *constructor argument*
  (`kotlin/state_machine.kt.jinja2:69`), and `EngineFactory` offers three —
  Rhino, Lua 5.4, QuickJS. Its callers in-tree are the JMH benchmarks
  (`backends/kotlin/benchmark/src/jmh/...`) and the Android app's benchmark
  harness; the W3C suite selects with `-Psce.script.engine=` and
  `ScriptEngineSelectionTest` guards that the selection is honoured.

**The one sentence.** Moving translation to build time costs the ability to run
one generated artifact on more than one engine, and the only place that ability
is exercised in this tree is Kotlin — where two of the three engines offered
(Rhino, QuickJS) answer every ECMA-262 case correctly and would be made
unreachable by a Lua-shaped artifact, so the design must take the target engine
as a codegen input (`script_engine_language` already exists on the manifest,
`sce-build/src/manifest.rs:209`) rather than assume one.

## What is not yet decided

- The C++ **Interpreter** has no build step at all, so it cannot receive
  build-time-translated text by this route. `sce-build`'s cdylib is the
  candidate answer — `Cargo.toml` records that there is no in-tree consumer of
  it today, and `sce-build-wasm` has already built that wrapping once.
- Whether the target engine becomes a codegen argument, a per-artifact variant,
  or a runtime-selected pair of emissions.

This file records the measurement, not that decision.
