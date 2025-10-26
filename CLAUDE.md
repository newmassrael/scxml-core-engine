## Core Development Principles

### Architecture First
- **Required Before Engine Modifications**: Always refer to ARCHITECTURE.md first before modifying Interpreter or AOT (Static) engines
- **Zero Duplication Principle**: Interpreter and AOT engines must share logic through Helper functions
- **Single Source of Truth**: Duplicate implementations prohibited, shared Helper classes required
  - Examples: SendHelper, TransitionHelper, ForeachHelper, GuardHelper

### No Workarounds or Band-Aid Solutions
- **Temporary Fixes Prohibited**: Never implement quick fixes that mask root causes
- **Analyze Before Acting**: When facing compilation/runtime errors, identify the true source of the problem
- **Root Cause Analysis Required**: Understand WHY something fails before attempting to fix it
- **Evidence-Based Solutions**: Use log analysis, debugger output, and W3C SCXML spec to guide fixes
- **Architecture Compliance**: Solutions must align with ARCHITECTURE.md principles, not work around them
- **Examples**:
  - ❌ Bad: Adding optional parameters to avoid circular dependencies
  - ✅ Good: Fixing circular dependencies with forward declarations or restructuring
  - ❌ Bad: "Let's add a flag to skip this validation"
  - ✅ Good: "The validation fails because X, let's fix X properly"
  - ❌ Bad: Changing function signatures to avoid template instantiation errors
  - ✅ Good: Understanding why template instantiation fails and fixing the underlying design issue

### W3C SCXML Perfect Compliance Requirement
- **Complete Implementation Mandatory**: Never implement simplified or partial solutions for W3C SCXML features
- **No "Good Enough" Shortcuts**: Solutions must fully satisfy W3C SCXML specification, not just pass specific tests
- **Forbidden Approaches**:
  - ❌ **Simple fixes that work for one test**: "Sort by document order" without implementing optimal transition set
  - ❌ **Partial algorithm implementation**: "Process first transition only" instead of collecting all enabled transitions
  - ❌ **Test-specific workarounds**: "If test 405, do X" instead of implementing W3C SCXML D.2 algorithm
  - ❌ **Edge case ignoring**: "This works for most cases" instead of handling all W3C SCXML scenarios
- **Required Approach**:
  - ✅ **Full W3C SCXML algorithm implementation**: Read spec sections (e.g., Appendix D) and implement completely
  - ✅ **Systematic architecture**: Build proper infrastructure (e.g., optimal transition set collection/execution)
  - ✅ **Future-proof solutions**: Implementation works for all current and future W3C SCXML tests
  - ✅ **Reference Interpreter**: Match Interpreter engine's complete W3C SCXML compliance
- **Examples**:
  - ❌ Bad: "Break after first eventless transition in parallel states"
  - ✅ Good: "Implement W3C SCXML optimal transition set selection (collect all enabled, execute in document order)"
  - ❌ Bad: "Sort active states by document order"
  - ✅ Good: "Implement microstep algorithm: exit all → execute all transitions → enter all"
  - ❌ Bad: "Add special case for parallel state eventless transitions"
  - ✅ Good: "Implement W3C SCXML Appendix D.2 transition execution order for all state types"
- **Rationale**:
  - Simplified solutions fail for future tests with different edge cases
  - W3C SCXML spec is comprehensive - partial implementations create technical debt
  - AOT engine must match Interpreter's perfect W3C compliance
  - "Just pass this test" mentality leads to unmaintainable codebase

## Code Modification Rules

### Python Code Generator (Production)
- **Production Tool**: `tools/codegen/codegen.py` - Python + Jinja2 template-based code generator
- **Templates**: Located in `tools/codegen/templates/` directory
  - `state_machine.jinja2` - Main state machine structure
  - `actions/*.jinja2` - Individual action handlers (send, assign, if, foreach, etc.)
  - `entry_exit_actions.jinja2` - Entry/exit action generation
  - `process_transition.jinja2` - Transition processing logic
- **Template Modification Rules**:
  - Always modify Jinja2 templates, never generate code directly
  - Test changes by regenerating affected test files
  - Follow existing template patterns for consistency
