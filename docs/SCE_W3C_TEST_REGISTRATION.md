# Adding W3C Tests

The procedure for registering a W3C conformance fixture. `CLAUDE.md` points
here so that the steps are read when a fixture is being added, not on every
session.

## Step 1: Verify Static Code Generation

```bash
# Convert TXML to SCXML
mkdir -p /tmp/test_verify
build/tools/txml_converter/txml-converter resources/XXX/testXXX.txml /tmp/test_verify/testXXX.scxml

# Try static code generation
sce-codegen generate /tmp/test_verify/testXXX.scxml -o /tmp/test_verify/ -l cpp

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

2. **CMake registration — there is none to write.** `tests/CMakeLists.txt`
   derives it: it asks `sce-codegen list-fixtures` for every id, reads each
   one's `harness`, and calls `sce_generate_static_w3c_test()` inside a
   `foreach` over that list, with no filter. A fixture in the registry and
   absent from the build is therefore not a state this tree can reach, which
   is why the parity that used to be checked is now a property of the loop.

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
