# Adding W3C Tests

The procedure for registering a W3C conformance fixture. `CLAUDE.md` points
here so that the steps are read when a fixture is being added, not on every
session.

## Step 1: Commit the SCXML and verify static code generation

Every backend generates a fixture from its committed SCXML, never from the TXML:
`sce-codegen generate-w3c` reads `resources/XXX/testXXX.scxml` and nothing else, and the C++ and C11
CMake route (`cmake/SCEStaticW3CTest.cmake`) reads the same file when the tree is configured and
stops the configuration of a registered fixture that has none. So the conversion is committed, beside
the TXML it came from, and so is the conversion of every sub-fixture the document starts by `src`
(`testXXXsub1.txml` becomes `testXXXsub1.scxml`): the route stages each child document the generator
names from the fixture's directory.

```bash
# Convert TXML to SCXML, into the fixture's directory, and commit the result
build/tools/txml_converter/txml-converter resources/XXX/testXXX.txml resources/XXX/testXXX.scxml

# Try static code generation
mkdir -p /tmp/test_verify
sce-codegen generate resources/XXX/testXXX.scxml -o /tmp/test_verify/ -l cpp

# Emits ONE JSON manifest line to stdout, e.g.:
#   {"v":1,"kind":"generate","artifacts":[{"path":".../testXXX_sm.h"}, ...],"needs_script_engine":false,"needs_event_scheduler":false}
# Interpret:
#   artifacts listed, no "rejected" key   → static generation OK
#   "needs_script_engine": false          → pure static
#   "needs_script_engine": true           → static + embedded script engine (Lua)
#   "needs_event_scheduler": false        → the host may drive it with step()
#   "needs_event_scheduler": true         → the host MUST drive it with tick()
#                                           (delayed <send>/<cancel>, or a child session to tick)
#   "rejected":{"spec":...,"name":...}    → cannot be statically generated (stub files written); Interpreter-only
```

**Do NOT** test against `build/tests/w3c_static_generated/testXXX.scxml` — it doesn't exist until registered.

## Step 2: Register the test

1. **Conformance registry** — `tests/w3c/conformance/fixtures.json`. This
   is the source of truth: `sce-codegen generate-w3c` reads it,
   `sce-codegen list-fixtures --catalog w3c` enumerates it, and the
   visualizer's test list is rendered from it. Entries stay sorted by
   `id`.
   ```json
   { "id": "XXX", "harness": "simple", "summary": "what the fixture exercises" }
   ```
   ⚠ `summary` states **no spec section**. The section a fixture targets is
   `specnum` in `resources/XXX/metadata.txt`, and
   `tools/mnemosyne-adoption/gen_verifies_catalog.py` derives it into
   `docs/spec/scxml/.atomic/verifies-catalog.json` — the one place that
   answers it. Measured 2026-09-13, a hand-typed section contradicted
   `specnum` in 74 of 202 summaries, so `W3cRegistry::load` refuses one.
   `harness` is `simple` (default), `scheduled` for a fixture that needs
   delayed `<send>` to fire, or `http` for one that sends over
   BasicHTTPEventProcessor. Omitting it means `simple`.

2. **CMake registration — there is none to write.** Both CMake routes derive
   it from the registry through one reading of it,
   `sce_w3c_registry_fixtures` (`cmake/SCEW3CRegistry.cmake`), which asks
   `sce-codegen list-fixtures` for every id, in registry order, and the
   `harness` of each. `tests/CMakeLists.txt` calls
   `sce_generate_static_w3c_test()` and `backends/c/tests/CMakeLists.txt`
   calls `sce_generate_static_w3c_c_test()` inside a `foreach` over that
   list, with no filter. A fixture in the registry and absent from either
   build is therefore not a state this tree can reach, which is why the
   parity that used to be checked is now a property of the loops
   (`sce-build/tests/w3c_registry_drives_cmake.rs` holds both routes to it).

   Nor is there a flag to write for what a C11 fixture links. The C11 route
   used to carry its own 202 calls, each flagged `NEEDS_LUA`, `NEEDS_DOM` or
   `NEEDS_HTTP` by hand. What a machine links is the generator's decision:
   each generated C11 machine states what it requires beyond the header-only
   runtime (`lua`, `dom`, `http`, `clock`) in the plan the route asks for
   (`--plan-requires`), includes each requirement's header exactly when it
   states it, and the runner is linked by `sce_c_link_runtime`, which links
   the library `backends/c/runtime` registers for each. How a fixture is run is its `harness`: an `http` fixture's ctest
   requires the BasicHTTP server fixture, on both routes.

   Nor is there a list of the files a fixture's generation writes. The route
   asks `sce-codegen --plan --plan-children generate` of the fixture when the
   tree is configured (`cmake/SCECodegenPlan.cmake`) and declares the answer:
   the machine, every child an inline `<invoke>` synthesizes, and anything a
   declaration adds, such as the header of a declared transition function,
   then the generation of every child document the fixture's generation
   writes or stages, each declared from the plan that answer holds for it. A
   sub-fixture the author wrote beside the fixture is staged from
   `resources/<id>/`; a hybrid invoke's stub is written by the fixture's
   generation and planned from the text it would write, so nothing of it is
   tracked (`hybrid_stubs_are_written_only_by_the_generator`).

   ⚠ This step used to say to hand-write one call per test and to remove the
   id from `W3C_INTERPRETER_ONLY_TESTS`. Measured 2026-09-11 and all three
   claims are false in the tree: `tests/CMakeLists.txt` carries **zero**
   hand-written `sce_generate_static_w3c_test(<id>` calls, the guard this
   section named — `sce-build/tests/w3c_registry_cmake_parity.rs` — **does
   not exist**, and `W3C_INTERPRETER_ONLY_TESTS` is **empty**.
   **Registering a fixture is step 1 alone.**

