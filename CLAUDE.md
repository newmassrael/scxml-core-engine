## Guiding Rules

- **ARCHITECTURE.md first**: Read it before modifying Interpreter or AOT engines
- **COMMIT_FORMAT.md first**: Read it before creating commits
- **SCE_ERROR_CONTRACT.md + docs/SCE_ACCEPTED_SUBSET.md first**: Read both before touching diagnostic emission, error types, or the `--error-format=json` wire format (including adding a new `DiagnosticCode` variant, changing a `Fix` shape, or extending a stage). New variants must land in the acceptance-doc appendix (`acceptance_doc_covers_every_code`). Schema-shape edits to `schemas/sce-diagnostic.v1.schema.json` follow §8.1 — while `SCHEMA_STATUS = "pre-release"` non-additive changes are allowed, and any flip to `"stable"` must update both the const and the schema file's `x-sce-schema-status` in one commit (`schema_file_declares_status` guards this).
- **SCE_WIRE_CONTRACTS.md first**: Read it before changing the stability status of any agent-facing wire surface (the diagnostic / forge-AST / sourcemap JSON schemas or the `sce-forge*.xsd` grammars). It is the single registry of which surfaces are `pre-release` vs `stable` and the flip procedure. A status flip must update the producer-side const, the schema-file header, and this registry's row in one commit (`sce-build/tests/wire_surface_stability.rs` + the per-surface `schema_file_declares_status` tests guard this).
- **Root cause only**: Never workaround, band-aid, or skip validation. Fix the actual problem.
- **No Interpreter fallback**: AOT failures must be fixed in the code generator or helpers, not bypassed with Interpreter
- **W3C SCXML complete algorithms**: Implement full spec sections (e.g., Appendix D.2), not test-specific fixes
- **Zero Duplication**: Shared Helper functions between engines (see ARCHITECTURE.md)

## Code Modification Rules

### Code Generator
- **Tool**: `sce-codegen` (Rust binary from `sce-build` crate, minijinja templates)
- **Build**: `cargo build --bin sce-codegen --features cli --release -p sce-build`
- **Templates**: `tools/codegen/templates/` — always modify templates, never generate code directly
- Test changes by regenerating affected test files
- Follow existing template patterns for consistency

### Code Comments
- No phase markers ("Phase 1", "Phase 2", etc.) in code or comments
- Use W3C SCXML spec references: `// W3C SCXML 6.2: Event scheduler for delayed send`
- Reference ARCHITECTURE.md sections for architectural context

## Adding W3C Tests

### Step 1: Verify Static Code Generation

```bash
# Convert TXML to SCXML
mkdir -p /tmp/test_verify
build/tools/txml_converter/txml-converter resources/XXX/testXXX.txml /tmp/test_verify/testXXX.scxml

# Try static code generation
sce-codegen generate /tmp/test_verify/testXXX.scxml -o /tmp/test_verify/ -l cpp

# Check output:
#   "Generated: ...testXXX_sm.h" → static generation OK
#   "Reason: ..." → cannot be statically generated
#   "Needs JSEngine: True/False"
```

**Do NOT** test against `build/tests/w3c_static_generated/testXXX.scxml` — it doesn't exist until registered.

### Step 2: Register in CMakeLists.txt

Add to `tests/CMakeLists.txt` in two places:

1. **Code generation call** (search for `sce_generate_static_w3c_test`):
   ```cmake
   sce_generate_static_w3c_test(XXX ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML X.Y: description
   ```

2. **AOT test registry list** (search for `W3C_AOT_TESTS`):
   ```cmake
   set(W3C_AOT_TESTS
       ...XXX...
   )
   ```
   Remove from `W3C_INTERPRETER_ONLY_TESTS` if present.

### Step 3: Create AOT Test Header

Create `tests/w3c/aot_tests/TestXXX.h`:

**Standard test** (SimpleAotTest):
```cpp
#pragma once
#include "SimpleAotTest.h"
#include "testXXX_sm.h"

namespace SCE::W3C::AotTests {

/// W3C SCXML X.Y.Z: Feature description
struct TestXXX : public SimpleAotTest<TestXXX, XXX> {
    static constexpr const char *DESCRIPTION = "Feature name (W3C X.Y.Z AOT)";
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

/// W3C SCXML C.2: BasicHTTP feature description
struct TestXXX : public HttpAotTest<TestXXX, XXX> {
    static constexpr const char *DESCRIPTION = "BasicHTTP feature (W3C C.2 AOT)";
    using SM = SCE::Generated::testXXX::testXXX;
};

inline static AotTestRegistrar<TestXXX> registrar_TestXXX;

}  // namespace SCE::W3C::AotTests
```

### Step 4: Verify

- `type="pure_static"` or `"static_hybrid"` in test output (NOT `"interpreter_fallback"`)
- Both Interpreter and AOT tests pass

### Tests That Cannot Be Statically Generated

If code generation fails, the test runs on **Interpreter only** — do NOT add to AOT tests.

Common exclusion reasons:
- `<invoke srcexpr="pathVar"/>` — dynamic file I/O
- No initial state — requires runtime default resolution
- `_event.origintype` — runtime metadata

## Code Review Checklist

- [ ] Zero Duplication: shared Helper functions, not duplicate implementations?
- [ ] No phase markers in code or comments?
- [ ] W3C spec references in comments?
- [ ] Jinja2 templates modified (not direct code generation)?
- [ ] Test registered in both CMakeLists.txt locations?
- [ ] TestXXX.h created in `tests/w3c/aot_tests/`?
- [ ] AOT execution verified (not interpreter_fallback)?
- [ ] No TODO, no partial features, no placeholders?
- [ ] Commits follow COMMIT_FORMAT.md?