- **ECMAScript Expression Handling**: Uses Static Hybrid approach
  - Detects ECMAScript features (`typeof`, `_event`, `In()`) automatically
  - Generates JSEngine-embedded code for expression evaluation
  - Maintains static state machine structure (enums, switch statements)
  - See ARCHITECTURE.md "Static Hybrid: ECMAScript Expression Handling" for philosophy

### Code Comments and Documentation
- **No Phase Markers**: Never use "Phase 1", "Phase 2", "Phase 3", "Phase 4" in production code comments or documentation
- **No Temporary Comments**: Avoid temporary markers like "TODO Phase X", "Coming in Phase Y"
- **Production-Ready Only**: All comments must be permanent, production-appropriate documentation
- **W3C References**: Use W3C SCXML specification references instead (e.g., "W3C SCXML 3.12.1", "W3C SCXML C.1")
- **Architecture References**: Reference ARCHITECTURE.md sections for context, not development phases
- **Examples**:
  - ❌ Bad: `// Phase 4: Event scheduler polling`
  - ✅ Good: `// W3C SCXML 6.2: Event scheduler for delayed send`
  - ❌ Bad: `// TODO: Implement in Phase 5`
  - ✅ Good: `// W3C SCXML 3.4: Parallel state support (see ARCHITECTURE.md Future Components)`

## Test Integration Guidelines

### Verifying Static Code Generation Capability
**Before adding tests to CMakeLists.txt**, verify whether they can use static generation or require Interpreter wrappers.

**Code Generator**: The project uses **Python + Jinja2 code generator** (`tools/codegen/codegen.py`) as the production code generator.

**Correct Verification Method**:
```bash
# 1. Convert TXML to SCXML
mkdir -p /tmp/test_verify
build/tools/txml_converter/txml-converter resources/XXX/testXXX.txml /tmp/test_verify/testXXX.scxml

# 2. Try static code generation with Python codegen
env SPDLOG_LEVEL=warn python3 tools/codegen/codegen.py /tmp/test_verify/testXXX.scxml -o /tmp/test_verify/

# 3. Check output
# - If "Generated: /tmp/test_verify/testXXX_sm.h" → static generation OK
# - If "Reason: ..." message → needs Interpreter wrapper
# - Check output for: "Needs JSEngine: True/False"
```

**What to Look For in SCXML**:
- ✅ **Static generation OK**: All event names, delays, targets are static strings
- ❌ **Needs wrapper**: Dynamic expressions (srcexpr, delayexpr, contentexpr), no initial state, invalid initial state