## Step 3: Create AOT Test Header

Create `tests/w3c/aot_tests/TestXXX.h`:

**Standard test** (SimpleAotTest):
```cpp
#pragma once
#include "SimpleAotTest.h"
#include "testXXX_sm.h"

namespace SCE::W3C::AotTests {

/// Feature description
struct TestXXX : public SimpleAotTest<TestXXX, XXX> {
    using SM = SCE::Generated::testXXX::testXXX;
};

inline static AotTestRegistrar<TestXXX> registrar_TestXXX;

}  // namespace SCE::W3C::AotTests
```

**HTTP test** (HttpAotTest) — for `<send type="BasicHTTPEventProcessor">`:
```cpp
#pragma once
#include "HttpAotTest.h"
#include "testXXX_sm.h"

namespace SCE::W3C::AotTests {

/// BasicHTTP feature description
struct TestXXX : public HttpAotTest<TestXXX, XXX> {
    using SM = SCE::Generated::testXXX::testXXX;
};

inline static AotTestRegistrar<TestXXX> registrar_TestXXX;

}  // namespace SCE::W3C::AotTests
```

⚠ The header's brief names no spec section. The section a fixture targets is
`specnum` in `resources/XXX/metadata.txt`, and the build refuses a header whose
`@brief` states one (`sce-codegen check-aot-briefs`, a dependency of
`w3c_test_cli`). Cite the sections the test touches in the comment's body
instead — those citations are expected there.

## Step 3b: Create the C11 runner

The C11 route compiles every registered fixture with its runner,
`backends/c/tests/aot_tests/test_XXX.c`: an `int main(void)` that drives the
generated machine to completion and returns 0 when it ended in its pass state,
1 otherwise (CTest reads the exit status as the verdict; the C11 backend links
no test framework). A registered fixture without a runner fails the build of
`sce_c_w3c_test_XXX`. What the fixture's machine needs beyond the generated
code is not written anywhere: the runner links what the generator states the
machine requires. The runner's header comment is where what the C11 lowering
of the fixture exercises is written; the registry's `summary` says what the
fixture tests. Every `_init*` answers whether the machine started
(`testXXX_start_status_t`, docs/adr/0096), and the runner reads it through
`REQUIRE_STARTED` (`backends/c/tests/require_started.h`), which fails the run
when the start is refused.

```c
#include <stdio.h>

#include "testXXX_sm.h"
#include "../require_started.h"

int main(void) {
    testXXX_t sm;
    REQUIRE_STARTED(testXXX_init(&sm), TESTXXX_START_OK);
    testXXX_run(&sm);
    int rc = testXXX_ended_in(&sm, TESTXXX_STATE_PASS) ? 0 : 1;
    if (rc != 0) {
        fprintf(stderr, "testXXX: FAIL — active = 0x%08x\n", (unsigned)testXXX_active_states(&sm));
    }
    testXXX_destroy(&sm);
    return rc;
}
```

A fixture that needs delayed `<send>` to fire (`scheduled`) drives the machine
with `testXXX_tick` until it ends or a deadline passes, as the existing
`scheduled` runners do; one of the `http` harness reads the endpoint from
`basic_http_test_endpoint.h`.

## Step 4: Verify

- `type="pure_static"` or `"static_hybrid"` in test output (NOT `"interpreter_fallback"`)
- Both Interpreter and AOT tests pass

## When code generation refuses a document

A refusal is still a thing the generator can emit — `manifest.rs` carries
`STATUS_REJECTED` and a `rejected` field, and Step 1 above shows the shape it
takes in the manifest line. What has changed is **what you do about it**.

⚠ **There is no exclusion list to add to.** `W3C_INTERPRETER_ONLY_TESTS` is
empty and registration is derived from the registry (Step 2), so a fixture
cannot be registered for the Interpreter alone by editing a list. If
`sce-codegen generate` refuses a document you were about to register, that
refusal is the thing to fix or to record — not to route around.

⚠⚠ **Do not trust a remembered list of reasons, including the one this section
used to carry.** It named three — `<invoke srcexpr>`, a document with no
initial state, and `_event.origintype`. Measured 2026-09-11: a document with
**no initial state generates cleanly** (`rc=0`, artifacts written, no
`rejected` key), so at least one of the three had stopped being true and
nothing said so. Ask the generator instead:

```bash
sce-codegen generate <file>.scxml -o /tmp/probe -l cpp
# a "rejected" key in the manifest line is the answer; its absence is too
```