**Common Mistake**:
- ❌ Wrong: Testing `build/tests/w3c_static_generated/testXXX.scxml` (doesn't exist until registered)
- ✅ Correct: Convert from `resources/XXX/testXXX.txml` first

### Adding W3C Tests with Pure Static Generation
**When**: Static code generation succeeds (all features are static, no dynamic expressions)

**Required Steps**:
1. **Add to `tests/CMakeLists.txt`**:
   - Use `rsm_generate_static_w3c_test(TEST_NUM ${STATIC_W3C_OUTPUT_DIR})`
   - Add W3C SCXML specification reference comment
   - Example:
     ```cmake
     rsm_generate_static_w3c_test(279 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML 5.2.2: early binding variable initialization
     ```

2. **Create AOT Test Registry File** `tests/w3c/aot_tests/TestXXX.h`:
   - Follow SimpleAotTest pattern for standard tests
   - Include W3C SCXML specification reference in docstring
   - Example:
     ```cpp
     #pragma once
     #include "SimpleAotTest.h"
     #include "testXXX_sm.h"

     namespace RSM::W3C::AotTests {

     /**
      * @brief W3C SCXML X.Y.Z: Feature description
      *
      * Detailed test description referencing W3C SCXML spec.
      */
     struct TestXXX : public SimpleAotTest<TestXXX, XXX> {
         static constexpr const char *DESCRIPTION = "Feature name (W3C X.Y.Z AOT)";
         using SM = RSM::Generated::testXXX::testXXX;
     };

     // Auto-register
     inline static AotTestRegistrar<TestXXX> registrar_TestXXX;

     }  // namespace RSM::W3C::AotTests
     ```

3. **Add to `tests/w3c/aot_tests/AllAotTests.h`**:
   - Include new test header in appropriate section
   - Example:
     ```cpp
     #include "Test278.h"
     #include "Test279.h"  // Add here
     ```

4. **Result**:
   - Static code generated to `build/tests/w3c_static_generated/testXXX_sm.h`
   - AOT test auto-registered via `AotTestRegistrar`
   - Both Interpreter and AOT tests pass with pure static code

### Handling Tests That Cannot Use Pure Static Generation
**IMPORTANT**: Interpreter wrappers are **NOT USED** in this project. All tests must either:
1. Use **Pure Static Generation** (all features are static)
2. Use **Static Hybrid** (static structure with JSEngine for ECMAScript expressions)
3. Be **excluded from AOT testing** if they cannot be statically generated

**When Static Code Generation Fails**:
If a test cannot be statically generated, it should **NOT be added to AOT tests**.

**Common Scenarios for Exclusion**:
- ❌ **No initial state**: W3C SCXML 3.6 defaults to first child - requires runtime logic
- ❌ **Dynamic file invoke**: `<invoke srcexpr="pathVar"/>` - requires runtime file I/O
- ❌ **Dynamic event names**: Events generated at runtime (e.g., HTTP.POST from content-only sends)
- ❌ **Parallel initial state**: Space-separated state IDs - requires parsing at runtime
- ❌ **Invalid initial state**: Initial state not found in model - requires validation

**Static Hybrid Invoke (Interpreter-Only Currently)**:
- ⚠️ **Content expression invoke**: `<invoke><content expr="var"/></invoke>` - Requires runtime child SCXML parsing and dynamic invocation
- ⚠️ **ContentExpr invoke**: `<invoke contentExpr="expr"/>` - Requires runtime child SCXML parsing and dynamic invocation
- **Note**: AOT engine currently lacks runtime invoke infrastructure. Tests with contentexpr invoke run on Interpreter engine only.
- **Future Enhancement**: AOT invoke with InvokeHelper for runtime child loading (requires AOT parent-child communication infrastructure)

**Exclusion Procedure**:
1. **Do NOT add to `tests/CMakeLists.txt`** for static generation
2. **Do NOT create AOT test registry file** in `tests/w3c/aot_tests/`
3. **Test only with Interpreter engine** - Interpreter tests will still run and pass
4. **Document the reason** in code comments or test documentation

**Result**:
- Test passes with Interpreter engine (100% W3C SCXML compliance)
- AOT engine skips the test (no false failures)
- Clear separation between Pure Static/Static Hybrid and dynamic features

### Adding W3C Tests Requiring HTTP Infrastructure (BasicHTTP Event I/O Processor)
**IMPORTANT**: W3C SCXML C.2 BasicHTTP Event I/O Processor tests require special infrastructure setup in AOT engine.

**When**: Test uses `<send type="http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor">` with HTTP target URLs

**Required Steps**:
1. **Add to `tests/CMakeLists.txt`**:
   - Use `rsm_generate_static_w3c_test(TEST_NUM ${STATIC_W3C_OUTPUT_DIR})`
   - Add W3C SCXML C.2 specification reference comment
   - Example:
     ```cmake
     rsm_generate_static_w3c_test(520 ${STATIC_W3C_OUTPUT_DIR})  # W3C SCXML C.2: BasicHTTP content element
     ```

2. **Create AOT Test Registry File** `tests/w3c/aot_tests/TestXXX.h`:
   - Use `HttpAotTest` base class (NOT SimpleAotTest)
   - Include W3C SCXML C.2 specification reference in docstring
   - Example:
     ```cpp
     #pragma once
     #include "HttpAotTest.h"
     #include "testXXX_sm.h"

     namespace RSM::W3C::AotTests {

     /**
      * @brief W3C SCXML C.2: BasicHTTP feature description
      *
      * Detailed test description referencing W3C SCXML spec.
      */
     struct TestXXX : public HttpAotTest<TestXXX, XXX> {
         static constexpr const char *DESCRIPTION = "BasicHTTP feature (W3C C.2 AOT)";
         using SM = RSM::Generated::testXXX::testXXX;
     };

     // Auto-register
     inline static AotTestRegistrar<TestXXX> registrar_TestXXX;

     }  // namespace RSM::W3C::AotTests
     ```

3. **Add to `tests/w3c/aot_tests/AllAotTests.h`**:
   - Include new test header in BasicHTTP section
   - Group with other HTTP tests for clarity

4. **Result**:
   - Static code generated with HTTP send support (EventWithMetadata with target/type fields)
   - HttpAotTest starts W3CHttpTestServer on localhost:8080/test
   - StaticExecutionEngine.raiseExternal() detects HTTP target and sends real HTTP POST
   - Server response triggers event → HttpAotTest callback routes to state machine
   - State machine processes event and transitions to pass

**HttpAotTest Architecture**:
- **HTTP Server**: Starts W3CHttpTestServer on localhost:8080/test
- **Event Callback**: Maps HTTP response event names to state machine Event enum
- **Dynamic Event Mapping**: Iterates Event enum to find matching event name string
- **Async Processing**: Polls state machine until final state or timeout
- **Direct Integration**: StaticExecutionEngine uses HttpEventTarget directly (no EventTargetFactory)

**Key Differences from SimpleAotTest**:
- **SimpleAotTest**: Synchronous, self-contained (initialize → tick → check final state)
- **HttpAotTest**: Asynchronous, HTTP server lifecycle management + event callback routing
- **HTTP Detection**: StaticExecutionEngine checks EventWithMetadata.originType for BasicHTTPEventProcessor
- **Zero Duplication**: Reuses Interpreter's HttpEventTarget, W3CHttpTestServer

**Common HTTP Test Patterns**:
- **Namelist encoding** (test 518): `<send event="test" namelist="Var1" type="BasicHTTP" target="http://localhost:8080/test">`
- **Param encoding** (test 519): `<send event="test" type="BasicHTTP"><param name="x" expr="1"/></send>`
- **Content-only** (test 520): `<send type="BasicHTTP" target="http://localhost:8080/test"><content>text</content></send>`

**Architecture Compliance**:
- ✅ **Zero Duplication**: HTTP infrastructure shared with Interpreter (HttpEventTarget, W3CHttpTestServer)
- ✅ **All-or-Nothing**: Pure AOT structure + external HTTP server (no engine mixing)
- ✅ **W3C Compliance**: Real HTTP POST operations, not fake/mock implementation
- ✅ **No False Positives**: Verified by execution time (307ms vs 1ms) and debug logs showing actual network traffic

## Code Review Guidelines

### Required References for Code Review
**When performing code reviews**, always refer to these documents in order:

1. **ARCHITECTURE.md** - Core architecture principles and design decisions
   - Zero Duplication Principle (Helper functions)
   - All-or-Nothing Strategy (AOT vs Interpreter)
   - Feature Handling Strategy (Static vs Dynamic)
   - Single Source of Truth requirements

2. **CLAUDE.md** - Code quality and documentation standards
   - No Phase Markers rule
   - W3C SCXML references required
   - Python code generator (codegen.py) template modification rules
   - Test Integration Guidelines

3. **COMMIT_FORMAT.md** - Git commit message conventions
   - Semantic commit prefixes
   - Descriptive messages
   - Professional language

### Code Review Checklist
- [ ] **Architecture Adherence**: Zero Duplication achieved via Helper functions?
- [ ] **Phase Markers**: No "Phase 1/2/3/4" in code or comments?
- [ ] **W3C References**: All comments use W3C SCXML specification references?
- [ ] **Template Modifications**: Jinja2 templates updated (not direct code generation)?
- [ ] **Test Integration**:
  - [ ] Static tests: CMakeLists.txt + AOT registry (TestXXX.h + AllAotTests.h)?
  - [ ] Wrapper tests: CMakeLists.txt + W3CTestRunner.cpp dynamic invoke section?
- [ ] **Implementation Completeness**: No TODO, no partial features, no placeholders?
- [ ] **Git Quality**: Semantic commits with professional descriptions?

## Git Commit Guidelines

### Commit Message Format
- **Required Before Committing**: Always refer to COMMIT_FORMAT.md for commit message format
- Follow the project's commit message conventions and structure